use crate::{discovery::Program, runner};
use anyhow::Context;
use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        BarChart, Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Sparkline,
        Table, Tabs, Wrap,
    },
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

// ─── Config ───────────────────────────────────────────────────────────────────

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

// ─── Data types ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ProgramEntry {
    pub program: Program,
    pub status: runner::ProgramStatus,
    pub attached: bool,
}

#[derive(Clone, Debug)]
pub struct EventRecord {
    pub timestamp: String,
    pub module: String,
    pub event_type: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveTab {
    Runner,
    Statistics,
}

// ─── App state ────────────────────────────────────────────────────────────────

pub struct App {
    pub entries: Vec<ProgramEntry>,
    pub selected: usize,
    pub list_state: ListState,
    pub last_message: String,
    /// Все строки лога Status (до 2000)
    pub status_lines: Vec<String>,
    /// Текущая строка скролла (0 = самый верх; по умолчанию = конец)
    pub status_scroll: usize,
    /// Если true — пользователь скроллил вручную, не двигаем автоматически
    pub status_pinned: bool,
    pub artifacts_dir: PathBuf,
    pub tx: mpsc::Sender<runner::RunnerEvent>,
    pub stop_flag: Arc<AtomicBool>,
    pub status_log_path: PathBuf,

    // Statistics
    pub active_tab: ActiveTab,
    pub event_log: VecDeque<EventRecord>,
    pub event_table_scroll: usize,
    /// События в секунду (последние 60 точек)
    pub events_per_second: VecDeque<u64>,
    pub last_second_tick: u64,
    pub current_second_count: u64,
    /// Счётчики для гистограммы (только реальные операции)
    pub count_build: u64,
    pub count_load: u64,
    pub count_run: u64,
    pub count_stop: u64,
    pub count_fail: u64,
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

        let now_sec = Local::now().timestamp() as u64;

        Self {
            entries,
            selected: 0,
            list_state,
            last_message: "Ready. Tab=switch view, l=load, s=stop, q=quit".to_string(),
            status_lines: Vec::new(),
            status_scroll: 0,
            status_pinned: false,
            artifacts_dir,
            tx,
            stop_flag: Arc::new(AtomicBool::new(false)),
            status_log_path,
            active_tab: ActiveTab::Runner,
            event_log: VecDeque::new(),
            event_table_scroll: 0,
            events_per_second: VecDeque::from(vec![0u64; 60]),
            last_second_tick: now_sec,
            current_second_count: 0,
            count_build: 0,
            count_load: 0,
            count_run: 0,
            count_stop: 0,
            count_fail: 0,
        }
    }

    pub fn switch_tab(&mut self) {
        self.active_tab = match self.active_tab {
            ActiveTab::Runner => ActiveTab::Statistics,
            ActiveTab::Statistics => ActiveTab::Runner,
        };
    }

    // ─── Module list ──────────────────────────────────────────────────────

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

    // ─── Status scroll (ПРОСТАЯ ЛОГИКА) ───────────────────────────────────
    //
    // status_scroll = номер первой видимой строки (от 0).
    // По умолчанию показываем конец (status_scroll = max возможный).
    // Вверх: уменьшаем status_scroll.
    // Вниз: увеличиваем status_scroll.
    // "g" или автоскролл: ставим status_scroll = конец.

    pub fn scroll_up(&mut self, step: usize) {
        self.status_pinned = true;
        self.status_scroll = self.status_scroll.saturating_sub(step);
    }

    pub fn scroll_down(&mut self, step: usize) {
        self.status_pinned = true;
        self.status_scroll = self.status_scroll.saturating_add(step);
        // Ограничим максимумом при рендере
    }

    pub fn scroll_to_end(&mut self) {
        self.status_pinned = false;
        // status_scroll будет пересчитан при рендере
        self.status_scroll = usize::MAX;
    }

    /// Вызывается при добавлении новой строки — если не pinned, двигаем вниз
    fn auto_scroll(&mut self) {
        if !self.status_pinned {
            self.status_scroll = usize::MAX;
        }
    }

    // ─── Event table scroll ───────────────────────────────────────────────

    pub fn event_table_up(&mut self) {
        self.event_table_scroll = self.event_table_scroll.saturating_sub(1);
    }

    pub fn event_table_down(&mut self) {
        let max = self.event_log.len().saturating_sub(1);
        if self.event_table_scroll < max {
            self.event_table_scroll += 1;
        }
    }

    // ─── Runner events ────────────────────────────────────────────────────

    pub fn apply_runner_event(&mut self, ev: runner::RunnerEvent) {
        match ev {
            runner::RunnerEvent::Status { index, status } => {
                if let Some(entry) = self.entries.get_mut(index) {
                    match &status {
                        runner::ProgramStatus::Running("build") => self.count_build += 1,
                        runner::ProgramStatus::Running("load") => self.count_load += 1,
                        runner::ProgramStatus::Running("run") => {
                            // Если мы вернулись в "run" после "verify", не удваиваем счётчик запусков
                            if !matches!(entry.status, runner::ProgramStatus::Running("verify")) {
                                self.count_run += 1;
                            }
                        }
                        runner::ProgramStatus::Stopped => self.count_stop += 1,
                        runner::ProgramStatus::Failed(_) => self.count_fail += 1,
                        _ => {}
                    }
                    entry.status = status;
                }
            }
            runner::RunnerEvent::Message { text } => {
                self.last_message = text.clone();
                self.push_status_line(self.last_message.clone());
                self.record_event("system", "message", &text);
            }
            runner::RunnerEvent::LogLine { index, line } => {
                let prefix = self
                    .entries
                    .get(index)
                    .map(|e| e.program.name.clone())
                    .unwrap_or_else(|| "module".to_string());
                self.push_status_line(format!("{} | {}", prefix, line));
                self.record_event(&prefix, "log", &line);
            }
            runner::RunnerEvent::TraceLine { line } => {
                self.push_status_line(format!("trace | {}", line));
                self.record_event("trace", "trace", &line);
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

    fn push_status_line(&mut self, line: String) {
        const MAX: usize = 2000;
        self.status_lines.push(line.clone());
        if self.status_lines.len() > MAX {
            self.status_lines.drain(0..self.status_lines.len() - MAX);
        }
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.status_log_path)
        {
            let _ = writeln!(f, "{}", line);
        }
        self.auto_scroll();
    }

    fn record_event(&mut self, module: &str, event_type: &str, message: &str) {
        const MAX_EVENTS: usize = 5000;
        let ts = Local::now().format("%H:%M:%S").to_string();
        self.event_log.push_back(EventRecord {
            timestamp: ts,
            module: module.to_string(),
            event_type: event_type.to_string(),
            message: message.chars().take(120).collect(),
        });
        while self.event_log.len() > MAX_EVENTS {
            self.event_log.pop_front();
        }

        // Sparkline
        let now_sec = Local::now().timestamp() as u64;
        if now_sec != self.last_second_tick {
            let gap = (now_sec - self.last_second_tick).min(60);
            for _ in 0..gap.saturating_sub(1) {
                self.events_per_second.push_back(0);
            }
            self.events_per_second.push_back(self.current_second_count);
            self.current_second_count = 0;
            self.last_second_tick = now_sec;
            while self.events_per_second.len() > 60 {
                self.events_per_second.pop_front();
            }
        }
        self.current_second_count += 1;
    }

    // ─── Actions ──────────────────────────────────────────────────────────

    pub fn request_stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    pub fn load_selected(&mut self) {
        self.run_action(runner::RunAction::Load);
    }

    pub fn stop_selected(&mut self) {
        self.run_action(runner::RunAction::Stop);
    }

    pub fn verify_selected(&mut self) {
        self.run_action(runner::RunAction::Verify);
    }

    fn run_action(&mut self, action: runner::RunAction) {
        if self.entries.is_empty() {
            return;
        }
        self.stop_flag.store(false, Ordering::Relaxed);
        // Для verify НЕ очищаем лог — трассировки добавляются к существующим
        if !matches!(action, runner::RunAction::Verify) {
            self.status_lines.clear();
            self.status_scroll = 0;
            self.status_pinned = false;
            let _ = fs::write(&self.status_log_path, b"");
        }
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
}

// ═══════════════════════════════════════════════════════════════════════════════
// RENDERING
// ═══════════════════════════════════════════════════════════════════════════════

pub fn render(frame: &mut Frame, app: &mut App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(frame.size());

    // Tab bar
    let tab_titles = vec![
        Line::from(" Runner "),
        Line::from(" Statistics "),
    ];
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title("ebpf-tui"))
        .select(match app.active_tab {
            ActiveTab::Runner => 0,
            ActiveTab::Statistics => 1,
        })
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    frame.render_widget(tabs, outer[0]);

    match app.active_tab {
        ActiveTab::Runner => render_runner_tab(frame, app, outer[1]),
        ActiveTab::Statistics => render_statistics_tab(frame, app, outer[1]),
    }
}

fn render_runner_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(4)])
        .split(area);

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[0]);

    // ─── Programs list ────────────────────────────────────────────────────
    let items: Vec<ListItem> = app
        .entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let status = format_program_status(&entry.status);
            let is_selected = idx == app.selected;
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default()
            };
            let status_style = if is_selected {
                style
            } else {
                status_color_style(&entry.status)
            };
            let line = Line::from(vec![
                Span::styled(entry.program.name.clone(), style.add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled(status, status_style),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Programs"))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::White));
    frame.render_stateful_widget(list, main[0], &mut app.list_state);

    // ─── Right: Status + Module card ──────────────────────────────────────
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main[1]);

    // Status window с прокруткой
    let visible_h = right[0].height.saturating_sub(2) as usize;
    let total = app.status_lines.len();

    // Нормализуем scroll: если usize::MAX или больше максимума — ставим на конец
    let max_scroll = total.saturating_sub(visible_h);
    if app.status_scroll > max_scroll {
        app.status_scroll = max_scroll;
    }

    let scroll_pos = app.status_scroll as u16;

    let status_content: Vec<Line<'static>> = if app.status_lines.is_empty() {
        vec![Line::from(app.last_message.clone())]
    } else {
        app.status_lines.iter().map(|s| Line::from(s.clone())).collect()
    };

    let pinned_indicator = if app.status_pinned { " PINNED" } else { "" };
    let info = Paragraph::new(status_content)
        .wrap(Wrap { trim: false })
        .scroll((scroll_pos, 0))
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Status [{}/{}{}]",
            app.status_scroll + 1,
            total,
            pinned_indicator,
        )));
    frame.render_widget(info, right[0]);

    // Module card
    let details = selected_program_details(app);
    let details_widget = Paragraph::new(details)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Module card"));
    frame.render_widget(details_widget, right[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Tab", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" view  "),
        Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" select  "),
        Span::styled("l", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" load  "),
        Span::styled("v", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" verify  "),
        Span::styled("s", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" stop  "),
        Span::styled("[/]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" scroll  "),
        Span::styled("g", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" end  "),
        Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" quit"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[1]);
}

fn render_statistics_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Sparkline + info
            Constraint::Length(9),  // Histogram
            Constraint::Min(8),    // Event table
            Constraint::Length(3), // Help
        ])
        .split(area);

    // ─── 1. Sparkline ─────────────────────────────────────────────────────
    let spark_data: Vec<u64> = app.events_per_second.iter().copied().collect();
    let current = app.current_second_count;
    let max_val = spark_data.iter().copied().max().unwrap_or(0).max(current);
    let avg: u64 = if spark_data.is_empty() {
        0
    } else {
        spark_data.iter().sum::<u64>() / spark_data.len() as u64
    };
    let total_events = app.event_log.len();

    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "Активность (60s) | now:{}/s max:{}/s avg:{}/s total:{}",
                    current, max_val, avg, total_events
                )),
        )
        .data(&spark_data)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(sparkline, chunks[0]);

    // ─── 2. Histogram (без trace) ────────────────────────────────────────
    let bar_data: Vec<(&str, u64)> = vec![
        ("build", app.count_build),
        ("load", app.count_load),
        ("run", app.count_run),
        ("stop", app.count_stop),
        ("fail", app.count_fail),
    ];
    let barchart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Гистограмма операций"),
        )
        .data(&bar_data)
        .bar_width(8)
        .bar_gap(1)
        .bar_style(Style::default().fg(Color::Green))
        .value_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    frame.render_widget(barchart, chunks[1]);

    // ─── 3. Event table ───────────────────────────────────────────────────
    let header_cells = ["Time", "Module", "Type", "Message"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells)
        .style(Style::default().fg(Color::Yellow))
        .height(1);

    let visible_height = chunks[2].height.saturating_sub(3) as usize;
    let total = app.event_log.len();
    let display_start = app.event_table_scroll.min(total.saturating_sub(visible_height.max(1)));
    let display_end = (display_start + visible_height).min(total);

    let rows: Vec<Row> = app
        .event_log
        .iter()
        .skip(display_start)
        .take(display_end.saturating_sub(display_start))
        .map(|ev| {
            let style = match ev.event_type.as_str() {
                "trace" => Style::default().fg(Color::Cyan),
                "log" => Style::default().fg(Color::White),
                "message" => Style::default().fg(Color::Yellow),
                _ => Style::default(),
            };
            Row::new(vec![
                Cell::from(ev.timestamp.clone()),
                Cell::from(ev.module.clone()),
                Cell::from(ev.event_type.clone()),
                Cell::from(ev.message.clone()),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(22),
            Constraint::Length(8),
            Constraint::Min(30),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(format!(
        "Таблица событий [{}/{}]",
        display_start + 1,
        total
    )));
    frame.render_widget(table, chunks[2]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Tab", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Runner  "),
        Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" scroll table  "),
        Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" quit"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[3]);
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn selected_program_details(app: &App) -> Vec<Line<'static>> {
    let Some(entry) = app.entries.get(app.selected) else {
        return vec![Line::from("No program selected")];
    };

    let (progress, tone) = status_progress(&entry.status);
    let micro = module_microcopy(&entry.program.name);
    let bar = progress_bar(progress, 18);
    let status = format_program_status(&entry.status);

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
            Span::styled("Info: ", Style::default().add_modifier(Modifier::BOLD)),
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
        runner::ProgramStatus::Running("verify") => (90, Color::Cyan),
        runner::ProgramStatus::Running(_) => (55, Color::Yellow),
        runner::ProgramStatus::Stopped => (100, Color::Green),
        runner::ProgramStatus::Failed(_) => (100, Color::Red),
        runner::ProgramStatus::MissingScripts => (100, Color::LightRed),
    }
}

fn status_color_style(status: &runner::ProgramStatus) -> Style {
    match status {
        runner::ProgramStatus::Idle => Style::default().fg(Color::Gray),
        runner::ProgramStatus::Running(_) => Style::default().fg(Color::Yellow),
        runner::ProgramStatus::Stopped => Style::default().fg(Color::Green),
        runner::ProgramStatus::Failed(_) => Style::default().fg(Color::Red),
        runner::ProgramStatus::MissingScripts => Style::default().fg(Color::LightRed),
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

fn format_program_status(status: &runner::ProgramStatus) -> String {
    match status {
        runner::ProgramStatus::Idle => "idle".to_string(),
        runner::ProgramStatus::Running(step) => format!("running: {}", step),
        runner::ProgramStatus::Stopped => "ok(stopped)".to_string(),
        runner::ProgramStatus::Failed(step) => format!("FAILED: {}", step),
        runner::ProgramStatus::MissingScripts => "missing scripts".to_string(),
    }
}
