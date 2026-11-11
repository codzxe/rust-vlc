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
// The drain-gate's trait surface (compiled wherever its sole
// constructor is — see DrainGatedDecoder's cfg note).
#[cfg(any(not(target_arch = "wasm32"), feature = "builtin-codecs"))]
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

use crate::audio::autoplay::{AutoplayGate, GateChange};
use crate::capabilities::{self, HostCapabilities, TargetProfile};
use crate::open::{self, MediaFacts, TrackFact, WebOpenOptions};
use crate::protocol::{HostToWorker, PumpReport, WorkerToHost};
use crate::render::backend::{RenderBackend, RenderLane};
use crate::render::present::{PresentMailbox, PresentStats, UploadFrame};
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
    /// The latest-wins present mailbox (card 7.H2: decoded video
    /// frames stage here; the pump drains ONE per pump message).
    mailbox: PresentMailbox,
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
                mailbox: PresentMailbox::new(),
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

    /// Take the mailbox's staged frame as a converted RGBA upload
    /// (the latest-wins drain — ONE per pump message, card 7.H2).
    #[must_use]
    pub fn take_present(&self) -> Option<UploadFrame> {
        lock_or_clone(&self.state).mailbox.take()
    }

    /// Whether the mailbox holds a not-yet-presented frame.
    #[must_use]
    pub fn present_pending(&self) -> bool {
        lock_or_clone(&self.state).mailbox.is_pending()
    }

    /// The mailbox's observables (the honest present evidence:
    /// stored / displaced / converted / refused).
    #[must_use]
    pub fn present_stats(&self) -> PresentStats {
        lock_or_clone(&self.state).mailbox.stats()
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
                // Card 7.H2's frame-upload path: stage the frame in
                // the latest-wins mailbox (a refcounted clone — the
                // RGBA conversion runs at take time, so a displaced
                // frame never pays for pixels). A typed upload
                // refusal (oversize / unsupported format) counts in
                // the mailbox's own stats — presentation bounds
                // never fail decode.
                let _ = guard.mailbox.store(&video);
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
/// Compiled only in decoder-less builds — the sole call sites are the
/// `not(feature = "builtin-codecs")` halves of the dispatch cfgs.
#[cfg(not(feature = "builtin-codecs"))]
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
            // The shipped wasm32 module: the honest typed refusal
            // (decoder-less, the capability snapshot's own row).
            // Opt-in `builtin-codecs` feature builds dispatch the
            // same table as native — doc 09 §2.4's per-codec
            // tree-shake lane; the table's cfg keeps both sides of
            // this arm compiling under every feature combination.
            TargetProfile::Wasm32 => {
                #[cfg(feature = "builtin-codecs")]
                {
                    builtin_decoder_for(self.profile, track, config, &self.facts, &self.probe)
                }
                #[cfg(not(feature = "builtin-codecs"))]
                {
                    Err(decoders_absent_error())
                }
            }
            TargetProfile::Native => {
                builtin_decoder_for(self.profile, track, config, &self.facts, &self.probe)
            }
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
/// Native always; wasm32 only in `builtin-codecs` feature builds
/// (the same gate as [`builtin_decoder_for`], its sole constructor):
/// the shipped wasm32 module wires no builtin decoders — the typed
/// refusal — so the wrapper would be dead code there.
#[cfg(any(not(target_arch = "wasm32"), feature = "builtin-codecs"))]
struct DrainGatedDecoder {
    inner: Box<dyn Decoder>,
    probe: Option<Arc<WebFrameConsumer>>,
}

#[cfg(any(not(target_arch = "wasm32"), feature = "builtin-codecs"))]
impl DrainGatedDecoder {
    fn audio_backlog(&self) -> usize {
        self.probe
            .as_ref()
            .map_or(0, |consumer| consumer.buffered_frames())
    }
}

#[cfg(any(not(target_arch = "wasm32"), feature = "builtin-codecs"))]
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

/// The builtin decoder table: the first vertical's MJPEG + PCM rows
/// plus the per-codec opt-in rows, dispatched on the track's registry
/// descriptor (the CLI host's own rows, mirrored — every other
/// descriptor is the honest typed refusal below).
///
/// # Which builds compile this table
///
/// Native builds: always (the codecs edge is unconditional there).
/// Wasm32 builds: only behind the `builtin-codecs` feature — the
/// doc 09 §2.4 per-codec tree-shake lane (card 7.K1). The SHIPPED
/// wasm32 module compiles the stub below instead (decoder-less, the
/// honest capability snapshot); a feature build is an explicitly
/// labeled opt-in/measurement configuration whose size is recorded
/// in docs/web/size-report.md.
///
/// # The opt-in rows' honest runtime posture
///
/// The web open path (AVI, card 7.H4) harvests PCM geometry facts
/// only — [`TrackFact`](crate::open::TrackFact) has no codec-blob
/// seam yet — so the `extra_data`-consuming decoders (flac, vorbis,
/// opus, aac, alac, adpcm) configure with whatever the engine passed
/// and their own configure gates refuse typed on fact-less lanes,
/// exactly like a fact-less native lane (the CLI host's own rule:
/// "the configure gate stays the codec's own honest authority").
/// The rows are still the REAL decoders — symbols, dispatch,
/// decode — not stubs; that is what the tree-shake measurement
/// measures. The AVI blob-harvest seam is later work [PROPOSAL].
#[cfg(any(not(target_arch = "wasm32"), feature = "builtin-codecs"))]
fn builtin_decoder_for(
    profile: TargetProfile,
    track: &Track,
    config: &DecoderConfig,
    facts: &MediaFacts,
    probe: &Option<Arc<WebFrameConsumer>>,
) -> Result<Box<dyn Decoder>, EngineError> {
    // The audio rows wrap in the drain-gate (the pcm row's own rule:
    // a `Drained` answer waits on the device ring actually emptying,
    // so end-of-media fires when the media has PLAYED).
    fn gated(decoder: Box<dyn Decoder>, probe: &Option<Arc<WebFrameConsumer>>) -> Box<dyn Decoder> {
        Box::new(DrainGatedDecoder {
            inner: decoder,
            probe: probe.clone(),
        })
    }
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
            Ok(gated(Box::new(decoder), probe))
        }
        // The in-house ADPCM family (card 5.J2, in-house — rides the
        // `builtin-codecs` in-house set). WAVEFORMAT-blob consumer:
        // refuses typed on fact-less lanes (the module docs above).
        #[cfg(feature = "builtin-codecs")]
        codec if codec.starts_with("adpcm_") => {
            let mut decoder = codecs::audio::adpcm::AdpcmDecoder::new();
            decoder.configure(config)?;
            Ok(gated(Box::new(decoder), probe))
        }
        // The per-codec external-engine rows (card 7.K1's tree-shake
        // lane — one row per opt-in feature; each mirrors the CLI
        // host's own row for the same codec).
        #[cfg(feature = "codec-flac")]
        "flac" => {
            let mut decoder = codecs::audio::flac::FlacDecoder::new();
            decoder.configure(config)?;
            Ok(gated(Box::new(decoder), probe))
        }
        #[cfg(feature = "codec-mp3")]
        "mp3" | "mp1" | "mp2" => {
            // In-band frame headers both layers — no extra_data seam.
            let mut decoder = if track.codec.as_str() == "mp2" {
                Box::new(codecs::audio::mp2::Mp2Decoder::new()) as Box<dyn Decoder>
            } else {
                Box::new(codecs::audio::mp3::Mp3Decoder::new())
            };
            decoder.configure(config)?;
            Ok(gated(decoder, probe))
        }
        #[cfg(feature = "codec-vorbis")]
        "vorbis" => {
            let mut decoder = codecs::audio::vorbis::VorbisDecoder::new();
            decoder.configure(config)?;
            Ok(gated(Box::new(decoder), probe))
        }
        #[cfg(feature = "codec-opus")]
        "opus" => {
            let mut decoder = codecs::audio::opus::OpusDecoder::new();
            decoder.configure(config)?;
            Ok(gated(Box::new(decoder), probe))
        }
        // The ADTS/MP4 descriptor family for the AAC-LC decoder —
        // Main/SSR/LTP descriptors stay unmatched and refuse typed
        // below (the decoder is LC-only by card 5.C2a, honestly).
        #[cfg(feature = "codec-aac")]
        "aac" | "aac-lc" => {
            let mut decoder = codecs::audio::aac::AacDecoder::new();
            decoder.configure(config)?;
            Ok(gated(Box::new(decoder), probe))
        }
        #[cfg(feature = "codec-alac")]
        "alac" => {
            let mut decoder = codecs::audio::alac::AlacDecoder::new();
            decoder.configure(config)?;
            Ok(gated(Box::new(decoder), probe))
        }
        #[cfg(feature = "codec-vp9")]
        "vp9" => {
            // In-band uncompressed frame headers — no extra_data seam.
            let mut decoder = codecs::video::vp9::Vp9Decoder::new();
            decoder.configure(config)?;
            Ok(Box::new(decoder))
        }
        #[cfg(feature = "codec-dnx")]
        "dnxhd" => {
            // Raw VC-3 ES decodes statelessly — no extra_data seam.
            let mut decoder = codecs::video::dnx::DnxDecoder::new();
            decoder.configure(config)?;
            Ok(Box::new(decoder))
        }
        other => Err(crate::open::host_error(
            PipelineStage::Decode,
            "web.codec_unavailable",
            format!(
                "no decoder for `{other}` in this build (landed: {} — this \
                 build's builtin-codecs feature set, see the capability snapshot)",
                capabilities::decoder_descriptors(profile).join(", ")
            ),
        )),
    }
}

/// The wasm32 stub of the builtin table (the shipped, decoder-less
/// module): the profile is always [`TargetProfile::Wasm32`] on that
/// target (`build_profile`'s cfg), so the arm above is compiled out
/// — and the honest answer is the same typed refusal.
#[cfg(all(target_arch = "wasm32", not(feature = "builtin-codecs")))]
fn builtin_decoder_for(
    _profile: TargetProfile,
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
    /// The render lane (card 7.H2): the negotiated backend + the
    /// runtime loss state. Unprobed until the handshake reports
    /// facts.
    render_lane: RenderLane,
    /// The present the pump drained but the ABI has not staged yet
    /// (ONE in flight — the latest-wins drain; the ABI's frame lane
    /// takes it per dispatch).
    pending_present: Option<UploadFrame>,
    /// The autoplay gate (card 7.H3): parked playback freezes the
    /// audio lane — a suspended context pulls nothing.
    autoplay: AutoplayGate,
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
            render_lane: RenderLane::new(),
            pending_present: None,
            autoplay: AutoplayGate::new(),
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

    /// Take the pump's staged present (the ABI's frame-lane pull:
    /// ONE upload in flight, valid until the next dispatch — the
    /// same contract the output JSON staging answers).
    pub fn take_pending_frame(&mut self) -> Option<UploadFrame> {
        self.pending_present.take()
    }

    /// The boot report's capability snapshot: the base report with
    /// the render lane's own truth patched in (card 7.H2: the
    /// ACTIVE backend — or the honest unprobed/unavailable
    /// sentence — never a guess, never an aspiration).
    #[must_use]
    pub fn booted_capabilities(&self) -> HostCapabilities {
        let mut snapshot = self.capabilities.clone();
        snapshot.renderer = match self.render_lane.lane_string() {
            Some(active) => active,
            None => "unprobed (the page did not report renderer facts)".to_string(),
        };
        snapshot
    }

    /// The render lane's current negotiation-shaped snapshot
    /// (tests + diagnostics; `None` before the handshake reported
    /// facts).
    #[must_use]
    pub fn render_snapshot(&self) -> Option<crate::render::RendererNegotiation> {
        self.render_lane.snapshot()
    }

    /// The active render backend, if the lane presents.
    #[must_use]
    pub fn render_backend(&self) -> Option<RenderBackend> {
        self.render_lane.active()
    }

    /// The present mailbox's observables (the honest present
    /// evidence: stored / displaced / converted / refused).
    #[must_use]
    pub fn present_stats(&self) -> PresentStats {
        self.consumer
            .as_deref()
            .map_or_else(PresentStats::default, WebFrameConsumer::present_stats)
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
        if !self.booted && !matches!(message, HostToWorker::Boot { .. }) {
            self.outbound.push_back(WorkerToHost::ControlError {
                code: "web.worker.not_booted".to_string(),
                message: "the worker answers nothing before the boot handshake (send \
                          `{\"type\":\"boot\"}` first)"
                    .to_string(),
            });
            return;
        }
        match message {
            HostToWorker::Boot { renderer } => {
                self.booted = true;
                // Card 7.H2 (doc 09 §4.4): the page's probe facts
                // walk the ladder BEFORE the boot report — the
                // ACTIVE backend is what the snapshot reports, never
                // a guess. No facts = the honest unprobed state.
                if let Some(caps) = renderer {
                    self.render_lane.boot(caps);
                }
                self.outbound.push_back(WorkerToHost::Booted {
                    engine_info: engine_info(self.profile),
                    capabilities: self.booted_capabilities(),
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
            HostToWorker::SurfaceLost { backend, reason } => {
                // The glue's `webglcontextlost` / `device.lost`
                // report (doc 09 §4.4): fall one step down the
                // ladder, surface the explicit transition — a stale
                // report for a backend the lane is not presenting is
                // ignored (answers nothing).
                if let Some(transition) = self.render_lane.surface_lost(backend, reason) {
                    self.outbound
                        .push_back(WorkerToHost::RenderDegraded { transition });
                }
            }
            HostToWorker::AudioContext { running } => {
                // Card 7.H3 (doc 09 §5.3): apply the page's
                // context-state report. The gate's surfacing runs in
                // the pump's observation (both arrival orders fire
                // exactly one event); the resume needs no message —
                // the page drove the gesture, and the freeze-thaw
                // is observable through the position reports.
                let _change = self.autoplay.report(running);
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
    /// loop — and the report answers the re-arm truth. After the
    /// ticks, ONE present drains (card 7.H2: the latest-wins
    /// mailbox → one `PresentFrame` per pump message) and the
    /// autoplay gate observes (card 7.H3: parked-while-playing
    /// surfaces the typed event exactly once).
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
        self.present_one();
        self.observe_autoplay();
        PumpReport {
            state: self.state(),
            position_us: self.position_us(),
            busy: self.busy(),
            events_flushed,
        }
    }

    /// The present role (card 7.H2): drain ONE frame from the
    /// mailbox — only while the render lane presents (the
    /// audio-only degrade stops presenting; decode and counting
    /// continue). The RGBA bytes stage for the ABI's frame lane;
    /// the message carries the metadata.
    fn present_one(&mut self) {
        if self.render_lane.active().is_none() {
            return;
        }
        let Some(consumer) = self.consumer.as_ref() else {
            return;
        };
        if let Some(upload) = consumer.take_present() {
            let message = WorkerToHost::PresentFrame {
                width: upload.width,
                height: upload.height,
                pts_ns: upload.pts_ns,
            };
            // One in flight: an untaken staged present (the port
            // contract violated — the caller did not drain) is
            // displaced by the newer frame (latest-wins, counted by
            // the mailbox's own stats as displaced at store time;
            // this edge is the caller's bug, not a silent drop).
            self.pending_present = Some(upload);
            self.outbound.push_back(message);
        }
    }

    /// The autoplay observation (card 7.H3): parked + playback
    /// wanting audio + not yet surfaced ⇒ the typed event, exactly
    /// once per parking cycle.
    fn observe_autoplay(&mut self) {
        let wants_audio = self.wants_audio();
        if let GateChange::Parked = self.autoplay.observe(wants_audio) {
            self.outbound.push_back(WorkerToHost::GestureRequired);
        }
    }

    /// Whether playback currently wants audio to flow (the autoplay
    /// gate's join condition): a live session in a playing or
    /// transient state with an audio lane wired.
    fn wants_audio(&self) -> bool {
        self.handle.is_some()
            && self.lane_exists()
            && matches!(
                self.state(),
                PlaybackState::Opening | PlaybackState::Buffering | PlaybackState::Playing
            )
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
        // The autoplay park (card 7.H3): a suspended context pulls
        // nothing — the audio lane freezes exactly like pause (the
        // sample-counter clock stops advancing; media time does not
        // move while the surfaced condition says paused-pending-
        // gesture — never a fake playing state). Video-only lanes
        // keep the silence clock (doc 09 §5.3: a video-only media
        // may present frames).
        if self.autoplay.is_parked() && self.lane_exists() {
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
        let engine_busy = match self.state() {
            PlaybackState::Opening | PlaybackState::Buffering | PlaybackState::Playing => true,
            PlaybackState::Ended => self.ring_backlog() > 0 || !self.event_backlog.is_empty(),
            _ => !self.event_backlog.is_empty() || !self.pending_receipts.is_empty(),
        };
        engine_busy || (self.render_lane.active().is_some() && self.present_pending())
    }

    /// Whether the mailbox holds a not-yet-presented frame (the
    /// busy tail that keeps the pump alive until the frame ships).
    #[must_use]
    pub fn present_pending(&self) -> bool {
        self.consumer
            .as_deref()
            .is_some_and(WebFrameConsumer::present_pending)
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
        self.pending_present = None;
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
    use testkit::{Must, fixture_bytes};

    fn request(command: Command) -> WireRequest {
        WireRequest {
            request_id: RequestId(1),
            command,
        }
    }

    /// The WebGL2-floor probe facts (the every-browser report —
    /// BCD-verified 2026-10-19 per doc 09 §4.1).
    const WEBGL2_CAPS: crate::render::RendererCaps = crate::render::RendererCaps {
        webgpu_api: false,
        webgpu_adapter: false,
        webgl2_context: true,
    };

    /// The both-backends probe facts (WebGPU preferred, the floor
    /// probed — the fallback-capable lane).
    const BOTH_CAPS: crate::render::RendererCaps = crate::render::RendererCaps {
        webgpu_api: true,
        webgpu_adapter: true,
        webgl2_context: true,
    };

    /// Open the first-vertical fixture through the REAL open path
    /// (blob registry → probe → AVI wiring → engine), booting with
    /// `renderer` probe facts (the page-probed handshake — card
    /// 7.H2's order: probe, then boot).
    fn open_fixture_with(host: &mut WebHost, renderer: Option<crate::render::RendererCaps>) {
        // The handshake first (the not-booted lane refuses everything
        // else — the same order the port contract pins).
        host.handle(HostToWorker::Boot { renderer });
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

    /// The unprobed-boot open (the legacy/old-glue lane).
    fn open_fixture(host: &mut WebHost) {
        open_fixture_with(host, None);
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
    /// testable without the target). Feature-honest (card 7.K1
    /// tree-shake): the typed `web.decoders_absent` refusal exists
    /// ONLY in decoder-less builds — a wasm32 build that ships the
    /// builtin codecs (the tree-shake flac/mp3 lanes) decodes them,
    /// so under `--all-features` the seam succeeds instead. This test
    /// pins the decoder-less contract; the with-codecs contract is
    /// pinned by the decoder_for dispatch tests.
    #[cfg(not(feature = "builtin-codecs"))]
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
        host.handle(HostToWorker::Boot { renderer: None });
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

    // -- card 7.H2: the render lane (negotiation + present + loss) ---

    /// The boot report's snapshot carries the ACTIVE backend from
    /// the page's probe facts — `webgpu` / `webgl2` / the honest
    /// `unavailable (…)`, or `unprobed (…)` when the page reported
    /// nothing (doc 09 §4.4: never a guess, never an aspiration).
    #[test]
    fn the_boot_snapshot_reports_the_active_backend() {
        let mut host = WebHost::new(TargetProfile::Native);
        host.handle(HostToWorker::Boot {
            renderer: Some(BOTH_CAPS),
        });
        let booted = host
            .take_outbound()
            .into_iter()
            .find_map(|message| match message {
                WorkerToHost::Booted { capabilities, .. } => Some(capabilities),
                _ => None,
            })
            .must();
        assert_eq!(booted.renderer, "webgpu");
        assert_eq!(
            host.render_backend(),
            Some(crate::render::RenderBackend::WebGpu)
        );

        let mut host = WebHost::new(TargetProfile::Native);
        host.handle(HostToWorker::Boot {
            renderer: Some(WEBGL2_CAPS),
        });
        let booted = host
            .take_outbound()
            .into_iter()
            .find_map(|message| match message {
                WorkerToHost::Booted { capabilities, .. } => Some(capabilities),
                _ => None,
            })
            .must();
        assert_eq!(booted.renderer, "webgl2");

        let mut host = WebHost::new(TargetProfile::Native);
        host.handle(HostToWorker::Boot {
            renderer: Some(crate::render::RendererCaps::NONE),
        });
        let booted = host
            .take_outbound()
            .into_iter()
            .find_map(|message| match message {
                WorkerToHost::Booted { capabilities, .. } => Some(capabilities),
                _ => None,
            })
            .must();
        assert!(
            booted.renderer.starts_with("unavailable ("),
            "{}",
            booted.renderer
        );

        let mut host = WebHost::new(TargetProfile::Native);
        host.handle(HostToWorker::Boot { renderer: None });
        let booted = host
            .take_outbound()
            .into_iter()
            .find_map(|message| match message {
                WorkerToHost::Booted { capabilities, .. } => Some(capabilities),
                _ => None,
            })
            .must();
        assert!(
            booted.renderer.starts_with("unprobed ("),
            "{}",
            booted.renderer
        );
    }

    /// The frame-upload path over the REAL fixture (headless-native,
    /// the same message paths the JS glue drives): decoded frames
    /// stage in the latest-wins mailbox and the pump ships ONE
    /// `PresentFrame` per pump message — the metadata matches the
    /// container's declared dimensions, the pts rides along, and an
    /// unprobed lane (old glue) ships nothing (the honest no-present
    /// state, not a crash).
    #[test]
    fn the_present_path_ships_one_frame_per_pump() {
        let mut host = WebHost::new(TargetProfile::Native);
        open_fixture_with(&mut host, Some(WEBGL2_CAPS));
        let dims = host
            .facts
            .as_ref()
            .must()
            .facts
            .iter()
            .find(|fact| fact.kind == MediaKind::Video)
            .and_then(|fact| fact.dimensions)
            .must();
        let mut presents = 0_u64;
        for _ in 0..64 {
            host.pump(4);
            let batch = host.take_outbound();
            let per_pump = batch
                .iter()
                .filter(|message| matches!(message, WorkerToHost::PresentFrame { .. }))
                .count();
            assert!(per_pump <= 1, "ONE frame per pump message, saw {per_pump}");
            for message in batch {
                if let WorkerToHost::PresentFrame {
                    width,
                    height,
                    pts_ns,
                } = message
                {
                    assert_eq!((width, height), dims, "the container's own dimensions");
                    assert!(pts_ns.is_some(), "the fixture carries a pts");
                    presents += 1;
                }
            }
            if host.state() == PlaybackState::Ended && !host.busy() {
                break;
            }
        }
        assert!(presents > 0, "the fixture's frames presented");
        let stats = host.present_stats();
        assert!(stats.converted >= presents);
        assert_eq!(stats.refused, 0, "no upload refusals on the fixture");

        // The unprobed lane (old glue): everything else works, no
        // frame ever ships — the render lane is absent, not broken.
        let mut host = WebHost::new(TargetProfile::Native);
        open_fixture(&mut host);
        for _ in 0..16 {
            host.pump(4);
        }
        let presents = host
            .take_outbound()
            .into_iter()
            .filter(|message| matches!(message, WorkerToHost::PresentFrame { .. }))
            .count();
        assert_eq!(presents, 0, "unprobed lanes present nothing");
    }

    /// The runtime loss ladder end to end (doc 09 §4.4): a WebGPU
    /// loss falls to the WebGL2 floor with the explicit
    /// `RenderDegraded` transition and presents CONTINUE; the floor's
    /// loss is the audio-only degrade — no further presents, playback
    /// (the clock, the counters) continues, and a stale loss report
    /// is ignored (no duplicate transition).
    #[test]
    fn surface_loss_falls_then_degrades_to_audio_only() {
        let mut host = WebHost::new(TargetProfile::Native);
        open_fixture_with(&mut host, Some(BOTH_CAPS));
        let _ = host.take_outbound();
        assert_eq!(
            host.render_backend(),
            Some(crate::render::RenderBackend::WebGpu)
        );
        // The WebGPU device dies EARLY — before the first present
        // (doc 09 §4.4's runtime loss): the explicit step-down to the
        // WebGL2 floor. (Loss late in a ~2 s fixture would leave no
        // media to present on the fallback — the ladder is exercised
        // from the start.)
        host.handle(HostToWorker::SurfaceLost {
            backend: crate::render::RenderBackend::WebGpu,
            reason: crate::render::backend::LossReason::DeviceLost,
        });
        let transitions: Vec<_> = host
            .take_outbound()
            .into_iter()
            .filter_map(|message| match message {
                WorkerToHost::RenderDegraded { transition } => Some(transition),
                _ => None,
            })
            .collect();
        assert_eq!(transitions.len(), 1, "one explicit transition");
        assert_eq!(
            transitions[0].to,
            Some(crate::render::RenderBackend::WebGl2)
        );
        assert!(!transitions[0].is_audio_only());
        assert_eq!(
            host.render_backend(),
            Some(crate::render::RenderBackend::WebGl2)
        );
        // The fallback lane keeps presenting: pump until the first
        // frame ships on the WebGL2 floor (bounded — decode-ahead
        // starts with the session).
        let mut presents = 0_usize;
        for _ in 0..32 {
            host.pump(2);
            presents += host
                .take_outbound()
                .into_iter()
                .filter(|message| matches!(message, WorkerToHost::PresentFrame { .. }))
                .count();
            if presents > 0 {
                break;
            }
        }
        assert!(presents > 0, "the WebGL2 floor presents");
        // A stale WebGPU loss (already consumed) is ignored.
        host.handle(HostToWorker::SurfaceLost {
            backend: crate::render::RenderBackend::WebGpu,
            reason: crate::render::backend::LossReason::DeviceLost,
        });
        assert!(
            host.take_outbound()
                .into_iter()
                .all(|message| !matches!(message, WorkerToHost::RenderDegraded { .. }))
        );
        // The floor dies: the audio-only degrade — mid-playback, so
        // the continue-past-the-loss observables have media to move.
        host.handle(HostToWorker::SurfaceLost {
            backend: crate::render::RenderBackend::WebGl2,
            reason: crate::render::backend::LossReason::ContextLost,
        });
        let transitions: Vec<_> = host
            .take_outbound()
            .into_iter()
            .filter_map(|message| match message {
                WorkerToHost::RenderDegraded { transition } => Some(transition),
                _ => None,
            })
            .collect();
        assert_eq!(transitions.len(), 1);
        assert!(transitions[0].is_audio_only());
        assert_eq!(host.render_backend(), None);
        // Audio-only: playback continues (the clock advances), no
        // frame ever ships again, the video counters keep counting
        // (decode is not bounded by presentation).
        let position_before = host.position_us();
        let mut frames_before = 0;
        if let Some(consumer) = host.consumer.as_ref() {
            frames_before = consumer.counters().video_frames;
        }
        let mut presents_after_loss = 0;
        for _ in 0..64 {
            host.pump(4);
            presents_after_loss += host
                .take_outbound()
                .into_iter()
                .filter(|message| matches!(message, WorkerToHost::PresentFrame { .. }))
                .count();
            if host.state() == PlaybackState::Ended && !host.busy() {
                break;
            }
        }
        assert_eq!(presents_after_loss, 0, "no presents after the total loss");
        assert!(
            host.position_us() > position_before,
            "audio-only playback continues"
        );
        if let Some(consumer) = host.consumer.as_ref() {
            assert!(
                consumer.counters().video_frames >= frames_before,
                "decode continues (counting, not presenting)"
            );
        }
    }

    // -- card 7.H3: the autoplay gate ------------------------------

    /// The autoplay park, end to end over the message paths (doc 09
    /// §5.3): the suspended-context report while playing surfaces the
    /// typed `GestureRequired` event exactly once and FREEZES media
    /// time (a suspended context pulls nothing — the clock stops,
    /// never a fake playing state); the gesture-satisfied report
    /// resumes the flow. Video-only lanes keep the silence clock.
    #[test]
    fn the_autoplay_gate_parks_freezes_and_resumes() {
        let mut host = WebHost::new(TargetProfile::Native);
        open_fixture_with(&mut host, Some(WEBGL2_CAPS));
        let _ = host.take_outbound();
        // The page's play attempt left the context suspended BEFORE
        // any audio flowed (doc 09 §5.3: `playback.play` is accepted,
        // the engine walks its normal pre-play path — the park is the
        // audio lane's, not the engine's). Parking from the start is
        // also what keeps the freeze observable: a ~2 s fixture
        // pumped to its end freezes nothing.
        host.handle(HostToWorker::AudioContext { running: false });
        // The engine plays while the audio lane is parked: the typed
        // event surfaces exactly once and media time NEVER advanced
        // (a suspended context pulls nothing — the clock never moved,
        // never a fake playing state).
        let mut events = 0;
        let mut reached_playing = false;
        for _ in 0..32 {
            host.pump(2);
            let batch = host.take_outbound();
            events += batch
                .iter()
                .filter(|message| matches!(message, WorkerToHost::GestureRequired))
                .count();
            if host.state() == PlaybackState::Playing {
                reached_playing = true;
            }
            if reached_playing && events > 0 {
                break;
            }
        }
        assert!(reached_playing, "the engine's own state machine plays");
        assert_eq!(events, 1, "the typed event, exactly once");
        let frozen = host.position_us();
        assert_eq!(frozen, 0, "parked media time never advanced");
        // More pumps + a duplicate suspended report: still frozen, no
        // re-fire (the once-per-cycle latch holds).
        host.handle(HostToWorker::AudioContext { running: false });
        for _ in 0..8 {
            host.pump(2);
        }
        assert_eq!(host.position_us(), frozen, "parked media time freezes");
        let events = host
            .take_outbound()
            .into_iter()
            .filter(|message| matches!(message, WorkerToHost::GestureRequired))
            .count();
        assert_eq!(events, 0, "duplicate reports never re-fire the event");
        // The gesture: the context resumes, playback truly flows.
        host.handle(HostToWorker::AudioContext { running: true });
        let mut advanced = false;
        for _ in 0..32 {
            host.pump(4);
            if host.position_us() > frozen {
                advanced = true;
                break;
            }
        }
        assert!(advanced, "resumed playback advances media time");
    }
}
