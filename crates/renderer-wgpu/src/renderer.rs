//! The renderer itself: one frame → one surface present, behind the
//! [`Presenter`] contract video-engine delivers through (doc 01 §2.6
//! "`Renderer` … implements `Presenter`"; card 2.D1).
//!
//! # Threading (doc 03 §4.11 c)
//!
//! Production wiring gives the renderer its own thread: one owner for
//! the surface, the backend and every GPU resource, fed by a
//! latest-wins mailbox (a new request replaces an unsent one —
//! rendering never back-pressures decode `[PROPOSAL: mailbox depth
//! 1]`). v0 is the synchronous core, exactly like 2.C4's scheduler
//! precedent: the playback engine (Phase 3) hosts the thread and the
//! mailbox; every code path presented here is the one that thread
//! runs.
//!
//! # Presentation timestamp honesty
//!
//! [`PresentInfo::presented_at`] is the request's scheduled slot: the
//! headless/Fifo present is synchronous, so the frame is on the
//! surface *at* the slot the scheduler chose — the vsync-quantized
//! reality of a hardware swapchain (wgpu `SurfaceTexture::present`
//! timing) is the hardware backend's to fill `[VERIFY: where a
//! compositor clock exists]`.

use media_types::frame::{FrameStorage, VideoFrame};
use media_types::{EngineError, PixelFormat};
use video_engine::{PresentInfo, PresentRequest, Presenter};

use crate::backend::{Compositor, SoftwareCompositor};
use crate::error::{RendererError, RendererResult};
use crate::params::{Extent2D, RenderParams, compute_placement};
use crate::staging::StagingUploader;
use crate::surface::{HeadlessSurface, RenderSurface, SurfaceConfig};

/// The video presenter: staged uploads, aspect placement, compositing,
/// surface presents (doc 03 §4.11 a).
///
/// Construct headless for tests and the software fallback
/// ([`Renderer::new_headless`]), or inject a surface + compositor pair
/// for the hardware path ([`Renderer::with_backend`]) — the wgpu
/// surface and compositor join behind the exact same constructor when
/// the Lead-gated import lands (crate docs).
pub struct Renderer {
    surface: Option<Box<dyn RenderSurface>>,
    compositor: Box<dyn Compositor>,
    staging: StagingUploader,
    params: RenderParams,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer")
            .field(
                "surface",
                &self.surface.as_ref().map(|s| (s.extent(), s.format())),
            )
            .field("backend", &self.compositor.name())
            .field("params", &self.params)
            .finish_non_exhaustive()
    }
}

impl Renderer {
    /// The general constructor: any surface, any compositor (the
    /// hardware path's injection point).
    #[must_use = "pure constructor"]
    pub fn with_backend(
        surface: Box<dyn RenderSurface>,
        compositor: Box<dyn Compositor>,
        params: RenderParams,
    ) -> Renderer {
        Renderer {
            surface: Some(surface),
            compositor,
            staging: StagingUploader::new(),
            params,
        }
    }

    /// The headless constructor: an offscreen surface of `size` under
    /// `config`, driven by the software compositor — runs on any CPU
    /// (the deterministic test/AC target and the 7.C1 fallback).
    ///
    /// # Errors
    /// Propagates [`HeadlessSurface::new`]'s typed errors (zero
    /// extent; non-Fifo present mode on the headless surface).
    pub fn new_headless(size: Extent2D, config: SurfaceConfig) -> RendererResult<Renderer> {
        let surface = HeadlessSurface::new(size, config)?;
        Ok(Renderer::with_backend(
            Box::new(surface),
            Box::new(SoftwareCompositor::new()),
            RenderParams::new(),
        ))
    }

    /// Detach the current surface (surface-loss handling: the session
    /// degrades while detached; audio continues — doc 03 §4.11 d).
    #[must_use = "the caller owns reattachment policy"]
    pub fn detach_surface(&mut self) -> Option<Box<dyn RenderSurface>> {
        self.surface.take()
    }

    /// (Re)attach a surface after a loss or a window change.
    pub fn attach_surface(&mut self, surface: Box<dyn RenderSurface>) {
        self.surface = Some(surface);
    }

    /// Whether a surface is currently attached.
    #[must_use = "pure accessor"]
    pub fn has_surface(&self) -> bool {
        self.surface.is_some()
    }

    /// The render parameters (aspect, crop, background, layers).
    #[must_use = "pure accessor"]
    pub fn params(&self) -> &RenderParams {
        &self.params
    }

    /// Replace the render parameters (cheap: the next present picks
    /// them up — no pipeline rebuild, the placement is per-frame).
    pub fn set_params(&mut self, params: RenderParams) {
        self.params = params;
    }

    /// Resize the attached surface (non-fatal by construction, doc 03
    /// §4.11 d).
    ///
    /// # Errors
    /// [`RendererError::NoSurface`] when detached;
    /// [`RendererError::ZeroExtent`] for a zero component.
    pub fn resize(&mut self, size: Extent2D) -> RendererResult<()> {
        let surface = self.surface.as_mut().ok_or(RendererError::NoSurface)?;
        surface.resize(size)
    }

    /// The attached surface's extent.
    ///
    /// # Errors
    /// [`RendererError::NoSurface`] when detached.
    pub fn extent(&self) -> RendererResult<Extent2D> {
        let surface = self.surface.as_ref().ok_or(RendererError::NoSurface)?;
        Ok(surface.extent())
    }

    /// The linked backend's identity (truthful capability reporting —
    /// "software" today, "wgpu:<adapter>" on the hardware path).
    #[must_use = "pure accessor"]
    pub fn backend_name(&self) -> &'static str {
        self.compositor.name()
    }

    /// Swap the linked compositor — the backend handover seam (card
    /// 7.C1): the surface, parameters and staging state survive the
    /// swap untouched; the next present runs on the new backend.
    /// [`crate::negotiate::Renderer::fallback_to_software`] is the
    /// fallback chain's policy wrapper over this (it also answers
    /// the typed capability report).
    pub fn swap_compositor(&mut self, compositor: Box<dyn Compositor>) {
        self.compositor = compositor;
    }

    /// The staging measurements so far (card 2.D1 "measured staging").
    #[must_use = "observable evidence"]
    pub fn staging_stats(&self) -> crate::staging::StagingStats {
        self.staging.stats()
    }

    /// Read the attached surface back as RGBA8 (the screenshot/AC
    /// path; doc 03 §4.11 f).
    ///
    /// # Errors
    /// [`RendererError::NoSurface`] when detached; the surface's own
    /// typed errors otherwise.
    pub fn readback_rgba(&self) -> RendererResult<Vec<u8>> {
        let surface = self.surface.as_ref().ok_or(RendererError::NoSurface)?;
        surface.readback_rgba()
    }

    /// The core path every present runs: validate → stage (measured) →
    /// place (pure math) → composite → present.
    ///
    /// # Errors
    /// The [`RendererError`] family, stage-tagged: YUV formats are
    /// refused with the shader path named; opaque hardware handles are
    /// refused pending the wgpu backend; size mismatches are decode
    /// corruption, never guessed.
    pub fn render(&mut self, frame: &VideoFrame) -> RendererResult<()> {
        let surface = self.surface.as_mut().ok_or(RendererError::NoSurface)?;

        // 1. Storage: the v0 path uploads CPU bytes; hardware handles
        //    wait for the wgpu backend's import (doc 01 §2.6).
        let bytes = match &frame.data {
            FrameStorage::Cpu(bytes) => bytes,
            FrameStorage::OpaqueHandle(_) => {
                return Err(RendererError::OpaqueHandleUnsupported);
            }
        };
        let extent = Extent2D {
            width: frame.size.0,
            height: frame.size.1,
        };

        // 2. Staging (measured copy; Bgra8 swizzles in the same pass).
        let staged = self.staging.stage(bytes, extent, frame.format)?;

        // 3. Placement (pure, unit-tested math).
        let placement = compute_placement(staged.extent, surface.extent(), &self.params)?;

        // 4. Composite + present through the backend seam.
        let background = self.params.background;
        self.compositor
            .composite(&staged, &placement, background, surface.as_mut())?;
        surface.present()
    }
}

impl Presenter for Renderer {
    fn present(&mut self, request: &PresentRequest) -> Result<PresentInfo, EngineError> {
        self.render(&request.frame)
            .map_err(RendererError::to_engine_error)?;
        // The synchronous Fifo present lands the frame on the surface
        // at the scheduled slot — see the module docs' timestamp
        // honesty note.
        Ok(PresentInfo {
            presented_at: request.when,
        })
    }
}

/// Format routing at the [`Presenter`] boundary: which pixel formats
/// the v0 renderer accepts, as data (used by tests and callers that
/// probe before wiring a decoder).
///
/// The YUV family converts on the GPU (doc 03 §4.11 e) — a later card;
/// until then the renderer refuses those frames with a typed error
/// naming this policy.
#[must_use = "pure classification"]
pub fn uploads_format(format: PixelFormat) -> bool {
    matches!(format, PixelFormat::Rgba8 | PixelFormat::Bgra8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{Rgba, SurfaceFormat};
    use crate::surface::PresentMode;
    use media_types::GenerationId;
    use media_types::frame::ColorDescription;
    use media_types::packet::SharedBytes;
    use media_types::time::{Stamp, Timestamp};
    use testkit::{Must, assert_err};

    fn request(w: u32, h: u32, fill: u8) -> PresentRequest {
        let mut pixels = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..w * h {
            pixels.extend_from_slice(&[fill, fill, fill, 255]);
        }
        PresentRequest {
            frame: VideoFrame {
                format: PixelFormat::Rgba8,
                size: (w, h),
                data: FrameStorage::Cpu(SharedBytes::from_vec(pixels)),
                pts: Stamp::Present(Timestamp::new(0, video_engine::NANO_BASE)),
                color: ColorDescription {
                    transfer: media_types::frame::TransferCharacteristics::Srgb,
                    ..ColorDescription::BT709_LIMITED
                },
                interlaced: false,
                generation: GenerationId::FIRST,
            },
            when: Timestamp::new(1_000_000_000, video_engine::NANO_BASE),
            generation: GenerationId::FIRST,
        }
    }

    fn headless_renderer(w: u32, h: u32) -> Renderer {
        Renderer::new_headless(
            Extent2D {
                width: w,
                height: h,
            },
            SurfaceConfig {
                format: SurfaceFormat::Bgra8Srgb,
                present_mode: PresentMode::Fifo,
            },
        )
        .must()
    }

    #[test]
    fn presenter_round_trip_byte_exact_and_reports_the_slot() {
        let mut renderer = headless_renderer(4, 4);
        assert_eq!(renderer.backend_name(), "software");
        let req = request(4, 4, 0x5A);
        let frame_bytes = match &req.frame.data {
            FrameStorage::Cpu(b) => b.as_slice().to_vec(),
            FrameStorage::OpaqueHandle(_) => Vec::new(),
        };
        let info = renderer.present(&req).must();
        // The slot is echoed (synchronous Fifo — module docs).
        assert_eq!(info.presented_at, req.when);
        // Byte-exact passthrough at 1:1 (the sRGB invariant).
        assert_eq!(renderer.readback_rgba().must(), frame_bytes);
        // Measured staging: exact size, one copy.
        let stats = renderer.staging_stats();
        assert_eq!(stats.bytes_staged, 4 * 4 * 4);
        assert_eq!(stats.staging_copies, 1);
    }

    #[test]
    fn format_routing_matches_the_upload_path() {
        assert!(uploads_format(PixelFormat::Rgba8));
        assert!(uploads_format(PixelFormat::Bgra8));
        assert!(!uploads_format(PixelFormat::Nv12));
        assert!(!uploads_format(PixelFormat::I420));
        assert!(!uploads_format(PixelFormat::Yuyv422));
        // And the renderer refuses what routing refuses.
        let mut renderer = headless_renderer(2, 2);
        let mut yuv = request(2, 2, 0);
        yuv.frame.format = PixelFormat::Nv12;
        yuv.frame.data = FrameStorage::Cpu(SharedBytes::from_vec(vec![0; 6]));
        let err = assert_err(renderer.present(&yuv), "Nv12 upload is refused");
        assert_eq!(err.context.code, "render.upload.format-unsupported");
    }

    #[test]
    fn opaque_handles_are_refused_with_the_named_path() {
        let mut renderer = headless_renderer(2, 2);
        let mut hw = request(2, 2, 0);
        hw.frame.data = FrameStorage::OpaqueHandle(7);
        let err = assert_err(renderer.present(&hw), "hardware handles need wgpu");
        assert_eq!(err.context.code, "render.upload.opaque-handle");
    }

    #[test]
    fn truncated_frames_are_decode_corruption_not_guesses() {
        let mut renderer = headless_renderer(4, 4);
        let mut short = request(4, 4, 0);
        short.frame.data = FrameStorage::Cpu(SharedBytes::from_vec(vec![0; 8]));
        let err = assert_err(renderer.present(&short), "truncated frame");
        assert_eq!(err.context.code, "render.upload.size-mismatch");
        assert!(err.to_string().contains("truncated"), "{}", err);
    }

    #[test]
    fn detached_renderer_degrades_not_crashes() {
        let mut renderer = headless_renderer(2, 2);
        let detached = renderer.detach_surface();
        assert!(detached.is_some());
        assert!(!renderer.has_surface());
        let err = assert_err(renderer.present(&request(2, 2, 0)), "no surface");
        assert_eq!(err.context.code, "render.surface.missing");
        // Reattach: back in business without losing params or stats.
        renderer.attach_surface(detached.must());
        renderer.present(&request(2, 2, 1)).must();
    }

    #[test]
    fn resize_flows_to_the_surface() {
        let mut renderer = headless_renderer(2, 2);
        renderer
            .resize(Extent2D {
                width: 6,
                height: 4,
            })
            .must();
        assert_eq!(
            renderer.extent().must(),
            Extent2D {
                width: 6,
                height: 4
            }
        );
        // A 2x2 frame letterboxes in the 6x4 target without error.
        renderer.present(&request(2, 2, 9)).must();
        let err = assert_err(
            renderer.resize(Extent2D {
                width: 0,
                height: 4,
            }),
            "zero resize",
        );
        assert!(matches!(err, RendererError::ZeroExtent { .. }));
    }

    #[test]
    fn params_swap_takes_effect_next_present() {
        let mut renderer = headless_renderer(4, 4);
        let mut params = RenderParams::new();
        params.background = Rgba::WHITE;
        renderer.set_params(params);
        // A 2x1 frame (aspect 2:1) letterboxes in the 4x4 target:
        // rows 0 and 3 are background, rows 1-2 carry the frame full
        // width.
        renderer.present(&request(2, 1, 0)).must();
        let out = renderer.readback_rgba().must();
        let px = |i: usize| &out[i * 4..i * 4 + 4];
        assert_eq!(px(0), &[255, 255, 255, 255], "row 0: background");
        assert_eq!(px(4), &[0, 0, 0, 255], "row 1: frame pixel (full width)");
        assert_eq!(px(15), &[255, 255, 255, 255], "row 3: background");
    }

    #[test]
    fn headless_constructor_validates_like_the_surface() {
        let err = assert_err(
            Renderer::new_headless(
                Extent2D {
                    width: 0,
                    height: 4,
                },
                SurfaceConfig::DEFAULT,
            ),
            "zero extent",
        );
        assert!(matches!(err, RendererError::ZeroExtent { .. }));
        let err = assert_err(
            Renderer::new_headless(
                Extent2D {
                    width: 4,
                    height: 4,
                },
                SurfaceConfig {
                    format: SurfaceFormat::Bgra8Srgb,
                    present_mode: PresentMode::Mailbox,
                },
            ),
            "mailbox on headless",
        );
        assert_eq!(
            err,
            RendererError::PresentModeUnsupported {
                requested: PresentMode::Mailbox,
                supported: &[PresentMode::Fifo],
            }
        );
    }
}
