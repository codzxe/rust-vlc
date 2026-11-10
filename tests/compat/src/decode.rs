//! The real decode pipeline driver: one corpus file → the honest
//! per-track record.
//!
//! The composition is the player's own (doc 03 §4.2, the same seams
//! `apps/cli`'s bench and `playback-engine` drive):
//!
//! 1. [`lumen_cli::demux_host::open_source`] — canonicalize →
//!    `input` access → `media-probe` → the container-host registry
//!    (all 15 wired families) → the started, wrapped demuxer + the
//!    harvested [`MediaFacts`].
//! 2. [`HostDecoderProvider`] — the player's real decoder dispatch
//!    (mjpeg, pcm, flac, mp3, mp2, vorbis, opus, aac-lc, alac,
//!    adpcm, av1, vp9, dnxhd), configured exactly as the engine
//!    mints configs ([`DecoderConfig::new`] with
//!    [`GenerationId::FIRST`] and no accelerator).
//! 3. The event loop — packets routed per track, per-frame damage
//!    concealed with the engine's 1000-consecutive budget, audio
//!    frames digested (RFC 1321 MD5, in-repo via `bench-vs-vlc`),
//!    video frames counted.
//!
//! Every outcome is a typed, serializable record — a refused probe, a
//! refused decoder or a mid-walk demux fatal is a RESULT, never a
//! crash (dirty-world rule D-3).

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink, ThreadBudget};
use commands::catalog::types::SourceSpec;
use lumen_cli::demux_host::open_source;
use lumen_cli::pipeline_host::HostDecoderProvider;
use media_types::error::DecodeCause;
use media_types::frame::SampleFormat;
use media_types::{EngineError, ErrorKind, Frame, GenerationId, MediaKind, Packet, TrackId};
use playback_engine::DecoderProvider;
use plugin_api::DemuxEvent;
use serde::Serialize;

/// The concealment budget — mirrors the engine's
/// `VIDEO_CONCEALMENT_BUDGET` (bench.rs's one-budget-for-both-limbs
/// design; a stream whose every packet fails is broken, not
/// comparable).
pub const CONCEALMENT_BUDGET: u32 = 1000;

/// Whether an error is per-frame damage — the class the engine
/// conceals (mirrored from the CLI bench, which mirrors
/// `playback-engine`; the engine's own is crate-private).
fn per_frame_damage(error: &EngineError) -> bool {
    matches!(
        &error.kind,
        ErrorKind::Corrupt { .. }
            | ErrorKind::Decoder {
                cause: DecodeCause::Bitstream
            }
    )
}

/// The digesting sink for one audio limb: MD5 over the accepted PCM
/// bytes (the documented parity digest), the counters, and — when the
/// caller asked for it (oracle comparison) — the accumulated payload
/// for a byte-exact window comparison.
pub struct AudioDigestSink {
    digest: bench_vs_vlc::md5::Md5,
    /// PCM bytes absorbed.
    pub bytes: u64,
    /// Audio frames accepted.
    pub frames: u64,
    /// Mid-stream format changes (0 in a sane stream).
    pub format_changes: u32,
    /// The first frame's sample format name (`s16`, `f32`, ...).
    pub sample: Option<&'static str>,
    /// The payload, when dumping was requested (bounded by the
    /// corpus's own file sizes; `None` otherwise).
    pub payload: Option<Vec<u8>>,
}

impl AudioDigestSink {
    /// A fresh sink; `dump` retains the decoded PCM bytes for the
    /// oracle window comparison.
    pub fn new(dump: bool) -> AudioDigestSink {
        AudioDigestSink {
            digest: bench_vs_vlc::md5::Md5::new(),
            bytes: 0,
            frames: 0,
            format_changes: 0,
            sample: None,
            payload: if dump { Some(Vec::new()) } else { None },
        }
    }

    /// The digest of everything accepted so far, lowercase hex.
    pub fn digest_hex(&self) -> String {
        let clone = self.digest.clone();
        clone.finalize_hex()
    }
}

fn sample_name(sample: SampleFormat) -> &'static str {
    match sample {
        SampleFormat::U8 => "u8",
        SampleFormat::S16 => "s16",
        SampleFormat::S24 => "s24",
        SampleFormat::S32 => "s32",
        SampleFormat::F32 => "f32",
        SampleFormat::F64 => "f64",
    }
}

impl FrameSink for AudioDigestSink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        let Frame::Audio(audio) = frame else {
            // A non-audio frame on an audio limb is a wiring anomaly;
            // surface it through the anomaly counter, never a crash.
            self.format_changes = self.format_changes.saturating_add(1);
            return Ok(());
        };
        let name = sample_name(audio.format.sample);
        match self.sample {
            Some(seen) if seen != name => {
                self.format_changes = self.format_changes.saturating_add(1);
            }
            None => self.sample = Some(name),
            _ => {}
        }
        let samples = audio.samples.as_slice();
        self.digest.update(samples);
        self.bytes += samples.len() as u64;
        self.frames += 1;
        if let Some(payload) = self.payload.as_mut() {
            payload.extend_from_slice(samples);
        }
        Ok(())
    }
}

/// The counting sink for non-audio limbs (video): frames counted,
/// nothing retained (this matrix measures decode support, not pixels;
/// the visual lane owns pixels).
#[derive(Default)]
pub struct CountSink {
    /// Frames accepted.
    pub frames: u64,
}

impl FrameSink for CountSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        self.frames += 1;
        Ok(())
    }
}

/// One track's decode outcome.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum TrackOutcome {
    /// The decoder existed and consumed packets.
    Decoded {
        /// Registry descriptor (`pcm_s16le`, `mjpeg`, ...).
        codec: String,
        /// Demuxed packets fed.
        packets: u64,
        /// Frames the decoder emitted.
        frames: u64,
        /// Per-frame damage concealed (nonzero ⇒ damaged decode).
        concealed: u64,
        /// Audio only: MD5 of the decoded PCM, lowercase hex.
        pcm_md5: Option<String>,
        /// Audio only: decoded PCM bytes.
        pcm_bytes: Option<u64>,
        /// Audio only: emitted sample format (`s16`, ...).
        sample: Option<String>,
        /// Mid-stream format changes (nonzero ⇒ anomaly).
        format_changes: u32,
    },
    /// No decoder in this build for the track's codec (the honest
    /// typed refusal, message verbatim).
    NoDecoder {
        /// Registry descriptor.
        codec: String,
        /// The refusal's message.
        message: String,
    },
}

/// One corpus file's whole record.
#[derive(Debug, Clone, Serialize)]
pub struct FileRecord {
    /// Corpus-relative id (config-rooted).
    pub rel: String,
    /// Absolute path (recorded provenance).
    pub path: String,
    /// File size in bytes.
    pub bytes: u64,
    /// Overall outcome.
    pub outcome: FileOutcome,
    /// The probe's format id (when the probe ran).
    pub probe_format: Option<String>,
    /// The per-track outcomes (when the open path ran).
    pub tracks: Vec<TrackOutcome>,
    /// Demux `Recovered` events.
    pub demux_recovered: u64,
    /// Bytes the demuxer skipped while recovering.
    pub demux_skip_bytes: u64,
    /// Facts tracks that are not Audio/Video (subtitles, chapters,
    /// data) — skipped by design: the engine's wired-limb set is the
    /// wrapper's `start()` answer (audio/video only), so these are
    /// not decode gaps.
    pub skipped_non_av_tracks: u32,
    /// Wall-clock decode seconds.
    pub decode_s: f64,
}

/// The overall outcome for one file.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum FileOutcome {
    /// Open + full walk + every track decoded (concealed damage, if
    /// any, is in the per-track counters).
    Decoded,
    /// Open + walk, but at least one track had no decoder (partial
    /// support — the `NoDecoder` rows name the codec).
    DecodedPartial,
    /// Refused at the probe stage (unknown/unsupported signature).
    RefusedProbe { message: String },
    /// Refused at the demux stage (header parse, no tracks).
    RefusedDemux { message: String },
    /// A mid-walk demux fatal (typed; the counts say how far it got).
    DemuxFatal { message: String },
    /// The input stage refused (missing file, unreadable).
    RefusedInput { message: String },
}

/// The decoded-payload dump for oracle comparison (first audio track).
pub struct DecodeDump {
    /// The first audio track's decoded PCM bytes.
    pub pcm: Vec<u8>,
    /// The digest sink's other answers (hex + sample format).
    pub md5_hex: String,
    /// The emitted sample format (`u8`/`s16`/`f32`).
    pub sample: &'static str,
}

/// One wired limb (decoder + its sinks + counters).
struct Limb {
    /// The demuxed track this limb consumes.
    track: TrackId,
    /// The real decoder behind the contract.
    decoder: Box<dyn Decoder>,
    /// Packets fed.
    packets: u64,
    /// Frames emitted.
    frames: u64,
    /// Per-frame damage concealed.
    concealed: u64,
    /// Consecutive damage streak (the budget's counter).
    consecutive: u32,
    /// A fatal decode error (non-damage): the typed message.
    fatal: Option<String>,
    /// The audio digest sink (audio limbs).
    audio: Option<AudioDigestSink>,
    /// The video counting sink (video limbs).
    video: CountSink,
    /// First audio limb marker (the dump's owner).
    first_audio: bool,
}

impl Limb {
    /// Feed one packet (the D-1 loop body: move, decode, count).
    fn feed(&mut self, packet: Packet) {
        self.packets += 1;
        let outcome = if let Some(sink) = self.audio.as_mut() {
            self.decoder.decode(packet, sink)
        } else {
            self.decoder.decode(packet, &mut self.video)
        };
        match outcome {
            Ok(DecodeOutcome::Emitted { frames }) => {
                self.frames += u64::from(frames);
                self.consecutive = 0;
            }
            Ok(DecodeOutcome::Recovered { .. }) => {
                self.consecutive = self.consecutive.saturating_add(1);
            }
            Ok(DecodeOutcome::NeedMoreInput | DecodeOutcome::Drained) => {}
            Err(error) if per_frame_damage(&error) => {
                self.concealed += 1;
                self.consecutive = self.consecutive.saturating_add(1);
                if self.consecutive >= CONCEALMENT_BUDGET {
                    self.fatal = Some(format!(
                        "{CONCEALMENT_BUDGET} consecutive damaged packets — concealment \
                         budget exhausted: {error}"
                    ));
                }
            }
            Err(error) => {
                self.fatal = Some(error.to_string());
            }
        }
    }

    /// Drain at end-of-stream.
    fn drain(&mut self) {
        let outcome = if let Some(sink) = self.audio.as_mut() {
            self.decoder.flush(sink)
        } else {
            self.decoder.flush(&mut self.video)
        };
        match outcome {
            Ok(DecodeOutcome::Emitted { frames }) => {
                self.frames += u64::from(frames);
            }
            Ok(_) => {}
            Err(error) => {
                if self.fatal.is_none() {
                    self.fatal = Some(format!("flush: {error}"));
                }
            }
        }
    }
}

/// Run one file through the real open/decode path.
///
/// `dump_audio` retains the FIRST audio track's decoded PCM (the
/// oracle comparison needs the bytes; bounded by the corpus's own
/// file sizes — the comparable family is ≤ 4 MB of PCM per file).
///
/// # Errors
/// Only tool-level failures (the path is unreadable). Every media
/// outcome — including refusals — is an `Ok((record, dump))`.
pub fn decode_file(
    rel: &str,
    path: &Path,
    dump_audio: bool,
) -> Result<(FileRecord, Option<DecodeDump>), String> {
    let bytes = std::fs::metadata(path)
        .map_err(|err| format!("cannot stat {}: {err}", path.display()))?
        .len();
    let started = Instant::now();

    // 1. The open path (probe → container registry → started demuxer
    //    + facts). A refusal here is a RESULT.
    let spec = SourceSpec::File {
        path: path.to_path_buf(),
    };
    let mut opened = match open_source(&spec, None, false, false) {
        Ok(opened) => opened,
        Err(err) => {
            return Ok((refused_record(rel, path, bytes, &err, started), None));
        }
    };
    let container = opened.facts.container.clone();

    // 2. Limbs: every AUDIO/VIDEO track gets its decoder attempt
    //    through the player's real provider (the matrix wants
    //    per-track truth, not first-of-kind only). Subtitle/data
    //    facts tracks are skipped by design — the engine's wired-limb
    //    set is the wrapper's `start()` answer (audio/video only).
    let facts = Arc::new(opened.facts.clone());
    let provider = HostDecoderProvider::new(facts.clone());
    let mut limbs: Vec<Limb> = Vec::new();
    let mut refused: Vec<TrackOutcome> = Vec::new();
    let mut first_audio_seen = false;
    let mut skipped_non_av_tracks: u32 = 0;
    for track in &facts.tracks {
        if !matches!(track.kind, MediaKind::Audio | MediaKind::Video) {
            skipped_non_av_tracks = skipped_non_av_tracks.saturating_add(1);
            continue;
        }
        let config = DecoderConfig::new(
            track.codec.clone(),
            GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            None,
        );
        let is_first_audio = matches!(track.kind, MediaKind::Audio) && !first_audio_seen;
        match provider.decoder_for(track, &config) {
            Ok(decoder) => {
                if matches!(track.kind, MediaKind::Audio) {
                    first_audio_seen = true;
                }
                limbs.push(Limb {
                    track: track.id,
                    decoder,
                    packets: 0,
                    frames: 0,
                    concealed: 0,
                    consecutive: 0,
                    fatal: None,
                    audio: if matches!(track.kind, MediaKind::Audio) {
                        Some(AudioDigestSink::new(dump_audio && is_first_audio))
                    } else {
                        None
                    },
                    video: CountSink::default(),
                    first_audio: is_first_audio,
                });
            }
            Err(err) => {
                refused.push(TrackOutcome::NoDecoder {
                    codec: track.codec.as_str().to_string(),
                    message: err.to_string(),
                });
            }
        }
    }

    // 3. The event loop (packets routed per track; per-frame damage
    //    concealed; the budget exhaustion is a typed fatal).
    let mut demux_recovered: u64 = 0;
    let mut demux_skip_bytes: u64 = 0;
    let mut demux_fatal: Option<String> = None;
    loop {
        match opened.demuxer.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                let track = packet.track;
                if let Some(limb) = limbs.iter_mut().find(|limb| limb.track == track)
                    && limb.fatal.is_none()
                {
                    limb.feed(packet);
                }
                // Packets of refused/unwired tracks are dropped (the
                // NoDecoder rows already named them).
            }
            Ok(DemuxEvent::Recovered { skipped, .. }) => {
                demux_recovered += 1;
                demux_skip_bytes = demux_skip_bytes.saturating_add(skipped);
            }
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(error) => {
                demux_fatal = Some(error.to_string());
                break;
            }
        }
    }
    for limb in limbs.iter_mut() {
        if limb.fatal.is_none() {
            limb.drain();
        }
    }

    // 4. The record: per-track outcomes, the overall verdict.
    let mut tracks: Vec<TrackOutcome> = refused;
    let mut dump: Option<DecodeDump> = None;
    let mut all_decoded = true;
    for limb in limbs {
        let fatal = limb.fatal.clone();
        let decoded = TrackOutcome::Decoded {
            codec: codec_of(&facts, limb.track).unwrap_or_default(),
            packets: limb.packets,
            frames: limb.frames,
            concealed: limb.concealed,
            pcm_md5: limb.audio.as_ref().map(AudioDigestSink::digest_hex),
            pcm_bytes: limb.audio.as_ref().map(|sink| sink.bytes),
            sample: limb
                .audio
                .as_ref()
                .and_then(|sink| sink.sample)
                .map(String::from),
            format_changes: limb.audio.as_ref().map_or(0, |sink| sink.format_changes),
        };
        if let Some(audio) = limb.audio
            && limb.first_audio
            && audio.bytes > 0
        {
            let md5_hex = audio.digest_hex();
            dump = Some(DecodeDump {
                pcm: audio.payload.unwrap_or_default(),
                md5_hex,
                sample: audio.sample.unwrap_or(""),
            });
        }
        if let Some(message) = fatal {
            // A decode-stage fatal on an otherwise-open file: record
            // it as the honest NoDecoder-shaped refusal (the limb
            // produced no complete decode).
            tracks.push(TrackOutcome::NoDecoder {
                codec: codec_of(&facts, limb.track).unwrap_or_default(),
                message: format!("decode fatal: {message}"),
            });
            all_decoded = false;
        } else {
            tracks.push(decoded);
        }
    }
    let outcome = match demux_fatal {
        Some(message) => FileOutcome::DemuxFatal { message },
        None => {
            if all_decoded
                && tracks
                    .iter()
                    .all(|t| matches!(t, TrackOutcome::Decoded { .. }))
            {
                FileOutcome::Decoded
            } else {
                FileOutcome::DecodedPartial
            }
        }
    };
    Ok((
        FileRecord {
            rel: rel.to_string(),
            path: path.display().to_string(),
            bytes,
            outcome,
            probe_format: Some(container),
            tracks,
            demux_recovered,
            demux_skip_bytes,
            skipped_non_av_tracks,
            decode_s: started.elapsed().as_secs_f64(),
        },
        dump,
    ))
}

/// The registry descriptor of one track id (from the facts).
fn codec_of(facts: &lumen_cli::demux_host::MediaFacts, track: TrackId) -> Option<String> {
    facts
        .tracks
        .iter()
        .find(|t| t.id == track)
        .map(|t| t.codec.as_str().to_string())
}

/// A record for an open-path refusal (stage-tagged).
fn refused_record(
    rel: &str,
    path: &Path,
    bytes: u64,
    err: &commands::CommandError,
    started: Instant,
) -> FileRecord {
    let outcome = match err.stage {
        commands::Stage::Probe => FileOutcome::RefusedProbe {
            message: err.message.clone(),
        },
        commands::Stage::Demux => FileOutcome::RefusedDemux {
            message: err.message.clone(),
        },
        _ => FileOutcome::RefusedInput {
            message: err.message.clone(),
        },
    };
    FileRecord {
        rel: rel.to_string(),
        path: path.display().to_string(),
        bytes,
        outcome,
        probe_format: None,
        tracks: Vec::new(),
        demux_recovered: 0,
        demux_skip_bytes: 0,
        skipped_non_av_tracks: 0,
        decode_s: started.elapsed().as_secs_f64(),
    }
}
