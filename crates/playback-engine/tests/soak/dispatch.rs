//! The soak's command layer (card 3.E4, ARCH-5): the single dispatch
//! entry every soak loop op goes through.
//!
//! # Why an in-test dispatch, and not `apps/cli`'s `lumen_dispatch`
//!
//! ARCH-5 says every host surface builds a [`WireRequest`] and calls
//! the one `lumen_dispatch`. The CLI's `lumen_dispatch` (apps/cli/src/
//! dispatch.rs) lives in the `lumen-cli` app crate, which **depends on
//! `playback-engine`** — a playback-engine test cannot link it (that
//! would be a dependency cycle, and the crate's manifest is Lead-owned
//! scope this card does not carry). The soak therefore drives the same
//! architecture one layer down at the exact seam `lumen_dispatch`
//! itself routes onto through `CliHost`:
//!
//! - the **same envelope types** — [`WireRequest`]/[`WireResponse`],
//!   [`Command`] (the catalog), the request-id allocator;
//! - the **same served playing subset** as the CLI's `SERVED` table
//!   (`session.open`, `playback.play`, `playback.pause`,
//!   `playback.stop`, `session.close`) **plus**
//!   `tracks.set-audio-track` (the CLI row still answers "track
//!   switching is card 3.A3", which has landed engine-side; the soak
//!   needs the switch op);
//! - the **same honest typed refusals** for everything else;
//! - routed onto [`SessionHandle`] — the identical submit path
//!   `CliHost::submit_and_await` drives — with receipt correlation
//!   and class-A terminal awaits (the `dispatch_seek`/
//!   `submit_and_await`/`rejection_wire` shapes, mirrored).
//!
//! Divergences from the CLI host are recorded at [`SoakHost::
//! open_source`] (no `media-probe` stage — it is not in this crate's
//! dev-dependency set; the open path is the `tests/
//! track_switch_fixture.rs` precedent) and in the report.
//!
//! # Determinism split (the card's requirement)
//!
//! The **scenario** (which op runs when, against which fixture, with
//! which recorded seek target) is deterministic by construction: the
//! media clock and the event-throttle source are a frozen virtual
//! source (doc 03 §8 — the same virtual clock every engine suite
//! injects), and the op script is a pure function of the cycle index.
//! The **memory measurement** is REAL wall clock: the host's
//! [`Sampling`] ticks on a fixed wall-clock cadence from inside
//! [`SoakHost::pump`] — every engine poll is a sampling point, so the
//! cadence holds regardless of where in the loop the wall finds us
//! (virtual time allocates nothing and proves nothing about memory).

use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use codec_api::{Decoder, DecoderConfig};
use commands::catalog::types::SeekTarget as WireSeekTarget;
use commands::catalog::types::SourceSpec;
use commands::catalog::types::TrackSelector as WireTrackSelector;
use commands::correlation::RequestIdAllocator;
use commands::envelope::{CommandOutcome, Generation, RequestId};
use commands::{Command, CommandError, CommandResult, ErrorCode, Stage, WireRequest, WireResponse};
use demux::avi::AviDemuxer;
use demux::avi::header::StreamFormat;
use media_clock::MonotonicSource;
use media_types::frame::FrameStorage;
use media_types::{EngineError, Frame, MediaKind, Nanoseconds, SourceUri, Track, TrackId};
use packet_queue::QueueLimits;
use playback_engine::state::{RejectionReason, SessionState};
use playback_engine::{
    CommandReceipt, DecoderProvider, Engine, EngineEvent, EngineEventKind, EngineOutput,
    FrameConsumer, QueuePlan, ReceiptResult, SeekTarget, SessionCommandKind, SessionConfig,
    SessionError, SessionHandle, TickBudget, TrackSelector, wire_error_code, wire_stage,
};
use plugin_api::{Access, Demuxer};
use testkit::Must;

use super::memory::CadenceSampler;

/// Poll bound for awaiting one receipt / terminal (a hang is a failed
/// test, never a timeout mystery — doc 03 §4.6(f); generous for the
/// real-stack decode path).
const AWAIT_POLLS: usize = 2048;

// ---------------------------------------------------------------------------
// The frozen virtual clock (doc 03 §8) — scenario determinism only
// ---------------------------------------------------------------------------

/// A never-advancing monotonic source: the scenario's script is fully
/// deterministic (decode is poll-driven; the clock is not the driver
/// in the v0 single-writer model — the `track_switch_fixture.rs` and
/// `seek_flow.rs` precedents).
pub struct FrozenNow;

impl MonotonicSource for FrozenNow {
    fn now_ns(&self) -> u64 {
        0
    }
}

// ---------------------------------------------------------------------------
// The wall-clock sampling hook (the memory instrument's cadence)
// ---------------------------------------------------------------------------

/// The host's wall-clock sampling state: the fixed-cadence sampler
/// plus the window bookkeeping (warmup flag, completed-cycle stamp)
/// and the live-bytes reader (a fn pointer into the process-global
/// counting allocator — zero-capture, so sampling allocates nothing
/// beyond the reserved sample slot).
pub struct Sampling {
    /// The fixed-cadence wall-clock sampler (series preallocated).
    pub sampler: CadenceSampler,
    /// True while inside the warmup window (samples are marked and
    /// excluded from the slope fit; kept in the series).
    pub warmup: bool,
    /// Completed cycles at sample time.
    pub cycles: u64,
    live: fn() -> isize,
}

impl Sampling {
    /// Sample now if the fixed cadence ticked.
    fn sample_if_due(&mut self) {
        if self.sampler.ticked() {
            self.sampler.sample((self.live)(), self.cycles, self.warmup);
        }
    }
}

// ---------------------------------------------------------------------------
// The sink: count and drop (the soak must not bank frames)
// ---------------------------------------------------------------------------

/// The soak's frame consumer: counts frames/bytes per track and DROPS
/// them. A collecting sink would itself be an unbounded Vec — the
/// soak's own harness would be the top leak suspect by construction.
/// Fixed-size counters only.
#[derive(Default)]
pub struct CountingSink {
    /// Frames per track handle (fixtures carry ≤ 4 tracks; handle ≥ 8
    /// folds into the last slot — counted, never dropped silently).
    frames: [AtomicU64; 8],
    /// Decoded payload bytes across all tracks.
    bytes: AtomicU64,
}

impl CountingSink {
    /// Total frames across tracks.
    #[must_use = "observable evidence"]
    pub fn frames_total(&self) -> u64 {
        self.frames.iter().map(|c| c.load(Ordering::Relaxed)).sum()
    }

    /// Total decoded payload bytes.
    #[must_use = "observable evidence"]
    pub fn bytes_total(&self) -> u64 {
        self.bytes.load(Ordering::Relaxed)
    }
}

impl FrameConsumer for CountingSink {
    fn accept(&self, frame: Frame, track: TrackId) -> Result<(), EngineError> {
        let index = (track.handle() as usize).min(7);
        self.frames[index].fetch_add(1, Ordering::Relaxed);
        let bytes = match &frame {
            Frame::Audio(audio) => audio.samples.len(),
            Frame::Video(video) => match &video.data {
                FrameStorage::Cpu(bytes) => bytes.len(),
                FrameStorage::OpaqueHandle(_) => 0,
            },
            Frame::Subtitle(_) => 0,
        };
        self.bytes.fetch_add(bytes as u64, Ordering::Relaxed);
        // The frame is dropped at the end of scope: the sink holds
        // nothing across calls.
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The decoder provider (the track_switch_fixture.rs shape, test-side)
// ---------------------------------------------------------------------------

/// The real decoder seam: MJPEG as-is, PCM seeded with the `strf`
/// facts harvested from the demuxer before it moved into the engine
/// (the `PcmDecoder::with_format` seam — the engine's wire passes no
/// WAVEFORMATEX `extra_data`). Decoder builds are counted by the
/// ENGINE's own lifetime stats (`EngineStats::decoders_built`/
/// `decoders_reused`) — the authoritative counters this soak reports.
pub struct SoakProvider {
    /// PCM `(channels, rate)` per track handle.
    pcm_facts: Vec<(u32, (u32, u32))>,
}

impl SoakProvider {
    /// A provider over the harvested PCM facts of one open.
    #[must_use = "provider"]
    pub fn new(pcm_facts: Vec<(u32, (u32, u32))>) -> SoakProvider {
        SoakProvider { pcm_facts }
    }
}

impl DecoderProvider for SoakProvider {
    fn decoder_for(
        &self,
        track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        match track.kind {
            MediaKind::Video => {
                let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
                decoder.configure(config)?;
                Ok(Box::new(decoder))
            }
            MediaKind::Audio => {
                let (channels, rate) = self
                    .pcm_facts
                    .iter()
                    .find(|(handle, _)| *handle == track.id.handle())
                    .map(|(_, facts)| *facts)
                    .unwrap_or((2, 48_000));
                let mut decoder = codecs::audio::pcm::PcmDecoder::with_format(channels, rate)?;
                decoder.configure(config)?;
                Ok(Box::new(decoder))
            }
            _ => Err(EngineError::new(
                media_types::PipelineStage::Decode,
                media_types::error::ErrorKind::Unsupported { format: None },
                media_types::error::ErrorContext::new("soak.decoder-kind-unsupported"),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// The queue plan / tick budget
// ---------------------------------------------------------------------------

/// The queue caps (the `track_switch_fixture.rs` shape): small bounded
/// queues — the soak's bounded-memory claim is only honest if the
/// queues themselves stay small — with caps that still prime early on
/// 2-second fixtures.
#[must_use = "queue plan"]
pub fn soak_queues() -> QueuePlan {
    let audio = QueueLimits {
        max_packets: NonZeroUsize::new(64).must(),
        max_bytes: NonZeroUsize::new(1 << 20).must(),
        max_duration: Nanoseconds(400_000_000),
    };
    let video = QueueLimits {
        max_packets: NonZeroUsize::new(64).must(),
        max_bytes: NonZeroUsize::new(8 << 20).must(),
        max_duration: Nanoseconds(400_000_000),
    };
    QueuePlan::new(audio, video)
}

/// The per-tick work bound: bounded, but wide enough that a cycle is a
/// few dozen polls, not a few thousand (soak iterations per minute is
/// the loop-count axis of the report).
#[must_use = "tick budget"]
pub fn soak_budget() -> TickBudget {
    TickBudget {
        demux_events: 256,
        packets_per_limb: 8,
        flush_calls_per_limb: 16,
    }
}

// ---------------------------------------------------------------------------
// The host + the one dispatch entry
// ---------------------------------------------------------------------------

/// The soak host (the `CliHost` shape, test-side): one [`Engine`] for
/// the whole soak (reopen = `close → open` on the same engine — the
/// harder, accretion-visible invariant), the request-id allocator,
/// the live session's handle + harvested facts, and the wall-clock
/// sampling hook.
pub struct SoakHost {
    engine: Engine,
    allocator: RequestIdAllocator,
    handle: Option<SessionHandle>,
    /// The open session's counting sink (frames/bytes the soak
    /// actually decoded — the loop-count axis's evidence). Replaced at
    /// every open; the closed session's sink dies with the next open.
    sink: Option<Arc<CountingSink>>,
    /// The wall-clock sampling hook (installed by the soak driver).
    sampling: Option<Sampling>,
    /// Whether the last completed seek landed (`Some(true)` =
    /// `Seeked`, `Some(false)` = `SeekFailed` — contract D4; `None`
    /// before the first seek of a session). Recorded by the ONE
    /// drain every output passes through — a terminal that lands in
    /// the same poll as its receipt is still seen (the harness bug
    /// this field's setter fixed: the awaited terminal was dropped
    /// unseen in the receipt poll and the await hung forever).
    pub last_seek_completed: Option<bool>,
    /// Whether the in-flight track switch's `tracks.changed`
    /// terminal landed (reset per switch submit; recorded by the
    /// same drain).
    pub switch_landed: bool,
    /// The last fatal's display sentence (triage: the Error lane's
    /// cause, surfaced instead of hidden behind a count).
    pub last_fatal: Option<String>,
    /// Events observed across the whole soak (receipts and events are
    /// drained every poll; only counts survive).
    pub events_seen: u64,
    /// Receipts observed across the whole soak.
    pub receipts_seen: u64,
    /// Track-selection terminals observed across the whole soak (the
    /// switch op's completion evidence).
    pub switch_terminals: u64,
    /// Polls driven across the whole soak.
    pub polls: u64,
}

impl SoakHost {
    /// A host over a fresh engine, no session open, no sampler.
    #[must_use = "host"]
    pub fn new() -> SoakHost {
        SoakHost {
            engine: Engine::new(),
            allocator: RequestIdAllocator::new(),
            handle: None,
            sink: None,
            sampling: None,
            last_seek_completed: None,
            switch_landed: false,
            last_fatal: None,
            events_seen: 0,
            receipts_seen: 0,
            switch_terminals: 0,
            polls: 0,
        }
    }

    /// Install the wall-clock sampler (the soak driver's instrument).
    /// `live` reads the process-global counting allocator's live
    /// bytes — a plain fn pointer, so sampling allocates nothing
    /// beyond the preallocated sample slot.
    pub fn install_sampling(&mut self, sampler: CadenceSampler, live: fn() -> isize) {
        self.sampling = Some(Sampling {
            sampler,
            warmup: true,
            cycles: 0,
            live,
        });
    }

    /// End the warmup window: samples from here on join the measured
    /// window (the slope-fit population).
    pub fn end_warmup(&mut self) {
        if let Some(sampling) = self.sampling.as_mut() {
            sampling.warmup = false;
        }
    }

    /// Note one completed cycle (the sample stamp).
    pub fn note_cycle_complete(&mut self) {
        if let Some(sampling) = self.sampling.as_mut() {
            sampling.cycles = sampling.cycles.saturating_add(1);
        }
    }

    /// Force one sample NOW (off-cadence; the driver's endpoint
    /// sample at the deadline — timestamped honestly, it simply does
    /// not wait for the next tick).
    pub fn force_sample(&mut self) {
        if let Some(sampling) = self.sampling.as_mut() {
            let live = (sampling.live)();
            let cycles = sampling.cycles;
            let warmup = sampling.warmup;
            sampling.sampler.sample(live, cycles, warmup);
        }
    }

    /// The sampler's wall-clock elapsed seconds (None if no sampler
    /// installed).
    #[must_use = "clock"]
    pub fn sampling_elapsed_s(&self) -> Option<f64> {
        self.sampling
            .as_ref()
            .map(|sampling| sampling.sampler.elapsed_s())
    }

    /// Take the sampling state back (the report's series). The host
    /// keeps running unsampled afterwards.
    #[must_use = "observable evidence"]
    pub fn take_sampling(&mut self) -> Option<Sampling> {
        self.sampling.take()
    }

    /// The engine's current session state.
    #[must_use = "state"]
    pub fn state(&self) -> SessionState {
        self.engine.state()
    }

    /// The engine's lifetime stats (frames delivered, decoder builds
    /// and cache hits — the triage's engine-side counters).
    #[must_use = "stats"]
    pub fn stats(&self) -> playback_engine::EngineStats {
        self.engine.stats()
    }

    /// The open session's counting sink (None between sessions; the
    /// just-closed session's sink survives until the next open — the
    /// cycle epilogue reads its totals).
    #[must_use = "observable evidence"]
    pub fn sink(&self) -> Option<&CountingSink> {
        self.sink.as_deref()
    }

    /// Allocate the next request id.
    fn next_request(&mut self) -> RequestId {
        self.allocator.allocate()
    }

    /// Drain one output batch, processing EVERY output exactly once:
    /// the receipt matching `looking_for` is returned (everything else
    /// is counted), seek/switch terminals are recorded into
    /// [`SoakHost::last_seek_completed`]/[`SoakHost::switch_landed`],
    /// fatals keep their sentence for triage. The wall-clock sampler
    /// ticks if due.
    ///
    /// The one-drain rule is load-bearing (found the hard way): the
    /// engine legitimately lands a seek's `Seeked` terminal in the
    /// SAME poll as the seek's receipt (the tick runs in the same
    /// poll as the apply) — a drain that only looks for receipts
    /// drops that terminal unseen and the terminal await then pumps
    /// forever against a state that has already moved past it.
    fn drain_outputs(&mut self, looking_for: Option<RequestId>) -> Option<CommandReceipt> {
        let mut found = None;
        for output in self.engine.take_outputs() {
            match output {
                EngineOutput::Receipt(receipt) => {
                    if looking_for == Some(receipt.request) {
                        found = Some(receipt);
                    } else {
                        self.receipts_seen = self.receipts_seen.saturating_add(1);
                    }
                }
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::Seeked { .. },
                    ..
                }) => {
                    self.events_seen = self.events_seen.saturating_add(1);
                    self.last_seek_completed = Some(true);
                }
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::SeekFailed { .. },
                    ..
                }) => {
                    self.events_seen = self.events_seen.saturating_add(1);
                    self.last_seek_completed = Some(false);
                }
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::TrackSelectionChanged { .. },
                    ..
                }) => {
                    self.events_seen = self.events_seen.saturating_add(1);
                    self.switch_terminals = self.switch_terminals.saturating_add(1);
                    self.switch_landed = true;
                }
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::Fatal { error },
                    ..
                }) => {
                    self.events_seen = self.events_seen.saturating_add(1);
                    self.last_fatal = Some(error.to_string());
                }
                EngineOutput::Event(EngineEvent { .. }) => {
                    self.events_seen = self.events_seen.saturating_add(1);
                }
            }
        }
        if let Some(sampling) = self.sampling.as_mut() {
            sampling.sample_if_due();
        }
        found
    }

    /// One poll: advance the engine a bounded tick and drain the
    /// outputs through the one drain (see [`SoakHost::drain_outputs`]
    /// for why every output passes through the same processing).
    pub fn pump(&mut self) {
        self.engine.poll();
        self.polls = self.polls.saturating_add(1);
        self.drain_outputs(None);
    }

    /// Pump until `predicate` holds on the state (bounded; a hang is
    /// a failed test — the caller asserts the returned truth).
    pub fn pump_until(&mut self, predicate: impl Fn(SessionState) -> bool, limit: usize) -> bool {
        for _ in 0..limit {
            if predicate(self.engine.state()) {
                return true;
            }
            self.pump();
        }
        predicate(self.engine.state())
    }

    /// Await one submitted command's receipt (bounded pumps). Events
    /// that land in the same polls are processed by the drain (a
    /// terminal arriving with the receipt is recorded, never lost).
    fn await_receipt(
        &mut self,
        request: RequestId,
        context: &str,
    ) -> Result<CommandReceipt, CommandError> {
        for _ in 0..AWAIT_POLLS {
            self.engine.poll();
            self.polls = self.polls.saturating_add(1);
            if let Some(receipt) = self.drain_outputs(Some(request)) {
                return Ok(receipt);
            }
        }
        Err(CommandError::new(
            Stage::Control,
            ErrorCode::Internal,
            format!("soak: the engine never answered {context} (bounded wait expired)"),
        ))
    }

    /// Await the seek terminal (`Seeked`/`SeekFailed`) — the class-A
    /// completion truth — pumping at most `AWAIT_POLLS` times. The
    /// terminal may already have landed (the drain records it even in
    /// the receipt's own poll); the flag was cleared at submit, so
    /// `Some` here is THIS seek's answer.
    fn await_seek_terminal(&mut self) -> Result<(), CommandError> {
        for _ in 0..AWAIT_POLLS {
            if self.last_seek_completed.is_some() {
                return Ok(());
            }
            self.engine.poll();
            self.polls = self.polls.saturating_add(1);
            self.drain_outputs(None);
        }
        if self.last_seek_completed.is_some() {
            return Ok(());
        }
        Err(CommandError::new(
            Stage::Control,
            ErrorCode::Internal,
            "soak: the engine never reported the seek terminal (bounded wait expired)",
        ))
    }

    /// Await the track-switch terminal (`TrackSelectionChanged`); the
    /// flag was cleared at submit, so `true` here is THIS switch's
    /// terminal (same-poll landings included).
    fn await_switch_terminal(&mut self) -> Result<(), CommandError> {
        for _ in 0..AWAIT_POLLS {
            if self.switch_landed {
                return Ok(());
            }
            self.engine.poll();
            self.polls = self.polls.saturating_add(1);
            self.drain_outputs(None);
        }
        if self.switch_landed {
            return Ok(());
        }
        Err(CommandError::new(
            Stage::Control,
            ErrorCode::Internal,
            "soak: the engine never reported the track-switch terminal (bounded wait expired)",
        ))
    }

    /// Submit a command and await its receipt, translating the receipt
    /// into the truthful-UI outcome (the `CliHost::submit_and_await` +
    /// `receipt_outcome` shapes).
    fn submit_and_await(
        &mut self,
        kind: SessionCommandKind,
        context: &str,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        let request = self.next_request();
        let handle = self.handle.as_ref().ok_or_else(|| {
            CommandError::new(Stage::Session, ErrorCode::NotReady, "soak: no session open")
        })?;
        handle
            .submit(request, kind)
            .map_err(|error| session_error_wire(&error))?;
        let receipt = self.await_receipt(request, context)?;
        receipt_outcome(receipt)
    }

    /// The `session.open` path (the `demux_host::open_source` shape,
    /// test-side). Recorded divergence: **no `media-probe` stage** —
    /// `media-probe` is not in this crate's dev-dependency set (the
    /// Lead-owned manifest + boundaries entry), so the open path is
    /// access → demuxer directly, the `tests/track_switch_fixture.rs`
    /// precedent. The probe sits upstream of the engine seam this soak
    /// measures either way.
    fn open_source(
        &mut self,
        source: &SourceSpec,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        let path = match source {
            SourceSpec::File { path } => path.clone(),
            SourceSpec::Url { url } => {
                return Err(CommandError::new(
                    Stage::Input,
                    ErrorCode::UnsupportedForHost,
                    format!("soak: URL sources are not in this harness: {url}"),
                ));
            }
        };
        let uri = file_uri(&path)?;
        // 1. Access (the 2.A1 stack).
        let access = input::FileAccess::new();
        let stream = access.open(&uri).map_err(|error| wire_error(&error))?;
        // 2. Demuxer + the host pre-pass (track list + strf facts).
        let mut demuxer = AviDemuxer::new(stream).map_err(|error| wire_error(&error))?;
        let tracks = demuxer.start().map_err(|error| wire_error(&error))?;
        let mut pcm_facts = Vec::with_capacity(tracks.len());
        for track in &tracks {
            if let Some(StreamFormat::Pcm {
                channels,
                samples_per_sec,
                ..
            }) = demuxer.stream_format(track.id)
            {
                pcm_facts.push((track.id.handle(), (u32::from(channels), samples_per_sec)));
            }
        }
        // 3. The session config (frozen virtual clock + throttle —
        //    scenario determinism; the memory measurement is the
        //    wall-clock cadence inside `pump`).
        let sink = Arc::new(CountingSink::default());
        let provider = Arc::new(SoakProvider::new(pcm_facts));
        let consumer: Arc<dyn FrameConsumer> = sink.clone();
        let clock = Arc::new(
            media_clock::MediaClock::system_monotonic(
                Arc::new(FrozenNow),
                media_types::Timestamp::new(0, media_clock::NANO_BASE),
                media_types::Rate::ONE,
            )
            .must(),
        );
        let mut config = SessionConfig::new(uri, Box::new(demuxer), provider, consumer, clock);
        config.queues = soak_queues();
        config.budget = soak_budget();
        config.throttle = playback_engine::EventPolicy {
            tick: playback_engine::TimeTickPolicy::default_policy(),
            throttle: Arc::new(FrozenNow),
        };
        // 4. Open on the engine (reopen after close is this same call).
        let handle = self
            .engine
            .open(config)
            .map_err(|error| session_error_wire(&error))?;
        self.handle = Some(handle);
        self.sink = Some(sink);
        self.last_seek_completed = None;
        // 5. Settle: pump to the FIRST state the playback ops serve
        //    (Buffering included — the earliest mid-media observation).
        //    The settle must NOT wait for a "stable" state: the soak's
        //    sink accepts frames the moment they decode, so a 2-second
        //    fixture drains to `Ended` within a tick or two of priming
        //    — settling to Playing/Ended would land every later op in
        //    the ended lane the table refuses. Exiting at the first
        //    Buffering/Playing sighting leaves the session mid-media
        //    with the seek/switch ops ahead of it (measured: with the
        //    stable-state settle, 1597/1597 cycles skipped the seek).
        let settled = self.pump_until(
            |state| {
                matches!(
                    state,
                    SessionState::Buffering
                        | SessionState::Playing
                        | SessionState::Paused
                        | SessionState::Ended
                        | SessionState::Error
                )
            },
            AWAIT_POLLS,
        );
        if !settled {
            return Err(CommandError::new(
                Stage::Control,
                ErrorCode::Internal,
                "soak: the open never settled to a stable state (bounded wait expired)",
            ));
        }
        Ok(CommandOutcome::Accepted {
            session: self.engine.session_id(),
            generation: self.engine.generation().unwrap_or(Generation(0)),
        })
    }
}

/// **The** soak dispatch entry (the ARCH-5 single-entry shape; see the
/// module docs for the recorded lineage: `apps/cli`'s
/// `lumen_dispatch` is unreachable from a playback-engine test —
/// dependency direction — so this mirror serves the same catalog
/// envelopes onto the same `SessionHandle` seam).
///
/// Served: `session.open`, `session.close`, `playback.play`,
/// `playback.pause`, `playback.stop`, `playback.seek`,
/// `tracks.set-audio-track`. Everything else answers the honest typed
/// `unsupported-for-host` refusal naming this harness.
#[must_use = "the response is the evidence"]
pub fn lumen_soak_dispatch(host: &mut SoakHost, request: &WireRequest) -> WireResponse {
    let request_id = request.request_id;
    let result = route(host, &request.command);
    WireResponse {
        request_id,
        session: host.engine.session_id(),
        generation: host.engine.generation().unwrap_or(Generation(0)),
        result,
    }
}

/// The served table (the CLI `route()` shape, playing subset + the
/// track-switch id).
fn route(
    host: &mut SoakHost,
    command: &Command,
) -> Result<CommandOutcome<CommandResult>, CommandError> {
    match command {
        Command::SessionOpen { source, .. } => host.open_source(source),
        Command::SessionClose => {
            // The engine's own executor composes close from an active
            // state (card 3.C1's `close_plan` macro: stop settles,
            // close destroys — ONE Completed receipt for the whole
            // composition, the honest `… → stopped → idle` trail), so
            // the harness submits the close as-is from every state.
            let outcome = host.submit_and_await(SessionCommandKind::Close, "session.close")?;
            // Pump to the settled Idle state (the session's teardown
            // completes on the poll path).
            if !host.pump_until(|state| state == SessionState::Idle, AWAIT_POLLS) {
                return Err(CommandError::new(
                    Stage::Control,
                    ErrorCode::Internal,
                    "soak: the close never settled to idle (bounded wait expired)",
                ));
            }
            host.handle = None;
            Ok(outcome)
        }
        Command::PlaybackPlay => host.submit_and_await(SessionCommandKind::Play, "playback.play"),
        Command::PlaybackPause => {
            host.submit_and_await(SessionCommandKind::Pause, "playback.pause")
        }
        Command::PlaybackStop => host.submit_and_await(SessionCommandKind::Stop, "playback.stop"),
        Command::PlaybackSeek { target } => dispatch_seek(host, *target),
        Command::TracksSetAudioTrack { selector } => dispatch_audio_switch(host, selector),
        other => Err(unserved(other)),
    }
}

/// `playback.seek` (the `CliHost::dispatch_seek` shape minus the
/// host-side audio ring — this harness's sink is immediate): wire
/// target → engine target (resolution and clamping stay the ENGINE's,
/// doc 05 §2.4), class-A accept, then the `Seeked`/`SeekFailed`
/// terminal awaited bounded.
fn dispatch_seek(
    host: &mut SoakHost,
    target: WireSeekTarget,
) -> Result<CommandOutcome<CommandResult>, CommandError> {
    let engine_target = match target {
        WireSeekTarget::Absolute { time } => SeekTarget::Absolute {
            position_ns: time.0.saturating_mul(1000),
        },
        WireSeekTarget::Relative { delta_ms } => SeekTarget::Relative {
            delta_ns: delta_ms.saturating_mul(1_000_000),
        },
        WireSeekTarget::Percent { fraction } => SeekTarget::Percent { fraction },
        WireSeekTarget::Chapter { index } => {
            return Err(CommandError::new(
                Stage::Session,
                ErrorCode::NotFound,
                format!(
                    "soak: no chapter {index}: AVI v0 carries no chapter markers (the chapter card owns this)"
                ),
            ));
        }
    };
    let request = host.next_request();
    // This seek's terminal flag (the drain records the landing even
    // in the receipt's own poll — see `drain_outputs`).
    host.last_seek_completed = None;
    let handle = host.handle.as_ref().ok_or_else(|| {
        CommandError::new(Stage::Session, ErrorCode::NotReady, "soak: no session open")
    })?;
    handle
        .seek(request, engine_target)
        .map_err(|error| session_error_wire(&error))?;
    let receipt = host.await_receipt(request, "playback.seek")?;
    let outcome = receipt_outcome(receipt)?;
    host.await_seek_terminal()?;
    Ok(outcome)
}

/// `tracks.set-audio-track`: the engine's selector vocabulary from the
/// wire's (Id/None — the soak's switch ops), class-A accept, then the
/// `tracks.changed` terminal awaited bounded.
fn dispatch_audio_switch(
    host: &mut SoakHost,
    selector: &WireTrackSelector,
) -> Result<CommandOutcome<CommandResult>, CommandError> {
    let engine_selector = match selector {
        WireTrackSelector::Id { id } => TrackSelector::Id {
            id: TrackId::new(id.0),
        },
        WireTrackSelector::None => TrackSelector::None,
        WireTrackSelector::Auto => {
            return Err(CommandError::new(
                Stage::Session,
                ErrorCode::UnsupportedForHost,
                "soak: Auto audio selection is the product's rule set, not the engine's",
            ));
        }
    };
    let request = host.next_request();
    // This switch's terminal flag (same-drain rule as the seek).
    host.switch_landed = false;
    let handle = host.handle.as_ref().ok_or_else(|| {
        CommandError::new(Stage::Session, ErrorCode::NotReady, "soak: no session open")
    })?;
    handle
        .set_audio_track(request, engine_selector)
        .map_err(|error| session_error_wire(&error))?;
    let receipt = host.await_receipt(request, "tracks.set-audio-track")?;
    let outcome = receipt_outcome(receipt)?;
    host.await_switch_terminal()?;
    Ok(outcome)
}

// ---------------------------------------------------------------------------
// The wire translations (mirrors of the CLI host's, over the same
// playback-engine published mappings)
// ---------------------------------------------------------------------------

/// One engine error → the command layer's redaction-safe form (the
/// `demux_host::wire_error` shape over playback-engine's own published
/// stage/code mappings).
fn wire_error(error: &EngineError) -> CommandError {
    let full = error.to_string();
    let stage_prefix = format!("[{}] ", error.stage);
    let base = full.strip_prefix(&stage_prefix).unwrap_or(full.as_str());
    CommandError::new(wire_stage(error.stage), wire_error_code(&error.kind), base)
        .with_detail("code", error.context.code.as_str())
}

/// A session-construction error's wire form (the
/// `CliHost::session_error_wire` shape).
fn session_error_wire(error: &SessionError) -> CommandError {
    match error {
        SessionError::Config(cause) => {
            CommandError::new(Stage::Session, ErrorCode::Unsupported, cause.to_string())
                .with_detail("code", "session.no-decodable-track")
        }
        SessionError::GenerationExhausted(engine) => wire_error(engine),
        SessionError::SessionClosed => CommandError::new(
            Stage::Session,
            ErrorCode::Canceled,
            "soak: the session closed before the command landed",
        ),
        SessionError::InboxBusy => CommandError::new(
            Stage::Session,
            ErrorCode::ResourceLimit,
            "soak: the session inbox is full — pump the engine, then retry",
        )
        .with_detail("limit", "inbox-capacity"),
        SessionError::Invariant(sentence) => CommandError::new(
            Stage::Control,
            ErrorCode::Internal,
            format!("soak: an engine invariant was violated: {sentence}"),
        ),
    }
}

/// One receipt → the truthful-UI outcome (the
/// `CliHost::receipt_outcome` shape: class A answers `Accepted`, class
/// S answers `Completed`, rejections answer the typed wire error).
fn receipt_outcome(receipt: CommandReceipt) -> Result<CommandOutcome<CommandResult>, CommandError> {
    match receipt.result {
        ReceiptResult::Accepted {
            session,
            generation,
        } => Ok(CommandOutcome::Accepted {
            session: Some(session),
            generation,
        }),
        ReceiptResult::Completed { session } => {
            let _ = session;
            Ok(CommandOutcome::Completed(CommandResult::Unit))
        }
        ReceiptResult::Rejected { state, reason } => Err(rejection_wire(state, reason)),
    }
}

/// A typed rejection → the command layer's error (the
/// `CliHost::rejection_wire` mirror, same stages and codes).
fn rejection_wire(state: SessionState, reason: RejectionReason) -> CommandError {
    let stage = Stage::Session;
    let state_name = playback_engine::wire_playback_state(state);
    match reason {
        RejectionReason::NoSession => {
            CommandError::new(stage, ErrorCode::NotReady, "no session open")
        }
        RejectionReason::NotReady => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!("the session is still opening (state: {state_name:?})"),
        ),
        RejectionReason::SettleFirst => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!(
                "close from {state_name:?} composes through stop (the executor applies it; \
                 the table stays pure)"
            ),
        ),
        RejectionReason::AlreadySettled => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!(
                "the session already settled (state: {state_name:?}); close is the remaining exit"
            ),
        ),
        RejectionReason::RestartNeedsSeek => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!(
                "play in {state_name:?} means restart-from-ended — the Phase-3 seek flow (card 3.A1)"
            ),
        ),
        RejectionReason::SourceReplacementPhase3 => CommandError::new(
            stage,
            ErrorCode::NotReady,
            "open on a live session is source replacement (card 3.A2)",
        ),
        RejectionReason::SignalNotRaisedHere => CommandError::new(
            Stage::Control,
            ErrorCode::Internal,
            format!(
                "a pipeline signal fired in a state whose driver never raises it ({state_name:?}) — bug report"
            ),
        ),
        RejectionReason::EndedNeedsWireEdge => CommandError::new(
            stage,
            ErrorCode::NotReady,
            format!(
                "seek in {state_name:?} needs the doc 05 §3.3 ended→buffering wire edge (Lead-owned change)"
            ),
        ),
        RejectionReason::MissingPayload => CommandError::new(
            Stage::Control,
            ErrorCode::Internal,
            "a seek command arrived without its target payload — caller bug",
        ),
        RejectionReason::TrackNotFound => CommandError::new(
            Stage::Control,
            ErrorCode::NotFound,
            "track selection named a track this session does not have",
        ),
    }
}

/// The typed refusal for a catalog command this harness does not serve
/// (the CLI `unserved()` shape — capability gaps are typed, never
/// silent).
fn unserved(command: &Command) -> CommandError {
    CommandError::new(
        Stage::Control,
        ErrorCode::UnsupportedForHost,
        format!(
            "`{command:?}` is not served by the soak harness (the playing subset + track switching)"
        ),
    )
    .with_detail("harness", "playback-engine/tests/soak")
}

// ---------------------------------------------------------------------------
// The file URI (the demux_host::encode_path rule, cited)
// ---------------------------------------------------------------------------

/// Percent-encode a `file:` URI path: keep RFC 3986 unreserved
/// characters plus `/`; escape everything else so `SourceUri::parse`
/// accepts the result and `input`'s resolver round-trips it (the
/// `apps/cli/src/demux_host.rs` `encode_path` rule, mirrored — it is
/// private to that crate).
fn encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/');
        if keep {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Canonicalize a path and build its `file:` URI (the
/// `demux_host::file_uri` shape).
fn file_uri(path: &Path) -> Result<SourceUri, CommandError> {
    let canonical = std::fs::canonicalize(path).map_err(|err| {
        CommandError::new(
            Stage::Input,
            ErrorCode::Inaccessible,
            format!("cannot resolve {}: {err}", path.display()),
        )
    })?;
    if !canonical.is_file() {
        return Err(CommandError::new(
            Stage::Input,
            ErrorCode::Inaccessible,
            format!("not a regular file: {}", canonical.display()),
        ));
    }
    let text = canonical.to_str().ok_or_else(|| {
        CommandError::new(
            Stage::Input,
            ErrorCode::Unsupported,
            "the path is not valid UTF-8",
        )
    })?;
    SourceUri::parse(&format!("file://{}", encode_path(text))).map_err(|err| {
        CommandError::new(
            Stage::Input,
            ErrorCode::Internal,
            format!("the encoded file URI was rejected: {err}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::catalog::types::Timestamp;
    use commands::envelope::RequestId;

    /// The unserved surface answers the typed refusal — never a
    /// panic, never silence (the CLI dispatch test's shape).
    #[test]
    fn unserved_commands_refuse_typed() {
        let _serial = testkit::alloc_count::serialize_tests();
        let mut host = SoakHost::new();
        let request = WireRequest {
            request_id: RequestId(1),
            command: Command::PlaybackSetRate { rate: 1.0 },
        };
        let response = lumen_soak_dispatch(&mut host, &request);
        match response.result {
            Err(error) => {
                assert_eq!(error.code, ErrorCode::UnsupportedForHost, "{error}");
                assert_eq!(error.stage, Stage::Control, "{error}");
            }
            Ok(_) => unreachable!("set-rate must not be served"),
        }
    }

    /// A seek with no session: the honest not-ready (not a crash).
    #[test]
    fn seek_without_session_refuses() {
        let _serial = testkit::alloc_count::serialize_tests();
        let mut host = SoakHost::new();
        let request = WireRequest {
            request_id: RequestId(2),
            command: Command::PlaybackSeek {
                target: WireSeekTarget::Absolute { time: Timestamp(0) },
            },
        };
        let response = lumen_soak_dispatch(&mut host, &request);
        match response.result {
            Err(error) => assert_eq!(error.code, ErrorCode::NotReady, "{error}"),
            Ok(_) => unreachable!("seek without a session must refuse"),
        }
    }
}
