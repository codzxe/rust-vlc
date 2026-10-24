//! Positioning and alignment primitives (card 5.L2): the 3×3 alignment
//! matrix, line/position percentages, and the relative→absolute
//! (percent → pixel) resolution.
//!
//! ## Semantics [PROPOSAL — Lead unifies with 5.L1's parser contract]
//!
//! WebVTT-aligned, because it is the cleanest published vocabulary that
//! covers SRT/ASS subsets:
//!
//! * [`Positioning::position`] is the **horizontal** percentage of the
//!   video width where the cue box's alignment anchor sits;
//! * [`Positioning::line`] is the **vertical** placement — either a
//!   percentage of the video height or a count of text lines from the
//!   top (negative counts from the bottom, WebVTT §`line`);
//! * [`Alignment`] names which point of the cue box the anchor binds
//!   (9 combinations: vertical {top, middle, bottom} × horizontal
//!   {left, center, right}).
//!
//! Out-of-range percentages are typed-invalid at construction
//! ([`ModelError::PercentOutOfRange`]) — a renderer never sees one.

use crate::model::error::ModelError;

/// A percentage in `0..=100` (typed-valid by construction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Percent(u8);

impl Percent {
    /// 0%.
    pub const ZERO: Self = Self(0);
    /// 50%.
    pub const HALF: Self = Self(50);
    /// 100%.
    pub const FULL: Self = Self(100);

    /// Construct a percentage.
    ///
    /// # Errors
    /// [`ModelError::PercentOutOfRange`] when `value` is not in
    /// `0..=100`.
    pub fn new(value: i32) -> Result<Self, ModelError> {
        if !(0..=100).contains(&value) {
            return Err(ModelError::PercentOutOfRange { value });
        }
        Ok(Self(value as u8))
    }

    /// The validated value in `0..=100`.
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// The vertical component of a cue-box alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerticalAlign {
    /// Anchor at the top edge of the cue box.
    Top,
    /// Anchor at the vertical middle.
    Middle,
    /// Anchor at the bottom edge (the classic subtitle position).
    Bottom,
}

/// The horizontal component of a cue-box alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HorizontalAlign {
    /// Anchor at the left edge of the cue box.
    Left,
    /// Anchor at the horizontal center.
    Center,
    /// Anchor at the right edge.
    Right,
}

/// A cue-box alignment: one of the nine combinations of
/// [`VerticalAlign`] × [`HorizontalAlign`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Alignment {
    /// Vertical anchor component.
    pub vertical: VerticalAlign,
    /// Horizontal anchor component.
    pub horizontal: HorizontalAlign,
}

impl Alignment {
    /// All nine alignments, in a fixed order (the roundtrip-matrix test
    /// sweeps exactly this array).
    pub const ALL: [Alignment; 9] = [
        Alignment {
            vertical: VerticalAlign::Top,
            horizontal: HorizontalAlign::Left,
        },
        Alignment {
            vertical: VerticalAlign::Top,
            horizontal: HorizontalAlign::Center,
        },
        Alignment {
            vertical: VerticalAlign::Top,
            horizontal: HorizontalAlign::Right,
        },
        Alignment {
            vertical: VerticalAlign::Middle,
            horizontal: HorizontalAlign::Left,
        },
        Alignment {
            vertical: VerticalAlign::Middle,
            horizontal: HorizontalAlign::Center,
        },
        Alignment {
            vertical: VerticalAlign::Middle,
            horizontal: HorizontalAlign::Right,
        },
        Alignment {
            vertical: VerticalAlign::Bottom,
            horizontal: HorizontalAlign::Left,
        },
        Alignment {
            vertical: VerticalAlign::Bottom,
            horizontal: HorizontalAlign::Center,
        },
        Alignment {
            vertical: VerticalAlign::Bottom,
            horizontal: HorizontalAlign::Right,
        },
    ];

    /// Construct from components.
    pub const fn new(vertical: VerticalAlign, horizontal: HorizontalAlign) -> Self {
        Self {
            vertical,
            horizontal,
        }
    }

    /// The stable 2-character wire code: one of
    /// `tl tc tr ml mc mr bl bc br`.
    pub const fn code(self) -> &'static str {
        match (self.vertical, self.horizontal) {
            (VerticalAlign::Top, HorizontalAlign::Left) => "tl",
            (VerticalAlign::Top, HorizontalAlign::Center) => "tc",
            (VerticalAlign::Top, HorizontalAlign::Right) => "tr",
            (VerticalAlign::Middle, HorizontalAlign::Left) => "ml",
            (VerticalAlign::Middle, HorizontalAlign::Center) => "mc",
            (VerticalAlign::Middle, HorizontalAlign::Right) => "mr",
            (VerticalAlign::Bottom, HorizontalAlign::Left) => "bl",
            (VerticalAlign::Bottom, HorizontalAlign::Center) => "bc",
            (VerticalAlign::Bottom, HorizontalAlign::Right) => "br",
        }
    }

    /// Parse a 2-character wire code produced by [`Alignment::code`].
    ///
    /// # Errors
    /// `None` (plain `Option`: this is a total, non-contextual lookup)
    /// when the code is not one of the nine.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "tl" => Some(Self {
                vertical: VerticalAlign::Top,
                horizontal: HorizontalAlign::Left,
            }),
            "tc" => Some(Self {
                vertical: VerticalAlign::Top,
                horizontal: HorizontalAlign::Center,
            }),
            "tr" => Some(Self {
                vertical: VerticalAlign::Top,
                horizontal: HorizontalAlign::Right,
            }),
            "ml" => Some(Self {
                vertical: VerticalAlign::Middle,
                horizontal: HorizontalAlign::Left,
            }),
            "mc" => Some(Self {
                vertical: VerticalAlign::Middle,
                horizontal: HorizontalAlign::Center,
            }),
            "mr" => Some(Self {
                vertical: VerticalAlign::Middle,
                horizontal: HorizontalAlign::Right,
            }),
            "bl" => Some(Self {
                vertical: VerticalAlign::Bottom,
                horizontal: HorizontalAlign::Left,
            }),
            "bc" => Some(Self {
                vertical: VerticalAlign::Bottom,
                horizontal: HorizontalAlign::Center,
            }),
            "br" => Some(Self {
                vertical: VerticalAlign::Bottom,
                horizontal: HorizontalAlign::Right,
            }),
            _ => None,
        }
    }
}

/// The vertical placement of a cue: a percentage of the video height, or
/// a count of text lines (negative counts from the bottom, WebVTT
/// `line` semantics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinePlacement {
    /// A percentage of the rendering viewport height.
    Percent(Percent),
    /// A text-line count from the top; negative from the bottom.
    LineNumber(i64),
}

/// A cue's positioning: alignment plus optional explicit placement.
///
/// `None` fields mean "format did not specify" — resolution then falls
/// back to the alignment's natural edge (see [`Positioning::resolve`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Positioning {
    /// Which point of the cue box the anchor binds.
    pub alignment: Alignment,
    /// Vertical placement, if the format specified one.
    pub line: Option<LinePlacement>,
    /// Horizontal position as a percentage of the viewport width.
    pub position: Option<Percent>,
}

impl Default for Positioning {
    /// Bottom-center, no explicit placement — the classic subtitle
    /// default (WebVTT's default when no settings are given).
    fn default() -> Self {
        Self {
            alignment: Alignment::new(VerticalAlign::Bottom, HorizontalAlign::Center),
            line: None,
            position: None,
        }
    }
}

impl Positioning {
    /// Construct with explicit fields.
    pub const fn new(
        alignment: Alignment,
        line: Option<LinePlacement>,
        position: Option<Percent>,
    ) -> Self {
        Self {
            alignment,
            line,
            position,
        }
    }
}

/// A rendering viewport in device pixels. Both extents must be nonzero
/// (nothing can be placed in an empty viewport — typed, not a silent 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Viewport {
    /// Width in pixels.
    pub width_px: u32,
    /// Height in pixels.
    pub height_px: u32,
}

impl Viewport {
    /// Construct a viewport.
    ///
    /// # Errors
    /// [`ModelError::EmptyViewport`] when either extent is zero.
    pub fn new(width_px: u32, height_px: u32) -> Result<Self, ModelError> {
        if width_px == 0 || height_px == 0 {
            return Err(ModelError::EmptyViewport);
        }
        Ok(Self {
            width_px,
            height_px,
        })
    }
}

/// A resolved (absolute, pixel-space) cue anchor: the minimal
/// relative→absolute primitive output. Copy — renderers stack it by
/// value (D-1: no heap for fixed-size placement data).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResolvedAnchor {
    /// Horizontal anchor position in pixels.
    pub x_px: u32,
    /// Vertical anchor position in pixels.
    pub y_px: u32,
    /// Which cue-box point binds to the anchor.
    pub alignment: Alignment,
}

/// Convert a percentage of `extent_px` to pixels, round-half-up,
/// saturating at `u32::MAX`. The `u64` intermediate cannot overflow
/// (`u32::MAX × 100 + 50 < 2^40`).
#[must_use]
pub const fn percent_to_px(percent: Percent, extent_px: u32) -> u32 {
    let scaled = (extent_px as u64 * percent.get() as u64).saturating_add(50) / 100;
    if scaled > u32::MAX as u64 {
        u32::MAX
    } else {
        scaled as u32
    }
}

/// Convert a WebVTT-style line number to pixels: `n >= 0` counts
/// `line_height_px` rows from the top, `n < 0` from the bottom; the
/// result is clamped into `[0, height_px]`. A zero `line_height_px`
/// degenerates to the respective edge (top rows collapse to 0, bottom
/// rows to `height_px`) — documented, not an error.
#[must_use]
pub fn line_number_to_px(line: i64, line_height_px: u32, height_px: u32) -> u32 {
    let raw = if line >= 0 {
        line.saturating_mul(line_height_px as i64)
    } else {
        (height_px as i64).saturating_add(line.saturating_mul(line_height_px as i64))
    };
    raw.clamp(0, height_px as i64) as u32
}

impl Positioning {
    /// Resolve this positioning to an absolute pixel anchor — the
    /// relative→absolute primitive (card 5.L2, minimal types).
    ///
    /// Unspecified fields fall back to the alignment's natural edge:
    /// no `position` → left 0 / center `w/2` / right `w`; no `line` →
    /// top 0 / middle `h/2` / bottom `h`.
    ///
    /// # Errors
    /// [`ModelError::EmptyViewport`] via [`Viewport::new`].
    pub fn resolve(
        &self,
        viewport: Viewport,
        line_height_px: u32,
    ) -> Result<ResolvedAnchor, ModelError> {
        let x_px = match self.position {
            Some(p) => percent_to_px(p, viewport.width_px),
            None => match self.alignment.horizontal {
                HorizontalAlign::Left => 0,
                HorizontalAlign::Center => viewport.width_px / 2,
                HorizontalAlign::Right => viewport.width_px,
            },
        };
        let y_px = match self.line {
            Some(LinePlacement::Percent(p)) => percent_to_px(p, viewport.height_px),
            Some(LinePlacement::LineNumber(n)) => {
                line_number_to_px(n, line_height_px, viewport.height_px)
            }
            None => match self.alignment.vertical {
                VerticalAlign::Top => 0,
                VerticalAlign::Middle => viewport.height_px / 2,
                VerticalAlign::Bottom => viewport.height_px,
            },
        };
        Ok(ResolvedAnchor {
            x_px,
            y_px,
            alignment: self.alignment,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Alignment, HorizontalAlign, LinePlacement, Percent, Positioning, VerticalAlign, Viewport,
        line_number_to_px, percent_to_px,
    };
    use crate::model::error::ModelError;
    use std::error::Error;

    #[test]
    fn percent_bounds() {
        assert_eq!(Percent::new(0), Ok(Percent::ZERO));
        assert_eq!(Percent::new(100), Ok(Percent::FULL));
        assert_eq!(
            Percent::new(101),
            Err(ModelError::PercentOutOfRange { value: 101 })
        );
        assert_eq!(
            Percent::new(-1),
            Err(ModelError::PercentOutOfRange { value: -1 })
        );
    }

    #[test]
    fn alignment_codes_roundtrip_all_nine() {
        for a in Alignment::ALL {
            assert_eq!(Alignment::from_code(a.code()), Some(a));
        }
        assert_eq!(Alignment::from_code("xx"), None);
        assert_eq!(Alignment::from_code("t"), None);
    }

    #[test]
    fn percent_to_px_rounds_half_up_and_saturates() -> Result<(), Box<dyn Error>> {
        assert_eq!(percent_to_px(Percent::HALF, 1920), 960);
        assert_eq!(percent_to_px(Percent::new(33)?, 1000), 330);
        assert_eq!(percent_to_px(Percent::HALF, 1), 1); // 0.5 → 1
        assert_eq!(percent_to_px(Percent::new(49)?, 1), 0); // 0.49 → 0
        assert_eq!(percent_to_px(Percent::FULL, u32::MAX), u32::MAX);
        Ok(())
    }

    #[test]
    fn resolve_fallbacks_and_explicit() -> Result<(), Box<dyn Error>> {
        let vp = Viewport::new(1920, 1080)?;
        // Default (bottom-center, nothing specified) → bottom-center of screen.
        let d = Positioning::default().resolve(vp, 42)?;
        assert_eq!((d.x_px, d.y_px), (960, 1080));
        // Explicit percent line/position win over alignment fallbacks.
        let p = Positioning::new(
            Alignment::new(VerticalAlign::Top, HorizontalAlign::Left),
            Some(LinePlacement::Percent(Percent::FULL)),
            Some(Percent::FULL),
        );
        let r = p.resolve(vp, 42)?;
        assert_eq!((r.x_px, r.y_px), (1920, 1080));
        // Line numbers: +2 rows from the top, -1 row from the bottom.
        let p = Positioning::new(
            Alignment::new(VerticalAlign::Top, HorizontalAlign::Left),
            Some(LinePlacement::LineNumber(2)),
            None,
        );
        let r = p.resolve(vp, 40)?;
        assert_eq!(r.y_px, 80);
        let p = Positioning::new(
            Alignment::new(VerticalAlign::Top, HorizontalAlign::Left),
            Some(LinePlacement::LineNumber(-1)),
            None,
        );
        let r = p.resolve(vp, 40)?;
        assert_eq!(r.y_px, 1040);
        Ok(())
    }

    #[test]
    fn empty_viewport_is_typed_invalid() {
        assert_eq!(Viewport::new(0, 1080), Err(ModelError::EmptyViewport));
        assert_eq!(Viewport::new(1920, 0), Err(ModelError::EmptyViewport));
    }

    #[test]
    fn line_number_clamps() {
        assert_eq!(line_number_to_px(i64::MAX, 40, 1080), 1080);
        assert_eq!(line_number_to_px(i64::MIN, 40, 1080), 0);
        assert_eq!(line_number_to_px(5, 0, 1080), 0);
        assert_eq!(line_number_to_px(-5, 0, 1080), 1080);
    }
}
