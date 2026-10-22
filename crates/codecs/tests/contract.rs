//! The codec-api contract suite run against `MjpegDecoder` (doc 01
//! §2.4(e): "the reusable test suite every codec in `codecs` … runs
//! against its implementations"). Real fixture frames are the
//! emitting packets; damaged variants drive the recovery paths.

mod common;

use codec_api::contract;
use codec_api::prelude::*;
use codecs::video::mjpeg::MjpegDecoder;
use common::{extract_avi_mjpeg_frames, fixture_bytes};
use media_types::ids::CodecDescriptor;
use media_types::packet::SharedBytes;
use media_types::{ErrorKind, GenerationId, Packet, Stamp, TrackId};

const AVI: &str = "generated/valid/avi_mjpeg_pcm.avi";

/// The 60 committed frames (extraction is sha256-pinned by the
/// fixtures suite; here they are just emitting packet bodies).
fn fixture_frames() -> Vec<Vec<u8>> {
    let bytes = match fixture_bytes(AVI) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    match extract_avi_mjpeg_frames(&bytes) {
        Ok(frames) => frames,
        Err(e) => unreachable!("{e}"),
    }
}

fn configured() -> (MjpegDecoder, GenerationId) {
    let descriptor = match CodecDescriptor::new("mjpeg") {
        Ok(d) => d,
        Err(e) => unreachable!("{e}"),
    };
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => unreachable!("{e}"),
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

fn packet(body: Vec<u8>, generation: GenerationId) -> Packet {
    Packet {
        track: TrackId::new(0),
        pts: Stamp::Missing,
        dts: Stamp::Missing,
        duration: None,
        data: SharedBytes::from_vec(body),
        keyframe: true,
        discontinuity: false,
        generation,
    }
}

#[test]
fn outcome_and_sink_agree_across_the_corpus() {
    let (mut decoder, generation) = configured();
    let frames = fixture_frames();
    for (index, jpeg) in frames.iter().enumerate() {
        let mut sink = contract::CountingFrameSink::new();
        let outcome = match decoder.decode(packet(jpeg.clone(), generation), &mut sink) {
            Ok(o) => o,
            Err(e) => unreachable!("frame {index}: {e}"),
        };
        match contract::decode_outcome_agrees_with_sink(outcome, sink.accepted()) {
            Ok(()) => {}
            Err(violation) => unreachable!("frame {index}: {violation}"),
        }
        assert_eq!(
            sink.accepted(),
            1,
            "one JPEG packet emits exactly one frame"
        );
    }
}

#[test]
fn stale_packets_are_dropped_on_sight() {
    let (mut decoder, generation) = configured();
    let frames = fixture_frames();
    // A packet minted under the previous generation.
    let stale = packet(frames[0].clone(), GenerationId::FIRST);
    match contract::stale_packet_is_dropped_on_sight(&mut decoder, &stale, generation) {
        Ok(()) => {}
        Err(violation) => unreachable!("{violation}"),
    }
    // The decoder still decodes afterwards (cancellation is not
    // fatality) — and the contract's own drain check agrees.
    match contract::flush_drains(&mut decoder) {
        Ok(()) => {}
        Err(violation) => unreachable!("{violation}"),
    }
}

#[test]
fn flush_drains_immediately_and_sticks() {
    // MJPEG is stateless between packets: flush must reach Drained on
    // the first call and stay there.
    let (mut decoder, _generation) = configured();
    match contract::flush_drains(&mut decoder) {
        Ok(()) => {}
        Err(violation) => unreachable!("{violation}"),
    }
}

#[test]
fn corrupt_input_recovers_or_errors_typed() {
    let (mut decoder, generation) = configured();
    let frames = fixture_frames();

    // Entropy truncation: header intact → concealed recovery.
    let mut truncated = frames[7].clone();
    truncated.truncate(truncated.len() / 2);
    match contract::corrupt_input_recovers_or_errors_typed(
        &mut decoder,
        &packet(truncated, generation),
    ) {
        Ok(()) => {}
        Err(violation) => unreachable!("entropy truncation: {violation}"),
    }

    // Header-level damage: typed Corrupt (nothing emitted).
    let mut no_soi = frames[7].clone();
    no_soi[0] = 0x00;
    match contract::corrupt_input_recovers_or_errors_typed(
        &mut decoder,
        &packet(no_soi, generation),
    ) {
        Ok(()) => {}
        Err(violation) => unreachable!("broken SOI: {violation}"),
    }

    // Empty packet: structural truncation.
    match contract::corrupt_input_recovers_or_errors_typed(
        &mut decoder,
        &packet(Vec::new(), generation),
    ) {
        Ok(()) => {}
        Err(violation) => unreachable!("empty packet: {violation}"),
    }

    // Marker soup: garbage declared lengths.
    let soup = vec![0xff, 0xd8, 0xff, 0xdb, 0xff, 0xff, 0x00, 0x11];
    match contract::corrupt_input_recovers_or_errors_typed(&mut decoder, &packet(soup, generation))
    {
        Ok(()) => {}
        Err(violation) => unreachable!("marker soup: {violation}"),
    }
}

#[test]
fn per_frame_damage_leaves_the_decoder_usable() {
    // Doc 03 §4.5(d)'s CorruptPacket contract: "`CorruptPacket`
    // affects only the current packet — the decoder must return to a
    // state where it can accept the next packet." MJPEG is stateless,
    // so a mid-stream header-smashed frame (audit M9's exact damage:
    // one corrupt JPEG killing the session) must be followed by a
    // clean decode of the very next frame — this is the guarantee the
    // engine's per-frame isolation (W1-08) leans on.
    let (mut decoder, generation) = configured();
    let frames = fixture_frames();
    let mut sink = contract::CountingFrameSink::new();

    // Frame 0 decodes cleanly (the "last good frame" exists).
    match decoder.decode(packet(frames[0].clone(), generation), &mut sink) {
        Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
        other => unreachable!("clean frame 0: {other:?}"),
    }

    // Frame 1 is header-smashed: a typed per-frame Corrupt error,
    // nothing emitted (the contract suite's own damage class).
    let mut no_soi = frames[1].clone();
    no_soi[0] = 0x00;
    let err = match decoder.decode(packet(no_soi, generation), &mut sink) {
        Ok(outcome) => unreachable!("header damage must not succeed: {outcome:?}"),
        Err(e) => e,
    };
    assert!(
        matches!(err.kind, ErrorKind::Corrupt { .. }),
        "header damage is a typed Corrupt error: {err}"
    );
    assert_eq!(sink.accepted(), 1, "nothing emitted for the damaged frame");

    // Frame 2 decodes normally — the decoder stayed usable, and the
    // reused context still produces bit-exact output.
    match decoder.decode(packet(frames[2].clone(), generation), &mut sink) {
        Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
        other => unreachable!("frame after per-frame damage: {other:?}"),
    }
    assert_eq!(sink.accepted(), 2, "the frame after the damage decodes");

    // An entropy-damaged frame (truncation past the header — the
    // in-decoder concealment class, the fixtures suite's proven 3/5
    // cut) is Recovered, and the very next frame is still clean:
    // mixed damage classes stay isolated.
    let mut truncated = frames[3].clone();
    let cut = truncated.len() * 3 / 5;
    truncated.truncate(cut);
    match decoder.decode(packet(truncated, generation), &mut sink) {
        Ok(DecodeOutcome::Recovered { .. }) => {}
        other => unreachable!("entropy truncation: {other:?}"),
    }
    match decoder.decode(packet(frames[4].clone(), generation), &mut sink) {
        Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
        other => unreachable!("frame after concealed damage: {other:?}"),
    }
    assert_eq!(sink.accepted(), 4);
}

#[test]
fn sink_errors_propagate_verbatim() {
    let (mut decoder, generation) = configured();
    let frames = fixture_frames();
    match contract::sink_errors_propagate(&mut decoder, &packet(frames[0].clone(), generation)) {
        Ok(()) => {}
        Err(violation) => unreachable!("{violation}"),
    }
}

#[test]
fn unsupported_features_are_typed_unsupported_not_corrupt() {
    // Out-of-subset JPEG is not damage: the honest answer is a typed
    // Unsupported error at the decode stage (the contract's corrupt
    // check deliberately excludes this kind — this test pins why).
    let (mut decoder, generation) = configured();
    // SOF2 (progressive) where the parents carry SOF0: patch a
    // synthesized minimal progressive header.
    let mut jpeg = vec![0xff, 0xd8];
    let seg = |jpeg: &mut Vec<u8>, marker: u8, payload: &[u8]| {
        jpeg.push(0xff);
        jpeg.push(marker);
        jpeg.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        jpeg.extend_from_slice(payload);
    };
    seg(&mut jpeg, 0xc2, &[8, 0, 8, 0, 8, 1, 1, 0x11, 0]);
    let err = match decoder.decode(
        packet(jpeg, generation),
        &mut contract::CountingFrameSink::new(),
    ) {
        Ok(outcome) => unreachable!("progressive must not decode: {outcome:?}"),
        Err(e) => e,
    };
    assert_eq!(err.stage, media_types::PipelineStage::Decode);
    assert!(
        matches!(err.kind, ErrorKind::Unsupported { .. }),
        "progressive JPEG is Unsupported, not Corrupt: {err}"
    );
}

#[test]
fn unconfigured_use_is_a_typed_decoder_state_error() {
    // The contract's precondition is "already configured"; the
    // decoder still owes a typed answer (not a panic) when misused.
    let mut decoder = MjpegDecoder::new();
    let err = match decoder.decode(
        packet(vec![0xff, 0xd8], GenerationId::FIRST),
        &mut contract::CountingFrameSink::new(),
    ) {
        Ok(outcome) => unreachable!("unconfigured decode must fail: {outcome:?}"),
        Err(e) => e,
    };
    assert!(
        matches!(err.kind, ErrorKind::Decoder { .. }),
        "unconfigured decode is a typed Decoder error: {err}"
    );
    let flush_err = match decoder.flush(&mut contract::CountingFrameSink::new()) {
        Ok(outcome) => unreachable!("unconfigured flush must fail: {outcome:?}"),
        Err(e) => e,
    };
    assert!(
        matches!(flush_err.kind, ErrorKind::Decoder { .. }),
        "unconfigured flush is a typed Decoder error: {flush_err}"
    );
}

#[test]
fn configure_rejects_foreign_codec_descriptors() {
    let mut decoder = MjpegDecoder::new();
    let descriptor = match CodecDescriptor::new("h264") {
        Ok(d) => d,
        Err(e) => unreachable!("{e}"),
    };
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => unreachable!("{e}"),
    };
    let err = match decoder.configure(&DecoderConfig::new(
        descriptor,
        generation,
        ThreadBudget::default(),
        None,
        None,
    )) {
        Ok(()) => unreachable!("h264 must be rejected by the mjpeg decoder"),
        Err(e) => e,
    };
    assert!(
        matches!(err.kind, ErrorKind::Unsupported { .. }),
        "foreign descriptor is Unsupported: {err}"
    );
}
