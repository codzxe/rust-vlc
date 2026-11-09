//! ASS alignment and margins → the shared positioning vocabularies
//! (card 5.M2b, render path step 1).
//!
//! Two projections of the same input, because the crate has two
//! positioning vocabularies:
//!
//! * [`model_positioning`] — the parser/cue-model vocabulary
//!   ([`Positioning`], WebVTT-flavored integer percents). This is the
//!   seam the 5.M2a parser half consumes when it builds
//!   [`crate::model::Cue`]s. Integer percents are LOSSY (a 20 px margin
//!   at PlayRes 384 is 5.208 % → `5`); the rounding policy is
//!   round-half-up and pinned by test.
//! * [`cue_layout`] — the compositing vocabulary ([`CueLayout`],
//!   exact rationals). This is what the render path actually feeds
//!   [`crate::compositing`]: margins become exact `Rational` anchor
//!   fractions of the SCRIPT resolution, so they scale with the video
//!   extent (the libass behavior, not VSFilter's fixed-pixel margins),
//!   and the cue bitmap is scaled by `reference = play_res`.
//!
//! ## The numpad grid
//!
//! ASS `Alignment` (the v4+ style field and the `\an` override) is a
//! 1–9 numpad: 1/2/3 bottom left/center/right, 4/5/6 middle, 7/8/9
//! top. Both projections map it onto the 3×3 matrix
//! (bottom/middle/top × left/center/right).
//!
//! ## Margin semantics (documented policy)
//!
//! `MarginL`/`MarginR` inset the anchor from the script's left/right
//! edge for left/right alignments (ignored for centered — the anchor
//! is the frame center). `MarginV` measures from the BOTTOM edge for
//! alignments 1–3 and from the TOP edge for 7–9. For middle
//! alignments (4–6) this path ignores `MarginV` and anchors at the
//! frame center — `[HONEST NOTE]` the ASS spec leaves mid-vertical
//! margin behavior to the renderer and libass's exact policy was not
//! source-verified this session; center-of-frame is the conservative
//! reading and the deviation is pinned by test so a later
//! source-verified pass can flip it deliberately.
//!
//! Margins larger than their script axis are typed-invalid (libass
//! would clamp; Lumen prefers MP §8 data over silent moves). The
//! compositor's [`SafeMargin::TitleSafe`] clamp still applies ON TOP
//! of explicit margins — an ASS cue authored flush to the frame edge
//! (margin 0) still lands inside the 5 % title-safe box. That is a
//! deliberate Lumen overlay-discipline deviation from raw libass,
//! recorded here so nobody hunts it as a placement bug.

use crate::ass::render::AssRenderError;
use crate::compositing::layout::{
    Anchor, CueAlignment, CueLayout, HorizontalAlign as CompHorizontal, Rational, SafeMargin,
    VerticalAlign as CompVertical,
};
use crate::model::position::{
    Alignment as ModelAlignment, HorizontalAlign as ModelHorizontal, LinePlacement, Percent,
    Positioning, VerticalAlign as ModelVertical,
};

/// The script resolution used when the ASS header omits `PlayResX`/
/// `PlayResY` (the libass default; VSFilter heritage).
pub const DEFAULT_PLAY_RES: (u32, u32) = (384, 288);

/// The horizontal third of the ASS numpad grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssHorizontal {
    /// Numpad columns 1/4/7.
    Left,
    /// Numpad columns 2/5/8.
    Center,
    /// Numpad columns 3/6/9.
    Right,
}

/// The vertical third of the ASS numpad grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssVertical {
    /// Numpad rows 1/2/3.
    Bottom,
    /// Numpad rows 4/5/6.
    Middle,
    /// Numpad rows 7/8/9.
    Top,
}

/// An ASS alignment, the 1–9 numpad grid (v4+ `Alignment` field,
/// `\an` override). Typed-valid by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssAlignment(u8);

impl AssAlignment {
    /// 1 — bottom left.
    pub const BOTTOM_LEFT: Self = Self(1);
    /// 2 — bottom center (the ASS `Default` style value).
    pub const BOTTOM_CENTER: Self = Self(2);
    /// 3 — bottom right.
    pub const BOTTOM_RIGHT: Self = Self(3);
    /// 4 — middle left.
    pub const MIDDLE_LEFT: Self = Self(4);
    /// 5 — middle center.
    pub const MIDDLE_CENTER: Self = Self(5);
    /// 6 — middle right.
    pub const MIDDLE_RIGHT: Self = Self(6);
    /// 7 — top left.
    pub const TOP_LEFT: Self = Self(7);
    /// 8 — top center.
    pub const TOP_CENTER: Self = Self(8);
    /// 9 — top right.
    pub const TOP_RIGHT: Self = Self(9);

    /// All nine, numpad order 1..=9 (the grid test sweeps exactly this).
    pub const ALL: [AssAlignment; 9] = [
        Self::BOTTOM_LEFT,
        Self::BOTTOM_CENTER,
        Self::BOTTOM_RIGHT,
        Self::MIDDLE_LEFT,
        Self::MIDDLE_CENTER,
        Self::MIDDLE_RIGHT,
        Self::TOP_LEFT,
        Self::TOP_CENTER,
        Self::TOP_RIGHT,
    ];

    /// Construct from the raw ASS value.
    ///
    /// # Errors
    /// [`AssRenderError::BadAlignment`] when `value` is not `1..=9`.
    pub fn new(value: u8) -> Result<Self, AssRenderError> {
        if !(1..=9).contains(&value) {
            return Err(AssRenderError::BadAlignment { value });
        }
        Ok(Self(value))
    }

    /// The raw ASS value (`1..=9`).
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }

    /// The horizontal third.
    #[must_use]
    pub const fn horizontal(self) -> AssHorizontal {
        match self.0 {
            1 | 4 | 7 => AssHorizontal::Left,
            2 | 5 | 8 => AssHorizontal::Center,
            _ => AssHorizontal::Right,
        }
    }

    /// The vertical third.
    #[must_use]
    pub const fn vertical(self) -> AssVertical {
        match self.0 {
            1..=3 => AssVertical::Bottom,
            4..=6 => AssVertical::Middle,
            _ => AssVertical::Top,
        }
    }

    /// Project onto the cue model's 3×3 [`ModelAlignment`].
    #[must_use]
    pub const fn to_model_alignment(self) -> ModelAlignment {
        let vertical = match self.vertical() {
            AssVertical::Bottom => ModelVertical::Bottom,
            AssVertical::Middle => ModelVertical::Middle,
            AssVertical::Top => ModelVertical::Top,
        };
        let horizontal = match self.horizontal() {
            AssHorizontal::Left => ModelHorizontal::Left,
            AssHorizontal::Center => ModelHorizontal::Center,
            AssHorizontal::Right => ModelHorizontal::Right,
        };
        ModelAlignment {
            vertical,
            horizontal,
        }
    }
}

/// ASS style/event margins, in SCRIPT (PlayRes) pixels.
///
/// `MarginV` measures from the bottom edge for alignments 1–3 and from
/// the top edge for 7–9 (see the module docs for the middle policy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssMargins {
    /// Left inset, script px (`MarginL`).
    pub left: u32,
    /// Right inset, script px (`MarginR`).
    pub right: u32,
    /// Vertical inset, script px (`MarginV`).
    pub vertical: u32,
}

impl AssMargins {
    /// The common `Default` style template: 10/10/10 script px.
    pub const ASS_DEFAULT: Self = Self {
        left: 10,
        right: 10,
        vertical: 10,
    };

    /// All margins zero (flush to the script edges; still subject to
    /// the compositor's title-safe clamp downstream).
    pub const ZERO: Self = Self {
        left: 0,
        right: 0,
        vertical: 0,
    };
}

/// Validate the shared invariants: nonzero script resolution, margins
/// within their axes (see the module docs — typed refusal, not clamping).
fn validate(margins: AssMargins, play_res: (u32, u32)) -> Result<(), AssRenderError> {
    if play_res.0 == 0 || play_res.1 == 0 {
        return Err(AssRenderError::ZeroPlayRes { play_res });
    }
    if margins.left > play_res.0 {
        return Err(AssRenderError::MarginExceedsPlayRes {
            axis: "left",
            margin: margins.left,
            extent: play_res.0,
        });
    }
    if margins.right > play_res.0 {
        return Err(AssRenderError::MarginExceedsPlayRes {
            axis: "right",
            margin: margins.right,
            extent: play_res.0,
        });
    }
    if margins.vertical > play_res.1 {
        return Err(AssRenderError::MarginExceedsPlayRes {
            axis: "vertical",
            margin: margins.vertical,
            extent: play_res.1,
        });
    }
    Ok(())
}

/// A margin as a percentage of its axis, round-half-up — the model's
/// coarse integer-percent vocabulary (documented lossy; see the module
/// docs). The `Result` is defensive: pre-validation keeps the value in
/// `0..=100`, so a future invariant break surfaces as the typed
/// [`AssRenderError::Model`], never as a panic (MP §8).
fn margin_percent(margin: u32, extent: u32) -> Result<Percent, AssRenderError> {
    let rounded = ((margin as u64) * 100 + (extent as u64) / 2) / u64::from(extent);
    Percent::new(rounded as i32).map_err(AssRenderError::Model)
}

/// Project an ASS alignment + margins onto the cue model's
/// [`Positioning`] (the 5.M2a parser seam).
///
/// `position`/`line` carry the margins as integer percents where the
/// alignment binds an edge (left/right/top/bottom); centered axes
/// leave them `None` to fall back to the alignment's natural edge.
/// Lossy by the model's integer-percent nature — the compositing path
/// ([`cue_layout`]) uses exact rationals instead.
///
/// # Errors
/// [`AssRenderError::ZeroPlayRes`] / [`AssRenderError::MarginExceedsPlayRes`]
/// via the shared validation.
pub fn model_positioning(
    alignment: AssAlignment,
    margins: AssMargins,
    play_res: (u32, u32),
) -> Result<Positioning, AssRenderError> {
    validate(margins, play_res)?;
    let position = match alignment.horizontal() {
        AssHorizontal::Left => Some(margin_percent(margins.left, play_res.0)?),
        AssHorizontal::Center => None,
        AssHorizontal::Right => {
            let right = margin_percent(margins.right, play_res.0)?;
            Some(Percent::new(100 - i32::from(right.get())).map_err(AssRenderError::Model)?)
        }
    };
    let line = match alignment.vertical() {
        AssVertical::Bottom => {
            let v = margin_percent(margins.vertical, play_res.1)?;
            Some(LinePlacement::Percent(
                Percent::new(100 - i32::from(v.get())).map_err(AssRenderError::Model)?,
            ))
        }
        AssVertical::Middle => None,
        AssVertical::Top => Some(LinePlacement::Percent(margin_percent(
            margins.vertical,
            play_res.1,
        )?)),
    };
    Ok(Positioning {
        alignment: alignment.to_model_alignment(),
        line,
        position,
    })
}

/// Project an ASS alignment + margins onto the compositing contract's
/// [`CueLayout`] — the render path's real output. Exact rationals; the
/// margins become anchor fractions of the script resolution, so they
/// scale with the presentation extent, and `reference = play_res`
/// scales the cue bitmap the same way (uniform, aspect-preserving).
///
/// # Errors
/// [`AssRenderError::ZeroPlayRes`] / [`AssRenderError::MarginExceedsPlayRes`]
/// via the shared validation; [`AssRenderError::Cue`] defensively for
/// any rational/anchor construction failure (pre-checked invariants
/// make those unreachable).
pub fn cue_layout(
    alignment: AssAlignment,
    margins: AssMargins,
    play_res: (u32, u32),
) -> Result<CueLayout, AssRenderError> {
    validate(margins, play_res)?;
    let (u, horizontal) = match alignment.horizontal() {
        AssHorizontal::Left => (
            Rational::new(margins.left, play_res.0)?,
            CompHorizontal::Start,
        ),
        AssHorizontal::Center => (Rational::new(1, 2)?, CompHorizontal::Center),
        AssHorizontal::Right => (
            Rational::new(play_res.0 - margins.right, play_res.0)?,
            CompHorizontal::End,
        ),
    };
    let (v, vertical) = match alignment.vertical() {
        AssVertical::Bottom => (
            Rational::new(play_res.1 - margins.vertical, play_res.1)?,
            CompVertical::Bottom,
        ),
        AssVertical::Middle => (Rational::new(1, 2)?, CompVertical::Center),
        AssVertical::Top => (
            Rational::new(margins.vertical, play_res.1)?,
            CompVertical::Top,
        ),
    };
    Ok(CueLayout {
        anchor: Anchor::new(u, v)?,
        alignment: CueAlignment {
            horizontal,
            vertical,
        },
        margin: SafeMargin::TitleSafe,
        reference: Some(play_res),
    })
}

/// Project an explicit `\pos`/`\move` anchor (G-2, issue #17) onto
/// the compositing contract: the alignment's anchor point of the cue
/// box binds to `(x_px, y_px)` in SCRIPT coordinates — margins are
/// overridden (the libass behaviour for explicit positions; a clamp
/// would move the cue silently, an error would drop it — neither is
/// honest for authored coordinates).
///
/// # Errors
/// [`AssRenderError::ZeroPlayRes`]; [`AssRenderError::Cue`]
/// defensively for rational/anchor construction (the pre-checked
/// invariants make those unreachable).
pub fn cue_layout_at(
    alignment: AssAlignment,
    play_res: (u32, u32),
    pos: (u32, u32),
) -> Result<CueLayout, AssRenderError> {
    if play_res.0 == 0 || play_res.1 == 0 {
        return Err(AssRenderError::ZeroPlayRes { play_res });
    }
    let horizontal = match alignment.horizontal() {
        AssHorizontal::Left => CompHorizontal::Start,
        AssHorizontal::Center => CompHorizontal::Center,
        AssHorizontal::Right => CompHorizontal::End,
    };
    let vertical = match alignment.vertical() {
        AssVertical::Bottom => CompVertical::Bottom,
        AssVertical::Middle => CompVertical::Center,
        AssVertical::Top => CompVertical::Top,
    };
    Ok(CueLayout {
        anchor: Anchor::new(
            Rational::new(pos.0, play_res.0)?,
            Rational::new(pos.1, play_res.1)?,
        )?,
        alignment: CueAlignment {
            horizontal,
            vertical,
        },
        margin: SafeMargin::TitleSafe,
        reference: Some(play_res),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AssAlignment, AssHorizontal, AssMargins, AssVertical, DEFAULT_PLAY_RES, cue_layout,
        cue_layout_at, model_positioning,
    };
    use crate::ass::render::AssRenderError;
    use crate::compositing::layout::VerticalAlign as CompVertical;
    use crate::model::position::{LinePlacement, Percent, Viewport};
    use testkit::Must;

    #[test]
    fn numpad_grid_maps_all_nine_to_the_model_matrix() {
        let expected = [["bl", "bc", "br"], ["ml", "mc", "mr"], ["tl", "tc", "tr"]];
        for (i, align) in AssAlignment::ALL.iter().enumerate() {
            assert_eq!(usize::from(align.get()), i + 1);
            assert_eq!(align.to_model_alignment().code(), expected[i / 3][i % 3]);
        }
        assert_eq!(
            AssAlignment::MIDDLE_CENTER.horizontal(),
            AssHorizontal::Center
        );
        assert_eq!(AssAlignment::TOP_RIGHT.vertical(), AssVertical::Top);
    }

    #[test]
    fn bad_alignment_is_typed() {
        for bad in [0u8, 10, 255] {
            assert_eq!(
                AssAlignment::new(bad),
                Err(AssRenderError::BadAlignment { value: bad })
            );
        }
    }

    #[test]
    fn model_positioning_carries_margins_as_rounded_percents() {
        // 20/384 = 5.208% -> 5; 30/288 = 10.4166% -> 10; bottom line
        // anchors at 100 - 10 = 90.
        let margins = AssMargins {
            left: 20,
            right: 20,
            vertical: 30,
        };
        let p = model_positioning(AssAlignment::BOTTOM_LEFT, margins, (384, 288)).must();
        assert_eq!(p.position, Some(Percent::new(5).must()));
        assert_eq!(
            p.line,
            Some(LinePlacement::Percent(Percent::new(90).must()))
        );
        // Resolve against the same script size: x = 5% of 384 = 19.7 ->
        // 19, y = 90% of 288 = 259.7 -> 259 (the documented LOSSY path;
        // the exact compositor path pins 20 / 258 in the render tests).
        let anchor = p.resolve(Viewport::new(384, 288).must(), 24).must();
        assert_eq!((anchor.x_px, anchor.y_px), (19, 259));

        // Right/top alignment: anchor pulled in from the opposite edge.
        let p = model_positioning(AssAlignment::TOP_RIGHT, margins, (384, 288)).must();
        assert_eq!(p.position, Some(Percent::new(95).must()));
        assert_eq!(
            p.line,
            Some(LinePlacement::Percent(Percent::new(10).must()))
        );

        // Centered axes leave the model's fallbacks in place.
        let p = model_positioning(
            AssAlignment::MIDDLE_CENTER,
            AssMargins::ASS_DEFAULT,
            DEFAULT_PLAY_RES,
        )
        .must();
        assert_eq!(p.position, None);
        assert_eq!(p.line, None);
    }

    #[test]
    fn cue_layout_builds_exact_rational_anchors() {
        let margins = AssMargins {
            left: 20,
            right: 20,
            vertical: 30,
        };
        let l = cue_layout(AssAlignment::BOTTOM_LEFT, margins, (384, 288)).must();
        // 20/384 = 5/96, (288-30)/288 = 43/48 — reduced, exact.
        assert_eq!((l.anchor.u.num, l.anchor.u.den), (5, 96));
        assert_eq!((l.anchor.v.num, l.anchor.v.den), (43, 48));
        assert_eq!(l.alignment.vertical, CompVertical::Bottom);
        // The reference extent is the script resolution: the cue bitmap
        // scales with the video, margins included.
        assert_eq!(l.reference, Some((384, 288)));

        // Top alignment anchors MarginV from the TOP edge.
        let l = cue_layout(AssAlignment::TOP_LEFT, margins, (384, 288)).must();
        assert_eq!((l.anchor.v.num, l.anchor.v.den), (5, 48));
    }

    #[test]
    fn middle_alignment_anchors_the_frame_center() {
        // The documented policy: MarginV ignored for 4-6 (module docs
        // [HONEST NOTE] — pinned here so a deliberate change is visible).
        let with = cue_layout(
            AssAlignment::MIDDLE_CENTER,
            AssMargins {
                left: 0,
                right: 0,
                vertical: 30,
            },
            (384, 288),
        )
        .must();
        let without = cue_layout(AssAlignment::MIDDLE_CENTER, AssMargins::ZERO, (384, 288)).must();
        assert_eq!(with.anchor, without.anchor);
        assert_eq!((with.anchor.v.num, with.anchor.v.den), (1, 2));
    }

    #[test]
    fn cue_layout_at_pins_the_pos_anchor_exactly() {
        // \pos(192,144) at 384×288 → the exact midpoint, margins
        // ignored, alignment still binding the same cue-box point.
        let l = cue_layout_at(AssAlignment::BOTTOM_CENTER, (384, 288), (192, 144)).must();
        assert_eq!((l.anchor.u.num, l.anchor.u.den), (1, 2));
        assert_eq!((l.anchor.v.num, l.anchor.v.den), (1, 2));
        assert_eq!(l.alignment.vertical, CompVertical::Bottom);
        assert_eq!(l.reference, Some((384, 288)));
        // Margins would have moved a non-pos layout; with pos they do
        // not (the documented override).
        let with_margins = cue_layout(
            AssAlignment::BOTTOM_CENTER,
            AssMargins {
                left: 50,
                right: 50,
                vertical: 60,
            },
            (384, 288),
        )
        .must();
        assert_ne!(with_margins.anchor, l.anchor);
        // Zero script resolution stays typed-invalid.
        assert_eq!(
            cue_layout_at(AssAlignment::BOTTOM_CENTER, (0, 288), (1, 1)),
            Err(AssRenderError::ZeroPlayRes { play_res: (0, 288) })
        );
    }

    #[test]
    fn margin_and_play_res_violations_are_typed() {
        let m = AssMargins {
            left: 385,
            right: 0,
            vertical: 0,
        };
        assert_eq!(
            cue_layout(AssAlignment::BOTTOM_LEFT, m, (384, 288)),
            Err(AssRenderError::MarginExceedsPlayRes {
                axis: "left",
                margin: 385,
                extent: 384
            })
        );
        let m = AssMargins {
            left: 0,
            right: 0,
            vertical: 289,
        };
        assert_eq!(
            model_positioning(AssAlignment::BOTTOM_CENTER, m, (384, 288)),
            Err(AssRenderError::MarginExceedsPlayRes {
                axis: "vertical",
                margin: 289,
                extent: 288
            })
        );
        assert_eq!(
            cue_layout(AssAlignment::BOTTOM_CENTER, AssMargins::ZERO, (0, 288)),
            Err(AssRenderError::ZeroPlayRes { play_res: (0, 288) })
        );
        assert_eq!(
            model_positioning(AssAlignment::BOTTOM_CENTER, AssMargins::ZERO, (384, 0)),
            Err(AssRenderError::ZeroPlayRes { play_res: (384, 0) })
        );
        // Margin exactly equal to the axis is the degenerate-but-legal
        // boundary (anchor at the opposite edge), not an error.
        assert!(
            cue_layout(
                AssAlignment::BOTTOM_CENTER,
                AssMargins {
                    left: 384,
                    right: 384,
                    vertical: 288
                },
                (384, 288)
            )
            .is_ok()
        );
    }
}
