use crate::discovery::Program;
use anyhow::{anyhow, Context};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[derive(Clone, Debug)]
pub enum ProgramStatus {
    Idle,
    Running(&'static str),
    Stopped,
    Failed(String),
    MissingScripts,
}

#[derive(Clone, Debug)]
pub enum RunnerEvent {
    Status { index: usize, status: ProgramStatus },
    Message { text: String },
    LogLine { index: usize, line: String },
    TraceLine { line: String },
    ModuleState { index: usize, attached: Option<bool> },
}

#[derive(Clone, Debug)]
pub struct RunConfig {
    pub artifacts_dir: PathBuf,
}

#[derive(Clone, Copy, Debug)]
pub enum RunAction {
    Load,
    Stop,
    Verify,
}

impl RunAction {
    fn label(self) -> &'static str {
        match self {
            RunAction::Load => "load",
            RunAction::Stop => "stop",
            RunAction::Verify => "verify",
        }
    }
}

pub fn spawn_run_action_selected(
    tx: mpsc::Sender<RunnerEvent>,
    stop_flag: Arc<AtomicBool>,
    index: usize,
    program: Program,
    config: RunConfig,
    action: RunAction,
) {
    thread::spawn(move || {
        if stop_flag.load(Ordering::Relaxed) {
            return;
        }
        let res = run_manual_action(&tx, &stop_flag, index, &program, &config, action);

        handle_run_result(&tx, index, &program, action, res);
    });
}

fn handle_run_result(
    tx: &mpsc::Sender<RunnerEvent>,
    index: usize,
    program: &Program,
    action: RunAction,
    res: anyhow::Result<()>,
) {
    if let Err(err) = res {
        if is_stop_error(&err) {
            let _ = tx.send(RunnerEvent::Status {
                index,
                status: ProgramStatus::Stopped,
            });
            let _ = tx.send(RunnerEvent::Message {
                text: format!("{}: STOPPED", program.name),
            });
            return;
        }
        let failed = action.label().to_string();
        let _ = tx.send(RunnerEvent::Status {
            index,
            status: ProgramStatus::Failed(failed),
        });
        let _ = tx.send(RunnerEvent::Message {
            text: format!("{}: FAILED: {:#}", program.name, err),
        });
    }
}

fn is_stop_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    msg.contains("Stop requested") || msg.contains("interrupted by stop request")
}

fn run_manual_action(
    tx: &mpsc::Sender<RunnerEvent>,
    stop_flag: &AtomicBool,
    index: usize,
    program: &Program,
    config: &RunConfig,
    action: RunAction,
) -> anyhow::Result<()> {
    let scripts = Scripts::detect(&program.dir);
    let out_dir = config.artifacts_dir.join(program.name.replace('/', "__"));
    fs::create_dir_all(&out_dir).with_context(|| format!("create {}", out_dir.display()))?;

    tx.send(RunnerEvent::Message {
        text: format!("{}: manual action {}", program.name, action.label()),
    })
    .ok();

    let native = NativeProgram::detect(&program.dir)?;
    if action.label() != "verify" && !scripts.is_complete() && native.is_none() {
        tx.send(RunnerEvent::Status {
            index,
            status: ProgramStatus::MissingScripts,
        })
        .ok();
        return Err(anyhow!(
            "Missing scripts/native sources in {}",
            program.dir.display()
        ));
    }

    match action {
        RunAction::Load => {
            if scripts.is_complete() {
                run_step_to_log(
                    tx,
                    stop_flag,
                    index,
                    program,
                    "build",
                    &scripts.build,
                    &out_dir.join("build.log"),
                )?;
                run_step_to_log(
                    tx,
                    stop_flag,
                    index,
                    program,
                    "load",
                    &scripts.load,
                    &out_dir.join("load.log"),
                )?;
                let state = ModuleStateFiles::new(&out_dir);
                state.set_attached(true)?;
                tx.send(RunnerEvent::ModuleState {
                    index,
                    attached: Some(true),
                })
                .ok();
                tx.send(RunnerEvent::Status {
                    index,
                    status: ProgramStatus::Running("run"),
                })
                .ok();
                tx.send(RunnerEvent::Message {
                    text: format!("{}: RUNNING (attached; press s to stop)", program.name),
                })
                .ok();
            } else if let Some(native) = native.as_ref() {
                run_native_build(tx, stop_flag, index, program, native, &out_dir)?;
                run_native_load(tx, stop_flag, index, program, native, &out_dir)?;
                tx.send(RunnerEvent::Status {
                    index,
                    status: ProgramStatus::Running("run"),
                })
                .ok();
                tx.send(RunnerEvent::Message {
                    text: format!("{}: RUNNING (native attached; press s to stop)", program.name),
                })
                .ok();
            }
        }
        RunAction::Stop => {
            if scripts.is_complete() {
                run_step_to_log(
                    tx,
                    stop_flag,
                    index,
                    program,
                    "unload",
                    &scripts.unload,
                    &out_dir.join("unload.log"),
                )?;
                let state = ModuleStateFiles::new(&out_dir);
                state.set_attached(false)?;
                tx.send(RunnerEvent::ModuleState {
                    index,
                    attached: Some(false),
                })
                .ok();
                tx.send(RunnerEvent::Status {
                    index,
                    status: ProgramStatus::Stopped,
                })
                .ok();
            } else if native.is_some() {
                run_native_unload(tx, stop_flag, index, program, &out_dir)?;
                tx.send(RunnerEvent::Status {
                    index,
                    status: ProgramStatus::Stopped,
                })
                .ok();
            }
        }
        RunAction::Verify => {
            // Скрипты verify лежат в <ebpf-tui>/verify/<module_folder_name>.sh
            // Определяем путь к папке verify относительно CARGO_MANIFEST_DIR или artifacts
            let module_folder = program.dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&program.name);
            let verify_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("verify");
            let verify_script = verify_dir.join(format!("{}.sh", module_folder));

            if !verify_script.exists() {
                // Fallback: ищем verify.sh в самой папке модуля (обратная совместимость)
                let fallback = program.dir.join("verify.sh");
                if !fallback.exists() {
                    tx.send(RunnerEvent::Message {
                        text: format!("{}: no verify script found", program.name),
                    })
                    .ok();
                    return Err(anyhow!(
                        "verify script not found: {} or {}",
                        verify_script.display(),
                        fallback.display()
                    ));
                }
                tx.send(RunnerEvent::Status {
                    index,
                    status: ProgramStatus::Running("verify"),
                })
                .ok();
                run_step_to_log(
                    tx, stop_flag, index, program, "verify",
                    &fallback, &out_dir.join("verify.log"),
                )?;
            } else {
                tx.send(RunnerEvent::Status {
                    index,
                    status: ProgramStatus::Running("verify"),
                })
                .ok();
                // Запускаем verify из папки модуля (cwd = program.dir)
                let command = format!("chmod +x '{}' && '{}'", verify_script.display(), verify_script.display());
                let result = run_shell_and_stream(
                    tx, stop_flag, index, &program.dir, &command, Some("verify"),
                )
                .with_context(|| format!("verify for {}", program.name))?;
                let log_path = out_dir.join("verify.log");
                fs::write(&log_path, result.output.as_bytes())
                    .with_context(|| format!("write {}", log_path.display()))?;
                if !result.success {
                    return Err(anyhow!("verify failed (see {})", log_path.display()));
                }
            }
            tx.send(RunnerEvent::Status {
                index,
                status: ProgramStatus::Stopped,
            })
            .ok();
            tx.send(RunnerEvent::Message {
                text: format!("{}: VERIFY completed", program.name),
            })
            .ok();
        }
    };
    Ok(())
}

fn kill_process_group(pid: u32) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGTERM);
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
    }
}

fn kill_process_group_force(pid: u32) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
    }
}

#[cfg(unix)]
fn configure_process_group(cmd: &mut Command) {
    unsafe {
        cmd.pre_exec(|| {
            libc::setpgid(0, 0);
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn configure_process_group(_cmd: &mut Command) {}


fn send_log_line(tx: &mpsc::Sender<RunnerEvent>, index: usize, line: String) {
    let _ = tx.send(RunnerEvent::LogLine { index, line });
}

fn send_trace_line(tx: &mpsc::Sender<RunnerEvent>, line: String) {
    let _ = tx.send(RunnerEvent::TraceLine { line });
}

fn run_native_build(
    tx: &mpsc::Sender<RunnerEvent>,
    stop_flag: &AtomicBool,
    index: usize,
    program: &Program,
    native: &NativeProgram,
    out_dir: &Path,
) -> anyhow::Result<()> {
    tx.send(RunnerEvent::Status {
        index,
        status: ProgramStatus::Running("build"),
    })
    .ok();

    let command = native.build_command();
    let log_path = out_dir.join("build.log");
    let result = run_shell_and_stream(tx, stop_flag, index, &program.dir, &command, Some("build"))
        .with_context(|| format!("native build for {}", program.name))?;
    fs::write(&log_path, result.output.as_bytes())
        .with_context(|| format!("write {}", log_path.display()))?;

    if stop_flag.load(Ordering::Relaxed) {
        return Err(anyhow!("build interrupted by stop request (see {})", log_path.display()));
    }
    if !result.success {
        return Err(anyhow!("build failed (see {})", log_path.display()));
    }

    Ok(())
}

fn run_native_load(
    tx: &mpsc::Sender<RunnerEvent>,
    stop_flag: &AtomicBool,
    index: usize,
    program: &Program,
    native: &NativeProgram,
    out_dir: &Path,
) -> anyhow::Result<()> {
    tx.send(RunnerEvent::Status {
        index,
        status: ProgramStatus::Running("load"),
    })
    .ok();

    let pid_path = out_dir.join(".native_pid");
    let run_log = out_dir.join("native_run.log");
    let load_log = out_dir.join("load.log");
    remove_stale_log(&out_dir.join("unload.log"));
    remove_stale_log(&out_dir.join("verify.log"));
    let command = format!(
        "set -euo pipefail\n\
         rm -f {pid}\n\
         : > {log}\n\
         chmod +x {bin}\n\
         setsid ./{bin_name} > {log} 2>&1 &\n\
         pid=$!\n\
         echo \"$pid\" > {pid}\n\
         for _ in $(seq 1 80); do\n\
           if grep -q '\\[RUN\\]\\|\\[VERIFY\\] PASS' {log}; then exit 0; fi\n\
           if ! kill -0 \"$pid\" 2>/dev/null; then wait \"$pid\"; exit $?; fi\n\
           sleep 0.1\n\
         done\n\
         exit 0",
        pid = sh_quote_path(&pid_path),
        log = sh_quote_path(&run_log),
        bin = sh_quote(&native.binary_name),
        bin_name = shell_escape_single_arg(&native.binary_name),
    );

    let result = run_shell_and_stream(tx, stop_flag, index, &program.dir, &command, Some("load"))
        .with_context(|| format!("native load for {}", program.name))?;
    fs::write(&load_log, result.output.as_bytes())
        .with_context(|| format!("write {}", load_log.display()))?;
    emit_log_file(tx, index, &run_log, Some("load"));

    if stop_flag.load(Ordering::Relaxed) {
        return Err(anyhow!("load interrupted by stop request (see {})", load_log.display()));
    }
    if !result.success {
        return Err(anyhow!("load failed (see {})", run_log.display()));
    }

    let state = ModuleStateFiles::new(out_dir);
    state.set_attached(true)?;
    tx.send(RunnerEvent::ModuleState {
        index,
        attached: Some(true),
    })
    .ok();
    tx.send(RunnerEvent::Status {
        index,
        status: ProgramStatus::Running("run"),
    })
    .ok();
    Ok(())
}

fn run_native_unload(
    tx: &mpsc::Sender<RunnerEvent>,
    stop_flag: &AtomicBool,
    index: usize,
    program: &Program,
    out_dir: &Path,
) -> anyhow::Result<()> {
    tx.send(RunnerEvent::Status {
        index,
        status: ProgramStatus::Running("unload"),
    })
    .ok();

    let pid_path = out_dir.join(".native_pid");
    let unload_log = out_dir.join("unload.log");
    let command = format!(
        "set -euo pipefail\n\
         if [ ! -s {pid} ]; then echo 'not attached'; exit 0; fi\n\
         pid=$(cat {pid})\n\
         kill -TERM -- -\"$pid\" 2>/dev/null || kill -TERM \"$pid\" 2>/dev/null || true\n\
         for _ in $(seq 1 50); do\n\
           if ! kill -0 \"$pid\" 2>/dev/null; then rm -f {pid}; exit 0; fi\n\
           sleep 0.1\n\
         done\n\
         kill -KILL -- -\"$pid\" 2>/dev/null || kill -KILL \"$pid\" 2>/dev/null || true\n\
         rm -f {pid}",
        pid = sh_quote_path(&pid_path),
    );

    let result = run_shell_and_stream(tx, stop_flag, index, &program.dir, &command, Some("unload"))
        .with_context(|| format!("native unload for {}", program.name))?;
    fs::write(&unload_log, result.output.as_bytes())
        .with_context(|| format!("write {}", unload_log.display()))?;
    emit_log_file(tx, index, &out_dir.join("native_run.log"), Some("unload"));

    if stop_flag.load(Ordering::Relaxed) {
        return Err(anyhow!("unload interrupted by stop request (see {})", unload_log.display()));
    }
    if !result.success {
        return Err(anyhow!("unload failed (see {})", unload_log.display()));
    }

    let state = ModuleStateFiles::new(out_dir);
    state.set_attached(false)?;
    tx.send(RunnerEvent::ModuleState {
        index,
        attached: Some(false),
    })
    .ok();
    Ok(())
}

fn emit_log_file(
    tx: &mpsc::Sender<RunnerEvent>,
    index: usize,
    path: &Path,
    step: Option<&'static str>,
) {
    if let Ok(raw) = fs::read_to_string(path) {
        for line in raw.lines().take(200) {
            send_log_line(tx, index, format_line_for_status(step, line));
        }
    }
}

fn remove_stale_log(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

fn sh_quote_path(path: &Path) -> String {
    sh_quote(&path.to_string_lossy())
}

fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn shell_escape_single_arg(value: &str) -> String {
    value.replace('\'', "'\\''")
}

fn run_step_to_log(
    tx: &mpsc::Sender<RunnerEvent>,
    stop_flag: &AtomicBool,
    index: usize,
    program: &Program,
    step: &'static str,
    script: &Path,
    log_path: &Path,
) -> anyhow::Result<()> {
    if !script.exists() {
        return Err(anyhow!(
            "missing {} script in {}",
            step,
            program.dir.display()
        ));
    }

    if stop_flag.load(Ordering::Relaxed) {
        return Err(anyhow!("Stop requested"));
    }

    tx.send(RunnerEvent::Status {
        index,
        status: ProgramStatus::Running(step),
    })
    .ok();

    let script_name = script
        .file_name()
        .ok_or_else(|| anyhow!("invalid script path: {}", script.display()))?
        .to_string_lossy();
    let command = format!("chmod +x '{}' && './{}'", script.display(), script_name);

    let result = run_shell_and_stream(
        tx,
        stop_flag,
        index,
        &program.dir,
        &command,
        Some(step),
    )
    .with_context(|| format!("run {} for {}", step, program.name))?;

    let mut file = fs::File::create(log_path)
        .with_context(|| format!("create log {}", log_path.display()))?;
    file.write_all(result.output.as_bytes()).ok();

    if stop_flag.load(Ordering::Relaxed) {
        return Err(anyhow!("{} interrupted by stop request (see {})", step, log_path.display()));
    }

    if !result.success {
        return Err(anyhow!("{} failed (see {})", step, log_path.display()));
    }

    Ok(())
}

struct ModuleStateFiles {
    attached_flag: PathBuf,
}

impl ModuleStateFiles {
    fn new(out_dir: &Path) -> Self {
        Self {
            attached_flag: out_dir.join(".attached"),
        }
    }

    fn set_attached(&self, attached: bool) -> anyhow::Result<()> {
        if attached {
            fs::write(&self.attached_flag, b"attached\n")
                .with_context(|| format!("write {}", self.attached_flag.display()))?;
        } else if self.attached_flag.exists() {
            fs::remove_file(&self.attached_flag)
                .with_context(|| format!("remove {}", self.attached_flag.display()))?;
        }
        Ok(())
    }
}

fn run_shell_and_stream(
    tx: &mpsc::Sender<RunnerEvent>,
    stop_flag: &AtomicBool,
    index: usize,
    current_dir: &Path,
    command: &str,
    step: Option<&'static str>,
) -> anyhow::Result<CommandResult> {
    send_log_line(
        tx,
        index,
        format_line_for_status(
            step,
            &format!("cwd={} cmd={}", current_dir.display(), command),
        ),
    );

    let mut cmd = Command::new("bash");
    cmd.arg("-lc")
        .arg(command)
        .current_dir(current_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_group(&mut cmd);
    let mut child = cmd
        .spawn()
        .with_context(|| format!("spawn command: {}", command))?;

    let stdout = child.stdout.take().context("capture stdout")?;
    let stderr = child.stderr.take().context("capture stderr")?;

    let (line_tx, line_rx) = mpsc::channel::<(bool, String)>();

    {
        let line_tx = line_tx.clone();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim_end_matches(['\r', '\n']);
                        let _ = line_tx.send((false, trimmed.to_string()));
                    }
                    Err(_) => break,
                }
            }
        });
    }

    thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim_end_matches(['\r', '\n']);
                    let _ = line_tx.send((true, trimmed.to_string()));
                }
                Err(_) => break,
            }
        }
    });

    let mut collected = String::new();
    let mut killed_by_stop = false;
    let mut stop_since: Option<Instant> = None;
    let child_pid = child.id();
    let success;

    loop {
        while let Ok((is_stderr, line)) = line_rx.try_recv() {
            if is_stderr {
                let rendered = format!("[stderr] {}", line);
                send_log_line(tx, index, format_line_for_status(step, &rendered));
                collected.push_str(&rendered);
                collected.push('\n');
            } else {
                send_log_line(tx, index, format_line_for_status(step, &line));
                collected.push_str(&line);
                collected.push('\n');
            }
        }

        if stop_flag.load(Ordering::Relaxed) {
            killed_by_stop = true;
            if stop_since.is_none() {
                stop_since = Some(Instant::now());
                kill_process_group(child_pid);
            } else if stop_since
                .map(|t| t.elapsed() > Duration::from_secs(2))
                .unwrap_or(false)
            {
                kill_process_group_force(child_pid);
            }
        }

        if let Some(status) = child.try_wait().context("wait command status")? {
            success = status.success() && !killed_by_stop;
            while let Ok((is_stderr, line)) = line_rx.try_recv() {
                if is_stderr {
                    let rendered = format!("[stderr] {}", line);
                    send_log_line(tx, index, format_line_for_status(step, &rendered));
                    collected.push_str(&rendered);
                } else {
                    send_log_line(tx, index, format_line_for_status(step, &line));
                    collected.push_str(&line);
                }
                collected.push('\n');
            }

            collected.push_str("--- EXIT CODE: ");
            collected.push_str(&status.code().unwrap_or(-1).to_string());
            collected.push_str(" ---\n");
            break;
        }

        thread::sleep(std::time::Duration::from_millis(100));
    }

    if killed_by_stop {
        send_log_line(tx, index, format_line_for_status(step, "stopped by user"));
    }

    Ok(CommandResult {
        output: collected,
        success,
    })
}

pub fn spawn_global_trace(
    tx: mpsc::Sender<RunnerEvent>,
    trace_cmd: String,
    artifacts_dir: PathBuf,
) {
    thread::spawn(move || {
        if let Err(err) = fs::create_dir_all(&artifacts_dir) {
            let _ = tx.send(RunnerEvent::Message {
                text: format!("trace: failed to create artifacts dir: {}", err),
            });
            return;
        }

        let log_path = artifacts_dir.join("trace_global.log");
        let log_file = match fs::File::create(&log_path) {
            Ok(f) => f,
            Err(err) => {
                let _ = tx.send(RunnerEvent::Message {
                    text: format!("trace: failed to create log: {}", err),
                });
                return;
            }
        };

        let log_writer = Arc::new(Mutex::new(std::io::BufWriter::new(log_file)));

        let mut cmd = Command::new("bash");
        cmd.arg("-lc")
            .arg(&trace_cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure_process_group(&mut cmd);

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(err) => {
                let _ = tx.send(RunnerEvent::Message {
                    text: format!("trace: failed to start: {}", err),
                });
                return;
            }
        };

        let _ = tx.send(RunnerEvent::Message {
            text: format!("trace started -> {}", log_path.display()),
        });

        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => return,
        };
        let stderr = match child.stderr.take() {
            Some(s) => s,
            None => return,
        };

        let tx_out = tx.clone();
        let log_out = log_writer.clone();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim_end_matches(['\r', '\n']);
                        send_trace_line(&tx_out, trimmed.to_string());
                        if let Ok(mut w) = log_out.lock() {
                            let _ = writeln!(w, "{}", trimmed);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let tx_err = tx.clone();
        let log_err = log_writer.clone();
        thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim_end_matches(['\r', '\n']);
                        let rendered = format!("[stderr] {}", trimmed);
                        send_trace_line(&tx_err, rendered.clone());
                        if let Ok(mut w) = log_err.lock() {
                            let _ = writeln!(w, "{}", rendered);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let _ = child.wait();

        let _ = tx.send(RunnerEvent::Message {
            text: "trace stopped".to_string(),
        });
    });
}

fn format_line_for_status(step: Option<&'static str>, line: &str) -> String {
    match step {
        Some(s) => format!("[{}] {}", s, line),
        None => line.to_string(),
    }
}

struct CommandResult {
    output: String,
    success: bool,
}

#[derive(Clone, Debug)]
struct NativeProgram {
    bpf_sources: Vec<PathBuf>,
    loader_source: PathBuf,
    binary_name: String,
}

impl NativeProgram {
    fn detect(dir: &Path) -> anyhow::Result<Option<Self>> {
        let mut bpf_sources = Vec::new();
        let mut c_sources = Vec::new();

        for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }

            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            if name.ends_with(".bpf.c") {
                bpf_sources.push(path);
            } else if name.ends_with(".c") {
                c_sources.push(path);
            }
        }

        if bpf_sources.is_empty() || c_sources.is_empty() {
            return Ok(None);
        }

        bpf_sources.sort();
        c_sources.sort();

        let loader_source = c_sources
            .iter()
            .find(|p| {
                !p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.ends_with("_loader.c"))
                    .unwrap_or(false)
                    && fs::read_to_string(p)
                        .map(|raw| raw.contains(".skel.h"))
                        .unwrap_or(false)
            })
            .or_else(|| {
                c_sources.iter().find(|p| {
                    fs::read_to_string(p)
                        .map(|raw| raw.contains(".skel.h"))
                        .unwrap_or(false)
                })
            })
            .or_else(|| c_sources.first())
            .cloned()
            .context("select native loader source")?;

        let binary_name = loader_source
            .file_stem()
            .and_then(|s| s.to_str())
            .context("native loader source without valid stem")?
            .to_string();

        Ok(Some(Self {
            bpf_sources,
            loader_source,
            binary_name,
        }))
    }

    fn build_command(&self) -> String {
        let mut command = String::from(
            "set -euo pipefail\n\
             if [ -r /sys/kernel/btf/vmlinux ]; then\n\
               bpftool btf dump file /sys/kernel/btf/vmlinux format c > vmlinux.h\n\
             fi\n",
        );

        for src in &self.bpf_sources {
            let name = src
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            let base = name.strip_suffix(".bpf.c").unwrap_or(name);
            let obj = format!("{}.bpf.o", base);
            let skel = format!("{}.skel.h", base);

            command.push_str(&format!(
                "clang -g -O2 -target bpf -D__TARGET_ARCH_x86 -I. -c {} -o {}\n",
                sh_quote(name),
                sh_quote(&obj)
            ));
            command.push_str(&format!(
                "bpftool gen skeleton {} > {}\n",
                sh_quote(&obj),
                sh_quote(&skel)
            ));
        }

        let loader = self
            .loader_source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        command.push_str(&format!(
            "gcc -g -O2 -I. {} -o {} -lbpf -lelf -lz\n",
            sh_quote(loader),
            sh_quote(&self.binary_name)
        ));

        command
    }
}


#[derive(Clone, Debug)]
struct Scripts {
    build: PathBuf,
    load: PathBuf,
    test: PathBuf,
    unload: PathBuf,
}

impl Scripts {
    fn detect(dir: &Path) -> Self {
        Self {
            build: dir.join("build.sh"),
            load: dir.join("load.sh"),
            test: dir.join("test.sh"),
            unload: dir.join("unload.sh"),
        }
    }

    fn is_complete(&self) -> bool {
        self.build.exists() && self.load.exists() && self.test.exists() && self.unload.exists()
    }
}
