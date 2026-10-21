//! # first-vertical — the 2.E1 end-to-end integration suite
//!
//! The first vertical, driven end to end **through the command layer**
//! and recorded as doc 10 §7 evidence:
//!
//! open the generated AVI fixture → probe → demux (AVI per D-018) →
//! decode (MJPEG + PCM) → audio sink (**null device — mock, honestly
//! labeled: no audio hardware exists in this sandbox; the real-device
//! path is audio-output's typed OS-backend stubs, ADR-005/2.C3**) →
//! video render (**headless software compositor — mock, honestly
//! labeled: no GPU in this sandbox; the wgpu backend is deferred per
//! the 2.D1 record**) — every step dispatched as a catalog
//! [`commands::Command`] through `lumen_cli::dispatch::lumen_dispatch`
//! (ARCH-5: one command layer; never a direct engine call), every
//! observation taken from the host's wire surfaces (responses,
//! events, state trail, headless accounting, the screenshot).
//!
//! # Layout
//!
//! - [`json`]     — the std-only JSON value + deterministic emitter
//!   (the transcript/ledger writer; no external deps by C5).
//! - [`digest`]   — the std-only SHA-256 (FIPS 180-4, vector-pinned)
//!   for the artifact digests the ledger records.
//! - [`harness`]  — the [`harness::SessionHarness`]: one scenario's
//!   command script + its recorded transcript, plus the doc 10 §7.1
//!   environment labels every artifact carries.
//! - [`scenarios`] — the named scenarios (doc 10 §6.4 IDs): the happy
//!   journey (`open`, `play`, `pause`, `seek`, `stop`, `run-to-end`),
//!   the class-B corpus (`corrupt-*`, `missing-file`), and the wire
//!   session (`wire-session`, through the real binary's
//!   `--control-stdio` NDJSON mode — the serialized wire form across
//!   a process boundary).
//! - `tests/evidence.rs` — the committed-artifact currency checks +
//!   the `#[ignore]` regeneration entry points (`cargo test -p
//!   first-vertical --test evidence -- --ignored` rewrites
//!   `evidence/` from a fresh run; the normal gate only READS it, so
//!   a gate run never dirties the tree).
//!
//! # Evidence shape (doc 10 §6.4 + §7)
//!
//! Every scenario produces `evidence/transcripts/<id>.json` — the
//! full event log, the command script with outcomes, and the final
//! observables. One `evidence/ledger.json` (schema `evidence/1`,
//! doc 10 §7.2's envelope + an additive per-scenario table
//! `[PROPOSAL]`) carries the two label axes on every artifact:
//! `environment.kind` (`local` here; `ci-vm` under a CI runner —
//! auto-detected via the `CI` variable) and `hardware_class` —
//! **`mock` for this whole suite**: null audio + software render.
//! The committed artifacts are the recorded evidence of the sandbox
//! run that generated them; the currency tests prove they match a
//! fresh run byte-for-byte (which is also the determinism proof).
//!
//! # Honest scope (recorded for the Lead)
//!
//! - The card's "green on 3 OS in CI" AC is **PENDING**: the workflow
//!   files exist (`.github/workflows-disabled/ci.yml`, actionlint-
//!   validated) but no runner executed them in this sandbox. Joining
//!   the workspace's `cargo test --workspace` set is what this crate
//!   contributes; the 3-OS verdict lands with the first push under a
//!   workflow-scoped token (2.E2-V's to-run list).
//! - The `advance` host verb (bounded deterministic pumps) is the
//!   session-thread stand-in — the 2.D2 shape; wall-clock pacing is
//!   card 3.B3.
//! - Seek composes as stop → close → reopen-at-target (the 2.D2
//!   shape); in-session seeks are 3.A1.

pub mod digest;
pub mod evidence;
pub mod harness;
pub mod json;
pub mod scenarios;
