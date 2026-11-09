//! Bounded reconnect for network fetches (card 6.D2): the retry and
//! backoff policy, resume-from-offset semantics, generation-safe
//! cancellation, and the events/state surface the playback engine
//! consumes — one module, three layers of the same doctrine
//! ("policy is data, the executor owns mutation"):
//!
//! 1. **Policy** ([`RetryPolicy`], [`BackoffSchedule`], [`Jitter`]) —
//!    pure, bounded, per-scheme: a *bounded attempt count*, an
//!    *exponential* delay sequence with *full jitter* (AWS "Exponential
//!    Backoff and Jitter": `sleep = uniform(0, min(cap, base · 2^n))`)
//!    under a hard ceiling, seeded from an injectable seed so tests are
//!    deterministic.
//! 2. **Executor** ([`RetryFetch`]) — an [`HttpFetch`] wrapper any
//!    transport composes with (the fetch/ranged/HLS paths adopt retry
//!    by *wrapping their transport handle*, zero call-site changes):
//!    every retryable failure backs off through the injectable
//!    [`WallClock`](crate::clock::WallClock) port, every in-flight
//!    fetch is generation-stamped, and exhaustion surfaces the last
//!    underlying error honestly.
//! 3. **Seam** ([`StreamEvent`], [`StreamEventHub`], [`ReconnectState`],
//!    [`health_state`]) — the engine-facing queries and the bounded
//!    event channel (the buffering model's signals bridge here too:
//!    one hub carries both halves of 6.D1/6.D2).
//!
//! # Resume from offset (the byte-correctness contract)
//!
//! The fetch port delivers **whole bodies**: a failed fetch delivered
//! nothing, so the caller's consumed position is unchanged — the
//! re-issued request resumes from exactly the offset the caller's
//! cursor sits at. For a *ranged* request (every ranged-resource window
//! fetch) that is a byte-exact resume: the identical `Range:` header
//! re-requests precisely the missing span. For a whole-body `GET` (an
//! HLS segment or playlist) the bounded body is re-fetched whole —
//! there is no partial delivery to resume from, and the body is
//! bounded by the request's `response_limit` either way. GET is
//! idempotent, which is what makes re-issue correct at all; the port
//! carries no other method (v0).
//!
//! # Generation-safe cancellation (the late-response rule)
//!
//! A fetch that starts under generation `g` must never deliver data
//! after `g` is invalidated — a stale response arriving late would be
//! spliced into the new generation's stream as fresh bytes (D-4:
//! no state leakage across transitions). [`GenerationCounter`] is the
//! shared counter the session-level owner bumps on every
//! world-changing event (a seek, a reopen, a track switch);
//! [`RetryFetch`] stamps `g` at entry and re-checks it **after every
//! inner completion and after every backoff sleep** — a late response
//! is discarded and answered
//! [`NetworkError::Cancelled`](crate::error::NetworkError::Cancelled),
//! never delivered.
//!
//! # Composition with the engine's retry window (recorded decision)
//!
//! The engine's recovery guard (`playback-engine` recovery.rs, card
//! 3.C3) retries `Network { Timeout | ConnectionLost | Dns }` reads
//! within its own window and treats `ReconnectExhausted` from a lower
//! layer as terminal. This layer is that lower layer for HTTP: it
//! spends a *small, fast* budget (sub-second jittered backoff, default
//! 4 retries ≤ 8 s ceiling) per fetch; the engine's window is the
//! slower session-level horizon on top. Both budgets are bounded, so
//! their composition is bounded (attempts ≤ engine-window × fetch
//! budget). Exhaustion here surfaces the **last underlying error**
//! (typed weather: 503, timeout, reset) rather than a synthesized
//! "exhausted" error — the resource maps it with its existing
//! `NetworkCause` table, the engine's guard classifies it correctly,
//! and the [`StreamEvent::ReconnectFailed`] event carries the attempt
//! count for telemetry. [PROPOSAL: when the engine's network lane
//! lands, an opt-out flag here can reserve all retry to the engine
//! side (single-owner mode) if measurement shows the product budget
//! double-spends; v0 ships both layers on, each honest about its own
//! budget.]
//!
//! # Bounded memory (D-1)
//!
//! The event hub is a bounded [`std::sync::mpsc::SyncSender`]: emits
//! are non-blocking, an overflowing hub **drops and counts** (never
//! blocks the fetch path, never grows). Retry state is one `Mutex`
//! holding `Copy` data — the fetch path takes it per outcome, never
//! across a sleep or an inner fetch. Nothing here allocates on any
//! per-frame path; the only allocations are the request clone per
//! retry and the event value per transition (both per-network-failure,
//! I/O-bound cold paths).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};

use crate::buffer::{BufferLevel, BufferSignal};
use crate::clock::WallClock;
use crate::error::NetworkError;
use crate::fetch::{HttpFetch, HttpRequest, HttpResponse};

/// Default bounded retry count per fetch (4 retries → 5 attempts
/// total; VLC-territory for a transport-level budget).
pub const DEFAULT_MAX_RETRIES: u32 = 4;

/// Ceiling on the configurable retry count (a policy asking for more
/// is a configuration bug — typed refusal, never a clamp).
pub const MAX_RETRIES_CEILING: u32 = 16;

/// Default first-backoff base: 500 ms.
pub const DEFAULT_INITIAL_BACKOFF_MS: u64 = 500;

/// Default backoff ceiling: 8 s (the hard cap on any single sleep).
pub const DEFAULT_MAX_BACKOFF_MS: u64 = 8_000;

/// Ceiling on the configurable backoff ceiling (a policy asking for a
/// longer single wait is a configuration bug).
pub const MAX_BACKOFF_CEILING_MS: u64 = 60_000;

/// Default bounded event-hub capacity: one drain interval's worth of
/// transitions (mirrors the engine's outbox sizing posture).
pub const DEFAULT_EVENT_CAPACITY: usize = 256;

// ---------------------------------------------------------------------------
// Policy
// ---------------------------------------------------------------------------

/// The bounded retry policy for one transport (module docs §1).
///
/// Invariants enforced by [`RetryPolicy::validate`]: `max_retries`
/// within `[0, MAX_RETRIES_CEILING]`, `initial_ms` positive, and
/// `initial_ms <= max_ms <= MAX_BACKOFF_CEILING_MS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Retries beyond the first attempt (0 = fail fast, no retries).
    pub max_retries: u32,
    /// The exponential base: the first retry's jitter ceiling.
    pub initial_ms: u64,
    /// The hard ceiling on any single backoff sleep.
    pub max_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            max_retries: DEFAULT_MAX_RETRIES,
            initial_ms: DEFAULT_INITIAL_BACKOFF_MS,
            max_ms: DEFAULT_MAX_BACKOFF_MS,
        }
    }
}

impl RetryPolicy {
    /// Validate the bounds (typed refusal, never a clamp — MP §5).
    ///
    /// # Errors
    /// [`String`] naming the violated invariant (a configuration bug;
    /// callers surface it through their own typed config errors).
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.max_retries > MAX_RETRIES_CEILING {
            return Err("max_retries exceeds the ceiling (16)");
        }
        if self.initial_ms == 0 {
            return Err("initial_ms must be positive");
        }
        if self.max_ms < self.initial_ms {
            return Err("max_ms must be at or above initial_ms");
        }
        if self.max_ms > MAX_BACKOFF_CEILING_MS {
            return Err("max_ms exceeds the ceiling (60000)");
        }
        Ok(())
    }

    /// The policy for a URL scheme (the per-scheme table). v0 ships
    /// **one honest posture for both `http` and `https`**: diverging
    /// them needs measured evidence (D-11), not invention — the table
    /// is the typed seam where that divergence lands. Unknown schemes
    /// get the same defaults (the transport refuses them at
    /// [`Url::parse`](crate::fetch::Url::parse) before any policy
    /// consult; the policy never guesses a posture for a scheme it
    /// does not fetch).
    #[must_use]
    pub fn per_scheme(scheme: &str) -> RetryPolicy {
        match scheme {
            "http" | "https" => RetryPolicy::default(),
            _ => RetryPolicy::default(),
        }
    }

    /// The jitter ceiling for retry number `attempt` (0-based): the
    /// exponential ladder `min(max_ms, initial_ms · 2^attempt)`,
    /// saturating (D-2: one power, one multiply — a raw shift would
    /// wrap to zero for large attempts, and a zero ceiling would
    /// collapse the retry sleep).
    #[must_use]
    pub fn ceiling_ms(&self, attempt: u32) -> u64 {
        if attempt >= 64 {
            return self.max_ms;
        }
        self.initial_ms
            .saturating_mul(2u64.saturating_pow(attempt))
            .min(self.max_ms)
    }
}

/// The jitter source: a seeded xorshift64\* generator (std-only, no
/// dependencies; the classic Marsaglia sequence with the star final
/// scramble — one multiply and three shifts per draw, D-2).
///
/// Determinism is the point: production seeds from the wall-clock
/// port's reading at construction (varies per session), tests inject a
/// fixed seed and the schedule becomes a reproducible sequence. A zero
/// seed maps to a fixed nonzero state (the all-zero state is the
/// generator's only fixed point).
#[derive(Debug, Clone)]
pub struct Jitter {
    state: u64,
}

/// The nonzero replacement for a zero seed (the golden-ratio
/// constant's upper bits — any odd, well-mixed value works; this one
/// is the fixed convention so `seeded(0)` is reproducible too).
const JITTER_ZERO_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

impl Jitter {
    /// A generator seeded with `seed` (0 maps to the fixed convention
    /// constant — never the generator's zero fixed point).
    #[must_use]
    pub fn seeded(seed: u64) -> Jitter {
        Jitter {
            state: if seed == 0 { JITTER_ZERO_SEED } else { seed },
        }
    }

    /// The next raw draw (xorshift64\*: shift, xor, shift, xor, shift,
    /// then the star multiply).
    #[must_use]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A uniform draw in `0..bound` (`bound` ≥ 1; a zero bound answers
    /// 0 — the empty range's only honest member). One multiply-high
    /// reduction, no modulo (D-2: no division).
    #[must_use]
    pub fn below(&mut self, bound: u64) -> u64 {
        if bound <= 1 {
            return 0;
        }
        // Lemire's multiply-high: (raw * bound) >> 64 is uniform when
        // raw is uniform over 2^64 — the rejection-free bound comes
        // from the 128-bit product's high word.
        let raw = self.next_u64();
        ((raw as u128 * u128::from(bound)) >> 64) as u64
    }
}

/// The bounded backoff schedule (module docs §1): full jitter over the
/// policy's exponential ladder.
///
/// `delay_ms(attempt)` draws `uniform(0, ceiling_ms(attempt))` — the
/// full-jitter shape: a delay of ~0 is possible on any attempt (the
/// distribution's honest low tail), and no delay ever exceeds the
/// policy ceiling. The schedule owns the [`Jitter`] state so a
/// session's draws never repeat within itself.
#[derive(Debug, Clone)]
pub struct BackoffSchedule {
    policy: RetryPolicy,
    jitter: Jitter,
}

impl BackoffSchedule {
    /// A schedule over `policy`, seeded with `seed`.
    #[must_use]
    pub fn new(policy: RetryPolicy, seed: u64) -> BackoffSchedule {
        BackoffSchedule {
            policy,
            jitter: Jitter::seeded(seed),
        }
    }

    /// The policy in force (telemetry).
    #[must_use]
    pub fn policy(&self) -> RetryPolicy {
        self.policy
    }

    /// The jittered delay for retry number `attempt` (0-based, the
    /// first retry): a uniform draw in `0..ceiling_ms(attempt)`.
    #[must_use]
    pub fn delay_ms(&mut self, attempt: u32) -> u64 {
        self.jitter.below(self.policy.ceiling_ms(attempt))
    }
}

// ---------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------

/// The shared generation counter (module docs: generation-safe
/// cancellation). Cheap to clone (one `Arc<AtomicU64>`); the
/// session-level owner holds one clone and **bumps** on every
/// world-changing event, the fetch side stamps and re-checks.
///
/// The counter starts at 0 and only ever advances
/// ([`GenerationCounter::bump`] is a fetch-add) — monotonic, so a
/// stamp never compares fresh against a moved-on counter by accident.
#[derive(Debug, Clone, Default)]
pub struct GenerationCounter {
    inner: Arc<AtomicU64>,
}

impl GenerationCounter {
    /// A counter frozen at generation 0.
    #[must_use]
    pub fn new() -> GenerationCounter {
        GenerationCounter::default()
    }

    /// The current generation.
    #[must_use]
    pub fn current(&self) -> u64 {
        self.inner.load(Ordering::Acquire)
    }

    /// Invalidate every in-flight stamp: advance the generation and
    /// return the new one.
    pub fn bump(&self) -> u64 {
        self.inner.fetch_add(1, Ordering::AcqRel) + 1
    }
}

// ---------------------------------------------------------------------------
// State, events, and the bounded hub (the seam)
// ---------------------------------------------------------------------------

/// The reconnect half of the reliable-delivery state (module docs
/// §3). `Copy` — the query path allocates nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReconnectState {
    /// Delivery is working; the last fetch succeeded first-try.
    #[default]
    Steady,
    /// A retryable failure occurred; retry `attempt` is scheduled
    /// after `delay_ms` (the state a live read sits in while the
    /// fetch path backs off).
    Reconnecting {
        /// The in-flight retry number (1 = the first retry).
        attempt: u32,
        /// The jittered delay that precedes it (ms).
        delay_ms: u64,
    },
    /// A retry succeeded — delivery resumed but not yet proven stable
    /// (the *next* first-try success returns to [`Steady`]).
    Recovering {
        /// Retries the recovered fetch needed.
        attempts: u32,
    },
    /// The bounded budget was spent without success (terminal for
    /// this fetch; a new fetch under the handle starts fresh).
    Failed {
        /// Total attempts the exhausted fetch made.
        attempts: u32,
    },
}

/// The composite network-source health — the **playback-engine seam
/// query** (buffer model + reconnect state in one verdict, module
/// docs §3). The engine mapping is recorded, not invented:
///
/// | Verdict | Engine surface |
/// |---|---|
/// | [`Buffering`](HealthState::Buffering) / [`Rebuffering`](HealthState::Rebuffering) / [`Recovering`](HealthState::Recovering) | `PlaybackState::Buffering` + `session.buffering` progress (the snapshot's `fill_fraction`) |
/// | [`Reconnecting`](HealthState::Reconnecting) | stay in `Buffering` and keep retrying within policy (doc 03 §4.1(d), the recovery guard's own rule) |
/// | [`Steady`](HealthState::Steady) | resume (`playing` once limbs are fed) |
/// | [`Failed`](HealthState::Failed) | the read error surfaces (`playback.error` fatal path) |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    /// The opening fill: the fill watermarks were never met.
    Buffering,
    /// Steady delivery at/above the fill watermarks.
    Steady,
    /// A rebuffer episode: was `High`, drained `Low`, refilling — an
    /// arrival deficit, no connection loss involved.
    Rebuffering,
    /// Connection lost; a bounded retry is in flight.
    Reconnecting,
    /// Reconnected (or refilling after reconnect); below the fill
    /// watermarks or not yet proven stable.
    Recovering,
    /// The retry budget was exhausted.
    Failed,
}

/// The composite verdict from the two models (module docs: the seam
/// query). Pure and total — the engine lane (or a test) feeds the
/// [`BufferModel`](crate::buffer::BufferModel)'s level, whether the
/// initial fill ever completed, and [`RetryFetch::state`].
#[must_use]
pub fn health_state(
    level: BufferLevel,
    initial_filled: bool,
    reconnect: ReconnectState,
) -> HealthState {
    match reconnect {
        // A spent budget is the terminal verdict whatever the buffer
        // holds — data in hand does not un-spend the retry budget.
        ReconnectState::Failed { .. } => HealthState::Failed,
        ReconnectState::Reconnecting { .. } => HealthState::Reconnecting,
        ReconnectState::Recovering { .. } => match level {
            BufferLevel::Low => HealthState::Recovering,
            BufferLevel::High => HealthState::Steady,
        },
        ReconnectState::Steady => match level {
            BufferLevel::High => HealthState::Steady,
            BufferLevel::Low if initial_filled => HealthState::Rebuffering,
            BufferLevel::Low => HealthState::Buffering,
        },
    }
}

/// One reliable-delivery event on the seam (both card halves share
/// this vocabulary: reconnect transitions here, the buffering model's
/// signals through [`StreamEventHub::forward_signal`]).
///
/// The engine mapping (recorded in [`HealthState`]'s table and the
/// bridge that lands with the engine's network lane):
/// `Buffering` → `session.buffering` (progress bookends and
/// in-episode fill); `ReconnectFailed` → the read error's fatal path;
/// `Reconnecting`/`Recovered` → engine-visible reconnect state (the
/// catalog's `wired-by-later` rows own their ids — no id is invented
/// here); `StallPredicted`/`StallCleared`/`Superseded` → engine-side
/// telemetry (no v1 wire row).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamEvent {
    /// A buffering episode edge or fill signal (the buffering model's
    /// watermark transitions; `progress` is the snapshot's bottleneck
    /// fill fraction: `1.0` on `Filled`, `0.0` on `Drained`).
    Buffering {
        /// `0.0..=1.0`.
        progress: f32,
    },
    /// The stall predictor fired (an underrun is predicted inside the
    /// horizon; carries the honest ETA).
    StallPredicted {
        /// Predicted wall-time until the runway is exhausted (ns).
        eta_ns: u64,
    },
    /// A previous stall prediction no longer holds.
    StallCleared,
    /// A retryable failure occurred; retry `attempt` fires after
    /// `delay_ms`.
    Reconnecting {
        /// The in-flight retry number (1-based).
        attempt: u32,
        /// The jittered delay that precedes it (ms).
        delay_ms: u64,
    },
    /// A fetch recovered after `attempts` retries (delivery resumed).
    Recovered {
        /// Retries the recovered fetch needed.
        attempts: u32,
    },
    /// The bounded retry budget was spent (the fetch error surfaces to
    /// the caller; this event carries the count for telemetry).
    ReconnectFailed {
        /// Total attempts the exhausted fetch made.
        attempts: u32,
    },
    /// An in-flight fetch was superseded by a newer generation (its
    /// late response was discarded — never delivered as fresh data).
    Superseded {
        /// The invalidated generation the abandoned fetch held.
        generation: u64,
    },
}

/// The bounded event hub (module docs: bounded memory): a
/// [`sync_channel`](std::sync::mpsc::sync_channel) sender plus a
/// drop counter. `emit` **never blocks** — a full hub drops the event
/// and counts it (the host that does not drain loses events before it
/// loses memory; D-1). Clone shares the sender and the counter.
#[derive(Debug, Clone)]
pub struct StreamEventHub {
    sender: SyncSender<StreamEvent>,
    dropped: Arc<AtomicU64>,
}

impl StreamEventHub {
    /// A hub with capacity for `capacity` undelivered events (0 = a
    /// rendezvous channel — emits succeed only while a receiver
    /// waits). Returns the hub and the receiving end.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> (StreamEventHub, Receiver<StreamEvent>) {
        let (sender, receiver) = sync_channel(capacity);
        (
            StreamEventHub {
                sender,
                dropped: Arc::new(AtomicU64::new(0)),
            },
            receiver,
        )
    }

    /// Emit one event: `true` when delivered, `false` when the hub was
    /// full or already disconnected (counted in [`dropped`] either
    /// way — the caller's policy decides whether a drop matters; the
    /// fetch path treats it as telemetry, never a failure).
    pub fn emit(&self, event: StreamEvent) -> bool {
        match self.sender.try_send(event) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    /// Events dropped so far (the bounded-memory counter).
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Forward one buffering-model transition (the 6.D1 ↔ 6.D2
    /// bridge): `Filled` → [`StreamEvent::Buffering`] `1.0`, `Drained`
    /// → `0.0`, the stall edges verbatim. `Steady` forwards nothing —
    /// the hub carries *transitions*, and the fill fraction is a
    /// poll-time query ([`BufferSnapshot`]'s
    /// `fill_fraction`), not a stream (the engine's ≤ 5 Hz progress
    /// coalescing owns that rate; doc 05 §3.4 rule 2).
    ///
    /// Returns whether anything was emitted (the steady answer).
    pub fn forward_signal(&self, signal: BufferSignal) -> bool {
        let event = match signal {
            BufferSignal::Filled => StreamEvent::Buffering { progress: 1.0 },
            BufferSignal::Drained => StreamEvent::Buffering { progress: 0.0 },
            BufferSignal::StallPredicted { eta_ns } => StreamEvent::StallPredicted { eta_ns },
            BufferSignal::StallCleared => StreamEvent::StallCleared,
            BufferSignal::Steady => return false,
        };
        self.emit(event)
    }
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

/// What the retry executor did (test/telemetry surface, `Copy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReconnectStats {
    /// `fetch` calls entered.
    pub fetches: u64,
    /// Retry fetches issued (the attempts beyond the first).
    pub retries: u64,
    /// Fetches that succeeded after at least one retry.
    pub recovered: u64,
    /// Fetches that exhausted the bounded retry budget.
    pub exhausted: u64,
    /// Fetches cancelled by a generation bump (late responses
    /// discarded).
    pub superseded: u64,
    /// Events the hub dropped (bounded-memory accounting).
    pub events_dropped: u64,
}

// ---------------------------------------------------------------------------
// The executor
// ---------------------------------------------------------------------------

/// The mutable core behind the [`Mutex`] (all `Copy`-cheap data; the
/// lock is never held across an inner fetch or a sleep).
#[derive(Debug)]
struct RetryCore {
    schedule: BackoffSchedule,
    state: ReconnectState,
    stats: ReconnectStats,
}

/// The retrying transport wrapper (module docs §2): an
/// [`HttpFetch`] decorator any consumer composes with — the ranged
/// resource, the HLS sessions, and any direct port user adopt bounded
/// retry by handing their transport handle to [`RetryFetch::new`] and
/// the wrapper to the consumer, unchanged.
///
/// - **Bounded**: [`RetryPolicy`] bounds the attempts and every
///   single sleep; the wall-clock port's own contract bounds waits.
/// - **Resuming**: re-issues the identical request (module docs:
///   resume-from-offset for ranged requests, whole-body re-fetch for
///   plain GETs).
/// - **Generation-safe**: stamps the generation at entry, re-checks
///   after every inner completion and every backoff sleep; a late or
///   mid-backoff invalidation answers
///   [`NetworkError::Cancelled`] and the response is discarded.
/// - **Observable**: transitions land on the [`StreamEventHub`] (set
///   with [`with_event_hub`](RetryFetch::with_event_hub)) and the
///   state/stats are queryable (`&self`, any time).
pub struct RetryFetch {
    inner: Arc<dyn HttpFetch>,
    policy: RetryPolicy,
    clock: Arc<dyn WallClock>,
    generation: GenerationCounter,
    hub: Option<StreamEventHub>,
    core: Mutex<RetryCore>,
}

impl RetryFetch {
    /// A retrying wrapper over `inner`: `policy` bounds the retries,
    /// `clock` drives the backoff sleeps (tests inject
    /// [`FakeClock`](crate::clock::FakeClock) — sleeps advance virtual
    /// time instantly and are recorded, the schedule proof). The
    /// jitter seed is the clock's reading at construction (a fresh
    /// `FakeClock` therefore seeds deterministically at 0).
    ///
    /// # Errors
    /// [`RetryPolicy::validate`]'s verdict, verbatim.
    pub fn new_checked(
        inner: Arc<dyn HttpFetch>,
        policy: RetryPolicy,
        clock: Arc<dyn WallClock>,
    ) -> Result<RetryFetch, &'static str> {
        policy.validate()?;
        let seed = clock.now_ms();
        Ok(RetryFetch {
            inner,
            policy,
            clock,
            generation: GenerationCounter::new(),
            hub: None,
            core: Mutex::new(RetryCore {
                schedule: BackoffSchedule::new(policy, seed),
                state: ReconnectState::Steady,
                stats: ReconnectStats::default(),
            }),
        })
    }

    /// [`new_checked`](RetryFetch::new_checked) with the default
    /// policy: the bounded 4-retry, 500 ms→8 s full-jitter posture.
    ///
    /// # Errors
    /// [`RetryPolicy::validate`] on the defaults (cannot fail — the
    /// defaults are the validated constant set; the `Result` keeps
    /// every constructor honest the same way).
    pub fn new(
        inner: Arc<dyn HttpFetch>,
        clock: Arc<dyn WallClock>,
    ) -> Result<RetryFetch, &'static str> {
        RetryFetch::new_checked(inner, RetryPolicy::default(), clock)
    }

    /// Attach the event hub (builder style; without one the wrapper
    /// stays silent — state and stats remain queryable).
    #[must_use]
    pub fn with_event_hub(mut self, hub: StreamEventHub) -> RetryFetch {
        self.hub = Some(hub);
        self
    }

    /// Use THIS generation counter (builder style): the session-level
    /// owner shares one counter across every transport it wraps — one
    /// bump then invalidates every in-flight fetch of the whole
    /// session, not just one transport's.
    #[must_use]
    pub fn with_generation(mut self, generation: GenerationCounter) -> RetryFetch {
        self.generation = generation;
        self
    }

    /// The generation handle (clone it, bump it from the
    /// session-level owner on every world-changing event).
    #[must_use]
    pub fn generation(&self) -> GenerationCounter {
        self.generation.clone()
    }

    /// The policy in force (telemetry).
    #[must_use]
    pub fn policy(&self) -> RetryPolicy {
        self.policy
    }

    /// The reconnect state (the seam query's input).
    #[must_use]
    pub fn state(&self) -> ReconnectState {
        self.lock_core().state
    }

    /// The counters (tests and telemetry).
    #[must_use]
    pub fn stats(&self) -> ReconnectStats {
        self.lock_core().stats
    }

    /// Lock the core (lock-poisoning recovery is the house pattern —
    /// a panicking holder leaves valid data).
    fn lock_core(&self) -> std::sync::MutexGuard<'_, RetryCore> {
        self.core
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Record a completed fetch under `attempt` retries used. The
    /// success transitions: retries > 0 → [`ReconnectState::Recovering`]
    /// (resumed, not yet proven stable), else → [`Steady`](ReconnectState::Steady).
    fn record_success(&self, attempt: u32) {
        let mut core = self.lock_core();
        if attempt > 0 {
            core.stats.recovered += 1;
            core.state = ReconnectState::Recovering { attempts: attempt };
            let event = StreamEvent::Recovered { attempts: attempt };
            drop(core);
            self.emit(event);
        } else {
            core.state = ReconnectState::Steady;
        }
    }

    /// Record a superseded fetch: the response (success or failure)
    /// is discarded, the retry state is void — the fresh generation's
    /// owner re-issues. Emits [`StreamEvent::Superseded`].
    fn record_superseded(&self, generation: u64) {
        let mut core = self.lock_core();
        core.stats.superseded += 1;
        core.state = ReconnectState::Steady;
        let event = StreamEvent::Superseded { generation };
        drop(core);
        self.emit(event);
    }

    /// Plan the next step after a retryable failure under `attempt`
    /// retries used: `Some(delay)` when a retry is due (state and
    /// event recorded), `None` when the budget is spent (exhaustion
    /// recorded) or the failure is not retryable (recorded nowhere —
    /// it is the caller's error to surface, the state stays honest).
    fn plan_retry(&self, attempt: u32, err: &NetworkError) -> Option<u64> {
        if !err.is_retryable() {
            return None;
        }
        let mut core = self.lock_core();
        if attempt >= self.policy.max_retries {
            let attempts = attempt + 1;
            core.stats.exhausted += 1;
            core.state = ReconnectState::Failed { attempts };
            let event = StreamEvent::ReconnectFailed { attempts };
            drop(core);
            self.emit(event);
            return None;
        }
        let delay_ms = core.schedule.delay_ms(attempt);
        let retry_number = attempt + 1;
        core.stats.retries += 1;
        core.state = ReconnectState::Reconnecting {
            attempt: retry_number,
            delay_ms,
        };
        let event = StreamEvent::Reconnecting {
            attempt: retry_number,
            delay_ms,
        };
        drop(core);
        self.emit(event);
        Some(delay_ms)
    }

    /// Emit through the hub when one is attached (no hub: a no-op).
    fn emit(&self, event: StreamEvent) {
        if let Some(hub) = &self.hub {
            hub.emit(event);
        }
    }
}

impl HttpFetch for RetryFetch {
    fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
        let observed = self.generation.current();
        self.lock_core().stats.fetches += 1;
        let mut attempt = 0u32;
        loop {
            // The inner fetch runs WITHOUT the core lock (it is
            // seconds of I/O; state queries stay live during it) and
            // under the stamped generation — the module docs' rule.
            let result = self.inner.fetch(request.clone());
            // Late-response rule: any completion arriving after the
            // generation moved is discarded, success included.
            if self.generation.current() != observed {
                self.record_superseded(observed);
                return Err(NetworkError::Cancelled {
                    generation: observed,
                });
            }
            match result {
                Ok(response) => {
                    self.record_success(attempt);
                    return Ok(response);
                }
                Err(err) => {
                    let Some(delay_ms) = self.plan_retry(attempt, &err) else {
                        return Err(err);
                    };
                    // The bounded wait (the clock port's contract);
                    // the lock is NOT held across it.
                    self.clock.sleep_ms(delay_ms);
                    // Mid-backoff rule: a bump during the wait cancels
                    // before another request goes out under a stale
                    // generation.
                    if self.generation.current() != observed {
                        self.record_superseded(observed);
                        return Err(NetworkError::Cancelled {
                            generation: observed,
                        });
                    }
                    attempt += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{FakeClock, SystemClock};
    use crate::fetch::ByteRange;
    use std::collections::VecDeque;
    use testkit::Must;

    /// The retryable weathers the scripts use (a `Copy` stand-in for
    /// the not-`Clone` [`NetworkError`] — the test table stays small
    /// and the mapping is one function).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Weather {
        /// A read deadline expiry.
        Timeout,
        /// Nothing listening.
        Refused,
        /// An HTTP status the server answered with.
        Status(u16),
    }

    impl Weather {
        fn err(self) -> NetworkError {
            match self {
                Weather::Timeout => NetworkError::ReadTimeout,
                Weather::Refused => NetworkError::ConnectRefused,
                Weather::Status(code) => NetworkError::http_status(code),
            }
        }
    }

    /// One scripted inner-transport outcome (the RouterFetch pattern
    /// the crate's tests use — no socket, unit-test speed, the same
    /// `HttpFetch` port the real transport serves).
    #[derive(Debug, Clone)]
    enum Outcome {
        /// Answer 200 with this body.
        Body(Vec<u8>),
        /// Fail with this weather.
        Fail(Weather),
        /// Bump the generation, THEN answer 200 (the late response).
        BodyAfterBump(Vec<u8>),
        /// Bump the generation, THEN fail (a superseded failure).
        FailAndBump(Weather),
    }

    /// An in-test transport serving scripted outcomes per fetch. When
    /// the script is exhausted the LAST outcome repeats (the "server
    /// never recovers" shape); every request is recorded (the
    /// resume-from-offset proof reads the `Range` headers back).
    struct ScriptedFetch {
        counter: Option<GenerationCounter>,
        outcomes: Mutex<VecDeque<Outcome>>,
        requests: Mutex<Vec<HttpRequest>>,
    }

    impl ScriptedFetch {
        fn new(outcomes: Vec<Outcome>) -> Arc<ScriptedFetch> {
            Arc::new(ScriptedFetch {
                counter: None,
                outcomes: Mutex::new(outcomes.into()),
                requests: Mutex::new(Vec::new()),
            })
        }

        /// Scripted outcomes that bump THIS counter (the late-response
        /// and superseded-failure shapes).
        fn with_counter(counter: GenerationCounter, outcomes: Vec<Outcome>) -> Arc<ScriptedFetch> {
            Arc::new(ScriptedFetch {
                counter: Some(counter),
                outcomes: Mutex::new(outcomes.into()),
                requests: Mutex::new(Vec::new()),
            })
        }

        /// The requests the transport saw, in order (the resume
        /// proof's assertion surface).
        fn requests(&self) -> Vec<HttpRequest> {
            match self.requests.lock() {
                Ok(guard) => guard.clone(),
                Err(poisoned) => poisoned.into_inner().clone(),
            }
        }

        fn ok(body: Vec<u8>) -> HttpResponse {
            HttpResponse {
                status: 200,
                reason: "OK".to_owned(),
                headers: Vec::new(),
                body,
            }
        }
    }

    impl HttpFetch for ScriptedFetch {
        fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
            match self.requests.lock() {
                Ok(mut guard) => guard.push(request),
                Err(poisoned) => poisoned.into_inner().push(request),
            }
            // Repeat-last: consume the head while more remain; the
            // tail element serves forever once it is the only one.
            let outcome = {
                let mut outcomes = self
                    .outcomes
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if outcomes.len() > 1 {
                    outcomes.pop_front()
                } else {
                    outcomes.front().cloned()
                }
            };
            match outcome {
                Some(Outcome::Body(body)) => Ok(Self::ok(body)),
                Some(Outcome::Fail(weather)) => Err(weather.err()),
                Some(Outcome::BodyAfterBump(body)) => {
                    if let Some(counter) = &self.counter {
                        counter.bump();
                    }
                    Ok(Self::ok(body))
                }
                Some(Outcome::FailAndBump(weather)) => {
                    if let Some(counter) = &self.counter {
                        counter.bump();
                    }
                    Err(weather.err())
                }
                None => Err(NetworkError::ReadTimeout),
            }
        }
    }

    /// A fast test policy: 3 retries, 100 ms → 800 ms ladder.
    fn fast_policy() -> RetryPolicy {
        RetryPolicy {
            max_retries: 3,
            initial_ms: 100,
            max_ms: 800,
        }
    }

    /// A wrapper under test with the fast policy, a fresh fake clock
    /// (reading 0 — deterministic jitter seed) and the event hub.
    fn setup(
        outcomes: Vec<Outcome>,
    ) -> (
        Arc<ScriptedFetch>,
        RetryFetch,
        Arc<FakeClock>,
        Receiver<StreamEvent>,
    ) {
        let inner = ScriptedFetch::new(outcomes);
        let clock = Arc::new(FakeClock::new());
        let (hub, receiver) = StreamEventHub::with_capacity(64);
        let wrapper = RetryFetch::new_checked(
            Arc::clone(&inner) as Arc<dyn HttpFetch>,
            fast_policy(),
            Arc::clone(&clock) as Arc<dyn WallClock>,
        )
        .must()
        .with_event_hub(hub);
        (inner, wrapper, clock, receiver)
    }

    /// Drain the hub into a Vec (assertion surface).
    fn drain(receiver: &Receiver<StreamEvent>) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            events.push(event);
        }
        events
    }

    /// The sleeps the fake clock records, mirroring its zero-sleep
    /// no-op contract.
    fn sleeps(clock: &FakeClock) -> Vec<u64> {
        clock.sleeps().into_iter().filter(|ms| *ms > 0).collect()
    }

    /// A plain GET for the unit origin (the scripts never touch the
    /// wire; the URL only has to parse).
    fn get(path: &str) -> HttpRequest {
        HttpRequest::get(format!("http://unit.test/{path}"))
    }

    /// Policy bounds are validated typed, never clamped.
    #[test]
    fn policy_bounds_are_validated() {
        fast_policy().validate().must();
        RetryPolicy::default().validate().must();
        let mut over_retries = fast_policy();
        over_retries.max_retries = MAX_RETRIES_CEILING + 1;
        assert!(over_retries.validate().is_err(), "retry count over ceiling");
        let mut zero_initial = fast_policy();
        zero_initial.initial_ms = 0;
        assert!(zero_initial.validate().is_err(), "zero base delay");
        let mut inverted = fast_policy();
        inverted.max_ms = inverted.initial_ms - 1;
        assert!(inverted.validate().is_err(), "ceiling below base");
        let mut over_ceiling = fast_policy();
        over_ceiling.max_ms = MAX_BACKOFF_CEILING_MS + 1;
        assert!(over_ceiling.validate().is_err(), "ceiling over budget");
        // Zero retries is LEGAL: the fail-fast passthrough posture.
        let mut none = fast_policy();
        none.max_retries = 0;
        none.validate().must();
    }

    /// The per-scheme table is typed and bounded for every scheme it
    /// is consulted with (v0 ships one honest posture — module docs).
    #[test]
    fn per_scheme_policy_is_bounded() {
        for scheme in ["http", "https", "rtsp"] {
            RetryPolicy::per_scheme(scheme).validate().must();
        }
        assert_eq!(RetryPolicy::per_scheme("https"), RetryPolicy::default());
        assert_eq!(RetryPolicy::per_scheme("http"), RetryPolicy::default());
    }

    /// The exponential ladder: doubling to the ceiling, then flat;
    /// shift-saturation at high attempt counts stays at the ceiling.
    #[test]
    fn ceiling_ladder_is_exponential_and_capped() {
        let policy = fast_policy();
        assert_eq!(policy.ceiling_ms(0), 100);
        assert_eq!(policy.ceiling_ms(1), 200);
        assert_eq!(policy.ceiling_ms(2), 400);
        assert_eq!(policy.ceiling_ms(3), 800);
        assert_eq!(policy.ceiling_ms(4), 800, "capped at max_ms");
        assert_eq!(policy.ceiling_ms(63), 800);
        assert_eq!(policy.ceiling_ms(64), 800, "shift saturation");
        assert_eq!(policy.ceiling_ms(1000), 800);
    }

    /// The jitter generator: deterministic per seed, bounded by the
    /// asked range, and never stuck at the zero fixed point.
    #[test]
    fn jitter_is_deterministic_and_bounded() {
        let mut a = Jitter::seeded(42);
        let mut b = Jitter::seeded(42);
        for _ in 0..16 {
            assert_eq!(a.next_u64(), b.next_u64(), "same seed, same sequence");
        }
        let mut c = Jitter::seeded(43);
        let differs = (0..8).any(|_| a.next_u64() != c.next_u64());
        assert!(differs, "different seeds diverge");
        let mut zero = Jitter::seeded(0);
        let mut nonzero_seen = false;
        for _ in 0..8 {
            let draw = zero.below(1000);
            assert!(draw < 1000, "below(bound) is strictly under bound");
            nonzero_seen |= draw > 0;
        }
        assert!(nonzero_seen, "seed 0 escapes the zero fixed point");
        assert_eq!(Jitter::seeded(7).below(1), 0, "bound 1 has only 0");
        assert_eq!(Jitter::seeded(7).below(0), 0, "empty range's only member");
    }

    /// Full jitter: every draw lands strictly inside the attempt's
    /// ceiling, and the sequence jitters (deterministic seed, the
    /// draws are not all one value).
    #[test]
    fn backoff_full_jitter_within_ceiling() {
        let mut schedule = BackoffSchedule::new(fast_policy(), 0);
        let mut distinct = std::collections::HashSet::new();
        for attempt in 0..6u32 {
            for _ in 0..24 {
                let delay = schedule.delay_ms(attempt);
                assert!(
                    delay < fast_policy().ceiling_ms(attempt),
                    "draw {delay} inside attempt {attempt} ceiling"
                );
                distinct.insert(delay);
            }
        }
        assert!(distinct.len() > 1, "the schedule jitters, not a constant");
        // Determinism: a fresh schedule with the same seed replays.
        let mut replay = BackoffSchedule::new(fast_policy(), 0);
        let mut original = BackoffSchedule::new(fast_policy(), 0);
        for _ in 0..8 {
            assert_eq!(original.delay_ms(0), replay.delay_ms(0));
        }
    }

    /// The generation counter starts at 0 and only advances.
    #[test]
    fn generation_counter_advances() {
        let counter = GenerationCounter::new();
        assert_eq!(counter.current(), 0);
        assert_eq!(counter.bump(), 1);
        assert_eq!(counter.current(), 1);
        let clone = counter.clone();
        assert_eq!(clone.bump(), 2, "clones share the counter");
        assert_eq!(counter.current(), 2);
    }

    /// The recovery path: two transient failures, then success — the
    /// delays, events, state and stats all recorded (the schedule
    /// proof: the wrapper's draws match a mirrored schedule with the
    /// same seed).
    #[test]
    fn retry_recovers_after_transient_failures() {
        let (inner, wrapper, clock, receiver) = setup(vec![
            Outcome::Fail(Weather::Timeout),
            Outcome::Fail(Weather::Refused),
            Outcome::Body(b"recovered".to_vec()),
        ]);
        let response = wrapper.fetch(get("media")).must();
        assert_eq!(response.body, b"recovered");
        // The schedule proof: mirror the wrapper's seeded schedule.
        let mut mirror = BackoffSchedule::new(fast_policy(), 0);
        let d1 = mirror.delay_ms(0);
        let d2 = mirror.delay_ms(1);
        let mut expected_sleeps = Vec::new();
        if d1 > 0 {
            expected_sleeps.push(d1);
        }
        if d2 > 0 {
            expected_sleeps.push(d2);
        }
        assert_eq!(sleeps(&clock), expected_sleeps);
        // Events: the two retry schedules, then the recovery.
        assert_eq!(
            drain(&receiver),
            vec![
                StreamEvent::Reconnecting {
                    attempt: 1,
                    delay_ms: d1
                },
                StreamEvent::Reconnecting {
                    attempt: 2,
                    delay_ms: d2
                },
                StreamEvent::Recovered { attempts: 2 },
            ]
        );
        let stats = wrapper.stats();
        assert_eq!(stats.fetches, 1);
        assert_eq!(stats.retries, 2);
        assert_eq!(stats.recovered, 1);
        assert_eq!(stats.exhausted, 0);
        assert_eq!(stats.superseded, 0);
        assert_eq!(
            wrapper.state(),
            ReconnectState::Recovering { attempts: 2 },
            "resumed, not yet proven stable"
        );
        assert_eq!(inner.requests().len(), 3, "three attempts, two retries");
    }

    /// After a recovery, the next first-try success proves stability:
    /// Recovering → Steady.
    #[test]
    fn steady_after_first_try_success_follows_recovery() {
        let (_inner, wrapper, _clock, _receiver) = setup(vec![
            Outcome::Fail(Weather::Timeout),
            Outcome::Body(b"first".to_vec()),
            Outcome::Body(b"second".to_vec()),
        ]);
        wrapper.fetch(get("a")).must();
        assert_eq!(wrapper.state(), ReconnectState::Recovering { attempts: 1 });
        wrapper.fetch(get("b")).must();
        assert_eq!(wrapper.state(), ReconnectState::Steady);
    }

    /// A fresh fetch after exhaustion starts a new episode: a
    /// first-try success returns the state to Steady.
    #[test]
    fn fresh_fetch_after_failure_resets_state() {
        let mut outcomes = Vec::new();
        for _ in 0..4 {
            outcomes.push(Outcome::Fail(Weather::Status(503)));
        }
        outcomes.push(Outcome::Body(b"later".to_vec()));
        let (_inner, wrapper, _clock, _receiver) = setup(outcomes);
        let err = match wrapper.fetch(get("a")) {
            Ok(_) => unreachable!("four 503s exhaust the budget"),
            Err(err) => err,
        };
        assert_eq!(err.code(), "streaming.net.http-status");
        assert_eq!(wrapper.state(), ReconnectState::Failed { attempts: 4 });
        wrapper.fetch(get("a")).must();
        assert_eq!(wrapper.state(), ReconnectState::Steady, "new episode");
    }

    /// Exhaustion: the bounded budget is spent, the LAST underlying
    /// error surfaces honestly (typed weather, not a synthesized
    /// "exhausted" — module docs), the state and events record the
    /// attempt count.
    #[test]
    fn retry_exhaustion_surfaces_last_error() {
        let (inner, wrapper, clock, receiver) = setup(vec![Outcome::Fail(Weather::Status(503))]);
        let err = match wrapper.fetch(get("x")) {
            Ok(_) => unreachable!("an always-503 server must exhaust"),
            Err(err) => err,
        };
        assert!(matches!(err, NetworkError::HttpStatus { code: 503, .. }));
        assert_eq!(wrapper.state(), ReconnectState::Failed { attempts: 4 });
        let stats = wrapper.stats();
        assert_eq!(stats.fetches, 1);
        assert_eq!(stats.retries, 3, "max_retries bounds the retries");
        assert_eq!(stats.exhausted, 1);
        assert_eq!(stats.recovered, 0);
        assert_eq!(inner.requests().len(), 4, "1 + 3 attempts");
        assert_eq!(sleeps(&clock).len(), 3);
        let events = drain(&receiver);
        assert_eq!(
            events.last(),
            Some(&StreamEvent::ReconnectFailed { attempts: 4 })
        );
        assert_eq!(events.len(), 4, "3 reconnecting + 1 failed");
    }

    /// A non-retryable failure passes through immediately: no retry,
    /// no sleep, no state change.
    #[test]
    fn non_retryable_failure_passes_through() {
        let (inner, wrapper, clock, receiver) = setup(vec![Outcome::Fail(Weather::Status(404))]);
        let err = match wrapper.fetch(get("x")) {
            Ok(_) => unreachable!("404 is the server's answer, not weather"),
            Err(err) => err,
        };
        assert!(matches!(err, NetworkError::HttpStatus { code: 404, .. }));
        assert_eq!(inner.requests().len(), 1);
        assert_eq!(wrapper.stats().retries, 0);
        assert!(clock.sleeps().is_empty(), "no backoff for a final answer");
        assert_eq!(wrapper.state(), ReconnectState::Steady);
        assert!(drain(&receiver).is_empty(), "no transitions to report");
    }

    /// Resume from offset: the re-issued request carries the IDENTICAL
    /// `Range:` header — byte-exact resume (the caller's cursor never
    /// moved; the retry re-requests precisely the missing span).
    #[test]
    fn resume_from_offset_reissues_identical_range() {
        let (inner, wrapper, _clock, _receiver) = setup(vec![
            Outcome::Fail(Weather::Timeout),
            Outcome::Body(b"0123456789abcdef".to_vec()),
        ]);
        let request = HttpRequest::get("http://unit.test/media").with_range(ByteRange::Bounded {
            start: 4096,
            end_exclusive: 4112,
        });
        let response = wrapper.fetch(request).must();
        assert_eq!(response.body, b"0123456789abcdef");
        let requests = inner.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].range, requests[1].range, "identical range");
        assert_eq!(
            requests[1].range,
            Some(ByteRange::Bounded {
                start: 4096,
                end_exclusive: 4112
            }),
            "the retry resumes from the consumed position, not byte 0"
        );
        assert_eq!(requests[0].url, requests[1].url);
    }

    /// Generation-safe cancellation, the late response: a fetch that
    /// completes AFTER its generation was invalidated is discarded —
    /// the body never surfaces, the caller sees `Cancelled`.
    #[test]
    fn late_response_discarded() {
        let counter = GenerationCounter::new();
        let inner = ScriptedFetch::with_counter(
            counter.clone(),
            vec![Outcome::BodyAfterBump(b"stale-bytes".to_vec())],
        );
        let clock = Arc::new(FakeClock::new());
        let (hub, receiver) = StreamEventHub::with_capacity(8);
        let wrapper = RetryFetch::new_checked(
            Arc::clone(&inner) as Arc<dyn HttpFetch>,
            fast_policy(),
            Arc::clone(&clock) as Arc<dyn WallClock>,
        )
        .must()
        .with_event_hub(hub)
        .with_generation(counter.clone());
        let err = match wrapper.fetch(get("x")) {
            Ok(_) => unreachable!("a superseded response must never surface"),
            Err(err) => err,
        };
        assert_eq!(counter.current(), 1, "the scripted fetch bumped");
        assert!(
            matches!(err, NetworkError::Cancelled { generation: 0 }),
            "the late response is discarded, typed: {err:?}"
        );
        assert_eq!(wrapper.stats().superseded, 1);
        assert_eq!(wrapper.state(), ReconnectState::Steady, "retry state void");
        assert_eq!(
            drain(&receiver),
            vec![StreamEvent::Superseded { generation: 0 }]
        );
    }

    /// Generation-safe cancellation, mid-backoff: a bump during the
    /// retry sleep cancels BEFORE another request goes out under the
    /// stale generation (the clock's sleep does the bumping here —
    /// the injected port is the honest interleave point).
    #[test]
    fn cancellation_during_backoff() {
        /// A wall clock whose sleep bumps a generation before advancing
        /// virtual time (the "world moved on while we waited" shape).
        struct BumpingClock {
            inner: FakeClock,
            counter: GenerationCounter,
        }
        impl WallClock for BumpingClock {
            fn now_ms(&self) -> u64 {
                self.inner.now_ms()
            }
            fn sleep_ms(&self, ms: u64) {
                self.counter.bump();
                self.inner.sleep_ms(ms);
            }
        }
        let counter = GenerationCounter::new();
        let clock = Arc::new(BumpingClock {
            inner: FakeClock::new(),
            counter: counter.clone(),
        });
        let inner = ScriptedFetch::new(vec![Outcome::Fail(Weather::Timeout)]);
        let (hub, receiver) = StreamEventHub::with_capacity(8);
        let wrapper = RetryFetch::new_checked(
            Arc::clone(&inner) as Arc<dyn HttpFetch>,
            fast_policy(),
            Arc::clone(&clock) as Arc<dyn WallClock>,
        )
        .must()
        .with_event_hub(hub)
        .with_generation(counter);
        let err = match wrapper.fetch(get("x")) {
            Ok(_) => unreachable!("cancelled mid-backoff"),
            Err(err) => err,
        };
        assert!(
            matches!(err, NetworkError::Cancelled { generation: 0 }),
            "{err:?}"
        );
        assert_eq!(
            inner.requests().len(),
            1,
            "no second request under the stale generation"
        );
        assert_eq!(wrapper.stats().superseded, 1);
        let d1 = BackoffSchedule::new(fast_policy(), 0).delay_ms(0);
        assert_eq!(
            drain(&receiver),
            vec![
                StreamEvent::Reconnecting {
                    attempt: 1,
                    delay_ms: d1
                },
                StreamEvent::Superseded { generation: 0 },
            ]
        );
    }

    /// A superseded FAILURE is discarded too — the wrapper answers
    /// `Cancelled`, never re-tries under the stale generation, and
    /// never surfaces the stale error as if it were fresh weather.
    #[test]
    fn superseded_failure_discarded() {
        let counter = GenerationCounter::new();
        let inner = ScriptedFetch::with_counter(
            counter.clone(),
            vec![Outcome::FailAndBump(Weather::Timeout)],
        );
        let clock = Arc::new(FakeClock::new());
        let wrapper = RetryFetch::new_checked(
            Arc::clone(&inner) as Arc<dyn HttpFetch>,
            fast_policy(),
            Arc::clone(&clock) as Arc<dyn WallClock>,
        )
        .must()
        .with_generation(counter);
        let err = match wrapper.fetch(get("x")) {
            Ok(_) => unreachable!("superseded"),
            Err(err) => err,
        };
        assert!(matches!(err, NetworkError::Cancelled { generation: 0 }));
        assert_eq!(inner.requests().len(), 1, "no retry after the bump");
        assert!(clock.sleeps().is_empty(), "no wait either");
    }

    /// The hub is bounded: emits never block, overflow drops and
    /// counts (D-1 — a host that does not drain loses events before
    /// it loses memory).
    #[test]
    fn hub_is_bounded_and_counts_drops() {
        let (hub, receiver) = StreamEventHub::with_capacity(2);
        assert!(hub.emit(StreamEvent::StallCleared), "first fits");
        assert!(hub.emit(StreamEvent::StallCleared), "second fills it");
        assert!(
            !hub.emit(StreamEvent::StallCleared),
            "the third drops, never blocks"
        );
        assert!(!hub.emit(StreamEvent::StallCleared));
        assert_eq!(hub.dropped(), 2);
        let held = drain(&receiver);
        assert_eq!(held.len(), 2, "exactly the capacity was held");
        // A drained hub accepts again.
        assert!(hub.emit(StreamEvent::StallCleared));
        assert_eq!(hub.dropped(), 2, "delivery does not un-drop");
    }

    /// The buffer-signal bridge: watermark edges and stall edges
    /// forward; steady observations forward nothing.
    #[test]
    fn hub_forwards_buffer_transitions() {
        let (hub, receiver) = StreamEventHub::with_capacity(8);
        assert!(hub.forward_signal(BufferSignal::Filled));
        assert!(hub.forward_signal(BufferSignal::Drained));
        assert!(hub.forward_signal(BufferSignal::StallPredicted { eta_ns: 900 }));
        assert!(hub.forward_signal(BufferSignal::StallCleared));
        assert!(!hub.forward_signal(BufferSignal::Steady));
        assert_eq!(
            drain(&receiver),
            vec![
                StreamEvent::Buffering { progress: 1.0 },
                StreamEvent::Buffering { progress: 0.0 },
                StreamEvent::StallPredicted { eta_ns: 900 },
                StreamEvent::StallCleared,
            ]
        );
    }

    /// The composite health verdict: every combination of buffer level,
    /// fill history and reconnect state maps (the engine seam's truth
    /// table — the module docs' table is this test).
    #[test]
    fn health_state_maps_both_models() {
        use HealthState::{self, *};
        let reconnecting = ReconnectState::Reconnecting {
            attempt: 1,
            delay_ms: 50,
        };
        // Reconnect state dominates the connection questions.
        assert_eq!(
            health_state(
                BufferLevel::High,
                true,
                ReconnectState::Failed { attempts: 4 }
            ),
            Failed
        );
        assert_eq!(
            health_state(BufferLevel::High, true, reconnecting),
            Reconnecting
        );
        assert_eq!(
            health_state(BufferLevel::Low, false, reconnecting),
            Reconnecting
        );
        // Recovering: below the fill watermarks (or not yet proven).
        assert_eq!(
            health_state(
                BufferLevel::Low,
                true,
                ReconnectState::Recovering { attempts: 2 }
            ),
            Recovering
        );
        assert_eq!(
            health_state(
                BufferLevel::High,
                true,
                ReconnectState::Recovering { attempts: 2 }
            ),
            Steady,
            "reconnected AND refilled is steady"
        );
        // Steady reconnect: the buffer model answers alone.
        assert_eq!(
            health_state(BufferLevel::High, false, ReconnectState::Steady),
            Steady
        );
        assert_eq!(
            health_state(BufferLevel::Low, false, ReconnectState::Steady),
            Buffering
        );
        assert_eq!(
            health_state(BufferLevel::Low, true, ReconnectState::Steady),
            Rebuffering
        );
        assert_eq!(
            health_state(BufferLevel::High, true, ReconnectState::Steady),
            Steady
        );
        // The enum is reachable by name for the host's matches.
        let _ = HealthState::Steady;
    }

    /// Without a hub the wrapper stays silent but fully functional
    /// (state and stats remain queryable — the hub is telemetry, not
    /// a dependency).
    #[test]
    fn wrapper_works_without_hub() {
        let inner = ScriptedFetch::new(vec![
            Outcome::Fail(Weather::Timeout),
            Outcome::Body(b"ok".to_vec()),
        ]);
        let clock = Arc::new(FakeClock::new());
        let wrapper = RetryFetch::new_checked(
            Arc::clone(&inner) as Arc<dyn HttpFetch>,
            fast_policy(),
            Arc::clone(&clock) as Arc<dyn WallClock>,
        )
        .must();
        let response = wrapper.fetch(get("x")).must();
        assert_eq!(response.body, b"ok");
        assert_eq!(wrapper.stats().retries, 1);
        assert_eq!(wrapper.state(), ReconnectState::Recovering { attempts: 1 });
        // The generation handle is shared, not borrowed: the host can
        // clone it out and bump from its own thread of control.
        let handle = wrapper.generation();
        assert_eq!(handle.current(), 0);
    }

    /// The default policy is the documented bounded posture, and the
    /// default constructor wires the system clock's shared handle
    /// (the injection doctrine's production default).
    #[test]
    fn default_policy_is_bounded() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, DEFAULT_MAX_RETRIES);
        assert_eq!(policy.initial_ms, DEFAULT_INITIAL_BACKOFF_MS);
        assert_eq!(policy.max_ms, DEFAULT_MAX_BACKOFF_MS);
        policy.validate().must();
        let inner = ScriptedFetch::new(vec![Outcome::Body(b"x".to_vec())]);
        let wrapper = RetryFetch::new(
            Arc::clone(&inner) as Arc<dyn HttpFetch>,
            SystemClock::shared(),
        )
        .must();
        assert_eq!(wrapper.policy(), policy);
        wrapper.fetch(get("x")).must();
    }
}
