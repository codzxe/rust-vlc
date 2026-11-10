//! The card 7.E1 job-lifecycle acceptance suite (issue #43 AC b/f):
//! queued→running→progress→completed/failed/cancelled over the real
//! two-thread pipeline, with a scripted demuxer the test drives
//! packet-by-packet — so the lifecycle, the bounded event hub and the
//! cancellation promptness are observed end-to-end, not just at the
//! unit level.
//!
//! # The scripted source
//!
//! A `Demuxer` backed by an mpsc channel: the test sends packets (or
//! drops the sender for end-of-stream) while the pump consumes them.
//! Holding the sender parks the pump inside `next_event`, which is
//! exactly the mid-flight observation point the cancel test needs;
//! nothing sleeps-and-hopes except the bounded `eventually` polls.
//!
//! # Bounds under proof (AC c/f)
//!
//! * the stage queue's high-water marks (surfaced in the terminal
//!   stats) never exceed the configured axes;
//! * a two-slot event hub drops and **counts** progress events while
//!   the pipeline runs to completion — the worker never blocks on
//!   observability;
//! * cancellation is honored at stage boundaries: the terminal is
//!   `Cancelled` (never a post-cancel `Completed`), and the partial
//!   WAV is structurally valid (our own demuxer re-parses it).

use std::io::{Seek, SeekFrom, Write};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use codec_api::Decoder;
use codecs::audio::pcm::PcmDecoder;
use demux::wav::WavDemuxer;
use media_types::ids::SourceUri;
use media_types::ids::{CodecDescriptor, MediaKind, StreamId, TrackFlags, TrackId};
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, TimeBase, TimeSpan, TrackTiming};
use media_types::{EngineError, GenerationId, Packet, Track};
use plugin_api::{DemuxEvent, Demuxer, SeekOrigin, SourcePosition, SourceStream};
use testkit::{Must, assert_err};
use transcode::graph::DecodeStage;
use transcode::graph::DecoderFactory;
use transcode::graph::SinkSpec;
use transcode::graph::SourceSpec;
use transcode::graph::TrackSelection;
use transcode::graph::TranscodeGraph;
use transcode::graph::WavPcmTarget;
use transcode::graph::WriteSeek;
use transcode::{JobEvent, JobLimits, JobOutcome, JobState, submit};

/// One scripted demuxer step.
enum Step {
    /// A packet of the given bytes.
    Packet(Vec<u8>),
    /// Explicit end of stream (dropping the sender says it too).
    Eos,
}

/// The scripted source: an mpsc-backed `Demuxer` the test drives.
struct ScriptedDemuxer {
    rx: mpsc::Receiver<Step>,
    track: Track,
}

impl Demuxer for ScriptedDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        Ok(vec![self.track.clone()])
    }
    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        match self.rx.recv() {
            Ok(Step::Packet(bytes)) => Ok(DemuxEvent::Packet(Packet {
                track: self.track.id,
                pts: Stamp::Missing,
                dts: Stamp::Missing,
                duration: None,
                data: SharedBytes::from_vec(bytes),
                keyframe: true,
                discontinuity: false,
                generation: GenerationId::FIRST,
            })),
            Ok(Step::Eos) | Err(_) => Ok(DemuxEvent::EndOfStream),
        }
    }
    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        Ok(TimeSpan::UNKNOWN)
    }
}

/// The one audio track every script carries (s16le stereo — the
/// shape the fixture corpus uses).
fn audio_track() -> Track {
    Track {
        id: TrackId::new(0),
        stream: StreamId::new(0),
        kind: MediaKind::Audio,
        language: None,
        codec: CodecDescriptor::new("pcm_s16le").must(),
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: TimeSpan::UNKNOWN,
            base: TimeBase::new(1, 1000).must(),
            gaps: Vec::new(),
        },
    }
}

/// The factory our scripted jobs decode through (s16le stereo 48 kHz
/// — the `PcmDecoder::with_format` seam; the script's facts).
struct ScriptPcm;

impl DecoderFactory for ScriptPcm {
    fn open(&self, _track: &Track) -> Result<Box<dyn Decoder>, EngineError> {
        let decoder = PcmDecoder::with_format(2, 48_000).must();
        Ok(Box::new(decoder))
    }
}

/// A `Write + Seek` sink into a shared buffer (the round-trip file's
/// twin-reader shape).
struct SharedCursor {
    twin: Arc<Mutex<Vec<u8>>>,
    pos: usize,
}

fn sink_pair() -> (Box<dyn WriteSeek>, Arc<Mutex<Vec<u8>>>) {
    let twin: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    (
        Box::new(SharedCursor {
            twin: Arc::clone(&twin),
            pos: 0,
        }),
        twin,
    )
}

impl Write for SharedCursor {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.twin.lock().unwrap_or_else(|p| p.into_inner());
        if self.pos + bytes.len() > guard.len() {
            guard.resize(self.pos + bytes.len(), 0);
        }
        guard[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
        self.pos += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for SharedCursor {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        match to {
            SeekFrom::Start(n) => self.pos = n as usize,
            SeekFrom::Current(n) => {
                self.pos = usize::try_from(self.pos as i64 + n).unwrap_or(self.pos);
            }
            SeekFrom::End(n) => {
                let len = self.twin.lock().unwrap_or_else(|p| p.into_inner()).len() as i64;
                self.pos = usize::try_from(len + n).unwrap_or(self.pos);
            }
        }
        Ok(self.pos as u64)
    }
}

/// An in-memory `SourceStream` (the re-demux vehicle for the partial
/// cancelled output).
struct MemStream {
    uri: SourceUri,
    data: Arc<Vec<u8>>,
    pos: usize,
}

impl MemStream {
    fn over(bytes: Vec<u8>) -> MemStream {
        MemStream {
            uri: SourceUri::parse("memory://transcode-lifecycle").must(),
            data: Arc::new(bytes),
            pos: 0,
        }
    }
}

impl SourceStream for MemStream {
    fn uri(&self) -> &SourceUri {
        &self.uri
    }
    fn size(&self) -> Option<u64> {
        Some(self.data.len() as u64)
    }
    fn supports_seek(&self) -> bool {
        true
    }
    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let end = (self.pos + len).min(self.data.len());
        let out = self.data[self.pos..end].to_vec();
        self.pos = end;
        Ok(out)
    }
    fn read_into(&mut self, out: &mut [u8]) -> Result<usize, EngineError> {
        let end = (self.pos + out.len()).min(self.data.len());
        out[..end - self.pos].copy_from_slice(&self.data[self.pos..end]);
        let got = end - self.pos;
        self.pos = end;
        Ok(got)
    }
    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let target: u64 = match origin {
            SeekOrigin::Start(to) => to,
            SeekOrigin::Current(delta) => match (self.pos as i64).checked_add(delta) {
                Some(n) if n >= 0 => n as u64,
                _ => return Err(mem_seek_error()),
            },
        };
        if target > self.data.len() as u64 {
            return Err(mem_seek_error());
        }
        self.pos = target as usize;
        Ok(SourcePosition(target))
    }
    fn position(&self) -> SourcePosition {
        SourcePosition(self.pos as u64)
    }
}

fn mem_seek_error() -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Open,
        media_types::ErrorKind::Inaccessible {
            reason: media_types::error::AccessDenied::NotFound,
        },
        media_types::error::ErrorContext::new("transcode.test.mem-seek-out-of-range"),
    )
}

/// Assemble the decode-path graph over a scripted source.
fn decode_graph(rx: mpsc::Receiver<Step>) -> (TranscodeGraph, Arc<Mutex<Vec<u8>>>) {
    let (sink, twin) = sink_pair();
    let graph = TranscodeGraph {
        source: SourceSpec {
            demuxer: Box::new(ScriptedDemuxer {
                rx,
                track: audio_track(),
            }),
            selection: TrackSelection::DefaultAudio,
            extra_data: None,
        },
        decode: Some(DecodeStage::new(Box::new(ScriptPcm))),
        filters: Vec::new(),
        sink: SinkSpec::WavPcm {
            output: sink,
            target: WavPcmTarget::Source,
        },
    };
    (graph, twin)
}

/// Poll `probe` until it answers `true`, or fail after a bounded wait
/// (deterministic tests on scheduler-dependent facts).
fn eventually<F: FnMut() -> bool>(mut probe: F, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if probe() {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    unreachable!("timed out waiting for {what}");
}

/// Read the sink's bytes back through the twin.
fn file_of(twin: &Arc<Mutex<Vec<u8>>>) -> Vec<u8> {
    twin.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

/// The s16le-stereo-48k scripted packet size: 16 KiB = 4096 frames
/// (the 64 KiB progress cadence fires every fourth packet).
const PACKET: usize = 16 * 1024;
const PACKET_FRAMES: u64 = (PACKET / 4) as u64;

#[test]
fn completed_lifecycle_runs_progresses_completes_and_stays_bounded() {
    let (tx, rx) = mpsc::channel();
    let (graph, twin) = decode_graph(rx);
    let job = submit(graph, JobLimits::default()).must();
    // Twenty packets of 16 KiB: 320 KiB through a 64-slot/1 MiB queue.
    for salt in 0..20u8 {
        tx.send(Step::Packet(vec![salt; PACKET])).must();
    }
    tx.send(Step::Eos).must();
    drop(tx);

    let stats = match job.wait() {
        JobOutcome::Completed(stats) => stats,
        other => unreachable!("completed, got {other:?}"),
    };
    assert_eq!(stats.packets_read, 20);
    assert_eq!(stats.bytes_read as usize, 20 * PACKET);
    assert_eq!(stats.bytes_written as usize, 20 * PACKET);
    assert_eq!(stats.sample_frames_written, 20 * PACKET_FRAMES);
    assert_eq!(stats.frames_decoded, 20, "one decoded frame per packet");
    // The bounded-resource proof, end to end: the stage queue's
    // high-water marks stayed inside the configured axes.
    assert!(
        stats.queue_peak_slots <= JobLimits::default().queue_slots(),
        "slots high-water {} past the axis",
        stats.queue_peak_slots
    );
    assert!(
        stats.queue_peak_bytes <= JobLimits::default().queue_bytes(),
        "bytes high-water {} past the axis",
        stats.queue_peak_bytes
    );

    // The terminal state is observable and `wait` is idempotent.
    assert_eq!(job.state(), JobState::Completed);
    assert!(matches!(job.wait(), JobOutcome::Completed(_)));

    // The event stream: Running first, then progress on the cadence,
    // nothing dropped at the default capacity.
    let mut events = Vec::new();
    while let Some(event) = job.try_next_event() {
        events.push(event);
    }
    assert!(
        matches!(events.first(), Some(JobEvent::Running)),
        "Running is the first event"
    );
    let snapshots: Vec<&JobEvent> = events
        .iter()
        .filter(|event| matches!(event, JobEvent::Progress(_)))
        .collect();
    assert!(
        snapshots.len() >= 4,
        "320 KiB at a 64 KiB cadence: at least four progress events, got {}",
        snapshots.len()
    );
    assert_eq!(job.dropped_events(), 0);
    // The last progress snapshot agrees with the terminal accounting.
    match snapshots.last() {
        Some(JobEvent::Progress(last)) => {
            assert_eq!(last.bytes_written, stats.bytes_written);
            assert_eq!(last.sample_frames_written, stats.sample_frames_written);
        }
        other => unreachable!("progress snapshots, got {other:?}"),
    }

    // The output is a whole, honest WAV: canonical header, twenty
    // salt blocks in order (FIFO through the whole pipeline).
    let output = file_of(&twin);
    assert_eq!(output.len(), 44 + 20 * PACKET);
    assert_eq!(&output[0..4], b"RIFF");
    for (i, salt) in (0..20u8).enumerate() {
        let at = 44 + i * PACKET;
        assert_eq!(&output[at..at + PACKET], &vec![salt; PACKET], "block {i}");
    }
}

#[test]
fn cancellation_is_prompt_terminal_and_leaves_a_truthful_prefix() {
    let (tx, rx) = mpsc::channel();
    let (graph, twin) = decode_graph(rx);
    let job = submit(graph, JobLimits::default()).must();
    // Five packets, then HOLD the sender: the pump parks inside
    // next_event once the channel empties — mid-flight, exactly.
    for salt in 0..5u8 {
        tx.send(Step::Packet(vec![salt; PACKET])).must();
    }
    eventually(
        || job.stats().bytes_written >= PACKET as u64,
        "at least one packet written",
    );
    // Cancel while the pipeline is mid-flight, then release the pump.
    job.cancel();
    drop(tx);

    let stats = match job.wait() {
        JobOutcome::Cancelled(stats) => stats,
        other => unreachable!("cancelled, got {other:?}"),
    };
    assert_eq!(job.state(), JobState::Cancelled);
    // A truthful prefix: at least the observed packet, at most all
    // five — whole packets only (writes are packet-atomic).
    assert!(
        stats.bytes_written >= PACKET as u64 && stats.bytes_written <= 5 * PACKET as u64,
        "prefix bytes {}",
        stats.bytes_written
    );
    assert_eq!(stats.bytes_written % PACKET as u64, 0, "whole packets only");

    // The partial file is structurally valid and tells the truth:
    // our own demuxer re-parses it, payload == the counted prefix.
    let output = file_of(&twin);
    assert_eq!(
        output.len(),
        44 + stats.bytes_written as usize,
        "header + exactly the counted payload"
    );
    let data_size = u32::from_le_bytes(output[40..44].try_into().must());
    assert_eq!(data_size as u64, stats.bytes_written, "patched data size");
    let mut redemux = WavDemuxer::new(Box::new(MemStream::over(output))).must();
    let tracks = redemux.start().must();
    assert_eq!(tracks.len(), 1);
    let mut payload = 0usize;
    loop {
        match redemux.next_event().must() {
            DemuxEvent::Packet(packet) => payload += packet.data.len(),
            DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => {}
            DemuxEvent::EndOfStream => break,
        }
    }
    assert_eq!(payload as u64, stats.bytes_written);
}

#[test]
fn a_full_event_hub_drops_and_counts_but_never_blocks_the_pipeline() {
    let (tx, rx) = mpsc::channel();
    let (graph, _twin) = decode_graph(rx);
    // A two-slot hub: politeness, not memory (the streaming stack's
    // drop-and-count doctrine).
    let limits = JobLimits::new(64, 1024 * 1024, 2, 64, 4).must();
    let job = submit(graph, limits).must();
    // Forty packets: 640 KiB → ten cadence progress events plus the
    // final one, against a hub that keeps two.
    for salt in 0..40u8 {
        tx.send(Step::Packet(vec![salt; PACKET])).must();
    }
    tx.send(Step::Eos).must();
    drop(tx);

    let stats = match job.wait() {
        JobOutcome::Completed(stats) => stats,
        other => unreachable!("the hub never blocks the worker, got {other:?}"),
    };
    assert_eq!(stats.bytes_written as usize, 40 * PACKET);
    // Exactly the capacity was kept (Running + the first progress;
    // nothing drains during the run), the rest dropped and counted.
    let mut seen = 0;
    while job.try_next_event().is_some() {
        seen += 1;
    }
    assert_eq!(seen, 2, "the hub kept exactly its capacity");
    assert_eq!(
        job.dropped_events(),
        10,
        "ten progress events (nine cadence + one final) dropped past the two kept"
    );
}

#[test]
fn a_source_without_audio_fails_the_job_typed() {
    let (tx, rx) = mpsc::channel();
    // A video-only script feeding a WAV-encode graph: the DefaultAudio
    // selection has nothing to pick.
    let track = Track {
        kind: MediaKind::Video,
        codec: CodecDescriptor::new("mjpeg").must(),
        ..audio_track()
    };
    let (sink, _twin) = sink_pair();
    let graph = TranscodeGraph {
        source: SourceSpec {
            demuxer: Box::new(ScriptedDemuxer { rx, track }),
            selection: TrackSelection::DefaultAudio,
            extra_data: None,
        },
        decode: Some(DecodeStage::new(Box::new(ScriptPcm))),
        filters: Vec::new(),
        sink: SinkSpec::WavPcm {
            output: sink,
            target: WavPcmTarget::Source,
        },
    };
    let job = submit(graph, JobLimits::default()).must();
    drop(tx); // immediate end of stream
    let err = match job.wait() {
        JobOutcome::Failed(err) => err,
        other => unreachable!("no audio track fails, got {other:?}"),
    };
    assert_eq!(err.context.code, "transcode.source.audio-track-missing");
    assert_eq!(job.state(), JobState::Failed);
}

#[test]
fn a_filter_request_is_refused_at_submit_not_midjob() {
    let (tx, rx) = mpsc::channel();
    let (mut graph, _twin) = decode_graph(rx);
    drop(tx); // the script never runs: validation refuses first
    graph.filters = vec![transcode::FilterStage {
        domain: transcode::FilterDomain::Audio,
        name: "loudnorm".to_owned(),
    }];
    let err = assert_err(submit(graph, JobLimits::default()), "v1 has no filters");
    assert_eq!(err.context.code, "transcode.graph.filter-unimplemented");
}

/// A decoder factory whose `open` refuses — the failing-stage seam
/// for the failure-lifecycle proof (a host registry with no decoder
/// for the track's codec descriptor).
struct RefusingFactory;

impl DecoderFactory for RefusingFactory {
    fn open(&self, _track: &Track) -> Result<Box<dyn Decoder>, EngineError> {
        Err(media_types::EngineError::new(
            media_types::PipelineStage::Decode,
            media_types::ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("transcode.test.no-decoder"),
        ))
    }
}

#[test]
fn a_failing_stage_fails_the_job_typed_with_the_stages_error() {
    let (tx, rx) = mpsc::channel();
    let (sink, _twin) = sink_pair();
    let graph = TranscodeGraph {
        source: SourceSpec {
            demuxer: Box::new(ScriptedDemuxer {
                rx,
                track: audio_track(),
            }),
            selection: TrackSelection::DefaultAudio,
            extra_data: None,
        },
        decode: Some(DecodeStage::new(Box::new(RefusingFactory))),
        filters: Vec::new(),
        sink: SinkSpec::WavPcm {
            output: sink,
            target: WavPcmTarget::Source,
        },
    };
    let job = submit(graph, JobLimits::default()).must();
    tx.send(Step::Packet(vec![0xAB; PACKET])).must();
    drop(tx);
    let err = match job.wait() {
        JobOutcome::Failed(err) => err,
        other => unreachable!("decoder-open refusal fails, got {other:?}"),
    };
    assert_eq!(err.context.code, "transcode.test.no-decoder");
    assert_eq!(job.state(), JobState::Failed);
}
