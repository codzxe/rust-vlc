//! The damage ledger (card 5.M1): malformed-tolerance bookkeeping.
//!
//! The SRT parser NEVER aborts a file and NEVER panics — damage is
//! data (MP §8). Every tolerated malformation (garbage block, bad
//! timestamp, unknown tag, …) appends one [`SrtDamage`] here instead
//! of failing the parse; the document's cue list simply carries the
//! cues that survived.
//!
//! Bounded memory (D-2): the ledger is capped at
//! [`MAX_DAMAGE_EVENTS`] entries; further events are counted in
//! `dropped_damage_events` (a `u32` saturating counter) so a file
//! crafted as "infinite small damage" cannot grow the ledger without
//! bound.

/// The ledger cap: at most this many [`SrtDamage`] events are kept;
/// the rest are counted, not stored.
pub const MAX_DAMAGE_EVENTS: usize = 256;

/// What kind of tolerated malformation was recorded.
///
/// `Copy` + `Eq`: the ledger is comparable in tests (differential
/// ACs assert exact damage sequences, not just counts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SrtDamageKind {
    /// The byte stream was not valid UTF-8; lossy replacement was
    /// substituted (the UTF-8-BOM decode path).
    InvalidUtf8,
    /// No BOM and invalid UTF-8: the byte stream was decoded as
    /// Windows-1252 (the common 8-bit fallback, ffmpeg's `sub_charenc`
    /// behaviour for legacy SRTs); one event per file.
    Windows1252Fallback,
    /// A Windows-1252 payload hit one of the five undefined code
    /// points (0x81/0x8D/0x8F/0x90/0x9D); U+FFFD was substituted.
    UndefinedWindows1252Byte,
    /// A UTF-16 payload had an odd trailing byte (half a code unit).
    OddTrailingUtf16Byte,
    /// A UTF-16 payload had an unpaired surrogate; U+FFFD replaced it.
    UnpairedUtf16Surrogate,
    /// A block contained no `-->` timing line at all; skipped whole.
    GarbageBlock,
    /// More than one line preceded the timing line inside a block.
    StrayLinesBeforeTiming,
    /// The index line was present but not a plain number; a synthetic
    /// id was minted instead.
    BadIndexLine,
    /// The index number did not fit `u64`; a synthetic id was minted.
    IndexOverflow,
    /// The timing line could not be parsed; the cue was skipped.
    BadTimestamp,
    /// The millisecond field had more than 3 digits; it was truncated
    /// to the first 3.
    LongMilliseconds,
    /// A timestamp overflowed the i64 microsecond domain; it
    /// saturated (saturating arithmetic — the cue survives).
    TimestampSaturated,
    /// `end < start` on a timing line; the cue is typed-invalid in
    /// the model and was skipped.
    ReversedInterval,
    /// Unrecognized text followed the end timestamp on the timing
    /// line; ignored, the cue survives.
    TrailingJunk,
    /// `X1:.. X2:.. Y1:.. Y2:..` pixel coordinates followed the end
    /// timestamp; SRT positioning is not modeled (R4) and they were
    /// ignored.
    IgnoredPosition,
    /// An unrecognized inline tag was stripped from the text.
    UnknownTag,
    /// A `<` with no closing `>`; kept literally.
    UnterminatedTag,
    /// A `&entity;` that is not one of the supported set; kept
    /// literally.
    UnknownEntity,
    /// A recognized style construct carried a value that could not be
    /// applied (e.g. a `<font color=…>` literal outside the supported
    /// hex forms); the style was dropped.
    BadStyleValue,
    /// A block had a timing line but no text lines; skipped.
    EmptyCueText,
}

impl SrtDamageKind {
    /// One-line human description (the ledger is diagnostics, D-4).
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::InvalidUtf8 => "invalid UTF-8 replaced lossily",
            Self::Windows1252Fallback => "no BOM, invalid UTF-8: decoded as Windows-1252",
            Self::UndefinedWindows1252Byte => "undefined Windows-1252 byte replaced",
            Self::OddTrailingUtf16Byte => "odd trailing UTF-16 byte dropped",
            Self::UnpairedUtf16Surrogate => "unpaired UTF-16 surrogate replaced",
            Self::GarbageBlock => "block without a timing line skipped",
            Self::StrayLinesBeforeTiming => "stray lines before the timing line",
            Self::BadIndexLine => "non-numeric index line; synthetic id minted",
            Self::IndexOverflow => "index beyond u64; synthetic id minted",
            Self::BadTimestamp => "unparseable timing line; cue skipped",
            Self::LongMilliseconds => "millisecond field longer than 3 digits, truncated",
            Self::TimestampSaturated => "timestamp saturated the microsecond domain",
            Self::ReversedInterval => "end before start; cue skipped",
            Self::TrailingJunk => "trailing junk after the end timestamp ignored",
            Self::IgnoredPosition => "X1:/Y1: pixel coordinates ignored (R4)",
            Self::UnknownTag => "unknown inline tag stripped",
            Self::UnterminatedTag => "unterminated '<' kept literally",
            Self::UnknownEntity => "unknown entity kept literally",
            Self::BadStyleValue => "unusable style value dropped",
            Self::EmptyCueText => "cue without text lines skipped",
        }
    }
}

impl std::fmt::Display for SrtDamageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.describe())
    }
}

/// One tolerated malformation: what happened, and the 1-based source
/// line it happened on (0 = before line accounting starts, i.e. the
/// byte-decode stage).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SrtDamage {
    /// What happened.
    pub kind: SrtDamageKind,
    /// 1-based line number in the (line-ending-normalized) source;
    /// 0 for byte-level (decoding) damage.
    pub line_no: u32,
}

/// The capped ledger the parser appends into.
///
/// Public because the encoding/markup sub-parsers take it as a
/// parameter (they record damage where they detect it).
#[derive(Debug, Default)]
pub struct DamageSink {
    events: Vec<SrtDamage>,
    dropped: u32,
}

impl DamageSink {
    /// An empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one event, or count it past the cap (never grows the
    /// `Vec` beyond [`MAX_DAMAGE_EVENTS`]).
    pub fn record(&mut self, kind: SrtDamageKind, line_no: u32) {
        if self.events.len() < MAX_DAMAGE_EVENTS {
            self.events.push(SrtDamage { kind, line_no });
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }

    /// Split into (kept events, dropped count).
    pub fn into_parts(self) -> (Vec<SrtDamage>, u32) {
        (self.events, self.dropped)
    }
}

#[cfg(test)]
mod tests {
    use super::{DamageSink, MAX_DAMAGE_EVENTS, SrtDamage, SrtDamageKind};

    #[test]
    fn ledger_caps_and_counts() {
        let mut sink = DamageSink::new();
        for _ in 0..(MAX_DAMAGE_EVENTS + 100) {
            sink.record(SrtDamageKind::UnknownTag, 7);
        }
        let (events, dropped) = sink.into_parts();
        assert_eq!(events.len(), MAX_DAMAGE_EVENTS);
        assert_eq!(dropped, 100);
        assert_eq!(
            events[0],
            SrtDamage {
                kind: SrtDamageKind::UnknownTag,
                line_no: 7
            }
        );
    }

    #[test]
    fn every_kind_describes_itself() {
        // Exhaustive at the time of writing; new kinds must extend it.
        let kinds = [
            SrtDamageKind::InvalidUtf8,
            SrtDamageKind::Windows1252Fallback,
            SrtDamageKind::UndefinedWindows1252Byte,
            SrtDamageKind::OddTrailingUtf16Byte,
            SrtDamageKind::UnpairedUtf16Surrogate,
            SrtDamageKind::GarbageBlock,
            SrtDamageKind::StrayLinesBeforeTiming,
            SrtDamageKind::BadIndexLine,
            SrtDamageKind::IndexOverflow,
            SrtDamageKind::BadTimestamp,
            SrtDamageKind::LongMilliseconds,
            SrtDamageKind::TimestampSaturated,
            SrtDamageKind::ReversedInterval,
            SrtDamageKind::TrailingJunk,
            SrtDamageKind::IgnoredPosition,
            SrtDamageKind::UnknownTag,
            SrtDamageKind::UnterminatedTag,
            SrtDamageKind::UnknownEntity,
            SrtDamageKind::BadStyleValue,
            SrtDamageKind::EmptyCueText,
        ];
        for k in kinds {
            assert!(!k.describe().is_empty());
            assert_eq!(k.to_string(), k.describe());
        }
    }
}
