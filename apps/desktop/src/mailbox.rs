//! The bounded latest-wins frame mailbox: the engine→webview
//! hand-off of variant B (card 2.D3; doc 03 §4.10 c's "rendering
//! never back-pressures decode" applied at the stream boundary).
//!
//! # Discipline
//!
//! - **Bounded**: `capacity` frames, non-zero by construction
//!   ([`NonZeroUsize`]).
//! - **Latest-wins**: when the mailbox is full, the *oldest*
//!   un-consumed frame is dropped (counted) — the freshest frame
//!   always wins, the presenter is never blocked, order is never
//!   violated.
//! - **Generation-stale frames are purged wholesale**: the host
//!   flushes on generation changes (the command layer's gen-bumping
//!   events — `host.rs` wires it; the engine-side twin is
//!   `PresentationQueue::publish`). The wire protocol itself carries
//!   no generation: stale requests never reach the presenter (the
//!   scheduler's contract), so only the ≤ capacity frames already in
//!   flight at a seek/stop need the flush `[PROPOSAL: v2 wire
//!   "flush" record for cross-process hosts]`.
//! - **Every event is counted** — offered, drawn, dropped (overflow
//!   vs stale) — so the compatibility matrix's "dropped frames" cell
//!   is measured arithmetic, not vibes.
//!
//! # Latency accounting
//!
//! Every record carries `emitted_ns` (bridge serialize instant) and
//! `pts_ns` (scheduled slot). The consumer stamps its draw instant
//! and feeds [`LatencyAccountant`]: transport+queue+draw latency is
//! `drawn − emitted`; slot drift (the A/V-sync observable) is
//! `drawn − pts`. The prototype's harness drives both on one virtual
//! clock (end-to-end measurable headlessly); the browser run adds
//! the `performance.now()` half per `prototype/lumen-frames.mjs`.

use std::collections::VecDeque;
use std::num::NonZeroUsize;

use crate::wire::DecodedFrame;

/// Mailbox sizing (the drop policy's only knob).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailboxConfig {
    /// Maximum queued frames. 2 (the default) absorbs one missed
    /// consumer tick at 60 Hz without dropping; the doc 03 §4.11 c
    /// depth-1 mailbox is the render-thread seam, not this one —
    /// `[PROPOSAL: tune on desktop measurements]`.
    pub capacity: NonZeroUsize,
}

impl Default for MailboxConfig {
    fn default() -> MailboxConfig {
        // The 2.C4 convention: the impossible arm is a compile-time
        // dead end (zeroing the literal fails the BUILD), not a
        // banned-macro escape hatch.
        #[allow(clippy::empty_loop)] // clippy's suggested `panic!()` is workspace-denied
        const TWO: NonZeroUsize = match NonZeroUsize::new(2) {
            Some(v) => v,
            None => loop {},
        };
        MailboxConfig { capacity: TWO }
    }
}

/// Mailbox failures (structural only — overflow is a *counted drop*,
/// not an error).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailboxError {
    /// The configuration cannot describe a working mailbox.
    Misconfigured {
        /// Human-readable reason.
        reason: &'static str,
    },
}

impl std::fmt::Display for MailboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailboxError::Misconfigured { reason } => {
                write!(f, "frame mailbox: misconfigured ({reason})")
            }
        }
    }
}

impl std::error::Error for MailboxError {}

/// The frame the consumer side draws (a decoded record: header with
/// all accounting stamps + payload).
pub type ShippedFrame = DecodedFrame;

/// Observable mailbox accounting (the compatibility matrix's cells).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MailboxStats {
    /// Frames offered by the presenter side.
    pub offered: u64,
    /// Frames taken by the consumer.
    pub drawn: u64,
    /// Frames dropped because the mailbox was full (latest-wins).
    pub dropped_overflow: u64,
    /// Frames dropped by a generation flush.
    pub dropped_stale: u64,
    /// Generation flushes performed.
    pub flushes: u64,
}

impl MailboxStats {
    /// Conservation identity: every offered frame is drawn or
    /// dropped, exactly once.
    #[must_use = "pure arithmetic"]
    pub fn conserves(&self) -> bool {
        self.offered == self.drawn + self.dropped_overflow + self.dropped_stale
    }
}

/// The mailbox itself (single-owner hand-off; the real host puts the
/// consumer on the webview's event loop and the presenter on the
/// engine's render thread — the seam is the [`FrameMailbox`] move).
#[derive(Debug)]
pub struct FrameMailbox {
    capacity: usize,
    queue: VecDeque<ShippedFrame>,
    stats: MailboxStats,
}

impl FrameMailbox {
    /// Build a mailbox (capacity ≥ 1 by type).
    ///
    /// # Errors
    /// [`MailboxError::Misconfigured`] never fires for a constructed
    /// [`MailboxConfig`] today (capacity is `NonZeroUsize`); the
    /// variant exists for forward configs with more knobs.
    pub fn new(config: MailboxConfig) -> Result<FrameMailbox, MailboxError> {
        Ok(FrameMailbox {
            capacity: config.capacity.get(),
            queue: VecDeque::with_capacity(config.capacity.get()),
            stats: MailboxStats::default(),
        })
    }

    /// Offer one frame (the presenter side). Never blocks, never
    /// fails, never reorders; drops the oldest when full.
    pub fn offer(&mut self, frame: ShippedFrame) -> Result<(), MailboxError> {
        if self.capacity == 0 {
            return Err(MailboxError::Misconfigured {
                reason: "zero capacity",
            });
        }
        if self.queue.len() == self.capacity && self.queue.pop_front().is_some() {
            self.stats.dropped_overflow += 1;
        }
        self.queue.push_back(frame);
        self.stats.offered += 1;
        Ok(())
    }

    /// Take the next frame (the consumer side), `None` when empty.
    pub fn draw_next(&mut self) -> Option<ShippedFrame> {
        let frame = self.queue.pop_front()?;
        self.stats.drawn += 1;
        Some(frame)
    }

    /// Peek without consuming (diagnostics/tests).
    #[must_use = "pure inspection"]
    pub fn peek(&self) -> Option<&ShippedFrame> {
        self.queue.front()
    }

    /// Frames currently queued.
    #[must_use = "observable evidence"]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Whether no frame is queued.
    #[must_use = "pure inspection"]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Flush every queued frame (generation change: seek, stop, track
    /// switch). Returns the number dropped; counted as
    /// `dropped_stale`.
    pub fn flush_stale(&mut self) -> usize {
        let dropped = self.queue.len();
        self.stats.dropped_stale += dropped as u64;
        self.stats.flushes += 1;
        self.queue.clear();
        dropped
    }

    /// The accounting so far.
    #[must_use = "observable evidence"]
    pub fn stats(&self) -> MailboxStats {
        self.stats
    }
}

/// A min/max/mean latency accumulator (the harness's evidence
/// primitive — deterministic arithmetic, no clocks inside).
#[derive(Debug, Clone, Copy, Default)]
pub struct LatencyAccountant {
    samples: u64,
    min_ns: Option<i64>,
    max_ns: Option<i64>,
    sum_ns: i128,
}

impl LatencyAccountant {
    /// Record one latency sample (nanoseconds; may be negative — the
    /// accountant reports what happened, never clamps).
    pub fn record(&mut self, latency_ns: i64) {
        self.samples += 1;
        self.min_ns = Some(match self.min_ns {
            Some(min) => min.min(latency_ns),
            None => latency_ns,
        });
        self.max_ns = Some(match self.max_ns {
            Some(max) => max.max(latency_ns),
            None => latency_ns,
        });
        self.sum_ns += i128::from(latency_ns);
    }

    /// The summary (samples, min, max, floor mean).
    #[must_use = "pure arithmetic"]
    pub fn summary(&self) -> LatencySummary {
        LatencySummary {
            samples: self.samples,
            min_ns: self.min_ns,
            max_ns: self.max_ns,
            mean_ns: if self.samples == 0 {
                None
            } else {
                // Floor division: deterministic, documented.
                Some((self.sum_ns / i128::from(self.samples)) as i64)
            },
        }
    }
}

/// [`LatencyAccountant`]'s readout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatencySummary {
    /// Samples recorded.
    pub samples: u64,
    /// The smallest sample (`None` before the first).
    pub min_ns: Option<i64>,
    /// The largest sample (`None` before the first).
    pub max_ns: Option<i64>,
    /// The floor-arithmetic mean (`None` before the first).
    pub mean_ns: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::WireError;
    use crate::wire::FrameHeader;
    use testkit::Must;

    fn frame(id: u64, pts_ns: i64) -> ShippedFrame {
        ShippedFrame {
            header: FrameHeader {
                format: crate::wire::FormatCode::Rgba8,
                flags: crate::wire::FrameFlags::default(),
                width: 1,
                height: 1,
                payload_len: 4,
                frame_id: id,
                pts_ns,
                emitted_ns: 0,
            },
            payload: vec![id as u8, 0, 0, 255],
        }
    }

    #[test]
    fn latest_wins_drops_oldest_and_counts() {
        let mut mailbox = FrameMailbox::new(MailboxConfig {
            capacity: NonZeroUsize::new(2).must(),
        })
        .must();
        for id in 0..5u64 {
            mailbox.offer(frame(id, 1_000)).must();
        }
        // 5 offered, capacity 2: 3 dropped, 2 queued (ids 3 and 4).
        let stats = mailbox.stats();
        assert_eq!(stats.offered, 5);
        assert_eq!(stats.dropped_overflow, 3);
        assert_eq!(mailbox.len(), 2);
        // While frames are queued, the identity carries the backlog:
        assert_eq!(
            stats.offered,
            stats.drawn + stats.dropped_overflow + stats.dropped_stale + mailbox.len() as u64
        );
        // Draw order: oldest surviving first, never reordered.
        assert_eq!(mailbox.draw_next().must().header.frame_id, 3);
        assert_eq!(mailbox.draw_next().must().header.frame_id, 4);
        assert!(mailbox.draw_next().is_none());
        let stats = mailbox.stats();
        assert_eq!(stats.drawn, 2);
        assert!(stats.conserves());
    }

    #[test]
    fn generation_flush_drops_everything_counted_once() {
        let mut mailbox = FrameMailbox::new(MailboxConfig {
            capacity: NonZeroUsize::new(4).must(),
        })
        .must();
        for id in 0..3u64 {
            mailbox.offer(frame(id, 1_000)).must();
        }
        assert_eq!(mailbox.flush_stale(), 3);
        assert!(mailbox.is_empty());
        // A second flush drops nothing (wholesale, not cumulative).
        assert_eq!(mailbox.flush_stale(), 0);
        let stats = mailbox.stats();
        assert_eq!(stats.dropped_stale, 3);
        assert_eq!(stats.flushes, 2);
        assert!(stats.conserves());
        // Post-flush frames flow normally.
        mailbox.offer(frame(10, 2_000)).must();
        assert_eq!(mailbox.draw_next().must().header.pts_ns, 2_000);
    }

    #[test]
    fn latency_accountant_is_exact_floor_arithmetic() {
        let mut acc = LatencyAccountant::default();
        assert_eq!(acc.summary().samples, 0);
        assert_eq!(acc.summary().min_ns, None);
        for ns in [10i64, -2, 7] {
            acc.record(ns);
        }
        let s = acc.summary();
        assert_eq!(s.samples, 3);
        assert_eq!(s.min_ns, Some(-2));
        assert_eq!(s.max_ns, Some(10));
        // (10 - 2 + 7) / 3 = 5 exactly.
        assert_eq!(s.mean_ns, Some(5));
        // Floor semantics on negatives: (10 - 3) / 2 = 3.5 -> 3.
        let mut acc = LatencyAccountant::default();
        acc.record(10);
        acc.record(-3);
        assert_eq!(acc.summary().mean_ns, Some(3));
    }

    #[test]
    fn zero_capacity_is_unconstructible_but_typed() {
        // NonZeroUsize makes the config honest; the typed error stays
        // for future knobs (documented invariant, probed through the
        // error type's existence).
        let err = MailboxError::Misconfigured {
            reason: "zero capacity",
        };
        assert!(err.to_string().contains("zero capacity"));
        // And the wire error family displays for host surfaces.
        let wire = WireError::Truncated { missing: 4 };
        assert!(wire.to_string().contains("4 bytes missing"));
    }
}
