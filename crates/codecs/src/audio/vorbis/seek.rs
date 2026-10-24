//! Vorbis granule arithmetic (card 5.B2's seek surface): the
//! sample↔time conversions and the landing rules on the Ogg granule
//! clock.
//!
//! # The clock
//!
//! Vorbis granules count **PCM samples at the identification rate**
//! (RFC 3533 §6 + Vorbis I §A.2; the demuxer's `Clock::Samples`). A
//! page's granule is "one past the last valid sample of the last
//! packet that completes on the page" — i.e. a sample COUNT on the
//! valid (delay/padding-trimmed) timeline. This module owns the
//! three arithmetic surfaces the codec needs:
//!
//! 1. **Emission accounting** — a cold decoder emits
//!    `blocksize / 2` samples per packet (half the MDCT block; the
//!    other half overlaps the next block); after the first packet the
//!    emission is `(prev_n + n) / 4`.
//! 2. **Re-anchoring** ([`granule_reanchor`]) — the decoder's raw
//!    emission counter (which counts the encoder delay too) is
//!    re-anchored to the container's stamped granule so it never
//!    drifts: monotonic max of the two.
//! 3. **Landing** ([`landing_first_sample`]) — after a seek reanchor
//!    at a page whose granule is `G`, the packets that complete on
//!    that page emit `S` samples ending at `G`, so the landing
//!    page's first packet starts at `G − S` (clamped at 0): the
//!    decoder's overlap reset pairs with this to make post-seek
//!    output identical to a cold start at that page (the seek
//!    roundtrip suite pins it).
//!
//! # What is NOT here (honest)
//!
//! Byte-level page hunting is the demuxer's (`demux::ogg`'s
//! `SlowLinear` scan + `reanchor`); the codec never touches bytes for
//! seeking. The head-delay measurement (`head_excess`, the difference
//! between the raw emission counter and the stamped granule once it
//! stabilizes) is a decoder observable, documented on
//! [`VorbisDecoder::next_first_sample`] — it is the trim coordinate
//! the playback layer uses to land on the container timeline.

use media_types::time::{Stamp, TimeBase};

/// Nanoseconds of `sample` at `rate` (u128 intermediate, clamped).
#[must_use]
pub fn sample_to_nanos(sample: u64, rate: u32) -> u64 {
    if rate == 0 {
        return 0;
    }
    u64::try_from(u128::from(sample) * 1_000_000_000u128 / u128::from(rate)).unwrap_or(u64::MAX)
}

/// The sample index of `nanos` at `rate` (rounded down, the demuxer's
/// `granule_of_nanos` complement).
#[must_use]
pub fn nanos_to_sample(nanos: u64, rate: u32) -> u64 {
    if rate == 0 {
        return 0;
    }
    u64::try_from(u128::from(nanos) * u128::from(rate) / 1_000_000_000u128).unwrap_or(u64::MAX)
}

/// Measure the timeline shift at a stamped packet (the re-anchor
/// core): a packet that ends at raw counter `raw_end` and carries the
/// container's granule `stamp` defines
/// `shift = raw_end − granule`, the constant by which the raw
/// emission timeline (which counts the encoder delay at the stream
/// head, or the landing-page prefix after a seek) leads the valid
/// granule timeline. `None` when the packet carries no stamp (or a
/// foreign base) — the shift is then simply retained.
///
/// The decoder applies `pts = raw_first − shift` for each packet and
/// re-measures at every stamp: at the stream head the shift IS the
/// encoder delay; mid-stream it stays constant; after a seek reanchor
/// it re-derives from the landing page; at the final page it grows by
/// the tail padding (the honest signal that the last raw samples sit
/// past the valid end).
#[must_use]
pub fn measure_shift(raw_end: u64, stamp: Stamp, base: &TimeBase) -> Option<i64> {
    match stamp {
        Stamp::Present(ts) if ts.base == *base => {
            Some(i64::try_from(raw_end).unwrap_or(i64::MAX) - ts.ticks)
        }
        _ => None,
    }
}

/// The landing rule: a reanchor lands at a page whose granule is
/// `granule` (a sample count on the valid timeline); the packets
/// completing on that page emit `page_emitted` samples ending at
/// `granule`, so the landing's first sample is
/// `granule − page_emitted`, clamped at 0. `page_emitted` is what the
/// demuxer's landing page hands the decoder before the next stamp.
#[must_use]
pub fn landing_first_sample(granule: u64, page_emitted: usize) -> u64 {
    granule.saturating_sub(page_emitted as u64)
}

/// The cold-start emission of one MDCT block (half the block).
#[must_use]
pub fn cold_block_emission(blocksize: u32) -> u64 {
    u64::from(blocksize) / 2
}

/// The steady-state emission between blocks of `prev_blocksize` and
/// `blocksize` (`(prev_n + n) / 4`, the overlap-add join).
#[must_use]
pub fn join_emission(prev_blocksize: u32, blocksize: u32) -> u64 {
    (u64::from(prev_blocksize) + u64::from(blocksize)) / 4
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::time::Timestamp;

    #[test]
    fn time_conversions_roundtrip_at_common_rates() {
        for rate in [8_000u32, 44_100, 48_000] {
            let base = match TimeBase::new(1, rate) {
                Ok(base) => base,
                Err(err) => unreachable!("{err:?}"),
            };
            let _ = base;
            // 1 second is exactly `rate` samples.
            assert_eq!(sample_to_nanos(u64::from(rate), rate), 1_000_000_000);
            assert_eq!(nanos_to_sample(1_000_000_000, rate), u64::from(rate));
            // Rounding floors.
            assert_eq!(nanos_to_sample(999_999_999, rate), u64::from(rate) - 1);
        }
        // Degenerate rate never divides by zero.
        assert_eq!(sample_to_nanos(100, 0), 0);
        assert_eq!(nanos_to_sample(100, 0), 0);
    }

    #[test]
    fn shift_measurement_covers_head_seek_and_tail_shapes() {
        let base = match TimeBase::new(1, 8_000) {
            Ok(base) => base,
            Err(err) => unreachable!("{err:?}"),
        };
        let present = |ticks: i64| Stamp::Present(Timestamp::new(ticks, base));
        // Stream head: raw 2560 emitted, first stamp granule 0 → the
        // shift IS the encoder delay.
        assert_eq!(measure_shift(2560, present(0), &base), Some(2560));
        // Mid-stream: raw 10_496 through a page stamped 7936 keeps
        // the head's constant (shift = the encoder delay).
        assert_eq!(measure_shift(10_496, present(7936), &base), Some(2560));
        // Final page: the shift grows by the tail padding.
        assert_eq!(measure_shift(34_816, present(32_000), &base), Some(2816));
        // A missing or foreign stamp measures nothing.
        assert_eq!(measure_shift(34_816, Stamp::Missing, &base), None);
        let other = match TimeBase::new(1, 44_100) {
            Ok(base) => base,
            Err(err) => unreachable!("{err:?}"),
        };
        assert_eq!(
            measure_shift(34_816, Stamp::Present(Timestamp::new(9, other)), &base),
            None
        );
    }

    #[test]
    fn landing_and_emission_arithmetic_pin_the_block_math() {
        // Cold start: half the block.
        assert_eq!(cold_block_emission(2048), 1024);
        assert_eq!(cold_block_emission(512), 256);
        // Joins: same-size blocks emit n/2; mixed sizes the average.
        assert_eq!(join_emission(2048, 2048), 1024);
        assert_eq!(join_emission(512, 2048), 640);
        assert_eq!(join_emission(2048, 512), 640);
        // Landing: a page stamped 7936 whose packets emit 8192 lands
        // at 0 (clamped — the first data page's delay shape).
        assert_eq!(landing_first_sample(7936, 8192), 0);
        assert_eq!(landing_first_sample(16_128, 8192), 7936);
        assert_eq!(landing_first_sample(32_000, 7680), 24_320);
    }
}
