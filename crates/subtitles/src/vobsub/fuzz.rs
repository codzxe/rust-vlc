//! The VobSub fuzz harness (card 5.N2 `+FB`; doc 04 §5 — registry
//! name `fuzz_sub_vobsub`).
//!
//! One call = one full untrusted idx+sub pass through **every**
//! decode surface: the idx text parse (palette/size/timestamp/
//! delay tolerances), the PS PES walk (start codes, lengths, PTS,
//! substream routing), per-entry fragment accumulation, the SPU
//! command sequences (dates, colormap, alpha, coords, offsets, HD
//! commands), the nibble-RLE interlaced bitmap, the palette/grayscale
//! application, the transparent-border crop, and the cue render
//! path. **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — malformed input is *expected* to
//! produce damage events or zero cues, both swallowed here; the AC
//! suites (unit + integration) pin the specific outcomes.
//!
//! # Wiring status (honest, srt/mjpeg precedent)
//!
//! The cargo-fuzz target file belongs at
//! `fuzz/fuzz_targets/fuzz_sub_vobsub.rs` exactly as doc 04 §8.5
//! registers it, delegating here. Building it needs the
//! `libfuzzer-sys` nightly scaffolding — a Lead-owned dependency
//! decision (AGENTS.md §3.2, C5); the `fuzz/` tree is outside this
//! card's file scope. Until the Lead wires it, the harness logic is
//! proven on stable by the deterministic survival runs in
//! `crates/subtitles/tests/` (committed mutated/truncated corpus +
//! a seeded LCG mutation loop).
//!
//! # Bounds (MP §12 / D-2)
//!
//! One pass, output ≤ O(input) + the 128 KiB SPU accumulation cap;
//! the damage ledger is capped at
//! [`crate::vobsub::MAX_DAMAGE_EVENTS`].

use crate::vobsub::{parse_vobsub, vobsub_cues};

/// Run one fuzz pass over `idx_data` + `sub_data` (see the module
/// docs). Never panics, never hangs, never allocates without a
/// ceiling — by construction of the parsers' bounds-checked paths.
pub fn fuzz_sub_vobsub(idx_data: &[u8], sub_data: &[u8]) {
    let doc = parse_vobsub(idx_data, sub_data);
    // Exercise the render path over whatever survived.
    let _ = vobsub_cues(&doc, 0);
    // Truncation sweeps over both halves.
    for cut in [sub_data.len() / 4, sub_data.len() / 2, sub_data.len().saturating_sub(3)] {
        let _ = parse_vobsub(idx_data, &sub_data[..cut]);
    }
    for cut in [idx_data.len() / 3, idx_data.len() / 2] {
        let _ = parse_vobsub(&idx_data[..cut], sub_data);
    }
}
