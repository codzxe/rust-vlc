//! The parent-fixture audio acceptance suite (card 2.B5 AC): the
//! committed AVI and MP4 parents each carry the same deterministic
//! PCM stream (2.A3's NCO-generated s16le 44100 stereo, one `01wb`
//! / `sowt` sample per frame tick); this suite **extracts** the audio
//! chunks (test-side walkers whose correctness is pinned by the
//! expectation's `audio_stream_sha256`), decodes every chunk through
//! the real codec-api [`Decoder`] path, and asserts the decoded
//! output is **bit-exact** — the concatenated decoded samples equal
//! the container's stream, and (on the little-endian hosts this
//! repo's CI targets) hash-match the writer-recorded sha256
//! directly.
//!
//! It also closes the `[audio_probe]` expectation loop (doc 10
//! §5.4): the MD5 the 2.A3 writer derived from its own samples is
//! recomputed here from this decoder's output — writer and decoder
//! pin each other.

mod common;

use codec_api::prelude::*;
use codecs::audio::pcm::PcmDecoder;
use common::{
    audio_probe_md5, extract_avi_audio_strf, extract_avi_pcm_chunks, extract_mp4_audio_facts,
    extract_mp4_pcm_chunks, fixture_bytes, md5_hex, parse_parent_expectation, sha256_hex,
};
use media_types::frame::{AudioFrame, AudioLayout, SampleFormat};
use media_types::ids::CodecDescriptor;
use media_types::packet::SharedBytes;
use media_types::time::{Duration, Nanoseconds, Stamp, TimeBase, Timestamp};
use media_types::{EngineError, Frame, GenerationId, Packet, TrackId};

const AVI: &str = "generated/valid/avi_mjpeg_pcm.avi";
const AVI_EXPECTATION: &str = "expectations/valid/avi_mjpeg_pcm.avi.toml";
const MP4: &str = "generated/valid/mp4_mjpeg_pcm.mp4";
const MP4_EXPECTATION: &str = "expectations/valid/mp4_mjpeg_pcm.mp4.toml";

/// One audio chunk per frame tick (60).
const CHUNK_COUNT: usize = 60;
/// Sample-instants per chunk (`44100 / 30`).
const TICK_INSTANTS: usize = 1470;
/// Bytes per chunk (stereo s16).
const TICK_BYTES: usize = TICK_INSTANTS * 4;
const RATE: u32 = 44_100;
const CHANNELS: u32 = 2;

/// A capturing sink (the decode path's only emission target).
struct CaptureSink {
    frames: Vec<AudioFrame>,
}

impl FrameSink for CaptureSink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        let Frame::Audio(audio) = frame else {
            unreachable!("the PCM decoder emits audio frames only: {frame:?}");
        };
        self.frames.push(audio);
        Ok(())
    }
}

/// Audio packet `index`: pts at the strh-declared sample tick (scale
/// 1, rate 44100 — the 2.B2 demuxer's stamping arithmetic for a
/// `dwSampleSize`-carrying audio stream), the whole chunk as data.
fn audio_packet(chunk: &[u8], index: usize, generation: GenerationId) -> Packet {
    let base = match TimeBase::new(1, RATE) {
        Ok(b) => b,
        Err(e) => unreachable!("time base: {e}"),
    };
    Packet {
        track: TrackId::new(1),
        pts: Stamp::Present(Timestamp::new(
            (index as i64) * (TICK_INSTANTS as i64),
            base,
        )),
        dts: Stamp::Missing,
        duration: None,
        data: SharedBytes::from_vec(chunk.to_vec()),
        keyframe: true,
        discontinuity: false,
        generation,
    }
}

/// One configured `pcm_s16le` decoder for the given stream-facts blob.
fn configured(extra_data: Vec<u8>) -> (PcmDecoder, GenerationId) {
    let descriptor = match CodecDescriptor::new("pcm_s16le") {
        Ok(d) => d,
        Err(e) => unreachable!("descriptor: {e}"),
    };
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => unreachable!("generation: {e}"),
    };
    let mut decoder = PcmDecoder::new();
    match decoder.configure(&DecoderConfig::new(
        descriptor,
        generation,
        ThreadBudget::default(),
        None,
        Some(SharedBytes::from_vec(extra_data)),
    )) {
        Ok(()) => (decoder, generation),
        Err(e) => unreachable!("configure: {e}"),
    }
}

/// Decode every chunk through the contract path; returns the emitted
/// audio frames and per-call outcomes in order, after a final flush.
fn decode_stream(
    decoder: &mut PcmDecoder,
    chunks: &[Vec<u8>],
    generation: GenerationId,
) -> (Vec<AudioFrame>, Vec<DecodeOutcome>) {
    let mut sink = CaptureSink { frames: Vec::new() };
    let mut outcomes = Vec::with_capacity(chunks.len());
    for (index, chunk) in chunks.iter().enumerate() {
        let outcome = match decoder.decode(audio_packet(chunk, index, generation), &mut sink) {
            Ok(outcome) => outcome,
            Err(e) => unreachable!("chunk {index} must decode: {e}"),
        };
        outcomes.push(outcome);
    }
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Drained) => {}
        other => unreachable!("flush must drain: {other:?}"),
    }
    (sink.frames, outcomes)
}

/// Concatenate decoded sample buffers.
fn concat(frames: &[AudioFrame]) -> Vec<u8> {
    let mut out = Vec::with_capacity(frames.iter().map(|f| f.samples.len()).sum());
    for frame in frames {
        out.extend_from_slice(frame.samples.as_slice());
    }
    out
}

#[test]
fn md5_reader_pinned_against_rfc1321_vectors() {
    // Pin the test-side MD5 against RFC 1321 §A.5 before the
    // [audio_probe] assertions rely on it (the sha256 sibling of this
    // check lives in the 2.B4 suite).
    assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
    assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    assert_eq!(
        md5_hex(b"message digest"),
        "f96b697d7cb7938d525a2f31aaf161d0"
    );
    assert_eq!(
        md5_hex(b"abcdefghijklmnopqrstuvwxyz"),
        "c3fcd3d76192e4007dfb496cca67e13b"
    );
}

#[test]
fn avi_audio_extraction_matches_writer_declared_stream_digest() {
    let bytes = match fixture_bytes(AVI) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let expect = match parse_parent_expectation(AVI_EXPECTATION) {
        Ok(e) => e,
        Err(e) => unreachable!("{e}"),
    };
    let chunks = match extract_avi_pcm_chunks(&bytes) {
        Ok(c) => c,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(chunks.len(), CHUNK_COUNT);
    let stream = concat_chunk_bodies(&chunks);
    assert_eq!(stream.len() as u64, expect.audio_stream_bytes);
    assert_eq!(
        sha256_hex(&stream),
        expect.audio_stream_sha256,
        "the movi 01wb walk must reproduce the writer's PCM stream"
    );
}

#[test]
fn mp4_audio_extraction_matches_writer_declared_stream_digest() {
    let bytes = match fixture_bytes(MP4) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let expect = match parse_parent_expectation(MP4_EXPECTATION) {
        Ok(e) => e,
        Err(e) => unreachable!("{e}"),
    };
    let chunks = match extract_mp4_pcm_chunks(&bytes) {
        Ok(c) => c,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(chunks.len(), CHUNK_COUNT);
    let stream = concat_chunk_bodies(&chunks);
    assert_eq!(stream.len() as u64, expect.audio_stream_bytes);
    assert_eq!(
        sha256_hex(&stream),
        expect.audio_stream_sha256,
        "the sowt stbl walk must reproduce the writer's PCM stream"
    );
}

/// Flatten extracted chunks (the helper the two extraction checks
/// share).
fn concat_chunk_bodies(chunks: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::with_capacity(chunks.iter().map(Vec::len).sum());
    for chunk in chunks {
        out.extend_from_slice(chunk);
    }
    out
}

#[test]
fn both_containers_carry_the_identical_audio_essence() {
    let avi = match fixture_bytes(AVI).map(|b| extract_avi_pcm_chunks(&b)) {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => unreachable!("{e}"),
        Err(e) => unreachable!("{e}"),
    };
    let mp4 = match fixture_bytes(MP4).map(|b| extract_mp4_pcm_chunks(&b)) {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => unreachable!("{e}"),
        Err(e) => unreachable!("{e}"),
    };
    // Byte-identical audio streams — the reason one digest serves
    // both containers (recorded honestly: each extraction is pinned
    // independently by its own sha256 match above).
    assert_eq!(avi, mp4);
}

#[test]
fn decoded_avi_audio_is_bit_exact_via_the_contract_path() {
    let bytes = match fixture_bytes(AVI) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let expect = match parse_parent_expectation(AVI_EXPECTATION) {
        Ok(e) => e,
        Err(e) => unreachable!("{e}"),
    };
    let chunks = match extract_avi_pcm_chunks(&bytes) {
        Ok(c) => c,
        Err(e) => unreachable!("{e}"),
    };
    // Configure from the container's own strf bytes (the real
    // WAVEFORMATEX the AVI writer emitted — no synthesized stand-in).
    let strf = match extract_avi_audio_strf(&bytes) {
        Ok(s) => s,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(strf.len(), 16, "the writer's PCMWAVEFORMAT");
    let (mut decoder, generation) = configured(strf);
    let (frames, outcomes) = decode_stream(&mut decoder, &chunks, generation);
    assert_bit_exact(frames, outcomes, &chunks, &expect, "AVI");
}

#[test]
fn decoded_mp4_audio_is_bit_exact_via_the_contract_path() {
    let bytes = match fixture_bytes(MP4) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let expect = match parse_parent_expectation(MP4_EXPECTATION) {
        Ok(e) => e,
        Err(e) => unreachable!("{e}"),
    };
    let chunks = match extract_mp4_pcm_chunks(&bytes) {
        Ok(c) => c,
        Err(e) => unreachable!("{e}"),
    };
    // Configure MP4-style: a WAVEFORMATEX synthesized from the
    // AudioSampleEntry's own facts (channelcount / samplerate /
    // samplesize) — what an MP4 demuxer adapter does at the
    // decoder-config seam.
    let facts = match extract_mp4_audio_facts(&bytes) {
        Ok(f) => f,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(facts.channels, CHANNELS as u16);
    assert_eq!(facts.rate, RATE);
    assert_eq!(facts.bits, 16);
    let (mut decoder, generation) = configured(synthesized_waveformat(&bytes));
    let (frames, outcomes) = decode_stream(&mut decoder, &chunks, generation);
    assert_bit_exact(frames, outcomes, &chunks, &expect, "MP4");
}

/// The shared AC body (card 2.B5: "bit-exact output vs fixtures").
///
/// Every chunk decodes cleanly (`Emitted { 1 }`, nothing concealed)
/// through the contract path, in the codec-native output format
/// (native-endian S16, the source's interleaved layout, the strf's
/// channels and rate), with the pts passed through and the duration
/// derived from the decoded sample count — and the concatenated
/// decoded samples are **byte-identical** to the container's stream:
/// directly equal after the host's own byte-order normalization, and
/// on little-endian hosts (the recorded corpus is s16le) equal to
/// the writer's stream digest verbatim.
fn assert_bit_exact(
    frames: Vec<AudioFrame>,
    outcomes: Vec<DecodeOutcome>,
    chunks: &[Vec<u8>],
    expect: &common::ParentExpectation,
    container: &str,
) {
    assert_eq!(frames.len(), CHUNK_COUNT, "{container}: emitted frames");
    assert_eq!(outcomes.len(), CHUNK_COUNT);
    for (index, outcome) in outcomes.iter().enumerate() {
        assert_eq!(
            outcome,
            &DecodeOutcome::Emitted { frames: 1 },
            "{container} chunk {index}: a clean chunk must report Emitted {{1}}"
        );
    }
    let base = match TimeBase::new(1, RATE) {
        Ok(b) => b,
        Err(e) => unreachable!("time base: {e}"),
    };
    for (index, frame) in frames.iter().enumerate() {
        assert_eq!(
            frame.format,
            media_types::frame::AudioFormat {
                sample: SampleFormat::S16,
                channels: CHANNELS,
                layout: AudioLayout::Interleaved,
                rate: RATE,
            },
            "{container} chunk {index}"
        );
        assert_eq!(
            frame.pts,
            Stamp::Present(Timestamp::new(index as i64 * TICK_INSTANTS as i64, base)),
            "{container} chunk {index}: pts passthrough"
        );
        assert_eq!(
            frame.duration,
            Some(Duration::Known(Nanoseconds(33_333_333))),
            "{container} chunk {index}: 1470 instants at 44100 Hz, floored"
        );
        assert_eq!(frame.samples.len(), TICK_BYTES, "{container} chunk {index}");
    }
    let decoded = concat(&frames);
    assert_eq!(decoded.len() as u64, expect.audio_stream_bytes);

    // The semantic oracle: the decoded stream equals the extracted
    // stream after the same native-order normalization the decoder
    // applies (identity on little-endian hosts, a per-sample byte
    // swap on big-endian ones — media-types' native-endian S16).
    let mut native = concat_chunk_bodies(chunks);
    if cfg!(target_endian = "big") {
        for pair in native.as_chunks_mut::<2>().0 {
            pair.reverse();
        }
    }
    assert_eq!(
        decoded, native,
        "{container}: decoded == extracted (native)"
    );

    // The card's literal hash-match: on little-endian hosts the
    // decode is the identity map, so the decoded stream must equal
    // the writer-recorded s16le digest directly.
    if cfg!(target_endian = "little") {
        assert_eq!(
            sha256_hex(&decoded),
            expect.audio_stream_sha256,
            "{container}: bit-exact decode must hash-match the writer's PCM stream"
        );
    }
}

#[test]
fn audio_probe_matches_the_expectation() {
    // Doc 10 §5.4's [audio_probe]: the pinned f32 conversion of the
    // first 4800 sample-instants, recorded by the 2.A3 writer from
    // its own generated samples (filled in by this card). Recompute
    // it from the decoder's output and close the loop — for both
    // parents, whose expectations must agree.
    for (fixture, expectation_path, configure) in [
        (AVI, AVI_EXPECTATION, ConfigureFrom::RealStrf),
        (MP4, MP4_EXPECTATION, ConfigureFrom::SowtFacts),
    ] {
        let bytes = match fixture_bytes(fixture) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        let expect = match parse_parent_expectation(expectation_path) {
            Ok(e) => e,
            Err(e) => unreachable!("{e}"),
        };
        let Some(probe) = expect.audio_probe_md5 else {
            unreachable!("the 2.B5 card fills [audio_probe]; it is absent");
        };
        let (extra, chunks) = match configure {
            ConfigureFrom::RealStrf => (
                match extract_avi_audio_strf(&bytes) {
                    Ok(s) => s,
                    Err(e) => unreachable!("{e}"),
                },
                match extract_avi_pcm_chunks(&bytes) {
                    Ok(c) => c,
                    Err(e) => unreachable!("{e}"),
                },
            ),
            ConfigureFrom::SowtFacts => {
                let chunks = match extract_mp4_pcm_chunks(&bytes) {
                    Ok(c) => c,
                    Err(e) => unreachable!("{e}"),
                };
                (synthesized_waveformat(&bytes), chunks)
            }
        };
        let (mut decoder, generation) = configured(extra);
        // 4800 instants spans four chunks (3 × 1470 = 4410 < 4800);
        // decode five and take the prefix.
        let mut sink = CaptureSink { frames: Vec::new() };
        for (index, chunk) in chunks.iter().enumerate().take(5) {
            match decoder.decode(audio_packet(chunk, index, generation), &mut sink) {
                Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
                other => unreachable!("{fixture} chunk {index}: {other:?}"),
            }
        }
        let stream = concat(&sink.frames);
        let digest = match audio_probe_md5(&stream, CHANNELS) {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(
            digest, probe,
            "{fixture}: [audio_probe] first_samples_md5 (doc 10 §5.4)"
        );
    }
    // Both class-A expectations carry the same value (identical
    // audio essence).
    let avi = match parse_parent_expectation(AVI_EXPECTATION) {
        Ok(e) => e,
        Err(e) => unreachable!("{e}"),
    };
    let mp4 = match parse_parent_expectation(MP4_EXPECTATION) {
        Ok(e) => e,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(avi.audio_probe_md5, mp4.audio_probe_md5);
}

/// How the MP4 probe test builds its stream-facts blob.
enum ConfigureFrom {
    /// AVI: the container's own `strf` bytes.
    RealStrf,
    /// MP4: a WAVEFORMATEX synthesized from the `sowt` entry facts.
    SowtFacts,
}

/// Synthesize the 16-byte WAVEFORMATEX from the MP4 parent's sowt
/// entry (see the MP4 bit-exact test).
fn synthesized_waveformat(bytes: &[u8]) -> Vec<u8> {
    let facts = match extract_mp4_audio_facts(bytes) {
        Ok(f) => f,
        Err(e) => unreachable!("{e}"),
    };
    let mut waveformat = Vec::with_capacity(16);
    waveformat.extend_from_slice(&1u16.to_le_bytes()); // WAVE_FORMAT_PCM
    waveformat.extend_from_slice(&facts.channels.to_le_bytes());
    waveformat.extend_from_slice(&facts.rate.to_le_bytes());
    let block_align = u32::from(facts.channels) * 2;
    let avg_bytes = match facts.rate.checked_mul(block_align) {
        Some(bytes) => bytes,
        None => unreachable!("avg-bytes-per-sec overflow at this corpus scale"),
    };
    waveformat.extend_from_slice(&avg_bytes.to_le_bytes());
    waveformat.extend_from_slice(&(block_align as u16).to_le_bytes());
    waveformat.extend_from_slice(&facts.bits.to_le_bytes());
    debug_assert_eq!(waveformat.len(), 16);
    waveformat
}
