//! Zero-allocation proof for the PCM contract path (card W5-19, the
//! D-1 audit): after warm-up, each steady-state `PcmDecoder::decode`
//! through the real contract surface (configure → decode → sink
//! handoff → drop) allocates **zero bytes** — measured by the counting
//! global allocator, not argued from code reading — on BOTH decode
//! shapes:
//!
//! 1. the zero-copy passthrough (native byte order, whole packets —
//!    the committed corpus's s16le shape: the frame IS the packet's
//!    `SharedBytes`, an Arc refcount bump);
//! 2. the conversion-carrying path (a byte-swapped s16be stream: the
//!    whole-frame prefix is group-swapped into a pooled slot — the
//!    bounded `CONVERT_SLOTS` rotation, refillable in place once the
//!    sink drops its handle).
//!
//! One `#[test]` per binary by design (the allocator is
//! process-global; sibling test threads would be counted — see
//! `testkit::alloc_count` docs, the `zero_alloc_quantum` precedent).

#[global_allocator]
static ALLOC: testkit::alloc_count::CountingGlobalAlloc =
    testkit::alloc_count::CountingGlobalAlloc::new();

mod common;

use codec_api::prelude::*;
use codecs::audio::pcm::PcmDecoder;
use common::{extract_avi_audio_strf, extract_avi_pcm_chunks, fixture_bytes};
use media_types::ids::GenerationId;
use media_types::packet::SharedBytes;
use media_types::{Packet, TrackId};
use testkit::alloc_count::serialize_tests;

/// The committed class-A parent fixture (external-writer-authored by
/// the 2.A3 generator; the audio stream digest is pinned by the
/// `pcm_fixtures` suite): 60 chunks of 1470-frame stereo s16le.
const AVI: &str = "generated/valid/avi_mjpeg_pcm.avi";
/// Measured steady-state packets per shape (well past warm-up).
const PACKETS: usize = 256;

/// A sink that drops frames after counting them (the handoff's real
/// cost is the ownership transfer, not retention — the slot returns
/// to the pool inside the measured window).
struct DroppingSink {
    frames: usize,
}

impl FrameSink for DroppingSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        self.frames += 1;
        Ok(())
    }
}

fn configured(descriptor: &str, extra: Vec<u8>) -> (PcmDecoder, GenerationId) {
    let descriptor = match media_types::ids::CodecDescriptor::new(descriptor) {
        Ok(d) => d,
        Err(e) => unreachable!("{e}"),
    };
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => unreachable!("{e}"),
    };
    let mut decoder = PcmDecoder::new();
    match decoder.configure(&DecoderConfig::new(
        descriptor,
        generation,
        ThreadBudget::default(),
        None,
        Some(SharedBytes::from_vec(extra)),
    )) {
        Ok(()) => (decoder, generation),
        Err(e) => unreachable!("configure: {e}"),
    }
}

fn packet_for(bytes: &[u8], generation: GenerationId) -> Packet {
    Packet {
        track: TrackId::new(1),
        pts: media_types::Stamp::Missing,
        dts: media_types::Stamp::Missing,
        duration: None,
        data: SharedBytes::from_vec(bytes.to_vec()),
        keyframe: true,
        discontinuity: false,
        generation,
    }
}

#[test]
fn pcm_contract_path_is_zero_allocation_per_buffer_after_warm_up() {
    let _serial = serialize_tests();

    // Setup: fixture read, chunk extraction, the container's own strf
    // — all outside every measured window.
    let bytes = match fixture_bytes(AVI) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let strf = match extract_avi_audio_strf(&bytes) {
        Ok(s) => s,
        Err(e) => unreachable!("{e}"),
    };
    let chunks = match extract_avi_pcm_chunks(&bytes) {
        Ok(c) => c,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(chunks.len(), 60);
    // The byte-swapped twin of the same stream (an s16be
    // representation of the fixture's samples: the conversion-carrying
    // shape).
    let swapped_chunks: Vec<Vec<u8>> = chunks
        .iter()
        .map(|chunk| {
            let mut out = chunk.clone();
            for pair in out.as_chunks_mut::<2>().0 {
                pair.reverse();
            }
            out
        })
        .collect();

    // ---- Shape 1: the zero-copy passthrough (s16le) --------------------
    let (mut decoder, generation) = configured("pcm_s16le", strf.clone());
    let mut sink = DroppingSink { frames: 0 };
    // Warm-up: the first decode installs the plan (already done by
    // configure) — the passthrough allocates nothing even cold, but
    // the warm-up pass keeps the two shapes' protocols identical.
    for chunk in &chunks {
        match decoder.decode(packet_for(chunk, generation), &mut sink) {
            Ok(_) => {}
            Err(e) => unreachable!("warm-up decode: {e}"),
        }
    }
    let warm = sink.frames;
    // The measured window: PACKETS steady-state contract decodes. The
    // input packets are built OUTSIDE the window (packet_for copies);
    // inside, only decode + sink + drop run.
    let packets: Vec<Packet> = chunks
        .iter()
        .cycle()
        .take(PACKETS)
        .map(|chunk| packet_for(chunk, generation))
        .collect();
    let guard = ALLOC.state().guard();
    for packet in packets {
        match decoder.decode(packet, &mut sink) {
            Ok(_) => {}
            Err(e) => unreachable!("measured decode: {e}"),
        }
    }
    let delta = guard.delta();
    assert_eq!(sink.frames, warm + PACKETS);
    // The pin: zero allocation events per buffer on the passthrough —
    // the frame is the packet's own SharedBytes (Arc clone).
    assert_eq!(
        delta.alloc_calls, 0,
        "s16le passthrough allocated {} events ({} bytes) over {PACKETS} buffers — \
         the zero-copy handoff regressed",
        delta.alloc_calls, delta.total_bytes,
    );

    // ---- Shape 2: the pooled conversion (s16be) ------------------------
    let (mut decoder, generation) = configured("pcm_s16be", strf);
    let mut sink = DroppingSink { frames: 0 };
    // Warm-up: the first packets of the geometry enter slots into the
    // bounded rotation; dropping the frames returns them to refcount 1.
    for chunk in &swapped_chunks {
        match decoder.decode(packet_for(chunk, generation), &mut sink) {
            Ok(_) => {}
            Err(e) => unreachable!("warm-up decode: {e}"),
        }
    }
    let warm = sink.frames;
    let packets: Vec<Packet> = swapped_chunks
        .iter()
        .cycle()
        .take(PACKETS)
        .map(|chunk| packet_for(chunk, generation))
        .collect();
    let guard = ALLOC.state().guard();
    for packet in packets {
        match decoder.decode(packet, &mut sink) {
            Ok(_) => {}
            Err(e) => unreachable!("measured decode: {e}"),
        }
    }
    let delta = guard.delta();
    assert_eq!(sink.frames, warm + PACKETS);
    // The pin: zero allocation events per converted buffer after
    // warm-up — the group-swap writes into the pooled slot in place.
    assert_eq!(
        delta.alloc_calls, 0,
        "s16be converted path allocated {} events ({} bytes) over {PACKETS} buffers — \
         the pooled-slot rotation regressed",
        delta.alloc_calls, delta.total_bytes,
    );
    eprintln!(
        "[pcm_zero_alloc] {PACKETS} buffers per shape: 0 alloc events (passthrough + converted)"
    );
}
