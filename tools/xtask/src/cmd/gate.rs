//! `xtask gate` — the Lumen local quality gate (card 1.A3).
//!
//! Runs the doc 10 §2.1 command table **verbatim and in order**, captures
//! per-command pass/fail, and prints the honest gate report of doc 10 §2.4.
//! The xtask shells out to cargo/pnpm/wasm-pack instead of reimplementing
//! checks, so "what CI runs" and "what the gate runs" are literally the same
//! command strings (doc 10 §2.2/§10.4 gate-CI parity).
//!
//! # Command table (doc 10 §2.1, verbatim)
//!
//! | #  | Command                                                          | Gate |
//! |----|------------------------------------------------------------------|------|
//! | 1  | `cargo fmt --all -- --check`                                     | always |
//! | 2  | `cargo check --workspace --all-targets --all-features`           | always |
//! | 3  | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | always |
//! | 4  | `cargo test --workspace --all-features`                          | always |
//! | 5  | `cargo tree --workspace`                                         | always |
//! | 6  | `pnpm --dir ui lint`                                             | requires `ui/` |
//! | 7  | `pnpm --dir ui typecheck`                                        | requires `ui/` |
//! | 8  | `pnpm --dir ui test`                                             | requires `ui/` |
//! | 9  | `cargo tauri build --debug --no-bundle` (cwd `apps/desktop`)     | requires `apps/desktop/` |
//! | 10 | `wasm-pack build crates/<engine-wasm-binding> --target web --dev` | requires the binding crate |
//!
//! Steps whose target does not exist yet are reported `SKIP (<path> not
//! present)` — an honest "not run on this tree" line in the spirit of doc 10
//! §2.4's `NOT RUN` — and do not fail the gate. A step whose target exists
//! but whose tool fails (including spawn failures) is a `FAIL`.
//!
//! # Flags (doc 10 §2.2)
//!
//! * `--fast` — steps 1–4 + 6–8 only (pre-push smoke).
//! * `--no-ui` — engine-only run: skips 6–9 (UI + desktop shell).
//!   `[PROPOSAL]` interpretation: doc 10 §2.2 says only "engine-only".
//! * `--json` — additionally print the report JSON to stdout.
//! * `--keep-going` — run every step, collect all failures.
//!   Default is fail-fast (doc 10 §2.2); steps after a failure report
//!   `NOT RUN (fail-fast)`.
//! * `--dry-run` — evaluate the table and preconditions, execute nothing.
//!
//! # Report block (doc 10 §2.4 shape)
//!
//! ```text
//! === LUMEN GATE REPORT ===
//! commit:   <short-sha> (dirty: yes|no|unknown)
//! started:  <iso8601>   host: <os> <arch>, toolchain <rustc -V>
//! 1 fmt .................. PASS  12s
//! 3 clippy ............... FAIL  5m02s   <first error line, bounded>
//! 4 test ................. NOT RUN (fail-fast)
//! 6 ui lint .............. SKIP (ui/ not present)
//! result: FAILURE (1 of 10 failed, 5 skipped)
//! full log: target/gate-report.json
//! ```
//!
//! Deviation from the doc's sketch, kept honest and bounded: the inline text
//! after `FAIL` is the first `error`-prefixed output line (≤72 chars) when
//! one exists; full bounded tails (≤40 lines each of stdout/stderr) for
//! failed commands are printed under a `--- failure details ---` section and
//! stored in the JSON log. The doc's example hint (a `file:line` pointer) is
//! a special case of this.
//!
//! # JSON log (doc 10 §7.2 envelope, schema `evidence/1`)
//!
//! Always written to `<workspace-root>/target/gate-report.json`
//! `[PROPOSAL]` (the doc names `gate-report.json`; `target/` keeps the git
//! tree clean — xtask does not own `.gitignore`). Fields:
//! `schema` ("evidence/1"), `run_id` ("local-<unix-seconds>"), `commit`
//! (full sha|null), `dirty` (bool|null), `started_at`, `environment`
//! (`kind:"local"`, `os`, `arch`, `image:null`, `hardware_class:"mock"`
//! `[PROPOSAL]` — the gate produces build/test evidence, not device
//! evidence; Lead may flip this, `devices:[]`), `flags`, `commands` (one
//! entry per step: `step`, `name`, `cmd` [the verbatim argv joined with
//! spaces — the CI-parity export of §10.4], `cwd`, `status`
//! (`pass|fail|skip|not_run|dry`), `exit`, `duration_s`, `reason`, `junit`
//! [null until nextest wiring, doc 10 §10.2 `[PROPOSAL]`; test counts are
//! honestly available in the captured `stdout_tail`], `stdout_tail`,
//! `stderr_tail`), `result`, `labels`.
//!
//! # Exit codes
//!
//! `0` every executed step passed (SKIP/NOT RUN do not fail the gate);
//! `1` at least one step failed; `2` xtask error (no workspace root, report
//! unwritable, bad flags) — see `main.rs`.
//!
//! # Ownership
//!
//! This module is owned by card 1.A3 and defines the honest-report interface
//! every later card's C3 duty depends on. Changes need Lead-granted
//! ownership. The dispatch table in `main.rs`/`cmd/mod.rs` is frozen.
//!
//! # Known v0 limitations (honest register)
//!
//! * No junit-style count extraction (see `junit` above).
//! * `pnpm` on Windows is a `.cmd` shim; `Command::new("pnpm")` may not
//!   spawn it `[VERIFY]` when `ui/` lands — revisit with the first UI card.
//! * No per-step timeout (cargo's own behavior governs).
//! * Output is captured, so nothing streams while a long step runs.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::XtaskError;

/// Bounded number of trailing output lines kept per stream, per command.
const OUTPUT_TAIL_LINES: usize = 40;

/// The wasm binding crate of doc 10 §2.1 row 10. The doc leaves the crate
/// name `[PROPOSAL]` (binding layout per doc 05 §5.2 / doc 09); update this
/// single constant when that card lands.
const WASM_BINDING_CRATE: &str = "crates/engine-wasm-binding";

/// Where the JSON log is written, relative to the workspace root.
const REPORT_REL: &str = "target/gate-report.json";

const GATE_USAGE: &str = "\
Usage: cargo xtask gate [--fast] [--no-ui] [--json] [--keep-going] [--dry-run]

Runs the doc 10 \u{a7}2.1 local gate command table in order and prints the
honest gate report (doc 10 \u{a7}2.4). Also writes target/gate-report.json.

  --fast        steps 1-4 + 6-8 only (doc 10 \u{a7}2.2 pre-push smoke)
  --no-ui       engine-only run: skip ui lint/typecheck/test + tauri shell
                [PROPOSAL: interpretation of doc 10 \u{a7}2.2]
  --json        additionally print the gate report JSON to stdout
  --keep-going  run every step and collect all failures (default: fail-fast)
  --dry-run     evaluate the step table and preconditions, execute nothing

Exit code: 0 all executed steps passed | 1 a step failed | 2 xtask error.";

/// Entry point for `cargo xtask gate`.
pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{GATE_USAGE}");
        return Ok(ExitCode::SUCCESS);
    }
    let flags = parse_flags(args)?;
    let cwd =
        std::env::current_dir().map_err(|e| XtaskError::io("determining current directory", e))?;
    let root = find_workspace_root(&cwd)?;
    let env = gather_env(&root);
    let results = run_steps(DOC10_STEPS, &root, flags);
    let report = GateReport {
        root,
        flags,
        env,
        results,
    };
    let json = report.render_json();
    write_report(&report.root, &json)?;
    print!("{}", report.render_block());
    print!("{}", report.render_details());
    if flags.json {
        println!("{json}");
    }
    if report.has_failures() {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

// ---------------------------------------------------------------------------
// Step table (doc 10 §2.1)
// ---------------------------------------------------------------------------

/// One gate step. `argv` is the executable form; the displayed/recorded
/// command string is `argv.join(" ")`, so the report can only ever name the
/// command that actually ran (doc 10 §2.2 verbatim rule, §10.4 parity).
struct Step {
    /// Doc 10 §2.1 row number (1-based).
    idx: usize,
    /// Short name for the report block (`fmt`, `check`, ...).
    name: &'static str,
    /// Executable + arguments.
    argv: &'static [&'static str],
    /// Working directory relative to the workspace root ("" = root).
    cwd_rel: &'static str,
    /// Directory (relative to the root) that must exist for the step to be
    /// runnable; `None` = always runnable.
    requires: Option<&'static str>,
}

/// The doc 10 §2.1 command table, in order. Row 9 runs in `apps/desktop`
/// per the doc; row 10's crate path is [`WASM_BINDING_CRATE`].
const DOC10_STEPS: &[Step] = &[
    Step {
        idx: 1,
        name: "fmt",
        argv: &["cargo", "fmt", "--all", "--", "--check"],
        cwd_rel: "",
        requires: None,
    },
    Step {
        idx: 2,
        name: "check",
        argv: &[
            "cargo",
            "check",
            "--workspace",
            "--all-targets",
            "--all-features",
        ],
        cwd_rel: "",
        requires: None,
    },
    Step {
        idx: 3,
        name: "clippy",
        argv: &[
            "cargo",
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
        cwd_rel: "",
        requires: None,
    },
    Step {
        idx: 4,
        name: "test",
        argv: &["cargo", "test", "--workspace", "--all-features"],
        cwd_rel: "",
        requires: None,
    },
    Step {
        idx: 5,
        name: "tree",
        argv: &["cargo", "tree", "--workspace"],
        cwd_rel: "",
        requires: None,
    },
    Step {
        idx: 6,
        name: "ui lint",
        argv: &["pnpm", "--dir", "ui", "lint"],
        cwd_rel: "",
        requires: Some("ui/"),
    },
    Step {
        idx: 7,
        name: "ui typecheck",
        argv: &["pnpm", "--dir", "ui", "typecheck"],
        cwd_rel: "",
        requires: Some("ui/"),
    },
    Step {
        idx: 8,
        name: "ui test",
        argv: &["pnpm", "--dir", "ui", "test"],
        cwd_rel: "",
        requires: Some("ui/"),
    },
    Step {
        idx: 9,
        name: "tauri",
        argv: &["cargo", "tauri", "build", "--debug", "--no-bundle"],
        cwd_rel: "apps/desktop",
        requires: Some("apps/desktop/"),
    },
    Step {
        idx: 10,
        name: "wasm-pack",
        argv: &[
            "wasm-pack",
            "build",
            WASM_BINDING_CRATE,
            "--target",
            "web",
            "--dev",
        ],
        cwd_rel: "",
        requires: Some(WASM_BINDING_CRATE),
    },
];

/// Which flag excludes this step from the active table, if any.
/// `--no-ui` is checked first so overlapping steps (9) report the more
/// specific exclusion.
fn step_excluded(step: &Step, flags: GateFlags) -> Option<&'static str> {
    if flags.no_ui && matches!(step.idx, 6..=9) {
        return Some("excluded: --no-ui");
    }
    if flags.fast && matches!(step.idx, 5 | 9 | 10) {
        return Some("excluded: --fast");
    }
    None
}

// ---------------------------------------------------------------------------
// Flags
// ---------------------------------------------------------------------------

/// Gate invocation flags (doc 10 §2.2). Defaults: full table, fail-fast,
/// execute for real, summary block only.
#[derive(Default, Clone, Copy)]
struct GateFlags {
    fast: bool,
    no_ui: bool,
    json: bool,
    keep_going: bool,
    dry_run: bool,
}

fn parse_flags(args: &[String]) -> Result<GateFlags, XtaskError> {
    let mut flags = GateFlags::default();
    for arg in args {
        match arg.as_str() {
            "--fast" => flags.fast = true,
            "--no-ui" => flags.no_ui = true,
            "--json" => flags.json = true,
            "--keep-going" => flags.keep_going = true,
            "--dry-run" => flags.dry_run = true,
            other => {
                return Err(XtaskError::Usage(format!(
                    "unexpected argument `{other}` for gate (flags only)\n{GATE_USAGE}"
                )));
            }
        }
    }
    Ok(flags)
}

// ---------------------------------------------------------------------------
// Engine (injectable for tests — never run the real table in unit tests)
// ---------------------------------------------------------------------------

/// Outcome of one step.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Status {
    Pass,
    Fail,
    Skip,
    NotRun,
    Dry,
}

impl Status {
    fn json(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Fail => "fail",
            Status::Skip => "skip",
            Status::NotRun => "not_run",
            Status::Dry => "dry",
        }
    }
}

/// Recorded result of one step (report rows + JSON entries).
struct StepResult {
    idx: usize,
    name: &'static str,
    /// Verbatim command string (`argv.join(" ")`) — what actually runs.
    cmd: String,
    cwd_rel: &'static str,
    status: Status,
    exit: Option<i32>,
    duration_s: Option<f64>,
    reason: Option<String>,
    stdout_tail: Vec<String>,
    stderr_tail: Vec<String>,
}

impl StepResult {
    fn shell(step: &Step, status: Status, reason: Option<String>) -> Self {
        StepResult {
            idx: step.idx,
            name: step.name,
            cmd: step.argv.join(" "),
            cwd_rel: step.cwd_rel,
            status,
            exit: None,
            duration_s: None,
            reason,
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
        }
    }

    /// First `error`-prefixed captured line (stderr first, then stdout),
    /// bounded to 72 chars — the doc 10 §2.4 inline FAIL hint.
    fn error_hint(&self) -> Option<String> {
        let in_stderr = self
            .stderr_tail
            .iter()
            .find(|l| l.trim_start().starts_with("error"));
        let line = in_stderr
            .or_else(|| {
                self.stdout_tail
                    .iter()
                    .find(|l| l.trim_start().starts_with("error"))
            })
            .map(String::as_str)?;
        let trimmed = line.trim();
        let mut hint: String = trimmed.chars().take(72).collect();
        if trimmed.chars().count() > 72 {
            hint.push('\u{2026}');
        }
        Some(hint)
    }
}

/// Run a step table against `root` under `flags`, honoring fail-fast,
/// exclusions, preconditions and dry-run. Pure with respect to the table
/// argument, so tests inject cheap fake steps instead of the real gate.
fn run_steps(steps: &[Step], root: &Path, flags: GateFlags) -> Vec<StepResult> {
    let mut results = Vec::with_capacity(steps.len());
    let mut halted = false;
    for step in steps {
        if halted {
            results.push(StepResult::shell(
                step,
                Status::NotRun,
                Some("fail-fast".into()),
            ));
            continue;
        }
        if let Some(reason) = step_excluded(step, flags) {
            results.push(StepResult::shell(step, Status::Skip, Some(reason.into())));
            continue;
        }
        if let Some(req) = step.requires
            && !root.join(req).is_dir()
        {
            results.push(StepResult::shell(
                step,
                Status::Skip,
                Some(format!("{req} not present")),
            ));
            continue;
        }
        if flags.dry_run {
            results.push(StepResult::shell(step, Status::Dry, None));
            continue;
        }
        let result = execute_step(step, root);
        let failed = result.status == Status::Fail;
        results.push(result);
        if failed && !flags.keep_going {
            halted = true;
        }
    }
    results
}

/// Execute one step: spawn, wait, capture, bound. Spawn failures are FAILs
/// with the reason recorded (the target exists, the tool must work).
fn execute_step(step: &Step, root: &Path) -> StepResult {
    let cwd = if step.cwd_rel.is_empty() {
        root.to_path_buf()
    } else {
        root.join(step.cwd_rel)
    };
    let (program, args) = match step.argv.split_first() {
        Some((p, rest)) => (*p, rest),
        None => {
            return StepResult::shell(
                step,
                Status::Fail,
                Some("internal error: step has an empty argv".into()),
            );
        }
    };
    let started = Instant::now();
    let output = Command::new(program).args(args).current_dir(&cwd).output();
    let duration_s = Some(started.elapsed().as_secs_f64());
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let status = if out.status.success() {
                Status::Pass
            } else {
                Status::Fail
            };
            StepResult {
                status,
                exit: out.status.code(),
                duration_s,
                reason: None,
                stdout_tail: tail_lines(&stdout, OUTPUT_TAIL_LINES),
                stderr_tail: tail_lines(&stderr, OUTPUT_TAIL_LINES),
                ..StepResult::shell(step, status, None)
            }
        }
        Err(err) => StepResult {
            reason: Some(format!("failed to spawn `{program}`: {err}")),
            duration_s,
            ..StepResult::shell(step, Status::Fail, None)
        },
    }
}

// ---------------------------------------------------------------------------
// Environment + report
// ---------------------------------------------------------------------------

/// Machine-written run metadata for the report header and JSON envelope.
struct EnvInfo {
    started_at: String,
    run_id: String,
    commit: Option<String>,
    dirty: Option<bool>,
    toolchain: Option<String>,
    os: &'static str,
    arch: &'static str,
}

/// Collect run metadata. Read-only git/rustc queries; failures degrade to
/// `unknown`/null rather than failing the gate (the gate tests the tree,
/// not the presence of git).
fn gather_env(root: &Path) -> EnvInfo {
    let now = SystemTime::now();
    let commit = capture(root, "git", &["rev-parse", "HEAD"]);
    let dirty = match Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
    {
        Ok(out) if out.status.success() => {
            Some(!String::from_utf8_lossy(&out.stdout).trim().is_empty())
        }
        _ => None,
    };
    let toolchain = capture(root, "rustc", &["-V"]);
    EnvInfo {
        started_at: iso8601_utc(now),
        run_id: format!("local-{}", unix_seconds(now)),
        commit,
        dirty,
        toolchain,
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
    }
}

/// Run a read-only command and return its trimmed stdout on success.
fn capture(root: &Path, program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The full gate run: environment + per-step results.
struct GateReport {
    root: PathBuf,
    flags: GateFlags,
    env: EnvInfo,
    results: Vec<StepResult>,
}

impl GateReport {
    fn count(&self, status: Status) -> usize {
        self.results.iter().filter(|r| r.status == status).count()
    }

    fn has_failures(&self) -> bool {
        self.count(Status::Fail) > 0
    }

    /// `result: SUCCESS (0 of 10 failed, 5 skipped)` — doc 10 §2.4 shape,
    /// extended with skipped/not-run counts so partial runs read as partial.
    fn result_line(&self) -> String {
        let total = self.results.len();
        if self.flags.dry_run {
            let skipped = self.count(Status::Skip);
            return format!("result: DRY RUN (0 of {total} executed, {skipped} skipped)");
        }
        let failed = self.count(Status::Fail);
        let mut line = if failed == 0 {
            format!("result: SUCCESS (0 of {total} failed")
        } else {
            format!("result: FAILURE ({failed} of {total} failed")
        };
        let skipped = self.count(Status::Skip);
        if skipped > 0 {
            line.push_str(&format!(", {skipped} skipped"));
        }
        let not_run = self.count(Status::NotRun);
        if not_run > 0 {
            line.push_str(&format!(", {not_run} not run"));
        }
        line.push(')');
        line
    }

    /// The doc 10 §2.4 fenced report block.
    fn render_block(&self) -> String {
        let mut s = String::from("=== LUMEN GATE REPORT ===\n");
        let sha = match &self.env.commit {
            Some(c) => c.chars().take(7).collect::<String>(),
            None => "unknown".to_string(),
        };
        let dirty = match self.env.dirty {
            Some(true) => "yes",
            Some(false) => "no",
            None => "unknown",
        };
        let toolchain = self.env.toolchain.as_deref().unwrap_or("unknown");
        let started = &self.env.started_at;
        let (os, arch) = (self.env.os, self.env.arch);
        s.push_str(&format!(
            "commit:   {sha} (dirty: {dirty})\n\
             started:  {started}   host: {os} {arch}, toolchain {toolchain}\n"
        ));
        for r in &self.results {
            let dots_str = ".".repeat(22usize.saturating_sub(r.name.len() + 1).max(3));
            let status = match r.status {
                Status::Pass => {
                    let dur = format_duration(r.duration_s.unwrap_or(0.0));
                    format!("PASS  {dur}")
                }
                Status::Fail => {
                    let dur = format_duration(r.duration_s.unwrap_or(0.0));
                    match r.error_hint() {
                        Some(hint) => format!("FAIL  {dur}   {hint}"),
                        None => format!("FAIL  {dur}"),
                    }
                }
                Status::Skip => {
                    let reason = r.reason.as_deref().unwrap_or("skipped");
                    format!("SKIP ({reason})")
                }
                Status::NotRun => {
                    let reason = r.reason.as_deref().unwrap_or("not run");
                    format!("NOT RUN ({reason})")
                }
                Status::Dry => "DRY".to_string(),
            };
            let (idx, name) = (r.idx, r.name);
            s.push_str(&format!("{idx} {name} {dots_str} {status}\n"));
        }
        s.push_str(&self.result_line());
        s.push('\n');
        s.push_str(&format!("full log: {REPORT_REL}\n"));
        s
    }

    /// Bounded output tails for failed commands (console extension of the
    /// doc block; the full bounded tails live in the JSON log).
    fn render_details(&self) -> String {
        let mut s = String::new();
        let failures: Vec<&StepResult> = self
            .results
            .iter()
            .filter(|r| r.status == Status::Fail)
            .collect();
        if failures.is_empty() {
            return s;
        }
        s.push_str("--- failure details (last lines per failed command) ---\n");
        for r in failures {
            let exit = match r.exit {
                Some(code) => format!("exit {code}"),
                None => "no exit code (spawn failure)".to_string(),
            };
            let (idx, name, cmd) = (r.idx, r.name, &r.cmd);
            s.push_str(&format!("[{idx}] {name} — {cmd} — {exit}\n"));
            if let Some(reason) = &r.reason {
                s.push_str(&format!("  | {reason}\n"));
            }
            for line in &r.stderr_tail {
                s.push_str(&format!("  | {line}\n"));
            }
            if !r.stderr_tail.is_empty() && !r.stdout_tail.is_empty() {
                s.push_str("  | [stdout]\n");
            }
            for line in &r.stdout_tail {
                s.push_str(&format!("  | {line}\n"));
            }
        }
        s
    }

    /// The JSON log (schema and field docs in the module header).
    fn render_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str("  \"schema\": \"evidence/1\",\n");
        s.push_str(&format!("  \"run_id\": {},\n", jstr(&self.env.run_id)));
        match &self.env.commit {
            Some(c) => s.push_str(&format!("  \"commit\": {},\n", jstr(c))),
            None => s.push_str("  \"commit\": null,\n"),
        }
        match self.env.dirty {
            Some(d) => s.push_str(&format!("  \"dirty\": {d},\n")),
            None => s.push_str("  \"dirty\": null,\n"),
        }
        s.push_str(&format!(
            "  \"started_at\": {},\n",
            jstr(&self.env.started_at)
        ));
        s.push_str("  \"environment\": {\n");
        s.push_str("    \"kind\": \"local\",\n");
        s.push_str(&format!("    \"os\": {},\n", jstr(self.env.os)));
        s.push_str(&format!("    \"arch\": {},\n", jstr(self.env.arch)));
        s.push_str("    \"image\": null,\n");
        s.push_str("    \"hardware_class\": \"mock\",\n");
        s.push_str("    \"devices\": []\n");
        s.push_str("  },\n");
        s.push_str(&format!(
            "  \"flags\": {{\"fast\": {}, \"no_ui\": {}, \"keep_going\": {}, \"dry_run\": {}}},\n",
            self.flags.fast, self.flags.no_ui, self.flags.keep_going, self.flags.dry_run
        ));
        s.push_str("  \"commands\": [\n");
        for (i, r) in self.results.iter().enumerate() {
            s.push_str("    {\n");
            s.push_str(&format!("      \"step\": {},\n", r.idx));
            s.push_str(&format!("      \"name\": {},\n", jstr(r.name)));
            s.push_str(&format!("      \"cmd\": {},\n", jstr(&r.cmd)));
            let cwd = if r.cwd_rel.is_empty() { "." } else { r.cwd_rel };
            s.push_str(&format!("      \"cwd\": {},\n", jstr(cwd)));
            s.push_str(&format!("      \"status\": \"{}\",\n", r.status.json()));
            match r.exit {
                Some(code) => s.push_str(&format!("      \"exit\": {code},\n")),
                None => s.push_str("      \"exit\": null,\n"),
            }
            match r.duration_s {
                Some(d) => s.push_str(&format!("      \"duration_s\": {},\n", round3(d))),
                None => s.push_str("      \"duration_s\": null,\n"),
            }
            match &r.reason {
                Some(reason) => s.push_str(&format!("      \"reason\": {},\n", jstr(reason))),
                None => s.push_str("      \"reason\": null,\n"),
            }
            s.push_str("      \"junit\": null,\n");
            s.push_str(&format!(
                "      \"stdout_tail\": {},\n",
                jarray(&r.stdout_tail)
            ));
            s.push_str(&format!(
                "      \"stderr_tail\": {}\n",
                jarray(&r.stderr_tail)
            ));
            s.push_str("    }");
            if i + 1 < self.results.len() {
                s.push(',');
            }
            s.push('\n');
        }
        s.push_str("  ],\n");
        let result = if self.flags.dry_run {
            "DRY-RUN"
        } else if self.has_failures() {
            "FAILURE"
        } else {
            "SUCCESS"
        };
        s.push_str(&format!("  \"result\": \"{result}\",\n"));
        s.push_str("  \"labels\": [\"local\", \"mock\"]\n");
        s.push_str("}\n");
        s
    }
}

/// Write the JSON log under `<root>/target/`, creating the directory.
fn write_report(root: &Path, json: &str) -> Result<PathBuf, XtaskError> {
    let dir = root.join("target");
    std::fs::create_dir_all(&dir)
        .map_err(|e| XtaskError::io("creating target/ for the gate report", e))?;
    let path = root.join(REPORT_REL);
    std::fs::write(&path, json).map_err(|e| XtaskError::io(format!("writing {REPORT_REL}"), e))?;
    Ok(path)
}

// ---------------------------------------------------------------------------
// Workspace root discovery
// ---------------------------------------------------------------------------

/// Walk up from `start` to the nearest `Cargo.toml` declaring `[workspace]`.
fn find_workspace_root(start: &Path) -> Result<PathBuf, XtaskError> {
    let mut dir: Option<&Path> = Some(start);
    while let Some(d) = dir {
        let manifest = d.join("Cargo.toml");
        if manifest.is_file() && manifest_declares_workspace(&manifest) {
            return Ok(d.to_path_buf());
        }
        dir = d.parent();
    }
    Err(XtaskError::internal(format!(
        "no Cargo.toml with [workspace] found in {} or any parent — \
         run `cargo xtask gate` from inside the Lumen workspace",
        start.display()
    )))
}

/// True if the manifest has a `[workspace]` section (line-start match;
/// `[workspace.lints]` in the ROOT manifest coexists with `[workspace]`,
/// while member manifests only carry `[lints] workspace = true`).
fn manifest_declares_workspace(path: &Path) -> bool {
    match std::fs::read_to_string(path) {
        Ok(contents) => contents
            .lines()
            .any(|line| line.trim_start().starts_with("[workspace]")),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// std-only helpers (no external dependencies — C5)
// ---------------------------------------------------------------------------

/// Days since 1970-01-01 → (year, month, day), proleptic Gregorian.
/// Howard Hinnant's `civil_from_days`, safe for any signed day count.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Whole (possibly negative) seconds since the Unix epoch.
fn unix_seconds(t: SystemTime) -> i64 {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
        Err(e) => -i64::try_from(e.duration().as_secs()).unwrap_or(0),
    }
}

/// ISO-8601 UTC timestamp (`2026-01-01T00:00:00Z`) — std-only.
fn iso8601_utc(t: SystemTime) -> String {
    let secs = unix_seconds(t);
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400); // [0, 86399]
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}

/// Duration for the report block: `12s`, `3m41s`, `5m02s` (doc 10 §2.4).
/// Whole seconds, zero-padded inside minutes; sub-second rounds to `0s`.
fn format_duration(secs: f64) -> String {
    let total = if secs.is_finite() && secs > 0.0 {
        secs.round() as u64
    } else {
        0
    };
    if total >= 60 {
        format!("{}m{:02}s", total / 60, total % 60)
    } else {
        format!("{total}s")
    }
}

/// Keep the last `limit` lines, dropping trailing then leading blank lines,
/// so the tail is "meaningful output" (bounded evidence, not padding).
fn tail_lines(output: &str, limit: usize) -> Vec<String> {
    let mut lines: Vec<&str> = output.lines().collect();
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    let start = lines.len().saturating_sub(limit);
    let mut kept: Vec<String> = lines[start..].iter().map(|l| l.to_string()).collect();
    while kept.first().is_some_and(String::is_empty) {
        kept.remove(0);
    }
    kept
}

/// Escape a string for JSON (RFC 8259): quotes, backslash, \b \f \n \r \t,
/// other control chars as \u00XX. Non-ASCII passes through (UTF-8 output).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// JSON string literal (quoted + escaped).
fn jstr(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

/// JSON array of string literals.
fn jarray(lines: &[String]) -> String {
    let items: Vec<String> = lines.iter().map(|l| jstr(l)).collect();
    format!("[{}]", items.join(", "))
}

/// Round to milliseconds for stable JSON numbers.
fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

// ---------------------------------------------------------------------------
// Tests (C2: unit tests in owned code, incl. error paths; the REAL gate
// table is never executed here — too slow — only injected cheap steps)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Cheap injected steps: pass / real nonzero exit / pass / spawn
    /// failure / unmet precondition. `cargo` exists wherever tests run.
    fn test_steps() -> Vec<Step> {
        vec![
            Step {
                idx: 1,
                name: "pass",
                argv: &["cargo", "--version"],
                cwd_rel: "",
                requires: None,
            },
            Step {
                idx: 2,
                name: "fail",
                argv: &["cargo", "--unknown-flag-a3"],
                cwd_rel: "",
                requires: None,
            },
            Step {
                idx: 3,
                name: "after",
                argv: &["cargo", "--version"],
                cwd_rel: "",
                requires: None,
            },
            Step {
                idx: 4,
                name: "spawnfail",
                argv: &["no-such-binary-a3", "--x"],
                cwd_rel: "",
                requires: None,
            },
            Step {
                idx: 5,
                name: "skipme",
                argv: &["cargo", "--version"],
                cwd_rel: "",
                requires: Some("no-such-dir-a3/"),
            },
        ]
    }

    fn test_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    // -- doc 10 §2.1 table ------------------------------------------------

    #[test]
    fn doc10_table_has_ten_ordered_steps() {
        assert_eq!(DOC10_STEPS.len(), 10);
        for (i, step) in DOC10_STEPS.iter().enumerate() {
            assert_eq!(step.idx, i + 1, "step numbering must match doc 10 §2.1");
            assert!(!step.name.is_empty());
            assert!(!step.argv.is_empty());
        }
    }

    #[test]
    fn doc10_commands_are_verbatim() {
        let cmds: Vec<String> = DOC10_STEPS.iter().map(|s| s.argv.join(" ")).collect();
        assert_eq!(cmds[0], "cargo fmt --all -- --check");
        assert_eq!(
            cmds[1],
            "cargo check --workspace --all-targets --all-features"
        );
        assert_eq!(
            cmds[2],
            "cargo clippy --workspace --all-targets --all-features -- -D warnings"
        );
        assert_eq!(cmds[3], "cargo test --workspace --all-features");
        assert_eq!(cmds[4], "cargo tree --workspace");
        assert_eq!(cmds[5], "pnpm --dir ui lint");
        assert_eq!(cmds[6], "pnpm --dir ui typecheck");
        assert_eq!(cmds[7], "pnpm --dir ui test");
        assert_eq!(cmds[8], "cargo tauri build --debug --no-bundle");
        assert_eq!(
            cmds[9],
            "wasm-pack build crates/engine-wasm-binding --target web --dev"
        );
    }

    #[test]
    fn doc10_preconditions_and_cwd() {
        assert_eq!(DOC10_STEPS[5].requires, Some("ui/"));
        assert_eq!(DOC10_STEPS[6].requires, Some("ui/"));
        assert_eq!(DOC10_STEPS[7].requires, Some("ui/"));
        assert_eq!(DOC10_STEPS[8].requires, Some("apps/desktop/"));
        assert_eq!(DOC10_STEPS[8].cwd_rel, "apps/desktop");
        // Row 10's command must embed its precondition path (kept in sync
        // with WASM_BINDING_CRATE by this test).
        let wasm = &DOC10_STEPS[9];
        assert_eq!(wasm.requires, Some(WASM_BINDING_CRATE));
        assert!(wasm.argv.contains(&WASM_BINDING_CRATE));
    }

    #[test]
    fn flag_exclusions_match_doc10() {
        let fast = GateFlags {
            fast: true,
            ..GateFlags::default()
        };
        for step in DOC10_STEPS {
            match step.idx {
                5 | 9 | 10 => {
                    assert_eq!(step_excluded(step, fast), Some("excluded: --fast"));
                }
                _ => assert_eq!(step_excluded(step, fast), None),
            }
        }
        let no_ui = GateFlags {
            no_ui: true,
            ..GateFlags::default()
        };
        for step in DOC10_STEPS {
            match step.idx {
                6..=9 => assert_eq!(step_excluded(step, no_ui), Some("excluded: --no-ui")),
                _ => assert_eq!(step_excluded(step, no_ui), None),
            }
        }
    }

    // -- engine semantics ---------------------------------------------------

    #[test]
    fn fail_fast_marks_remaining_steps_not_run() {
        let results = run_steps(&test_steps(), &test_root(), GateFlags::default());
        let statuses: Vec<Status> = results.iter().map(|r| r.status).collect();
        assert_eq!(
            statuses,
            vec![
                Status::Pass,
                Status::Fail,
                Status::NotRun,
                Status::NotRun,
                Status::NotRun
            ]
        );
        assert_eq!(results[2].reason.as_deref(), Some("fail-fast"));
    }

    #[test]
    fn keep_going_collects_every_failure() {
        let flags = GateFlags {
            keep_going: true,
            ..GateFlags::default()
        };
        let results = run_steps(&test_steps(), &test_root(), flags);
        let statuses: Vec<Status> = results.iter().map(|r| r.status).collect();
        assert_eq!(
            statuses,
            vec![
                Status::Pass,
                Status::Fail,
                Status::Pass,
                Status::Fail,
                Status::Skip
            ]
        );
        // The real nonzero exit is captured (exact code is cargo's choice)...
        assert!(results[1].exit.is_some_and(|c| c != 0));
        // ...the spawn failure has no exit code but a recorded reason...
        assert_eq!(results[3].exit, None);
        assert!(
            results[3]
                .reason
                .as_deref()
                .is_some_and(|r| r.contains("failed to spawn")),
            "spawn failure reason missing: {:?}",
            results[3].reason
        );
        // ...and the unmet precondition says what is missing.
        assert_eq!(
            results[4].reason.as_deref(),
            Some("no-such-dir-a3/ not present")
        );
    }

    #[test]
    fn dry_run_executes_nothing_but_reports_preconditions() {
        let flags = GateFlags {
            dry_run: true,
            ..GateFlags::default()
        };
        let results = run_steps(&test_steps(), &test_root(), flags);
        let statuses: Vec<Status> = results.iter().map(|r| r.status).collect();
        assert_eq!(
            statuses,
            vec![
                Status::Dry,
                Status::Dry,
                Status::Dry,
                Status::Dry,
                Status::Skip
            ]
        );
        for r in &results {
            assert!(r.exit.is_none());
            assert!(r.duration_s.is_none());
        }
    }

    #[test]
    fn pass_captures_exit_code_and_bounded_output() {
        let results = run_steps(&test_steps()[0..1], &test_root(), GateFlags::default());
        assert_eq!(results[0].status, Status::Pass);
        assert_eq!(results[0].exit, Some(0));
        assert!(results[0].duration_s.is_some_and(|d| d >= 0.0));
        assert!(
            results[0]
                .stdout_tail
                .first()
                .is_some_and(|l| l.starts_with("cargo")),
            "cargo --version output not captured"
        );
    }

    // -- report rendering ---------------------------------------------------

    fn synthetic_report() -> GateReport {
        let results = vec![
            StepResult {
                status: Status::Pass,
                exit: Some(0),
                duration_s: Some(12.0),
                reason: None,
                stdout_tail: Vec::new(),
                stderr_tail: Vec::new(),
                ..StepResult::shell(&DOC10_STEPS[0], Status::Pass, None)
            },
            StepResult {
                status: Status::Pass,
                exit: Some(0),
                duration_s: Some(221.0),
                reason: None,
                stdout_tail: Vec::new(),
                stderr_tail: Vec::new(),
                ..StepResult::shell(&DOC10_STEPS[1], Status::Pass, None)
            },
            StepResult {
                status: Status::Fail,
                exit: Some(101),
                duration_s: Some(302.0),
                reason: None,
                stdout_tail: vec!["Finished `dev` profile".to_string()],
                stderr_tail: vec![
                    "warning: something minor".to_string(),
                    "error: synthetic failure".to_string(),
                    String::new(),
                ],
                ..StepResult::shell(&DOC10_STEPS[2], Status::Fail, None)
            },
            StepResult::shell(&DOC10_STEPS[3], Status::NotRun, Some("fail-fast".into())),
            StepResult::shell(&DOC10_STEPS[4], Status::Skip, Some("synthetic skip".into())),
        ];
        GateReport {
            root: test_root(),
            flags: GateFlags::default(),
            env: EnvInfo {
                started_at: "2026-01-01T00:00:00Z".to_string(),
                run_id: "local-0".to_string(),
                commit: Some("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string()),
                dirty: Some(true),
                toolchain: Some("rustc 1.99.1 (synthetic)".to_string()),
                os: "linux",
                arch: "x86_64",
            },
            results,
        }
    }

    #[test]
    fn block_matches_doc10_shape() {
        let report = synthetic_report();
        let expected = "\
=== LUMEN GATE REPORT ===
commit:   deadbee (dirty: yes)
started:  2026-01-01T00:00:00Z   host: linux x86_64, toolchain rustc 1.99.1 (synthetic)
1 fmt .................. PASS  12s
2 check ................ PASS  3m41s
3 clippy ............... FAIL  5m02s   error: synthetic failure
4 test ................. NOT RUN (fail-fast)
5 tree ................. SKIP (synthetic skip)
result: FAILURE (1 of 5 failed, 1 skipped, 1 not run)
full log: target/gate-report.json
";
        assert_eq!(report.render_block(), expected);
    }

    #[test]
    fn success_result_line_omits_zero_counts() {
        let mut report = synthetic_report();
        report.results.truncate(2); // two passes only
        assert_eq!(report.result_line(), "result: SUCCESS (0 of 2 failed)");
    }

    #[test]
    fn dry_run_result_line() {
        let mut report = synthetic_report();
        report.flags.dry_run = true;
        assert_eq!(
            report.result_line(),
            "result: DRY RUN (0 of 5 executed, 1 skipped)"
        );
    }

    #[test]
    fn details_section_lists_failures_with_tails() {
        let details = synthetic_report().render_details();
        assert!(
            details.starts_with("--- failure details (last lines per failed command) ---"),
            "unexpected details header:\n{details}"
        );
        assert!(details.contains("[3] clippy — cargo clippy"));
        assert!(details.contains("exit 101"));
        assert!(details.contains("  | error: synthetic failure"));
        assert!(details.contains("  | [stdout]"));
    }

    #[test]
    fn details_section_empty_without_failures() {
        let mut report = synthetic_report();
        report.results.truncate(2);
        assert_eq!(report.render_details(), "");
    }

    #[test]
    fn json_report_has_stable_schema_and_escapes_strings() {
        let mut report = synthetic_report();
        report.results[4].reason = Some("missing \"ui\" \u{2192} skipped".into());
        let json = report.render_json();
        assert!(json.starts_with("{\n"));
        assert!(json.ends_with("}\n"));
        assert!(json.contains("\"schema\": \"evidence/1\","));
        assert!(json.contains("\"run_id\": \"local-0\","));
        assert!(json.contains("\"commit\": \"deadbeefdeadbeefdeadbeefdeadbeefdeadbeef\","));
        assert!(json.contains("\"dirty\": true,"));
        assert!(json.contains("\"started_at\": \"2026-01-01T00:00:00Z\","));
        assert!(json.contains("\"kind\": \"local\","));
        assert!(json.contains("\"hardware_class\": \"mock\","));
        assert!(json.contains("\"step\": 3,"));
        assert!(json.contains(
            "\"cmd\": \"cargo clippy --workspace --all-targets --all-features -- -D warnings\","
        ));
        assert!(json.contains("\"status\": \"fail\","));
        assert!(json.contains("\"exit\": 101,"));
        assert!(json.contains("\"duration_s\": 302,"));
        assert!(json.contains("\"junit\": null,"));
        // string escaping: quotes and non-ASCII arrow
        assert!(json.contains("\"reason\": \"missing \\\"ui\\\" \u{2192} skipped\""));
        assert!(json.contains("\"result\": \"FAILURE\","));
        assert!(json.contains("\"labels\": [\"local\", \"mock\"]"));
    }

    #[test]
    fn json_result_dry_run() {
        let mut report = synthetic_report();
        report.flags.dry_run = true;
        assert!(report.render_json().contains("\"result\": \"DRY-RUN\","));
    }

    // -- workspace root discovery -------------------------------------------

    #[test]
    fn finds_repo_root_from_crate_dir() {
        let found = find_workspace_root(&test_root());
        assert!(found.is_ok(), "workspace root not found from crate dir");
        if let Ok(root) = found {
            assert!(root.join("crates").is_dir());
            assert!(root.join("tools").is_dir());
            assert!(manifest_declares_workspace(&root.join("Cargo.toml")));
        }
    }

    #[test]
    fn no_workspace_root_above_temp_dir() {
        let dir = std::env::temp_dir().join("xtask-a3-no-workspace-probe");
        if std::fs::create_dir_all(&dir).is_ok() {
            assert!(find_workspace_root(&dir).is_err());
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    // -- helpers --------------------------------------------------------------

    #[test]
    fn iso8601_known_instants() {
        let at = |secs: i64| UNIX_EPOCH + Duration::from_secs(secs.unsigned_abs());
        assert_eq!(iso8601_utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601_utc(at(951_782_400)), "2000-02-29T00:00:00Z");
        assert_eq!(iso8601_utc(at(1_791_849_600)), "2026-10-13T00:00:00Z");
        assert_eq!(iso8601_utc(at(1_771_076_709)), "2026-02-14T13:45:09Z");
        // pre-epoch clock
        assert_eq!(
            iso8601_utc(UNIX_EPOCH - Duration::from_secs(1)),
            "1969-12-31T23:59:59Z"
        );
    }

    #[test]
    fn duration_formatting_matches_doc10_examples() {
        assert_eq!(format_duration(12.0), "12s");
        assert_eq!(format_duration(221.0), "3m41s");
        assert_eq!(format_duration(302.4), "5m02s");
        assert_eq!(format_duration(60.0), "1m00s");
        assert_eq!(format_duration(3659.0), "60m59s");
        assert_eq!(format_duration(0.4), "0s");
    }

    #[test]
    fn tail_lines_bounds_and_trims_blanks() {
        let body = vec!["a"; 50].join("\n");
        let output = format!("\n\n\n\n\n{body}\n\n\n");
        let tail = tail_lines(&output, 40);
        assert_eq!(tail.len(), 40);
        assert_eq!(tail[0], "a");
        assert_eq!(tail[39], "a");
        assert!(tail_lines("", 40).is_empty());
        assert_eq!(
            tail_lines("x\ny\n\n\n", 40),
            vec!["x".to_string(), "y".to_string()]
        );
    }

    #[test]
    fn json_escape_specials_and_passthrough() {
        assert_eq!(
            json_escape("quote:\" slash:\\ newline:\n tab:\t"),
            "quote:\\\" slash:\\\\ newline:\\n tab:\\t"
        );
        assert_eq!(json_escape("\u{01}"), "\\u0001");
        assert_eq!(json_escape("h\u{e9}llo\u{2192}"), "h\u{e9}llo\u{2192}");
        assert_eq!(jstr("a"), "\"a\"");
        assert_eq!(jarray(&["x".into(), "y".into()]), "[\"x\", \"y\"]");
        assert_eq!(jarray(&[]), "[]");
    }

    #[test]
    fn round3_millis() {
        assert_eq!(round3(302.25), 302.25);
        assert_eq!(round3(0.0), 0.0);
        assert!((round3(302.123_456) - 302.123).abs() < 1e-9);
    }

    #[test]
    fn hint_is_first_error_line_bounded() {
        let mut r = StepResult::shell(&DOC10_STEPS[2], Status::Fail, None);
        r.stderr_tail = vec!["warning: w".into(), "error: the real problem".into()];
        assert_eq!(r.error_hint().as_deref(), Some("error: the real problem"));
        let long: String = "error: ".to_string() + &"x".repeat(100);
        r.stderr_tail = vec![long.clone()];
        assert_eq!(
            r.error_hint().as_deref(),
            Some((long.chars().take(72).collect::<String>() + "\u{2026}").as_str())
        );
        r.stderr_tail = vec!["no errors here".into()];
        r.stdout_tail = vec!["error: from stdout".into()];
        assert_eq!(r.error_hint().as_deref(), Some("error: from stdout"));
        r.stdout_tail = Vec::new();
        assert_eq!(r.error_hint(), None);
    }
}
