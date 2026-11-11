//! The transcode job graph (card 7.E1): typed stages, validated
//! before any thread spawns.
//!
//! # The stage vocabulary (v1)
//!
//! ```text
//! source/demux ──► decode ──► [filters] ──► encode/sink
//! (Box<dyn       (factory-     v1: none      WavPcm (decode path)
//!  Demuxer>)      built        implemented   WavCopyPcm / RawEsCopy (copy path)
//!                Decoder)      — refused)
//! ```
//!
//! The graph is **plain data a host assembles**, then hands to
//! [`submit`](crate::submit). Two shape families are legal:
//!
//! * **decode path** — [`SinkSpec::WavPcm`]: the selected track is
//!   decoded (any host-provided [`Decoder`] through a
//!   [`DecoderFactory`]) and the decoded frames are written as RIFF/
//!   WAVE PCM (16/24/32-bit integer, f32/f64 float — identity or
//!   lossless widening only, never a silent quantization).
//! * **copy path** — [`SinkSpec::WavCopyPcm`], [`SinkSpec::RawEsCopy`]
//!   and [`SinkSpec::Mp4Remux`]: demuxed packets flow to the sink
//!   verbatim, no decoder. WAV-from-PCM is legal only for tracks the
//!   registry vocabulary calls unpacked little-endian PCM
//!   (`pcm_u8` … `pcm_f64le`) with host-supplied `fmt `-level facts;
//!   the raw elementary-stream sink accepts any track; the MP4-remux
//!   sink (card 7.E2) accepts H.264/AAC tracks only, with
//!   host-harvested sample-entry bytes per track. **No other remux
//!   exists in v1** — every conversion outside the families above is
//!   a typed `ErrorKind::Unsupported`, never a degraded output.
//!
//! [`FilterStage`] exists in the model (the graph *carries* filter
//! stages) and validation refuses every one of them — the honest
//! capability rule: the stage is modeled, not faked.
//!
//! # Validation order (pinned by tests)
//!
//! 1. filter rows over the [`JobLimits`] budget, then any filter row
//!    at all (v1 implements none);
//! 2. sink shape: WAV-PCM needs a decode stage; copy sinks reject
//!    one; the MP4-remux sink additionally needs a non-empty,
//!    duplicate-free, budget-bounded config list with in-budget
//!    sample entries;
//! 3. WAV-from-PCM copy needs explicit PCM facts (the generic
//!    `Demuxer` contract carries no `fmt ` parameters — the host that
//!    opened a WAV source knows them, the runner cannot invent them).
//!
//! Track-level legality (audio present, unambiguous, PCM-compatible
//! codec for WAV copy) is runtime knowledge: it is checked right
//! after `Demuxer::start`, before the first packet flows, and fails
//! the job with the same typed vocabulary (see `runner`).

use std::io::{Seek, Write};

use codec_api::Decoder;
use demux::wav::format::PcmSample;
use media_types::ids::TrackId;
use media_types::packet::SharedBytes;
use media_types::{EngineError, Track};
use plugin_api::Demuxer;

use crate::error::{
    filter_stages_over_budget, filter_unimplemented, graph_copy_facts_missing,
    graph_decode_forbidden, graph_decode_required, graph_selection_tracks_mismatch,
    mp4_sample_entry_over_budget, mp4_track_duplicate, mp4_tracks_empty, tracks_over_budget,
};
use crate::limits::JobLimits;
use crate::mp4mux::{MAX_SAMPLE_ENTRY_BYTES, Mp4TrackConfig};

/// A sink the writers stream into and finalize (patch RIFF sizes) on.
///
/// Deliberately `Write + Seek + Send`: WAV finalization rewrites the
/// `RIFF`/`data` size fields, so a write-only pipe cannot host an
/// honestly-sized file. Opening the sink is the host's job (the
/// crate owns no filesystem policy); `std::fs::File` implements this
/// trait directly, as do the test sinks.
pub trait WriteSeek: Write + Seek + Send {}
impl<T: Write + Seek + Send> WriteSeek for T {}

/// Which track(s) of the source the job consumes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackSelection {
    /// The default audio pick: the source must carry exactly one
    /// audio track (zero or several are typed failures — the host
    /// must name one).
    DefaultAudio,
    /// A specific track by its engine handle.
    Track(TrackId),
    /// Several tracks by their engine handles (the MP4-remux sink's
    /// shape: a remux copies every configured track). Each id must
    /// exist in the source's track table; duplicates are refused at
    /// graph validation (the sink's config list is the de-duplicated
    /// authority — `submit` derives this selection from it).
    Tracks(Vec<TrackId>),
}

/// Builds the decode stage's decoder for the selected track.
///
/// The trait seam keeps this crate honest about layers: concrete
/// decoders live in `codecs`/plugins (L3+); the runner speaks
/// [`Decoder`] only and asks the host's factory once per job
/// (the playback-engine's decoder-provider precedent).
pub trait DecoderFactory: Send {
    /// Build a decoder for `track`.
    ///
    /// # Errors
    /// Typed [`EngineError`] (typically `Unsupported`) when no
    /// decoder serves the track's codec descriptor.
    fn open(&self, track: &Track) -> Result<Box<dyn Decoder>, EngineError>;
}

/// The decode stage: a factory plus nothing else (configuration —
/// codec descriptor, generation, out-of-band bytes — is the runner's
/// job, from the selected track and the source spec).
pub struct DecodeStage {
    factory: Box<dyn DecoderFactory>,
}

impl std::fmt::Debug for DecodeStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The factory is a host closure-like seam; the stage's shape
        // is the Debug surface, not the host's guts.
        f.write_str("DecodeStage(factory)")
    }
}

impl DecodeStage {
    /// Assemble the stage around a host factory.
    pub fn new(factory: Box<dyn DecoderFactory>) -> DecodeStage {
        DecodeStage { factory }
    }

    /// Consume the stage, taking its factory (the runner moves the
    /// factory to the worker thread).
    pub(crate) fn into_factory(self) -> Box<dyn DecoderFactory> {
        self.factory
    }
}

/// One requested filter stage. v1 implements **none** — the model
/// carries the request (name + domain), validation refuses it
/// ([`filter_unimplemented`]); rows above the configured budget
/// refuse with [`filter_stages_over_budget`] first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterStage {
    /// Which pipeline domain the filter would run in.
    pub domain: FilterDomain,
    /// The filter the host asked for (recorded verbatim, refused).
    pub name: String,
}

/// The domain a [`FilterStage`] targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterDomain {
    /// Between decode and the audio sink.
    Audio,
    /// Between decode and a video sink (no video sink exists in v1).
    Video,
}

/// The `fmt `-level PCM facts a WAV stream-copy needs before its
/// first packet (channels and rate are not in the L0 [`Track`]; the
/// host that opened the WAV source holds them — the runner never
/// invents them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcmFacts {
    /// The unpacked-PCM sample representation of the payload.
    pub sample: PcmSample,
    /// Channel count (validated 1..=`MAX_CHANNELS` at writer build).
    pub channels: u16,
    /// Samples per second (validated nonzero).
    pub rate: u32,
}

/// The output sample format the WAV encode path writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavPcmTarget {
    /// Write whatever the decode path yields, byte-identically
    /// (after planar→interleaved and native→little-endian
    /// normalization) — the zero-conversion default.
    Source,
    /// Write a specific PCM sample format; legal only when it is the
    /// source's own or a lossless widening of it (s16→s32, f32→f64,
    /// …). Narrowing and int↔float are refused typed — a transcode
    /// engine must never quantize silently.
    Sample(PcmSample),
}

/// What the job produces — the encode/sink stage.
pub enum SinkSpec {
    /// Decode path: decoded audio frames → RIFF/WAVE PCM.
    WavPcm {
        /// The streaming, size-patching writer target.
        output: Box<dyn WriteSeek>,
        /// The sample format to write.
        target: WavPcmTarget,
    },
    /// Copy path: unpacked-PCM packets → a WAV container, payload
    /// bytes verbatim (no decode, no sample conversion).
    WavCopyPcm {
        /// The streaming, size-patching writer target.
        output: Box<dyn WriteSeek>,
        /// The source's `fmt `-level facts. `None` is the typed
        /// [`graph_copy_facts_missing`] refusal at validation — the
        /// runner cannot learn these from the generic `Demuxer`
        /// contract.
        facts: Option<PcmFacts>,
    },
    /// Copy path: any track's packets → one raw elementary-stream
    /// file, payload bytes verbatim concatenated. Legal for every
    /// codec (the sink makes no container claim — it is a byte
    /// dump, honestly named).
    RawEsCopy {
        /// The streaming writer target.
        output: Box<dyn WriteSeek>,
    },
    /// Copy path (card 7.E2): H.264/AAC packets → a minimal
    /// progressive MP4 container, sample payloads byte-verbatim
    /// under fresh sample tables (see [`crate::mp4mux`]).
    ///
    /// The **config list is the track selection**: `submit` derives
    /// the pump's [`TrackSelection::Tracks`] from it (a host that
    /// wants fewer tracks configures fewer), so the graph's
    /// `source.selection` is not consulted for this sink — assemble
    /// through [`Remuxer`](crate::record::Remuxer) to keep that
    /// single-sourced. Each config names the source track and
    /// carries its verbatim `stsd` entry payload (the host that
    /// opened the source harvests it — the runner never invents
    /// codec config bytes).
    Mp4Remux {
        /// The streaming, size-patching writer target.
        output: Box<dyn WriteSeek>,
        /// One entry per remuxed track, in output track order.
        configs: Vec<Mp4TrackConfig>,
    },
}

impl std::fmt::Debug for SinkSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The sink target is a host `Write + Seek` object; Debug
        // shows the shape (which sink, which facts), never the
        // host's stream guts.
        match self {
            SinkSpec::WavPcm { target, .. } => f.debug_tuple("WavPcm").field(target).finish(),
            SinkSpec::WavCopyPcm { facts, .. } => f.debug_tuple("WavCopyPcm").field(facts).finish(),
            SinkSpec::RawEsCopy { .. } => f.write_str("RawEsCopy"),
            SinkSpec::Mp4Remux { configs, .. } => {
                f.debug_tuple("Mp4Remux").field(&configs.len()).finish()
            }
        }
    }
}

/// The source stage: a demuxer, the track to consume, and the
/// out-of-band codec bytes the decoder config needs.
pub struct SourceSpec {
    /// The opened source's demuxer. `start`/`next_event` run on the
    /// job's pump thread (the plugin-api calling context).
    pub demuxer: Box<dyn Demuxer>,
    /// Which of the demuxer's tracks this job consumes.
    pub selection: TrackSelection,
    /// Out-of-band decoder configuration bytes (the WAV `fmt `
    /// payload, an AVC config record, …) when the container
    /// separates them from the stream — handed to the decoder
    /// factory's product at configure time.
    pub extra_data: Option<SharedBytes>,
}

impl std::fmt::Debug for SourceSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The demuxer is a host plugin object; Debug shows the
        // selection and extradata presence, not the plugin's guts.
        f.debug_struct("SourceSpec")
            .field("selection", &self.selection)
            .field("extra_data", &self.extra_data)
            .finish_non_exhaustive()
    }
}

/// The whole job graph — the four stages, as plain host-assembled
/// data. Validate with [`TranscodeGraph::validate`] (or let
/// [`submit`](crate::submit) do it).
pub struct TranscodeGraph {
    /// Source/demux stage.
    pub source: SourceSpec,
    /// Decode stage (`None` on copy paths).
    pub decode: Option<DecodeStage>,
    /// Requested filter rows (v1: must be empty — see
    /// [`FilterStage`]).
    pub filters: Vec<FilterStage>,
    /// Encode/sink stage.
    pub sink: SinkSpec,
}

impl std::fmt::Debug for TranscodeGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranscodeGraph")
            .field("source", &self.source)
            .field("decode", &self.decode)
            .field("filters", &self.filters)
            .field("sink", &self.sink)
            .finish()
    }
}

impl TranscodeGraph {
    /// Validate the graph's shape against a limits envelope.
    ///
    /// Pure and synchronous: no thread spawns, no I/O — the runner
    /// calls this before touching the source.
    ///
    /// # Errors
    /// Typed at stage `Transcode` (module docs' validation order):
    /// filter rows over budget, any filter row (v1 implements none),
    /// WAV-PCM without decode, decode on a copy sink, WAV-from-PCM
    /// copy without facts.
    pub fn validate(&self, limits: &JobLimits) -> Result<(), EngineError> {
        if self.filters.len() > limits.max_filter_stages() {
            return Err(filter_stages_over_budget(
                self.filters.len(),
                limits.max_filter_stages(),
            ));
        }
        if !self.filters.is_empty() {
            return Err(filter_unimplemented());
        }
        match &self.sink {
            SinkSpec::WavPcm { .. } => {
                if self.decode.is_none() {
                    return Err(graph_decode_required());
                }
            }
            SinkSpec::WavCopyPcm { facts, .. } => {
                if self.decode.is_some() {
                    return Err(graph_decode_forbidden());
                }
                if facts.is_none() {
                    return Err(graph_copy_facts_missing());
                }
            }
            SinkSpec::RawEsCopy { .. } => {
                if self.decode.is_some() {
                    return Err(graph_decode_forbidden());
                }
            }
            SinkSpec::Mp4Remux { configs, .. } => {
                if self.decode.is_some() {
                    return Err(graph_decode_forbidden());
                }
                if configs.is_empty() {
                    return Err(mp4_tracks_empty());
                }
                if configs.len() > limits.max_tracks() {
                    return Err(tracks_over_budget(configs.len(), limits.max_tracks()));
                }
                let mut seen = std::collections::HashSet::new();
                for config in configs {
                    if !seen.insert(config.track) {
                        return Err(mp4_track_duplicate());
                    }
                    if config.sample_entry.len() > MAX_SAMPLE_ENTRY_BYTES {
                        return Err(mp4_sample_entry_over_budget(
                            config.sample_entry.len(),
                            MAX_SAMPLE_ENTRY_BYTES,
                        ));
                    }
                }
            }
        }
        // A multi-track selection feeds the MP4-remux sink (whose
        // configs derive it); every other sink consumes exactly one
        // track and must not silently drop the rest.
        if matches!(self.source.selection, TrackSelection::Tracks(_))
            && !matches!(self.sink, SinkSpec::Mp4Remux { .. })
        {
            return Err(graph_selection_tracks_mismatch());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::time::TimeSpan;
    use testkit::{Must, assert_err};

    /// A do-nothing factory (the graph never calls it during
    /// validation — shape only).
    struct NopFactory;
    impl DecoderFactory for NopFactory {
        fn open(&self, _track: &Track) -> Result<Box<dyn Decoder>, EngineError> {
            Err(graph_decode_required())
        }
    }

    /// A bare `Demuxer` impl that yields nothing (shape only).
    struct NopDemuxer;
    impl Demuxer for NopDemuxer {
        fn start(&mut self) -> Result<Vec<Track>, EngineError> {
            Ok(Vec::new())
        }
        fn next_event(&mut self) -> Result<plugin_api::DemuxEvent, EngineError> {
            Ok(plugin_api::DemuxEvent::EndOfStream)
        }
        fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
            Ok(TimeSpan::UNKNOWN)
        }
    }

    /// A Write + Seek sink that discards bytes (std's `io::sink`
    /// carries no Seek; WAV finalization needs it).
    struct SeekableSink;

    impl std::io::Write for SeekableSink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl std::io::Seek for SeekableSink {
        fn seek(&mut self, _to: std::io::SeekFrom) -> std::io::Result<u64> {
            Ok(0)
        }
    }

    fn sink() -> SinkSpec {
        SinkSpec::RawEsCopy {
            output: Box::new(SeekableSink),
        }
    }

    fn graph(decode: Option<DecodeStage>, sink: SinkSpec) -> TranscodeGraph {
        TranscodeGraph {
            source: SourceSpec {
                demuxer: Box::new(NopDemuxer),
                selection: TrackSelection::DefaultAudio,
                extra_data: None,
            },
            decode,
            filters: Vec::new(),
            sink,
        }
    }

    /// A graph whose non-filter stages are legal (the filter-table
    /// refusals must not be confounded with sink-shape refusals).
    fn legal_copy_graph() -> TranscodeGraph {
        graph(None, sink())
    }

    #[test]
    fn wav_pcm_requires_decode_and_copy_sinks_do_not() {
        // WavPcm without a decode stage is refused: PCM encoding is a
        // decode-path sink (frames, not packets, feed it).
        let wav = SinkSpec::WavPcm {
            output: Box::new(SeekableSink),
            target: WavPcmTarget::Source,
        };
        let err = assert_err(
            graph(None, wav).validate(&JobLimits::default()),
            "wav sink without decode",
        );
        assert_eq!(err.context.code, "transcode.graph.decode-required");
        // The copy sinks consume demuxed packets — no decode stage
        // wanted (their refusal is the decode-present direction, pinned
        // by `copy_sinks_reject_decode`).
        graph(None, sink()).validate(&JobLimits::default()).must();
    }

    #[test]
    fn copy_sinks_reject_decode() {
        let err = assert_err(
            graph(Some(DecodeStage::new(Box::new(NopFactory))), sink())
                .validate(&JobLimits::default()),
            "raw copy with decode",
        );
        assert_eq!(err.context.code, "transcode.graph.decode-forbidden");

        let wav_copy = SinkSpec::WavCopyPcm {
            output: Box::new(SeekableSink),
            facts: Some(PcmFacts {
                sample: PcmSample::S16Le,
                channels: 2,
                rate: 48_000,
            }),
        };
        let err = assert_err(
            graph(Some(DecodeStage::new(Box::new(NopFactory))), wav_copy)
                .validate(&JobLimits::default()),
            "wav copy with decode",
        );
        assert_eq!(err.context.code, "transcode.graph.decode-forbidden");
    }

    #[test]
    fn wav_copy_without_facts_is_refused() {
        let wav_copy = SinkSpec::WavCopyPcm {
            output: Box::new(SeekableSink),
            facts: None,
        };
        let err = assert_err(
            graph(None, wav_copy).validate(&JobLimits::default()),
            "wav copy without facts",
        );
        assert_eq!(err.context.code, "transcode.graph.copy-facts-missing");
    }

    #[test]
    fn filter_rows_refuse_over_budget_then_unimplemented() {
        let mut g = legal_copy_graph();
        g.filters = (0..5)
            .map(|i| FilterStage {
                domain: FilterDomain::Audio,
                name: format!("filter-{i}"),
            })
            .collect();
        // Default budget is 4: five rows is over budget first.
        let err = assert_err(
            g.validate(&JobLimits::default()),
            "five rows over the default budget of four",
        );
        assert_eq!(
            err.context.code,
            "transcode.graph.filter-stages-over-budget"
        );
        // One row is inside the budget but v1 implements none.
        g.filters.truncate(1);
        let err = assert_err(
            g.validate(&JobLimits::default()),
            "one row inside the budget",
        );
        assert_eq!(err.context.code, "transcode.graph.filter-unimplemented");
    }

    #[test]
    fn legal_graphs_validate_clean() {
        // Decode path.
        let wav = SinkSpec::WavPcm {
            output: Box::new(SeekableSink),
            target: WavPcmTarget::Source,
        };
        graph(Some(DecodeStage::new(Box::new(NopFactory))), wav)
            .validate(&JobLimits::default())
            .must();
        // Copy paths.
        graph(None, sink()).validate(&JobLimits::default()).must();
        let wav_copy = SinkSpec::WavCopyPcm {
            output: Box::new(SeekableSink),
            facts: Some(PcmFacts {
                sample: PcmSample::S24Le,
                channels: 2,
                rate: 48_000,
            }),
        };
        graph(None, wav_copy).validate(&JobLimits::default()).must();
        // The MP4-remux copy path: one config is the legal shape.
        graph(None, mp4_sink())
            .validate(&JobLimits::default())
            .must();
    }

    /// An MP4-remux sink with one h264-shaped config (track handle +
    /// a minimal sample-entry payload).
    fn mp4_sink() -> SinkSpec {
        SinkSpec::Mp4Remux {
            output: Box::new(SeekableSink),
            configs: vec![Mp4TrackConfig {
                track: TrackId::new(0),
                sample_entry: SharedBytes::from_vec(vec![0u8; 86]),
            }],
        }
    }

    #[test]
    fn mp4_remux_rejects_decode_empty_duplicate_and_over_budget_configs() {
        // A decode stage on the remux sink: copy path, refused.
        let err = assert_err(
            graph(Some(DecodeStage::new(Box::new(NopFactory))), mp4_sink())
                .validate(&JobLimits::default()),
            "remux with decode",
        );
        assert_eq!(err.context.code, "transcode.graph.decode-forbidden");
        // No configs at all: nothing to remux.
        let empty = SinkSpec::Mp4Remux {
            output: Box::new(SeekableSink),
            configs: Vec::new(),
        };
        let err = assert_err(
            graph(None, empty).validate(&JobLimits::default()),
            "remux without configs",
        );
        assert_eq!(err.context.code, "transcode.mp4.tracks-empty");
        // The same track twice: one output track per source track.
        let twice = SinkSpec::Mp4Remux {
            output: Box::new(SeekableSink),
            configs: vec![
                Mp4TrackConfig {
                    track: TrackId::new(0),
                    sample_entry: SharedBytes::from_vec(vec![0u8; 8]),
                },
                Mp4TrackConfig {
                    track: TrackId::new(0),
                    sample_entry: SharedBytes::from_vec(vec![0u8; 8]),
                },
            ],
        };
        let err = assert_err(
            graph(None, twice).validate(&JobLimits::default()),
            "duplicate remux track",
        );
        assert_eq!(err.context.code, "transcode.mp4.track-duplicate");
        // A config list over the track budget (default 64): the budget
        // rule fires before the per-config entry-size walk
        // (validation order).
        let many = SinkSpec::Mp4Remux {
            output: Box::new(SeekableSink),
            configs: (0..65)
                .map(|i| Mp4TrackConfig {
                    track: TrackId::new(i),
                    sample_entry: SharedBytes::from_vec(vec![0u8; 8]),
                })
                .collect(),
        };
        let err = assert_err(
            graph(None, many).validate(&JobLimits::default()),
            "config list over the track budget",
        );
        assert_eq!(err.context.code, "transcode.source.tracks-over-budget");
        // One config with an over-budget sample entry: the config's
        // own ceiling.
        let huge = SinkSpec::Mp4Remux {
            output: Box::new(SeekableSink),
            configs: vec![Mp4TrackConfig {
                track: TrackId::new(0),
                sample_entry: SharedBytes::from_vec(vec![0u8; MAX_SAMPLE_ENTRY_BYTES + 1]),
            }],
        };
        let err = assert_err(
            graph(None, huge).validate(&JobLimits::default()),
            "oversized sample entry",
        );
        assert_eq!(err.context.code, "transcode.mp4.sample-entry-over-budget");
    }

    #[test]
    fn multi_track_selection_is_remux_only() {
        // A Tracks selection on a single-track sink is refused — the
        // sink would have to drop tracks silently, and it must not.
        let mut g = graph(None, sink());
        g.source.selection = TrackSelection::Tracks(vec![TrackId::new(0), TrackId::new(1)]);
        let err = assert_err(
            g.validate(&JobLimits::default()),
            "Tracks selection on the raw-ES sink",
        );
        assert_eq!(
            err.context.code,
            "transcode.graph.selection-tracks-mismatch"
        );
        // The MP4-remux sink consumes the whole selection.
        let mut g = graph(None, mp4_sink());
        g.source.selection = TrackSelection::Tracks(vec![TrackId::new(0)]);
        g.validate(&JobLimits::default()).must();
    }
}
