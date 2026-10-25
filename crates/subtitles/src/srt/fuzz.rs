//! The SRT fuzz harness (card 5.M1 `+FB`; doc 04 §5 table row
//! "SubRip (.srt) + variants" — registry name `fuzz_sub_srt`).
//!
//! One call = one full untrusted-SRT pass through **every** parse
//! surface: byte decode (all three BOM paths + lossy fallback),
//! line normalization, block walk, timing parse, markup parse, and
//! the render path (box + placement) over every surviving cue.
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — malformed input is *expected* to
//! produce damage events or zero cues, both swallowed here; the AC
//! suites (unit + integration) pin the specific outcomes.
//!
//! # Wiring status (honest, mjpeg 2.B4 precedent)
//!
//! The cargo-fuzz target file belongs at
//! `fuzz/fuzz_targets/fuzz_sub_srt.rs` exactly as doc 04 §8.5
//! registers it, delegating here. Building it needs the
//! `libfuzzer-sys` nightly scaffolding — a Lead-owned dependency
//! decision (AGENTS.md §3.2, C5); the `fuzz/` tree is outside this
//! card's file scope. Until the Lead wires it, the harness logic is
//! proven on stable by the deterministic survival runs in
//! `crates/subtitles/tests/` (committed mutated/truncated corpus +
//! seeded LCG mutation loop).
//!
//! # Bounds (MP §12 / D-2)
//!
//! * Parse: one pass, output ≤ O(input); the damage ledger is
//!   capped (256 events) and the cue count is bounded by the block
//!   count (≤ input bytes / 2).
//! * Render: per cue, a transient probe bitmap of at most
//!   [`MAX_PROBE_PIXELS`] pixels (16 MiB), freed on return —
//!   plan-time cost, never per frame.

use crate::srt::parse;
use crate::srt::render::{TextMetrics, place_plain_text};

/// Run one fuzz pass over `data` (see the module docs). Never
/// panics, never hangs, never allocates without a ceiling — by
/// construction of the parser's bounds-checked paths.
pub fn fuzz_sub_srt(data: &[u8]) {
    let doc = parse(data);
    // Exercise the render path over whatever survived the parse.
    let metrics = TextMetrics::new(40, 20);
    for cue in &doc.cues {
        let _ = place_plain_text(cue, &metrics, (1920, 1080));
    }
}
