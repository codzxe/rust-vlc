//! `xtask schema-gen` — settings-schema generators (doc 07 §5).
//!
//! Owned by card **1.B2**. Registered by card 1.A3 (extension-first
//! dispatch).
//!
//! Subprocess bridge (the Lead-ratified [PROPOSAL] pattern first recorded
//! by card 1.B4 / theme-tokens): `tools/xtask` cannot take a path
//! dependency on `crates/settings-schema` without editing its frozen
//! manifest, so this subcommand shells out to the crate's `schema-gen`
//! binary, which reads `schemas/settings.toml` (+ `product.toml` for the
//! rule-D1 join), validates the D1–D6 no-duplicate rules and (re)generates
//! the three committed outputs:
//!
//! * `schemas/generated/settings.d.ts`     — G1, TS type surface
//! * `schemas/generated/settings-help.txt` — G2, CLI `--help` golden
//! * `schemas/generated/settings-docs.md`  — G3, settings reference
//!
//! Extra args are forwarded verbatim (e.g. `cargo xtask schema-gen
//! --check` for the CI drift mode; the generator's exit codes are 0 ok /
//! 1 drift / 2 error). Inherited stdio keeps the generator's actionable
//! diagnostics visible; its exit status is forwarded as-is (a failed
//! regeneration must fail the gate).

use std::path::Path;
use std::process::Command;
use std::process::ExitCode;

use crate::XtaskError;

/// Workspace root, derived from this crate's manifest location
/// (`<root>/tools/xtask`). Compile-time constant: the xtask binary only
/// ever runs from inside the workspace it was built by.
const WORKSPACE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// The `cargo` driver that invoked us, when available (`cargo xtask`),
/// falling back to a plain `cargo` lookup on PATH.
fn cargo_driver() -> String {
    match std::env::var("CARGO") {
        Ok(path) => path,
        Err(_) => String::from("cargo"),
    }
}

/// Workspace root at runtime: `<root>/tools/xtask` minus two components.
fn workspace_root() -> &'static Path {
    Path::new(WORKSPACE_ROOT)
        .parent()
        .and_then(Path::parent)
        .unwrap_or(Path::new(WORKSPACE_ROOT))
}

/// Run the `schema-gen` binary through cargo and forward its status.
///
/// `--quiet` suppresses cargo's own compilation noise so the only output
/// is the generator's report; compilation errors still surface verbatim.
pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    let program = cargo_driver();
    let mut command = Command::new(&program);
    command
        .args([
            "run",
            "--quiet",
            "-p",
            "settings-schema",
            "--bin",
            "schema-gen",
            "--",
        ])
        .args(args)
        .current_dir(workspace_root());
    let mut child = command.spawn().map_err(|source| XtaskError::Io {
        context: format!("spawning `{program} run -p settings-schema --bin schema-gen`"),
        source,
    })?;

    let status = child.wait().map_err(|source| XtaskError::Io {
        context: String::from("waiting for `schema-gen` to finish"),
        source,
    })?;

    match status.code() {
        Some(code) => Ok(ExitCode::from(u8::try_from(code).unwrap_or(2))),
        // Terminated by a signal: report failure rather than success.
        None => Ok(ExitCode::from(2)),
    }
}
