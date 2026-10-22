//! Zero-allocation proof for the audio quantum (ED-FIX-2, the D-6
//! "runtime green" instrument's first production target): after one
//! warm-up quantum, `process → mix → take_output → drop` over N
//! further quanta allocates **zero bytes** — measured by the counting
//! global allocator, not argued from code reading.
//!
//! One `#[test]` per binary by design (the allocator is
//! process-global; sibling test threads and concurrent harness
//! reporting would be counted — see `testkit::alloc_count` docs). The
//! binary runs identically under `--test-threads=1` and the default
//! parallel runner.

#[global_allocator]
static ALLOC: testkit::alloc_count::CountingGlobalAlloc =
    testkit::alloc_count::CountingGlobalAlloc::new();

use audio_engine::{AudioPipeline, PipelineSpec};
use media_types::frame::{AudioFormat, AudioFrame, AudioLayout, SampleFormat};
use media_types::ids::GenerationId;
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, Timestamp};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

const RATE: u32 = 44_100;
const QUANTUM_FRAMES: usize = 1470;
const QUANTA: usize = 512;

/// A 1470-frame stereo s16 quantum with content that varies per
/// quantum (a stale-slot leak would surface as wrong bytes in the
/// sine-hash and value ACs; here it only needs to be a real payload).
fn quantum_frame(index: usize) -> AudioFrame {
    let mut bytes = Vec::with_capacity(QUANTUM_FRAMES * 4);
    for i in 0..QUANTUM_FRAMES {
        let v = (((index * 37 + i * 11) % 4096) as i16) - 2048;
        bytes.extend_from_slice(&v.to_le_bytes());
        bytes.extend_from_slice(&(-v).to_le_bytes());
    }
    AudioFrame {
        format: AudioFormat {
            sample: SampleFormat::S16,
            channels: 2,
            layout: AudioLayout::Interleaved,
            rate: RATE,
        },
        samples: SharedBytes::from_vec(bytes),
        pts: Stamp::Present(Timestamp::from_parts((index * QUANTUM_FRAMES) as i64, 1, RATE).must()),
        duration: None,
        generation: GenerationId::FIRST,
    }
}

#[test]
fn audio_quantum_path_is_zero_allocation_after_warm_up() {
    let _serial = serialize_tests();

    // Setup + input materialization: allocates freely, outside every
    // measured window.
    let mut pipeline = AudioPipeline::new(PipelineSpec {
        rate: RATE,
        channels: 2,
        tracks: 1,
    })
    .must();
    let frames: Vec<AudioFrame> = (0..QUANTA).map(quantum_frame).collect();

    // Warm-up: exactly one quantum sizes the stage-A scratch planes,
    // the staged slot, the mixer planes and the first pooled output
    // slot (doc 03 §4.8 c — "the first quantum warms the capacities").
    pipeline.process(0, &frames[0]).must();
    assert!(pipeline.mix().must());
    drop(pipeline.take_output().must());

    // The measured window: the pump's whole per-quantum path.
    let guard = ALLOC.state().guard();
    for frame in &frames {
        pipeline.process(0, frame).must();
        assert!(pipeline.mix().must());
        // The pump's consumption shape: take, hand to the sink, drop —
        // the slot returns to refcount 1 inside the window (a drop is
        // a dealloc, never an alloc).
        drop(pipeline.take_output().must());
    }
    guard.assert_zero("audio quantum: process + mix + take_output over 512 quanta");

    // A geometry change is the only thing that may allocate again
    // (a different quantum length enters a fresh slot into the
    // bounded rotation) — then silent at the new geometry too.
    // Proven here so the cold path is characterized, not just avoided.
    let full = quantum_frame(0);
    let small = AudioFrame {
        samples: SharedBytes::from_vec(full.samples.as_slice()[..QUANTUM_FRAMES * 2].to_vec()),
        ..full
    };
    pipeline.process(0, &small).must();
    assert!(pipeline.mix().must());
    drop(pipeline.take_output().must());
    let guard = ALLOC.state().guard();
    for _ in 0..8 {
        pipeline.process(0, &small).must();
        assert!(pipeline.mix().must());
        drop(pipeline.take_output().must());
    }
    guard.assert_zero("audio quantum at the changed geometry");
}
