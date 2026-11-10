//! `apps/web/src/render` — the web render pillar (card 7.H2, issue
//! #48; doc 09 §4).
//!
//! # What this module tree is
//!
//! The typed render-surface seam for the browser lane, in three
//! Rust-side halves plus the JS glue that completes them:
//!
//! - [`backend`] — the **backend negotiation state machine** (doc 09
//!   §4.4): the page probes `navigator.gpu` / `getContext("webgl2")`
//!   and reports the raw facts; this side walks the ladder — WebGPU
//!   preferred, WebGL2 the universal floor, `unavailable` the honest
//!   answer — and the ACTIVE backend is what the capability snapshot
//!   reports (never a guess, never an aspiration).
//! - [`canvas`] — the **canvas seam** (doc 09 §4.3): DPR-aware pixel
//!   extent (the same pure arithmetic as native), non-fatal resize,
//!   and the typed surface-lost vocabulary.
//! - [`present`] — the **frame-upload path** (doc 09 §4.2, row 1):
//!   decoded CPU frames → RGBA (the single allowed CPU→GPU-adjacent
//!   copy; I420 conversion uses the frame's own colorimetry) → the
//!   latest-wins present mailbox → one `PresentFrame` message per
//!   pump slice → the JS glue's canvas draw.
//!
//! # What this is NOT (the honest boundary)
//!
//! No wgpu here: `renderer-wgpu`'s WebGPU/WebGL2 backends are the
//! Lead-gated wgpu import, and this crate is zero-dep by card — so
//! the SURFACE is typed at the contract level (the same
//! resize/present/latest-wins discipline
//! `renderer_wgpu::surface::RenderSurface` pins), and the v1 upload
//! lane is the CPU-staged RGBA path over the MessagePort. When wgpu
//! is admitted to the web app, its surface slots in behind this same
//! seam ([PROPOSAL: Lead decision, doc 09 §4.1's [VERIFY] note on
//! wgpu's web backend]).
//!
//! # Evidence labels
//!
//! The negotiation ladder, the DPR arithmetic, the mailbox's
//! latest-wins contract and the I420/RGBA conversions are proven
//! headless-native by this crate's tests. Everything on the browser
//! side — the actual `navigator.gpu` probe, the WebGL2 texture
//! upload, `requestVideoFrameCallback` pacing, device loss — is
//! `[VERIFY]` (no browser in this sandbox; card 7.I1's matrix owns
//! those rows).

pub mod backend;
pub mod canvas;
pub mod present;

pub use backend::{RenderBackend, RendererCaps, RendererNegotiation};
pub use canvas::{CanvasSurface, pixel_extent};
pub use present::{PresentMailbox, UploadFrame, rgba_from_video};
