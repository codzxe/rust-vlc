//! The Lumen crate template (task 1.A1).
//!
//! Copy `crates/_template` to `crates/<crate-name>` for every new crate, then:
//!
//! 1. Set the `[package] name` in `Cargo.toml` and keep `[lints] workspace =
//!    true` — mandatory. The workspace lint policy (doc 10 §3) is inherited
//!    through it: no `unsafe`, no `unwrap`/`expect`/`panic!`/`todo!`/
//!    `unimplemented!`/`dbg!`, no dropped `#[must_use]` results, no `.await`
//!    across a held lock. Typed error propagation everywhere.
//! 2. Delete the template-only canary proof: `build.rs`,
//!    `src/lint_canary.rs`, and the `#[cfg(lint_canary)]` declaration below.
//! 3. Replace `template_marker()` and `tests/smoke.rs` with the crate's real
//!    responsibility, public interface and tests (MP §4: no empty scaffolds).
//!
//! The canary proves the lint-inheritance mechanism itself — see
//! `src/lint_canary.rs` for the exact reproducible proof commands.

#[cfg(lint_canary)]
pub mod lint_canary;

/// Template placeholder — replace with the crate's real public API.
pub fn template_marker() -> &'static str {
    "lumen-crate-template" // lumen-allow-literal: crate-template placeholder value — replaced when the template is instantiated
}
