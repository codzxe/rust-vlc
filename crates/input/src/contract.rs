//! The scripted synthetic source + access conformance checks (card
//! 2.A1; the codec-api `contract` pattern).
//!
//! [`ScriptSource`] is an in-memory byte source with scriptable faults —
//! shrink (file truncated mid-read), injected failures — plus
//! instrumentation of the largest chunk any reader asked it to read.
//! The `assert_*` checks encode the plugin-api `SourceStream` contract
//! (short reads as data, `Ok(0)` only at EOS, position consistency,
//! typed seek errors) so every access implementation — this crate,
//! `streaming` (Phase 6), the conformance kit (doc 06 §7.2) — runs the
//! same executable contract from its own tests.
