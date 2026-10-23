//! The one-file worker (card 7-d): probe → demux → decode-to-null
//! over the REAL first-vertical engine crates, printing exactly one
//! `SWEEP\t...` marker line as its last act.
//!
//! The pipeline pieces and their order mirror tools/mem-audit's
//! `walk.rs` and apps/cli's `decode-bench` (card W4-R2) exactly —
//! `input` file access → `media-probe` → `demux`'s AVI demuxer →
//! `codecs`' MJPEG/PCM decoders behind `codec-api`'s [`Decoder`]
//! contract — because the sweep must fire the same code the player
//! runs. This module is a direct engine driver (no command layer,
//! no engine, no queues).
//!
//! # No catch, by design
//!
//! This worker deliberately does NOT wrap the engine in
//! `catch_unwind`: an unwound panic is exactly the signal the
//! census wants to reach the supervisor as a process death (the
//! supervisor classifies it). `catch_unwind` here would also be a
//! lie for the harder cases — aborts (`panic=abort`, OOM, stack
//! overflow) and hangs are invisible to it — so the process
//! boundary is the only honest isolation (see main.rs).
//!
//! # Per-frame damage (mirrored from the engine's rule)
//!
//! `Corrupt` and `Decoder { Bitstream }` errors are per-frame
//! damage: concealed and counted, exactly like `playback-engine`'s
//! `per_frame_damage`, with the same 1000-consecutive budget — a
//! stream whose every packet fails is not being swept, it is
//! broken.

use std::path::Path;

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink, ThreadBudget};
use demux::avi::AviDemuxer;
use demux::avi::header::StreamFormat;
use media_types::ids::CodecDescriptor;
use media_types::{EngineError, ErrorKind, Frame, GenerationId, MediaKind, Packet, TrackId};
use plugin_api::{Access, DemuxEvent, Demuxer};

use crate::outcome::sanitize;

/// The maximum run of consecutive concealed per-frame damage before
/// the walk gives up (mirrors `playback-engine`'s
/// `VIDEO_CONCEALMENT_BUDGET` / mem-audit's walk.rs).
const CONCEALMENT_BUDGET: u32 = 1000;

/// One file's walk, as plain data (formatted after the fact; the
/// marker line is built from this).
pub struct WalkResult {
    /// `ok` | `unsupported` | `corrupt` | `error`.
    pub status: &'static str,
    /// The `PipelineStage` the file stopped at (`"-"` for ok).
    pub stage: String,
    /// Short `ErrorKind` tag (`"-"` for ok).
    pub kind: String,
    /// Stable dotted context code (`"-"` for ok).
    pub code: String,
    /// Video packets fed / video frames decoded.
    pub v_pkts: u64,
    pub v_dec: u64,
    /// Audio packets fed / audio frames decoded.
    pub a_pkts: u64,
    pub a_dec: u64,
    /// Per-frame video damage concealed.
    pub v_concealed: u64,
    /// Per-frame audio damage concealed.
    pub a_concealed: u64,
    /// Demux `Recovered` events (container resyncs past damage).
    pub demux_recovered: u64,
    /// Demuxed track count (coverage truth vs the two limbs).
    pub tracks: usize,
    /// Which limbs decoded: "v+a" | "v" | "a" | "none".
    pub limbs: &'static str,
    /// The container the probe named.
    pub container: String,
    /// The error `Display` (sanitized; `"-"` for ok).
    pub detail: String,
}

/// The `ok`/`stopped` skeleton with zeroed counters.
fn result_skeleton(status: &'static str, error: Option<&EngineError>) -> WalkResult {
    let (stage, kind, code, detail) = match error {
        None => (
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
        ),
        Some(error) => (
            error.stage.to_string(),
            kind_tag(error),
            error.context.code.clone(),
            sanitize(&error.to_string()),
        ),
    };
    WalkResult {
        status,
        stage,
        kind,
        code,
        v_pkts: 0,
        v_dec: 0,
        a_pkts: 0,
        a_dec: 0,
        v_concealed: 0,
        a_concealed: 0,
        demux_recovered: 0,
        tracks: 0,
        limbs: "none",
        container: "-".to_string(),
        detail,
    }
}

/// The sweep status for a typed engine error (engine-side
/// classification; the card's `unsupported`/`dirty-reject`/
/// `typed-error` split).
fn status_of(error: &EngineError) -> &'static str {
    match &error.kind {
        ErrorKind::Unsupported { .. } => "unsupported",
        ErrorKind::Corrupt { .. } => "corrupt",
        _ => "error",
    }
}

/// A short stable tag for the `ErrorKind` discriminant (the full
/// `Debug` payload can be large; the census only needs the shape).
fn kind_tag(error: &EngineError) -> String {
    match &error.kind {
        ErrorKind::Unsupported { .. } => "unsupported",
        ErrorKind::Corrupt { .. } => "corrupt",
        ErrorKind::Inaccessible { .. } => "inaccessible",
        ErrorKind::Network { .. } => "network",
        ErrorKind::Decoder { .. } => "decoder",
        ErrorKind::Device { .. } => "device",
        ErrorKind::ResourceLimit { .. } => "resource-limit",
    }
    .to_string()
}

/// Whether an error is per-frame damage (the engine's concealment
/// class) — `playback-engine`'s `per_frame_damage` rule, mirrored
/// by mem-audit's walk.rs (the engine's own is crate-private).
fn per_frame_damage(error: &EngineError) -> bool {
    matches!(
        &error.kind,
        ErrorKind::Corrupt { .. }
            | ErrorKind::Decoder {
                cause: media_types::error::DecodeCause::Bitstream
            }
    )
}

/// The percent-encoding set for a `file:` URI path — the minimal
/// RFC 3986 form apps/cli's `demux_host::encode_path` uses (same
/// set as mem-audit's walk.rs).
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

/// Canonicalize a path into its `file:` URI (driver setup).
fn file_uri(path: &Path) -> Result<media_types::SourceUri, EngineError> {
    let inaccessible = |code: &'static str, reason| {
        EngineError::new(
            media_types::PipelineStage::Open,
            ErrorKind::Inaccessible { reason },
            media_types::error::ErrorContext::new(code),
        )
    };
    let canonical = match std::fs::canonicalize(path) {
        Ok(canonical) => canonical,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(inaccessible(
                "sweep.path.not_found",
                media_types::error::AccessDenied::NotFound,
            ));
        }
        Err(_) => {
            return Err(inaccessible(
                "sweep.path.permission",
                media_types::error::AccessDenied::PermissionDenied,
            ));
        }
    };
    if !canonical.is_file() {
        return Err(inaccessible(
            "sweep.path.not_file",
            media_types::error::AccessDenied::NotFound,
        ));
    }
    let Some(text) = canonical.to_str() else {
        return Err(EngineError::new(
            media_types::PipelineStage::Open,
            ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("sweep.path.not_utf8"),
        ));
    };
    media_types::SourceUri::parse(&format!("file://{}", encode_path(text)))
}

/// Configure one decoder for the walk (the bench pattern: the
/// track's own codec descriptor, the demuxer's FIRST generation,
/// one thread, no accelerator, no extradata).
fn configure(decoder: &mut dyn Decoder, codec: &CodecDescriptor) -> Result<(), EngineError> {
    let config = DecoderConfig::new(
        codec.clone(),
        GenerationId::FIRST,
        ThreadBudget::default(),
        None,
        None,
    );
    decoder.configure(&config)
}

/// One decoded limb: the decoder behind the contract plus its
/// counters (stack arithmetic only).
struct Limb {
    decoder: Box<dyn Decoder>,
    /// The demuxed track this limb consumes.
    track: TrackId,
    /// Packets fed.
    pkts: u64,
    /// Frames the decoder emitted.
    decoded: u64,
    /// Per-frame damage concealed (kept going).
    concealed: u64,
}

impl Limb {
    /// Feed one packet: move, decode, count.
    fn feed(
        &mut self,
        packet: Packet,
        sink: &mut dyn FrameSink,
        consecutive: &mut u32,
    ) -> Result<(), EngineError> {
        self.pkts += 1;
        match self.decoder.decode(packet, sink) {
            Ok(DecodeOutcome::Emitted { frames }) => {
                self.decoded += u64::from(frames);
                *consecutive = 0;
            }
            Ok(DecodeOutcome::Recovered { .. }) => {
                *consecutive = consecutive.saturating_add(1);
            }
            Ok(DecodeOutcome::NeedMoreInput | DecodeOutcome::Drained) => {}
            Err(error) if per_frame_damage(&error) => {
                self.concealed += 1;
                *consecutive = consecutive.saturating_add(1);
                if *consecutive >= CONCEALMENT_BUDGET {
                    return Err(EngineError::new(
                        media_types::PipelineStage::Decode,
                        ErrorKind::ResourceLimit {
                            limit: media_types::error::LimitKind::Memory,
                            budget: u64::from(CONCEALMENT_BUDGET),
                            attempted: u64::from(*consecutive),
                        },
                        media_types::error::ErrorContext::new("sweep.concealment.budget"),
                    ));
                }
            }
            Err(error) => return Err(error),
        }
        Ok(())
    }

    /// Drain at end-of-stream (stateless codecs answer immediately).
    fn drain(&mut self, sink: &mut dyn FrameSink) {
        if let Ok(DecodeOutcome::Emitted { frames }) = self.decoder.flush(sink) {
            self.decoded += u64::from(frames);
        }
    }
}

/// The decode-to-null sink: accepts and drops every frame (the
/// sweep's `-f null -`). No per-frame work at all.
struct NullSink;

impl FrameSink for NullSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        Ok(())
    }
}

/// What a successful open produced.
struct Opened {
    demuxer: AviDemuxer,
    video: Option<Limb>,
    audio: Option<Limb>,
    tracks: usize,
    container: String,
}

/// The open phase: access → probe → demuxer start → limb selection
/// (the mem-audit walk pattern, simplified: no allocator windows).
fn open(uri: &media_types::SourceUri) -> Result<Opened, EngineError> {
    let access = input::FileAccess::new();
    let mut stream = access.open(uri)?;
    let registry = media_probe::ProbeRegistry::with_builtins();
    let config = media_probe::ProbeConfig::default();
    let report = media_probe::probe_stream(&registry, &mut *stream, &config)?;
    let container = report.format.as_str().to_string();
    if container != "avi" {
        // The first vertical carries exactly one demuxer (D-018) —
        // the same refusal apps/cli's open path answers with. This
        // is the expected honest outcome for every non-AVI corpus
        // class until the demuxer matrix grows.
        return Err(EngineError::new(
            media_types::PipelineStage::Probe,
            ErrorKind::Unsupported {
                format: Some(media_types::error::FormatInfo {
                    name: Some(container),
                    sniffed: Vec::new(),
                }),
            },
            media_types::error::ErrorContext::new("sweep.container.no_demuxer"),
        ));
    }
    let mut demuxer = AviDemuxer::new(stream)?;
    let tracks = demuxer.start()?;
    // Limb selection over the strf facts (the bench pattern): the
    // first video (mjpeg) and first audio (pcm) streams the v0
    // decoders cover; other tracks are counted, not decoded.
    let mut video: Option<Limb> = None;
    let mut audio: Option<Limb> = None;
    for track in &tracks {
        let Some(format) = demuxer.stream_format(track.id) else {
            continue;
        };
        match (track.kind, format) {
            (MediaKind::Video, StreamFormat::MotionJpeg { .. }) if video.is_none() => {
                let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
                configure(&mut decoder, &track.codec)?;
                video = Some(Limb {
                    decoder: Box::new(decoder),
                    track: track.id,
                    pkts: 0,
                    decoded: 0,
                    concealed: 0,
                });
            }
            (
                MediaKind::Audio,
                StreamFormat::Pcm {
                    channels,
                    samples_per_sec,
                    ..
                },
            ) if audio.is_none() => {
                let mut decoder = codecs::audio::pcm::PcmDecoder::with_format(
                    u32::from(channels),
                    samples_per_sec,
                )?;
                configure(&mut decoder, &track.codec)?;
                audio = Some(Limb {
                    decoder: Box::new(decoder),
                    track: track.id,
                    pkts: 0,
                    decoded: 0,
                    concealed: 0,
                });
            }
            _ => {}
        }
    }
    Ok(Opened {
        demuxer,
        video,
        audio,
        tracks: tracks.len(),
        container: "avi".to_string(),
    })
}

/// The packet loop + drains. Returns the fatal, if one struck.
fn run_loop(
    demuxer: &mut AviDemuxer,
    video: &mut Option<Limb>,
    audio: &mut Option<Limb>,
    sink: &mut dyn FrameSink,
    demux_recovered: &mut u64,
) -> Option<EngineError> {
    let mut consecutive: u32 = 0;
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                // Route by track (the packet moves whole into the
                // limb — nothing else happens per packet).
                let track = packet.track;
                let routed = match (video.as_mut(), audio.as_mut()) {
                    (Some(limb), _) if limb.track == track => Some(limb),
                    (_, Some(limb)) if limb.track == track => Some(limb),
                    _ => None,
                };
                if let Some(limb) = routed
                    && let Err(error) = limb.feed(packet, sink, &mut consecutive)
                {
                    return Some(error);
                }
                // Packets of unlimbed tracks drop here (the tracks
                // count in the result says how many there were).
            }
            Ok(DemuxEvent::Recovered { .. }) => {
                *demux_recovered += 1;
            }
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(error) => return Some(error),
        }
    }
    if let Some(limb) = video.as_mut() {
        limb.drain(sink);
    }
    if let Some(limb) = audio.as_mut() {
        limb.drain(sink);
    }
    None
}

/// Run one file's full walk and return the plain-data result. All
/// failure modes are DATA (typed errors) — the only way out that is
/// not data is the process dying, which is the supervisor's census
/// (see the module docs).
#[must_use]
pub fn walk(path: &Path) -> WalkResult {
    let uri = match file_uri(path) {
        Ok(uri) => uri,
        Err(error) => {
            let mut result = result_skeleton(status_of(&error), Some(&error));
            result.container = probe_format_of(&error);
            return result;
        }
    };
    let opened = match open(&uri) {
        Ok(opened) => opened,
        Err(error) => {
            let mut result = result_skeleton(status_of(&error), Some(&error));
            result.container = probe_format_of(&error);
            return result;
        }
    };
    let mut demuxer = opened.demuxer;
    let mut video = opened.video;
    let mut audio = opened.audio;
    let mut sink = NullSink;
    let mut demux_recovered: u64 = 0;
    let fatal = run_loop(
        &mut demuxer,
        &mut video,
        &mut audio,
        &mut sink,
        &mut demux_recovered,
    );
    let (v_pkts, v_dec, v_concealed) = video
        .as_ref()
        .map_or((0, 0, 0), |limb| (limb.pkts, limb.decoded, limb.concealed));
    let (a_pkts, a_dec, a_concealed) = audio
        .as_ref()
        .map_or((0, 0, 0), |limb| (limb.pkts, limb.decoded, limb.concealed));
    let limbs = match (&video, &audio) {
        (Some(_), Some(_)) => "v+a",
        (Some(_), None) => "v",
        (None, Some(_)) => "a",
        (None, None) => "none",
    };
    let mut result = match fatal {
        None => result_skeleton("ok", None),
        Some(error) => result_skeleton(status_of(&error), Some(&error)),
    };
    result.v_pkts = v_pkts;
    result.v_dec = v_dec;
    result.a_pkts = a_pkts;
    result.a_dec = a_dec;
    result.v_concealed = v_concealed;
    result.a_concealed = a_concealed;
    result.demux_recovered = demux_recovered;
    result.tracks = opened.tracks;
    result.limbs = limbs;
    result.container = opened.container;
    result
}

/// The format name a probe-stage refusal carries (for the result's
/// container column; "-" when the error names none).
fn probe_format_of(error: &EngineError) -> String {
    if let ErrorKind::Unsupported { format: Some(info) } = &error.kind {
        return info.name.clone().unwrap_or_else(|| "-".to_string());
    }
    "-".to_string()
}

/// Render the `SWEEP\t...` marker line (the supervisor's protocol;
/// see outcome::parse_worker_line for the field order).
#[must_use]
pub fn marker_line(result: &WalkResult) -> String {
    format!(
        "SWEEP\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        result.status,
        result.stage,
        result.kind,
        result.code,
        result.v_pkts,
        result.v_dec,
        result.a_pkts,
        result.a_dec,
        result.v_concealed,
        result.a_concealed,
        result.demux_recovered,
        result.tracks,
        result.limbs,
        result.container,
        result.detail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_path_percent_encodes_spaces_but_keeps_slashes() {
        assert_eq!(encode_path("/tmp/a b/c.avi"), "/tmp/a%20b/c.avi");
    }

    #[test]
    fn missing_file_is_a_typed_inaccessible_refusal_not_a_crash() {
        let result = walk(Path::new("/nonexistent/sweep-probe-missing.avi"));
        assert_eq!(result.status, "error");
        assert_eq!(result.kind, "inaccessible");
        assert_eq!(result.stage, "open");
        assert_eq!(result.limbs, "none");
    }

    #[test]
    fn marker_line_shape_matches_the_parser() {
        let mut result = result_skeleton("ok", None);
        result.v_pkts = 3;
        result.v_dec = 3;
        result.limbs = "v";
        result.container = "avi".to_string();
        let line = marker_line(&result);
        let parsed = crate::outcome::parse_worker_line(&line);
        let Some(parsed) = parsed else {
            assert!(false, "marker line must round-trip the parser");
            return;
        };
        assert_eq!(parsed.status, "ok");
        assert_eq!(parsed.v_pkts, 3);
        assert_eq!(parsed.container, "avi");
    }
}
