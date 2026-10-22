//! Zero-allocation proof for the renderer's present path (W2-06,
//! materializing the evidence the W1-20 re-audit found missing:
//! staging.rs claimed a "counting-allocator proof binary" that did
//! not exist — this is that binary, and now the claim is real).
//!
//! After warm-up, N × (`Presenter::present` → stage → place →
//! composite → present) allocates **zero bytes** — measured by the
//! counting global allocator. The reuse that makes it true:
//!
//! - `StagingUploader`'s bounded recycle pool hands the same frame
//!   buffer back for every stage (C5a — `RecyclePool`, 3 slots);
//! - `SoftwareCompositor`'s destination block is cleared and refilled
//!   in place (C5b — grown only on a geometry change);
//! - `HeadlessSurface`'s framebuffer is allocated once at
//!   construction; `present` is a counter bump;
//! - placement math is pure integer arithmetic.
//!
//! The request is built once and passed by reference, so the measured
//! window sees only the present path itself.
//!
//! One `#[test]` per binary by design (the allocator is
//! process-global; see `testkit::alloc_count` docs — the same shape
//! as `zero_alloc_quantum` / `zero_alloc_null_sink`).

#[global_allocator]
static ALLOC: testkit::alloc_count::CountingGlobalAlloc =
    testkit::alloc_count::CountingGlobalAlloc::new();

use media_types::GenerationId;
use media_types::frame::{ColorDescription, FrameStorage, VideoFrame};
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, Timestamp};
use renderer_wgpu::Renderer;
use renderer_wgpu::color::SurfaceFormat;
use renderer_wgpu::params::Extent2D;
use renderer_wgpu::surface::{PresentMode, SurfaceConfig};
use testkit::Must;
use testkit::alloc_count::serialize_tests;
use video_engine::{NANO_BASE, PresentRequest, Presenter};

/// A modest but non-trivial target (letterboxed source exercises the
/// background pass too).
const EXTENT: Extent2D = Extent2D {
    width: 64,
    height: 64,
};
/// Warm-up presents: enough to size the pool, the compositor block
/// and every capacity the steady state reuses.
const WARM_UP: usize = 8;
/// Measured steady-state presents.
const CYCLES: usize = 128;

fn request() -> PresentRequest {
    // A 32x16 RGBA frame (2:1 aspect) fitted into the 64x64 square
    // target: Fit scales by min(64/32, 64/16) = 2 → a 64x32 centered
    // destination with 16-px letterbox bars top and bottom — every
    // present runs the background fill, the resample and the surface
    // write: the full composite, not a degenerate 1:1 only.
    let (w, h) = (32u32, 16u32);
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            pixels.extend_from_slice(&[(x * 8) as u8, (y * 16) as u8, 0x5A, 255]);
        }
    }
    PresentRequest {
        frame: VideoFrame {
            format: media_types::PixelFormat::Rgba8,
            size: (w, h),
            data: FrameStorage::Cpu(SharedBytes::from_vec(pixels)),
            pts: Stamp::Present(Timestamp::new(0, NANO_BASE)),
            color: ColorDescription {
                transfer: media_types::frame::TransferCharacteristics::Srgb,
                ..ColorDescription::BT709_LIMITED
            },
            interlaced: false,
            generation: GenerationId::FIRST,
        },
        when: Timestamp::new(1_000_000_000, NANO_BASE),
        generation: GenerationId::FIRST,
    }
}

#[test]
fn present_path_is_zero_allocation_after_warm_up() {
    let _serial = serialize_tests();

    // Setup: renderer + request construction, outside the window.
    let mut renderer = Renderer::new_headless(
        EXTENT,
        SurfaceConfig {
            format: SurfaceFormat::Bgra8Srgb,
            present_mode: PresentMode::Fifo,
        },
    )
    .must();
    assert_eq!(renderer.backend_name(), "software");
    let request = request();

    // Warm-up: sizes the recycle pool, the compositor's destination
    // block and the stats path.
    for _ in 0..WARM_UP {
        renderer.present(&request).must();
    }
    // The path really composited (not a vacuous loop): the 2:1 frame
    // Fits the square target as a 64x32 band at rows 16..48 — surface
    // (0,16) is frame (0,0) → (0,0,0x5A); the band's center (32,32)
    // is frame (16,8) → (128,128,0x5A); rows 0 and 63 are the default
    // black background bars. Readback allocates — before the measured
    // window, never inside it.
    let out = renderer.readback_rgba().must();
    let px = |x: usize, y: usize| {
        let at = (y * EXTENT.width as usize + x) * 4;
        [out[at], out[at + 1], out[at + 2], out[at + 3]]
    };
    assert_eq!(px(0, 16), [0, 0, 0x5A, 255], "frame top-left (band start)");
    assert_eq!(px(32, 32), [128, 128, 0x5A, 255], "frame center");
    assert_eq!(px(0, 0), [0, 0, 0, 255], "letterbox bar (background)");
    assert_eq!(px(63, 63), [0, 0, 0, 255], "letterbox bar (background)");

    // The measured window: CYCLES full presents through the
    // Presenter trait (stage → place → composite → present).
    let guard = ALLOC.state().guard();
    for _ in 0..CYCLES {
        renderer.present(&request).must();
    }
    guard.assert_zero("renderer present path: stage + place + composite + present");
}
