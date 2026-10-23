//! Family 1 — timestamp arithmetic (card 3.D3; doc 03 §8 "Time-base
//! conversions: overflow, discontinuity, missing timestamps").
//!
//! Black-box over `media-types`' public conversion surfaces —
//! [`Timestamp::convert`], [`Timestamp::as_nanos`], the batch
//! [`Scaler`] forms, construction and stamp discipline — against an
//! independently authored u128 oracle. The adversarial value space:
//!
//! - ticks: the `i64` edges (`MIN`, `MAX`, ±1 off them), powers of
//!   two, full-range uniform draws, and near-`i64::MAX` PTS clusters
//!   (the "huge PTS" wrap scenario — wrap-around must be impossible:
//!   overflow is a typed error);
//! - bases: the demuxer-common rates, uniform nonzero `u32` pairs,
//!   and the **degenerate struct-literal bases** (zero components —
//!   construction cannot produce them, but struct literals can, and
//!   the documented contract answers them typed, never a panic);
//! - the rounding corners: ties (exactly half quotients), quotients
//!   that land on `±2^63` (representable as `i64::MIN`, not as the
//!   positive), and negative ticks whose magnitude rounds to zero
//!   (the `as_nanos` subtlety: only a *non-zero* rounded negative
//!   value is an error).
//!
//! Every operation must answer `Ok(exactly-rounded)` or the typed
//! [`TimeError::Overflow`] with full context — never a panic, never a
//! wrapped value. [`Scaler`] must agree with the one-shot forms on
//! every input (the two fast-path tiers and the reference are one
//! contract). Conversions are monotone and sign-preserving; a
//! round-trip through an intermediate base recovers the original
//! within one tick (two half-away roundings).

use media_types::time::{
    Nanoseconds, Rate, Scaler, Stamp, TimeBase, TimeError, TimeSpan, Timestamp,
};
use testkit::Must;

use super::rng::{XorShift, run_case, seeds};

// ---------------------------------------------------------------------------
// The oracle (independent u128 math — no shared code with the crate)
// ---------------------------------------------------------------------------

/// The exactly-rounded (half away from zero) rational conversion
/// `ticks · from.num · to.den / (from.den · to.num)`, as `Some(i64)`;
/// `None` where the documented contract answers the typed
/// [`TimeError::Overflow`]:
///
/// - a zero divisor (`from.den == 0 || to.num == 0`) — the degenerate
///   struct-literal shape the reference's `div_round_i128` refuses;
/// - a rounded magnitude beyond `i64` — except the exact `−2^63`,
///   which is `i64::MIN` and representable.
///
/// A zero *dividend* factor (`from.num == 0 || to.den == 0`) with a
/// live divisor converts everything to exactly `0` (the reference's
/// `numer == 0` path) — not an error.
fn oracle_convert(ticks: i64, from: (u32, u32), to: (u32, u32)) -> Option<i64> {
    let divisor = u128::from(from.1) * u128::from(to.0);
    if divisor == 0 {
        return None;
    }
    let multiplier = u128::from(from.0) * u128::from(to.1);
    if multiplier == 0 {
        return Some(0);
    }
    rounded_i64(
        ticks,
        u128::from(ticks.unsigned_abs()) * multiplier,
        divisor,
    )
}

/// The exactly-rounded nanoseconds of `ticks` under `base`, as
/// `Some(u64)`; `None` where the contract answers typed overflow:
/// a zero `base.den`, a rounded magnitude beyond `u64`, or a negative
/// value (a negative tick whose magnitude rounds to zero is `Ok(0)`
/// — the reference rounds to zero before the `u64` fit check).
fn oracle_nanos(ticks: i64, base: (u32, u32)) -> Option<u64> {
    if base.1 == 0 {
        return None;
    }
    if base.0 == 0 {
        return Some(0);
    }
    let multiplier = u128::from(base.0) * 1_000_000_000;
    let dividend = u128::from(ticks.unsigned_abs()) * multiplier;
    let divisor = u128::from(base.1);
    let (quotient, remainder) = (dividend / divisor, dividend % divisor);
    let rounded = quotient + u128::from(2 * remainder >= divisor);
    if ticks < 0 && rounded > 0 {
        return None;
    }
    if rounded > u64::MAX as u128 {
        return None;
    }
    Some(rounded as u64)
}

/// `sign(ticks) · round(|ticks| · multiplier / divisor)` fitted to
/// `i64` (the `−2^63` corner included).
fn rounded_i64(ticks: i64, dividend: u128, divisor: u128) -> Option<i64> {
    let (quotient, remainder) = (dividend / divisor, dividend % divisor);
    let rounded = quotient + u128::from(2 * remainder >= divisor);
    if ticks < 0 {
        if rounded > 1u128 << 63 {
            return None;
        }
        Some(-(rounded as i128) as i64)
    } else {
        if rounded > i64::MAX as u128 {
            return None;
        }
        Some(rounded as i64)
    }
}

/// The nanosecond base the error payloads carry (`to` of an
/// `as_nanos` overflow) — `NANOS_BASE` is crate-private, spelled out
/// here.
fn nanos_base() -> TimeBase {
    TimeBase::new(1, 1_000_000_000).must()
}

// ---------------------------------------------------------------------------
// The adversarial generators
// ---------------------------------------------------------------------------

/// A raw base spec: any `(num, den)` pair, degenerate shapes
/// included (built as a struct literal when a component is zero —
/// the constructor's typed refusal is the constructor's own law,
/// pinned separately below).
fn raw_base(rng: &mut XorShift) -> (u32, u32) {
    let common: [(u32, u32); 9] = [
        (1, 90000),
        (1, 1000),
        (1, 1_000_000),
        (1, 1_000_000_000),
        (1, 24),
        (1001, 24000),
        (1, 1),
        (1, 600),
        (1, 48000),
    ];
    match rng.below(8) {
        0 => *rng.pick(&common),
        1 => (0, rng.u32()),
        2 => (rng.u32(), 0),
        3 => (0, 0),
        _ => (rng.u32(), rng.u32()),
    }
}

/// Adversarial ticks: the `i64` edges and near-edges, power-of-two
/// boundaries, and full-range uniform draws.
fn adversarial_ticks(rng: &mut XorShift) -> i64 {
    let edges: [i64; 10] = [
        i64::MAX,
        i64::MAX - 1,
        i64::MIN,
        i64::MIN + 1,
        0,
        1,
        -1,
        1 << 32,
        -(1 << 32),
        90000,
    ];
    match rng.below(8) {
        0 => *rng.pick(&edges),
        // Near-MAX PTS clusters: the wrap scenario's front line.
        1 => i64::MAX - rng.below(1024) as i64,
        2 => i64::MIN + rng.below(1024) as i64,
        3 => {
            let power = rng.below(63);
            let magnitude = 1i64.checked_shl(power as u32).unwrap_or(0);
            if rng.flip() { magnitude } else { -magnitude }
        }
        _ => rng.i64(),
    }
}

/// Build the base from a raw spec (struct literal for the degenerate
/// shapes — the public fields are the documented escape hatch the
/// reference must survive).
fn build_base(raw: (u32, u32)) -> TimeBase {
    if raw.0 == 0 || raw.1 == 0 {
        TimeBase {
            num: raw.0,
            den: raw.1,
        }
    } else {
        TimeBase::new(raw.0, raw.1).must()
    }
}

// ---------------------------------------------------------------------------
// The op stream (stateless ops — prefix shrinking is exact)
// ---------------------------------------------------------------------------

/// One conversion-shaped operation with its inputs recorded for the
/// failure message.
#[derive(Debug, Clone, Copy)]
enum TimeOp {
    Convert {
        ticks: i64,
        from: (u32, u32),
        to: (u32, u32),
    },
    Nanos {
        ticks: i64,
        base: (u32, u32),
    },
    ScalerAgreement {
        ticks: i64,
        from: (u32, u32),
        to: (u32, u32),
    },
    RoundTrip {
        ticks: i64,
        via: (u32, u32),
        back: (u32, u32),
    },
    Monotone {
        lesser: i64,
        greater: i64,
        from: (u32, u32),
        to: (u32, u32),
    },
}

fn generate_ops(rng: &mut XorShift, count: usize) -> Vec<TimeOp> {
    (0..count)
        .map(|_| match rng.below(5) {
            0 => TimeOp::Convert {
                ticks: adversarial_ticks(rng),
                from: raw_base(rng),
                to: raw_base(rng),
            },
            1 => TimeOp::Nanos {
                ticks: adversarial_ticks(rng),
                base: raw_base(rng),
            },
            2 => TimeOp::ScalerAgreement {
                ticks: adversarial_ticks(rng),
                from: raw_base(rng),
                to: raw_base(rng),
            },
            3 => TimeOp::RoundTrip {
                ticks: adversarial_ticks(rng),
                via: raw_base(rng),
                back: raw_base(rng),
            },
            _ => {
                let mut lesser = adversarial_ticks(rng);
                let mut greater = adversarial_ticks(rng);
                if lesser > greater {
                    std::mem::swap(&mut lesser, &mut greater);
                }
                TimeOp::Monotone {
                    lesser,
                    greater,
                    from: raw_base(rng),
                    to: raw_base(rng),
                }
            }
        })
        .collect()
}

fn check_convert(ticks: i64, from_raw: (u32, u32), to_raw: (u32, u32)) -> Result<(), String> {
    let from = build_base(from_raw);
    let to = build_base(to_raw);
    let context = format!("convert {ticks} ticks of {from} → {to} (raw {from_raw:?} → {to_raw:?})");
    let oracle = oracle_convert(ticks, from_raw, to_raw);
    match Timestamp::new(ticks, from).convert(&to) {
        Ok(out) => {
            let Some(expected) = oracle else {
                return Err(format!(
                    "{context}: Ok({out}) but the oracle demands typed Overflow"
                ));
            };
            if out.ticks != expected || out.base != to {
                return Err(format!(
                    "{context}: got {out}, oracle says {expected} ticks"
                ));
            }
        }
        Err(TimeError::Overflow {
            from: ef,
            to: et,
            ticks: eticks,
        }) => match oracle {
            Some(expected) => {
                return Err(format!(
                    "{context}: typed Overflow but the oracle says {expected}"
                ));
            }
            None => {
                if (ef, et, eticks) != (from, to, ticks) {
                    return Err(format!(
                        "{context}: Overflow payload ({ef}, {et}, {eticks}) lost its context"
                    ));
                }
            }
        },
        Err(other) => {
            return Err(format!("{context}: impossible error variant {other}"));
        }
    }
    Ok(())
}

fn check_nanos(ticks: i64, base_raw: (u32, u32)) -> Result<(), String> {
    let base = build_base(base_raw);
    let context = format!("as_nanos {ticks} ticks of {base} (raw {base_raw:?})");
    let oracle = oracle_nanos(ticks, base_raw);
    match Timestamp::new(ticks, base).as_nanos() {
        Ok(Nanoseconds(nanos)) => {
            let Some(expected) = oracle else {
                return Err(format!(
                    "{context}: Ok({nanos}) but the oracle demands typed Overflow"
                ));
            };
            if nanos != expected {
                return Err(format!("{context}: got {nanos}, oracle says {expected}"));
            }
        }
        Err(TimeError::Overflow {
            from: ef,
            to: et,
            ticks: eticks,
        }) => match oracle {
            Some(expected) => {
                return Err(format!(
                    "{context}: typed Overflow but the oracle says {expected}"
                ));
            }
            None => {
                if (ef, et, eticks) != (base, nanos_base(), ticks) {
                    return Err(format!(
                        "{context}: Overflow payload ({ef}, {et}, {eticks}) lost its context"
                    ));
                }
            }
        },
        Err(other) => {
            return Err(format!("{context}: impossible error variant {other}"));
        }
    }
    Ok(())
}

fn check_scaler(ticks: i64, from_raw: (u32, u32), to_raw: (u32, u32)) -> Result<(), String> {
    let from = build_base(from_raw);
    let to = build_base(to_raw);
    let scaler: Scaler = from.scaler_to(&to);
    let context = format!("scaler {ticks} ticks of {from} → {to} (raw {from_raw:?} → {to_raw:?})");
    // convert_ticks ≡ Timestamp::convert — value and error alike.
    let one_shot = Timestamp::new(ticks, from).convert(&to);
    match scaler.convert_ticks(ticks) {
        Ok(out) => match one_shot {
            Ok(expected) if out == expected => {}
            other => {
                return Err(format!(
                    "{context}: scaler said {out:?}, one-shot said {other:?}"
                ));
            }
        },
        Err(TimeError::Overflow { .. }) => {
            if one_shot.is_ok() {
                return Err(format!(
                    "{context}: scaler overflowed, one-shot said {one_shot:?}"
                ));
            }
        }
        Err(other) => return Err(format!("{context}: impossible variant {other}")),
    }
    // nanos ≡ as_nanos (any scaler's nanos is its from-base value).
    let one_shot_nanos = Timestamp::new(ticks, from).as_nanos();
    match scaler.nanos(ticks) {
        Ok(Nanoseconds(nanos)) => match one_shot_nanos {
            Ok(Nanoseconds(expected)) if nanos == expected => {}
            other => {
                return Err(format!(
                    "{context}: scaler nanos said {nanos}, one-shot said {other:?}"
                ));
            }
        },
        Err(TimeError::Overflow { .. }) => {
            if one_shot_nanos.is_ok() {
                return Err(format!(
                    "{context}: scaler nanos overflowed, one-shot said {one_shot_nanos:?}"
                ));
            }
        }
        Err(other) => return Err(format!("{context}: impossible variant {other}")),
    }
    // The dedicated nanosecond scaler (the batch hot-loop form of
    // `as_nanos` — the multiply+shift fast tier) agrees too.
    let nanos_scaler: Scaler = from.nanos_scaler();
    match nanos_scaler.nanos(ticks) {
        Ok(Nanoseconds(nanos)) => match one_shot_nanos {
            Ok(Nanoseconds(expected)) if nanos == expected => {}
            other => {
                return Err(format!(
                    "{context}: nanos scaler said {nanos}, one-shot said {other:?}"
                ));
            }
        },
        Err(TimeError::Overflow { .. }) => {
            if one_shot_nanos.is_ok() {
                return Err(format!(
                    "{context}: nanos scaler overflowed, one-shot said {one_shot_nanos:?}"
                ));
            }
        }
        Err(other) => return Err(format!("{context}: impossible variant {other}")),
    }
    Ok(())
}

fn check_roundtrip(ticks: i64, via_raw: (u32, u32), back_raw: (u32, u32)) -> Result<(), String> {
    // Only constructor-built (non-degenerate) bases: a round-trip
    // through a degenerate literal is not a contract.
    if via_raw.0 == 0 || via_raw.1 == 0 || back_raw.0 == 0 || back_raw.1 == 0 {
        return Ok(());
    }
    let start = build_base(via_raw);
    let back = build_base(back_raw);
    let context = format!("roundtrip {ticks} ticks {start} → {back} → {start}");
    // Typed overflow on either leg is a legal answer — the property
    // only binds when both legs fit.
    let Some(via) = Timestamp::new(ticks, start).convert(&back).ok() else {
        return Ok(());
    };
    let Some(home) = via.convert(&start).ok() else {
        return Ok(());
    };
    // Two half-away roundings over a rate ratio r = num/den bound the
    // homecoming drift by 0.5/r + 0.5 input ticks (the forward error
    // scales by the inverse rate, the backward error is half an input
    // tick): drift ≤ (num + den) / (2·num), integer division on both
    // sides — i128 for the difference so no magnitude is lost, u128
    // for the bound so no product wraps.
    let num = u128::from(start.num) * u128::from(back.den);
    let den = u128::from(start.den) * u128::from(back.num);
    let drift = (i128::from(home.ticks) - i128::from(ticks)).abs();
    let bound = (num + den) / (2 * num);
    if drift > i128::try_from(bound).unwrap_or(i128::MAX) {
        return Err(format!(
            "{context}: came home {drift} ticks away ({}) — beyond the two-rounding bound {bound}",
            home.ticks
        ));
    }
    Ok(())
}

fn check_monotone(
    lesser: i64,
    greater: i64,
    from_raw: (u32, u32),
    to_raw: (u32, u32),
) -> Result<(), String> {
    if from_raw.0 == 0 || from_raw.1 == 0 || to_raw.0 == 0 || to_raw.1 == 0 {
        return Ok(());
    }
    let from = build_base(from_raw);
    let to = build_base(to_raw);
    let Some(low) = Timestamp::new(lesser, from).convert(&to).ok() else {
        return Ok(());
    };
    let Some(high) = Timestamp::new(greater, from).convert(&to).ok() else {
        return Ok(());
    };
    if low.ticks > high.ticks {
        return Err(format!(
            "{lesser} ≤ {greater} but {low} > {high} converting {from} → {to} \
             (rounding must be monotone)"
        ));
    }
    // Sign preservation: a negative input converts to a negative or
    // zero output (and vice versa) — wrap-around is impossible.
    if (lesser < 0 && low.ticks > 0) || (lesser > 0 && low.ticks < 0) {
        return Err(format!(
            "{lesser} converted to {low} under {from} → {to} — sign flipped (wrap?)"
        ));
    }
    Ok(())
}

fn run(seed: u64, ops: &[TimeOp]) -> Result<(), String> {
    let canonical = ops.is_empty();
    let mut rng = XorShift::new(seed);
    let ops = if canonical {
        generate_ops(&mut rng, 512)
    } else {
        ops.to_vec()
    };
    for (index, op) in ops.iter().enumerate() {
        let outcome = match *op {
            TimeOp::Convert { ticks, from, to } => check_convert(ticks, from, to),
            TimeOp::Nanos { ticks, base } => check_nanos(ticks, base),
            TimeOp::ScalerAgreement { ticks, from, to } => check_scaler(ticks, from, to),
            TimeOp::RoundTrip { ticks, via, back } => check_roundtrip(ticks, via, back),
            TimeOp::Monotone {
                lesser,
                greater,
                from,
                to,
            } => check_monotone(lesser, greater, from, to),
        };
        if let Err(failure) = outcome {
            return Err(format!("seed {seed} op {index}: {failure}\n[op] {op:?}"));
        }
    }
    Ok(())
}

/// The full bank over the conversion surfaces: 64 seeds × 512 ops of
/// adversarial (ticks, base, base) triples, every answer checked
/// against the independent u128 oracle — typed errors or exact
/// values, never a panic, never a wrap.
#[test]
fn conversions_answer_typed_errors_or_exact_values_never_panics() {
    for seed in seeds() {
        let mut rng = XorShift::new(seed);
        let ops = generate_ops(&mut rng, 512);
        if let Err(failure) = run_case(seed, &ops, run) {
            unreachable!("{failure}");
        }
    }
}

// ---------------------------------------------------------------------------
// Construction and stamp discipline (the typed, never-panic laws that
// frame the arithmetic)
// ---------------------------------------------------------------------------

/// Zero components refuse typed at construction, and normalization
/// makes equal rates compare equal (the `2/180000` ≡ `1/90000` law).
#[test]
fn base_construction_is_typed_and_normalizing() {
    let mut rng = XorShift::new(0x5EED_5EED_5EED_5EED);
    for _ in 0..512 {
        let num = rng.u32();
        let den = rng.u32();
        assert!(
            TimeBase::new(num, den).is_ok(),
            "nonzero pair ({num}, {den}) must construct"
        );
    }
    for bad in [(0u32, 1u32), (1, 0), (0, 0)] {
        let error = match TimeBase::new(bad.0, bad.1) {
            Ok(base) => unreachable!("({bad:?}) constructed: {base}"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("zero"),
            "zero component must refuse typed: {error}"
        );
    }
    // Scaling both components by the same factor is invisible.
    let a = TimeBase::new(1, 90000).must();
    let b = TimeBase::new(2, 180000).must();
    let c = TimeBase::new(90, 8_100_000).must();
    assert_eq!(a, b);
    assert_eq!(a, c);
    // Rate follows the same laws (rates and bases are one vocabulary).
    assert_eq!(Rate::new(1, 2).must(), Rate::new(3, 6).must());
    assert!(Rate::new(0, 1).is_err() && Rate::new(1, 0).is_err());
}

/// Absence is a type, not a sentinel: a missing stamp answers the
/// typed [`TimeError::Missing`], a present one unwraps — both ends of
/// the fully-unknown span included.
#[test]
fn stamp_absence_is_typed_never_fabricated() {
    let present = Stamp::Present(Timestamp::new(42, TimeBase::new(1, 90000).must()));
    assert_eq!(
        present.timestamp().must().ticks,
        42,
        "a present stamp unwraps its own value"
    );
    assert_eq!(
        Stamp::Missing.timestamp(),
        Err(TimeError::Missing),
        "a missing stamp is the typed Missing error"
    );
    assert_eq!(present.map_timestamp(|ts| ts.ticks), Some(42));
    assert_eq!(Stamp::Missing.map_timestamp(|ts| ts.ticks), None);
    assert_eq!(TimeSpan::UNKNOWN.start.timestamp(), Err(TimeError::Missing));
    assert_eq!(TimeSpan::UNKNOWN.end.timestamp(), Err(TimeError::Missing));
}

/// The negative-DTS-offset corner as pure arithmetic: a DTS strictly
/// below its PTS (and strictly below zero) still converts exactly —
/// offsets are data, dipping into negative ticks is legal, and only
/// representability decides the answer.
#[test]
fn negative_offsets_are_legal_ticks() {
    let mut rng = XorShift::new(0x0D75_0FF5);
    let base = TimeBase::new(1, 90000).must();
    for _ in 0..256 {
        let pts = rng.below(1 << 40) as i64;
        let offset = rng.below(1 << 32) as i64 + 1;
        // pts − dts can dip below zero (the negative-DTS-offset shape).
        let ticks = pts - offset;
        let oracle = oracle_nanos(ticks, (1, 90000));
        match Timestamp::new(ticks, base).as_nanos() {
            Ok(Nanoseconds(nanos)) => {
                let Some(expected) = oracle else {
                    unreachable!("oracle disagreement on {ticks} ticks");
                };
                assert_eq!(nanos, expected, "{ticks} ticks of {base}");
            }
            Err(TimeError::Overflow { .. }) => {
                assert!(
                    oracle.is_none(),
                    "{ticks} ticks of {base} must be Ok({oracle:?})"
                );
            }
            Err(other) => unreachable!("impossible variant {other}"),
        }
    }
}
