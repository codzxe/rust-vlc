//! Card 3.D3's engine-side property suites (doc 03 §8, doc 13 wave 3.D):
//! black-box, seeded, deterministic property loops over the public
//! APIs of `playback-engine`, `packet-queue` and `media-types` — one
//! file per invariant family:
//!
//! - [`timestamp_arithmetic`] — overflow / discontinuity / degenerate
//!   bases / near-`i64::MAX` stamps: every conversion answers a typed
//!   error or the exactly-rounded value, never a panic (family 1);
//! - [`queue_invariants`] — the pipeline's boundedness laws under
//!   seeded playback storms: caps honored with N ≫ cap packets,
//!   backpressure exercised, nothing fabricated (family 2, engine
//!   half; the queue primitive's own suite lives in
//!   `packet-queue/tests/property.rs`);
//! - [`generation_monotonicity`] — session epochs and pipeline
//!   generations strictly increase across reopen/seek/switch storms;
//!   superseded pairs are stale forever; stale handles refuse typed
//!   (family 3);
//! - [`seek_roundtrip`] — seek-to-time → first-emitted-frame boundary
//!   properties on scripted synthetic timelines, including the
//!   pts<dts / negative-offset stamp shapes and the supersession
//!   storm (family 4).
//!
//! # Determinism discipline (no proptest — Lead-only dependency
//! approval, C5)
//!
//! Every driver is a seeded [`XorShift`] loop over a fixed seed bank
//! of 64 seeds ([`SEED_BANK`]); `LUMEN_PROP_SEEDS=<csv>` widens the
//! sweep (the corpus suites' env-gated precedent). A failure prints
//! its seed and op index; `LUMEN_PROP_SHRINK=<seed>` reruns that seed
//! under the documented shrink procedure — bisection on the failing
//! op sequence ([`shrink_prefix`]) — and reports the minimal failing
//! op prefix as the counterexample. Shrunk counterexamples become
//! committed regressions with fixed values (the card's AC), named
//! after the defect they pin.

mod engine_fx;
mod generation_monotonicity;
mod queue_invariants;
mod rng;
mod seek_roundtrip;
mod timestamp_arithmetic;
