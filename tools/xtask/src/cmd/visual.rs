//! `xtask visual` — screenshot-diff fidelity harness driver (card 4.G1,
//! doc 08 §6.3). The harness itself lives in `ui/e2e/visual/` +
//! `ui/scripts/visual-run.mjs` (Playwright Chromium, zero new npm deps);
//! this subcommand is a thin, honest shell-out with verbatim exit-code
//! propagation, mirroring the `cmd::tokens` precedent of forwarding a
//! child tool's status.
//!
//! ```text
//! Usage: cargo xtask visual [--mode=pr|nightly] [--update] [--seed]
//!
//!   --mode=pr       PR-lite lane (default): 1280×720 @1×, dark + light — the
//!                   fast, strict per-PR gate (doc 08 §9.3)
//!   --mode=nightly  full matrix: {1280×720, 1920×1080} × DPI {1,2} × {dark,light}
//!   --update        re-record baselines (doc 08 §6.5 policy: only in a commit
//!                   referencing a token/layout/design change)
//!   --seed          SEED_2PX=1 — inject the 2-px regression proof shift; the
//!                   harness must FAIL (acceptance proof, never in CI)
//!
//! Exit code: forwarded verbatim from `npm run visual` (0 pass | 1 visual
//! diff/failure | 2 hard error). 2 also covers xtask-side failures.
//! ```
//!
//! Registered under card 4.G1's granted ownership of the frozen registry
//! (`cmd/mod.rs`), same precedent as `cmd::check_panics` (card 7-e).

use std::path::PathBuf;
use std::process::{Command, ExitCode};

use crate::XtaskError;

/// The `ui/` package dir, resolved from this crate's location
/// (`tools/xtask` → workspace root → `ui`). Missing `ui/package.json` is a
/// hard error, never a silent skip.
fn ui_dir() -> Result<PathBuf, XtaskError> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(PathBuf::from)
        .ok_or_else(|| XtaskError::internal("cannot resolve workspace root from xtask dir"))?;
    let ui = root.join("ui");
    if !ui.join("package.json").is_file() {
        return Err(XtaskError::internal(format!(
            "{} is not the ui package (package.json missing) — run from the workspace root",
            ui.display()
        )));
    }
    Ok(ui)
}

/// `npm` resolves as `npm.cmd` on Windows (std does not append PATHEXT
/// extensions for `Command::new`; the gate.rs §Windows note applied to npm).
fn npm_program() -> &'static str {
    if cfg!(windows) { "npm.cmd" } else { "npm" }
}

pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{}", include_str!("visual_help.txt"));
        return Ok(ExitCode::SUCCESS);
    }

    let ui = ui_dir()?;
    println!("[xtask visual] cwd={} args={:?}", ui.display(), args);

    // Arguments are forwarded verbatim; scripts/visual-run.mjs owns
    // validation (unknown args exit 2 with usage).
    let status = Command::new(npm_program())
        .arg("run")
        .arg("visual")
        .arg("--")
        .args(args)
        .current_dir(&ui)
        .status()
        .map_err(|source| XtaskError::io("spawning `npm run visual` in ui/", source))?;

    match status.code() {
        Some(0) => Ok(ExitCode::SUCCESS),
        Some(code @ 1..=255) => Ok(ExitCode::from(code as u8)),
        Some(_) => Ok(ExitCode::from(2)),
        // Signal-terminated (e.g. Ctrl-C): report failure.
        None => Ok(ExitCode::FAILURE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_dir_resolves_to_the_workspace_ui_package() {
        let ui = match ui_dir() {
            Ok(ui) => ui,
            Err(err) => {
                let resolved = false;
                assert!(resolved, "ui dir must resolve inside this repo: {err}");
                return;
            }
        };
        assert!(
            ui.join("package.json").is_file(),
            "ui/package.json missing at {}",
            ui.display()
        );
        assert!(
            ui.join("playwright.visual.config.ts").is_file(),
            "visual config missing — harness not landed?"
        );
    }

    #[test]
    fn npm_program_uses_the_cmd_shim_on_windows() {
        assert_eq!(npm_program(), if cfg!(windows) { "npm.cmd" } else { "npm" });
    }
}
