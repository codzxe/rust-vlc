//! The web engine host (card 7.H1, issue #47) — the CLI host's web
//! twin at v1 scope, driven through **cooperative pump slices**
//! (doc 09 §3.2).
//!
//! # What it owns
//!
//! [`WebHost`] owns the SAME std-only engine core the CLI drives
//! (probe → demux → [`Engine`] → consumer → clock), correlates
//! receipts to requests, converts engine events to the wire stream
//! through the staleness filter — and, the web-specific half, runs it
//! all under the pump: one [`HostToWorker::Pump`] message is one
//! bounded slice that visits the roles in doc 09 §3.2's priority
//! order (audio fill first, then a bounded engine-poll batch — each
//! poll is one inbox drain plus one bounded pipeline tick (demux
//! ≤ 64 events, decode ≤ 16 packets per limb, the engine's own
//! [`TickBudget`]) — then a bounded event flush), after which control
//! RETURNS to the Worker event loop. Nothing blocks, nothing spins:
//! [`SliceBudgets`] bounds every role per tick and caps the tick
//! count per pump message, and the JS glue re-arms the pump from
//! `setTimeout(0)` ticks while the [`PumpReport`] says `busy`.
//!
//! # The count-form of the slice budget (an honest deviation)
//!
//! Doc 09 §3.2 sketches the slice as a *deadline* ("4 ms
//! audio-critical / 8 ms total per pump", both `[PROPOSAL]` —
//! numbers). This host implements the **count form**: bounded
//! operations per role instead of bounded wall time, because
//! `std::time::Instant` **panics at runtime on
//! `wasm32-unknown-unknown`** (the target provides no monotonic
//! clock; std's stub aborts) — a deadline form would need the host
//! page's `performance.now()` fed in through the ABI, which is the
//! 7.H-pillar integration. The count form is also the deterministic
//! one: every observable is a pure function of the pump count, the
//! same property the CLI's headless lane proof relies on. The
//! mapping is recorded in [`SliceBudgets`]'s docs.
//!
//! # Time on this host
//!
//! The session clock is ALWAYS the null device's sample counter
//! (audio-master, [`MediaClock::audio_master`]) — never the system
//! monotonic (which panics on wasm32, above). With an audio limb,
//! decode stages real audio into the ring and the pump's bounded
//! renders play it out; without one (video-only, `--no-audio`) the
//! pump renders silence quanta so the counter — and with it media
//! time — still advances deterministically. The event-throttle
//! source is the [`PumpClock`]: a monotonic counter advanced by the
//! pump's own renders, so doc 05 §3.4's emission windows track media
//! time, not wall time. Zero wall-clock reads anywhere on the path.
//!
//! # The honest posture
//!
//! The served command surface is the first vertical's playing subset
//! (open/close/play/pause/toggle/stop + the class-S snapshots +
//! engine-info/list-commands — see [`SERVED`]); `playback.seek`
//! answers the typed refusal naming the missing host duty (the seek
//! boundary's ring-discard + generation-publish halves, whose
//! reference implementation lives in the CLI host), and
//! `config.*` answers the typed refusal naming the web settings
//! card (localStorage persistence, doc 09 §9.6). Every other catalog
//! id refuses `unsupported-for-host` with its owning subsystem named
//! — typed, never silent (doc 05 §5.2).
//!
//! # Evidence labels
//!
//! Everything here is exercised headless-native by this crate's
//! tests (the SAME message paths the JS glue drives, through
//! [`crate::abi`]). Browser-runtime behavior (worker instantiation,
//! `postMessage`, `setTimeout` re-arming) is `[VERIFY]` — no browser
//! exists in this sandbox.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use audio_engine::{AudioPipeline, PipelineSpec};
use audio_output::null::{NullDevice, NullSink};
use audio_output::{NullBackend, NullConfig, NullStream};
use codec_api::{Decoder, DecoderConfig};
// The drain-gate's trait surface (native only, with its sole
// constructor — see DrainGatedDecoder's cfg note).
#[cfg(not(target_arch = "wasm32"))]
use codec_api::{DecodeOutcome, FrameSink};
use commands::catalog::command::{CommandListing, CommandResult};
use commands::catalog::types::{
    Chapter, DecoderInfo, Duration, EngineInfo, MediaInfo, TimeRange, Timestamp,
};
use commands::envelope::{CommandOutcome, Generation, RequestId};
use commands::{
    Command, CommandError, CommandId, PlaybackState, RequestIdAllocator, StalenessFilter,
    WireEvent, WireRequest, WireResponse,
};
use media_clock::{MediaClock, MonotonicSource, NANO_BASE, PlaybackClock};
use media_types::frame::{AudioFormat, AudioFrame, AudioLayout, SampleFormat};
use media_types::time::Timestamp as MediaTimestamp;
use media_types::{EngineError, Frame, MediaKind, PipelineStage, Rate, Track, TrackId};
use playback_engine::event::EngineEventKind;
use playback_engine::{
    CommandReceipt, DecoderProvider, Engine, EngineOutput, EventPolicy, FrameConsumer,
    ReceiptResult, SessionCommandKind, SessionConfig, SessionError, SessionHandle, SessionState,
    TimeTickPolicy,
};

use crate::capabilities::{self, HostCapabilities, TargetProfile};
use crate::open::{self, MediaFacts, TrackFact, WebOpenOptions};
use crate::protocol::{HostToWorker, PumpReport, WorkerToHost};
use crate::source::BlobRegistry;

/// The audio render quantum (frames per device render) — the CLI
/// host's own constant, mirrored (the null device's chunk shape;
/// `[PROPOSAL]` there: a device-period setting joins with real
/// backends per ADR-005).
pub const RENDER_QUANTUM: usize = 1470;

/// The null output ring's capacity (frames) — the CLI host's own
/// bound, mirrored: ~3 s at 44.1 kHz absorbs a full rebuffer cycle
/// with >2× margin; beyond it the sink's own `ring_full` refusal is
/// the honest bound.
pub const RING_FRAMES: usize = 131_072;

/// The bounded command-await: submit → poll → receipt, at most this
/// many engine polls before the typed "engine never answered"
/// refusal (the CLI host's `AWAIT_PUMPS` twin — a bound, never a
/// block, doc 09 §3.2).
pub const AWAIT_POLLS: usize = 64;

/// The cooperative slice budgets (doc 09 §3.2, count form — see the
/// module docs). Every field is a hard per-pump bound; a `Pump`
/// message asking for more ticks than [`Self::max_ticks_per_pump`]
/// is CAPPED, never honored (one message can never hog the Worker
/// loop).
///
/// # The deadline mapping (honest, recorded)
///
/// Doc 09 §3.2's `[PROPOSAL]` numbers are 4 ms audio-critical / 8 ms
/// total per pump. The count form's defaults: 16 audio quanta ≈
/// 533 ms of ring play-out per tick (audio-fill runs FIRST and only
/// over content the ring already holds — the decode-ahead regime
/// the first vertical runs), 4 engine polls × the engine's own
/// `TickBudget` (demux ≤ 64 events, decode ≤ 16 packets/limb) per
/// tick, 64 wire events flushed per tick, 32 ticks per pump message,
/// 8 settle polls at open. On the null device these are countable
/// and deterministic; when the `performance.now()` time source
/// arrives with the H-pillars the deadline form slots in behind the
/// same [`WebHost::pump`] entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceBudgets {
    /// Audio renders per pump tick (the audio-fill priority role).
    pub audio_quanta_per_tick: u32,
    /// Engine polls per pump tick (inbox drain + one bounded
    /// pipeline tick each — the demux/decode batch role).
    pub polls_per_tick: u32,
    /// Wire events flushed per pump tick (the events-flush role).
    pub events_per_flush: u32,
    /// The tick cap for one `Pump` message (the never-hog bound).
    pub max_ticks_per_pump: u32,
    /// Engine polls at open before the class-A answer (catches
    /// synchronous fatals — corrupt header, no decodable track —
    /// in the response; deeper settling continues on pump slices).
    pub settle_polls: u32,
}

impl Default for SliceBudgets {
    fn default() -> SliceBudgets {
        SliceBudgets {
            audio_quanta_per_tick: 16,
            polls_per_tick: 4,
            events_per_flush: 64,
            max_ticks_per_pump: 32,
            settle_polls: 8,
        }
    }
}

/// The pump's own monotonic time source (doc 05 §3.4's emission
/// windows): advanced by the pump's renders, read by the engine's
/// event throttle. Media time, not wall time — deterministic and
/// wasm-safe (`std::time::Instant` panics on
/// `wasm32-unknown-unknown`; this counter is the honest replacement
/// until the H-pillars feed `performance.now()` through the ABI).
#[derive(Debug, Default)]
pub struct PumpClock {
    nanos: AtomicU64,
}

impl PumpClock {
    /// A clock anchored at zero.
    #[must_use]
    pub fn new() -> PumpClock {
        PumpClock::default()
    }

    /// Advance by `nanos` (the frames the pump just rendered).
    pub fn advance_ns(&self, nanos: u64) {
        self.nanos.fetch_add(nanos, Ordering::Release);
    }

    /// The current reading.
    #[must_use]
    pub fn nanos(&self) -> u64 {
        self.nanos.load(Ordering::Acquire)
    }
}

impl MonotonicSource for PumpClock {
    fn now_ns(&self) -> u64 {
        self.nanos()
    }
}

/// The audio processing lane: the mix pipeline plus the null sink's
/// write half (the CLI host's `AudioLimb`, mirrored at v1 scope —
/// no drift governor arming: nothing on this host paces production
/// at the consumption rate yet, so arming would "correct" decode
/// buffering as drift; see the CLI's regime note). Constructed by
/// the open path ([`WebHost`]); the fields stay private.
pub struct AudioLane {
    pipeline: AudioPipeline,
    sink: NullSink,
}

impl AudioLane {
    /// Stage + mix + write one decoded audio frame into the ring.
    ///
    /// # Errors
    /// The pipeline's typed errors (format mismatch, mix shape) and
    /// the sink's (ring full) — already stage-tagged.
    fn stage(&mut self, frame: &AudioFrame) -> Result<(), EngineError> {
        self.pipeline.process(0, frame)?;
        self.pipeline.mix()?;
        if let Some(buffer) = self.pipeline.take_output() {
            self.sink.write_bytes(buffer.data.as_slice())?;
        }
        Ok(())
    }

    /// Frames the device has not played yet.
    fn buffered_frames(&self) -> usize {
        self.sink.buffered_frames()
    }
}

/// The v1 headless presenter's delivery counters (the honest video
/// truth while the canvas surface is card 7.H2: every frame the
/// engine delivered is counted, never dropped silently).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConsumerCounters {
    /// Frames that reached `accept` (both limbs).
    pub frames_accepted: u64,
    /// Frames the consumer refused (typed, propagated to decode).
    pub frames_refused: u64,
    /// Decoded video frames counted by the headless presenter.
    pub video_frames: u64,
    /// Video bytes delivered (coded-plane storage).
    pub video_bytes: u64,
    /// Audio frames staged into the ring.
    pub audio_frames: u64,
    /// Subtitle updates delivered.
    pub subtitle_frames: u64,
    /// The last delivered video frame's pts (ns) — the headless
    /// presenter's latch (the position truth for audio-less lanes).
    pub last_presented_ns: Option<i64>,
}

struct ConsumerState {
    /// The audio lane, when the session has one.
    lane: Option<AudioLane>,
    counters: ConsumerCounters,
}

/// The web host's [`FrameConsumer`]: the audio half stages into the
/// null ring (the clock's truth source), the video half counts into
/// the headless presenter (7.H2 replaces it with the frame-upload
/// path), the subtitle half counts. One short internal lock, never
/// held across I/O (the media-clock concurrency contract).
pub struct WebFrameConsumer {
    state: Mutex<ConsumerState>,
}

impl WebFrameConsumer {
    /// A consumer over `lane` (audio sessions) or limb-less
    /// (video-only / `--no-audio`).
    #[must_use]
    pub fn new(lane: Option<AudioLane>) -> WebFrameConsumer {
        WebFrameConsumer {
            state: Mutex::new(ConsumerState {
                lane,
                counters: ConsumerCounters::default(),
            }),
        }
    }

    /// The delivery counters (the headless presenter's evidence).
    #[must_use]
    pub fn counters(&self) -> ConsumerCounters {
        lock_or_clone(&self.state).counters
    }

    /// Frames the device ring still holds (the play-out tail).
    #[must_use]
    pub fn buffered_frames(&self) -> usize {
        lock_or_clone(&self.state)
            .lane
            .as_ref()
            .map_or(0, AudioLane::buffered_frames)
    }
}

/// Lock a consumer state, recovering from poisoning by taking the
/// inner value (the state is plain counters + the lane; a panicked
/// critical section cannot corrupt them, and the worker is
/// single-threaded anyway — the lock exists for the trait's
/// `Send + Sync` bound).
fn lock_or_clone(state: &Mutex<ConsumerState>) -> std::sync::MutexGuard<'_, ConsumerState> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

impl FrameConsumer for WebFrameConsumer {
    fn accept(&self, frame: Frame, _track: TrackId) -> Result<(), EngineError> {
        let mut guard = lock_or_clone(&self.state);
        match frame {
            Frame::Audio(audio) => {
                let Some(lane) = guard.lane.as_mut() else {
                    // The engine wires the audio limb only for tracks
                    // with geometry; a frame arriving without one is a
                    // wiring bug — the typed refusal, never a panic.
                    guard.counters.frames_refused += 1;
                    return Err(crate::open::host_error(
                        PipelineStage::AudioOutput,
                        "web.audio_lane_absent",
                        "an audio frame reached a consumer with no audio lane",
                    ));
                };
                let bytes_per_frame = u64::from(
                    audio
                        .format
                        .channels
                        .saturating_mul(audio.format.sample.bytes_per_sample())
                        .max(1),
                );
                let frames = (audio.samples.len() as u64) / bytes_per_frame;
                match lane.stage(&audio) {
                    Ok(()) => {
                        guard.counters.frames_accepted += 1;
                        guard.counters.audio_frames += frames;
                        Ok(())
                    }
                    Err(error) => {
                        guard.counters.frames_refused += 1;
                        Err(error)
                    }
                }
            }
            Frame::Video(video) => {
                let bytes = match &video.data {
                    media_types::frame::FrameStorage::Cpu(bytes) => bytes.len() as u64,
                    media_types::frame::FrameStorage::OpaqueHandle(_) => 0,
                };
                guard.counters.frames_accepted += 1;
                guard.counters.video_frames += 1;
                guard.counters.video_bytes += bytes;
                guard.counters.last_presented_ns = video
                    .pts
                    .map_timestamp(|stamp| stamp.convert(&NANO_BASE).ok().map(|ns| ns.ticks))
                    .flatten();
                Ok(())
            }
            Frame::Subtitle(_) => {
                guard.counters.frames_accepted += 1;
                guard.counters.subtitle_frames += 1;
                Ok(())
            }
        }
    }
}

/// The typed error for the wasm32 profile's decoder seam (the
/// capability snapshot's own blocked row, as an engine error).
fn decoders_absent_error() -> EngineError {
    crate::open::host_error(
        PipelineStage::Decode,
        "web.decoders_absent",
        "the wasm32 module ships the engine core without the builtin decoders (the \
         codecs crate's external engines do not build for wasm32-unknown-unknown) — \
         the typed refusal until the crate splits (Lead decision, see the \
         capability snapshot's builtin-software-decoders row)",
    )
}

/// The web host's decoder seam. The profile selects the table: native
/// builds dispatch the first vertical's real decoders (MJPEG + PCM,
/// the `codecs` crate — a target-gated dependency), the wasm32
/// build refuses every descriptor typed (the honest gap above).
///
/// The PCM row is wrapped in the drain-gate (the CLI host's own
/// `DrainGatedDecoder`, mirrored): the decoder's `Drained` answer
/// waits on the device ring actually emptying, so end-of-media
/// fires when the media has PLAYED, not when it was decoded — the
/// headless-time handshake (renders are the only time mover).
pub struct WebDecoderProvider {
    profile: TargetProfile,
    facts: Arc<MediaFacts>,
    probe: Option<Arc<WebFrameConsumer>>,
}

impl WebDecoderProvider {
    /// Build over the open path's harvested facts.
    #[must_use]
    pub fn new(profile: TargetProfile, facts: Arc<MediaFacts>) -> WebDecoderProvider {
        WebDecoderProvider {
            profile,
            facts,
            probe: None,
        }
    }

    /// Attach the frame consumer (the drain-gate's backlog probe).
    /// Called before the engine wires limbs; the open path does it
    /// right after constructing the consumer.
    pub fn attach_consumer(&mut self, consumer: Arc<WebFrameConsumer>) {
        self.probe = Some(consumer);
    }
}

impl DecoderProvider for WebDecoderProvider {
    fn decoder_for(
        &self,
        track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        match self.profile {
            TargetProfile::Wasm32 => Err(decoders_absent_error()),
            TargetProfile::Native => native_decoder_for(track, config, &self.facts, &self.probe),
        }
    }
}

/// A decoder whose `Drained` answer waits on the device ring's
/// backlog (the CLI host's `DrainGatedDecoder`, mirrored — see the
/// provider docs). The sub-quantum residue releases the gate: the
/// end-tail play-out (gated on `Ended`) renders it to the exact
/// media end; demanding `backlog == 0` would strand every media
/// whose frame count is not a multiple of the quantum.
///
/// Native only (the same gate as [`native_decoder_for`], its sole
/// constructor): the wasm32 build wires no builtin decoders — the
/// typed refusal — so the wrapper would be dead code there.
#[cfg(not(target_arch = "wasm32"))]
struct DrainGatedDecoder {
    inner: Box<dyn Decoder>,
    probe: Option<Arc<WebFrameConsumer>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl DrainGatedDecoder {
    fn audio_backlog(&self) -> usize {
        self.probe
            .as_ref()
            .map_or(0, |consumer| consumer.buffered_frames())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Decoder for DrainGatedDecoder {
    fn configure(&mut self, config: &DecoderConfig) -> Result<(), EngineError> {
        self.inner.configure(config)
    }

    fn decode(
        &mut self,
        packet: media_types::Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        self.inner.decode(packet, out)
    }

    fn flush(&mut self, out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        match self.inner.flush(out)? {
            DecodeOutcome::Drained if self.audio_backlog() >= RENDER_QUANTUM => {
                // Everything is decoded and sitting in the ring: not
                // drained until the pump's renders have played it.
                Ok(DecodeOutcome::NeedMoreInput)
            }
            outcome => Ok(outcome),
        }
    }
}

/// The native decoder table: the first vertical's MJPEG + PCM rows,
/// dispatched on the track's registry descriptor (the CLI host's
/// own rows, mirrored — every other descriptor is the honest typed
/// refusal below).
#[cfg(not(target_arch = "wasm32"))]
fn native_decoder_for(
    track: &Track,
    config: &DecoderConfig,
    facts: &MediaFacts,
    probe: &Option<Arc<WebFrameConsumer>>,
) -> Result<Box<dyn Decoder>, EngineError> {
    match track.codec.as_str() {
        "mjpeg" => {
            let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
            decoder.configure(config)?;
            Ok(Box::new(decoder))
        }
        codec if codec.starts_with("pcm_") => {
            // The strf facts the open path harvested (the
            // with_format seam — the engine passes no extra_data).
            let Some((channels, rate)) = facts.fact_for(track.id).and_then(|fact| fact.pcm) else {
                return Err(crate::open::host_error(
                    PipelineStage::Decode,
                    "web.pcm_facts_absent",
                    format!("no stream facts for PCM track {}", track.id.handle()),
                ));
            };
            let mut decoder = codecs::audio::pcm::PcmDecoder::with_format(channels, rate)?;
            decoder.configure(config)?;
            Ok(Box::new(DrainGatedDecoder {
                inner: Box::new(decoder),
                probe: probe.clone(),
            }))
        }
        other => Err(crate::open::host_error(
            PipelineStage::Decode,
            "web.codec_unavailable",
            format!(
                "no decoder for `{other}` in this build (landed: mjpeg, pcm_s16le — the \
                 wasm32 web host wires the first vertical only)"
            ),
        )),
    }
}

/// The wasm32 stub of the native table: the profile is always
/// [`TargetProfile::Wasm32`] on that target (`build_profile`'s cfg),
/// so the arm above is unreachable — but it must still compile, and
/// the honest answer is the same typed refusal.
#[cfg(target_arch = "wasm32")]
fn native_decoder_for(
    _track: &Track,
    _config: &DecoderConfig,
    _facts: &MediaFacts,
    _probe: &Option<Arc<WebFrameConsumer>>,
) -> Result<Box<dyn Decoder>, EngineError> {
    Err(decoders_absent_error())
}

/// The engine-info snapshot (class S): the web host's own row —
/// `host: "web"`, the profile's real decoder descriptors, the
/// honest feature tags (the null device, the headless presenter,
/// the cooperative pump).
#[must_use]
pub fn engine_info(profile: TargetProfile) -> EngineInfo {
    EngineInfo {
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        catalog_version: "1".to_string(),
        protocol_version: "unwired (control-protocol is a later card)".to_string(),
        host: "web".to_string(),
        features: vec![
            "first-vertical:avi-mjpeg-pcm".to_string(),
            "audio.null-device".to_string(),
            "video.headless-counter-presenter".to_string(),
            "scheduler.cooperative-pump".to_string(),
        ],
        plugins: Vec::new(),
        decoders: capabilities::decoder_descriptors(profile)
            .into_iter()
            .map(|name| DecoderInfo {
                kind: if name == "mjpeg" {
                    commands::catalog::types::TrackKind::Video
                } else {
                    commands::catalog::types::TrackKind::Audio
                },
                name,
                hardware: false,
                backend: None,
                active: false,
            })
            .collect(),
        sessions: Vec::new(),
    }
}

/// The command catalog listing (class S — the catalog as data, the
/// layer's own table).
#[must_use]
pub fn command_listings() -> Vec<CommandListing> {
    commands::catalog::catalog_data()
        .iter()
        .map(|spec| CommandListing {
            id: spec.id.to_string(),
            class: spec.class,
            since: "1".to_string(),
            deprecated: None,
            requires: Vec::new(),
        })
        .collect()
}

/// The catalog ids this host serves (the first vertical's playing
/// subset — the same shape as the CLI's own `SERVED`, minus the
/// rows this host refuses typed below).
pub const SERVED: &[CommandId] = &[
    CommandId::SessionOpen,
    CommandId::SessionClose,
    CommandId::PlaybackPlay,
    CommandId::PlaybackPause,
    CommandId::PlaybackTogglePlayPause,
    CommandId::PlaybackStop,
    CommandId::MediaGetInfo,
    CommandId::MediaGetCurrentTime,
    CommandId::MediaGetDuration,
    CommandId::TracksList,
    CommandId::SystemGetEngineInfo,
    CommandId::SystemListCommands,
];

/// The web host: the engine, its session-scoped halves, the blob
/// registry, the honest capability snapshot, and the outbound
/// message queue the ABI drains. One instance per Worker.
pub struct WebHost {
    engine: Engine,
    handle: Option<SessionHandle>,
    consumer: Option<Arc<WebFrameConsumer>>,
    device: Option<NullDevice>,
    clock: Option<Arc<MediaClock>>,
    facts: Option<Arc<MediaFacts>>,
    source: Option<commands::catalog::types::SourceSpec>,
    tracks: Vec<commands::catalog::types::TrackInfo>,
    registry: BlobRegistry,
    profile: TargetProfile,
    capabilities: HostCapabilities,
    budgets: SliceBudgets,
    pump_clock: Arc<PumpClock>,
    audio_rate: u32,
    allocator: RequestIdAllocator,
    staleness: StalenessFilter,
    pending_receipts: Vec<CommandReceipt>,
    /// The wire events drained but not yet flushed (bounded by
    /// `events_per_flush` per pump tick).
    event_backlog: Vec<WireEvent>,
    /// The outbound message queue (the ABI drains it per dispatch).
    outbound: std::collections::VecDeque<WorkerToHost>,
    /// The state-transition trail (tests + the lane evidence).
    state_history: Vec<(PlaybackState, PlaybackState)>,
    last_fatal: Option<CommandError>,
    booted: bool,
    /// The reused output-drain buffer (D-1): take → clear → drain →
    /// process → move back; steady state allocates nothing.
    outbox_scratch: Vec<EngineOutput>,
}

impl WebHost {
    /// An idle host (no session) for `profile`.
    #[must_use]
    pub fn new(profile: TargetProfile) -> WebHost {
        WebHost {
            engine: Engine::new(),
            handle: None,
            consumer: None,
            device: None,
            clock: None,
            facts: None,
            source: None,
            tracks: Vec::new(),
            registry: BlobRegistry::new(),
            capabilities: capabilities::capabilities(profile),
            profile,
            budgets: SliceBudgets::default(),
            pump_clock: Arc::new(PumpClock::new()),
            audio_rate: 48_000,
            allocator: RequestIdAllocator::new(),
            staleness: StalenessFilter::new(),
            pending_receipts: Vec::new(),
            event_backlog: Vec::new(),
            outbound: std::collections::VecDeque::new(),
            state_history: Vec::new(),
            last_fatal: None,
            booted: false,
            outbox_scratch: Vec::new(),
        }
    }

    /// The build profile this host runs as.
    #[must_use]
    pub fn profile(&self) -> TargetProfile {
        self.profile
    }

    /// The slice budgets (tests tighten them to prove the bounds).
    pub fn set_budgets(&mut self, budgets: SliceBudgets) {
        self.budgets = budgets;
    }

    /// The slice budgets in force.
    #[must_use]
    pub fn budgets(&self) -> SliceBudgets {
        self.budgets
    }

    /// The observed wire events (flushed AND backlog — the full
    /// causal record, test evidence).
    #[must_use]
    pub fn events(&self) -> Vec<WireEvent> {
        // The flushed events live in the outbound queue; the backlog
        // is pending. Both in order — the queue was appended first.
        let mut events = Vec::new();
        for message in &self.outbound {
            if let WorkerToHost::Events { events: batch } = message {
                events.extend_from_slice(batch);
            }
        }
        events.extend_from_slice(&self.event_backlog);
        events
    }

    /// The state-transition trail.
    #[must_use]
    pub fn state_history(&self) -> &[(PlaybackState, PlaybackState)] {
        &self.state_history
    }

    /// The headless presenter's counters.
    #[must_use]
    pub fn counters(&self) -> ConsumerCounters {
        self.consumer
            .as_ref()
            .map_or_else(ConsumerCounters::default, |consumer| consumer.counters())
    }

    /// The engine's current state (wire form).
    #[must_use]
    pub fn state(&self) -> PlaybackState {
        self.engine.state().as_wire()
    }

    /// The live session id, if any.
    #[must_use]
    pub fn session_id(&self) -> Option<commands::envelope::SessionId> {
        self.engine.session_id()
    }

    /// The live generation.
    #[must_use]
    pub fn generation(&self) -> Option<Generation> {
        self.engine.generation()
    }

    /// The current position (µs) — the sample-counter clock's own
    /// reading (audio-master on this host; deterministic because the
    /// renders are the only time mover).
    #[must_use]
    pub fn position_us(&self) -> i64 {
        match self.clock.as_ref() {
            Some(clock) => clock.now().ticks / 1000,
            None => 0,
        }
    }

    /// Drain every queued outbound message (the ABI's pull).
    #[must_use]
    pub fn take_outbound(&mut self) -> Vec<WorkerToHost> {
        self.outbound.drain(..).collect()
    }

    /// Handle one downstream message (the Worker's whole dispatch).
    pub fn handle(&mut self, message: HostToWorker) {
        // The outbound bound (MP §11): a caller that never drains
        // the output is refused typed — never allowed to grow the
        // queue unboundedly. The port contract drains after every
        // dispatch; reaching the cap means the caller broke it.
        if self.outbound.len() >= crate::abi::MAX_PENDING_MESSAGES {
            self.outbound.clear();
            self.outbound.push_back(WorkerToHost::ControlError {
                code: "web.worker.output_backpressure".to_string(),
                message: format!(
                    "the caller is not draining the output queue (the port contract drains \
                     after every dispatch); pending messages were dropped at the \
                     {}-message bound",
                    crate::abi::MAX_PENDING_MESSAGES
                ),
            });
            return;
        }
        if !self.booted && !matches!(message, HostToWorker::Boot) {
            self.outbound.push_back(WorkerToHost::ControlError {
                code: "web.worker.not_booted".to_string(),
                message: "the worker answers nothing before the boot handshake (send \
                          `{\"type\":\"boot\"}` first)"
                    .to_string(),
            });
            return;
        }
        match message {
            HostToWorker::Boot => {
                self.booted = true;
                self.outbound.push_back(WorkerToHost::Booted {
                    engine_info: engine_info(self.profile),
                    capabilities: capabilities::capabilities(self.profile),
                });
            }
            HostToWorker::LoadBlob { name, bytes } => match self.registry.load(&name, bytes) {
                Ok(size) => {
                    self.outbound
                        .push_back(WorkerToHost::BlobLoaded { name, size });
                }
                Err(error) => {
                    self.outbound.push_back(WorkerToHost::ControlError {
                        code: "web.blob_registry".to_string(),
                        message: error.to_string(),
                    });
                }
            },
            HostToWorker::Command(request) => {
                let response = self.dispatch(&request);
                self.outbound.push_back(WorkerToHost::Response(response));
            }
            HostToWorker::Pump { ticks } => {
                let report = self.pump(ticks);
                self.outbound.push_back(WorkerToHost::Pumped { report });
            }
            HostToWorker::Shutdown => {
                self.shutdown();
            }
        }
    }

    /// Queue one typed control-lane error (the ABI's parse/overflow
    /// answers and the port's own failures — command-lane errors
    /// ride `Response`, this is the control lane's twin).
    pub fn handle_control_error(&mut self, code: &str, message: impl Into<String>) {
        self.outbound.push_back(WorkerToHost::ControlError {
            code: code.to_string(),
            message: message.into(),
        });
    }

    /// The graceful shutdown (doc 09 §7: close sessions, flush
    /// events, acknowledge; the hard terminate deadline is the
    /// page's business, not the worker's).
    fn shutdown(&mut self) {
        if self.engine.session_id().is_some() {
            if let Some(handle) = self.handle.clone() {
                let request = self.allocator.allocate();
                if handle.submit(request, SessionCommandKind::Close).is_ok() {
                    for _ in 0..AWAIT_POLLS {
                        self.poll_engine();
                        if self.engine.state() == SessionState::Idle {
                            break;
                        }
                    }
                }
            }
            self.teardown_session_scoped();
        }
        // Flush the remaining backlog honestly (the graceful-close
        // promise — bounded by what the engine actually emitted).
        if !self.event_backlog.is_empty() {
            let events: Vec<WireEvent> = std::mem::take(&mut self.event_backlog);
            self.outbound.push_back(WorkerToHost::Events { events });
        }
        self.outbound.push_back(WorkerToHost::ShutdownComplete);
    }

    // -- the pump (doc 09 §3.2) ------------------------------------------

    /// One bounded pump slice: `ticks` ticks, each visiting the
    /// roles in priority order (audio fill → engine poll batch →
    /// event flush), every role bounded by [`SliceBudgets`]. The
    /// tick count is capped — one message can never hog the Worker
    /// loop — and the report answers the re-arm truth.
    pub fn pump(&mut self, ticks: u32) -> PumpReport {
        let ticks = ticks.min(self.budgets.max_ticks_per_pump);
        let mut events_flushed = 0_u32;
        for _ in 0..ticks {
            self.fill_audio();
            for _ in 0..self.budgets.polls_per_tick {
                self.poll_engine();
            }
            events_flushed += self.flush_events();
        }
        PumpReport {
            state: self.state(),
            position_us: self.position_us(),
            busy: self.busy(),
            events_flushed,
        }
    }

    /// The audio-fill role (first by priority, doc 09 §3.2): render
    /// whole quanta the ring already holds — each render advances
    /// the sample counter, which is the clock's truth. Skipped
    /// entirely while the session clock is frozen (pause/underrun:
    /// a paused clock means the device pulls nothing). Limb-less
    /// sessions render silence quanta so media time still advances
    /// deterministically. At `Ended`, the residue is played out to
    /// the exact media end (the end-tail — the drain-gate's other
    /// half).
    fn fill_audio(&mut self) {
        if self.device.is_none() || self.consumer.is_none() {
            return;
        }
        if self.clock.as_ref().is_some_and(|clock| clock.is_paused()) {
            return;
        }
        // Snapshot every read the loop needs BEFORE the mutable
        // device borrow (the disjoint-field discipline).
        let channels = self.audio_channels();
        let has_lane = self.lane_exists();
        let ended = self.engine.state() == SessionState::Ended;
        let quanta = self.budgets.audio_quanta_per_tick;
        let rate = self.audio_rate;
        let pump_clock = Arc::clone(&self.pump_clock);
        let Some(device) = self.device.as_mut() else {
            return;
        };
        // One scratch render buffer per fill (D-1): reused across the
        // slice's quanta; the tail render uses a prefix of it.
        let mut scratch = vec![0.0_f32; RENDER_QUANTUM * channels];
        for _ in 0..quanta {
            let buffered = device.buffered_frames();
            let take = if has_lane {
                if ended {
                    if buffered == 0 {
                        break;
                    }
                    buffered.min(RENDER_QUANTUM)
                } else if buffered >= RENDER_QUANTUM {
                    RENDER_QUANTUM
                } else {
                    break;
                }
            } else {
                // No audio lane: the silence clock (see the module
                // docs) — a whole quantum of silence advances the
                // counter, nothing else does.
                RENDER_QUANTUM
            };
            let out = &mut scratch[..take * channels];
            if device.render(out).is_err() {
                break;
            }
            pump_clock.advance_ns(frames_to_ns(take as u64, rate));
        }
    }

    /// Whether the session wired an audio lane (the facts' own
    /// truth: any audio-kind track with geometry wires one — the
    /// open path's rule).
    fn lane_exists(&self) -> bool {
        self.facts.as_ref().is_some_and(|facts| {
            facts
                .facts
                .iter()
                .any(|fact| fact.kind == MediaKind::Audio && fact.audio_geometry().is_some())
        })
    }

    /// The audio limb's channel count (render-buffer sizing).
    fn audio_channels(&self) -> usize {
        match self.facts.as_ref().and_then(|facts| {
            facts
                .facts
                .iter()
                .find(|fact| fact.kind == MediaKind::Audio)
                .and_then(|fact| fact.audio_geometry())
        }) {
            Some((channels, _rate)) => channels as usize,
            None => 2,
        }
    }

    /// One engine poll + the output drain (receipts stashed, events
    /// converted + staleness-filtered into the backlog). The engine
    /// poll itself is the bounded pipeline tick (its own
    /// `TickBudget`): demux ≤ 64 events, decode ≤ 16 packets per
    /// limb — the decode-batch role of doc 09 §3.2.
    fn poll_engine(&mut self) {
        self.engine.poll();
        self.drain_outputs();
    }

    /// Drain the engine's output hub into the host halves (the
    /// reused scratch buffer — D-1).
    fn drain_outputs(&mut self) {
        let mut outputs = std::mem::take(&mut self.outbox_scratch);
        outputs.clear();
        self.engine.drain_outputs_into(&mut outputs);
        for output in outputs.drain(..) {
            match output {
                EngineOutput::Receipt(receipt) => self.pending_receipts.push(receipt),
                EngineOutput::Event(event) => {
                    if event.kind.is_pipeline_scoped() {
                        if let EngineEventKind::StateChanged { from, to } = &event.kind {
                            self.state_history.push((from.as_wire(), to.as_wire()));
                        }
                        if let EngineEventKind::Fatal { error } = &event.kind {
                            self.last_fatal = Some(open::wire_error(error));
                        }
                        if let EngineEventKind::TracksChanged { tracks, selected } = &event.kind {
                            let facts = self.facts.as_ref();
                            self.tracks = tracks
                                .iter()
                                .zip(selected.iter())
                                .map(|(track, &is_selected)| {
                                    enrich_track_info(
                                        playback_engine::track_info_wire(track, is_selected),
                                        facts.and_then(|facts| facts.fact_for(track.id)),
                                    )
                                })
                                .collect();
                        }
                    }
                    if let Some(wire) = event.as_wire_event()
                        && self.staleness.filter_event(&wire)
                    {
                        self.event_backlog.push(wire);
                    }
                }
            }
        }
        self.outbox_scratch = outputs;
    }

    /// The event-flush role (bounded by `events_per_flush` per
    /// tick): one `Events` message per tick, the remainder carried
    /// to the next slice (`busy` stays true while the backlog is
    /// non-empty). Answers the count flushed.
    fn flush_events(&mut self) -> u32 {
        let bound = self.budgets.events_per_flush as usize;
        if self.event_backlog.len() <= bound {
            let events: Vec<WireEvent> = std::mem::take(&mut self.event_backlog);
            let count = events.len() as u32;
            if count > 0 {
                self.outbound.push_back(WorkerToHost::Events { events });
            }
            return count;
        }
        let events: Vec<WireEvent> = self.event_backlog.drain(..bound).collect();
        let count = events.len() as u32;
        self.outbound.push_back(WorkerToHost::Events { events });
        count
    }

    /// The re-arm truth: the host keeps scheduling pump slices while
    /// the engine is in a transient or playing state, while the
    /// play-out tail still holds audio, or while undelivered
    /// outputs remain.
    #[must_use]
    pub fn busy(&self) -> bool {
        match self.state() {
            PlaybackState::Opening | PlaybackState::Buffering | PlaybackState::Playing => true,
            PlaybackState::Ended => self.ring_backlog() > 0 || !self.event_backlog.is_empty(),
            _ => !self.event_backlog.is_empty() || !self.pending_receipts.is_empty(),
        }
    }

    /// Frames the device ring still holds.
    fn ring_backlog(&self) -> usize {
        self.device.as_ref().map_or(0, NullDevice::buffered_frames)
    }

    // -- command routing ---------------------------------------------------

    /// The dispatch entry (doc 05 §5.1's shape for the web host):
    /// correlate, route through the engine, answer the wire
    /// response. The response's `session`/`generation` carry the
    /// context at answering time; `request_id` is always echoed.
    pub fn dispatch(&mut self, request: &WireRequest) -> WireResponse {
        let request_id = request.request_id;
        let result = self.route(&request.command);
        WireResponse {
            request_id,
            session: self.session_id(),
            generation: self.generation().unwrap_or(Generation(0)),
            result,
        }
    }

    /// One catalog command → its outcome (the served table + the
    /// honest typed refusals).
    fn route(&mut self, command: &Command) -> Result<CommandOutcome<CommandResult>, CommandError> {
        match command {
            Command::SessionOpen { source, options } => self.open_session(source, options),
            Command::SessionClose => self
                .submit_and_await(SessionCommandKind::Close)
                .inspect(|_| self.session_closed()),
            Command::PlaybackPlay => self.submit_and_await(SessionCommandKind::Play),
            Command::PlaybackPause => self.submit_and_await(SessionCommandKind::Pause),
            Command::PlaybackTogglePlayPause => {
                let kind = if self.state() == PlaybackState::Playing {
                    SessionCommandKind::Pause
                } else {
                    SessionCommandKind::Play
                };
                self.submit_and_await(kind)
            }
            Command::PlaybackStop => self.submit_and_await(SessionCommandKind::Stop),
            Command::MediaGetInfo => Ok(CommandOutcome::Completed(self.media_info()?)),
            Command::MediaGetCurrentTime => Ok(CommandOutcome::Completed(self.current_time()?)),
            Command::MediaGetDuration => Ok(CommandOutcome::Completed(self.duration()?)),
            Command::TracksList => Ok(CommandOutcome::Completed(CommandResult::TrackList {
                tracks: self.tracks.clone(),
            })),
            Command::SystemGetEngineInfo => {
                Ok(CommandOutcome::Completed(CommandResult::EngineInfo {
                    info: engine_info(self.profile),
                }))
            }
            Command::SystemListCommands => {
                Ok(CommandOutcome::Completed(CommandResult::CommandListings {
                    commands: command_listings(),
                }))
            }
            other => Err(unserved(other)),
        }
    }

    /// The `session.open` path (the web host's source grammar +
    /// the null-lane clock, then the engine's own open).
    fn open_session(
        &mut self,
        source: &commands::catalog::types::SourceSpec,
        options: &commands::catalog::types::OpenOptions,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        if self.engine.session_id().is_some() {
            return Err(CommandError::new(
                commands::Stage::Session,
                commands::ErrorCode::NotReady,
                "a session is already open — close it first (source replacement is card 3.A2)",
            ));
        }
        // v1's honest open-scope refusals: the options the web host
        // cannot honor yet answer typed, naming the owning card
        // (never a silent ignore — doc 05 §5.2).
        if let Some(time) = options.start_time {
            return Err(CommandError::new(
                commands::Stage::Session,
                commands::ErrorCode::UnsupportedForHost,
                format!(
                    "start_time ({}) is not served by the web v1 host — the open-path \
                     skip watermark lands with the H-pillar cards",
                    time.0
                ),
            ));
        }
        if options.start_paused {
            return Err(CommandError::new(
                commands::Stage::Session,
                commands::ErrorCode::UnsupportedForHost,
                "start_paused is not served by the web v1 host (compose open → pause)",
            ));
        }
        if options.rate.is_some() {
            return Err(CommandError::new(
                commands::Stage::Session,
                commands::ErrorCode::UnsupportedForHost,
                "an initial rate is not served by the web v1 host (rate is card 3.A1)",
            ));
        }
        if options.video_track.is_some()
            || options.audio_track.is_some()
            || options.subtitle_track.is_some()
            || options.forced_subtitle_only
        {
            return Err(CommandError::new(
                commands::Stage::Session,
                commands::ErrorCode::UnsupportedForHost,
                "track pre-selection is card 3.A3 (the web v1 host opens the default set)",
            ));
        }
        let opened = open::open(
            source,
            &self.registry,
            &self.capabilities,
            WebOpenOptions {
                no_video: options.no_video,
                no_audio: options.no_audio,
            },
        )?;
        let facts = Arc::new(opened.facts);
        let geometry = facts
            .facts
            .iter()
            .find(|fact| fact.kind == MediaKind::Audio)
            .and_then(|fact| fact.audio_geometry());
        let (lane, device, clock) = open_null_lane(geometry, opened.start_position_ns)?;
        self.audio_rate = geometry.map_or(48_000, |(_channels, rate)| rate);
        let consumer = Arc::new(WebFrameConsumer::new(lane));
        let mut provider = WebDecoderProvider::new(self.profile, Arc::clone(&facts));
        provider.attach_consumer(Arc::clone(&consumer));
        let provider = Arc::new(provider);
        let mut config = SessionConfig::new(
            opened.uri,
            opened.demuxer,
            provider,
            Arc::clone(&consumer) as Arc<dyn FrameConsumer>,
            Arc::clone(&clock),
        );
        config.container = Some(facts.container.clone());
        config.throttle = EventPolicy {
            tick: TimeTickPolicy::default_policy(),
            throttle: Arc::clone(&self.pump_clock) as Arc<dyn MonotonicSource>,
        };
        let handle = self
            .engine
            .open(config)
            .map_err(|error| session_error_wire(&error))?;
        self.handle = Some(handle);
        self.consumer = Some(consumer);
        self.device = Some(device);
        self.clock = Some(clock);
        self.facts = Some(facts);
        self.source = Some(source.clone());
        self.tracks.clear();
        // The bounded settle: catch the synchronous fatals (no
        // decodable track) so they surface in THIS response; deeper
        // settling continues on pump slices (cooperative — doc 09
        // §3.2; the class-A truth arrives as events either way).
        for _ in 0..self.budgets.settle_polls {
            self.poll_engine();
            if self.state() != PlaybackState::Opening {
                break;
            }
        }
        if let Some(error) = self.last_fatal.take() {
            self.teardown_after_fatal();
            return Err(error);
        }
        Ok(CommandOutcome::Accepted {
            session: self.engine.session_id(),
            generation: self.generation().unwrap_or(Generation(0)),
        })
    }

    /// After a fatal open failure the engine sits in `Error` with a
    /// dead session — close it so the host is reusable.
    fn teardown_after_fatal(&mut self) {
        if self.state() == PlaybackState::Error {
            let _ = self.submit_and_await(SessionCommandKind::Close);
        }
        self.teardown_session_scoped();
    }

    /// Clear every session-scoped host half (after a settled Close
    /// or a fatal): later snapshots refuse honestly.
    fn teardown_session_scoped(&mut self) {
        self.handle = None;
        self.consumer = None;
        self.device = None;
        self.clock = None;
        self.facts = None;
        self.source = None;
        self.tracks.clear();
        self.pending_receipts.clear();
    }

    /// A settled Close: drop the session-scoped halves.
    fn session_closed(&mut self) {
        self.teardown_session_scoped();
    }

    /// Submit one session command and await its receipt (bounded
    /// engine polls — a bound, never a block).
    fn submit_and_await(
        &mut self,
        kind: SessionCommandKind,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        let request = self.allocator.allocate();
        let handle = self.handle.as_ref().ok_or_else(|| {
            CommandError::new(
                commands::Stage::Session,
                commands::ErrorCode::NotReady,
                "no session open",
            )
        })?;
        handle
            .submit(request, kind)
            .map_err(|_| session_error_wire(&SessionError::SessionClosed))?;
        self.await_receipt(request)
    }

    /// Await one submitted command's receipt.
    fn await_receipt(
        &mut self,
        request: RequestId,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        for _ in 0..AWAIT_POLLS {
            self.poll_engine();
            if let Some(position) = self
                .pending_receipts
                .iter()
                .position(|receipt| receipt.request == request)
            {
                let receipt = self.pending_receipts.remove(position);
                return self.receipt_outcome(receipt);
            }
        }
        Err(CommandError::new(
            commands::Stage::Control,
            commands::ErrorCode::Internal,
            "the engine never answered the command (bounded wait expired)",
        ))
    }

    /// Translate one receipt into the truthful-UI outcome.
    fn receipt_outcome(
        &self,
        receipt: CommandReceipt,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        match receipt.result {
            ReceiptResult::Accepted {
                session,
                generation,
            } => Ok(CommandOutcome::Accepted {
                session: Some(session),
                generation,
            }),
            ReceiptResult::Completed { session: _ } => {
                Ok(CommandOutcome::Completed(CommandResult::Unit))
            }
            ReceiptResult::Rejected { state, reason } => Err(rejection_wire(state, reason)),
        }
    }

    // -- the class-S snapshots ---------------------------------------------

    /// The `media.get-info` snapshot.
    fn media_info(&self) -> Result<CommandResult, CommandError> {
        let Some(facts) = self.facts.as_ref() else {
            return Err(not_ready());
        };
        Ok(CommandResult::MediaInfo {
            info: MediaInfo {
                source: self.source.clone().ok_or_else(not_ready)?,
                container: Some(facts.container.clone()),
                title: None,
                duration: wire_duration(facts.duration_us()),
                seekable: true,
                is_live: false,
                metadata: std::collections::BTreeMap::new(),
                tracks: self.tracks.clone(),
                chapters: Vec::<Chapter>::new(),
            },
        })
    }

    /// The `media.get-current-time` snapshot.
    fn current_time(&self) -> Result<CommandResult, CommandError> {
        let Some(facts) = self.facts.as_ref() else {
            return Err(not_ready());
        };
        let position_us = self.position_us();
        let duration_us = facts.duration_us();
        let fraction = match duration_us {
            Some(duration) if duration > 0 => {
                position_us.clamp(0, duration) as f32 / duration as f32
            }
            _ => 0.0,
        };
        let buffered = match duration_us {
            Some(duration) => vec![TimeRange {
                start: Timestamp(0),
                end: Timestamp(duration),
            }],
            None => Vec::new(),
        };
        Ok(CommandResult::CurrentTime {
            time: Timestamp(position_us.max(0)),
            position: fraction,
            buffered,
        })
    }

    /// The `media.get-duration` snapshot.
    fn duration(&self) -> Result<CommandResult, CommandError> {
        let Some(facts) = self.facts.as_ref() else {
            return Err(not_ready());
        };
        Ok(CommandResult::Duration {
            duration: wire_duration(facts.duration_us()),
        })
    }
}

/// Frames → nanoseconds at `rate` (saturating, never wrapping).
fn frames_to_ns(frames: u64, rate: u32) -> u64 {
    if rate == 0 {
        return 0;
    }
    u64::try_from((frames as u128).saturating_mul(1_000_000_000_u128) / u128::from(rate))
        .unwrap_or(u64::MAX)
}

/// The null audio lane: the device + counter clock ALWAYS (the
/// wasm-safe time source — no `Instant` anywhere), the processing
/// pipeline only when geometry exists. Limb-less sessions run the
/// silence clock (the module docs).
fn open_null_lane(
    geometry: Option<(u32, u32)>,
    anchor_ns: i64,
) -> Result<(Option<AudioLane>, NullDevice, Arc<MediaClock>), CommandError> {
    let (channels, rate) = geometry.unwrap_or((2, 48_000));
    let format = AudioFormat {
        sample: SampleFormat::F32,
        channels,
        layout: AudioLayout::Interleaved,
        rate,
    };
    let NullStream { sink, device } =
        NullBackend::open_stream(&format, RING_FRAMES, NullConfig { capture: false })
            .map_err(|error| open::wire_error(&error))?;
    // Capture the counter source before the sink halves split.
    let clock_source = sink.clock_source();
    let clock = Arc::new(
        MediaClock::audio_master(
            clock_source,
            MediaTimestamp::new(anchor_ns, NANO_BASE),
            Rate::ONE,
        )
        .map_err(|error| open::wire_error(&error))?,
    );
    let lane = match geometry {
        Some((channels, rate)) => {
            let pipeline = AudioPipeline::new(PipelineSpec {
                rate,
                channels,
                tracks: 1,
            })
            .map_err(|error| open::wire_error(&error))?;
            Some(AudioLane { pipeline, sink })
        }
        None => None,
    };
    Ok((lane, device, clock))
}

/// The honest not-ready refusal.
fn not_ready() -> CommandError {
    CommandError::new(
        commands::Stage::Session,
        commands::ErrorCode::NotReady,
        "no session open",
    )
}

/// The µs extent as the wire duration.
fn wire_duration(duration_us: Option<i64>) -> Duration {
    match duration_us {
        Some(us) => Duration::Known {
            time: Timestamp(us),
        },
        None => Duration::Unknown,
    }
}

/// Fold the open path's harvested facts into one wire track row (the
/// CLI host's own enrichment, mirrored: the L0 [`Track`] carries no
/// codec parameters, but the demuxer's `strf` did).
fn enrich_track_info(
    mut info: commands::catalog::types::TrackInfo,
    fact: Option<&TrackFact>,
) -> commands::catalog::types::TrackInfo {
    let Some(fact) = fact else {
        return info;
    };
    if let Some((width, height)) = fact.dimensions {
        info.video = Some(commands::catalog::types::VideoTrackInfo {
            width,
            height,
            frame_rate: fact.frame_ns.map(|ns| (1.0e9 / ns as f64) as f32),
            bit_depth: None,
            hdr: false,
        });
    }
    if let Some((channels, rate)) = fact.audio_geometry() {
        info.audio = Some(commands::catalog::types::AudioTrackInfo {
            channels: channels.min(u16::MAX as u32) as u16,
            sample_rate: Some(rate),
            bit_depth: None,
            codec_params: None,
        });
    }
    info
}

/// The typed refusal for a catalog command this host does not serve
/// (doc 05 §5.2: capability gaps are typed, not silent).
fn unserved(command: &Command) -> CommandError {
    let id = command_id_string(command);
    let owner = owner_of(&id);
    CommandError::new(
        commands::Stage::Control,
        commands::ErrorCode::UnsupportedForHost,
        format!("`{id}` is not served by this build's web host ({owner})"),
    )
    .with_detail("command", id)
    .with_detail("owner", owner)
}

/// The catalog id of one command, read from the layer's own wire
/// form (error-path only).
fn command_id_string(command: &Command) -> String {
    match serde_json::to_value(command) {
        Ok(value) => value
            .get("cmd")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| "(unnamed)".to_string()),
        Err(_) => "(unserializable)".to_string(),
    }
}

/// The owning subsystem for an unserved id.
fn owner_of(id: &str) -> &'static str {
    if id == "playback.seek" {
        "the seek boundary's host duties (ring discard + generation publish) land with the \
         H-pillar pump integration; the CLI host owns the reference implementation"
    } else if id.starts_with("config.") {
        "web settings persistence (localStorage, doc 09 §9.6) is a later card"
    } else if id.starts_with("playlist.") {
        "the playlist model is a Phase-4 card"
    } else if id.starts_with("audio.") {
        "the AudioWorklet sink is card 7.H3 (the null device is the only output)"
    } else if id.starts_with("video.") {
        "the canvas render surface is card 7.H2"
    } else if id.starts_with("subtitle.") {
        "the subtitle path is a Phase-5 card"
    } else if id.starts_with("tracks.set-") {
        "track switching is card 3.A3"
    } else if id.starts_with("system.plugin") {
        "the plugin registry is a Phase-6 card"
    } else if id.starts_with("playback.set-") || id.starts_with("playback.step-") {
        "rate and frame stepping are card 3.A1"
    } else if id.starts_with("media.") {
        "chapters and their navigation are the chapter card"
    } else {
        "not in the first vertical's served set (see system.list-commands)"
    }
}

/// A rejection's wire form (the CLI host's mapping, mirrored).
fn rejection_wire(state: SessionState, reason: playback_engine::RejectionReason) -> CommandError {
    let stage = commands::Stage::Session;
    let state_name = state.as_wire();
    match reason {
        playback_engine::RejectionReason::NoSession => not_ready(),
        playback_engine::RejectionReason::NotReady => CommandError::new(
            stage,
            commands::ErrorCode::NotReady,
            format!("the session is still opening (state: {state_name:?})"),
        ),
        playback_engine::RejectionReason::SettleFirst => CommandError::new(
            stage,
            commands::ErrorCode::NotReady,
            format!("close from {state_name:?} composes through stop (the executor applies it)"),
        ),
        playback_engine::RejectionReason::AlreadySettled => CommandError::new(
            stage,
            commands::ErrorCode::NotReady,
            format!(
                "the session already settled (state: {state_name:?}); close is the remaining exit"
            ),
        ),
        playback_engine::RejectionReason::RestartNeedsSeek => CommandError::new(
            stage,
            commands::ErrorCode::NotReady,
            format!("play in {state_name:?} means restart-from-ended — the seek flow (card 3.A1)"),
        ),
        playback_engine::RejectionReason::SourceReplacementPhase3 => CommandError::new(
            stage,
            commands::ErrorCode::NotReady,
            "open on a live session is source replacement (card 3.A2)",
        ),
        playback_engine::RejectionReason::SignalNotRaisedHere => CommandError::new(
            commands::Stage::Control,
            commands::ErrorCode::Internal,
            format!(
                "a pipeline signal fired in a state whose driver never raises it ({state_name:?}) \
                 — bug report"
            ),
        ),
        playback_engine::RejectionReason::EndedNeedsWireEdge => CommandError::new(
            stage,
            commands::ErrorCode::NotReady,
            format!(
                "seek in {state_name:?} needs the doc 05 §3.3 ended→buffering wire edge \
                 (Lead-owned change)"
            ),
        ),
        playback_engine::RejectionReason::MissingPayload => CommandError::new(
            commands::Stage::Control,
            commands::ErrorCode::Internal,
            "a seek command arrived without its target payload — caller bug",
        ),
        playback_engine::RejectionReason::TrackNotFound => CommandError::new(
            commands::Stage::Control,
            commands::ErrorCode::NotFound,
            "track selection named a track this session does not have",
        ),
    }
}

/// A session-construction error's wire form (the CLI host's mapping,
/// mirrored).
fn session_error_wire(error: &SessionError) -> CommandError {
    match error {
        SessionError::Config(cause) => CommandError::new(
            commands::Stage::Session,
            commands::ErrorCode::Unsupported,
            cause.to_string(),
        )
        .with_detail("code", "session.no-decodable-track"),
        SessionError::GenerationExhausted(engine) => open::wire_error(engine),
        SessionError::SessionClosed => CommandError::new(
            commands::Stage::Session,
            commands::ErrorCode::Canceled,
            "the session closed before the command landed",
        ),
        SessionError::InboxBusy => CommandError::new(
            commands::Stage::Session,
            commands::ErrorCode::ResourceLimit,
            "the session inbox is full — poll the engine, then retry",
        )
        .with_detail("limit", "inbox-capacity"),
        SessionError::Invariant(detail) => CommandError::new(
            commands::Stage::Control,
            commands::ErrorCode::Internal,
            format!("engine invariant breach: {detail}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::catalog::types::SourceSpec;
    use commands::envelope::RequestId;
    use testkit::{Must, assert_err, fixture_bytes};

    fn request(command: Command) -> WireRequest {
        WireRequest {
            request_id: RequestId(1),
            command,
        }
    }

    /// Open the first-vertical fixture through the REAL open path
    /// (blob registry → probe → AVI wiring → engine).
    fn open_fixture(host: &mut WebHost) {
        // The handshake first (the not-booted lane refuses everything
        // else — the same order the port contract pins).
        host.handle(HostToWorker::Boot);
        let bytes = fixture_bytes("generated/valid/avi_mjpeg_pcm.avi").must();
        host.handle(HostToWorker::LoadBlob {
            name: "fixture".to_string(),
            bytes,
        });
        let response = host.dispatch(&request(Command::SessionOpen {
            source: SourceSpec::Url {
                url: "blob:fixture".to_string(),
            },
            options: Default::default(),
        }));
        match response.result {
            Ok(CommandOutcome::Accepted { .. }) => {}
            other => unreachable!("open must accept: {other:?}"),
        }
    }

    /// The lane proof's core: open → pump slices → the engine plays
    /// the fixture to its end, every observable a pure function of
    /// the pump count (no wall clock anywhere).
    #[test]
    fn pump_slices_play_the_fixture_to_its_end() {
        let mut host = WebHost::new(TargetProfile::Native);
        open_fixture(&mut host);
        // The settle polls moved us past Opening.
        assert_ne!(host.state(), PlaybackState::Opening);
        // Pump slices to the end: bounded iterations, every slice
        // bounded by the budgets.
        let mut reports = Vec::new();
        for _ in 0..64 {
            let report = host.pump(4);
            reports.push(report);
            if !report.busy {
                break;
            }
        }
        let last = reports.last().must();
        assert_eq!(last.state, PlaybackState::Ended, "run to the end");
        assert!(!last.busy, "the re-arm truth releases at the end");
        // The play-out reached the media end: position ≈ duration.
        let facts = host.facts.as_ref().must();
        let duration_us = facts.duration_us().must();
        let position = host.position_us();
        assert!(
            (position - duration_us).abs() < 5_000,
            "position {position} must reach duration {duration_us}"
        );
        // Both limbs delivered: MJPEG frames counted by the headless
        // presenter, PCM audio staged into the ring (and played out).
        let counters = host.counters();
        assert!(counters.video_frames > 0, "video frames were delivered");
        assert!(counters.audio_frames > 0, "audio frames were staged");
        // The state trail crossed Playing.
        assert!(
            host.state_history()
                .iter()
                .any(|(from, _)| *from == PlaybackState::Playing),
            "the engine played: {:?}",
            host.state_history()
        );
    }

    /// The slice budgets are hard bounds: a pump message asking for
    /// an unbounded tick count is capped, and the event flush per
    /// tick is bounded (doc 09 §3.2 — one message can never hog the
    /// Worker loop).
    #[test]
    fn pump_ticks_are_capped_and_flushes_bounded() {
        let mut host = WebHost::new(TargetProfile::Native);
        open_fixture(&mut host);
        let report = host.pump(u32::MAX);
        // events_flushed ≤ max_ticks_per_pump × events_per_flush.
        let budgets = host.budgets();
        let bound = u64::from(budgets.max_ticks_per_pump) * u64::from(budgets.events_per_flush);
        assert!(
            u64::from(report.events_flushed) <= bound,
            "flushed {} > bound {bound}",
            report.events_flushed
        );
    }

    /// The audio fill is bounded per tick: one tick can render at
    /// most `audio_quanta_per_tick` quanta, whatever the ring holds.
    #[test]
    fn audio_fill_is_bounded_per_tick() {
        let mut host = WebHost::new(TargetProfile::Native);
        open_fixture(&mut host);
        // Let decode-ahead fill the ring (bounded polls).
        for _ in 0..8 {
            host.poll_engine();
        }
        let before = host.position_us();
        host.fill_audio();
        let advanced_us = host.position_us() - before;
        let budgets = host.budgets();
        // Multiply first, divide once: the clock's µs come from exact
        // sample counts, so the bound must not pre-truncate per
        // quantum (16 × 33333.33 µs is 533_333, not 16 × 33_333).
        let bound_us =
            (RENDER_QUANTUM as u64) * u64::from(budgets.audio_quanta_per_tick) * 1_000_000
                / u64::from(host.audio_rate);
        let bound_us = i64::try_from(bound_us).unwrap_or(i64::MAX);
        assert!(
            advanced_us <= bound_us,
            "one tick advanced {advanced_us} µs > bound {bound_us} µs"
        );
    }

    /// The honest served surface: seek and config refuse typed with
    /// the owner named; the snapshots refuse not-ready without a
    /// session.
    #[test]
    fn unserved_surface_refuses_typed() {
        let mut host = WebHost::new(TargetProfile::Native);
        let response = host.dispatch(&request(Command::PlaybackSeek {
            target: commands::catalog::types::SeekTarget::Absolute { time: Timestamp(0) },
        }));
        match response.result {
            Err(error) => {
                assert_eq!(
                    error.code,
                    commands::ErrorCode::UnsupportedForHost,
                    "{error}"
                );
                assert_eq!(
                    error.details.get("command").map(String::as_str),
                    Some("playback.seek")
                );
                assert!(error.details.contains_key("owner"), "{error}");
            }
            Ok(_) => unreachable!("seek must refuse typed"),
        }
        let response = host.dispatch(&request(Command::ConfigGetSetting {
            area: "engine".to_string(),
            key: "log-level".to_string(),
        }));
        match response.result {
            Err(error) => assert_eq!(
                error.code,
                commands::ErrorCode::UnsupportedForHost,
                "{error}"
            ),
            Ok(_) => unreachable!("config must refuse typed"),
        }
        let response = host.dispatch(&request(Command::MediaGetInfo));
        match response.result {
            Err(error) => assert_eq!(error.code, commands::ErrorCode::NotReady, "{error}"),
            Ok(_) => unreachable!("info without a session must refuse"),
        }
    }

    /// The class-S snapshots answer over the open fixture.
    #[test]
    fn snapshots_answer_over_the_open_fixture() {
        let mut host = WebHost::new(TargetProfile::Native);
        open_fixture(&mut host);
        for _ in 0..64 {
            if host.pump(4).state == PlaybackState::Ended {
                break;
            }
        }
        match host.dispatch(&request(Command::MediaGetDuration)).result {
            Ok(CommandOutcome::Completed(CommandResult::Duration {
                duration: Duration::Known { time },
            })) => {
                assert!(time.0 > 0, "duration {time:?}");
            }
            other => unreachable!("duration must complete: {other:?}"),
        }
        match host.dispatch(&request(Command::MediaGetCurrentTime)).result {
            Ok(CommandOutcome::Completed(CommandResult::CurrentTime {
                time, position, ..
            })) => {
                assert!(time.0 > 0, "position {time:?}");
                assert!((0.0..=1.0).contains(&position), "fraction {position}");
            }
            other => unreachable!("current-time must complete: {other:?}"),
        }
        match host.dispatch(&request(Command::TracksList)).result {
            Ok(CommandOutcome::Completed(CommandResult::TrackList { tracks })) => {
                assert_eq!(tracks.len(), 2, "one video + one audio");
                assert!(
                    tracks.iter().any(|track| track.video.is_some()),
                    "the video row carries the strf dimensions"
                );
                assert!(
                    tracks.iter().any(|track| track.audio.is_some()),
                    "the audio row carries the strf geometry"
                );
            }
            other => unreachable!("tracks must complete: {other:?}"),
        }
        match host.dispatch(&request(Command::SystemGetEngineInfo)).result {
            Ok(CommandOutcome::Completed(CommandResult::EngineInfo { info })) => {
                assert_eq!(info.host, "web");
                assert!(
                    info.decoders.iter().any(|d| d.name == "mjpeg"),
                    "the native profile's real decoders: {:?}",
                    info.decoders
                );
            }
            other => unreachable!("engine-info must complete: {other:?}"),
        }
    }

    /// Play → pause freezes the position exactly (the paused clock
    /// pulls nothing — the null-device contract).
    #[test]
    fn pause_freezes_the_counter_clock() {
        let mut host = WebHost::new(TargetProfile::Native);
        open_fixture(&mut host);
        for _ in 0..8 {
            host.poll_engine();
            if host.state() == PlaybackState::Playing {
                break;
            }
        }
        assert_eq!(host.state(), PlaybackState::Playing);
        host.fill_audio();
        let frozen = host.position_us();
        match host.dispatch(&request(Command::PlaybackPause)).result {
            Ok(CommandOutcome::Accepted { .. }) => {}
            other => unreachable!("pause must accept: {other:?}"),
        }
        for _ in 0..8 {
            host.pump(2);
        }
        assert_eq!(host.position_us(), frozen, "paused media time freezes");
    }

    /// The wasm32-profile decoder seam refuses typed on a native
    /// build too (the profile is data — the refusal path is
    /// testable without the target).
    #[test]
    fn wasm32_profile_decoder_seam_refuses_typed() {
        let facts = Arc::new(MediaFacts {
            tracks: Vec::new(),
            facts: Vec::new(),
            container: "avi".to_string(),
        });
        let provider = WebDecoderProvider::new(TargetProfile::Wasm32, facts);
        let track = media_types::Track {
            id: TrackId::new(1),
            stream: media_types::StreamId::new(1),
            kind: MediaKind::Video,
            language: None,
            codec: media_types::ids::CodecDescriptor::new("mjpeg").must(),
            title: None,
            flags: media_types::TrackFlags::NONE,
            timing: media_types::TrackTiming {
                span: media_types::TimeSpan::UNKNOWN,
                base: media_types::TimeBase::new(1, 1_000_000).must(),
                gaps: Vec::new(),
            },
        };
        let config = DecoderConfig::new(
            media_types::ids::CodecDescriptor::new("mjpeg").must(),
            media_types::GenerationId::FIRST,
            codec_api::ThreadBudget::default(),
            None,
            None,
        );
        let error = assert_err(provider.decoder_for(&track, &config), "wasm refusal");
        assert_eq!(error.stage, PipelineStage::Decode);
        assert_eq!(error.context.code.as_str(), "web.decoders_absent");
    }

    /// The shutdown path: an open session closes, the host tears
    /// down, the acknowledgment is queued, and later snapshots
    /// refuse not-ready (the honest closed state).
    #[test]
    fn shutdown_closes_and_acknowledges() {
        let mut host = WebHost::new(TargetProfile::Native);
        host.handle(HostToWorker::Boot);
        let _ = host.take_outbound();
        open_fixture(&mut host);
        let _ = host.take_outbound();
        host.handle(HostToWorker::Shutdown);
        let outbound = host.take_outbound();
        assert!(
            outbound.contains(&WorkerToHost::ShutdownComplete),
            "the shutdown acknowledgment is queued: {outbound:?}"
        );
        assert!(host.session_id().is_none(), "the session is closed");
        match host.dispatch(&request(Command::MediaGetDuration)).result {
            Err(error) => assert_eq!(error.code, commands::ErrorCode::NotReady, "{error}"),
            Ok(_) => unreachable!("snapshots after shutdown refuse not-ready"),
        }
    }

    /// Messages before the boot handshake answer the typed
    /// control error (the handshake is required, doc 09 §7).
    #[test]
    fn commands_before_boot_refuse_typed() {
        let mut host = WebHost::new(TargetProfile::Native);
        host.handle(HostToWorker::Command(request(Command::PlaybackPlay)));
        let outbound = host.take_outbound();
        match outbound.first() {
            Some(WorkerToHost::ControlError { code, .. }) => {
                assert_eq!(code, "web.worker.not_booted");
            }
            other => unreachable!("the boot gate must answer typed: {other:?}"),
        }
    }

    /// `frames_to_ns` is exact for the fixture rate and saturating
    /// at the extremes.
    #[test]
    fn frames_to_ns_is_exact_and_saturating() {
        assert_eq!(frames_to_ns(44_100, 44_100), 1_000_000_000);
        assert_eq!(frames_to_ns(0, 44_100), 0);
        assert_eq!(frames_to_ns(u64::MAX, 1), u64::MAX);
    }
}
