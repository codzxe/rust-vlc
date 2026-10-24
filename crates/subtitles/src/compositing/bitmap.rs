//! Cue payloads for compositing: an RGBA8 bitmap with its baseline
//! row, and the timed/z-ordered [`CompositedCue`] the compositor
//! consumes. Timestamp conversion happens ONCE, at build — per-frame
//! timing is a plain `i64` nanosecond compare ([`timing`]).
//!
//! This is the format-agnostic layer doc 04 §5 requires: PGS/DVB/
//! VobSub decoded bitmaps AND renderer-rasterized text cues both
//! reduce to a [`CueBitmap`] + [`CueLayout`] + span + z; the parsers
//! (5.M*) and the text stack (renderer) feed the same contract.

use crate::compositing::layout::CueLayout;
use crate::compositing::target::Rgba;
use media_types::time::{Stamp, TimeError, TimeSpan};

/// Typed cue-construction errors (MP §8: data, never a panic).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CueError {
    /// A zero width or height.
    ZeroDimension {
        /// The offending bitmap size.
        size: (u32, u32),
    },
    /// `rgba.len() != width * height * 4` (straight-alpha RGBA8,
    /// row-major, no padding).
    StrideMismatch {
        /// The expected byte length.
        expected: usize,
        /// The provided byte length.
        got: usize,
    },
    /// `baseline > height`.
    BaselinePastBottom {
        /// The offending baseline row.
        baseline: u32,
        /// The bitmap height.
        height: u32,
    },
    /// A rational with zero denominator.
    ZeroDenominator,
    /// An anchor fraction outside `[0, 1]`.
    AnchorOutOfRange {
        /// Which axis (`"u"` / `"v"`).
        axis: &'static str,
        /// The offending numerator.
        num: u32,
        /// The offending denominator.
        den: u32,
    },
    /// A stamp could not be converted to the clock's nanosecond base.
    Time(TimeError),
}

impl std::fmt::Display for CueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CueError::ZeroDimension { size } => {
                write!(f, "cue bitmap has a zero dimension: {size:?}")
            }
            CueError::StrideMismatch { expected, got } => write!(
                f,
                "cue bitmap stride mismatch: expected {expected} RGBA8 bytes, got {got}"
            ),
            CueError::BaselinePastBottom { baseline, height } => write!(
                f,
                "cue baseline row {baseline} is past the bitmap bottom ({height} rows)"
            ),
            CueError::ZeroDenominator => f.write_str("rational with zero denominator"),
            CueError::AnchorOutOfRange { axis, num, den } => {
                write!(f, "anchor fraction {axis} = {num}/{den} outside [0, 1]")
            }
            CueError::Time(err) => write!(f, "cue timestamp conversion failed: {err}"),
        }
    }
}

impl std::error::Error for CueError {}

impl From<TimeError> for CueError {
    fn from(err: TimeError) -> CueError {
        CueError::Time(err)
    }
}

/// A straight-alpha RGBA8 cue bitmap, row-major, no padding, plus the
/// baseline row (the anchor for [`crate::compositing::VerticalAlign::Baseline`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CueBitmap {
    /// Pixel width (never zero).
    pub width: u32,
    /// Pixel height (never zero).
    pub height: u32,
    /// The text baseline: rows from the bitmap top (<= height; equals
    /// `height` for pure bottom-aligned bitmaps).
    pub baseline: u32,
    /// `width * height * 4` RGBA8 bytes.
    pub rgba: Vec<u8>,
}

impl CueBitmap {
    /// Validate and adopt a bitmap.
    ///
    /// # Errors
    /// [`CueError::ZeroDimension`], [`CueError::StrideMismatch`] or
    /// [`CueError::BaselinePastBottom`] — see the variants.
    pub fn from_rgba(
        width: u32,
        height: u32,
        baseline: u32,
        rgba: Vec<u8>,
    ) -> Result<CueBitmap, CueError> {
        if width == 0 || height == 0 {
            return Err(CueError::ZeroDimension {
                size: (width, height),
            });
        }
        let expected = (width as u64)
            .checked_mul(height as u64)
            .and_then(|px| px.checked_mul(4))
            .ok_or(CueError::StrideMismatch {
                expected: usize::MAX,
                got: rgba.len(),
            })?;
        if expected != rgba.len() as u64 {
            return Err(CueError::StrideMismatch {
                expected: expected as usize,
                got: rgba.len(),
            });
        }
        if baseline > height {
            return Err(CueError::BaselinePastBottom { baseline, height });
        }
        Ok(CueBitmap {
            width,
            height,
            baseline,
            rgba,
        })
    }

    /// A solid synthetic bitmap (test scaffolding and degenerate
    /// boxes): every pixel `color`, baseline at the bottom.
    ///
    /// # Errors
    /// [`CueError::ZeroDimension`] when a dimension is zero.
    pub fn solid(width: u32, height: u32, color: Rgba) -> Result<CueBitmap, CueError> {
        if width == 0 || height == 0 {
            return Err(CueError::ZeroDimension {
                size: (width, height),
            });
        }
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for _ in 0..width as usize * height as usize {
            rgba.extend_from_slice(&[color.r, color.g, color.b, color.a]);
        }
        Ok(CueBitmap {
            width,
            height,
            baseline: height,
            rgba,
        })
    }

    /// The byte offset of pixel `(x, y)`.
    pub(crate) fn offset(&self, x: u32, y: u32) -> usize {
        (y as usize * self.width as usize + x as usize) * 4
    }

    /// One pixel (synthetic-test and debug vocabulary).
    ///
    /// # Panics
    /// Never in release; debug-asserts bounds (the constructor
    /// validated the stride, callers pass in-range coordinates).
    #[must_use = "reading a pixel you will not look at is a bug in the test"]
    pub fn pixel(&self, x: u32, y: u32) -> Rgba {
        debug_assert!(x < self.width && y < self.height);
        let i = self.offset(x, y);
        Rgba {
            r: self.rgba[i],
            g: self.rgba[i + 1],
            b: self.rgba[i + 2],
            a: self.rgba[i + 3],
        }
    }
}

/// One cue ready for compositing: bitmap + layout + z-order + the
/// active interval, with both span ends pre-converted to the clock's
/// nanosecond base (once, here — never per frame).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositedCue {
    /// The pixels to blend.
    pub bitmap: CueBitmap,
    /// Where they go.
    pub layout: CueLayout,
    /// Z-order: higher draws on top of lower; ties break by insertion
    /// order (later cue wins — the track's own ordering).
    pub z: u16,
    /// Interval start in nanoseconds; `None` = live from the
    /// beginning (WebVTT open-ended cues).
    pub start: Option<i64>,
    /// Interval end in nanoseconds; `None` = open-ended (live).
    pub end: Option<i64>,
}

impl CompositedCue {
    /// Build a cue from a media-types span (any time base — converted
    /// to nanoseconds here) with bottom-center default positioning
    /// and the given z.
    ///
    /// # Errors
    /// [`CueError`] from the bitmap validation or the span conversion.
    pub fn new(
        span: TimeSpan,
        bitmap: CueBitmap,
        layout: CueLayout,
        z: u16,
    ) -> Result<CompositedCue, CueError> {
        let stamp_ns = |stamp: &Stamp| -> Result<Option<i64>, CueError> {
            match stamp {
                Stamp::Present(ts) => {
                    let ns = ts.as_nanos()?;
                    Ok(Some(ns.0 as i64))
                }
                Stamp::Missing => Ok(None),
            }
        };
        Ok(CompositedCue {
            bitmap,
            layout,
            z,
            start: stamp_ns(&span.start)?,
            end: stamp_ns(&span.end)?,
        })
    }

    /// Is this cue active at `now_ns`? The half-open `[t0, t1)` rule —
    /// see [`crate::compositing::timing`].
    #[must_use = "the caller composes exactly the active set"]
    pub fn active_at(&self, now_ns: i64) -> bool {
        crate::compositing::timing::active_at(self.start, self.end, now_ns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::time::Timestamp;
    use testkit::Must;

    #[test]
    fn stride_is_validated() {
        let err = CueBitmap::from_rgba(4, 4, 4, vec![0; 63]);
        assert!(matches!(
            err,
            Err(CueError::StrideMismatch {
                expected: 64,
                got: 63
            })
        ));
        let err = CueBitmap::from_rgba(0, 4, 0, vec![]);
        assert!(matches!(err, Err(CueError::ZeroDimension { .. })));
        let err = CueBitmap::from_rgba(4, 4, 5, vec![0; 64]);
        assert!(matches!(
            err,
            Err(CueError::BaselinePastBottom {
                baseline: 5,
                height: 4
            })
        ));
    }

    #[test]
    fn span_converts_to_ns_once() {
        // 90 kHz stamps (the MPEG world) become exact nanoseconds.
        let span = TimeSpan {
            start: Stamp::Present(Timestamp::from_parts(90_000, 1, 90_000).must()),
            end: Stamp::Present(Timestamp::from_parts(180_000, 1, 90_000).must()),
        };
        let cue = CompositedCue::new(
            span,
            CueBitmap::solid(8, 8, Rgba::WHITE).must(),
            crate::compositing::CueLayout::BOTTOM_CENTER,
            0,
        )
        .must();
        assert_eq!(cue.start, Some(1_000_000_000));
        assert_eq!(cue.end, Some(2_000_000_000));
        // The half-open rule at the boundaries.
        assert!(!cue.active_at(999_999_999));
        assert!(cue.active_at(1_000_000_000));
        assert!(cue.active_at(1_999_999_999));
        assert!(!cue.active_at(2_000_000_000));
    }

    #[test]
    fn missing_stamps_are_open_ended() {
        let span = TimeSpan {
            start: Stamp::Missing,
            end: Stamp::Present(Timestamp::from_parts(1_000_000_000, 1, 1_000_000_000).must()),
        };
        let cue = CompositedCue::new(
            span,
            CueBitmap::solid(8, 8, Rgba::WHITE).must(),
            crate::compositing::CueLayout::BOTTOM_CENTER,
            0,
        )
        .must();
        assert_eq!(cue.start, None);
        assert!(cue.active_at(i64::MIN));
        assert!(cue.active_at(0));
        assert!(!cue.active_at(1_000_000_000));
    }
}
