//! Event-completeness policy (card 3.B4, doc 05 §3.4): the
//! throttling and coalescing rules the engine applies to its event
//! stream, as data-driven types — the emission points live in
//! [`crate::session`], the payload vocabulary in [`crate::event`].
//!
//! # The epoch-aware staleness stamp (card ED-FIX-4/M11, D-4)
//!
//! This module also owns the **drain-time staleness policy** of the
//! engine's output hub: [`StalenessStamp`] is the composite
//! (epoch, pipeline-counter) identity an event is judged against,
//! and [`DrainStaleness`] is the extension that derives it from an
//! [`EngineEvent`](crate::event::EngineEvent)'s correlation envelope
//! and applies the doc 05 §4.2 rule epoch-aware. The hub itself
//! lives in [`crate::session`] (sibling-owned); the policy is
//! defined here so the hub's check is a one-line consumer — the
//! exact `session.rs` wiring line is registered as a Lead-merge
//! follow-up in the Wave-1 worklog. The bare-counter comparison it
//! replaces is the trap [`crate::epoch`]'s own docs name: a pipeline
//! counter **resets each epoch**, so an old-epoch counter can
//! masquerade as fresh (or even compare newer) against the live
//! session's.
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

use crate::epoch::{GenerationPair, SessionEpoch};
use crate::event::{EngineEvent, EngineEventKind};

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

/// The composite (epoch, pipeline-counter) stamp the output hub
/// compares for staleness (card ED-FIX-4/M11, directive D-4 — the
/// 2026-10-22 directives-audit finding `session.rs` vs `epoch.rs`).
///
/// # Why a composite (the trap `epoch.rs` names)
///
/// A pipeline counter **resets each epoch** (doc 03 §5: minted from
/// 0 at open), so a bare counter has no identity across sessions:
/// epoch 2's generation 0 is *not* epoch 1's generation 0, and an
/// old-epoch counter can even compare **newer** than the live one
/// (epoch 1 ended at pipeline 5; epoch 2 is live at pipeline 0).
/// The output hub is engine-scoped — it outlives sessions, and an
/// undrained event of a closed session stays resident across the
/// open of its successor — so its liveness test must be the doc 03
/// §5 pair rule exactly: *different epoch OR different pipeline ⇒
/// stale*. An event from a superseded epoch is stale **by
/// construction**, whatever its counter says.
///
/// The stamp is two `u64` words: `Copy`, no allocation, two integer
/// compares per decision (directives D-1/D-2 — the drain path stays
/// flat whether the resident population is one event or a storm).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StalenessStamp {
    /// The owning session epoch (the wire `session`, doc 05 §4.2's
    /// explicit SessionId ↔ session-epoch mapping).
    epoch: SessionEpoch,
    /// The pipeline generation within the epoch (the wire `gen`).
    counter: u64,
}

impl StalenessStamp {
    /// A stamp from its two words.
    #[must_use]
    pub fn new(epoch: SessionEpoch, counter: u64) -> StalenessStamp {
        StalenessStamp { epoch, counter }
    }

    /// The stamp of a live pair — the drain-side "current" the hub
    /// judges residents against.
    #[must_use]
    pub fn from_pair(pair: GenerationPair) -> StalenessStamp {
        StalenessStamp {
            epoch: pair.epoch,
            counter: pair.pipeline.0,
        }
    }

    /// The owning session epoch.
    #[must_use]
    pub fn epoch(self) -> SessionEpoch {
        self.epoch
    }

    /// The pipeline counter within the epoch.
    #[must_use]
    pub fn counter(self) -> u64 {
        self.counter
    }

    /// The doc 03 §5 liveness rule, pair-wise: stale iff the epoch
    /// differs **or** the pipeline counter differs. The epoch arm
    /// runs first, so a superseded epoch is stale regardless of its
    /// counter — the bare-counter masquerade is dead by construction.
    ///
    /// For genuinely-emitted events the within-epoch arm is exactly
    /// "older": the session's minter only moves forward, so `!=` and
    /// `<` agree on every reachable state — `!=` is merely the
    /// fail-safe form for impossible ones (a same-epoch counter
    /// ahead of the live pair cannot be minted; if a bug ever
    /// produced one, suppression is the conservative answer).
    #[must_use]
    pub fn is_stale_against(self, current: GenerationPair) -> bool {
        self.epoch != current.epoch || self.counter != current.pipeline.0
    }
}

/// The epoch-aware drain policy for the event stream's counter
/// carriers (card ED-FIX-4/M11): the correlation envelope's
/// `session` + `generation` fields ARE the two halves of the
/// composite [`StalenessStamp`]. This trait is the seam that lets
/// the output hub compare the **pair** instead of the bare counter,
/// without editing the carrier's own definition — implemented here
/// for [`EngineEvent`], the hub's only event carrier.
///
/// The hub-side wiring is deliberately one line (the `drain_into`
/// staleness check consumes [`DrainStaleness::is_stale_at_drain`]);
/// this module owns the policy, `session.rs` owns the buffer it
/// polices.
pub trait DrainStaleness {
    /// The composite (epoch, counter) stamp of this carrier's
    /// correlation envelope.
    #[must_use]
    fn staleness_stamp(&self) -> StalenessStamp;

    /// Whether the hub suppresses this carrier at drain, judged
    /// against the live `current` pair: the doc 05 §4.2 rule made
    /// epoch-aware, with its one exemption kept **intra-epoch** — a
    /// `Seeked`/`SeekFailed` terminal of a completed generation
    /// survives later mints of the *same* epoch (doc 05 §4.3
    /// invariant 4: exactly one terminal per completed generation).
    ///
    /// Cross-epoch there is no exemption: an event of a superseded
    /// epoch is stale by construction (directive D-4 — no state,
    /// sound or visible, crosses a session boundary; a host that
    /// wants a dying session's last words drains before opening the
    /// next one, which every drain-per-poll host does).
    #[must_use]
    fn is_stale_at_drain(&self, current: &GenerationPair) -> bool;
}

impl DrainStaleness for EngineEvent {
    fn staleness_stamp(&self) -> StalenessStamp {
        // The envelope's session id IS the epoch (doc 05 §4.2's
        // mapping) and its generation IS the pipeline counter — the
        // wire form of the pair the engine compares.
        StalenessStamp {
            epoch: SessionEpoch(self.session.0),
            counter: self.generation.0,
        }
    }

    fn is_stale_at_drain(&self, current: &GenerationPair) -> bool {
        let stamp = self.staleness_stamp();
        if !stamp.is_stale_against(*current) {
            return false; // the live pair
        }
        if stamp.epoch != current.epoch {
            return true; // superseded epoch: stale BY CONSTRUCTION
        }
        // Superseded within the epoch: only the seek terminals
        // survive (doc 05 §4.3 invariant 4). Terminals are always
        // stamped with the live epoch (`emit_seek_terminal`), so
        // this arm is the exemption's whole reachable domain —
        // cross-epoch terminals exited above.
        !matches!(
            self.kind,
            EngineEventKind::Seeked { .. } | EngineEventKind::SeekFailed { .. }
        )
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

    // -----------------------------------------------------------------
    // M11: the epoch-aware staleness stamp (ED-FIX-4, D-4)
    // -----------------------------------------------------------------

    use crate::epoch::{PipelineGeneration, TokenMinter};
    use crate::seek::SeekFailure;
    use crate::session::{CommandReceipt, EngineOutput, ReceiptResult, SessionCommandKind};
    use crate::state::SessionState;
    use commands::envelope::{Generation, RequestId, SessionId};
    use testkit::Must;

    fn pair(epoch: u64, pipeline: u64) -> GenerationPair {
        GenerationPair {
            epoch: SessionEpoch(epoch),
            pipeline: PipelineGeneration(pipeline),
        }
    }

    fn stamped_event(epoch: u64, counter: u64, kind: EngineEventKind) -> EngineEvent {
        EngineEvent {
            seq: 0,
            session: SessionId(epoch),
            generation: Generation(counter),
            origin: None,
            kind,
        }
    }

    fn receipt(request: u64, epoch: u64) -> EngineOutput {
        EngineOutput::Receipt(CommandReceipt {
            request: RequestId(request),
            command: SessionCommandKind::Play,
            result: ReceiptResult::Completed {
                session: SessionId(epoch),
            },
        })
    }

    /// AC (a): a superseded epoch is stale **by construction**,
    /// whatever its counter says — the same-counter masquerade
    /// (today's bare `event.generation.0 < live` answers "fresh"),
    /// the lower-counter case it happens to catch, and the
    /// higher-counter leak it must not.
    #[test]
    fn cross_epoch_is_stale_by_construction_whatever_the_counter() {
        let current = pair(2, 3);
        // Same counter, different epoch — both masquerade directions
        // (older epoch, and defensively a newer one).
        for other_epoch in [1u64, 3] {
            assert!(
                StalenessStamp::new(SessionEpoch(other_epoch), 3).is_stale_against(current),
                "epoch {other_epoch} counter 3 must be stale against {current:?}"
            );
        }
        // Old-epoch counter below the live one (the only case the
        // bare `<` ever caught) — still stale.
        assert!(StalenessStamp::new(SessionEpoch(1), 0).is_stale_against(current));
        // Old-epoch counter ABOVE the live one — the leak shape:
        // `5 < 3` is false, the bare comparison waves it through.
        assert!(StalenessStamp::new(SessionEpoch(1), 5).is_stale_against(current));
        // The fresh-open shape: the closed session's gen-0 events
        // against the new session's gen-0.
        assert!(StalenessStamp::new(SessionEpoch(1), 0).is_stale_against(pair(2, 0)));
        // The drain decision agrees, for every kind — seek terminals
        // included: cross-epoch, the exemption does not apply.
        let kinds = [
            EngineEventKind::EndOfMedia,
            EngineEventKind::StateChanged {
                from: SessionState::Playing,
                to: SessionState::Paused,
            },
            EngineEventKind::TimeTick {
                position_ns: 0,
                fraction: 0.0,
            },
            EngineEventKind::Seeked {
                requested: 0,
                actual: 0,
            },
            EngineEventKind::SeekFailed {
                reason: SeekFailure::NotSeekable,
                position: 0,
            },
        ];
        for kind in kinds {
            assert!(
                stamped_event(1, 3, kind.clone()).is_stale_at_drain(&current),
                "an epoch-1 {kind:?} must be stale against {current:?}"
            );
        }
    }

    /// AC (b): the live pair passes; the near misses do not.
    #[test]
    fn fresh_epoch_fresh_counter_passes() {
        let current = pair(2, 0);
        assert!(!StalenessStamp::new(SessionEpoch(2), 0).is_stale_against(current));
        assert!(!stamped_event(2, 0, EngineEventKind::EndOfMedia).is_stale_at_drain(&current));
        // Near misses: same epoch, superseded counter…
        assert!(StalenessStamp::new(SessionEpoch(2), 1).is_stale_against(current));
        // …same counter, wrong epoch.
        assert!(StalenessStamp::new(SessionEpoch(1), 0).is_stale_against(current));
        // A same-epoch superseded non-terminal is suppressed at drain.
        assert!(stamped_event(2, 0, EngineEventKind::EndOfMedia).is_stale_at_drain(&pair(2, 1)));
    }

    /// AC (c): supersede (open the next epoch), then push a storm —
    /// every resident event of the superseded epoch is dropped and
    /// counted at drain, receipts survive untouched, and nothing is
    /// retained. The decision itself is two integer compares (see
    /// `tests/epoch_staleness_alloc.rs` for the measured
    /// zero-allocation proof — bounded work, not argued work).
    #[test]
    fn supersede_then_push_storm_all_dropped_counted_bounded() {
        let current = pair(2, 0);
        // The storm: 10_000 undelivered events of the superseded
        // epoch, mixed kinds — seek terminals included (the
        // cross-epoch arm must not let the exemption resurrect
        // them) — with receipts interleaved.
        let mut queue = std::collections::VecDeque::new();
        for seq in 0..10_000u64 {
            let kind = match seq % 4 {
                0 => EngineEventKind::EndOfMedia,
                1 => EngineEventKind::TimeTick {
                    position_ns: seq as i64,
                    fraction: 0.5,
                },
                2 => EngineEventKind::StateChanged {
                    from: SessionState::Playing,
                    to: SessionState::Paused,
                },
                _ => EngineEventKind::Seeked {
                    requested: 0,
                    actual: 0,
                },
            };
            let mut event = stamped_event(1, 3, kind);
            event.seq = seq;
            queue.push_back(EngineOutput::Event(event));
            if seq % 1_000 == 0 {
                queue.push_back(receipt(seq, 1));
            }
        }
        // The drain, in the hub's own shape (pop, decide, count).
        let mut suppressed_stale = 0u64;
        let mut delivered = Vec::new();
        while let Some(output) = queue.pop_front() {
            match output {
                EngineOutput::Receipt(_) => delivered.push(output),
                EngineOutput::Event(event) => {
                    if event.is_stale_at_drain(&current) {
                        suppressed_stale = suppressed_stale.saturating_add(1);
                    } else {
                        delivered.push(EngineOutput::Event(event));
                    }
                }
            }
        }
        assert_eq!(suppressed_stale, 10_000, "every storm event dropped");
        let receipts: Vec<_> = delivered
            .iter()
            .filter(|output| matches!(output, EngineOutput::Receipt(_)))
            .collect();
        assert_eq!(receipts.len(), 10, "receipts are never suppressed");
        assert_eq!(delivered.len(), 10, "nothing else survives the boundary");
    }

    /// The seek-terminal exemption is **intra-epoch only** (doc 05
    /// §4.3 invariant 4): a completed generation's terminal survives
    /// later mints of the same epoch; no terminal of another epoch
    /// does — the exemption cannot resurrect a dead session's words.
    #[test]
    fn seek_terminal_exemption_is_intra_epoch_only() {
        // Same epoch, superseded counter: the terminals survive…
        let current = pair(1, 2);
        assert!(
            !stamped_event(
                1,
                1,
                EngineEventKind::Seeked {
                    requested: 5,
                    actual: 5,
                }
            )
            .is_stale_at_drain(&current)
        );
        assert!(
            !stamped_event(
                1,
                1,
                EngineEventKind::SeekFailed {
                    reason: SeekFailure::NotSeekable,
                    position: 5,
                }
            )
            .is_stale_at_drain(&current)
        );
        // …while their same-generation siblings do not.
        assert!(
            stamped_event(
                1,
                1,
                EngineEventKind::TimeTick {
                    position_ns: 5,
                    fraction: 0.0
                }
            )
            .is_stale_at_drain(&current)
        );
        // Cross-epoch, the very same terminal is stale — even with a
        // counter that compares fresh against the live pair.
        assert!(
            stamped_event(
                2,
                2,
                EngineEventKind::Seeked {
                    requested: 5,
                    actual: 5,
                }
            )
            .is_stale_at_drain(&current)
        );
        // The real direction: the dead session's terminal against
        // the successor's live pair (same counter, new epoch).
        assert!(
            stamped_event(
                1,
                1,
                EngineEventKind::Seeked {
                    requested: 5,
                    actual: 5,
                }
            )
            .is_stale_at_drain(&pair(3, 1))
        );
    }

    /// D-5 (escape the echo chamber): the stamp's pair rule is
    /// differential-checked against [`crate::epoch::CancelToken`] —
    /// the independently-written carrier of the same doc 03 §5 rule
    /// (mint, publish, compare) — across an epoch × counter matrix.
    #[test]
    fn the_stamp_rule_agrees_with_the_cancel_token_oracle() {
        for current_epoch in [1u64, 2, 3] {
            for current_pipeline in 0u64..4 {
                let mut minter = TokenMinter::new(SessionEpoch(current_epoch));
                for _ in 0..current_pipeline {
                    minter.mint_pipeline().must();
                }
                let token = minter.token();
                let current = minter.pair();
                for epoch in [1u64, 2, 3] {
                    for counter in 0u64..4 {
                        let candidate = pair(epoch, counter);
                        assert_eq!(
                            StalenessStamp::from_pair(candidate).is_stale_against(current),
                            token.is_stale(&candidate),
                            "stamp vs token disagree: {candidate:?} against {current:?}"
                        );
                    }
                }
            }
        }
    }

    /// The stamp's own vocabulary: constructors, accessors, the
    /// envelope derivation, and the u64 boundary epochs (no wrapping
    /// arithmetic anywhere in the rule).
    #[test]
    fn stamps_round_trip_through_pairs_and_envelopes() {
        let stamp = StalenessStamp::new(SessionEpoch(7), 4);
        assert_eq!(stamp.epoch(), SessionEpoch(7));
        assert_eq!(stamp.counter(), 4);
        assert_eq!(StalenessStamp::from_pair(pair(7, 4)), stamp);
        // The envelope derivation: session IS the epoch, generation
        // IS the counter (doc 05 §4.2's mapping).
        assert_eq!(
            stamped_event(7, 4, EngineEventKind::EndOfMedia).staleness_stamp(),
            stamp
        );
        // Boundaries: max epochs/counters never wrap into freshness.
        let max_stamp = StalenessStamp::new(SessionEpoch(u64::MAX), u64::MAX);
        assert!(!max_stamp.is_stale_against(pair(u64::MAX, u64::MAX)));
        assert!(max_stamp.is_stale_against(pair(1, u64::MAX)));
        assert!(StalenessStamp::new(SessionEpoch(1), u64::MAX).is_stale_against(pair(1, 0)));
    }
}
