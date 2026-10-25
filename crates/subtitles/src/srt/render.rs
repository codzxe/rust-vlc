//! The SRT render path (card 5.M1 step 3): model [`Cue`] → the
//! compositing contract's [`CueLayout`] and the plain-text-box
//! bounds.
//!
//! Division of labor (doc 04 §5: "text rendering itself is a
//! renderer concern"): this module owns **geometry**, the renderer
//! (`renderer-wgpu`, a later card) owns fonts and rasterization.
//! The renderer supplies nominal metrics via [`TextMetrics`] (a
//! seam, not a font stack), and this module produces:
//!
//! * [`plain_text`] — the tag-stripped display text of a cue;
//! * [`text_box`] — the cue's box dimensions (longest line ×
//!   char width, line count × line height, baseline at the bottom)
//!   plus the mapped [`CueLayout`];
//! * [`place_plain_text`] — the full path: box → [`place_cue`] →
//!   the destination [`Rect`] on the presentation extent.
//!
//! ## Mapping model → compositing (exact, no floats — MP §8)
//!
//! * horizontal anchor: `position` percent → that fraction of the
//!   width; unset → the alignment's natural edge (0, 1/2, 1);
//! * vertical anchor: `line` percent → that fraction of the height;
//!   `line` count → `(n · line_height) / height` as an exact
//!   rational (negative from the bottom, clamped to `[0, height]`);
//!   unset → the alignment's natural edge (0, 1/2, 1);
//! * alignment: model Left/Center/Right → compositing
//!   Start/Center/End; model Top/Middle/Bottom → Top/Center/Bottom;
//! * safe margin: 5 % title-safe (the subtitle default — an
//!   unset-bottom cue anchored at v = 1 is pulled inside by the
//!   clamp, which is the broadcast bottom margin in practice).
//!
//! ## Bounds (D-2)
//!
//! [`place_plain_text`] builds a transient all-zero probe bitmap of
//! the box dimensions (clamped into the extent, at most
//! [`MAX_PROBE_PIXELS`] pixels = 16 MiB RGBA8) purely to reuse
//! [`place_cue`]'s pixel-exact placement; it is freed on return and
//! is never per-frame (plan once per cue, not per presented frame).

use crate::compositing::bitmap::{CueBitmap, CueError};
use crate::compositing::layout::{
    Anchor, CueAlignment, CueLayout, HorizontalAlign as LayoutHorizontal, Rational, Rect,
    SafeMargin, VerticalAlign as LayoutVertical, place_cue,
};
use crate::compositing::target::ComposeError;
use crate::model::cue::{Cue, CueText};
use crate::model::position::{HorizontalAlign, LinePlacement, Positioning, VerticalAlign};

/// The probe-bitmap area ceiling: 4 Mi pixels (16 MiB of RGBA8).
pub const MAX_PROBE_PIXELS: u64 = 1 << 22;

/// Nominal font metrics, supplied by the renderer (the seam: this
/// crate does not do fonts — doc 04 §5). `char_width_px` is the
/// advance used to size the box around the longest line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextMetrics {
    /// One text line's height in pixels.
    pub line_height_px: u32,
    /// One nominal character cell's width in pixels.
    pub char_width_px: u32,
}

impl TextMetrics {
    /// Construct from the two metrics.
    pub const fn new(line_height_px: u32, char_width_px: u32) -> Self {
        Self {
            line_height_px,
            char_width_px,
        }
    }
}

impl Default for TextMetrics {
    /// A nominal 1080p subtitle metrics (40 px line, 20 px cell) —
    /// a stand-in until the renderer's real font metrics arrive.
    fn default() -> Self {
        Self::new(40, 20)
    }
}

/// Typed render-path failures (MP §8).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SrtRenderError {
    /// The presentation extent is empty.
    ZeroExtent {
        /// The offending extent.
        extent: (u32, u32),
    },
    /// The anchor construction failed (a zero extent denominator).
    BadAnchor {
        /// The compositing-layer reason.
        source: CueError,
    },
    /// The probe bitmap construction failed.
    ProbeBitmap {
        /// The compositing-layer reason.
        source: CueError,
    },
    /// The box exceeds [`MAX_PROBE_PIXELS`] after clamping into the
    /// extent (a pathological text/extent combination; bounded
    /// memory wins over exact placement — D-2).
    ProbeTooLarge {
        /// The clamped box dimensions.
        size: (u32, u32),
    },
    /// `place_cue` failed.
    Compose {
        /// The compositing-layer reason.
        source: ComposeError,
    },
}

impl std::fmt::Display for SrtRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroExtent { extent } => write!(f, "presentation extent {extent:?} is empty"),
            Self::BadAnchor { source } => write!(f, "cue anchor construction failed: {source}"),
            Self::ProbeBitmap { source } => write!(f, "probe bitmap construction failed: {source}"),
            Self::ProbeTooLarge { size } => write!(
                f,
                "text box {size:?} exceeds the {}-pixel probe ceiling",
                MAX_PROBE_PIXELS
            ),
            Self::Compose { source } => write!(f, "placement failed: {source}"),
        }
    }
}

impl std::error::Error for SrtRenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BadAnchor { source } | Self::ProbeBitmap { source } => Some(source),
            Self::Compose { source } => Some(source),
            _ => None,
        }
    }
}

/// The plain display text of a cue (style runs concatenated, markup
/// already consumed by the parser). One allocation.
#[must_use]
pub fn plain_text(cue: &Cue) -> String {
    match &cue.text {
        CueText::Plain(s) => s.clone(),
        CueText::Styled(segments) => segments.iter().map(|s| s.text.as_str()).collect::<String>(),
    }
}

/// A cue's plain-text box: its pixel dimensions plus the compositing
/// [`CueLayout`] that places it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextBoxLayout {
    /// The compositing layout (anchor/alignment/safe margin).
    pub layout: CueLayout,
    /// Box width in pixels (longest line, clamped to the extent, ≥ 1).
    pub width_px: u32,
    /// Box height in pixels (line count, clamped to the extent, ≥ 1).
    pub height_px: u32,
    /// The baseline row (bottom-aligned box: equals `height_px`).
    pub baseline_px: u32,
}

/// The model → compositing layout mapping (see the module docs).
///
/// # Errors
/// [`SrtRenderError::ZeroExtent`] when `extent` is empty (a
/// line-count placement needs the height as its rational
/// denominator); [`SrtRenderError::BadAnchor`] never fires for
/// model-constructed positionings (they are valid by construction)
/// but guards pub-field escape hatches.
pub fn cue_layout(
    cue: &Cue,
    extent: (u32, u32),
    metrics: &TextMetrics,
) -> Result<CueLayout, SrtRenderError> {
    let zero = || SrtRenderError::ZeroExtent { extent };
    if extent.0 == 0 || extent.1 == 0 {
        return Err(zero());
    }
    let p: &Positioning = &cue.positioning;

    let u = match p.position {
        Some(percent) => Rational::new(percent.get() as u32, 100)
            .map_err(|e| SrtRenderError::BadAnchor { source: e })?,
        None => match p.alignment.horizontal {
            HorizontalAlign::Left => Rational::ZERO,
            HorizontalAlign::Center => Rational::new(1, 2).map_err(bad_anchor)?,
            HorizontalAlign::Right => Rational::ONE,
        },
    };
    let v = match p.line {
        Some(LinePlacement::Percent(percent)) => Rational::new(percent.get() as u32, 100)
            .map_err(|e| SrtRenderError::BadAnchor { source: e })?,
        Some(LinePlacement::LineNumber(n)) => {
            // Exact rational (n·line_height)/height, negative from the
            // bottom, clamped into [0, height] — the model's own
            // `line_number_to_px` semantics as a fraction.
            let h = i64::from(extent.1);
            let lh = i64::from(metrics.line_height_px);
            let raw = if n >= 0 {
                n.saturating_mul(lh)
            } else {
                h.saturating_add(n.saturating_mul(lh))
            };
            let num = raw.clamp(0, h) as u32;
            Rational::new(num, extent.1).map_err(|e| SrtRenderError::BadAnchor { source: e })?
        }
        None => match p.alignment.vertical {
            VerticalAlign::Top => Rational::ZERO,
            VerticalAlign::Middle => Rational::new(1, 2).map_err(bad_anchor)?,
            VerticalAlign::Bottom => Rational::ONE,
        },
    };
    let anchor = Anchor::new(u, v).map_err(|e| SrtRenderError::BadAnchor { source: e })?;
    Ok(CueLayout {
        anchor,
        alignment: CueAlignment {
            horizontal: match p.alignment.horizontal {
                HorizontalAlign::Left => LayoutHorizontal::Start,
                HorizontalAlign::Center => LayoutHorizontal::Center,
                HorizontalAlign::Right => LayoutHorizontal::End,
            },
            vertical: match p.alignment.vertical {
                VerticalAlign::Top => LayoutVertical::Top,
                VerticalAlign::Middle => LayoutVertical::Center,
                VerticalAlign::Bottom => LayoutVertical::Bottom,
            },
        },
        margin: SafeMargin::TitleSafe,
        reference: None,
    })
}

fn bad_anchor(e: CueError) -> SrtRenderError {
    SrtRenderError::BadAnchor { source: e }
}

/// Compute the cue's plain-text box and its compositing layout.
///
/// # Errors
/// [`SrtRenderError::ZeroExtent`] via [`cue_layout`].
pub fn text_box(
    cue: &Cue,
    metrics: &TextMetrics,
    extent: (u32, u32),
) -> Result<TextBoxLayout, SrtRenderError> {
    let layout = cue_layout(cue, extent, metrics)?;
    let text = plain_text(cue);
    // Characters, not bytes: Unicode honesty in the box width.
    let line_count = text.split('\n').count().max(1);
    let longest = text
        .split('\n')
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0)
        .max(1);
    let width = (longest as u64)
        .saturating_mul(u64::from(metrics.char_width_px))
        .min(u64::from(extent.0))
        .max(1) as u32;
    let height = (line_count as u64)
        .saturating_mul(u64::from(metrics.line_height_px))
        .min(u64::from(extent.1))
        .max(1) as u32;
    Ok(TextBoxLayout {
        layout,
        width_px: width,
        height_px: height,
        baseline_px: height,
    })
}

/// The full render path: cue → box → [`place_cue`] → destination
/// [`Rect`] on the presentation extent. The renderer rasterizes the
/// text INTO that rect (fonts are its concern); this function only
/// ever produces geometry.
///
/// # Errors
/// [`SrtRenderError`] — see the variants (empty extent, probe
/// ceiling, compositing-layer failures).
pub fn place_plain_text(
    cue: &Cue,
    metrics: &TextMetrics,
    extent: (u32, u32),
) -> Result<Rect, SrtRenderError> {
    let box_layout = text_box(cue, metrics, extent)?;
    // The probe: box dims already clamped into the extent by
    // `text_box`; enforce the area ceiling (D-2).
    let pixels = u64::from(box_layout.width_px) * u64::from(box_layout.height_px);
    if pixels > MAX_PROBE_PIXELS {
        return Err(SrtRenderError::ProbeTooLarge {
            size: (box_layout.width_px, box_layout.height_px),
        });
    }
    let probe_bytes = pixels as usize * 4;
    let probe = CueBitmap::from_rgba(
        box_layout.width_px,
        box_layout.height_px,
        box_layout.baseline_px,
        vec![0u8; probe_bytes],
    )
    .map_err(|e| SrtRenderError::ProbeBitmap { source: e })?;
    place_cue(&probe, &box_layout.layout, extent).map_err(|e| SrtRenderError::Compose { source: e })
}

#[cfg(test)]
mod tests {
    use super::{TextMetrics, cue_layout, place_plain_text, plain_text, text_box};
    use crate::compositing::layout::Rect;
    use crate::model::cue::{Cue, CueId, CueText, StyledSegment};
    use crate::model::position::{
        Alignment, HorizontalAlign, LinePlacement, Percent, Positioning, VerticalAlign,
    };
    use crate::model::style::TextStyle;
    use crate::model::time::CueInterval;
    use crate::srt::render::SrtRenderError;
    use std::error::Error;

    fn cue(text: CueText, positioning: Positioning) -> Result<Cue, Box<dyn Error>> {
        Ok(Cue::new(
            CueId::new(1),
            CueInterval::new(0, 1_000_000)?,
            text,
            positioning,
        ))
    }

    const HD: (u32, u32) = (1920, 1080);
    const METRICS: TextMetrics = TextMetrics::new(40, 20);

    #[test]
    fn plain_text_joins_segments() -> Result<(), Box<dyn Error>> {
        let styled = CueText::Styled(vec![
            StyledSegment {
                text: "Bold".into(),
                style: TextStyle::new(true, false, false, None),
            },
            StyledSegment {
                text: " tail".into(),
                style: TextStyle::PLAIN,
            },
        ]);
        assert_eq!(
            plain_text(&cue(styled, Positioning::default())?),
            "Bold tail"
        );
        assert_eq!(
            plain_text(&cue(CueText::Plain("p".into()), Positioning::default())?),
            "p"
        );
        Ok(())
    }

    #[test]
    fn default_cue_lands_bottom_center_inside_title_safe() -> Result<(), Box<dyn Error>> {
        // "Hello" = 5 chars × 20 px = 100 px wide, 1 line = 40 px.
        // Anchor u=1/2, v=1 (model Bottom), bottom-center aligned:
        // y0 = 1080 - 40 = 1040, then the 5 % title-safe clamp pulls
        // the bottom edge to ≤ 1026: y = 1026 - 40 = 986.
        let c = cue(CueText::Plain("Hello".into()), Positioning::default())?;
        let rect = place_plain_text(&c, &METRICS, HD)?;
        assert_eq!(
            rect,
            Rect {
                x: (1920 - 100) / 2,
                y: 986,
                width: 100,
                height: 40
            }
        );
        Ok(())
    }

    #[test]
    fn multiline_and_long_text_boxes() -> Result<(), Box<dyn Error>> {
        let c = cue(
            CueText::Plain("one\ntwo\nthree".into()),
            Positioning::default(),
        )?;
        let b = text_box(&c, &METRICS, HD)?;
        // Longest line "three" = 5 chars × 20 px; 3 lines × 40 px.
        assert_eq!((b.width_px, b.height_px, b.baseline_px), (100, 120, 120));
        // Text wider than the extent clamps to the extent.
        let long = "x".repeat(500);
        let c = cue(CueText::Plain(long), Positioning::default())?;
        let b = text_box(&c, &METRICS, HD)?;
        assert_eq!(b.width_px, 1920);
        Ok(())
    }

    #[test]
    fn percent_and_line_placements_map_exactly() -> Result<(), Box<dyn Error>> {
        // position 25 % + line 75 % (bottom-right alignment).
        let p = Positioning::new(
            Alignment::new(VerticalAlign::Bottom, HorizontalAlign::Right),
            Some(LinePlacement::Percent(Percent::new(75)?)),
            Some(Percent::new(25)?),
        );
        let c = cue(CueText::Plain("ab".into()), p)?;
        let rect = place_plain_text(&c, &METRICS, HD)?;
        // Anchor (480, 810), End/Bottom: x0 = 480-40 = 440 (inside the
        // safe box [96, 1824-40]), y0 = 810-40 = 770 — no clamps.
        assert_eq!(
            rect,
            Rect {
                x: 440,
                y: 770,
                width: 40,
                height: 40
            }
        );
        // Line numbers: +2 rows from the top, top-left alignment:
        // anchor v = (2·40)/1080 = 80/1080, y0 = 80 exactly.
        let p = Positioning::new(
            Alignment::new(VerticalAlign::Top, HorizontalAlign::Left),
            Some(LinePlacement::LineNumber(2)),
            None,
        );
        let c = cue(CueText::Plain("ab".into()), p)?;
        let rect = place_plain_text(&c, &METRICS, HD)?;
        // x clamps into the 5 % box: 0 → 96; y = 80 stays (>= 54).
        assert_eq!(rect.y, 80);
        assert_eq!(rect.x, 96);
        // -1 row from the bottom: v = (1080-40)/1080, y0 = 1040-40 =
        // 1000 → bottom edge 1040 > safe 1026 → y = 986.
        let p = Positioning::new(
            Alignment::new(VerticalAlign::Top, HorizontalAlign::Left),
            Some(LinePlacement::LineNumber(-1)),
            None,
        );
        let c = cue(CueText::Plain("ab".into()), p)?;
        let rect = place_plain_text(&c, &METRICS, HD)?;
        assert_eq!(rect.y, 986);
        Ok(())
    }

    #[test]
    fn layout_mapping_fallbacks() -> Result<(), Box<dyn Error>> {
        let c = cue(CueText::Plain("x".into()), Positioning::default())?;
        let l = cue_layout(&c, HD, &METRICS)?;
        assert_eq!(l.anchor.u.num, 1);
        assert_eq!(l.anchor.u.den, 2);
        assert_eq!(l.anchor.v.num, 1);
        assert_eq!(l.anchor.v.den, 1);
        Ok(())
    }

    #[test]
    fn typed_failures() -> Result<(), Box<dyn Error>> {
        let c = cue(CueText::Plain("x".into()), Positioning::default())?;
        assert_eq!(
            place_plain_text(&c, &METRICS, (0, 1080)),
            Err(SrtRenderError::ZeroExtent { extent: (0, 1080) })
        );
        // A pathological text/extent combination over the probe
        // ceiling (D-2: bounded memory wins) is typed-refused:
        // 3 lines × 3000 chars × 20 px = 60 000 × 120 px = 7.2 M px.
        let row = "x".repeat(3000);
        let big = format!("{row}\n{row}\n{row}");
        let c = cue(CueText::Plain(big), Positioning::default())?;
        assert!(matches!(
            place_plain_text(&c, &METRICS, (65536, 65536)),
            Err(SrtRenderError::ProbeTooLarge { .. })
        ));
        // Under a normal extent the same cue places fine (clamped).
        assert!(place_plain_text(&c, &METRICS, HD).is_ok());
        Ok(())
    }
}
