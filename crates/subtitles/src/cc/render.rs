//! The CC render path (card 5.N3 step 4): one 608 grid snapshot →
//! the compositing contract's [`CompositedCue`].
//!
//! # Metrics policy (the `ass::render` monospace precedent)
//!
//! Fonts belong to the renderer (doc 04 §5); this crate has no font
//! stack. The 608 grid is therefore rasterized with the same honest
//! placeholder coverage as the ASS plain-text path: every cell is a
//! SOLID BLOCK in the cell's pen color. The geometry is exact:
//!
//! * the reference extent is **704×480** — the NTSC 525-line active
//!   area — so the 15×32 grid tiles it EXACTLY: cell advance 22 px,
//!   cell line box 32 px (`32 × 22 = 704`, `15 × 32 = 480`);
//! * the glyph block fills the ascent portion of the line box
//!   (25 of 32 rows, ≈80 % — the `ass::render` split); underline
//!   adds a 3-row bar at the cell bottom;
//! * the bitmap covers the grid's occupied bounding box; its layout
//!   anchors at `(min_col / 32, top-of-first-occupied-row / 15)` of
//!   the frame — exact rational anchors, no floats — and the
//!   compositor scales it with the presentation extent via the
//!   `reference` field, then clamps into the 5 % title-safe box
//!   (captions are authored inside it).
//!
//! Per-cell colors are honored (white/green/blue/cyan/red/yellow/
//! magenta/black — opaque black, see [`super::eia608::CcColor`]);
//! italics cannot slant a solid block (recorded, not faked); a
//! blank cell is transparent.
//!
//! # Never
//!
//! Panics or unbounded allocation: the bitmap is bounded by the
//! 15×32 grid (704×480 RGBA8 = 1.35 MiB, allocation-checked through
//! [`CueBitmap::from_rgba`]).

use crate::cc::eia608::{COLS, CcColor, CcGrid, ROWS};
use crate::compositing::bitmap::{CompositedCue, CueBitmap, CueError};
use crate::compositing::layout::{
    Anchor, CueAlignment, CueLayout, HorizontalAlign, Rational, SafeMargin, VerticalAlign,
};
use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};

/// The CC authoring reference extent: NTSC 704×480, tiled exactly by
/// the 15×32 grid.
pub const CC_REF_EXTENT: (u32, u32) = (704, 480);

/// One cell advance: 704 / 32.
pub const CELL_ADVANCE_PX: u32 = 22;

/// One cell line box: 480 / 15.
pub const CELL_LINE_PX: u32 = 32;

/// The glyph-block (ascent) portion of the cell line box, ≈80 %.
pub const CELL_ASCENT_PX: u32 = 25;

/// The underline bar height.
pub const UNDERLINE_PX: u32 = 3;

/// A rendered grid: the bitmap plus its grid coordinates (for the
/// exact layout anchor).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedCcGrid {
    /// The rasterized bounding box.
    pub bitmap: CueBitmap,
    /// 0-based top occupied row.
    pub min_row: usize,
    /// 0-based leftmost occupied column.
    pub min_col: usize,
    /// 0-based bottom occupied row.
    pub max_row: usize,
    /// 0-based rightmost occupied column.
    pub max_col: usize,
}

impl RenderedCcGrid {
    /// The exact layout: top-left anchored at
    /// `(min_col / 32, (15 - min_row) / 15)` — v measured from the
    /// frame bottom — with the CC reference extent for scaling.
    #[must_use]
    pub fn layout(&self) -> CueLayout {
        // Infallible by construction: the denominators are the grid
        // constants and the numerators are in-grid coordinates.
        let u = Rational::new(self.min_col as u32, COLS as u32).unwrap_or(Rational::ZERO);
        let v = Rational::new((ROWS - self.min_row) as u32, ROWS as u32).unwrap_or(Rational::ZERO);
        CueLayout {
            anchor: Anchor::new(u, v).unwrap_or(Anchor::BOTTOM_CENTER),
            alignment: CueAlignment {
                horizontal: HorizontalAlign::Start,
                vertical: VerticalAlign::Top,
            },
            margin: SafeMargin::TitleSafe,
            reference: Some(CC_REF_EXTENT),
        }
    }
}

/// Rasterize the occupied bounding box of `grid`. `None` when the
/// grid is empty (a cleared screen renders no cue).
///
/// # Errors
/// [`CueError`] from the bitmap validation (defensive — the grid
/// bounds the size by construction).
pub fn render_grid(grid: &CcGrid) -> Result<Option<RenderedCcGrid>, CueError> {
    let mut min_row = ROWS;
    let mut max_row = 0usize;
    let mut min_col = COLS;
    let mut max_col = 0usize;
    for row in 0..ROWS {
        for col in 0..COLS {
            if grid.cell(row, col).ch != ' ' {
                min_row = min_row.min(row);
                max_row = max_row.max(row);
                min_col = min_col.min(col);
                max_col = max_col.max(col);
            }
        }
    }
    if min_row == ROWS {
        return Ok(None); // empty screen
    }
    let rows = (max_row - min_row + 1) as u32;
    let cols = (max_col - min_col + 1) as u32;
    let width = cols * CELL_ADVANCE_PX;
    let height = rows * CELL_LINE_PX;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for row in min_row..=max_row {
        for col in min_col..=max_col {
            let cell = grid.cell(row, col);
            if cell.ch == ' ' {
                continue;
            }
            let (r, g, b) = cell.color.rgb();
            let cell_x = ((col - min_col) as u32) * CELL_ADVANCE_PX;
            let cell_top = ((row - min_row) as u32) * CELL_LINE_PX;
            // The glyph block: rows [0, ascent) of the cell.
            for y in cell_top..cell_top + CELL_ASCENT_PX {
                for x in cell_x..cell_x + CELL_ADVANCE_PX {
                    let off = ((y * width + x) * 4) as usize;
                    rgba[off] = r;
                    rgba[off + 1] = g;
                    rgba[off + 2] = b;
                    rgba[off + 3] = 255;
                }
            }
            if cell.underline {
                for y in cell_top + CELL_ASCENT_PX..cell_top + CELL_ASCENT_PX + UNDERLINE_PX {
                    for x in cell_x..cell_x + CELL_ADVANCE_PX {
                        let off = ((y * width + x) * 4) as usize;
                        rgba[off] = r;
                        rgba[off + 1] = g;
                        rgba[off + 2] = b;
                        rgba[off + 3] = 255;
                    }
                }
            }
        }
    }
    // The baseline of the LAST occupied row (the ass::render rule).
    let baseline = (rows - 1) * CELL_LINE_PX + CELL_ASCENT_PX;
    Ok(Some(RenderedCcGrid {
        bitmap: CueBitmap::from_rgba(width, height, baseline, rgba)?,
        min_row,
        min_col,
        max_row,
        max_col,
    }))
}

/// Build the composited cue for one rendered grid over `[start, end)`
/// nanoseconds (either side `None` = open).
///
/// # Errors
/// [`CueError`] from the span/bitmap validation.
pub fn grid_cue(
    rendered: &RenderedCcGrid,
    start_ns: Option<i64>,
    end_ns: Option<i64>,
    z: u16,
) -> Result<CompositedCue, CueError> {
    // The 1/1e9 nanosecond base: `num`/`den` are coprime and
    // non-zero, so this literal is exactly `TimeBase::new(1,
    // 1_000_000_000)`'s Ok value — constructed directly (the DVB
    // builder precedent).
    let ns_base = TimeBase {
        num: 1,
        den: 1_000_000_000,
    };
    let ns = |v: Option<i64>| -> Stamp {
        match v {
            Some(ns) => Stamp::Present(Timestamp::new(ns, ns_base)),
            None => Stamp::Missing,
        }
    };
    CompositedCue::new(
        TimeSpan {
            start: ns(start_ns),
            end: ns(end_ns),
        },
        rendered.bitmap.clone(),
        rendered.layout(),
        z,
    )
}

/// The color of one rendered pixel (test vocabulary).
#[must_use]
pub fn cell_color_of(color: CcColor) -> (u8, u8, u8) {
    color.rgb()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cc::eia608::CcCell;
    use testkit::Must;

    fn grid_with(cells: &[(usize, usize, CcCell)]) -> CcGrid {
        let mut g = CcGrid::new();
        for &(row, col, cell) in cells {
            g.cells[row * COLS + col] = cell;
        }
        g
    }

    #[test]
    fn empty_grid_renders_none() {
        assert!(render_grid(&CcGrid::new()).must().is_none());
    }

    #[test]
    fn bounding_box_and_metrics_are_exact() {
        let mut g = CcGrid::new();
        g.cells[3 * COLS + 5] = CcCell {
            ch: 'H',
            ..CcCell::blank()
        };
        g.cells[4 * COLS + 7] = CcCell {
            ch: 'i',
            ..CcCell::blank()
        };
        let r = render_grid(&g).must().must();
        assert_eq!((r.min_row, r.max_row, r.min_col, r.max_col), (3, 4, 5, 7));
        assert_eq!(r.bitmap.width, 3 * CELL_ADVANCE_PX);
        assert_eq!(r.bitmap.height, 2 * CELL_LINE_PX);
        assert_eq!(r.bitmap.baseline, CELL_LINE_PX + CELL_ASCENT_PX);
        // The 'H' block: opaque white across the ascent rows.
        let px = r.bitmap.pixel(0, 0);
        assert_eq!((px.r, px.g, px.b, px.a), (255, 255, 255, 255));
        // The row below the ascent (the descent gap) is transparent.
        let px = r.bitmap.pixel(0, CELL_ASCENT_PX);
        assert_eq!(px.a, 0);
        // Column 3 (past the bbox width) is not in the bitmap at all
        // — pixel() is bounded by construction; check the right edge
        // of the 'i' cell (grid row 4, col 7): the last bbox column's
        // ascent is opaque.
        let px = r
            .bitmap
            .pixel(3 * CELL_ADVANCE_PX - 1, CELL_LINE_PX + CELL_ASCENT_PX - 1);
        assert_eq!(px.a, 255);
    }

    #[test]
    fn per_cell_colors_and_underline() {
        let g = grid_with(&[
            (
                0,
                0,
                CcCell {
                    ch: 'R',
                    color: CcColor::Red,
                    ..CcCell::blank()
                },
            ),
            (
                0,
                1,
                CcCell {
                    ch: 'G',
                    color: CcColor::Green,
                    underline: true,
                    ..CcCell::blank()
                },
            ),
        ]);
        let r = render_grid(&g).must().must();
        let red = r.bitmap.pixel(1, 1);
        assert_eq!((red.r, red.g, red.b), (255, 0, 0));
        let green = r.bitmap.pixel(CELL_ADVANCE_PX + 2, 2);
        assert_eq!((green.r, green.g, green.b), (0, 255, 0));
        // Underline bar: rows [ascent, ascent+3).
        let bar = r.bitmap.pixel(CELL_ADVANCE_PX + 2, CELL_ASCENT_PX + 1);
        assert_eq!((bar.r, bar.g, bar.b, bar.a), (0, 255, 0, 255));
        let below = r
            .bitmap
            .pixel(CELL_ADVANCE_PX + 2, CELL_ASCENT_PX + UNDERLINE_PX);
        assert_eq!(below.a, 0);
    }

    #[test]
    fn layout_anchor_is_rational_exact() {
        let g = grid_with(&[(
            14,
            10,
            CcCell {
                ch: 'x',
                ..CcCell::blank()
            },
        )]);
        let r = render_grid(&g).must().must();
        let layout = r.layout();
        assert_eq!(layout.anchor.u, Rational::new(10, 32).must());
        assert_eq!(layout.anchor.v, Rational::new(1, 15).must());
        assert_eq!(layout.reference, Some(CC_REF_EXTENT));
        assert_eq!(layout.margin, SafeMargin::TitleSafe);
    }
}
