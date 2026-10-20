//! The presentation boundary: [`PresentRequest`], [`PresentInfo`] and
//! the [`Presenter`] trait (card 2.C4; doc 03 §4.10 b, doc 01 §2.2
//! "Defines the `Presenter` trait that renderers implement").
//!
//! This is the seam where video-engine ends and rendering begins: the
//! concrete wgpu renderer is card 2.D1 (L4 `renderer-wgpu`), which
//! implements [`Presenter`] — this crate defines the contract only, and
//! never touches a GPU, window or surface API (doc 01 §2.2 d "Never:
//! GPU/rendering APIs").
//!
//! # Adaptations of the doc sketches (recorded for review)
//!
//! - doc 03 §4.10 b `PresentRequest { frame: VideoFrameHandle, when,
//!   gen }` — the frame is media-types' [`VideoFrame`]: its
//!   [`FrameStorage::OpaqueHandle`] variant *is* the handle vocabulary
//!   for hardware frames, so one type covers both CPU and GPU paths.
//! - doc 01 §2.2 `Presenter::present(&mut self, frame: &VideoFrame)` —
//!   takes `&PresentRequest` instead: presentation needs the scheduled
//!   slot and the generation (doc 03 §5 checking point 7), not just the
//!   pixels.

use media_types::frame::VideoFrame;
use media_types::time::Timestamp;
use media_types::{EngineError, GenerationId};

/// One scheduled presentation, handed to a [`Presenter`] (doc 03
/// §4.10 b).
///
/// Emitted by [`crate::PresentationScheduler::tick`] when a frame is
/// due (or repeated, within the repeat budget); delivered in order by
/// [`crate::PresentationQueue`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentRequest {
    /// The decoded frame to put on screen. Cloning is cheap for CPU
    /// frames (the bytes are shared) and exact for hardware handles.
    pub frame: VideoFrame,
    /// The frame's scheduled slot: its own presentation timestamp,
    /// normalized to the clock's nanosecond base ([`media_clock::
    /// NANO_BASE`]). The queue delivers the request at the tick the
    /// scheduler judged due; `when` records *which media slot* the
    /// frame fills — the A/V-sync observable (compare against the
    /// clock at delivery).
    pub when: Timestamp,
    /// The session generation the schedule ran under. Requests whose
    /// generation differs from the live one are stale (doc 03 §5
    /// checking point 7) and are dropped wholesale, never presented.
    /// (The doc sketch names this field `gen`; Rust 2024 reserves that
    /// identifier, so it carries the media-types spelling.)
    pub generation: GenerationId,
}

impl PresentRequest {
    /// The request's presentation timestamp as a [`Stamp`] — the frame's
    /// own stamp re-expressed on the clock's nanosecond base (missing
    /// never happens here: the scheduler infers or holds missing-PTS
    /// frames before emitting).
    #[must_use = "the slot is what A/V-sync accounting compares"]
    pub fn pts(&self) -> media_types::time::Stamp {
        media_types::time::Stamp::Present(self.when)
    }
}

/// What a presenter reports back for one presentation (doc 01 §2.2
/// `Presenter::present -> Result<PresentInfo, EngineError>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentInfo {
    /// The clock position at which the presenter actually put the frame
    /// on screen `[PROPOSAL: the A/V-sync observability primitive (MP
    /// §11) — 2.D1's renderer fills it with its vsync-quantized
    /// position; test presenters echo the request's slot]`.
    pub presented_at: Timestamp,
}

/// The render boundary (doc 01 §2.2): turns [`PresentRequest`]s into
/// pixels on a surface — implemented by the 2.D1 wgpu renderer, the
/// Phase-2 webview prototype's canvas bridge, and test recorders.
///
/// # Contract for implementors
///
/// - **Non-fatal by preference** (doc 03 §4.10 d): a lost surface or a
///   busy device is a session-level degradation, not a crash; report it
///   as an [`EngineError`] and let the caller decide (the default is
///   audio continues, video degrades — audio is never dropped for
///   video's sake).
/// - **Order is the caller's guarantee**: requests arrive in
///   non-decreasing PTS order, one screen-refresh's worth at a time; a
///   presenter that reorders them breaks the scheduler's invariant
///   (doc 03 §4.10 f "no frame is presented out of order").
/// - **Repeat requests** ([`PresentAction::RepeatLast`][repeat])
///   re-present the previous slot: surface-persisting renderers may
///   treat them as no-ops; compositors that need a redraw (canvas
///   bridges) must honor them.
///
/// [repeat]: crate::PresentAction::RepeatLast
pub trait Presenter: Send {
    /// Put one scheduled frame on screen.
    ///
    /// # Errors
    /// An [`EngineError`] (stage `Present` for renderer-side faults)
    /// when the presentation could not be performed; the caller keeps
    /// the request (retry or degrade is its policy — never a panic).
    fn present(&mut self, request: &PresentRequest) -> Result<PresentInfo, EngineError>;
}
