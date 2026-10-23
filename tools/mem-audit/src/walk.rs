//! The decode-walk driver (card W5-15): open → demux → decode to
//! null over the REAL first-vertical engine crates, with the counting
//! allocator's windows bracketing each phase.
//!
//! The pipeline pieces and their order mirror apps/cli's `decode-bench`
//! (card W4-R2) exactly — `input` file access → `media-probe` →
//! `demux`'s AVI demuxer → `codecs`' MJPEG/PCM decoders behind
//! `codec-api`'s [`Decoder`] contract — because the audit must measure
//! the same code the player runs. This module is a direct engine
//! driver (no command layer, no engine, no queues): the memory
//! behavior of open + demux + decode in isolation.
//!
//! # Window discipline (the honesty contract)
//!
//! - Driver setup (path canonicalization, `file:` URI building) runs
//!   BEFORE the first window — it is host boilerplate, not engine
//!   memory.
//! - The **open window** covers access → probe → demuxer start →
//!   strf harvest → decoder selection/configure: the one-time,
//!   per-stream cost.
//! - The **walk window** covers the packet loop, the drains, and the
//!   teardown drops (every engine object is dropped before `close`,
//!   so the phase's `dealloc` side is complete): the steady-state
//!   path the D-1 ratios divide by.
//! - Inside a window the driver performs NO allocation of its own —
//!   `u64` counters on the stack, packets moved by value. All
//!   formatting (error strings, the report line) happens after the
//!   windows close: whatever the counters saw in a window is the
//!   audited code's. (The one exception is the concealment-budget
//!   fatal, whose minted error allocates on a rare terminal path —
//!   the file's verdict is `Fatal` there, and its ratios are flagged
//!   as such in the report.)
//!
//! # Per-frame damage (mirrored from the engine's rule)
//!
//! `Corrupt` and `Decoder { Bitstream }` errors are per-frame damage:
//! concealed and counted, exactly like `playback-engine`'s
//! `per_frame_damage` (the rule `apps/cli`'s bench mirrors too), with
//! the same 1000-consecutive budget — a stream whose every packet
//! fails is not being measured, it is broken.

use std::path::Path;

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink, ThreadBudget};
use demux::avi::AviDemuxer;
use demux::avi::header::StreamFormat;
use media_types::{EngineError, ErrorKind, Frame, GenerationId, MediaKind, Packet, TrackId};
use plugin_api::{Access, DemuxEvent, Demuxer};

use crate::allocator::{Phase, Window};

/// The maximum run of consecutive concealed per-frame damage before
/// the walk gives up (mirrors `playback-engine`'s
/// `VIDEO_CONCEALMENT_BUDGET` / bench.rs's `CONCEALMENT_BUDGET`).
const CONCEALMENT_BUDGET: u32 = 1000;

/// How one file's walk ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// EndOfStream reached, limbs drained.
    Decoded,
    /// A typed open-phase refusal (probe miss, non-AVI container,
    /// demuxer start refusal): an honest, bounded answer — no walk
    /// numbers exist for this file.
    Refused,
    /// A fatal mid-walk error (demux sticky fatal, concealment budget
    /// exhausted, non-per-frame decode error): partial counts are
    /// still reported.
    Fatal,
}

/// The per-file report (plain data — built without allocation inside
/// any window; strings are formatted after the windows close).
#[derive(Debug, Clone)]
pub struct WalkReport {
    /// End state.
    pub verdict: Verdict,
    /// The pipeline stage the walk stopped at (refusals/fatals only).
    pub stage: Option<String>,
    /// The typed error line (refusals/fatals only).
    pub error: Option<String>,
    /// File size in bytes (0 when the stat failed — the error line
    /// carries that truth).
    pub size: u64,
    /// Video packets fed to the decoder.
    pub v_pkts: u64,
    /// Audio packets fed to the decoder.
    pub a_pkts: u64,
    /// Video frames the decoder emitted.
    pub v_dec: u64,
    /// Audio frames the decoder emitted.
    pub a_dec: u64,
    /// Per-frame video damage concealed.
    pub v_concealed: u64,
    /// Per-frame audio damage concealed.
    pub a_concealed: u64,
    /// Demux `Recovered` events (resyncs past container damage).
    pub demux_recovered: u64,
    /// The open-phase window result.
    pub open: Phase,
    /// The walk-phase window result.
    pub walk: Phase,
    /// Demuxed track count (the coverage truth: how many tracks the
    /// walk saw vs the two first-of-kind limbs it decoded).
    pub tracks: usize,
    /// Which limbs decoded: "v+a", "v", "a", "none".
    pub limbs: &'static str,
    /// The container the probe named ("avi"; the refused format, or
    /// "-" when the walk never reached a container answer).
    pub container: String,
}

impl WalkReport {
    /// Packets fed to decoders — the per-frame ratio denominator
    /// (every fed packet exercised the hot path, decoded or not).
    #[must_use]
    pub fn fed_packets(&self) -> u64 {
        self.v_pkts + self.a_pkts
    }
}

/// Whether an error is per-frame damage (the engine's concealment
/// class) — the `per_frame_damage` rule of `playback-engine`'s
/// pipeline.rs, mirrored locally by bench.rs and here (the engine's
/// own is crate-private).
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
/// RFC 3986 form apps/cli's `demux_host::encode_path` uses (the
/// smallest set that round-trips `SourceUri::parse` + `input`'s
/// resolver; the audit's file set is ASCII).
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

/// Canonicalize a path into its `file:` URI (driver setup — outside
/// every window).
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
                "memaudit.path.not_found",
                media_types::error::AccessDenied::NotFound,
            ));
        }
        Err(_) => {
            return Err(inaccessible(
                "memaudit.path.permission",
                media_types::error::AccessDenied::PermissionDenied,
            ));
        }
    };
    if !canonical.is_file() {
        return Err(inaccessible(
            "memaudit.path.not_file",
            media_types::error::AccessDenied::NotFound,
        ));
    }
    let Some(text) = canonical.to_str() else {
        return Err(EngineError::new(
            media_types::PipelineStage::Open,
            ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("memaudit.path.not_utf8"),
        ));
    };
    media_types::SourceUri::parse(&format!("file://{}", encode_path(text)))
}

/// Configure one decoder for the walk (the bench pattern: the track's
/// own codec descriptor, the demuxer's FIRST generation, one thread,
/// no accelerator, no extradata — the strf facts went in through
/// `PcmDecoder::with_format`).
fn configure(
    decoder: &mut dyn Decoder,
    codec: &media_types::ids::CodecDescriptor,
) -> Result<(), EngineError> {
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
/// counters (stack arithmetic only — no allocation in the loop). In
/// `--demux-only` runs the decoder is `None`: the limb then only
/// counts the packets its track delivered (the drilldown that
/// isolates demux+pool allocations from decoder allocations).
struct Limb {
    decoder: Option<Box<dyn Decoder>>,
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
    /// Feed one packet: move, decode, count (the D-1 loop body).
    /// A counting-only limb (`--demux-only`) skips the decode — its
    /// allocations are pure demux+pool.
    fn feed(
        &mut self,
        packet: Packet,
        sink: &mut dyn FrameSink,
        consecutive: &mut u32,
    ) -> Result<(), EngineError> {
        self.pkts += 1;
        let Some(decoder) = self.decoder.as_mut() else {
            drop(packet);
            return Ok(());
        };
        match decoder.decode(packet, sink) {
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
                        media_types::error::ErrorContext::new("memaudit.concealment.budget"),
                    ));
                }
            }
            Err(error) => return Err(error),
        }
        Ok(())
    }

    /// Drain at end-of-stream (stateless codecs answer immediately;
    /// counting-only limbs have nothing to drain).
    fn drain(&mut self, sink: &mut dyn FrameSink) {
        if let Some(decoder) = self.decoder.as_mut()
            && let Ok(DecodeOutcome::Emitted { frames }) = decoder.flush(sink)
        {
            self.decoded += u64::from(frames);
        }
    }

    /// The limb's counters as plain data (read before teardown).
    fn stats(&self) -> (u64, u64, u64) {
        (self.pkts, self.decoded, self.concealed)
    }
}

/// The decode-to-null sink: accepts and drops every frame (this
/// audit's `-f null -`). No per-frame work at all.
struct NullSink;

impl FrameSink for NullSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        Ok(())
    }
}

/// What a successful open produced (all engine objects alive, ready
/// for the walk window).
struct Opened {
    demuxer: AviDemuxer,
    video: Option<Limb>,
    audio: Option<Limb>,
    tracks: usize,
    container: String,
}

/// A mid-walk fatal: the typed error plus which stage stopped.
struct Fatal(EngineError, &'static str);

/// The open phase: access → probe → demuxer start → limb selection.
/// Runs inside the open window; every allocation it makes is engine
/// (or engine-adjacent configure) work. `demux_only` builds
/// counting-only limbs (no decoders — the drilldown that isolates
/// demux+pool allocations).
fn open(uri: &media_types::SourceUri, demux_only: bool) -> Result<Opened, EngineError> {
    let access = input::FileAccess::new();
    let mut stream = access.open(uri)?;
    let registry = media_probe::ProbeRegistry::with_builtins();
    let config = media_probe::ProbeConfig::default();
    let report = media_probe::probe_stream(&registry, &mut *stream, &config)?;
    let container = report.format.as_str().to_string();
    if container != "avi" {
        // The first vertical carries exactly one demuxer (D-018) —
        // the same refusal `apps/cli`'s open path answers with.
        return Err(EngineError::new(
            media_types::PipelineStage::Probe,
            ErrorKind::Unsupported {
                format: Some(media_types::error::FormatInfo {
                    name: Some(container),
                    sniffed: Vec::new(),
                }),
            },
            media_types::error::ErrorContext::new("memaudit.container.no_demuxer"),
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
                let decoder = if demux_only {
                    None
                } else {
                    let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
                    configure(&mut decoder, &track.codec)?;
                    Some(Box::new(decoder) as Box<dyn Decoder>)
                };
                video = Some(Limb {
                    decoder,
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
                let decoder = if demux_only {
                    None
                } else {
                    let mut decoder = codecs::audio::pcm::PcmDecoder::with_format(
                        u32::from(channels),
                        samples_per_sec,
                    )?;
                    configure(&mut decoder, &track.codec)?;
                    Some(Box::new(decoder) as Box<dyn Decoder>)
                };
                audio = Some(Limb {
                    decoder,
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

/// The packet loop + drains (inside the walk window). Returns the
/// fatal, if one struck.
fn run_loop(
    demuxer: &mut AviDemuxer,
    video: &mut Option<Limb>,
    audio: &mut Option<Limb>,
    sink: &mut dyn FrameSink,
    demux_recovered: &mut u64,
) -> Option<Fatal> {
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
                    return Some(Fatal(error, "decode"));
                }
                // Packets of unbenched tracks drop here (the tracks
                // count in the report says how many there were).
            }
            Ok(DemuxEvent::Recovered { .. }) => {
                *demux_recovered += 1;
            }
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(error) => return Some(Fatal(error, "demux")),
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

/// The format name a probe-stage refusal carries (for the report's
/// container column; "-" when the error names none).
fn probe_format_of(error: &EngineError) -> String {
    if let ErrorKind::Unsupported { format: Some(info) } = &error.kind {
        return info.name.clone().unwrap_or_else(|| "-".to_string());
    }
    "-".to_string()
}

/// The refusal report: open-phase numbers only, an honest zeroed walk
/// phase (formatted after the windows closed).
fn refused_report(size: u64, error: EngineError, container: &str) -> WalkReport {
    let stage = error.stage.to_string();
    WalkReport {
        verdict: Verdict::Refused,
        stage: Some(stage),
        error: Some(error.to_string()),
        size,
        v_pkts: 0,
        a_pkts: 0,
        v_dec: 0,
        a_dec: 0,
        v_concealed: 0,
        a_concealed: 0,
        demux_recovered: 0,
        open: Phase {
            alloc_bytes: 0,
            alloc_calls: 0,
            dealloc_bytes: 0,
            dealloc_calls: 0,
            peak_live: 0,
            baseline_live: 0,
        },
        walk: Phase {
            alloc_bytes: 0,
            alloc_calls: 0,
            dealloc_bytes: 0,
            dealloc_calls: 0,
            peak_live: 0,
            baseline_live: 0,
        },
        tracks: 0,
        limbs: "none",
        container: container.to_string(),
    }
}

/// Run one file's full walk (decode included) — the plain form the
/// tests use (`main` always goes through [`walk_with`]).
#[cfg(test)]
#[must_use]
pub fn walk(path: &Path) -> WalkReport {
    walk_with(path, false)
}

/// Run one file's walk: open (windowed), loop (windowed), teardown
/// inside the walk window, report formatted outside the windows.
/// `demux_only` builds counting-only limbs (the drilldown mode —
/// see `Limb`).
pub fn walk_with(path: &Path, demux_only: bool) -> WalkReport {
    // ---- Driver setup (outside every window) ------------------------
    let size = std::fs::metadata(path).map_or(0, |meta| meta.len());
    let uri = match file_uri(path) {
        Ok(uri) => uri,
        Err(error) => return refused_report(size, error, "-"),
    };

    // ---- Open window: access → probe → demuxer start → limbs -------
    let open_window = Window::open();
    let opened = open(&uri, demux_only);
    let open_phase = open_window.close();
    let opened = match opened {
        Ok(opened) => opened,
        Err(error) => {
            let container = probe_format_of(&error);
            let mut report = refused_report(size, error, &container);
            report.open = open_phase;
            return report;
        }
    };

    // ---- Walk window: packet loop → drains → teardown drops ---------
    let walk_window = Window::open();
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
    // Read the limb counters, then drop everything INSIDE the window
    // (limbs, sink, demuxer + its payload pool) so the walk phase's
    // dealloc side is complete.
    let (v_pkts, v_dec, v_concealed) = video.as_ref().map_or((0, 0, 0), Limb::stats);
    let (a_pkts, a_dec, a_concealed) = audio.as_ref().map_or((0, 0, 0), Limb::stats);
    let limbs = match (&video, &audio) {
        (Some(_), Some(_)) => "v+a",
        (Some(_), None) => "v",
        (None, Some(_)) => "a",
        (None, None) => "none",
    };
    drop(video);
    drop(audio);
    drop(demuxer);
    let walk_phase = walk_window.close();

    // ---- Report (outside every window; formatting is free) ---------
    let (verdict, stage, error) = match fatal {
        None => (Verdict::Decoded, None, None),
        Some(Fatal(error, stage)) => (
            Verdict::Fatal,
            Some(stage.to_string()),
            Some(error.to_string()),
        ),
    };
    WalkReport {
        verdict,
        stage,
        error,
        size,
        v_pkts,
        a_pkts,
        v_dec,
        a_dec,
        v_concealed,
        a_concealed,
        demux_recovered,
        open: open_phase,
        walk: walk_phase,
        tracks: opened.tracks,
        limbs,
        container: opened.container,
    }
}
