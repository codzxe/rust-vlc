//! Bounded subprocess execution (ffmpeg regeneration, the VLC oracle,
//! the lane steps). Every child gets a wall-clock budget from the
//! config; an overrun is killed and recorded, never hung. Output goes
//! to temp files (never OS pipes — a chatty child cannot deadlock the
//! bounded wait) and only a bounded head/tail slice is retained.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::ToolError;

/// How much captured output is retained per stream (bytes).
const OUTPUT_CAP: usize = 64 * 1024;

/// One executed command's record.
#[derive(Debug, Clone, Serialize)]
pub struct ProcRecord {
    /// The argv (element 0 = binary).
    pub argv: Vec<String>,
    /// Exit status: `exit <n>`, `killed (timeout)` or the termination.
    pub outcome: String,
    /// Wall seconds.
    pub wall_s: f64,
    /// First bytes of stdout.
    pub stdout_head: String,
    /// Last bytes of stderr.
    pub stderr_tail: String,
}

/// A unique temp-file pair for one child's output.
struct TempOutputs {
    stdout: PathBuf,
    stderr: PathBuf,
}

impl TempOutputs {
    fn new() -> Result<TempOutputs, ToolError> {
        let base = std::env::temp_dir().join(format!(
            "compat-matrix-{}-{}",
            std::process::id(),
            started_counter()
        ));
        let stdout = base.with_extension("out");
        let stderr = base.with_extension("err");
        Ok(TempOutputs { stdout, stderr })
    }

    fn cleanup(&self) {
        let _ = std::fs::remove_file(&self.stdout);
        let _ = std::fs::remove_file(&self.stderr);
    }
}

/// A per-process counter for unique temp names (no atomics needed:
/// single-threaded tool).
fn started_counter() -> u64 {
    use std::cell::Cell;
    thread_local! {
        static COUNTER: Cell<u64> = const { Cell::new(0) };
    }
    COUNTER.with(|counter| {
        let next = counter.get().wrapping_add(1);
        counter.set(next);
        next
    })
}

/// Run `argv` with a deadline. `cwd` and `env` augment the child's
/// environment (env entries are set verbatim).
///
/// # Errors
/// Only spawn-level failures (binary missing). A nonzero exit or a
/// timeout is an `Ok(ProcRecord)` — the caller decides what it means.
pub fn run_bounded(
    argv: &[String],
    cwd: Option<&Path>,
    env: &[(String, String)],
    timeout_s: u64,
) -> Result<ProcRecord, ToolError> {
    if argv.is_empty() {
        return Err(ToolError::Subprocess("empty argv".to_string()));
    }
    let temp = TempOutputs::new()?;
    let out_file = match std::fs::File::create(&temp.stdout) {
        Ok(file) => file,
        Err(error) => {
            return Err(ToolError::io(
                format!("creating temp stdout {}", temp.stdout.display()),
                error,
            ));
        }
    };
    let err_file = match std::fs::File::create(&temp.stderr) {
        Ok(file) => file,
        Err(error) => {
            return Err(ToolError::io(
                format!("creating temp stderr {}", temp.stderr.display()),
                error,
            ));
        }
    };
    let mut command = Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::from(out_file))
        .stderr(Stdio::from(err_file));
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    for (key, value) in env {
        command.env(key, value);
    }
    let started = Instant::now();
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            temp.cleanup();
            return Err(ToolError::io(format!("spawning {}", argv[0]), error));
        }
    };

    // Bounded wait: poll the child; kill on deadline.
    let deadline = Duration::from_secs(timeout_s.max(1));
    let outcome = loop {
        match child.try_wait() {
            Ok(Some(status)) => break describe_status(&status),
            Ok(None) => {
                if started.elapsed() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break "killed (timeout)".to_string();
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => {
                temp.cleanup();
                return Err(ToolError::io(format!("waiting for {}", argv[0]), error));
            }
        }
    };
    let wall_s = started.elapsed().as_secs_f64();
    let stdout = read_capped(&temp.stdout);
    let stderr = read_capped(&temp.stderr);
    temp.cleanup();
    Ok(ProcRecord {
        argv: argv.to_vec(),
        outcome,
        wall_s,
        stdout_head: stdout,
        stderr_tail: stderr,
    })
}

fn describe_status(status: &std::process::ExitStatus) -> String {
    if let Some(code) = status.code() {
        format!("exit {code}")
    } else {
        format!("terminated ({status})")
    }
}

/// Read a file's first bytes (output here is advisory, not evidence
/// cargo — the exit code and artifacts are).
fn read_capped(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes).to_string();
            clip(&text, OUTPUT_CAP, true)
        }
        Err(_) => String::new(),
    }
}

/// Clip a string to `limit` bytes from the head or the tail.
fn clip(text: &str, limit: usize, from_head: bool) -> String {
    if text.len() <= limit {
        return text.to_string();
    }
    if from_head {
        let mut end = limit;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &text[..end])
    } else {
        let mut start = text.len() - limit;
        while start < text.len() && !text.is_char_boundary(start) {
            start += 1;
        }
        format!("…{}", &text[start..])
    }
}
