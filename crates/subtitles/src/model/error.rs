//! Typed errors for the cue model (MP §8: failure is data, not a panic).
//!
//! Two taxa, both `#[non_exhaustive]` (5.L1's format parsers and the
//! Lead's unification may grow variants):
//!
//! * [`ModelError`] — constructing/resolving a model value failed
//!   (e.g. `end < start`, a percentage outside `0..=100`);
//! * [`WireError`] — parsing the stable wire form failed, with the
//!   1-based offending line number for dirty-world diagnosis (D-4).

use std::fmt;

/// A model-value construction or resolution failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModelError {
    /// `end_us < start_us` — the interval is typed-invalid.
    /// Zero-duration (`end == start`) is legal; reversal is not.
    EndBeforeStart {
        /// The rejected start (µs).
        start_us: i64,
        /// The rejected end (µs).
        end_us: i64,
    },
    /// A percentage outside `0..=100`.
    PercentOutOfRange {
        /// The offending value.
        value: i32,
    },
    /// A viewport extent was zero — nothing can be placed in an empty
    /// viewport, so resolution is typed-invalid rather than a silent 0.
    EmptyViewport,
    /// A color literal was malformed (not `#rrggbbaa` hex).
    BadColor,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EndBeforeStart { start_us, end_us } => write!(
                f,
                "cue interval end {end_us} µs precedes start {start_us} µs (zero-duration is legal, reversal is not)"
            ),
            Self::PercentOutOfRange { value } => {
                write!(f, "percentage {value} outside 0..=100")
            }
            Self::EmptyViewport => write!(f, "viewport extent is zero"),
            Self::BadColor => write!(f, "malformed color literal (expected #rrggbbaa)"),
        }
    }
}

impl std::error::Error for ModelError {}

/// A wire-form parse failure. Line numbers are 1-based into the source
/// document (the magic line is line 1).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WireError {
    /// The document did not start with the `LUMEN-CUES 1` magic line.
    MissingMagic,
    /// A line had an unknown leading tag byte.
    UnknownTag {
        /// 1-based line number.
        line_no: usize,
        /// The offending tag byte.
        tag: u8,
    },
    /// A required field was absent.
    MissingField {
        /// 1-based line number.
        line_no: usize,
        /// Which field.
        field: &'static str,
    },
    /// More fields were present than the record admits.
    ExtraFields {
        /// 1-based line number.
        line_no: usize,
    },
    /// A numeric field failed to parse.
    BadInt {
        /// 1-based line number.
        line_no: usize,
        /// Which field.
        field: &'static str,
    },
    /// The 2-character alignment code was not one of the nine.
    BadAlignment {
        /// 1-based line number.
        line_no: usize,
    },
    /// A numeric field was well-formed but out of domain (e.g. a
    /// percent token `p<n>` with `n > 100`, flags not `0`/`1`).
    BadValue {
        /// 1-based line number.
        line_no: usize,
        /// Which field.
        field: &'static str,
    },
    /// A color token was neither `-` nor `#rrggbbaa`.
    BadColor {
        /// 1-based line number.
        line_no: usize,
    },
    /// A cue header carried `end < start` (typed invalid at parse too).
    EndBeforeStart {
        /// 1-based line number of the header.
        line_no: usize,
        /// The rejected start (µs).
        start_us: i64,
        /// The rejected end (µs).
        end_us: i64,
    },
    /// A percentage outside `0..=100` appeared in the wire form.
    PercentOutOfRange {
        /// 1-based line number.
        line_no: usize,
        /// The offending value.
        value: i32,
    },
    /// A `C` header was not followed by a payload (`P`/`S`) line.
    MissingPayload {
        /// 1-based line number of the orphan header.
        line_no: usize,
    },
    /// A `\` escape introduced a character that is not one of
    /// `\\`, `\n`, `\r`, `\t`.
    BadEscape {
        /// 1-based line number.
        line_no: usize,
    },
    /// A styled payload (`S <n>`) ended before all `n` segments arrived.
    UnterminatedStyled {
        /// 1-based line number of the `S` line.
        line_no: usize,
        /// Declared segment count.
        expected: usize,
        /// Segments actually seen.
        seen: usize,
    },
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMagic => write!(
                f,
                "document does not start with the LUMEN-CUES 1 magic line"
            ),
            Self::UnknownTag { line_no, tag } => {
                write!(
                    f,
                    "line {line_no}: unknown record tag {:?}",
                    char::from(*tag)
                )
            }
            Self::MissingField { line_no, field } => {
                write!(f, "line {line_no}: missing field `{field}`")
            }
            Self::ExtraFields { line_no } => write!(f, "line {line_no}: trailing fields"),
            Self::BadInt { line_no, field } => {
                write!(f, "line {line_no}: field `{field}` is not an integer")
            }
            Self::BadValue { line_no, field } => {
                write!(f, "line {line_no}: field `{field}` out of domain")
            }
            Self::BadAlignment { line_no } => write!(f, "line {line_no}: bad alignment code"),
            Self::BadColor { line_no } => write!(f, "line {line_no}: bad color token"),
            Self::EndBeforeStart {
                line_no,
                start_us,
                end_us,
            } => write!(
                f,
                "line {line_no}: interval end {end_us} µs precedes start {start_us} µs"
            ),
            Self::PercentOutOfRange { line_no, value } => {
                write!(f, "line {line_no}: percentage {value} outside 0..=100")
            }
            Self::MissingPayload { line_no } => {
                write!(f, "line {line_no}: cue header without a payload line")
            }
            Self::BadEscape { line_no } => write!(f, "line {line_no}: bad backslash escape"),
            Self::UnterminatedStyled {
                line_no,
                expected,
                seen,
            } => write!(
                f,
                "line {line_no}: styled payload declared {expected} segments, saw {seen}"
            ),
        }
    }
}

impl std::error::Error for WireError {}
