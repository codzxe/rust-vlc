//! Lint-inheritance canary (task 1.A1) — the ONLY sanctioned lint violation
//! in this repository.
//!
//! This module is compiled ONLY when `--cfg lint_canary` is passed to rustc
//! (the cfg name is registered by this crate's `build.rs`; it is never set by
//! `cargo build`/`check`/`test`/`clippy`, so the doc 10 §2 gate never sees
//! it). It exists to prove that `[workspace.lints]` (root `Cargo.toml`,
//! doc 10 §3.1) is genuinely inherited by a crate whose manifest carries
//! nothing but `[lints] workspace = true`.
//!
//! Proof — run from the repository root; BOTH commands must FAIL:
//!
//! ```text
//! cargo clippy -p template --all-targets -- --cfg lint_canary
//! RUSTFLAGS="--cfg lint_canary" cargo check -p template --all-targets
//! ```
//!
//! Neither command passes any deny flag of its own, so every `error:` in the
//! output is sourced exclusively from the inherited `[workspace.lints]`:
//! `clippy::unwrap_used` (clippy table) and `unused_variables` (rust table).
//! The verbatim failure output of the original proof run is recorded in the
//! worklog under task 1.A1.
//!
//! DELETE THIS FILE (plus its declaration in `src/lib.rs`, plus `build.rs`)
//! when copying the template for a real crate.

/// Intentional violations of the workspace lint policy.
pub fn canary(maybe: Option<u8>) -> u8 {
    let unused_var = 7_u8; // rust: unused_variables — denied via [workspace.lints]
    maybe.unwrap() // clippy::unwrap_used — denied via [workspace.lints]
}
