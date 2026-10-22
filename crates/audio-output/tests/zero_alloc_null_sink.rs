//! Zero-allocation proof for the null sink's write path (ED-FIX-2,
//! card M2): after one warm-up cycle, N × (`write_bytes` +
//! `AudioSink::write` + device `render`) allocates **zero bytes** —
//! measured by the counting global allocator. The old per-quantum
//! `Vec<f32>` materialization is gone (fixed stack chunks); the ring
//! bytes are byte-identical (pinned by the capture ACs in the null
//! backend suite and the sine-hash AC).
//!
//! One `#[test]` per binary by design (the allocator is
//! process-global; see `testkit::alloc_count` docs).

#[global_allocator]
static ALLOC: testkit::alloc_count::CountingGlobalAlloc =
    testkit::alloc_count::CountingGlobalAlloc::new();

use audio_engine::AudioSink;
use audio_engine::buffer::AudioBuffer;
use audio_output::{NullBackend, NullConfig};
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::Stamp;
use testkit::Must;
use testkit::alloc_count::serialize_tests;

const RATE: u32 = 44_100;
/// One device period of stereo f32 (matches the sine-hash AC's shape).
const QUANTUM_SAMPLES: usize = 1470 * 2;
const CYCLES: usize = 256;

fn f32_format() -> AudioFormat {
    AudioFormat {
        sample: SampleFormat::F32,
        channels: 2,
        layout: AudioLayout::Interleaved,
        rate: RATE,
    }
}

#[test]
fn null_sink_write_path_is_zero_allocation_after_warm_up() {
    let _serial = serialize_tests();

    // Setup: input materialization allocates freely, outside the
    // measured window.
    let format = f32_format();
    let mut stream = NullBackend::open_stream(&format, 8192, NullConfig { capture: false }).must();
    let mut bytes = Vec::with_capacity(QUANTUM_SAMPLES * 4);
    for i in 0..QUANTUM_SAMPLES {
        let v = ((i % 251) as f32 - 125.0) / 128.0;
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    let buffer = AudioBuffer {
        format,
        data: SharedBytes::from_vec(bytes.clone()),
        pts: Stamp::Missing,
    };
    // The device render target lives on the stack (the real-time
    // callback's own shape — a fixed buffer the caller owns).
    let mut out = [0f32; QUANTUM_SAMPLES];

    // Warm-up: one full write + render cycle through both entry
    // points (write_bytes and the AudioSink::write trait path).
    stream.sink.write_bytes(&bytes).must();
    stream.device.render(&mut out).must();
    stream.sink.write(&buffer).must();
    stream.device.render(&mut out).must();

    // The measured window: the pump→sink→device hand-off, both entry
    // points, N cycles. Each render drains exactly one quantum, so
    // the ring never fills (backpressure is a different, typed path).
    let guard = ALLOC.state().guard();
    for _ in 0..CYCLES {
        stream.sink.write_bytes(&bytes).must();
        stream.device.render(&mut out).must();
        stream.sink.write(&buffer).must();
        stream.device.render(&mut out).must();
    }
    guard.assert_zero("null sink: write_bytes + write + render over 256 cycles");
}
