//! The Lumen bounded, generation-aware queue primitive (card 2.B1,
//! doc 03 §4.4; registry row doc 01 §2.2: layer L2, deps media-types).
//!
//! One generic, heavily tested primitive reused for every stage
//! boundary (doc 01 "crates/packet-queue"): demux→decode packet
//! queues, decode→engine frame queues. The landing order of this crate
//! is: limits and stamps first (this commit), then the queue core with
//! backpressure, then the generation machinery — see the worklog for
//! card 2.B1.
//!
//! What the finished crate enforces, per doc 03 §4.4 and MP §8:
//!
//! - **Boundedness (Q1).** Three simultaneous caps — packets, bytes,
//!   media duration — and *whichever hits first* refuses the next
//!   push. An unbounded queue is unconstructible: caps are
//!   `NonZeroUsize` / known-positive `media_types::time::Duration` by
//!   typed constructor ([`QueueLimits::new`],
//!   [`QueueLimits::audio_default`], [`QueueLimits::video_default`]).
//! - **Backpressure (Q3).** Producers never spin and never drop fresh
//!   data: a non-blocking push returns a typed full signal (with the
//!   item back, untouched), and the parking push sleeps on a condvar
//!   until space appears, the generation moves, the queue closes, or
//!   a caller-supplied deadline expires — never unboundedly by
//!   surprise.
//! - **Generations (Q2).** Items are [`Stamped`] with the generation
//!   they were produced under; the queue publishes one generation at
//!   a time. A flush drops every resident item wholesale and publishes
//!   the new generation; a later push of an older-generation item is
//!   dropped *at the boundary* with a typed stale signal, counted in
//!   the queue's stats (doc 03 §5 metric `stale_dropped`). Consumers
//!   observe the wholesale drop as a typed discontinuity — never as
//!   silent data loss.
//! - **Watermarks (Q4).** High/low hit flags plus resident usage are
//!   the single input to `Buffering` transitions (the policy itself
//!   is the playback engine's).
//!
//! # Threading (doc 03 §4.4(c))
//!
//! `Mutex<VecDeque>` + condvars — MPMC-capable: demux thread and
//! decode workers produce, audio pump and video scheduler consume. The
//! lock is held only for deque pushes/pops — never across I/O, decode,
//! or user code: [`Weigh`] is evaluated once per push *outside* the
//! critical section, and critical sections contain only infallible
//! operations.
//!
//! # Module map
//!
//! - [`limits`] — [`QueueLimits`], [`QueueUsage`], [`LimitAxis`],
//!   [`WatermarkThresholds`], [`Watermark`], [`QueueConfigError`].
//! - [`stamped`] — [`Stamped`], the [`Generational`] and [`Weigh`]
//!   payload seams (implemented for media-types `Packet`).

pub mod limits;
pub mod stamped;

pub use limits::{
    LimitAxis, QueueConfigError, QueueLimits, QueueUsage, Watermark, WatermarkThresholds,
};
pub use stamped::{Generational, Stamped, Weigh};
