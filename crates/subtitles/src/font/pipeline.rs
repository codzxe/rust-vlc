//! The glyph pipeline seam (issue #17, G-5): span metrics + per-glyph
//! rasterization behind one trait, so the placeholder implementation
//! and the audited real-font stack ([PROPOSAL] — see
//! `docs/dependencies/records/font-stack-0.2.10.md`) are
//! interchangeable at the render paths.
//!
//! * [`GlyphMetrics`] — the per-span geometry (moved here from
//!   `ass::render`, which re-exports it: same public path, new home
//!   beside the seam that produces it).
//! * [`GlyphMask`] — one rasterized glyph as a straight-alpha coverage
//!   mask the caller TINTS with the span colour (the placeholder uses
//!   0/255 coverage; a real pipeline emits anti-aliased coverage —
//!   the tint math is identical).
//! * [`GlyphPipeline`] — `metrics` + `rasterize` + `overhang`.
//! * [`MonospaceCellPipeline`] — the no-new-deps implementation:
//!   today's documented placeholder block glyphs, extended with
//!   DECORATION-AWARE masks (bold/italic/underline/strikeout get
//!   honest placeholder geometry — the G-2 override-tag subset needs
//!   visible, testable per-tag differences, not silent uniformity).
//!
//! ## Placeholder geometry (exact integers, no floats)
//!
//! * advance `3/5` em, line box `6/5` em, ascent `80 %` of the box —
//!   the ratios `ass::render` pinned (monospace-widest-char policy).
//! * **bold**: the block widens by `max(1, advance/6)` on the right
//!   (faux-bold's stem thickening, one integer ratio).
//! * **italic**: the block shears into an oblique parallelogram —
//!   row `y` (0 = top) shifts right by `shear·(ascent−1−y)/ascent`
//!   with `shear = advance/4` (a ~14° slant).
//! * **strikeout**: a strip of `max(1, (ascent+3)/8)` rows centred on
//!   `ascent/2`, across the full mask width.
//! * **underline**: the same strip height, in the descent area
//!   directly under the ascent (mask height grows accordingly).
//! * U+0020 contributes no block ink, but its DECORATION strips
//!   still render (an underlined space is visible in real typography
//!   too — the placeholder honors that).
//!
//! No panics: typed [`MetricsError`] for zero/oversized sizes; the
//! rasterizer is total over `char`.

use std::fmt;

/// The largest accepted font size, script px (moved from `ass::render`
/// — the seam owns the guard now; `ass::render` re-exports it).
pub const MAX_FONT_SIZE_PX: u32 = 8192;

/// Typed metric failures (MP §8) — `ass::render` maps these onto its
/// own `AssRenderError` variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricsError {
    /// A font size of zero — no glyph box exists.
    ZeroFontSize,
    /// A font size beyond [`MAX_FONT_SIZE_PX`].
    FontSizeTooLarge {
        /// The offending size, script px.
        size_px: u32,
        /// The accepted maximum.
        max: u32,
    },
}

impl fmt::Display for MetricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetricsError::ZeroFontSize => f.write_str("font size is zero"),
            MetricsError::FontSizeTooLarge { size_px, max } => {
                write!(f, "font size {size_px} px beyond the {max} px guard")
            }
        }
    }
}

impl std::error::Error for MetricsError {}

/// The monospace-widest-char glyph metrics (the placeholder policy —
/// see `ass::render`'s module docs for the honest estimation ledger).
/// All exact integer ratios, rounded half-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphMetrics {
    /// One cell advance: `3/5` em (0.6 em, the widest-glyph advance of
    /// the common monospace faces).
    pub advance_px: u32,
    /// The line box: `6/5` em (1.2 em).
    pub line_height_px: u32,
    /// The ascent portion of the line box: 80 % — the glyph block
    /// height, and the baseline offset within a line.
    pub ascent_px: u32,
}

impl GlyphMetrics {
    /// Derive the metrics from a font size, script px.
    ///
    /// # Errors
    /// [`MetricsError::ZeroFontSize`] /
    /// [`MetricsError::FontSizeTooLarge`].
    pub fn from_font_size(size_px: u32) -> Result<Self, MetricsError> {
        if size_px == 0 {
            return Err(MetricsError::ZeroFontSize);
        }
        if size_px > MAX_FONT_SIZE_PX {
            return Err(MetricsError::FontSizeTooLarge {
                size_px,
                max: MAX_FONT_SIZE_PX,
            });
        }
        let advance = (u64::from(size_px) * 3 + 2) / 5; // 0.6 em, ties up
        let line_height = (u64::from(size_px) * 12 + 5) / 10; // 1.2 em
        let ascent = (line_height * 8 + 5) / 10; // 80 % of the line box
        // Bounded by MAX_FONT_SIZE_PX: 1.2 x 8192 < 2^14 — both fit.
        Ok(Self {
            advance_px: advance as u32,
            line_height_px: line_height as u32,
            ascent_px: ascent as u32,
        })
    }
}

/// The raster-relevant style flags of one span (the G-2 subset: the
/// `\b`/`\i`/`\u`/`\s` override tags and the style's base flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct GlyphDecorations {
    /// Bold (`\b1`) — thickens the placeholder block.
    pub bold: bool,
    /// Italic (`\i1`) — shears the placeholder block.
    pub italic: bool,
    /// Underline (`\u1`) — a strip under the ascent.
    pub underline: bool,
    /// Strikeout (`\s1`) — a strip through the ascent.
    pub strikeout: bool,
}

impl GlyphDecorations {
    /// All decorations off.
    pub const NONE: Self = Self {
        bold: false,
        italic: false,
        underline: false,
        strikeout: false,
    };
}

/// One rasterized glyph: a straight-alpha coverage mask, row-major,
/// `width × height` samples. The caller tints with the span colour:
/// `out_a = color_a · alpha / 255`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphMask {
    /// Mask width, px.
    pub width: u32,
    /// Mask height, px.
    pub height: u32,
    /// Coverage, `width * height` samples, row-major.
    pub alpha: Vec<u8>,
}

impl GlyphMask {
    /// Construct from parts.
    ///
    /// # Errors
    /// [`GlyphMaskError::SizeMismatch`] when `alpha.len()` is not
    /// `width * height` (checked in `u64` — no overflow).
    pub fn from_alpha(width: u32, height: u32, alpha: Vec<u8>) -> Result<Self, GlyphMaskError> {
        let expected = u64::from(width) * u64::from(height);
        if u64::try_from(alpha.len()).unwrap_or(u64::MAX) != expected {
            return Err(GlyphMaskError::SizeMismatch {
                width,
                height,
                got: alpha.len(),
            });
        }
        Ok(Self {
            width,
            height,
            alpha,
        })
    }

    /// Coverage at `(x, y)`; out-of-bounds reads are 0 (a blitter
    /// convenience — clipping instead of indexing).
    #[must_use]
    pub fn alpha_at(&self, x: u32, y: u32) -> u8 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        let off = (u64::from(y) * u64::from(self.width) + u64::from(x)) as usize;
        self.alpha[off]
    }

    /// Whether every sample is zero (a space cell, or a blank glyph).
    #[must_use]
    pub fn is_blank(&self) -> bool {
        self.alpha.iter().all(|&a| a == 0)
    }
}

/// Typed [`GlyphMask`] failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphMaskError {
    /// `alpha.len()` disagrees with `width * height`.
    SizeMismatch {
        /// The declared width.
        width: u32,
        /// The declared height.
        height: u32,
        /// The samples actually supplied.
        got: usize,
    },
}

impl fmt::Display for GlyphMaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GlyphMaskError::SizeMismatch { width, height, got } => write!(
                f,
                "mask alpha has {got} samples, expected {width} x {height}"
            ),
        }
    }
}

impl std::error::Error for GlyphMaskError {}

/// The shaping/rasterization seam (G-5): everything the render paths
/// need from a font stack, behind one trait.
///
/// Implementations:
/// * [`MonospaceCellPipeline`] — the in-crate placeholder (no new
///   dependencies; today's geometry, decoration-aware).
/// * [PROPOSAL] the real stack (`swash` + `skrifa` per the audit
///   record) — same trait, real outlines, real shaping.
pub trait GlyphPipeline {
    /// Stable pipeline name for ledgers.
    fn name(&self) -> &'static str;

    /// Span metrics at `size_px`, script px.
    ///
    /// # Errors
    /// [`MetricsError`] on zero/oversized sizes.
    fn metrics(&self, size_px: u32) -> Result<GlyphMetrics, MetricsError>;

    /// Rasterize one character under `deco` — total, never panics.
    fn rasterize(&self, ch: char, metrics: &GlyphMetrics, deco: &GlyphDecorations) -> GlyphMask;

    /// How far the mask may extend BEYOND the advance on the right
    /// (faux-bold widening + oblique shear here; real overhangs for a
    /// real stack) — the caller adds it to the line width so the last
    /// glyph's decorations are not clipped.
    fn overhang(&self, metrics: &GlyphMetrics, deco: &GlyphDecorations) -> u32;
}

/// The no-new-deps placeholder pipeline: solid cell blocks with the
/// documented decoration geometry (module docs). `Copy` — stateless.
#[derive(Debug, Clone, Copy, Default)]
pub struct MonospaceCellPipeline;

impl MonospaceCellPipeline {
    /// The strike/underline strip height: `max(1, (ascent+3)/8)` —
    /// ~12.5 % of the ascent, at least one row.
    fn strip_height(ascent: u32) -> u32 {
        ((ascent + 3) / 8).max(1)
    }

    /// The faux-bold right extension: `max(1, advance/6)`.
    fn bold_extra(advance: u32) -> u32 {
        (advance / 6).max(1)
    }

    /// The oblique shear: `advance/4` (a ~14° slant at 0.6 em).
    fn shear(advance: u32) -> u32 {
        advance / 4
    }
}

impl GlyphPipeline for MonospaceCellPipeline {
    fn name(&self) -> &'static str {
        "monospace-cell-placeholder"
    }

    fn metrics(&self, size_px: u32) -> Result<GlyphMetrics, MetricsError> {
        GlyphMetrics::from_font_size(size_px)
    }

    fn rasterize(&self, ch: char, metrics: &GlyphMetrics, deco: &GlyphDecorations) -> GlyphMask {
        let advance = metrics.advance_px;
        let ascent = metrics.ascent_px;
        let is_space = ch == ' ';
        let bold_extra = if deco.bold { Self::bold_extra(advance) } else { 0 };
        let shear = if deco.italic { Self::shear(advance) } else { 0 };
        let strip_h = Self::strip_height(ascent);
        let underline_rows = if deco.underline { strip_h } else { 0 };
        let width = u64::from(advance) + u64::from(bold_extra) + u64::from(shear);
        let height = u64::from(ascent) + u64::from(underline_rows);
        // Bounded by MAX_FONT_SIZE_PX-derived metrics (< 2^14 each):
        // no overflow.
        let mut alpha = vec![0u8; (width * height) as usize];
        let put = |alpha: &mut [u8], x: u64, y: u64| {
            let off = (y * width + x) as usize;
            alpha[off] = 255;
        };
        // The (sheared, widened) solid block — spaces contribute no
        // ink (their decorations still do, below).
        if !is_space {
            for y in 0..u64::from(ascent) {
                // shift = shear * (ascent-1-y) / ascent, exact
                // integer, 0 (bottom) ..= shear (top).
                let numer = u64::from(shear) * (u64::from(ascent) - 1 - y);
                let shift = numer / u64::from(ascent);
                for x in shift..shift + u64::from(advance) + u64::from(bold_extra) {
                    put(&mut alpha, x, y);
                }
            }
        }
        // Strikeout: strip centred on ascent/2, full mask width.
        if deco.strikeout {
            let top = u64::from(ascent / 2) - u64::from(strip_h / 2);
            for y in top..top + u64::from(strip_h) {
                for x in 0..width {
                    put(&mut alpha, x, y);
                }
            }
        }
        // Underline: the first strip_h rows of the descent area.
        for y in u64::from(ascent)..u64::from(ascent) + u64::from(underline_rows) {
            for x in 0..width {
                put(&mut alpha, x, y);
            }
        }
        GlyphMask {
            width: width as u32,
            height: height as u32,
            alpha,
        }
    }

    fn overhang(&self, metrics: &GlyphMetrics, deco: &GlyphDecorations) -> u32 {
        let mut over = 0;
        if deco.bold {
            over += Self::bold_extra(metrics.advance_px);
        }
        if deco.italic {
            over += Self::shear(metrics.advance_px);
        }
        over
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GlyphDecorations, GlyphMask, GlyphMaskError, GlyphMetrics, GlyphPipeline,
        MAX_FONT_SIZE_PX, MetricsError, MonospaceCellPipeline,
    };
    use testkit::Must;

    const PIPE: MonospaceCellPipeline = MonospaceCellPipeline;

    #[test]
    fn metrics_ratios_are_the_pinned_ones() {
        // The same table ass::render pinned before the move.
        assert_eq!(
            GlyphMetrics::from_font_size(20).must(),
            GlyphMetrics {
                advance_px: 12,
                line_height_px: 24,
                ascent_px: 19
            }
        );
        assert_eq!(
            GlyphMetrics::from_font_size(10).must(),
            GlyphMetrics {
                advance_px: 6,
                line_height_px: 12,
                ascent_px: 10
            }
        );
        assert_eq!(
            GlyphMetrics::from_font_size(25).must(),
            GlyphMetrics {
                advance_px: 15,
                line_height_px: 30,
                ascent_px: 24
            }
        );
        assert_eq!(
            GlyphMetrics::from_font_size(0),
            Err(MetricsError::ZeroFontSize)
        );
        assert_eq!(
            GlyphMetrics::from_font_size(MAX_FONT_SIZE_PX + 1),
            Err(MetricsError::FontSizeTooLarge {
                size_px: MAX_FONT_SIZE_PX + 1,
                max: MAX_FONT_SIZE_PX
            })
        );
    }

    #[test]
    fn plain_block_and_space_geometry() {
        let m = GlyphMetrics::from_font_size(20).must(); // 12 x 24, ascent 19
        let plain = PIPE.rasterize('A', &m, &GlyphDecorations::NONE);
        assert_eq!((plain.width, plain.height), (12, 19));
        assert!(!plain.is_blank());
        assert_eq!(plain.alpha_at(0, 0), 255);
        assert_eq!(plain.alpha_at(11, 18), 255);
        let space = PIPE.rasterize(' ', &m, &GlyphDecorations::NONE);
        assert_eq!((space.width, space.height), (12, 19));
        assert!(space.is_blank());
        // Overhang with no decorations: none.
        assert_eq!(PIPE.overhang(&m, &GlyphDecorations::NONE), 0);
    }

    #[test]
    fn bold_widens_the_block() {
        let m = GlyphMetrics::from_font_size(20).must(); // advance 12
        let deco = GlyphDecorations {
            bold: true,
            ..GlyphDecorations::NONE
        };
        let bold = PIPE.rasterize('A', &m, &deco);
        // max(1, 12/6) = 2 extra columns.
        assert_eq!(bold.width, 14);
        assert_eq!(bold.alpha_at(13, 0), 255);
        assert_eq!(PIPE.overhang(&m, &deco), 2);
        // Tiny advances still get >= 1 column of bold.
        let tiny = GlyphMetrics::from_font_size(5).must(); // advance 3
        let b = PIPE.rasterize('A', &tiny, &deco);
        assert_eq!(b.width, 4);
    }

    #[test]
    fn italic_shears_top_row_only_at_the_top() {
        let m = GlyphMetrics::from_font_size(20).must(); // advance 12, ascent 19
        let deco = GlyphDecorations {
            italic: true,
            ..GlyphDecorations::NONE
        };
        let it = PIPE.rasterize('A', &m, &deco);
        // shear = 12/4 = 3; width = 12 + 3 = 15.
        assert_eq!(it.width, 15);
        assert_eq!(PIPE.overhang(&m, &deco), 3);
        // Top row (y=0): shifted right by floor(3*18/19) = 2 — first
        // 2 columns empty, block starts at 2.
        assert_eq!(it.alpha_at(0, 0), 0);
        assert_eq!(it.alpha_at(1, 0), 0);
        assert_eq!(it.alpha_at(2, 0), 255);
        // Bottom row (y = ascent-1 = 18): shift = 3*0/19 = 0.
        assert_eq!(it.alpha_at(0, 18), 255);
        assert_eq!(it.alpha_at(11, 18), 255);
        assert_eq!(it.alpha_at(12, 18), 0);
        // Height unchanged (shear is horizontal only).
        assert_eq!(it.height, 19);
    }

    #[test]
    fn underline_sits_in_the_descent() {
        let m = GlyphMetrics::from_font_size(20).must(); // ascent 19
        let deco = GlyphDecorations {
            underline: true,
            ..GlyphDecorations::NONE
        };
        let u = PIPE.rasterize('A', &m, &deco);
        // strip = max(1, (19+3)/8) = 2 rows below the ascent.
        assert_eq!(u.height, 21);
        assert_eq!(u.alpha_at(0, 19), 255);
        assert_eq!(u.alpha_at(0, 20), 255);
        // The strip is BELOW the glyph block top area (row 0 is the
        // block itself for a solid placeholder — the distinguishing
        // geometry is the descent rows existing at all).
        assert_eq!(u.width, 12);
    }

    #[test]
    fn strikeout_cuts_through_the_ascent() {
        let m = GlyphMetrics::from_font_size(20).must(); // ascent 19
        let deco = GlyphDecorations {
            strikeout: true,
            ..GlyphDecorations::NONE
        };
        let s = PIPE.rasterize(' ', &m, &deco);
        // On a SPACE: no block ink, but the strike strip still
        // renders (an underlined/struck space is visible in real
        // typography — the placeholder honors that; pinned here as
        // the documented v1 behavior).
        assert!(!s.is_blank());
        // strip = 2, top = 19/2 - 2/2 = 9 - 1 = 8.
        assert_eq!(s.alpha_at(0, 8), 255);
        assert_eq!(s.alpha_at(0, 9), 255);
        assert_eq!(s.alpha_at(0, 7), 0);
        assert_eq!(s.alpha_at(0, 10), 0);
        // Height stays at the ascent (strike lives inside it).
        assert_eq!(s.height, 19);
    }

    #[test]
    fn combined_decorations_stack_geometrically() {
        let m = GlyphMetrics::from_font_size(20).must();
        let deco = GlyphDecorations {
            bold: true,
            italic: true,
            underline: true,
            strikeout: true,
        };
        let g = PIPE.rasterize('A', &m, &deco);
        // 12 + 2 (bold) + 3 (shear) wide, 19 + 2 (underline) tall.
        assert_eq!((g.width, g.height), (17, 21));
        assert_eq!(PIPE.overhang(&m, &deco), 5);
        // Every decorated region is set: the top row's block spans
        // [shift, shift+advance+bold) = [2, 16).
        assert_eq!(g.alpha_at(15, 0), 255);
        assert_eq!(g.alpha_at(16, 0), 0);
        assert_eq!(g.alpha_at(0, 20), 255); // underline tail
    }

    #[test]
    fn mask_validates_its_own_shape() {
        assert_eq!(
            GlyphMask::from_alpha(2, 2, vec![0; 5]),
            Err(GlyphMaskError::SizeMismatch {
                width: 2,
                height: 2,
                got: 5
            })
        );
        let ok = GlyphMask::from_alpha(2, 2, vec![1, 2, 3, 4]).must();
        assert_eq!(ok.alpha_at(1, 0), 2);
        assert_eq!(ok.alpha_at(0, 1), 3);
        // Out-of-bounds reads are 0, not a panic.
        assert_eq!(ok.alpha_at(9, 9), 0);
    }

    #[test]
    fn rasterize_is_total_over_junk_chars() {
        let m = GlyphMetrics::from_font_size(24).must();
        for ch in ['\u{0}', '\u{7f}', 'é', '漢', '\u{10FFFF}', '\t', '\u{a0}'] {
            let mask = PIPE.rasterize(ch, &m, &GlyphDecorations::NONE);
            assert_eq!(mask.width, m.advance_px);
            assert_eq!(mask.height, m.ascent_px);
        }
    }
}
