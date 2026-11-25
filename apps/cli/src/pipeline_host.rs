//! The pipeline composition the engine drives (card 2.D2): the
//! decoder provider and the frame consumer that carry the first
//! vertical's audio and video paths.
//!
//! # Decoder provider (the registry shim)
//!
//! [`HostDecoderProvider`] answers the engine's per-track
//! [`DecoderProvider`] seam with the real builtin decoders: MJPEG
//! (2.B4) as-is, and PCM (2.B5) seeded with the `strf` facts the
//! open-path pre-pass harvested ([`PcmDecoder::with_format`] — the
//! typed seam 2.B5 documented for exactly this host shape, because
//! the engine's `Pipeline::wire` passes no WAVEFORMATEX `extra_data`).
//!
//! # Frame consumer (audio + video halves)
//!
//! [`CliFrameConsumer`] implements the engine's [`FrameConsumer`]
//! seam on the poll thread:
//!
//! - **audio**: `AudioPipeline` (2.C3: s16→f32 conversion; the
//!   FX3-A03 master chain — volume/mute/balance/EQ, unity until
//!   controlled; identity map, straight mix) → the output ring of
//!   the backend the factory posture routed (the instrumented null
//!   backend — the headless default; see
//!   [`open_audio_session_for`]). The ring's shared counter is the
//!   session clock's truth source; a sink clocked at another rate
//!   than the track rides the FX3-A04 rate-mismatch resampler at
//!   the same seam.
//! - **video**: an I420→RGBA host conversion (BT.601 full-range, the
//!   JFIF equations — the renderer's v0 upload path is RGBA-only and
//!   the color-conversion card has not landed; 2.D1's worklog named
//!   this exact "converts upstream" slot) → the `PresentationScheduler`
//!   (2.C4) → the bounded `PresentationQueue` → the software `Renderer`
//!   (2.D1) on a headless surface. RGBA-native frames pass through
//!   unconverted. Since w31.1 an optional [`PresentTap`] mirrors every
//!   request the renderer presents (the desktop's webview frame
//!   channel; `None` in the pure CLI — byte-identical behavior).
//!
//! The device half of the null audio session and the scheduler's
//! tick/present driving live in [`crate::host`]'s pump — the consumer
//! only takes delivery; the pump advances time (audio renders move
//! the counter, which moves the clock, which gates video presents).
//!
//! # Determinism
//!
//! Everything here is driven, never scheduled: the pump renders audio
//! quanta and ticks the scheduler in lockstep, so the whole path —
//! decode, mix, ring, counter, clock, scheduler, presents — advances
//! deterministically with zero wall-clock reads on the audio-bearing
//! path. `[PROPOSAL: the render quantum (1470 frames ≈ 33.33 ms at
//! 44.1 kHz) matches the fixture's own audio chunk shape; a
//! device-period setting joins with real backends per ADR-005]`

use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use audio_engine::resample::RateResampler;
use audio_engine::{AudioBuffer, AudioControls, AudioPipeline, PipelineSpec};
use audio_output::null::{NullDevice, NullSink};
use audio_output::{NullBackend, NullConfig, NullStream, cpal_linked};
use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink, ThreadBudget};
use codecs::audio::aac::AacDecoder;
use codecs::audio::adpcm::AdpcmDecoder;
use codecs::audio::alac::AlacDecoder;
use codecs::audio::flac::FlacDecoder;
use codecs::audio::mp2::Mp2Decoder;
use codecs::audio::mp3::Mp3Decoder;
use codecs::audio::opus::OpusDecoder;
use codecs::audio::vorbis::VorbisDecoder;
use codecs::video::av1::Av1Decoder;
use codecs::video::dnx::DnxDecoder;
use codecs::video::h264::H264SwDecoder;
use codecs::video::vp9::Vp9Decoder;
use media_clock::{MediaClock, PlaybackClock};
use media_types::frame::{
    AudioFormat, AudioFrame, AudioLayout, ColorMatrix, ColorRange, FrameStorage, SampleFormat,
    VideoFrame,
};
use media_types::ids::CodecDescriptor;
use media_types::packet::SharedBytes;
use media_types::time::{TimeBase, TimeSpan, Timestamp, TrackTiming};
use media_types::{
    EngineError, ErrorKind, Frame, GenerationId, MediaKind, Packet, PipelineStage, Rate, StreamId,
    Track, TrackFlags, TrackId,
};
use playback_engine::{DecoderProvider, FrameConsumer};
use renderer_wgpu::color::SurfaceFormat;
use renderer_wgpu::params::Extent2D;
use renderer_wgpu::renderer::Renderer;
use renderer_wgpu::screenshot;
use renderer_wgpu::surface::{PresentMode, SurfaceConfig};
use video_engine::scheduler::{PresentAction, PresentationScheduler, PushOutcome, TickOutcome};
use video_engine::sink::{PresentRequest, Presenter};
use video_engine::{EnqueueOutcome, PresentationQueue, SchedulerCfg};

use crate::demux_host::MediaFacts;

/// The audio render quantum (frames per device render — see the module
/// docs' determinism note).
pub const RENDER_QUANTUM: usize = 1470;

/// The drift governor's target ring fill (ns; card 3.B1, W1-18 §3.2's
/// `[PROPOSAL]` figure: 2× the doc 03 §4.8 c jitter margin of 20 ms).
/// The hard acceptance band around it is the governor's own
/// (`DriftGovernor`); the ring's 3 s capacity is the structural
/// ceiling. The composition that arms the governor must pace
/// production at roughly the consumption rate (one quantum per
/// device period — see [`AudioLimb::observe_drift`]).
pub const DRIFT_TARGET_FILL_NS: u64 = 40_000_000;

/// The null output ring's capacity (frames; power of two — the ring's
/// own shape rule). Sizing math (the invariant): one engine poll
/// decodes at most `TickBudget::default().packets_per_limb` (16)
/// audio packets → 16 × 1470 = 23 520 frames pushed between the
/// pump's render passes, which drain to a < 1470 residue — so the
/// Playing steady state needs only ~25k. The hazard is the
/// **rebuffer** cycle: an underrun freezes the session clock (so the
/// pump's renders stop) while `Buffering` keeps decoding into the
/// ring until the queues re-prime (~2–3 pumps at 23 520 each). 131
/// 072 frames (≈3 s at 44.1 kHz, 1 MiB stereo f32) absorbs a full
/// rebuffer cycle with >2× margin; beyond that the typed
/// `audio.ring_full` refusal is the honest bound (a fatal, never a
/// drop). `[PROPOSAL: sizing tied to the default tick budget and the
/// fixture's quantum; a device-period setting joins with real
/// backends per ADR-005]`
pub const RING_FRAMES: usize = 131_072;

/// The decode-park fill threshold (frames): while the device ring
/// holds MORE than this, [`CliFrameConsumer`]'s `ready_for(Audio)`
/// probe answers false and the engine's decode slice parks the audio
/// limb (card 3.B3 — packets stay queued, bounded by the queue caps;
/// decode resumes when the pump has rendered the fill back down).
///
/// Sizing proof (the invariant the threshold must uphold): the gate
/// runs BEFORE a packet pop, so the ring must absorb the parked
/// fill PLUS the worst single-packet decode burst. The packetizer
/// caps a packet at 64 KiB (`wav::MAX_PACKET_BYTES`, the demux's
/// own bound), whose worst-case PCM expansion is 8-bit mono —
/// 65 536 frames. 32 768 (park fill) + 65 536 (burst) = 98 304 <
/// 131 072, with 32 768 frames (≈0.74 s at 44.1 kHz) of margin.
/// Every real lane bursts far smaller (AAC 1024, MP3 1152, the
/// 16-bit PCM packetizers ≤ 32 768).
pub const AUDIO_PARK_FILL: usize = 32_768;

/// A presentation-queue slot count (one per scheduler tick, drained
/// in the same pump step).
#[allow(clippy::empty_loop)]
const QUEUE_CAPACITY: NonZeroUsize = match NonZeroUsize::new(4) {
    Some(value) => value,
    None => loop {}, // provably dead const arm (policy.rs precedent)
};

/// A host-minted engine error: stage + stable dotted code + a human
/// sentence. The sentence rides the `source` chain-root slot (L0's
/// own semantics for I/O roots) so [`crate::demux_host::wire_error`]
/// surfaces it as the command layer's message.
fn host_error(stage: PipelineStage, code: &'static str, message: impl Into<String>) -> EngineError {
    EngineError::new(
        stage,
        ErrorKind::Unsupported { format: None },
        media_types::error::ErrorContext::new(code),
    )
    .with_source(std::io::Error::other(message.into()))
}

// ---------------------------------------------------------------------------
// Decoder provider
// ---------------------------------------------------------------------------

/// The engine's decoder seam, answered with the builtin codecs
/// (the registry shim until the plugin registry card lands).
pub struct HostDecoderProvider {
    facts: Arc<MediaFacts>,
    /// The audio-output backlog probe (set at open time once the
    /// null session exists): gates the PCM limb's `Drained` answer
    /// on the device ring actually emptying, so end-of-media fires
    /// when the media has PLAYED, not when it has been decoded
    /// (headless determinism — `advance` is the only time mover).
    consumer: Option<Arc<CliFrameConsumer>>,
}

impl HostDecoderProvider {
    /// Build over the open path's harvested facts.
    #[must_use]
    pub fn new(facts: Arc<MediaFacts>) -> HostDecoderProvider {
        HostDecoderProvider {
            facts,
            consumer: None,
        }
    }

    /// Attach the frame consumer (the drain-gate probe source). Must
    /// be called before the engine wires limbs; the open path does it
    /// right after constructing the consumer.
    pub fn attach_consumer(&mut self, consumer: Arc<CliFrameConsumer>) {
        self.consumer = Some(consumer);
    }

    /// The config the extra_data-consuming codecs configure with: the
    /// engine's minted config with `extra_data` seeded from the
    /// open-path facts (the engine mints `extra_data: None` — its
    /// `obtain_decoder` has no container knowledge; the esds ASC, the
    /// ALAC magic cookie, the Vorbis three-header chain and the ADPCM
    /// WAVEFORMATEX body live in the container, and the host's facts
    /// pre-pass is the only side that harvested them — §4.3.3 of the
    /// open-path wiring doc). A codec whose facts carry no blob
    /// configures with the engine's `None` unchanged (the honest
    /// `configure` refusal stays the codec's own). PCM keeps its
    /// `with_format` facts seam below, not this one.
    fn seeded_config(&self, track: &Track, config: &DecoderConfig) -> DecoderConfig {
        DecoderConfig {
            extra_data: self
                .facts
                .fact_for(track.id)
                .and_then(|fact| fact.extra_data.clone()),
            ..config.clone()
        }
    }

    /// Build, seed, configure and wrap one `extra_data`-consuming
    /// decoder (the seeded-lane shape every such row below shares):
    /// the `configure` gate stays the codec's own honest authority —
    /// a fact-less lane refuses typed at open — and the
    /// [`ReseedingDecoder`] wrapper carries the blob across the
    /// engine's generation reconfigures (the seek/switch path that
    /// mints `extra_data: None`; the 6.FIX-ogg-reanchor seam).
    fn seeded<D: Decoder + 'static>(
        &self,
        track: &Track,
        config: &DecoderConfig,
        build: impl FnOnce() -> D,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        let seeded = self.seeded_config(track, config);
        let mut decoder = build();
        decoder.configure(&seeded)?;
        Ok(Box::new(ReseedingDecoder::new(
            Box::new(decoder),
            seeded.extra_data,
        )))
    }
}

impl DecoderProvider for HostDecoderProvider {
    fn decoder_for(
        &self,
        track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        match track.codec.as_str() {
            "mjpeg" => {
                let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
                decoder.configure(config)?;
                Ok(Box::new(decoder))
            }
            // The Phase-5 R3 registration rows (card 5.R3-REG): every
            // decoder whose codec card landed in waves 4–7, dispatched
            // on the track's registry descriptor — the decoder's own
            // `configure` gate stays the honest authority (an ALAC
            // stream mislabeled flac refuses typed there, never
            // decodes garbage).
            "flac" => {
                // Requires the container's 34-byte STREAMINFO in
                // `extra_data` (decoder.rs configure refuses to guess
                // geometry without it) — seeded from the facts (the
                // WAV wiring unit's flac-in-wav harvest; the doc §4.3.3
                // one-time seeding rule, applied to the flac row).
                self.seeded(track, config, FlacDecoder::new)
            }
            "mp3" | "mp1" => {
                let mut decoder = Mp3Decoder::new();
                decoder.configure(config)?;
                Ok(Box::new(decoder))
            }
            "mp2" => {
                let mut decoder = Mp2Decoder::new();
                decoder.configure(config)?;
                Ok(Box::new(decoder))
            }
            "vorbis" => {
                // Requires the three-header chain in `extra_data`
                // (decoder.rs:356-372) — seeded from the facts.
                self.seeded(track, config, VorbisDecoder::new)
            }
            "opus" => {
                // Requires the OpusHead ‖ OpusTags chain in
                // `extra_data` (the Ogg host's verbatim join; the
                // decoder's head gate refuses to guess the channel
                // geometry) — seeded from the facts.
                self.seeded(track, config, OpusDecoder::new)
            }
            // The ADTS/MP4 descriptor family for the AAC-LC decoder:
            // `aac` (ISO-BMFF esds) and `aac-lc` (the ADTS profile
            // mapping); Main/SSR/LTP descriptors stay unmatched and
            // refuse typed below — the decoder is LC-only by card
            // 5.C2a, honestly.
            "aac" | "aac-lc" => {
                // ASC mode needs the esds blob; ADTS mode ignores
                // `extra_data` (in-band) — seed unconditionally.
                self.seeded(track, config, AacDecoder::new)
            }
            "alac" => {
                // Requires the ALAC magic cookie in `extra_data`
                // (decoder.rs:620-626) — seeded from the facts.
                self.seeded(track, config, AlacDecoder::new)
            }
            codec if codec.starts_with("adpcm_") => {
                // Parses its WAVEFORMAT (tag/channels/rate/block_align)
                // from `extra_data` (mod.rs:346-358) — seeded from
                // the facts (the WAV `fmt` chunk body).
                self.seeded(track, config, AdpcmDecoder::new)
            }
            "av1" | "av01" => {
                let mut decoder = Av1Decoder::new();
                decoder.configure(config)?;
                Ok(Box::new(decoder))
            }
            // VP9 (card 6.M-vp9, rusty_vp9 =0.1.3 — the admission
            // record's exact pin): in-band uncompressed frame
            // headers, no `extra_data` seeding (the ivf/webm lanes;
            // the ogg/opus seeding precedent does not apply).
            "vp9" => {
                let mut decoder = Vp9Decoder::new();
                decoder.configure(config)?;
                Ok(Box::new(decoder))
            }
            // DNxHD/DNxHR (card 6.M-dnx, filmcraft-dnx =0.9.0
            // default-features OFF — the record's rayon-free pin):
            // raw VC-3 ES decodes statelessly. No container lane
            // mints the `dnxhd` descriptor yet (the mkv V_MS/FOURCC
            // and ASF futures own that) — the codecs-crate
            // differential is the lane; the decoder's `configure`
            // gate is the honest authority (the only accepted
            // descriptor, `dnxhd`).
            "dnxhd" => {
                let mut decoder = DnxDecoder::new();
                decoder.configure(config)?;
                Ok(Box::new(decoder))
            }
            // H.264 (the ADR-001 software track, Lead-admitted
            // 2026-11-15 — `rust_h264` 0.6.1 behind the codec-api
            // contract; admission record
            // docs/dependencies/records/rust_h264-0.6.1.md, differential
            // evidence crates/codecs/tests/h264_sw_differential.rs:
            // bit-exact vs ffmpeg 7.1.5 on six lanes × both entry
            // forms). Requires the container's `avcC` record in
            // `extra_data` for ISO-BMFF tracks (the framing switch);
            // Annex-B `CodecPrivate` and in-band parameter sets
            // configure lazily — seeded from the facts (the mkv/mp4
            // hosts' `config_blob` harvest). The hw lane
            // (`H264Decoder::from_setting`, `playback.hw-decode-policy`)
            // stays the opt-in Phase-7 path: the shipped
            // `software-only` default resolves here, and the decoder's
            // own `configure` gate stays the honest authority.
            "h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),
            codec if codec.starts_with("pcm_") => {
                // The strf facts the open path harvested (2.B5's
                // with_format seam — the engine passes no extra_data).
                let Some((channels, rate)) =
                    self.facts.fact_for(track.id).and_then(|fact| fact.pcm)
                else {
                    return Err(host_error(
                        PipelineStage::Decode,
                        "cli.pcm_facts_absent",
                        format!("no stream facts for PCM track {}", track.id.handle()),
                    ));
                };
                let mut decoder = codecs::audio::pcm::PcmDecoder::with_format(channels, rate)?;
                decoder.configure(config)?;
                // Gate the drain answer on the device ring: the
                // engine's `is_drained` (all limbs drained + queues
                // empty) then waits for the audio to actually play
                // out through the null device — `advance`'s renders,
                // not the decode burst.
                let probe = self.consumer.clone();
                Ok(Box::new(DrainGatedDecoder {
                    inner: Box::new(decoder),
                    probe,
                }))
            }
            other => Err(host_error(
                PipelineStage::Decode,
                "cli.codec_unavailable",
                codec_unavailable_message(other),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatch registry (the one source of truth, FIX-PLAN F0.2)
// ---------------------------------------------------------------------------

/// One row of the decoder dispatch registry: the family name the
/// user-facing surfaces print, the lane's kind, and the descriptors
/// the dispatch's arms accept for the family.
///
/// [`LANDED_DECODER_FAMILIES`] below is the match in
/// [`HostDecoderProvider::decoder_for`] exported as data — the
/// `landed:` refusal string (the `other` arm's message),
/// `engine-info`'s decoder list and `decode-bench`'s coverage note
/// all derive from it, never from a hand-maintained copy beside
/// it. The wave-13 F-003 finding was exactly that drift: the
/// hand-list omitted vp9/dnxhd while both arms lived in this same
/// file, and answered differently from engine-info and bench.
#[derive(Debug, Clone, Copy)]
pub struct DecoderFamily {
    /// The family name user-facing strings print (the refusal
    /// string's `landed:` list is these names, in registry order).
    pub name: &'static str,
    /// The family's lane kind (`engine-info`'s decoder rows derive
    /// name and kind from this row).
    pub kind: MediaKind,
    /// The descriptors the dispatch accepts for the family — the
    /// arm's literal patterns where the arm matches literals
    /// (`"h264" | "avc1"`), a representative descriptor where it
    /// matches a prefix (`pcm_*`, `adpcm_*`). Every listed
    /// descriptor must have a live arm — the `fx2_a03` regression
    /// tests pin this against the match itself.
    pub descriptors: &'static [&'static str],
}

/// The dispatch table's landed decoder families — one row per arm
/// family of [`HostDecoderProvider::decoder_for`], in the arms'
/// top-to-bottom order. THE registry: every user-facing capability
/// list this binary prints derives from this table (the refusal
/// string via [`codec_unavailable_message`]; `engine-info` and
/// `decode-bench` per their own files) — there is no second list.
pub const LANDED_DECODER_FAMILIES: &[DecoderFamily] = &[
    DecoderFamily {
        name: "mjpeg",
        kind: MediaKind::Video,
        descriptors: &["mjpeg"],
    },
    DecoderFamily {
        name: "flac",
        kind: MediaKind::Audio,
        descriptors: &["flac"],
    },
    DecoderFamily {
        name: "mp3",
        kind: MediaKind::Audio,
        descriptors: &["mp3", "mp1"],
    },
    DecoderFamily {
        name: "mp2",
        kind: MediaKind::Audio,
        descriptors: &["mp2"],
    },
    DecoderFamily {
        name: "vorbis",
        kind: MediaKind::Audio,
        descriptors: &["vorbis"],
    },
    DecoderFamily {
        name: "opus",
        kind: MediaKind::Audio,
        descriptors: &["opus"],
    },
    DecoderFamily {
        name: "aac-lc",
        kind: MediaKind::Audio,
        descriptors: &["aac-lc", "aac"],
    },
    DecoderFamily {
        name: "alac",
        kind: MediaKind::Audio,
        descriptors: &["alac"],
    },
    DecoderFamily {
        name: "adpcm",
        kind: MediaKind::Audio,
        descriptors: &["adpcm_ima_wav", "adpcm_ms"],
    },
    DecoderFamily {
        name: "av1",
        kind: MediaKind::Video,
        descriptors: &["av1", "av01"],
    },
    DecoderFamily {
        name: "vp9",
        kind: MediaKind::Video,
        descriptors: &["vp9"],
    },
    DecoderFamily {
        name: "dnxhd",
        kind: MediaKind::Video,
        descriptors: &["dnxhd"],
    },
    DecoderFamily {
        name: "h264",
        kind: MediaKind::Video,
        descriptors: &["h264", "avc1"],
    },
    DecoderFamily {
        name: "pcm",
        kind: MediaKind::Audio,
        descriptors: &["pcm_s16le", "pcm_u8"],
    },
];

/// The catch-all arm's human message — derived from
/// [`LANDED_DECODER_FAMILIES`], never hand-maintained (the wave-13
/// F-003 fix: the hand-list had drifted inside this very file). The
/// shape is unchanged from the hand-list era — one line, the
/// unknown codec in backticks, the landed families comma-joined —
/// so every consumer that matches on the `no decoder for \`X\``
/// prefix keeps working; only the list's content can now move with
/// the arms, which is the point.
#[must_use]
pub fn codec_unavailable_message(codec: &str) -> String {
    format!(
        "no decoder for `{codec}` in this build (landed: {})",
        LANDED_DECODER_FAMILIES
            .iter()
            .map(|row| row.name)
            .collect::<Vec<&str>>()
            .join(", ")
    )
}

/// Whether the dispatch table has an arm accepting `descriptor` —
/// the registry-introspection seam: `engine-info`, `decode-bench`
/// and the `fx2_a03` regression tests ask the match itself, never a
/// list beside it.
///
/// The probe drives one fact-less track through
/// [`HostDecoderProvider::decoder_for`]: an arm whose honest
/// `configure` gate refuses a fact-less lane (flac without its
/// STREAMINFO, pcm without its strf facts) still counts as landed —
/// the gate's typed refusal carries the codec's own error code,
/// not this registry's `cli.codec_unavailable` catch-all. Only the
/// catch-all itself answers `false`.
#[must_use]
pub fn descriptor_is_landed(descriptor: &str) -> bool {
    let Ok(codec) = CodecDescriptor::new(descriptor) else {
        // A malformed descriptor cannot be dispatched by any arm.
        return false;
    };
    let provider = HostDecoderProvider::new(Arc::new(MediaFacts {
        tracks: Vec::new(),
        facts: Vec::new(),
        container: String::new(),
    }));
    let config = DecoderConfig::new(
        codec.clone(),
        GenerationId::FIRST,
        ThreadBudget::default(),
        None,
        None,
    );
    match provider.decoder_for(&probe_track(codec), &config) {
        Ok(_) => true,
        Err(err) => err.context.code.as_str() != "cli.codec_unavailable",
    }
}

/// The minimal fact-less probe track (the dispatch's match reads
/// the codec descriptor only; the timing/flags fields are inert
/// filler the registry never consults).
fn probe_track(codec: CodecDescriptor) -> Track {
    Track {
        id: TrackId::new(0),
        stream: StreamId::new(0),
        kind: MediaKind::Video,
        language: None,
        codec,
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: TimeSpan::UNKNOWN,
            base: TimeBase { num: 1, den: 1000 },
            gaps: Vec::new(),
        },
    }
}

/// A decoder that carries its open-path `extra_data` seeding across
/// the engine's generation reconfigures (the 6.FIX-ogg-reanchor
/// seam).
///
/// The codec-api contract calls [`Decoder::configure`] "again, on a
/// fresh generation, after seeks" — and the engine honors it by
/// minting a fresh [`DecoderConfig`] at every such reconfigure
/// (`Pipeline::seek_flush`, the decoder-cache recycle in
/// `obtain_decoder`). But the engine has **no container
/// knowledge**: the config it mints carries `extra_data: None` (the
/// documented design — the esds ASC, the Vorbis three-header chain,
/// the OpusHead ‖ OpusTags join and the ALAC/ADPCM cookies live in
/// the container, and the host's facts pre-pass is the only side
/// that harvested them; §4.3.3 of the open-path wiring doc).
/// Unwrapped, the in-session seek reconfigures an already-seeded
/// decoder into a header-less generation and the codec's honest
/// `configure` gate refuses typed — the measured Wave-4 defect: ogg
/// 0047 reanchor, 0.2 ms to `opus.head.missing` (vorbis twin:
/// `vorbis.headers.missing`), while the `--start-time` open path
/// (which builds its decoder through the provider, seeded) lands.
///
/// The wrapper retains exactly the blob the open path used — the
/// [`TrackFact`](crate::demux_host::TrackFact) harvest, which
/// survives the seek (the facts are the session's own, not the
/// demuxer cursor's) — and re-seeds it into every `configure` that
/// arrives without one. Precedence: an incoming config that carries
/// its own blob wins (the caller's seeding is more specific than the
/// retained default); a fact-less lane retains `None` and stays a
/// pure passthrough (the codec's own open-time refusal is the
/// honest gate, unchanged).
struct ReseedingDecoder {
    inner: Box<dyn Decoder>,
    /// The open-path seeding (the fact's `extra_data`, verbatim).
    extra: Option<SharedBytes>,
}

impl ReseedingDecoder {
    /// Wrap `inner`, retaining `extra` as its generation-reconfigure
    /// seeding.
    fn new(inner: Box<dyn Decoder>, extra: Option<SharedBytes>) -> ReseedingDecoder {
        ReseedingDecoder { inner, extra }
    }

    /// The config a reconfigure actually sees: the incoming one,
    /// with the retained open-path blob re-seeded when the caller
    /// minted it header-less.
    fn effective(&self, cfg: &DecoderConfig) -> DecoderConfig {
        match (&cfg.extra_data, &self.extra) {
            (None, Some(blob)) => DecoderConfig {
                extra_data: Some(blob.clone()),
                ..cfg.clone()
            },
            _ => cfg.clone(),
        }
    }
}

impl Decoder for ReseedingDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        // The engine's generation bump arrives header-less; the
        // retained open-path blob is the same seeding the open used.
        let config = self.effective(cfg);
        self.inner.configure(&config)
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        self.inner.decode(packet, out)
    }

    fn flush(&mut self, out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        self.inner.flush(out)
    }
}

/// A decoder whose `Drained` answer is gated on the audio-output
/// ring actually emptying. The engine's end-of-media condition is
/// "every limb drained + queues empty"; without this gate a short
/// media decodes faster than it plays and the session ends while
/// the device still holds seconds of unrendered audio. With it,
/// `advance` (the null device's render driver) is the only time
/// mover — the headless session stays `Playing` until the media has
/// genuinely played out.
struct DrainGatedDecoder {
    inner: Box<dyn Decoder>,
    probe: Option<Arc<CliFrameConsumer>>,
}

impl DrainGatedDecoder {
    /// The device backlog this decoder's drain answer waits on
    /// (frames the ring still holds). No consumer / no audio limb
    /// (video-only or `--no-audio`) answers 0 — no gating.
    fn audio_backlog(&self) -> usize {
        self.probe
            .as_ref()
            .map(|consumer| {
                consumer
                    .lock()
                    .audio
                    .as_ref()
                    .map_or(0, AudioLimb::buffered_frames)
            })
            .unwrap_or(0)
    }
}

impl Decoder for DrainGatedDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        self.inner.configure(cfg)
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        self.inner.decode(packet, out)
    }

    fn flush(&mut self, out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        match self.inner.flush(out)? {
            DecodeOutcome::Drained if self.audio_backlog() >= RENDER_QUANTUM => {
                // Everything is decoded and sitting in the device
                // ring: not drained until the device has played it.
                // `NeedMoreInput` keeps the limb's flush loop polling
                // (bounded per tick); the pump's whole-quantum renders
                // drain the ring down and the next flush answers
                // `Drained`.
                //
                // The sub-quantum residue is the end-tail play-out's,
                // not the render loop's (6.FIX-pcm): a ring holding
                // fewer than [`RENDER_QUANTUM`] frames can never be
                // drained by `advance`'s `>= quantum` render gate, so
                // demanding `backlog == 0` here deadlocks every media
                // whose frame count is not a multiple of the quantum
                // (measured: 32 000 frames at 8 kHz stranded 1 130 —
                // no terminal, position frozen at 96.5%). Releasing
                // the gate below one quantum lets `Ended` fire, and
                // the host's end-tail play-out (gated on `Ended`)
                // renders the residue to the exact media end — the
                // two halves of the handshake the gate was splitting.
                // Mid-media the question never arises: the engine
                // only calls `flush` at end-of-stream (and across a
                // seek's residue purge, where the answer changes
                // nothing — the generation check drops the residue
                // either way).
                Ok(DecodeOutcome::NeedMoreInput)
            }
            outcome => Ok(outcome),
        }
    }
}

// ---------------------------------------------------------------------------
// Frame consumer
// ---------------------------------------------------------------------------

/// The audio half: the processing pipeline plus the null sink.
pub struct AudioLimb {
    pipeline: AudioPipeline,
    sink: NullSink,
    /// The negotiated rate (the counter→ns arithmetic for the drift
    /// window's injected time stamp — captured once, never re-derived
    /// per quantum).
    rate_hz: u32,
    /// The session's live master chain (FX3-A03's [`AudioControls`]):
    /// volume, mute, balance and the 3-band EQ, run over the mixed
    /// quantum at the seam below — the same stage order and the same
    /// position FX3-A03 placed it (between the mixer and the device
    /// adaptation). Identity until a control arrives, so the
    /// uncontrolled lane keeps [`AudioLimb::stage`]'s write path
    /// byte-identical. This is the `&mut` hand-off the pipeline's own
    /// surface declines — see [`AudioLimb::audio_controls`] for the
    /// one gate that moves to process time with it.
    controls: AudioControls,
    /// The post-mix seam's machinery: the rate conversion (when the
    /// output clock runs at another rate than the track — see
    /// [`open_audio_session_for`]) plus the reused planes/bytes both
    /// seams share (D-1 — after the first quantum nothing allocates).
    seam: PostMixSeam,
}

/// The post-mix seam's scratch (see the limb's `seam` field).
struct PostMixSeam {
    /// The track-rate → sink-rate converter (`None` at equal rates —
    /// the null fallback's headless default).
    resampler: Option<RateResampler>,
    /// The deinterleaved working planes (reused per quantum — the
    /// master chain's and the resampler's shared shape).
    planes: Vec<Vec<f32>>,
    /// The interleaved output bytes (reused per quantum).
    bytes: Vec<u8>,
}

impl AudioLimb {
    /// The sink half's clock source (the device counter).
    #[must_use]
    pub fn clock_source(&self) -> Arc<dyn media_clock::SampleCounterSource> {
        self.sink.clock_source()
    }

    /// Frames the device has not played yet (the drain gate's truth:
    /// end-of-media waits until this reaches zero).
    #[must_use]
    pub fn buffered_frames(&self) -> usize {
        self.sink.buffered_frames()
    }

    /// The decode-park probe (card 3.B3): `false` while the device
    /// ring holds more than [`AUDIO_PARK_FILL`] frames — the engine's
    /// decode slice parks the audio limb, packets stay queued, and
    /// the pump's renders bring the fill back under the threshold.
    /// This is the fix for the GUI-session `audio.ring_full` fatal
    /// (a settle loop that polls decode without advancing time fills
    /// the 131 072-frame ring in ~8 polls on an AAC lane): parking
    /// bounds the ring by construction instead of hoping the caller
    /// paces production — the same posture as a full packet queue
    /// parking the demuxer.
    #[must_use]
    pub fn ring_ready(&self) -> bool {
        self.sink.buffered_frames() <= AUDIO_PARK_FILL
    }

    /// Cumulative frames pushed into the ring (the write-side twin of
    /// the device counter's consumed count — the seek boundary's
    /// exact-prefix arithmetic, 6.FIX-ogg-reanchor).
    #[must_use]
    pub fn written_frames(&self) -> u64 {
        self.sink.written_frames()
    }

    /// One per-quantum drift observation (card 3.B1's loop closure —
    /// W1-18 §7 step 8): sample the sink's measured latency, stamp it
    /// with the device counter's nanoseconds, and feed the armed
    /// governor, which re-evaluates the correction at its ≤1 Hz window
    /// and applies it to the micro-resampler. An unarmed pipeline
    /// ignores the sample (correction stays `ZERO`); an unmeasurable
    /// sink parks the governor at zero — honest degradation, never a
    /// fabricated estimate.
    ///
    /// **Regime note (the arming contract).** The error signal — ring
    /// fill − target — is a drift estimate only when production paces
    /// at roughly the consumption rate (steady state: one quantum
    /// staged per device period). The first vertical's decode-ahead
    /// regime (16-packet bursts into a 3 s ring, drained per pump)
    /// makes the fill a *decode-ahead depth* metric instead; arming
    /// the governor there would "correct" buffering as if it were
    /// clock drift. The governor is therefore armed by compositions
    /// that pace (the S8 convergence harness, and the production pump
    /// once 3.B3's backpressure lands steady-state pacing).
    ///
    /// Zero heap traffic (D-1/D-8): integer arithmetic only; the
    /// counter read is the same atomic the clock itself reads.
    pub fn observe_drift(&mut self) -> media_clock::DriftCorrection {
        if self.rate_hz == 0 {
            return media_clock::DriftCorrection::ZERO;
        }
        let count = u128::from(self.sink.sample_count());
        let now_ns = (count * 1_000_000_000_u128 / u128::from(self.rate_hz)) as u64;
        let fill_ns = match audio_engine::AudioSink::latency(&self.sink) {
            Some(media_types::time::Duration::Known(nanos)) => Some(nanos.0),
            _ => None,
        };
        self.pipeline.observe_drift(now_ns, fill_ns)
    }

    /// A new timing regime (a seek's re-anchor): restart the drift
    /// measurement and the resampler's stream phase (the S7 transient
    /// is one step-limiter increment at most).
    pub fn reset_drift(&mut self) {
        self.pipeline.reset_drift();
    }

    /// Arm the drift governor (the late-arming composition path —
    /// [`AudioLimb::observe_drift`]'s regime note): the production
    /// open path leaves the limb unarmed; a composition that paces
    /// production at the consumption rate arms with
    /// [`crate::pipeline_host::DRIFT_TARGET_FILL_NS`] (or its own
    /// target).
    pub fn arm_drift_governor(&mut self, target_ns: u64) {
        self.pipeline.arm_drift_governor(target_ns);
    }

    /// The correction in force (the micro-resampler's current ratio —
    /// always within `±1000 ppm`, the limiter invariant).
    #[must_use]
    pub fn drift(&self) -> media_clock::DriftCorrection {
        self.pipeline.drift()
    }

    /// Stage + mix + write one decoded audio frame into the ring (the
    /// [`FrameConsumer`] path's per-frame step — also the composition
    /// root's staging seam for paced producers, e.g. the 3.B1 sync
    /// harness). The micro-resampler's correction applies inside the
    /// mix. The uncontrolled equal-rate lane writes the mixed bytes
    /// verbatim; a controlled chain (FX3-A03's master chain) or a sink
    /// clocked at another rate than the track (FX3-A04's
    /// [`RateResampler`]) rides the post-mix seam instead — the
    /// controls first, then the rate conversion, exactly the order
    /// the engine places them between the mixer and the device.
    ///
    /// # Errors
    /// The pipeline's typed errors (format mismatch, mix shape), the
    /// seam's (the master chain's process-time gate, the whole-frame
    /// guard) and the sink's (ring full) — all already stage-tagged.
    pub fn stage(&mut self, frame: &AudioFrame) -> Result<(), EngineError> {
        self.pipeline.process(0, frame)?;
        self.pipeline.mix()?;
        if let Some(buffer) = self.pipeline.take_output() {
            if self.controls.is_identity() && self.seam.resampler.is_none() {
                // The uncontrolled equal-rate lane: the mixed bytes
                // ride to the sink verbatim (no deinterleave, no
                // second buffer — bit-identical to the pre-FX4 stage).
                self.sink.write_bytes(buffer.data.as_slice())?;
            } else {
                Self::write_through_seam(
                    &mut self.sink,
                    &mut self.controls,
                    &mut self.seam,
                    &buffer,
                )?;
            }
        }
        Ok(())
    }

    /// The live master chain — the FX3-A03 surface the host verbs
    /// route (FX4-A02's `audio.set-volume`/`set-mute` through
    /// [`ConsumerState::audio_mut`]): a `&mut` hand-off onto the
    /// session's own chain, set between quanta and applied at the
    /// next mixed quantum.
    ///
    /// **The one gate that moves with the raw hand-off:** the
    /// pipeline's own surface pre-validates balance's stereo shape at
    /// set time; this raw handle cannot (the chain does not know the
    /// session's channel count), so a mono session's `set_balance`
    /// takes the `Ok` and the gate fires at process time instead — a
    /// typed `audio.effects_balance_channels` from the next `stage`,
    /// never a silent mislabel (the hazard the engine's read-only
    /// `controls()` doc records is answered by surfacing it loudly).
    pub fn audio_controls(&mut self) -> &mut AudioControls {
        &mut self.controls
    }

    /// [`AudioLimb::stage`]'s post-mix seam: deinterleave the mixed
    /// quantum into the reused planes, run the live master chain,
    /// cross the rate grids when the sink is clocked differently,
    /// interleave into the reused byte scratch, write to the sink.
    /// (A free-standing arm — the sink, the controls and the seam are
    /// field-disjoint halves of the limb, so passing all three is the
    /// borrow shape that keeps the single `stage` body.)
    ///
    /// # Errors
    /// The master chain's own process-time gate (the balance stereo
    /// shape — see [`AudioLimb::audio_controls`]),
    /// `audio.partial_frame` (the mixed buffer is not whole frames —
    /// the same gate [`NullSink::write_bytes`] applies on the
    /// equal-rate path) and the sink's own write errors.
    fn write_through_seam(
        sink: &mut NullSink,
        controls: &mut AudioControls,
        seam: &mut PostMixSeam,
        buffer: &AudioBuffer,
    ) -> Result<(), EngineError> {
        let data = buffer.data.as_slice();
        let frame_bytes = buffer.format.frame_bytes() as usize;
        if !data.len().is_multiple_of(frame_bytes) {
            return Err(host_error(
                PipelineStage::AudioOutput,
                "audio.partial_frame",
                format!(
                    "the post-mix seam writes whole frames: {len} bytes against a \
                     {frame_bytes}-byte frame",
                    len = data.len()
                ),
            ));
        }
        let channels = buffer.format.channels as usize;
        let frames = data.len() / frame_bytes;
        seam.planes.resize(channels, Vec::new());
        for plane in &mut seam.planes {
            plane.resize(frames, 0.0);
        }
        for (i, bytes) in data.as_chunks::<4>().0.iter().enumerate() {
            seam.planes[i % channels][i / channels] = f32::from_le_bytes(*bytes);
        }
        // The live master chain (FX3-A03's position: the mixed quantum
        // between the mixer and the device adaptation).
        controls.process(&mut seam.planes)?;
        // The rate conversion onto the sink's grid, when it differs
        // (FX3-A04 — after the controls, the same order the engine
        // places stage D).
        let (planes, out_frames) = match seam.resampler.as_mut() {
            Some(resampler) => {
                let out_frames = resampler.process(&seam.planes, frames);
                (resampler.planes(), out_frames)
            }
            None => (seam.planes.as_slice(), frames),
        };
        seam.bytes.clear();
        seam.bytes.reserve(out_frames * frame_bytes);
        for frame in 0..out_frames {
            for plane in planes {
                seam.bytes.extend_from_slice(&plane[frame].to_le_bytes());
            }
        }
        sink.write_bytes(&seam.bytes)
    }

    /// The governor's polled observability (counters only — the
    /// no-event-storm policy; all-zero when no governor is armed).
    #[must_use]
    pub fn drift_stats(&self) -> audio_engine::DriftStats {
        self.pipeline.drift_stats()
    }
}

/// The presented-frame tap (w31.1, lane `luf1-channel`): the additive
/// seam through which a host that wants the frames (the desktop's
/// webview) mirrors every frame the renderer presents — WITHOUT the
/// pure CLI ever constructing one (the slot defaults to `None` and
/// the CLI binary constructs no tap, so its behavior stays
/// byte-identical with this seam in place).
///
/// The tap is called on the pump thread, immediately after the
/// renderer took the request (success path only — a stale or failed
/// present never taps), with the scheduler's own repeat bit so the
/// honest `PresentAction` entry is available (fresh present vs
/// `RepeatLast`). Implementations must not block the pump (the
/// reference desktop tap is the `CanvasBridge`→`FrameMailbox` path,
/// bounded latest-wins — `desktop::frame_tap::FrameTap`); failures
/// are the presenter's typed errors and ride the existing degrade
/// path (audio continues — doc 03 §4.10 d).
pub trait PresentTap: Send + Sync {
    /// Mirror one successfully presented request (`repeat` set for
    /// the scheduler's `PresentAction::RepeatLast`).
    ///
    /// # Errors
    /// The implementor's typed failure (surfaced by the pump like any
    /// present-path error; audio continues).
    fn tap_present(&self, request: &PresentRequest, repeat: bool) -> Result<(), EngineError>;
}

/// The video half: scheduler + queue + renderer.
pub struct VideoLimb {
    scheduler: PresentationScheduler,
    queue: PresentationQueue,
    renderer: Renderer,
    /// The session clock — read for the live generation at latch
    /// reads (a seek mints a new generation before the clock
    /// publishes it; a latched slot of an unpublished generation is
    /// stale, and the anchor is the truth until the next frame
    /// presents).
    clock: Arc<MediaClock>,
    /// The presentation latch (F3, issue #16): `(generation, slot
    /// ns)` of the last request the renderer took. The deterministic
    /// position truth for an audio-less session — its clock is the
    /// system monotonic, so `now()` there extrapolates wall elapsed,
    /// never a media fact.
    last_presented: Option<(GenerationId, i64)>,
    /// Frames handed to the renderer (presents + repeats).
    pub presented: u64,
    /// Frames the scheduler refused (window full / unschedulable) —
    /// observable, never silent.
    pub refused: u64,
}

/// A [`Presenter`] seam that records the last request the renderer
/// took — the latch above is written exactly when a frame really
/// reaches the screen, through the queue's drain (batches included)
/// and the direct-render path alike (F3, issue #16). The optional
/// [`PresentTap`] rides the same two paths: every request the
/// renderer took is also mirrored to the tap, after the latch (the
/// latch is the position truth; a tap failure must not unwrite it).
struct LatchingPresenter<'a> {
    renderer: &'a mut Renderer,
    latch: &'a mut Option<(GenerationId, i64)>,
    /// The presented-frame tap (w31.1) — `None` in the pure CLI.
    tap: Option<&'a Arc<dyn PresentTap>>,
    /// This tick's `PresentAction` repeat bit. The queue holds at
    /// most the tick's own request in this wiring (one push per tick
    /// plus a full drain), so the action's bit describes every
    /// drained request; a future wiring that batches mixed actions
    /// must ride the bit per-request instead `[PROPOSAL]`.
    repeat: bool,
}

impl Presenter for LatchingPresenter<'_> {
    fn present(
        &mut self,
        request: &PresentRequest,
    ) -> Result<video_engine::sink::PresentInfo, EngineError> {
        let info = self.renderer.present(request)?;
        *self.latch = Some((request.generation, request.when.ticks));
        if let Some(tap) = self.tap {
            tap.tap_present(request, self.repeat)?;
        }
        Ok(info)
    }
}

impl VideoLimb {
    /// One scheduler tick: enqueue the action's request and drain the
    /// queue to the renderer, mirroring what presented to the tap
    /// (w31.1 — `None` in the pure CLI: nothing is mirrored).
    ///
    /// # Errors
    /// The present path's typed errors (surface loss, upload refusal)
    /// — the pump surfaces them and degrades honestly.
    fn tick_once(&mut self, tap: Option<&Arc<dyn PresentTap>>) -> Result<(), EngineError> {
        let TickOutcome { action, .. } = self.scheduler.tick();
        match action {
            PresentAction::Present(request) => self.enqueue_and_drain(request, false, tap),
            PresentAction::RepeatLast(request) => self.enqueue_and_drain(request, true, tap),
            PresentAction::Background => Ok(()),
        }
    }

    fn enqueue_and_drain(
        &mut self,
        request: PresentRequest,
        repeat: bool,
        tap: Option<&Arc<dyn PresentTap>>,
    ) -> Result<(), EngineError> {
        match self.queue.try_push(request) {
            EnqueueOutcome::Enqueued => {}
            EnqueueOutcome::Full { request, .. } => {
                // One slot per tick with spare capacity: unreachable
                // in this wiring; honored honestly by presenting the
                // refused request directly rather than dropping it.
                return self.render_request(request, tap, repeat);
            }
            EnqueueOutcome::Stale { request, .. } => {
                // The generation moved between tick and drain (a
                // stop/seek landed): the request fell behind the
                // published generation — dropped at the boundary and
                // counted by the queue, never presented stale (and
                // never tapped: the webview must not see it either).
                let _ = request;
                return Ok(());
            }
        }
        let (renderer, latch, queue) = (
            &mut self.renderer,
            &mut self.last_presented,
            &mut self.queue,
        );
        let delivered = match queue.drain_to(&mut LatchingPresenter {
            renderer,
            latch,
            tap,
            repeat,
        }) {
            Ok(count) => count,
            Err(error) => {
                // The delivered-before-failure count is the queue's own
                // truth (its stats also carry it); the presenter's error
                // propagates typed.
                self.presented = self.presented.saturating_add(error.delivered as u64);
                return Err(error.source);
            }
        };
        self.presented = self.presented.saturating_add(delivered as u64);
        Ok(())
    }

    fn render_request(
        &mut self,
        request: PresentRequest,
        tap: Option<&Arc<dyn PresentTap>>,
        repeat: bool,
    ) -> Result<(), EngineError> {
        self.renderer.present(&request)?;
        self.last_presented = Some((request.generation, request.when.ticks));
        self.presented = self.presented.saturating_add(1);
        if let Some(tap) = tap {
            tap.tap_present(&request, repeat)?;
        }
        Ok(())
    }

    /// Push one held frame into the scheduler (the hold-retry path —
    /// no conversion: the hold stores already-converted frames).
    fn scheduler_push_for_hold(&mut self, frame: VideoFrame) -> HoldRetry {
        match self.scheduler.push(frame) {
            PushOutcome::Scheduled | PushOutcome::Inferred { .. } => HoldRetry::Pushed,
            PushOutcome::Held | PushOutcome::MissingDropped { .. } => HoldRetry::Pushed,
            PushOutcome::WindowFull { .. } => HoldRetry::Refused,
            PushOutcome::UnschedulablePts { .. } | PushOutcome::Stale { .. } => HoldRetry::Dropped,
        }
    }

    /// The seek boundary's presentation-queue publish (doc 03 §6
    /// step 3, “clear renderer mailbox”, the half behind this host's
    /// seams): every resident present-request of the old generation
    /// is dropped wholesale and counted, and later pushes of it are
    /// refused at the boundary (doc 03 §5(7) — the mailbox is
    /// replaced, not filtered). The scheduler window self-purges at
    /// its own generation snapshot (the clock carries the mint after
    /// step 6), so this is the only presentation-side boundary the
    /// host must move.
    fn seek_boundary(&mut self, generation: GenerationId) {
        let _ = self.queue.publish(generation);
    }

    /// The scheduler's lifetime counters.
    #[must_use]
    pub fn scheduler_stats(&self) -> video_engine::SchedulerStats {
        self.scheduler.stats()
    }

    /// The last presented frame's slot, in nanoseconds — the
    /// deterministic position latch for an audio-less session
    /// (F3, issue #16). `None` when nothing has presented under the
    /// **live** (clock-published) generation yet — nothing ever
    /// presented, or a seek just minted a new generation and the
    /// pre-seek latch is stale (the anchor is the truth until the
    /// next frame presents).
    #[must_use]
    pub fn last_presented_ns(&self) -> Option<i64> {
        let (generation, slot_ns) = self.last_presented?;
        (generation == self.clock.snapshot().generation).then_some(slot_ns)
    }

    /// The renderer's staging measurements (frames, bytes, copies).
    #[must_use]
    pub fn staging_stats(&self) -> renderer_wgpu::staging::StagingStats {
        self.renderer.staging_stats()
    }

    /// The presentation queue's lifetime counters (the seek-boundary
    /// observability: `flushed` counts the generation-publish drops,
    /// `published` is the live boundary).
    #[must_use]
    pub fn queue_stats(&self) -> video_engine::QueueStats {
        self.queue.stats()
    }

    /// The presentation queue's published generation (the live
    /// boundary the seek duties move).
    #[must_use]
    pub fn published_generation(&self) -> GenerationId {
        self.queue.published()
    }

    /// Encode the current surface as a PNG screenshot (2.D1's
    /// byte-stable encoder) — the headless frame evidence.
    ///
    /// # Errors
    /// The renderer's typed errors (no surface, readback shape).
    pub fn screenshot_png(&self) -> Result<Vec<u8>, EngineError> {
        let rgba = self
            .renderer
            .readback_rgba()
            .map_err(|err| err.to_engine_error())?;
        let extent = self
            .renderer
            .extent()
            .map_err(|err| err.to_engine_error())?;
        screenshot::encode_rgba(extent, &rgba).map_err(|err| err.to_engine_error())
    }
}

/// The window-full hold bound. [PROPOSAL: v0 CLI value — 64 frames
/// ≈ 2.1 s at 30 fps ≈ 19 MiB of RGBA at 320×240; large enough for
/// the fixture burst, small enough to bound memory honestly. The
/// principled fix — engine-level decode backpressure from the
/// presentation window — is Phase 3 work (the 3.B queue/sync cards).]
pub const MAX_VIDEO_HOLD: usize = 64;

/// One hold-retry attempt's outcome.
enum HoldRetry {
    /// Admitted to the scheduler window (or held as missing-stamp).
    Pushed,
    /// The window is still full: keep holding, retry after the next
    /// tick.
    Refused,
    /// Stale or unschedulable: the frame can never be scheduled —
    /// dropped and counted.
    Dropped,
}

/// The consumer's mutable state (the engine calls `accept` on the poll
/// thread; the pump ticks the video half between polls — one mutex,
/// no contention in the single-writer model).
pub struct ConsumerState {
    audio: Option<AudioLimb>,
    video: Option<VideoLimb>,
    /// Window-full overflow hold (FIFO of already-converted frames).
    /// The decode stage runs ahead of the clock by design (the
    /// headless pump only moves time on `advance`); the scheduler
    /// window is bounded (16), so a decode burst would otherwise
    /// drop frames that are merely early, not late. The hold is
    /// retried after every scheduler tick — the window drains as
    /// the clock advances, the hold refills it in order. Bounded by
    /// [`MAX_VIDEO_HOLD`]; beyond it the refusal is the honest
    /// drop-and-count (the pre-hold behavior).
    video_hold: std::collections::VecDeque<VideoFrame>,
    /// A present-path failure (renderer/queue). Surfaced by the pump;
    /// audio continues (the 2.D1 degrade pattern).
    video_error: Option<EngineError>,
    /// Frames that reached `accept`.
    pub frames_accepted: u64,
    /// Frames the scheduler refused (window full / unschedulable pts).
    pub frames_refused: u64,
    /// The last audio frame generation staged into the ring (the
    /// change-point bookkeeping for the boundary's exact flush).
    audio_seen_gen: GenerationId,
    /// The ring's cumulative written-frame counter at the last audio
    /// generation change (the first new-generation frame's write
    /// position — the old-prefix length in write order).
    audio_written_at_change: u64,
    /// The audio generation floor (armed at each accepted seek's
    /// boundary): frames older than the mint are accept-window
    /// stragglers — dropped and counted, never staged.
    audio_floor: GenerationId,
    /// Audio frames dropped by the floor gate (observable honesty —
    /// the boundary's own semantics, like the scheduler's stale
    /// drops; never an error).
    pub audio_stale_dropped: u64,
    /// The presented-frame tap (w31.1, lane `luf1-channel`): `None`
    /// in the pure CLI (the inert default — the CLI binary constructs
    /// no tap, byte-identical behavior); the desktop shell installs
    /// its bridge→mailbox tap so the webview's frames ride the same
    /// presents the renderer takes. Read per scheduler tick, so an
    /// install at ANY session state takes effect on the next pump.
    present_tap: Option<Arc<dyn PresentTap>>,
}

impl ConsumerState {
    /// The video half (for the pump's ticking and the AC's accounting).
    #[must_use]
    pub fn video(&self) -> Option<&VideoLimb> {
        self.video.as_ref()
    }

    /// The audio half, mutable — the live-control seam: the host's
    /// audio verbs reach the limb's master chain through the
    /// consumer's locked state ([`AudioLimb::audio_controls`] is the
    /// handle they take from here — FX4-A02's `audio_limb_controls`
    /// is the one call site).
    pub fn audio_mut(&mut self) -> Option<&mut AudioLimb> {
        self.audio.as_mut()
    }

    /// The audio half (for the pump's accounting).
    #[must_use]
    pub fn audio(&self) -> Option<&AudioLimb> {
        self.audio.as_ref()
    }

    /// Tick the video scheduler once (the pump's per-quantum step),
    /// then retry the window-full hold: the tick presented (or
    /// backgrounded) and the window may have room for the frames the
    /// decode burst stashed. The presented-frame tap (w31.1) rides
    /// the tick — every request the renderer took is mirrored to it
    /// (`None` in the pure CLI: nothing is mirrored).
    ///
    /// # Errors
    /// The present path's typed errors (recorded, not swallowed).
    pub fn tick_video(&mut self) -> Result<(), EngineError> {
        // Split borrows: the tap rides BESIDE the limb, not inside it
        // (the limb is built lazily from the frames themselves, so a
        // tap installed after the limb exists needs no rebuild).
        let (video, tap) = (&mut self.video, &mut self.present_tap);
        if let Some(video) = video.as_mut() {
            video.tick_once(tap.as_ref())?;
        }
        self.retry_video_hold();
        Ok(())
    }

    /// Push held frames into the scheduler window, in order, until it
    /// refuses again (or the hold empties). Stale/unschedulable
    /// outcomes drop-and-count honestly — those frames can never be
    /// scheduled.
    fn retry_video_hold(&mut self) {
        while let Some(front) = self.video_hold.front().cloned() {
            let outcome = match self.video.as_mut() {
                Some(video) => video.scheduler_push_for_hold(front),
                None => break,
            };
            match outcome {
                HoldRetry::Pushed => {
                    self.video_hold.pop_front();
                }
                HoldRetry::Refused => break,
                HoldRetry::Dropped => {
                    self.video_hold.pop_front();
                    if let Some(video) = self.video.as_mut() {
                        video.refused = video.refused.saturating_add(1);
                    }
                    self.frames_refused = self.frames_refused.saturating_add(1);
                }
            }
        }
    }

    /// Record a present-path failure (surfaced once by the pump).
    pub fn note_video_error(&mut self, error: EngineError) {
        if self.video_error.is_none() {
            self.video_error = Some(error);
        }
    }

    /// The pump's per-quantum drift observation on the audio limb
    /// (card 3.B1's loop closure — one call per rendered quantum, the
    /// [`AudioLimb::observe_drift`] contract). No audio limb: a no-op.
    pub fn observe_audio_drift(&mut self) {
        if let Some(audio) = self.audio.as_mut() {
            audio.observe_drift();
        }
    }

    /// The audio limb's drift-governor counters (polled observability
    /// — the no-event-storm policy; all-zero when no governor is
    /// armed).
    #[must_use]
    pub fn audio_drift_stats(&self) -> audio_engine::DriftStats {
        self.audio
            .as_ref()
            .map_or(audio_engine::DriftStats::default(), AudioLimb::drift_stats)
    }

    /// The seek boundary's host duties on the consumer state (doc 03
    /// §6 step 3 — the halves behind the [`FrameConsumer`] seam the
    /// engine's pipeline flush cannot reach): drop the window-full
    /// hold's PRE-MINT frames (generation-filtered, not cleared: the
    /// accept poll's decode may already have stashed the landing's
    /// own frames behind the old ones — FIFO — and those must
    /// survive; presenting an old-generation frame after the landing
    /// would be the stale leak D-4 forbids) and publish the
    /// presentation queue's generation (the renderer-mailbox clear).
    /// The engine flushed the packet queues, the held demux packet
    /// and the decoder reorder buffers in the same poll that accepted
    /// the seek; this completes the boundary on the presentation
    /// side. Cold path — once per accepted seek, never per tick.
    ///
    /// The audio twin (6.FIX-ogg-reanchor): arming the audio
    /// generation floor — frames older than the mint arriving after
    /// the boundary are stragglers from the accept window's in-flight
    /// decode, dropped and counted, never staged into the ring (the
    /// same contract the presentation queue's publish enforces on
    /// the video side; the ring's own exact-prefix flush is computed
    /// by [`Self::audio_stale_prefix`] before this runs).
    pub fn seek_boundary(&mut self, generation: GenerationId) {
        self.video_hold
            .retain(|frame| frame.generation >= generation);
        if let Some(video) = self.video.as_mut() {
            video.seek_boundary(generation);
        }
        // The audio drift measurement is a new regime post-seek (the
        // fill-to-target relationship restarts at the landing; the
        // scheduler's estimator equally resets at the anchor-epoch
        // bump). The correction drops to zero with the measurement.
        if let Some(audio) = self.audio.as_mut() {
            audio.reset_drift();
        }
        self.audio_floor = generation;
    }

    /// The ring's exact old-generation prefix, in frames — the count
    /// the seek boundary's flush must discard (6.FIX-ogg-reanchor).
    ///
    /// The arithmetic (write-order truth, never a submit-time
    /// snapshot): `consumed` is the device counter's frames (every
    /// render + every prior discard — all old-generation, the ring
    /// being FIFO and the flush not yet run); the old-prefix length
    /// in WRITE order is the ring's cumulative `written` count at the
    /// first new-generation frame's arrival when the landing's own
    /// decode has already staged (the fast-completion accept), or the
    /// whole cumulative `written` when it has not (everything
    /// resident is old). The difference is the ring's live
    /// old-generation content exactly — including the frames the
    /// accept window's in-flight decode staged between the seek's
    /// submission and its accept, which a submit-time snapshot
    /// misses (the measured overrun: a vorbis lane's 1.2 s seek
    /// landing left ~1.0 s of pre-seek audio playing out past the
    /// media end, the clock at 2.54 s of a 1.5 s file).
    #[must_use]
    pub fn audio_stale_prefix(&self, mint: GenerationId, consumed: u64) -> u64 {
        match self.audio.as_ref() {
            Some(audio) => {
                let written_old = if self.audio_seen_gen >= mint {
                    // The landing's own frames already staged: the
                    // change-point counter is the exact old-prefix
                    // write position.
                    self.audio_written_at_change
                } else {
                    // Nothing new staged yet: everything written is
                    // the old generation's.
                    audio.written_frames()
                };
                written_old.saturating_sub(consumed)
            }
            None => 0,
        }
    }
}

/// The engine's sink seam: decoded frames enter the audio/video paths
/// here, on the poll thread.
pub struct CliFrameConsumer {
    state: Mutex<ConsumerState>,
    facts: Arc<MediaFacts>,
    clock: Arc<MediaClock>,
}

impl CliFrameConsumer {
    /// Build the consumer with its audio half pre-wired (the open path
    /// constructs the null session before the engine exists, because
    /// the session clock needs the counter). The video half is built
    /// lazily on the first decoded video frame (the surface extent
    /// comes from the frames themselves).
    #[must_use]
    pub fn new(
        audio: Option<AudioLimb>,
        facts: Arc<MediaFacts>,
        clock: Arc<MediaClock>,
    ) -> CliFrameConsumer {
        CliFrameConsumer {
            state: Mutex::new(ConsumerState {
                audio,
                video: None,
                video_hold: std::collections::VecDeque::new(),
                video_error: None,
                frames_accepted: 0,
                frames_refused: 0,
                audio_seen_gen: GenerationId::FIRST,
                audio_written_at_change: 0,
                audio_floor: GenerationId::FIRST,
                audio_stale_dropped: 0,
                present_tap: None,
            }),
            facts,
            clock,
        }
    }

    /// Lock the consumer state (the pump's access path). A poisoned
    /// lock means a poll-thread panic already happened — the state is
    /// recovered, not trusted-blindly (std's own recovery semantics).
    pub fn lock(&self) -> std::sync::MutexGuard<'_, ConsumerState> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Install (or replace) the presented-frame tap (w31.1). Safe at
    /// any session state: the tap is read per scheduler tick, so a
    /// live video limb picks it up on the next pump — the desktop
    /// shell binds its webview frame channel mid-playback this way.
    pub fn set_present_tap(&self, tap: Arc<dyn PresentTap>) {
        self.lock().present_tap = Some(tap);
    }

    /// A present-path failure, once (surfaced by the pump).
    #[must_use]
    pub fn take_video_error(&self) -> Option<EngineError> {
        self.lock().video_error.take()
    }

    /// The presentation latch (F3, issue #16): the last frame the
    /// video scheduler presented under the live generation, in
    /// nanoseconds — `None` while nothing has presented. The
    /// deterministic position truth for audio-less sessions (their
    /// clock is the system monotonic; `now()` there is wall-elapsed,
    /// never a media fact).
    #[must_use]
    pub fn last_presented_ns(&self) -> Option<i64> {
        self.lock()
            .video
            .as_ref()
            .and_then(VideoLimb::last_presented_ns)
    }
}

impl FrameConsumer for CliFrameConsumer {
    fn accept(&self, frame: Frame, track: TrackId) -> Result<(), EngineError> {
        let mut state = self.lock();
        state.frames_accepted = state.frames_accepted.saturating_add(1);
        match frame {
            Frame::Audio(audio) => accept_audio(&mut state, &audio),
            Frame::Video(video) => {
                accept_video(&mut state, &video, track, &self.facts, &self.clock)
            }
            // Subtitle/data limbs are not wired in the first vertical.
            Frame::Subtitle(_) => Ok(()),
        }
    }

    fn ready_for(&self, kind: MediaKind) -> bool {
        match kind {
            MediaKind::Audio => self.lock().audio.as_ref().is_none_or(AudioLimb::ring_ready),
            // Video's boundedness is the scheduler window's (the
            // window-full hold already backpressures by refusing and
            // holding) — always ready.
            _ => true,
        }
    }
}

/// Stage + mix + write one audio frame into the null ring.
fn accept_audio(state: &mut ConsumerState, frame: &AudioFrame) -> Result<(), EngineError> {
    let Some(audio) = state.audio.as_mut() else {
        // The engine only wires limbs the wrapper's track list named;
        // an audio frame with no audio limb means the composition is
        // inconsistent — the honest typed refusal, never a drop.
        return Err(host_error(
            PipelineStage::Mix,
            "cli.audio_limb_absent",
            "an audio frame arrived with no audio limb wired",
        ));
    };
    // The boundary's straggler gate (6.FIX-ogg-reanchor): a frame
    // older than the armed floor is an accept-window in-flight
    // decode completing after the flush — dropped and counted, never
    // staged (it would land BEHIND the discarded prefix and play as
    // old-position audio past the landing). Checked before the
    // change-point bookkeeping so a straggler cannot move it.
    if frame.generation < state.audio_floor {
        state.audio_stale_dropped = state.audio_stale_dropped.saturating_add(1);
        return Ok(());
    }
    if frame.generation != state.audio_seen_gen {
        // The generation changed: record the ring's cumulative write
        // position at the change (BEFORE staging this frame — the
        // first new-generation frame's own writes belong to the new
        // prefix). The boundary's exact flush reads this as the
        // old-prefix length in write order.
        state.audio_seen_gen = frame.generation;
        state.audio_written_at_change = audio.written_frames();
    }
    audio.stage(frame)
}

/// Convert (when needed) and schedule one video frame.
fn accept_video(
    state: &mut ConsumerState,
    frame: &VideoFrame,
    track: TrackId,
    facts: &Arc<MediaFacts>,
    clock: &Arc<MediaClock>,
) -> Result<(), EngineError> {
    if state.video.is_none() {
        let frame_ns = facts
            .fact_for(track)
            .and_then(|fact| fact.frame_ns)
            .unwrap_or(33_333_333);
        let cfg = match SchedulerCfg::for_frame_duration(media_types::Nanoseconds(frame_ns)) {
            Ok(cfg) => cfg,
            Err(err) => {
                return Err(host_error(
                    PipelineStage::Present,
                    "cli.scheduler_cfg",
                    err.to_string(),
                ));
            }
        };
        let extent = Extent2D {
            width: frame.size.0,
            height: frame.size.1,
        };
        let renderer = Renderer::new_headless(
            extent,
            SurfaceConfig {
                format: SurfaceFormat::Bgra8Srgb,
                present_mode: PresentMode::Fifo,
            },
        )
        .map_err(|err| err.to_engine_error())?;
        let queue = PresentationQueue::with_generation(QUEUE_CAPACITY, frame.generation);
        state.video = Some(VideoLimb {
            scheduler: PresentationScheduler::new(
                cfg,
                Arc::clone(clock) as Arc<dyn media_clock::PlaybackClock>,
            ),
            queue,
            renderer,
            clock: Arc::clone(clock),
            last_presented: None,
            presented: 0,
            refused: 0,
        });
    }
    let Some(video) = state.video.as_mut() else {
        return Ok(());
    };
    // Convert to the renderer's upload vocabulary when needed.
    let uploadable = match frame.format {
        media_types::PixelFormat::Rgba8 | media_types::PixelFormat::Bgra8 => frame.clone(),
        media_types::PixelFormat::I420 => convert_i420_to_rgba(frame)?,
        media_types::PixelFormat::Nv12 | media_types::PixelFormat::Yuyv422 => {
            return Err(host_error(
                PipelineStage::Present,
                "cli.pixel_format_unconverted",
                format!(
                    "pixel format {:?} needs the color-conversion card (first vertical: I420 + RGBA)",
                    frame.format
                ),
            ));
        }
    };
    match video.scheduler.push(uploadable) {
        PushOutcome::Scheduled | PushOutcome::Inferred { .. } => Ok(()),
        PushOutcome::Held | PushOutcome::MissingDropped { .. } => Ok(()),
        PushOutcome::WindowFull { .. } if state.video_hold.len() < MAX_VIDEO_HOLD => {
            // Early, not late: hold the frame and retry after the next
            // scheduler tick (the pump moves time on `advance`; the
            // window drains as the clock advances).
            //
            // [KNOWN DEFECT, registered as the W1-06 follow-up card
            // (Lead decision required): this arm stashes the
            // UNCONVERTED original (`frame.clone()`), so every
            // hold-retried I420 frame is refused at the renderer
            // (`render.upload.format-unsupported`) and the surface
            // freezes on an early window frame. Stashing the CONVERTED
            // frame (the outcome hands it back) is the obvious fix —
            // W1-06 proved it — but it EXPOSES a second, load-bearing
            // defect: `advance_time`'s render loop drains the whole
            // ring per pump, so with the renderer accepting frames one
            // `advance` tick plays the ENTIRE 2.000 s fixture (the
            // mid-media pause/stop scenarios become unreachable, and a
            // longer media would fatal `ring_full` under a
            // one-quantum-per-pump bound — the decode side has no
            // backpressure from the ring yet, doc 03 §8 / 3.B). Main's
            // pause/stop pacing is an artifact of this renderer
            // refusal breaking the render loop early. The complete fix
            // needs the Lead's pacing/backpressure decision; until
            // then this keeps main's exact behavior (D-5: no test was
            // weakened — the scenarios keep their bug-masked shape and
            // the defect is on the register).]
            state.video_hold.push_back(frame.clone());
            Ok(())
        }
        PushOutcome::WindowFull { .. } | PushOutcome::UnschedulablePts { .. } => {
            video.refused = video.refused.saturating_add(1);
            state.frames_refused = state.frames_refused.saturating_add(1);
            Ok(())
        }
        PushOutcome::Stale { .. } => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// I420 → RGBA (the frame's own tagged colorimetry)
// ---------------------------------------------------------------------------

/// One (matrix, range) pair's YCbCr→RGB coefficient set, Q16 fixed
/// point: `R = (ys·(Y−yo) + rCr·Cr + 2^15) >> 16`,
/// `G = (ys·(Y−yo) − gCb·Cb − gCr·Cr + 2^15) >> 16`,
/// `B = (ys·(Y−yo) + bCb·Cb + 2^15) >> 16` (chroma centered −128).
struct YuvToRgb {
    /// Luma scale (1.0 full-range, ≈1.164383 studio).
    y_scale: i32,
    /// Luma offset (0 full-range, 16 studio).
    y_offset: i32,
    r_cr: i32,
    g_cb: i32,
    g_cr: i32,
    b_cb: i32,
}

/// BT.601 full range (the JFIF equations — the first vertical's
/// tagged case): R = Y + 1.402·Cr; G = Y − 0.344136·Cb − 0.714136·Cr;
/// B = Y + 1.772·Cb.
const BT601_FULL: YuvToRgb = YuvToRgb {
    y_scale: 65_536,
    y_offset: 0,
    r_cr: 91_881,
    g_cb: 22_553,
    g_cr: 46_802,
    b_cb: 116_130,
};

/// BT.601 limited range (ITU-R BT.601-7 §2.5.3 studio swing):
/// Y' = 1.164383·(Y−16); R = Y' + 1.596027·Cr;
/// G = Y' − 0.391762·Cb − 0.812968·Cr; B = Y' + 2.017232·Cb.
const BT601_LIMITED: YuvToRgb = YuvToRgb {
    y_scale: 76_309,
    y_offset: 16,
    r_cr: 104_597,
    g_cb: 25_675,
    g_cr: 53_279,
    b_cb: 132_201,
};

/// BT.709 full range: R = Y + 1.5748·Cr;
/// G = Y − 0.1873·Cb − 0.4681·Cr; B = Y + 1.8556·Cb.
const BT709_FULL: YuvToRgb = YuvToRgb {
    y_scale: 65_536,
    y_offset: 0,
    r_cr: 103_206,
    g_cb: 12_275,
    g_cr: 30_677,
    b_cb: 121_609,
};

/// BT.709 limited range (ITU-R BT.709-6 §3, studio swing):
/// Y' = 1.164383·(Y−16); R = Y' + 1.792741·Cr;
/// G = Y' − 0.213249·Cb − 0.532909·Cr; B = Y' + 2.112402·Cb.
const BT709_LIMITED: YuvToRgb = YuvToRgb {
    y_scale: 76_309,
    y_offset: 16,
    r_cr: 117_489,
    g_cb: 13_975,
    g_cr: 34_925,
    b_cb: 138_438,
};

/// Convert one 4:2:0 I420 frame to RGBA8 under the frame's own
/// tagged colorimetry — the four (BT.601/BT.709 matrix × full/
/// limited range) combinations the landed codecs carry (mjpeg's
/// JFIF BT.601 full; vp9/av1/h264's BT.709 limited; the VUI-tagged
/// variants of each). The chroma planes are box-upsampled (nearest
/// co-located sample), the v0 convention. BT.2020 refuses typed
/// (no landed lane carries it — the general color-conversion card
/// owns it).
///
/// The 2026-11-15 extension (the h264 software lane's landing):
/// this was the BT.601-full-only gate (`cli.i420_color_unsupported`
/// parking vp9/av1/h264 sessions in `Error`); the four landed
/// combinations convert now, and the typed refusal vocabulary stays
/// for everything else.
fn convert_i420_to_rgba(frame: &VideoFrame) -> Result<VideoFrame, EngineError> {
    let coeffs = match (frame.color.matrix, frame.color.range) {
        (ColorMatrix::Bt601, ColorRange::Full) => BT601_FULL,
        (ColorMatrix::Bt601, ColorRange::Limited) => BT601_LIMITED,
        (ColorMatrix::Bt709, ColorRange::Full) => BT709_FULL,
        (ColorMatrix::Bt709, ColorRange::Limited) => BT709_LIMITED,
        (ColorMatrix::Bt2020Ncl, _) => {
            return Err(host_error(
                PipelineStage::Present,
                "cli.i420_color_unsupported",
                "I420 BT.2020 needs the color-conversion card (landed: BT.601/BT.709 × full/limited)".to_string(),
            ));
        }
    };
    let (width, height) = (frame.size.0 as usize, frame.size.1 as usize);
    if width == 0 || height == 0 {
        return Err(EngineError::new(
            PipelineStage::Present,
            ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
            },
            media_types::error::ErrorContext::new("cli.frame_zero_extent"),
        ));
    }
    let chroma_width = width.div_ceil(2);
    let chroma_height = height.div_ceil(2);
    let y_len = width * height;
    let c_len = chroma_width * chroma_height;
    let bytes = match &frame.data {
        FrameStorage::Cpu(bytes) => bytes,
        FrameStorage::OpaqueHandle(_) => {
            return Err(host_error(
                PipelineStage::Present,
                "cli.opaque_handle_unsupported",
                "hardware frames need the wgpu backend (2.D3)",
            ));
        }
    };
    let data = bytes.as_slice();
    if data.len() < y_len + 2 * c_len {
        return Err(EngineError::new(
            PipelineStage::Present,
            ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::Truncated {
                    expected: (y_len + 2 * c_len) as u64,
                    found: data.len() as u64,
                },
            },
            media_types::error::ErrorContext::new("cli.i420_truncated"),
        ));
    }
    let (y_plane, u_plane, v_plane) = (
        &data[..y_len],
        &data[y_len..y_len + c_len],
        &data[y_len + c_len..],
    );
    let mut rgba = Vec::with_capacity(width * height * 4);
    for row in 0..height {
        let chroma_row = row / 2;
        for column in 0..width {
            let y = i32::from(y_plane[row * width + column]);
            let chroma_column = column / 2;
            let cb = i32::from(u_plane[chroma_row * chroma_width + chroma_column]) - 128;
            let cr = i32::from(v_plane[chroma_row * chroma_width + chroma_column]) - 128;
            // The frame's own (matrix, range) pair in Q16 fixed point
            // (the coefficient tables above carry the spec equations).
            let luma = coeffs.y_scale * (y - coeffs.y_offset);
            let r = clamp_u8((luma + coeffs.r_cr * cr + 32_768) >> 16);
            let g = clamp_u8((luma - coeffs.g_cb * cb - coeffs.g_cr * cr + 32_768) >> 16);
            let b = clamp_u8((luma + coeffs.b_cb * cb + 32_768) >> 16);
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    Ok(VideoFrame {
        format: media_types::PixelFormat::Rgba8,
        size: frame.size,
        data: FrameStorage::Cpu(media_types::packet::SharedBytes::from_vec(rgba)),
        pts: frame.pts,
        color: frame.color,
        interlaced: frame.interlaced,
        generation: frame.generation,
    })
}

fn clamp_u8(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

// ---------------------------------------------------------------------------
// The audio session wiring (open-path construction)
// ---------------------------------------------------------------------------

/// The driven audio session halves plus the processing pipeline: what
/// the open path builds when the media has an audio track, over the
/// backend the factory posture routed ([`RoutedAudioBackend`]).
pub struct AudioSession {
    /// The consumer's audio half (pipeline + sink).
    pub limb: AudioLimb,
    /// The device half (the pump drives it).
    pub device: NullDevice,
    /// The session clock built over the sink's counter.
    pub clock: Arc<MediaClock>,
    /// The channel count (render buffer sizing).
    pub channels: u32,
    /// The backend the factory posture routed for this session — the
    /// routing decision's own record ([`RoutedAudioBackend`]).
    pub routed: RoutedAudioBackend,
}

impl AudioSession {
    /// The live master chain over the whole session — the same
    /// `&mut` hand-off [`AudioLimb::audio_controls`] yields (the
    /// session owns the limb; one chain, two reachable entries — the
    /// host verbs route through whichever half they hold).
    pub fn audio_controls(&mut self) -> &mut AudioControls {
        self.limb.audio_controls()
    }
}

/// The audio-output backend posture for session construction — the
/// `--audio-backend` grammar's session-construction vocabulary
/// (parse.rs's `AudioBackend` maps one-to-one onto it at the host
/// seam), threaded into [`open_audio_session_for`].
///
/// - [`Auto`](AudioBackendPosture::Auto) (the default): prefer a real
///   device backend when this build carries one (the audio-output
///   `backend-cpal` feature — [`audio_output::builtin_backends`]
///   puts the admitted adapter first), falling back to the
///   instrumented null backend (the headless default).
/// - [`Null`](AudioBackendPosture::Null): force the instrumented
///   null backend — samples counted (the counter is the session
///   clock's truth source) and discarded: deterministic, no audio
///   device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioBackendPosture {
    /// `--audio-backend auto`: real backend first when the build
    /// carries one, null fallback otherwise.
    Auto,
    /// `--audio-backend null`: force the instrumented null backend.
    Null,
}

/// The backend the session's audio path actually routed to — the
/// factory consultation's own record (see
/// [`open_audio_session_for`]'s routing gate): every session this
/// host constructs today routes the instrumented null backend; a
/// real-device variant joins this vocabulary when the host-pump
/// rework can drive an OS-callback device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutedAudioBackend {
    /// The instrumented null backend: the driven halves, the discard
    /// device, the sample counter the session clock reads.
    Null,
}

/// Open the driven audio session + the audio-master clock for a media
/// whose PCM facts are known. `anchor_ns` is the position the session
/// starts at (0, or the seek target). The default posture
/// ([`AudioBackendPosture::Auto`]) and matching rates — see
/// [`open_audio_session_for`] for the factory posture, the clock
/// semantics and the rate pair.
///
/// # Errors
/// The backend's typed errors (unsupported format shape) and the
/// clock's construction errors, both already stage-tagged.
pub fn open_audio_session(
    channels: u32,
    rate: u32,
    anchor_ns: i64,
) -> Result<AudioSession, EngineError> {
    open_audio_session_for(channels, rate, rate, anchor_ns, AudioBackendPosture::Auto)
}

/// Open the driven audio session through the audio-output backend
/// factory posture (FX3-A01's admission): the posture selects, the
/// build's truth ([`cpal_linked`]) decides what
/// [`audio_output::builtin_backends`] actually carries, and the
/// session routes the backend the driven CLI host can drive.
///
/// **The routing gate (this wave's honest bound).** The CLI host's
/// session is the DRIVEN null session: the host pump renders the
/// concrete [`NullDevice`] half (host.rs's `advance_time` render
/// loop, gated on the null ring's fill), and the limb's probe
/// surface — the drain gate's ring fill
/// ([`AudioLimb::buffered_frames`]), the decode-park gate
/// ([`AudioLimb::ring_ready`]), the seek boundary's written prefix
/// ([`AudioLimb::written_frames`]) and the drift observation's
/// sample count — reads the null driver's own counters. The
/// [`audio_output::AudioBackend`] trait carries
/// open/pause/resume/clock_source/underruns only
/// (crates/audio-output/src/device.rs): the cpal adapter's
/// buffered/written/sample probes are concrete
/// (crates/audio-output/src/cpal.rs), not trait-reachable — so an
/// opened real device cannot yet be ROUTED as this session's sink.
/// The `Auto` posture's real-device preference therefore resolves
/// to the null fallback — the same backend the `Null` posture
/// forces — exactly what the `--audio-backend` grammar's own doc
/// records (parse.rs: "this build carries no real device backend,
/// so both values construct the instrumented null backend today").
/// The real-device activation is the host-pump rework (the desktop
/// lane, FX4-A03/FX4-A04) plus the trait's probe surface.
///
/// **The clock semantics (preserved per selected sink).**
/// [`MediaClock::audio_master`] anchors over the selected sink's
/// `clock_source`: every backend that can be selected provides one
/// after open — the null backend's shared counter
/// (crates/audio-output/src/null.rs, advanced by the pump's device
/// renders — the deterministic clock rides the null path) and, once
/// routable, the cpal backend's own `SampleCounter`
/// (crates/audio-output/src/cpal.rs — advanced by the OS callback
/// through `render_into`; crates/audio-output/src/pump.rs's `PcmPump`
/// is the reference pacing shape for that lane).
///
/// **The rate pair (the FX3-A04 seam).** `track_rate` is the
/// pipeline's rate — the media's own sampling grid — and `sink_rate`
/// is the output clock's: equal in the null fallback (the headless
/// default — the resampler seam is absent and [`AudioLimb::stage`]'s
/// write path is byte-identical), different when the device's clock
/// runs at another rate (the real-device lane's shape; the null
/// backend models it). A differing pair wires
/// [`RateResampler`] at this session seam — the engine pipeline
/// itself still refuses track-rate mismatch (`audio.rate_mismatch`,
/// crates/audio-engine/src/pipeline.rs), so the conversion lives
/// here, between the mixed quantum and the sink.
///
/// # Errors
/// The backend's typed errors (unsupported format shape), the
/// resampler's shape refusal and the clock's construction errors —
/// all already stage-tagged.
pub fn open_audio_session_for(
    channels: u32,
    track_rate: u32,
    sink_rate: u32,
    anchor_ns: i64,
    posture: AudioBackendPosture,
) -> Result<AudioSession, EngineError> {
    // The factory consultation: the Auto posture prefers a real
    // device backend when this build carries one —
    // builtin_backends() puts the admitted adapter first under the
    // `backend-cpal` feature, and cpal_linked() is the runtime
    // truth — while the Null posture forces the null backend
    // outright. The routing gate above keeps the driven session on
    // the null backend either way this wave; the routed fact is the
    // session's own record.
    let routed = match posture {
        AudioBackendPosture::Auto if cpal_linked() => {
            // A build carrying a real device backend: the preference
            // consulted the factory; the routing gate (the pump and
            // probe contract above) keeps the driven null session
            // until the host-pump rework routes OS-callback devices
            // (the desktop lane's follow-up).
            RoutedAudioBackend::Null
        }
        AudioBackendPosture::Auto | AudioBackendPosture::Null => RoutedAudioBackend::Null,
    };
    let sink_format = AudioFormat {
        sample: SampleFormat::F32,
        channels,
        layout: AudioLayout::Interleaved,
        rate: sink_rate,
    };
    let NullStream { sink, device } =
        NullBackend::open_stream(&sink_format, RING_FRAMES, NullConfig { capture: false })?;
    let pipeline = AudioPipeline::new(PipelineSpec {
        rate: track_rate,
        channels,
        tracks: 1,
    })?;
    // FX3-A04's rate-mismatch resampler at the session seam: wired
    // exactly when the output clock runs at a different rate than
    // the track; equal rates (the null fallback) skip it. The live
    // master chain (FX3-A03) arms at the track rate — the mixed
    // quantum's own grid.
    let resampler = match RateResampler::new(channels, track_rate, sink_rate) {
        Ok(resampler) if !resampler.is_passthrough() => Some(resampler),
        Ok(_) => None,
        Err(error) => return Err(error),
    };
    let clock = Arc::new(MediaClock::audio_master(
        sink.clock_source(),
        Timestamp::new(anchor_ns, media_clock::NANO_BASE),
        Rate::ONE,
    )?);
    Ok(AudioSession {
        limb: AudioLimb {
            pipeline,
            sink,
            rate_hz: sink_rate,
            controls: AudioControls::new(track_rate),
            seam: PostMixSeam {
                resampler,
                planes: Vec::new(),
                bytes: Vec::new(),
            },
        },
        device,
        clock,
        channels,
        routed,
    })
}

/// The no-audio fallback clock: system monotonic (wall time). The
/// audio-bearing path is the deterministic one; video-only media runs
/// on the real clock `[PROPOSAL: an injectable virtual monotonic
/// source for tests joins with the host clock seam 3.B2]`.
///
/// # Errors
/// The clock's construction errors (already stage-tagged).
pub fn wall_clock(anchor_ns: i64) -> Result<Arc<MediaClock>, EngineError> {
    Ok(Arc::new(MediaClock::system_monotonic(
        Arc::new(media_clock::InstantSource::new()),
        Timestamp::new(anchor_ns, media_clock::NANO_BASE),
        Rate::ONE,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::frame::ColorDescription;
    use media_types::packet::SharedBytes;
    use media_types::time::Stamp;
    use media_types::{GenerationId, Packet, PixelFormat};
    use testkit::Must;

    fn i420_frame(width: usize, height: usize, y: u8, cb: u8, cr: u8) -> VideoFrame {
        let chroma_w = width.div_ceil(2);
        let chroma_h = height.div_ceil(2);
        let mut data = vec![y; width * height];
        data.extend(vec![cb; chroma_w * chroma_h]);
        data.extend(vec![cr; chroma_w * chroma_h]);
        VideoFrame {
            format: PixelFormat::I420,
            size: (width as u32, height as u32),
            data: FrameStorage::Cpu(SharedBytes::from_vec(data)),
            pts: Stamp::Missing,
            color: ColorDescription {
                primaries: media_types::frame::ColorPrimaries::Bt601,
                transfer: media_types::frame::TransferCharacteristics::Srgb,
                matrix: ColorMatrix::Bt601,
                range: ColorRange::Full,
            },
            interlaced: false,
            generation: GenerationId::FIRST,
        }
    }

    fn pixel(frame: &VideoFrame, x: usize, y: usize) -> [u8; 4] {
        let bytes = match &frame.data {
            FrameStorage::Cpu(bytes) => bytes.as_slice(),
            FrameStorage::OpaqueHandle(_) => [].as_slice(),
        };
        let base = (y * frame.size.0 as usize + x) * 4;
        [
            bytes[base],
            bytes[base + 1],
            bytes[base + 2],
            bytes[base + 3],
        ]
    }

    /// JFIF vectors: neutral chroma passes luma through; the extreme
    /// chroma rows match the standard equations to ±1 (Q8 rounding).
    #[test]
    fn i420_conversion_matches_jfif_vectors() {
        // Gray: Y=128, neutral chroma → (128,128,255).
        let gray = match convert_i420_to_rgba(&i420_frame(8, 8, 128, 128, 128)) {
            Ok(frame) => frame,
            Err(err) => unreachable!("{err}"),
        };
        assert_eq!(gray.format, PixelFormat::Rgba8);
        assert_eq!(pixel(&gray, 3, 5), [128, 128, 128, 255]);
        // Black / white.
        let black = match convert_i420_to_rgba(&i420_frame(8, 8, 0, 128, 128)) {
            Ok(frame) => frame,
            Err(err) => unreachable!("{err}"),
        };
        assert_eq!(pixel(&black, 0, 0), [0, 0, 0, 255]);
        let white = match convert_i420_to_rgba(&i420_frame(8, 8, 255, 128, 128)) {
            Ok(frame) => frame,
            Err(err) => unreachable!("{err}"),
        };
        assert_eq!(pixel(&white, 7, 7), [255, 255, 255, 255]);
        // Red: Cr=255 (Cr−128 = 127), Cb neutral, Y=50 → the JFIF
        // equations in-range: R = 50 + 1.402·127 ≈ 228,
        // G = 50 − 0.714·127 − 0 ≈ −41 → clamps to 0, B = 50.
        let red = match convert_i420_to_rgba(&i420_frame(8, 8, 50, 128, 255)) {
            Ok(frame) => frame,
            Err(err) => unreachable!("{err}"),
        };
        let [r, g, b, a] = pixel(&red, 2, 2);
        assert_eq!(a, 255);
        assert!((i32::from(r) - 228).abs() <= 1, "{r}");
        assert_eq!(g, 0, "green clamps below zero");
        assert_eq!(b, 50, "neutral Cb keeps blue at luma");
        // Blue: Cb=255 (Cb−128 = 127), Cr neutral, Y=128 →
        // B = 128 + 1.772·127 ≈ 353 → clamps to 255; the JFIF R has
        // NO Cb term, so R stays at luma.
        let blue = match convert_i420_to_rgba(&i420_frame(8, 8, 128, 255, 128)) {
            Ok(frame) => frame,
            Err(err) => unreachable!("{err}"),
        };
        let [r, _g, b, _a] = pixel(&blue, 5, 1);
        assert_eq!(b, 255, "Y=128 + 225 clamps to 255");
        assert_eq!(r, 128, "neutral Cr keeps red at luma (R has no Cb term)");
    }

    /// The 2026-11-15 extension vectors: the studio-swing pairs
    /// (BT.601/BT.709 limited) and BT.709 full convert under the
    /// frame's own tag — computed from the same Q16 tables the
    /// converter uses, cross-checked against the spec equations.
    #[test]
    fn i420_conversion_covers_the_landed_color_pairs() {
        let tag = |matrix: ColorMatrix, range: ColorRange| ColorDescription {
            primaries: media_types::frame::ColorPrimaries::Bt709,
            transfer: media_types::frame::TransferCharacteristics::Bt709,
            matrix,
            range,
        };
        let frame = |color: ColorDescription| VideoFrame {
            format: PixelFormat::I420,
            size: (8, 8),
            data: {
                let mut data = vec![0u8; 8 * 8];
                data.extend(vec![128u8; 16]);
                data.extend(vec![128u8; 16]);
                FrameStorage::Cpu(SharedBytes::from_vec(data))
            },
            pts: Stamp::Missing,
            color,
            interlaced: false,
            generation: GenerationId::FIRST,
        };
        // Studio-swing black/white/gray map exactly on both matrices.
        for matrix in [ColorMatrix::Bt601, ColorMatrix::Bt709] {
            let black = {
                let mut f = frame(tag(matrix, ColorRange::Limited));
                let FrameStorage::Cpu(bytes) = &mut f.data else {
                    unreachable!("cpu frame");
                };
                let bytes = media_types::packet::SharedBytes::get_mut(bytes).must();
                for (i, b) in bytes.iter_mut().enumerate() {
                    *b = if i < 64 { 16 } else { 128 };
                }
                match convert_i420_to_rgba(&f) {
                    Ok(out) => out,
                    Err(err) => unreachable!("{err}"),
                }
            };
            assert_eq!(pixel(&black, 0, 0), [0, 0, 0, 255], "{matrix:?} black");
            let white = {
                let mut f = frame(tag(matrix, ColorRange::Limited));
                let FrameStorage::Cpu(bytes) = &mut f.data else {
                    unreachable!("cpu frame");
                };
                let bytes = media_types::packet::SharedBytes::get_mut(bytes).must();
                for (i, b) in bytes.iter_mut().enumerate() {
                    *b = if i < 64 { 235 } else { 128 };
                }
                match convert_i420_to_rgba(&f) {
                    Ok(out) => out,
                    Err(err) => unreachable!("{err}"),
                }
            };
            assert_eq!(
                pixel(&white, 7, 7),
                [255, 255, 255, 255],
                "{matrix:?} white"
            );
        }
        // BT.709 limited mid-gray: Y=126 → (128,128,128) ±1 (Q16).
        let gray = {
            let mut f = frame(tag(ColorMatrix::Bt709, ColorRange::Limited));
            let FrameStorage::Cpu(bytes) = &mut f.data else {
                unreachable!("cpu frame");
            };
            let bytes = media_types::packet::SharedBytes::get_mut(bytes).must();
            for (i, b) in bytes.iter_mut().enumerate() {
                *b = if i < 64 { 126 } else { 128 };
            }
            match convert_i420_to_rgba(&f) {
                Ok(out) => out,
                Err(err) => unreachable!("{err}"),
            }
        };
        let [r, g, b, _a] = pixel(&gray, 4, 4);
        for channel in [r, g, b] {
            assert!((i32::from(channel) - 128).abs() <= 1, "{channel}");
        }
        // BT.709 full-range gray passes luma through (the JFIF rule,
        // 709 coefficients): Y=200 → (200,200,200).
        let gray200 = {
            let mut f = frame(tag(ColorMatrix::Bt709, ColorRange::Full));
            let FrameStorage::Cpu(bytes) = &mut f.data else {
                unreachable!("cpu frame");
            };
            let bytes = media_types::packet::SharedBytes::get_mut(bytes).must();
            for (i, b) in bytes.iter_mut().enumerate() {
                *b = if i < 64 { 200 } else { 128 };
            }
            match convert_i420_to_rgba(&f) {
                Ok(out) => out,
                Err(err) => unreachable!("{err}"),
            }
        };
        assert_eq!(pixel(&gray200, 1, 6), [200, 200, 200, 255]);
        // BT.2020 refuses typed (the honest not-yet vocabulary).
        let err = convert_i420_to_rgba(&frame(tag(ColorMatrix::Bt2020Ncl, ColorRange::Limited)))
            .err()
            .must();
        assert_eq!(err.context.code, "cli.i420_color_unsupported");
    }

    /// Odd dimensions use ceil chroma geometry; the buffer length is
    /// validated (truncated planes are typed corruption, never OOB).
    #[test]
    fn i420_conversion_validates_and_handles_odd_sizes() {
        let frame = i420_frame(7, 5, 200, 128, 128);
        let converted = match convert_i420_to_rgba(&frame) {
            Ok(frame) => frame,
            Err(err) => unreachable!("{err}"),
        };
        assert_eq!(converted.size, (7, 5));
        assert_eq!(pixel(&converted, 6, 4), [200, 200, 200, 255]);
        // Truncated data: typed corrupt.
        let mut truncated = i420_frame(8, 8, 128, 128, 128);
        let short = match &truncated.data {
            FrameStorage::Cpu(bytes) => bytes.as_slice()[..10].to_vec(),
            FrameStorage::OpaqueHandle(_) => Vec::new(),
        };
        truncated.data = FrameStorage::Cpu(SharedBytes::from_vec(short));
        match convert_i420_to_rgba(&truncated) {
            Err(err) => assert_eq!(
                err.kind,
                ErrorKind::Corrupt {
                    detail: media_types::error::CorruptDetail::Truncated {
                        expected: 96,
                        found: 10
                    }
                }
            ),
            Ok(_) => unreachable!("truncated planes must be refused"),
        }
    }

    /// The 2026-11-15 behavior flip, pinned: limited-range tags now
    /// CONVERT (the pre-extension refusal was the honest gate; the
    /// extended converter carries the studio-swing equations — the
    /// only remaining typed refusal in the vocabulary is BT.2020,
    /// asserted in `i420_conversion_covers_the_landed_color_pairs`).
    #[test]
    fn i420_conversion_accepts_limited_range() {
        let mut frame = i420_frame(8, 8, 128, 128, 128);
        frame.color.range = ColorRange::Limited;
        let converted = match convert_i420_to_rgba(&frame) {
            Ok(out) => out,
            Err(err) => unreachable!("limited range must convert: {err}"),
        };
        assert_eq!(converted.format, PixelFormat::Rgba8);
        // BT.601 limited, Y=128 neutral: ≈ (130,130,130) (studio
        // swing: 1.164383·(128−16) ≈ 130.4; ±1 Q16).
        let [r, g, b, _a] = pixel(&converted, 2, 2);
        for channel in [r, g, b] {
            assert!((i32::from(channel) - 130).abs() <= 1, "{channel}");
        }
    }

    /// A media-types packet field sanity check for the imports this
    /// module's public surface touches (compile-level pin).
    #[test]
    fn shared_bytes_roundtrip() {
        let bytes = SharedBytes::from_vec(vec![1, 2, 3]);
        assert_eq!(bytes.as_slice(), &[1, 2, 3]);
        let _packet_shape = Packet {
            track: TrackId::new(0),
            pts: Stamp::Missing,
            dts: Stamp::Missing,
            duration: None,
            data: bytes,
            keyframe: true,
            discontinuity: false,
            generation: GenerationId::FIRST,
        };
    }

    /// A recording stand-in decoder: every `configure` call's
    /// `extra_data` is captured, in order, into a shared log (the
    /// re-seeding wrapper's observation point — the wrapper owns the
    /// decoder, so the log must outlive it).
    struct RecordingDecoder {
        seen: Arc<Mutex<Vec<Option<SharedBytes>>>>,
    }

    impl Decoder for RecordingDecoder {
        fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
            match self.seen.lock() {
                Ok(mut log) => log.push(cfg.extra_data.clone()),
                // Recording never panics, so poison is unreachable;
                // recovering the inner log is the honest answer.
                Err(poisoned) => poisoned.into_inner().push(cfg.extra_data.clone()),
            }
            Ok(())
        }
        fn decode(
            &mut self,
            _packet: Packet,
            _out: &mut dyn FrameSink,
        ) -> Result<DecodeOutcome, EngineError> {
            Ok(DecodeOutcome::NeedMoreInput)
        }
        fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
            Ok(DecodeOutcome::Drained)
        }
    }

    /// `lock` without `unwrap`: poison recovers the inner log
    /// (recording never panics — the arm is provably dead, kept
    /// honest rather than unreachable!-bridged).
    fn recorded(log: &Arc<Mutex<Vec<Option<SharedBytes>>>>) -> Vec<Option<SharedBytes>> {
        match log.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    fn test_config(generation: GenerationId, extra: Option<SharedBytes>) -> DecoderConfig {
        DecoderConfig::new(
            media_types::ids::CodecDescriptor::new("vorbis")
                .unwrap_or_else(|e| unreachable!("{e}")),
            generation,
            codec_api::ThreadBudget::default(),
            None,
            extra,
        )
    }

    /// The 6.FIX-ogg-reanchor seam, pinned at the unit: a header-less
    /// generation reconfigure (the engine's `seek_flush` mint —
    /// `extra_data: None`, measured as `opus.head.missing` /
    /// `vorbis.headers.missing` through the binary before the fix)
    /// arrives at the inner decoder carrying the SAME blob the open
    /// path seeded. An explicit incoming blob wins; a fact-less
    /// wrapper stays a pure passthrough.
    #[test]
    fn reseeded_generation_reconfigure_replays_the_open_path_blob() {
        let blob = SharedBytes::from_vec(vec![7u8; 16]);
        let log: Arc<Mutex<Vec<Option<SharedBytes>>>> = Arc::new(Mutex::new(Vec::new()));
        let mut wrapper = ReseedingDecoder::new(
            Box::new(RecordingDecoder {
                seen: Arc::clone(&log),
            }),
            Some(blob),
        );
        // The open configure (the provider's seeded config)…
        wrapper
            .configure(&test_config(
                GenerationId::FIRST,
                Some(SharedBytes::from_vec(vec![7u8; 16])),
            ))
            .unwrap_or_else(|e| unreachable!("{e}"));
        // …then the engine's generation bump, minted header-less.
        let second = GenerationId::FIRST
            .next()
            .unwrap_or_else(|e| unreachable!("{e}"));
        wrapper
            .configure(&test_config(second, None))
            .unwrap_or_else(|e| unreachable!("{e}"));
        // Both configs reached the inner decoder carrying the blob:
        // the second was re-seeded from the retained open-path one.
        let seen = recorded(&log);
        assert_eq!(seen.len(), 2, "one configure per call, forwarded once");
        assert!(
            seen.iter().all(|extra| extra
                .as_ref()
                .is_some_and(|bytes| bytes.as_slice() == [7u8; 16])),
            "the header-less generation bump was re-seeded: {seen:?}"
        );

        // Precedence: an explicit incoming blob wins over the retained
        // default (the caller's seeding is more specific).
        wrapper
            .configure(&test_config(
                second,
                Some(SharedBytes::from_vec(vec![9u8; 4])),
            ))
            .unwrap_or_else(|e| unreachable!("{e}"));
        let seen = recorded(&log);
        assert!(
            seen.last()
                .and_then(|extra| extra.as_ref())
                .is_some_and(|bytes| bytes.as_slice() == [9u8; 4]),
            "the explicit incoming blob wins: {seen:?}"
        );

        // A fact-less wrapper (an in-band codec lane) is a pure
        // passthrough: the header-less mint stays header-less and the
        // codec's own configure gate stays the honest authority.
        let bare_log: Arc<Mutex<Vec<Option<SharedBytes>>>> = Arc::new(Mutex::new(Vec::new()));
        let mut bare = ReseedingDecoder::new(
            Box::new(RecordingDecoder {
                seen: Arc::clone(&bare_log),
            }),
            None,
        );
        bare.configure(&test_config(second, None))
            .unwrap_or_else(|e| unreachable!("{e}"));
        let seen = recorded(&bare_log);
        assert!(
            seen.len() == 1 && seen[0].is_none(),
            "nothing is invented for a fact-less lane: {seen:?}"
        );
    }
}
