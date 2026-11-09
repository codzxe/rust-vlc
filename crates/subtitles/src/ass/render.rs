//! The ASS plain-text render path (card 5.M2b, step 2): one plain-text
//! cue → the compositing contract's [`CompositedCue`] (an RGBA8
//! [`CueBitmap`] + [`CueLayout`] + z + span).
//!
//! ## Scope and the entry seam
//!
//! This module defines its own entry types ([`AssRenderCue`],
//! [`AssRenderStyle`]) STANDALONE — the parallel 5.M2a half owns the
//! parser (`ass/parser.rs`), and the Lead wires parser output into
//! these types at merge (either by having the parser produce them or
//! through an adapter; both halves' types are intentionally small and
//! structural so the union is mechanical).
//!
//! ## Glyph metrics policy (honest, documented)
//!
//! **Fonts belong to the renderer** (doc 04 §5: "text rendering itself
//! is a renderer concern") — this crate has no font stack and never
//! will. To produce a real bitmap anyway, this path uses
//! **monospace-widest-char metrics**:
//!
//! * every codepoint advances one cell of `3/5 em` (0.6 em — the
//!   widest-glyph advance of the common monospace faces, e.g. DejaVu
//!   Sans Mono 1233/2048 ≈ 0.602 em, Liberation Mono ≈ 0.600 em),
//!   rounded half-up in exact integer arithmetic (no floats near a
//!   pixel, the compositor's own discipline);
//! * the line box is `6/5 em` (1.2 em) with the ascent at 80 % of it
//!   (the typical monospace ascent/descent split);
//! * every non-space codepoint is rasterized as a SOLID CELL BLOCK in
//!   the primary colour; U+0020 is a blank cell.
//!
//! The result is placeholder glyph coverage: correct geometry (box,
//! baseline, multi-line stacking, colour, placement, timing) with
//! deliberately crude pixels. When the real font stack lands in
//! `renderer-wgpu`, only [`GlyphMetrics`] and the fill loop change —
//! the placement/colour/timing plumbing below them is the contract.
//!
//! `[HONEST NOTE]` no East-Asian wide-width handling: a CJK codepoint
//! also occupies one cell, so CJK-heavy lines UNDER-estimate width
//! relative to real fonts (which give CJK a full em). The 0.6 em
//! monospace advance overshoots proportional Latin text by roughly the
//! same order, so mixed-script cues land in the right neighbourhood,
//! never off the screen (the safe-area clamp guarantees that floor).
//!
//! ## Colour mapping (card step 3)
//!
//! ASS stores `PrimaryColour` as `&HAABBGGRR` — a u32 whose high byte
//! is an INVERTED alpha (00 = opaque, FF = fully transparent) followed
//! by blue, green, red. [`ass_colour_to_model`] converts to the model's
//! straight-alpha [`Color`](crate::model::style::Color) (the
//! style-token color subset); the parser hands over the already-parsed
//! u32, so the byte order policy lives in exactly one place, here.
//!
//! ## No panics
//!
//! Every failure is [`AssRenderError`] (MP §8): typed refusal for
//! empty text, zero/oversized font sizes, reversed intervals, margins
//! beyond the script resolution, and bitmaps past the [`MAX_BITMAP_BYTES`]
//! guard (which also keeps the allocation bounded — no
//! capacity-overflow abort).

use crate::ass::layout::{self, AssAlignment, AssMargins};
use crate::compositing::bitmap::{CompositedCue, CueBitmap, CueError};
use crate::font::MetricsError;
use crate::model::style::{Color as ModelColor, TextStyle};
use crate::model::{CueId, ModelError};
use media_types::time::{Stamp, TimeError, TimeSpan, Timestamp};

// The glyph-metrics guard + type moved to the font-stack seam
// (issue #17 G-5, `crate::font::pipeline`) — re-exported here so the
// historical `ass::render::GlyphMetrics` path (and its tests) keep
// resolving; the render path is now a CONSUMER of the seam.
pub use crate::font::{GlyphMetrics, MAX_FONT_SIZE_PX};

/// The bitmap byte guard: `width × height × 4` beyond this is
/// typed-refused. 64 MiB covers a full 4K-frame-sized cue with room to
/// spare while keeping the one allocation bounded (no abort path).
pub const MAX_BITMAP_BYTES: u64 = 64 * 1024 * 1024;

/// Typed failures of the ASS plain-text render path (MP §8: data,
/// never a panic).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AssRenderError {
    /// An ASS alignment outside the 1–9 numpad.
    BadAlignment {
        /// The offending raw value.
        value: u8,
    },
    /// `FontSize` of zero — no glyph box exists.
    ZeroFontSize,
    /// `FontSize` beyond [`MAX_FONT_SIZE_PX`].
    FontSizeTooLarge {
        /// The offending size, script px.
        size_px: u32,
        /// The accepted maximum.
        max: u32,
    },
    /// The cue text has no renderable line (empty, or only `\N`).
    EmptyText,
    /// `end < start` (parity with the model's `CueInterval` rule;
    /// zero-duration is legal, reversal is not).
    EndBeforeStart {
        /// The rejected start, µs.
        start_us: i64,
        /// The rejected end, µs.
        end_us: i64,
    },
    /// A zero script-resolution axis.
    ZeroPlayRes {
        /// The offending `(PlayResX, PlayResY)`.
        play_res: (u32, u32),
    },
    /// A margin beyond its script axis (typed refusal; libass would
    /// clamp — see `ass::layout` docs).
    MarginExceedsPlayRes {
        /// Which margin (`"left"` / `"right"` / `"vertical"`).
        axis: &'static str,
        /// The offending margin, script px.
        margin: u32,
        /// The script extent of that axis, px.
        extent: u32,
    },
    /// The rasterized bitmap would exceed [`MAX_BITMAP_BYTES`] (or a
    /// `u32` dimension).
    TooLarge {
        /// The offending bitmap width, px.
        width: u32,
        /// The offending bitmap height, px.
        height: u32,
    },
    /// A compositing-contract failure (bitmap validation, defensive
    /// rational/anchor construction).
    Cue(CueError),
    /// A cue-model failure (defensive integer-percent construction).
    Model(ModelError),
    /// A timestamp conversion failure (negative or overflowing µs).
    Time(TimeError),
    /// The internal `1/1_000_000` time base failed to construct —
    /// defensive only: the base is compile-time valid, so this variant
    /// exists to keep the no-panic rule total, not to fire.
    Timebase,
}

impl std::fmt::Display for AssRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssRenderError::BadAlignment { value } => {
                write!(f, "ASS alignment {value} outside the 1-9 numpad")
            }
            AssRenderError::ZeroFontSize => f.write_str("ASS FontSize is zero"),
            AssRenderError::FontSizeTooLarge { size_px, max } => {
                write!(f, "ASS FontSize {size_px} px beyond the {max} px guard")
            }
            AssRenderError::EmptyText => f.write_str("ASS cue text has no renderable line"),
            AssRenderError::EndBeforeStart { start_us, end_us } => write!(
                f,
                "ASS cue end {end_us} µs precedes start {start_us} µs (zero-duration is legal, reversal is not)"
            ),
            AssRenderError::ZeroPlayRes { play_res } => {
                write!(f, "ASS script resolution is zero: {play_res:?}")
            }
            AssRenderError::MarginExceedsPlayRes {
                axis,
                margin,
                extent,
            } => write!(
                f,
                "ASS {axis} margin {margin} px exceeds its {extent} px script axis"
            ),
            AssRenderError::TooLarge { width, height } => write!(
                f,
                "rasterized cue bitmap {width}x{height} px exceeds the byte guard"
            ),
            AssRenderError::Cue(err) => write!(f, "compositing contract rejected the cue: {err}"),
            AssRenderError::Model(err) => write!(f, "cue model rejected a value: {err}"),
            AssRenderError::Time(err) => write!(f, "cue timestamp conversion failed: {err}"),
            AssRenderError::Timebase => f.write_str("the 1/1e6 time base failed to construct"),
        }
    }
}

impl std::error::Error for AssRenderError {}

impl From<CueError> for AssRenderError {
    fn from(err: CueError) -> AssRenderError {
        AssRenderError::Cue(err)
    }
}

impl From<ModelError> for AssRenderError {
    fn from(err: ModelError) -> AssRenderError {
        AssRenderError::Model(err)
    }
}

impl From<TimeError> for AssRenderError {
    fn from(err: TimeError) -> AssRenderError {
        AssRenderError::Time(err)
    }
}

impl From<MetricsError> for AssRenderError {
    fn from(err: MetricsError) -> AssRenderError {
        match err {
            MetricsError::ZeroFontSize => AssRenderError::ZeroFontSize,
            MetricsError::FontSizeTooLarge { size_px, max } => {
                AssRenderError::FontSizeTooLarge { size_px, max }
            }
        }
    }
}

/// The style inputs the plain-text render path needs (card step 3's
/// default-style mapping). Structural and small by design — the 5.M2a
/// parser seam projects ASS `[V4+ Styles]` fields onto this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssRenderStyle {
    /// The 1–9 numpad alignment (`Alignment` / `\an`).
    pub alignment: AssAlignment,
    /// Style/event margins, script px (`MarginL/R/V`).
    pub margins: AssMargins,
    /// `PrimaryColour` as the parsed `&HAABBGGRR` u32 (byte order and
    /// alpha inversion are handled by [`ass_colour_to_model`]).
    pub primary_colour: u32,
    /// `FontSize` in script (PlayRes) pixels.
    pub font_size_px: u32,
}

impl AssRenderStyle {
    /// The common `Default` style template: bottom-center, 10/10/10
    /// margins, opaque white (`&H00FFFFFF`), 20 script px.
    pub const ASS_DEFAULT: Self = Self {
        alignment: AssAlignment::BOTTOM_CENTER,
        margins: AssMargins::ASS_DEFAULT,
        primary_colour: 0x00FF_FFFF, // &H00FFFFFF
        font_size_px: 20,
    };

    /// The style-token subset projection: ASS `PrimaryColour` → the
    /// model's [`TextStyle`] color token. Bold/italic/underline come
    /// from the ASS `\b`/`\i`/`\u` flags — the parser's projection,
    /// not this path's (they do not change the bitmap geometry).
    #[must_use]
    pub fn style_token(&self) -> TextStyle {
        TextStyle::new(
            false,
            false,
            false,
            Some(ass_colour_to_model(self.primary_colour)),
        )
    }
}

/// One plain-text ASS cue, ready to render (the entry type — see the
/// module docs for the parser seam).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssRenderCue {
    /// Stable identity (ASS cue index / event number).
    pub id: CueId,
    /// Interval start, µs.
    pub start_us: i64,
    /// Interval end, µs (half-open `[start, end)` once composed).
    pub end_us: i64,
    /// Plain text. Override tags are the parser's concern (stripped
    /// upstream); the forced line break `\N` survives into this path
    /// and forces a new line (see [`split_lines`]).
    pub text: String,
    /// Style inputs (alignment, margins, colour, font size).
    pub style: AssRenderStyle,
    /// Z-order (the ASS `Layer` field; higher draws on top).
    pub z: u16,
}

/// Convert an ASS `&HAABBGGRR` colour to the model's straight-alpha
/// [`ModelColor`].
///
/// The u32 layout (as parsed from the literal): bits 31–24 alpha
/// (INVERTED — 00 opaque, FF transparent), 23–16 blue, 15–8 green,
/// 7–0 red. The classic ASS default `&H00FFFFFF` is opaque white; the
/// equally classic `&H0000FFFF` is opaque yellow.
#[must_use]
pub const fn ass_colour_to_model(value: u32) -> ModelColor {
    let r = (value & 0xFF) as u8;
    let g = ((value >> 8) & 0xFF) as u8;
    let b = ((value >> 16) & 0xFF) as u8;
    let ass_alpha = ((value >> 24) & 0xFF) as u8; // 00 = opaque
    ModelColor::new(r, g, b, 255 - ass_alpha)
}

/// Split cue text into display lines.
///
/// Break points: the ASS forced break `\N` (the two-byte literal,
/// backslash + `N`); the two-byte literal `\n` (the ASS soft break,
/// honored as a hard break here because this path does no reflow —
/// `WrapStyle` reflow is out of scope, honestly noted); and a bare
/// U+000A (for parsers that pre-normalize `\N` to a real newline).
/// All break bytes are ASCII, so the byte-index slicing never cuts a
/// multi-byte codepoint.
#[must_use]
pub fn split_lines(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                lines.push(&text[start..i]);
                i += 1;
                start = i;
            }
            b'\\' if i + 1 < bytes.len() && (bytes[i + 1] == b'N' || bytes[i + 1] == b'n') => {
                lines.push(&text[start..i]);
                i += 2;
                start = i;
            }
            _ => i += 1,
        }
    }
    lines.push(&text[start..]);
    lines
}

/// Render one plain-text ASS cue to the compositing contract.
///
/// The output carries: the rasterized [`CueBitmap`] (placeholder
/// glyph coverage — module docs), the exact-rational [`CueLayout`]
/// from [`layout::cue_layout`], the z-order, and the half-open span in
/// nanoseconds (converted once, here — never per frame).
///
/// # Errors
/// [`AssRenderError`] — see the variants; no input panics.
pub fn render_plain_text_cue(
    cue: &AssRenderCue,
    play_res: (u32, u32),
) -> Result<CompositedCue, AssRenderError> {
    if cue.end_us < cue.start_us {
        return Err(AssRenderError::EndBeforeStart {
            start_us: cue.start_us,
            end_us: cue.end_us,
        });
    }
    let metrics = GlyphMetrics::from_font_size(cue.style.font_size_px)?;
    let lines = split_lines(&cue.text);
    let max_chars = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    if max_chars == 0 {
        return Err(AssRenderError::EmptyText);
    }

    // Dimensions in u64, then u32-checked (no silent truncation).
    let width = max_chars as u64 * u64::from(metrics.advance_px);
    let height = lines.len() as u64 * u64::from(metrics.line_height_px);
    let (width, height) = {
        let w = u32::try_from(width);
        let h = u32::try_from(height);
        match (w, h) {
            (Ok(w), Ok(h)) => (w, h),
            _ => {
                return Err(AssRenderError::TooLarge {
                    width: u32::MAX,
                    height: u32::MAX,
                });
            }
        }
    };
    let total_bytes = u64::from(width) * u64::from(height) * 4;
    if total_bytes > MAX_BITMAP_BYTES {
        return Err(AssRenderError::TooLarge { width, height });
    }

    // The LAST line's baseline is the cue baseline (the anchor a
    // `Baseline`-aligned layout would pin).
    let baseline =
        (lines.len() as u64 - 1) * u64::from(metrics.line_height_px) + u64::from(metrics.ascent_px);

    // Rasterize: one solid cell block per non-space codepoint.
    let color = ass_colour_to_model(cue.style.primary_colour);
    let mut rgba = vec![0u8; total_bytes as usize];
    for (li, line) in lines.iter().enumerate() {
        let top = li as u64 * u64::from(metrics.line_height_px);
        for (ci, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let cell_x = ci as u64 * u64::from(metrics.advance_px);
            for y in top..top + u64::from(metrics.ascent_px) {
                for x in cell_x..cell_x + u64::from(metrics.advance_px) {
                    let off = ((y * u64::from(width) + x) * 4) as usize;
                    rgba[off] = color.r;
                    rgba[off + 1] = color.g;
                    rgba[off + 2] = color.b;
                    rgba[off + 3] = color.a;
                }
            }
        }
    }

    let bitmap = CueBitmap::from_rgba(width, height, baseline as u32, rgba)?;
    let cue_layout = layout::cue_layout(cue.style.alignment, cue.style.margins, play_res)?;
    // The 1/1e6 base is compile-time valid; the map_err keeps the
    // no-panic rule total against future edits (Timebase, above).
    let start =
        Timestamp::from_parts(cue.start_us, 1, 1_000_000).map_err(|_| AssRenderError::Timebase)?;
    let end =
        Timestamp::from_parts(cue.end_us, 1, 1_000_000).map_err(|_| AssRenderError::Timebase)?;
    CompositedCue::new(
        TimeSpan {
            start: Stamp::Present(start),
            end: Stamp::Present(end),
        },
        bitmap,
        cue_layout,
        cue.z,
    )
    .map_err(AssRenderError::from)
}

#[cfg(test)]
mod tests {
    use super::{
        AssRenderCue, AssRenderError, AssRenderStyle, GlyphMetrics, MAX_BITMAP_BYTES,
        ass_colour_to_model, render_plain_text_cue, split_lines,
    };
    use crate::ass::layout::{AssAlignment, AssMargins, DEFAULT_PLAY_RES};
    use crate::compositing::SubtitleCompositor;
    use crate::compositing::bitmap::CueError;
    use crate::compositing::layout::{
        HorizontalAlign as CompHorizontal, Rect, VerticalAlign as CompVertical, place_cue,
    };
    use crate::compositing::target::{Rgba, Rgba8Surface};
    use crate::model::CueId;
    use crate::model::style::Color as ModelColor;
    use media_types::time::Timestamp;
    use testkit::Must;

    /// Yellow `&H0000FFFF` — the classic ASS default primary colour.
    const ASS_YELLOW: u32 = 0x0000_FFFF;

    fn cue(text: &str, alignment: AssAlignment) -> AssRenderCue {
        AssRenderCue {
            id: CueId::new(1),
            start_us: 0,
            end_us: 1_000_000,
            text: text.into(),
            style: AssRenderStyle {
                alignment,
                margins: AssMargins {
                    left: 20,
                    right: 20,
                    vertical: 30,
                },
                primary_colour: ASS_YELLOW,
                font_size_px: 20,
            },
            z: 0,
        }
    }

    #[test]
    fn colour_conversion_table() {
        // (&HAABBGGRR literal, expected straight-alpha RGBA)
        let table: &[(u32, ModelColor)] = &[
            (0x00FF_FFFF, ModelColor::new(255, 255, 255, 255)), // white opaque
            (0x0000_FFFF, ModelColor::new(255, 255, 0, 255)),   // yellow opaque
            (0x0000_00FF, ModelColor::new(255, 0, 0, 255)),     // red opaque
            (0x0000_FF00, ModelColor::new(0, 255, 0, 255)),     // green opaque
            (0x00FF_0000, ModelColor::new(0, 0, 255, 255)),     // blue opaque
            (0xFF00_0000, ModelColor::new(0, 0, 0, 0)),         // black, fully transparent
            (0x80FF_FFFF, ModelColor::new(255, 255, 255, 127)), // white, half alpha
            (0xFFFF_FFFF, ModelColor::new(255, 255, 255, 0)),   // white, fully transparent
        ];
        for &(raw, expected) in table {
            assert_eq!(ass_colour_to_model(raw), expected, "raw {raw:#010x}");
        }
        // The style-token projection carries the colour into the
        // model's subset (card step 3).
        let token = AssRenderStyle::ASS_DEFAULT.style_token();
        assert_eq!(token.color, Some(ModelColor::new(255, 255, 255, 255)));
    }

    #[test]
    fn metrics_pin_the_documented_ratios() {
        // 20 px: 0.6 em = 12, 1.2 em = 24, 80 % of 24 = 19.
        let m = GlyphMetrics::from_font_size(20).must();
        assert_eq!((m.advance_px, m.line_height_px, m.ascent_px), (12, 24, 19));
        // 10 px: 6, 12, 10 — exact, no rounding ambiguity.
        let m = GlyphMetrics::from_font_size(10).must();
        assert_eq!((m.advance_px, m.line_height_px, m.ascent_px), (6, 12, 10));
        // 25 px: 15, 30, 24.
        let m = GlyphMetrics::from_font_size(25).must();
        assert_eq!((m.advance_px, m.line_height_px, m.ascent_px), (15, 30, 24));
        assert_eq!(
            GlyphMetrics::from_font_size(0).map_err(AssRenderError::from),
            Err(AssRenderError::ZeroFontSize)
        );
        assert_eq!(
            GlyphMetrics::from_font_size(8193).map_err(AssRenderError::from),
            Err(AssRenderError::FontSizeTooLarge {
                size_px: 8193,
                max: 8192
            })
        );
    }

    #[test]
    fn line_splitting_honors_all_three_breaks() {
        assert_eq!(split_lines("a\\Nb"), vec!["a", "b"]);
        assert_eq!(split_lines("a\\nb"), vec!["a", "b"]); // literal \n
        assert_eq!(split_lines("a\nb"), vec!["a", "b"]); // U+000A
        assert_eq!(split_lines("é\\N漢"), vec!["é", "漢"]);
        assert_eq!(split_lines("a\\N"), vec!["a", ""]);
        assert_eq!(split_lines("\\N"), vec!["", ""]);
        assert_eq!(split_lines(""), vec![""]);
        assert_eq!(split_lines("plain"), vec!["plain"]);
        // A lone trailing backslash is text, not a break.
        assert_eq!(split_lines("a\\"), vec!["a\\"]);
    }

    #[test]
    fn nine_position_grid_lands_pixel_exact() {
        // PlayRes == extent (no scaling), margins 20/20/30, "Hi" at
        // 20 px -> a 24x24 bitmap. Expected rects hand-derived through
        // place_cue's exact math (anchors 5/96, 1/2, 91/96 x;
        // 43/48, 1/2, 5/48 y; 5 % title-safe insets 19/14 px).
        let expected: [(u32, u32); 9] = [
            (20, 234),  // 1 bottom-left
            (180, 234), // 2 bottom-center
            (340, 234), // 3 bottom-right
            (20, 132),  // 4 middle-left
            (180, 132), // 5 middle-center
            (340, 132), // 6 middle-right
            (20, 30),   // 7 top-left
            (180, 30),  // 8 top-center
            (340, 30),  // 9 top-right
        ];
        for (i, alignment) in AssAlignment::ALL.iter().enumerate() {
            let rendered = render_plain_text_cue(&cue("Hi", *alignment), (384, 288)).must();
            let rect = place_cue(&rendered.bitmap, &rendered.layout, (384, 288)).must();
            assert_eq!(
                rect,
                Rect {
                    x: expected[i].0,
                    y: expected[i].1,
                    width: 24,
                    height: 24
                },
                "alignment {} ({})",
                alignment.get(),
                alignment.to_model_alignment().code()
            );
            // The compositor-side alignment matches the grid thirds.
            let horizontal = match alignment.horizontal() {
                crate::ass::layout::AssHorizontal::Left => CompHorizontal::Start,
                crate::ass::layout::AssHorizontal::Center => CompHorizontal::Center,
                crate::ass::layout::AssHorizontal::Right => CompHorizontal::End,
            };
            assert_eq!(rendered.layout.alignment.horizontal, horizontal);
            let vertical = match alignment.vertical() {
                crate::ass::layout::AssVertical::Bottom => CompVertical::Bottom,
                crate::ass::layout::AssVertical::Middle => CompVertical::Center,
                crate::ass::layout::AssVertical::Top => CompVertical::Top,
            };
            assert_eq!(rendered.layout.alignment.vertical, vertical);
        }
    }

    #[test]
    fn zero_margins_still_land_inside_the_title_safe_box() {
        // Margin 0 does NOT mean "touch the bezel" here: the
        // compositor's 5 % clamp engages (documented deviation).
        let mut c = cue("Hi", AssAlignment::BOTTOM_LEFT);
        c.style.margins = AssMargins::ZERO;
        let rendered = render_plain_text_cue(&c, (384, 288)).must();
        let rect = place_cue(&rendered.bitmap, &rendered.layout, (384, 288)).must();
        assert_eq!(
            rect,
            Rect {
                x: 19,  // clamped from 0 into the 5 % inset
                y: 250, // clamped so the bottom edge sits at 274, not 288
                width: 24,
                height: 24
            }
        );
    }

    #[test]
    fn margins_and_bitmap_scale_with_the_video_extent() {
        // 2x presentation extent: the 24x24 bitmap scales to 48x48 and
        // the margin anchors double (20 -> 40, 258-24 -> 516-48).
        let rendered =
            render_plain_text_cue(&cue("Hi", AssAlignment::BOTTOM_LEFT), (384, 288)).must();
        let rect = place_cue(&rendered.bitmap, &rendered.layout, (768, 576)).must();
        assert_eq!(
            rect,
            Rect {
                x: 40,
                y: 468,
                width: 48,
                height: 48
            }
        );
    }

    #[test]
    fn multi_line_stacking_via_hard_break() {
        let rendered = render_plain_text_cue(
            &cue("one\\Ntwo\\Nthree", AssAlignment::BOTTOM_CENTER),
            DEFAULT_PLAY_RES,
        )
        .must();
        let bmp = &rendered.bitmap;
        // 5 widest chars x 12 = 60 wide; 3 lines x 24 = 72 tall; the
        // baseline is the LAST line's: 2x24 + 19 = 67.
        assert_eq!((bmp.width, bmp.height, bmp.baseline), (60, 72, 67));
        let yellow = Rgba {
            r: 255,
            g: 255,
            b: 0,
            a: 255,
        };
        let transparent = Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
        // Line 0 occupies rows 0..19 ("one": columns 0..36).
        assert_eq!(bmp.pixel(0, 0), yellow);
        assert_eq!(bmp.pixel(35, 0), yellow);
        assert_eq!(bmp.pixel(36, 0), transparent);
        assert_eq!(bmp.pixel(0, 18), yellow);
        // The descent gap between lines is transparent.
        assert_eq!(bmp.pixel(0, 19), transparent);
        assert_eq!(bmp.pixel(0, 23), transparent);
        // Line 1 starts at row 24, line 2 at row 48 ("three" spans
        // the full 60-px width).
        assert_eq!(bmp.pixel(0, 24), yellow);
        assert_eq!(bmp.pixel(0, 48), yellow);
        assert_eq!(bmp.pixel(59, 48), yellow);
        assert_eq!(bmp.pixel(0, 66), yellow);
        // Below the last line's glyph block: transparent.
        assert_eq!(bmp.pixel(0, 67), transparent);
        assert_eq!(bmp.pixel(0, 71), transparent);
        // Lines are left-aligned inside the box (documented).
        assert_eq!(bmp.pixel(59, 0), transparent);

        // Spaces render as blank cells but still occupy their cell.
        let rendered =
            render_plain_text_cue(&cue("a b", AssAlignment::BOTTOM_CENTER), DEFAULT_PLAY_RES)
                .must();
        let bmp = &rendered.bitmap;
        assert_eq!(bmp.width, 36);
        assert_eq!(bmp.pixel(0, 0), yellow);
        assert_eq!(bmp.pixel(12, 0), transparent); // the space cell
        assert_eq!(bmp.pixel(24, 0), yellow);
    }

    #[test]
    fn composed_cue_carries_identity_timing_and_z() {
        let mut c = cue("Hi", AssAlignment::BOTTOM_CENTER);
        c.id = CueId::new(42);
        c.start_us = 1_500_000;
        c.end_us = 3_000_000;
        c.z = 7;
        let rendered = render_plain_text_cue(&c, (384, 288)).must();
        assert_eq!(rendered.z, 7);
        assert_eq!(rendered.start, Some(1_500_000_000));
        assert_eq!(rendered.end, Some(3_000_000_000));
        // Half-open [t0, t1) at both boundaries.
        assert!(!rendered.active_at(1_499_999_999));
        assert!(rendered.active_at(1_500_000_000));
        assert!(rendered.active_at(2_999_999_999));
        assert!(!rendered.active_at(3_000_000_000));
        // The layout is bottom-center with the default margins.
        assert_eq!(rendered.layout.reference, Some((384, 288)));
        assert_eq!(rendered.layout.alignment.vertical, CompVertical::Bottom);

        // Zero-duration cues are legal (the model's parity rule).
        let mut c = cue("Hi", AssAlignment::BOTTOM_CENTER);
        c.start_us = 5_000_000;
        c.end_us = 5_000_000;
        assert!(render_plain_text_cue(&c, (384, 288)).is_ok());
        // Reversal is typed.
        c.end_us = 4_999_999;
        assert_eq!(
            render_plain_text_cue(&c, (384, 288)),
            Err(AssRenderError::EndBeforeStart {
                start_us: 5_000_000,
                end_us: 4_999_999
            })
        );
    }

    #[test]
    fn end_to_end_compose_draws_the_cue_pixels() {
        // The full contract: render -> place -> blend onto a surface.
        // Default style (bottom-center, 10/10/10 margins, yellow):
        // anchor v = 278/288 -> y0 = 254, which the 5 % title-safe
        // clamp pulls to 250 (bottom edge at the 274 inset line);
        // x: 192 - 12 = 180.
        let mut c = cue("Hi", AssAlignment::BOTTOM_CENTER);
        c.style = AssRenderStyle::ASS_DEFAULT;
        c.style.primary_colour = ASS_YELLOW;
        let rendered = render_plain_text_cue(&c, (384, 288)).must();
        let mut surface = Rgba8Surface::new((384, 288), Rgba::BLACK).must();
        let mut compositor = SubtitleCompositor::new();
        let now = Timestamp::from_parts(500_000, 1, 1_000_000).must();
        let drawn = compositor
            .compose(std::slice::from_ref(&rendered), now, &mut surface)
            .must();
        assert_eq!(drawn, 1);
        let yellow = Rgba {
            r: 255,
            g: 255,
            b: 0,
            a: 255,
        };
        assert_eq!(surface.pixel(180, 250).must(), yellow);
        assert_eq!(surface.pixel(203, 250).must(), yellow);
        // Outside the placed rect and in the transparent descent rows:
        // untouched video black.
        assert_eq!(surface.pixel(179, 250).must(), Rgba::BLACK);
        assert_eq!(surface.pixel(180, 269).must(), Rgba::BLACK);
        // Inactive at another clock position: nothing drawn.
        let mut surface = Rgba8Surface::new((384, 288), Rgba::BLACK).must();
        let now = Timestamp::from_parts(2_000_000, 1, 1_000_000).must();
        assert_eq!(
            compositor
                .compose(std::slice::from_ref(&rendered), now, &mut surface)
                .must(),
            0
        );
        assert_eq!(surface.pixel(180, 250).must(), Rgba::BLACK);
    }

    #[test]
    fn degenerate_inputs_are_typed_never_panics() {
        // Empty text (also via bare breaks).
        for text in ["", "\\N", "\\N\\N", "   "] {
            // "   " is spaces-only: cells exist, so it renders blank —
            // only truly-empty geometry is typed-invalid.
            let c = cue(text, AssAlignment::BOTTOM_CENTER);
            let result = render_plain_text_cue(&c, (384, 288));
            if text == "   " {
                let rendered = result.must();
                assert_eq!((rendered.bitmap.width, rendered.bitmap.height), (36, 24));
            } else {
                assert_eq!(result, Err(AssRenderError::EmptyText), "text {text:?}");
            }
        }
        // Zero font size / oversized font.
        let mut c = cue("Hi", AssAlignment::BOTTOM_CENTER);
        c.style.font_size_px = 0;
        assert_eq!(
            render_plain_text_cue(&c, (384, 288)),
            Err(AssRenderError::ZeroFontSize)
        );
        // The byte guard: 300 cells x 4915 px wide, 9830 tall -> ~54 GB.
        c.style.font_size_px = 8192;
        c.text = "x".repeat(300);
        assert_eq!(
            render_plain_text_cue(&c, (384, 288)),
            Err(AssRenderError::TooLarge {
                width: 1_474_500,
                height: 9830
            })
        );
        // (Pin the arithmetic behind the guard: ~54 GiB >> 64 MiB.)
        const _: () = assert!(MAX_BITMAP_BYTES < 1_474_500u64 * 9830 * 4);
        // Zero script resolution.
        let c = cue("Hi", AssAlignment::BOTTOM_CENTER);
        assert_eq!(
            render_plain_text_cue(&c, (0, 288)),
            Err(AssRenderError::ZeroPlayRes { play_res: (0, 288) })
        );
        // Margins beyond the script axis.
        let mut c = cue("Hi", AssAlignment::BOTTOM_LEFT);
        c.style.margins.left = 385;
        assert_eq!(
            render_plain_text_cue(&c, (384, 288)),
            Err(AssRenderError::MarginExceedsPlayRes {
                axis: "left",
                margin: 385,
                extent: 384
            })
        );
        // Negative times: typed via the timestamp conversion (the
        // compositor's CueError::Time wrapper carries it).
        let mut c = cue("Hi", AssAlignment::BOTTOM_CENTER);
        c.start_us = -1;
        assert!(matches!(
            render_plain_text_cue(&c, (384, 288)),
            Err(AssRenderError::Cue(CueError::Time(_)))
        ));
    }

    #[test]
    fn junk_inputs_never_panic() {
        // Fuzz-lite (the codecs-crate discipline): deterministic
        // splitmix64 junk through the whole path, Ok or typed Err,
        // never a panic. (A raw LCG's low bits would cycle — splitmix64
        // mixes them, keeping the %-derived fields independent.)
        let mut state = 0x005A_175A_17B2u64;
        let mut next = move || {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        const ALPHABET: &[char] = &[
            'a', 'Z', ' ', '\\', 'N', 'n', '\n', 'é', '漢', '0', '\u{a0}',
        ];
        let mut oks = 0u32;
        let mut errs = 0u32;
        for _ in 0..2048 {
            let font = (next() % 70) as u32; // includes 0
            let align_raw = (next() % 12) as u8; // includes 0, 10, 11
            let margins = AssMargins {
                left: (next() % 400) as u32,
                right: (next() % 400) as u32,
                vertical: (next() % 300) as u32,
            };
            let play_res = match next() % 6 {
                0 | 1 => DEFAULT_PLAY_RES,
                2 => (640, 480),
                3 => (100, 100),
                4 => (0, 0),
                _ => (640, 0),
            };
            let len = (next() % 40) as usize;
            let text: String = (0..len)
                .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize])
                .collect();
            let start = (next() % 1_000_000) as i64;
            let end = start + (next() % 2000) as i64 - 500; // reversals included
            let Ok(alignment) = AssAlignment::new(align_raw) else {
                errs += 1;
                continue;
            };
            let c = AssRenderCue {
                id: CueId::new(next()),
                start_us: start,
                end_us: end,
                text,
                style: AssRenderStyle {
                    alignment,
                    margins,
                    primary_colour: next() as u32,
                    font_size_px: font,
                },
                z: (next() % 7) as u16,
            };
            match render_plain_text_cue(&c, play_res) {
                Ok(rendered) => {
                    oks += 1;
                    // Every accepted cue places cleanly at sane extents.
                    for extent in [(384, 288), (1920, 1080), (100, 100)] {
                        let rect = place_cue(&rendered.bitmap, &rendered.layout, extent).must();
                        assert!(rect.x + rect.width <= extent.0);
                        assert!(rect.y + rect.height <= extent.1);
                    }
                }
                Err(_) => errs += 1,
            }
        }
        // The sweep must actually exercise both arms.
        assert!(oks > 200, "oks = {oks}");
        assert!(errs > 200, "errs = {errs}");
    }
}
