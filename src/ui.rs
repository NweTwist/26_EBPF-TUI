use crate::{discovery::Program, runner};
use anyhow::Context;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use serde::Deserialize;
use std::{
    collections::VecDeque,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
        Arc,
    },
};

#[derive(Debug, Deserialize, Default)]
pub struct ConfigFile {
    pub trace_cmd: Option<String>,
    pub artifacts_dir: Option<PathBuf>,
}

pub fn load_config(config_path: Option<&Path>, repo_root: &Path) -> anyhow::Result<ConfigFile> {
    let path = config_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| repo_root.join("ebpf-tui.yaml"));

    if !path.exists() {
        return Ok(ConfigFile::default());
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("read config {}", path.display()))?;
    let cfg: ConfigFile = serde_yaml::from_str(&content)
        .with_context(|| format!("parse yaml {}", path.display()))?;
    Ok(cfg)
}

#[derive(Clone, Debug)]
pub struct ProgramEntry {
    pub program: Program,
    pub status: runner::ProgramStatus,
    pub attached: bool,
}

pub struct App {
    pub entries: Vec<ProgramEntry>,
    pub selected: usize,
    pub list_state: ListState,
    pub last_message: String,
    pub status_lines: VecDeque<String>,
    pub status_scroll_offset: u16,
    pub status_user_scrolled: bool,
    pub artifacts_dir: PathBuf,
    pub tx: mpsc::Sender<runner::RunnerEvent>,
    pub stop_flag: Arc<AtomicBool>,
    pub status_log_path: PathBuf,
}

impl App {
    pub fn new(
        _repo_root: PathBuf,
        programs: Vec<Program>,
        artifacts_dir: PathBuf,
        tx: mpsc::Sender<runner::RunnerEvent>,
    ) -> Self {
        let entries: Vec<ProgramEntry> = programs
            .into_iter()
            .map(|p| ProgramEntry {
                program: p,
                status: runner::ProgramStatus::Idle,
                attached: false,
            })
            .collect();

        let status_log_path = artifacts_dir.join("status_window.log");
        if let Some(parent) = status_log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&status_log_path, b"");

        let mut list_state = ListState::default();
        if !entries.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            entries,
            selected: 0,
            list_state,
            last_message: "Ready. Keys: l load+run, s stop, q quit".to_string(),
            status_lines: VecDeque::new(),
            status_scroll_offset: 0,
            status_user_scrolled: false,
            artifacts_dir,
            tx,
            stop_flag: Arc::new(AtomicBool::new(false)),
            status_log_path,
        }
    }

    pub fn select_prev(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        if self.selected == 0 {
            self.selected = self.entries.len() - 1;
        } else {
            self.selected -= 1;
        }
        self.list_state.select(Some(self.selected));
    }

    pub fn select_next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.entries.len();
        self.list_state.select(Some(self.selected));
    }

    pub fn apply_runner_event(&mut self, ev: runner::RunnerEvent) {
        match ev {
            runner::RunnerEvent::Status { index, status } => {
                if let Some(entry) = self.entries.get_mut(index) {
                    entry.status = status;
                }
            }
            runner::RunnerEvent::Message { text } => {
                self.last_message = text;
                self.push_status_line(self.last_message.clone());
                self.scroll_status_to_bottom();
            }
            runner::RunnerEvent::LogLine { index, line } => {
                let prefix = self
                    .entries
                    .get(index)
                    .map(|e| e.program.name.clone())
                    .unwrap_or_else(|| "module".to_string());
                self.push_status_line(format!("{} | {}", prefix, line));
                self.scroll_status_to_bottom();
            }
            runner::RunnerEvent::TraceLine { line } => {
                self.push_status_line(format!("trace | {}", line));
                self.scroll_status_to_bottom();
            }
            runner::RunnerEvent::ModuleState { index, attached } => {
                if let Some(entry) = self.entries.get_mut(index) {
                    if let Some(v) = attached {
                        entry.attached = v;
                    }
                }
            }
        }
    }

    pub fn request_stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    pub fn load_selected(&mut self) {
        self.run_action(runner::RunAction::Load);
    }

    pub fn stop_selected(&mut self) {
        self.run_action(runner::RunAction::Stop);
    }

    fn run_action(&mut self, action: runner::RunAction) {
        if self.entries.is_empty() {
            return;
        }
        self.stop_flag.store(false, Ordering::Relaxed);
        // Очищаем status при новом запуске, чтобы лог обновлялся
        self.status_lines.clear();
        self.status_scroll_offset = 0;
        self.status_user_scrolled = false;
        // Перезаписываем файл лога status_window
        let _ = fs::write(&self.status_log_path, b"");
        let index = self.selected;
        let program = self.entries[index].program.clone();
        let config = runner::RunConfig {
            artifacts_dir: self.artifacts_dir.clone(),
        };
        runner::spawn_run_action_selected(
            self.tx.clone(),
            self.stop_flag.clone(),
            index,
            program,
            config,
            action,
        );
    }

    fn push_status_line(&mut self, line: String) {
        const MAX_STATUS_LINES: usize = 2000;
        self.status_lines.push_back(line.clone());
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.status_log_path)
        {
            let _ = writeln!(f, "{}", line);
        }
        while self.status_lines.len() > MAX_STATUS_LINES {
            self.status_lines.pop_front();
        }
    }

    fn status_lines_for_render(&self) -> Vec<Line<'static>> {
        if self.status_lines.is_empty() {
            return vec![Line::from(self.last_message.clone())];
        }

        self.status_lines
            .iter()
            .map(|s| Line::from(s.clone()))
            .collect()
    }

    pub fn scroll_status_up(&mut self) {
        let max = self.status_lines.len().saturating_sub(1) as u16;
        self.status_scroll_offset = self.status_scroll_offset.saturating_add(3).min(max);
        self.status_user_scrolled = true;
    }

    pub fn scroll_status_down(&mut self) {
        if self.status_scroll_offset <= 3 {
            self.status_scroll_offset = 0;
            self.status_user_scrolled = false;
        } else {
            self.status_scroll_offset = self.status_scroll_offset.saturating_sub(3);
        }
    }

    fn scroll_status_to_bottom(&mut self) {
        // Автоскролл только если пользователь не скроллил вручную
        if !self.status_user_scrolled {
            self.status_scroll_offset = 0;
        }
    }

    /// Сброс ручного скролла (Home / g)
    pub fn scroll_status_reset(&mut self) {
        self.status_scroll_offset = 0;
        self.status_user_scrolled = false;
    }

    /// Вычисляет позицию скролла для виджета Status.
    /// offset=0 означает "показать конец", offset>0 — подняться вверх.
    fn status_scroll_position(&self, visible_height: u16) -> u16 {
        let total_lines = self.status_lines.len() as u16;
        if total_lines <= visible_height {
            return 0;
        }
        let max_scroll = total_lines.saturating_sub(visible_height);
        max_scroll.saturating_sub(self.status_scroll_offset)
    }
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(7)])
        .split(frame.size());

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let items: Vec<ListItem> = app
        .entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let status = status_text(&entry.status);
            let is_selected = idx == app.selected;
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(entry.program.name.clone(), style.add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled(status, style),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Programs"))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::White));
    frame.render_stateful_widget(list, main[0], &mut app.list_state);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(main[1]);

    // Вычисляем видимую высоту Status (минус 2 на рамку)
    let status_visible_height = right[0].height.saturating_sub(2);
    let scroll_pos = app.status_scroll_position(status_visible_height);

    let status_content = app.status_lines_for_render();
    let info = Paragraph::new(status_content)
        .wrap(Wrap { trim: false })
        .scroll((scroll_pos, 0))
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Status [lines:{} scroll:{}]",
            app.status_lines.len(),
            app.status_scroll_offset,
        )));
    frame.render_widget(info, right[0]);

    let details = selected_program_details(app);
    let details_widget = Paragraph::new(details)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Module card"));
    frame.render_widget(details_widget, right[1]);

    let help = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "↑/↓",
            Style::default().add_modifier(Modifier::BOLD),
        ), Span::raw(" select  ")]),
        Line::from(vec![
            Span::styled("l", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" load+run  "),
            Span::styled("s", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" stop/detach  "),
            Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" quit"),
        ]),
        Line::from(vec![
            Span::styled("[/]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" or "),
            Span::styled("Shift+↑/↓", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" or "),
            Span::styled("PgUp/PgDn", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" scroll Status  "),
            Span::styled("g", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" bottom"),
        ]),
        Line::from(vec![Span::raw("Trace: always on (trace_global.log)")]),
        Line::from(vec![Span::raw(
            "Logs: artifacts/<program>/ | Full status: artifacts/status_window.log",
        )]),
    ])
    .block(Block::default().borders(Borders::ALL).title("Help"));
    frame.render_widget(help, chunks[1]);
}

fn selected_program_details(app: &App) -> Vec<Line<'static>> {
    let Some(entry) = app.entries.get(app.selected) else {
        return vec![Line::from("No program selected")];
    };

    let (progress, tone) = status_progress(&entry.status);
    let micro = module_microcopy(&entry.program.name);
    let bar = progress_bar(progress, 18);
    let status = status_text(&entry.status);

    vec![
        Line::from(vec![
            Span::styled("Module: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(entry.program.name.clone()),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(status, Style::default().fg(tone)),
        ]),
        Line::from(vec![
            Span::styled("Progress: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(bar, Style::default().fg(tone)),
            Span::raw(format!(" {}%", progress)),
        ]),
        Line::from(vec![
            Span::styled("Attached: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(if entry.attached { "yes" } else { "no" }),
            Span::raw("    "),
            Span::styled("Trace: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("on"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Micro: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(micro.to_string()),
        ]),
    ]
}

fn status_progress(status: &runner::ProgramStatus) -> (u8, Color) {
    match status {
        runner::ProgramStatus::Idle => (0, Color::Gray),
        runner::ProgramStatus::Running("build") => (25, Color::Yellow),
        runner::ProgramStatus::Running("load") => (45, Color::Yellow),
        runner::ProgramStatus::Running("test") => (75, Color::Yellow),
        runner::ProgramStatus::Running("unload") => (90, Color::Yellow),
        runner::ProgramStatus::Running("trace") => (60, Color::Yellow),
        runner::ProgramStatus::Running("trace-stop") => (95, Color::Yellow),
        runner::ProgramStatus::Running("run") => (80, Color::Yellow),
        runner::ProgramStatus::Running(_) => (55, Color::Yellow),
        runner::ProgramStatus::Stopped => (100, Color::LightYellow),
        runner::ProgramStatus::Failed(_) => (100, Color::Red),
        runner::ProgramStatus::MissingScripts => (100, Color::LightRed),
    }
}

fn progress_bar(progress: u8, width: usize) -> String {
    let filled = (usize::from(progress) * width) / 100;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "#".repeat(filled), "-".repeat(empty))
}

fn module_microcopy(name: &str) -> &'static str {
    if name.contains("XDP") {
        "Fast path packet gate. Hooks early in RX path."
    } else if name.contains("TRACEPOINT") || name.contains("KPROBE") {
        "Kernel observability probe. Captures execution-level events."
    } else if name.contains("CGROUP") {
        "Policy lens for cgroup-bound resource and network controls."
    } else if name.contains("SOCKET") || name.contains("SOCK") {
        "Socket pipeline control. Shapes behavior at transport boundaries."
    } else if name.contains("SCHED") {
        "Scheduler datapath module. Classify and steer queued traffic."
    } else if name.contains("NETFILTER") {
        "Netfilter hook module. Makes decisions on packet traversal."
    } else {
        "eBPF module under validation. Build, attach, trigger, verify."
    }
}

fn status_text(status: &runner::ProgramStatus) -> String {
    match status {
        runner::ProgramStatus::Idle => "idle".to_string(),
        runner::ProgramStatus::Running(step) => format!("running: {}", step),
        runner::ProgramStatus::Stopped => "stopped".to_string(),
        runner::ProgramStatus::Failed(step) => format!("failed: {}", step),
        runner::ProgramStatus::MissingScripts => "missing scripts".to_string(),
    }
}
