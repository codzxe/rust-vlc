//! Record-to-WAV (card 7.E2, issue #44): the one-call assembly of
//! the decode-path graph — "record" names the *operation* the host
//! asks for (capture a source's audio into a WAV file), the pipeline
//! underneath is the 7.E1 decode stage driving the
//! [`WavPcmWriter`](crate::wav) live, without the playback clock.
//!
//! # What "record" means here (scope, stated honestly)
//!
//! * **Offline recording of any decodable input.** The recorder
//!   (re)opens a source through the host's [`Demuxer`], drives the
//!   host's [`DecoderFactory`] over every packet of the selected
//!   audio track, and streams the decoded frames into a RIFF/WAVE
//!   file. "Any decodable input" is decided by the factory the host
//!   wires (the plugin registry's decoder set), never claimed here:
//!   a codec with no decoder fails the job with the factory's own
//!   typed error, and the partial output's sizes tell the truth
//!   about the prefix that decoded.
//! * **Live capture is per-source, not a mid-pipeline tap.** v1 has
//!   no seam into a *playing* session's demux queue (the
//!   playback-engine owns that queue and no tap trait exists yet —
//!   an honestly-recorded gap, not a hidden one). Recording a live
//!   network source means pointing the recorder at the same URI and
//!   letting it run its own pipeline; the packets it captures are
//!   its own demux session's, byte-identical for static files,
//!   legitimately *not* guaranteed identical for a live edge.
//! * **Bounded, cancel-safe, honest partials** — inherited from the
//!   7.E1 runner, not re-implemented: the stage queue is bounded on
//!   two axes with parking backpressure, [`CancelToken`] fires
//!   cooperatively at stage boundaries, and on cancel the writer
//!   finalizes best-effort so the `RIFF`/`data` sizes describe
//!   exactly the samples that made it (a partial recording is a
//!   *playable prefix*, never a lie about its own length).
//!
//! # The PCM fast path is a different graph (and stays one)
//!
//! A source that is *already* unpacked little-endian PCM does not
//! need recording's decode stage at all — the 7.E1
//! [`SinkSpec::WavCopyPcm`](crate::SinkSpec::WavCopyPcm) stream-copy
//! re-wraps its payload bytes verbatim. [`Recorder::record`] always
//! decodes (the honest "record what the decoder produces" contract,
//! including the planar→interleaved and byte-order normalizations);
//! hosts that know their source is raw PCM can assemble the copy
//! graph through [`submit`](crate::submit) directly.
//!
//! # Example (a host that owns a demuxer and a decoder factory)
//!
//! ```no_run
//! use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
//! use media_types::{EngineError, Frame, Track};
//! use plugin_api::{Demuxer, DemuxEvent};
//! use transcode::record::Recorder;
//! use transcode::{JobLimits, JobOutcome};
//! # struct NopFactory;
//! # impl transcode::graph::DecoderFactory for NopFactory {
//! #     fn open(&self, _track: &Track) -> Result<Box<dyn Decoder>, EngineError> {
//! #         Ok(Box::new(NopDecoder))
//! #     }
//! # }
//! # struct NopDecoder;
//! # impl Decoder for NopDecoder {
//! #     fn configure(&mut self, _cfg: &DecoderConfig) -> Result<(), EngineError> {
//! #         Ok(())
//! #     }
//! #     fn decode(&mut self, _packet: media_types::Packet, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
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
//! # fn make_output() -> Box<dyn transcode::WriteSeek> {
//! #     struct Sink;
//! #     impl std::io::Write for Sink {
//! #         fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> { Ok(buf.len()) }
//! #         fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
//! #     }
//! #     impl std::io::Seek for Sink {
//! #         fn seek(&mut self, _to: std::io::SeekFrom) -> std::io::Result<u64> { Ok(0) }
//! #     }
//! #     Box::new(Sink)
//! # }
//! # fn main() {
//! let recorder = Recorder::new(
//!     Box::new(NopDemuxer),
//!     Box::new(NopFactory),
//!     make_output(),
//! );
//! let job = match recorder.record(JobLimits::default()) {
//!     Ok(job) => job,
//!     Err(err) => {
//!         eprintln!("record refused: {err}");
//!         return;
//!     }
//! };
//! match job.wait() {
//!     JobOutcome::Completed(stats) => {
//!         eprintln!("recorded {} sample frames", stats.sample_frames_written)
//!     }
//!     JobOutcome::Cancelled(stats) => {
//!         eprintln!("stopped at {} bytes (partial file is honest)", stats.bytes_written)
//!     }
//!     JobOutcome::Failed(err) => eprintln!("record failed: {err}"),
//! }
//! # }
//! ```

use media_types::EngineError;
use plugin_api::Demuxer;

use crate::graph::{
    DecodeStage, DecoderFactory, SinkSpec, SourceSpec, TrackSelection, TranscodeGraph,
    WavPcmTarget, WriteSeek,
};
use crate::job::JobHandle;
use crate::limits::JobLimits;
use crate::runner::submit;

/// The one-call record-to-WAV assembly (module docs): a demuxer, the
/// host's decoder factory, and the output sink, plus the knobs a
/// recording host actually turns — which track, and which PCM flavor
/// to write.
///
/// Build with [`Recorder::new`], optionally
/// [`select`](Recorder::select) a track (the default is the source's
/// single audio track — zero or several audio tracks are typed
/// failures asking the host to name one) or
/// [`target`](Recorder::target) a sample flavor, then
/// [`record`](Recorder::record) submits the job.
pub struct Recorder {
    demuxer: Box<dyn Demuxer>,
    factory: Box<dyn DecoderFactory>,
    output: Box<dyn WriteSeek>,
    selection: TrackSelection,
    target: WavPcmTarget,
    extra_data: Option<media_types::packet::SharedBytes>,
}

impl Recorder {
    /// Assemble a recorder over one source and one output.
    pub fn new(
        demuxer: Box<dyn Demuxer>,
        factory: Box<dyn DecoderFactory>,
        output: Box<dyn WriteSeek>,
    ) -> Recorder {
        Recorder {
            demuxer,
            factory,
            output,
            selection: TrackSelection::DefaultAudio,
            target: WavPcmTarget::Source,
            extra_data: None,
        }
    }

    /// Which track to record (the default: the source's one audio
    /// track). Recording transcribes audio — a non-audio selection
    /// fails the job typed at track selection, never silently.
    #[must_use]
    pub fn select(mut self, selection: TrackSelection) -> Recorder {
        self.selection = selection;
        self
    }

    /// The PCM flavor to write (the default: whatever the decode
    /// path yields, byte-identical after the planar→interleaved and
    /// native→little-endian normalizations). A specific flavor is
    /// legal only when it is the source's own or a lossless widening
    /// of it — see [`WavPcmTarget`].
    #[must_use]
    pub fn target(mut self, target: WavPcmTarget) -> Recorder {
        self.target = target;
        self
    }

    /// Out-of-band decoder configuration bytes (the WAV `fmt `
    /// payload, an AAC AudioSpecificConfig, …) when the container
    /// separates them from the stream — handed to the decoder at
    /// configure time. The host that opened the source knows whether
    /// its container carries any; the recorder never invents them.
    #[must_use]
    pub fn extra_data(mut self, extra: media_types::packet::SharedBytes) -> Recorder {
        self.extra_data = Some(extra);
        self
    }

    /// Submit the recording job (validate, spawn, hand back the
    /// handle). This is synchronous and does no container I/O — the
    /// demuxer's `start` runs on the job's pump thread.
    ///
    /// # Errors
    /// [`submit`]'s typed refusals (a filter row — recording
    /// requests none; sink shape — assembled legal here; thread
    /// spawn refusal). Track-level legality (audio present,
    /// unambiguous) is runtime knowledge checked on the pump thread
    /// and surfaced as the job's `Failed` outcome.
    pub fn record(self, limits: JobLimits) -> Result<JobHandle, EngineError> {
        let Recorder {
            demuxer,
            factory,
            output,
            selection,
            target,
            extra_data,
        } = self;
        let graph = TranscodeGraph {
            source: SourceSpec {
                demuxer,
                selection,
                extra_data,
            },
            decode: Some(DecodeStage::new(factory)),
            filters: Vec::new(),
            sink: SinkSpec::WavPcm { output, target },
        };
        submit(graph, limits)
    }
}
