//! The bounded latest-wins frame mailbox: the engine→webview
//! hand-off of variant B (card 2.D3; doc 03 §4.10 c's "rendering
//! never back-pressures decode" applied at the stream boundary).
//!
//! # Discipline
//!
//! - **Bounded**: `capacity` frames, non-zero by construction
//!   ([`NonZeroUsize`]). Occupancy can never exceed the bound no
//!   matter how fast the producer runs — a stalled consumer cannot
//!   grow this side without limit (the W14-A07 unbounded-queue class
//!   lives down-lane in the host channel, never here). At ~3.7 MB per
//!   1280×720 RGBA frame the default capacity 2 is ≈7.4 MB worst-case
//!   resident payload.
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
//! - **Every event is counted AND attributed** — offered, drawn,
//!   dropped (overflow vs stale), the last drop's reason, and the
//!   last enqueued/dequeued emit stamps — so the compatibility
//!   matrix's "dropped frames" cell is measured arithmetic, not
//!   vibes, and the lane's produced-vs-dropped instrumentation is a
//!   [`MailboxStats`] read, not a reimplementation.
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
//! The mailbox itself reads no clock (deterministic by
//! construction): `last_enqueued_ns`/`last_dequeued_ns` are the
//! offered/drawn frames' own `emitted_ns` stamps, so a monitor can
//! read the backlog's freshness without draining.

use std::collections::VecDeque;
use std::num::NonZeroUsize;

use crate::wire::DecodedFrame;

/// Mailbox sizing (the drop policy's only knob).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailboxConfig {
    /// Maximum queued frames. 2 (the default) absorbs one missed
    /// consumer tick at 60 Hz without dropping; the doc 03 §4.11 c
    /// depth-1 mailbox is the render-thread seam, not this one —
    /// `[PROPOSAL: tune on desktop measurements]`. The bound is
    /// memory, not just slots: ~3.7 MB per 1280×720 RGBA frame means
    /// capacity N holds ≈3.7·N MB of resident payload, and the
    /// pacer's 10 ms drain is exhaustive (depth would only age
    /// frames into the one-per-tick shipper's drops), so the default
    /// moves only on hardware evidence, never speculatively.
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

/// Why the mailbox last dropped a frame — drops are counted AND
/// attributed, never silent (the FX6-A01 observability rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailboxDropReason {
    /// The mailbox was full: the OLDEST queued frame lost its slot
    /// (latest-wins — the freshest frame always survives).
    Overflow,
    /// A generation flush dropped queued frames wholesale (the
    /// seek/stop class-A acceptance path).
    Stale,
}

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
    /// Why the most recent drop happened (`None` before the first).
    pub last_drop_reason: Option<MailboxDropReason>,
    /// The `emitted_ns` stamp carried by the most recently offered
    /// frame (the backlog's freshest emit stamp — the mailbox reads
    /// no clock of its own, stamps come from the frames).
    pub last_enqueued_ns: Option<u64>,
    /// The `emitted_ns` stamp carried by the most recently drawn
    /// frame (the consumer's end-to-end latency observable:
    /// `now − last_dequeued_ns`).
    pub last_dequeued_ns: Option<u64>,
}

impl MailboxStats {
    /// Frames produced into the mailbox so far (the produced half of
    /// the produced-vs-dropped instrumentation; == `offered`).
    #[must_use = "pure arithmetic"]
    pub fn frames_produced(&self) -> u64 {
        self.offered
    }

    /// Frames dropped so far, either reason (overflow + stale).
    #[must_use = "pure arithmetic"]
    pub fn frames_dropped(&self) -> u64 {
        self.dropped_overflow + self.dropped_stale
    }

    /// Conservation identity: every offered frame is drawn or
    /// dropped, exactly once — which holds when the queue is empty;
    /// mid-stream, `offered` exceeds the right side by exactly the
    /// frames still queued (`len()`).
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
            self.stats.last_drop_reason = Some(MailboxDropReason::Overflow);
        }
        self.stats.last_enqueued_ns = Some(frame.header.emitted_ns);
        self.queue.push_back(frame);
        self.stats.offered += 1;
        Ok(())
    }

    /// Take the next frame (the consumer side), `None` when empty.
    pub fn draw_next(&mut self) -> Option<ShippedFrame> {
        let frame = self.queue.pop_front()?;
        self.stats.drawn += 1;
        self.stats.last_dequeued_ns = Some(frame.header.emitted_ns);
        Some(frame)
    }

    /// Drain every queued frame into `out`, FIFO order (the consumer
    /// primitive the pacer's exhaustive 10 ms drain is the lane-wide
    /// instance of). Per-record timestamps ride each frame's header.
    /// Accounting is exactly repeated [`FrameMailbox::draw_next`]:
    /// every drained frame ticks `drawn` and re-stamps
    /// `last_dequeued_ns`. Returns how many frames were appended.
    pub fn drain_to(&mut self, out: &mut Vec<ShippedFrame>) -> usize {
        let count = self.queue.len();
        while let Some(frame) = self.queue.pop_front() {
            self.stats.drawn += 1;
            self.stats.last_dequeued_ns = Some(frame.header.emitted_ns);
            out.push(frame);
        }
        count
    }

    /// Peek without consuming (diagnostics/tests).
    #[must_use = "pure inspection"]
    pub fn peek(&self) -> Option<&ShippedFrame> {
        self.queue.front()
    }

    /// The configured bound — occupancy can never exceed it (the
    /// observable half of "bounded": a monitor computes
    /// `len()/capacity()` without touching config plumbing).
    #[must_use = "observable evidence"]
    pub fn capacity(&self) -> usize {
        self.capacity
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
        if dropped > 0 {
            self.stats.last_drop_reason = Some(MailboxDropReason::Stale);
        }
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
        // Hand-built frames are plain owners (no pool attachment —
        // the `detached` convention from the wire module).
        ShippedFrame::detached(
            FrameHeader {
                format: crate::wire::FormatCode::Rgba8,
                flags: crate::wire::FrameFlags::default(),
                width: 1,
                height: 1,
                payload_len: 4,
                frame_id: id,
                pts_ns,
                emitted_ns: 0,
            },
            vec![id as u8, 0, 0, 255],
        )
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

    // ---- FX6-A01: the hardened-backpressure surface (drop policy
    // pinned, timestamps passed through, counters + drop reason +
    // emit stamps observable, bounded occupancy, drain primitive).

    /// A frame with distinct presentation and emit stamps (the
    /// passthrough oracle: what went in must come out untouched).
    fn stamped(id: u64, pts_ns: i64, emitted_ns: u64) -> ShippedFrame {
        ShippedFrame::detached(
            FrameHeader {
                format: crate::wire::FormatCode::Rgba8,
                flags: crate::wire::FrameFlags::default(),
                width: 1,
                height: 1,
                payload_len: 4,
                frame_id: id,
                pts_ns,
                emitted_ns,
            },
            vec![id as u8, 0, 0, 255],
        )
    }

    /// The stalled-consumer property at this seam: production never
    /// stops, nobody ever drains, and occupancy NEVER exceeds the
    /// configured bound — the W14-A07 unbounded-queue class is
    /// impossible here by construction (every excess offer is a
    /// counted, attributed drop).
    #[test]
    fn fx6_a01_occupancy_never_exceeds_capacity_under_endless_production() {
        let capacity = 3;
        let mut mailbox = FrameMailbox::new(MailboxConfig {
            capacity: NonZeroUsize::new(capacity).must(),
        })
        .must();
        assert_eq!(mailbox.capacity(), capacity);
        for id in 0..25u64 {
            mailbox
                .offer(stamped(id, id as i64 * 1_000, 10_000 + id))
                .must();
            assert!(
                mailbox.len() <= capacity,
                "offer {id} pushed occupancy past the bound"
            );
        }
        assert_eq!(mailbox.len(), capacity);
        let stats = mailbox.stats();
        assert_eq!(stats.offered, 25);
        assert_eq!(stats.frames_dropped(), 22);
        assert_eq!(stats.dropped_overflow, 22);
        assert_eq!(stats.last_drop_reason, Some(MailboxDropReason::Overflow));
        // The oldest survivor is the 22nd offer (latest-wins kept the
        // three freshest frames).
        assert_eq!(mailbox.peek().must().header.frame_id, 22);
    }

    /// The drop policy is drop-oldest/latest-wins AND attributed: the
    /// last drop's reason is `Overflow` under pressure and flips to
    /// `Stale` on a flush that drops; a flush that drops nothing
    /// changes no observable.
    #[test]
    fn fx6_a01_drop_policy_reports_last_reason_and_keeps_latest() {
        let mut mailbox = FrameMailbox::new(MailboxConfig::default()).must();
        for id in 0..5u64 {
            mailbox
                .offer(stamped(id, id as i64 * 1_000, 500 + id))
                .must();
        }
        // Capacity 2, 5 offered: the two freshest survive.
        assert_eq!(mailbox.len(), 2);
        assert_eq!(mailbox.peek().must().header.frame_id, 3);
        assert_eq!(
            mailbox.stats().last_drop_reason,
            Some(MailboxDropReason::Overflow)
        );
        // A flush that drops reports the stale reason instead.
        assert_eq!(mailbox.flush_stale(), 2);
        let stats = mailbox.stats();
        assert_eq!(stats.last_drop_reason, Some(MailboxDropReason::Stale));
        assert_eq!(stats.dropped_stale, 2);
        assert_eq!(stats.flushes, 1);
        // A flush that drops nothing changes no observable.
        assert_eq!(mailbox.flush_stale(), 0);
        assert_eq!(
            mailbox.stats().last_drop_reason,
            Some(MailboxDropReason::Stale)
        );
        assert!(mailbox.stats().conserves());
    }

    /// Timestamp passthrough: every drained record carries the exact
    /// pts/emitted stamps it was offered with, and the mailbox-level
    /// enqueued/dequeued observables track the frames' own emitted
    /// stamps (no clock is read inside the mailbox).
    #[test]
    fn fx6_a01_timestamps_pass_through_and_stamp_the_observables() {
        let mut mailbox = FrameMailbox::new(MailboxConfig {
            capacity: NonZeroUsize::new(2).must(),
        })
        .must();
        mailbox.offer(stamped(1, 1_000, 10_000)).must();
        mailbox.offer(stamped(2, 2_000, 20_000)).must();
        mailbox.offer(stamped(3, 3_000, 30_000)).must(); // id 1 dropped
        let stats = mailbox.stats();
        assert_eq!(stats.last_enqueued_ns, Some(30_000));
        assert_eq!(stats.last_dequeued_ns, None);
        let first = mailbox.draw_next().must();
        let second = mailbox.draw_next().must();
        assert_eq!(first.header.frame_id, 2);
        assert_eq!(first.header.pts_ns, 2_000);
        assert_eq!(first.header.emitted_ns, 20_000);
        assert_eq!(second.header.frame_id, 3);
        assert_eq!(second.header.pts_ns, 3_000);
        assert_eq!(second.header.emitted_ns, 30_000);
        assert_eq!(mailbox.stats().last_dequeued_ns, Some(30_000));
        assert!(mailbox.draw_next().is_none());
    }

    /// The counters tick through the whole lifecycle (offers, overflow
    /// drops, one draw, a flush) and conserve exactly at the end; the
    /// policy-governed default capacity is part of the tested
    /// contract.
    #[test]
    fn fx6_a01_counters_tick_through_the_lifecycle_and_conserve() {
        let mut mailbox = FrameMailbox::new(MailboxConfig::default()).must();
        for id in 0..4u64 {
            mailbox
                .offer(stamped(id, id as i64 * 250, 1_000 + id))
                .must();
        }
        assert_eq!(mailbox.draw_next().must().header.frame_id, 2);
        assert_eq!(mailbox.flush_stale(), 1);
        let stats = mailbox.stats();
        assert_eq!(stats.frames_produced(), 4);
        assert_eq!(stats.drawn, 1);
        assert_eq!(stats.dropped_overflow, 2);
        assert_eq!(stats.dropped_stale, 1);
        assert_eq!(stats.frames_dropped(), 3);
        assert!(stats.conserves());
        // Policy-governed capacity: the default (which frame_tap's
        // bound test pins at the lane's only default consumer) is 2.
        assert_eq!(MailboxConfig::default().capacity.get(), 2);
    }

    /// `drain_to` is the exhaustive consumer primitive: FIFO order,
    /// per-record stamps intact, accounting identical to repeated
    /// `draw_next`, and a second drain is a no-op.
    #[test]
    fn fx6_a01_drain_to_empties_fifo_and_accounts_like_draws() {
        let mut mailbox = FrameMailbox::new(MailboxConfig {
            capacity: NonZeroUsize::new(4).must(),
        })
        .must();
        for id in 0..3u64 {
            mailbox
                .offer(stamped(id, id as i64 * 500, 1_000 + 1_000 * id))
                .must();
        }
        let mut drained: Vec<ShippedFrame> = Vec::new();
        assert_eq!(mailbox.drain_to(&mut drained), 3);
        assert_eq!(
            drained
                .iter()
                .map(|frame| frame.header.frame_id)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        // Per-record stamps survive the drain untouched.
        assert_eq!(drained[1].header.pts_ns, 500);
        assert_eq!(drained[1].header.emitted_ns, 2_000);
        assert!(mailbox.is_empty());
        let stats = mailbox.stats();
        assert_eq!(stats.drawn, 3);
        assert_eq!(stats.last_dequeued_ns, Some(3_000));
        // Draining twice appends nothing (the drain is exhaustive).
        assert_eq!(mailbox.drain_to(&mut drained), 0);
        assert_eq!(drained.len(), 3);
        assert!(mailbox.stats().conserves());
    }
}
