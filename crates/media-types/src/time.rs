//! Timing types (doc 01 §3.2).
//!
//! The core rule: **a timestamp value and its absence are different
//! types, and conversion failure is data, not a panic** (MP §8). All
//! arithmetic is checked; overflow returns [`TimeError::Overflow`] with
//! full context, never wrap-around.
//!
//! ## Hot-path conversions (M18)
//!
//! `Timestamp::convert` and `Timestamp::as_nanos` run per packet and
//! per frame on every pipeline path (demux stamping, playback
//! pipeline, video scheduler, clock mapping), and the original
//! implementation paid an `i128` software division **and** modulo —
//! two `__divti3`-class library calls, since mainstream targets have
//! no 128÷64 hardware divide — for every single stamp. M18 adds the
//! classic division-by-invariant-multiplication fast path
//! (Granlund–Montgomery, PLDI '94) in two tiers:
//!
//! * the default methods reduce the conversion ratio to `n/d` (cheap
//!   2/5-smooth partial reduction — shifts and constant-divisor magic
//!   multiplies, no division instruction) and take **one** `u64`
//!   hardware division inside the proven domain `|ticks|·n < 2^63`;
//! * [`Scaler`] precomputes the ratio's reciprocal `m = ceil(2^k/d)`
//!   once per base pair, so a whole run of stamps converts by
//!   multiply+shift alone — zero divisions (D-2).
//!
//! Both tiers are **bit-identical** to the `i128` reference
//! ([`convert_reference`]/[`as_nanos_reference`] — the pre-M18 bodies,
//! kept verbatim as the oracle and fallback): ties away from zero,
//! checked overflow, identical error payloads. Outside the proven
//! domain they fall back to the reference (D-2/D-6: correctness
//! outranks speed). Equivalence is enforced by the `fast_paths_*` /
//! `scaler_*` proof tests.

use crate::error::{EngineError, ErrorContext, ErrorKind, PipelineStage};
use crate::ids::GenerationId;

/// Ticks per second = `num / den` (e.g. `1/90000` for MPEG timing).
///
/// Constructed through [`TimeBase::new`], which rejects zero components
/// and normalizes to lowest terms, so two bases describing the same rate
/// compare equal (MPEG's `1/90000` and a demuxer's `2/180000` unify).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeBase {
    pub num: u32,
    pub den: u32,
}

impl TimeBase {
    /// A valid, reduced time base.
    ///
    /// # Errors
    /// [`TimeError::Missing`] is *not* used for invalid construction (a
    /// missing timestamp is runtime data, not a constructor bug);
    /// construction errors are [`EngineError`]s with kind
    /// [`Unsupported`](ErrorKind::Unsupported) and stage `Demux` when
    /// either component is zero.
    pub fn new(num: u32, den: u32) -> Result<TimeBase, EngineError> {
        if num == 0 || den == 0 {
            return Err(EngineError::new(
                PipelineStage::Demux,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("timebase.zero_component"),
            ));
        }
        let g = gcd(num, den);
        Ok(TimeBase {
            num: num / g,
            den: den / g,
        })
    }

    /// The ticks-per-second rate as an exact rational `(num, den)`,
    /// already reduced.
    pub fn as_parts(self) -> (u32, u32) {
        (self.num, self.den)
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl std::fmt::Display for TimeBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

impl std::str::FromStr for TimeBase {
    type Err = EngineError;

    /// Parse `"num/den"` (e.g. `"1/90000"`). Enables the serialization
    /// round-trip required by doc 01 §2.2(e) without pulling serde into
    /// this std-only crate `[PROPOSAL]`.
    fn from_str(s: &str) -> Result<TimeBase, EngineError> {
        let bad = || {
            EngineError::new(
                PipelineStage::Demux,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("timebase.parse"),
            )
        };
        let (num, den) = s.split_once('/').ok_or_else(bad)?;
        let num: u32 = num.trim().parse().map_err(|_| bad())?;
        let den: u32 = den.trim().parse().map_err(|_| bad())?;
        TimeBase::new(num, den)
    }
}

/// Signed ticks in a [`TimeBase`]. Negative ticks exist in the wild
/// `[VERIFY: which containers — doc 04 evidence]`, so ticks are `i64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timestamp {
    pub ticks: i64,
    pub base: TimeBase,
}

impl Timestamp {
    /// Construct a timestamp (the base is validated by construction).
    ///
    /// # Errors
    /// Propagates [`TimeBase::new`]'s error for zero components.
    pub fn new(ticks: i64, base: TimeBase) -> Timestamp {
        Timestamp { ticks, base }
    }

    /// Build from parts, validating the base.
    ///
    /// # Errors
    /// See [`TimeBase::new`].
    pub fn from_parts(ticks: i64, num: u32, den: u32) -> Result<Timestamp, EngineError> {
        Ok(Timestamp::new(ticks, TimeBase::new(num, den)?))
    }

    /// Checked conversion to another base. Overflow is an explicit error,
    /// never wrap-around; inexact ratios round to the nearest tick (ties
    /// away from zero) — the demuxer-standard rescale.
    ///
    /// Hot path (M18): inside the proven domain `|ticks|·n < 2^63` the
    /// value is converted through a reduced `u64` rational with a single
    /// hardware division, bit-identical to the `i128` reference (module
    /// docs; `convert_fast_path_is_bit_identical_to_reference` proves
    /// it). Everything else — values beyond that bound, degenerate
    /// struct-literal bases — falls back to the reference. Batch loops
    /// converting many stamps over one base pair should hold a
    /// [`Scaler`] instead (multiply+shift, no division at all).
    ///
    /// # Errors
    /// [`TimeError::Overflow`] when the converted tick count cannot be
    /// represented as `i64` (the reference's intermediate math is `i128`).
    pub fn convert(&self, to: &TimeBase) -> Result<Timestamp, TimeError> {
        if let Some(ticks) = try_fast_convert(self.ticks, self.base, *to) {
            return Ok(Timestamp::new(ticks, *to));
        }
        convert_reference(self.ticks, self.base, *to)
    }

    /// Checked conversion to nanoseconds.
    ///
    /// Hot path (M18): as [`Timestamp::convert`] — one `u64` division
    /// inside the proven domain, the `i128` reference outside it; see
    /// [`Scaler`] for the zero-division batch form.
    ///
    /// # Errors
    /// [`TimeError::Overflow`] when the value is negative or exceeds
    /// `u64` nanoseconds (≈ 584 years — generous, but streams are
    /// adversarial).
    pub fn as_nanos(&self) -> Result<Nanoseconds, TimeError> {
        if let Some(nanos) = try_fast_nanos(self.ticks, self.base) {
            return Ok(Nanoseconds(nanos));
        }
        as_nanos_reference(self.ticks, self.base)
    }
}

/// The `1/1_000_000_000` base used by [`Nanoseconds`] (internal only).
const NANOS_BASE: TimeBase = TimeBase {
    num: 1,
    den: 1_000_000_000,
};

/// Divide with rounding (ties away from zero). `None` when the result
/// cannot fit `i128` after rounding, or when the denominator is ≤ 0.
/// The latter cannot come from a `TimeBase::new` value, but struct
/// literals bypass the constructor (sibling crates hold `TimeBase { .. }`
/// consts), so a degenerate denominator is real input — mapped to
/// `None`, the callers' typed-overflow, never a panic (M18 regression:
/// `degenerate_literal_bases_still_match_the_reference`).
fn div_round_i128(numer: i128, denom: i128) -> Option<i128> {
    if denom <= 0 {
        return None;
    }
    let (q, r) = (numer / denom, numer % denom);
    let twice_r = r.checked_mul(2)?;
    let adjusted = if twice_r >= denom {
        q.checked_add(1)?
    } else if twice_r <= -denom {
        q.checked_sub(1)?
    } else {
        q
    };
    Some(adjusted)
}

// ---- M18: division-by-invariant-multiplication fast paths ------------------
//
// The two conversion surfaces above run per packet and per frame on
// every engine path. Their pre-M18 bodies — kept verbatim as
// `convert_reference` / `as_nanos_reference` below — cost one `i128`
// software division plus one `i128` software modulo per call
// (`__divti3` + `__modti3`; mainstream targets have no 128÷64 hardware
// divide). This section replaces that with u64 arithmetic, in two
// tiers:
//
//   Tier 2 (default methods): reduce the conversion ratio to n/d
//   ([`reduce_smooth`]), then ONE hardware u64 division for any
//   |ticks|·n < 2^63 ([`div_round_u64`]).
//
//   Tier 1 (`Scaler`): the ratio's reciprocal m = ceil(2^k/d)
//   ([`magic_u64`]) precomputed once per base pair, then multiply+shift
//   only — zero division instructions (D-2 mechanical sympathy).
//
// Bit-exactness contract: both tiers reproduce the reference —
// ties-away-from-zero rounding, checked overflow, identical error
// payloads — for every input they accept; everything else falls back.
// The contract is enforced by the `fast_paths_*` / `scaler_*` proof
// tests at the bottom of this file.

/// The fast paths' proven dividend bound: `|ticks| × n < 2^63`.
///
/// Three properties hinge on this exact constant:
///
/// 1. the scaled magnitude `p` fits `u64` (Tier 2's single division);
/// 2. `p·m < 2^128` for every magic `m` [`magic_u64`] emits, so
///    Tier 1's product cannot overflow `u128`;
/// 3. the rounded quotient `q ≤ p < 2^63` fits `i64` (and `u64`), so
///    no in-domain input can produce an overflow error — the fast path
///    only ever returns `Ok`, exactly like the reference would.
const FAST_DOMAIN_LIMIT: u128 = 1u128 << 63;

/// Euclidean GCD — construction-time only ([`Scaler`] setup). The
/// per-call paths use the cheaper [`reduce_smooth`] instead.
fn gcd64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// 5^16, 5^8, 5^4, 5^2, 5^1 — binary-lifting table for
/// [`reduce_smooth`]: any exponent ≤ 31 composes from these steps, and
/// the lifting loop is bounded at five constant-divisor checks however
/// 5-smooth the operands are (versus up to 27 for a naive loop).
const FIVE_POW: [u64; 5] = [152_587_890_625, 390_625, 625, 25, 5];

/// Divide `n` and `d` by their common 2- and 5-powers.
///
/// The rational `n/d` is preserved **exactly** (only common factors are
/// removed), so this never changes a conversion's value — it only
/// widens the fast domain. 2s and 5s are targeted because the
/// nanosecond constant `10^9 = 2^9·5^9` makes 2/5-smooth ratios the
/// overwhelmingly common case (ns/µs/ms bases, 90 kHz after
/// cross-reduction, …); odd common factors are left in and merely
/// narrow the domain. Costs shifts and constant-divisor magic
/// multiplies (`% 5^k` compiles to `mulhi`+`shift`) — no division
/// instruction (D-2).
fn reduce_smooth(n: &mut u64, d: &mut u64) {
    debug_assert!(*n >= 1 && *d >= 1);
    let s = n.trailing_zeros().min(d.trailing_zeros());
    *n >>= s;
    *d >>= s;
    for f in FIVE_POW {
        if (*n).is_multiple_of(f) && (*d).is_multiple_of(f) {
            *n /= f;
            *d /= f;
        }
    }
}

/// The Granlund–Montgomery magic pair for divisor `d ≥ 1`
/// ("Division by Invariant Integers using Multiplication", PLDI 1994):
/// `k = 127 − leading_zeros(d)` and `m = ceil(2^k / d)`, with two
/// proven properties (enforced exhaustively-style by
/// `mulshift_magic_is_bit_exact_for_small_divisors`):
///
/// * **exactness**: `floor(p / d) == (p·m) >> k` for every
///   `p < 2^63`. Proof: write `m·d = 2^k + e` with `0 ≤ e < d`. Then
///   `p·m/2^k = p/d + p·e/(d·2^k) < p/d + p/2^k ≤ p/d + 1/d`, and
///   `frac(p/d) ≤ (d−1)/d`; the fractional parts sum below 1, so the
///   floor cannot tip over.
/// * **bounds**: `m ≤ 2^64 + 1` (since `d ≥ 2^(63 − lz)`), and
///   therefore `p·m < 2^63·(2^64 + 1) < 2^128` — the Tier-1 product
///   always fits `u128`.
fn magic_u64(d: u64) -> (u128, u32) {
    debug_assert!(d >= 1);
    let k = 127 - d.leading_zeros();
    let m = (1u128 << k).div_ceil(u128::from(d));
    (m, k)
}

/// `round(p / d)` (half away from zero on the magnitude — callers
/// re-apply the sign) via the single hardware division of the Tier-2
/// path. `p < 2^63`, `d ≥ 1` (debug-checked).
fn div_round_u64(p: u64, d: u64) -> u64 {
    debug_assert!(u128::from(p) < FAST_DOMAIN_LIMIT && d >= 1);
    let (q, r) = (p / d, p % d);
    // 2r ≥ d without overflow: r < d, so d − r ≥ 1.
    q + u64::from(r >= d - r)
}

/// `round(p / d)` via multiply+shift (Tier 1): the exact floor from
/// [`magic_u64`], the remainder recovered by one multiply-subtract,
/// half-away rounding applied to it. No division instruction anywhere.
fn div_round_mulshift(p: u64, m: u128, k: u32, d: u64) -> u64 {
    debug_assert!(u128::from(p) < FAST_DOMAIN_LIMIT && d >= 1);
    let q = ((u128::from(p) * m) >> k) as u64; // exact floor(p / d)
    let r = (u128::from(p) - u128::from(q) * u128::from(d)) as u64; // p mod d
    q + u64::from(r >= d - r)
}

/// Tier-2 fast path for [`Timestamp::convert`]: `Some(converted ticks)`
/// iff the input lies inside the proven-exact `u64` domain; `None`
/// means the caller must use [`convert_reference`].
fn try_fast_convert(ticks: i64, from: TimeBase, to: TimeBase) -> Option<i64> {
    // Identity is exact for every representable input (x·num·den over
    // den·num is x with zero remainder) — including the i64::MIN corner
    // the general domain check below must refuse. Degenerate (zero
    // component) bases are the reference's business: it reports them as
    // Overflow, the pre-M18 behavior for struct-literal bases.
    if from == to {
        if from.num != 0 && from.den != 0 {
            return Some(ticks);
        }
        return None;
    }
    if from.num == 0 || from.den == 0 || to.num == 0 || to.den == 0 {
        return None;
    }
    let mut n = u64::from(from.num) * u64::from(to.den);
    let mut d = u64::from(from.den) * u64::from(to.num);
    reduce_smooth(&mut n, &mut d);
    let p = u128::from(ticks.unsigned_abs()) * u128::from(n);
    if p >= FAST_DOMAIN_LIMIT {
        return None;
    }
    let mag = div_round_u64(p as u64, d);
    // `mag ≤ p < 2^63`, so the narrowing is inside the proven domain.
    Some(if ticks < 0 {
        -(mag as i64)
    } else {
        mag as i64
    })
}

/// Tier-2 fast path for [`Timestamp::as_nanos`] — same contract as
/// [`try_fast_convert`]. Negative ticks stay on the reference: their
/// result is the typed `Overflow` error, and keeping every error
/// decision in one place keeps the paths provably identical.
fn try_fast_nanos(ticks: i64, base: TimeBase) -> Option<u64> {
    if ticks < 0 || base.num == 0 || base.den == 0 {
        return None;
    }
    let mut n = u64::from(base.num) * 1_000_000_000;
    let mut d = u64::from(base.den);
    reduce_smooth(&mut n, &mut d);
    let p = (ticks as u128) * u128::from(n); // ticks ≥ 0 here, so the cast is lossless
    if p >= FAST_DOMAIN_LIMIT {
        return None;
    }
    Some(div_round_u64(p as u64, d))
}

/// The pre-M18 `Timestamp::convert` body, verbatim — the bit-exactness
/// oracle for the fast paths, their fallback outside the proven domain,
/// and the "before" leg of the M18 benchmark. DO NOT "optimize".
fn convert_reference(ticks: i64, from: TimeBase, to: TimeBase) -> Result<Timestamp, TimeError> {
    // value_seconds = ticks * from.num / from.den
    // ticks_to       = value_seconds * to.den / to.num
    //               = ticks * from.num * to.den / (from.den * to.num)
    let numer = i128::from(ticks) * i128::from(from.num) * i128::from(to.den);
    let denom = i128::from(from.den) * i128::from(to.num);
    let converted = match div_round_i128(numer, denom) {
        Some(v) => v,
        None => return Err(TimeError::Overflow { from, to, ticks }),
    };
    match i64::try_from(converted) {
        Ok(out) => Ok(Timestamp::new(out, to)),
        Err(_) => Err(TimeError::Overflow { from, to, ticks }),
    }
}

/// The pre-M18 `Timestamp::as_nanos` body, verbatim — same role as
/// [`convert_reference`].
fn as_nanos_reference(ticks: i64, base: TimeBase) -> Result<Nanoseconds, TimeError> {
    let nanos = match div_round_i128(
        i128::from(ticks) * i128::from(base.num) * 1_000_000_000i128,
        i128::from(base.den),
    ) {
        Some(v) => v,
        None => return Err(TimeError::Overflow {
            from: base,
            to: NANOS_BASE,
            ticks,
        }),
    };
    match u64::try_from(nanos) {
        Ok(v) => Ok(Nanoseconds(v)),
        Err(_) => Err(TimeError::Overflow {
            from: base,
            to: NANOS_BASE,
            ticks,
        }),
    }
}

/// A precomputed conversion direction: the Granlund–Montgomery
/// reciprocal of one `(from, to)` base pair, built once and applied to
/// a whole run of timestamps.
///
/// This is the once-per-TimeBase-pair cache the M18 card prescribed,
/// in the one shape that does not bloat [`TimeBase`] itself: the base
/// stays 8 bytes because sibling crates construct it by struct literal
/// (`media-clock`'s pub `NANO_BASE` const) and because it is embedded
/// in every [`Timestamp`]/[`Stamp`] — a ~40-byte reciprocal inline
/// would tax every packet and frame header (D-1/D-2 mechanical
/// sympathy). Hot loops (video scheduler, playback pipeline) hold one
/// `Scaler` next to their base pair instead and convert by
/// multiply+shift alone — zero division instructions.
///
/// Equivalence to the `i128` reference is enforced by test for every
/// value the fast domain admits; outside it, applies fall back to the
/// reference (correctness outranks speed, D-2/D-6).
#[derive(Debug, Clone, Copy)]
pub struct Scaler {
    from: TimeBase,
    to: TimeBase,
    /// Reduced ratio: `target_ticks = round(ticks × n / d)`.
    n: u64,
    /// The reduced divisor — also the fallback sentinel: `0` marks a
    /// degenerate base pair whose every apply uses the reference.
    d: u64,
    /// `ceil(2^k / d)` — see [`magic_u64`].
    m: u128,
    /// The reciprocal's shift.
    k: u32,
}

impl TimeBase {
    /// Precompute the `self → to` conversion direction — the batch
    /// hot-loop form of [`Timestamp::convert`].
    ///
    /// Construction performs the pair's full gcd reduction and one
    /// `u128` division: cheap enough once per stream or pipeline
    /// stage, and afterwards every apply is multiply+shift only.
    #[must_use = "a scaler nobody applies converts nothing"]
    pub fn scaler_to(&self, to: &TimeBase) -> Scaler {
        let mut n = u64::from(self.num) * u64::from(to.den);
        let mut d = u64::from(self.den) * u64::from(to.num);
        if n == 0 || d == 0 {
            // Degenerate struct-literal bases: every apply defers to the
            // reference, which reports Overflow / Ok(0) exactly as the
            // raw conversions always did.
            return Scaler {
                from: *self,
                to: *to,
                n: 0,
                d: 0,
                m: 0,
                k: 0,
            };
        }
        let g = gcd64(n, d);
        n /= g;
        d /= g;
        let (m, k) = magic_u64(d);
        Scaler {
            from: *self,
            to: *to,
            n,
            d,
            m,
            k,
        }
    }

    /// Precompute the `self → nanoseconds` direction — the batch
    /// hot-loop form of [`Timestamp::as_nanos`].
    #[must_use = "a scaler nobody applies converts nothing"]
    pub fn nanos_scaler(&self) -> Scaler {
        self.scaler_to(&NANOS_BASE)
    }
}

impl Scaler {
    /// The base this scaler converts from.
    #[must_use]
    pub fn from_base(&self) -> TimeBase {
        self.from
    }

    /// The base this scaler converts to.
    #[must_use]
    pub fn target_base(&self) -> TimeBase {
        self.to
    }

    /// Whether `ticks` lies inside the proven multiply+shift domain
    /// (`|ticks| × n < 2^63`, non-degenerate pair).
    fn in_fast_domain(&self, ticks: i64) -> bool {
        self.d != 0 && u128::from(ticks.unsigned_abs()) * u128::from(self.n) < FAST_DOMAIN_LIMIT
    }

    /// Convert `ticks` (counted in [`Scaler::from_base`]) into the
    /// target base — the batch hot-loop form of [`Timestamp::convert`].
    ///
    /// # Errors
    /// [`TimeError::Overflow`] exactly as [`Timestamp::convert`] —
    /// only reachable outside the fast domain, via the reference
    /// fallback.
    pub fn convert_ticks(&self, ticks: i64) -> Result<Timestamp, TimeError> {
        if self.in_fast_domain(ticks) {
            // p = |ticks|·n < 2^63 by the domain check, so the u64
            // multiply cannot overflow and fits the mulshift contract.
            let p = ticks.unsigned_abs() * self.n;
            let mag = div_round_mulshift(p, self.m, self.k, self.d);
            // `mag ≤ p < 2^63`, so the narrowing is inside the proven
            // domain.
            let out = if ticks < 0 {
                -(mag as i64)
            } else {
                mag as i64
            };
            return Ok(Timestamp::new(out, self.to));
        }
        convert_reference(ticks, self.from, self.to)
    }

    /// The nanoseconds value of `ticks` (counted in
    /// [`Scaler::from_base`]) — the batch hot-loop form of
    /// [`Timestamp::as_nanos`], fast for scalers built by
    /// [`TimeBase::nanos_scaler`] (for an arbitrary scaler this is the
    /// from-base nanosecond value, via the reference).
    ///
    /// # Errors
    /// [`TimeError::Overflow`] exactly as [`Timestamp::as_nanos`]
    /// (negative ticks, or values beyond `u64` nanoseconds — the
    /// latter only outside the fast domain).
    pub fn nanos(&self, ticks: i64) -> Result<Nanoseconds, TimeError> {
        if ticks >= 0 && self.to == NANOS_BASE && self.in_fast_domain(ticks) {
            // p = ticks·n < 2^63 by the domain check (see convert_ticks).
            let p = ticks.unsigned_abs() * self.n;
            return Ok(Nanoseconds(div_round_mulshift(p, self.m, self.k, self.d)));
        }
        as_nanos_reference(ticks, self.from)
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Ticks + base — the unambiguous, round-trippable rendering
        // (seconds renderings can be lossy and invite parse-back bugs).
        write!(f, "{}@{}", self.ticks, self.base)
    }
}

/// Time-domain errors (doc 01 §3.2). Exactly the three spec variants —
/// overflow, discontinuity, missing — each carrying full context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeError {
    /// A conversion that cannot be represented. The caller decides:
    /// rescale via nanoseconds, clamp with an event, or fail.
    Overflow {
        from: TimeBase,
        to: TimeBase,
        ticks: i64,
    },
    /// Arithmetic across a discontinuity without an explicit clock reset.
    /// The reset is a generation-scoped operation (doc 01 §3.2).
    Discontinuity {
        expected: Option<Timestamp>,
        found: Timestamp,
        generation: GenerationId,
    },
    /// A required timestamp was absent. Propagates; the presentation
    /// scheduler treats it as "infer from previous frame + duration,
    /// else hold" — the inference is an observable event, never silent
    /// invention (MP §8: never fabricate).
    Missing,
}

impl std::fmt::Display for TimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeError::Overflow { from, to, ticks } => {
                write!(
                    f,
                    "time conversion overflow: {ticks} ticks of {from} cannot be represented in {to}"
                )
            }
            TimeError::Discontinuity {
                expected,
                found,
                generation,
            } => match expected {
                Some(expected) => write!(
                    f,
                    "time discontinuity: expected {expected}, found {found} (generation {})",
                    generation.counter()
                ),
                None => write!(
                    f,
                    "time discontinuity: no expected timestamp, found {found} (generation {})",
                    generation.counter()
                ),
            },
            TimeError::Missing => f.write_str("timestamp missing"),
        }
    }
}

impl std::error::Error for TimeError {}

impl From<TimeError> for EngineError {
    /// Time errors become stage-tagged engine errors at the `Clock`
    /// stage (the clock owns conversion policy; demuxer/decoder errors
    /// that carry a `TimeError` source route through their own stages).
    fn from(err: TimeError) -> EngineError {
        let code = match &err {
            TimeError::Overflow { .. } => "time.overflow",
            TimeError::Discontinuity { .. } => "time.discontinuity",
            TimeError::Missing => "time.missing",
        };
        EngineError::new(
            PipelineStage::Clock,
            ErrorKind::Corrupt {
                detail: crate::error::CorruptDetail::Timing,
            },
            ErrorContext::new(code),
        )
        .with_source(err)
    }
}

/// Packet/frame stamping: present or absent — no `-1` sentinel (MP §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stamp {
    Present(Timestamp),
    Missing,
}

impl Stamp {
    /// Unwrap the timestamp, mapping absence to [`TimeError::Missing`].
    ///
    /// # Errors
    /// [`TimeError::Missing`] when the stamp is absent.
    pub fn timestamp(&self) -> Result<Timestamp, TimeError> {
        match self {
            Stamp::Present(ts) => Ok(*ts),
            Stamp::Missing => Err(TimeError::Missing),
        }
    }

    /// `Some(true/false)` when present, `None` when missing — for
    /// comparison-style call sites that treat absence as "unknown".
    pub fn map_timestamp<U>(&self, f: impl FnOnce(Timestamp) -> U) -> Option<U> {
        match self {
            Stamp::Present(ts) => Some(f(*ts)),
            Stamp::Missing => None,
        }
    }
}

/// A start/end span in stamps. Either end may be missing (live streams).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeSpan {
    pub start: Stamp,
    pub end: Stamp,
}

impl TimeSpan {
    /// A span with both ends missing (fully unknown extent).
    pub const UNKNOWN: TimeSpan = TimeSpan {
        start: Stamp::Missing,
        end: Stamp::Missing,
    };
}

/// Track timing summary reported with every [`crate::ids::Track`]:
/// span, time base, known gaps (doc 01 §3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackTiming {
    /// The track's extent (either end may be `Missing`).
    pub span: TimeSpan,
    /// The base all track timestamps share.
    pub base: TimeBase,
    /// Container-declared gaps (edits, splices) in track time.
    pub gaps: Vec<TimeSpan>,
}

/// Nanoseconds — the engine's interchange time unit for wall-clock-style
/// durations (doc 01 §3.1). Newtype, not a bare `u64`, so a duration can
/// never be confused with a byte count or a tick count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Nanoseconds(pub u64);

impl Nanoseconds {
    /// Exact seconds when the value divides evenly, else `None`.
    pub fn as_secs_exact(self) -> Option<u64> {
        self.0
            .is_multiple_of(1_000_000_000)
            .then_some(self.0 / 1_000_000_000)
    }
}

/// Unknown durations are common in streams — an explicit variant, never
/// `0` and never `-1` (MP §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Duration {
    Known(Nanoseconds),
    Unknown,
}

impl Duration {
    /// Unwrap known durations; `Duration::Unknown` maps to
    /// [`TimeError::Missing`].
    ///
    /// # Errors
    /// [`TimeError::Missing`] when unknown.
    pub fn nanos(&self) -> Result<Nanoseconds, TimeError> {
        match self {
            Duration::Known(n) => Ok(*n),
            Duration::Unknown => Err(TimeError::Missing),
        }
    }

    /// Format as `H:MM:SS.mmm` for display; `Unknown` renders as `"--:--"`
    /// (a deliberately non-numeric string — it can never be parsed back
    /// into a number, so no sentinel-value confusion is possible).
    pub fn display(&self) -> String {
        match self {
            Duration::Unknown => "--:--".to_string(),
            Duration::Known(n) => {
                let total_ms = n.0 / 1_000_000;
                let (h, m, s, ms) = (
                    total_ms / 3_600_000,
                    (total_ms / 60_000) % 60,
                    (total_ms / 1_000) % 60,
                    total_ms % 1_000,
                );
                format!("{h}:{m:02}:{s:02}.{ms:03}")
            }
        }
    }
}

/// Rational playback rate. `1×` = `1/1`; `1.25×` = `5/4` — exact, no
/// float drift (doc 01 §3.1).
///
/// Negative rates (reverse playback) are out of v1 scope `[PROPOSAL]`;
/// the feature matrix tracks the gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rate {
    pub num: u32,
    pub den: u32,
}

impl Rate {
    /// Normal (forward) playback.
    pub const ONE: Rate = Rate { num: 1, den: 1 };

    /// A valid, reduced positive rate.
    ///
    /// # Errors
    /// [`EngineError`] with kind
    /// [`Unsupported`](ErrorKind::Unsupported) and stage `Control` when
    /// either component is zero (a zero rate is pause, not a rate).
    pub fn new(num: u32, den: u32) -> Result<Rate, EngineError> {
        if num == 0 || den == 0 {
            return Err(EngineError::new(
                PipelineStage::Control,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("rate.zero_component"),
            ));
        }
        let g = gcd(num, den);
        Ok(Rate {
            num: num / g,
            den: den / g,
        })
    }

    /// The rate as an exact rational `(num, den)`, already reduced.
    pub fn as_parts(self) -> (u32, u32) {
        (self.num, self.den)
    }
}

impl std::fmt::Display for Rate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

impl std::str::FromStr for Rate {
    type Err = EngineError;

    /// Parse `"num/den"` (e.g. `"5/4"`); pairs with `Display` for the
    /// doc 01 §2.2(e) serialization round-trip.
    fn from_str(s: &str) -> Result<Rate, EngineError> {
        let bad = || {
            EngineError::new(
                PipelineStage::Control,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("rate.parse"),
            )
        };
        let (num, den) = s.split_once('/').ok_or_else(bad)?;
        let num: u32 = num.trim().parse().map_err(|_| bad())?;
        let den: u32 = den.trim().parse().map_err(|_| bad())?;
        Rate::new(num, den)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A tiny deterministic LCG — "property tests" without a dependency
    // (std-only crate, doc 01 §2.2(c)); every case is reproducible from
    // the seed printed in the failure message.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0 >> 11
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    // ---- TimeBase -----------------------------------------------------------

    #[test]
    fn timebase_rejects_zero_components() {
        assert!(TimeBase::new(0, 90000).is_err());
        assert!(TimeBase::new(1, 0).is_err());
    }

    #[test]
    fn timebase_reduces_to_lowest_terms() {
        let tb = TimeBase::new(2, 180_000).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            tb,
            TimeBase::new(1, 90_000).unwrap_or_else(|e| unreachable!("{e}"))
        );
        assert_eq!(tb.to_string(), "1/90000");
    }

    #[test]
    fn timebase_display_parse_roundtrip() {
        for (num, den) in [(1u32, 90_000u32), (1, 1000), (1001, 30000), (3, 7)] {
            let tb = TimeBase::new(num, den).unwrap_or_else(|e| unreachable!("{e}"));
            let s = tb.to_string();
            let back: TimeBase = s.parse().unwrap_or_else(|e| unreachable!("{e}"));
            assert_eq!(tb, back, "roundtrip failed for {s}");
        }
    }

    // ---- Timestamp::convert -------------------------------------------------

    #[test]
    fn convert_is_exact_when_bases_agree() {
        let ts = Timestamp::from_parts(1234, 1, 90_000).unwrap_or_else(|e| unreachable!("{e}"));
        let same = ts
            .convert(&TimeBase::new(1, 90_000).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(same.ticks, 1234);
    }

    #[test]
    fn convert_rescales_mpeg_to_milliseconds() {
        // 90000 ticks at 1/90000 = 1 second = 1000 ticks at 1/1000.
        let ts = Timestamp::from_parts(90_000, 1, 90_000).unwrap_or_else(|e| unreachable!("{e}"));
        let ms = ts
            .convert(&TimeBase::new(1, 1000).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(ms.ticks, 1000);
        assert_eq!(
            ms.base,
            TimeBase::new(1, 1000).unwrap_or_else(|e| unreachable!("{e}"))
        );
    }

    #[test]
    fn convert_rounds_ties_away_from_zero() {
        // A true tie: from=1/2, to=1/1, ticks=1 => 1*1*1/2 = 0.5 -> 1.
        let ts = Timestamp::from_parts(1, 1, 2).unwrap_or_else(|e| unreachable!("{e}"));
        let conv = ts
            .convert(&TimeBase::new(1, 1).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(conv.ticks, 1, "0.5 rounds away from zero");

        let neg = Timestamp::from_parts(-1, 1, 2).unwrap_or_else(|e| unreachable!("{e}"));
        let conv = neg
            .convert(&TimeBase::new(1, 1).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(conv.ticks, -1, "-0.5 rounds away from zero");
    }

    #[test]
    fn convert_overflow_is_an_explicit_error() {
        // i64::MAX ticks of 1/90000 into nanoseconds (1/1e9) overflows i64.
        let ts = Timestamp::from_parts(i64::MAX, 1, 90_000).unwrap_or_else(|e| unreachable!("{e}"));
        match ts.convert(&NANOS_BASE) {
            Err(TimeError::Overflow { from, to, ticks }) => {
                assert_eq!(ticks, i64::MAX);
                assert_eq!(
                    from,
                    TimeBase::new(1, 90_000).unwrap_or_else(|e| unreachable!("{e}"))
                );
                assert_eq!(to, NANOS_BASE);
            }
            other => unreachable!("expected overflow, got {other:?}"),
        }
    }

    #[test]
    fn as_nanos_matches_reference_for_common_bases() {
        // Deterministic sweep instead of floats: for a grid of bases and
        // ticks, the i128 reference and as_nanos agree within rounding.
        let mut rng = Lcg(0xC0FFEE);
        for _ in 0..2000 {
            let den = 1 + rng.below(100_000);
            let num = 1 + rng.below(4);
            let base =
                TimeBase::new(num as u32, den as u32).unwrap_or_else(|e| unreachable!("{e}"));
            // Keep the product safely inside u64 nanoseconds:
            // ticks * num * 1e9 <= u64::MAX.
            let max_ticks = u64::MAX / 1_000_000_000 / num * den;
            let ticks = rng.below(max_ticks) as i64;
            let ts = Timestamp::new(ticks, base);
            let nanos = ts.as_nanos().unwrap_or_else(|e| unreachable!("{e}"));
            // reference: exact rational in i128
            let exact =
                i128::from(ticks) * i128::from(base.num) * 1_000_000_000i128 / i128::from(base.den);
            let got = i128::from(nanos.0);
            let tolerance = 1i128; // rounding to nearest: |error| <= 0.5 tick scaled
            assert!(
                (got - exact).abs() <= tolerance,
                "nanos drifted: base {base}, ticks {ticks}, exact {exact}, got {got}"
            );
        }
    }

    #[test]
    fn convert_roundtrip_is_identity_when_exact() {
        let mut rng = Lcg(0xBEEF);
        for _ in 0..2000 {
            let b1 = TimeBase::new(1, 1 + rng.below(90_000) as u32)
                .unwrap_or_else(|e| unreachable!("{e}"));
            let b2 = TimeBase::new(1, 1 + rng.below(1000) as u32)
                .unwrap_or_else(|e| unreachable!("{e}"));
            let ticks = rng.below(1 << 32) as i64;
            let ts = Timestamp::new(ticks, b1);
            if let Ok(mid) = ts.convert(&b2)
                && let Ok(back) = mid.convert(&b1)
            {
                // Each direction rounds to nearest: total absolute
                // error <= 0.5*(a+b) seconds expressed in a-ticks:
                //   err_ticks <= 0.5 * (1 + (a.den*b.num)/(a.num*b.den))
                // plus 2 ticks of slack for the two exact divisions.
                let ratio = (u128::from(b1.den) * u128::from(b2.num))
                    / (u128::from(b1.num) * u128::from(b2.den));
                let allowed = (ratio / 2) as i64 + 2;
                assert!(
                    (back.ticks - ticks).abs() <= allowed,
                    "roundtrip drifted: {ticks} {b1} -> {b2} -> {} {b1} (allowed {allowed})",
                    back.ticks
                );
            }
        }
    }

    #[test]
    fn nanos_overflow_errors_rather_than_wrapping() {
        let ts = Timestamp::from_parts(i64::MAX, 1, 1).unwrap_or_else(|e| unreachable!("{e}"));
        assert!(ts.as_nanos().is_err());
    }

    // ---- Stamp / Duration -----------------------------------------------------

    #[test]
    fn stamp_missing_maps_to_time_error_missing() {
        assert_eq!(Stamp::Missing.timestamp().err(), Some(TimeError::Missing));
        let ts = Timestamp::from_parts(5, 1, 1000).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            Stamp::Present(ts)
                .timestamp()
                .unwrap_or_else(|e| unreachable!("{e}")),
            ts
        );
        assert_eq!(Stamp::Missing.map_timestamp(|t| t.ticks), None);
        assert_eq!(Stamp::Present(ts).map_timestamp(|t| t.ticks), Some(5));
    }

    #[test]
    fn duration_unknown_is_never_zero_or_neg_one() {
        assert_eq!(Duration::Unknown.display(), "--:--");
        assert_eq!(Duration::Unknown.nanos().err(), Some(TimeError::Missing));
        let d = Duration::Known(Nanoseconds(3_723_004_000_000));
        assert_eq!(d.display(), "1:02:03.004");
        assert_eq!(
            d.nanos()
                .unwrap_or_else(|e| unreachable!("{e}"))
                .as_secs_exact(),
            None
        );
        assert_eq!(
            Duration::Known(Nanoseconds(2_000_000_000))
                .nanos()
                .unwrap_or_else(|e| unreachable!("{e}"))
                .as_secs_exact(),
            Some(2)
        );
    }

    // ---- Rate -------------------------------------------------------------------

    #[test]
    fn rate_is_exact_and_reduced() {
        let r = Rate::new(10, 8).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(r.as_parts(), (5, 4)); // 1.25x exact
        assert_eq!(Rate::ONE.as_parts(), (1, 1));
        assert!(Rate::new(0, 1).is_err());
        assert!(Rate::new(1, 0).is_err());
    }

    #[test]
    fn rate_display_parse_roundtrip() {
        for (num, den) in [(5u32, 4u32), (1, 1), (3, 2), (29, 7)] {
            let r = Rate::new(num, den).unwrap_or_else(|e| unreachable!("{e}"));
            let back: Rate = r
                .to_string()
                .parse()
                .unwrap_or_else(|e| unreachable!("{e}"));
            assert_eq!(r, back);
        }
    }

    // ---- TimeError display + EngineError conversion -----------------------------

    #[test]
    fn time_error_displays_with_context() {
        let e = TimeError::Overflow {
            from: TimeBase::new(1, 90_000).unwrap_or_else(|e| unreachable!("{e}")),
            to: TimeBase::new(1, 1).unwrap_or_else(|e| unreachable!("{e}")),
            ticks: -7,
        };
        let msg = e.to_string();
        assert!(msg.contains("overflow"), "{msg}");
        assert!(msg.contains("-7"), "{msg}");
        let engine: EngineError = e.into();
        assert_eq!(engine.stage, PipelineStage::Clock);
        assert_eq!(engine.context.code, "time.overflow");
        assert!(matches!(
            engine.kind,
            ErrorKind::Corrupt {
                detail: crate::error::CorruptDetail::Timing
            }
        ));
    }

    // ---- M18: division-by-invariant-multiplication proofs ---------------
    //
    // The fast tiers must be BIT-IDENTICAL to the i128 reference —
    // same values, same ties-away-from-zero rounding, same error
    // payloads — for every input they accept, and must visibly accept
    // the realistic hot-path inputs (a fast path that silently always
    // falls back would pass a naive equivalence check).

    /// Struct-literal construction on purpose: real callers do this
    /// (media-clock's pub `NANO_BASE` const), so the proofs must cover
    /// shapes `TimeBase::new` would have rejected or reduced.
    fn literal_tb(num: u32, den: u32) -> TimeBase {
        TimeBase { num, den }
    }

    /// Tick probes for a conversion whose reduced multiplier is `n`:
    /// dense small values, strided mid-range, powers of two ±1, the
    /// extremes (`0`, `±1`, `i64::MAX`/`MIN`, `MAX/2` — the i64 analog
    /// of the u64::MAX/2 corner), realistic four-hour stream positions,
    /// and — the valuable part — ±2 straddling the exact fast/reference
    /// domain edge `|t|·n = 2^63`.
    fn probe_ticks(n: u64, full: bool) -> Vec<i64> {
        let mut v: Vec<i64> = Vec::with_capacity(512);
        let dense: i64 = if full { 48 } else { 16 };
        for t in 0..=dense {
            v.push(t);
        }
        let (stride, stop): (i64, i64) = if full {
            (9_973, 1_000_000)
        } else {
            (9_973, 100_000)
        };
        let mut t: i64 = 97;
        while t <= stop {
            v.push(t);
            t += stride;
        }
        let js: &[u32] = if full {
            &[1, 4, 9, 14, 19, 24, 29, 34, 39, 44, 49, 54, 59, 62]
        } else {
            &[7, 17, 31, 47, 62]
        };
        for &j in js {
            let p = (1i64 << j) - 1;
            v.push(p);
            v.push(p + 1);
            v.push(p + 2);
        }
        v.extend([
            i64::MAX,
            i64::MIN,
            i64::MAX / 2,
            i64::MIN / 2,
            4 * 3600 * 90_000, // 4 h of 90 kHz video
            4 * 3600 * 48_000, // 4 h of 48 kHz audio
        ]);
        // The exact domain edge. (2^63 − 1)/n ≤ i64::MAX for n ≥ 1, so
        // the cast cannot truncate.
        let edge: i64 = ((FAST_DOMAIN_LIMIT - 1) / u128::from(n.max(1))) as i64;
        for delta in [0i64, 1, 2] {
            v.push(edge.saturating_add(delta));
            v.push(edge.saturating_sub(delta));
        }
        // Mirror negatives: convert exercises the sign path (ties away
        // from zero), as_nanos errors identically on both paths.
        let positives = v.clone();
        v.extend(positives.into_iter().map(|t| t.saturating_neg()));
        v
    }

    /// The raw Granlund–Montgomery claim — `floor(p/d) == (p·m) >> k`
    /// for every `p < 2^63` — plus the rounded form against both the
    /// u64 division and the i128 reference, for EVERY divisor ≤ 4096
    /// × a dense/strided/boundary dividend sweep.
    #[test]
    fn mulshift_magic_is_bit_exact_for_small_divisors() {
        let mut cases: u64 = 0;
        for d in 1u64..=4096 {
            let (m, k) = magic_u64(d);
            let mut check = |p: u64| {
                let want_floor = p / d;
                let got_floor = ((u128::from(p) * m) >> k) as u64;
                assert_eq!(got_floor, want_floor, "floor diverged: p {p}, d {d}");
                let fast = div_round_mulshift(p, m, k, d);
                let div = div_round_u64(p, d);
                assert_eq!(fast, div, "mulshift vs div diverged: p {p}, d {d}");
                let reference = match div_round_i128(i128::from(p), i128::from(d)) {
                    Some(v) => v,
                    None => unreachable!("reference returned None for d = {d} ≥ 1"),
                };
                assert_eq!(
                    i128::from(fast),
                    reference,
                    "reference diverged: p {p}, d {d}"
                );
                cases += 1;
            };
            for p in 0..=257u64 {
                check(p);
            }
            let mut p = 258u64;
            while p < 1 << 16 {
                check(p);
                p += 991;
            }
            for j in (10..=62u32).step_by(4) {
                let base = 1u64 << j;
                for q in [base - 1, base, base + 1] {
                    check(q);
                }
            }
            // The domain ceiling and d-straddling remainders.
            let top = FAST_DOMAIN_LIMIT as u64 - 1;
            for q in [
                top,
                top - 1,
                top - 2,
                top - d,
                top - d + 1,
                top - d - 1,
                d - 1,
                d,
                d + 1,
                2 * d - 1,
                2 * d,
                2 * d + 1,
            ] {
                check(q);
            }
        }
        assert!(cases > 1_000_000, "suspiciously small proof domain: {cases}");
        println!("mulshift magic proof: {cases} (p, d) cases over ALL d ≤ 4096");
    }

    #[test]
    fn as_nanos_fast_path_is_bit_identical_to_reference() {
        let mut cases: u64 = 0;
        let mut fast_hits: u64 = 0;
        let mut nonneg: u64 = 0;
        let sweep = |base: TimeBase, probes: &[i64], cases: &mut u64, fast: &mut u64, nn: &mut u64| {
            for &t in probes {
                let got = Timestamp::new(t, base).as_nanos();
                let want = as_nanos_reference(t, base);
                assert_eq!(got, want, "as_nanos diverged: base {base}, ticks {t}");
                if t >= 0 {
                    *nn += 1;
                    if try_fast_nanos(t, base).is_some() {
                        *fast += 1;
                    }
                }
                *cases += 1;
            }
        };
        // Every den ≤ 4096 × a spread of nums (lite probes).
        for den in 1u32..=4096 {
            for num in [1u32, 2, 3, 1001, 4096] {
                let base = literal_tb(num, den);
                let (mut n, mut d) = (u64::from(num) * 1_000_000_000, u64::from(den));
                reduce_smooth(&mut n, &mut d);
                let probes = probe_ticks(n, false);
                sweep(base, &probes, &mut cases, &mut fast_hits, &mut nonneg);
            }
        }
        // Strided dens × the full num set (full probes, incl. deeper
        // boundary straddling).
        for den in (1u32..=4096).step_by(97) {
            for num in [1u32, 2, 3, 4, 5, 7, 999, 1000, 1001, 2048, 4095, 4096] {
                let base = literal_tb(num, den);
                let (mut n, mut d) = (u64::from(num) * 1_000_000_000, u64::from(den));
                reduce_smooth(&mut n, &mut d);
                let probes = probe_ticks(n, true);
                sweep(base, &probes, &mut cases, &mut fast_hits, &mut nonneg);
            }
        }
        assert!(
            fast_hits * 2 > nonneg,
            "fast path barely exercised: {fast_hits}/{nonneg} non-negative cases"
        );
        println!(
            "as_nanos equivalence: {cases} cases ({fast_hits}/{nonneg} non-negative in fast domain)"
        );
        assert!(cases > 1_000_000);
    }

    #[test]
    fn convert_fast_path_is_bit_identical_to_reference() {
        let mut froms: Vec<TimeBase> = Vec::new();
        for den in (1u32..=4096).step_by(59) {
            froms.push(literal_tb(1, den));
            froms.push(literal_tb(1001, den));
        }
        let mut tos: Vec<TimeBase> = Vec::new();
        for den in (1u32..=4096).step_by(61) {
            tos.push(literal_tb(1, den));
            tos.push(literal_tb(1001, den));
        }
        // Real-world bases beyond the ≤ 4096 grid (extra, not a
        // substitute for it).
        for (num, den) in [
            (1u32, 90_000u32),
            (1, 1000),
            (1001, 30_000),
            (1, 48_000),
            (1, 44_100),
            (1, 1_000_000),
            (1, 1_000_000_000),
            (24_000, 1001),
        ] {
            froms.push(literal_tb(num, den));
            tos.push(literal_tb(num, den));
        }
        let mut cases: u64 = 0;
        for from in &froms {
            for to in &tos {
                let mut n = u64::from(from.num) * u64::from(to.den);
                let mut d = u64::from(from.den) * u64::from(to.num);
                if n == 0 || d == 0 {
                    continue;
                }
                reduce_smooth(&mut n, &mut d);
                for t in probe_ticks(n, false) {
                    let got = Timestamp::new(t, *from).convert(to);
                    let want = convert_reference(t, *from, *to);
                    assert_eq!(got, want, "convert diverged: {t} {from} -> {to}");
                    cases += 1;
                }
            }
        }
        // Randomized pairs inside the declared ≤ 4096 domain (LCG —
        // deterministic, seed printed on failure).
        let mut rng = Lcg(0x1118_C0DE);
        for _ in 0..20_000 {
            let from = literal_tb(
                1 + rng.below(4096) as u32,
                1 + rng.below(4096) as u32,
            );
            let to = literal_tb(1 + rng.below(4096) as u32, 1 + rng.below(4096) as u32);
            let mut n = u64::from(from.num) * u64::from(to.den);
            let mut d = u64::from(from.den) * u64::from(to.num);
            reduce_smooth(&mut n, &mut d);
            let edge = ((FAST_DOMAIN_LIMIT - 1) / u128::from(n.max(1))) as i64;
            let t = rng.below(1 << 62) as i64;
            for probe in [
                t,
                edge,
                edge.saturating_add(1),
                edge.saturating_sub(1),
                i64::MAX,
                i64::MIN,
                -t,
            ] {
                assert_eq!(
                    Timestamp::new(probe, from).convert(&to),
                    convert_reference(probe, from, to),
                    "convert diverged (random): {probe} {from} -> {to}"
                );
                cases += 1;
            }
        }
        let pairs = froms.len() * tos.len();
        println!("convert equivalence: {cases} cases over {pairs} base pairs (+ 20k random pairs)");
        assert!(cases > 1_000_000);
    }

    #[test]
    fn scaler_is_bit_identical_to_reference() {
        let mut froms: Vec<TimeBase> = Vec::new();
        for den in (1u32..=4096).step_by(137) {
            froms.push(literal_tb(1, den));
            froms.push(literal_tb(1001, den));
        }
        froms.extend([literal_tb(1, 90_000), literal_tb(1, 1_000_000_000)]);
        let mut tos: Vec<TimeBase> = Vec::new();
        for den in (1u32..=4096).step_by(131) {
            tos.push(literal_tb(1, den));
            tos.push(literal_tb(1001, den));
        }
        tos.extend([literal_tb(1001, 30_000), literal_tb(1, 1000)]);
        let mut cases: u64 = 0;
        for from in &froms {
            for to in &tos {
                let scaler = from.scaler_to(to);
                // Full probes against the scaler's own (fully-gcd-reduced)
                // domain edge — a different edge than the smooth-reduced
                // one above, so the tiers' boundaries are both straddled
                // somewhere in the suite.
                for t in probe_ticks(scaler.n, true) {
                    assert_eq!(
                        scaler.convert_ticks(t),
                        convert_reference(t, *from, *to),
                        "scaler diverged: {t} {from} -> {to}"
                    );
                    cases += 1;
                }
            }
        }
        // The nanos direction.
        for den in (1u32..=4096).step_by(31) {
            for num in [1u32, 2, 1001, 4096] {
                let base = literal_tb(num, den);
                let scaler = base.nanos_scaler();
                for t in probe_ticks(scaler.n, true) {
                    assert_eq!(
                        scaler.nanos(t),
                        as_nanos_reference(t, base),
                        "scaler nanos diverged: base {base}, t {t}"
                    );
                    cases += 1;
                }
            }
        }
        println!("scaler equivalence: {cases} cases");
        assert!(cases > 500_000);
    }

    #[test]
    fn fast_domain_covers_realistic_stream_positions() {
        // Four-hour positions in every common base, into the common
        // targets, must take the fast tiers — guards against the fast
        // path silently degenerating into always-fallback.
        let four_h: i64 = 4 * 3600;
        let bases: Vec<(TimeBase, i64)> = vec![
            (literal_tb(1, 90_000), four_h * 90_000),
            (literal_tb(1, 48_000), four_h * 48_000),
            (literal_tb(1, 44_100), four_h * 44_100),
            (literal_tb(1001, 30_000), four_h * 30_000 / 1001 + 1),
            (literal_tb(1, 24_000), four_h * 24_000),
            (literal_tb(1, 1000), four_h * 1000),
            (literal_tb(1, 1_000_000), four_h * 1_000_000),
            (literal_tb(1, 1_000_000_000), four_h * 1_000_000_000), // Matroska ns
            (literal_tb(1, 600), four_h * 600),                     // MP4 timescale
        ];
        let targets = [
            literal_tb(1, 1000),
            literal_tb(1, 1_000_000_000),
            literal_tb(1001, 30_000),
            literal_tb(1, 90_000),
        ];
        for (base, t) in &bases {
            assert!(
                try_fast_nanos(*t, *base).is_some(),
                "as_nanos slow for {base} at {t}"
            );
            assert!(
                base.nanos_scaler().in_fast_domain(*t),
                "nanos scaler slow for {base}"
            );
            for to in &targets {
                assert!(
                    try_fast_convert(*t, *base, *to).is_some(),
                    "convert slow for {base} -> {to} at {t}"
                );
                assert!(
                    base.scaler_to(to).in_fast_domain(*t),
                    "scaler slow for {base} -> {to} at {t}"
                );
            }
        }
        // Extremes must fall back — the fallback contract itself.
        assert!(try_fast_nanos(i64::MAX, literal_tb(1, 1)).is_none());
        assert!(try_fast_nanos(-1, literal_tb(1, 90_000)).is_none());
        assert!(
            try_fast_convert(i64::MIN, literal_tb(1, 1), literal_tb(1, 1000)).is_none()
        );
    }

    #[test]
    fn degenerate_literal_bases_still_match_the_reference() {
        // Zero components cannot come from TimeBase::new, but struct
        // literals are a real caller shape; neither tier may panic or
        // diverge on them.
        let degenerate = [
            literal_tb(0, 1),
            literal_tb(1, 0),
            literal_tb(0, 0),
            literal_tb(2, 0),
        ];
        for base in degenerate {
            for t in [0i64, 1, -1, i64::MAX, i64::MIN] {
                assert_eq!(
                    Timestamp::new(t, base).as_nanos(),
                    as_nanos_reference(t, base),
                    "as_nanos diverged on {base}"
                );
                for to in [NANOS_BASE, base, literal_tb(1, 1)] {
                    assert_eq!(
                        Timestamp::new(t, base).convert(&to),
                        convert_reference(t, base, to),
                        "convert diverged on {base} -> {to}"
                    );
                    let scaler = base.scaler_to(&to);
                    assert_eq!(
                        scaler.convert_ticks(t),
                        convert_reference(t, base, to),
                        "scaler diverged on {base} -> {to}"
                    );
                    assert_eq!(
                        scaler.nanos(t),
                        as_nanos_reference(t, base),
                        "scaler nanos diverged on {base}"
                    );
                }
            }
        }
    }

    #[test]
    fn reduce_smooth_preserves_the_rational_exactly() {
        let mut rng = Lcg(0x5A00_5EED);
        for _ in 0..100_000 {
            let n = 1 + rng.below(1 << 32);
            let d = 1 + rng.below(1 << 32);
            let (mut rn, mut rd) = (n, d);
            reduce_smooth(&mut rn, &mut rd);
            // Cross-multiplication in u128: the rational is unchanged.
            assert_eq!(
                u128::from(n) * u128::from(rd),
                u128::from(d) * u128::from(rn),
                "rational changed: {n}/{d} -> {rn}/{rd}"
            );
            // The targeted factors are actually exhausted.
            assert!(rn % 2 != 0 || rd % 2 != 0, "2s left in {rn}/{rd}");
            assert!(rn % 5 != 0 || rd % 5 != 0, "5s left in {rn}/{rd}");
            // Only common factors were removed.
            assert_eq!(n % rn, 0, "{rn} does not divide {n}");
            assert_eq!(d % rd, 0, "{rd} does not divide {d}");
        }
    }

    #[test]
    fn five_pow_table_is_exact() {
        assert_eq!(
            FIVE_POW,
            [
                5u64.pow(16),
                5u64.pow(8),
                5u64.pow(4),
                5u64.pow(2),
                5u64.pow(1)
            ]
        );
    }

    /// Measured, not claimed (D-6/D-13): 10M conversions through each
    /// tier on realistic 90 kHz positions, "before" = the kept i128
    /// reference (the exact pre-M18 code path). Run with:
    /// `cargo test -p media-types --release -- --ignored --nocapture m18_bench`
    #[test]
    #[ignore = "manual M18 benchmark — run with --release --nocapture"]
    fn m18_bench_10m_conversions() {
        use std::hint::black_box;
        use std::time::Instant;

        const N: usize = 10_000_000;
        // black_box the bases so LLVM cannot fold the reference's i128
        // division into its own constant-divisor magic (the real callers
        // hold runtime stream bases, not literals — without this the
        // "before" leg measures LLVM, not the code). The Scaler's
        // reciprocal is likewise derived from opaque runtime values.
        let from = black_box(literal_tb(1, 90_000));
        let to = black_box(literal_tb(1, 1000));
        let scaler = black_box(from.scaler_to(&to));
        let nanos_scaler = black_box(from.nanos_scaler());

        // Realistic magnitudes: up to ~4 h of 90 kHz positions,
        // strided to defeat branch and cache predictability.
        let ticks = |i: usize| (i as i64 * 7919) % 1_296_000_000;

        fn run(label: &str, n: usize, mut f: impl FnMut(usize) -> i64) {
            let t0 = Instant::now();
            let mut acc = 0i64;
            for i in 0..n {
                acc = acc.wrapping_add(black_box(f(i)));
            }
            black_box(acc);
            let elapsed = t0.elapsed();
            println!(
                "{label:<36} {:>11.3?} total   {:>7.2} ns/conv",
                elapsed,
                elapsed.as_nanos() as f64 / n as f64
            );
        }

        println!("M18 benchmark — {N} conversions each, 1/90000 -> 1/1000:");
        run("before: i128 reference convert", N, |i| {
            match convert_reference(ticks(i), from, to) {
                Ok(t) => t.ticks,
                Err(_) => 0,
            }
        });
        run("after: Timestamp::convert (tier 2)", N, |i| {
            match Timestamp::new(ticks(i), from).convert(&to) {
                Ok(t) => t.ticks,
                Err(_) => 0,
            }
        });
        run("after: Scaler::convert_ticks (tier 1)", N, |i| {
            match scaler.convert_ticks(ticks(i)) {
                Ok(t) => t.ticks,
                Err(_) => 0,
            }
        });
        println!("M18 benchmark — {N} conversions each, 1/90000 -> nanoseconds:");
        run("before: i128 reference as_nanos", N, |i| {
            match as_nanos_reference(ticks(i), from) {
                Ok(v) => v.0 as i64,
                Err(_) => 0,
            }
        });
        run("after: Timestamp::as_nanos (tier 2)", N, |i| {
            match Timestamp::new(ticks(i), from).as_nanos() {
                Ok(v) => v.0 as i64,
                Err(_) => 0,
            }
        });
        run("after: Scaler::nanos (tier 1)", N, |i| {
            match nanos_scaler.nanos(ticks(i)) {
                Ok(v) => v.0 as i64,
                Err(_) => 0,
            }
        });
    }
}
