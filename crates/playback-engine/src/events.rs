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
//! | `tracks.changed` | **emitted** (3.A3) | `{trackKind, selected}` — the class-A set-*-track terminal under the switch's own gen; the class-S subtitle selection at the unbumped gen (no engine-side track-loss trigger exists in v0 — the demuxer cannot report a disappearing track) |
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
//! 12 of 23 rows are engine-emitted today — the 12 ids above
//! (card 3.A3 moved `tracks.changed` into the emitted bucket after
//! the 11-row arithmetic was first written; the map below is the
//! machine-checked count). 11 rows are honestly owned by later
//! cards. The catalog itself (61/23) is frozen data (decision
//! D-017) — this card changes emission completeness only.
//!
//! The machine-checked twin of this table is [`CATALOG_EMISSIONS`]
//! (the completeness map — asserted bijective against the generated
//! `EventId::ALL`, the source of truth), cross-checked by the
//! catalog-side 23-row ledger
//! (`crates/commands/tests/catalog_emission_ledger.rs` — still
//! carrying the pre-3.A3 11/12 split for `tracks.changed`; a
//! handoff for its owner) and proven live by the contract suites
//! (`crates/playback-engine/tests/contract_suite.rs` and
//! `tests/events_contract.rs`), which drive every "emitted" row out
//! of the real engine.

use std::collections::VecDeque;
use std::fmt;
use std::num::NonZeroUsize;
use std::sync::Arc;

use commands::envelope::{Generation, RequestId};
use commands::{EventId, EventKind, PlaybackState, WireEvent};
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

    /// The window's length in milliseconds (the rate cap's period).
    #[must_use]
    pub fn window_ms(&self) -> u64 {
        self.window_ms
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
    /// terminal (`Seeked`/`SeekFailed`, and since card 3.A3
    /// `TrackSelectionChanged`) of a completed generation survives
    /// later mints of the *same* epoch (doc 05 §4.3 invariant 4:
    /// exactly one terminal per completed generation).
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
        // Superseded within the epoch: only the terminals survive
        // (doc 05 §4.3 invariant 4) — the seek pair and, since card
        // 3.A3, the track-selection terminal (a switch completed
        // under its own generation must not be swallowed by a
        // follow-up mint that raced the completion). Terminals are
        // always stamped with the live epoch (`emit_terminal`), so
        // this arm is the exemption's whole reachable domain —
        // cross-epoch terminals exited above.
        !matches!(
            self.kind,
            EngineEventKind::Seeked { .. }
                | EngineEventKind::SeekFailed { .. }
                | EngineEventKind::TrackSelectionChanged { .. }
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

// ---------------------------------------------------------------------------
// The completeness map (doc 05 §3.2 — 23 rows, D-017-frozen)
// ---------------------------------------------------------------------------

/// The engine-side emission status of one catalog row (doc 05 §3.2).
///
/// "Emitted" means proven: the contract suites drive the row out of
/// the real engine with its doc-05 payload. Anything else is
/// [`EmissionStatus::WiredLater`] with the owning card named — honest
/// incompleteness, never silence (a row with no trigger and no owner
/// is a bug the map's bijection test surfaces).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmissionStatus {
    /// The engine emits this row today; the contract suites
    /// (`tests/contract_suite.rs`, `tests/events_contract.rs`) prove
    /// the payload live.
    Emitted,
    /// No engine-side trigger exists yet: the subsystem that owns the
    /// row is not in the engine. `owner` names the card/phase that
    /// will close it.
    WiredLater {
        /// The owning card / phase note.
        owner: &'static str,
    },
}

impl EmissionStatus {
    /// Whether the row is engine-emitted today.
    #[must_use]
    pub fn is_emitted(self) -> bool {
        matches!(self, EmissionStatus::Emitted)
    }
}

/// One row of the engine-side completeness map.
#[derive(Debug, Clone, Copy)]
pub struct CatalogEmission {
    /// The catalog row (a D-017-frozen id).
    pub id: EventId,
    /// Its engine-side status.
    pub status: EmissionStatus,
}

/// How many rows the engine emits today (post-3.A3 arithmetic —
/// `tracks.changed` joined the emitted bucket with the track-switch
/// terminal; the count is asserted against the map below).
pub const ENGINE_EMITTED_ROWS: usize = 12;

/// The engine-side completeness map: every doc 05 §3.2 row, in table
/// order, classified. The unit tests assert it is a bijection onto
/// `EventId::ALL` (the generated catalog — the source of truth), so a
/// catalog change that does not update the map fails the build rather
/// than drifting silently.
pub const CATALOG_EMISSIONS: &[CatalogEmission] = &[
    // session (§3.2)
    CatalogEmission {
        id: EventId::SessionStateChanged,
        status: EmissionStatus::Emitted,
    },
    CatalogEmission {
        id: EventId::SessionOpeningProgress,
        status: EmissionStatus::WiredLater {
            owner: "streaming opens are Phase 6; local opens are synchronous",
        },
    },
    CatalogEmission {
        id: EventId::SessionBuffering,
        status: EmissionStatus::Emitted,
    },
    // timing (§3.2)
    CatalogEmission {
        id: EventId::TimingTimeTick,
        status: EmissionStatus::Emitted,
    },
    CatalogEmission {
        id: EventId::TimingDurationKnown,
        status: EmissionStatus::Emitted,
    },
    CatalogEmission {
        id: EventId::TimingSeekCompleted,
        status: EmissionStatus::Emitted,
    },
    CatalogEmission {
        id: EventId::TimingSeekFailed,
        status: EmissionStatus::Emitted,
    },
    // tracks (§3.2)
    CatalogEmission {
        id: EventId::TracksListReady,
        status: EmissionStatus::Emitted,
    },
    CatalogEmission {
        id: EventId::TracksChanged,
        status: EmissionStatus::Emitted,
    },
    // playback (§3.2)
    CatalogEmission {
        id: EventId::PlaybackEndOfTrack,
        status: EmissionStatus::Emitted,
    },
    CatalogEmission {
        id: EventId::PlaybackEndOfMedia,
        status: EmissionStatus::Emitted,
    },
    CatalogEmission {
        id: EventId::PlaybackError,
        status: EmissionStatus::Emitted,
    },
    CatalogEmission {
        id: EventId::PlaybackRateChanged,
        status: EmissionStatus::WiredLater {
            owner: "3.B1 (rate support; no set-rate engine input exists)",
        },
    },
    // audio (§3.2)
    CatalogEmission {
        id: EventId::AudioDeviceLost,
        status: EmissionStatus::WiredLater {
            owner: "2.C3/7.C1 (device observation; the null device cannot be lost)",
        },
    },
    CatalogEmission {
        id: EventId::AudioDeviceChanged,
        status: EmissionStatus::WiredLater {
            owner: "2.C3/7.C1 (same)",
        },
    },
    // video (§3.2)
    CatalogEmission {
        id: EventId::VideoScreenshotSaved,
        status: EmissionStatus::WiredLater {
            owner: "renderer/host work (the sink seam never writes files)",
        },
    },
    // playlist (§3.2)
    CatalogEmission {
        id: EventId::PlaylistChanged,
        status: EmissionStatus::WiredLater {
            owner: "no playlist model in the engine (Phase 4, 4.D1)",
        },
    },
    // config (§3.2)
    CatalogEmission {
        id: EventId::ConfigSettingChanged,
        status: EmissionStatus::WiredLater {
            owner: "settings writes are host/config-layer work (doc 07, Phase 4)",
        },
    },
    // plugins (§3.2)
    CatalogEmission {
        id: EventId::PluginsLoaded,
        status: EmissionStatus::WiredLater {
            owner: "plugin registry (doc 06)",
        },
    },
    CatalogEmission {
        id: EventId::PluginsFailed,
        status: EmissionStatus::WiredLater {
            owner: "plugin registry (doc 06)",
        },
    },
    CatalogEmission {
        id: EventId::PluginsCrashed,
        status: EmissionStatus::WiredLater {
            owner: "sandboxed WASM host (plugin-host-wasm, Phase 7)",
        },
    },
    // system (§3.2)
    CatalogEmission {
        id: EventId::SystemLog,
        status: EmissionStatus::WiredLater {
            owner: "no logging bridge; the §3.4 burst-bound drop refinement activates here",
        },
    },
    // media (§3.2)
    CatalogEmission {
        id: EventId::MediaInfoReady,
        status: EmissionStatus::Emitted,
    },
];

/// The status of one catalog row, by id.
#[must_use]
pub fn emission_status(id: EventId) -> Option<EmissionStatus> {
    CATALOG_EMISSIONS
        .iter()
        .find(|row| row.id == id)
        .map(|row| row.status)
}

/// Whether the engine emits this catalog row today.
#[must_use]
pub fn is_engine_emitted(id: EventId) -> bool {
    emission_status(id).is_some_and(EmissionStatus::is_emitted)
}

/// The catalog id of a wire event payload — the total
/// `EventKind` ↔ `EventId` bridge the completeness map and the
/// ordering guard speak.
///
/// The match is compiler-exhaustive **by design**: a new catalog row
/// breaks this function at compile time, forcing the map (and the
/// guard's review) to grow with the catalog — D-017's frozen count is
/// a reviewed change, never a silent drift.
#[must_use]
pub fn wire_event_id(kind: &EventKind) -> EventId {
    match kind {
        EventKind::SessionStateChanged { .. } => EventId::SessionStateChanged,
        EventKind::SessionOpeningProgress { .. } => EventId::SessionOpeningProgress,
        EventKind::SessionBuffering { .. } => EventId::SessionBuffering,
        EventKind::TimingTimeTick { .. } => EventId::TimingTimeTick,
        EventKind::TimingDurationKnown { .. } => EventId::TimingDurationKnown,
        EventKind::TimingSeekCompleted { .. } => EventId::TimingSeekCompleted,
        EventKind::TimingSeekFailed { .. } => EventId::TimingSeekFailed,
        EventKind::TracksListReady { .. } => EventId::TracksListReady,
        EventKind::TracksChanged { .. } => EventId::TracksChanged,
        EventKind::PlaybackEndOfTrack { .. } => EventId::PlaybackEndOfTrack,
        EventKind::PlaybackEndOfMedia => EventId::PlaybackEndOfMedia,
        EventKind::PlaybackError { .. } => EventId::PlaybackError,
        EventKind::PlaybackRateChanged { .. } => EventId::PlaybackRateChanged,
        EventKind::AudioDeviceLost { .. } => EventId::AudioDeviceLost,
        EventKind::AudioDeviceChanged { .. } => EventId::AudioDeviceChanged,
        EventKind::VideoScreenshotSaved { .. } => EventId::VideoScreenshotSaved,
        EventKind::PlaylistChanged { .. } => EventId::PlaylistChanged,
        EventKind::ConfigSettingChanged { .. } => EventId::ConfigSettingChanged,
        EventKind::PluginsLoaded { .. } => EventId::PluginsLoaded,
        EventKind::PluginsFailed { .. } => EventId::PluginsFailed,
        EventKind::PluginsCrashed { .. } => EventId::PluginsCrashed,
        EventKind::SystemLog { .. } => EventId::SystemLog,
        EventKind::MediaInfoReady { .. } => EventId::MediaInfoReady,
    }
}

// ---------------------------------------------------------------------------
// The ordering guard (doc 05 §3.3 + §4.3 — the debug_assert form)
// ---------------------------------------------------------------------------

/// A named doc 05 §3.3/§4.3 ordering violation the
/// [`OrderingGuard`] can catch. The names quote the doc's own rule
/// language so a failure reads as a spec citation, not a stack trace.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderingViolation {
    /// §4.3.1 — `seq` is engine-global, strictly increasing; a host
    /// observes gaps (filtered/dropped), never regressions.
    SeqRegression { observed: u64, last: u64 },
    /// §3.3 — the transition is not a legal wire edge (and the trail
    /// must be contiguous: every transition is emitted, C8 echoes
    /// included, so `from` must equal the engine's current state).
    IllegalTransition {
        from: PlaybackState,
        to: PlaybackState,
        engine_at: PlaybackState,
    },
    /// §3.2 — `timing.time-tick` reports "continuous position while
    /// `playing`" (plus the documented immediate ticks). A tick in any
    /// other state — the PositionUpdate-after-Stopped class — is a
    /// violation.
    TickOutsidePlaying { at: PlaybackState },
    /// §4.3.4 — `duration-known` is delivered before any `time-tick`
    /// that assumes it (a nonzero fraction; the engine's honest floor
    /// is `0.0` while the duration is unknown).
    TickAssumesUnknownDuration,
    /// §3.3 — `state-changed → error` is always accompanied by its
    /// fatal `playback.error`, **ordered first**.
    MissingFatalBeforeError,
    /// §3.3 — a fatal `playback.error` is always followed by
    /// `state-changed → error`; no other transition may intervene and
    /// none may remain dangling at the end of the stream.
    FatalWithoutErrorState { at: PlaybackState },
    /// §4.3.4 — `end-of-media` precedes `state-changed → ended`.
    EndOfMediaAfterEnded,
    /// §3.2 — `end-of-track` fires for a limb that ends *while the
    /// media continues*; after `end-of-media` the last limb's end IS
    /// the media's end (no second end-of-track).
    EndOfTrackAfterMedia,
    /// §4.3.4 — `tracks.list-ready` is delivered before the first
    /// `state-changed → playing` of a session.
    FirstPlayingWithoutTracksList,
    /// §3.2 — `media.info-ready` is the snapshot *just before* first
    /// `playing`.
    FirstPlayingWithoutMediaInfo,
    /// §3.2 — a buffering episode "must open with `0.0`".
    EpisodeOpenedWithoutZero { offered: f32 },
    /// §3.2 — a buffering episode "must close with `1.0`": leaving the
    /// buffering/paused-hold family with the episode still open is a
    /// violation.
    EpisodeUnclosedAtExit {
        from: PlaybackState,
        to: PlaybackState,
    },
    /// §4.3.4 / finish — an episode that never closed strands a `0.0`
    /// on the wire.
    EpisodeNeverClosed { at: PlaybackState },
    /// §3.4.2 — progress events are monotonic within one episode.
    NonMonotonicProgress { previous: f32, offered: f32 },
    /// §3.2 — progress is `0.0..=1.0` (NaN included: a NaN payload is
    /// out of bounds, not merely unordered).
    ProgressOutOfBounds { offered: f32 },
    /// §3.2 — a seek terminal carries the completing generation (the
    /// completion-matching `gen` of §1.4 C3).
    TerminalWithoutGeneration,
    /// §4.3.4 — exactly one terminal seek event per completed seek
    /// generation.
    DuplicateSeekTerminal { generation: Generation },
    /// §4.3.2 — the response for request R precedes every event
    /// attributable to R (one ordered channel, causal order).
    EventBeforeItsReceipt { origin: RequestId },
}

/// How many recent seek terminals the guard remembers for the
/// duplicate check (doc 05 §4.3.4). Bounded memory (D-2): a sliding
/// window, O(window) per terminal — terminals are rare (one per
/// seek), so the constant is the honest bound.
const SEEK_TERMINAL_WINDOW: usize = 64;

/// How many recent request ids the guard remembers as answered
/// (receipt-seen). Bounded memory (D-2); a sliding window.
const ANSWERED_REQUEST_WINDOW: usize = 128;

/// The per-session wire-ordering policy checker (doc 05 §3.3 + §4.3):
/// feed it one session's ordered output stream — every wire event,
/// plus receipts via [`OrderingGuard::observe_receipt`] — and it
/// answers whether each observation keeps the documented ordering
/// invariants. [`OrderingGuard::finish`] checks the dangling ones
/// (an episode left open, a fatal never reaching `error`).
///
/// This is the policy layer's *guard* half: the emission sites
/// ( [`crate::session`] ) are the production carriers; hosts and the
/// contract suites run the guard over what actually crossed the wire
/// — the differential check that keeps the policy and the engine
/// honest about each other (D-5). The [`debug_assert_wire_ordered`]
/// form wraps [`OrderingGuard::observe`] for engine-internal
/// self-checks: zero cost in release builds, a named violation in
/// debug builds.
///
/// Cross-session streams are out of scope by the doc itself (§4.3.3:
/// cross-session order is unspecified) — one guard per session.
#[derive(Debug)]
pub struct OrderingGuard {
    /// The last observed engine-global `seq` (§4.3.1).
    last_seq: Option<u64>,
    /// The session's current playback state (every transition is
    /// emitted, so the trail reconstructs the state exactly).
    state: PlaybackState,
    /// Whether the first `state-changed → playing` has been seen.
    playing_seen: bool,
    /// Whether `tracks.list-ready` has been seen (§4.3.4).
    list_ready_seen: bool,
    /// Whether `media.info-ready` has been seen (§3.2).
    info_ready_seen: bool,
    /// Whether `timing.duration-known` has been seen (§4.3.4).
    duration_known: bool,
    /// Whether a buffering episode is currently open (§3.2 bookends).
    episode_open: bool,
    /// The last in-episode progress value (§3.4.2 monotonicity).
    episode_progress: f32,
    /// A fatal `playback.error` awaiting its `state-changed → error`.
    pending_fatal: bool,
    /// Whether `playback.end-of-media` has been seen (§4.3.4).
    eom_seen: bool,
    /// Whether the immediately-preceding event was a seek completion
    /// (the documented immediate tick after `seek-completed` is the
    /// one legal tick outside `playing`).
    prev_was_seek_completion: bool,
    /// Recent seek-terminal generations (bounded, §4.3.4).
    seek_terminal_gens: Vec<Generation>,
    /// Recent request ids answered by a receipt (bounded, §4.3.2).
    answered_requests: Vec<RequestId>,
}

impl OrderingGuard {
    /// A guard for a fresh session (state `idle`, nothing seen).
    #[must_use]
    pub fn new() -> OrderingGuard {
        OrderingGuard {
            last_seq: None,
            state: PlaybackState::Idle,
            playing_seen: false,
            list_ready_seen: false,
            info_ready_seen: false,
            duration_known: false,
            episode_open: false,
            episode_progress: 0.0,
            pending_fatal: false,
            eom_seen: false,
            prev_was_seek_completion: false,
            seek_terminal_gens: Vec::new(),
            answered_requests: Vec::new(),
        }
    }

    /// Observe one wire event; `Err` names the violated rule.
    ///
    /// Rows without engine-side rules yet (rate, devices, playlist,
    /// config, plugins, log, `tracks.changed`, `opening-progress`)
    /// are observed without judgement — their rules land with their
    /// wiring, tracked by [`CATALOG_EMISSIONS`].
    pub fn observe(&mut self, event: &WireEvent) -> Result<(), OrderingViolation> {
        // §4.3.1 — engine-global, strictly increasing. Gaps are legal
        // (filtered or dropped events); regressions never are.
        if let Some(last) = self.last_seq
            && event.seq <= last
        {
            return Err(OrderingViolation::SeqRegression {
                observed: event.seq,
                last,
            });
        }
        self.last_seq = Some(event.seq);
        // §4.3.2 — a caused event must not precede its own receipt.
        if let Some(origin) = event.origin
            && !self.answered_requests.contains(&origin)
        {
            return Err(OrderingViolation::EventBeforeItsReceipt { origin });
        }
        let verdict = match &event.kind {
            EventKind::SessionStateChanged { from, to } => self.observe_state_change(*from, *to),
            EventKind::TimingTimeTick { position, .. } => self.observe_tick(*position),
            EventKind::TimingDurationKnown { .. } => {
                self.duration_known = true;
                Ok(())
            }
            EventKind::TracksListReady { .. } => {
                self.list_ready_seen = true;
                Ok(())
            }
            EventKind::MediaInfoReady { .. } => {
                self.info_ready_seen = true;
                Ok(())
            }
            EventKind::SessionBuffering { progress } => self.observe_buffering(*progress),
            EventKind::PlaybackEndOfMedia => {
                self.eom_seen = true;
                Ok(())
            }
            EventKind::PlaybackEndOfTrack { .. } => {
                if self.eom_seen {
                    Err(OrderingViolation::EndOfTrackAfterMedia)
                } else {
                    Ok(())
                }
            }
            EventKind::PlaybackError { fatal, .. } => {
                if *fatal {
                    self.pending_fatal = true;
                }
                Ok(())
            }
            EventKind::TimingSeekCompleted { .. } | EventKind::TimingSeekFailed { .. } => {
                self.observe_seek_terminal(event.generation)
            }
            _ => Ok(()),
        };
        self.prev_was_seek_completion = matches!(event.kind, EventKind::TimingSeekCompleted { .. });
        verdict
    }

    /// Observe one command receipt on the same ordered channel
    /// (§4.3.2): the response for request `R` positions causally
    /// before every event attributable to `R`.
    pub fn observe_receipt(&mut self, request: RequestId) {
        self.answered_requests.push(request);
        if self.answered_requests.len() > ANSWERED_REQUEST_WINDOW {
            let drop = self.answered_requests.len() - ANSWERED_REQUEST_WINDOW;
            self.answered_requests.drain(0..drop);
        }
    }

    /// The dangling invariants (call at end-of-stream): a buffering
    /// episode must have closed with `1.0` (§3.2), and a fatal
    /// `playback.error` must have reached `state-changed → error`
    /// (§3.3).
    pub fn finish(&self) -> Result<(), OrderingViolation> {
        if self.pending_fatal {
            return Err(OrderingViolation::FatalWithoutErrorState { at: self.state });
        }
        if self.episode_open {
            return Err(OrderingViolation::EpisodeNeverClosed { at: self.state });
        }
        Ok(())
    }

    fn observe_state_change(
        &mut self,
        from: PlaybackState,
        to: PlaybackState,
    ) -> Result<(), OrderingViolation> {
        if from != self.state || !from.can_transition_to(to) {
            return Err(OrderingViolation::IllegalTransition {
                from,
                to,
                engine_at: self.state,
            });
        }
        // §3.3 — a fatal is always followed by `state-changed →
        // error`, and by nothing else.
        if self.pending_fatal && to != PlaybackState::Error {
            return Err(OrderingViolation::FatalWithoutErrorState { at: self.state });
        }
        // §3.2 — the episode's `1.0` exit bookend precedes every exit
        // of the buffering/paused-hold family. Moves *within* the
        // family (the buffered pause's hold, and the mid-episode
        // seek/switch self-transitions) keep the episode open — the
        // engine's card 3.C1 discipline.
        let in_family = |state: PlaybackState| {
            matches!(state, PlaybackState::Buffering | PlaybackState::Paused)
        };
        if in_family(from) && !in_family(to) && self.episode_open {
            return Err(OrderingViolation::EpisodeUnclosedAtExit { from, to });
        }
        if to == PlaybackState::Error {
            if !self.pending_fatal {
                return Err(OrderingViolation::MissingFatalBeforeError);
            }
            self.pending_fatal = false;
        }
        // §4.3.4 — `end-of-media` precedes `state-changed → ended`.
        if to == PlaybackState::Ended && !self.eom_seen {
            return Err(OrderingViolation::EndOfMediaAfterEnded);
        }
        // §4.3.4 / §3.2 — the open facts precede the first `playing`.
        if to == PlaybackState::Playing && !self.playing_seen {
            if !self.list_ready_seen {
                return Err(OrderingViolation::FirstPlayingWithoutTracksList);
            }
            if !self.info_ready_seen {
                return Err(OrderingViolation::FirstPlayingWithoutMediaInfo);
            }
            self.playing_seen = true;
        }
        self.state = to;
        Ok(())
    }

    fn observe_tick(&mut self, position: f32) -> Result<(), OrderingViolation> {
        // §3.2 — the continuous stream runs "while `playing`"; the
        // documented immediate ticks (seek completion, pause entry,
        // rate change) are the exceptions. Pause entry fires while
        // still `playing`; seek completion is tracked by its
        // immediately-preceding terminal; rate changes do not exist
        // in the engine yet (the map's row).
        let legal_state = self.state == PlaybackState::Playing || self.prev_was_seek_completion;
        if !legal_state {
            return Err(OrderingViolation::TickOutsidePlaying { at: self.state });
        }
        // §4.3.4 — a tick that assumes the duration (a nonzero
        // fraction) must follow `duration-known`.
        if position > 0.0 && !self.duration_known {
            return Err(OrderingViolation::TickAssumesUnknownDuration);
        }
        Ok(())
    }

    fn observe_buffering(&mut self, progress: f32) -> Result<(), OrderingViolation> {
        // §3.2 — progress is `0.0..=1.0`, NaN included in the ban.
        if progress.is_nan() || !(0.0..=1.0).contains(&progress) {
            return Err(OrderingViolation::ProgressOutOfBounds { offered: progress });
        }
        if !self.episode_open {
            // The open bookend.
            if progress != 0.0 {
                return Err(OrderingViolation::EpisodeOpenedWithoutZero { offered: progress });
            }
            self.episode_open = true;
            self.episode_progress = 0.0;
            return Ok(());
        }
        if progress == 1.0 {
            // The close bookend.
            self.episode_open = false;
            return Ok(());
        }
        // §3.4.2 — monotonic within one episode.
        if progress <= self.episode_progress {
            return Err(OrderingViolation::NonMonotonicProgress {
                previous: self.episode_progress,
                offered: progress,
            });
        }
        self.episode_progress = progress;
        Ok(())
    }

    fn observe_seek_terminal(
        &mut self,
        generation: Option<Generation>,
    ) -> Result<(), OrderingViolation> {
        let Some(generation) = generation else {
            return Err(OrderingViolation::TerminalWithoutGeneration);
        };
        if self.seek_terminal_gens.contains(&generation) {
            return Err(OrderingViolation::DuplicateSeekTerminal { generation });
        }
        self.seek_terminal_gens.push(generation);
        if self.seek_terminal_gens.len() > SEEK_TERMINAL_WINDOW {
            let drop = self.seek_terminal_gens.len() - SEEK_TERMINAL_WINDOW;
            self.seek_terminal_gens.drain(0..drop);
        }
        Ok(())
    }
}

impl Default for OrderingGuard {
    fn default() -> OrderingGuard {
        OrderingGuard::new()
    }
}

/// The violation as a one-line spec citation (the `must()`-friendly
/// form; hosts log this verbatim).
impl fmt::Display for OrderingViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderingViolation::SeqRegression { observed, last } => {
                write!(f, "doc 05 §4.3.1: seq regressed to {observed} after {last}")
            }
            OrderingViolation::IllegalTransition {
                from,
                to,
                engine_at,
            } => write!(
                f,
                "doc 05 §3.3: illegal wire transition {from:?}->{to:?} (engine at {engine_at:?})"
            ),
            OrderingViolation::TickOutsidePlaying { at } => {
                write!(f, "doc 05 §3.2: a time-tick outside `playing` (at {at:?})")
            }
            OrderingViolation::TickAssumesUnknownDuration => write!(
                f,
                "doc 05 §4.3.4: a time-tick assumed a duration never delivered"
            ),
            OrderingViolation::MissingFatalBeforeError => write!(
                f,
                "doc 05 §3.3: state-changed -> error without its fatal playback.error first"
            ),
            OrderingViolation::FatalWithoutErrorState { at } => write!(
                f,
                "doc 05 §3.3: a fatal playback.error never reached state-changed -> error (at {at:?})"
            ),
            OrderingViolation::EndOfMediaAfterEnded => write!(
                f,
                "doc 05 §4.3.4: state-changed -> ended before end-of-media"
            ),
            OrderingViolation::EndOfTrackAfterMedia => write!(
                f,
                "doc 05 §3.2: end-of-track after end-of-media (the last limb's end IS the media's end)"
            ),
            OrderingViolation::FirstPlayingWithoutTracksList => write!(
                f,
                "doc 05 §4.3.4: first state-changed -> playing before tracks.list-ready"
            ),
            OrderingViolation::FirstPlayingWithoutMediaInfo => write!(
                f,
                "doc 05 §3.2: first state-changed -> playing before media.info-ready"
            ),
            OrderingViolation::EpisodeOpenedWithoutZero { offered } => write!(
                f,
                "doc 05 §3.2: a buffering episode opened with {offered} (must open with 0.0)"
            ),
            OrderingViolation::EpisodeUnclosedAtExit { from, to } => write!(
                f,
                "doc 05 §3.2: left {from:?}->{to:?} with the buffering episode open (must close with 1.0)"
            ),
            OrderingViolation::EpisodeNeverClosed { at } => write!(
                f,
                "doc 05 §3.2: the stream ended with the buffering episode open (at {at:?})"
            ),
            OrderingViolation::NonMonotonicProgress { previous, offered } => write!(
                f,
                "doc 05 §3.4.2: buffering progress {offered} after {previous} (monotonic within one episode)"
            ),
            OrderingViolation::ProgressOutOfBounds { offered } => write!(
                f,
                "doc 05 §3.2: buffering progress {offered} outside 0.0..=1.0"
            ),
            OrderingViolation::TerminalWithoutGeneration => write!(
                f,
                "doc 05 §3.2: a seek terminal without its completing generation"
            ),
            OrderingViolation::DuplicateSeekTerminal { generation } => write!(
                f,
                "doc 05 §4.3.4: a second seek terminal for generation {generation:?}"
            ),
            OrderingViolation::EventBeforeItsReceipt { origin } => write!(
                f,
                "doc 05 §4.3.2: an event caused by {origin:?} preceded its own receipt"
            ),
        }
    }
}

/// The `debug_assert` form of the ordering guard (card 3.B4): a
/// one-line self-check emission sites and hosts can sprinkle over
/// their streams — a named violation with a panic in debug builds,
/// compiled away to nothing in release (the state tracking itself is
/// a handful of integer compares; the *assertion* is the zero-cost
/// half).
///
/// The guard still tracks state in release builds (it is a
/// [`Result`]-returning validator too); only the panic disappears.
#[inline]
pub fn debug_assert_wire_ordered(guard: &mut OrderingGuard, event: &WireEvent) {
    let verdict = guard.observe(event);
    if verdict.is_err() {
        debug_assert!(
            false,
            "doc 05 §3/§4 ordering violation at seq {}: {:?}",
            event.seq, verdict
        );
    }
}

// ---------------------------------------------------------------------------
// The latest-value coalescer (doc 05 §3.4 rule 2 — drop-coalesce
// that keeps the LATEST value)
// ---------------------------------------------------------------------------

/// The drop-coalesce throttle window for progress streams (doc 05
/// §3.4 rule 2: "progress events are monotonic within one episode
/// and coalesced to their rate cap").
///
/// Semantics, made precise — the part the doc names but does not
/// define:
///
/// - **Monotonic**: an offer that does not strictly exceed the last
///   *emitted* value is dropped (counted) — the emitted stream is
///   monotonic by construction, whatever the input does.
/// - **Drop-coalesce, latest wins**: an offer the rate window denies
///   is dropped *from the stream* but retained as **pending**; a
///   later offer overwrites it. When the window reopens (or at a
///   terminal [`LatestCoalescer::flush_now`]), the **latest** offered
///   value is the one that emits — intermediate values never cross
///   the wire, and the newest fact is never lost to the window.
///
/// `[PROPOSAL]` — the doc's "coalesced to their rate cap" admits a
/// lossier reading (drop until the next sample arrives); the
/// keep-the-latest form is this policy layer's choice, and the
/// engine's in-episode `session.buffering` path
/// ([`crate::session`], the `EmissionState` fields) can adopt it as
/// a one-structure swap — recorded as a handoff, not wired here
/// (scope discipline: the emission site is sibling-owned).
///
/// All windows read an injected monotonic source, never ambient time
/// (the media-clock injection doctrine; see the module docs).
#[derive(Debug)]
pub struct LatestCoalescer {
    window: ProgressWindow,
    last_emitted: Option<f32>,
    pending: Option<f32>,
    dropped_stale: u64,
}

impl LatestCoalescer {
    /// A coalescer over a window of `window_ms` milliseconds.
    #[must_use]
    pub fn new(window_ms: u64) -> LatestCoalescer {
        LatestCoalescer {
            window: ProgressWindow::new(window_ms),
            last_emitted: None,
            pending: None,
            dropped_stale: 0,
        }
    }

    /// The `session.buffering` rate window ([`BUFFERING_WINDOW_MS`],
    /// the ≤ 5 Hz cap).
    #[must_use]
    pub fn for_buffering() -> LatestCoalescer {
        LatestCoalescer::new(BUFFERING_WINDOW_MS)
    }

    /// Offer the newest sample. `Some(value)` when an emission is due
    /// now (the window permits it and the value is monotonic);
    /// otherwise the value is retained as pending (latest wins) or
    /// dropped as stale (non-monotonic, counted).
    pub fn offer(&mut self, value: f32, now_ns: u64) -> Option<f32> {
        if let Some(last) = self.last_emitted
            && value <= last
        {
            self.dropped_stale = self.dropped_stale.saturating_add(1);
            return None;
        }
        if self.window.permits(now_ns) {
            self.last_emitted = Some(value);
            // A fresh emission subsumes anything still pending — the
            // value just emitted is by definition at least as new.
            self.pending = None;
            return Some(value);
        }
        // Drop-coalesce: the denied value is retained; the LATEST
        // wins. `max` keeps the retained set monotonic even for a
        // non-monotonic offer burst (in the engine's domain — queue
        // fill during a refill — the climb is monotonic, so latest
        // and max coincide; this is the fail-safe form).
        self.pending = Some(match self.pending {
            Some(pending) => pending.max(value),
            None => value,
        });
        None
    }

    /// Flush the pending latest value if the rate window has elapsed
    /// (call per tick; a closed window with nothing pending is left
    /// untouched, so idle calls never push the window forward).
    pub fn flush_if_due(&mut self, now_ns: u64) -> Option<f32> {
        // Short-circuit: `permits` is never consulted with nothing
        // pending (an idle caller must not open a window).
        if self.pending.is_some() && self.window.permits(now_ns) {
            return self.flush_now();
        }
        None
    }

    /// Flush unconditionally (terminal points: episode exit, end of
    /// media — the doc's window is a *rate* rule, not a loss rule).
    pub fn flush_now(&mut self) -> Option<f32> {
        let value = self.pending?;
        self.pending = None;
        self.last_emitted = Some(value);
        Some(value)
    }

    /// Reset for a fresh episode: a fresh window, no memory of the
    /// previous climb (the `0.0` bookend starts a new monotonic
    /// episode). The stale-drop counter is lifetime observability
    /// and survives.
    pub fn reset(&mut self) {
        self.window = ProgressWindow::new(self.window_ms());
        self.last_emitted = None;
        self.pending = None;
    }

    /// The retained latest value, if any.
    #[must_use]
    pub fn pending(&self) -> Option<f32> {
        self.pending
    }

    /// Offers dropped as non-monotonic (lifetime observability).
    #[must_use]
    pub fn dropped_stale(&self) -> u64 {
        self.dropped_stale
    }

    /// The rate window's length in milliseconds.
    #[must_use]
    pub fn window_ms(&self) -> u64 {
        self.window.window_ms()
    }
}

// ---------------------------------------------------------------------------
// The bounded queue (engineering directive D-2 — the policy twin)
// ---------------------------------------------------------------------------

/// The bounded-queue policy (D-2: bounded memory everywhere): a
/// fixed-capacity FIFO that drops the **oldest** item at capacity and
/// counts the drop — the same overflow policy the engine's output
/// hub applies to live events
/// ([`crate::session::OUTBOX_CAPACITY`]; receipts pinned, ticks
/// replaced — the hub's two refinements stay there, where the
/// channel's semantics live).
///
/// This is the policy twin for the engine's *edges*: hosts, test
/// collectors and future bridges that accumulate wire events must be
/// as bounded as the engine feeding them — an unbounded collector on
/// a bounded channel is a leak with extra steps. The contract suite
/// (`tests/events_contract.rs`) collects its streams through this
/// queue, so the policy is proven on every scenario it runs.
///
/// [`BoundedEventQueue::drain_into`] is the reuse API (D-1): the
/// caller keeps one `Vec` alive across drains; the queue itself
/// never allocates after construction.
pub struct BoundedEventQueue<T> {
    queue: VecDeque<T>,
    capacity: usize,
    dropped: u64,
}

impl<T> BoundedEventQueue<T> {
    /// A queue of exactly `capacity` live items (nonzero — a
    /// zero-capacity queue is a drop-everything sink, not a policy).
    #[must_use]
    pub fn new(capacity: NonZeroUsize) -> BoundedEventQueue<T> {
        BoundedEventQueue {
            queue: VecDeque::new(),
            capacity: capacity.get(),
            dropped: 0,
        }
    }

    /// Append one item; at capacity the oldest live item is dropped
    /// and counted (O(1), no reallocation — the deque's ring).
    pub fn push(&mut self, item: T) {
        if self.queue.len() >= self.capacity {
            self.queue.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        self.queue.push_back(item);
    }

    /// Drain everything live into a caller-owned buffer (reuse API,
    /// D-1 — no per-drain allocation), leaving the queue empty.
    pub fn drain_into(&mut self, out: &mut Vec<T>) {
        out.extend(self.queue.drain(..));
    }

    /// Live items right now (always ≤ the capacity, whatever the
    /// push history — the D-2 structural bound).
    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Whether no live items remain.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// The live-item bound.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Items dropped for capacity over the queue's lifetime.
    #[must_use]
    pub fn dropped_total(&self) -> u64 {
        self.dropped
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

    /// Card 3.A3: the track-selection terminal joins the intra-epoch
    /// exemption (a switch completed under its own generation must
    /// not be swallowed by a follow-up mint that raced the
    /// completion — doc 05 §4.3 invariant 4), and cross-epoch it is
    /// stale like every other event of a dead session.
    #[test]
    fn track_selection_terminal_joins_the_intra_epoch_exemption() {
        let kind = EngineEventKind::TrackSelectionChanged {
            track_kind: media_types::MediaKind::Audio,
            selected: Some(media_types::TrackId::new(2)),
        };
        // Same epoch, superseded counter: the terminal survives.
        assert!(!stamped_event(1, 1, kind.clone()).is_stale_at_drain(&pair(1, 2)));
        // Cross-epoch, fresh-looking counter: stale by construction.
        assert!(stamped_event(2, 2, kind).is_stale_at_drain(&pair(1, 2)));
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

    // -----------------------------------------------------------------
    // W5-05 (card 3.B4 rebuild): the completeness map, the ordering
    // guard, the latest-value coalescer, the bounded queue
    // -----------------------------------------------------------------

    /// The map is a bijection onto the generated catalog (the source
    /// of truth): same count, same order, every id exactly once, and
    /// the arithmetic is the module-doc claim (12 + 11 = 23, D-017).
    #[test]
    fn the_completeness_map_partitions_the_catalog_exactly() {
        assert_eq!(
            CATALOG_EMISSIONS.len(),
            EventId::ALL.len(),
            "the map must carry one row per catalog id"
        );
        assert_eq!(EventId::ALL.len(), 23, "D-017: the frozen count");
        // Table order mirrors the catalog's own order.
        for (row, id) in CATALOG_EMISSIONS.iter().zip(EventId::ALL) {
            assert_eq!(row.id, *id, "the map must mirror doc 05 §3.2 order");
        }
        let emitted = CATALOG_EMISSIONS
            .iter()
            .filter(|row| row.status.is_emitted())
            .count();
        let later = CATALOG_EMISSIONS.len() - emitted;
        assert_eq!(emitted, ENGINE_EMITTED_ROWS, "the emitted count");
        assert_eq!(later, 11, "the honest later bucket");
        assert_eq!(emitted + later, 23, "12 emitted + 11 later = 23");
        // Every later row names its owner (a trigger-less row with no
        // owner is silence, not honesty).
        for row in CATALOG_EMISSIONS {
            if let EmissionStatus::WiredLater { owner } = row.status
                && owner.is_empty()
            {
                unreachable!("{} carries an empty owner note", row.id.as_str());
            }
        }
        // The lookups agree with the map.
        assert!(is_engine_emitted(EventId::SessionStateChanged));
        assert!(!is_engine_emitted(EventId::PlaylistChanged));
        assert_eq!(
            emission_status(EventId::PlaybackError),
            Some(EmissionStatus::Emitted)
        );
        assert!(emission_status(EventId::SystemLog).is_some());
    }

    /// The `EventKind` ↔ `EventId` bridge: a representative sample
    /// maps onto the right ids, and every emitted row's sample lands
    /// in the emitted bucket (the full 23-sample bijection proof is
    /// the contract suite's job — `tests/events_contract.rs`).
    #[test]
    fn the_wire_id_bridge_agrees_with_the_map() {
        let samples: [(EventKind, EventId, bool); 8] = [
            (
                EventKind::SessionStateChanged {
                    from: PlaybackState::Idle,
                    to: PlaybackState::Opening,
                },
                EventId::SessionStateChanged,
                true,
            ),
            (
                EventKind::SessionBuffering { progress: 0.5 },
                EventId::SessionBuffering,
                true,
            ),
            (
                EventKind::TimingTimeTick {
                    time: commands::catalog::types::Timestamp(0),
                    position: 0.0,
                },
                EventId::TimingTimeTick,
                true,
            ),
            (
                EventKind::TimingSeekFailed {
                    error: commands::CommandError::new(
                        commands::Stage::Control,
                        commands::ErrorCode::Unsupported,
                        "nope",
                    ),
                },
                EventId::TimingSeekFailed,
                true,
            ),
            (
                EventKind::TracksChanged {
                    track_kind: commands::catalog::types::TrackChangeKind::Audio,
                    selected: None,
                },
                EventId::TracksChanged,
                true,
            ),
            (
                EventKind::PlaybackEndOfMedia,
                EventId::PlaybackEndOfMedia,
                true,
            ),
            (
                EventKind::PlaybackRateChanged { rate: 2.0 },
                EventId::PlaybackRateChanged,
                false,
            ),
            (
                EventKind::SystemLog {
                    level: commands::catalog::types::LogLevel::Info,
                    target: "lumen::engine".to_owned(), // lumen-allow-literal: tracing-target fixture value — the log target under test
                    message: "line".to_owned(),
                },
                EventId::SystemLog,
                false,
            ),
        ];
        for (kind, id, emitted) in samples {
            assert_eq!(wire_event_id(&kind), id);
            assert_eq!(
                is_engine_emitted(id),
                emitted,
                "{id} must sit in the bucket the map says"
            );
        }
    }

    /// One guard helper: a bare wire event (seq 1, 2, … — no session
    /// or origin unless the scenario says otherwise).
    fn wire(seq: u64, kind: EventKind) -> WireEvent {
        WireEvent {
            seq,
            session: None,
            generation: None,
            origin: None,
            kind,
        }
    }

    fn state_changed(seq: u64, from: PlaybackState, to: PlaybackState) -> WireEvent {
        wire(seq, EventKind::SessionStateChanged { from, to })
    }

    /// `tracks.list-ready` with an empty list (the §4.3.4 open fact,
    /// minimal form).
    fn list_ready(seq: u64) -> WireEvent {
        wire(seq, EventKind::TracksListReady { tracks: Vec::new() })
    }

    /// `media.info-ready`, minimal snapshot (the §3.2 open fact).
    fn info_ready(seq: u64) -> WireEvent {
        wire(
            seq,
            EventKind::MediaInfoReady {
                info: commands::catalog::types::MediaInfo {
                    source: commands::catalog::types::SourceSpec::Url {
                        url: "fixture://x".to_owned(),
                    },
                    container: None,
                    title: None,
                    duration: commands::catalog::types::Duration::Unknown,
                    seekable: true,
                    is_live: false,
                    metadata: std::collections::BTreeMap::new(),
                    tracks: Vec::new(),
                    chapters: Vec::new(),
                },
            },
        )
    }

    /// The minimal **legal** open-to-playing prefix: both §3.2/§4.3.4
    /// open facts (`tracks.list-ready`, `media.info-ready`) precede
    /// the first `state-changed → playing` — the prefix every
    /// violation scenario builds on top of (seq 1..=5).
    fn open_to_playing() -> Vec<WireEvent> {
        vec![
            state_changed(1, PlaybackState::Idle, PlaybackState::Opening),
            list_ready(2),
            state_changed(3, PlaybackState::Opening, PlaybackState::Buffering),
            info_ready(4),
            state_changed(5, PlaybackState::Buffering, PlaybackState::Playing),
        ]
    }

    /// The guard walks a legal happy-path prefix without a single
    /// complaint (the full-session proofs live in the contract suite;
    /// this pins the guard's own machinery on a hand-built stream).
    #[test]
    fn the_guard_accepts_a_legal_hand_built_stream() {
        let mut guard = OrderingGuard::new();
        // The engine's real open-to-pause order: the 0.0 bookend
        // precedes its state change, the 1.0 exit precedes the primed
        // transition, the open facts precede the first `playing`, and
        // the pause-entry tick is the last fact of the playing era.
        for event in [
            state_changed(1, PlaybackState::Idle, PlaybackState::Opening),
            wire(2, EventKind::TracksListReady { tracks: Vec::new() }),
            wire(3, EventKind::SessionBuffering { progress: 0.0 }),
            state_changed(4, PlaybackState::Opening, PlaybackState::Buffering),
            wire(
                5,
                EventKind::TimingDurationKnown {
                    duration: commands::catalog::types::Timestamp(1),
                },
            ),
            wire(
                6,
                EventKind::MediaInfoReady {
                    info: commands::catalog::types::MediaInfo {
                        source: commands::catalog::types::SourceSpec::Url {
                            url: "fixture://x".to_owned(),
                        },
                        container: None,
                        title: None,
                        duration: commands::catalog::types::Duration::Unknown,
                        seekable: true,
                        is_live: false,
                        metadata: std::collections::BTreeMap::new(),
                        tracks: Vec::new(),
                        chapters: Vec::new(),
                    },
                },
            ),
            wire(7, EventKind::SessionBuffering { progress: 1.0 }),
            state_changed(8, PlaybackState::Buffering, PlaybackState::Playing),
            wire(
                9,
                EventKind::TimingTimeTick {
                    time: commands::catalog::types::Timestamp(250_000),
                    position: 0.25,
                },
            ),
            state_changed(10, PlaybackState::Playing, PlaybackState::Paused),
        ] {
            guard.observe(&event).must();
        }
        guard.finish().must();
    }

    /// The guard's negative matrix, hand-built: every violation name
    /// is reachable and names its rule (the contract suite plants the
    /// same violations against the real engine's stream shape).
    #[test]
    fn the_guard_catches_the_planted_violations() {
        use PlaybackState as S;
        let seq_regression = || {
            let mut guard = OrderingGuard::new();
            let a = state_changed(5, S::Idle, S::Opening);
            let b = state_changed(5, S::Opening, S::Buffering);
            guard.observe(&a).must();
            guard.observe(&b)
        };
        assert_eq!(
            seq_regression().err(),
            Some(OrderingViolation::SeqRegression {
                observed: 5,
                last: 5
            })
        );

        let illegal = || {
            let mut guard = OrderingGuard::new();
            guard.observe(&state_changed(1, S::Stopped, S::Playing))
        };
        assert_eq!(
            illegal().err(),
            Some(OrderingViolation::IllegalTransition {
                from: S::Stopped,
                to: S::Playing,
                engine_at: S::Idle,
            })
        );

        // The card's named audit shape: a PositionUpdate (time-tick)
        // after Stopped.
        let tick_after_stop = || {
            let mut guard = OrderingGuard::new();
            let mut events = open_to_playing();
            events.push(state_changed(6, S::Playing, S::Stopped));
            for event in events {
                guard.observe(&event).must();
            }
            guard.observe(&wire(
                7,
                EventKind::TimingTimeTick {
                    time: commands::catalog::types::Timestamp(1),
                    position: 0.0,
                },
            ))
        };
        assert_eq!(
            tick_after_stop().err(),
            Some(OrderingViolation::TickOutsidePlaying { at: S::Stopped })
        );

        let tick_before_duration = || {
            let mut guard = OrderingGuard::new();
            for event in open_to_playing() {
                guard.observe(&event).must();
            }
            guard.observe(&wire(
                6,
                EventKind::TimingTimeTick {
                    time: commands::catalog::types::Timestamp(1),
                    position: 0.5,
                },
            ))
        };
        assert_eq!(
            tick_before_duration().err(),
            Some(OrderingViolation::TickAssumesUnknownDuration)
        );

        let error_without_fatal = || {
            let mut guard = OrderingGuard::new();
            for event in [
                state_changed(1, S::Idle, S::Opening),
                state_changed(2, S::Opening, S::Buffering),
            ] {
                guard.observe(&event).must();
            }
            guard.observe(&state_changed(3, S::Buffering, S::Error))
        };
        assert_eq!(
            error_without_fatal().err(),
            Some(OrderingViolation::MissingFatalBeforeError)
        );

        let fatal_then_wrong_state = || {
            let mut guard = OrderingGuard::new();
            guard.observe(&state_changed(1, S::Idle, S::Opening)).must();
            guard
                .observe(&wire(
                    2,
                    EventKind::PlaybackError {
                        stage: commands::Stage::Demux,
                        code: commands::ErrorCode::Corrupt,
                        message: "boom".to_owned(),
                        details: std::collections::BTreeMap::new(),
                        fatal: true,
                    },
                ))
                .must();
            guard.observe(&state_changed(3, S::Opening, S::Stopped))
        };
        assert_eq!(
            fatal_then_wrong_state().err(),
            Some(OrderingViolation::FatalWithoutErrorState { at: S::Opening })
        );

        let ended_without_eom = || {
            let mut guard = OrderingGuard::new();
            for event in open_to_playing() {
                guard.observe(&event).must();
            }
            guard.observe(&state_changed(6, S::Playing, S::Ended))
        };
        assert_eq!(
            ended_without_eom().err(),
            Some(OrderingViolation::EndOfMediaAfterEnded)
        );

        let eot_after_eom = || {
            let mut guard = OrderingGuard::new();
            let mut events = open_to_playing();
            events.push(wire(6, EventKind::PlaybackEndOfMedia));
            for event in events {
                guard.observe(&event).must();
            }
            guard.observe(&wire(
                7,
                EventKind::PlaybackEndOfTrack {
                    track_kind: commands::catalog::types::TrackKind::Audio,
                    id: commands::catalog::types::TrackId(1),
                },
            ))
        };
        assert_eq!(
            eot_after_eom().err(),
            Some(OrderingViolation::EndOfTrackAfterMedia)
        );

        let playing_without_tracks = || {
            let mut guard = OrderingGuard::new();
            for event in [
                state_changed(1, S::Idle, S::Opening),
                state_changed(2, S::Opening, S::Buffering),
            ] {
                guard.observe(&event).must();
            }
            guard.observe(&state_changed(3, S::Buffering, S::Playing))
        };
        assert_eq!(
            playing_without_tracks().err(),
            Some(OrderingViolation::FirstPlayingWithoutTracksList)
        );
        let playing_without_info = || {
            let mut guard = OrderingGuard::new();
            guard.observe(&state_changed(1, S::Idle, S::Opening)).must();
            guard
                .observe(&wire(2, EventKind::TracksListReady { tracks: Vec::new() }))
                .must();
            guard
                .observe(&state_changed(3, S::Opening, S::Buffering))
                .must();
            guard.observe(&state_changed(4, S::Buffering, S::Playing))
        };
        assert_eq!(
            playing_without_info().err(),
            Some(OrderingViolation::FirstPlayingWithoutMediaInfo)
        );

        let episode_bad_open = || {
            let mut guard = OrderingGuard::new();
            guard.observe(&wire(1, EventKind::SessionBuffering { progress: 0.3 }))
        };
        assert_eq!(
            episode_bad_open().err(),
            Some(OrderingViolation::EpisodeOpenedWithoutZero { offered: 0.3 })
        );

        let episode_unclosed_exit = || {
            let mut guard = OrderingGuard::new();
            for event in [
                state_changed(1, S::Idle, S::Opening),
                state_changed(2, S::Opening, S::Buffering),
                wire(3, EventKind::SessionBuffering { progress: 0.0 }),
            ] {
                guard.observe(&event).must();
            }
            guard.observe(&state_changed(4, S::Buffering, S::Stopped))
        };
        assert_eq!(
            episode_unclosed_exit().err(),
            Some(OrderingViolation::EpisodeUnclosedAtExit {
                from: S::Buffering,
                to: S::Stopped,
            })
        );

        let non_monotonic = || {
            let mut guard = OrderingGuard::new();
            guard
                .observe(&wire(1, EventKind::SessionBuffering { progress: 0.0 }))
                .must();
            guard
                .observe(&wire(2, EventKind::SessionBuffering { progress: 0.4 }))
                .must();
            guard.observe(&wire(3, EventKind::SessionBuffering { progress: 0.2 }))
        };
        assert_eq!(
            non_monotonic().err(),
            Some(OrderingViolation::NonMonotonicProgress {
                previous: 0.4,
                offered: 0.2,
            })
        );

        let out_of_bounds = || {
            let mut guard = OrderingGuard::new();
            guard.observe(&wire(1, EventKind::SessionBuffering { progress: 1.5 }))
        };
        assert_eq!(
            out_of_bounds().err(),
            Some(OrderingViolation::ProgressOutOfBounds { offered: 1.5 })
        );
        let nan_progress = || {
            let mut guard = OrderingGuard::new();
            guard.observe(&wire(1, EventKind::SessionBuffering { progress: f32::NAN }))
        };
        // NaN breaks `PartialEq` — the variant name is the assertion.
        assert!(matches!(
            nan_progress().err(),
            Some(OrderingViolation::ProgressOutOfBounds { .. })
        ));

        let terminal_without_gen = || {
            let mut guard = OrderingGuard::new();
            guard.observe(&wire(
                1,
                EventKind::TimingSeekCompleted {
                    position: commands::catalog::types::Timestamp(0),
                    seek_kind: commands::catalog::types::SeekKind::Command,
                },
            ))
        };
        assert_eq!(
            terminal_without_gen().err(),
            Some(OrderingViolation::TerminalWithoutGeneration)
        );

        let duplicate_terminal = || {
            let mut guard = OrderingGuard::new();
            let terminal = EventKind::TimingSeekCompleted {
                position: commands::catalog::types::Timestamp(0),
                seek_kind: commands::catalog::types::SeekKind::Command,
            };
            let mut event = wire(1, terminal.clone());
            event.generation = Some(Generation(3));
            guard.observe(&event).must();
            let mut event = wire(2, terminal);
            event.generation = Some(Generation(3));
            guard.observe(&event)
        };
        assert_eq!(
            duplicate_terminal().err(),
            Some(OrderingViolation::DuplicateSeekTerminal {
                generation: Generation(3)
            })
        );

        let event_before_receipt = || {
            let mut guard = OrderingGuard::new();
            let mut event = wire(1, EventKind::PlaybackEndOfMedia);
            event.origin = Some(RequestId(7));
            guard.observe(&event)
        };
        assert_eq!(
            event_before_receipt().err(),
            Some(OrderingViolation::EventBeforeItsReceipt {
                origin: RequestId(7)
            })
        );

        let episode_never_closed = || {
            let mut guard = OrderingGuard::new();
            guard
                .observe(&wire(1, EventKind::SessionBuffering { progress: 0.0 }))
                .must();
            guard.finish()
        };
        assert_eq!(
            episode_never_closed().err(),
            Some(OrderingViolation::EpisodeNeverClosed { at: S::Idle })
        );
    }

    /// Drop-coalesce keeps the LATEST value (doc 05 §3.4 rule 2):
    /// a frozen window swallows a climb, the window's opening emits
    /// the newest sample — and the rate cap holds over a long
    /// virtual timeline without ever losing the last fact.
    #[test]
    fn the_latest_coalescer_keeps_the_newest_value() {
        let mut coalescer = LatestCoalescer::for_buffering();
        assert_eq!(coalescer.window_ms(), BUFFERING_WINDOW_MS);
        // The frozen window (t=0): the first offer opens and emits,
        // the climb is retained, latest wins.
        assert_eq!(coalescer.offer(0.1, 0), Some(0.1));
        assert_eq!(coalescer.offer(0.4, 0), None);
        assert_eq!(coalescer.offer(0.9, 0), None);
        assert_eq!(coalescer.pending(), Some(0.9), "the LATEST value");
        // Inside the window: no flush.
        assert_eq!(coalescer.flush_if_due(100_000_000), None);
        // The window opens at t=200 ms: the latest value emits.
        assert_eq!(coalescer.flush_if_due(200_000_000), Some(0.9));
        assert_eq!(coalescer.pending(), None);
        // A non-monotonic offer is dropped and counted (§3.4.2).
        assert_eq!(coalescer.offer(0.5, 250_000_000), None);
        assert_eq!(coalescer.dropped_stale(), 1);
        // The rate cap over a virtual second: one emission per
        // elapsed window, monotonic, and the terminal flush never
        // loses the last fact.
        let mut emissions = Vec::new();
        for step in 0..20_u64 {
            let t = 300_000_000_u64 + step * 50_000_000;
            let value = 0.9 + f32::from(step as u16) * 0.005;
            if let Some(emitted) = coalescer.offer(value, t) {
                emissions.push(emitted);
            }
            if let Some(emitted) = coalescer.flush_if_due(t) {
                emissions.push(emitted);
            }
        }
        assert!(
            emissions.len() <= 5,
            "≤ 5 Hz over the virtual second: {emissions:?}"
        );
        assert!(
            emissions.windows(2).all(|pair| pair[0] < pair[1]),
            "monotonic: {emissions:?}"
        );
        let last = coalescer.flush_now();
        assert_eq!(last, Some(0.9 + f32::from(19_u16) * 0.005));
        assert_eq!(coalescer.flush_now(), None);
        // Reset: a fresh episode forgets the climb.
        coalescer.reset();
        assert_eq!(coalescer.pending(), None);
        assert_eq!(coalescer.offer(0.05, 0), Some(0.05));
    }

    /// The bounded queue's structural guarantee (D-2): whatever the
    /// push history, the live population never exceeds the capacity;
    /// drops are oldest-first and counted; drains preserve order and
    /// reuse the caller's buffer.
    #[test]
    fn the_bounded_queue_stays_bounded_under_any_push_count() {
        let capacity = NonZeroUsize::new(256).must();
        let mut queue = BoundedEventQueue::new(capacity);
        assert_eq!(queue.capacity(), 256);
        for seq in 0..10_000_u64 {
            queue.push(wire(seq, EventKind::PlaybackEndOfMedia));
            assert!(
                queue.len() <= 256,
                "the structural bound must hold mid-storm too"
            );
        }
        assert_eq!(queue.len(), 256);
        assert_eq!(queue.dropped_total(), 10_000 - 256);
        let mut drained = Vec::new();
        queue.drain_into(&mut drained);
        assert_eq!(drained.len(), 256);
        assert!(queue.is_empty());
        // Oldest-first drops: the survivors are the NEWEST 256, in
        // order.
        let seqs: Vec<u64> = drained.iter().map(|event| event.seq).collect();
        assert_eq!(seqs.first(), Some(&9_744));
        assert_eq!(seqs.last(), Some(&9_999));
        // The reuse API appends into the caller's buffer.
        queue.push(wire(10_000, EventKind::PlaybackEndOfMedia));
        queue.drain_into(&mut drained);
        assert_eq!(drained.len(), 257);
        assert_eq!(drained.last().map(|event| event.seq), Some(10_000));
    }
}
