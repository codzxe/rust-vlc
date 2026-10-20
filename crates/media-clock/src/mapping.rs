//! MediaTime mapping: sample position ↔ media [`Timestamp`] (card 2.C1;
//! doc 03 §4.7 a).
//!
//! The audio-master clock's arithmetic, exposed as pure checked
//! functions so the engine crates (and tests) can map sample-counter
//! positions without going through a clock instance:
//!
//! - [`sample_delta_to_media_nanos`] — the core scaling:
//!   `media = counter_elapsed × rate` (see the note below on the doc's
//!   sketch formula);
//! - [`map_samples_to_timestamp`] / [`map_timestamp_to_samples`] — the
//!   forward and inverse mappings over an anchor [`Timestamp`];
//! - [`samples_until_end`] — extent arithmetic where
//!   [`Duration::Unknown`] propagates as [`TimeError::Missing`] per the
//!   taxonomy (absence is data, never `0`, never `-1`; MP §8).
//!
//! All arithmetic is checked `i128` with ties-away-from-zero rounding —
//! the same discipline as `media-types`' `Timestamp::convert` (doc 01
//! §3.2: conversion failure is data, never a panic, never a wrap).
//!
//! # Rate semantics (deviation, recorded in the worklog)
//!
//! Doc 03 §4.7(e) sketches `now() = anchor.media + (device_samples -
//! anchor.at_sample) / rate`. That formula is only dimensionally
//! consistent if "rate" means *seconds of output per second of media*.
//! Lumen's [`Rate`] is the opposite convention (doc 01 §3.1): a speed
//! multiplier — `5/4` = 1.25× = five media-seconds per four
//! counter-seconds. This module therefore scales as
//! `media_elapsed = counter_elapsed × rate.num / rate.den`, the standard
//! player semantics and the one the card's rate acceptance tests encode.
//!
//! # Degenerate inputs
//!
//! A zero sample rate or a zero-component [`Rate`] cannot be interpreted
//! as time. Following `media-types`' precedent for impossible divisions
//! (`div_round_i128` → `None` → `Overflow`, `time.rs`), they are
//! reported as [`TimeError::Overflow`] — the taxonomy's
//! "cannot be represented" variant — rather than panicking or silently
//! guessing. Clock-internal rate storage is additionally validated at
//! the `set_rate` boundary (see [`crate::clock`]).

use media_types::time::{Duration, Rate, TimeBase, TimeError, Timestamp};

/// The canonical nanosecond base every clock position uses: a
/// [`Timestamp`] whose ticks *are* nanoseconds — the implemented analog
/// of doc 03 §3's `MediaTime(i64)` (normalized media time in ns).
///
/// `now()` returns positions in this base; callers convert to their
/// stream's base with [`Timestamp::convert`] (checked, doc 01 §3.2).
pub const NANO_BASE: TimeBase = TimeBase {
    num: 1,
    den: 1_000_000_000,
};

const NANOS_PER_SEC: i128 = 1_000_000_000;

/// Divide with rounding, ties away from zero (`None` on a degenerate
/// denominator) — the `media-types` `time.rs` discipline, reimplemented
/// here because the original is crate-private.
fn div_round(numer: i128, denom: i128) -> Option<i128> {
    if denom <= 0 {
        return None;
    }
    let (q, r) = (numer / denom, numer % denom);
    let twice_r = r.checked_mul(2)?;
    if twice_r >= denom {
        q.checked_add(1)
    } else if twice_r <= -denom {
        q.checked_sub(1)
    } else {
        Some(q)
    }
}

/// A taxonomy-consistent overflow for degenerate/oversized conversions.
/// The `from`/`to` fields carry the bases involved; `ticks` carries the
/// offending magnitude (saturated to `i64` — context, not a value used
/// for arithmetic).
fn overflow(from: TimeBase, to: TimeBase, magnitude: i128) -> TimeError {
    TimeError::Overflow {
        from,
        to,
        ticks: i64::try_from(magnitude).unwrap_or(if magnitude >= 0 { i64::MAX } else { i64::MIN }),
    }
}

/// Checked conversion of a [`Timestamp`] to signed nanoseconds — the
/// signed complement of `Timestamp::as_nanos` (which rejects negative
/// positions; negative media time exists in the wild, doc 01 §3.2).
/// Positions cap at the `i64` tick range in [`nanos_to_timestamp`];
/// this `i128` stage is the checked intermediate.
///
/// # Errors
/// [`TimeError::Overflow`] when the value cannot be represented —
/// practically unreachable with `i128` intermediates (kept for
/// exhaustive checkedness, the `media-types` `div_round_i128`
/// precedent).
pub fn timestamp_to_nanos(ts: Timestamp) -> Result<i128, TimeError> {
    let numer = i128::from(ts.ticks)
        .checked_mul(i128::from(ts.base.num))
        .and_then(|v| v.checked_mul(NANOS_PER_SEC));
    let Some(numer) = numer else {
        return Err(overflow(ts.base, NANO_BASE, i128::from(ts.ticks)));
    };
    div_round(numer, i128::from(ts.base.den))
        .ok_or_else(|| overflow(ts.base, NANO_BASE, i128::from(ts.ticks)))
}

/// Checked construction of a nanosecond-base [`Timestamp`] from signed
/// nanoseconds.
///
/// # Errors
/// [`TimeError::Overflow`] when `ns` exceeds the `i64` tick range
/// (±292 years — generous, but streams are adversarial).
pub fn nanos_to_timestamp(ns: i128) -> Result<Timestamp, TimeError> {
    let ticks = i64::try_from(ns).map_err(|_| overflow(NANO_BASE, NANO_BASE, ns))?;
    Ok(Timestamp::new(ticks, NANO_BASE))
}

/// Validate that a rate can scale time (non-zero components), else map
/// to the taxonomy's unrepresentable-conversion variant (see module
/// docs: degenerate denominators follow the `media-types` precedent).
fn checked_rate(rate: Rate) -> Result<Rate, TimeError> {
    if rate.num == 0 || rate.den == 0 {
        return Err(overflow(NANO_BASE, NANO_BASE, 0));
    }
    Ok(rate)
}

/// The core audio-master scaling: elapsed media nanoseconds for
/// `delta_samples` consumed at `rate_hz`, under playback `rate`
/// (media seconds per counter second; module docs).
///
/// # Errors
/// [`TimeError::Overflow`] when the computation cannot be represented
/// (extreme sample counts or rates — doc 03 §4.7 f "overflow" tests),
/// or when `rate_hz`/`rate` has a zero component (degenerate, module
/// docs).
pub fn sample_delta_to_media_nanos(
    delta_samples: u64,
    rate_hz: u32,
    rate: Rate,
) -> Result<i128, TimeError> {
    let rate = checked_rate(rate)?;
    // media_ns = round(round(delta × 1e9 / rate_hz) × num / den), done as
    // one rational to round once: delta × 1e9 × num / (rate_hz × den).
    let numer = i128::from(delta_samples)
        .checked_mul(NANOS_PER_SEC)
        .and_then(|v| v.checked_mul(i128::from(rate.num)));
    let denom = i128::from(rate_hz).checked_mul(i128::from(rate.den));
    let (Some(numer), Some(denom)) = (numer, denom) else {
        return Err(overflow(NANO_BASE, NANO_BASE, i128::from(delta_samples)));
    };
    div_round(numer, denom).ok_or_else(|| overflow(NANO_BASE, NANO_BASE, i128::from(delta_samples)))
}

/// Forward mapping: the media position after consuming
/// `delta_samples` (at `rate_hz`, playback `rate`) starting from
/// `origin`.
///
/// # Errors
/// [`TimeError::Overflow`] when `origin` or the result cannot be
/// represented (checked `i128` throughout), or on degenerate
/// `rate_hz`/`rate` (module docs).
pub fn map_samples_to_timestamp(
    origin: Timestamp,
    delta_samples: u64,
    rate_hz: u32,
    rate: Rate,
) -> Result<Timestamp, TimeError> {
    let origin_ns = timestamp_to_nanos(origin)?;
    let media_ns = sample_delta_to_media_nanos(delta_samples, rate_hz, rate)?;
    let total = origin_ns
        .checked_add(media_ns)
        .ok_or_else(|| overflow(origin.base, NANO_BASE, origin_ns))?;
    nanos_to_timestamp(total)
}

/// Inverse mapping: the sample-counter delta that carries `origin` to
/// `target` at `rate_hz` under playback `rate` — "when does the counter
/// reach this media time". Exact inverse of
/// [`map_samples_to_timestamp`] whenever the values divide evenly.
///
/// # Errors
/// [`TimeError::Overflow`] when `target` precedes `origin` (a negative
/// sample count is not representable in `u64` — the taxonomy's
/// "cannot be represented" variant, module docs), when the result
/// exceeds `u64` samples, or on degenerate `rate_hz`/`rate`.
pub fn map_timestamp_to_samples(
    origin: Timestamp,
    target: Timestamp,
    rate_hz: u32,
    rate: Rate,
) -> Result<u64, TimeError> {
    let rate = checked_rate(rate)?;
    let origin_ns = timestamp_to_nanos(origin)?;
    let target_ns = timestamp_to_nanos(target)?;
    let Some(delta_ns) = target_ns.checked_sub(origin_ns) else {
        return Err(overflow(NANO_BASE, NANO_BASE, target_ns));
    };
    if delta_ns < 0 {
        // target before origin: the answer is a negative sample count.
        return Err(overflow(NANO_BASE, NANO_BASE, delta_ns));
    }
    // counter_ns = media_ns × den / num; samples = counter_ns × rate_hz / 1e9
    // — one rational, one rounding.
    let numer = delta_ns
        .checked_mul(i128::from(rate.den))
        .and_then(|v| v.checked_mul(i128::from(rate_hz)));
    let denom = i128::from(rate.num).checked_mul(NANOS_PER_SEC);
    let (Some(numer), Some(denom)) = (numer, denom) else {
        return Err(overflow(NANO_BASE, NANO_BASE, delta_ns));
    };
    let samples =
        div_round(numer, denom).ok_or_else(|| overflow(NANO_BASE, NANO_BASE, delta_ns))?;
    u64::try_from(samples).map_err(|_| overflow(NANO_BASE, NANO_BASE, samples))
}

/// Sample-counter delta from `origin` until the end of the media
/// extent — the end-of-media arithmetic the playback engine needs for
/// `Ended` detection and the pump needs for draining.
///
/// # Errors
/// [`TimeError::Missing`] when `extent` is [`Duration::Unknown`] —
/// absence is data, propagated per the taxonomy, never guessed as `0`
/// or `-1` (MP §8). [`TimeError::Overflow`] as in
/// [`map_timestamp_to_samples`].
pub fn samples_until_end(
    origin: Timestamp,
    extent: Duration,
    rate_hz: u32,
    rate: Rate,
) -> Result<u64, TimeError> {
    let end_ns = extent.nanos()?; // Unknown -> TimeError::Missing (taxonomy)
    let origin_ns = timestamp_to_nanos(origin)?;
    let end = origin_ns
        .checked_add(i128::from(end_ns.0))
        .ok_or_else(|| overflow(origin.base, NANO_BASE, origin_ns))?;
    let end_ts = nanos_to_timestamp(end)?;
    map_timestamp_to_samples(origin, end_ts, rate_hz, rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(ticks: i64, num: u32, den: u32) -> Timestamp {
        Timestamp::from_parts(ticks, num, den).unwrap_or_else(|e| unreachable!("{e}"))
    }

    fn rate(num: u32, den: u32) -> Rate {
        Rate::new(num, den).unwrap_or_else(|e| unreachable!("{e}"))
    }

    // ---- nanosecond canonicalization ------------------------------------

    #[test]
    fn timestamp_to_nanos_handles_bases_and_signs() {
        // 90000 ticks of 1/90000 = 1 s.
        assert_eq!(
            timestamp_to_nanos(ts(90_000, 1, 90_000)).unwrap_or(0),
            1_000_000_000
        );
        // Negative ticks are legal positions (doc 01 §3.2).
        assert_eq!(
            timestamp_to_nanos(ts(-500, 1, 1_000)).unwrap_or(0),
            -500_000_000
        );
        // Sub-nanosecond values round ties away from zero (0.5 ns -> 1).
        assert_eq!(timestamp_to_nanos(ts(1, 1, 2_000_000_000)).unwrap_or(0), 1);
        assert_eq!(timestamp_to_nanos(ts(1, 1, 3_000_000_000)).unwrap_or(0), 0);
    }

    #[test]
    fn nanos_roundtrip_through_nano_base() {
        let original = ts(123_456, 1, 90_000);
        let ns = timestamp_to_nanos(original).unwrap_or(0);
        let back = nanos_to_timestamp(ns).unwrap_or_else(|e| unreachable!("{e}"));
        // Round-trip into the nano base is lossless for divisible bases...
        assert_eq!(back.base, NANO_BASE);
        // ...and re-converting the nano-base value to the origin base
        // recovers it (media-types' checked convert).
        let recovered = back
            .convert(&original.base)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(recovered.ticks, original.ticks);
    }

    #[test]
    fn extreme_timestamps_overflow_typed() {
        // The i128 pipeline holds ~9.2e21 seconds, but a POSITION must
        // fit the i64 tick range of the nano base (~292 years): the
        // overflow surfaces at nanos_to_timestamp, typed.
        let ns = timestamp_to_nanos(ts(i64::MAX, 1000, 1)).unwrap_or(0);
        assert!(ns > i128::from(i64::MAX));
        assert!(nanos_to_timestamp(ns).is_err());
        assert!(nanos_to_timestamp(i128::from(i64::MAX)).is_ok());
        assert!(nanos_to_timestamp(i128::from(i64::MIN)).is_ok());
    }

    // ---- sample scaling ---------------------------------------------------

    #[test]
    fn one_second_of_samples_is_one_second_at_rate_one() {
        let ns = sample_delta_to_media_nanos(48_000, 48_000, Rate::ONE).unwrap_or(0);
        assert_eq!(ns, 1_000_000_000);
        let ns = sample_delta_to_media_nanos(44_100, 44_100, Rate::ONE).unwrap_or(0);
        assert_eq!(ns, 1_000_000_000);
    }

    #[test]
    fn rate_scales_media_time() {
        // 5/4: four counter-seconds carry the position five media-seconds.
        let ns = sample_delta_to_media_nanos(4 * 48_000, 48_000, rate(5, 4)).unwrap_or(0);
        assert_eq!(ns, 5_000_000_000);
        // 1/2: two counter-seconds = one media-second.
        let ns = sample_delta_to_media_nanos(2 * 48_000, 48_000, rate(1, 2)).unwrap_or(0);
        assert_eq!(ns, 1_000_000_000);
        // 2/1: one counter-second = two media-seconds.
        let ns = sample_delta_to_media_nanos(48_000, 48_000, rate(2, 1)).unwrap_or(0);
        assert_eq!(ns, 2_000_000_000);
    }

    #[test]
    fn rounding_is_deterministic_and_monotone_in_delta() {
        // 1 sample at 44100 Hz = 22675.7... ns -> 22676 (ties away).
        let a = sample_delta_to_media_nanos(1, 44_100, Rate::ONE).unwrap_or(0);
        assert_eq!(a, 22_676);
        let b = sample_delta_to_media_nanos(2, 44_100, Rate::ONE).unwrap_or(0);
        assert_eq!(b, 45_351); // 2 x 1e9 / 44100 = 45351.47... -> 45351
        assert!(b >= a);
    }

    #[test]
    fn degenerate_rates_overflow_typed() {
        assert!(sample_delta_to_media_nanos(1, 0, Rate::ONE).is_err());
        // Rate with a zero component (constructible via pub fields,
        // bypassing Rate::new): must not panic, must be typed.
        let zero_rate = Rate { num: 0, den: 1 };
        assert!(sample_delta_to_media_nanos(1, 48_000, zero_rate).is_err());
        let zero_den = Rate { num: 1, den: 0 };
        assert!(sample_delta_to_media_nanos(1, 48_000, zero_den).is_err());
    }

    #[test]
    fn extreme_sample_counts_overflow_typed() {
        // u64::MAX samples at 1 Hz is ~584 years: the i128 delta is
        // representable, but the mapped POSITION overflows the nano
        // base's i64 ticks — typed error, never a wrap (doc 03 §4.7 f).
        let ns = sample_delta_to_media_nanos(u64::MAX, 1, Rate::ONE).unwrap_or(0);
        assert!(ns > i128::from(i64::MAX));
        let zero = ts(0, 1, 1_000_000_000);
        assert!(map_samples_to_timestamp(zero, u64::MAX, 1, Rate::ONE).is_err());
        // At a real audio rate the count is ~12,000 years — still beyond
        // the nano base's ~292-year range; a 4-hour session is fine.
        assert!(map_samples_to_timestamp(zero, u64::MAX, 48_000, Rate::ONE).is_err());
        let four_hours = 4 * 3600 * 48_000;
        assert!(map_samples_to_timestamp(zero, four_hours, 48_000, Rate::ONE).is_ok());
    }

    // ---- forward / inverse mapping ---------------------------------------

    #[test]
    fn forward_mapping_composes_origin_and_delta() {
        let origin = ts(1_000_000_000, 1, 1_000_000_000); // 1 s in ns base
        let mapped = map_samples_to_timestamp(origin, 48_000, 48_000, Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(mapped.ticks, 2_000_000_000);
        assert_eq!(mapped.base, NANO_BASE);
    }

    #[test]
    fn inverse_mapping_roundtrips_exactly() {
        let origin = ts(500_000_000, 1, 1_000_000_000);
        let forward = map_samples_to_timestamp(origin, 144_000, 48_000, rate(5, 4))
            .unwrap_or_else(|e| unreachable!("{e}"));
        // 3 counter-seconds at 5/4 = 3.75 media-seconds from 0.5 s -> 4.25 s.
        assert_eq!(forward.ticks, 4_250_000_000);
        let back = map_timestamp_to_samples(origin, forward, 48_000, rate(5, 4))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(back, 144_000);
    }

    #[test]
    fn inverse_mapping_rejects_target_before_origin() {
        let origin = ts(1_000_000_000, 1, 1_000_000_000);
        let target = ts(500_000_000, 1, 1_000_000_000);
        assert!(map_timestamp_to_samples(origin, target, 48_000, Rate::ONE).is_err());
    }

    #[test]
    fn inverse_mapping_overflows_typed_for_huge_targets() {
        let origin = ts(0, 1, 1_000_000_000);
        let target = ts(i64::MAX, 1000, 1); // ~9.2e21 seconds
        assert!(map_timestamp_to_samples(origin, target, 48_000, Rate::ONE).is_err());
    }

    // ---- extent / duration taxonomy ---------------------------------------

    #[test]
    fn known_extent_maps_to_samples() {
        let origin = ts(0, 1, 1_000_000_000);
        let extent = Duration::Known(media_types::time::Nanoseconds(10_000_000_000));
        let samples = samples_until_end(origin, extent, 48_000, rate(2, 1))
            .unwrap_or_else(|e| unreachable!("{e}"));
        // 10 s of media at 2x consumes 5 counter-seconds = 240000 samples.
        assert_eq!(samples, 240_000);
    }

    #[test]
    fn unknown_extent_is_missing_per_taxonomy() {
        let origin = ts(0, 1, 1_000_000_000);
        match samples_until_end(origin, Duration::Unknown, 48_000, Rate::ONE) {
            Err(TimeError::Missing) => {}
            other => unreachable!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn extent_overflow_is_typed() {
        let origin = ts(i64::MAX - 1_000_000_000, 1, 1_000_000_000);
        let extent = Duration::Known(media_types::time::Nanoseconds(u64::MAX));
        assert!(samples_until_end(origin, extent, 48_000, Rate::ONE).is_err());
    }
}
