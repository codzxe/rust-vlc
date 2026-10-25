//! The MPEG-PS/VOB fuzz harness (card 5.I2 `+FB`; the AVI/WAV/ADTS
//! pattern). One call = one full untrusted-container pass:
//! construct, `start`, drain every event to end-of-stream (bounded),
//! then probe seeks at data-derived targets. **The harness contract
//! is absence-of-panic/hang/unbounded-allocation, nothing else** —
//! malformed input is *expected* to produce typed errors, which are
//! swallowed here; the conformance suite pins the flavors.
//!
//! # Bounds (MP §12)
//!
//! - Event budget: hard cap per pass; the walk is monotone.
//! - Recovery is bounded by
//!   [`MAX_RECOVERY_SCAN_BYTES`](super::MAX_RECOVERY_SCAN_BYTES);
//!   packet memory by
//!   [`MAX_PACKET_BYTES`](super::MAX_PACKET_BYTES) (the pool's
//!   2-arena bound).

use media_types::time::{Stamp, TimeBase, TimeSpan};
use plugin_api::{DemuxEvent, Demuxer};

use super::PsDemuxer;

/// Maximum events drained per pass `[PROPOSAL: 500_000 — two orders
/// of magnitude above the largest committed fixture's event count]`.
pub const FUZZ_EVENT_BUDGET: usize = 500_000;

/// Events drained after each seek probe `[PROPOSAL: 64]`.
pub const FUZZ_POST_SEEK_DRAIN: usize = 64;

/// Number of data-derived seek probes per pass.
pub const FUZZ_SEEK_PROBES: usize = 3;

/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates without a ceiling.
pub fn fuzz_demux_ps(data: &[u8]) {
    let Ok(mut demuxer) = PsDemuxer::new(match super::test_support::mem_stream(data.to_vec()) {
        Ok(stream) => stream,
        Err(_) => return,
    }) else {
        return;
    };
    if demuxer.start().is_err() {
        return;
    }
    let mut drained = 0usize;
    let mut last_scr = 0u64;
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(DemuxEvent::Packet(_)) | Ok(_) => {
                last_scr = demuxer.current_scr();
            }
            Err(_) => break,
        }
        drained += 1;
        if drained >= FUZZ_EVENT_BUDGET {
            break;
        }
    }
    // Seek probes: land at fractions of the observed SCR extent,
    // drain a little after each (the post-seek paths are fuzz surface
    // too).
    if let Ok(base) = TimeBase::new(1, 90_000) {
        for fraction in 1..=FUZZ_SEEK_PROBES {
            let ticks = (last_scr.saturating_mul(fraction as u64) / FUZZ_SEEK_PROBES as u64) as i64;
            let Ok(target) = media_types::time::Timestamp::from_parts(ticks, base.num, base.den)
            else {
                continue;
            };
            let span = TimeSpan {
                start: Stamp::Present(target),
                end: Stamp::Missing,
            };
            if demuxer.seek(span).is_ok() && demuxer.reanchor(span).is_ok() {
                for _ in 0..FUZZ_POST_SEEK_DRAIN {
                    match demuxer.next_event() {
                        Ok(DemuxEvent::EndOfStream) | Err(_) => break,
                        Ok(_) => {}
                    }
                }
            }
        }
    }
}
