//! The Lumen video presentation scheduler (card 2.C4, doc 03 §4.10;
//! doc 01 §2.2 registry row — layer L3, "video filters and
//! presentation scheduling").
//!
//! # What this crate is
//!
//! The stage after decode on the video path: it decides *which frame
//! is on screen when* (doc 03 §4.10 a). The [`PresentationScheduler`]
//! consumes decoded [`VideoFrame`]s, compares each frame's PTS against
//! the injected [`PlaybackClock`] snapshot (doc 01 §1.3 rule 9:
//! present now, wait, or skip because late — skips are observable
//! events, never silent), applies the [`DropPolicy`], and emits
//! [`PresentRequest`]s; the bounded [`PresentationQueue`] delivers them
//! in order to a [`Presenter`] — the trait the 2.D1 wgpu renderer
//! implements. This crate contains **no GPU, window or surface code**
//! (doc 01 §2.2 d).
//!
//! # What this crate is not (yet)
//!
//! - The **video filter chain** (`VideoFilter` — color adjust, crop,
//!   deinterlace per doc 01 §2.2): a later card; the pipeline seam is
//!   reserved. Inter-frame reorder (B-frames) is likewise not here —
//!   it is the decoder's job; the scheduler handles display order only
//!   (doc 03 §4.10 e), keeping a small reorder window for arrival
//!   jitter.
//! - The **scheduler thread** (doc 03 §4.10 c): production wiring —
//!   the playback engine hosts this state machine and paces it
//!   (vsync callback or timer). v0 is the deterministic, explicitly
//!   stepped core, so every acceptance test runs on injected virtual
//!   time with zero sleeps (doc 03 §8).
//!
//! # Time and clock vocabulary
//!
//! Media time is [`media-types`]' everywhere (doc 01 §3): positions
//! are [`Timestamp`]s on the clock's canonical nanosecond base
//! ([`NANO_BASE`]), durations are [`Nanoseconds`], and PTS
//! normalization from stream bases is checked — an unconvertible stamp
//! is handed back with its [`TimeError`], never guessed (MP §8).
//! The clock is consumed through the [`PlaybackClock`] trait with one
//! consistent [`ClockSnapshot`] per tick (position, rate, state,
//! generation in a single lock-ordered read — everything a scheduler
//! needs).
//!
//! # Cancellation (doc 03 §5)
//!
//! Frames carry the demux [`GenerationId`]. The clock snapshot's
//! generation is the live one: on a change, the scheduler drops every
//! pending frame whose stamp differs (counted — checking point 7),
//! refuses older-generation pushes at the boundary, and never repeats
//! the dead generation's last frame. The presentation queue mirrors
//! the discipline: [`PresentationQueue::publish`] replaces the mailbox
//! wholesale.
//!
//! # Module map
//!
//! - [`policy`]    — [`DropPolicy`], [`SchedulerCfg`] and the pure
//!   per-frame classifier [`FrameDecision`] (grid-testable, doc 03
//!   §4.10 f).
//! - [`scheduler`] — the [`PresentationScheduler`] window state
//!   machine: present-now / wait / drop-late, supersede, repeat and
//!   background, generation purge, missing-PTS inference.
//! - [`queue`]     — the bounded, in-order [`PresentationQueue`] with
//!   typed bound and stale signals.
//! - [`sink`]      — the presentation boundary: [`PresentRequest`],
//!   [`PresentInfo`], the [`Presenter`] trait.
//!
//! # Deviations from the doc sketches (recorded for Lead review)
//!
//! - doc 03 §4.10 b `DropPolicy::Auto` — accepted at the type level;
//!   v0 resolves it to [`DropPolicy::Late`] with the doc's default
//!   threshold (one nominal frame duration, requiring
//!   [`SchedulerCfg::frame_duration`]). The adaptive refinement is
//!   `[PROPOSAL]` Phase-3 hardening (see [`DropPolicy::Auto`]).
//! - doc 03 §4.10 b `PresentRequest { frame: VideoFrameHandle, .. }` —
//!   the frame is media-types' [`VideoFrame`]: its `FrameStorage::
//!   OpaqueHandle` variant is the handle vocabulary for hardware
//!   frames, so one type covers both paths (see [`sink`]).
//! - doc 01 §2.2 `PresentAction { Present, WaitUntil, SkipLate }` —
//!   the per-frame decision lives on as [`FrameDecision`]
//!   (payload-free: the window scheduler owns frames); the tick-level
//!   [`PresentAction`] carries the request plus the repeat/background
//!   terminal states of doc 03 §4.10 e, with the wait hint exposed as
//!   [`TickOutcome::next_due`].
//! - doc 01 §2.2 `SchedulerCfg { policy, max_repeat, target_lead }` —
//!   extended with `frame_duration` (Auto resolution + missing-PTS
//!   inference) and `max_pending` (the §4.10 c reorder window, bounded
//!   by construction).
//!
//! # Example
//!
//! The wiring this crate's tests exercise end-to-end — scheduler over
//! an injected clock, requests through the queue, recorder at the
//! boundary — on virtual time, no sleeps:
//!
//! ```
//! use std::num::NonZeroUsize;
//! use std::sync::Arc;
//!
//! use media_clock::{MediaClock, PlaybackClock};
//! use media_types::frame::{FrameStorage, VideoFrame};
//! use media_types::packet::SharedBytes;
//! use media_types::time::{Nanoseconds, Rate, Stamp, Timestamp};
//! use media_types::{EngineError, GenerationId, PixelFormat};
//! use video_engine::{
//!     DropPolicy, EnqueueOutcome, PresentAction, PresentationQueue,
//!     PresentationScheduler, Presenter, PresentInfo, PresentRequest,
//!     SchedulerCfg,
//! };
//!
//! // A virtual monotonic source: the scheduler sees exactly the time
//! // the test advances (doc 03 §8 — never ambient time).
//! struct VirtualNs(std::sync::atomic::AtomicU64);
//! impl media_clock::MonotonicSource for VirtualNs {
//!     fn now_ns(&self) -> u64 {
//!         self.0.load(std::sync::atomic::Ordering::Relaxed)
//!     }
//! }
//!
//! struct Recorder(Vec<i64>);
//! impl Presenter for Recorder {
//!     fn present(&mut self, request: &PresentRequest) -> Result<PresentInfo, EngineError> {
//!         self.0.push(request.when.ticks);
//!         Ok(PresentInfo { presented_at: request.when })
//!     }
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let source = Arc::new(VirtualNs(std::sync::atomic::AtomicU64::new(0)));
//! let clock = MediaClock::system_monotonic(
//!     source.clone(),
//!     Timestamp::new(0, media_clock::NANO_BASE),
//!     Rate::ONE,
//! )?;
//! let cfg = SchedulerCfg::for_frame_duration(Nanoseconds(33_333_334))?;
//! let mut scheduler = PresentationScheduler::new(cfg, Arc::new(clock));
//! let mut queue =
//!     PresentationQueue::new(NonZeroUsize::new(2).ok_or("zero capacity")?);
//! let mut sink = Recorder(Vec::new());
//!
//! let mut delivered = 0usize;
//! for frame_index in 0..3i64 {
//!     let pts = frame_index * 33_333_334;
//!     let frame = VideoFrame {
//!         format: PixelFormat::Nv12,
//!         size: (2, 2),
//!         data: FrameStorage::Cpu(SharedBytes::from_vec(vec![0u8; 6])),
//!         pts: Stamp::Present(Timestamp::new(pts, media_clock::NANO_BASE)),
//!         color: media_types::frame::ColorDescription::BT709_LIMITED,
//!         interlaced: false,
//!         generation: GenerationId::FIRST,
//!     };
//!     let _ = scheduler.push(frame);
//!     // Advance virtual time to the frame's slot, then schedule and
//!     // deliver (enqueue + drain is the render-side wiring).
//!     source
//!         .0
//!         .store(pts.unsigned_abs(), std::sync::atomic::Ordering::Relaxed);
//!     if let PresentAction::Present(request) = scheduler.tick().action {
//!         if let EnqueueOutcome::Enqueued = queue.try_push(request) {}
//!     }
//!     delivered += queue.drain_to(&mut sink)?;
//! }
//! assert_eq!(delivered, 3);
//! assert_eq!(sink.0, vec![0, 33_333_334, 66_666_668]); // in order
//! # Ok(())
//! # }
//! ```
//!
//! # Never
//!
//! GPU/rendering APIs, demux/decode types beyond media-types frames,
//! UI, device access (doc 01 §2.2 d). Std-only by C5: zero external
//! dependencies, no unsafe.

pub mod policy;
pub mod queue;
pub mod scheduler;
pub mod sink;

#[cfg(test)]
mod test_support;

pub use policy::{
    DEFAULT_MAX_PENDING, DEFAULT_MAX_REPEAT, DropPolicy, FrameDecision, SchedulerCfg,
    SchedulerCfgError, SyncPolicy,
};
pub use queue::{DrainError, EnqueueOutcome, PresentationQueue, QueueStats};
pub use scheduler::{
    DropTrail, MAX_DROP_TRAIL, PresentAction, PresentationScheduler, PushOutcome, SchedulerStats,
    SyncSample, TickDrops, TickOutcome,
};
pub use sink::{PresentInfo, PresentRequest, Presenter};

// Re-exported for call-site convenience: the canonical nanosecond base
// every position this crate schedules against uses (media-clock owns
// it; re-exported so downstream wiring needs one fewer use-line).
pub use media_clock::NANO_BASE;
