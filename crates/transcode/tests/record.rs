//! The card 7.E2 record acceptance suite (issue #44 AC): the
//! [`Recorder`] one-call assembly records a committed fixture to
//! WAV, and the recording is verified by OUR own demux + decode —
//! bit-exact against the source's own decoded samples. Plus the
//! honesty refusals: a source with no audio track fails typed, and
//! a mid-record cancel leaves a *playable honest prefix* (sizes
//! patched to the bytes that made it, fewer sample frames than the
//! source).
//!
//! Fixtures: the committed ffmpeg tones under
//! `tests/fixtures/generated/valid/` (the 7.E1 round-trip suite's
//! own corpus — the record path shares its decode→encode spine, so
//! the same truths apply).

use std::io::{Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use codec_api::{Decoder, DecoderConfig, FrameSink, ThreadBudget};
use codecs::audio::pcm::PcmDecoder;
use demux::wav::WavDemuxer;
use demux::wav::format::PcmSample;
use media_types::frame::Frame;
use media_types::ids::SourceUri;
use media_types::ids::{CodecDescriptor, TrackId};
use media_types::packet::SharedBytes;
use media_types::time::Stamp;
use media_types::{EngineError, GenerationId, Packet, Track};
use plugin_api::{Access, DemuxEvent, Demuxer, SeekOrigin, SourcePosition, SourceStream};
use testkit::{Must, fixture_path};
use transcode::graph::DecoderFactory;
use transcode::record::Recorder;
use transcode::{JobLimits, JobOutcome};

// ---------------------------------------------------------------------------
// Shared plumbing (the 7.E1 round-trip suite's shapes, reproduced
// test-locally — integration suites do not share code across files)
// ---------------------------------------------------------------------------

/// A `Write + Seek` sink into a shared buffer (the readable twin).
struct SharedCursor {
    twin: Arc<Mutex<Vec<u8>>>,
    pos: usize,
}

fn sink_pair() -> (Box<dyn transcode::WriteSeek>, Arc<Mutex<Vec<u8>>>) {
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

/// An in-memory `SourceStream` (size-known, seekable).
struct MemStream {
    uri: SourceUri,
    data: Arc<Vec<u8>>,
    pos: usize,
}

impl MemStream {
    fn over(bytes: Vec<u8>) -> MemStream {
        MemStream {
            uri: SourceUri::parse("memory://record-suite").must(),
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

fn open_fixture(relative: &str) -> WavDemuxer {
    let path = fixture_path(relative).must();
    let uri = SourceUri::parse(&format!("file://{}", path.to_str().unwrap_or_default())).must();
    let stream = input::FileAccess::new().open(&uri).must();
    WavDemuxer::new(stream).must()
}

struct Demuxed {
    codec: String,
    fmt: Vec<u8>,
    payload: Vec<u8>,
}

fn demux_all(demuxer: &mut WavDemuxer) -> Demuxed {
    let tracks = demuxer.start().must();
    let codec = tracks
        .first()
        .map(|track| track.codec.as_str().to_owned())
        .unwrap_or_default();
    let fmt = demuxer.fmt_chunk_bytes().unwrap_or(&[]).to_vec();
    let mut payload = Vec::new();
    loop {
        match demuxer.next_event().must() {
            DemuxEvent::Packet(packet) => payload.extend_from_slice(&packet.data),
            DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => {}
            DemuxEvent::EndOfStream => break,
        }
    }
    Demuxed {
        codec,
        fmt,
        payload,
    }
}

/// The flattened decoded samples of a PCM payload (our decoder).
fn decoded_bytes(fmt: &[u8], codec: &str, payload: &[u8]) -> Vec<u8> {
    let mut decoder = PcmDecoder::new();
    let config = DecoderConfig::new(
        CodecDescriptor::new(codec).must(),
        GenerationId::FIRST,
        ThreadBudget::default(),
        None,
        Some(SharedBytes::from_vec(fmt.to_vec())),
    );
    decoder.configure(&config).must();
    let packet = Packet {
        track: TrackId::new(0),
        pts: Stamp::Missing,
        dts: Stamp::Missing,
        duration: None,
        data: SharedBytes::from_vec(payload.to_vec()),
        keyframe: true,
        discontinuity: false,
        generation: GenerationId::FIRST,
    };
    let mut sink = FlattenSink::default();
    let _ = decoder.decode(packet, &mut sink).must();
    let _ = decoder.flush(&mut sink).must();
    sink.bytes
}

#[derive(Default)]
struct FlattenSink {
    bytes: Vec<u8>,
}

impl FrameSink for FlattenSink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        if let Frame::Audio(audio) = frame {
            self.bytes.extend_from_slice(audio.samples.as_slice());
        }
        Ok(())
    }
}

/// The fixture-seeded PCM factory (the `PcmDecoder::with_format`
/// seam — the host wiring point for `fmt `-level facts).
struct FixturePcm {
    channels: u32,
    rate: u32,
}

impl DecoderFactory for FixturePcm {
    fn open(&self, _track: &Track) -> Result<Box<dyn Decoder>, EngineError> {
        let decoder = PcmDecoder::with_format(self.channels, self.rate).must();
        Ok(Box::new(decoder))
    }
}

/// A factory that refuses everything — "no decoder serves this
/// codec" is a recording-time fact, surfaced as the factory's own
/// typed error.
struct NoDecoder;

impl DecoderFactory for NoDecoder {
    fn open(&self, track: &Track) -> Result<Box<dyn Decoder>, EngineError> {
        Err(EngineError::new(
            media_types::PipelineStage::Decode,
            media_types::ErrorKind::Unsupported {
                format: Some(media_types::error::FormatInfo {
                    name: Some(track.codec.as_str().to_owned()),
                    sniffed: Vec::new(),
                }),
            },
            media_types::error::ErrorContext::new("record.test.no-decoder"),
        ))
    }
}

// ---------------------------------------------------------------------------
// AC: recorded output verified by our own demux + decode
// ---------------------------------------------------------------------------

#[test]
fn recorder_output_redemuxes_and_redecodes_bitexactly() {
    const WHAT: &str = "generated/valid/wav_pcm_s16le_44k1_st.wav";
    // The verification side: the source's own demux + decode.
    let mut verify = open_fixture(WHAT);
    let demuxed = demux_all(&mut verify);
    let want = decoded_bytes(&demuxed.fmt, "pcm_s16le", &demuxed.payload);

    // The recording: a fresh source, the Recorder one-call assembly.
    let (sink, twin) = sink_pair();
    let job = Recorder::new(
        Box::new(open_fixture(WHAT)),
        Box::new(FixturePcm {
            channels: 2,
            rate: 44_100,
        }),
        sink,
    )
    .record(JobLimits::default())
    .must();
    let stats = match job.wait() {
        JobOutcome::Completed(stats) => stats,
        other => unreachable!("record completes, got {other:?}"),
    };
    assert_eq!(stats.sample_frames_written, 8_820);
    assert_eq!(stats.bytes_written as usize, demuxed.payload.len());
    assert_eq!(stats.frames_decoded, stats.packets_read);

    // Our demuxer accepts the recording…
    let output = twin.lock().unwrap_or_else(|p| p.into_inner()).clone();
    let mut redemux = WavDemuxer::new(Box::new(MemStream::over(output))).must();
    let round = demux_all(&mut redemux);
    assert_eq!(round.codec, "pcm_s16le");
    // …and our decoder returns the source's samples bit for bit.
    let got = decoded_bytes(&round.fmt, "pcm_s16le", &round.payload);
    assert_eq!(got, want, "the recording decodes to the source's samples");
    assert_eq!(round.payload, demuxed.payload, "payload identity");
}

/// A widening target records losslessly (s16 source → s32 output):
/// every source sample's value survives, width-extended.
#[test]
fn recorder_widening_target_records_losslessly() {
    const WHAT: &str = "generated/valid/wav_pcm_s16le_44k1_st.wav";
    let mut verify = open_fixture(WHAT);
    let demuxed = demux_all(&mut verify);

    let (sink, twin) = sink_pair();
    let job = Recorder::new(
        Box::new(open_fixture(WHAT)),
        Box::new(FixturePcm {
            channels: 2,
            rate: 44_100,
        }),
        sink,
    )
    .target(transcode::WavPcmTarget::Sample(PcmSample::S32Le))
    .record(JobLimits::default())
    .must();
    let stats = match job.wait() {
        JobOutcome::Completed(stats) => stats,
        other => unreachable!("widening records, got {other:?}"),
    };
    assert_eq!(stats.sample_frames_written, 8_820);
    assert_eq!(stats.bytes_written as usize, demuxed.payload.len() * 2);

    // The widened recording re-demuxes as s32 stereo…
    let output = twin.lock().unwrap_or_else(|p| p.into_inner()).clone();
    let mut redemux = WavDemuxer::new(Box::new(MemStream::over(output))).must();
    let round = demux_all(&mut redemux);
    assert_eq!(round.codec, "pcm_s32le");
    // …and every s16 sample's value survives the widening exactly.
    let s16: Vec<i16> = demuxed
        .payload
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| i16::from_le_bytes(*c))
        .collect();
    let s32: Vec<i32> = round
        .payload
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| i32::from_le_bytes(*c))
        .collect();
    assert_eq!(s32.len(), s16.len());
    // The widening is the fixed-point value-preserving shift (the
    // conversion module's own pinned convention): a sample's
    // full-scale position survives, and the shift round-trips.
    for (i, (narrow, wide)) in s16.iter().zip(&s32).enumerate() {
        assert_eq!(
            i32::from(*narrow) << 16,
            *wide,
            "sample {i} widened losslessly"
        );
    }
}

// ---------------------------------------------------------------------------
// Honesty refusals
// ---------------------------------------------------------------------------

/// A source whose track has no decoder fails the job with the
/// factory's own typed error — the recorder never claims a codec it
/// cannot decode.
#[test]
fn recorder_surfaces_the_factory_refusal_typed() {
    let (sink, _twin) = sink_pair();
    let job = Recorder::new(
        Box::new(open_fixture("generated/valid/wav_pcm_f32le_48k_st.wav")),
        Box::new(NoDecoder),
        sink,
    )
    .record(JobLimits::default())
    .must();
    let err = match job.wait() {
        JobOutcome::Failed(err) => err,
        other => unreachable!("no decoder fails, got {other:?}"),
    };
    assert_eq!(err.context.code, "record.test.no-decoder");
    assert_eq!(err.stage, media_types::PipelineStage::Decode);
}

/// A source with no audio track at all fails typed at selection
/// (recording transcribes audio).
#[test]
fn recorder_without_audio_fails_typed() {
    /// A demuxer exposing one VIDEO track and no packets.
    struct VideoOnlyDemuxer;
    impl Demuxer for VideoOnlyDemuxer {
        fn start(&mut self) -> Result<Vec<Track>, EngineError> {
            Ok(vec![Track {
                id: TrackId::new(0),
                stream: media_types::ids::StreamId::new(0),
                kind: media_types::MediaKind::Video,
                language: None,
                codec: CodecDescriptor::new("h264").must(),
                title: None,
                flags: media_types::ids::TrackFlags::NONE,
                timing: media_types::time::TrackTiming {
                    span: media_types::time::TimeSpan::UNKNOWN,
                    base: media_types::time::TimeBase::new(1, 900).must(),
                    gaps: Vec::new(),
                },
            }])
        }
        fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
            Ok(DemuxEvent::EndOfStream)
        }
        fn seek(
            &mut self,
            _target: media_types::TimeSpan,
        ) -> Result<media_types::TimeSpan, EngineError> {
            Ok(media_types::TimeSpan::UNKNOWN)
        }
    }
    let (sink, _twin) = sink_pair();
    let job = Recorder::new(Box::new(VideoOnlyDemuxer), Box::new(NoDecoder), sink)
        .record(JobLimits::default())
        .must();
    let err = match job.wait() {
        JobOutcome::Failed(err) => err,
        other => unreachable!("video-only fails, got {other:?}"),
    };
    // The selection refusal precedes any decoder open.
    assert_eq!(err.context.code, "transcode.source.audio-track-missing");
}

// ---------------------------------------------------------------------------
// Cancel: honest partial output (the 7.E1 lifecycle suite's scripted
// source, driven through the Recorder assembly)
// ---------------------------------------------------------------------------

/// One scripted demuxer step (a packet; dropping the sender says
/// end-of-stream).
enum Step {
    Packet(Vec<u8>),
}

/// An mpsc-backed `Demuxer` the test drives packet-by-packet —
/// holding the sender parks the pump mid-flight, the observation
/// point a mid-record cancel needs.
struct ScriptedDemuxer {
    rx: std::sync::mpsc::Receiver<Step>,
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
            Err(_) => Ok(DemuxEvent::EndOfStream),
        }
    }
    fn seek(
        &mut self,
        _target: media_types::TimeSpan,
    ) -> Result<media_types::TimeSpan, EngineError> {
        Ok(media_types::TimeSpan::UNKNOWN)
    }
}

fn scripted_track() -> Track {
    Track {
        id: TrackId::new(0),
        stream: media_types::ids::StreamId::new(0),
        kind: media_types::MediaKind::Audio,
        language: None,
        codec: CodecDescriptor::new("pcm_s16le").must(),
        title: None,
        flags: media_types::ids::TrackFlags::NONE,
        timing: media_types::time::TrackTiming {
            span: media_types::time::TimeSpan::UNKNOWN,
            base: media_types::time::TimeBase::new(1, 44_100).must(),
            gaps: Vec::new(),
        },
    }
}

/// Bounded poll: run `probe` until it answers true or the budget
/// dies (the lifecycle suite's `eventually` — no sleep-and-hope).
fn eventually(probe: impl Fn() -> bool, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !probe() {
        if Instant::now() > deadline {
            unreachable!("timed out waiting for {what}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn mid_record_cancel_leaves_an_honest_playable_prefix() {
    // Whole stereo s16 sample frames: 4 bytes each.
    const PACKET: usize = 1024;
    let (tx, rx) = std::sync::mpsc::channel::<Step>();
    let (sink, twin) = sink_pair();
    let job = Recorder::new(
        Box::new(ScriptedDemuxer {
            rx,
            track: scripted_track(),
        }),
        Box::new(FixturePcm {
            channels: 2,
            rate: 44_100,
        }),
        sink,
    )
    .record(JobLimits::default())
    .must();
    // Five packets, then HOLD the sender: the pump parks inside
    // next_event once the channel empties — mid-record, exactly.
    for salt in 0..5u8 {
        tx.send(Step::Packet(vec![salt; PACKET])).must();
    }
    eventually(
        || job.stats().bytes_written >= PACKET as u64,
        "at least one recorded packet",
    );
    job.cancel();
    drop(tx);

    let stats = match job.wait() {
        JobOutcome::Cancelled(stats) => stats,
        other => unreachable!("mid-record stop cancels, got {other:?}"),
    };
    assert_eq!(job.state(), transcode::JobState::Cancelled);
    // A truthful prefix: whole packets only, somewhere between the
    // observed packet and all five.
    assert!(
        stats.bytes_written >= PACKET as u64 && stats.bytes_written <= 5 * PACKET as u64,
        "prefix bytes {}",
        stats.bytes_written
    );
    assert_eq!(stats.bytes_written % PACKET as u64, 0, "whole packets");

    // The partial file is a structurally valid WAV whose sizes tell
    // the truth about the prefix (the honest-partial contract).
    let output = twin.lock().unwrap_or_else(|p| p.into_inner()).clone();
    assert!(output.len() >= 44 + PACKET, "header + some payload");
    assert_eq!(&output[0..4], b"RIFF");
    let riff_size = u32::from_le_bytes(output[4..8].try_into().must());
    let data_size = u32::from_le_bytes(output[40..44].try_into().must());
    assert_eq!(
        output.len(),
        44 + data_size as usize,
        "sizes match the file"
    );
    assert_eq!(riff_size as usize, output.len() - 8, "RIFF size honest");
    assert_eq!(data_size as u64, stats.bytes_written, "data size = stats");
    assert_eq!(data_size as usize % 4, 0, "whole stereo s16 frames");
    // And it re-demuxes: the prefix is playable, not a torn file.
    let mut redemux = WavDemuxer::new(Box::new(MemStream::over(output))).must();
    let round = demux_all(&mut redemux);
    assert_eq!(round.codec, "pcm_s16le");
    assert_eq!(round.payload.len(), data_size as usize);
}
