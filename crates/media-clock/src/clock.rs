//! The audio-master clock (card 2.C1; doc 03 §4.7; doc 01 §2.2 "one
//! clock" invariant 8).
//!
//! [`MediaClock`] provides `now()` for the whole pipeline, anchored to
//! the audio output position when audio exists ([`ClockMaster::
//! AudioOutput`]), falling back explicitly to an injected monotonic
//! source when it does not ([`ClockMaster::SystemMonotonic`] — the UI
//! is told; truthful state, doc 03 §4.7 d). Playback time derives from
//! the sample counter — samples consumed by the output — never from a
//! wall clock; a wall/virtual time source is injectable for the
//! fallback and for tests (doc 03 §8).
//!
//! # Semantics (doc 03 §4.7 e — binding)
//!
//! - **Audio-master**: `now() = anchor + (counter_elapsed × rate)`
//!   within one anchor epoch. The device sample counter is the only
//!   trusted progress source; video never advances the clock.
//! - **Monotonic**: within one anchor epoch `now()` never goes
//!   backwards — equal at worst. On [`PlaybackClock::reanchor`] (open,
//!   seek, discontinuity) a jump is expected; the epoch in
//!   [`ClockSnapshot`] separates the stretches.
//! - **Pause**: `now()` freezes. Resume continues from the same media
//!   time with no discontinuity — even when the underlying source kept
//!   running while paused (the clock re-anchors at the frozen position;
//!   it does not trust the source to have frozen).
//! - **Discontinuity**: a source that regresses below the anchor or
//!   changes its sample rate (device switch) makes the arithmetic
//!   meaningless; [`PlaybackClock::now`] then *holds* its last position
//!   (monotonic: equal at worst) and [`PlaybackClock::try_now`]
//!   reports [`TimeError::Discontinuity`] with full context until the
//!   session re-anchors — arithmetic across a discontinuity without an
//!   explicit clock reset is an error, never silent invention (doc 01
//!   §3.2).
//! - **Overflow**: extreme sample counts and rates saturate `now()` at
//!   the nano base's `i64` bound (monotonic even at the clamp) and are
//!   reported by `try_now()` as [`TimeError::Overflow`] (doc 03 §4.7 f).
//!
//! # Threading (doc 03 §4.7 c)
//!
//! No dedicated thread. One short internal [`Mutex`] guards the anchor;
//! critical sections are one source read plus pure checked arithmetic —
//! never held across I/O (sources contractually non-blocking,
//! [`crate::source`]). Poisoning cannot originate here (no panicking
//! operation inside); a poisoned lock is recovered via
//! `PoisonError::into_inner` (possibly-stale, structurally-valid
//! state). The lock-free/seqlock reader of doc 03 §4.7(c) is Phase-3
//! hardening; the short-lock v0 honors the "never across I/O" clause.
//!
//! # Interface deviations from the sketches (recorded in the worklog)
//!
//! - Positions are [`Timestamp`]s in [`crate::NANO_BASE`] — doc 03 §3's
//!   `MediaTime` analog — not a new `MediaTime` type (reuse
//!   media-types' vocabulary, card constraint).
//! - `set_rate`/`reanchor` take [`Rate`] / [`Timestamp`], and return
//!   [`Result`] (range-check per the sketch's "range-checked"; overflow
//!   via the taxonomy).
//! - [`ClockSnapshot`] carries the doc 01 §2.2 fields plus `master`
//!   (which truth source) and `anchor_epoch` (the clock's own
//!   re-anchor counter, for per-epoch monotonicity reasoning).
//! - [`ClockAnchor::at_wall_ns`] replaces the sketch's
//!   `at_instant: Option<Instant>`: the wall source is injectable
//!   (virtual time in tests), so no concrete `Instant` is stored.
//! - Card 3.B2 extends the doc's two-variant `ClockMaster` sketch
//!   with [`ClockMaster::VideoOutput`], [`ClockMaster::External`]
//!   and [`ClockMaster::FreeRunning`] (the card's own scope), and
//!   [`ClockState`] with [`ClockState::Ended`]. The 3.B1
//!   single-master constructors and their semantics are unchanged;
//!   the multi-backend surface is strictly opt-in via
//!   [`MediaClock::with_backends`] (see [`crate::backends`]).

use std::sync::{Arc, Mutex};

use media_types::GenerationId;
use media_types::time::{Rate, TimeError, Timestamp};

use crate::backends::{BackendCore, MonotonicKind};
use crate::mapping::{NANO_BASE, timestamp_to_nanos};
use crate::source::{MonotonicSource, SampleCounterSource};

/// Which truth source drives the clock (doc 03 §4.7 b; card 3.B2
/// extends the doc's two-variant sketch with the backend modes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClockMaster {
    /// The audio output's sample counter — the default when audio
    /// exists (MP §3). `ClockMasterChanged` events on fallback are the
    /// playback-engine's to emit (doc 03 §4.7 d); this crate has no
    /// event bus by design (L2, no I/O) — the typed records the
    /// engine wraps live in [`crate::backends::MasterChanged`].
    AudioOutput,
    /// An injected monotonic source — the explicit fallback for
    /// audio-less media (doc 01 §1.4 invariant 8 `[PROPOSAL]`).
    SystemMonotonic,
    /// Video frames drive the clock (card 3.B2): the clock
    /// interpolates on the injected monotonic source between
    /// [`MediaClock::observe_video`] steers, and the audio side
    /// resyncs to video within bounded limits (the existing ±1000
    /// ppm [`crate::DriftCorrection`] budget — no new mechanism).
    VideoOutput,
    /// An external device clock (a WASAPI/CoreAudio-style clock
    /// domain) drives the clock — modeled as an injectable monotonic
    /// source (card 3.B2).
    External,
    /// Free-running wall drift — the terminal master-loss fallback
    /// (card 3.B2): no master stream remains, the clock coasts on
    /// wall time, and the drift vs media time is unbounded and
    /// uncorrectable. The [`crate::backends::MasterChanged`] trail
    /// says so; the clock never fabricates a seamless master.
    FreeRunning,
}

impl std::fmt::Display for ClockMaster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ClockMaster::AudioOutput => "audio-output",
            ClockMaster::SystemMonotonic => "system-monotonic",
            ClockMaster::VideoOutput => "video-output",
            ClockMaster::External => "external",
            ClockMaster::FreeRunning => "free-running",
        })
    }
}

/// The clock's run state (doc 01 §2.2 snapshot field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClockState {
    /// The counter advances; `now()` follows it.
    Running,
    /// The position is frozen; resume continues from it.
    Paused,
    /// Playback reached the end and the clock was told
    /// ([`MediaClock::mark_ended`], card 3.B2): the position is
    /// frozen at the end and stays frozen — the only re-arm is
    /// [`MediaClock::seek_back`] (paused at the new position).
    /// `resume()` is a no-op in this state: continuing past the end
    /// without a seek would be a fabricated position.
    Ended,
}

/// One consistent read of the clock (doc 01 §2.2 interface sketch):
/// everything a scheduler needs in a single lock-ordered snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockSnapshot {
    /// The session generation the engine last set
    /// ([`MediaClock::set_generation`]); stale-generation consumers
    /// drop their data (doc 01 §3.2, generation-scoped resets).
    pub generation: GenerationId,
    /// The position — same value `now()` would return.
    pub position: Timestamp,
    /// The current playback rate.
    pub rate: Rate,
    /// Running or paused.
    pub state: ClockState,
    /// Which truth source drives the clock.
    pub master: ClockMaster,
    /// The clock's own anchor epoch: bumped on every re-anchor
    /// (reanchor, set_rate, resume, master switches) and on
    /// out-of-bound video steers ([`MediaClock::observe_video`]).
    /// Monotonicity of `position` is guaranteed *within* one epoch;
    /// across epochs a jump is expected (seek) or impossible by
    /// construction (rate change, resume, master switch — the
    /// position carries). Card 3.B2 refinement: an in-bound video
    /// snap moves the anchor **without** an epoch bump — the
    /// monotone clamp makes it invisible (ahead: advances within
    /// the snap bound; behind: holds until the wall catches up), so
    /// bumping would advertise a jump that never happens. The epoch
    /// advertises jumps, not anchor arithmetic.
    pub anchor_epoch: u64,
}

/// The current anchor (doc 03 §4.7 b), for introspection. Which half
/// is set depends on the master: audio anchors carry `at_sample`,
/// monotonic anchors carry `at_wall_ns`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockAnchor {
    /// The media position the anchor was taken at.
    pub media: Timestamp,
    /// The sample counter at the anchor (audio master).
    pub at_sample: Option<u64>,
    /// The monotonic source's nanoseconds at the anchor (fallback
    /// master) — `at_instant` in the sketch, made injectable.
    pub at_wall_ns: Option<u64>,
    /// The playback rate in force from this anchor.
    pub rate: Rate,
    /// Running or paused at the time of the read.
    pub state: ClockState,
}

/// The pipeline-facing clock contract (doc 03 §4.7 b + doc 01 §2.2).
///
/// Object-safe; hold it as `Arc<dyn PlaybackClock>`. Bring the trait
/// into scope to call the methods on [`MediaClock`].
pub trait PlaybackClock: Send + Sync {
    /// The current media position, in [`crate::NANO_BASE`] ticks
    /// (nanoseconds). Monotonic within an anchor epoch — never
    /// backwards, equal at worst; saturates (never wraps) on extreme
    /// inputs; holds its last value across a source discontinuity.
    /// The hot read path: infallible by design (doc 03 §4.7 b).
    fn now(&self) -> Timestamp;

    /// The typed-error read (card 2.C1): like [`PlaybackClock::now`],
    /// but reports the taxonomy instead of saturating/holding —
    /// [`TimeError::Discontinuity`] when the source regressed or
    /// changed rate since the anchor (arithmetic across a
    /// discontinuity without a reset, doc 01 §3.2), and
    /// [`TimeError::Overflow`] when the extrapolated position cannot
    /// be represented.
    ///
    /// # Errors
    /// See above; `Ok` exactly when the position is sound.
    fn try_now(&self) -> Result<Timestamp, TimeError>;

    /// Which truth source drives this clock.
    fn master(&self) -> ClockMaster;

    /// Re-anchor at `media` — used at open, seek and after a
    /// discontinuity (doc 03 §4.7 b). Starts a new anchor epoch: a
    /// position jump (either direction) is expected and legal
    /// immediately after. While paused, the freeze moves to `media`.
    ///
    /// # Errors
    /// [`TimeError::Overflow`] when `media` cannot be represented as
    /// a nanosecond-base position (checked; the clock is unchanged on
    /// error).
    fn reanchor(&self, media: Timestamp) -> Result<(), TimeError>;

    /// Change the playback rate, re-anchoring at the *current*
    /// position so the change never jumps it (doc 03 §4.7 b
    /// "range-checked": a zero-component [`Rate`] — constructible via
    /// the pub fields, bypassing `Rate::new` — is rejected with the
    /// same typed error `Rate::new` produces). Takes effect for all
    /// progress after the call; while paused, the frozen position is
    /// kept and the new rate applies from resume.
    ///
    /// # Errors
    /// [`media_types::EngineError`] (kind `Unsupported`, stage
    /// `Control`) on a zero-component rate — `Rate::new`'s error.
    fn set_rate(&self, rate: Rate) -> Result<(), media_types::EngineError>;

    /// One consistent read: position, rate, state, master, generation
    /// and anchor epoch, ordered by the internal lock (doc 01 §2.2).
    fn snapshot(&self) -> ClockSnapshot;
}

/// The injected truth, boxed by constructor choice. Card 3.B2: the
/// source lives inside [`Core`]'s lock so a master switch swaps it
/// atomically with the re-anchor it performs — a read of an old
/// source can never be extrapolated against a new master's anchor.
pub(crate) enum Source {
    Samples(Arc<dyn SampleCounterSource>),
    /// A monotonic-nanosecond source, labeled by the master role it
    /// plays right now (card 3.B2). The reading math is identical
    /// for all four roles; the label is the truthful state the UI
    /// and the fallback policy reason over.
    Monotonic {
        source: Arc<dyn MonotonicSource>,
        kind: MonotonicKind,
    },
}

impl Source {
    /// The truthful master label this source currently plays.
    pub(crate) fn master(&self) -> ClockMaster {
        match self {
            Source::Samples(_) => ClockMaster::AudioOutput,
            Source::Monotonic {
                kind: MonotonicKind::System,
                ..
            } => ClockMaster::SystemMonotonic,
            Source::Monotonic {
                kind: MonotonicKind::VideoWall,
                ..
            } => ClockMaster::VideoOutput,
            Source::Monotonic {
                kind: MonotonicKind::External,
                ..
            } => ClockMaster::External,
            Source::Monotonic {
                kind: MonotonicKind::Free,
                ..
            } => ClockMaster::FreeRunning,
        }
    }
}

/// One reading of the source, converted to its own nanosecond scale
/// (card 3.B2: the backend switch inspects readability).
pub(crate) enum Reading {
    /// Interpretable: counter nanoseconds (its own scale — audio: from
    /// the sample count; monotonic: the reading itself), the raw
    /// sample count, and the rate it was scaled by. May be below the
    /// anchor (regression — judged by the caller).
    Usable { ns: i128, sample: u64, rate_hz: u32 },
    /// Not interpretable (audio source reporting a zero sample rate —
    /// a broken device). The clock holds its position and reports a
    /// discontinuity rather than guessing.
    Unusable,
}

/// The mutable core. Every field is written only inside the lock;
/// critical sections are one source read plus pure checked
/// arithmetic.
pub(crate) struct Core {
    /// Engine-set session generation (snapshots, error context).
    pub(crate) generation: GenerationId,
    /// The anchored media position (nanoseconds, i64 range by
    /// construction — reanchor checks).
    pub(crate) anchor_media_ns: i64,
    /// The source's own scale at the anchor (nanoseconds). i128: the
    /// raw sample-to-ns conversion can legitimately exceed i64 for
    /// extreme counts; the *position* clamps in i64, the anchor does
    /// not need to.
    pub(crate) anchor_source_ns: i128,
    /// The raw sample count at the anchor (introspection; audio only).
    pub(crate) anchor_sample: u64,
    /// The sample rate the anchor was taken against (audio only;
    /// monotonic masters anchor with 0 = "no sample scale").
    pub(crate) anchor_rate_hz: u32,
    /// The playback rate in force (validated: non-zero components).
    pub(crate) rate: Rate,
    /// Paused: `now()` returns `frozen_ns`.
    pub(crate) paused: bool,
    /// The frozen position while paused (and while the Ended latch
    /// holds, card 3.B2).
    pub(crate) frozen_ns: i64,
    /// The last source reading successfully extrapolated from (its own
    /// nanosecond scale). Regression detection compares against THIS,
    /// not the anchor: a device reset from a mid-stream counter back to
    /// zero must be caught even when the anchor was taken at zero.
    /// Updated only inside the lock, only for sound readings.
    pub(crate) last_source_ns: i128,
    /// The last value returned by `now()` — the monotonic clamp.
    pub(crate) last_ns: i64,
    /// The anchor-epoch counter (bumped on every re-anchor).
    pub(crate) epoch: u64,
    /// The truth source (card 3.B2: inside the lock — see [`Source`]).
    pub(crate) source: Source,
    /// Card 3.B2 backend state: the wired fallback sources, the loss
    /// bitset, the video snap bound, the Ended latch and the bounded
    /// master-change event ring.
    pub(crate) backend: BackendCore,
    /// A master switch landed on a source that could not yet be read
    /// (an audio backend reporting a zero sample rate): the clock
    /// holds the carried position and adopts the source's **first**
    /// usable reading as the fresh anchor baseline — no jump, no
    /// false discontinuity from comparing a new counter domain
    /// against the old master's anchor (card 3.B2; construction
    /// keeps 3.B1's hold-until-re-anchor semantics instead).
    pub(crate) awaiting_anchor: bool,
}

/// What the extrapolation over one reading produced.
pub(crate) enum Extrapolation {
    Paused(i64),
    /// The raw position (i128; the caller saturates or reports
    /// overflow against the i64 position range).
    Running(i128),
    /// The source broke monotonicity or changed scale since the
    /// anchor; `found_ns` is the best-effort raw extrapolation for
    /// error context.
    Discontinuity {
        found_ns: i128,
    },
}

impl Core {
    /// Extrapolate the position for `reading` under this anchor.
    /// Pure: no source reads, no mutation.
    pub(crate) fn extrapolate(&self, reading: &Reading) -> Extrapolation {
        // The Ended latch (card 3.B2) freezes exactly like a pause —
        // but only [`MediaClock::seek_back`] clears it.
        if self.paused || self.backend.ended {
            return Extrapolation::Paused(self.frozen_ns);
        }
        let Reading::Usable { ns, rate_hz, .. } = reading else {
            return Extrapolation::Discontinuity {
                found_ns: i128::from(self.last_ns),
            };
        };
        if *ns < self.last_source_ns || *rate_hz != self.anchor_rate_hz {
            // Regression (below the last sound reading) or scale
            // change: still compute the naive value for diagnostic
            // context (it is meaningless as a position — that is
            // precisely the discontinuity).
            return Extrapolation::Discontinuity {
                found_ns: naive_position(self, *ns),
            };
        }
        // delta >= 0 here; media_delta = delta x num / den, checked.
        let delta = ns - self.anchor_source_ns;
        let Some(scaled) = delta.checked_mul(i128::from(self.rate.num)) else {
            return Extrapolation::Running(i128::MAX);
        };
        let Some(media_delta) = checked_div_round(scaled, i128::from(self.rate.den)) else {
            return Extrapolation::Running(i128::MAX);
        };
        match i128::from(self.anchor_media_ns).checked_add(media_delta) {
            Some(raw) => Extrapolation::Running(raw),
            None => Extrapolation::Running(i128::MAX),
        }
    }
}

/// The unchecked (diagnostic) extrapolation used for `found` context.
fn naive_position(core: &Core, source_ns: i128) -> i128 {
    let delta = source_ns.saturating_sub(core.anchor_source_ns);
    let scaled = delta.saturating_mul(i128::from(core.rate.num));
    let media_delta = checked_div_round(scaled, i128::from(core.rate.den)).unwrap_or(i128::MAX);
    i128::from(core.anchor_media_ns).saturating_add(media_delta)
}

/// Divide with rounding, ties away from zero; `None` on a degenerate
/// denominator (the `media-types` discipline; the rate is validated at
/// the boundaries so this is exhaustive-checkedness only).
fn checked_div_round(numer: i128, denom: i128) -> Option<i128> {
    if denom <= 0 {
        return None;
    }
    let (q, r) = (numer / denom, numer % denom);
    let twice_r = r.checked_mul(2)?;
    if twice_r >= denom {
        q.checked_add(1)
    } else if twice_r <= -denom {
        q.checked_sub(1)
    } else {
        Some(q)
    }
}

/// Convert an anchor media position to its i64 nanosecond form
/// (checked both ways; the clock's positions live in the nano base's
/// `i64` tick range).
///
/// # Errors
/// [`TimeError::Overflow`] when `media` cannot be represented.
pub(crate) fn position_nanos(media: Timestamp) -> Result<i64, TimeError> {
    i64::try_from(timestamp_to_nanos(media)?).map_err(|_| TimeError::Overflow {
        from: media.base,
        to: NANO_BASE,
        ticks: media.ticks,
    })
}

/// Saturate an i128 nanosecond value into the i64 position range.
pub(crate) fn saturate_position(ns: i128) -> i64 {
    ns.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

/// The clock's run state under this core (card 3.B2: the Ended
/// latch outranks the pause flag).
pub(crate) fn clock_state(core: &Core) -> ClockState {
    if core.backend.ended {
        ClockState::Ended
    } else if core.paused {
        ClockState::Paused
    } else {
        ClockState::Running
    }
}

/// The clock (doc 03 §4.7). Construct with [`MediaClock::audio_master`],
/// [`MediaClock::system_monotonic`] or — for the card 3.B2 backend
/// surface (video/external masters, master-loss fallback, the Ended
/// latch) — [`MediaClock::with_backends`].
pub struct MediaClock {
    pub(crate) core: Mutex<Core>,
}

impl MediaClock {
    /// An audio-master clock anchored at `media` and the source's
    /// current reading, running at `rate` (doc 03 §4.7 e: the device
    /// sample counter is the only trusted progress source).
    ///
    /// # Errors
    /// [`media_types::EngineError`] — an unrepresentable `media`
    /// position arrives as the Clock-stage conversion of
    /// [`TimeError::Overflow`] (media-types' prescribed mapping), a
    /// zero-component [`Rate`] as `Rate::new`'s `Unsupported` error
    /// (defensive re-validation, see [`PlaybackClock::set_rate`]).
    pub fn audio_master(
        source: Arc<dyn SampleCounterSource>,
        media: Timestamp,
        rate: Rate,
    ) -> Result<MediaClock, media_types::EngineError> {
        Self::build(
            Source::Samples(source.clone()),
            BackendCore::no_fallback(),
            media,
            rate,
        )
    }

    /// The explicit fallback master for audio-less media (doc 03 §4.7
    /// d): the injected monotonic source drives progress. Tests inject
    /// virtual time here (doc 03 §8) — production wires
    /// [`crate::InstantSource`] or the platform's monotonic clock.
    ///
    /// # Errors
    /// As [`MediaClock::audio_master`].
    pub fn system_monotonic(
        source: Arc<dyn MonotonicSource>,
        media: Timestamp,
        rate: Rate,
    ) -> Result<MediaClock, media_types::EngineError> {
        Self::build(
            Source::Monotonic {
                source,
                kind: MonotonicKind::System,
            },
            BackendCore::no_fallback(),
            media,
            rate,
        )
    }

    pub(crate) fn build(
        source: Source,
        backend: BackendCore,
        media: Timestamp,
        rate: Rate,
    ) -> Result<MediaClock, media_types::EngineError> {
        let media_ns = position_nanos(media)?;
        let validated = crate::validate_rate(rate)?;
        let reading = read_source(&source);
        // `adopt_anchor` seeds both the anchor and the regression
        // baseline from the reading; an unusable reading (zero sample
        // rate — a broken device) leaves the zero anchors from
        // `anchor_core`, so every read holds/reports until a
        // re-anchor: the honest behavior, never a guess.
        let mut core = anchor_core(media_ns, validated, source, backend);
        adopt_anchor(&mut core, &reading);
        Ok(MediaClock {
            core: Mutex::new(core),
        })
    }

    /// Freeze the position (idempotent). The frozen value survives a
    /// source that keeps running while paused — the clock does not
    /// trust the source to freeze (doc 03 §4.7 e defines pause via the
    /// *device* pausing; the clock enforces it regardless). Under the
    /// Ended latch (card 3.B2) this is a no-op: the ended freeze is
    /// terminal and only [`MediaClock::seek_back`] re-arms it.
    pub fn pause(&self) {
        let mut core = self.lock_core();
        if core.paused || core.backend.ended {
            return;
        }
        let reading = read_source(&core.source);
        core.frozen_ns = now_locked(&mut core, &reading);
        core.paused = true;
    }

    /// Continue from the frozen position (idempotent): re-anchors at
    /// (frozen position, current source reading) so progress resumes
    /// with **no discontinuity** (card 2.C1 AC). Under the Ended
    /// latch (card 3.B2) this is a no-op — resuming past the end
    /// without a seek would fabricate a position; the re-arm is
    /// [`MediaClock::seek_back`].
    pub fn resume(&self) {
        let mut core = self.lock_core();
        if !core.paused || core.backend.ended {
            return;
        }
        let reading = read_source(&core.source);
        core.anchor_media_ns = core.frozen_ns;
        adopt_anchor(&mut core, &reading);
        core.paused = false;
        core.epoch += 1;
    }

    /// Whether the clock is paused (a clock in the Ended latch
    /// reports `false`: Ended is the terminal freeze, not a pause —
    /// see [`ClockState::Ended`]).
    pub fn is_paused(&self) -> bool {
        self.lock_core().paused
    }

    /// Record the session generation (the engine mints generations —
    /// doc 01 §3.1; the clock only carries it in snapshots and
    /// discontinuity context). No epoch bump: generation changes
    /// accompany open/seek, which re-anchor anyway.
    pub fn set_generation(&self, generation: GenerationId) {
        self.lock_core().generation = generation;
    }

    /// The current anchor, for introspection (doc 03 §4.7 b).
    pub fn anchor(&self) -> ClockAnchor {
        let core = self.lock_core();
        let (at_sample, at_wall_ns) = match &core.source {
            Source::Samples(_) => (Some(core.anchor_sample), None),
            Source::Monotonic { .. } => (
                None,
                Some(saturate_position(core.anchor_source_ns.max(0)) as u64),
            ),
        };
        ClockAnchor {
            media: Timestamp::new(core.anchor_media_ns, NANO_BASE),
            at_sample,
            at_wall_ns,
            rate: core.rate,
            state: clock_state(&core),
        }
    }

    /// Lock the core, recovering from poisoning (critical sections are
    /// pure checked arithmetic — poisoning cannot originate here; a
    /// foreign panic leaves at worst a stale-but-valid anchor).
    pub(crate) fn lock_core(&self) -> std::sync::MutexGuard<'_, Core> {
        match self.core.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

/// One consistent snapshot computed under a held lock (shared by
/// `PlaybackClock::snapshot`, [`MediaClock::mark_ended`] and
/// [`MediaClock::seek_back`]).
pub(crate) fn snapshot_locked(core: &mut Core) -> ClockSnapshot {
    let reading = read_source(&core.source);
    let position = Timestamp::new(now_locked(core, &reading), NANO_BASE);
    ClockSnapshot {
        generation: core.generation,
        position,
        rate: core.rate,
        state: clock_state(core),
        master: core.source.master(),
        anchor_epoch: core.epoch,
    }
}

/// A fresh anchor core at `media_ns` with `rate` (card 3.B2: the
/// source and backend state come with it).
fn anchor_core(media_ns: i64, rate: Rate, source: Source, backend: BackendCore) -> Core {
    Core {
        generation: GenerationId::FIRST,
        anchor_media_ns: media_ns,
        anchor_source_ns: 0,
        anchor_sample: 0,
        anchor_rate_hz: 0,
        rate,
        paused: false,
        frozen_ns: media_ns,
        last_source_ns: 0,
        last_ns: media_ns,
        epoch: 0,
        source,
        backend,
        awaiting_anchor: false,
    }
}

/// Adopt a reading as the new anchor reference (reanchor/resume/
/// set_rate). An unusable reading keeps the previous source anchor:
/// the position stays held/reported until the source recovers and the
/// session re-anchors.
pub(crate) fn adopt_anchor(core: &mut Core, reading: &Reading) {
    if let Reading::Usable {
        ns,
        sample,
        rate_hz,
    } = reading
    {
        core.anchor_source_ns = *ns;
        core.anchor_sample = *sample;
        core.anchor_rate_hz = *rate_hz;
        core.last_source_ns = *ns;
    }
}

/// Read the source (non-blocking by contract; card 3.B2: callers
/// hold the core lock — the read is part of the critical section
/// doc 03 §4.7 c already describes, "one source read plus pure
/// checked arithmetic").
pub(crate) fn read_source(source: &Source) -> Reading {
    match source {
        Source::Samples(samples) => {
            let sample = samples.sample_count();
            let rate_hz = samples.sample_rate_hz();
            if rate_hz == 0 {
                return Reading::Unusable;
            }
            // ns = sample x 1e9 / rate_hz, checked; a zero rate is
            // handled above so the division is well-defined.
            let numer = i128::from(sample).checked_mul(1_000_000_000);
            match numer.and_then(|n| checked_div_round(n, i128::from(rate_hz))) {
                Some(ns) => Reading::Usable {
                    ns,
                    sample,
                    rate_hz,
                },
                None => Reading::Unusable,
            }
        }
        Source::Monotonic { source, .. } => Reading::Usable {
            ns: i128::from(source.now_ns()),
            sample: 0,
            rate_hz: 0,
        },
    }
}

/// Adopt the first usable reading after a master switch landed on an
/// unreadable source (card 3.B2): fresh baseline at the carried
/// position — no jump, and no false discontinuity from comparing a
/// new counter domain against the old master's anchor.
pub(crate) fn adopt_if_awaiting(core: &mut Core, reading: &Reading) {
    if core.awaiting_anchor {
        if let Reading::Usable {
            ns,
            sample,
            rate_hz,
        } = reading
        {
            core.anchor_source_ns = *ns;
            core.anchor_sample = *sample;
            core.anchor_rate_hz = *rate_hz;
            core.last_source_ns = *ns;
            core.awaiting_anchor = false;
        }
    }
}

/// The clamped `now()` computation, shared by `now`, `snapshot` and
/// `pause`. Updates the monotonic clamp (`last_ns`) and, for sound
/// readings, the regression baseline (`last_source_ns`).
pub(crate) fn now_locked(core: &mut Core, reading: &Reading) -> i64 {
    adopt_if_awaiting(core, reading);
    match core.extrapolate(reading) {
        Extrapolation::Paused(pos) => {
            if pos > core.last_ns {
                core.last_ns = pos;
            }
            pos
        }
        Extrapolation::Running(raw) => {
            if let Reading::Usable { ns, .. } = reading {
                core.last_source_ns = *ns;
            }
            let pos = saturate_position(raw).max(core.last_ns);
            core.last_ns = pos;
            pos
        }
        Extrapolation::Discontinuity { .. } => core.last_ns,
    }
}

/// The typed extrapolation for `try_now` (no position clamping; the
/// regression baseline does advance — the reading was sound).
pub(crate) fn try_now_locked(core: &mut Core, reading: &Reading) -> Result<Timestamp, TimeError> {
    adopt_if_awaiting(core, reading);
    match core.extrapolate(reading) {
        Extrapolation::Paused(pos) => Ok(Timestamp::new(pos, NANO_BASE)),
        Extrapolation::Running(raw) => {
            if let Reading::Usable { ns, .. } = reading {
                core.last_source_ns = *ns;
            }
            let ticks = i64::try_from(raw).map_err(|_| TimeError::Overflow {
                from: NANO_BASE,
                to: NANO_BASE,
                ticks: saturate_position(raw),
            })?;
            Ok(Timestamp::new(ticks, NANO_BASE))
        }
        Extrapolation::Discontinuity { found_ns } => Err(TimeError::Discontinuity {
            expected: Some(Timestamp::new(core.anchor_media_ns, NANO_BASE)),
            found: Timestamp::new(saturate_position(found_ns), NANO_BASE),
            generation: core.generation,
        }),
    }
}

impl PlaybackClock for MediaClock {
    fn now(&self) -> Timestamp {
        let mut core = self.lock_core();
        let reading = read_source(&core.source);
        Timestamp::new(now_locked(&mut core, &reading), NANO_BASE)
    }

    fn try_now(&self) -> Result<Timestamp, TimeError> {
        let mut core = self.lock_core();
        let reading = read_source(&core.source);
        try_now_locked(&mut core, &reading)
    }

    fn master(&self) -> ClockMaster {
        self.lock_core().source.master()
    }

    fn reanchor(&self, media: Timestamp) -> Result<(), TimeError> {
        let media_ns = position_nanos(media)?;
        let mut core = self.lock_core();
        let reading = read_source(&core.source);
        core.anchor_media_ns = media_ns;
        adopt_anchor(&mut core, &reading);
        core.epoch += 1;
        // A re-anchor jump is expected: reset the monotonic clamp to
        // the new position, and move the freeze if paused. Card 3.B2:
        // the Ended latch survives a re-anchor (the freeze moves; only
        // `seek_back` re-arms) — a plain re-anchor must never turn an
        // ended clock back into a free-running one.
        core.last_ns = media_ns;
        if core.paused || core.backend.ended {
            core.frozen_ns = media_ns;
        }
        Ok(())
    }

    fn set_rate(&self, rate: Rate) -> Result<(), media_types::EngineError> {
        let validated = crate::validate_rate(rate)?;
        let mut core = self.lock_core();
        let reading = read_source(&core.source);
        // Re-anchor at the current (clamped) position: the change never
        // jumps it. While paused (or ended, card 3.B2) the frozen
        // position is kept.
        let pos = now_locked(&mut core, &reading);
        core.anchor_media_ns = pos;
        if core.paused || core.backend.ended {
            core.frozen_ns = pos;
        }
        adopt_anchor(&mut core, &reading);
        core.rate = validated;
        core.epoch += 1;
        Ok(())
    }

    fn snapshot(&self) -> ClockSnapshot {
        let mut core = self.lock_core();
        snapshot_locked(&mut core)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A deterministic fake counter (unit-level; the acceptance tests
    /// use testkit virtual time — `tests/virtual_time.rs`).
    struct FakeCounter {
        sample: AtomicU64,
        rate_hz: u32,
    }

    impl FakeCounter {
        fn new(rate_hz: u32) -> FakeCounter {
            FakeCounter {
                sample: AtomicU64::new(0),
                rate_hz,
            }
        }

        fn advance_samples(&self, samples: u64) {
            let _ = self.sample.fetch_add(samples, Ordering::Relaxed);
        }

        fn reset(&self) {
            self.sample.store(0, Ordering::Relaxed);
        }
    }

    impl SampleCounterSource for FakeCounter {
        fn sample_count(&self) -> u64 {
            self.sample.load(Ordering::Relaxed)
        }

        fn sample_rate_hz(&self) -> u32 {
            self.rate_hz
        }
    }

    /// A deterministic fake monotonic source.
    struct FakeMonotonic(AtomicU64);

    impl MonotonicSource for FakeMonotonic {
        fn now_ns(&self) -> u64 {
            self.0.load(Ordering::Relaxed)
        }
    }

    fn ts(ns: i64) -> Timestamp {
        Timestamp::new(ns, NANO_BASE)
    }

    fn audio_clock(rate: Rate) -> MediaClock {
        MediaClock::audio_master(Arc::new(FakeCounter::new(48_000)), ts(0), rate)
            .unwrap_or_else(|e| unreachable!("{e}"))
    }

    // ---- anchor arithmetic incl. rate != 1 (doc 03 §4.7 f) -------------

    #[test]
    fn anchor_arithmetic_tracks_the_counter() {
        let counter = Arc::new(FakeCounter::new(48_000));
        let clock = MediaClock::audio_master(counter.clone(), ts(1_000_000_000), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(clock.now().ticks, 1_000_000_000);
        counter.advance_samples(48_000);
        assert_eq!(clock.now().ticks, 2_000_000_000);
    }

    #[test]
    fn anchor_arithmetic_scales_by_rate() {
        // rate 5/4: 4 counter-seconds from 0 -> 5 media-seconds.
        let counter = Arc::new(FakeCounter::new(48_000));
        let clock = MediaClock::audio_master(
            counter.clone(),
            ts(0),
            Rate::new(5, 4).unwrap_or_else(|e| unreachable!("{e}")),
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        counter.advance_samples(4 * 48_000);
        assert_eq!(clock.now().ticks, 5_000_000_000);
    }

    #[test]
    fn monotonic_master_tracks_injected_wall_time() {
        let wall = Arc::new(FakeMonotonic(AtomicU64::new(0)));
        let clock = MediaClock::system_monotonic(wall.clone(), ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(clock.master(), ClockMaster::SystemMonotonic);
        let _ = wall.0.fetch_add(2_500_000_000, Ordering::Relaxed);
        assert_eq!(clock.now().ticks, 2_500_000_000);
    }

    // ---- reanchor ---------------------------------------------------------

    #[test]
    fn reanchor_jumps_are_expected_and_epoch_bounded() {
        let clock = audio_clock(Rate::ONE);
        let before = clock.snapshot().anchor_epoch;
        clock
            .reanchor(ts(7_000_000_000))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(clock.now().ticks, 7_000_000_000);
        // Backward seek: legal, expected, epoch separates the stretches.
        clock
            .reanchor(ts(2_000_000_000))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(clock.now().ticks, 2_000_000_000);
        assert_eq!(clock.snapshot().anchor_epoch, before + 2);
    }

    #[test]
    fn reanchor_overflow_leaves_clock_unchanged() {
        let clock = audio_clock(Rate::ONE);
        let position = clock.now();
        // ~9.2e18 ticks of 1000/1 s cannot be an i64 nano position.
        let huge = Timestamp::from_parts(i64::MAX, 1000, 1).unwrap_or_else(|e| unreachable!("{e}"));
        assert!(clock.reanchor(huge).is_err());
        assert_eq!(clock.now(), position);
    }

    // ---- monotonicity -----------------------------------------------------

    #[test]
    fn counter_regression_holds_position_never_backwards() {
        let counter = Arc::new(FakeCounter::new(48_000));
        let clock = MediaClock::audio_master(counter.clone(), ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        counter.advance_samples(96_000);
        let held = clock.now();
        assert_eq!(held.ticks, 2_000_000_000);
        // Device switch: the counter restarts. now() must not regress.
        counter.reset();
        assert_eq!(clock.now(), held);
        counter.advance_samples(24_000);
        assert_eq!(clock.now(), held); // still held: below the anchor
        match clock.try_now() {
            Err(TimeError::Discontinuity {
                expected, found, ..
            }) => {
                assert_eq!(expected.map(|t| t.ticks), Some(0));
                assert!(found.ticks < 0 || found.ticks < held.ticks);
            }
            other => unreachable!("expected Discontinuity, got {other:?}"),
        }
        // The cure is the documented one: re-anchor at the new counter.
        clock
            .reanchor(ts(2_000_000_000))
            .unwrap_or_else(|e| unreachable!("{e}"));
        counter.advance_samples(48_000);
        assert_eq!(clock.now().ticks, 3_000_000_000);
    }

    #[test]
    fn sample_rate_change_is_a_discontinuity() {
        struct SwitchingRate {
            sample: AtomicU64,
            rate: AtomicU64,
        }
        impl SampleCounterSource for SwitchingRate {
            fn sample_count(&self) -> u64 {
                self.sample.load(Ordering::Relaxed)
            }
            fn sample_rate_hz(&self) -> u32 {
                u32::try_from(self.rate.load(Ordering::Relaxed)).unwrap_or(48_000)
            }
        }
        let source = Arc::new(SwitchingRate {
            sample: AtomicU64::new(0),
            rate: AtomicU64::new(48_000),
        });
        let clock = MediaClock::audio_master(source.clone(), ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        let _ = source.sample.fetch_add(48_000, Ordering::Relaxed);
        assert_eq!(clock.now().ticks, 1_000_000_000);
        // Device re-opened at 44100 Hz: same counts, different scale.
        source.rate.store(44_100, Ordering::Relaxed);
        let held = clock.now();
        assert_eq!(held.ticks, 1_000_000_000);
        assert!(clock.try_now().is_err());
    }

    #[test]
    fn broken_source_is_unusable_not_panic() {
        struct ZeroRate(AtomicU64);
        impl SampleCounterSource for ZeroRate {
            fn sample_count(&self) -> u64 {
                self.0.load(Ordering::Relaxed)
            }
            fn sample_rate_hz(&self) -> u32 {
                0
            }
        }
        let clock =
            MediaClock::audio_master(Arc::new(ZeroRate(AtomicU64::new(7))), ts(5), Rate::ONE)
                .unwrap_or_else(|e| unreachable!("{e}"));
        // Anchored, held, reported — never a guess, never a panic.
        assert_eq!(clock.now().ticks, 5);
        assert!(clock.try_now().is_err());
    }

    // ---- overflow (doc 03 §4.7 f) ----------------------------------------

    #[test]
    fn extreme_positions_saturate_monotonically() {
        // Anchor near the top of the i64 nano range; the counter keeps
        // counting. now() saturates at i64::MAX and never wraps or
        // regresses; try_now reports the typed overflow.
        let counter = Arc::new(FakeCounter::new(1));
        let clock = MediaClock::audio_master(
            counter.clone(),
            Timestamp::new(i64::MAX - 100, NANO_BASE),
            Rate::ONE,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        counter.advance_samples(1_000);
        assert_eq!(clock.now().ticks, i64::MAX);
        counter.advance_samples(1_000);
        assert_eq!(clock.now().ticks, i64::MAX); // equal at worst
        assert!(matches!(clock.try_now(), Err(TimeError::Overflow { .. })));
    }

    // ---- rate changes -----------------------------------------------------

    #[test]
    fn set_rate_never_jumps_and_rescales() {
        let counter = Arc::new(FakeCounter::new(48_000));
        let clock = MediaClock::audio_master(counter.clone(), ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        counter.advance_samples(48_000);
        assert_eq!(clock.now().ticks, 1_000_000_000);
        clock
            .set_rate(Rate::new(2, 1).unwrap_or_else(|e| unreachable!("{e}")))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(clock.now().ticks, 1_000_000_000); // no jump
        counter.advance_samples(48_000);
        assert_eq!(clock.now().ticks, 3_000_000_000); // 2x from here
    }

    #[test]
    fn set_rate_rejects_zero_components() {
        let clock = audio_clock(Rate::ONE);
        assert!(clock.set_rate(Rate { num: 0, den: 1 }).is_err());
        assert!(clock.set_rate(Rate { num: 1, den: 0 }).is_err());
        assert_eq!(clock.now().ticks, 0);
    }

    // ---- pause / resume ---------------------------------------------------

    #[test]
    fn pause_freezes_resume_continues_without_jump() {
        let counter = Arc::new(FakeCounter::new(48_000));
        let clock = MediaClock::audio_master(counter.clone(), ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        counter.advance_samples(48_000);
        clock.pause();
        let frozen = clock.now();
        assert_eq!(frozen.ticks, 1_000_000_000);
        // The source keeps running while paused — the clock must not.
        counter.advance_samples(10 * 48_000);
        assert_eq!(clock.now(), frozen);
        clock.resume();
        assert_eq!(clock.now(), frozen); // no discontinuity on resume
        counter.advance_samples(48_000);
        assert_eq!(clock.now().ticks, 2_000_000_000); // continues
    }

    #[test]
    fn pause_and_resume_are_idempotent() {
        let clock = audio_clock(Rate::ONE);
        clock.pause();
        clock.pause();
        assert_eq!(clock.snapshot().state, ClockState::Paused);
        clock.resume();
        clock.resume();
        assert_eq!(clock.snapshot().state, ClockState::Running);
    }

    #[test]
    fn reanchor_while_paused_moves_the_freeze() {
        let clock = audio_clock(Rate::ONE);
        clock.pause();
        clock
            .reanchor(ts(9_000_000_000))
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(clock.snapshot().state, ClockState::Paused);
        assert_eq!(clock.now().ticks, 9_000_000_000);
        clock.resume();
        assert_eq!(clock.now().ticks, 9_000_000_000);
    }

    #[test]
    fn rate_survives_pause_resume() {
        let counter = Arc::new(FakeCounter::new(48_000));
        let clock = MediaClock::audio_master(
            counter.clone(),
            ts(0),
            Rate::new(2, 1).unwrap_or_else(|e| unreachable!("{e}")),
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        counter.advance_samples(48_000);
        clock.pause();
        let frozen = clock.now();
        assert_eq!(frozen.ticks, 2_000_000_000);
        clock.resume();
        counter.advance_samples(48_000);
        assert_eq!(clock.now().ticks, 4_000_000_000);
    }

    // ---- snapshots / anchors / generation ---------------------------------

    #[test]
    fn snapshot_is_consistent_and_carries_context() {
        let clock = audio_clock(Rate::ONE);
        clock.set_generation(GenerationId::new(41));
        let snap = clock.snapshot();
        assert_eq!(snap.generation, GenerationId::new(41));
        assert_eq!(snap.position, clock.now());
        assert_eq!(snap.rate, Rate::ONE);
        assert_eq!(snap.state, ClockState::Running);
        assert_eq!(snap.master, ClockMaster::AudioOutput);
        assert_eq!(snap.anchor_epoch, 0);
    }

    #[test]
    fn anchor_introspection_reflects_the_master() {
        let clock = audio_clock(Rate::ONE);
        let anchor = clock.anchor();
        assert_eq!(anchor.media.ticks, 0);
        assert_eq!(anchor.at_sample, Some(0));
        assert_eq!(anchor.at_wall_ns, None);

        let wall = Arc::new(FakeMonotonic(AtomicU64::new(123)));
        let mono = MediaClock::system_monotonic(wall, ts(0), Rate::ONE)
            .unwrap_or_else(|e| unreachable!("{e}"));
        let anchor = mono.anchor();
        assert_eq!(anchor.at_sample, None);
        assert_eq!(anchor.at_wall_ns, Some(123));
    }

    #[test]
    fn send_sync_object_safe() {
        fn assert_traits<T: PlaybackClock + Send + Sync>(_: &T) {}
        let clock = audio_clock(Rate::ONE);
        assert_traits(&clock);
        let _boxed: Arc<dyn PlaybackClock> = Arc::new(audio_clock(Rate::ONE));
    }
}
