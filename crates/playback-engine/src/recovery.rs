//! Environment-level recovery policies (card 3.C3; doc 03 §4.1(d),
//! §4.6, §4.9(d), §4.11(d); doc 05 §3.2/§3.3).
//!
//! # What this module is
//!
//! The three **environment** failure policies — the failures that
//! come from the *world around the media*, not from the media's own
//! bytes (media-level recovery is card 3.C2, `tests/recovery_media.rs`):
//!
//! - [`SourceRetryGuard`] — the **interrupted network source**
//!   (doc 03 §4.1(d)): a live/network read that fails stays
//!   non-fatal while a *retry window* — bounded attempts with
//!   bounded backoff inside a bounded virtual-time window — has room
//!   left; exhaustion answers the typed network error
//!   ([`media_types::error::NetworkCause::ReconnectExhausted`]),
//!   never an infinite retry, never a fabricated read.
//! - [`AudioDeviceGuard`] — the **audio device loss** policies
//!   (doc 03 §4.9(d)): *suspend* (pause the media clock, remember
//!   the position, emit the typed `audio.device-lost` catalog
//!   event) and *reopen* (re-acquire the device with the same
//!   bounded-window discipline, resume from the remembered
//!   position, tolerate repeated loss). The doc's
//!   `[PROPOSAL: suspend-and-notify default]` is the implemented
//!   policy: when re-acquisition cannot succeed, the session stays
//!   suspended at the remembered position with a non-fatal
//!   `playback.error` — playback is *never silently restarted*.
//! - [`SurfaceGuard`] — the **renderer surface loss** (doc 03
//!   §4.11(d)): mark the surface dirty, request a recreate, hold
//!   compositing while audio continues, and resume presenting when
//!   the new surface arrives — no engine restart, no pipeline
//!   teardown, and resize stays non-fatal by construction.
//!
//! # The seam these policies sit at
//!
//! Each guard is a **pure policy core**: it observes typed facts and
//! answers a [`RecoveryStep`] — one [`RecoveryAction`] for the
//! engine side plus zero or more catalog [`commands::EventKind`]s
//! for the wire side. The guards never touch the engine directly;
//! the host (input layer, audio pump, renderer thread, or a test)
//! drives them. This is the same doctrine as the event throttle
//! (doc 03 §8 / `events.rs`): *policy is data, the executor owns
//! mutation*.
//!
//! State-level moves flow through the **command layer** exactly as
//! doc 05 §3 prescribes:
//!
//! - the suspend action is realized by submitting `playback.pause`
//!   (the engine's `Pause` row freezes the clock, doc 03 §4.6), and
//!   the resume action by `playback.play`;
//! - the exhaustion action carries the typed
//!   [`media_types::EngineError`] that the engine's `Failed` input
//!   already knows how to emit as the fatal `playback.error`
//!   (`event.rs`'s wire mapping: stage `input`, code `network`,
//!   `fatal: true`) followed by `state-changed → error`;
//! - the guards' own emissions reuse **existing catalog rows only**
//!   (`audio.device-lost`, `audio.device-changed`, `playback.error`
//!   non-fatal, `session.buffering` bookends) — no new event kinds
//!   are invented.
//!
//! # Time doctrine
//!
//! Every window takes `now_ns` **from its caller** (the
//! `media-clock` drift-correction doctrine, doc 03 §8): the guards
//! are deterministic under virtual time, no sleeps anywhere. Tests
//! step a shared atomic clock between engine polls.
//!
//! # Event ownership (who may emit what)
//!
//! The retry window's `session.buffering` bookends have **two
//! legitimate owners** and a host bridges exactly one:
//!
//! - the **engine**, when the stall surfaces as queue starvation —
//!   the underrun row opens the episode and the primed/error
//!   settle closes it (the common integrated path, proven by the
//!   engine-level tests in `tests/recovery_env.rs`);
//! - the **input-layer host**, when the source guard gates a demux
//!   thread's reads below the engine's sight — then the guard's
//!   own bookends (emitted here) are the wire truth.
//!
//! Bridging both would double every bookend; the engine-level tests
//! therefore assert the engine's episodes and deliberately drop the
//! guard's. `[PROPOSAL: doc 05 should name the buffering-episode
//! owner per integration shape; until then this module's docs are
//! the contract.]`
//!
//! # Gaps recorded (no invented kinds)
//!
//! - There is **no v1 catalog event for surface loss/reattachment**
//!   (doc 05 §3.2 has `audio.device-lost`/`-changed` but no
//!   `video.*` sibling). The loss surfaces as the non-fatal
//!   `playback.error` (the doc 03 §4.11(d) "explicit event" — the
//!   catalog's generic row); reattachment is observable through
//!   [`SurfaceGuard::compositing`] and the host's own create
//!   callback. `[PROPOSAL: video.surface-changed for symmetry with
//!   the audio rows.]`
//! - Mid-episode `session.buffering` progress during a *source*
//!   retry window is undefined (buffer fill is not knowable at the
//!   input layer); the guard emits only the `0.0`/`1.0` bookends.
//!   `[PROPOSAL: define retry-schedule progress semantics.]`
//!
//! # Allocation note (engineering directive D-1)
//!
//! Recovery is an *event-rate* path, not a per-frame hot path:
//! losses happen at human timescales, and every guard method runs
//! at most once per observed fact or per poll. The `Vec<EventKind>`
//! in a step stays empty (`Vec::new()` allocates nothing) on the
//! healthy path and holds at most one event on any recovery path,
//! so the steady-state cost of carrying these policies is zero.

use std::collections::BTreeMap;

use commands::ErrorCode;
use commands::EventKind;
use commands::Stage;
use media_types::EngineError;
use media_types::PipelineStage;
use media_types::error::ErrorContext;
use media_types::error::ErrorKind;
use media_types::error::NetworkCause;

// ---------------------------------------------------------------------------
// The shared bounded-backoff window
// ---------------------------------------------------------------------------

/// A bounded retry budget: how many attempts, inside how large a
/// virtual-time window, with what backoff schedule (card 3.C3's
/// "bounded retries with backoff within a configured window").
///
/// Two independent bounds — attempt count and window duration —
/// either of which ends the retrying. Backoff grows exponentially
/// from `base_backoff_ns` and is capped at `max_backoff_ns`
/// (saturating arithmetic; no overflow, no wrap).
///
/// The figures are `[PROPOSAL: policy defaults — doc 03 §4.1(d)
/// says only "retrying within policy"; the numbers await the
/// product-config card that wires `product.toml` limits]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryWindow {
    /// Maximum **retry** attempts after the first failure (the
    /// first failure itself is not a retry). `0` exhausts on the
    /// first failure.
    pub max_attempts: u32,
    /// The total virtual-time span the window may cover, measured
    /// from the first failure. A retry whose backoff would land
    /// beyond the window's end is never scheduled.
    pub window_ns: u64,
    /// The backoff before the first retry.
    pub base_backoff_ns: u64,
    /// The backoff cap (exponential growth stops here).
    pub max_backoff_ns: u64,
}

impl RetryWindow {
    /// The default window: 6 retries inside 10 s, backoff 250 ms
    /// doubling to a 4 s cap. `[PROPOSAL figures]`
    pub const DEFAULT: RetryWindow = RetryWindow {
        max_attempts: 6,
        window_ns: 10_000_000_000,
        base_backoff_ns: 250_000_000,
        max_backoff_ns: 4_000_000_000,
    };

    /// The backoff that precedes `attempt` (1-based: the wait
    /// before the *first* retry is the base). Exponential, capped,
    /// saturating — `min(base · 2^(attempt-1), max_backoff_ns)`.
    #[must_use]
    pub fn backoff_ns(&self, attempt: u32) -> u64 {
        let shift = u64::from(attempt.saturating_sub(1)).min(63);
        let grown = self.base_backoff_ns.saturating_mul(1u64 << shift);
        grown.min(self.max_backoff_ns)
    }
}

impl Default for RetryWindow {
    fn default() -> RetryWindow {
        RetryWindow::DEFAULT
    }
}

// ---------------------------------------------------------------------------
// Steps and actions
// ---------------------------------------------------------------------------

/// What the host should do next, as decided by a guard. Payloads
/// move (the typed fatal is consumed by the engine's `Failed`
/// input); the engine-facing half of a [`RecoveryStep`].
#[derive(Debug)]
#[non_exhaustive]
pub enum RecoveryAction {
    /// Nothing to do for this observation.
    None,
    /// A source read may be re-attempted now: the backoff elapsed
    /// and the window still has room. A *permission*, not a
    /// one-shot grant — the caller performs the read and reports
    /// the outcome back through
    /// [`SourceRetryGuard::read`](SourceRetryGuard::read).
    RetryRead {
        /// The upcoming attempt number (1-based).
        attempt: u32,
    },
    /// The retry budget is spent (or the failure was never
    /// retryable): the typed, stage-tagged fatal. Apply it through
    /// the engine's `Failed` input — it emits the catalog
    /// `playback.error` (fatal) and moves the state to `error`.
    /// Never a panic, never a silent success.
    FailSource(EngineError),
    /// Suspend the session at `position_ns`: submit `playback.pause`
    /// through the command layer (the engine's `Pause` row freezes
    /// the media clock — doc 03 §4.6).
    SuspendClock {
        /// The position to remember (the clock's `now` at loss).
        position_ns: i64,
    },
    /// Resume the session from the *remembered* position: submit
    /// `playback.play` (the engine's `Play` row resumes the clock
    /// from its frozen value — no restart, doc 03 §4.7(e)).
    ResumeClock {
        /// The remembered suspend position (the engine restarts
        /// nothing; the clock continues from its own frozen value).
        position_ns: i64,
    },
    /// Re-acquire the audio device now (the reopen seam is the
    /// host's — doc 03 §4.9's `AudioSink::open`; the guard owns
    /// *when*, the host owns *how*). Report the outcome through
    /// [`AudioDeviceGuard::observe_reopen_result`](AudioDeviceGuard::observe_reopen_result).
    ReopenDevice,
    /// Ask the host for a fresh render surface (the renderer thread
    /// recreates its swapchain/surface; the engine keeps running).
    RequestSurfaceRecreate,
}

/// One guard decision: the engine-side action plus the catalog
/// events the observation produced (wire-side truth, doc 05 §3).
///
/// `events` is empty on every healthy-path call (no allocation);
/// it carries at most one event on any recovery path.
#[derive(Debug)]
pub struct RecoveryStep {
    /// What the host should do.
    pub action: RecoveryAction,
    /// The catalog events for the wire (already
    /// [`commands::EventKind`] — no invention, no mapping needed).
    pub events: Vec<EventKind>,
}

impl RecoveryStep {
    /// A step with nothing to say (the healthy path).
    fn quiet() -> RecoveryStep {
        RecoveryStep {
            action: RecoveryAction::None,
            events: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// 1. Interrupted network source — the retry window
// ---------------------------------------------------------------------------

/// One completed source read (the observation half of the
/// [`SourceRetryGuard`] protocol).
#[derive(Debug)]
#[non_exhaustive]
pub enum ReadOutcome {
    /// The read delivered `units` of data (bytes or packets — the
    /// caller's unit; only the count of *facts* matters to the
    /// window). Data is never fabricated: `units > 0` means real
    /// bytes crossed the seam.
    Data {
        /// How many units the read delivered.
        units: u64,
    },
    /// The read failed with this typed error.
    Interrupted(EngineError),
}

/// Whether a failure is retryable network weather (doc 03 §4.1(d)
/// "live/network source → stay in Buffering and keep retrying within
/// policy").
///
/// - `Network { Timeout | ConnectionLost | Dns }` retry: transient
///   network weather.
/// - `Network { Tls }` does **not** retry: a certificate failure
///   retried on a weaker path would be a downgrade, not a recovery
///   (the L0 taxonomy's own rule, `media-types`'s `NetworkCause`).
/// - `Network { ReconnectExhausted }` does not retry: a lower layer
///   already spent its budget — surfacing it honestly beats
///   re-spending ours.
/// - Every non-network kind (corrupt media, unsupported, device…)
///   is out of this guard's scope: media-level policy owns it
///   (card 3.C2).
#[must_use]
pub fn is_retryable_network(error: &EngineError) -> bool {
    matches!(
        error.kind,
        ErrorKind::Network {
            cause: NetworkCause::Timeout | NetworkCause::ConnectionLost | NetworkCause::Dns
        }
    )
}

/// The typed error the window answers when it is spent: stage
/// [`PipelineStage::Open`] (the source seam), kind
/// `Network { ReconnectExhausted { attempts } }`, context
/// `source.retry.exhausted`, with the last observed failure chained
/// as the error's `source` when one is at hand (the
/// `NoDecodableKnown` chaining pattern — structured data, never a
/// secret).
#[must_use]
pub fn retry_exhausted_error(attempts: u32, last: Option<&EngineError>) -> EngineError {
    let mut error = EngineError::new(
        PipelineStage::Open,
        ErrorKind::Network {
            cause: NetworkCause::ReconnectExhausted { attempts },
        },
        ErrorContext::new("source.retry.exhausted"),
    );
    if let Some(last) = last {
        // A shallow chain root: stage + kind + code (the boxed
        // `source` chain is not `Clone` — the Fatal-clone doctrine,
        // `event.rs`).
        error = error.with_source(EngineError::new(
            last.stage,
            last.kind.clone(),
            ErrorContext {
                code: last.context.code.clone(),
            },
        ));
    }
    error
}

/// The retry-window phase.
#[derive(Debug)]
enum SourcePhase {
    /// The source is delivering; no window is open.
    Streaming,
    /// A window is open: `retries_used` attempts have failed, the
    /// next retry is due at `next_retry_ns`, and the window ends at
    /// `window_end_ns` (both bounds still hold).
    Waiting {
        /// Failed retry attempts so far.
        retries_used: u32,
        /// When the next retry may run (virtual ns).
        next_retry_ns: u64,
        /// When the window ends (virtual ns).
        window_end_ns: u64,
    },
    /// Terminal: the budget is spent. The typed error stands; every
    /// later observation of the dead source answers it again
    /// (idempotent — the honest answer does not decay).
    Exhausted { error: EngineError },
}

/// The interrupted-network-source policy (card 3.C3 item 1).
///
/// Drive it as a read cycle: the host performs a read, reports the
/// outcome ([`read`](SourceRetryGuard::read)), and between reads
/// asks [`poll`](SourceRetryGuard::poll) whether a retry is due.
/// The guard answers [`RecoveryAction::RetryRead`] permissions
/// while the window has room and [`RecoveryAction::FailSource`]
/// with the typed network error when it does not — there is no
/// state in which it retries forever: the attempt count and the
/// window duration bound it independently.
///
/// While a window is open the guard emits the `session.buffering`
/// bookends (`0.0` at open, `1.0` at close — doc 05 §3.2's episode
/// discipline; see the module docs for the ownership rule). It
/// closes the episode on resume *and* on exhaustion, mirroring the
/// engine's own settle-close (an open `0.0` must never strand on
/// the wire).
#[derive(Debug)]
pub struct SourceRetryGuard {
    policy: RetryWindow,
    phase: SourcePhase,
    windows_opened: u64,
}

impl SourceRetryGuard {
    /// A guard with the given window policy.
    #[must_use]
    pub fn new(policy: RetryWindow) -> SourceRetryGuard {
        SourceRetryGuard {
            policy,
            phase: SourcePhase::Streaming,
            windows_opened: 0,
        }
    }

    /// Report one completed read at virtual time `now_ns`.
    ///
    /// - `Data` — if a window was open it closes here (the `1.0`
    ///   bookend): data flowed again, the episode is over. The
    ///   step's action is `None` — the data itself is the
    ///   progress.
    /// - `Interrupted(err)` — first failure opens the window (the
    ///   `0.0` bookend) and schedules the first retry; a failed
    ///   *retry* advances the schedule; a spent budget (attempts
    ///   or window) answers `FailSource` with
    ///   [`retry_exhausted_error`]; a non-retryable error is an
    ///   immediate typed fatal passthrough (TLS, corrupt, …).
    pub fn read(&mut self, now_ns: u64, outcome: ReadOutcome) -> RecoveryStep {
        match outcome {
            ReadOutcome::Data { .. } => self.observe_data(),
            ReadOutcome::Interrupted(error) => self.observe_failure(now_ns, error),
        }
    }

    /// Ask whether a retry is due at virtual time `now_ns`.
    ///
    /// Answers `RetryRead { attempt }` when the backoff elapsed and
    /// the window still has room; answers `FailSource` when the
    /// window *expired by time* (attempts may remain — the window
    /// is the second, independent bound); on an exhausted guard it
    /// answers the standing terminal (the same idempotent honesty
    /// as [`read`](SourceRetryGuard::read): a dead source does not
    /// come back to life because time passed).
    pub fn poll(&mut self, now_ns: u64) -> RecoveryStep {
        let (retries_used, next_retry_ns, window_end_ns) = match &self.phase {
            SourcePhase::Waiting {
                retries_used,
                next_retry_ns,
                window_end_ns,
            } => (*retries_used, *next_retry_ns, *window_end_ns),
            SourcePhase::Exhausted { .. } => return self.terminal_answer(),
            SourcePhase::Streaming => return RecoveryStep::quiet(),
        };
        if now_ns >= window_end_ns {
            return self.exhaust_by_time(retries_used);
        }
        if now_ns >= next_retry_ns {
            return RecoveryStep {
                action: RecoveryAction::RetryRead {
                    attempt: retries_used + 1,
                },
                events: Vec::new(),
            };
        }
        RecoveryStep::quiet()
    }

    /// Whether a retry window is currently open.
    #[must_use]
    pub fn retrying(&self) -> bool {
        matches!(self.phase, SourcePhase::Waiting { .. })
    }

    /// Failed retry attempts in the open window (`None` while
    /// streaming; the count while a window is open or after
    /// exhaustion — the standing budget truth).
    #[must_use]
    pub fn retries_used(&self) -> Option<u32> {
        match &self.phase {
            SourcePhase::Streaming => None,
            SourcePhase::Waiting { retries_used, .. } => Some(*retries_used),
            SourcePhase::Exhausted { error } => match error.kind {
                ErrorKind::Network {
                    cause: NetworkCause::ReconnectExhausted { attempts },
                } => Some(attempts),
                _ => Some(0),
            },
        }
    }

    /// When the next retry is due (`None` when none is scheduled).
    #[must_use]
    pub fn next_retry_ns(&self) -> Option<u64> {
        match &self.phase {
            SourcePhase::Waiting { next_retry_ns, .. } => Some(*next_retry_ns),
            _ => None,
        }
    }

    /// When the open window ends (`None` when no window is open).
    #[must_use]
    pub fn window_end_ns(&self) -> Option<u64> {
        match &self.phase {
            SourcePhase::Waiting { window_end_ns, .. } => Some(*window_end_ns),
            _ => None,
        }
    }

    /// How many retry windows have been opened over this guard's
    /// life (a recovered source that fails again opens a fresh
    /// window with a fresh budget — per-episode bounding, never a
    /// lifetime budget that a long-lived stream could exhaust).
    #[must_use]
    pub fn windows_opened(&self) -> u64 {
        self.windows_opened
    }

    /// Data flowed: close any open window with the resume bookend.
    fn observe_data(&mut self) -> RecoveryStep {
        if self.retrying() {
            self.phase = SourcePhase::Streaming;
            return RecoveryStep {
                action: RecoveryAction::None,
                events: vec![EventKind::SessionBuffering { progress: 1.0 }],
            };
        }
        RecoveryStep::quiet()
    }

    /// A read failed: open, advance, or terminate the window.
    fn observe_failure(&mut self, now_ns: u64, error: EngineError) -> RecoveryStep {
        if !is_retryable_network(&error) {
            // Not retryable (TLS, exhausted-below-us, or not network
            // at all): the typed fatal passes through unchanged —
            // retrying it would be a downgrade or a scope breach,
            // and fabricating success is not on the table.
            return self.fail_early(error);
        }
        if let SourcePhase::Exhausted { .. } = self.phase {
            // The standing answer to a dead source.
            return self.terminal_answer();
        }
        if !self.retrying() {
            // First failure of a fresh episode: open the window.
            self.windows_opened += 1;
            let window_end_ns = now_ns.saturating_add(self.policy.window_ns);
            let next_retry_ns = now_ns.saturating_add(self.policy.backoff_ns(1));
            if next_retry_ns > window_end_ns {
                // Degenerate window (shorter than one backoff):
                // honest exhaustion with zero retries made.
                return self.exhaust_after_failure(0, error);
            }
            self.phase = SourcePhase::Waiting {
                retries_used: 0,
                next_retry_ns,
                window_end_ns,
            };
            return RecoveryStep {
                action: RecoveryAction::None,
                events: vec![EventKind::SessionBuffering { progress: 0.0 }],
            };
        }
        // A granted retry was performed and failed again.
        let (retries_used, window_end_ns) = match &self.phase {
            SourcePhase::Waiting {
                retries_used,
                window_end_ns,
                ..
            } => (*retries_used + 1, *window_end_ns),
            // The retrying() check above pins the phase.
            _ => unreachable!("retrying implies Waiting"),
        };
        if retries_used >= self.policy.max_attempts {
            return self.exhaust_after_failure(retries_used, error);
        }
        let next_retry_ns = now_ns.saturating_add(self.policy.backoff_ns(retries_used + 1));
        if next_retry_ns > window_end_ns {
            return self.exhaust_after_failure(retries_used, error);
        }
        self.phase = SourcePhase::Waiting {
            retries_used,
            next_retry_ns,
            window_end_ns,
        };
        RecoveryStep::quiet()
    }

    /// An immediate typed fatal (non-retryable failure): close any
    /// open episode bookend, keep the original error, go terminal.
    fn fail_early(&mut self, error: EngineError) -> RecoveryStep {
        let mut events = Vec::new();
        if self.retrying() {
            events.push(EventKind::SessionBuffering { progress: 1.0 });
        }
        self.phase = SourcePhase::Exhausted {
            error: shallow_error(&error),
        };
        RecoveryStep {
            action: RecoveryAction::FailSource(error),
            events,
        }
    }

    /// Exhaust the window after `attempts` failed retries; `error`
    /// is the final observed failure (chained as the typed
    /// error's `source`). Answers the **typed network error**
    /// ([`retry_exhausted_error`]) — the card's exhaustion
    /// contract.
    fn exhaust_after_failure(&mut self, attempts: u32, error: EngineError) -> RecoveryStep {
        let mut events = Vec::new();
        if self.retrying() {
            // The settle bookend (the engine's own error-transition
            // discipline — never strand an open 0.0).
            events.push(EventKind::SessionBuffering { progress: 1.0 });
        }
        let error = retry_exhausted_error(attempts, Some(&error));
        self.phase = SourcePhase::Exhausted {
            error: shallow_error(&error),
        };
        RecoveryStep {
            action: RecoveryAction::FailSource(error),
            events,
        }
    }

    /// Time-expiry exhaustion (no fresh failure at hand): the
    /// window ended with `attempts` retries made.
    fn exhaust_by_time(&mut self, attempts: u32) -> RecoveryStep {
        let mut events = Vec::new();
        if self.retrying() {
            events.push(EventKind::SessionBuffering { progress: 1.0 });
        }
        let error = retry_exhausted_error(attempts, None);
        self.phase = SourcePhase::Exhausted {
            error: shallow_error(&error),
        };
        RecoveryStep {
            action: RecoveryAction::FailSource(error),
            events,
        }
    }

    /// The idempotent terminal answer (shallow copy of the standing
    /// error — the boxed `source` chain moves once, with the first
    /// `FailSource`).
    fn terminal_answer(&mut self) -> RecoveryStep {
        match &self.phase {
            SourcePhase::Exhausted { error } => RecoveryStep {
                action: RecoveryAction::FailSource(shallow_error(error)),
                events: Vec::new(),
            },
            _ => RecoveryStep::quiet(),
        }
    }
}

/// Rebuild an [`EngineError`] without its boxed `source` chain (the
/// Fatal-clone doctrine from `event.rs`: stage, kind and context
/// code carry the wire-visible truth; chains move, they don't
/// duplicate).
fn shallow_error(error: &EngineError) -> EngineError {
    EngineError::new(
        error.stage,
        error.kind.clone(),
        ErrorContext {
            code: error.context.code.clone(),
        },
    )
}

// ---------------------------------------------------------------------------
// 2. Audio device loss — suspend and reopen
// ---------------------------------------------------------------------------

/// The device-loss policy phase.
#[derive(Debug)]
enum DevicePhase {
    /// A device is acquired; playback runs on it.
    Active {
        /// The active device's id.
        device_id: String,
    },
    /// Suspended after a loss (doc 03 §4.9(d) suspend-and-notify):
    /// the position is remembered, re-acquisition is being retried
    /// inside a bounded window, and the terminal notice has been
    /// emitted once the budget is spent.
    Suspended {
        /// The device that was lost.
        device_id: String,
        /// The remembered media position (ns) — resume continues
        /// from here, never from zero.
        position_ns: i64,
        /// Failed reopen attempts so far.
        reopen_used: u32,
        /// When the next reopen attempt may run (virtual ns).
        next_reopen_ns: u64,
        /// When the reopen window ends (virtual ns).
        window_end_ns: u64,
        /// Whether the non-fatal suspension notice has been emitted
        /// (the terminal of an unsuccessful reopen window — exactly
        /// once).
        notified: bool,
    },
}

/// The audio-device-loss policy (card 3.C3 item 2): **suspend**
/// (pause the media clock at the loss position, remember it, emit
/// `audio.device-lost`) plus **reopen** (re-acquire with bounded
/// backoff, resume from the remembered position, emit
/// `audio.device-changed`, tolerate repeated loss).
///
/// Every loss — including a loss of the replacement device — opens
/// a *fresh* suspend window with a *fresh* reopen budget: repeated
/// loss is tolerated without bound on the number of *episodes*
/// (each episode is individually bounded; the position is preserved
/// across all of them, and the session is never restarted).
///
/// The terminal of an unsuccessful window is the doc's
/// suspend-and-notify: a **non-fatal** `playback.error` (stage
/// `audio-output`, code `device`) while the session stays suspended
/// at the remembered position — the alternative (a fatal error)
/// would tear down a session whose media is perfectly fine.
#[derive(Debug)]
pub struct AudioDeviceGuard {
    policy: RetryWindow,
    phase: DevicePhase,
    losses: u64,
    reopens: u64,
}

impl AudioDeviceGuard {
    /// A guard with the given reopen window policy.
    #[must_use]
    pub fn new(policy: RetryWindow) -> AudioDeviceGuard {
        AudioDeviceGuard {
            policy,
            phase: DevicePhase::Active {
                device_id: String::new(),
            },
            losses: 0,
            reopens: 0,
        }
    }

    /// Observe the loss of the active device at virtual time
    /// `now_ns` (doc 03 §4.9's `SinkEvent::DeviceLost`).
    ///
    /// Emits the catalog `audio.device-lost` and answers
    /// [`RecoveryAction::SuspendClock`] — the host submits
    /// `playback.pause`, which freezes the media clock (the
    /// position is thereby preserved *by the engine*, and
    /// remembered here for the resume). The first reopen attempt is
    /// due immediately (a re-plugged device answers at once);
    /// later attempts follow the backoff schedule.
    pub fn observe_device_lost(
        &mut self,
        now_ns: u64,
        device_id: &str,
        position_ns: i64,
    ) -> RecoveryStep {
        self.losses += 1;
        self.phase = DevicePhase::Suspended {
            device_id: device_id.to_string(),
            position_ns,
            reopen_used: 0,
            next_reopen_ns: now_ns,
            window_end_ns: now_ns.saturating_add(self.policy.window_ns),
            notified: false,
        };
        RecoveryStep {
            action: RecoveryAction::SuspendClock { position_ns },
            events: vec![EventKind::AudioDeviceLost {
                device_id: device_id.to_string(),
            }],
        }
    }

    /// Ask whether a reopen attempt is due at virtual time `now_ns`
    /// (answers [`RecoveryAction::ReopenDevice`]), or emit the
    /// one-time non-fatal notice when the reopen window expired.
    pub fn poll(&mut self, now_ns: u64) -> RecoveryStep {
        let (device_id, reopen_used, next_reopen_ns, window_end_ns, notified) = match &self.phase {
            DevicePhase::Suspended {
                device_id,
                reopen_used,
                next_reopen_ns,
                window_end_ns,
                notified,
                ..
            } => (
                device_id.clone(),
                *reopen_used,
                *next_reopen_ns,
                *window_end_ns,
                *notified,
            ),
            DevicePhase::Active { .. } => return RecoveryStep::quiet(),
        };
        if !notified && now_ns >= window_end_ns {
            let notice = self.mark_notified(&device_id, reopen_used, None);
            return RecoveryStep {
                action: RecoveryAction::None,
                events: vec![notice],
            };
        }
        if !notified && reopen_used < self.policy.max_attempts && now_ns >= next_reopen_ns {
            return RecoveryStep {
                action: RecoveryAction::ReopenDevice,
                events: Vec::new(),
            };
        }
        RecoveryStep::quiet()
    }

    /// Report the outcome of a reopen attempt at virtual time
    /// `now_ns`.
    ///
    /// - `Ok(new_device_id)` — the device is re-acquired: emits the
    ///   catalog `audio.device-changed` and answers
    ///   [`RecoveryAction::ResumeClock`] from the remembered
    ///   position (the host submits `playback.play`; the clock
    ///   continues from its frozen value — a re-acquired device
    ///   never restarts the media).
    /// - `Err(err)` — the attempt failed: the backoff schedule
    ///   advances inside the window, or the budget is spent and the
    ///   one-time non-fatal notice is emitted (the session stays
    ///   suspended at the remembered position).
    pub fn observe_reopen_result(
        &mut self,
        now_ns: u64,
        result: Result<String, EngineError>,
    ) -> RecoveryStep {
        match result {
            Ok(device_id) => {
                let position_ns = match &self.phase {
                    DevicePhase::Suspended { position_ns, .. } => *position_ns,
                    DevicePhase::Active { .. } => return RecoveryStep::quiet(),
                };
                self.reopens += 1;
                self.phase = DevicePhase::Active {
                    device_id: device_id.clone(),
                };
                RecoveryStep {
                    action: RecoveryAction::ResumeClock { position_ns },
                    events: vec![EventKind::AudioDeviceChanged { device_id }],
                }
            }
            Err(error) => {
                let (device_id, reopen_used, window_end_ns) = match &self.phase {
                    DevicePhase::Suspended {
                        device_id,
                        reopen_used,
                        window_end_ns,
                        ..
                    } => (device_id.clone(), *reopen_used, *window_end_ns),
                    DevicePhase::Active { .. } => return RecoveryStep::quiet(),
                };
                let reopen_used = reopen_used + 1;
                let next_reopen_ns = now_ns.saturating_add(self.policy.backoff_ns(reopen_used + 1));
                if reopen_used >= self.policy.max_attempts || next_reopen_ns > window_end_ns {
                    let notice = self.mark_notified(&device_id, reopen_used, Some(&error));
                    return RecoveryStep {
                        action: RecoveryAction::None,
                        events: vec![notice],
                    };
                }
                if let DevicePhase::Suspended {
                    reopen_used: used,
                    next_reopen_ns: due,
                    ..
                } = &mut self.phase
                {
                    *used = reopen_used;
                    *due = next_reopen_ns;
                }
                RecoveryStep::quiet()
            }
        }
    }

    /// Whether the guard is currently suspended (device lost,
    /// reopen not yet succeeded).
    #[must_use]
    pub fn suspended(&self) -> bool {
        matches!(self.phase, DevicePhase::Suspended { .. })
    }

    /// The remembered suspend position (`None` while active).
    #[must_use]
    pub fn suspended_position_ns(&self) -> Option<i64> {
        match &self.phase {
            DevicePhase::Suspended { position_ns, .. } => Some(*position_ns),
            DevicePhase::Active { .. } => None,
        }
    }

    /// The active device's id (`None` while suspended; the empty
    /// string before any loss was observed on a default-constructed
    /// guard).
    #[must_use]
    pub fn active_device(&self) -> Option<&str> {
        match &self.phase {
            DevicePhase::Active { device_id } => Some(device_id.as_str()),
            DevicePhase::Suspended { .. } => None,
        }
    }

    /// Total device-loss episodes observed.
    #[must_use]
    pub fn losses(&self) -> u64 {
        self.losses
    }

    /// Total successful re-opens.
    #[must_use]
    pub fn reopens(&self) -> u64 {
        self.reopens
    }

    /// Emit (and latch) the one-time non-fatal suspension notice:
    /// the terminal of an unsuccessful reopen window.
    fn mark_notified(
        &mut self,
        device_id: &str,
        reopen_used: u32,
        last: Option<&EngineError>,
    ) -> EventKind {
        if let DevicePhase::Suspended { notified, .. } = &mut self.phase {
            *notified = true;
        }
        let mut details = BTreeMap::new();
        details.insert("context".to_string(), "audio.device.suspend".to_string());
        details.insert("device".to_string(), device_id.to_string());
        details.insert("reopen_attempts".to_string(), reopen_used.to_string());
        let message = match last {
            Some(error) => format!(
                "audio device `{device_id}` lost and re-acquisition failed \
                 ({error}); playback is suspended at the remembered position"
            ),
            None => format!(
                "audio device `{device_id}` lost and the re-acquisition \
                 window expired; playback is suspended at the remembered \
                 position"
            ),
        };
        EventKind::PlaybackError {
            stage: Stage::AudioOutput,
            code: ErrorCode::Device,
            message,
            details,
            fatal: false,
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Renderer surface loss — dirty, recreate, resume compositing
// ---------------------------------------------------------------------------

/// The renderer-surface-loss policy (card 3.C3 item 3): mark the
/// surface dirty, request a recreate, hold compositing while audio
/// continues, resume when the new surface arrives — the engine and
/// the session survive untouched (doc 03 §4.11(d): "the session
/// degrades to audio-only with an explicit event, or reattaches
/// when a new surface is provided").
///
/// The loss notice is the **non-fatal** `playback.error` (stage
/// `renderer`, code `device`) — the catalog's generic row; there is
/// no `video.surface-*` event in v1 (the recorded
/// `[PROPOSAL]` gap, see the module docs). Reattachment is
/// observable through [`SurfaceGuard::compositing`] and the host's
/// own surface-create callback. Resize never marks the surface
/// dirty (non-fatal by construction, doc 03 §4.11(d)).
#[derive(Debug, Default)]
pub struct SurfaceGuard {
    dirty: bool,
    losses: u64,
    recreations: u64,
    suppressed_presents: u64,
    resizes: u64,
}

impl SurfaceGuard {
    /// A guard over a healthy surface.
    #[must_use]
    pub fn new() -> SurfaceGuard {
        SurfaceGuard::default()
    }

    /// Observe the loss of the render surface (doc 03 §4.11(d)'s
    /// `RenderError::SurfaceLost` — window closed, GPU reset).
    ///
    /// Marks the surface dirty (compositing holds — see
    /// [`gate_present`](SurfaceGuard::gate_present)), emits the
    /// non-fatal audio-only-degradation event, and answers
    /// [`RecoveryAction::RequestSurfaceRecreate`]. Idempotent: a
    /// second loss before the recreate is the same loss (no
    /// duplicate event, no duplicate request).
    pub fn observe_surface_lost(&mut self) -> RecoveryStep {
        if self.dirty {
            return RecoveryStep::quiet();
        }
        self.dirty = true;
        self.losses += 1;
        let mut details = BTreeMap::new();
        details.insert("context".to_string(), "renderer.surface.lost".to_string());
        details.insert("slot".to_string(), "display".to_string());
        details.insert("cause".to_string(), "lost".to_string());
        RecoveryStep {
            action: RecoveryAction::RequestSurfaceRecreate,
            events: vec![EventKind::PlaybackError {
                stage: Stage::Renderer,
                code: ErrorCode::Device,
                message: "render surface lost: compositing suspended, \
                          audio-only playback continues"
                    .to_string(),
                details,
                fatal: false,
            }],
        }
    }

    /// Observe that the host provided a fresh surface: compositing
    /// resumes (the next presents flow — no engine restart, no
    /// pipeline teardown, no generation bump).
    ///
    /// Emits nothing — the recorded gap: v1 has no surface event
    /// kind (see the module docs' `[PROPOSAL]`).
    pub fn observe_surface_recreated(&mut self) -> RecoveryStep {
        if self.dirty {
            self.dirty = false;
            self.recreations += 1;
        }
        RecoveryStep::quiet()
    }

    /// Observe a surface resize: non-fatal by construction (doc 03
    /// §4.11(d)) — the surface is NOT lost, compositing continues,
    /// nothing is emitted.
    pub fn observe_resize(&mut self) -> RecoveryStep {
        self.resizes += 1;
        RecoveryStep::quiet()
    }

    /// Whether compositing may run (`false` while the surface is
    /// dirty).
    #[must_use]
    pub fn compositing(&self) -> bool {
        !self.dirty
    }

    /// The renderer's present gate: consult once per present
    /// request. Answers `false` (and counts the suppression) while
    /// the surface is dirty; `true` once compositing resumed. The
    /// held frames are the video engine's business (doc 03 §4.10's
    /// latest-wins mailbox keeps at most one pending present, so
    /// suppression loses at most the unsent request — bounded by
    /// construction).
    pub fn gate_present(&mut self) -> bool {
        if self.dirty {
            self.suppressed_presents += 1;
            return false;
        }
        true
    }

    /// Total surface-loss episodes observed.
    #[must_use]
    pub fn losses(&self) -> u64 {
        self.losses
    }

    /// Total successful recreations.
    #[must_use]
    pub fn recreations(&self) -> u64 {
        self.recreations
    }

    /// Present requests suppressed while dirty.
    #[must_use]
    pub fn suppressed_presents(&self) -> u64 {
        self.suppressed_presents
    }

    /// Resizes observed (all non-fatal).
    #[must_use]
    pub fn resizes(&self) -> u64 {
        self.resizes
    }
}
