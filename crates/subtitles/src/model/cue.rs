//! The cue itself: identity, timing interval, text payload and
//! positioning (card 5.L2). One [`Cue`] is one on-screen subtitle
//! event; a document is an ordered list of them.

use crate::model::position::Positioning;
use crate::model::time::CueInterval;

/// A cue's identity within a track — opaque, stable, comparable.
/// Newtype over `u64` (SRT numbering, Matroska block UIDs and WebVTT
/// cue identifiers all fit). Copy + `Ord` so documents can be sorted
/// or deduplicated without borrowing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CueId(u64);

impl CueId {
    /// Construct from a raw number (no validity domain — any `u64` is a
    /// legal identifier; absence of a cue id is `Option<CueId>`).
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw number.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// One styled run of text: a slice of the cue's text plus the
/// \[PROPOSAL\]-subset style that applies to it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StyledSegment {
    /// The run's text (may be empty — a style change with no glyphs).
    pub text: String,
    /// The style applied to [`StyledSegment::text`].
    pub style: crate::model::style::TextStyle,
}

/// A cue's text payload: either fully plain (the overwhelming SRT
/// majority — no per-run allocation) or a run list with styles.
///
/// The split is deliberate (memory care, D-1): plain cues pay one
/// `String`, styled cues pay one `Vec` of runs, and neither pays for
/// the other.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CueText {
    /// Unstyled text.
    Plain(String),
    /// Styled runs, in display order (may be empty).
    Styled(Vec<StyledSegment>),
}

impl CueText {
    /// Total UTF-8 byte length of the payload across all runs
    /// (saturating — the sum of `usize` lengths).
    #[must_use]
    pub fn total_len(&self) -> usize {
        match self {
            Self::Plain(s) => s.len(),
            Self::Styled(segs) => segs
                .iter()
                .map(|s| s.text.len())
                .fold(0usize, usize::saturating_add),
        }
    }

    /// True for [`CueText::Plain`].
    #[must_use]
    pub fn is_plain(&self) -> bool {
        matches!(self, Self::Plain(_))
    }
}

/// One subtitle event: id, half-open timing interval, text payload,
/// positioning.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Cue {
    /// Stable identity within the track.
    pub id: CueId,
    /// `[start, end)` in microseconds (zero-duration is legal).
    pub interval: CueInterval,
    /// The text payload (plain or styled).
    pub text: CueText,
    /// Alignment and placement.
    pub positioning: Positioning,
}

impl Cue {
    /// Construct a cue. Non-fallible: every component carries its own
    /// typed validity ([`CueInterval::new`], `Percent::new`), so a
    /// constructed `Cue` is always well-formed.
    pub const fn new(
        id: CueId,
        interval: CueInterval,
        text: CueText,
        positioning: Positioning,
    ) -> Self {
        Self {
            id,
            interval,
            text,
            positioning,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Cue, CueId, CueText, StyledSegment};
    use crate::model::error::ModelError;
    use crate::model::position::Positioning;
    use crate::model::style::TextStyle;
    use crate::model::time::CueInterval;
    use std::error::Error;

    #[test]
    fn cue_assembles_typed_parts() -> Result<(), Box<dyn Error>> {
        let interval = CueInterval::new(0, 1_500_000)?;
        let text = CueText::Styled(vec![StyledSegment {
            text: "hello".into(),
            style: TextStyle::new(true, false, false, None),
        }]);
        let cue = Cue::new(CueId::new(7), interval, text, Positioning::default());
        assert_eq!(cue.id.get(), 7);
        assert_eq!(cue.interval, interval);
        assert!(!cue.text.is_plain());
        assert_eq!(cue.text.total_len(), 5);
        Ok(())
    }

    #[test]
    fn typed_invalidity_stays_in_components() {
        let err = CueInterval::new(10, 9).err();
        assert_eq!(
            err,
            Some(ModelError::EndBeforeStart {
                start_us: 10,
                end_us: 9
            })
        );
        assert_eq!(CueText::Plain(String::new()).total_len(), 0);
        assert_eq!(CueText::Styled(Vec::new()).total_len(), 0);
        let styled = TextStyle::new(true, false, true, None);
        assert!(styled.bold);
        assert!(!styled.italic);
        assert!(styled.underline);
        assert_eq!(styled.color, None);
    }
}
