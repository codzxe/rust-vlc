//! One-buffer-per-frame proof for the MJPEG contract path (W2-06,
//! hardening the ED-FIX-1 C4 claim the W1-20 re-audit flagged: the
//! old `DhtKey::of` cloned the whole DHT payload every frame, and the
//! marker walk built fresh metadata `Vec`s per frame — both are gone;
//! see `src/video/mjpeg/decoder.rs` module docs).
//!
//! After warm-up, each steady-state `MjpegDecoder::decode` through the
//! real contract surface (configure → decode → sink handoff) allocates
//! exactly **two** events, both belonging to the frame's own bytes:
//!
//! 1. the frame's output buffer — `DecodeContext::take_output` moved
//!    the previous one into the sink, so this decode's assembly
//!    `resize` grows the fresh one (the one buffer the decoder.rs
//!    claim counts); and
//! 2. the `SharedBytes` (`Arc<[u8]>`) block the frame contract's
//!    ownership handoff wraps that buffer in (an ownership header,
//!    not frame bytes — collapsing it needs a pooled-`Arc` flavor in
//!    media-types, the registered Lead-owned follow-up).
//!
//! Everything else — marker walk (persistent `JpegInfo` scratch),
//! padded planes, Huffman LUTs, the DHT structural compare, cursors —
//! is reuse (D-1). The input packet is cloned per iteration (an
//! `Arc` refcount bump — allocation-free), so the measured window
//! sees only the decode side.
//!
//! One `#[test]` per binary by design (the allocator is
//! process-global; see `testkit::alloc_count` docs — the same shape
//! as `zero_alloc_quantum` / `zero_alloc_null_sink`).

#[global_allocator]
static ALLOC: testkit::alloc_count::CountingGlobalAlloc =
    testkit::alloc_count::CountingGlobalAlloc::new();

mod common;

use codec_api::prelude::*;
use codecs::video::mjpeg::MjpegDecoder;
use common::{extract_avi_mjpeg_frames, fixture_bytes};
use media_types::{GenerationId, Packet, TrackId};
use testkit::alloc_count::serialize_tests;

/// The largest committed fixture (external-writer-authored by the
/// 2.A3 generator, per-frame sha256 committed): 60 deterministic
/// 320x240 4:2:0 frames — real media, not self-authored bytes (D-5).
const AVI: &str = "generated/valid/avi_mjpeg_pcm.avi";
/// Measured steady-state frames (well past warm-up; fast even in
/// debug — the decode itself is the microbenchmark's own subject).
const FRAMES: usize = 64;

/// A sink that drops frames after counting them (the handoff's real
/// cost is the ownership transfer, not retention).
struct DroppingSink {
    frames: usize,
}

impl FrameSink for DroppingSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        self.frames += 1;
        Ok(())
    }
}

fn packet_for(jpeg: &[u8], generation: GenerationId) -> Packet {
    Packet {
        track: TrackId::new(0),
        pts: media_types::Stamp::Missing,
        dts: media_types::Stamp::Missing,
        duration: None,
        data: media_types::packet::SharedBytes::from_vec(jpeg.to_vec()),
        keyframe: true,
        discontinuity: false,
        generation,
    }
}

#[test]
fn mjpeg_contract_path_allocates_exactly_one_buffer_per_frame() {
    let _serial = serialize_tests();

    // Setup: fixture read, frame extraction, decoder configure — all
    // outside the measured window.
    let bytes = match fixture_bytes(AVI) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let jpegs = match extract_avi_mjpeg_frames(&bytes) {
        Ok(f) => f,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(jpegs.len(), 60);
    // Every fixture frame is the same stream (one geometry, one DHT
    // set) — the honest steady state the claim describes.
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => unreachable!("{e}"),
    };
    let descriptor = match media_types::ids::CodecDescriptor::new("mjpeg") {
        Ok(d) => d,
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
        Ok(()) => {}
        Err(e) => unreachable!("configure: {e}"),
    }
    // One packet, cloned per iteration (Arc bump, no allocation).
    let packet = packet_for(&jpegs[0], generation);

    // Warm-up: first decodes size the planes, the LUTs, the header
    // scratch and the pool geometry.
    let mut sink = DroppingSink { frames: 0 };
    for _ in 0..4 {
        match decoder.decode(packet.clone(), &mut sink) {
            Ok(_) => {}
            Err(e) => unreachable!("warm-up decode: {e}"),
        }
    }
    assert_eq!(sink.frames, 4);

    // The measured window: FRAMES steady-state contract decodes.
    let guard = ALLOC.state().guard();
    for _ in 0..FRAMES {
        match decoder.decode(packet.clone(), &mut sink) {
            Ok(_) => {}
            Err(e) => unreachable!("measured decode: {e}"),
        }
    }
    let delta = guard.delta();
    assert_eq!(sink.frames, 4 + FRAMES);

    // The pin: exactly two allocation events per frame — the frame's
    // own output buffer (the decoder.rs "one buffer" claim) plus the
    // SharedBytes Arc block the handoff wraps it in. Any third event
    // is a regression of the reuse discipline (D-1) and must fail
    // loudly here, not hide in a doc claim.
    assert_eq!(
        delta.alloc_calls,
        2 * FRAMES as u64,
        "expected exactly 2 allocation events per steady-state frame \
         (output buffer + SharedBytes handoff block); measured {} events \
         for {FRAMES} frames ({} bytes) — the reuse discipline regressed",
        delta.alloc_calls,
        delta.total_bytes,
    );
    eprintln!(
        "[zero_alloc_decode] {FRAMES} frames: {} alloc events, {} bytes \
         (per frame: output buffer + Arc handoff block)",
        delta.alloc_calls, delta.total_bytes
    );
}
