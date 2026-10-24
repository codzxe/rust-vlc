//! The MP4 fuzz harness (card 5.C1 `+FB`; doc 04 §8.5 registry name
//! `fuzz_demux_mp4`).
//!
//! One call = one full untrusted-container pass: construct, `start`,
//! drain every event to end-of-stream (bounded), then probe seeks at
//! data-derived targets and drain a little after each. **The harness
//! contract is absence-of-panic/hang/unbounded-allocation, nothing
//! else** — malformed input is *expected* to produce typed errors,
//! which are swallowed here; the conformance suite pins the specific
//! error flavors.
//!
//! # Bounds (MP §12)
//!
//! - Event budget: hard cap on every loop.
//! - Structural reads are ceiling-checked inside the demuxer
//!   (`moov`/`moof` ≤ 16 MiB/1 MiB; samples ≤ 16 MiB each through
//!   the 2-slot pool).
//! - The fragment scan is bounded by
//!   [`MAX_FRAGMENT_SCAN_BOXES`](super::MAX_FRAGMENT_SCAN_BOXES).

use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};
use plugin_api::{DemuxEvent, Demuxer};

use super::Mp4Demuxer;

/// Maximum events drained per pass.
pub const FUZZ_EVENT_BUDGET: usize = 500_000;

/// Events drained after each seek probe.
pub const FUZZ_POST_SEEK_DRAIN: usize = 64;

/// Number of data-derived seek probes per pass.
pub const FUZZ_SEEK_PROBES: usize = 3;

fn span(nanos: u64) -> TimeSpan {
    let base = TimeBase::new(1, 1_000_000_000).unwrap_or(TimeBase { num: 1, den: 1 });
    TimeSpan {
        start: Stamp::Present(Timestamp::new(nanos as i64, base)),
        end: Stamp::Missing,
    }
}

/// Run one fuzz pass over `data` (see the module docs). Never panics
/// by construction of the demuxer and the budgets above.
pub fn fuzz_demux_mp4(data: &[u8]) {
    let Ok(stream) = super::test_support::mem_stream(data.to_vec()) else {
        return;
    };
    let Ok(mut mp4) = Mp4Demuxer::new(stream) else {
        return;
    };
    let mut events = 0usize;
    if mp4.start().is_err() {
        return;
    }
    loop {
        events += 1;
        if events > FUZZ_EVENT_BUDGET {
            return;
        }
        match mp4.next_event() {
            Ok(DemuxEvent::EndOfStream) | Err(_) => break,
            Ok(_) => {}
        }
    }
    // Seek probes at data-derived targets.
    for probe in 0..FUZZ_SEEK_PROBES {
        let at = data
            .get(data.len().saturating_sub(probe + 1))
            .map_or(0, |b| u64::from(*b) * 10_000_000);
        if mp4.seek(span(at)).is_ok() {
            for _ in 0..FUZZ_POST_SEEK_DRAIN {
                events += 1;
                if events > FUZZ_EVENT_BUDGET {
                    return;
                }
                match mp4.next_event() {
                    Ok(DemuxEvent::EndOfStream) | Err(_) => break,
                    Ok(_) => {}
                }
            }
        }
    }
}
