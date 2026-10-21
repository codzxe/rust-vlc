//! The Lumen video renderer (card 2.D1, doc 03 §4.11; doc 01 §2.6
//! registry row — layer L4, deps `media-types` + `video-engine`, the
//! video-output plugin kind and "the only GPU citizen").
//!
//! # What this crate is
//!
//! It turns [`PresentRequest`]s into pixels on a
//! [`RenderSurface`](surface::RenderSurface): RGBA8 upload through a
//! **measured** staging path ([`staging`]), pure aspect/crop placement
//! math ([`params`], doc 03 §4.11 e), sRGB-correct presentation
//! ([`color`]), vsync configuration ([`surface::PresentMode`]) — all
//! behind the [`Presenter`] trait video-engine's queue delivers
//! through (the 2.C4 seam).
//!
//! # The wgpu decision (recorded honestly — the deferral record
//! `docs/dependencies/records/wgpu-30.0.1.md`; worklog 2.D1)
//!
//! The crate is *named* renderer-wgpu because doc 01 names it so; **v0
//! does not import wgpu**, for four verified reasons:
//!
//! 1. **This sandbox has no GPU stack at all** (environment evidence,
//!    `records/wgpu-30.0.1.md` §1 — verified by a live probe): no
//!    `/dev/dri`; the Vulkan loader exists but every ICD directory is
//!    empty (no lavapipe/llvmpipe); `libEGL` is absent from the linker
//!    map entirely; no X11/Wayland. An empirical wgpu 30.0.1
//!    `request_adapter` probe returned `Err` naming every backend
//!    un-loadable — the import would ship code with zero executable
//!    evidence, against this program's honest-gate culture.
//! 2. **`deny.toml` bans `libloading`**, and wgpu 30.0.1's
//!    default-features Linux graph pulls `libloading ^0.8` — via `ash`
//!    (the **Vulkan** backend's loader dlopen) and `dlib` (GL), not
//!    just the GL backend (verified by a throwaway-project `cargo
//!    tree` + the crates.io API, 2026-10: `records/wgpu-30.0.1.md` §2).
//!    Importing wgpu with default features would fail the Lead's own
//!    cargo-deny gate; the feature-set/ban decision is Lead-owned
//!    (AGENTS §1).
//! 3. **The dependency envelope is not granted**: the doc 13 card and
//!    the doc 01 registry row list `media-types, video-engine` — no
//!    external crates. Pinning wgpu (80-package default graph, E2
//!    graphics stack — `records/wgpu-30.0.1.md` §2) is the preaudit's
//!    "conditional, full R1–R10 record at pin time" path (doc 11 §2;
//!    allowlist §5).
//! 4. **Gate economics**: `full_gate.sh` runs `--all-features`, so a
//!    feature-gated wgpu would compile the whole graphics stack on
//!    every future gate for code that cannot run in this sandbox
//!    (DISK-INCIDENT-1 sensitivity).
//!
//! Instead, the architecture is complete behind the
//! [`backend::Compositor`] seam: the **software compositor**
//! (a minimal CPU rasterizer — the planned 7.C1 "software present"
//! fallback, arrived early) is linked and carries the acceptance proof;
//! every wgpu integration point is typed and shape-pinned
//! ([`surface::PresentMode::wgpu_present_mode`],
//! [`color::SurfaceFormat::wgpu_texture_format`], the
//! [`Renderer::with_backend`] injection point — the full decision
//! record with evidence is `docs/dependencies/records/wgpu-30.0.1.md`).
//! The hardware backend joins when the Lead ratifies the import (2.D3
//! prototype / ADR-003 evidence path, Phase 7 platform adapters).
//!
//! # What this crate is not (yet)
//!
//! - **GPU color conversion** (YUV → RGB in shaders, doc 03 §4.11 e /
//!   MP §11): v0 uploads RGBA8/Bgra8 and refuses the YUV family with a
//!   typed error naming this path. The MJPEG decoder's I420 output
//!   converts when that card lands.
//! - **Windowed surface creation** (raw window handles, swapchains):
//!   the L5 platform adapters / ADR-003 own it; v0 types the contract
//!   and ships the headless surface.
//! - **The render thread + latest-wins mailbox** (doc 03 §4.11 c
//!   `[PROPOSAL: mailbox depth 1]`): Phase-3 session wiring; v0 is the
//!   synchronous core (the 2.C4 scheduler precedent).
//! - **Subtitle compositing** (doc 03 §4.11's overlay proposal): the
//!   layer vocabulary exists on [`params::RenderParams`]; blending
//!   joins with the subtitles crate (Phase 5).
//!
//! # Example
//!
//! The acceptance path in four calls — synthetic frame, headless
//! surface, present, read back:
//!
//! ```
//! use renderer_wgpu::color::SurfaceFormat;
//! use renderer_wgpu::params::Extent2D;
//! use renderer_wgpu::surface::{PresentMode, SurfaceConfig};
//! use renderer_wgpu::synthetic::synthetic_frame;
//! use renderer_wgpu::{Renderer, screenshot};
//! use video_engine::{Presenter, PresentRequest};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut renderer = Renderer::new_headless(
//!     Extent2D { width: 64, height: 48 },
//!     SurfaceConfig { format: SurfaceFormat::Bgra8Srgb, present_mode: PresentMode::Fifo },
//! )?;
//! assert_eq!(renderer.backend_name(), "software");
//!
//! let frame = synthetic_frame(Extent2D { width: 64, height: 48 });
//! let request = PresentRequest {
//!     when: frame.pts.timestamp()?,
//!     generation: frame.generation,
//!     frame,
//! };
//! let info = renderer.present(&request)?;
//!
//! // The readback is byte-exact RGBA8; the PNG declares sRGB.
//! let rgba = renderer.readback_rgba()?;
//! let png = screenshot::encode_rgba(Extent2D { width: 64, height: 48 }, &rgba)?;
//! assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
//! assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
//! # let _ = info;
//! # Ok(())
//! # }
//! ```
//!
//! # Never
//!
//! Demux/decode types beyond media-types frames, UI, layout, media
//! decisions (ARCH-4); `unsafe`; external dependencies (std-only by
//! C5 until the Lead-gated wgpu import changes that).

pub mod backend;
pub mod color;
pub mod error;
pub mod params;
pub mod renderer;
pub mod screenshot;
pub mod staging;
pub mod surface;
pub mod synthetic;

pub use backend::{Compositor, SoftwareCompositor};
pub use color::{Rgba, SurfaceFormat};
pub use error::{RendererError, RendererResult, RendererStage};
pub use params::{AspectMode, Extent2D, Placement, Rect, RenderParams};
pub use renderer::{Renderer, uploads_format};
pub use staging::{StagedFrame, StagingStats, StagingUploader};
pub use surface::{HeadlessSurface, PresentMode, RenderSurface, SurfaceConfig};
pub use video_engine::{PresentInfo, PresentRequest, Presenter};
