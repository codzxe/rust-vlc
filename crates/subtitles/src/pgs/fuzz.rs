//! The PGS fuzz harness (card 5.N1 `+FB`; doc 04 §5 — registry name
//! `fuzz_sub_pgs`).
//!
//! One call = one full untrusted-payload pass through **every**
//! decode surface: the framing detector, the segment walk
//! (type/size/END-overrun), the PCS grammar (states, palette refs,
//! object refs incl. crop), WDS, PDS (5-byte entries), ODS (first/
//! continuation fragments, length accounting), the epoch flushes,
//! the RLE tables (literals, `0x00` escapes, 16-bit runs, EOL snap)
//! and the cue render path over whatever survived. **The harness
//! contract is absence-of-panic/hang/unbounded-allocation, nothing
//! else** — malformed input is *expected* to produce damage events
//! or zero objects, both swallowed here; the AC suites (unit +
//! integration) pin the specific outcomes.
//!
//! # Wiring status (honest, srt/dvb precedent)
//!
//! The cargo-fuzz target file belongs at
//! `fuzz/fuzz_targets/fuzz_sub_pgs.rs` exactly as doc 04 §8.5
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
//! One pass, output ≤ O(input) + the object canvas cap
//! ([`crate::pgs::rle::MAX_OBJECT_PIXELS`]); the damage ledger is
//! capped at [`crate::pgs::MAX_DAMAGE_EVENTS`].

use crate::pgs::{PgsCueBuilder, PgsDecoder};

/// Run one fuzz pass over `data` (see the module docs). Never
/// panics, never hangs, never allocates without a ceiling — by
/// construction of the decoder's bounds-checked paths.
pub fn fuzz_sub_pgs(data: &[u8]) {
    let mut decoder = PgsDecoder::new();
    // Split the payload into pseudo display sets at every END
    // segment the mutator left us, so the epoch state machine is
    // exercised across display sets, not just one.
    let mut chunks: Vec<&[u8]> = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i + 3 <= data.len() {
        if data[i] == 0x80 && data[i + 1] == 0x00 && data[i + 2] == 0x00 && i > start {
            chunks.push(&data[start..i + 3]);
            start = i + 3;
        }
        i += 1;
    }
    if start < data.len() || chunks.is_empty() {
        chunks.push(&data[start.min(data.len())..]);
    }
    if chunks.len() == 1 {
        // Single chunk: feed whole, then a truncation sweep so the
        // typed truncation paths run.
        let _ = decoder.decode_display_set(data, Some(0));
        for cut in [data.len() / 3, data.len() / 2, data.len().saturating_sub(1)] {
            let _ = decoder.decode_display_set(&data[..cut], Some(90_000));
        }
    } else {
        for (n, chunk) in chunks.iter().enumerate() {
            let _ = decoder.decode_display_set(chunk, Some(90_000 * (n as i64 + 1)));
        }
    }
    // One reset + re-feed exercises the state machine's re-lock.
    decoder.reset();
    let _ = decoder.decode_display_set(data, None);

    // The render path over the LAST set whatever it was.
    let set = decoder.decode_display_set(&[], None);
    let builder = PgsCueBuilder::new();
    let _ = builder.cues(&set, Some(1_000_000_000), 0);
}
