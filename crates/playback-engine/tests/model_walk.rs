//! The model-walk suite (card 3.C1's model-based AC, the doc 03
//! §4.6(f) mandate): seeded random command sequences driving the REAL
//! [`Engine`] through the shared fake harness (`tests/common` — the
//! same fakes the session suite uses), asserting after every step
//! that the machine never (i) reaches an illegal state, (ii) emits
//! events out of the documented order, or (iii) hangs. Doc 10 §6.3
//! property 5 ("illegal transitions and stuck states are failures")
//! is the same contract; the no-stuck half is the BFS property at
//! the bottom of this file.
//!
//! The driver is a hand-rolled splitmix64-seeded xorshift64* PRNG
//! (the repo convention — `packet-queue`/`limits.rs`, the codecs/demux
//! `fuzz_loop_stable` suites; NO proptest dependency exists in
//! Cargo.lock). A failed seed reruns the exact interleaving: every
//! failure message names the walk seed and step.
//!
//! Per-step assertions (the doc 03 §4.6(f) trio, made precise):
//!
//! - **every delivered `state-changed` edge is wire-legal**
//!   (`PlaybackState::can_transition_to`, doc 05 §3.3) — no illegal
//!   state is reachable through any submitted command mix, including
//!   the close-from-active macro and stop-during-opening;
//! - **every submitted command yields exactly one receipt**
//!   (Accepted/Completed/Rejected-typed — never silence, never a
//!   duplicate; a stale handle's typed `SessionClosed` submit error
//!   is the honest no-receipt case, and the bounded inbox's
//!   `InboxBusy` is retried after a pump — the host shape);
//! - **the doc 05 §4.3 ordering invariants hold on the delivered
//!   stream**: `seq` engine-global strictly increasing (across
//!   closes and reopens), session ids strictly increasing per
//!   reopen (the epoch), generations monotone per session, the fatal
//!   `playback.error` strictly before `state-changed → error`,
//!   `tracks.list-ready` before the first `→ playing` of its
//!   session, and buffering episodes bracketed `0.0`/`1.0` (an
//!   episode open at a session's delivered end is legal only when
//!   the session was superseded — the replacement's suppression
//!   contract, doc 05 §4.2: a superseded generation owes no
//!   terminal).
//!
//! The command mix deliberately includes the completeness items this
//! card owns: close-from-active (the macro), stop-during-opening,
//! double-failure reopen cycles (`Error → Close → Open → … →
//! Error`), the buffered pause (`Buffering + Pause`, the `Paused +
//! Primed` bookend), and underrun flapping (`Playing ↔ Buffering`
//! rapid cycling — legal at table level; 3.B3's debounce owns the
//! anti-flap policy). Vacuity guards (the fuzz-suite convention):
//! every named behavior has a deep-entry floor — the 500 walks must
//! hit each at least once, or the suite fails for testing nothing.
//!
//! The bounded-memory half of the reopen-cycle mandate (the
//! `AllocSnapshot` delta) lives in `tests/reopen_alloc.rs`: the
//! counting allocator needs its own process (testkit's discipline).

mod common;

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use commands::envelope::RequestId;
use media_types::error::ErrorKind;
use media_types::{EngineError, MediaKind, PipelineStage, SourceUri, TrackId};
use playback_engine::state::{SessionState, TRANSITIONS};
use playback_engine::{
    Engine, EngineEvent, EngineEventKind, EngineOutput, FrameConsumer, ReceiptResult, SeekTarget,
    SessionConfig, SessionError, SessionHandle, close_plan,
};
use plugin_api::DemuxEvent;
use testkit::Must;

use common::{
    Collector, FakeDemuxer, FakeProvider, base, frozen_throttle, packet, tiny_budget, tiny_queues,
    track,
};

// ---------------------------------------------------------------------------
// The seeded driver (splitmix64-decorrelated xorshift64*, the repo
// convention — deterministic per seed, no external dependency)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    /// Seed decorrelation: splitmix64 scrambles the walk index so
    /// nearby seeds diverge immediately.
    fn new(seed: u64) -> Rng {
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Rng(z ^ (z >> 31))
    }

    fn next(&mut self) -> u64 {
        // xorshift64* (Marsaglia): the multiply scrambles the low
        // bits plain xorshift leaves weak.
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

// ---------------------------------------------------------------------------
// The delivered-stream invariant checker (doc 05 §4.3)
// ---------------------------------------------------------------------------

/// The per-walk stream checker: ingests every drained output and
/// proves the ordering invariants incrementally.
struct StreamChecker {
    /// Last delivered `seq` (engine-global, strictly increasing).
    last_seq: u64,
    /// The newest session id seen — a larger id means a reopen or
    /// replacement (the epoch is strictly increasing).
    last_session: u64,
    /// Per-session invariants, reset at each session change.
    session: SessionFacts,
    /// Receipts seen, by request id (exactly one per submitted
    /// command — duplicates and silences are both failures).
    receipts: HashMap<u64, usize>,
    /// Floor counters (the vacuity guards).
    floors: Floors,
    /// A small delivered-stream tail, kept for failure diagnostics
    /// (the seed names the walk; the tail names the crime scene).
    tail: std::collections::VecDeque<String>,
}

#[derive(Default)]
struct SessionFacts {
    /// The last generation this session delivered (monotone).
    last_generation: u64,
    /// `tracks.list-ready` arrived before the first `→ playing`.
    tracks_seen: bool,
    /// A buffering episode is currently open (a `0.0` without its
    /// `1.0` yet).
    episode_open: bool,
    /// A fatal is pending its `state-changed → error`.
    fatal_pending: bool,
    /// The last delivered event was an episode's 1.0 (the bookend
    /// pairing window for the Paused + Primed row).
    bookend_just_closed: bool,
}

#[derive(Default, Debug)]
struct Floors {
    wire_legal_edges: usize,
    underrun_flaps: usize,
    buffered_pauses: usize,
    paused_primed_bookends: usize,
    stop_during_opening: usize,
    close_from_active: usize,
    reopens: usize,
    error_cycles: usize,
    paused_self_confirmations: usize,
}

impl StreamChecker {
    fn new() -> StreamChecker {
        StreamChecker {
            last_seq: 0,
            last_session: 0,
            session: SessionFacts::default(),
            receipts: HashMap::new(),
            floors: Floors::default(),
            tail: std::collections::VecDeque::new(),
        }
    }

    /// The delivered tail (for failure messages).
    fn tail_text(&self) -> String {
        self.tail.iter().map(|line| format!("\n  {line}")).collect()
    }

    /// Ingest one drained batch, checking every invariant as we go.
    fn ingest(&mut self, outputs: Vec<EngineOutput>, walk: u64, step: usize) {
        for output in outputs {
            match output {
                EngineOutput::Receipt(receipt) => {
                    self.tail_push(format!(
                        "receipt {} {:?}",
                        receipt.request.0, receipt.result
                    ));
                    let count = self.receipts.entry(receipt.request.0).or_insert(0);
                    *count += 1;
                    assert_eq!(
                        *count, 1,
                        "walk {walk} step {step}: duplicate receipt for {}",
                        receipt.request.0
                    );
                    // Every receipt answers a command with one of the
                    // three honest shapes — never a fourth.
                    assert!(
                        matches!(
                            receipt.result,
                            ReceiptResult::Accepted { .. }
                                | ReceiptResult::Completed { .. }
                                | ReceiptResult::Rejected { .. }
                        ),
                        "walk {walk} step {step}: receipt {} carries no result shape",
                        receipt.request.0
                    );
                }
                EngineOutput::Event(event) => self.check_event(event, walk, step),
            }
        }
    }

    /// Keep the last 24 delivered lines (bounded — the diagnostic
    /// tail cannot itself become a leak).
    fn tail_push(&mut self, line: String) {
        if self.tail.len() == 24 {
            self.tail.pop_front();
        }
        self.tail.push_back(line);
    }

    fn check_event(&mut self, event: EngineEvent, walk: u64, step: usize) {
        self.tail_push(format!(
            "s{} seq{} g{} {}",
            event.session.0,
            event.seq,
            event.generation.0,
            describe(&event.kind)
        ));
        // (iv) seq engine-global strictly increasing.
        assert!(
            event.seq > self.last_seq,
            "walk {walk} step {step}: seq {} not past {}",
            event.seq,
            self.last_seq
        );
        self.last_seq = event.seq;
        // (iv) session ids strictly increase (the epoch per reopen).
        let session = event.session.0;
        if session != self.last_session {
            assert!(
                session > self.last_session,
                "walk {walk} step {step}: session id went backwards ({session} after {})",
                self.last_session
            );
            // The previous session's episode may be left open only
            // via replacement suppression (doc 05 §4.2: a superseded
            // generation owes no terminal) — a fresh session's facts
            // start clean.
            self.session = SessionFacts::default();
            self.last_session = session;
        }
        // (v) generations monotone per session.
        assert!(
            event.generation.0 >= self.session.last_generation,
            "walk {walk} step {step}: generation went backwards in session {session}"
        );
        self.session.last_generation = event.generation.0;
        match event.kind {
            EngineEventKind::StateChanged { from, to } => {
                // (i) every delivered edge is wire-legal.
                assert!(
                    from.as_wire().can_transition_to(to.as_wire()),
                    "walk {walk} step {step}: illegal wire edge {from} → {to}"
                );
                self.floors.wire_legal_edges += 1;
                if to == SessionState::Playing && from != SessionState::Playing {
                    // (iii) tracks before the first → playing.
                    assert!(
                        self.session.tracks_seen,
                        "walk {walk} step {step}: → playing before tracks (session {session})"
                    );
                }
                if to == SessionState::Error {
                    // (iii) the fatal strictly before → error.
                    assert!(
                        self.session.fatal_pending,
                        "walk {walk} step {step}: → error without its fatal first"
                    );
                    self.session.fatal_pending = false;
                    self.floors.error_cycles += 1;
                }
                if from == SessionState::Playing && to == SessionState::Buffering {
                    self.floors.underrun_flaps += 1;
                }
                if from == SessionState::Buffering && to == SessionState::Paused {
                    self.floors.buffered_pauses += 1;
                }
                if from == SessionState::Paused && to == SessionState::Paused {
                    // A paused self-confirmation: the C8 idempotence
                    // or the 3.C1 bookend row (the 1.0 pairing is
                    // tracked separately — only the seek_flow fake
                    // can hold a pre-roll open under pause, so the
                    // walk's counter conflates the two; the
                    // deterministic row-4 pin lives in seek_flow's
                    // `pause_during_buffering_holds_and_primes_
                    // silently`).
                    self.floors.paused_self_confirmations += 1;
                }
                if from == SessionState::Paused
                    && to == SessionState::Paused
                    && self.session.bookend_just_closed
                {
                    self.floors.paused_primed_bookends += 1;
                }
                self.session.bookend_just_closed = false;
                if from == SessionState::Opening && to == SessionState::Stopped {
                    self.floors.stop_during_opening += 1;
                }
                if to == SessionState::Idle {
                    // An episode cannot survive the session's close —
                    // the executor's bookend closes it first (the 1.0
                    // precedes the → idle).
                    assert!(
                        !self.session.episode_open,
                        "walk {walk} step {step}: episode left open at → idle{}",
                        self.tail_text()
                    );
                }
            }
            EngineEventKind::TracksChanged { .. } => {
                self.session.tracks_seen = true;
            }
            EngineEventKind::Buffering { progress } => {
                // (iii) episodes bracket 0.0/1.0.
                if progress <= 0.0 {
                    assert!(
                        !self.session.episode_open,
                        "walk {walk} step {step}: episode 0.0 inside an open episode"
                    );
                    self.session.episode_open = true;
                } else if progress >= 1.0 {
                    assert!(
                        self.session.episode_open,
                        "walk {walk} step {step}: episode 1.0 without an open episode"
                    );
                    self.session.episode_open = false;
                    // The 3.C1 bookend signature: the 1.0 that rides
                    // the Paused + Primed row is followed by the
                    // work-carrying Paused → Paused confirmation.
                    self.session.bookend_just_closed = true;
                } else {
                    assert!(
                        self.session.episode_open,
                        "walk {walk} step {step}: mid-episode progress outside an episode"
                    );
                }
            }
            EngineEventKind::Fatal { .. } => {
                self.session.fatal_pending = true;
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// The walker: one real Engine, a weighted command mix, bounded pumps
// ---------------------------------------------------------------------------

/// One delivered line for the diagnostic tail.
fn describe(kind: &EngineEventKind) -> String {
    match kind {
        EngineEventKind::StateChanged { from, to } => format!("{from} → {to}"),
        EngineEventKind::TracksChanged { .. } => "tracks".to_string(),
        EngineEventKind::Buffering { progress } => format!("buffering {progress}"),
        EngineEventKind::EndOfMedia => "end-of-media".to_string(),
        EngineEventKind::Fatal { error } => format!("fatal {}", error.context.code),
        EngineEventKind::Seeked { .. } => "seeked".to_string(),
        EngineEventKind::SeekFailed { .. } => "seek-failed".to_string(),
        other => format!("{:?}", std::mem::discriminant(other)),
    }
}

/// How many packets a fresh walk session's script carries before the
/// stall flag governs (enough to prime — the high watermark is 12 —
/// and short enough that underruns are reachable).
const SCRIPT_PACKETS: u64 = 40;
/// The refill action's packet batch (one underrun→recovery flap).
const REFILL_PACKETS: u64 = 24;

enum SessionCommand {
    Play,
    Pause,
    Stop,
    Close,
    Seek,
}

struct Walker {
    engine: Engine,
    handle: Option<SessionHandle>,
    collector: Arc<Collector>,
    script: Arc<Mutex<VecDeque<DemuxEvent>>>,
    stall: Arc<Mutex<bool>>,
    script_index: u64,
    request: u64,
    /// The request ids that actually entered a live inbox (each owes
    /// exactly one receipt by the walk's end).
    entered: Vec<u64>,
    checker: StreamChecker,
    walk: u64,
    step: usize,
}

impl Walker {
    /// Open a session on the walker's engine (a fresh open when
    /// Idle, the replacement composition otherwise), keeping the
    /// shared script/stall handles the mix toggles.
    fn open_session(&mut self, fail_start: bool) {
        let tracks = vec![track(1, MediaKind::Audio)];
        let script: Vec<DemuxEvent> = (self.script_index..self.script_index + SCRIPT_PACKETS)
            .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
            .collect();
        self.script_index += SCRIPT_PACKETS;
        let script = Arc::new(Mutex::new(VecDeque::from(script)));
        let stall = Arc::new(Mutex::new(true));
        let collector = Arc::new(Collector::default());
        let clock = Arc::new(
            media_clock::MediaClock::system_monotonic(
                Arc::new(common::SharedNow(AtomicU64::new(0))),
                media_types::Timestamp::new(0, base()),
                media_types::Rate::ONE,
            )
            .must(),
        );
        let failure = fail_start.then(|| {
            EngineError::new(
                PipelineStage::Open,
                ErrorKind::Inaccessible {
                    reason: media_types::error::AccessDenied::NotFound,
                },
                media_types::error::ErrorContext::new("input.file.missing"),
            )
        });
        let mut config = SessionConfig::new(
            SourceUri::parse("file:///tmp/lumen-model-walk.media").must(),
            Box::new(FakeDemuxer {
                tracks,
                script: Arc::clone(&script),
                stall: Arc::clone(&stall),
                fail_start: failure,
            }),
            Arc::new(FakeProvider { fail_select: None }),
            Arc::clone(&collector) as Arc<dyn FrameConsumer>,
            clock,
        );
        config.queues = tiny_queues();
        config.budget = tiny_budget();
        config.throttle = frozen_throttle();
        let was_idle = self.engine.state() == SessionState::Idle;
        let handle = self
            .engine
            .open(config)
            .unwrap_or_else(|error| unreachable!("walk {}: open failed: {error}", self.walk));
        if was_idle {
            self.checker.floors.reopens += 1;
        }
        self.handle = Some(handle);
        self.script = script;
        self.stall = stall;
        self.collector = collector;
    }

    fn state(&self) -> SessionState {
        self.engine.state()
    }

    /// Submit a command (bounded-inbox retry: `InboxBusy` pumps once
    /// and resubmits — the honest host shape). Returns whether the
    /// command entered (a stale handle's typed `SessionClosed` is the
    /// no-receipt case).
    fn submit(&mut self, kind: &SessionCommand) -> bool {
        let Some(handle) = self.handle.clone() else {
            return false;
        };
        self.request += 1;
        let request = RequestId(self.request);
        loop {
            let outcome = match kind {
                SessionCommand::Play => handle.play(request),
                SessionCommand::Pause => handle.pause(request),
                SessionCommand::Stop => handle.stop(request),
                SessionCommand::Close => handle.close(request),
                SessionCommand::Seek => handle.seek(
                    request,
                    SeekTarget::Absolute {
                        position_ns: 1_000_000,
                    },
                ),
            };
            match outcome {
                Ok(()) => {
                    self.entered.push(request.0);
                    return true;
                }
                Err(SessionError::InboxBusy) => {
                    let outputs = self.engine.take_outputs();
                    self.checker.ingest(outputs, self.walk, self.step);
                    self.engine.poll();
                }
                Err(SessionError::SessionClosed) => return false,
                Err(other) => {
                    unreachable!("walk {}: submit failed: {other}", self.walk)
                }
            }
        }
    }

    /// Pump a bounded batch and ingest everything delivered.
    fn pump(&mut self, count: usize) {
        for _ in 0..count {
            self.engine.poll();
        }
        let outputs = self.engine.take_outputs();
        self.checker.ingest(outputs, self.walk, self.step);
    }

    /// Pump until `wanted` holds or the bound trips (the loud
    /// hang-proof bound — a hang is a failed test).
    fn pump_until(&mut self, wanted: SessionState, bound: usize, context: &str) {
        for _ in 0..bound {
            if self.state() == wanted {
                return;
            }
            self.engine.poll();
        }
        let outputs = self.engine.take_outputs();
        self.checker.ingest(outputs, self.walk, self.step);
        assert_eq!(
            self.state(),
            wanted,
            "walk {}: {context} not reached in {bound} pumps",
            self.walk
        );
    }

    /// The walk's final discipline: a bounded tail drain, then every
    /// command that entered must have answered exactly once.
    fn finish(&mut self) {
        self.pump(4);
        for request in &self.entered {
            assert_eq!(
                self.checker.receipts.get(request),
                Some(&1),
                "walk {}: request {request} answered {:?} times (exactly one required)",
                self.walk,
                self.checker.receipts.get(request)
            );
        }
    }
}

/// One seeded walk: a fresh Engine, `steps` weighted actions, every
/// output checked. Returns the walk's floor contributions.
fn run_walk(seed: u64, steps: usize) -> Floors {
    let mut rng = Rng::new(seed);
    let mut walker = Walker {
        engine: Engine::new(),
        handle: None,
        collector: Arc::new(Collector::default()),
        script: Arc::new(Mutex::new(VecDeque::new())),
        stall: Arc::new(Mutex::new(false)),
        script_index: 0,
        request: 0,
        entered: Vec::new(),
        checker: StreamChecker::new(),
        walk: seed,
        step: 0,
    };
    // Every walk starts with a live session (some open clean, some
    // open straight into the failure arm — the double-failure mix).
    let fail_first = rng.below(4) == 0;
    walker.open_session(fail_first);
    for step in 0..steps {
        walker.step = step;
        match rng.below(16) {
            0..=2 => {
                let pumps = 1 + rng.below(4) as usize;
                walker.pump(pumps);
            }
            3 => {
                walker.submit(&SessionCommand::Play);
                walker.pump(1 + rng.below(3) as usize);
            }
            4 => {
                walker.submit(&SessionCommand::Pause);
                walker.pump(1 + rng.below(3) as usize);
            }
            5 => {
                walker.submit(&SessionCommand::Stop);
                walker.pump(1 + rng.below(3) as usize);
            }
            6 => {
                // Close from wherever we are — from an active state
                // this is the macro composition.
                let active = matches!(
                    walker.state(),
                    SessionState::Playing | SessionState::Paused | SessionState::Buffering
                );
                let entered = walker.submit(&SessionCommand::Close);
                if entered && active {
                    walker.checker.floors.close_from_active += 1;
                }
                walker.pump(1 + rng.below(3) as usize);
            }
            7 => {
                walker.submit(&SessionCommand::Seek);
                walker.pump(2 + rng.below(4) as usize);
            }
            8 => {
                // The underrun flap's stall half.
                *walker
                    .stall
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
                walker.pump(2 + rng.below(4) as usize);
            }
            9 => {
                // The refill half: extend the script, release the
                // stall — one recovery flap when the queue was low.
                let mut script = walker
                    .script
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                for index in walker.script_index..walker.script_index + REFILL_PACKETS {
                    script.push_back(DemuxEvent::Packet(packet(TrackId::new(1), index)));
                }
                walker.script_index += REFILL_PACKETS;
                drop(script);
                *walker
                    .stall
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = false;
                walker.pump(2 + rng.below(4) as usize);
            }
            10 | 11 => {
                // Reopen (Idle) or replace (live): the source
                // replacement composition, mostly clean opens, every
                // fourth one armed to fail (the Error → Close → Open
                // → … → Error double-failure cycles).
                let fail = rng.below(4) == 0;
                walker.open_session(fail);
                walker.pump(1 + rng.below(3) as usize);
            }
            12 => {
                // Stop during opening: the row fires only when the
                // stop is applied while the open step is still
                // pending — submit it before the next pump.
                if walker.state() == SessionState::Opening {
                    walker.submit(&SessionCommand::Stop);
                    walker.pump(2);
                } else {
                    walker.pump(1);
                }
            }
            13 => {
                // The buffered pause: settle to Buffering, pause (the
                // episode stays open), then let the queues fill to
                // the watermark while held — the `Paused + Primed`
                // bookend row.
                if walker.state() == SessionState::Buffering {
                    walker.submit(&SessionCommand::Pause);
                    walker.pump(6);
                } else {
                    walker.pump(1);
                }
            }
            14 => {
                // Close from an active state, guaranteed: settle to
                // an active state first, then close — the macro's
                // dedicated deep-entry path.
                match walker.state() {
                    SessionState::Playing | SessionState::Paused | SessionState::Buffering => {
                        let entered = walker.submit(&SessionCommand::Close);
                        if entered {
                            walker.checker.floors.close_from_active += 1;
                        }
                        walker.pump(3);
                    }
                    _ => walker.pump(1),
                }
            }
            _ => {
                walker.pump(1 + rng.below(2) as usize);
            }
        }
    }
    walker.finish();
    std::mem::take(&mut walker.checker.floors)
}

// ---------------------------------------------------------------------------
// The walks (doc 03 §4.6(f): random command sequences, N=500 × ≤64)
// ---------------------------------------------------------------------------

#[test]
fn seeded_model_walks_never_break_the_machine() {
    let mut total = Floors::default();
    for seed in 0..500u64 {
        let steps = 8 + (seed % 57) as usize; // 8..=64, seed-derived
        let floors = run_walk(seed, steps);
        total.wire_legal_edges += floors.wire_legal_edges;
        total.underrun_flaps += floors.underrun_flaps;
        total.buffered_pauses += floors.buffered_pauses;
        total.paused_primed_bookends += floors.paused_primed_bookends;
        total.stop_during_opening += floors.stop_during_opening;
        total.close_from_active += floors.close_from_active;
        total.reopens += floors.reopens;
        total.error_cycles += floors.error_cycles;
    }
    // The vacuity guards (deep-entry floors): 500 walks must exercise
    // every named behavior or the suite is testing nothing.
    assert!(total.wire_legal_edges > 4_000, "edges checked: {total:?}");
    assert!(total.underrun_flaps >= 1, "underrun flaps: {total:?}");
    assert!(total.buffered_pauses >= 1, "buffered pauses: {total:?}");
    assert!(
        total.paused_primed_bookends >= 1,
        "paused+primed bookends: {total:?}"
    );
    assert!(total.stop_during_opening >= 1, "stop in opening: {total:?}");
    assert!(total.close_from_active >= 1, "close macro: {total:?}");
    assert!(total.reopens >= 1, "reopens: {total:?}");
    assert!(total.error_cycles >= 2, "double-failure cycles: {total:?}");
}

/// The receipt discipline, pinned apart from the walk loop for a
/// precise count: one deterministic sequence where every submitted
/// command must answer exactly once (never silence, never a
/// duplicate) — including the close-from-active macro's single
/// Completed for its two-row composition.
#[test]
fn every_submitted_command_answers_exactly_one_receipt() {
    let mut walker = Walker {
        engine: Engine::new(),
        handle: None,
        collector: Arc::new(Collector::default()),
        script: Arc::new(Mutex::new(VecDeque::new())),
        stall: Arc::new(Mutex::new(false)),
        script_index: 0,
        request: 0,
        entered: Vec::new(),
        checker: StreamChecker::new(),
        walk: u64::MAX,
        step: 0,
    };
    walker.open_session(false);
    walker.pump_until(SessionState::Playing, 24, "priming");
    // A burst of commands across states, draining between each.
    for (kind, pumps) in [
        (&SessionCommand::Pause, 2usize),
        (&SessionCommand::Play, 2),
        (&SessionCommand::Seek, 4),
        (&SessionCommand::Pause, 2),
        (&SessionCommand::Stop, 2),
        (&SessionCommand::Play, 1), // AlreadySettled: typed rejection
        (&SessionCommand::Close, 2),
    ] {
        let entered = walker.submit(kind);
        assert!(entered, "the handle is live");
        walker.pump(pumps);
    }
    assert_eq!(walker.state(), SessionState::Idle, "the close landed");
    walker.finish();
    assert_eq!(walker.request, 7, "seven commands submitted");
}

// ---------------------------------------------------------------------------
// The no-stuck property (doc 10 §6.3 property 5): BFS over the rows
// plus the close macro — Idle reachable from all 8 states
// ---------------------------------------------------------------------------

/// The successor set of `state`: every row's landing state, plus the
/// close macro's destination (the plan walked step by step — every
/// step is itself a row, pinned by `table.rs`'s close-plan test).
fn successors(state: SessionState) -> Vec<SessionState> {
    let mut out: Vec<SessionState> = TRANSITIONS
        .iter()
        .filter(|row| row.from == state)
        .map(|row| row.to)
        .collect();
    // The macro: walk the plan from `state` to its destination.
    let mut cursor = state;
    for input in close_plan(state) {
        let row = playback_engine::state::apply_input(cursor, *input)
            .unwrap_or_else(|rejection| unreachable!("close_plan step illegal: {rejection}"));
        cursor = row.to;
    }
    out.push(cursor);
    out
}

#[test]
fn idle_is_reachable_from_every_state_over_rows_and_the_close_macro() {
    for start in SessionState::ALL {
        // BFS from `start` over rows + the close macro.
        let mut seen: HashSet<SessionState> = HashSet::new();
        let mut frontier: Vec<SessionState> = vec![start];
        seen.insert(start);
        while let Some(state) = frontier.pop() {
            for next in successors(state) {
                if seen.insert(next) {
                    frontier.push(next);
                }
            }
        }
        assert!(
            seen.contains(&SessionState::Idle),
            "{start} cannot reach Idle — a stuck state"
        );
        // And the graph is strongly connected through Idle: every
        // state is reachable from every start (no isolated pockets).
        assert_eq!(
            seen.len(),
            SessionState::ALL.len(),
            "{start} sees only {seen:?} of the 8 states"
        );
    }
}

// ---------------------------------------------------------------------------
// The reopen cycles (doc 03 §4.6(f) fast form): 1000 × open → play →
// stop → close, epoch/seq/receipt continuity throughout. The
// bounded-memory half (the AllocSnapshot delta) is
// `tests/reopen_alloc.rs`.
// ---------------------------------------------------------------------------

#[test]
fn reopen_cycles_1000x_keep_epoch_and_seq_continuous() {
    let mut walker = Walker {
        engine: Engine::new(),
        handle: None,
        collector: Arc::new(Collector::default()),
        script: Arc::new(Mutex::new(VecDeque::new())),
        stall: Arc::new(Mutex::new(false)),
        script_index: 0,
        request: 0,
        entered: Vec::new(),
        checker: StreamChecker::new(),
        walk: u64::MAX - 1,
        step: 0,
    };
    let mut last_session = 0u64;
    for cycle in 0..1000u64 {
        walker.open_session(false);
        // open_session holds the stall; release it so the cycle
        // primes and plays (the script's 40 packets suffice).
        *walker
            .stall
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = false;
        walker.pump_until(SessionState::Playing, 24, "cycle priming");
        let session = walker
            .engine
            .session_id()
            .unwrap_or_else(|| unreachable!("cycle {cycle}: no session while playing"))
            .0;
        assert_eq!(
            session,
            cycle + 1,
            "cycle {cycle}: the epoch is one per reopen"
        );
        assert!(session > last_session);
        last_session = session;
        // stop → close (the settled exit), then the next cycle opens.
        assert!(walker.submit(&SessionCommand::Stop));
        walker.pump(2);
        assert!(walker.submit(&SessionCommand::Close));
        walker.pump(2);
        assert_eq!(
            walker.state(),
            SessionState::Idle,
            "cycle {cycle}: the close landed"
        );
        // The collector is drained per cycle (the frames are the
        // walk's own evidence, not a leak — the alloc binary pins
        // the memory flatness).
        let frames = walker
            .collector
            .frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len();
        assert!(frames > 0, "cycle {cycle}: the media actually played");
        walker
            .collector
            .frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }
    walker.finish();
    assert_eq!(walker.request, 2000, "two commands per cycle");
}
