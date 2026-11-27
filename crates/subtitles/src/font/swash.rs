//! The real glyph pipeline (admitted 2026-11-27): swash 0.2.10 over the
//! bundled DejaVu Sans face — real per-glyph anti-aliased masks, real
//! face metrics and real per-character advances, behind the same
//! [`GlyphPipeline`] seam as the placeholder
//! ([`MonospaceCellPipeline`]).
//!
//! ## What is real here
//!
//! * **Metrics** — the face's scaled ascent/descent/leading (swash
//!   scale metrics at the requested size) replace the placeholder's
//!   `3/5`-em ratios. The nominal cell advance ([`GlyphMetrics::advance_px`])
//!   is the face's real DIGIT ZERO advance at that size (the tabular
//!   cell convention; DejaVu Sans is ~0.64 em, close to the
//!   placeholder's 0.6 em, so downstream layout degrades gracefully).
//! * **Per-character advances** — [`GlyphPipeline::advance`] is
//!   overridden: each character's advance comes from shaping that one
//!   character at the same em size (an `i` is narrower than a `W`; the
//!   placeholder's one-cell-for-everything is gone).
//! * **Masks** — real anti-aliased coverage from the swash render
//!   path (the `Render` builder over the outline source),
//!   baseline-aligned: rows `[0, ascent)` sit
//!   above the baseline, descent ink and the underline strip live
//!   below it.
//!
//! ## What stays placeholder-geometric (honest, documented)
//!
//! * **Decorations** — underline/strikeout strips use the SAME strip
//!   geometry as the placeholder (`max(1, (ascent+3)/8)` rows;
//!   strikeout centred on `ascent/2`, underline in the descent area),
//!   drawn at full coverage over the real glyph ink. Faux bold dilates
//!   the real ink right by `max(1, advance/6)` columns (stem
//!   thickening); faux italic shears it by `advance/4` — the
//!   placeholder's documented ratios, applied to real outlines.
//! * **Overhang** — decoration extras only (the placeholder's
//!   semantics); per-glyph ink beyond the advance is not reportable
//!   through that signature (it takes no character).
//!
//! ## Totality (no panics on ANY input)
//!
//! Construction is the ONLY fallible step ([`SwashPipeline::new`]
//! returns `None` on an unparseable or degenerate face); every trait
//! method is then total. The audit's F-003 degenerate-metrics hole is
//! closed here: a zero/one-pixel ascent clamps instead of
//! underflowing, absurd caller metrics are dimension-capped at
//! `2 x MAX_FONT_SIZE_PX`, and a character with no glyph falls back to
//! `.notdef` or a blank advance-wide mask.
//!
//! The swash API surface used is deliberately narrow (the crate's
//! documented surface: `FontRef::from_index`, the face metrics via
//! `FontRef::metrics` + `Metrics::scale`, the `ScaleContext` scaler +
//! `Render` builder for images, and single-character shaping via the
//! `ShapeContext` shaper (`add_str` + `shape_with`)): everything else is
//! the seam's own geometry over those outputs.

use std::cell::RefCell;

use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::shape::ShapeContext;
use swash::{FontRef, GlyphId};

use super::pipeline::{
    GlyphDecorations, GlyphMask, GlyphMetrics, GlyphPipeline, MetricsError, MAX_FONT_SIZE_PX,
};

/// The reference em size the face ratios are measured at (any size
/// works — the face scales linearly; 64 px keeps the ratio arithmetic
/// exact in f32 for every common upem).
const REFERENCE_EM_PX: f32 = 64.0;

/// The cap on any mask dimension — degenerate caller metrics can ask
/// for an absurd box; the placeholder's exposure at
/// `MAX_FONT_SIZE_PX` is the same order.
const MAX_MASK_PX: u32 = 2 * MAX_FONT_SIZE_PX;

/// The real-font pipeline: one face's parsed form plus the cached swash
/// contexts (contexts are reused across calls — swash documents them
/// as the caching layer; they need `&mut` to build scalers/shapers,
/// and the trait is `&self`, hence the cells. `Box<dyn GlyphPipeline>`
/// carries no `Send`/`Sync` bound, so single-threaded cells suffice).
pub struct SwashPipeline {
    /// The parsed face; borrows the caller's `'static` font bytes (the
    /// bundled TTF via `include_bytes!` — attached-font wiring is the
    /// demux card's seam).
    font: FontRef<'static>,
    /// Face ascent in px per em px (from the scale metrics at
    /// [`REFERENCE_EM_PX`]) — cached: the face is immutable.
    ascent_ratio: f32,
    /// Full line height (ascent + |descent| + |leading|), px per em px.
    line_ratio: f32,
    scale_ctx: RefCell<ScaleContext>,
    shape_ctx: RefCell<ShapeContext>,
}

impl SwashPipeline {
    /// Build over raw face bytes (a TrueType/OTTO/collection container
    /// this seam accepts — the sniff happened at [`super::FontFace`]
    /// intake).
    ///
    /// `None` when the bytes are not a parseable face, or the face's
    /// scale metrics are degenerate (non-finite or implausible
    /// ascent/line ratios). Construction is the ONLY fallible step;
    /// every [`GlyphPipeline`] method is then total.
    #[must_use]
    pub fn new(data: &'static [u8], index: u32) -> Option<Self> {
        // swash 0.2.10 takes the face index as `usize` (font.rs) — the
        // u32 seam is lossless on every target this crate builds for.
        let font = FontRef::from_index(data, index as usize)?;
        // The face's own metrics (font units) scaled to the reference
        // em — the same linear scaling the shaper applies when it
        // materializes these metrics at a size.
        let m = font.metrics(&[]).scale(REFERENCE_EM_PX);
        let ascent_ratio = m.ascent / REFERENCE_EM_PX;
        let line_ratio = (m.ascent + m.descent.abs() + m.leading.abs()) / REFERENCE_EM_PX;
        if !ascent_ratio.is_finite()
            || !line_ratio.is_finite()
            || !(0.25..4.0).contains(&ascent_ratio)
            || line_ratio < ascent_ratio
            || line_ratio > 64.0
        {
            return None;
        }
        Some(Self {
            font,
            ascent_ratio,
            line_ratio,
            scale_ctx: RefCell::new(ScaleContext::new()),
            shape_ctx: RefCell::new(ShapeContext::new()),
        })
    }

    /// The em size (px) whose real face ascent matches the passed
    /// metrics' ascent — the scale every advance and mask for those
    /// metrics is derived at, so glyph ink lines up with the baseline
    /// row the caller laid out from the same metrics.
    fn em_for(&self, metrics: &GlyphMetrics) -> f32 {
        metrics.ascent_px.max(1) as f32 / self.ascent_ratio
    }

    /// Shape one character at `em` px: its glyph id and advance (px).
    ///
    /// `None` only when the shaper yields no glyph at all (an empty
    /// cluster — control characters, dropped ignorables; callers fall
    /// back honestly).
    fn shape_char(&self, ch: char, em: f32) -> Option<(GlyphId, f32)> {
        let mut ctx = self.shape_ctx.borrow_mut();
        let mut shaper = ctx.builder(self.font).size(em).build();
        // One character in, one cluster out: `add_str` runs the real
        // parser + charmap over the single-character string (the
        // crate's documented path for simple runs); `shape_with` then
        // hands back the shaped glyph cluster.
        let mut scratch = [0u8; 4];
        shaper.add_str(ch.encode_utf8(&mut scratch));
        let mut shaped = None;
        shaper.shape_with(|cluster| {
            if shaped.is_none() {
                if let Some(glyph) = cluster.glyphs.first() {
                    shaped = Some((glyph.id, glyph.advance));
                }
            }
        });
        shaped
    }

    /// Render one glyph to a coverage image at `em` px — the real
    /// `Render` builder over the scaler, sources in priority order
    /// (the scalable outline first, an embedded bitmap strike as the
    /// fallback — swash 0.2.10 has no synthetic box source), reduced to
    /// the placement and alpha samples the mask compose step needs.
    ///
    /// `None` when no source serves the glyph (no outline in the
    /// face); callers treat that as "no ink".
    fn render_glyph(
        &self,
        gid: GlyphId,
        em: f32,
    ) -> Option<(i32, i32, u32, u32, Vec<u8>)> {
        let mut ctx = self.scale_ctx.borrow_mut();
        let mut scaler = ctx.builder(self.font).size(em).build();
        let image = Render::new(&[Source::Outline, Source::Bitmap(StrikeWith::BestFit)])
            .render(&mut scaler, gid)?;
        let placement = image.placement;
        Some((
            placement.left,
            placement.top,
            placement.width,
            placement.height,
            image.data.to_vec(),
        ))
    }

    /// The strike/underline strip height — the placeholder's pinned
    /// ratio (`MonospaceCellPipeline::strip_height`, duplicated here
    /// because the pipeline half of the seam is frozen): `max(1,
    /// (ascent+3)/8)`.
    fn strip_height(ascent: u32) -> u32 {
        (ascent.saturating_add(3) / 8).max(1)
    }
}

impl GlyphPipeline for SwashPipeline {
    fn name(&self) -> &'static str {
        "swash-dejavu-sans"
    }

    fn metrics(&self, size_px: u32) -> Result<GlyphMetrics, MetricsError> {
        if size_px == 0 {
            return Err(MetricsError::ZeroFontSize);
        }
        if size_px > MAX_FONT_SIZE_PX {
            return Err(MetricsError::FontSizeTooLarge {
                size_px,
                max: MAX_FONT_SIZE_PX,
            });
        }
        let em = size_px as f32;
        let ascent_px = (self.ascent_ratio * em).round().max(1.) as u32;
        let line_height_px = (self.line_ratio * em).round().max(ascent_px as f32) as u32;
        // The nominal cell: the face's real digit-zero advance (the
        // tabular-cell convention, module docs). Shaping at the exact
        // em of these metrics keeps advance() and metrics() coherent.
        let advance_px = match self.shape_char('0', em) {
            Some((_, advance)) => advance.round().max(1.) as u32,
            None => em.round().max(1.) as u32,
        };
        Ok(GlyphMetrics {
            advance_px,
            line_height_px,
            ascent_px,
        })
    }

    fn advance(&self, ch: char, metrics: &GlyphMetrics) -> u32 {
        let em = self.em_for(metrics);
        match self.shape_char(ch, em) {
            // No clamp to 1: combining marks legitimately advance by
            // zero; only negative/NaN shaping output is floored.
            Some((_, advance)) => advance.round().max(0.) as u32,
            None => metrics.advance_px,
        }
    }

    fn rasterize(&self, ch: char, metrics: &GlyphMetrics, deco: &GlyphDecorations) -> GlyphMask {
        // Degenerate caller metrics clamp, never underflow (the audit's
        // F-003 hole, closed for this implementation).
        let ascent = metrics.ascent_px.max(1);
        let baseline = i64::from(ascent);
        let em = self.em_for(metrics);
        // Real glyph: shape the character (its id and its advance);
        // .notdef with the nominal cell when even shaping yields
        // nothing (a broken face — construction-checked).
        let (gid, glyph_advance) = self
            .shape_char(ch, em)
            // `GlyphId` is a plain `u16` in swash 0.2.10 (a type alias,
            // no constructor) — `0` is .notdef.
            .unwrap_or((0, metrics.advance_px as f32));
        let advance = glyph_advance.ceil().max(1.) as u32;
        // Real coverage: the outline source's alpha samples, if any.
        let (ink_left, ink_top, ink_w, ink_h, ink_data) = match self.render_glyph(gid, em) {
            Some(parts) => parts,
            None => (0, 0, 0, 0, Vec::new()),
        };
        // Faux-bold widening / faux-italic shear extents — the
        // placeholder's pinned ratios, module docs.
        let bold_extra = if deco.bold { (advance / 6).max(1) } else { 0 };
        let shear = if deco.italic { advance / 4 } else { 0 };
        let strip_h = Self::strip_height(ascent);
        let underline_rows = if deco.underline { strip_h } else { 0 };
        // The mask box: at least the advance wide (ink may extend
        // further right), the ascent rows above the baseline, and the
        // larger of descent ink / underline strip below it.
        let ink_top_row = baseline - i64::from(ink_top);
        let ink_right = i64::from(ink_left) + i64::from(ink_w);
        let ink_bottom_row = ink_top_row + i64::from(ink_h);
        let below = (ink_bottom_row - baseline).max(i64::from(underline_rows));
        let width = i64::from(advance)
            .max(ink_right)
            .saturating_add(i64::from(bold_extra) + i64::from(shear))
            .max(1)
            .min(i64::from(MAX_MASK_PX));
        let height = (baseline + below).max(1).min(i64::from(MAX_MASK_PX));
        let mut alpha = vec![0u8; (width * height) as usize];
        // Blit the coverage samples at (ink_left, ink_top_row); rows
        // above the mask top are clipped (a caller choosing this
        // pipeline's own metrics never clips: the face ascent exceeds
        // every cap-height accent), short sample data is skipped
        // whole (totality over a misbehaving strike).
        let samples = ink_w as usize * ink_h as usize;
        if samples > 0 && ink_data.len() >= samples {
            for gy in 0..ink_h as usize {
                let row = ink_top_row + gy as i64;
                if row < 0 || row >= height {
                    continue;
                }
                for gx in 0..ink_w as usize {
                    let col = i64::from(ink_left) + gx as i64;
                    if col < 0 || col >= width {
                        continue;
                    }
                    let coverage = ink_data[gy * ink_w as usize + gx];
                    let dst = (row * width + col) as usize;
                    if coverage > alpha[dst] {
                        alpha[dst] = coverage;
                    }
                }
            }
        }
        // Faux bold: dilate the real ink `bold_extra` columns right.
        for _ in 0..bold_extra {
            for y in 0..height as usize {
                let row_off = y * width as usize;
                for x in (1..width as usize).rev() {
                    let carried = alpha[row_off + x - 1];
                    if carried > alpha[row_off + x] {
                        alpha[row_off + x] = carried;
                    }
                }
            }
        }
        // Faux italic: row y (0 = top) shifts right by
        // shear*(ascent-1-y)/ascent — the placeholder's formula,
        // applied to the real ink.
        if shear > 0 {
            let mut sheared = vec![0u8; alpha.len()];
            for y in 0..height as usize {
                let row_off = y * width as usize;
                let dy = y as i64;
                let shift = if dy < baseline {
                    (i64::from(shear) * (baseline - 1 - dy)) / baseline
                } else {
                    0
                };
                for x in 0..width as usize {
                    let coverage = alpha[row_off + x];
                    if coverage == 0 {
                        continue;
                    }
                    let dst_x = x + shift as usize;
                    if dst_x < width as usize {
                        let dst = row_off + dst_x;
                        if coverage > sheared[dst] {
                            sheared[dst] = coverage;
                        }
                    }
                }
            }
            alpha = sheared;
        }
        // Strikeout: a strip centred on ascent/2, full mask width.
        if deco.strikeout {
            let top = (ascent / 2).saturating_sub(strip_h / 2);
            for y in top..top.saturating_add(strip_h) {
                if y >= height as u32 {
                    break;
                }
                let row_off = y as usize * width as usize;
                for x in 0..width as usize {
                    alpha[row_off + x] = 255;
                }
            }
        }
        // Underline: the first strip_h rows of the descent area, full
        // mask width (below any descent ink — the mask already grew).
        for y in ascent..ascent.saturating_add(underline_rows) {
            if y >= height as u32 {
                break;
            }
            let row_off = y as usize * width as usize;
            for x in 0..width as usize {
                alpha[row_off + x] = 255;
            }
        }
        GlyphMask {
            width: width as u32,
            height: height as u32,
            alpha,
        }
    }

    fn overhang(&self, metrics: &GlyphMetrics, deco: &GlyphDecorations) -> u32 {
        // Decoration extras only (the placeholder's semantics): real
        // per-glyph ink beyond the advance is not reportable through
        // this signature (it takes no character), module docs.
        let mut over = 0;
        if deco.bold {
            over += (metrics.advance_px / 6).max(1);
        }
        if deco.italic {
            over += metrics.advance_px / 4;
        }
        over
    }
}
