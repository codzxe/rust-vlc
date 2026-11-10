//! The card 7.E1 round-trip acceptance suites (issue #43 AC d/e): a
//! real committed fixture is decoded by OUR decoder, encoded by OUR
//! WAV writer, then re-demuxed and re-decoded by OUR stack — and the
//! samples come back identical. Plus the stream-copy paths: PCM
//! re-wrapped without decoding (payload bytes verbatim) and the raw
//! elementary-stream dump.
//!
//! # Provenance (no media binaries are created by this crate)
//!
//! The fixtures are the committed ffmpeg products under
//! `tests/fixtures/generated/valid/` (lavfi tones; see the demux
//! crate's WAV/AIFF suite for the recorded ffmpeg truths). Nothing is
//! written to disk by these tests: job output lands in an in-memory
//! `Write + Seek` sink, and the re-demux rides a test-local
//! in-memory `SourceStream` (the same shape the demux suites use).
//!
//! # The claim under test (the honest zero-conversion encode)
//!
//! For an unpacked-LE-PCM source the decode→WAV-encode path is a
//! byte-identity pipeline: the PCM decoder emits native-endian
//! frames, the writer normalizes back to little-endian, so the output
//! WAV's `data` payload equals the source's payload **bit for bit**,
//! and decoding both sides with the same decoder yields the same
//! sample bytes. The comparison is sample-level (decode both sides,
//! compare every frame's format and bytes), with the container-level
//! payload identity asserted alongside as the stronger bonus.

use std::io::{Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

use codec_api::{Decoder, DecoderConfig, FrameSink, ThreadBudget};
use codecs::audio::pcm::PcmDecoder;
use demux::wav::WavDemuxer;
use demux::wav::format::PcmSample;
use media_types::frame::AudioFormat;
use media_types::frame::Frame;
use media_types::ids::SourceUri;
use media_types::ids::{CodecDescriptor, TrackId};
use media_types::packet::SharedBytes;
use media_types::time::Stamp;
use media_types::{EngineError, GenerationId, Packet, Track};
use plugin_api::{Access, Demuxer, DemuxEvent, SeekOrigin, SourcePosition, SourceStream};
use testkit::{Must, assert_err, fixture_path};
use transcode::graph::DecodeStage;
use transcode::graph::DecoderFactory;
use transcode::graph::PcmFacts;
use transcode::graph::SinkSpec;
use transcode::graph::SourceSpec;
use transcode::graph::TrackSelection;
use transcode::graph::TranscodeGraph;
use transcode::graph::WavPcmTarget;
use transcode::graph::WriteSeek;
use transcode::{JobLimits, JobOutcome, submit};

// ---------------------------------------------------------------------------
// Shared plumbing (test-local; the demux suites' own precedents)
// ---------------------------------------------------------------------------

/// A `Write + Seek` sink into a shared buffer — the job's output
/// target. The twin `Arc` reads the bytes back after `wait()` (a
/// `Box<dyn WriteSeek>` cannot be read; the shared buffer is the
/// seam).
struct SharedCursor {
    twin: Arc<Mutex<Vec<u8>>>,
    pos: usize,
}

/// Build the sink and its readable twin.
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

/// An in-memory `SourceStream` — the re-demux vehicle for job output
/// (the demux suites' `BorrowMemSource` shape: size-known, seekable).
struct MemStream {
    uri: SourceUri,
    data: Arc<Vec<u8>>,
    pos: usize,
}

impl MemStream {
    fn over(bytes: Vec<u8>) -> MemStream {
        MemStream {
            uri: SourceUri::parse("memory://transcode-roundtrip").must(),
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

/// The typed refusal for seeks outside the in-memory extent.
fn mem_seek_error() -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Open,
        media_types::ErrorKind::Inaccessible {
            reason: media_types::error::AccessDenied::NotFound,
        },
        media_types::error::ErrorContext::new("transcode.test.mem-seek-out-of-range"),
    )
}

/// Open a committed fixture through the real file access stack (the
/// first vertical's `input` composition — the demux suites'
/// `open_via_input` shape).
fn open_fixture(relative: &str) -> WavDemuxer {
    let path = fixture_path(relative).must();
    let uri = SourceUri::parse(&format!(
        "file://{}",
        path.to_str().unwrap_or_default()
    ))
    .must();
    let stream = input::FileAccess::new().open(&uri).must();
    WavDemuxer::new(stream).must()
}

/// One fully demuxed WAV source: the track, the verbatim `fmt ` bytes
/// (the decoder-config seam), and the concatenated packet payload.
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

/// A `FrameSink` that keeps every audio frame's format and samples —
/// the round-trip comparison side.
#[derive(Default)]
struct CollectSink {
    frames: Vec<(AudioFormat, Vec<u8>)>,
}

impl FrameSink for CollectSink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        match frame {
            Frame::Audio(audio) => {
                let bytes = audio.samples.as_slice().to_vec();
                self.frames.push((audio.format, bytes));
            }
            // Unpacked PCM decodes to audio only.
            _ => unreachable!("pcm decode yields audio frames only"),
        }
        Ok(())
    }
}

/// Decode a PCM payload with OUR decoder (configured from the
/// source's own `fmt ` bytes — the engine's real config path).
fn decode_all(fmt: &[u8], codec: &str, payload: &[u8]) -> CollectSink {
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
    let mut sink = CollectSink::default();
    match decoder.decode(packet, &mut sink).must() {
        codec_api::DecodeOutcome::Emitted { frames } => {
            assert_eq!(frames, 1, "one audio frame per PCM packet");
        }
        other => unreachable!("whole-payload PCM packet decodes, got {other:?}"),
    }
    match decoder.flush(&mut sink).must() {
        codec_api::DecodeOutcome::Drained => {}
        other => unreachable!("PCM flush drains, got {other:?}"),
    }
    sink
}

/// The decoded samples as one byte string (the sample-level compare).
fn flattened(sink: &CollectSink) -> Vec<u8> {
    let mut out = Vec::new();
    for (_, bytes) in &sink.frames {
        out.extend_from_slice(bytes);
    }
    out
}

/// A decoder factory that opens OUR PCM decoder seeded with the
/// fixture's channel/rate facts (the `PcmDecoder::with_format` seam —
/// the runner cannot learn `fmt ` facts before `Demuxer::start`, and
/// the factory is the host's wiring point).
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

// ---------------------------------------------------------------------------
// The decode→encode round trip (AC d)
// ---------------------------------------------------------------------------

/// One fixture's expected shape (the ffmpeg-measured truths recorded
/// by the demux suite: `frames` is ffprobe's `duration_ts`).
struct Case {
    fixture: &'static str,
    codec: &'static str,
    channels: u32,
    rate: u32,
    frames: u64,
}

const CASES: [Case; 4] = [
    Case {
        fixture: "generated/valid/wav_pcm_u8_44k1_mono.wav",
        codec: "pcm_u8",
        channels: 1,
        rate: 44_100,
        frames: 8_820,
    },
    Case {
        fixture: "generated/valid/wav_pcm_s16le_44k1_st.wav",
        codec: "pcm_s16le",
        channels: 2,
        rate: 44_100,
        frames: 8_820,
    },
    Case {
        fixture: "generated/valid/wav_pcm_s24le_48k_st.wav",
        codec: "pcm_s24le",
        channels: 2,
        rate: 48_000,
        frames: 9_600,
    },
    Case {
        fixture: "generated/valid/wav_pcm_f32le_48k_st.wav",
        codec: "pcm_f32le",
        channels: 2,
        rate: 48_000,
        frames: 9_600,
    },
];

#[test]
fn fixtures_roundtrip_decode_encode_redemux_redecode_bitexactly() {
    for case in CASES {
        let what = case.fixture;
        // The verification side: demux + decode the fixture itself.
        let mut verify = open_fixture(what);
        let demuxed = demux_all(&mut verify);
        assert_eq!(demuxed.codec, case.codec, "{what}: track codec");
        let decoded = decode_all(&demuxed.fmt, case.codec, &demuxed.payload);

        // The job: a fresh source instance (the first is consumed),
        // decode → WAV-PCM encode into memory.
        let (sink, twin) = sink_pair();
        let graph = TranscodeGraph {
            source: SourceSpec {
                demuxer: Box::new(open_fixture(what)),
                selection: TrackSelection::DefaultAudio,
                extra_data: None,
            },
            decode: Some(DecodeStage::new(Box::new(FixturePcm {
                channels: case.channels,
                rate: case.rate,
            }))),
            filters: Vec::new(),
            sink: SinkSpec::WavPcm {
                output: sink,
                target: WavPcmTarget::Source,
            },
        };
        let job = submit(graph, JobLimits::default()).must();
        let stats = match job.wait() {
            JobOutcome::Completed(stats) => stats,
            other => unreachable!("{what}: expected Completed, got {other:?}"),
        };
        // Real accounting: every sample frame and byte is counted.
        assert_eq!(stats.sample_frames_written, case.frames, "{what}: frames");
        assert_eq!(stats.bytes_written as usize, demuxed.payload.len(), "{what}: bytes");
        assert_eq!(stats.bytes_read as usize, demuxed.payload.len(), "{what}: read");
        assert!(stats.packets_read >= 1, "{what}: packetized source");
        assert_eq!(stats.frames_decoded, stats.packets_read, "{what}: one frame per packet");

        // The output file: canonical 44-byte header, honest sizes.
        let output = twin.lock().unwrap_or_else(|p| p.into_inner()).clone();
        assert_eq!(output.len(), 44 + demuxed.payload.len(), "{what}: file size");
        assert_eq!(&output[0..4], b"RIFF", "{what}");
        assert_eq!(&output[8..12], b"WAVE", "{what}");
        let riff_size = u32::from_le_bytes(output[4..8].try_into().must());
        let data_size = u32::from_le_bytes(output[40..44].try_into().must());
        assert_eq!(riff_size as usize, 36 + demuxed.payload.len(), "{what}: RIFF size");
        assert_eq!(data_size as usize, demuxed.payload.len(), "{what}: data size");

        // Re-demux OUR output with OUR demuxer.
        let mut redemux = WavDemuxer::new(Box::new(MemStream::over(output))).must();
        let round = demux_all(&mut redemux);
        assert_eq!(round.codec, case.codec, "{what}: round-trip codec");
        // The container-level identity: payload bytes verbatim.
        assert_eq!(round.payload, demuxed.payload, "{what}: payload identity");

        // The sample-level identity: re-decode with OUR decoder and
        // compare every frame's format and every sample byte.
        let redecoded = decode_all(&round.fmt, round.codec.as_str(), &round.payload);
        assert_eq!(
            redecoded.frames.len(),
            decoded.frames.len(),
            "{what}: frame count"
        );
        for (i, ((want_format, want_bytes), (got_format, got_bytes))) in
            decoded.frames.iter().zip(&redecoded.frames).enumerate()
        {
            assert_eq!(want_format, got_format, "{what}: frame {i} format");
            assert_eq!(want_bytes, got_bytes, "{what}: frame {i} samples");
        }
        assert_eq!(flattened(&redecoded), flattened(&decoded), "{what}: samples");
    }
}

// ---------------------------------------------------------------------------
// The stream-copy paths (AC e)
// ---------------------------------------------------------------------------

#[test]
fn wav_from_pcm_copy_rewraps_without_decoding() {
    const WHAT: &str = "generated/valid/wav_pcm_s16le_44k1_st.wav";
    let mut verify = open_fixture(WHAT);
    let demuxed = demux_all(&mut verify);

    let (sink, twin) = sink_pair();
    let graph = TranscodeGraph {
        source: SourceSpec {
            demuxer: Box::new(open_fixture(WHAT)),
            selection: TrackSelection::DefaultAudio,
            extra_data: None,
        },
        // No decode stage: the copy path consumes demuxed packets.
        decode: None,
        filters: Vec::new(),
        sink: SinkSpec::WavCopyPcm {
            output: sink,
            facts: Some(PcmFacts {
                sample: PcmSample::S16Le,
                channels: 2,
                rate: 44_100,
            }),
        },
    };
    let job = submit(graph, JobLimits::default()).must();
    let stats = match job.wait() {
        JobOutcome::Completed(stats) => stats,
        other => unreachable!("copy completes, got {other:?}"),
    };
    assert_eq!(stats.frames_decoded, 0, "no decoder ran");
    assert_eq!(stats.sample_frames_written, 8_820);
    assert_eq!(stats.bytes_written as usize, demuxed.payload.len());

    // Verbatim claim: the payload bytes are the source's, untouched.
    let output = twin.lock().unwrap_or_else(|p| p.into_inner()).clone();
    assert_eq!(output.len(), 44 + demuxed.payload.len());
    assert_eq!(&output[44..], demuxed.payload.as_slice());
    // And the rewrapped file is a WAV our own demuxer accepts, with
    // the same decoded samples as the source.
    let mut redemux = WavDemuxer::new(Box::new(MemStream::over(output))).must();
    let round = demux_all(&mut redemux);
    assert_eq!(round.codec, "pcm_s16le");
    assert_eq!(round.payload, demuxed.payload);
    let from_source = decode_all(&demuxed.fmt, "pcm_s16le", &demuxed.payload);
    let from_copy = decode_all(&round.fmt, "pcm_s16le", &round.payload);
    assert_eq!(flattened(&from_copy), flattened(&from_source));
}

#[test]
fn raw_es_copy_dumps_the_payload_verbatim() {
    const WHAT: &str = "generated/valid/wav_pcm_s24le_48k_st.wav";
    let mut verify = open_fixture(WHAT);
    let demuxed = demux_all(&mut verify);

    let (sink, twin) = sink_pair();
    let graph = TranscodeGraph {
        source: SourceSpec {
            demuxer: Box::new(open_fixture(WHAT)),
            selection: TrackSelection::DefaultAudio,
            extra_data: None,
        },
        decode: None,
        filters: Vec::new(),
        sink: SinkSpec::RawEsCopy { output: sink },
    };
    let job = submit(graph, JobLimits::default()).must();
    let stats = match job.wait() {
        JobOutcome::Completed(stats) => stats,
        other => unreachable!("es dump completes, got {other:?}"),
    };
    // The ES sink makes no sample claim; bytes only, no container.
    assert_eq!(stats.sample_frames_written, 0);
    assert_eq!(stats.bytes_written as usize, demuxed.payload.len());
    let output = twin.lock().unwrap_or_else(|p| p.into_inner()).clone();
    assert_eq!(output, demuxed.payload, "the dump is the payload, no header");
}

#[test]
fn copy_facts_disagreeing_with_the_track_fail_typed() {
    const WHAT: &str = "generated/valid/wav_pcm_s16le_44k1_st.wav";
    let (sink, _twin) = sink_pair();
    let graph = TranscodeGraph {
        source: SourceSpec {
            demuxer: Box::new(open_fixture(WHAT)),
            selection: TrackSelection::DefaultAudio,
            extra_data: None,
        },
        decode: None,
        filters: Vec::new(),
        sink: SinkSpec::WavCopyPcm {
            output: sink,
            // The track says pcm_s16le; the facts claim s24.
            facts: Some(PcmFacts {
                sample: PcmSample::S24Le,
                channels: 2,
                rate: 44_100,
            }),
        },
    };
    let job = submit(graph, JobLimits::default()).must();
    let err = match job.wait() {
        JobOutcome::Failed(err) => err,
        other => unreachable!("facts mismatch fails, got {other:?}"),
    };
    assert_eq!(err.context.code, "transcode.copy.facts-mismatch");
}

#[test]
fn submit_refuses_a_wav_pcm_graph_without_decode() {
    let (sink, _twin) = sink_pair();
    let graph = TranscodeGraph {
        source: SourceSpec {
            demuxer: Box::new(open_fixture(
                "generated/valid/wav_pcm_s16le_44k1_st.wav",
            )),
            selection: TrackSelection::DefaultAudio,
            extra_data: None,
        },
        decode: None,
        filters: Vec::new(),
        sink: SinkSpec::WavPcm {
            output: sink,
            target: WavPcmTarget::Source,
        },
    };
    let err = assert_err(submit(graph, JobLimits::default()), "encode needs decode");
    assert_eq!(err.context.code, "transcode.graph.decode-required");
}
