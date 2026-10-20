//! Render parameters and the pure aspect/crop math (doc 03 §4.11 b/e).
//!
//! Scaling and aspect placement is **pure integer math, unit-tested
//! against reference rectangles** (doc 03 §4.11 e) — it never touches a
//! GPU, so it is the same code on every backend (the shader path
//! consumes its output as uniform data).
//!
//! # Doc-sketch adaptations (recorded for Lead review)
//!
//! - doc 03 §4.11 b `RenderParams.background: ColorDesc` — media-types'
//!   [`ColorDescription`](media_types::frame::ColorDescription) is a
//!   frame *interpretation* (primaries/transfer/matrix/range), not a
//!   paintable color; the background here is an sRGB RGBA8 value
//!   ([`Rgba`]) — the thing a compositor actually clears with.
//! - `AspectMode::Crop16x9` ships as the doc names it; the general
//!   config-driven crop-to-arbitrary-ratio set is `[PROPOSAL]` (MP §5).

use crate::color::Rgba;
use crate::error::{RendererError, RendererResult};

/// A 2D size in pixels (doc 03 §4.11 b `Extent2D`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Extent2D {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Extent2D {
    /// The all-ones minimum extent (aspect math clamps to this so a
    /// placement is never empty).
    pub const ONE: Extent2D = Extent2D {
        width: 1,
        height: 1,
    };

    /// Bytes an RGBA8 buffer of this extent occupies, checked.
    ///
    /// # Errors
    /// [`RendererError::ZeroExtent`] for a zero component; the u64
    /// product is saturated beyond 4 GiB × 4 GiB frames, which the
    /// caller's size match rejects.
    #[must_use = "callers must handle the typed extent error"]
    pub fn rgba8_bytes(self) -> RendererResult<u64> {
        self.checked_pixels().map(|p| p.saturating_mul(4))
    }

    /// Pixel count, checked for zero components.
    #[must_use = "callers must handle the typed extent error"]
    pub fn checked_pixels(self) -> RendererResult<u64> {
        if self.width == 0 || self.height == 0 {
            return Err(RendererError::ZeroExtent {
                extent: self,
                from_frame: false,
            });
        }
        Ok(u64::from(self.width) * u64::from(self.height))
    }
}

/// An axis-aligned rectangle (pixel coordinates, top-left origin).
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

impl Rect {
    /// The full extent as a rectangle at the origin.
    #[must_use = "pure constructor"]
    pub fn covering(extent: Extent2D) -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: extent.width,
            height: extent.height,
        }
    }

    /// Intersection with another rectangle, `None` when disjoint.
    #[must_use = "pure geometry"]
    pub fn intersect(self, other: Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        if right > x && bottom > y {
            Some(Rect {
                x,
                y,
                width: right - x,
                height: bottom - y,
            })
        } else {
            None
        }
    }

    /// The rectangle's extent.
    #[must_use = "pure geometry"]
    pub fn extent(self) -> Extent2D {
        Extent2D {
            width: self.width,
            height: self.height,
        }
    }

    /// Centered sub-rectangle of `frame`'s extent with the given size,
    /// clamped into the extent (used by Fill/Crop16x9 source crops).
    #[must_use = "pure geometry"]
    pub fn centered_in(size: Extent2D, frame: Extent2D) -> Rect {
        let x = frame.width.saturating_sub(size.width) / 2;
        let y = frame.height.saturating_sub(size.height) / 2;
        Rect {
            x,
            y,
            width: size.width.min(frame.width),
            height: size.height.min(frame.height),
        }
    }
}

/// How the frame is fitted into the target (doc 03 §4.11 b; the
/// config-driven extension set is `[PROPOSAL]`, MP §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AspectMode {
    /// Letterbox/pillarbox: the whole frame is visible, background
    /// bars fill the rest.
    Fit,
    /// Cover: the target is completely covered, the frame's overflow
    /// is center-cropped.
    Fill,
    /// Ignore aspect: the frame is stretched to the target exactly.
    Stretch,
    /// Center-crop the frame to 16:9 first, then [`Fit`](AspectMode::Fit).
    Crop16x9,
}

/// Per-presentation render parameters (doc 03 §4.11 b).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderParams {
    /// How the frame is fitted into the surface.
    pub aspect: AspectMode,
    /// User crop applied to the frame *before* aspect placement (frame
    /// pixel coordinates).
    pub crop: Option<Rect>,
    /// The clear color for areas the frame does not cover.
    pub background: Rgba,
    /// An optional subtitle layer to alpha-blend over the video
    /// (doc 03 §4.11 "Subtitle compositing [PROPOSAL]" — the texture
    /// id vocabulary; blending itself is the Phase-5 card).
    pub subtitle_layer: Option<u64>,
}

impl RenderParams {
    /// Defaults: Fit, no crop, black background, no subtitle layer.
    #[must_use = "pure constructor"]
    pub fn new() -> RenderParams {
        RenderParams {
            aspect: AspectMode::Fit,
            crop: None,
            background: Rgba::BLACK,
            subtitle_layer: None,
        }
    }
}

impl Default for RenderParams {
    fn default() -> Self {
        RenderParams::new()
    }
}

/// Where a frame lands: the visible source rectangle (frame pixels)
/// and the destination rectangle (surface pixels) — one pure function's
/// output, consumed as shader uniforms (GPU path) or loop bounds
/// (software path).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    /// The visible part of the frame.
    pub source: Rect,
    /// Where it goes on the surface.
    pub destination: Rect,
}

/// Round `a / b` to nearest, ties away from zero (both positive here).
fn div_round(a: u64, b: u64) -> u64 {
    (a + b / 2) / b
}

/// Compute source/destination rectangles for a frame in a target under
/// an aspect mode, after applying the user crop (doc 03 §4.11 e: pure
/// and unit-tested against reference rectangles).
///
/// Invariants (property-tested): both rectangles are non-empty and
/// inside their extents; `Stretch`/`Fill` cover the whole target; `Fit`
/// never scales past the target on either axis.
///
/// # Errors
/// [`RendererError::ZeroExtent`] for zero-sized frames or targets;
/// [`RendererError::CropEmpty`] when the user crop misses the frame.
#[must_use = "callers must handle the typed placement errors"]
pub fn compute_placement(
    frame: Extent2D,
    target: Extent2D,
    params: &RenderParams,
) -> RendererResult<Placement> {
    if frame.width == 0 || frame.height == 0 {
        return Err(RendererError::ZeroExtent {
            extent: frame,
            from_frame: true,
        });
    }
    if target.width == 0 || target.height == 0 {
        return Err(RendererError::ZeroExtent {
            extent: target,
            from_frame: false,
        });
    }

    // 1. User crop (frame pixels), intersected with the frame.
    let mut source = Rect::covering(frame);
    if let Some(crop) = params.crop {
        source = crop
            .intersect(source)
            .ok_or(RendererError::CropEmpty { crop, frame })?;
    }

    // 2. Mode-specific source crop: Crop16x9 narrows the source to the
    //    16:9 center band first (crop, then Fit).
    if params.aspect == AspectMode::Crop16x9 {
        source = crop_to_ratio(source, 16, 9);
    }

    let src_w = u64::from(source.width);
    let src_h = u64::from(source.height);
    let tgt_w = u64::from(target.width);
    let tgt_h = u64::from(target.height);

    let placement = match params.aspect {
        AspectMode::Stretch => Placement {
            source,
            destination: Rect::covering(target),
        },
        AspectMode::Fit | AspectMode::Crop16x9 => {
            // scale = min(tw/sw, th/sh), rounded destination, centered.
            let (dst_w, dst_h) = if src_w * tgt_h >= tgt_w * src_h {
                // Frame at least as wide (relatively) as the target:
                // width-limited.
                (tgt_w, div_round(tgt_w * src_h, src_w).max(1))
            } else {
                (div_round(tgt_h * src_w, src_h).max(1), tgt_h)
            };
            let dst_w = dst_w.min(tgt_w);
            let dst_h = dst_h.min(tgt_h);
            let x = u32::try_from((tgt_w - dst_w) / 2).unwrap_or(0);
            let y = u32::try_from((tgt_h - dst_h) / 2).unwrap_or(0);
            Placement {
                source,
                destination: Rect {
                    x,
                    y,
                    width: dst_w as u32,
                    height: dst_h as u32,
                },
            }
        }
        AspectMode::Fill => {
            // Cover: crop the SOURCE to the target's aspect ratio
            // (center), then map 1:1-logically onto the full target.
            let src = crop_source_to_target_ratio(source, target);
            Placement {
                source: src,
                destination: Rect::covering(target),
            }
        }
    };
    Ok(placement)
}

/// Center-crop a rectangle to a `num:den` aspect ratio (widescreen
/// band), never enlarging it.
fn crop_to_ratio(rect: Rect, num: u64, den: u64) -> Rect {
    let w = u64::from(rect.width);
    let h = u64::from(rect.height);
    if w * den > num * h {
        // Too wide: narrow the width.
        let new_w = div_round(num * h, den).max(1);
        let x = rect.x + u32::try_from((w - new_w) / 2).unwrap_or(0);
        Rect {
            x,
            y: rect.y,
            width: new_w as u32,
            height: rect.height,
        }
    } else {
        // Too tall (or exact): narrow the height.
        let new_h = div_round(den * w, num).max(1);
        let y = rect.y + u32::try_from((h - new_h) / 2).unwrap_or(0);
        Rect {
            x: rect.x,
            y,
            width: rect.width,
            height: new_h as u32,
        }
    }
}

/// Crop the source rectangle so its aspect matches the target's, for
/// cover-style placement (Fill).
fn crop_source_to_target_ratio(source: Rect, target: Extent2D) -> Rect {
    let sw = u64::from(source.width);
    let sh = u64::from(source.height);
    let tw = u64::from(target.width);
    let th = u64::from(target.height);
    if sw * th >= tw * sh {
        // Source relatively wider than the target: crop width.
        let new_w = div_round(sh * tw, th).max(1);
        let x = source.x + u32::try_from((sw - new_w) / 2).unwrap_or(0);
        Rect {
            x,
            y: source.y,
            width: new_w as u32,
            height: source.height,
        }
    } else {
        let new_h = div_round(sw * th, tw).max(1);
        let y = source.y + u32::try_from((sh - new_h) / 2).unwrap_or(0);
        Rect {
            x: source.x,
            y,
            width: source.width,
            height: new_h as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::{Must, assert_err};

    fn rect(x: u32, y: u32, w: u32, h: u32) -> Rect {
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    fn place(frame: (u32, u32), target: (u32, u32), aspect: AspectMode) -> Placement {
        let params = RenderParams {
            aspect,
            ..RenderParams::new()
        };
        compute_placement(
            Extent2D {
                width: frame.0,
                height: frame.1,
            },
            Extent2D {
                width: target.0,
                height: target.1,
            },
            &params,
        )
        .must()
    }

    #[test]
    fn reference_rectangles_fit() {
        // 16:9 frame into 4:3 target: full width, letterbox bars.
        let p = place((640, 360), (320, 240), AspectMode::Fit);
        assert_eq!(p.destination, rect(0, 30, 320, 180));
        assert_eq!(p.source, rect(0, 0, 640, 360));

        // 4:3 frame into 16:9 target: pillarbox, full height.
        let p = place((320, 240), (640, 360), AspectMode::Fit);
        assert_eq!(p.destination, rect(80, 0, 480, 360));

        // Square into square: identity.
        let p = place((100, 100), (100, 100), AspectMode::Fit);
        assert_eq!(p.destination, rect(0, 0, 100, 100));

        // 2:1 frame into 2:1 target: identity.
        let p = place((200, 100), (300, 150), AspectMode::Fit);
        assert_eq!(p.destination, rect(0, 0, 300, 150));
    }

    #[test]
    fn reference_rectangles_fill_and_stretch() {
        // Fill: 4:3 source into 16:9 target crops the source height.
        let p = place((320, 240), (640, 360), AspectMode::Fill);
        assert_eq!(p.destination, rect(0, 0, 640, 360));
        assert_eq!(p.source, rect(0, 30, 320, 180));

        // Fill in the other axis: 16:9 source into 4:3 target crops width.
        let p = place((640, 360), (320, 240), AspectMode::Fill);
        assert_eq!(p.source, rect(80, 0, 480, 360));
        assert_eq!(p.destination, rect(0, 0, 320, 240));

        // Stretch: everything maps to everything.
        let p = place((320, 240), (640, 360), AspectMode::Stretch);
        assert_eq!(p.source, rect(0, 0, 320, 240));
        assert_eq!(p.destination, rect(0, 0, 640, 360));
    }

    #[test]
    fn reference_rectangles_crop16x9() {
        // A 4:3 frame under Crop16x9: the source narrows to the 16:9
        // center band, then Fits (the band is 16:9; the 4:3 target
        // letterboxes it).
        let p = place((320, 240), (320, 240), AspectMode::Crop16x9);
        assert_eq!(p.source, rect(0, 30, 320, 180));
        assert_eq!(p.destination, rect(0, 30, 320, 180));

        // A 16:9 frame is unchanged by the crop step (pure Fit then).
        let p = place((640, 360), (320, 240), AspectMode::Crop16x9);
        assert_eq!(p.source, rect(0, 0, 640, 360));
        assert_eq!(p.destination, rect(0, 30, 320, 180));
    }

    #[test]
    fn user_crop_applies_before_aspect() {
        let params = RenderParams {
            crop: Some(rect(10, 10, 100, 100)),
            ..RenderParams::new()
        };
        let p = compute_placement(
            Extent2D {
                width: 320,
                height: 240,
            },
            Extent2D {
                width: 200,
                height: 200,
            },
            &params,
        )
        .must();
        assert_eq!(p.source, rect(10, 10, 100, 100));
        // A square crop Fits a square target exactly.
        assert_eq!(p.destination, rect(0, 0, 200, 200));
    }

    #[test]
    fn disjoint_crop_is_a_typed_error() {
        let params = RenderParams {
            crop: Some(rect(500, 500, 10, 10)),
            ..RenderParams::new()
        };
        let err = assert_err(
            compute_placement(
                Extent2D {
                    width: 320,
                    height: 240,
                },
                Extent2D {
                    width: 200,
                    height: 200,
                },
                &params,
            ),
            "disjoint crop is a typed error",
        );
        assert_eq!(
            err,
            RendererError::CropEmpty {
                crop: rect(500, 500, 10, 10),
                frame: Extent2D {
                    width: 320,
                    height: 240
                },
            }
        );
    }

    #[test]
    fn zero_extents_are_typed_errors() {
        let params = RenderParams::new();
        let err = assert_err(
            compute_placement(
                Extent2D {
                    width: 0,
                    height: 100,
                },
                Extent2D {
                    width: 100,
                    height: 100,
                },
                &params,
            ),
            "zero-width frame",
        );
        assert_eq!(
            err,
            RendererError::ZeroExtent {
                extent: Extent2D {
                    width: 0,
                    height: 100
                },
                from_frame: true
            }
        );

        let err = assert_err(
            compute_placement(
                Extent2D {
                    width: 100,
                    height: 100,
                },
                Extent2D {
                    width: 100,
                    height: 0,
                },
                &params,
            ),
            "zero-height target",
        );
        assert_eq!(
            err,
            RendererError::ZeroExtent {
                extent: Extent2D {
                    width: 100,
                    height: 0
                },
                from_frame: false
            }
        );
    }

    /// Doc 03 §4.11 f property: aspect/crop math never produces empty
    /// or inverted rects — a deterministic grid sweep over the mode ×
    /// size space (no property framework; doc 04 candidate, 2.C4
    /// precedent).
    #[test]
    fn placement_is_never_empty_or_inverted_on_a_size_grid() {
        let sizes = [1u32, 2, 3, 4, 5, 7, 8, 16, 17, 33, 64, 100, 101, 191, 255];
        let modes = [
            AspectMode::Fit,
            AspectMode::Fill,
            AspectMode::Stretch,
            AspectMode::Crop16x9,
        ];
        let mut checked = 0usize;
        for &fw in &sizes {
            for &fh in &sizes {
                for &tw in &sizes {
                    for &th in &sizes {
                        for &mode in &modes {
                            let params = RenderParams {
                                aspect: mode,
                                ..RenderParams::new()
                            };
                            let p = compute_placement(
                                Extent2D {
                                    width: fw,
                                    height: fh,
                                },
                                Extent2D {
                                    width: tw,
                                    height: th,
                                },
                                &params,
                            )
                            .must();
                            // Non-empty and in-bounds on both sides.
                            assert!(p.source.width >= 1 && p.source.height >= 1);
                            assert!(p.destination.width >= 1 && p.destination.height >= 1);
                            assert!(p.source.x + p.source.width <= fw, "src oob {p:?}");
                            assert!(p.source.y + p.source.height <= fh, "src oob {p:?}");
                            assert!(p.destination.x + p.destination.width <= tw, "dst oob {p:?}");
                            assert!(
                                p.destination.y + p.destination.height <= th,
                                "dst oob {p:?}"
                            );
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(
            checked,
            sizes.len() * sizes.len() * sizes.len() * sizes.len() * modes.len()
        );
    }

    #[test]
    fn fit_never_upscales_past_the_target() {
        let sizes = [1u32, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233];
        for &fw in &sizes {
            for &fh in &sizes {
                for &tw in &sizes {
                    for &th in &sizes {
                        let p = place((fw, fh), (tw, th), AspectMode::Fit);
                        assert!(p.destination.width <= tw);
                        assert!(p.destination.height <= th);
                    }
                }
            }
        }
    }

    #[test]
    fn rect_intersection_geometry() {
        let a = rect(0, 0, 10, 10);
        assert_eq!(a.intersect(rect(5, 5, 10, 10)), Some(rect(5, 5, 5, 5)));
        assert_eq!(a.intersect(rect(10, 10, 5, 5)), None);
        assert_eq!(a.intersect(rect(3, 3, 1, 1)), Some(rect(3, 3, 1, 1)));
        // Touching edges do not intersect (zero-area).
        assert_eq!(a.intersect(rect(10, 0, 5, 5)), None);
    }

    #[test]
    fn centered_in_clamps_into_the_frame() {
        let r = Rect::centered_in(
            Extent2D {
                width: 100,
                height: 50,
            },
            Extent2D {
                width: 60,
                height: 60,
            },
        );
        assert_eq!(r, rect(0, 5, 60, 50));
        // Oversized request clamps.
        let r = Rect::centered_in(
            Extent2D {
                width: 999,
                height: 999,
            },
            Extent2D {
                width: 60,
                height: 60,
            },
        );
        assert_eq!(r, rect(0, 0, 60, 60));
    }

    #[test]
    fn extent_bytes_are_checked() {
        let e = Extent2D {
            width: 4,
            height: 4,
        };
        assert_eq!(e.rgba8_bytes().must(), 64);
        assert_eq!(e.checked_pixels().must(), 16);
        let bad = Extent2D {
            width: 0,
            height: 4,
        };
        assert!(bad.rgba8_bytes().is_err());
    }
}
