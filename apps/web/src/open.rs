//! The open path (card 7.H1): blob bytes → probe → the AVI container
//! wiring → the engine's [`SessionConfig`] inputs.
//!
//! This mirrors the CLI's `demux_host::open_source` +
//! `containers::avi` composition (the established open-path shape,
//! `docs/plans/open-path-wiring.md` §4.2/§4.3) over the web host's
//! in-memory sources. The lumen-cli crate itself cannot be a
//! dependency here: it wires `codecs` unconditionally, and the
//! codecs crate does not build for `wasm32-unknown-unknown` (see
//! `Cargo.toml`'s header) — so the AVI wiring is mirrored, not
//! imported, and the mirror is pinned to the same probe→pre-pass→
//! wrapper shape.
//!
//! # Honest registry posture
//!
//! v1 wires exactly the first vertical's family (`avi`). Every other
//! probe id answers the CLI's own honest typed refusal — the format
//! is NAMED, stage `probe`, code `unsupported` — never a silent
//! fallback (and never a crash). The container-host registry pattern
//! (one module per family) lands with the H-pillars that need more
//! families.

use commands::catalog::types::SourceSpec;
use commands::{CommandError, ErrorCode, Stage};
use demux::avi::header::StreamFormat;
use demux::avi::AviDemuxer;
use media_clock::NANO_BASE;
use media_probe::{ProbeConfig, ProbeRegistry};
use media_types::ids::SourceUri;
use media_types::time::{Stamp, TimeSpan, Timestamp};
use media_types::{MediaKind, TimeBase, Track, TrackId};
use plugin_api::{Demuxer, SourceStream};

use crate::capabilities::HostCapabilities;
use crate::source::BlobRegistry;

/// Per-track facts the open path harvested (the CLI's
/// `demux_host::TrackFact`, mirrored — see the module docs).
#[derive(Debug, Clone)]
pub struct TrackFact {
    /// The track's id.
    pub id: TrackId,
    /// What the track carries.
    pub kind: MediaKind,
    /// The registry codec descriptor (`mjpeg`, `pcm_s16le`, …).
    pub codec: String,
    /// The track's timestamp base (one tick = num/den seconds).
    pub base: TimeBase,
    /// Declared span end in ticks (`None` when unknown).
    pub span_end_ticks: Option<i64>,
    /// PCM channels + rate, when the track is PCM.
    pub pcm: Option<(u32, u32)>,
    /// Video frame duration in ns (from the track base), when known.
    pub frame_ns: Option<u64>,
    /// Coded dimensions (video), when the container declares them.
    pub dimensions: Option<(u32, u32)>,
}

impl TrackFact {
    /// The track's declared extent in microseconds (`None` unknown).
    #[must_use]
    pub fn span_end_us(&self) -> Option<i64> {
        let ticks = self.span_end_ticks?;
        let stamp = Timestamp::new(ticks, self.base);
        let nanos = stamp.convert(&NANO_BASE).ok()?;
        Some(nanos.ticks / 1000)
    }

    /// The output geometry (channels + rate) for the audio limb —
    /// the PCM pair (the first vertical's only audio family).
    #[must_use]
    pub fn audio_geometry(&self) -> Option<(u32, u32)> {
        if self.kind == MediaKind::Audio {
            self.pcm
        } else {
            None
        }
    }
}

/// The harvested media description (the `media.get-info` backbone).
#[derive(Debug, Clone)]
pub struct MediaFacts {
    /// The demuxed track list (unfiltered).
    pub tracks: Vec<Track>,
    /// Per-track facts, parallel to `tracks`.
    pub facts: Vec<TrackFact>,
    /// The probe's format id (`avi`).
    pub container: String,
}

impl MediaFacts {
    /// The media duration in µs: the greatest declared track extent.
    #[must_use]
    pub fn duration_us(&self) -> Option<i64> {
        self.facts.iter().filter_map(TrackFact::span_end_us).max()
    }

    /// The fact row for one track id.
    #[must_use]
    pub fn fact_for(&self, id: TrackId) -> Option<&TrackFact> {
        self.facts.iter().find(|fact| fact.id == id)
    }
}

/// What the open path produced for the engine's
/// [`SessionConfig`](playback_engine::SessionConfig).
pub struct OpenedMedia {
    /// The wrapped demuxer (skip/selection wrapper).
    pub demuxer: Box<dyn Demuxer>,
    /// The harvested facts.
    pub facts: MediaFacts,
    /// The position the session anchors at (ns; `start_time` or 0).
    pub start_position_ns: i64,
    /// The resolved source URI.
    pub uri: SourceUri,
}

/// The web host's open options (the catalog `OpenOptions` the web
/// lane supports — see [`open`]'s refusals for the rest).
#[derive(Debug, Clone, Copy, Default)]
pub struct WebOpenOptions {
    /// `--no-video` (drop the video track at the wrapper).
    pub no_video: bool,
    /// `--no-audio` (drop the audio track at the wrapper).
    pub no_audio: bool,
}

/// Resolve one [`SourceSpec`] against the registry + capability
/// snapshot — the web host's source grammar (doc 09 §6.1/§6.3).
///
/// - `blob:<name>` [`SourceSpec::Url`]s resolve to the registry;
/// - `http(s)` answers the typed `unsupported-for-host` naming card
///   7.H4 (fetch + Range);
/// - `rtsp`/`udp`/`rtp` answer the typed blocked-capability refusal
///   (doc 09 §6.3 — no raw sockets in browsers);
/// - [`SourceSpec::File`] answers the typed no-filesystem refusal
///   (doc 09 §9.2 — user-selected blobs travel the no-upload path).
///
/// # Errors
/// The typed [`CommandError`] for every refusal above.
pub fn resolve_source(
    source: &SourceSpec,
    registry: &BlobRegistry,
    capabilities: &HostCapabilities,
) -> Result<SourceUri, CommandError> {
    let unsupported_for_host = |scheme: &str, detail: &str| {
        CommandError::new(
            Stage::Input,
            ErrorCode::UnsupportedForHost,
            format!("source scheme `{scheme}` is not available on this host: {detail}"),
        )
    };
    match source {
        SourceSpec::File { .. } => Err(CommandError::new(
            Stage::Input,
            ErrorCode::UnsupportedForHost,
            "the web host has no filesystem (doc 09 §9.2) — load the \
             user-selected file as a blob and open `blob:<name>`",
        )
        .with_detail("capability", "local-filesystem")),
        SourceSpec::Url { url } => {
            let scheme = url.split(':').next().unwrap_or("");
            match scheme {
                "blob" => {
                    let name = url.strip_prefix("blob:").unwrap_or("");
                    match registry.size_of(name) {
                        Some(_) => SourceUri::parse(url).map_err(|err| {
                            CommandError::new(
                                Stage::Input,
                                ErrorCode::InvalidArgument,
                                format!("the blob URL was rejected: {err}"),
                            )
                        }),
                        None => Err(CommandError::new(
                            Stage::Input,
                            ErrorCode::NotFound,
                            format!("no blob named `{name}` is loaded (send loadBlob first)"),
                        )),
                    }
                }
                "http" | "https" => Err(unsupported_for_host(
                    scheme,
                    "network streaming is card 7.H4 (fetch + Range seekability detection)",
                )
                .with_detail("capability", "http-streaming")),
                "rtsp" | "rtp" | "udp" => {
                    let row = capabilities
                        .blocked_row("rtsp-rtp-udp-access")
                        .map(|row| row.reason.as_str())
                        .unwrap_or("no raw sockets in browsers (doc 09 §6.3)");
                    Err(unsupported_for_host(scheme, row)
                        .with_detail("capability", "rtsp-rtp-udp-access"))
                }
                other => Err(unsupported_for_host(
                    other,
                    "no access adapter serves this scheme on the web host",
                )),
            }
        }
    }
}

/// The whole open path: resolve the source, probe, wire the AVI
/// family (or refuse honestly), harvest the facts, wrap the demuxer
/// in the skip/selection wrapper.
///
/// # Errors
/// Typed [`CommandError`]s at the honest stages: `input` (access),
/// `probe` (no/unknown signature or an unwired family), `demux`
/// (header parse, seek range).
pub fn open(
    source: &SourceSpec,
    registry: &BlobRegistry,
    capabilities: &HostCapabilities,
    options: WebOpenOptions,
) -> Result<OpenedMedia, CommandError> {
    let uri = resolve_source(source, registry, capabilities)?;
    let name = uri
        .as_str()
        .strip_prefix("blob:")
        .unwrap_or_default()
        .to_string();
    let mut stream = registry.open(&name).map_err(|err| {
        CommandError::new(
            Stage::Input,
            ErrorCode::Inaccessible,
            err.to_string(),
        )
    })?;
    // 1. Probe (the bounded, non-consuming window).
    let registry_probes = ProbeRegistry::with_builtins();
    let config = ProbeConfig::default();
    let report = media_probe::probe_stream(&registry_probes, &mut *stream, &config)
        .map_err(|err| wire_error(&err))?;
    let container = report.format.as_str().to_string();
    // 2. The per-family wiring: `avi` is v1's wired family; every
    //    other probe id is the honest named refusal (the CLI's own
    //    registry default arm, mirrored).
    if container != "avi" {
        return Err(CommandError::new(
            Stage::Probe,
            ErrorCode::Unsupported,
            format!(
                "format `{container}` probes but has no web-host wiring yet \
                 (v1 wires the first vertical's `avi` family; the \
                 container-host registry grows with the H-pillar cards)"
            ),
        )
        .with_detail("format", container));
    }
    // 3. The AVI wiring (the CLI's containers/avi.rs, mirrored).
    let mut demuxer = AviDemuxer::new(Box::new(stream)).map_err(|err| wire_error(&err))?;
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_facts(&demuxer, &tracks);
    let wrapper = SelectDemuxer::new(demuxer, options.no_video, options.no_audio);
    Ok(OpenedMedia {
        demuxer: Box::new(wrapper),
        facts,
        start_position_ns: 0,
        uri,
    })
}

/// Harvest the per-track facts from a started AVI demuxer (the
/// `strf` stream formats are the decoder-config seam: MJPEG
/// dimensions, PCM channels/rate).
fn harvest_facts(demuxer: &AviDemuxer, tracks: &[Track]) -> MediaFacts {
    let mut facts = Vec::with_capacity(tracks.len());
    for track in tracks {
        let (codec, pcm, frame_ns, dimensions) = match demuxer.stream_format(track.id) {
            Some(StreamFormat::MotionJpeg { width, height }) => {
                // One tick of the video base is one frame (AVI scale=1).
                let frame_ns = (u64::from(track.timing.base.num)).saturating_mul(1_000_000_000)
                    / u64::from(track.timing.base.den);
                ("mjpeg".to_string(), None, Some(frame_ns), Some((width, height)))
            }
            Some(stream_format @ StreamFormat::Pcm {
                channels,
                samples_per_sec,
                ..
            }) => {
                let codec = stream_format.codec_descriptor().to_string();
                (
                    codec,
                    Some((u32::from(channels), samples_per_sec)),
                    None,
                    None,
                )
            }
            None => ("unknown".to_string(), None, None, None),
        };
        let span_end_ticks = track
            .timing
            .span
            .end
            .timestamp()
            .ok()
            .map(|ts| ts.ticks);
        facts.push(TrackFact {
            id: track.id,
            kind: track.kind,
            codec,
            base: track.timing.base,
            span_end_ticks,
            pcm,
            frame_ns,
            dimensions,
        });
    }
    MediaFacts {
        tracks: tracks.to_vec(),
        facts,
        container: "avi".to_string(),
    }
}

/// The skip/selection wrapper (the CLI's `demux_host::SelectDemuxer`
/// mirrored at v1's scope: track-kind selection; the start-time skip
/// watermark is omitted — the web v1 opens at 0, and `start_time`
/// answers the typed refusal in the host's dispatch).
struct SelectDemuxer<D: Demuxer> {
    inner: D,
    disable_video: bool,
    disable_audio: bool,
}

impl<D: Demuxer> SelectDemuxer<D> {
    fn new(inner: D, disable_video: bool, disable_audio: bool) -> SelectDemuxer<D> {
        SelectDemuxer {
            inner,
            disable_video,
            disable_audio,
        }
    }
}

impl<D: Demuxer> Demuxer for SelectDemuxer<D> {
    fn start(&mut self) -> Result<Vec<Track>, media_types::EngineError> {
        let tracks = self.inner.start()?;
        Ok(tracks
            .into_iter()
            .filter(|track| match track.kind {
                MediaKind::Video => !self.disable_video,
                MediaKind::Audio => !self.disable_audio,
                _ => false,
            })
            .collect())
    }

    fn next_event(&mut self) -> Result<plugin_api::DemuxEvent, media_types::EngineError> {
        self.inner.next_event()
    }

    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, media_types::EngineError> {
        self.inner.seek(target)
    }

    fn seekability(&self) -> plugin_api::SeekSupport {
        self.inner.seekability()
    }
}

/// Bridge one engine error into the command layer's redaction-safe
/// form (the CLI's `demux_host::wire_error`, over the
/// playback-engine's published wire mappings — one implementation of
/// the stage/code mapping in the tree, re-used not re-invented).
#[must_use]
pub fn wire_error(error: &media_types::EngineError) -> CommandError {
    let full = error.to_string();
    let stage_prefix = format!("[{}] ", error.stage);
    let base = full.strip_prefix(&stage_prefix).unwrap_or(full.as_str());
    let message = match error.source.as_ref() {
        Some(root) => format!("{base}: {root}"),
        None => base.to_string(),
    };
    CommandError::new(
        playback_engine::event::wire_stage(error.stage),
        playback_engine::event::wire_error_code(&error.kind),
        message,
    )
    .with_detail("code", error.context.code.as_str())
}

/// A host-minted engine error (the CLI's `pipeline_host::host_error`
/// shape): stage + stable dotted code + the sentence in the chain
/// root, so [`wire_error`] surfaces it as the message.
pub(crate) fn host_error(
    stage: media_types::PipelineStage,
    code: &'static str,
    message: impl Into<String>,
) -> media_types::EngineError {
    media_types::EngineError::new(
        stage,
        media_types::ErrorKind::Unsupported { format: None },
        media_types::error::ErrorContext::new(code),
    )
    .with_source(std::io::Error::other(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::capabilities;
    use testkit::{assert_err, fixture_bytes, Must};

    /// The open path over the REAL first-vertical fixture: probe
    /// names `avi`, the facts carry the MJPEG dimensions and the PCM
     /// geometry, and the wrapper's track list is the full pair.
    #[test]
    fn open_first_vertical_fixture_harvests_facts() {
        let bytes = fixture_bytes("generated/valid/avi_mjpeg_pcm.avi").must();
        let mut registry = BlobRegistry::new();
        registry.load("fixture", bytes).must();
        let caps = capabilities(crate::capabilities::build_profile());
        let opened = open(
            &SourceSpec::Url {
                url: "blob:fixture".to_string(),
            },
            &registry,
            &caps,
            WebOpenOptions::default(),
        )
        .must();
        assert_eq!(opened.facts.container, "avi");
        assert_eq!(opened.facts.tracks.len(), 2, "one video + one audio");
        let video = opened
            .facts
            .facts
            .iter()
            .find(|fact| fact.kind == MediaKind::Video)
            .must();
        assert_eq!(video.codec, "mjpeg");
        assert!(video.dimensions.is_some());
        let audio = opened
            .facts
            .facts
            .iter()
            .find(|fact| fact.kind == MediaKind::Audio)
            .must();
        assert_eq!(audio.codec, "pcm_s16le");
        assert_eq!(audio.audio_geometry(), Some((2, 44_100)));
        assert!(opened.facts.duration_us().is_some());
        // The wrapper passes the full track pair through.
        let mut demuxer = opened.demuxer;
        let tracks = demuxer.start().must();
        assert_eq!(tracks.len(), 2);
    }

    /// `--no-audio` drops the audio track at the wrapper (the
    /// engine wires limbs only for what it sees).
    #[test]
    fn selection_flags_drop_their_kinds() {
        let bytes = fixture_bytes("generated/valid/avi_mjpeg_pcm.avi").must();
        let mut registry = BlobRegistry::new();
        registry.load("fixture", bytes).must();
        let caps = capabilities(crate::capabilities::build_profile());
        let opened = open(
            &SourceSpec::Url {
                url: "blob:fixture".to_string(),
            },
            &registry,
            &caps,
            WebOpenOptions {
                no_video: false,
                no_audio: true,
            },
        )
        .must();
        let mut demuxer = opened.demuxer;
        let tracks = demuxer.start().must();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].kind, MediaKind::Video);
    }

    /// The honest registry default: a non-AVI blob probes but is
    /// refused with the format NAMED (the typed probe-stage
    /// refusal, never a silent fallback).
    #[test]
    fn unwired_family_is_the_named_refusal() {
        // A WAV file probes as `wav` — wired in the CLI's registry,
        // not in the web v1's.
        let bytes = fixture_bytes("generated/valid/wav_pcm_s16le_44k1_st.wav").must();
        let mut registry = BlobRegistry::new();
        registry.load("fixture", bytes).must();
        let caps = capabilities(crate::capabilities::build_profile());
        let err = assert_err(
            open(
                &SourceSpec::Url {
                    url: "blob:fixture".to_string(),
                },
                &registry,
                &caps,
                WebOpenOptions::default(),
            ),
            "unwired family",
        );
        assert_eq!(err.stage, Stage::Probe);
        assert_eq!(err.code, ErrorCode::Unsupported);
        assert!(err.message.contains("format `wav`"), "{err}");
    }

    /// Garbage bytes are the probe's own honest `unsupported` (the
    /// sniffed-bytes refusal — stage `probe`).
    #[test]
    fn garbage_is_the_probe_refusal() {
        let mut registry = BlobRegistry::new();
        registry.load("garbage", vec![0u8; 4096]).must();
        let caps = capabilities(crate::capabilities::build_profile());
        let err = assert_err(
            open(
                &SourceSpec::Url {
                    url: "blob:garbage".to_string(),
                },
                &registry,
                &caps,
                WebOpenOptions::default(),
            ),
            "garbage",
        );
        assert_eq!(err.stage, Stage::Probe);
    }

    /// The source grammar's refusals are typed and cite the
    /// capability rows (doc 09 §6.3/§9.2): filesystem, rtsp, http,
    /// unknown blob.
    #[test]
    fn source_grammar_refusals_are_typed() {
        let registry = BlobRegistry::new();
        let caps = capabilities(crate::capabilities::build_profile());
        let file = SourceSpec::File {
            path: std::path::PathBuf::from("/tmp/x.avi"),
        };
        let err = assert_err(resolve_source(&file, &registry, &caps), "file on web");
        assert_eq!(err.code, ErrorCode::UnsupportedForHost);
        assert_eq!(
            err.details.get("capability").map(String::as_str),
            Some("local-filesystem")
        );
        for (url, capability) in [
            ("rtsp://camera/mpeg1", "rtsp-rtp-udp-access"),
            ("udp://239.0.0.1:1234", "rtsp-rtp-udp-access"),
            ("https://example.com/a.avi", "http-streaming"),
        ] {
            let err = assert_err(
                resolve_source(
                    &SourceSpec::Url {
                        url: url.to_string(),
                    },
                    &registry,
                    &caps,
                ),
                url,
            );
            assert_eq!(err.code, ErrorCode::UnsupportedForHost, "{url}");
            assert_eq!(
                err.details.get("capability").map(String::as_str),
                Some(capability),
                "{url}"
            );
        }
        let err = assert_err(
            resolve_source(
                &SourceSpec::Url {
                    url: "blob:not-loaded".to_string(),
                },
                &registry,
                &caps,
            ),
            "unknown blob",
        );
        assert_eq!(err.code, ErrorCode::NotFound);
    }
}
