//! The Matroska/WebM fuzz harness (card 5.Q1; doc 04 §8.5 registry
//! name `fuzz_demux_mkv`, table §2.1 row "Matroska / WebM").
//!
//! One call = one full untrusted-container pass: construct, `start`,
//! drain every event to end-of-stream (budgeted). **The harness
//! contract is absence-of-panic/hang/unbounded-allocation, nothing
//! else** — malformed input is *expected* to produce typed errors,
//! concealed blocks or skipped packets, all swallowed here; the
//! conformance suites (mkv_corpus / mkv_editions_tags / …) pin the
//! specific outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_demux_mkv.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's integration corpora. Seek probing rides the
//! mkv integration suites (cues-indexed); this pass covers the full
//! header/cluster/block walk, which is where untrusted bytes land.
//!
//! # Bounds (MP §12)
//!
//! Event budget: hard cap below; the walk's positions are monotone.
//! Payload memory: the demuxer's own ceilings (`MAX_*` in the module)
//! apply — this harness adds none.

use plugin_api::{DemuxEvent, Demuxer};

use super::MkvDemuxer;

/// Maximum events drained per pass `[PROPOSAL: 200_000 — two orders
/// of magnitude above the largest sane Matroska's packet count]`.
pub const FUZZ_EVENT_BUDGET: usize = 200_000;

/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates without a ceiling — by construction
/// of the demuxer and the budget above.
pub fn fuzz_demux_mkv(data: &[u8]) {
    let Ok(stream) = crate::avi::test_support::mem_stream(data.to_vec()) else {
        return;
    };
    let Ok(mut demuxer) = MkvDemuxer::new(stream) else {
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
    fn empty_and_marker_inputs_survive() {
        fuzz_demux_mkv(&[]);
        fuzz_demux_mkv(b"\x1A\x45\xDF\xA3");
    }

    #[test]
    fn hostile_wide_inputs_survive() {
        // All-0xFF (EBML maximal varint false positives) and
        // all-0x00 (zero-length ID/size fields).
        fuzz_demux_mkv(&vec![0xff; 4096]);
        fuzz_demux_mkv(&vec![0x00; 4096]);
        for length in 1..=48usize {
            let data: Vec<u8> = (0..length).map(|i| (i * 53 % 256) as u8).collect();
            fuzz_demux_mkv(&data);
        }
    }
}
