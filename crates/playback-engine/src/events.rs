//! Event-completeness policy (card 3.B4, doc 05 §3.4): the
//! throttling and coalescing rules the engine applies to its event
//! stream, as data-driven types — the emission points live in
//! [`crate::session`], the payload vocabulary in [`crate::event`].
//!
//! # The doc 05 §3.4 rules, formalized
//!
//! 1. **`timing.time-tick` is the only continuous stream** — its
//!    interval is config-driven (`playback.time-tick-interval-ms`,
//!    default 250 ms, floor 50 ms — [`TimeTickPolicy`]) and it is
//!    **never queued**: at most one tick is in flight per session and
//!    a newer tick *replaces* an undelivered older one (the outbox
//!    replacement in [`crate::session::OutputHub`], counted in
//!    [`crate::session::EngineStats::time_ticks_replaced`]).
//! 2. **Progress events are monotonic within one episode and
//!    coalesced to their rate cap** — `session.buffering` to ≤ 5 Hz
//!    ([`BUFFERING_WINDOW_MS`]); the episode bookends (`0.0` on
//!    enter, `1.0` on exit) are table effects and bypass the window
//!    (doc 05 §3.2 pins them, not a rate).
//! 3. **All other events are discrete and unthrottled**, bounded
//!    naturally by the operations that cause them — except non-fatal
//!    `playback.error`, which is coalesced per 1 s window with a
//!    count `[PROPOSAL]` ([`DamageCoalescer`]).
//! 4. **The burst bound**: no more than 1 000 events/s sustained to
//!    any host connection `[PROPOSAL: number to be validated against
//!    the UI budget]`. The v0 mechanism is the outbox capacity
//!    ([`crate::session::OUTBOX_CAPACITY`] = 1024 live events, one
//!    drain interval's worth): at capacity the oldest *event* is
//!    dropped and counted while receipts are pinned — and the
//!    receipts themselves are hard-capped at
//!    [`crate::session::RECEIPT_CAPACITY`] = 4096 (beyond it the
//!    oldest undelivered receipt is dropped and counted — a host
//!    that never drains loses responses before it loses memory;
//!    engineering directive D-1 boundedness). Doc 05 §3.4 names
//!    `system.log` as the first drop candidate — that refinement
//!    activates when `system.log` emission exists (audit row: no
//!    logging bridge yet); today's policy drops oldest-event-first,
//!    which is strictly more conservative for every other kind.
//!
//! # Timing sources
//!
//! Every window reads the session's **injected** monotonic source
//! ([`SessionConfig::throttle`](crate::session::SessionConfig)),
//! never ambient time — the media-clock injection doctrine (doc 03
//! §8). Hosts that want wall-clock windows inject
//! [`media_clock::InstantSource`]; deterministic tests inject a
//! virtual counter and advance it by hand.
//!
//! The time-tick interval is judged against the **media position**
//! (the playback clock's `now()`), not the throttle source: at the
//! v0 constant rate 1.0 the two are equivalent, and a frozen test
//! clock therefore produces no ticks at all (deterministic).
//! `[PROPOSAL: when card 3.B1 lands variable rate, the interval
//! check moves to the throttle source so "every 250 ms" stays
//! wall-time true at 2× playback]`.
//!
//! # The emission audit (doc 05 §3.2 — 23 rows)
//!
//! The engine-side status of every catalog row. "Emitted" rows are
//! proven by the contract suite (`tests/contract_suite.rs`); the
//! rest record the honest owner of the missing trigger.
//!
//! | Event | Status | Payload check |
//! |---|---|---|
//! | `session.state-changed` | **emitted** (2.C2) | `{from,to}` from the transition row, self-transitions included (C8) |
//! | `session.opening-progress` | wired-by-later — network opens do not exist in the engine yet (streaming sources are Phase 6; local opens are synchronous) | n/a |
//! | `session.buffering` | **emitted** (2.C2 bookends; 3.B4 adds the ≤ 5 Hz monotonic in-episode progress) | bookends `0.0`/`1.0`; in-episode `0.0..1.0` queue fill |
//! | `timing.time-tick` | **emitted** (3.B4) | `{time, position}` — playback-clock position µs, clamped fraction of the known duration |
//! | `timing.duration-known` | **emitted** (3.B4) | `{duration}` — the greatest declared track extent, once per open when derivable |
//! | `timing.seek-completed` | **emitted** (3.A1) | `{position, seekKind: "command"}` — the terminal for the generation that completed, stamped with the in-flight seek's own gen |
//! | `timing.seek-failed` | **emitted** (3.A1) | `{error}` — typed D4 refusal, old position; exactly one terminal per completed gen |
//! | `tracks.list-ready` | **emitted** (2.C2) | `{tracks}` — the wired list with selection truth, before the first `→ playing` |
//! | `tracks.changed` | wired-by-sibling 3.A3 (track selection commands; no engine-side track-loss trigger exists in v0 — the demuxer cannot report a disappearing track) | n/a |
//! | `playback.end-of-track` | **emitted** (3.B4) | `{trackKind, id}` — a limb newly drained while another still has work |
//! | `playback.end-of-media` | **emitted** (2.C2) | `{}` — before `state-changed → ended` |
//! | `playback.error` | **emitted** (2.C2 fatal; 3.B4 adds the non-fatal bounded-recovery class) | fatal: `{stage,code,message,details,fatal:true}`; non-fatal: same shape, `fatal:false`, coalesced count |
//! | `playback.rate-changed` | wired-by-later 3.B1 (rate support; no `set-rate` engine input exists) | n/a |
//! | `audio.device-lost` | wired-by-later (device observation, 2.C3/7.C1; the null device cannot be lost) | n/a |
//! | `audio.device-changed` | wired-by-later (same) | n/a |
//! | `video.screenshot-saved` | wired-by-later (the screenshot path is renderer/host work) | n/a |
//! | `playlist.changed` | wired-by-later (no playlist model in the engine; Phase 4) | n/a |
//! | `config.setting-changed` | wired-by-later (settings writes are host/config-layer work) | n/a |
//! | `plugins.loaded` | wired-by-later (plugin registry, doc 06) | n/a |
//! | `plugins.failed` | wired-by-later (same) | n/a |
//! | `plugins.crashed` | wired-by-later (sandboxed WASM host, Phase 5) | n/a |
//! | `system.log` | wired-by-later (no logging bridge; the burst-bound drop refinement activates here) | n/a |
//! | `media.info-ready` | **emitted** (3.B4) | `{info}` — the engine's open-time snapshot (source, duration, tracks), once per open, before the first `→ playing` |
//!
//! 11 of 23 rows are engine-emitted today (the 11 ids above,
//! counting both seek terminals and both halves of
//! `playback.error`); 12 rows are honestly owned by later cards.
//! The catalog itself (61/23) is frozen data (decision D-017) — this
//! card changes emission completeness only. The machine-checked
//! twin of this table is the catalog-side 23-row ledger
//! (`crates/commands/tests/catalog_emission_ledger.rs`) plus the
//! contract suite (`crates/playback-engine/tests/contract_suite.rs`),
//! which proves every "emitted" row against the real engine.

use std::fmt;
use std::sync::Arc;

use media_clock::MonotonicSource;

/// The default `timing.time-tick` interval (doc 05 §3.2: "default
/// every 250 ms").
pub const TIME_TICK_DEFAULT_MS: u32 = 250;

/// The floor for the `timing.time-tick` interval (doc 05 §3.2:
/// "floor 50 ms" — `playback.time-tick-interval-ms` cannot go below
/// it, whatever the config says).
pub const TIME_TICK_FLOOR_MS: u32 = 50;

/// The `session.buffering` rate cap as a coalescing window (doc 05
/// §3.2: "≤ 5 Hz" — at most one in-episode progress event per
/// window; the `0.0`/`1.0` bookends are table effects outside it).
pub const BUFFERING_WINDOW_MS: u64 = 200;

/// The non-fatal `playback.error` coalescing window (doc 05 §3.2:
/// "non-fatal same-code errors coalesced per 1 s window with a
/// count" — `[PROPOSAL]` in the doc, formalized here).
pub const DAMAGE_WINDOW_MS: u64 = 1_000;

/// The `timing.time-tick` policy (doc 05 §3.2/§3.4 rule 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeTickPolicy {
    /// The interval in milliseconds, at or above
    /// [`TIME_TICK_FLOOR_MS`].
    pub interval_ms: u32,
}

impl TimeTickPolicy {
    /// The policy from a configured interval, clamped to the doc
    /// floor (a config value below 50 ms — or 0, the unset marker —
    /// lands on the floor; the floor is a contract, not a
    /// suggestion).
    #[must_use]
    pub fn from_config_ms(interval_ms: u32) -> TimeTickPolicy {
        TimeTickPolicy {
            interval_ms: interval_ms.max(TIME_TICK_FLOOR_MS),
        }
    }

    /// The doc default (250 ms).
    #[must_use]
    pub fn default_policy() -> TimeTickPolicy {
        TimeTickPolicy::from_config_ms(TIME_TICK_DEFAULT_MS)
    }

    /// Whether a tick is due, judged on media position (nanoseconds
    /// — see the module docs for the rate-1.0 equivalence note):
    /// `None` baseline means "started, not yet ticked" and only
    /// *sets* the baseline (doc 05 §3.2 lists no immediate tick on
    /// play; the immediate cases are seek-completion, pause and rate
    /// change).
    #[must_use]
    pub fn is_due(&self, last_emitted_ns: Option<i64>, position_ns: i64) -> bool {
        match last_emitted_ns {
            None => false,
            Some(last) => {
                position_ns.saturating_sub(last) >= i64::from(self.interval_ms) * 1_000_000
            }
        }
    }
}

impl Default for TimeTickPolicy {
    fn default() -> TimeTickPolicy {
        TimeTickPolicy::default_policy()
    }
}

/// A fixed coalescing window over an injected monotonic source:
/// accepts a value, answers whether the window has elapsed (and a
/// new one opens). The carrier of doc 05 §3.4 rule 2 for
/// `session.buffering`.
#[derive(Debug)]
pub struct ProgressWindow {
    window_ms: u64,
    opened_ns: Option<u64>,
}

impl ProgressWindow {
    /// A window of the given length.
    #[must_use]
    pub fn new(window_ms: u64) -> ProgressWindow {
        ProgressWindow {
            window_ms,
            opened_ns: None,
        }
    }

    /// The buffering-rate window ([`BUFFERING_WINDOW_MS`]).
    #[must_use]
    pub fn for_buffering() -> ProgressWindow {
        ProgressWindow::new(BUFFERING_WINDOW_MS)
    }

    /// Whether an emission is permitted now: the first emission of a
    /// window opens it; subsequent ones wait for it to elapse. A
    /// never-advancing source (frozen virtual time) therefore
    /// permits exactly one emission — deterministic in tests.
    #[must_use]
    pub fn permits(&mut self, now_ns: u64) -> bool {
        match self.opened_ns {
            None => {
                self.opened_ns = Some(now_ns);
                true
            }
            Some(opened) => {
                if now_ns.saturating_sub(opened) >= self.window_ms * 1_000_000 {
                    self.opened_ns = Some(now_ns);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Open the window at `now_ns` if it is not already open — the
    /// carrier for "the first pending record starts the clock". An
    /// already-open window is left alone (the elapsed check stays
    /// anchored to the window's original opening).
    pub(crate) fn open_if_closed(&mut self, now_ns: u64) {
        if self.opened_ns.is_none() {
            self.opened_ns = Some(now_ns);
        }
    }
}

/// The non-fatal `playback.error` coalescer (doc 05 §3.2): one
/// pending count per pipeline stage, flushed when the 1 s window
/// elapses or at a terminal point (so the last count is never
/// silently lost — the doc's window is a *rate* rule, not a loss
/// rule `[PROPOSAL]`).
#[derive(Debug)]
pub struct DamageCoalescer {
    window: ProgressWindow,
    /// Pending per-stage counts (demux and decode are the only v0
    /// recovery sites).
    demux: u64,
    decode: u64,
}

impl DamageCoalescer {
    /// A coalescer with the doc window.
    #[must_use]
    pub fn new() -> DamageCoalescer {
        DamageCoalescer {
            window: ProgressWindow::new(DAMAGE_WINDOW_MS),
            demux: 0,
            decode: 0,
        }
    }

    /// Record recovered units per stage. The first record with
    /// pending units opens the coalescing window at `now_ns` — the
    /// window measures from the first damage, not from the first
    /// flush *check* (the original draft's bug: `flush_if_due`
    /// opening the window on its first call made the very first
    /// check flush immediately regardless of elapsed time).
    pub fn record(&mut self, demux: u64, decode: u64, now_ns: u64) {
        let was_pending = self.is_pending();
        self.demux = self.demux.saturating_add(demux);
        self.decode = self.decode.saturating_add(decode);
        if !was_pending && self.is_pending() {
            self.window.open_if_closed(now_ns);
        }
    }

    /// Whether anything is pending.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.demux > 0 || self.decode > 0
    }

    /// Flush if the window has elapsed, answering the per-stage
    /// counts that became due (demux first — damage surfaces in
    /// pipeline order).
    #[must_use]
    pub fn flush_if_due(&mut self, now_ns: u64) -> Option<(u64, u64)> {
        if self.is_pending() && self.window.permits(now_ns) {
            return Some(self.take_pending());
        }
        None
    }

    /// Flush unconditionally (terminal points: teardown, ended) —
    /// the completeness half of the policy.
    #[must_use]
    pub fn flush_all(&mut self) -> Option<(u64, u64)> {
        if self.is_pending() {
            return Some(self.take_pending());
        }
        None
    }

    fn take_pending(&mut self) -> (u64, u64) {
        let out = (self.demux, self.decode);
        self.demux = 0;
        self.decode = 0;
        // Re-open the window: the next burst starts a fresh one.
        self.window = ProgressWindow::new(DAMAGE_WINDOW_MS);
        out
    }
}

impl Default for DamageCoalescer {
    fn default() -> DamageCoalescer {
        DamageCoalescer::new()
    }
}

/// The full per-session event policy (doc 05 §3.4), carried by
/// [`SessionConfig`](crate::session::SessionConfig).
///
/// `Debug` is **manual**: the injected [`MonotonicSource`] has no
/// `Debug` supertrait (and must not grow one just for this) — the
/// debug form reports the tick policy and the source's presence.
pub struct EventPolicy {
    /// The `timing.time-tick` interval policy.
    pub tick: TimeTickPolicy,
    /// The injected monotonic source every window reads (never
    /// ambient time — the media-clock injection doctrine).
    pub throttle: Arc<dyn MonotonicSource>,
}

impl fmt::Debug for EventPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventPolicy")
            .field("tick", &self.tick)
            .field("throttle", &"<injected MonotonicSource>")
            .finish()
    }
}

impl EventPolicy {
    /// The doc-default policy over real monotonic time.
    #[must_use]
    pub fn default_policy() -> EventPolicy {
        EventPolicy {
            tick: TimeTickPolicy::default_policy(),
            throttle: Arc::new(media_clock::InstantSource::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interval_is_floored_at_50ms() {
        assert_eq!(TimeTickPolicy::from_config_ms(250).interval_ms, 250);
        assert_eq!(
            TimeTickPolicy::from_config_ms(10).interval_ms,
            TIME_TICK_FLOOR_MS
        );
        assert_eq!(
            TimeTickPolicy::from_config_ms(0).interval_ms,
            TIME_TICK_FLOOR_MS
        );
        assert_eq!(
            TimeTickPolicy::from_config_ms(u32::MAX).interval_ms,
            u32::MAX
        );
    }

    #[test]
    fn a_fresh_policy_only_sets_the_baseline() {
        let policy = TimeTickPolicy::from_config_ms(250);
        // Doc 05 §3.2 lists no immediate tick on play: the first
        // baseline does not emit.
        assert!(!policy.is_due(None, 0));
        assert!(!policy.is_due(None, 10_000_000_000));
    }

    #[test]
    fn ticks_are_due_only_at_the_interval() {
        let policy = TimeTickPolicy::from_config_ms(250);
        assert!(!policy.is_due(Some(0), 249_999_999));
        assert!(policy.is_due(Some(0), 250_000_000));
        assert!(policy.is_due(Some(0), 1_000_000_000));
        // A position that went backwards (frozen clock, clamp) is
        // never due.
        assert!(!policy.is_due(Some(1_000_000_000), 500_000_000));
    }

    #[test]
    fn a_frozen_source_permits_exactly_one_progress_emission() {
        let mut window = ProgressWindow::for_buffering();
        assert!(window.permits(0));
        assert!(!window.permits(0));
        assert!(!window.permits(199_999_999));
        assert!(window.permits(200_000_000));
    }

    #[test]
    fn damage_coalesces_then_flushes_on_the_window() {
        let mut coalescer = DamageCoalescer::new();
        // The first record opens the window at t=0.
        coalescer.record(3, 0, 0);
        coalescer.record(0, 4, 400_000_000);
        assert!(coalescer.is_pending());
        // Inside the window: nothing due.
        assert_eq!(coalescer.flush_if_due(500_000_000), None);
        // At the window: both stages flush, demux first.
        assert_eq!(coalescer.flush_if_due(1_000_000_000), Some((3, 4)));
        assert!(!coalescer.is_pending());
        // A fresh burst starts a fresh window (t=1s).
        coalescer.record(1, 0, 1_000_000_000);
        assert_eq!(coalescer.flush_if_due(1_500_000_000), None);
        assert_eq!(coalescer.flush_all(), Some((1, 0)));
    }

    #[test]
    fn terminal_flush_answers_pending_counts_and_reopens() {
        let mut coalescer = DamageCoalescer::new();
        assert_eq!(coalescer.flush_all(), None);
        coalescer.record(7, 2, 0);
        assert_eq!(coalescer.flush_all(), Some((7, 2)));
        coalescer.record(0, 1, 0);
        // The reopened window makes an immediate second flush due
        // (terminal flushes are unconditional).
        assert_eq!(coalescer.flush_all(), Some((0, 1)));
        assert_eq!(coalescer.flush_all(), None);
    }
}
