//! Zero-allocation proof for the 3.B1 drift-corrected quantum (the
//! companion of `zero_alloc_quantum.rs`, over the corrected path):
//! with a **non-zero** `DriftCorrection` in force and the governor
//! armed and observing per quantum, `process → mix → take_output →
//! drop → observe_drift` over N further quanta allocates **zero
//! bytes** — the micro-resampler's planes, the phase carry and the
//! governor's window state are all fixed-size (D-1/D-8), measured by
//! the counting global allocator, not argued from code reading.
//!
//! One `#[test]` per binary by design (the allocator is
//! process-global; sibling test threads and concurrent harness
//! reporting would be counted — see `testkit::alloc_count` docs).

#[global_allocator]
static ALLOC: testkit::alloc_count::CountingGlobalAlloc =
    testkit::alloc_count::CountingGlobalAlloc::new();

use audio_engine::{AudioPipeline, PipelineSpec};
use media_clock::DriftCorrection;
use media_types::frame::{AudioFormat, AudioFrame, AudioLayout, SampleFormat};
use media_types::ids::GenerationId;
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, Timestamp};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

const RATE: u32 = 44_100;
const QUANTUM_FRAMES: usize = 1470;
const QUANTA: usize = 512;
/// The wiring's fill target (W1-18 §3.2: 2× the doc 03 §4.8 c jitter
/// margin) — only its magnitude matters to the allocator.
const TARGET_FILL_NS: u64 = 40_000_000;

/// A 1470-frame stereo s16 quantum with per-quantum varying content
/// (a stale-slot or stale-phase leak would surface as wrong bytes).
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

/// The virtual device time between quanta (the injected `now_ns` —
/// the sample counter's ns, advancing one quantum per render).
const QUANTUM_NS: u64 = QUANTUM_FRAMES as u64 * 1_000_000_000 / 44_100;

#[test]
fn drift_corrected_quantum_path_is_zero_allocation_after_warm_up() {
    let _serial = serialize_tests();

    // Setup: the armed governor (the wiring's loop closure shape) and
    // a fixed +600 ppm correction (the resampler's ratio source).
    let mut pipeline = AudioPipeline::new(PipelineSpec {
        rate: RATE,
        channels: 2,
        tracks: 1,
    })
    .must()
    .with_drift_governor(TARGET_FILL_NS);
    pipeline.set_drift(DriftCorrection::new(600));
    let frames: Vec<AudioFrame> = (0..QUANTA).map(quantum_frame).collect();

    // Warm-up: enough quanta to size the resampler's output planes
    // and STABILIZE the pooled rotation. At +600 ppm the quantum's
    // device-sample count is 1470 × 1.0006 ⇒ 1470.88 — roughly seven
    // 1471-frame quanta per 1470-frame one — so BOTH slot lengths
    // appear and each needs its slots in the bounded rotation before
    // the measured window opens (128 quanta ≈ 15 short-length hits;
    // the rotation stabilizes, after which reuse is in place).
    let mut now = 0_u64;
    for frame in &frames[..128] {
        pipeline.process(0, frame).must();
        assert!(pipeline.mix().must());
        drop(pipeline.take_output().must());
        now = now.wrapping_add(QUANTUM_NS);
        // The same equilibrium fill as the measured window (below):
        // the correction holds +600 ppm throughout, so the warm-up
        // sees exactly the lengths the measured window will.
        let _ = pipeline.observe_drift(now, Some(TARGET_FILL_NS - 12_000_000));
    }
    // The governor's window has opened; the correction is live.
    assert_eq!(pipeline.drift_stats().samples, 128);

    // The measured window: the pump's whole per-quantum path with the
    // correction in force and the governor observing per quantum. The
    // fill script holds the governor's EQUILIBRIUM for +600 ppm
    // (fill = target − W_c × 600 ppm = target − 12 ms): the desired
    // correction equals the one in force, so the step limiter holds
    // it there — the ratio (and with it the quantum's two possible
    // device-sample lengths) stays CONSTANT, which is exactly the
    // steady state whose allocation behavior is being measured
    // (a correction CHANGE is a cold-path geometry change by design:
    // new lengths enter fresh pooled slots, bounded by the rotation).
    let guard = ALLOC.state().guard();
    for frame in &frames {
        now = now.wrapping_add(QUANTUM_NS);
        let correction = pipeline.observe_drift(now, Some(TARGET_FILL_NS - 12_000_000));
        assert!(correction.ppm().abs() <= 1000);
        pipeline.process(0, frame).must();
        assert!(pipeline.mix().must());
        drop(pipeline.take_output().must());
    }
    guard.assert_zero(
        "drift-corrected quantum: observe + process + mix + take_output over 512 quanta",
    );
    // The loop actually ran: the governor took every sample and the
    // correction stayed live in the resampler.
    assert_eq!(pipeline.drift_stats().samples, QUANTA as u64 + 128);
    assert_eq!(pipeline.drift().ppm(), 600);
}
