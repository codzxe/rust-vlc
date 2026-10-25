//! The DVB fuzz harness (card 5.N2 `+FB`; doc 04 §5 — registry name
//! `fuzz_sub_dvb`).
//!
//! One call = one full untrusted-PES pass through **every** decode
//! surface: the segment walk (sync/length/type), page composition
//! (state machine, resets, duplicate/short entries), region
//! composition (size caps, depth fallbacks, object placements),
//! CLUT definitions (full/limited range, depth flags), object data
//! (all three RLE tables, map-table sub-blocks, the 0xF0 field
//! switch, non-modifying color), display definitions, the
//! end-of-display-set composition, and the cue render path over
//! whatever survived. **The harness contract is
//! absence-of-panic/hang/unbounded-allocation, nothing else** —
//! malformed input is *expected* to produce damage events or zero
//! regions, both swallowed here; the AC suites (unit + integration)
//! pin the specific outcomes.
//!
//! # Wiring status (honest, srt/mjpeg precedent)
//!
//! The cargo-fuzz target file belongs at
//! `fuzz/fuzz_targets/fuzz_sub_dvb.rs` exactly as doc 04 §8.5
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
//! One pass, output ≤ O(input) + the region pixel-buffer cap
//! ([`crate::dvb::MAX_REGION_PIXELS`]); the damage ledger is capped
//! at [`crate::dvb::MAX_DAMAGE_EVENTS`].

use crate::dvb::{DvbCueBuilder, DvbDecoder};

/// Run one fuzz pass over `data` (see the module docs). Never
/// panics, never hangs, never allocates without a ceiling — by
/// construction of the decoder's bounds-checked paths.
pub fn fuzz_sub_dvb(data: &[u8]) {
    let mut decoder = DvbDecoder::new();
    // Split the payload into pseudo-PES chunks at every 0x0F sync
    // byte the mutator left us, so the page state machine is
    // exercised across display sets, not just one.
    let mut chunks: Vec<&[u8]> = Vec::new();
    let mut start = 0usize;
    for (i, w) in data.windows(2).enumerate() {
        if w == [0x0F, 0x10] && i > start {
            chunks.push(&data[start..i]);
            start = i;
        }
    }
    chunks.push(&data[start..]);
    if chunks.len() == 1 {
        // Single chunk: feed whole, then a truncation sweep so the
        // typed truncation paths run.
        let _ = decoder.decode_pes(data, Some(0));
        for cut in [data.len() / 3, data.len() / 2, data.len().saturating_sub(1)] {
            let _ = decoder.decode_pes(&data[..cut], Some(90_000));
        }
    } else {
        for (n, chunk) in chunks.iter().enumerate() {
            let _ = decoder.decode_pes(chunk, Some(90_000 * (n as i64 + 1)));
        }
    }
    // One reset + re-feed exercises the state machine's re-lock.
    decoder.reset();
    let _ = decoder.decode_pes(data, None);

    // The render path over the LAST update whatever it was.
    let update = decoder.decode_pes(&[], None);
    let builder = DvbCueBuilder::new(update.display_extent);
    let _ = builder.cues(&update, Some(1_000_000), 0);
}
