//! Cue placement: safe areas, anchors, baseline alignment, scaling
//! with the video extent — exact integer/rational arithmetic only
//! (no floats near a pixel; MP §8 determinism).
//!
//! The model (the common denominator of WebVTT line/position, ASS
//! `\an`/`\pos` and broadcast safe-area practice):
//!
//! 1. An [`Anchor`] is an exact rational point of the PRESENTATION
//!    extent — `(1/2, 9/10)` is "centered, 90 % down" at every video
//!    size, which is the "scales with the video" property.
//! 2. [`CueAlignment`] picks WHICH point of the (scaled) cue sits on
//!    the anchor: `Start/Center/End` horizontally, and
//!    `Top/Center/Baseline/Bottom` vertically. `Baseline` pins the
//!    cue's baseline row (WebVTT line snapping; ASS anchoring).
//! 3. If the cue was authored for a different extent
//!    ([`CueLayout::reference`]), it is scaled by the uniform
//!    aspect-preserving `min(extent/reference)` factor — never
//!    distorted, never exceeding either axis.
//! 4. The result is clamped into the [`SafeMargin`] box (5 % title
//!    safe / 10 % action safe, broadcast convention); a cue larger
//!    than the box centers on the full extent instead.

use crate::compositing::bitmap::{CueBitmap, CueError};
use crate::compositing::target::ComposeError;

/// An exact non-negative rational, reduced at construction.
///
/// The placement vocabulary: anchors, scale factors and safe-area
/// percentages are all rationals, so every pixel coordinate is one
/// rounded integer division away from exact — never a float.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    /// Numerator.
    pub num: u32,
    /// Denominator (never zero).
    pub den: u32,
}

impl Rational {
    /// Zero (`0/1`, already reduced).
    pub const ZERO: Rational = Rational { num: 0, den: 1 };
    /// One (`1/1`, already reduced).
    pub const ONE: Rational = Rational { num: 1, den: 1 };

    /// The reduced rational `num / den`.
    ///
    /// # Errors
    /// [`CueError::ZeroDenominator`] when `den == 0`.
    pub fn new(num: u32, den: u32) -> Result<Rational, CueError> {
        if den == 0 {
            return Err(CueError::ZeroDenominator);
        }
        let g = gcd(num, den);
        Ok(Rational {
            num: num / g,
            den: den / g,
        })
    }

    /// `(num * v + den / 2) / den` — the value scaled by the rational,
    /// rounded to nearest, ties up. `u64` domain: no truncation before
    /// the caller rounds.
    pub(crate) fn scale_u64(&self, v: u64) -> u64 {
        (self.num as u64 * v + self.den as u64 / 2) / self.den as u64
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    if a == 0 { 1 } else { a }
}

/// An anchor point of the presentation extent, exact rationals in
/// `[0, 1]` per axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Anchor {
    /// Horizontal fraction of the extent width.
    pub u: Rational,
    /// Vertical fraction of the extent height.
    pub v: Rational,
}

impl Anchor {
    /// The broadcast-default subtitle anchor: centered, 90 % down.
    pub const BOTTOM_CENTER: Anchor = Anchor {
        u: Rational { num: 1, den: 2 },
        v: Rational { num: 9, den: 10 },
    };

    /// A validated anchor.
    ///
    /// # Errors
    /// [`CueError::AnchorOutOfRange`] when either fraction is outside
    /// `[0, 1]`, or the fraction's denominator is zero.
    pub fn new(u: Rational, v: Rational) -> Result<Anchor, CueError> {
        for (axis, r) in [("u", u), ("v", v)] {
            if r.den == 0 || r.num > r.den {
                return Err(CueError::AnchorOutOfRange {
                    axis,
                    num: r.num,
                    den: r.den,
                });
            }
        }
        Ok(Anchor { u, v })
    }
}

/// Horizontal alignment of the cue relative to its anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HorizontalAlign {
    /// The cue's left edge sits on the anchor.
    Start,
    /// The cue is horizontally centered on the anchor.
    #[default]
    Center,
    /// The cue's right edge sits on the anchor.
    End,
}

/// Vertical alignment of the cue relative to its anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VerticalAlign {
    /// The cue's top edge sits on the anchor.
    Top,
    /// The cue is vertically centered on the anchor.
    Center,
    /// The cue's baseline row sits on the anchor (WebVTT line
    /// snapping; the baseline is `CueBitmap::baseline` rows from the
    /// cue top, scaled with the cue).
    Baseline,
    /// The cue's bottom edge sits on the anchor (the subtitle
    /// default: bottom margins measured from the frame bottom).
    #[default]
    Bottom,
}

/// Which point of the cue sits on the anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CueAlignment {
    /// Horizontal rule.
    pub horizontal: HorizontalAlign,
    /// Vertical rule.
    pub vertical: VerticalAlign,
}

/// The safe-area convention the placement clamps into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SafeMargin {
    /// 5 % title-safe inset (subtitles default).
    #[default]
    TitleSafe,
    /// 10 % action-safe inset.
    ActionSafe,
}

impl SafeMargin {
    /// The inset percentage.
    fn percent(self) -> u64 {
        match self {
            SafeMargin::TitleSafe => 5,
            SafeMargin::ActionSafe => 10,
        }
    }

    /// The inset in pixels for `extent`, per axis, rounded to nearest
    /// (ties up): `(w * pct + 50) / 100`.
    fn inset_px(self, extent: (u32, u32)) -> (u64, u64) {
        let pct = self.percent();
        (
            (extent.0 as u64 * pct + 50) / 100,
            (extent.1 as u64 * pct + 50) / 100,
        )
    }
}

/// An axis-aligned pixel rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

/// Everything needed to place one cue on a presentation extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CueLayout {
    /// The anchor point (fractions of the presentation extent).
    pub anchor: Anchor,
    /// Which cue point sits on the anchor.
    pub alignment: CueAlignment,
    /// The safe area the placement is clamped into.
    pub margin: SafeMargin,
    /// The video extent the cue was authored for; `None` means
    /// "authored at presentation extent" (no scaling). When set and
    /// different, the cue scales by `min(extent / reference)` per
    /// axis — uniform, aspect-preserving.
    pub reference: Option<(u32, u32)>,
}

impl CueLayout {
    /// The subtitle default: centered 90 % down, bottom-aligned into
    /// the 5 % title-safe box, no scaling.
    pub const BOTTOM_CENTER: CueLayout = CueLayout {
        anchor: Anchor::BOTTOM_CENTER,
        alignment: CueAlignment {
            horizontal: HorizontalAlign::Center,
            vertical: VerticalAlign::Bottom,
        },
        margin: SafeMargin::TitleSafe,
        reference: None,
    };
}

/// The uniform scale from `reference` to `extent` as a reduced
/// fraction (1/1 when equal or unanchored). `min` per axis,
/// cross-multiplied — no float, no division.
fn uniform_scale(reference: (u32, u32), extent: (u32, u32)) -> (u64, u64) {
    let rw = reference.0 as u64;
    let rh = reference.1 as u64;
    let ew = extent.0 as u64;
    let eh = extent.1 as u64;
    // scale_x = ew / rw, scale_y = eh / rh; pick the smaller.
    let (num, den) = if ew * rh <= rw * eh {
        (ew, rw)
    } else {
        (eh, rh)
    };
    let g = gcd_u64(num, den);
    (num / g, den / g)
}

fn gcd_u64(a: u64, b: u64) -> u64 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// A dimension scaled by `sn / sd`, rounded to nearest (ties up).
fn scaled_dim(dim: u32, sn: u64, sd: u64) -> u64 {
    (dim as u64 * sn + sd / 2) / sd
}

/// Compute the cue's destination rectangle on `extent` (see the
/// module docs for the four-step model). Pure function — the SAME
/// contract a GPU path implements; the reference blend uses it, and
/// the engine may call it directly for shader-side compositing.
///
/// # Errors
/// [`ComposeError::ZeroExtent`] when the extent is empty;
/// [`ComposeError::InvalidLayout`] when the layout's anchor is out of
/// range (pub-field escape hatch: constructors validate, this guards).
#[must_use = "the placement is the contract — compute it or pass it on"]
pub fn place_cue(
    bitmap: &CueBitmap,
    layout: &CueLayout,
    extent: (u32, u32),
) -> Result<Rect, ComposeError> {
    if extent.0 == 0 || extent.1 == 0 {
        return Err(ComposeError::ZeroExtent { extent });
    }
    if let Err(err) = Anchor::new(layout.anchor.u, layout.anchor.v) {
        return Err(ComposeError::InvalidLayout {
            detail: "anchor out of [0,1] or zero denominator",
            source: err,
        });
    }

    // 1. Scale with the video extent (uniform, aspect-preserving).
    let (sn, sd) = match layout.reference {
        Some(reference) if reference != extent => uniform_scale(reference, extent),
        _ => (1, 1),
    };
    let sw = scaled_dim(bitmap.width, sn, sd);
    let sh = scaled_dim(bitmap.height, sn, sd);
    let sbaseline = scaled_dim(bitmap.baseline, sn, sd);

    // 2. The anchor point, in presentation pixels (exact rational ×
    //    extent, rounded to nearest).
    let ax = layout.anchor.u.scale_u64(extent.0 as u64);
    let ay = layout.anchor.v.scale_u64(extent.1 as u64);

    // 3. The cue's reference point on the SCALED cue.
    let rx = match layout.alignment.horizontal {
        HorizontalAlign::Start => 0,
        HorizontalAlign::Center => sw / 2,
        HorizontalAlign::End => sw,
    };
    let ry = match layout.alignment.vertical {
        VerticalAlign::Top => 0,
        VerticalAlign::Center => sh / 2,
        VerticalAlign::Baseline => sbaseline,
        VerticalAlign::Bottom => sh,
    };
    let x0 = ax.saturating_sub(rx);
    let y0 = ay.saturating_sub(ry);

    // 4. Clamp into the safe box (center on the full extent when the
    //    cue does not fit).
    let rect = Rect {
        x: clamp_axis(x0, sw, extent.0 as u64, layout.margin, true),
        y: clamp_axis(y0, sh, extent.1 as u64, layout.margin, false),
        width: sw.min(extent.0 as u64) as u32,
        height: sh.min(extent.1 as u64) as u32,
    };
    Ok(rect)
}

/// Clamp one axis: `origin` into `[inset, extent - inset - size]`,
/// centering on the whole extent when the cue overflows it.
fn clamp_axis(origin: u64, size: u64, extent: u64, margin: SafeMargin, horizontal: bool) -> u32 {
    let inset = if horizontal {
        margin.inset_px((extent as u32, 1)).0
    } else {
        margin.inset_px((1, extent as u32)).1
    };
    let box_left = inset;
    let box_right = extent.saturating_sub(inset);
    let mut pos = origin.clamp(box_left, box_right.saturating_sub(size).max(box_left));
    // The overflow case: the cue is wider than the safe box (or the
    // box is degenerate) — center on the full extent instead.
    if pos + size > extent {
        pos = extent.saturating_sub(size) / 2;
    }
    pos as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositing::bitmap::CueBitmap;
    use crate::compositing::target::Rgba;
    use testkit::Must;

    fn bitmap(w: u32, h: u32, baseline: u32) -> CueBitmap {
        let mut bm = CueBitmap::solid(w, h, Rgba::BLACK).must();
        bm.baseline = baseline;
        bm
    }

    #[test]
    fn anchor_math_is_pixel_exact_on_1080p() {
        // 1/10 of 1920 = 192; 9/10 of 1080 = 972 — exact, and the
        // 128x32 center-aligned cue lands at (128, 956) INSIDE the
        // 5% title-safe box (x >= 96, bottom <= 1026): no clamp.
        let cue = bitmap(128, 32, 32);
        let mut layout = CueLayout::BOTTOM_CENTER;
        layout.anchor =
            Anchor::new(Rational::new(1, 10).must(), Rational::new(9, 10).must()).must();
        layout.alignment.horizontal = HorizontalAlign::Center;
        layout.alignment.vertical = VerticalAlign::Center;
        let rect = place_cue(&cue, &layout, (1920, 1080)).must();
        assert_eq!(
            rect,
            Rect {
                x: 128,
                y: 956,
                width: 128,
                height: 32
            }
        );
    }

    #[test]
    fn baseline_alignment_pins_the_baseline_row() {
        // Baseline row 8 of a 32-row cue: the anchor v pins ROW 8,
        // not the top or bottom — the WebVTT line-snap rule.
        let cue = bitmap(64, 32, 8);
        let mut layout = CueLayout::BOTTOM_CENTER;
        layout.anchor = Anchor::new(Rational::new(1, 2).must(), Rational::new(1, 2).must()).must();
        layout.alignment.vertical = VerticalAlign::Baseline;
        // v = 1/2 of 200 = 100; top = 100 - 8 = 92.
        let rect = place_cue(&cue, &layout, (200, 200)).must();
        assert_eq!(rect.y, 92);
        assert_eq!(rect.height, 32);
        // The same cue BOTTOM-aligned instead: top = 100 - 32 = 68 —
        // the baseline rule differs from the bottom rule by
        // (height - baseline) rows.
        layout.alignment.vertical = VerticalAlign::Bottom;
        let rect = place_cue(&cue, &layout, (200, 200)).must();
        assert_eq!(rect.y, 68);
    }

    #[test]
    fn safe_area_clamps_overflow() {
        // Anchor 1/10 (x=96 at 960 wide — exactly the 5% inset edge)
        // with Start alignment: 96 is legal; a 0-anchor Start cue
        // would sit at x=0 and must clamp INTO the box at x=48
        // (5% of 960 = 48).
        let cue = bitmap(64, 16, 16);
        let mut layout = CueLayout::BOTTOM_CENTER;
        layout.alignment.horizontal = HorizontalAlign::Start;
        layout.anchor = Anchor::new(Rational::ZERO, Rational::new(9, 10).must()).must();
        let rect = place_cue(&cue, &layout, (960, 540)).must();
        assert_eq!(rect.x, 48);
        // Bottom edge: v=9/10 -> 486; bottom-aligned 16-row cue at
        // 470..486 <= 540-27=513 (5% of 540 = 27): no clamp.
        assert_eq!(rect.y, 470);

        // Action-safe (10%): the same cue clamps to x=96.
        layout.margin = SafeMargin::ActionSafe;
        let rect = place_cue(&cue, &layout, (960, 540)).must();
        assert_eq!(rect.x, 96);
    }

    #[test]
    fn cue_larger_than_safe_box_centers_on_extent() {
        // 400 px wide cue in a 480 px frame: 5% box is [24, 456] =
        // 432 px wide — does not fit; center on the full extent:
        // (480-400)/2 = 40.
        let cue = bitmap(400, 16, 16);
        let layout = CueLayout::BOTTOM_CENTER;
        let rect = place_cue(&cue, &layout, (480, 270)).must();
        assert_eq!(rect.x, 40);
    }

    #[test]
    fn scaling_is_uniform_min_ratio() {
        // CIF -> 4K: min(3840/352, 2160/288) = 7.5; 128x32 -> 960x240.
        let cue = bitmap(128, 32, 32);
        let mut layout = CueLayout::BOTTOM_CENTER;
        layout.reference = Some((352, 288));
        let rect = place_cue(&cue, &layout, (3840, 2160)).must();
        assert_eq!((rect.width, rect.height), (960, 240));
        // The anchor stays RELATIVE: v=9/10 of 2160 = 1944, bottom
        // aligned: y = 1944-240 = 1704; clamped into [108, 2052]:
        // 1704+240=1944 <= 2052 OK.
        assert_eq!(rect.y, 1704);

        // Downscale 4K -> CIF: min(352/3840, 288/2160) = 352/3840;
        // 360x180 -> (360*11+60)/120 = 33, (180*11+60)/120 = 17.
        let cue = bitmap(360, 180, 180);
        let mut layout = CueLayout::BOTTOM_CENTER;
        layout.reference = Some((3840, 2160));
        let rect = place_cue(&cue, &layout, (352, 288)).must();
        assert_eq!((rect.width, rect.height), (33, 17));
    }

    #[test]
    fn zero_extent_is_typed() {
        let cue = bitmap(8, 8, 8);
        let err = place_cue(&cue, &CueLayout::BOTTOM_CENTER, (0, 0));
        assert!(matches!(err, Err(ComposeError::ZeroExtent { .. })));
    }

    #[test]
    fn rounding_is_nearest_ties_up() {
        // 1/3 of 100 = 33.33 -> 33; 1/8 of 100 = 12.5 -> 13.
        assert_eq!(Rational::new(1, 3).must().scale_u64(100), 33);
        assert_eq!(Rational::new(1, 8).must().scale_u64(100), 13);
    }
}
