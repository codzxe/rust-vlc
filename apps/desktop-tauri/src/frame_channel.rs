//! The tauri frame channel (w31.1, lane `luf1-channel`): the
//! webview-registered channel id rehydrated into a real
//! `tauri::ipc::Channel` bound to the main webview, so the pacer's
//! per-tick drain ships LUF1 records as BINARY payloads.
//!
//! # Why this lives in the wrapper (not `apps/desktop`)
//!
//! The std-only library cannot name `tauri` types (boundaries.toml:
//! the L5 wrapper owns the webview seam). The id → channel
//! rehydration is three lines of tauri API — it lives here, beside
//! the invocation seam it serves.
//!
//! # The payload discipline
//!
//! `Channel<InvokeResponseBody>` with [`InvokeResponseBody::Raw`]:
//! raw bytes ride the fetch-based IPC path for large payloads (a
//! 1280×720 RGBA frame is ~3.7 MB — the JSON-array form a plain
//! `Channel<Vec<u8>>` would produce is 4-6× that and evaluates as a
//! script; the Raw form is the binary lane tauri's channel protocol
//! exists for). The webview side (the UI's channel consumer, the
//! `lumen-frames.mjs` reference port) receives an `ArrayBuffer` per
//! message — one complete LUF1 record.
//!
//! # The bounded ship side (FX6-A03, FIX-PLAN §3 F1.2 / W14-A07)
//!
//! [`FrameLane`] is the bounded, counted queue between the ticker's
//! drain and the webview dispatch — the queue feeding the tauri
//! Channel is an explicit small structure, not an implicit
//! accumulate-and-discard (the W14-A07 unbounded-queue class, handed
//! to FX6 by FX4-A04's report). It is the frame lane's
//! `BoundedEventSink` twin (`apps/desktop/src/dispatch.rs`: bounded,
//! drop-oldest, counted, conserving):
//!
//! - **Bounded**: [`FRAME_LANE_CAPACITY`] records (2 — one landing
//!   plus one in flight, the upstream mailbox's own reasoning; at
//!   ~3.7 MB per 1280×720 RGBA record the lane's worst-case retained
//!   memory is ~7.4 MB, a constant, never a function of the
//!   consumer's speed).
//! - **Latest-wins drop policy**: a full lane drops the OLDEST
//!   queued record (counted) — the freshest always wins, order is
//!   never violated, the ticker is never blocked. The dispatch half
//!   ([`FrameLane::take_newest`]) ships the newest record and
//!   discards the superseded older ones (counted) — the
//!   one-frame-per-tick channel contract, made explicit and counted
//!   where the old ticker's `records.last()` discarded silently.
//! - **A bind is a fresh generation**: the webview reloaded, so
//!   anything still queued is from the previous page —
//!   [`FrameLane::flush_stale`] drops it wholesale, counted (the
//!   mailbox's own `flush_stale` semantics, mirrored at the ship
//!   side; the mailbox's policy itself is not re-implemented here:
//!   the lane holds wire records, the mailbox decoded frames).
//! - **Timestamps pass through verbatim**: a queued record is a
//!   complete LUF1 record — header (with `pts_ns`/`emitted_ns`) +
//!   payload — and the lane ships the exact bytes it was offered;
//!   it never reads, rewrites, or fabricates them (app.rs's `ShipLog`
//!   is the reader that observes them off the dispatched record).
//!
//! # The composition with the ticker lane (FX6-A02's half)
//!
//! `spawn_event_pump` drains the mailbox into this lane (`offer`),
//! rebuilds the channel on a fresh bind (`flush_stale` first — a
//! bind is a new generation), then dispatches one
//! [`FrameLane::take_newest`] per tick into
//! [`send_record`](fn@send_record). app.rs's `ShipLog` latches the
//! dispatched record's own wire stamps (`record_stamps`) and prints
//! the lane's produced-vs-dropped receipt every
//! `FRAME_RECEIPT_EVERY` ships; the lane's own conservation
//! identity (`offered == shipped + dropped_overflow +
//! dropped_superseded + dropped_stale + queued`) carries the whole
//! accounting.
//!
//! # The honest backpressure limit
//!
//! tauri 2.12.5's own per-webview fetch queue (`ChannelDataIpcQueue`)
//! is unbounded and unobservable from the app crate: `Channel::send`
//! is fire-and-forget, the queue's entries are private, and the
//! fetch that drains it is a tauri-internal command — no depth API,
//! no ack, no natural backpressure signal exists (verified against
//! the vendored source, `tauri-2.12.5/src/ipc/channel.rs:47-106`).
//! This lane therefore bounds the queue FEEDING the channel, and the
//! ticker's one-record-per-tick cadence bounds the hand-off rate; a
//! webview that never fetches still parks its handed records inside
//! tauri's own queue (the residual W14-A07 risk, recorded in the
//! FX6-A03 report). Closing it needs a webview→shell ack epoch — a
//! UI-side protocol change, out of this crate's file domain.

use std::collections::VecDeque;
use std::num::NonZeroUsize;

use tauri::ipc::{InvokeResponseBody, JavaScriptChannelId};
use tauri::{AppHandle, Manager, Wry};

/// The send side of the webview's frame channel (one record per
/// message; see the module docs for the payload discipline).
pub type FrameChannel = tauri::ipc::Channel<InvokeResponseBody>;

/// The lane's queue capacity (the drop policy's only knob): 2 —
/// one landing record plus one in flight, the same reasoning as the
/// upstream mailbox's capacity 2. At ~3.7 MB per 1280×720 RGBA
/// record the lane's worst-case retained memory is ~7.4 MB.
pub const FRAME_LANE_CAPACITY: NonZeroUsize = {
    match NonZeroUsize::new(2) {
        Some(value) => value,
        // The impossible arm is a compile-time dead end (the 2.C4
        // convention), not a banned-macro escape hatch.
        #[allow(clippy::empty_loop)] // clippy's suggested `panic!()` is workspace-denied
        None => loop {},
    }
};

/// Rehydrate a bound channel id (the bare `u32` the UI sent in its
/// `session.set-frame-channel` command — tauri's `__CHANNEL__:<id>`
/// reference form) into the send-side channel on the main webview.
///
/// # Errors
/// A `String` sentence (logged by the ticker, never fatal): the id
/// form is invalid (a UI bug), or the main webview is absent (the
/// window closed mid-bind — the next bind retries).
pub fn frame_channel(app: &AppHandle<Wry>, channel: u32) -> Result<FrameChannel, String> {
    let reference = format!("__CHANNEL__:{channel}");
    let id: JavaScriptChannelId = reference
        .parse()
        .map_err(|err: &str| format!("the channel reference {reference:?} is invalid: {err}"))?;
    let webview = app
        .get_webview_window("main")
        .ok_or_else(|| "the main webview window is absent".to_string())?;
    Ok(id.channel_on(webview.as_ref().clone()))
}

/// Ship one LUF1 record as a raw binary message.
///
/// The record MOVES in: the caller's lane owns the bytes, so no
/// second full-frame copy (a 1280×720 RGBA record is ~3.7 MB; the
/// old `&[u8]` signature forced `to_vec()`, copying the identical
/// record twice per ship — the audit's F-003).
///
/// # Errors
/// The tauri send error (the webview went away — a reload or close;
/// the caller drops the channel and the next bind re-establishes).
pub fn send_record(channel: &FrameChannel, record: Vec<u8>) -> Result<(), tauri::Error> {
    channel.send(InvokeResponseBody::Raw(record))
}

/// Observable ship-side accounting (the channel's cells, beside the
/// mailbox's own — FIX-PLAN §3 F1.2's instrumentation: frames shipped
/// vs frames dropped at the seam the webview actually consumes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameLaneStats {
    /// Records offered by the ticker (the tap's whole drain).
    pub offered: u64,
    /// Records handed to the webview dispatch (`send_record`).
    pub shipped: u64,
    /// Records dropped because the lane's queue was full
    /// (latest-wins: the OLDEST was dropped, counted).
    pub dropped_overflow: u64,
    /// Records discarded at dispatch because a newer record was
    /// shipped (the one-frame-per-tick coalesce — what the old
    /// `records.last()` silently discarded).
    pub dropped_superseded: u64,
    /// Records dropped by a bind-generation flush (the webview
    /// reloaded; wholesale, never cumulative).
    pub dropped_stale: u64,
}

impl FrameLaneStats {
    /// Conservation identity: every offered record is shipped or
    /// dropped, exactly once, or still queued.
    #[must_use = "pure arithmetic"]
    pub fn conserves(&self, queued: usize) -> bool {
        self.offered
            == self.shipped
                + self.dropped_overflow
                + self.dropped_superseded
                + self.dropped_stale
                + queued as u64
    }
}

/// The bounded queue between the ticker's drain and the webview
/// dispatch (see the module docs for the policy). Pure std — no
/// tauri types — so the bound and the accounting are provable
/// headlessly, webview or not.
#[derive(Debug)]
pub struct FrameLane {
    capacity: usize,
    queue: VecDeque<Vec<u8>>,
    stats: FrameLaneStats,
}

impl FrameLane {
    /// The reference lane (capacity [`FRAME_LANE_CAPACITY`]).
    pub fn new() -> FrameLane {
        Self::with_capacity(FRAME_LANE_CAPACITY)
    }

    /// A lane with an explicit capacity (≥ 1 by type; tests and
    /// probes).
    #[must_use = "pure constructor"]
    pub fn with_capacity(capacity: NonZeroUsize) -> FrameLane {
        FrameLane {
            capacity: capacity.get(),
            queue: VecDeque::with_capacity(capacity.get()),
            stats: FrameLaneStats::default(),
        }
    }

    /// Offer one drained record (the ticker side). Never blocks,
    /// never fails, never reorders; a full lane drops the OLDEST
    /// queued record (counted) — the freshest always wins.
    pub fn offer(&mut self, record: Vec<u8>) {
        if self.queue.len() == self.capacity && self.queue.pop_front().is_some() {
            self.stats.dropped_overflow += 1;
        }
        self.queue.push_back(record);
        self.stats.offered += 1;
    }

    /// The dispatch half: take the NEWEST queued record for one
    /// `Channel::send` (the one-frame-per-tick contract — the webview
    /// composites at the pacer's own cadence). The superseded older
    /// records are discarded and counted; the lane is empty after.
    ///
    /// The returned record is the exact bytes offered — the record's
    /// own header (timestamps included) passes through verbatim.
    pub fn take_newest(&mut self) -> Option<Vec<u8>> {
        let newest = self.queue.pop_back()?;
        self.stats.shipped += 1;
        while self.queue.pop_front().is_some() {
            self.stats.dropped_superseded += 1;
        }
        Some(newest)
    }

    /// Flush every queued record (generation change: the webview
    /// reloaded and re-bound). Returns the number dropped, counted as
    /// `dropped_stale` — the mailbox's own `flush_stale` semantics.
    pub fn flush_stale(&mut self) -> usize {
        let dropped = self.queue.len();
        self.stats.dropped_stale += dropped as u64;
        self.queue.clear();
        dropped
    }

    /// Records currently queued (occupancy).
    #[must_use = "observable evidence"]
    pub fn queued(&self) -> usize {
        self.queue.len()
    }

    /// Whether the lane's queue is at capacity — the ship-side stall
    /// signature (offers keep flowing while the dispatch stopped
    /// taking).
    #[must_use = "pure inspection"]
    pub fn is_full(&self) -> bool {
        self.queue.len() == self.capacity
    }

    /// The accounting so far.
    #[must_use = "observable evidence"]
    pub fn stats(&self) -> FrameLaneStats {
        self.stats
    }

    /// Conservation over the lane's own queue.
    #[must_use = "pure arithmetic"]
    pub fn conserves(&self) -> bool {
        self.stats.conserves(self.queue.len())
    }
}

impl Default for FrameLane {
    fn default() -> Self {
        Self::new()
    }
}
