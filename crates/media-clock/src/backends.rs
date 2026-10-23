//! Clock backends: master switching, loss fallback, video steering,
//! the Ended latch (card 3.B2; doc 03 §4.7/§4.10; doc 13 card 3.B2).
//!
//! # What this module adds over 3.B1's audio-master clock
//!
//! The 3.B1 [`MediaClock`](crate::MediaClock) is single-master: it is
//! born audio-master (or system-monotonic for audio-less media) and
//! its truth source never changes. Real playback needs more:
//!
//! - **Video-master mode** — video frames drive the clock (the
//!   audio-less-with-video case, and the user-chosen mode). Between
//!   frames the clock interpolates on the injected monotonic source;
//!   each presented frame steers it through
//!   [`MediaClock::observe_video`]. Audio resyncs to video within
//!   bounded limits — the existing ±1000 ppm
//!   [`DriftCorrection`](crate::DriftCorrection) budget on the audio
//!   governor; no new resync mechanism exists or is needed here.
//! - **External-master mode** — an external time source (a
//!   WASAPI/CoreAudio-style device clock domain, future platform
//!   adapters) drives the clock, modeled as an injectable
//!   [`MonotonicSource`](crate::MonotonicSource) exactly like the
//!   3.B1 wall fallback; only the truthful label differs.
//! - **Master-loss fallback policy** — when the current master's
//!   stream ends or disappears mid-playback, the clock demotes along
//!   a **deterministic** chain (card 3.B2):
//!   audio-master → video-master → free-running wall drift, with a
//!   wired external clock slotted after audio `[PROPOSAL: an external
//!   device clock outlives its stream and is drift-free against the
//!   device]`. Every switch carries the position (no fabricated
//!   seamlessness: the [`MasterChanged`] trail is the receipt),
//!   bumps the anchor epoch, and records a typed event in a bounded
//!   ring the engine drains and forwards as doc 03 §4.6's
//!   `ClockMasterChanged`.
//! - **Ended-state seek-back** — after playback reaches the end, the
//!   clock is latched ([`MediaClock::mark_ended`]): the position
//!   freezes at the end forever — a wall-driven master would
//!   otherwise run past the media's end, which would be a fabricated
//!   position. A seek re-arms it via [`MediaClock::seek_back`]:
//!   **paused at the new position**, never free-running; play
//!   continues only on the engine's explicit `resume`.
//!
//! # Determinism
//!
//! The fallback plan is a pure function of (wired backends, lost
//! kinds, current master) — [`plan_fallback`]. The same loss script
//! over two identically-wired clocks yields identical master
//! sequences, events and epochs (unit-tested). No wall-clock reading
//! participates in the *decision*; wall time only ever advances the
//! *position*.
//!
//! # Bounded, allocation-free (D-1/D-2)
//!
//! Steady-state tick paths (`now`/`try_now`/`snapshot`/
//! `observe_video`/`take_master_change`, and the switches
//! themselves) allocate nothing — measured by the counting global
//! allocator in `tests/zero_alloc_backends.rs`, not argued. The
//! event trail is a fixed 8-slot ring; an overflow increments a
//! lost-count instead of growing (the receipt, never silent loss).
//! The backend state is a fixed handful of integers plus the
//! construction-time `Arc`s.
//!
//! # Threading
//!
//! Same discipline as 3.B1: one short internal lock, critical
//! sections are source reads plus checked arithmetic, never held
//! across I/O. The card 3.B2 change: the truth source lives **inside**
//! the lock so a master switch swaps it atomically with the
//! re-anchor it performs.
//!
//! # Engine wiring (handoff note)
//!
//! playback-engine currently constructs single-master clocks and
//! drives seeks through `reanchor` + `set_generation`; adopting the
//! backend surface means constructing via
//! [`MediaClock::with_backends`], reporting stream losses through
//! [`MediaClock::on_master_lost`], steering per presented frame in
//! video-master mode, calling [`MediaClock::mark_ended`] at EOF and
//! [`MediaClock::seek_back`] for the play-from-ended restart. That
//! rewiring is a playback-engine card; this crate's 3.B1 API stays
//! byte-compatible.

use std::sync::Arc;

use media_types::GenerationId;
use media_types::time::{Rate, TimeError, Timestamp};

use crate::clock::{
    Core, Extrapolation, MediaClock, MonotonicKind, Source, adopt_anchor, clock_state,
    now_locked, position_nanos, read_source, snapshot_locked,
};
use crate::mapping::NANO_BASE;
use crate::source::{MonotonicSource, SampleCounterSource};

/// The default video-snap bound `[PROPOSAL]`: one 25 fps frame
/// duration (40 ms). A presented frame within this bound of the
/// clock's extrapolated position steers the anchor invisibly (the
/// monotone clamp absorbs it); beyond it the steer is a typed
/// discontinuity ([`VideoSteer::Jumped`]) with an epoch bump. The
/// engine may configure per-media values (frame-duration-derived).
pub const DEFAULT_VIDEO_SNAP_NS: u64 = 40_000_000;

/// The master-change event ring capacity (bounded memory, D-2).
/// Master changes are policy events — O(1) per session in normal
/// playback; a consumer that lets 8 pile up un-drained gets the
/// lost-count receipt, never unbounded growth.
const MASTER_EVENT_CAPACITY: usize = 8;

/// Which stream a master follows — the loss-report vocabulary the
/// engine uses (card 3.B2). `Free`/`System` have no stream and can
/// never be lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MasterStream {
    /// The audio output's device stream ended/disappeared.
    Audio,
    /// The video limb's stream ended/disappeared.
    Video,
    /// The external clock domain went away (device removed).
    External,
}

impl MasterStream {
    /// The stable wire-style name (doc 05 vocabulary discipline).
    pub fn as_str(self) -> &'static str {
        match self {
            MasterStream::Audio => "audio",
            MasterStream::Video => "video",
            MasterStream::External => "external",
        }
    }
}

/// The runtime master vocabulary the fallback planner orders. The
/// engine-facing labels are [`crate::ClockMaster`]; this is the
/// crate-internal planning set (plus `System`, the audio-less
/// construction-time provenance that is never a fallback target).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MasterKind {
    Audio,
    External,
    Video,
    Free,
    System,
}

impl MasterKind {
    /// The public label for this kind (`System` and `Free` are both
    /// wall-driven; they differ only in provenance, which the event
    /// trail records).
    fn label(self) -> crate::ClockMaster {
        match self {
            MasterKind::Audio => crate::ClockMaster::AudioOutput,
            MasterKind::External => crate::ClockMaster::External,
            MasterKind::Video => crate::ClockMaster::VideoOutput,
            MasterKind::Free => crate::ClockMaster::FreeRunning,
            MasterKind::System => crate::ClockMaster::SystemMonotonic,
        }
    }

    /// The loss bit for this kind (`System`/`Free` are not losable).
    fn loss_bit(self) -> u8 {
        match self {
            MasterKind::Audio => LOST_AUDIO,
            MasterKind::External => LOST_EXTERNAL,
            MasterKind::Video => LOST_VIDEO,
            MasterKind::Free | MasterKind::System => 0,
        }
    }
}

impl From<MasterStream> for MasterKind {
    fn from(stream: MasterStream) -> MasterKind {
        match stream {
            MasterStream::Audio => MasterKind::Audio,
            MasterStream::Video => MasterKind::Video,
            MasterStream::External => MasterKind::External,
        }
    }
}

/// The planner vocabulary bit for a lost audio backend.
const LOST_AUDIO: u8 = 1 << 0;
/// The planner vocabulary bit for a lost external backend.
const LOST_EXTERNAL: u8 = 1 << 1;
/// The planner vocabulary bit for a lost video limb.
const LOST_VIDEO: u8 = 1 << 2;

/// The deterministic demotion order (card 3.B2):
/// audio-master → video-master → free-running wall drift, with a
/// wired external device clock slotted after audio `[PROPOSAL]`.
const DEMOTION_ORDER: [MasterKind; 4] = [
    MasterKind::Audio,
    MasterKind::External,
    MasterKind::Video,
    MasterKind::Free,
];

/// The typed master-change record (card 3.B2) — the crate-side
/// payload doc 03 §4.6's `ClockMasterChanged` event carries. `Copy`
/// and fixed-size: draining the ring allocates nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MasterChanged {
    /// The master that was left.
    pub from: crate::ClockMaster,
    /// The master now in force.
    pub to: crate::ClockMaster,
    /// Why the switch happened.
    pub reason: MasterChangeReason,
    /// The position the switch carried (the seam's receipt — the
    /// switch is *reported*, never hidden as seamless).
    pub position: Timestamp,
    /// The anchor epoch after the switch (consumers reason about
    /// per-epoch monotonicity; a switch is an epoch boundary).
    pub anchor_epoch: u64,
    /// The session generation in force at the switch.
    pub generation: GenerationId,
}

/// Why a master changed (card 3.B2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MasterChangeReason {
    /// The named master's stream ended or disappeared mid-playback
    /// and the deterministic demotion chain ran.
    MasterLost(MasterStream),
    /// The engine switched master explicitly (mode change).
    EngineSwitch,
}

impl MasterChangeReason {
    /// The stable wire-style name (doc 05 vocabulary discipline).
    pub fn as_str(self) -> &'static str {
        match self {
            MasterChangeReason::MasterLost(_) => "master-lost",
            MasterChangeReason::EngineSwitch => "engine-switch",
        }
    }
}

/// The outcome of a master-loss report (card 3.B2) — typed, never a
/// silent no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterLoss {
    /// The current master's stream was the one lost; the
    /// deterministic chain demoted to `to` (event in the ring).
    Demoted {
        from: crate::ClockMaster,
        to: crate::ClockMaster,
    },
    /// The loss was recorded (the kind can no longer be a fallback
    /// target) but the current master was a *different* kind — no
    /// switch, no event. Stale or preemptive loss reports land here.
    Recorded {
        current: crate::ClockMaster,
    },
    /// The current master's stream was lost and **nothing** wired
    /// remains to fall back to (a single-master 3.B1 clock, or a
    /// backends clock whose every other backend was already lost).
    /// The clock stays on the dead master and its position holds —
    /// frozen truth, never a fabricated seamless clock. The engine
    /// must re-anchor or rebuild.
    NoFallback {
        current: crate::ClockMaster,
    },
}

/// The engine's initial-master choice for
/// [`MediaClock::with_backends`] (card 3.B2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InitialMaster {
    /// The policy's best: audio if wired, else video if declared,
    /// else the system wall (doc 03 §4.7 d's letter for audio-less
    /// media without a video limb). An external clock is never
    /// chosen silently — it is a deliberate mode.
    #[default]
    Auto,
    /// Explicit audio master (requires the audio backend).
    Audio,
    /// Explicit video master (requires the video limb declaration).
    Video,
    /// Explicit external-device-clock master (requires the external
    /// backend) — the WASAPI/CoreAudio-style mode.
    External,
    /// Explicit system wall master (audio-less media, no video).
    System,
}

/// A runtime master the engine may switch to explicitly
/// ([`MediaClock::switch_master`]). The wall's two provenances are
/// not choosable: `System` is construction-time only, `Free` is the
/// loss terminal — choosing to free-run is not a mode, it is a
/// failure the policy owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MasterTarget {
    Audio,
    Video,
    External,
}

impl MasterTarget {
    /// The stable wire-style name (doc 05 vocabulary discipline).
    pub fn as_str(self) -> &'static str {
        match self {
            MasterTarget::Audio => "audio",
            MasterTarget::Video => "video",
            MasterTarget::External => "external",
        }
    }
}

/// The outcome of an engine-initiated master switch (card 3.B2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterSwitch {
    /// The switch happened (event in the ring, reason
    /// [`MasterChangeReason::EngineSwitch`]).
    Switched {
        from: crate::ClockMaster,
        to: crate::ClockMaster,
    },
    /// The clock already runs that master — idempotent no-op.
    Already {
        current: crate::ClockMaster,
    },
}

/// A [`MediaClock::switch_master`] refusal — typed, the clock
/// unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchError {
    /// The target backend was not wired at construction.
    NotWired { target: MasterTarget },
    /// The target's stream was already reported lost.
    Lost { target: MasterTarget },
}

/// A [`MediaClock::seek_back`] refusal — typed, the clock unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekBackError {
    /// The clock is not in the Ended latch: the normal seek path is
    /// [`crate::PlaybackClock::reanchor`]; the Ended re-arm is a
    /// distinct, explicitly-terminal transition and is never
    /// silently coerced (card 3.B2).
    NotEnded {
        state: crate::ClockState,
        position: Timestamp,
    },
    /// The target cannot be represented as a nanosecond position
    /// (the taxonomy's overflow; the clock stays latched at its
    /// frozen end position).
    Overflow {
        target: Timestamp,
        source: TimeError,
    },
}

/// The outcome of a video-master steering observation (card 3.B2) —
/// typed, never a silent adopt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoSteer {
    /// The frame was adopted with `|error_ns|` within the snap
    /// bound: the anchor moved to the frame against the current
    /// wall reading. No epoch bump — the monotone clamp makes the
    /// move invisible (a frame ahead advances within the bound; a
    /// frame behind holds until the wall catches up).
    Snapped { error_ns: i128 },
    /// The frame was adopted as a discontinuity (`|error_ns|` beyond
    /// the snap bound): anchor epoch bumped, the monotone clamp
    /// reset — a visible, expected jump (a dirty PTS or a
    /// discontinuous stream; the outcome is the receipt).
    Jumped { error_ns: i128 },
    /// Video is not the master: frames schedule *against* the clock
    /// (doc 03 §4.10), they do not steer it. The observation was
    /// not adopted.
    NotMaster { master: crate::ClockMaster },
    /// The clock is paused: the freeze is authoritative — frames
    /// shown while paused belong to an engine-arranged re-anchor
    /// (scrub), not steering. Not adopted.
    Paused,
    /// The clock is in the Ended latch: no post-end steering. Not
    /// adopted.
    Ended,
}

/// The wired backend set (card 3.B2) — construction-time truth.
/// Cloning the `Arc`s for a switch is refcount work, not allocation.
#[derive(Clone)]
pub(crate) struct WiredBackends {
    audio: Option<Arc<dyn SampleCounterSource>>,
    external: Option<Arc<dyn MonotonicSource>>,
    wall: Option<Arc<dyn MonotonicSource>>,
    video: bool,
}

impl WiredBackends {
    /// The source object a kind would run on, if wired.
    fn source_for(&self, kind: MasterKind) -> Option<Source> {
        match kind {
            MasterKind::Audio => self.audio.clone().map(Source::Samples),
            MasterKind::External => {
                self.external
                    .clone()
                    .map(|source| Source::Monotonic {
                        source,
                        kind: MonotonicKind::External,
                    })
            }
            MasterKind::Video => {
                self.wall
                    .clone()
                    .map(|source| Source::Monotonic {
                        source,
                        kind: MonotonicKind::VideoWall,
                    })
            }
            MasterKind::Free => {
                self.wall
                    .clone()
                    .map(|source| Source::Monotonic {
                        source,
                        kind: MonotonicKind::Free,
                    })
            }
            MasterKind::System => {
                self.wall
                    .clone()
                    .map(|source| Source::Monotonic {
                        source,
                        kind: MonotonicKind::System,
                    })
            }
        }
    }

    /// Whether a kind is wired (ignores losses).
    fn wired(&self, kind: MasterKind) -> bool {
        match kind {
            MasterKind::Audio => self.audio.is_some(),
            MasterKind::External => self.external.is_some(),
            MasterKind::Video => self.video,
            MasterKind::Free | MasterKind::System => self.wall.is_some(),
        }
    }
}

/// The card 3.B2 per-core backend state (fixed size, D-2).
pub(crate) struct BackendCore {
    /// The wired fallback sources (construction-time).
    wired: WiredBackends,
    /// The loss bitset — kinds that can no longer be chosen.
    lost: u8,
    /// The video snap bound (ns).
    video_snap_ns: u64,
    /// The Ended latch (card 3.B2).
    ended: bool,
    /// The bounded master-change event ring.
    events: EventRing,
}

impl BackendCore {
    /// No fallback backends at all (the 3.B1 single-master
    /// constructors — `audio_master` and `system_monotonic` alike):
    /// the truth source lives in `Core::source`, nothing is wired
    /// as a switch target, and losses are
    /// [`MasterLoss::NoFallback`].
    pub(crate) fn no_fallback() -> BackendCore {
        BackendCore {
            wired: WiredBackends {
                audio: None,
                external: None,
                wall: None,
                video: false,
            },
            lost: 0,
            video_snap_ns: DEFAULT_VIDEO_SNAP_NS,
            ended: false,
            events: EventRing::new(),
        }
    }

    /// The multi-backend state (card 3.B2's `with_backends`).
    fn multi(wired: WiredBackends, video_snap_ns: u64) -> BackendCore {
        BackendCore {
            wired,
            lost: 0,
            video_snap_ns,
            ended: false,
            events: EventRing::new(),
        }
    }
}

/// The bounded master-change event ring (card 3.B2, D-2): fixed
/// capacity, FIFO, a lost-count receipt on overflow instead of
/// growth. All operations are allocation-free.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EventRing {
    slots: [Option<MasterChanged>; MASTER_EVENT_CAPACITY],
    /// The next write index.
    head: usize,
    /// Live entry count.
    len: usize,
    /// Events dropped because the ring was full — the honest
    /// receipt for a consumer that does not drain.
    lost: u64,
}

impl EventRing {
    const fn new() -> EventRing {
        EventRing {
            slots: [None; MASTER_EVENT_CAPACITY],
            head: 0,
            len: 0,
            lost: 0,
        }
    }

    /// Record an event; on a full ring, count the loss (bounded).
    fn push(&mut self, event: MasterChanged) {
        if self.len == MASTER_EVENT_CAPACITY {
            self.lost += 1;
            return;
        }
        self.slots[self.head] = Some(event);
        self.head = (self.head + 1) % MASTER_EVENT_CAPACITY;
        self.len += 1;
    }

    /// Drain the oldest event, FIFO.
    fn pop(&mut self) -> Option<MasterChanged> {
        if self.len == 0 {
            return None;
        }
        let oldest = (self.head + MASTER_EVENT_CAPACITY - self.len) % MASTER_EVENT_CAPACITY;
        let event = self.slots[oldest].take();
        self.len -= 1;
        event
    }
}

/// The deterministic fallback planner (card 3.B2's core policy,
/// pure): given what is wired, what has been lost, and the current
/// master, the next master — or `None` when the current master was
/// not lost (nothing to do) or nothing remains (the caller reports
/// [`MasterLoss::NoFallback`] — the planner never invents a master).
pub(crate) fn plan_fallback(
    wired: &WiredBackends,
    lost: u8,
    current: MasterKind,
) -> Option<MasterKind> {
    if current.loss_bit() & lost == 0 {
        // The current master's own stream is not the one lost: no
        // demotion (a *recorded* loss only).
        return None;
    }
    DEMOTION_ORDER
        .into_iter()
        .find(|kind| kind.loss_bit() & lost == 0 && wired.wired(*kind))
}

/// The construction-time builder for the multi-backend clock (card
/// 3.B2). The wall source is **required**: video-master interpolates
/// on it between frames and the free-running terminal runs on it.
/// A clock that cannot provide a wall uses the 3.B1 single-master
/// constructors instead.
///
/// # Example
///
/// ```
/// use media_clock::{ClockBackends, MediaClock, PlaybackClock};
/// use media_types::time::{Rate, Timestamp};
/// use std::sync::atomic::{AtomicU64, Ordering};
/// use std::sync::Arc;
///
/// struct Wall(AtomicU64);
/// impl media_clock::MonotonicSource for Wall {
///     fn now_ns(&self) -> u64 { self.0.load(Ordering::Relaxed) }
/// }
/// struct Counter(AtomicU64);
/// impl media_clock::SampleCounterSource for Counter {
///     fn sample_count(&self) -> u64 { self.0.load(Ordering::Relaxed) }
///     fn sample_rate_hz(&self) -> u32 { 48_000 }
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let wall = Arc::new(Wall(AtomicU64::new(0)));
/// let counter = Arc::new(Counter(AtomicU64::new(0)));
/// let clock = MediaClock::with_backends(
///     ClockBackends::new(wall).audio(counter).video(true),
///     Timestamp::new(0, media_clock::NANO_BASE),
///     Rate::ONE,
/// )?;
/// // Audio wired: audio-master.
/// assert_eq!(clock.master(), media_clock::ClockMaster::AudioOutput);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct ClockBackends {
    wall: Arc<dyn MonotonicSource>,
    audio: Option<Arc<dyn SampleCounterSource>>,
    external: Option<Arc<dyn MonotonicSource>>,
    video: bool,
    video_snap_ns: u64,
    initial: InitialMaster,
}

impl ClockBackends {
    /// The backend set over a required wall source.
    pub fn new(wall: Arc<dyn MonotonicSource>) -> ClockBackends {
        ClockBackends {
            wall,
            audio: None,
            external: None,
            video: false,
            video_snap_ns: DEFAULT_VIDEO_SNAP_NS,
            initial: InitialMaster::Auto,
        }
    }

    /// Wire the audio sample-counter backend (the audio output
    /// adapter; doc 03 §4.9 `sample_counter`).
    pub fn audio(mut self, source: Arc<dyn SampleCounterSource>) -> Self {
        self.audio = Some(source);
        self
    }

    /// Wire an external device-clock backend (a WASAPI/CoreAudio-
    /// style monotonic clock domain).
    pub fn external(mut self, source: Arc<dyn MonotonicSource>) -> Self {
        self.external = Some(source);
        self
    }

    /// Declare a video limb that will steer the clock in
    /// video-master mode (the observations arrive per presented
    /// frame through [`MediaClock::observe_video`]). This is a
    /// declaration, not a source: the engine knows whether a video
    /// stream exists; the clock cannot infer it.
    pub fn video(mut self, present: bool) -> Self {
        self.video = present;
        self
    }

    /// The video snap bound (ns) — see [`DEFAULT_VIDEO_SNAP_NS`].
    pub fn video_snap_ns(mut self, ns: u64) -> Self {
        self.video_snap_ns = ns;
        self
    }

    /// The initial master (default [`InitialMaster::Auto`]).
    pub fn initial(mut self, initial: InitialMaster) -> Self {
        self.initial = initial;
        self
    }
}

impl MediaClock {
    /// The multi-backend clock (card 3.B2): video-master and
    /// external-master modes, the deterministic master-loss
    /// demotion chain, and the Ended latch with its `seek_back`
    /// re-arm — one clock whose master can change honestly, over
    /// the same anchor arithmetic, monotone clamp and typed
    /// discontinuity defense as the 3.B1 single-master clocks.
    ///
    /// The initial master is [`ClockBackends::initial`] (default
    /// `Auto`: audio if wired, else video if declared, else the
    /// system wall). Every later switch carries the position, bumps
    /// the anchor epoch and records a [`MasterChanged`] event.
    ///
    /// # Errors
    /// [`media_types::EngineError`] — as [`MediaClock::audio_master`]
    /// (unrepresentable position, zero-component rate), plus kind
    /// `Unsupported` at stage `Control` when the requested initial
    /// master is not wired.
    pub fn with_backends(
        backends: ClockBackends,
        media: Timestamp,
        rate: Rate,
    ) -> Result<MediaClock, media_types::EngineError> {
        let wired = WiredBackends {
            audio: backends.audio,
            external: backends.external,
            wall: Some(backends.wall),
            video: backends.video,
        };
        let initial = match backends.initial {
            InitialMaster::Auto => [
                MasterKind::Audio,
                MasterKind::Video,
                MasterKind::System,
            ]
            .into_iter()
            .find(|kind| wired.wired(*kind))
            .unwrap_or(MasterKind::System),
            InitialMaster::Audio => MasterKind::Audio,
            InitialMaster::Video => MasterKind::Video,
            InitialMaster::External => MasterKind::External,
            InitialMaster::System => MasterKind::System,
        };
        let Some(source) = wired.source_for(initial) else {
            return Err(media_types::EngineError::new(
                media_types::PipelineStage::Control,
                media_types::ErrorKind::Unsupported { format: None },
                media_types::ErrorContext::new("clock.backend-not-wired"),
            ));
        };
        let backend = BackendCore::multi(wired, backends.video_snap_ns);
        MediaClock::build(source, backend, media, rate)
    }

    /// Report a master stream's end/disappearance mid-playback
    /// (card 3.B2) and run the deterministic fallback:
    ///
    /// - the current master's own stream → demote along
    ///   [`plan_fallback`]'s chain (audio → external-if-wired →
    ///   video → free-running wall), position carried, epoch
    ///   bumped, event recorded;
    /// - a different stream → the loss is *recorded* (that kind can
    ///   no longer be a fallback target) but no switch happens;
    /// - nothing left wired → [`MasterLoss::NoFallback`]: the clock
    ///   stays on the dead master and its position holds — frozen
    ///   truth, never a fabricated seamless clock.
    ///
    /// Idempotent per stream: a second report of the same loss is a
    /// `Recorded` no-op.
    pub fn on_master_lost(&self, stream: MasterStream) -> MasterLoss {
        let mut core = self.lock_core();
        let current = core.source.master_kind();
        // Record the loss either way: the kind is spent as a target.
        core.backend.lost |= MasterKind::from(stream).loss_bit();
        if MasterKind::from(stream) != current {
            return MasterLoss::Recorded {
                current: current.label(),
            };
        }
        match plan_fallback(&core.backend.wired, core.backend.lost, current) {
            Some(next) => {
                let (from, to) =
                    switch_locked(&mut core, next, MasterChangeReason::MasterLost(stream));
                MasterLoss::Demoted { from, to }
            }
            None => MasterLoss::NoFallback {
                current: current.label(),
            },
        }
    }

    /// Switch master explicitly (engine-initiated mode change,
    /// card 3.B2): position carried, epoch bumped, event recorded
    /// with reason [`MasterChangeReason::EngineSwitch`]. The wall's
    /// two provenances are not choosable — `System` is
    /// construction-time, `Free` belongs to the loss policy.
    ///
    /// # Errors
    /// [`SwitchError::NotWired`] when the target backend was not
    /// wired at construction; [`SwitchError::Lost`] when the
    /// target's stream was already reported lost. The clock is
    /// unchanged on error.
    pub fn switch_master(&self, target: MasterTarget) -> Result<MasterSwitch, SwitchError> {
        let kind = match target {
            MasterTarget::Audio => MasterKind::Audio,
            MasterTarget::Video => MasterKind::Video,
            MasterTarget::External => MasterKind::External,
        };
        let mut core = self.lock_core();
        if !core.backend.wired.wired(kind) {
            return Err(SwitchError::NotWired { target });
        }
        if kind.loss_bit() & core.backend.lost != 0 {
            return Err(SwitchError::Lost { target });
        }
        if core.source.master_kind() == kind {
            return Ok(MasterSwitch::Already {
                current: kind.label(),
            });
        }
        let (from, to) = switch_locked(&mut core, kind, MasterChangeReason::EngineSwitch);
        Ok(MasterSwitch::Switched { from, to })
    }

    /// Steer the video-master clock with a presented frame's media
    /// position (card 3.B2): the video sink calls this once per
    /// frame it actually presents. In-bound frames (within the snap
    /// bound) move the anchor invisibly; out-of-bound frames are
    /// adopted as typed discontinuities with an epoch bump. In any
    /// other master the observation is **not** adopted — frames
    /// schedule against the clock (doc 03 §4.10), they do not steer
    /// it.
    ///
    /// # Errors
    /// [`TimeError::Overflow`] when `frame_pts` cannot be
    /// represented as a nanosecond position (the clock is
    /// unchanged; a dirty PTS is data, never a panic).
    pub fn observe_video(&self, frame_pts: Timestamp) -> Result<VideoSteer, TimeError> {
        let pts_ns = position_nanos(frame_pts)?;
        let mut core = self.lock_core();
        if core.source.master_kind() != MasterKind::Video {
            return Ok(VideoSteer::NotMaster {
                master: core.source.master(),
            });
        }
        if core.backend.ended {
            return Ok(VideoSteer::Ended);
        }
        if core.paused {
            return Ok(VideoSteer::Paused);
        }
        let reading = read_source(&core.source);
        // The comparison base: the raw extrapolated position, or the
        // held one across a dirty-world wall regression (the frame
        // may cure it — a re-anchor always resets the baseline).
        let raw_now = match core.extrapolate(&reading) {
            Extrapolation::Running(raw) => raw,
            _ => i128::from(core.last_ns),
        };
        // i128 by construction: both operands are i64-range — the
        // difference of two i64s always fits (card 3.B2's overflow
        // edge tests pin this at the bounds).
        let error_ns = i128::from(pts_ns) - raw_now;
        let bound = i128::from(core.backend.video_snap_ns);
        core.anchor_media_ns = pts_ns;
        adopt_anchor(&mut core, &reading);
        if error_ns.abs() <= bound {
            // No epoch bump: the monotone clamp makes a snap
            // invisible (see ClockSnapshot::anchor_epoch's docs).
            Ok(VideoSteer::Snapped { error_ns })
        } else {
            // A visible, expected jump: epoch boundary, clamp reset.
            core.epoch += 1;
            core.last_ns = pts_ns;
            Ok(VideoSteer::Jumped { error_ns })
        }
    }

    /// Latch the Ended state (card 3.B2): the engine's end-of-media
    /// signal. The position freezes at the current reading and
    /// stays frozen — including for wall-driven masters, which
    /// would otherwise run past the media's end (a fabricated
    /// position). `resume()` is a no-op under the latch; the only
    /// re-arm is [`MediaClock::seek_back`]. Idempotent; a second
    /// call returns the same frozen snapshot.
    ///
    /// The engine's honest end sequence is
    /// `reanchor(end_position); mark_ended()` when it wants the
    /// reported position to be the media's end rather than wherever
    /// the clock happened to be.
    pub fn mark_ended(&self) -> crate::ClockSnapshot {
        let mut core = self.lock_core();
        if !core.backend.ended {
            let reading = read_source(&core.source);
            core.frozen_ns = now_locked(&mut core, &reading);
            core.backend.ended = true;
            // Ended subsumes pause: one terminal freeze, not two.
            core.paused = false;
        }
        snapshot_locked(&mut core)
    }

    /// The Ended-state seek-back re-arm (card 3.B2): after playback
    /// reached the end, a seek re-arms the clock **paused at the new
    /// position** — never free-running. Play continues only on the
    /// engine's explicit `resume`; between the two, the position
    /// stays exactly at `media` no matter what the sources do.
    ///
    /// # Errors
    /// [`SeekBackError::NotEnded`] when the clock is not latched
    /// (the normal seek path is `reanchor`; the Ended re-arm is a
    /// distinct transition, never silently coerced);
    /// [`SeekBackError::Overflow`] when `media` cannot be
    /// represented — the clock stays latched at its frozen end.
    pub fn seek_back(&self, media: Timestamp) -> Result<crate::ClockSnapshot, SeekBackError> {
        let mut core = self.lock_core();
        if !core.backend.ended {
            let reading = read_source(&core.source);
            let position = Timestamp::new(now_locked(&mut core, &reading), NANO_BASE);
            return Err(SeekBackError::NotEnded {
                state: clock_state(&core),
                position,
            });
        }
        let target_ns = position_nanos(media).map_err(|source| SeekBackError::Overflow {
            target: media,
            source,
        })?;
        core.backend.ended = false;
        core.paused = true;
        core.anchor_media_ns = target_ns;
        core.frozen_ns = target_ns;
        core.last_ns = target_ns;
        let reading = read_source(&core.source);
        if matches!(reading, crate::clock::Reading::Usable { .. }) {
            adopt_anchor(&mut core, &reading);
        } else {
            // The source cannot be read right now: hold at the
            // target and adopt its first usable reading (fresh
            // baseline — no false discontinuity against the stale
            // anchor of the pre-end master).
            core.awaiting_anchor = true;
        }
        core.epoch += 1;
        Ok(snapshot_locked(&mut core))
    }

    /// Whether the Ended latch holds (card 3.B2).
    pub fn is_ended(&self) -> bool {
        self.lock_core().backend.ended
    }

    /// Drain the oldest master-change event (card 3.B2), FIFO,
    /// allocation-free. The engine forwards these as doc 03 §4.6's
    /// `ClockMasterChanged` events.
    pub fn take_master_change(&self) -> Option<MasterChanged> {
        self.lock_core().backend.events.pop()
    }

    /// How many master-change events were dropped because the ring
    /// was full (the honest receipt for a consumer that does not
    /// drain; zero in any healthy wiring).
    pub fn master_changes_lost(&self) -> u64 {
        self.lock_core().backend.events.lost
    }
}

impl Source {
    /// The planner vocabulary for this source (card 3.B2).
    pub(crate) fn master_kind(&self) -> MasterKind {
        match self {
            Source::Samples(_) => MasterKind::Audio,
            Source::Monotonic {
                kind: MonotonicKind::System,
                ..
            } => MasterKind::System,
            Source::Monotonic {
                kind: MonotonicKind::VideoWall,
                ..
            } => MasterKind::Video,
            Source::Monotonic {
                kind: MonotonicKind::External,
                ..
            } => MasterKind::External,
            Source::Monotonic {
                kind: MonotonicKind::Free,
                ..
            } => MasterKind::Free,
        }
    }
}

/// Perform a master switch under a held lock (card 3.B2's one
/// atomic seam): carry the clamped position from the old master,
/// swap the source, re-anchor against the new master's current
/// reading, bump the epoch, record the event. A new source that
/// cannot be read yet (an audio backend reporting a zero rate)
/// parks the clock at the carried position with the
/// awaiting-first-reading latch instead of comparing a fresh
/// counter domain against the old master's anchor.
fn switch_locked(
    core: &mut Core,
    to: MasterKind,
    reason: MasterChangeReason,
) -> (crate::ClockMaster, crate::ClockMaster) {
    // 1. The carried position under the OLD source (clamped —
    //    holds across the old master's own discontinuity).
    let old_reading = read_source(&core.source);
    let position = now_locked(core, &old_reading);
    let from = core.source.master();
    // 2. Swap the truth source (inside the lock — atomic with the
    //    re-anchor; see crate::clock::Source).
    let Some(next_source) = core.backend.wired.source_for(to) else {
        // Unreachable by construction: every caller validated the
        // target against the same wired set (plan_fallback /
        // switch_master). Defensive: refuse rather than invent.
        return (from, from);
    };
    core.source = next_source;
    let to_label = core.source.master();
    // 3. Re-anchor at the carried position against the NEW source.
    core.anchor_media_ns = position;
    let new_reading = read_source(&core.source);
    if matches!(new_reading, crate::clock::Reading::Usable { .. }) {
        adopt_anchor(core, &new_reading);
    } else {
        core.awaiting_anchor = true;
    }
    // 4. The freeze (paused/ended) carries at the same position.
    if core.paused || core.backend.ended {
        core.frozen_ns = position;
    }
    // 5. Epoch boundary: a switch is a re-anchor (the position
    //    carries, so no jump — but per-epoch reasoning restarts).
    core.epoch += 1;
    // 6. The seam's receipt — bounded ring, typed record.
    core.backend.events.push(MasterChanged {
        from,
        to: to_label,
        reason,
        position: Timestamp::new(position, NANO_BASE),
        anchor_epoch: core.epoch,
        generation: core.generation,
    });
    (from, to_label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClockMaster::{AudioOutput, External, FreeRunning, SystemMonotonic, VideoOutput};
    use crate::ClockState::{Ended, Paused, Running};
    use crate::{NANO_BASE, PlaybackClock};
    use std::sync::atomic::{AtomicU64, Ordering};
    use testkit::Must;

    /// A deterministic fake sample counter.
    struct FakeCounter {
        sample: AtomicU64,
        rate_hz: AtomicU64,
    }

    impl FakeCounter {
        fn new(rate_hz: u32) -> FakeCounter {
            FakeCounter {
                sample: AtomicU64::new(0),
                rate_hz: AtomicU64::new(u64::from(rate_hz)),
            }
        }

        fn advance_samples(&self, samples: u64) {
            let _ = self.sample.fetch_add(samples, Ordering::Relaxed);
        }
    }

    impl crate::SampleCounterSource for FakeCounter {
        fn sample_count(&self) -> u64 {
            self.sample.load(Ordering::Relaxed)
        }
        fn sample_rate_hz(&self) -> u32 {
            u32::try_from(self.rate_hz.load(Ordering::Relaxed)).unwrap_or(0)
        }
    }

    /// A deterministic fake monotonic source (wall / external).
    struct FakeMonotonic(AtomicU64);

    impl FakeMonotonic {
        fn new() -> FakeMonotonic {
            FakeMonotonic(AtomicU64::new(0))
        }

        fn advance_ns(&self, ns: u64) {
            let _ = self.0.fetch_add(ns, Ordering::Relaxed);
        }
    }

    impl crate::MonotonicSource for FakeMonotonic {
        fn now_ns(&self) -> u64 {
            self.0.load(Ordering::Relaxed)
        }
    }

    fn ts(ns: i64) -> Timestamp {
        Timestamp::new(ns, NANO_BASE)
    }

    fn backends(
        wall: std::sync::Arc<FakeMonotonic>,
    ) -> ClockBackends {
        ClockBackends::new(wall)
    }

    // ---- construction / initial master ------------------------------------

    #[test]
    fn auto_picks_audio_when_wired() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(
            backends(wall)
                .audio(std::sync::Arc::new(FakeCounter::new(48_000)))
                .video(true),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(clock.master(), AudioOutput);
    }

    #[test]
    fn auto_picks_video_for_audioless_with_video() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock =
            MediaClock::with_backends(backends(wall).video(true), ts(0), Rate::ONE)
                .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(clock.master(), VideoOutput);
    }

    #[test]
    fn auto_picks_system_wall_when_only_wall() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(backends(wall), ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(clock.master(), SystemMonotonic);
        wall.advance_ns(1_000_000_000);
        assert_eq!(clock.now().ticks, 1_000_000_000);
    }

    #[test]
    fn external_master_is_explicit_and_reads_the_injected_source() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let device = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(
            backends(wall)
                .audio(std::sync::Arc::new(FakeCounter::new(48_000)))
                .external(device.clone()),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        // Auto never chooses external silently even with audio wired.
        assert_eq!(clock.master(), AudioOutput);

        let explicit = MediaClock::with_backends(
            backends(wall)
                .external(device.clone())
                .initial(InitialMaster::External),
            ts(2_000_000_000),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(explicit.master(), External);
        device.advance_ns(500_000_000);
        assert_eq!(explicit.now().ticks, 2_500_000_000);
    }

    #[test]
    fn unwired_initial_master_is_a_typed_refusal() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let err = MediaClock::with_backends(
            backends(wall).initial(InitialMaster::Audio),
            ts(0),
            Rate::ONE,
        )
        .err()
        .must("audio initial without an audio backend must refuse");
        assert_eq!(err.stage, media_types::PipelineStage::Control);
        assert!(matches!(
            err.kind,
            media_types::ErrorKind::Unsupported { .. }
        ));
    }

    // ---- the master-switch matrix (AC) ------------------------------------

    /// The pure planner, exhaustively over every wiring × loss
    /// state × current master (2^4 wirings × 2^3 loss sets × 5
    /// kinds = 640 cells): the plan is deterministic, never picks a
    /// lost or unwired kind, demotes exactly when the current
    /// master is lost and something remains, and never invents a
    /// master out of nothing.
    #[test]
    fn fallback_plan_is_deterministic_over_the_full_matrix() {
        let counter = std::sync::Arc::new(FakeCounter::new(48_000));
        let mono = std::sync::Arc::new(FakeMonotonic::new());
        let all_kinds = [
            MasterKind::Audio,
            MasterKind::External,
            MasterKind::Video,
            MasterKind::Free,
            MasterKind::System,
        ];
        for wiring in 0..16u8 {
            let wired = WiredBackends {
                audio: (wiring & 1 != 0).then(|| {
                    let arc: std::sync::Arc<dyn SampleCounterSource> = counter.clone();
                    arc
                }),
                external: (wiring & 2 != 0).then(|| {
                    let arc: std::sync::Arc<dyn MonotonicSource> = mono.clone();
                    arc
                }),
                wall: (wiring & 4 != 0).then(|| {
                    let arc: std::sync::Arc<dyn MonotonicSource> = mono.clone();
                    arc
                }),
                video: wiring & 8 != 0,
            };
            for lost in 0..8u8 {
                for &current in &all_kinds {
                    let plan = plan_fallback(&wired, lost, current);
                    let current_lost = current.loss_bit() & lost != 0;
                    if !current_lost {
                        assert_eq!(
                            plan, None,
                            "wiring {wiring} lost {lost}: un-lost current must not switch"
                        );
                        continue;
                    }
                    let Some(next) = plan else {
                        // None requires nothing wired-and-unlost.
                        for kind in DEMOTION_ORDER {
                            let available =
                                kind.loss_bit() & lost == 0 && wired.wired(kind);
                            assert!(
                                !available,
                                "wiring {wiring} lost {lost}: {kind:?} available but plan None"
                            );
                        }
                        continue;
                    };
                    assert_eq!(
                        next.loss_bit() & lost,
                        0,
                        "wiring {wiring} lost {lost}: picked a lost kind {next:?}"
                    );
                    assert!(
                        wired.wired(next),
                        "wiring {wiring} lost {lost}: picked unwired {next:?}"
                    );
                    // Deterministic: the first available in order.
                    let first = DEMOTION_ORDER
                        .into_iter()
                        .find(|k| k.loss_bit() & lost == 0 && wired.wired(*k));
                    assert_eq!(Some(next), first, "wiring {wiring} lost {lost}");
                }
            }
        }
    }

    /// The behavioral matrix (AC): every demotion step carries the
    /// position, bumps the epoch exactly once, and records the
    /// typed event — audio → video → free-running over a fully
    /// wired clock (the card's chain, external unwired).
    #[test]
    fn master_loss_chain_audio_video_free_runs_with_events() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let counter = std::sync::Arc::new(FakeCounter::new(48_000));
        let clock = MediaClock::with_backends(
            backends(wall.clone()).audio(counter.clone()).video(true),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));

        // Play 2 s of audio.
        counter.advance_samples(2 * 48_000);
        assert_eq!(clock.now().ticks, 2_000_000_000);
        let epoch_audio = clock.snapshot().anchor_epoch;

        // Audio ends: demote to video-master at the same position.
        assert_eq!(
            clock.on_master_lost(MasterStream::Audio),
            MasterLoss::Demoted {
                from: AudioOutput,
                to: VideoOutput
            }
        );
        assert_eq!(clock.master(), VideoOutput);
        assert_eq!(clock.now().ticks, 2_000_000_000); // carried
        assert_eq!(clock.snapshot().anchor_epoch, epoch_audio + 1);

        // The wall was not advanced during the audio epoch; the
        // video clock now interpolates from the switch instant.
        wall.advance_ns(1_000_000_000);
        assert_eq!(clock.now().ticks, 3_000_000_000);

        // Video ends too: terminal free-running wall drift.
        assert_eq!(
            clock.on_master_lost(MasterStream::Video),
            MasterLoss::Demoted {
                from: VideoOutput,
                to: FreeRunning
            }
        );
        assert_eq!(clock.master(), FreeRunning);
        let carried = clock.now().ticks;
        wall.advance_ns(1_000_000_000);
        assert_eq!(clock.now().ticks, carried + 1_000_000_000);

        // The trail: two typed events, FIFO, correct seams.
        let first = clock.take_master_change().must("first event");
        assert_eq!(
            first,
            MasterChanged {
                from: AudioOutput,
                to: VideoOutput,
                reason: MasterChangeReason::MasterLost(MasterStream::Audio),
                position: ts(2_000_000_000),
                anchor_epoch: epoch_audio + 1,
                generation: GenerationId::FIRST,
            }
        );
        let second = clock.take_master_change().must("second event");
        assert_eq!(second.from, VideoOutput);
        assert_eq!(second.to, FreeRunning);
        assert_eq!(
            second.reason,
            MasterChangeReason::MasterLost(MasterStream::Video)
        );
        assert_eq!(second.position.ticks, carried);
        assert_eq!(clock.take_master_change(), None);
        assert_eq!(clock.master_changes_lost(), 0);
    }

    /// The external slot: a lost audio demotes to a wired external
    /// device clock before video `[PROPOSAL in module docs]`.
    #[test]
    fn lost_audio_demotes_to_wired_external_before_video() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let device = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(
            backends(wall)
                .audio(std::sync::Arc::new(FakeCounter::new(48_000)))
                .external(device.clone())
                .video(true),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            clock.on_master_lost(MasterStream::Audio),
            MasterLoss::Demoted {
                from: AudioOutput,
                to: External
            }
        );
        device.advance_ns(1_000_000_000);
        assert_eq!(clock.now().ticks, 1_000_000_000);
        // And the chain continues honestly: external lost → video.
        assert_eq!(
            clock.on_master_lost(MasterStream::External),
            MasterLoss::Demoted {
                from: External,
                to: VideoOutput
            }
        );
    }

    /// A stale or preemptive loss report (not the current master)
    /// is recorded, not switched: no event, master unchanged — and
    /// the recorded kind is spent as a later fallback target.
    #[test]
    fn non_current_loss_is_recorded_not_switched() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let counter = std::sync::Arc::new(FakeCounter::new(48_000));
        let clock = MediaClock::with_backends(
            backends(wall).audio(counter).video(true),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        // Video "ends" while audio is master: recorded, no switch.
        assert_eq!(
            clock.on_master_lost(MasterStream::Video),
            MasterLoss::Recorded { current: AudioOutput }
        );
        assert_eq!(clock.master(), AudioOutput);
        assert_eq!(clock.take_master_change(), None);
        // Now audio is lost: video is spent, the wall takes over.
        assert_eq!(
            clock.on_master_lost(MasterStream::Audio),
            MasterLoss::Demoted {
                from: AudioOutput,
                to: FreeRunning
            }
        );
    }

    /// Idempotent loss: re-reporting the same stream is a Recorded
    /// no-op against the new master.
    #[test]
    fn repeated_loss_reports_are_idempotent() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(
            backends(wall)
                .audio(std::sync::Arc::new(FakeCounter::new(48_000)))
                .video(true),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        let _ = clock.on_master_lost(MasterStream::Audio);
        assert_eq!(
            clock.on_master_lost(MasterStream::Audio),
            MasterLoss::Recorded { current: VideoOutput }
        );
        assert_eq!(clock.master(), VideoOutput);
    }

    /// A single-master 3.B1 clock has no fallback: the typed
    /// refusal, a held position, no fabricated master.
    #[test]
    fn single_master_clock_reports_no_fallback_and_holds() {
        let counter = std::sync::Arc::new(FakeCounter::new(48_000));
        let clock = MediaClock::audio_master(counter.clone(), ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        counter.advance_samples(48_000);
        let held = clock.now();
        assert_eq!(held.ticks, 1_000_000_000);
        assert_eq!(
            clock.on_master_lost(MasterStream::Audio),
            MasterLoss::NoFallback { current: AudioOutput }
        );
        assert_eq!(clock.master(), AudioOutput);
        assert_eq!(clock.now(), held); // frozen truth, not invented time
        assert_eq!(clock.take_master_change(), None);
    }

    /// The loss chain under the Ended latch: the demotion still
    /// happens (the event is the receipt) but the freeze is
    /// terminal — the position does not run on the new master.
    #[test]
    fn loss_while_ended_switches_but_stays_frozen() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let counter = std::sync::Arc::new(FakeCounter::new(48_000));
        let clock = MediaClock::with_backends(
            backends(wall).audio(counter).video(true),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        counter.advance_samples(48_000);
        let snap = clock.mark_ended();
        assert_eq!(snap.state, Ended);
        assert_eq!(snap.position.ticks, 1_000_000_000);
        assert_eq!(
            clock.on_master_lost(MasterStream::Audio),
            MasterLoss::Demoted {
                from: AudioOutput,
                to: VideoOutput
            }
        );
        // The wall runs; the ended clock does not.
        wall.advance_ns(5_000_000_000);
        assert_eq!(clock.now().ticks, 1_000_000_000);
        assert_eq!(clock.snapshot().state, Ended);
    }

    /// Determinism (AC): two identically-wired clocks fed the same
    /// loss script produce identical master sequences, event trails
    /// and epochs.
    #[test]
    fn loss_scripts_are_deterministic_across_identical_clocks() {
        fn run() -> (Vec<crate::ClockMaster>, Vec<MasterChanged>, u64) {
            let wall = std::sync::Arc::new(FakeMonotonic::new());
            let clock = MediaClock::with_backends(
                backends(wall)
                    .audio(std::sync::Arc::new(FakeCounter::new(48_000)))
                    .external(std::sync::Arc::new(FakeMonotonic::new()))
                    .video(true),
                ts(0),
                Rate::ONE,
            )
            .unwrap_or_else(|e| unreachable!("{e}"));
            let _ = clock.on_master_lost(MasterStream::Video); // recorded
            let _ = clock.on_master_lost(MasterStream::Audio); // → external
            let _ = clock.on_master_lost(MasterStream::External); // → free
            let masters = vec![clock.master()];
            let mut events = Vec::new();
            while let Some(event) = clock.take_master_change() {
                events.push(event);
            }
            (masters, events, clock.snapshot().anchor_epoch)
        }
        assert_eq!(run(), run());
    }

    // ---- engine-initiated switches ----------------------------------------

    #[test]
    fn engine_switch_carries_position_and_records_the_reason() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let counter = std::sync::Arc::new(FakeCounter::new(48_000));
        let clock = MediaClock::with_backends(
            backends(wall).audio(counter.clone()).video(true),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        counter.advance_samples(48_000);
        assert_eq!(
            clock.switch_master(MasterTarget::Video),
            Ok(MasterSwitch::Switched {
                from: AudioOutput,
                to: VideoOutput
            })
        );
        assert_eq!(clock.now().ticks, 1_000_000_000); // carried
        let event = clock.take_master_change().must("engine switch event");
        assert_eq!(event.reason, MasterChangeReason::EngineSwitch);
        assert_eq!(event.position.ticks, 1_000_000_000);
        // Idempotent.
        assert_eq!(
            clock.switch_master(MasterTarget::Video),
            Ok(MasterSwitch::Already { current: VideoOutput })
        );
        assert_eq!(clock.take_master_change(), None);
    }

    #[test]
    fn engine_switch_refusals_are_typed() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(
            backends(wall)
                .audio(std::sync::Arc::new(FakeCounter::new(48_000)))
                .video(true),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            clock.switch_master(MasterTarget::External),
            Err(SwitchError::NotWired {
                target: MasterTarget::External
            })
        );
        let _ = clock.on_master_lost(MasterStream::Audio); // → video
        assert_eq!(
            clock.switch_master(MasterTarget::Audio),
            Err(SwitchError::Lost {
                target: MasterTarget::Audio
            })
        );
        assert_eq!(clock.master(), VideoOutput);
    }

    /// Switching onto an audio backend whose device has not opened
    /// yet (zero rate): the position holds and the first usable
    /// reading becomes the anchor — no jump, no false
    /// discontinuity from the old wall anchor.
    #[test]
    fn switch_onto_unreadable_audio_awaits_first_usable_reading() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let counter = std::sync::Arc::new(FakeCounter::new(48_000));
        counter.rate_hz.store(0, Ordering::Relaxed); // device not open
        let clock = MediaClock::with_backends(
            ClockBackends::new(wall.clone())
                .audio(counter.clone())
                .video(true)
                .initial(InitialMaster::Video),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        wall.advance_ns(3_000_000_000);
        assert_eq!(clock.now().ticks, 3_000_000_000);
        assert_eq!(
            clock.switch_master(MasterTarget::Audio),
            Ok(MasterSwitch::Switched {
                from: VideoOutput,
                to: AudioOutput
            })
        );
        // Unreadable: the position holds at the carried value.
        assert_eq!(clock.now().ticks, 3_000_000_000);
        assert!(clock.try_now().is_err());
        // The device opens at sample 0: the fresh baseline adopts
        // with no jump and no discontinuity error.
        counter.rate_hz.store(48_000, Ordering::Relaxed);
        assert_eq!(clock.now().ticks, 3_000_000_000);
        assert!(clock.try_now().is_ok());
        counter.advance_samples(48_000);
        assert_eq!(clock.now().ticks, 4_000_000_000);
    }

    // ---- video-master steering ---------------------------------------------

    #[test]
    fn video_master_snaps_within_the_bound_and_interpolates_between() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(
            backends(wall.clone()).video(true).video_snap_ns(50_000_000),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(clock.master(), VideoOutput);

        // First frame at 30 ms (within the 50 ms bound): snapped.
        wall.advance_ns(10_000_000);
        assert_eq!(
            clock.observe_video(ts(30_000_000)),
            Ok(VideoSteer::Snapped { error_ns: 20_000_000 })
        );
        assert_eq!(clock.now().ticks, 30_000_000);

        // Interpolation: 100 ms of wall with no frames.
        wall.advance_ns(100_000_000);
        assert_eq!(clock.now().ticks, 130_000_000);

        // A frame behind by 20 ms (within bound): snapped, but the
        // monotone clamp holds until the wall catches up.
        assert_eq!(
            clock.observe_video(ts(110_000_000)),
            Ok(VideoSteer::Snapped { error_ns: -20_000_000 })
        );
        assert_eq!(clock.now().ticks, 130_000_000); // held
        wall.advance_ns(20_000_000);
        assert_eq!(clock.now().ticks, 130_000_000); // caught up exactly
        wall.advance_ns(5_000_000);
        assert_eq!(clock.now().ticks, 135_000_000); // flowing again
    }

    #[test]
    fn out_of_bound_frame_is_a_typed_jump_with_epoch_bump() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(
            backends(wall).video(true).video_snap_ns(40_000_000),
            ts(1_000_000_000),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        let epoch = clock.snapshot().anchor_epoch;
        // 5 s off: a discontinuous stream's dirty PTS.
        assert_eq!(
            clock.observe_video(ts(6_000_000_000)),
            Ok(VideoSteer::Jumped {
                error_ns: 5_000_000_000
            })
        );
        assert_eq!(clock.now().ticks, 6_000_000_000);
        assert_eq!(clock.snapshot().anchor_epoch, epoch + 1);
        // A visible backward jump is legal immediately after.
        assert_eq!(
            clock.observe_video(ts(1_000_000_000)),
            Ok(VideoSteer::Jumped { error_ns: -5_000_000_000 })
        );
        assert_eq!(clock.now().ticks, 1_000_000_000);
    }

    #[test]
    fn steering_is_rejected_outside_video_master() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let counter = std::sync::Arc::new(FakeCounter::new(48_000));
        let clock = MediaClock::with_backends(
            backends(wall).audio(counter.clone()).video(true),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            clock.observe_video(ts(1_000_000)),
            Ok(VideoSteer::NotMaster { master: AudioOutput })
        );
        // The anchor is untouched: the counter still rules.
        counter.advance_samples(48_000);
        assert_eq!(clock.now().ticks, 1_000_000_000);
    }

    #[test]
    fn steering_is_rejected_while_paused_and_ended() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(
            backends(wall).video(true),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        clock.pause();
        assert_eq!(clock.observe_video(ts(1_000_000)), Ok(VideoSteer::Paused));
        clock.resume();
        let _ = clock.mark_ended();
        assert_eq!(clock.observe_video(ts(1_000_000)), Ok(VideoSteer::Ended));
    }

    #[test]
    fn unrepresentable_frame_pts_is_a_typed_overflow() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(backends(wall).video(true), ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        let huge =
            Timestamp::from_parts(i64::MAX, 1000, 1).unwrap_or_else(|e| unreachable!("{e}"));
        assert!(matches!(
            clock.observe_video(huge),
            Err(TimeError::Overflow { .. })
        ));
        assert_eq!(clock.now().ticks, 0); // unchanged
    }

    /// A wall regression (contract violation, dirty world) while
    /// video-master: the clock holds; an in-bound frame cures it by
    /// re-anchoring on the now-current wall.
    #[test]
    fn wall_regression_holds_and_an_in_bound_frame_cures() {
        struct Regressable {
            ns: AtomicU64,
        }
        impl crate::MonotonicSource for Regressable {
            fn now_ns(&self) -> u64 {
                self.ns.load(Ordering::Relaxed)
            }
        }
        let wall = std::sync::Arc::new(Regressable { ns: AtomicU64::new(0) });
        let clock = MediaClock::with_backends(
            ClockBackends::new(wall.clone()).video(true).video_snap_ns(50_000_000),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        wall.ns.store(1_000_000_000, Ordering::Relaxed);
        assert_eq!(clock.now().ticks, 1_000_000_000);
        // The wall regresses (violation): hold, typed report.
        wall.ns.store(200_000_000, Ordering::Relaxed);
        assert_eq!(clock.now().ticks, 1_000_000_000);
        assert!(clock.try_now().is_err());
        // A frame 30 ms past the held position cures the anchor.
        assert_eq!(
            clock.observe_video(ts(1_030_000_000)),
            Ok(VideoSteer::Snapped { error_ns: 30_000_000 })
        );
        assert!(clock.try_now().is_ok());
        // The wall then jumps to 1.05 s: a forward jump is elapsed
        // time per the monotonic contract — 850 ms past the steer.
        wall.ns.store(1_050_000_000, Ordering::Relaxed);
        assert_eq!(clock.now().ticks, 1_880_000_000);
    }

    // ---- the Ended latch / seek-back (AC) -----------------------------------

    #[test]
    fn ended_latch_freezes_and_seek_back_re_arms_paused_not_free_running() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(backends(wall.clone()), ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        wall.advance_ns(2_000_000_000);
        assert_eq!(clock.now().ticks, 2_000_000_000);

        // End of media: the latch freezes — even though the wall
        // keeps running (a free-running position past the end would
        // be fabricated).
        let snap = clock.mark_ended();
        assert_eq!(snap.state, Ended);
        assert_eq!(snap.position.ticks, 2_000_000_000);
        wall.advance_ns(10_000_000_000);
        assert_eq!(clock.now().ticks, 2_000_000_000);
        assert!(clock.is_ended());

        // The card's trap: resume after Ended must NOT free-run.
        clock.resume();
        assert_eq!(clock.snapshot().state, Ended);
        assert_eq!(clock.now().ticks, 2_000_000_000);
        clock.pause(); // also a no-op under the latch
        assert_eq!(clock.snapshot().state, Ended);

        // Seek-back: re-armed paused at the new position.
        let epoch = clock.snapshot().anchor_epoch;
        let re_armed = clock.seek_back(ts(500_000_000)).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(re_armed.state, Paused);
        assert_eq!(re_armed.position.ticks, 500_000_000);
        assert_eq!(re_armed.anchor_epoch, epoch + 1);
        assert!(!clock.is_ended());

        // Not free-running: the wall keeps running, the position
        // stays at the target until the engine resumes.
        wall.advance_ns(7_000_000_000);
        assert_eq!(clock.now().ticks, 500_000_000);
        assert_eq!(clock.snapshot().state, Paused);

        // Resume continues from the target with no jump.
        clock.resume();
        assert_eq!(clock.now().ticks, 500_000_000);
        wall.advance_ns(1_000_000_000);
        assert_eq!(clock.now().ticks, 1_500_000_000);
    }

    #[test]
    fn seek_back_on_audio_master_resumes_without_jump_after_drain() {
        let counter = std::sync::Arc::new(FakeCounter::new(48_000));
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(
            backends(wall).audio(counter.clone()),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        counter.advance_samples(48_000);
        let _ = clock.mark_ended();
        // The device drained a little past the end signal — dirty
        // world; the latch froze at 1 s regardless.
        counter.advance_samples(24_000);
        assert_eq!(clock.now().ticks, 1_000_000_000);

        let re_armed = clock.seek_back(ts(250_000_000)).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(re_armed.state, Paused);
        assert_eq!(re_armed.position.ticks, 250_000_000);
        // The counter is at 72 000 samples; resume anchors there —
        // no jump backward, progress from the target.
        clock.resume();
        assert_eq!(clock.now().ticks, 250_000_000);
        counter.advance_samples(48_000);
        assert_eq!(clock.now().ticks, 1_250_000_000);
    }

    #[test]
    fn seek_back_refuses_when_not_ended_leaving_the_clock_alone() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(backends(wall.clone()), ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        wall.advance_ns(1_000_000_000);
        assert_eq!(
            clock.seek_back(ts(0)),
            Err(SeekBackError::NotEnded {
                state: Running,
                position: ts(1_000_000_000),
            })
        );
        assert_eq!(clock.snapshot().state, Running);
        wall.advance_ns(1_000_000_000);
        assert_eq!(clock.now().ticks, 2_000_000_000); // untouched
    }

    #[test]
    fn seek_back_overflow_keeps_the_latch_and_the_frozen_position() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(backends(wall), ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        wall.advance_ns(1_000_000_000);
        let _ = clock.mark_ended();
        let huge =
            Timestamp::from_parts(i64::MAX, 1000, 1).unwrap_or_else(|e| unreachable!("{e}"));
        assert!(matches!(
            clock.seek_back(huge),
            Err(SeekBackError::Overflow { .. })
        ));
        assert!(clock.is_ended());
        assert_eq!(clock.now().ticks, 1_000_000_000);
    }

    #[test]
    fn mark_ended_is_idempotent_and_reports_the_same_freeze() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(backends(wall.clone()), ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        wall.advance_ns(1_000_000_000);
        let first = clock.mark_ended();
        wall.advance_ns(1_000_000_000);
        let second = clock.mark_ended();
        assert_eq!(first, second);
        assert_eq!(clock.now().ticks, 1_000_000_000);
    }

    /// A plain re-anchor under the latch moves the freeze but never
    /// re-arms the clock — only seek_back clears Ended (the
    /// free-running trap is closed by construction).
    #[test]
    fn reanchor_while_ended_moves_the_freeze_but_keeps_the_latch() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(backends(wall.clone()), ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        wall.advance_ns(1_000_000_000);
        let _ = clock.mark_ended();
        clock
            .reanchor(ts(9_000_000_000))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(clock.snapshot().state, Ended);
        assert_eq!(clock.now().ticks, 9_000_000_000);
        wall.advance_ns(1_000_000_000);
        assert_eq!(clock.now().ticks, 9_000_000_000); // still frozen
        clock.resume();
        assert_eq!(clock.snapshot().state, Ended); // resume did not re-arm
    }

    /// The play-from-ended restart shape (doc 05 §234): seek-back
    /// to zero, then resume.
    #[test]
    fn play_from_ended_restarts_through_seek_back_zero() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(backends(wall.clone()), ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        wall.advance_ns(10_000_000_000);
        let _ = clock.mark_ended();
        let re_armed = clock.seek_back(ts(0)).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(re_armed.state, Paused);
        assert_eq!(re_armed.position.ticks, 0);
        clock.resume();
        wall.advance_ns(1_000_000_000);
        assert_eq!(clock.now().ticks, 1_000_000_000);
    }

    // ---- discontinuity / overflow edges (AC, property-style) ----------------

    /// The switch's position carry across the i64 edge set: no
    /// panic, no wrap, monotone at worst, and the event's position
    /// is representable.
    #[test]
    fn switches_across_i64_position_edges_carry_saturated() {
        let edges = [
            i64::MIN,
            i64::MIN + 1,
            -1,
            0,
            1,
            i64::MAX - 1,
            i64::MAX,
        ];
        for &edge in &edges {
            let wall = std::sync::Arc::new(FakeMonotonic::new());
            let clock = MediaClock::with_backends(
                backends(wall)
                    .audio(std::sync::Arc::new(FakeCounter::new(48_000)))
                    .video(true),
                Timestamp::new(edge, NANO_BASE),
                Rate::ONE,
            )
            .unwrap_or_else(|e| unreachable!("{e}"));
            assert_eq!(
                clock.on_master_lost(MasterStream::Audio),
                MasterLoss::Demoted {
                    from: AudioOutput,
                    to: VideoOutput
                }
            );
            let now = clock.now();
            assert_eq!(now.ticks, edge); // carried exactly at the edge
            let event = clock.take_master_change().must("edge event");
            assert_eq!(event.position.ticks, edge);
            // The wall advances a second: i128 extrapolation
            // saturates at the bounds, never wraps.
            wall.advance_ns(1_000_000_000);
            let advanced = clock.now();
            if edge == i64::MAX {
                assert_eq!(advanced.ticks, i64::MAX); // saturated
            } else {
                assert!(
                    advanced.ticks > now.ticks,
                    "edge {edge}: {now} did not advance to {advanced}"
                );
            }
        }
    }

    /// Video steering at the i64 bounds: the i128 error math never
    /// overflows, and the classification is exact at the snap
    /// boundary itself.
    #[test]
    fn video_steer_errors_at_the_i64_bounds_are_i128_safe() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(
            backends(wall).video(true).video_snap_ns(1_000_000_000),
            Timestamp::new(i64::MAX - 5_000_000_000, NANO_BASE),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        // A frame at the extreme opposite end: |error| ~ 2^64 ns,
        // far beyond any bound — a typed Jump, no overflow panic.
        let steer = clock
            .observe_video(Timestamp::new(i64::MIN + 5_000_000_000, NANO_BASE))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert!(matches!(steer, VideoSteer::Jumped { .. }));
        assert_eq!(clock.now().ticks, i64::MIN + 5_000_000_000);

        // Exactly at the bound: still a snap (<=).
        let at_bound = clock
            .observe_video(Timestamp::new(
                i64::MIN + 5_000_000_000 + 1_000_000_000,
                NANO_BASE,
            ))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            at_bound,
            VideoSteer::Snapped {
                error_ns: 1_000_000_000
            }
        );
    }

    /// A wall source saturated at u64::MAX (the InstantSource
    /// shape): construction, steering and switching all behave —
    /// the reading math is i128 and the position clamps.
    #[test]
    fn saturated_u64_wall_source_is_handled() {
        struct MaxWall;
        impl crate::MonotonicSource for MaxWall {
            fn now_ns(&self) -> u64 {
                u64::MAX
            }
        }
        let clock = MediaClock::with_backends(
            ClockBackends::new(std::sync::Arc::new(MaxWall)).video(true),
            ts(123),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(clock.now().ticks, 123); // delta 0 at the anchor
        assert_eq!(
            clock.observe_video(ts(123)),
            Ok(VideoSteer::Snapped { error_ns: 0 })
        );
        assert_eq!(clock.now().ticks, 123);
        assert!(clock.try_now().is_ok());
    }

    /// An extreme playback rate under the backend surface: the
    /// extrapolation stays checked (i128) and the position
    /// saturates rather than wrapping.
    #[test]
    fn extreme_rate_saturates_rather_than_wrapping_under_switching() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let huge = Rate::new(u32::MAX, 1).unwrap_or_else(|e| unreachable!("{e}"));
        let clock = MediaClock::with_backends(
            backends(wall).video(true),
            Timestamp::new(i64::MAX / 2, NANO_BASE),
            huge,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        // 2 s of wall at a ~4.29e9x rate: ~8.6e18 ns of media time
        // on top of a half-range anchor — past i64::MAX.
        wall.advance_ns(2_000_000_000);
        assert_eq!(clock.now().ticks, i64::MAX); // saturated, monotone
        assert!(matches!(clock.try_now(), Err(TimeError::Overflow { .. })));
        // Switching under saturation still carries the saturated
        // position (frozen truth at the clamp).
        assert_eq!(
            clock.on_master_lost(MasterStream::Video),
            MasterLoss::Demoted {
                from: VideoOutput,
                to: FreeRunning
            }
        );
        assert_eq!(clock.now().ticks, i64::MAX);
    }

    // ---- the event ring's bounded-overflow receipt ---------------------------

    #[test]
    fn event_ring_overflow_counts_instead_of_growing() {
        let mut ring = EventRing::new();
        let event = MasterChanged {
            from: AudioOutput,
            to: FreeRunning,
            reason: MasterChangeReason::MasterLost(MasterStream::Audio),
            position: ts(1),
            anchor_epoch: 1,
            generation: GenerationId::FIRST,
        };
        for _ in 0..MASTER_EVENT_CAPACITY {
            ring.push(event);
        }
        ring.push(event); // overflow: counted, not stored
        assert_eq!(ring.lost, 1);
        assert_eq!(ring.len, MASTER_EVENT_CAPACITY);
        // FIFO drain of the original 8.
        for i in 0..MASTER_EVENT_CAPACITY {
            assert_eq!(ring.pop(), Some(event), "drain {i}");
        }
        assert_eq!(ring.pop(), None);
    }

    /// Through the public API: a switch storm without draining
    /// fills the ring and surfaces the lost-count.
    #[test]
    fn switch_storm_without_drain_surfaces_the_lost_count() {
        let wall = std::sync::Arc::new(FakeMonotonic::new());
        let clock = MediaClock::with_backends(
            backends(wall)
                .audio(std::sync::Arc::new(FakeCounter::new(48_000)))
                .video(true),
            ts(0),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        for i in 0..(MASTER_EVENT_CAPACITY as u64 + 1) {
            let target = if i % 2 == 0 {
                MasterTarget::Video
            } else {
                MasterTarget::Audio
            };
            let _ = clock.switch_master(target);
        }
        assert_eq!(clock.master_changes_lost(), 1);
        // The drained events are the first 8, in order.
        let mut drained = 0;
        while clock.take_master_change().is_some() {
            drained += 1;
        }
        assert_eq!(drained, MASTER_EVENT_CAPACITY);
    }

    // ---- vocabulary ----------------------------------------------------------

    #[test]
    fn vocabulary_strings_are_stable() {
        assert_eq!(MasterStream::Audio.as_str(), "audio");
        assert_eq!(MasterStream::Video.as_str(), "video");
        assert_eq!(MasterStream::External.as_str(), "external");
        assert_eq!(
            MasterChangeReason::MasterLost(MasterStream::Audio).as_str(),
            "master-lost"
        );
        assert_eq!(MasterChangeReason::EngineSwitch.as_str(), "engine-switch");
        assert_eq!(MasterTarget::Audio.as_str(), "audio");
        assert_eq!(AudioOutput.to_string(), "audio-output");
        assert_eq!(VideoOutput.to_string(), "video-output");
        assert_eq!(External.to_string(), "external");
        assert_eq!(FreeRunning.to_string(), "free-running");
        assert_eq!(SystemMonotonic.to_string(), "system-monotonic");
    }
}
