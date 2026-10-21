//! The backend seam: compositors turn staged frames into target
//! pixels (doc 03 §4.11 a; doc 01 §2.6 "the only GPU citizen").
//!
//! One trait, two worlds — the decision record for which runs where:
//!
//! - [`SoftwareCompositor`] (this file): a minimal CPU rasterizer —
//!   point sampling (nearest neighbor) over the placement rectangles,
//!   background fill outside. It runs **anywhere a CPU runs**, and is
//!   the compositor the 7.C1 fallback chain formalizes
//!   ("auto-negotiated wgpu backend → software present"). This
//!   sandbox has no GPU stack at all (no `/dev/dri`, no Vulkan ICDs,
//!   no libEGL, no display — `records/wgpu-30.0.1.md` §1, including
//!   the live adapter probe), so v0 links exactly this one and the
//!   acceptance proof runs through it.
//! - The wgpu compositor (owed to the Lead-gated import — the crate
//!   docs and `records/wgpu-30.0.1.md`): same trait, real pipelines —
//!   GPU color conversion (doc 03 §4.11 e), bilinear filtering in
//!   linear light, subtitle-layer blending in the shader. It implements
//!   the same [`Compositor`] contract against `wgpu::Device`/`Queue`
//!   and bypasses the CPU write protocol with its own draw path.
//!
//! # Sampling honesty
//!
//! Point sampling needs no gamma-domain conversion: source bytes are
//! copied, never blended, so the byte-exact sRGB passthrough invariant
//! (crate `color` docs) holds under scaling too. Bilinear filtering
//! belongs to the linear-light shader path and is deliberately absent
//! from v0.

use crate::color::Rgba;
use crate::error::{RendererError, RendererResult};
use crate::params::{Placement, Rect};
use crate::staging::StagedFrame;
use crate::surface::RenderSurface;

/// Composites one staged frame onto one target (the hardware/software
/// seam — see the module docs).
pub trait Compositor: Send {
    /// The backend's identity for capability reporting (doc 01 §2.7
    /// truthfulness precedent: the renderer reports what is actually
    /// linked — `Renderer::backend_name`).
    fn name(&self) -> &'static str;

    /// Place `frame` onto `target` per `placement`, clearing anything
    /// the placement does not cover to `background` first.
    ///
    /// # Errors
    /// Propagates the target's protocol errors ([`RendererError`]
    /// family); never panics on pixel coordinates (the placement math
    /// bounds them; sampling saturates).
    fn composite(
        &mut self,
        frame: &StagedFrame,
        placement: &Placement,
        background: Rgba,
        target: &mut dyn RenderSurface,
    ) -> RendererResult<()>;
}

/// The CPU rasterizer: point-sampled nearest-neighbor placement over
/// the [`RenderSurface`] write protocol (module docs).
#[derive(Debug, Default)]
pub struct SoftwareCompositor;

impl SoftwareCompositor {
    /// A fresh software compositor (stateless — the placement carries
    /// all per-frame state).
    #[must_use = "pure constructor"]
    pub fn new() -> SoftwareCompositor {
        SoftwareCompositor
    }
}

impl Compositor for SoftwareCompositor {
    fn name(&self) -> &'static str {
        "software"
    }

    fn composite(
        &mut self,
        frame: &StagedFrame,
        placement: &Placement,
        background: Rgba,
        target: &mut dyn RenderSurface,
    ) -> RendererResult<()> {
        let target_extent = target.extent();
        // 1. Background pass: everything the frame will not cover.
        target.fill_rect(Rect::covering(target_extent), background)?;

        // 2. Frame pass: nearest-neighbor sampling into the
        //    destination rectangle, delivered as one RGBA block per
        //    composite (the write-protocol's natural batch).
        let dst = placement.destination;
        if dst.width == 0 || dst.height == 0 {
            // An empty destination is legal placement output only for
            // zero-area targets, which surfaces reject at construction;
            // reaching here means the placement math was bypassed.
            return Err(RendererError::ZeroExtent {
                extent: dst.extent(),
                from_frame: false,
            });
        }

        let src = placement.source;
        let mut block = Vec::with_capacity(dst.width as usize * dst.height as usize * 4);
        for dy in 0..dst.height {
            // Source row: nearest neighbor with the destination pixel
            // center mapped back into the source rectangle.
            let sy = src.y + nearest(dy, dst.height, src.height);
            for dx in 0..dst.width {
                let sx = src.x + nearest(dx, dst.width, src.width);
                block.extend_from_slice(&frame.pixel(sx, sy));
            }
        }
        target.write_rect(dst, &block)
    }
}

/// Nearest-neighbor coordinate map: the `i`-th of `dst_n` destination
/// pixels (0-based) to a source pixel index in `0..src_n`.
///
/// Invariant (test-pinned): the result is always `< src_n` for
/// `dst_n ≥ 1`, `src_n ≥ 1`; at `dst_n == src_n` it is the identity —
/// the 1:1 placement is a byte-exact passthrough.
#[must_use = "pure sampling math"]
fn nearest(i: u32, dst_n: u32, src_n: u32) -> u32 {
    if dst_n == 1 {
        // A single destination pixel samples the source's center.
        return src_n / 2;
    }
    // (i + 0.5) * src_n / dst_n, rounded down, clamped to src_n - 1.
    let v = (u64::from(i) * 2 + 1) * u64::from(src_n) / (u64::from(dst_n) * 2);
    (v.min(u64::from(src_n) - 1)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::SurfaceFormat;
    use crate::params::Extent2D;
    use crate::surface::{HeadlessSurface, PresentMode, SurfaceConfig};
    use testkit::Must;

    fn headless(w: u32, h: u32, format: SurfaceFormat) -> HeadlessSurface {
        HeadlessSurface::new(
            Extent2D {
                width: w,
                height: h,
            },
            SurfaceConfig {
                format,
                present_mode: PresentMode::Fifo,
            },
        )
        .must()
    }

    fn staged(w: u32, h: u32, paint: impl Fn(u32, u32) -> [u8; 4]) -> StagedFrame {
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&paint(x, y));
            }
        }
        StagedFrame {
            extent: Extent2D {
                width: w,
                height: h,
            },
            rgba,
        }
    }

    fn full_placement(frame: Extent2D, target: Extent2D) -> Placement {
        Placement {
            source: Rect::covering(frame),
            destination: Rect::covering(target),
        }
    }

    #[test]
    fn one_to_one_placement_is_a_byte_exact_passthrough() {
        // The sRGB no-double-conversion invariant, at the compositor
        // level: 2x2 frame → 2x2 target through the BGRA surface.
        let frame = staged(2, 2, |x, y| [x as u8 * 40 + 10, y as u8 * 80 + 5, 7, 255]);
        let mut target = headless(2, 2, SurfaceFormat::Bgra8Srgb);
        SoftwareCompositor::new()
            .composite(
                &frame,
                &full_placement(frame.extent, target.extent()),
                Rgba::BLACK,
                &mut target,
            )
            .must();
        let out = target.readback_rgba().must();
        assert_eq!(out, frame.rgba, "1:1 point sampling must copy bytes");
    }

    #[test]
    fn background_fills_what_the_frame_does_not_cover() {
        // 1x1 frame letterboxed in a 3x3 target: bars everywhere but
        // the center pixel.
        let frame = staged(1, 1, |_, _| [255, 0, 0, 255]);
        let mut target = headless(3, 3, SurfaceFormat::Rgba8Srgb);
        let placement = Placement {
            source: Rect::covering(Extent2D {
                width: 1,
                height: 1,
            }),
            destination: Rect {
                x: 1,
                y: 1,
                width: 1,
                height: 1,
            },
        };
        SoftwareCompositor::new()
            .composite(&frame, &placement, Rgba::WHITE, &mut target)
            .must();
        let out = target.readback_rgba().must();
        let px = |x: usize, y: usize| &out[(y * 3 + x) * 4..(y * 3 + x) * 4 + 4];
        assert_eq!(px(0, 0), &[255, 255, 255, 255]);
        assert_eq!(px(1, 1), &[255, 0, 0, 255]);
        assert_eq!(px(2, 2), &[255, 255, 255, 255]);
    }

    #[test]
    fn upscale_repeats_source_pixels_nearest_neighbor() {
        // 2x1 frame into 4x1 target: each source pixel covers two
        // destination pixels.
        let frame = staged(2, 1, |x, _| [if x == 0 { 10 } else { 20 }, 0, 0, 255]);
        let mut target = headless(4, 1, SurfaceFormat::Rgba8Srgb);
        SoftwareCompositor::new()
            .composite(
                &frame,
                &full_placement(
                    Extent2D {
                        width: 2,
                        height: 1,
                    },
                    target.extent(),
                ),
                Rgba::BLACK,
                &mut target,
            )
            .must();
        let out = target.readback_rgba().must();
        let reds: Vec<u8> = out.as_chunks::<4>().0.iter().map(|p| p[0]).collect();
        assert_eq!(reds, vec![10, 10, 20, 20]);
    }

    #[test]
    fn downscale_picks_source_pixel_centers() {
        // 4x1 frame into 2x1 target: centers at 0.25/0.75 → sources
        // 1 and... (i+0.5)*4/2: i=0 → 1.0 → 1; i=1 → 3.0 → 3.
        let frame = staged(4, 1, |x, _| [x as u8 * 10, 0, 0, 255]);
        let mut target = headless(2, 1, SurfaceFormat::Rgba8Srgb);
        SoftwareCompositor::new()
            .composite(
                &frame,
                &full_placement(
                    Extent2D {
                        width: 4,
                        height: 1,
                    },
                    target.extent(),
                ),
                Rgba::BLACK,
                &mut target,
            )
            .must();
        let out = target.readback_rgba().must();
        let reds: Vec<u8> = out.as_chunks::<4>().0.iter().map(|p| p[0]).collect();
        assert_eq!(reds, vec![10, 30]);
    }

    #[test]
    fn source_crop_rect_selects_the_visible_band() {
        // A 4x1 frame with a source rect covering only the last two
        // pixels, into a 2x1 target: byte passthrough of the band.
        let frame = staged(4, 1, |x, _| [x as u8 * 10, 0, 0, 255]);
        let placement = Placement {
            source: Rect {
                x: 2,
                y: 0,
                width: 2,
                height: 1,
            },
            destination: Rect {
                x: 0,
                y: 0,
                width: 2,
                height: 1,
            },
        };
        let mut target = headless(2, 1, SurfaceFormat::Rgba8Srgb);
        SoftwareCompositor::new()
            .composite(&frame, &placement, Rgba::BLACK, &mut target)
            .must();
        let out = target.readback_rgba().must();
        let reds: Vec<u8> = out.as_chunks::<4>().0.iter().map(|p| p[0]).collect();
        assert_eq!(reds, vec![20, 30]);
    }

    #[test]
    fn nearest_math_is_bounded_identity_and_center() {
        // Identity at equal sizes.
        for i in 0..8u32 {
            assert_eq!(nearest(i, 8, 8), i);
        }
        // Always in bounds.
        for dst_n in 1..17u32 {
            for src_n in 1..17u32 {
                for i in 0..dst_n {
                    let v = nearest(i, dst_n, src_n);
                    assert!(v < src_n, "nearest({i},{dst_n},{src_n}) = {v}");
                }
            }
        }
        // Singleton destination samples the source center.
        assert_eq!(nearest(0, 1, 4), 2);
        assert_eq!(nearest(0, 1, 5), 2);
        assert_eq!(nearest(0, 1, 1), 0);
    }

    #[test]
    fn compositor_reports_its_identity() {
        assert_eq!(SoftwareCompositor::new().name(), "software");
    }
}
