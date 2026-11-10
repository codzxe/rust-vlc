//! The open path and the demuxer host wrapper (card 2.D2; generalized
//! by 6.OPW-PREP per `docs/plans/open-path-wiring.md` §4.3): file URI
//! → `input` access → `media-probe` → **the container-host registry**
//! (`crate::containers` — one wired family per entry), plus the
//! generic seek/start-time skip wrapper and the track-facts pre-pass
//! shape that feeds the decoder provider and `media.get-info`.
//!
//! # The open path (doc 03 §4.2 — probe between access and demux)
//!
//! 1. `SourceSpec::File` → canonical absolute path → percent-encoded
//!    `file:` URI (the `input` crate resolves it back through
//!    `canonicalize` — the open path is symlink-safe by construction).
//! 2. `FileAccess::open` (the 2.A1 access stack) → seekable stream.
//! 3. `probe_stream` (2.A2) over the builtin signature registry — the
//!    bounded, non-consuming window read. A miss is the honest typed
//!    `unsupported` (stage `probe`); a hit is handed to
//!    [`crate::containers::open`], the single registry: a wired
//!    family constructs its demuxer + runs its facts pre-pass, an
//!    unwired one lands in the registry's typed refusal (the
//!    D-018-era message shape, format named).
//! 4. Per-container host (e.g. [`crate::containers::avi`]): the
//!    demuxer `new` + a **host pre-pass `start()`** — the CLI is the
//!    composition root, so it reads the demuxed track list and the
//!    container's codec-facts accessors BEFORE the demuxer moves into
//!    the engine. The engine's own `start()` call then returns the
//!    cached tracks (the demuxer's idempotent re-start).
//! 5. `--start-time`: the demuxer's `seek` is a **pure query** by
//!    contract (it does not move the read position — the engine owns
//!    acting on it) and [`SelectDemuxer`] drops packets that
//!    predate the target as the (linear, bounded-by-file) rewalk
//!    proceeds. In-session position jumps are the engine's native
//!    seek flow (card 3.A1, wired by W1-06): the wrapper forwards
//!    [`Demuxer::reanchor`] — the trait's compute-then-commit
//!    positioning hook — and clears the skip watermark at the
//!    landing (the seek's own landing is the new truth; the
//!    open-time filter would otherwise starve a backward seek below
//!    the start time).
//!
//! # EngineError → CommandError
//!
//! [`wire_error`] is the single bridge (doc 05 §2.13): stage and code
//! through `playback-engine`'s own published mappings
//! (`wire_stage`/`wire_error_code`), message through the command
//! layer's redaction-safe constructor.

use std::path::{Path, PathBuf};

use commands::{CommandError, ErrorCode, Stage};
use media_types::ids::SourceUri;
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};
use media_types::{EngineError, MediaKind, Track, TrackId};
use playback_engine::{QueuePlan, wire_error_code, wire_stage};
use plugin_api::{Access, DemuxEvent, Demuxer, SeekSupport};

/// The product's queue packet caps — the `[limits]` figures
/// product.toml ships (`queue_packets_audio = 1024`,
/// `queue_packets_video = 512`, product.toml:199-200). The CLI loads
/// no product-config yet (that join is the rebrand-propagation card's,
/// see `host.rs`'s `default_value` `[PROPOSAL]`), so these constants
/// mirror the shipped values and the open path consumes the product
/// posture **today**; when the product-config load lands it replaces
/// this pair 1:1 (`limits.queue_packets_audio/video`). `[PROPOSAL]`
pub const PRODUCT_QUEUE_PACKETS_AUDIO: u64 = 1024;
/// The video half of the same mirror.
pub const PRODUCT_QUEUE_PACKETS_VIDEO: u64 = 512;

/// The host-side queue-plan join (card 3.B3, design A4): map the
/// product limits' per-kind packet caps onto the engine's
/// [`QueuePlan`]. A value the schema would reject surfaces here as a
/// typed `CommandError` — the open failure, never a clamp and never a
/// silent default.
///
/// # Errors
/// [`CommandError`] (stage `Session`, code `InvalidArgument`) carrying
/// the engine's [`playback_engine::QueuePlanError`] message when a
/// packet cap is outside the schema range.
pub fn queue_plan_from_product_limits(
    audio_packets: u64,
    video_packets: u64,
) -> Result<QueuePlan, CommandError> {
    QueuePlan::from_product_limits(audio_packets, video_packets).map_err(|err| {
        CommandError::new(
            Stage::Session,
            ErrorCode::InvalidArgument,
            format!("queue plan rejected: {err}"),
        )
        .with_detail("stage", "open.queues")
    })
}

/// Bridge one engine error into the command layer's redaction-safe
/// form (the single conversion point; stages/codes via
/// playback-engine's published wire mappings).
///
/// The message: the engine's own display (stage tag stripped — the
/// command layer re-tags), with the error's chain root appended when
/// one rides along (host-minted errors carry their one-sentence
/// explanation there; I/O errors carry the OS sentence).
#[must_use]
pub fn wire_error(error: &EngineError) -> CommandError {
    let full = error.to_string();
    let stage_prefix = format!("[{}] ", error.stage);
    let base = full.strip_prefix(&stage_prefix).unwrap_or(full.as_str());
    // The chain root (host-minted errors carry their one-sentence
    // explanation there; I/O errors carry the OS sentence).
    let message = match error.source.as_ref() {
        Some(root) => format!("{base}: {root}"),
        None => base.to_string(),
    };
    CommandError::new(
        wire_stage(error.stage),
        wire_error_code(&error.kind),
        message,
    )
    .with_detail("code", error.context.code.as_str())
}

/// The access-layer refusal with the source named by its redacted
/// form (V8: the basename survives, the machine path does not —
/// MP §12; the input crate's errors are deliberately path-free, so
/// the HOST layer — the one that knows the URI — attaches the name).
fn access_refusal(error: &EngineError, uri: &SourceUri) -> CommandError {
    let base = wire_error(error);
    let name = uri.redacted();
    if base.message.contains(&name) {
        return base;
    }
    CommandError::new(base.stage, base.code, format!("{}: {name}", base.message))
        .with_detail("code", error.context.code.as_str())
}

/// Per-track facts the CLI harvested at the pre-pass — everything the
/// decoder provider, the sinks and `media.get-info` need that the L0
/// [`Track`] deliberately does not carry (codec parameters).
#[derive(Debug, Clone)]
pub struct TrackFact {
    /// The track's id (the AVI stream ordinal).
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
    /// Out-of-band codec initialization bytes (the esds ASC, the
    /// ALAC magic cookie, the Vorbis three-header chain, the
    /// ADPCM WAVEFORMATEX body) when the container separates them
    /// from the stream — `None` for AVI (its `strf` pair above is
    /// the PCM truth; MJPEG is self-describing per frame). The host
    /// provider seeds the decoder config from this (the engine
    /// mints `extra_data: None`); the exact blob is each container
    /// harvest's own truth.
    pub extra_data: Option<SharedBytes>,
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
        let nanos = stamp.convert(&media_clock::NANO_BASE).ok()?;
        Some(nanos.ticks / 1000)
    }

    /// The output geometry (channels + rate in Hz) for ANY
    /// audio-kind track the harvest can name — the generic
    /// audio-limb truth (card 6.OPW-ogg's seam fix: the open path
    /// previously built the limb only from `pcm`, leaving every
    /// non-PCM audio session limb-less on the wall clock).
    ///
    /// PCM tracks answer the `pcm` pair verbatim. Non-PCM tracks
    /// answer from the `extra_data` blob's own leading
    /// identification header — the Vorbis chain through the codec
    /// crate's own published parser
    /// ([`parse_identification`](codecs::audio::vorbis::parse_identification),
    /// no local grammar copy), OpusHead per RFC 7845 (channel
    /// count at byte 9; the decoder output rate is ALWAYS 48 kHz —
    /// the demuxer's own granule-clock precedent). `None` for
    /// video tracks, for audio without a geometry-bearing blob
    /// (ADTS: the config rides in-band and the demuxer exposes no
    /// blob — that family stays honestly limb-less until its
    /// harvest grows one), or for a blob that does not parse: the
    /// session then opens limb-less, never a guess.
    #[must_use]
    pub fn audio_geometry(&self) -> Option<(u32, u32)> {
        if self.kind != MediaKind::Audio {
            return None;
        }
        if let Some(pair) = self.pcm {
            return Some(pair);
        }
        let data = self.extra_data.as_ref()?.as_slice();
        if let Ok(ident) = codecs::audio::vorbis::parse_identification(data) {
            return Some((u32::from(ident.channels), ident.sample_rate));
        }
        if data.starts_with(b"OpusHead") && data.len() >= 19 {
            // RFC 7845 §5.1: channels at byte 9 of the 19-byte
            // fixed identification header; the output rate is
            // always 48 kHz (§4.2).
            return Some((u32::from(data[9]), 48_000));
        }
        None
    }
}

/// The harvested media description (the `media.get-info` backbone).
#[derive(Debug, Clone)]
pub struct MediaFacts {
    /// The demuxed track list (unfiltered — selection truth is the
    /// engine's wired-limb answer).
    pub tracks: Vec<Track>,
    /// Per-track facts, parallel to `tracks`.
    pub facts: Vec<TrackFact>,
    /// The probe's format id (`avi`).
    pub container: String,
}

impl MediaFacts {
    /// The media duration in µs: the greatest declared track extent
    /// (`None` when no track declares one).
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

/// The open options every container host receives (today's
/// `open_source` arguments, resolved): the start time, the track
/// selection flags, and the source URI the [`OpenedMedia`] owes the
/// engine's `SessionConfig` (the container modules never see the
/// `SourceSpec` — the resolved URI is host truth).
#[derive(Debug, Clone)]
pub struct OpenOptions {
    /// The resolved source URI (the engine's `SessionConfig.source`).
    pub uri: SourceUri,
    /// `--start-time` (µs), when given.
    pub start_time_us: Option<i64>,
    /// `--no-video`.
    pub no_video: bool,
    /// `--no-audio`.
    pub no_audio: bool,
}

/// What [`open_source`] produced: the wrapped demuxer ready for the
/// engine's [`SessionConfig`](playback_engine::SessionConfig), plus
/// the harvested facts.
pub struct OpenedMedia {
    /// The container demuxer behind the skip/selection wrapper
    /// (the engine consumes it as `dyn Demuxer` — the box is the
    /// seam's whole point: no container type crosses into the
    /// engine).
    pub demuxer: Box<dyn Demuxer>,
    /// The harvested facts.
    pub facts: MediaFacts,
    /// The position the session anchors at (ns; `start_time` or 0).
    pub start_position_ns: i64,
    /// The resolved source URI (the engine's `SessionConfig.source`).
    pub uri: SourceUri,
}

/// The percent-encoding set for a `file:` URI path: keep RFC 3986
/// unreserved characters plus `/`; escape everything else (space, `%`,
/// `?`, `#`, non-ASCII) so [`SourceUri::parse`] accepts the result and
/// `input`'s resolver round-trips it. `[PROPOSAL: minimal RFC 3986
/// path-encoding; a full encoder joins `input` when a non-ASCII-path
/// test matrix lands]`
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

/// Canonicalize a user path and build its `file:` URI.
///
/// # Errors
/// `inaccessible` when the path does not exist / cannot be resolved
/// (the canonicalize failure is the truth — `input`'s own resolver
/// would hit the same wall later); `unsupported` for non-UTF-8 paths.
fn file_uri(path: &Path) -> Result<SourceUri, CommandError> {
    let canonical: PathBuf = std::fs::canonicalize(path).map_err(|err| {
        CommandError::new(
            Stage::Input,
            ErrorCode::Inaccessible,
            format!("cannot resolve {}: {err}", path.display()),
        )
    })?;
    // A directory (or any non-regular file) is an input-truth
    // refusal, not an unsupported-format one: there is nothing to
    // read here (66 EX_NOINPUT, the unsupported-path suite's
    // contract).
    if !canonical.is_file() {
        return Err(CommandError::new(
            Stage::Input,
            ErrorCode::Inaccessible,
            format!("not a regular file: {}", path.display()),
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

/// The whole open path (probe → demuxer pre-pass → wrapper), producing
/// an [`OpenedMedia`].
///
/// # Errors
/// Typed [`CommandError`]s at the honest stages: `input` (access),
/// `probe` (no/unknown signature), `demux` (header parse, seek range).
pub fn open_source(
    source: &commands::catalog::types::SourceSpec,
    start_time_us: Option<i64>,
    no_video: bool,
    no_audio: bool,
) -> Result<OpenedMedia, CommandError> {
    let uri = match source {
        commands::catalog::types::SourceSpec::File { path } => file_uri(path)?,
        commands::catalog::types::SourceSpec::Url { url } => {
            return Err(CommandError::new(
                Stage::Input,
                ErrorCode::UnsupportedForHost,
                format!("URL sources need the streaming crate (Phase 6): {url}"),
            )
            .with_detail("scheme", url.split(':').next().unwrap_or("?")));
        }
    };
    // 1. Access (the 2.A1 stack).
    let access = input::FileAccess::new();
    let mut stream = access
        .open(&uri)
        .map_err(|err| access_refusal(&err, &uri))?;
    // 2. Probe (the 2.A2 bounded window; non-consuming).
    let registry = media_probe::ProbeRegistry::with_builtins();
    let config = media_probe::ProbeConfig::default();
    let report = media_probe::probe_stream(&registry, &mut *stream, &config)
        .map_err(|err| wire_error(&err))?;
    let container = report.format.as_str().to_string();
    // 3. The container-host registry (§4.4): the single dispatch — a
    //    wired family constructs + pre-passes + wraps its demuxer;
    //    anything else is the honest typed refusal (format named).
    let opts = OpenOptions {
        uri,
        start_time_us,
        no_video,
        no_audio,
    };
    crate::containers::open(&container, stream, &opts)
}

/// A demuxer behind the CLI's open options: track selection
/// (`--no-video`/`--no-audio`) and the start-position skip filter.
/// Generic over the inner demuxer (§4.3.1 — the skip/selection/filter
/// logic was never AVI-specific; the wrapper is the shared half of
/// every per-container wiring unit, and the engine consumes the
/// result as `dyn Demuxer`).
///
/// `start()` answers the track list minus disabled kinds (the engine
/// wires limbs only for what it sees — an all-disabled media lands in
/// the engine's honest `NoDecodableTrack` failure). Packets of
/// disabled tracks still flow from the inner demuxer; the engine
/// drops them at the unwired-track boundary, exactly as it does for
/// subtitle/data tracks. Packets that predate [`Self::skip_until_ns`]
/// (converted per-track through the harvested bases) are dropped here
/// — they are the pre-landing rewalk the seek query described.
pub struct SelectDemuxer<D: Demuxer> {
    inner: D,
    skip_until_ns: Option<i64>,
    bases: Vec<(TrackId, TimeBase)>,
    disable_video: bool,
    disable_audio: bool,
}

impl<D: Demuxer> SelectDemuxer<D> {
    /// Wrap a started demuxer with the harvested track bases, the
    /// open-time skip watermark (ns; `None` when no `--start-time`)
    /// and the selection flags.
    pub(crate) fn new(
        inner: D,
        skip_until_ns: Option<i64>,
        bases: Vec<(TrackId, TimeBase)>,
        disable_video: bool,
        disable_audio: bool,
    ) -> SelectDemuxer<D> {
        SelectDemuxer {
            inner,
            skip_until_ns,
            bases,
            disable_video,
            disable_audio,
        }
    }

    /// Whether the packet's presentation stamp predates the skip
    /// watermark. Unjudgeable stamps (missing pts, unknown base,
    /// unconvertible) are passed through — never dropped on a guess.
    fn before_skip(&self, packet: &media_types::Packet) -> bool {
        let Some(skip_ns) = self.skip_until_ns else {
            return false;
        };
        let Stamp::Present(pts) = packet.pts else {
            return false;
        };
        if !self.bases.iter().any(|(id, _)| *id == packet.track) {
            return false;
        }
        match pts.convert(&media_clock::NANO_BASE) {
            Ok(ns) => ns.ticks < skip_ns,
            Err(_) => false,
        }
    }
}

impl<D: Demuxer> Demuxer for SelectDemuxer<D> {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
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

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        loop {
            match self.inner.next_event()? {
                DemuxEvent::Packet(packet) => {
                    if self.before_skip(&packet) {
                        continue;
                    }
                    return Ok(DemuxEvent::Packet(packet));
                }
                other => return Ok(other),
            }
        }
    }

    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        self.inner.seek(target)
    }

    fn seekability(&self) -> SeekSupport {
        // The engine's seek-flow step 1 asks the demuxer (doc 03 §6):
        // the wrapper adds no capability the inner demuxer does not
        // have (e.g. AVI's idx1 → `Fast { indexed }`, no index → the
        // bounded slow scan, broken → the honest `None`). The trait
        // default would answer `None` and refuse every native seek
        // typed — forwarded, not invented.
        self.inner.seekability()
    }

    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // Doc 03 §6 step 4 through the wrapper: the inner demuxer's
        // compute-then-commit `reanchor` (contract D4 — a failure
        // leaves it at its pre-seek position). The open-time skip
        // watermark dies here: the landing is the new position
        // truth, and a `--start-time` filter left standing would
        // drop every packet of a backward seek below the start time
        // (the pre-roll would starve and the media would end at the
        // landing — honest-looking, wrong).
        let landed = self.inner.reanchor(target)?;
        self.skip_until_ns = None;
        Ok(landed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures")
            .join(relative)
    }

    /// The parent fixture opens end-to-end through the real access →
    /// probe → demuxer path, with the recorded track facts.
    #[test]
    fn parent_fixture_opens_with_expected_facts() {
        let opened = match open_source(
            &commands::catalog::types::SourceSpec::File {
                path: fixture("generated/valid/avi_mjpeg_pcm.avi"),
            },
            None,
            false,
            false,
        ) {
            Ok(opened) => opened,
            Err(err) => unreachable!("{err}"),
        };
        assert_eq!(opened.facts.container, "avi");
        assert_eq!(opened.facts.tracks.len(), 2);
        assert_eq!(opened.facts.duration_us(), Some(2_000_000));
        let video = opened
            .facts
            .facts
            .iter()
            .find(|f| f.kind == MediaKind::Video);
        let audio = opened
            .facts
            .facts
            .iter()
            .find(|f| f.kind == MediaKind::Audio);
        match (video, audio) {
            (Some(v), Some(a)) => {
                assert_eq!(v.codec, "mjpeg");
                // 30 fps → one frame every 33_333_333..33_333_334 ns.
                assert!(
                    matches!(v.frame_ns, Some(33_333_333..=33_333_334)),
                    "{:?}",
                    v.frame_ns
                );
                assert_eq!(a.codec, "pcm_s16le");
                assert_eq!(a.pcm, Some((2, 44100)));
            }
            other => unreachable!("expected video+audio facts: {other:?}"),
        }
        assert_eq!(opened.start_position_ns, 0);
    }

    /// A start-time past the duration clamps (doc 05 §2.4), and the
    /// seek query validates the target against the real index. The
    /// skip watermark is asserted behaviorally (through the trait
    /// object — the wrapper is a `dyn Demuxer` from here on): every
    /// packet that survives the wrapper lands at/after the clamped
    /// target, and a target clamped to the duration admits no packet
    /// before the end.
    #[test]
    fn start_time_clamps_to_duration() {
        let mut opened = match open_source(
            &commands::catalog::types::SourceSpec::File {
                path: fixture("generated/valid/avi_mjpeg_pcm.avi"),
            },
            Some(9_999_999),
            false,
            false,
        ) {
            Ok(opened) => opened,
            Err(err) => unreachable!("{err}"),
        };
        assert_eq!(opened.start_position_ns, 2_000_000_000);
        let _tracks = opened
            .demuxer
            .start()
            .unwrap_or_else(|e| unreachable!("{e}"));
        let mut packets = 0u32;
        loop {
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    packets += 1;
                    if let Stamp::Present(pts) = packet.pts {
                        let ns = pts
                            .convert(&media_clock::NANO_BASE)
                            .unwrap_or_else(|e| unreachable!("{e:?}"));
                        assert!(
                            ns.ticks >= 2_000_000_000,
                            "packet before the clamped start survived: {}",
                            ns.ticks
                        );
                    }
                }
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("{err}"),
            }
        }
        // The whole media predates a target clamped to its end.
        assert_eq!(packets, 0);
    }

    /// The wrapper's generalization is real, not cosmetic (§4.3.1):
    /// `SelectDemuxer` instantiates over a NON-AVI demuxer — the WAV
    /// demuxer over the shared WAV fixture — without touching the
    /// registry (wiring `wav` is the 6.OPW-wav unit's file, not this
    /// card's). The kind filter and the skip watermark behave exactly
    /// as over AVI: bases come from the started demuxer's own tracks.
    #[test]
    fn select_demuxer_is_generic_over_a_non_avi_demuxer() {
        let bytes: std::sync::Arc<[u8]> =
            std::fs::read(fixture("generated/valid/wav_pcm_s16le_44k1_st.wav"))
                .unwrap_or_else(|e| unreachable!("fixture unreadable: {e}"))
                .into();
        let started = |skip: Option<i64>| -> (SelectDemuxer<demux::wav::WavDemuxer>, usize) {
            let stream = demux::avi::test_support::mem_stream(bytes.clone())
                .unwrap_or_else(|e| unreachable!("{e}"));
            let mut demuxer =
                demux::wav::WavDemuxer::new(stream).unwrap_or_else(|e| unreachable!("{e}"));
            let tracks = demuxer.start().unwrap_or_else(|e| unreachable!("{e}"));
            let count = tracks.len();
            let bases = tracks
                .iter()
                .map(|track| (track.id, track.timing.base))
                .collect();
            (
                SelectDemuxer::new(demuxer, skip, bases, false, false),
                count,
            )
        };
        // The selection filter: --no-video keeps a video-less media's
        // audio track (filtering is by kind, not by codec).
        let (mut wrapper, track_count) = started(None);
        assert_eq!(track_count, 1, "the fixture is one audio track");
        let visible = wrapper.start().unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(visible.len(), 1);
        // The skip watermark: a target past the whole media drops every
        // packet — the wrapper answers the end of stream without
        // emitting one.
        let (mut skipping, _) = started(Some(i64::MAX / 2));
        let _ = skipping.start().unwrap_or_else(|e| unreachable!("{e}"));
        let mut packets = 0u32;
        loop {
            match skipping.next_event() {
                Ok(DemuxEvent::Packet(_)) => packets += 1,
                Ok(DemuxEvent::EndOfStream) => break,
                Ok(_) => {}
                Err(err) => unreachable!("{err}"),
            }
        }
        assert_eq!(packets, 0, "every WAV packet predates the watermark");
    }

    /// Pure garbage: the probe's honest typed unknown (stage probe,
    /// code unsupported), not a demux-stage crash.
    #[test]
    fn garbage_file_is_a_probe_stage_refusal() {
        let garbage = std::env::temp_dir().join("lumen-cli-garbage.bin"); // lumen-allow-literal: cfg(test) scratch filename — test-only, never a shipped surface
        std::fs::write(&garbage, [0x37u8, 0x13, 0x00, 0x42, 0xff, 0xee].repeat(64)).ok();
        match open_source(
            &commands::catalog::types::SourceSpec::File {
                path: garbage.clone(),
            },
            None,
            false,
            false,
        ) {
            Err(err) => {
                assert_eq!(err.stage, Stage::Probe, "{err}");
                assert_eq!(err.code, ErrorCode::Unsupported, "{err}");
            }
            Ok(_) => unreachable!("garbage must not open"),
        }
        std::fs::remove_file(&garbage).ok();
    }

    /// A magic-broken class-B derivative (flip16 breaks RIFF[2]):
    /// probe-stage refusal naming the sniffed bytes.
    #[test]
    fn magic_broken_derivative_is_refused_at_probe() {
        match open_source(
            &commands::catalog::types::SourceSpec::File {
                path: fixture("generated/malformed/avi_mjpeg_pcm__flip16.avi"),
            },
            None,
            false,
            false,
        ) {
            Err(err) => {
                assert_eq!(err.stage, Stage::Probe, "{err}");
                assert_eq!(err.code, ErrorCode::Unsupported, "{err}");
                assert!(err.message.contains("sniffed"), "{err}");
            }
            Ok(_) => unreachable!("a magic-broken file must not open"),
        }
    }

    /// A missing file: the input-stage inaccessible truth.
    #[test]
    fn missing_file_is_inaccessible() {
        match open_source(
            &commands::catalog::types::SourceSpec::File {
                path: fixture("generated/valid/no-such-file.avi"),
            },
            None,
            false,
            false,
        ) {
            Err(err) => {
                assert_eq!(err.stage, Stage::Input, "{err}");
                assert_eq!(err.code, ErrorCode::Inaccessible, "{err}");
            }
            Ok(_) => unreachable!("a missing file must not open"),
        }
    }

    /// URL sources: the honest host-capability refusal.
    #[test]
    fn url_sources_are_refused_with_the_phase_named() {
        match open_source(
            &commands::catalog::types::SourceSpec::Url {
                url: "https://example.org/x.mp4".to_string(), // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
            },
            None,
            false,
            false,
        ) {
            Err(err) => {
                assert_eq!(err.code, ErrorCode::UnsupportedForHost, "{err}");
                assert!(err.message.contains("Phase 6"), "{err}");
            }
            Ok(_) => unreachable!("URLs are out of the first vertical"),
        }
    }

    /// The path encoder escapes exactly the delimiter set the URI
    /// grammar and the resolver require.
    #[test]
    fn path_encoder_escapes_the_delimiters() {
        assert_eq!(encode_path("/a b/c%d?e#f"), "/a%20b/c%25d%3Fe%23f");
        assert_eq!(encode_path("/plain/path.avi"), "/plain/path.avi");
        assert_eq!(encode_path("/üño"), "/%C3%BC%C3%B1o");
    }

    /// The queue-plan join (card 3.B3): the shipped product figures
    /// build a plan; schema-range violations surface as typed
    /// `CommandError`s naming the axis — reject, never clamp.
    #[test]
    fn queue_plan_join_maps_product_limits_or_refuses_typed() {
        let plan = queue_plan_from_product_limits(
            PRODUCT_QUEUE_PACKETS_AUDIO,
            PRODUCT_QUEUE_PACKETS_VIDEO,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            plan.audio.max_packets.get(),
            PRODUCT_QUEUE_PACKETS_AUDIO as usize
        );
        assert_eq!(
            plan.video.max_packets.get(),
            PRODUCT_QUEUE_PACKETS_VIDEO as usize
        );

        let err = match queue_plan_from_product_limits(63, 512) {
            Ok(_) => unreachable!("below the floor must refuse"),
            Err(err) => err,
        };
        assert_eq!(err.code, ErrorCode::InvalidArgument);
        assert!(err.message.contains("audio"), "{err}");
        assert!(err.message.contains("floor"), "{err}");

        let err = match queue_plan_from_product_limits(1024, 4097) {
            Ok(_) => unreachable!("above the ceiling must refuse"),
            Err(err) => err,
        };
        assert!(err.message.contains("video"), "{err}");
        assert!(err.message.contains("ceiling"), "{err}");
    }
}
