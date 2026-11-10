//! Card 7.C1 — the backend fallback chain's acceptance suite (issue
//! #42): **simulated backend failures** against the real renderer.
//!
//! # What these tests prove (the card's AC, verbatim)
//!
//! *"GPU/backend fallback chain (auto-negotiated wgpu backend →
//! software present) in renderer-wgpu; capability report surfaced;
//! hotplug tests with simulated device events"*:
//!
//! - a backend that **fails mid-run** (the simulated adapter loss:
//!   a compositor double that starts failing) is degraded to
//!   software present through [`Renderer::fallback_to_software`];
//!   the surface, parameters and staging state survive the swap, and
//!   presents **resume** — no teardown, no reconstruction, the same
//!   renderer object keeps rendering (the no-restart contract the
//!   engine's hotplug hooks enforce on their side);
//! - the degradation answers a **typed capability report** naming the
//!   new backend, the cause, and the backend it replaced — the exact
//!   `(backend, reason)` pair `Engine::observe_renderer_backend`
//!   relays onto the wire as `video.backend-changed`;
//! - the failure itself is surfaced honestly (the present that hit
//!   the dead backend returns the compositor's typed error — never a
//!   panic, never a silent frame);
//! - a fallback while already on software is **idempotent and
//!   honest** (re-initialized, `fallback_from: "software"`).
//!
//! # The simulated backend (the mission's sanctioned test double)
//!
//! No hardware backend exists in this build (the wgpu import is
//! Lead-gated — `records/wgpu-30.0.1.md`), so "a hardware backend
//! that dies mid-run" is simulated the same way the engine's
//! device-loss tests simulate a vanishing sink: a [`FlakyBackend`]
//! compositor that fails every composite after its arming point with
//! a typed [`RendererError::SurfaceLost`]. The renderer under test
//! is the real one; only the *backend's* failure behavior is
//! scripted.

use renderer_wgpu::color::{Rgba, SurfaceFormat};
use renderer_wgpu::negotiate;
use renderer_wgpu::params::{Extent2D, Placement, RenderParams};
use renderer_wgpu::staging::StagedFrame;
use renderer_wgpu::surface::{HeadlessSurface, PresentMode, RenderSurface, SurfaceConfig};
use renderer_wgpu::{BackendTier, CHAIN, Compositor, Renderer, SoftwareCompositor, TierStatus};
use testkit::Must;
use video_engine::{PresentRequest, Presenter};

use media_types::frame::{ColorDescription, FrameStorage, VideoFrame};
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, Timestamp};
use media_types::{GenerationId, PixelFormat};

/// The suite's extent.
const EXTENT: Extent2D = Extent2D {
    width: 32,
    height: 24,
};

/// A solid-color RGBA frame at the suite's extent.
fn frame(fill: u8) -> VideoFrame {
    let mut pixels = Vec::with_capacity((EXTENT.width * EXTENT.height * 4) as usize);
    for _ in 0..EXTENT.width * EXTENT.height {
        pixels.extend_from_slice(&[fill, fill, fill, 255]);
    }
    VideoFrame {
        format: PixelFormat::Rgba8,
        size: (EXTENT.width, EXTENT.height),
        data: FrameStorage::Cpu(SharedBytes::from_vec(pixels)),
        pts: Stamp::Present(Timestamp::new(0, video_engine::NANO_BASE)),
        color: ColorDescription {
            transfer: media_types::frame::TransferCharacteristics::Srgb,
            ..ColorDescription::BT709_LIMITED
        },
        interlaced: false,
        generation: GenerationId::FIRST,
    }
}

/// A present request carrying `frame`.
fn request(frame: VideoFrame) -> PresentRequest {
    PresentRequest {
        when: frame.pts.timestamp().must(),
        generation: frame.generation,
        frame,
    }
}

/// The simulated dying backend: composites normally until armed, then
/// fails every composite with the typed surface-lost error (the
/// adapter-loss script).
struct FlakyBackend {
    /// How many composites succeed before the backend dies.
    healthy_presents: usize,
    name: &'static str,
}

impl FlakyBackend {
    fn healthy_for(presents: usize) -> FlakyBackend {
        FlakyBackend {
            healthy_presents: presents,
            name: "simulated-hardware",
        }
    }
}

impl Compositor for FlakyBackend {
    fn name(&self) -> &'static str {
        self.name
    }

    fn composite(
        &mut self,
        _frame: &StagedFrame,
        _placement: &Placement,
        _background: Rgba,
        _target: &mut dyn RenderSurface,
    ) -> Result<(), renderer_wgpu::RendererError> {
        if self.healthy_presents == 0 {
            // The adapter is gone: the typed, non-panic failure doc
            // 03 §4.11 d defines.
            return Err(renderer_wgpu::RendererError::SurfaceLost);
        }
        self.healthy_presents -= 1;
        // Delegate the healthy path to the REAL software compositor —
        // only the failure behavior is simulated.
        SoftwareCompositor::new().composite(_frame, _placement, _background, _target)
    }
}

/// A headless surface at the suite's extent (the swap must preserve
/// exactly this surface).
fn surface() -> HeadlessSurface {
    HeadlessSurface::new(
        EXTENT,
        SurfaceConfig {
            format: SurfaceFormat::Bgra8Srgb,
            present_mode: PresentMode::Fifo,
        },
    )
    .must()
}

/// A renderer over the dying backend: `healthy` presents before the
/// adapter loss.
fn renderer_over_flaky(healthy: usize) -> Renderer {
    Renderer::with_backend(
        Box::new(surface()),
        Box::new(FlakyBackend::healthy_for(healthy)),
        renderer_wgpu::params::RenderParams::new(),
    )
}

// ---------------------------------------------------------------------------
// The mid-run fallback (the flagship)
// ---------------------------------------------------------------------------

/// The backend dies mid-run: the present that hits the dead backend
/// fails with the typed error, the host degrades through
/// `fallback_to_software`, and presents RESUME on software — same
/// renderer, same surface, same parameters (no restart), with the
/// typed capability report naming the cause and the replaced backend.
#[test]
fn backend_failure_mid_run_falls_back_to_software_and_presents_resume() {
    let mut renderer = renderer_over_flaky(2);
    assert_eq!(renderer.backend_name(), "simulated-hardware");

    // Two healthy presents (content-checked through readback).
    renderer.present(&request(frame(64))).must();
    renderer.present(&request(frame(96))).must();
    let before_failure = renderer.readback_rgba().must();

    // --- The adapter dies: the third present fails, typed. ---
    let failure = renderer.present(&request(frame(128))).err().must();
    let message = failure.to_string();
    assert!(
        message.contains("surface"),
        "the failure names the surface loss: {message}"
    );

    // --- The host degrades: the fallback chain's mid-run step. ---
    let report = renderer.fallback_to_software("simulated adapter loss");
    assert_eq!(renderer.backend_name(), "software");
    assert_eq!(report.backend(), "software");
    assert_eq!(report.fallback_from(), Some("simulated-hardware"));
    assert!(
        report.reason().contains("fell back to software present"),
        "the reason states the degradation: {}",
        report.reason()
    );
    assert!(
        report.reason().contains("simulated adapter loss"),
        "the cause is display-safe and specific: {}",
        report.reason()
    );

    // --- Presents resume: the SAME renderer, SAME surface. ---
    renderer.present(&request(frame(160))).must();
    renderer.present(&request(frame(192))).must();
    let after = renderer.readback_rgba().must();
    assert_ne!(
        after, before_failure,
        "new frames landed on the surface after the fallback"
    );
    // Content truth: the last present's fill is what is on screen.
    assert_eq!(after[0], 192, "the final frame's fill is on screen");
    assert_eq!(after[1], 192);
    assert_eq!(after[2], 192);
    assert_eq!(after[3], 255);
}

/// The fallback keeps the render parameters: the swap touches only
/// the compositor.
#[test]
fn fallback_preserves_params_and_surface_state() {
    let mut renderer = renderer_over_flaky(1);
    let mut params = RenderParams::new();
    params.background = Rgba([9, 8, 7, 255]);
    renderer.set_params(params.clone());
    renderer.present(&request(frame(50))).must();
    let extent_before = renderer.extent().must();

    let _report = renderer.fallback_to_software("second failure (different cause)");

    assert_eq!(renderer.extent().must(), extent_before, "same surface");
    assert_eq!(renderer.params().background, params.background);
    // The background survived: a 1x1 frame letterboxed against it.
    let mut tiny = frame(200);
    tiny.size = (1, 1);
    let mut pixels = Vec::new();
    pixels.extend_from_slice(&[200, 200, 200, 255]);
    tiny.data = FrameStorage::Cpu(SharedBytes::from_vec(pixels));
    renderer.present(&request(tiny)).must();
    let out = renderer.readback_rgba().must();
    // Corner pixel = the params' background, not black.
    assert_eq!(&out[0..4], &[9, 8, 7, 255]);
}

/// Falling back while already on software is idempotent and honest:
/// the compositor is re-initialized, the report says so.
#[test]
fn fallback_while_already_on_software_is_idempotent_and_honest() {
    let mut renderer = Renderer::new_headless(
        EXTENT,
        SurfaceConfig {
            format: SurfaceFormat::Bgra8Srgb,
            present_mode: PresentMode::Fifo,
        },
    )
    .must();
    assert_eq!(renderer.backend_name(), "software");
    let report = renderer.fallback_to_software("software compositor reset (simulated)");
    assert_eq!(report.backend(), "software");
    assert_eq!(
        report.fallback_from(),
        Some("software"),
        "the report does not hide which backend ran"
    );
    // And it still presents.
    renderer.present(&request(frame(20))).must();
}

// ---------------------------------------------------------------------------
// The startup auto-negotiation (chain end-to-end over the renderer)
// ---------------------------------------------------------------------------

/// A renderer wired through the auto-negotiated chain presents
/// normally and reports `software` — the honest verdict of this build
/// (the hardware tier's typed absence rides in the report).
#[test]
fn negotiated_renderer_presents_and_reports_software() {
    let negotiated = negotiate::negotiate();
    let compositor = negotiated.compositor.must();
    assert_eq!(compositor.name(), "software");

    let mut renderer = Renderer::with_backend(
        Box::new(surface()),
        compositor,
        renderer_wgpu::params::RenderParams::new(),
    );
    renderer.present(&request(frame(80))).must();
    assert_eq!(renderer.backend_name(), "software");

    let report = negotiated.report;
    assert_eq!(report.backend(), "software");
    assert_eq!(report.fallback_from(), None);
    // The startup report is what the engine replays at the first
    // session open — it must be self-contained.
    let (backend, reason) = report.relay_pair();
    assert_eq!(backend, "software");
    assert!(
        reason.contains("hardware-wgpu unavailable") && reason.contains("software present active"),
        "the startup report carries the whole negotiation: {reason}"
    );
}

/// The chain's shape is the documented policy: hardware first,
/// software present as the terminal tier — and the hardware tier's
/// probe answers the typed, recorded absence (never a fake
/// availability).
#[test]
fn the_chain_is_hardware_first_software_terminal_with_typed_absence() {
    assert_eq!(CHAIN, [BackendTier::Hardware, BackendTier::Software]);
    match BackendTier::Hardware.probe() {
        TierStatus::Selected => unreachable!("no hardware backend is compiled in this build"),
        TierStatus::Unavailable { reason } => {
            assert!(
                reason.contains("Lead-gated") && reason.contains("records/wgpu-30.0.1.md"),
                "the absence is the recorded decision: {reason}"
            );
        }
    }
    assert!(BackendTier::Software.probe().is_selected());
}
