//! `xtask check-identity` — the C2 no-hardcoded-identity gate (doc 07
//! §9, card 7.w21a / issue #61 "check-identity").
//!
//! The reverse assertion of the rebrand system (doc 07 §8.3 step 6):
//! in the un-rebranded tree, no engine/app crate hard-codes an
//! identity string where `product-config` should be used. This is why
//! `cargo xtask rebrand` CAN work — every identity-bearing surface
//! either reads `product.toml` (the config layer), is generated from
//! it, or carries an audited allowlist entry.
//!
//! # What it checks
//!
//! Everything `lint-literals` checks for the identity kind, over the
//! same scan domain (`crates/*/src/**`, `apps/*/src/**`), with the
//! identity set broadened from the brand trio to the full doc 07 §9
//! C2 row — "identity literals (`name`, `short_name`, ids, accent
//! hex)":
//!
//! * `identity.name`, `identity.short_name`, `identity.publisher` —
//!   case-insensitive whole-word matches (the brand strings; lowercase
//!   included, mirroring the rebrand token table);
//! * `identity.ids.*` (reverse-DNS root, per-OS ids, the MSI upgrade
//!   GUID) — exact case-sensitive substrings;
//! * `brand.accent_color` — case-insensitive substring.
//!
//! `product.toml` itself, the generated artifacts (`schemas/generated/`)
//! and everything outside the scan domain are outside this check by
//! construction. The allowlist policy is `lint_literals`' verbatim
//! (config-layer crates, tests/fixtures dirs, comments, loopback-URL
//! test exception, the inline `// lumen-allow-literal: <reason>`
//! marker — printed, never silent); see that module's docs for the
//! rationale, including why the rebrand allowlist's crate-wide
//! `crates/` entry is NOT consumed here.
//!
//! A missing or malformed `product.toml` (or a missing identity field)
//! is exit 2 — an xtask error, not a clean pass: the check's whole
//! premise is the config file it reads.
//!
//! Usage: `cargo xtask check-identity [--root <dir>]`
//!
//! Exit code: 0 no violations | 1 violations found | 2 xtask error.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::XtaskError;

use super::lint_literals::{self, Identity, Scope};

/// The plain-text usage (mirrors the module docs).
const USAGE: &str = "\
Usage: cargo xtask check-identity [--root <dir>]

  Verifies identity propagation (doc 07 §9 C2): loads product.toml, then
  scans string literals in crates/*/src/** and apps/*/src/** for the
  identity strings (name, short_name, publisher, the per-OS ids, the
  MSI GUID, the accent hex) that must come from crates/product-config
  instead. Same allowlist as lint-literals (config layer, tests//fixtures/
  dirs, comments, inline `// lumen-allow-literal: <reason>` markers).

  --root <dir>  check the workspace rooted at <dir> instead of walking up
                from the current directory (fixture-test seam)

Exit code: 0 no violations | 1 violations found | 2 xtask error.";

/// Entry point for `cargo xtask check-identity`.
pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    let mut root_override: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(ExitCode::SUCCESS);
            }
            "--root" => {
                i += 1;
                let Some(dir) = args.get(i) else {
                    return Err(XtaskError::Usage(format!(
                        "--root requires a directory argument\n{USAGE}"
                    )));
                };
                root_override = Some(PathBuf::from(dir));
            }
            other => {
                return Err(XtaskError::Usage(format!(
                    "unexpected argument `{other}`\n{USAGE}"
                )));
            }
        }
        i += 1;
    }
    let root = match root_override {
        Some(dir) => dir,
        None => {
            let cwd = std::env::current_dir()
                .map_err(|e| XtaskError::io("determining current directory", e))?;
            lint_literals::find_root(&cwd)?
        }
    };
    let root = root
        .canonicalize()
        .map_err(|e| XtaskError::io(format!("resolving root {}", root.display()), e))?;

    // The C2 premise: the identity comes from product.toml. A missing
    // field is exit 2 (Identity::load reports every missing key).
    let identity = Identity::load(&root)?;
    let outcome = lint_literals::scan_workspace(&root, &identity, Scope::C2)?;
    outcome.print(&root, "check-identity");
    if outcome.violations.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}
