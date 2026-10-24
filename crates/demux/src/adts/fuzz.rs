//! The ADTS/ADIF fuzz harness (card 5.C2b `+FB`; doc 04 §8.5
//! registry name `fuzz_demux_adts`).
//!
//! One call = one full untrusted-container pass: construct, `start`,
//! drain every event to end-of-stream (bounded), then probe seeks at
//! data-derived targets and drain a little after each. **The harness
//! contract is absence-of-panic/hang/unbounded-allocation, nothing
//! else** — malformed input is *expected* to produce typed errors,
//! which are swallowed here; the conformance suite pins the specific
//! error flavors.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file is a Lead-owned dependency decision
//! (`libfuzzer-sys`), exactly as the AVI/WAV/Ogg cards recorded; until
//! it lands the harness logic is proven by the deterministic in-crate
//! sweeps (bit-flip/truncation families over the committed fixtures
//! and the real corpus, run by the unit + integration suites).
//!
//! # Bounds (MP §12)
//!
//! - Event budget: hard cap per pass; the walk is monotone (positions
//!   only advance), so the budget is a second belt over a structurally
//!   bounded first one.
//! - Recovery is bounded by
//!   [`MAX_RECOVERY_SCAN_BYTES`](super::MAX_RECOVERY_SCAN_BYTES);
//!   packet memory by [`MAX_FRAME_BYTES`](super::MAX_FRAME_BYTES).

use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};
use plugin_api::{DemuxEvent, Demuxer};

use super::AdtsDemuxer;

/// Maximum events drained per pass `[PROPOSAL: 500_000 — two orders
/// of magnitude above the largest sane raw-AAC file's frame count]`.
pub const FUZZ_EVENT_BUDGET: usize = 500_000;

/// Events drained after each seek probe `[PROPOSAL: 64]`.
pub const FUZZ_POST_SEEK_DRAIN: usize = 64;

/// Number of data-derived seek probes per pass.
pub const FUZZ_SEEK_PROBES: usize = 3;

/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates without a ceiling — by construction
/// of the demuxer and the budgets above.
pub fn fuzz_demux_adts(data: &[u8]) {
    let Ok(mut demuxer) = AdtsDemuxer::new(match super::test_support::mem_stream(data.to_vec()) {
        Ok(stream) => stream,
        Err(_) => return,
    }) else {
        return;
    };
    if demuxer.start().is_err() {
        return;
    }
    let mut drained = 0usize;
    let mut last_ticks = 0i64;
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(DemuxEvent::Packet(p)) => {
                if let Stamp::Present(t) = p.pts {
                    last_ticks = t.ticks;
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
        drained += 1;
        if drained >= FUZZ_EVENT_BUDGET {
            break;
        }
    }
    // Seek probes: land at fractions of the observed extent, drain a
    // little after each (the post-seek paths are fuzz surface too).
    if let Ok(base) = TimeBase::new(1, 48_000) {
        for fraction in 1..=FUZZ_SEEK_PROBES {
            let ticks = last_ticks.saturating_mul(fraction as i64)
                / i64::try_from(FUZZ_SEEK_PROBES).unwrap_or(3);
            let target = TimeSpan {
                start: Stamp::Present(Timestamp::new(ticks.max(0), base)),
                end: Stamp::Missing,
            };
            if demuxer.reanchor(target).is_err() {
                continue;
            }
            for _ in 0..FUZZ_POST_SEEK_DRAIN {
                match demuxer.next_event() {
                    Ok(DemuxEvent::EndOfStream) | Err(_) => break,
                    Ok(_) => {}
                }
            }
        }
    }
}
