//! The parent-fixture acceptance suite (card 2.B4 AC): the committed
//! AVI and MP4 parents each carry the same 60 deterministic JPEG
//! frames; this suite **extracts** them (test-side walkers whose
//! correctness is pinned by the expectation's per-frame sha256),
//! decodes every frame through the real codec-api [`Decoder`] path,
//! and hash-matches each decoded output against the committed
//! reference digests (`tests/data/mjpeg_decode_reference.txt`,
//! provenance + ffmpeg-oracle tolerance recorded in its header and in
//! worklog 2.B4).
//!
//! It also closes the `[first_frame]` expectation loop (doc 10 §5.4):
//! the digest the 2.A3 writer recorded from "Lumen's first compliant
//! decode" is recomputed here from this decoder's output.

mod common;

use codec_api::prelude::*;
use codecs::video::mjpeg::MjpegDecoder;
use common::{
    extract_avi_mjpeg_frames, extract_mp4_mjpeg_frames, fixture_bytes, luma_block_mean_digest,
    parse_parent_expectation, reference_digests, sha256_hex,
};
use media_types::frame::FrameStorage;
use media_types::time::{Stamp, TimeBase, Timestamp};
use media_types::{EngineError, ErrorKind, Frame, GenerationId, Packet};

const AVI: &str = "generated/valid/avi_mjpeg_pcm.avi";
const AVI_EXPECTATION: &str = "expectations/valid/avi_mjpeg_pcm.avi.toml";
const MP4: &str = "generated/valid/mp4_mjpeg_pcm.mp4";
const MP4_EXPECTATION: &str = "expectations/valid/mp4_mjpeg_pcm.mp4.toml";

const FRAME_COUNT: usize = 60;
const FRAME_WIDTH: usize = 320;
const FRAME_HEIGHT: usize = 240;
/// I420 plane sizes for the fixture geometry (4:2:0).
const LUMA_LEN: usize = FRAME_WIDTH * FRAME_HEIGHT;
const CHROMA_LEN: usize = (FRAME_WIDTH / 2) * (FRAME_HEIGHT / 2);

/// A capturing sink (the decode path's only emission target).
struct CaptureSink {
    frames: Vec<Frame>,
}

impl FrameSink for CaptureSink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        self.frames.push(frame);
        Ok(())
    }
}

/// One configured decoder + its generation, ready for packets.
fn configured() -> (MjpegDecoder, GenerationId) {
    let descriptor = match media_types::ids::CodecDescriptor::new("mjpeg") {
        Ok(d) => d,
        Err(e) => unreachable!("descriptor: {e}"),
    };
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => unreachable!("generation: {e}"),
    };
    let mut decoder = MjpegDecoder::new();
    match decoder.configure(&DecoderConfig::new(
        descriptor,
        generation,
        ThreadBudget::default(),
        None,
        None,
    )) {
        Ok(()) => (decoder, generation),
        Err(e) => unreachable!("configure: {e}"),
    }
}

/// Frame `index`'s presentation stamp: 30 fps on a microsecond base.
fn frame_stamp(index: usize) -> Stamp {
    let base = match TimeBase::new(1, 1_000_000) {
        Ok(b) => b,
        Err(e) => unreachable!("time base: {e}"),
    };
    Stamp::Present(Timestamp::new((index as i64) * 1_000_000 / 30, base))
}

/// Wrap one extracted JPEG as a decode packet.
fn packet_for(jpeg: &[u8], index: usize, generation: GenerationId) -> Packet {
    Packet {
        track: media_types::TrackId::new(0),
        pts: frame_stamp(index),
        dts: Stamp::Missing,
        duration: None,
        data: media_types::packet::SharedBytes::from_vec(jpeg.to_vec()),
        keyframe: true,
        discontinuity: false,
        generation,
    }
}

/// Decode all frames through the contract path; returns the emitted
/// video frames in order (one per packet, asserted by the caller).
fn decode_all(jpegs: &[Vec<u8>]) -> (Vec<Frame>, Vec<DecodeOutcome>) {
    let (mut decoder, generation) = configured();
    let mut sink = CaptureSink { frames: Vec::new() };
    let mut outcomes = Vec::with_capacity(jpegs.len());
    for (index, jpeg) in jpegs.iter().enumerate() {
        let outcome = match decoder.decode(packet_for(jpeg, index, generation), &mut sink) {
            Ok(outcome) => outcome,
            Err(e) => unreachable!("frame {index} must decode: {e}"),
        };
        outcomes.push(outcome);
    }
    match decoder.flush(&mut sink) {
        Ok(DecodeOutcome::Drained) => {}
        other => unreachable!("flush must drain: {other:?}"),
    }
    (sink.frames, outcomes)
}

/// The pixel bytes of a decoded frame (fails the test on anything
/// but CPU-stored video).
fn frame_bytes<'a>(frame: &'a Frame, context: &str) -> &'a [u8] {
    let Frame::Video(video) = frame else {
        unreachable!("{context}: not a video frame: {frame:?}");
    };
    match &video.data {
        FrameStorage::Cpu(bytes) => bytes.as_slice(),
        FrameStorage::OpaqueHandle(_) => unreachable!("{context}: unexpected opaque handle"),
    }
}

#[test]
fn sha256_reader_pinned_against_committed_file_digests() {
    // Pin the test-side SHA-256 against the corpus's committed
    // whole-file hashes before any digest assertions rely on it.
    for (fixture, expectation_path) in [(AVI, AVI_EXPECTATION), (MP4, MP4_EXPECTATION)] {
        let bytes = match fixture_bytes(fixture) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        let expect = match parse_parent_expectation(expectation_path) {
            Ok(e) => e,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(sha256_hex(&bytes), expect.sha256, "{fixture}");
        assert_eq!(bytes.len() as u64, expect.size_bytes, "{fixture}");
    }
}

#[test]
fn avi_extraction_matches_writer_declared_frame_digests() {
    let bytes = match fixture_bytes(AVI) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let expect = match parse_parent_expectation(AVI_EXPECTATION) {
        Ok(e) => e,
        Err(e) => unreachable!("{e}"),
    };
    let frames = match extract_avi_mjpeg_frames(&bytes) {
        Ok(f) => f,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(frames.len(), FRAME_COUNT);
    assert_eq!(expect.video_sample_count, FRAME_COUNT as u64);
    assert_eq!(expect.video_sample_sha256.len(), FRAME_COUNT);
    for (index, frame) in frames.iter().enumerate() {
        assert_eq!(
            sha256_hex(frame),
            expect.video_sample_sha256[index],
            "AVI frame {index}"
        );
    }
}

#[test]
fn mp4_extraction_matches_writer_declared_frame_digests() {
    let bytes = match fixture_bytes(MP4) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let expect = match parse_parent_expectation(MP4_EXPECTATION) {
        Ok(e) => e,
        Err(e) => unreachable!("{e}"),
    };
    let frames = match extract_mp4_mjpeg_frames(&bytes) {
        Ok(f) => f,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(frames.len(), FRAME_COUNT);
    for (index, frame) in frames.iter().enumerate() {
        assert_eq!(
            sha256_hex(frame),
            expect.video_sample_sha256[index],
            "MP4 frame {index}"
        );
    }
}

#[test]
fn both_containers_carry_the_identical_frame_essence() {
    let avi = match fixture_bytes(AVI).map(|b| extract_avi_mjpeg_frames(&b)) {
        Ok(Ok(f)) => f,
        Ok(Err(e)) => unreachable!("{e}"),
        Err(e) => unreachable!("{e}"),
    };
    let mp4 = match fixture_bytes(MP4).map(|b| extract_mp4_mjpeg_frames(&b)) {
        Ok(Ok(f)) => f,
        Ok(Err(e)) => unreachable!("{e}"),
        Err(e) => unreachable!("{e}"),
    };
    // Byte-identical compressed frames — the reason one reference
    // digest list serves both containers (recorded honestly: the
    // per-frame sha256 match above pins each set independently).
    assert_eq!(avi, mp4);
}

#[test]
fn decoded_avi_outputs_match_reference_digests() {
    let bytes = match fixture_bytes(AVI) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let jpegs = match extract_avi_mjpeg_frames(&bytes) {
        Ok(f) => f,
        Err(e) => unreachable!("{e}"),
    };
    assert_reference_match(&jpegs, "AVI");
}

#[test]
fn decoded_mp4_outputs_match_reference_digests() {
    let bytes = match fixture_bytes(MP4) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let jpegs = match extract_mp4_mjpeg_frames(&bytes) {
        Ok(f) => f,
        Err(e) => unreachable!("{e}"),
    };
    assert_reference_match(&jpegs, "MP4");
}

/// The shared AC body: every frame decodes cleanly (Emitted, one
/// frame per packet, nothing concealed) through the contract path,
/// in the codec-native I420 format at the declared geometry, with
/// the pts passed through and the JFIF color tag attached — and the
/// pixel bytes hash-match the committed reference digest.
fn assert_reference_match(jpegs: &[Vec<u8>], container: &str) {
    let reference = match reference_digests() {
        Ok(r) => r,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(reference.len(), FRAME_COUNT, "reference digest count");
    let (frames, outcomes) = decode_all(jpegs);
    assert_eq!(frames.len(), FRAME_COUNT, "{container}: emitted frames");
    assert_eq!(outcomes.len(), FRAME_COUNT);
    for (index, outcome) in outcomes.iter().enumerate() {
        assert_eq!(
            outcome,
            &DecodeOutcome::Emitted { frames: 1 },
            "{container} frame {index}: clean decode must report Emitted {{1}}"
        );
    }
    let base = match TimeBase::new(1, 1_000_000) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    for (index, frame) in frames.iter().enumerate() {
        let Frame::Video(video) = frame else {
            unreachable!("{container} frame {index}: not video");
        };
        assert_eq!(video.format, media_types::PixelFormat::I420, "{container} frame {index}");
        assert_eq!(video.size, (320, 240), "{container} frame {index}");
        assert_eq!(
            video.pts,
            Stamp::Present(Timestamp::new(index as i64 * 1_000_000 / 30, base)),
            "{container} frame {index}: pts passthrough"
        );
        assert_eq!(video.color.matrix, media_types::frame::ColorMatrix::Bt601);
        assert_eq!(video.color.range, media_types::frame::ColorRange::Full);
        assert!(!video.interlaced);
        let bytes = frame_bytes(frame, &format!("{container} frame {index}"));
        assert_eq!(bytes.len(), LUMA_LEN + 2 * CHROMA_LEN, "{container} frame {index}");
        assert_eq!(sha256_hex(bytes), reference[index], "{container} frame {index}");
    }
}

#[test]
fn first_frame_descriptor_matches_the_expectation() {
    // Doc 10 §5.4's [first_frame]: the 8x8-block mean descriptor of
    // the first decoded frame's luma, recorded by the 2.A3 writer
    // from Lumen's first compliant decode (filled in by card 2.B4).
    // Recompute it here and close the loop.
    let bytes = match fixture_bytes(AVI) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let jpegs = match extract_avi_mjpeg_frames(&bytes) {
        Ok(f) => f,
        Err(e) => unreachable!("{e}"),
    };
    let expect = match parse_parent_expectation(AVI_EXPECTATION) {
        Ok(e) => e,
        Err(e) => unreachable!("{e}"),
    };
    let Some(hash_y) = expect.first_frame_hash_y else {
        unreachable!("the 2.B4 card fills [first_frame]; it is absent");
    };
    assert_eq!(expect.first_frame_stamp_us, Some(0));
    let (mut decoder, generation) = configured();
    let mut sink = CaptureSink { frames: Vec::new() };
    let outcome = match decoder.decode(packet_for(&jpegs[0], 0, generation), &mut sink) {
        Ok(o) => o,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(outcome, DecodeOutcome::Emitted { frames: 1 });
    assert_eq!(sink.frames.len(), 1);
    let bytes = frame_bytes(&sink.frames[0], "first frame");
    let digest = match luma_block_mean_digest(&bytes[..LUMA_LEN], FRAME_WIDTH, FRAME_HEIGHT) {
        Ok(d) => d,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(digest, hash_y, "first_frame hash_y (doc 10 §5.4 descriptor)");
    // The same descriptor for the MP4 parent (identical essence, and
    // its own expectation file must agree).
    let mp4_expect = match parse_parent_expectation(MP4_EXPECTATION) {
        Ok(e) => e,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(mp4_expect.first_frame_hash_y.as_deref(), Some(hash_y.as_str()));
}

#[test]
fn entropy_truncation_conceals_and_reports_recovery() {
    // Cut the first frame's entropy data well into the scan: the
    // header structure is intact, so the frame must be *emitted* with
    // the undecodable tail concealed mid-gray, and the outcome must
    // say Recovered { ConcealedFrame } — bounded, observable, never
    // silent (doc 03 §4.5(d)).
    let bytes = match fixture_bytes(AVI) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let jpegs = match extract_avi_mjpeg_frames(&bytes) {
        Ok(f) => f,
        Err(e) => unreachable!("{e}"),
    };
    let mut damaged = jpegs[0].clone();
    // Keep SOI..SOS + ~35% of the entropy bytes (frame 0 is 1193
    // bytes; the scan starts after the SOS segment around byte 620 —
    // the exact cut only needs to be inside the entropy region).
    let cut = damaged.len() * 3 / 5;
    damaged.truncate(cut);
    let (mut decoder, generation) = configured();
    let mut sink = CaptureSink { frames: Vec::new() };
    let outcome = match decoder.decode(packet_for(&damaged, 0, generation), &mut sink) {
        Ok(o) => o,
        Err(e) => unreachable!("concealable damage must not error: {e}"),
    };
    assert_eq!(
        outcome,
        DecodeOutcome::Recovered { skipped: 0, reason: codec_api::RecoverReason::ConcealedFrame },
        "entropy truncation reports concealed recovery"
    );
    assert_eq!(sink.frames.len(), 1, "the damaged frame is still emitted");
    let bytes = frame_bytes(&sink.frames[0], "concealed frame");
    assert_eq!(bytes.len(), LUMA_LEN + 2 * CHROMA_LEN);
    // The concealed tail is mid-gray: with the cut this early the
    // bottom of the image was never decoded.
    let tail = &bytes[LUMA_LEN - FRAME_WIDTH * 8..LUMA_LEN];
    assert!(
        tail.iter().all(|&b| b == 128),
        "the final 8 luma rows must be the mid-gray concealment fill"
    );
    // …while the top of the frame decoded (the fixture's first row is
    // bright, not gray — the content generator's background).
    let head = &bytes[..FRAME_WIDTH];
    assert!(
        head.iter().any(|&b| b != 128),
        "the undamaged prefix must carry real content"
    );
    // And the decoder stayed usable (contract: recover, keep going).
    let mut drain = CaptureSink { frames: Vec::new() };
    assert_eq!(
        decoder.flush(&mut drain).ok(),
        Some(DecodeOutcome::Drained),
        "decoder usable after concealment"
    );
}

#[test]
fn header_damage_is_a_typed_error_with_nothing_emitted() {
    // Flip the SOI: no honest frame can be built from a broken
    // marker walk — nothing is emitted and the error is typed at the
    // decode stage (fabricating a frame would not be concealment).
    let bytes = match fixture_bytes(AVI) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let jpegs = match extract_avi_mjpeg_frames(&bytes) {
        Ok(f) => f,
        Err(e) => unreachable!("{e}"),
    };
    let mut damaged = jpegs[0].clone();
    damaged[0] = 0x00;
    let (mut decoder, generation) = configured();
    let mut sink = CaptureSink { frames: Vec::new() };
    let err = match decoder.decode(packet_for(&damaged, 0, generation), &mut sink) {
        Ok(outcome) => unreachable!("header damage must not succeed: {outcome:?}"),
        Err(e) => e,
    };
    assert_eq!(err.stage, media_types::PipelineStage::Decode);
    assert!(
        matches!(err.kind, ErrorKind::Corrupt { .. }),
        "header damage is a typed Corrupt error: {err}"
    );
    assert!(sink.frames.is_empty(), "nothing is emitted for header damage");
}
