//! `lumen-sweep` — the corpus-v4 sweep instrument (card 7-d).
//!
//! ```text
//! Usage: lumen-sweep [--timeout-secs N] [--max-files N] [(CLASS=)DIR-OR-FILE]...
//!        lumen-sweep --file PATH   (worker mode — internal, spawned per file)
//! ```
//!
//! - Supervisor mode walks every given directory (recursively,
//!   dot-entries skipped, deterministic sort) or file, and for EACH
//!   file spawns itself in worker mode (`--file`), enforces the
//!   per-file deadline (`--timeout-secs`, default 30), classifies
//!   the outcome (ok / unsupported / dirty-reject / typed-error /
//!   panic / timeout), prints one tab-separated `SWEEPLINE` per
//!   file, and finishes with the per-class tally tables.
//! - `CLASS=PATH` overrides the corpus-class derivation (labels are
//!   `[a-z0-9_-]+`; the auto rule reads the generator's
//!   `NNNN-<class>.<ext>` naming — see `outcome::class_of`).
//! - `--max-files N` caps the run (a smoke-valve, not a way to hide
//!   files: the summary always states the cap).
//! - Raw logs = redirected stdout (`> sweep.log`): every SWEEPLINE
//!   plus the summary block is the raw evidence.
//!
//! Exit codes: 0 the run completed (refusals, panics and timeouts
//! are honest per-file outcomes, not tool failures); 64 usage; 70
//! the supervisor could not locate its own executable.
//!
//! # Panic isolation: subprocess-per-file (the ONE choice, justified)
//!
//! The card allows subprocess-per-file OR `catch_unwind`; this tool
//! uses SUBPROCESSES, because `catch_unwind` is not an isolation
//! boundary for the cases the census actually cares about:
//!
//! 1. **Aborts are invisible to it.** A `panic=abort` build, an OOM
//!    abort, or a stack overflow (deep recursion in a fuzzer-bait
//!    corpus file) kills the process — `catch_unwind` never runs.
//!    Only the process boundary survives all three.
//! 2. **It cannot enforce a timeout.** A hung decode (a corpus
//!    crafted to spin, or a pathological resync loop) computes
//!    forever; only the supervisor can kill it, and only a child
//!    process can be killed.
//! 3. **It leaks state into the next file.** Whatever the panicking
//!    path allocated, half-updated, or left poisoned poisons every
//!    later file's measurement in-process; a fresh worker starts
//!    from zero.
//!
//! The cost — one process spawn per file — is noise next to any
//! decode (the v4 smoke manifest is 126 files), so the simpler
//! boundary wins on honesty AND simplicity. `mem-audit`'
//! catch-free walk precedent is mirrored in `worker.rs`: the worker
//! deliberately does not catch anything either.

mod outcome;
mod worker;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use outcome::{classify, crash_note, class_of, parse_worker_line, Outcome, Tally, WorkerLine};

/// Usage (stderr; exit 64 path).
const USAGE: &str = "Usage: lumen-sweep [--timeout-secs N] [--max-files N] [(CLASS=)DIR-OR-FILE]...";

/// Default per-file deadline (seconds). The v4 smoke files are
/// small; generous enough that a timeout means a real hang.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// The supervisor's poll interval while waiting on a worker.
const POLL_MS: u64 = 25;

/// The entry point: parse, dispatch, print. Never panics on bad
/// input — usage errors are typed lines + exit 64.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = run(&args);
    std::process::exit(code);
}

/// Run one command-line invocation; returns the exit code.
fn run(args: &[String]) -> i32 {
    // Worker mode first: `--file PATH` runs ONE file and prints its
    // marker line (the subprocess side of the isolation design).
    if args.first().is_some_and(|arg| arg == "--file") {
        let Some(path) = args.get(1) else {
            eprintln!("Usage: lumen-sweep --file PATH");
            return 64;
        };
        let result = worker::walk(Path::new(path));
        println!("{}", worker::marker_line(&result));
        return 0;
    }

    let mut timeout_secs = DEFAULT_TIMEOUT_SECS;
    let mut max_files: Option<usize> = None;
    let mut entries: Vec<String> = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--timeout-secs" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{USAGE}");
                    return 64;
                };
                match value.parse::<u64>() {
                    Ok(secs) if secs > 0 => timeout_secs = secs,
                    _ => {
                        eprintln!("bad --timeout-secs: {value}");
                        return 64;
                    }
                }
                index += 2;
            }
            "--max-files" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("{USAGE}");
                    return 64;
                };
                match value.parse::<usize>() {
                    Ok(cap) if cap > 0 => max_files = Some(cap),
                    _ => {
                        eprintln!("bad --max-files: {value}");
                        return 64;
                    }
                }
                index += 2;
            }
            arg if arg.starts_with('-') => {
                eprintln!("unknown flag: {arg}\n{USAGE}");
                return 64;
            }
            arg => {
                entries.push(arg.to_string());
                index += 1;
            }
        }
    }
    if entries.is_empty() {
        eprintln!("{USAGE}");
        return 64;
    }

    let Some(exe) = std::env::current_exe().ok() else {
        eprintln!("sweep: cannot locate own executable (worker spawn impossible)");
        return 70;
    };

    // Collect the file set (deterministic: sorted, dot-entries out).
    let mut files: Vec<(Option<String>, PathBuf)> = Vec::new();
    for entry in &entries {
        let (class, path) = parse_entry(entry);
        match std::fs::metadata(&path) {
            Ok(meta) if meta.is_dir() => {
                let mut walked: Vec<PathBuf> = Vec::new();
                walk_dir(&path, &mut walked);
                for file in walked {
                    files.push((class.clone(), file));
                }
            }
            Ok(meta) if meta.is_file() => {
                files.push((class, path));
            }
            _ => {
                eprintln!("sweep: skipping missing entry {path:?}");
            }
        }
    }
    files.sort_by(|a, b| a.1.cmp(&b.1));
    let discovered = files.len();
    if let Some(cap) = max_files
        && files.len() > cap
    {
        files.truncate(cap);
    }

    // Sweep every file, one subprocess each.
    let mut tally = Tally::new();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for (class_override, path) in &files {
        let file_result = sweep_one(&exe, path, timeout_secs);
        let class = class_override
            .clone()
            .unwrap_or_else(|| class_of(path));
        tally.record(&class, file_result.outcome);
        let _ = writeln!(out, "{}", sweepline(&class, path, &file_result));
        let _ = out.flush();
    }

    // The summary block (also part of the raw log).
    let mut summary = tally.render();
    if let Some(cap) = max_files {
        summary.push_str(&format!(
            "\n(max-files cap {cap} of {discovered} discovered files applied)\n"
        ));
    }
    print!("{summary}");
    0
}

/// One file's supervised sweep result (what the SWEEPLINE needs).
struct FileResult {
    outcome: Outcome,
    /// The parsed worker marker line, when the worker got far
    /// enough to print one.
    line: Option<WorkerLine>,
    /// Panic/timeout/tool note ("-" when the marker carries all).
    note: String,
    /// Wall seconds for the file (supervisor-side, includes the
    /// process spawn — recorded honestly as such).
    secs: f64,
}

/// Spawn the worker for one file, enforce the deadline, classify.
fn sweep_one(exe: &Path, path: &Path, timeout_secs: u64) -> FileResult {
    let start = Instant::now();
    let spawned = Command::new(exe)
        .arg("--file")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let Ok(mut child) = spawned else {
        return FileResult {
            outcome: Outcome::Unknown,
            line: None,
            note: "spawn-failed".to_string(),
            secs: start.elapsed().as_secs_f64(),
        };
    };
    let (status, timed_out) = wait_with_timeout(&mut child, timeout_secs);
    // Read the pipes AFTER exit/kill: the worker writes at most one
    // small marker line to stdout (panics are small too), far below
    // the OS pipe buffer — a worker that somehow floods a pipe
    // blocks, misses its deadline, and is honestly counted a
    // timeout. (Deliberate simplicity over a reader-thread dance.)
    let mut stdout_text = String::new();
    let mut stderr_text = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = pipe.read_to_string(&mut stdout_text);
    }
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr_text);
    }
    let marker = stdout_text
        .lines()
        .find(|line| line.starts_with("SWEEP\t"))
        .and_then(parse_worker_line);
    let exit_code = status.as_ref().and_then(|status| status.code());
    let outcome = classify(marker.as_ref(), timed_out, exit_code);
    let note = if marker.is_some() {
        "-".to_string()
    } else if timed_out {
        format!("timeout-after-{timeout_secs}s")
    } else {
        crash_note(&stderr_text, exit_code)
    };
    FileResult {
        outcome,
        line: marker,
        note,
        secs: start.elapsed().as_secs_f64(),
    }
}

/// Wait for the child up to `timeout_secs`; on the deadline, kill
/// it and report the timeout. Polls `try_wait` so the deadline is
/// actually enforced (a blocking `wait` would hang with it).
fn wait_with_timeout(child: &mut Child, timeout_secs: u64) -> (Option<std::process::ExitStatus>, bool) {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            return (Some(status), false);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let status = child.wait().ok();
            return (status, true);
        }
        std::thread::sleep(Duration::from_millis(POLL_MS));
    }
}

/// Parse one positional argument: `CLASS=path` (class is
/// `[a-z0-9_-]+`) or a bare path (classified from the name).
fn parse_entry(arg: &str) -> (Option<String>, PathBuf) {
    if let Some((class, path)) = arg.split_once('=')
        && !class.is_empty()
        && !path.is_empty()
        && class
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_'))
    {
        return (Some(class.to_string()), PathBuf::from(path));
    }
    (None, PathBuf::from(arg))
}

/// Walk a directory recursively (sorted, dot-entries skipped) —
/// every file sweeps, extension-agnostic: garbage and no-extension
/// files are corpus classes too.
fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    for path in paths {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk_dir(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Render one `SWEEPLINE` (tab-separated; the per-file raw log
/// record). Fields: class, file, outcome, stage, kind, code,
/// v_pkts/v_dec, a_pkts/a_dec, v_conc/a_conc, recovered, tracks,
/// limbs, container, secs, detail-or-note.
fn sweepline(class: &str, path: &Path, result: &FileResult) -> String {
    let line = result.line.as_ref();
    let field = |pick: Option<&String>| -> String {
        match pick {
            Some(value) => value.clone(),
            None => "-".to_string(),
        }
    };
    format!(
        "SWEEPLINE\t{}\t{}\t{}\t{}\t{}\t{}\t{}/{}\t{}/{}\t{}/{}\t{}\t{}\t{}\t{}\t{:.2}\t{}",
        class,
        path.display(),
        result.outcome.as_str(),
        field(line.map(|line| &line.stage)),
        field(line.map(|line| &line.kind)),
        field(line.map(|line| &line.code)),
        line.map_or(0, |line| line.v_pkts),
        line.map_or(0, |line| line.v_dec),
        line.map_or(0, |line| line.a_pkts),
        line.map_or(0, |line| line.a_dec),
        line.map_or(0, |line| line.v_concealed),
        line.map_or(0, |line| line.a_concealed),
        line.map_or(0, |line| line.demux_recovered),
        line.map_or(0, |line| line.tracks),
        field(line.map(|line| &line.limbs)),
        field(line.map(|line| &line.container)),
        result.secs,
        line.map(|line| line.detail.clone()).unwrap_or_else(|| result.note.clone()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_entry_handles_overrides_and_bare_paths() {
        let (class, path) = parse_entry("avi-video=/tmp/x/0001-a.avi");
        assert_eq!(class.as_deref(), Some("avi-video"));
        assert_eq!(path, PathBuf::from("/tmp/x/0001-a.avi"));
        // A path containing '=' without a valid class label stays whole.
        let (class, path) = parse_entry("/tmp/we=ird.avi");
        assert!(class.is_none());
        assert_eq!(path, PathBuf::from("/tmp/we=ird.avi"));
        // Uppercase classes are not the label grammar → a path.
        let (class, _) = parse_entry("BAD=/tmp/x.avi");
        assert!(class.is_none());
    }

    #[test]
    fn sweepline_for_a_timeout_has_dashes_and_the_note() {
        let result = FileResult {
            outcome: Outcome::Timeout,
            line: None,
            note: "timeout-after-3s".to_string(),
            secs: 3.01,
        };
        let line = sweepline("truncated", Path::new("/tmp/x/0009-truncated.avi"), &result);
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields[0], "SWEEPLINE");
        assert_eq!(fields[3], "timeout");
        assert_eq!(fields[4], "-");
        assert_eq!(fields[15], "timeout-after-3s");
    }

    #[test]
    fn sweepline_for_an_ok_file_carries_the_worker_fields() {
        let line = WorkerLine {
            status: "ok".to_string(),
            stage: "-".to_string(),
            kind: "-".to_string(),
            code: "-".to_string(),
            v_pkts: 10,
            v_dec: 10,
            a_pkts: 0,
            a_dec: 0,
            v_concealed: 2,
            a_concealed: 0,
            demux_recovered: 1,
            tracks: 2,
            limbs: "v".to_string(),
            container: "avi".to_string(),
            detail: "-".to_string(),
        };
        let result = FileResult {
            outcome: Outcome::Ok,
            line: Some(line),
            note: "-".to_string(),
            secs: 0.25,
        };
        let rendered = sweepline("avi-video", Path::new("/tmp/x/0000-avi-video.avi"), &result);
        let fields: Vec<&str> = rendered.split('\t').collect();
        assert_eq!(fields[3], "ok");
        assert_eq!(fields[7], "10/10");
        assert_eq!(fields[9], "2/0");
        assert_eq!(fields[10], "1");
        assert_eq!(fields[13], "avi");
    }
}
