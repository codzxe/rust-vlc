//! The Lumen bounded, generation-aware queue primitive (card 2.B1,
//! doc 03 §4.4; registry row doc 01 §2.2: layer L2, deps media-types).
//!
//! One generic, heavily tested primitive reused for every stage
//! boundary (doc 01 "crates/packet-queue"): demux→decode packet
//! queues, decode→engine frame queues. What the crate enforces, per
//! doc 03 §4.4 and MP §8:
//!
//! - **Boundedness (Q1).** Three simultaneous caps — packets, bytes,
//!   media duration — and *whichever hits first* refuses the next
//!   push. An unbounded queue is unconstructible: caps are
//!   `NonZeroUsize` / known-positive `media_types::time::Duration` by
//!   typed constructor ([`QueueLimits::new`],
//!   [`QueueLimits::audio_default`], [`QueueLimits::video_default`]).
//! - **Backpressure (Q3).** Backpressure is the *only* flow control.
//!   Producers never spin and never drop fresh data: a non-blocking
//!   [`try_push`](BoundedQueue::try_push) returns a typed
//!   [`PushOutcome::Full`] signal (with the item back, untouched), and
//!   the parking [`push_wait`](BoundedQueue::push_wait) sleeps on a
//!   condvar until space appears, the queue closes, or a
//!   caller-supplied deadline expires — never unboundedly by surprise.
//!   The session thread only ever uses the non-parking operations.
//! - **Generations (Q2).** Items are [`Stamped`] with the generation
//!   they were produced under; the queue publishes one generation at
//!   a time. A flush drops every resident item wholesale and publishes
//!   the new generation; a later push of an older-generation item is
//!   dropped *at the boundary* with a typed stale signal, counted in
//!   the queue's stats (doc 03 §5 metric `stale_dropped`). Consumers
//!   observe the wholesale drop as a typed discontinuity — never as
//!   silent data loss.
//! - **Watermarks (Q4).** [`watermark`](BoundedQueue::watermark)
//!   reports high/low hit flags plus resident usage — the single
//!   input to `Buffering` transitions (the policy itself is the
//!   playback engine's).
//!
//! # Threading (doc 03 §4.4(c))
//!
//! `Mutex<VecDeque>` + two condvars (space, items) — MPMC-capable:
//! demux thread and decode workers produce, audio pump and video
//! scheduler consume. The lock is held only for deque pushes/pops —
//! never across I/O, decode, or user code: [`Weigh`] is evaluated
//! once per push *outside* the critical section, and critical
//! sections contain only infallible operations.
//!
//! # Deviations from the doc 03 §4.4 sketch (adaptations, card 2.B1)
//!
//! - `Generation` → [`media_types::GenerationId`] (the L0 type; the
//!   doc's bare `u64` alias was superseded by it).
//! - `push(&self, item, gen_check: &CancelToken)` → the parking
//!   [`push_wait`](BoundedQueue::push_wait) plus non-blocking
//!   [`try_push`](BoundedQueue::try_push). `CancelToken` is minted by
//!   the playback engine (L4, Phase 3) and does not exist yet; per
//!   doc 03 §5 checking point 3 the *queue's own published
//!   generation* is the boundary check, and a parked producer wakes
//!   on its movement exactly as the token would.
//! - `pop(&self) -> Result<_, QueueEmpty>` →
//!   [`try_pop`](BoundedQueue::try_pop) returning [`PopOutcome`]:
//!   `Err(Empty)`-means-would-block survives as [`PopOutcome::Empty`],
//!   alongside the typed discontinuity and closed signals the card
//!   requires.
//! - `QueueLimits.max_duration: MediaTime` → known-positive
//!   [`media_types::time::Duration`] at the constructor boundary,
//!   stored as [`media_types::Nanoseconds`].
//!
//! # Example
//!
//! A bounded queue of media-types packets (the full backpressure and
//! stale-drop proofs live in `tests/backpressure.rs` and
//! `tests/generation.rs`):
//!
//! ```
//! use media_types::packet::SharedBytes;
//! use media_types::time::{Duration, Nanoseconds};
//! use media_types::{GenerationId, Packet, Stamp, TrackId};
//! use packet_queue::{
//!     BoundedQueue, PopOutcome, PushOutcome, QueueLimits, Stamped,
//! };
//! use std::num::NonZeroUsize;
//! use std::time::{Duration as WallClock, Instant};
//! use testkit::Must;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let limits = QueueLimits::new(
//!     NonZeroUsize::new(2).must(),
//!     NonZeroUsize::new(4096).must(),
//!     Duration::Known(Nanoseconds(1_000)),
//! )?;
//! let queue = BoundedQueue::with_default_watermarks(limits);
//!
//! let packet = Packet {
//!     track: TrackId::new(1),
//!     pts: Stamp::Missing,
//!     dts: Stamp::Missing,
//!     duration: Some(Duration::Known(Nanoseconds(10))),
//!     data: SharedBytes::from_vec(vec![0xA5; 32]),
//!     keyframe: true,
//!     discontinuity: false,
//!     generation: GenerationId::FIRST,
//! };
//!
//! // Producers learn fullness without blocking (typed signal, item
//! // handed back untouched):
//! assert_eq!(queue.try_push(Stamped::from_payload(packet.clone())), PushOutcome::Pushed);
//! assert_eq!(queue.try_push(Stamped::from_payload(packet.clone())), PushOutcome::Pushed);
//! match queue.try_push(Stamped::from_payload(packet.clone())) {
//!     PushOutcome::Full { item, .. } => assert_eq!(item.value, packet),
//!     other => unreachable!("expected Full, got {other:?}"),
//! }
//!
//! // A consumer drains; the parked producer then completes:
//! assert!(matches!(queue.try_pop(), PopOutcome::Item(_)));
//! match queue.push_wait(
//!     Stamped::from_payload(packet),
//!     Some(Instant::now() + WallClock::from_millis(50)),
//! ) {
//!     Ok(()) => {}
//!     Err(abort) => unreachable!("queue has space after the drain: {abort:?}"),
//! }
//!
//! // Watermark state feeds the engine's Buffering decisions (Q4):
//! let watermark = queue.watermark();
//! assert_eq!(watermark.usage.packets, 2);
//! # Ok(())
//! # }
//! ```
//!
//! # Module map
//!
//! - [`limits`] — [`QueueLimits`], [`QueueUsage`], [`LimitAxis`],
//!   [`WatermarkThresholds`], [`Watermark`], [`QueueConfigError`].
//! - [`stamped`] — [`Stamped`], the [`Generational`] and [`Weigh`]
//!   payload seams (implemented for media-types `Packet`).
//! - [`queue`] — [`BoundedQueue`] and its typed signals:
//!   [`PushOutcome`], [`PushAborted`], [`PopOutcome`],
//!   [`QueueStats`]; [`PacketQueue`] pins the packet flavor.

pub mod limits;
pub mod queue;
pub mod stamped;

pub use limits::{
    LimitAxis, QueueConfigError, QueueLimits, QueueUsage, Watermark, WatermarkThresholds,
};
pub use queue::{BoundedQueue, Discontinuity, PopOutcome, PushAborted, PushOutcome, QueueStats};
pub use stamped::{Generational, Stamped, Weigh};

/// The demux→decode packet queue flavor of [`BoundedQueue`] —
/// media-types [`media_types::Packet`] payloads, audio/video caps from
/// [`QueueLimits::audio_default`] / [`QueueLimits::video_default`].
pub type PacketQueue = BoundedQueue<media_types::Packet>;
