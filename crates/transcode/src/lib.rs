//! The Lumen transcode graph core (card 7.E1, issue #43; doc 01
//! §2.5 registry row `transcode` — "recording, conversion, stream
//! output", the Phase-7 surface of MP §16).
//!
//! # What this crate is
//!
//! A pipeline that **reuses** the engine's probe/demux/decode stacks
//! — without the playback clock — to convert media, driven by an
//! explicit job graph the host assembles:
//!
//! ```text
//! source/demux ──► decode ──► [filters] ──► encode/sink
//! (Box<dyn       (factory-     v1: none      WavPcm (decode path)
//!  Demuxer>)      built        implemented   WavCopyPcm / RawEsCopy / Mp4Remux
//!                Decoder)      — refused)    (copy paths)
//! ```
//!
//! Assemble a [`TranscodeGraph`], then [`submit`] it (a
//! host-wired decode job; the plumbing stubs are the host's real
//! demuxer/decoder, elided):
//!
//! ```no_run
//! use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
//! use media_types::{EngineError, Frame, Track};
//! use plugin_api::{Demuxer, DemuxEvent};
//! use transcode::graph::{
//!     DecodeStage, DecoderFactory, SinkSpec, SourceSpec, TrackSelection, TranscodeGraph,
//!     WavPcmTarget,
//! };
//! use transcode::{JobLimits, JobOutcome, submit};
//!
//! # struct NopFactory;
//! # impl DecoderFactory for NopFactory {
//! #     fn open(&self, _track: &Track) -> Result<Box<dyn Decoder>, EngineError> {
//! #         Ok(Box::new(NopDecoder))
//! #     }
//! # }
//! # struct NopDecoder;
//! # impl Decoder for NopDecoder {
//! #     fn configure(&mut self, _cfg: &DecoderConfig) -> Result<(), EngineError> {
//! #         Ok(())
//! #     }
//! #     fn decode(
//! #         &mut self,
//! #         _packet: media_types::Packet,
//! #         _out: &mut dyn FrameSink,
//! #     ) -> Result<DecodeOutcome, EngineError> {
//! #         Ok(DecodeOutcome::NeedMoreInput)
//! #     }
//! #     fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
//! #         Ok(DecodeOutcome::Drained)
//! #     }
//! # }
//! # struct NopDemuxer;
//! # impl Demuxer for NopDemuxer {
//! #     fn start(&mut self) -> Result<Vec<Track>, EngineError> {
//! #         Ok(Vec::new())
//! #     }
//! #     fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
//! #         Ok(DemuxEvent::EndOfStream)
//! #     }
//! #     fn seek(&mut self, _target: media_types::TimeSpan) -> Result<media_types::TimeSpan, EngineError> {
//! #         Ok(media_types::TimeSpan::UNKNOWN)
//! #     }
//! # }
//! # fn open_source() -> Result<Box<dyn Demuxer>, EngineError> {
//! #     Ok(Box::new(NopDemuxer))
//! # }
//! fn main() {
//!     let demuxer = match open_source() {
//!         Ok(demuxer) => demuxer,
//!         Err(err) => {
//!             eprintln!("open failed: {err}");
//!             return;
//!         }
//!     };
//!     let graph = TranscodeGraph {
//!         source: SourceSpec {
//!             demuxer,
//!             selection: TrackSelection::DefaultAudio,
//!             extra_data: None,
//!         },
//!         decode: Some(DecodeStage::new(Box::new(NopFactory))),
//!         filters: Vec::new(),
//!         sink: SinkSpec::WavPcm {
//!             output: Box::new(std::io::Cursor::new(Vec::new())),
//!             target: WavPcmTarget::Source,
//!         },
//!     };
//!     let job = match submit(graph, JobLimits::default()) {
//!         Ok(job) => job,
//!         Err(err) => {
//!             eprintln!("graph refused: {err}");
//!             return;
//!         }
//!     };
//!     match job.wait() {
//!         JobOutcome::Completed(stats) => {
//!             eprintln!("wrote {} sample frames", stats.sample_frames_written)
//!         }
//!         JobOutcome::Failed(err) => eprintln!("failed: {err}"),
//!         JobOutcome::Cancelled(stats) => {
//!             eprintln!("cancelled after {} bytes", stats.bytes_written)
//!         }
//!     }
//! }
//! ```
//!
//! # Honest capabilities (v1 — nothing is claimed that is not here)
//!
//! * **WAV PCM encode** — a real RIFF/WAVE writer (16/24/32-bit
//!   integer and f32/f64 float), fed by any host-provided
//!   [`Decoder`](codec_api::Decoder); conversions are identity,
//!   planar→interleaved, byte-order normalization and the lossless
//!   widenings only — every lossy pair is a typed refusal.
//! * **Stream-copy where legal** — unpacked LE-PCM sources re-wrap
//!   into WAV without decoding (payload bytes verbatim), any
//!   track dumps to a raw elementary-stream file, and **H.264/AAC
//!   remux into a minimal in-tree MP4 container** (card 7.E2, see
//!   [`mp4mux`]): sample payloads byte-verbatim under fresh sample
//!   tables, the `stsd` entry bytes carried over from the source.
//!   Every other conversion is a typed `ErrorKind::Unsupported`,
//!   never a degraded output.
//! * **No filters** — the graph *models* filter stages and
//!   validation refuses each one (the stage is modeled, not faked).
//! * **No codec encoders** — WAV PCM is container serialization;
//!   the codec-api `Encoder` seam joins with card 7.E3
//!   `[PROPOSAL: the decode stage's output feeds a factory-built
//!   Encoder whose packet drain is the raw ES sink]`.
//!
//! # Bounded resources (MP §12 — the contract map)
//!
//! Every accumulation point has an explicit, documented ceiling
//! (see [`JobLimits`]' module table): the demux→worker stage queue
//! is bounded on two axes (slots and payload bytes, the streaming
//! stack's doctrine) with parking backpressure; the event hub drops
//! and counts past capacity, never blocking the worker; the track
//! table is budget-checked right after `Demuxer::start`; the WAV
//! writer streams through with no payload buffer; all counters are
//! saturating. The round-trip and backpressure suites pin these as
//! tests, not hopes.
//!
//! # Layer posture
//!
//! L3 (`doc 01 §2.5`), std-only (C5), no `unsafe`. Consumes L0
//! (`media-types`), L1 (`plugin-api` for the `Demuxer` contract,
//! `codec-api` for the decode contract) and L2 (`demux` for the WAV
//! `fmt `-facts vocabulary and the writer's accepted-set). The
//! concrete decoders stay behind [`DecoderFactory`] — the
//! playback-engine's decoder-provider precedent — so no same-layer
//! production edge exists.

pub mod cancel;
pub mod convert;
pub mod error;
pub mod graph;
pub mod job;
pub mod limits;
pub mod mp4mux;
pub mod queue;
pub mod record;
pub mod runner;
pub mod wav;

pub use cancel::CancelToken;
pub use graph::{
    DecodeStage, DecoderFactory, FilterDomain, FilterStage, PcmFacts, SinkSpec, SourceSpec,
    TrackSelection, TranscodeGraph, WavPcmTarget, WriteSeek,
};
pub use job::{JobEvent, JobHandle, JobOutcome, JobState, JobStats};
pub use limits::JobLimits;
pub use mp4mux::{
    MAX_REMUX_SAMPLES, MAX_SAMPLE_ENTRY_BYTES, Mp4Remuxer, Mp4TrackConfig, Mp4TrackSetup,
};
pub use record::{Recorder, Remuxer};
pub use runner::submit;
