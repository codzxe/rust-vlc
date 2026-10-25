//! The MPEG-TS fuzz harness (card 5.Q1; doc 04 §8.5 registry name
//! `fuzz_demux_ts`, table §2.1 row "MPEG-TS").
//!
//! One call = one full untrusted-transport pass: construct, `start`,
//! drain every event to end-of-stream (budgeted) — the 188-byte
//! packet walk, PSI assembly (PAT/PMT/SDT), resync-on-garbage and the
//! PES timing model all ride that drain. **The harness contract is
//! absence-of-panic/hang/unbounded-allocation, nothing else** —
//! broadcast-style damage is *expected* to surface as typed errors or
//! `Recovered` events, both swallowed here; `ts_corpus` /
//! `ts_pes_corpus` pin the specific outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_demux_ts.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's TS corpora.
//!
//! # Bounds (MP §12)
//!
//! Event budget: hard cap below; positions are monotone. Memory: the
//! demuxer's own ceilings (section assembler caps, damage-log cap)
//! apply — this harness adds none.

use plugin_api::{DemuxEvent, Demuxer};

use super::TsDemuxer;

/// Maximum events drained per pass `[PROPOSAL: 200_000 — two orders
/// of magnitude above the largest sane capture's packet count]`.
pub const FUZZ_EVENT_BUDGET: usize = 200_000;

/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates without a ceiling — by construction
/// of the demuxer and the budget above.
pub fn fuzz_demux_ts(data: &[u8]) {
    let Ok(stream) = super::test_support::mem_stream(data.to_vec()) else {
        return;
    };
    let Ok(mut demuxer) = TsDemuxer::new(stream) else {
        return;
    };
    if demuxer.start().is_err() {
        return;
    }
    let mut drained = 0usize;
    loop {
        if drained >= FUZZ_EVENT_BUDGET {
            return;
        }
        drained += 1;
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(_))
            | Ok(DemuxEvent::Recovered { .. })
            | Ok(DemuxEvent::Discontinuity) => continue,
            Ok(DemuxEvent::EndOfStream) | Err(_) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_sync_inputs_survive() {
        fuzz_demux_ts(&[]);
        fuzz_demux_ts(&[0x47]);
        fuzz_demux_ts(&[0x47; 188]);
    }

    #[test]
    fn hostile_wide_inputs_survive() {
        // All-0x47: every offset looks like a sync byte (maximal
        // false-positive resync); all-0xFF and all-0x00: the two
        // degenerate adaptation-field/pointer patterns.
        fuzz_demux_ts(&vec![0x47; 4096]);
        fuzz_demux_ts(&vec![0xff; 4096]);
        fuzz_demux_ts(&vec![0x00; 4096]);
        for length in 1..=96usize {
            let data: Vec<u8> = (0..length).map(|i| (i * 37 % 256) as u8).collect();
            fuzz_demux_ts(&data);
        }
    }
}
