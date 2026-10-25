//! Cue timing: half-open intervals `[start, end)` in signed microseconds
//! (card 5.L2 AC: zero-duration legal, `end < start` typed-invalid,
//! touching ≠ overlap).
//!
//! ## Representation choice
//!
//! Signed i64 microseconds — wide enough for ±292 k years, uniform with
//! the demux layer's nanosecond-precision stamps after one ×1000
//! scaling, and signed because real subtitle streams do carry slightly
//! negative lead-in timestamps (e.g. ASS `0:00:00.00` clamps, Matroska
//! blocks before the cluster timestamp). [PROPOSAL: the Lead unifies
//! the exact tick type with 5.L1's parser outputs and `media-types`
//! bridging at wave close.]
//!
//! ## Safe math
//!
//! All arithmetic is **saturating** — `shift_us` never wraps and never
//! breaks the `start <= end` invariant (saturating add is monotone),
//! `duration_us` at the extreme pair `(MIN, MAX)` saturates to `MAX`
//! rather than overflowing. The constructor is the single validity
//! gate: once a [`CueInterval`] exists, `start_us <= end_us` holds.

use crate::model::error::ModelError;

/// A half-open cue timing interval `[start_us, end_us)` in microseconds.
///
/// Invariant (enforced by [`CueInterval::new`]): `start_us <= end_us`.
/// A zero-duration interval (`start == end`) is legal and **empty** —
/// it contains no instant and overlaps nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CueInterval {
    start_us: i64,
    end_us: i64,
}

impl CueInterval {
    /// The legal empty interval at time zero.
    pub const EMPTY: Self = Self {
        start_us: 0,
        end_us: 0,
    };

    /// Construct a valid interval.
    ///
    /// # Errors
    /// [`ModelError::EndBeforeStart`] when `end_us < start_us`
    /// (zero-duration `end == start` is legal).
    pub fn new(start_us: i64, end_us: i64) -> Result<Self, ModelError> {
        if end_us < start_us {
            return Err(ModelError::EndBeforeStart { start_us, end_us });
        }
        Ok(Self { start_us, end_us })
    }

    /// The inclusive lower bound (µs).
    pub const fn start_us(&self) -> i64 {
        self.start_us
    }

    /// The exclusive upper bound (µs).
    pub const fn end_us(&self) -> i64 {
        self.end_us
    }

    /// Duration `end - start` (µs), saturating at the `i64` extreme
    /// pair; never negative by the construction invariant.
    pub const fn duration_us(&self) -> i64 {
        self.end_us.saturating_sub(self.start_us)
    }

    /// True when the interval is empty (`start == end`): it contains no
    /// instant, overlaps nothing, and is legal.
    pub const fn is_empty(&self) -> bool {
        self.start_us == self.end_us
    }

    /// Half-open membership: `start <= t < end`. An empty interval
    /// contains nothing — not even its own bound.
    pub const fn contains(&self, t_us: i64) -> bool {
        self.start_us <= t_us && t_us < self.end_us
    }

    /// True when the two intervals share interior time. **Touching is
    /// not overlap**: `[0,10)` and `[10,20)` do not overlap. Empty
    /// intervals never overlap anything (their intersection is the
    /// empty set — including with an interval that spans them).
    pub const fn overlaps(&self, other: &Self) -> bool {
        // intersection non-empty: max(starts) < min(ends), written
        // branch-wise because `Ord::min/max` are not const-callable
        // on this toolchain.
        let lo = if self.start_us > other.start_us {
            self.start_us
        } else {
            other.start_us
        };
        let hi = if self.end_us < other.end_us {
            self.end_us
        } else {
            other.end_us
        };
        lo < hi
    }

    /// True when the intervals do not overlap but share exactly one
    /// boundary point (`self.end == other.start` or vice versa).
    /// `[0,10)` and `[10,20)` touch. An empty interval `[t,t)` touches
    /// any interval with a boundary at `t`.
    pub const fn touches(&self, other: &Self) -> bool {
        !self.overlaps(other) && (self.end_us == other.start_us || other.end_us == self.start_us)
    }

    /// The intersection `self ∩ other` as a half-open interval.
    ///
    /// `None` when the interiors do not meet — **disjoint *and*
    /// touching** both yield `None** (a touching intersection is the
    /// empty set, not a zero-length interval; zero-length is reserved
    /// for a genuine point-specified cue).
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let lo = self.start_us.max(other.start_us);
        let hi = self.end_us.min(other.end_us);
        if lo < hi {
            Some(Self {
                start_us: lo,
                end_us: hi,
            })
        } else {
            None
        }
    }

    /// The smallest interval covering both (unchecked hull — always
    /// valid because both inputs are valid).
    pub fn hull(&self, other: &Self) -> Self {
        Self {
            start_us: self.start_us.min(other.start_us),
            end_us: self.end_us.max(other.end_us),
        }
    }

    /// Shift both bounds by `delta_us`, **saturating** at the `i64`
    /// bounds. Saturating addition is monotone, so the `start <= end`
    /// invariant survives every shift (an interval pushed far past
    /// `i64::MAX` collapses to a legal empty interval at `MAX`, never
    /// to a reversed one).
    pub const fn shift_us(&self, delta_us: i64) -> Self {
        Self {
            start_us: self.start_us.saturating_add(delta_us),
            end_us: self.end_us.saturating_add(delta_us),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CueInterval;
    use crate::model::error::ModelError;
    use std::error::Error;

    #[test]
    fn zero_duration_is_legal_and_empty() -> Result<(), Box<dyn Error>> {
        let z = CueInterval::new(1_000, 1_000)?;
        assert!(z.is_empty());
        assert_eq!(z.duration_us(), 0);
        assert!(!z.contains(1_000));
        assert!(!z.contains(999));
        assert!(!z.overlaps(&z));
        assert!(!z.overlaps(&CueInterval::new(0, 2_000)?));
        Ok(())
    }

    #[test]
    fn end_before_start_is_typed_invalid() {
        assert_eq!(
            CueInterval::new(5, 4),
            Err(ModelError::EndBeforeStart {
                start_us: 5,
                end_us: 4
            })
        );
    }

    #[test]
    fn touching_is_not_overlap() -> Result<(), Box<dyn Error>> {
        let a = CueInterval::new(0, 10)?;
        let b = CueInterval::new(10, 20)?;
        assert!(!a.overlaps(&b));
        assert!(!b.overlaps(&a));
        assert!(a.touches(&b));
        assert!(b.touches(&a));
        assert_eq!(a.intersection(&b), None);
        assert_eq!(a.hull(&b), CueInterval::new(0, 20)?);
        Ok(())
    }

    #[test]
    fn overlap_windows() -> Result<(), Box<dyn Error>> {
        let a = CueInterval::new(0, 10)?;
        assert!(a.overlaps(&CueInterval::new(9, 20)?));
        assert!(a.overlaps(&CueInterval::new(5, 7)?));
        assert!(!a.overlaps(&CueInterval::new(20, 30)?));
        assert_eq!(
            a.intersection(&CueInterval::new(5, 15)?),
            Some(CueInterval::new(5, 10)?)
        );
        Ok(())
    }

    #[test]
    fn saturating_extremes() -> Result<(), Box<dyn Error>> {
        let full = CueInterval::new(i64::MIN, i64::MAX)?;
        assert_eq!(full.duration_us(), i64::MAX);
        // A moderate shift moves the start, saturates nothing (MIN + 1e6
        // is representable), and the end pins at MAX.
        let shifted = full.shift_us(1_000_000);
        assert_eq!(shifted.start_us(), i64::MIN + 1_000_000);
        assert_eq!(shifted.end_us(), i64::MAX);
        assert!(shifted.end_us() >= shifted.start_us());
        // A saturating shift collapses to a legal empty interval at MAX.
        let edge = CueInterval::new(i64::MAX - 5, i64::MAX)?;
        let pushed = edge.shift_us(i64::MAX);
        assert_eq!(pushed.start_us(), i64::MAX);
        assert_eq!(pushed.end_us(), i64::MAX);
        assert!(pushed.is_empty());
        Ok(())
    }

    #[test]
    fn contains_is_half_open() -> Result<(), Box<dyn Error>> {
        let a = CueInterval::new(100, 200)?;
        assert!(!a.contains(99));
        assert!(a.contains(100));
        assert!(a.contains(199));
        assert!(!a.contains(200));
        Ok(())
    }
}
