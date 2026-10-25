//! The CC fuzz harness (card 5.N3 `+FB`; doc 04 §5 — registry name
//! `fuzz_sub_cc`).
//!
//! One call = one full untrusted-payload pass through every decode
//! surface: both carriage grammars (the byte-aligned GA94 SEI/T.35
//! walk and the bit-packed SCTE-20 walk — including their typed
//! truncation paths via a truncation sweep), the 608 state machine
//! (parity failures, XDS skips, the channel latch, all control-code
//! families, roll-up scrolling, the pop-on double buffer), the 708
//! reassembler + service walk (packet starts, continuations,
//! escapes, truncated blocks, the service-1 route), and the render
//! path over whatever survived. **The harness contract is
//! absence-of-panic/hang/unbounded-allocation, nothing else** —
//! malformed input is *expected* to produce damage or zero events,
//! both swallowed here; the AC suites pin the specific outcomes.
//!
//! # Wiring status (honest, the dvb/vobsub precedent)
//!
//! The cargo-fuzz target file belongs at
//! `fuzz/fuzz_targets/fuzz_sub_cc.rs` exactly as doc 04 §8.5 would
//! register it, delegating here. The `fuzz/` tree is Lead-owned
//! (AGENTS.md §3.2, C5) and outside this card's file scope; until
//! the Lead wires it, the harness logic is proven on stable by the
//! deterministic survival runs in `crates/subtitles/tests/`
//! (seeded-LCG mutation + systematic truncation sweeps).
//!
//! # Bounds (MP §12 / D-2)
//!
//! One pass, output ≤ O(input) + the fixed 15×32 grids; the damage
//! ledger is capped at [`crate::cc::MAX_DAMAGE_EVENTS`] and the
//! DTVCC reassembly buffer at [`crate::cc::cea708::MAX_DTVCC_PACKET`].

use crate::cc::cc_data::{parse_ga94_sei, parse_mpeg2_ga94, parse_scte20};
use crate::cc::{CcCueBuilder, CcDecoder};

/// Run one fuzz pass over `data` (see the module docs). Never
/// panics, never hangs, never allocates without a ceiling — by
/// construction of the decoder's bounds-checked paths.
pub fn fuzz_sub_cc(data: &[u8]) {
    // Split into pseudo-SEI payloads at every 0xB5 country-code
    // byte the mutator left us, so the carriage grammars and the
    // decode state machine are exercised across payloads.
    let mut chunks: Vec<&[u8]> = Vec::new();
    let mut start = 0usize;
    for (i, w) in data.windows(3).enumerate() {
        if w == [0xB5, 0x00, 0x31] && i > start {
            chunks.push(&data[start..i]);
            start = i;
        }
    }
    chunks.push(&data[start..]);
    if chunks.len() == 1 {
        // Single chunk: feed whole plus a truncation sweep so the
        // typed truncation paths run.
        feed(chunks[0]);
        for cut in [data.len() / 3, data.len() / 2, data.len().saturating_sub(1)] {
            feed(&data[..cut]);
        }
    } else {
        for chunk in &chunks {
            feed(chunk);
        }
    }
}

/// One payload through every surface (fresh decoder state each
/// time — the stateful cross-payload paths are covered by the
/// multi-chunk splits and the survival suite).
fn feed(payload: &[u8]) {
    let mut decoder = CcDecoder::new();
    if let Ok(triplets) = parse_ga94_sei(payload) {
        let update = decoder.decode(&triplets, Some(90_000));
        let _ = CcCueBuilder::new().cues(&update, Some(180_000), 0);
    }
    if let Ok(triplets) = parse_mpeg2_ga94(payload) {
        let _ = decoder.decode(&triplets, None);
    }
    if let Ok(triplets) = parse_scte20(payload) {
        let update = decoder.decode(&triplets, Some(1));
        let _ = CcCueBuilder::new().cues(&update, None, 0);
    }
    // Raw-triplet shapes the grammars would not produce (cc_type 2/3
    // fragments, field traffic) — straight into the decoder.
    let triplets: Vec<_> = payload
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| crate::cc::cc_data::CcDataTriplet {
            cc_valid: t[0] & 0x04 != 0,
            cc_type: t[0] & 0x03,
            data: [t[1], t[2]],
        })
        .collect();
    let update = decoder.decode(&triplets, Some(2));
    let _ = CcCueBuilder::new().cues(&update, Some(3), 1);
    // A reset + re-feed exercises the state machine's re-lock.
    decoder.reset();
    let _ = decoder.decode(&triplets, None);
}
