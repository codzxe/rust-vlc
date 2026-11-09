//! The wave-14 PCM pipeline proof (issue #1): **decoded frames → the
//! bounded device queue → the sink trait, paced by the playback
//! clock** — the whole path behind the ADR-005 seam, on the null
//! backend, with no new dependencies.
//!
//! What each test proves:
//!
//! 1. `clock_synced_pump_delivers_the_exact_sine_stream` — a
//!    [`SinePattern`] (440 Hz) is pushed through a [`PcmPump`] whose
//!    pacing clock is the **same `SampleCounterSource` a real
//!    `MediaClock::audio_master` reads**; the drive loop renders only
//!    when the pump says the device is fed. Result: the device
//!    capture is **bit-for-bit the oracle's sample stream**, zero
//!    underruns, zero drops, and the media clock lands on exactly
//!    1.000 s after 44 100 instants.
//! 2. `drop_policy_bounds_memory_and_keeps_the_prefix` — with the
//!    device starved, `DropPolicy::Drop` keeps the queue at/below the
//!    high-water mark forever, counts every dropped frame, and the
//!    sink still received the exact in-order prefix.
//! 3. `backpressure_is_typed_and_clears_as_the_clock_advances` —
//!    `DropPolicy::Backpressure` refuses over-budget buffers with the
//!    typed `audio.pump_backpressure` error, and the *same* push
//!    succeeds after the device (the clock) consumed frames.
//! 4. `the_cpal_seam_refuses_with_the_typed_marker` — the ADR-005
//!    flip point answers with the typed marker, never a fake success.

use std::sync::Arc;

use audio_engine::buffer::AudioBuffer;
use audio_output::cpal::{self, AUDIO_ADR005_ADMITTED, BackendNotAdmitted};
use audio_output::null::{NullBackend, NullConfig};
use audio_output::pattern::SinePattern;
use audio_output::pump::{DropPolicy, PcmPump};
use media_clock::{
    ClockMaster, MediaClock, NANO_BASE, PlaybackClock, SampleCounterSource,
};
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::{Rate, Stamp, Timestamp};
use testkit::Must;

const RATE: u32 = 44_100;

fn stereo_f32() -> AudioFormat {
    AudioFormat {
        sample: SampleFormat::F32,
        channels: 2,
        layout: AudioLayout::Interleaved,
        rate: RATE,
    }
}

/// Interleaved f32 samples → a decoded-format `AudioBuffer`.
fn buffer(format: AudioFormat, samples: &[f32]) -> AudioBuffer {
    let mut bytes = Vec::with_capacity(samples.len() * 4);
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    AudioBuffer {
        format,
        data: SharedBytes::from_vec(bytes),
        pts: Stamp::Missing,
    }
}

/// Captured LE bytes → f32 samples (the capture side of the oracle).
fn as_f32s(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

#[test]
fn clock_synced_pump_delivers_the_exact_sine_stream() {
    let format = stereo_f32();
    const CHUNK: usize = 1_470; // 30 ticks = exactly 1.000 s
    const HIGH_WATER: u64 = 2_048; // ≥ CHUNK: one buffer always fits
    let stream = NullBackend::open_stream(&format, HIGH_WATER as usize, NullConfig {
        capture: true,
    })
    .must();

    // The playback-clock integration: the pump paces on exactly the
    // counter the media clock reads.
    let clock: Arc<dyn SampleCounterSource> = stream.sink.clock_source();
    let mut pump = PcmPump::new(stream.sink, DropPolicy::Backpressure, HIGH_WATER).must();
    pump.attach_clock(clock.clone());
    let mut device = stream.device;
    let media =
        MediaClock::audio_master(clock, Timestamp::new(0, NANO_BASE), Rate::new(1, 1).must())
            .must();
    assert_eq!(media.master(), ClockMaster::AudioOutput);

    let mut pattern = SinePattern::new(440.0, &format, 0.5).must();
    let ticks = (RATE as usize) / CHUNK;
    for tick in 0..ticks {
        let mut samples = vec![0.0f32; CHUNK * 2];
        assert_eq!(pattern.fill(&mut samples), CHUNK);
        // Clock-synced pacing: render only while the next buffer does
        // not fit the high-water budget.
        while !pump.fits(CHUNK as u64) {
            let mut out = vec![0.0f32; CHUNK * 2];
            let stats = device.render(&mut out).must();
            assert!(
                stats.fully_served(),
                "underrun at tick {tick}: the pump starved the device"
            );
            assert_eq!(stats.silence_filled, 0, "silence at tick {tick}");
        }
        let outcome = pump.push(&buffer(format, &samples)).must();
        assert_eq!(outcome.accepted, CHUNK as u64, "tick {tick}");
        assert_eq!(outcome.dropped, 0, "tick {tick}");
        assert!(outcome.in_flight <= HIGH_WATER, "tick {tick}");
    }
    // Drain: everything written is played before the session ends.
    while pump.in_flight() > 0 {
        let mut out = vec![0.0f32; CHUNK * 2];
        let stats = device.render(&mut out).must();
        assert!(stats.fully_served());
    }

    assert_eq!(pump.written_frames(), u64::from(RATE));
    assert_eq!(pump.dropped_frames(), 0);
    assert_eq!(pump.in_flight(), 0);
    assert_eq!(
        media.now(),
        Timestamp::new(1_000_000_000, NANO_BASE),
        "44 100 instants at 44 100 Hz is exactly one second"
    );

    // The exactness claim: the captured stream equals the oracle,
    // bit-for-bit, every channel of every frame.
    let played = as_f32s(&device.take_capture());
    assert_eq!(played.len(), RATE as usize * 2);
    for instant in 0..RATE as usize {
        let expected = pattern.sample_at(instant as u64);
        assert_eq!(played[instant * 2], expected, "left @{instant}");
        assert_eq!(played[instant * 2 + 1], expected, "right @{instant}");
    }
    assert_eq!(pump.sink().sample_count(), u64::from(RATE));
}

#[test]
fn drop_policy_bounds_memory_and_keeps_the_prefix() {
    let format = stereo_f32();
    const CAPACITY: u64 = 1_024;
    let stream = NullBackend::open_stream(&format, CAPACITY as usize, NullConfig {
        capture: true,
    })
    .must();
    // No clock attached, no renders: the device consumes nothing —
    // the starved-path worst case.
    let mut pump = PcmPump::new(stream.sink, DropPolicy::Drop, CAPACITY).must();
    let mut device = stream.device;
    let mut pattern = SinePattern::new(440.0, &format, 0.5).must();

    const CHUNK: u64 = 256;
    for tick in 0..6u64 {
        let mut samples = vec![0.0f32; CHUNK as usize * 2];
        assert_eq!(pattern.fill(&mut samples), CHUNK as usize);
        let outcome = pump.push(&buffer(format, &samples)).must();
        if tick < 4 {
            assert_eq!(outcome.accepted, CHUNK, "tick {tick}");
            assert_eq!(outcome.dropped, 0, "tick {tick}");
        } else {
            assert_eq!(outcome.accepted, 0, "tick {tick}");
            assert_eq!(outcome.dropped, CHUNK, "tick {tick}");
        }
        // Bounded, proven live: neither the pacing budget nor the
        // ring ever grows past the capacity, however much is pushed.
        assert!(pump.in_flight() <= CAPACITY, "tick {tick}");
        assert!(pump.sink().buffered_frames() <= CAPACITY as usize, "tick {tick}");
    }
    assert_eq!(pump.written_frames(), CAPACITY);
    assert_eq!(pump.dropped_frames(), 2 * CHUNK);

    // Play out everything queued: the capture is the exact accepted
    // prefix, in order, nothing else.
    let mut out = vec![0.0f32; 512 * 2];
    for _ in 0..2 {
        let stats = device.render(&mut out).must();
        assert!(stats.fully_served());
    }
    let played = as_f32s(&device.take_capture());
    assert_eq!(played.len(), CAPACITY as usize * 2);
    for instant in 0..CAPACITY as usize {
        let expected = pattern.sample_at(instant as u64);
        assert_eq!(played[instant * 2], expected, "left @{instant}");
        assert_eq!(played[instant * 2 + 1], expected, "right @{instant}");
    }
}

#[test]
fn backpressure_is_typed_and_clears_as_the_clock_advances() {
    let format = stereo_f32();
    let stream = NullBackend::open_stream(&format, 2_048, NullConfig {
        capture: false,
    })
    .must();
    let clock: Arc<dyn SampleCounterSource> = stream.sink.clock_source();
    const HIGH_WATER: u64 = 512;
    let mut pump = PcmPump::new(stream.sink, DropPolicy::Backpressure, HIGH_WATER).must();
    pump.attach_clock(clock);
    let mut device = stream.device;

    let chunk = |pattern: &mut SinePattern| {
        let mut samples = vec![0.0f32; 256 * 2];
        pattern.fill(&mut samples);
        buffer(format, &samples)
    };
    let mut pattern = SinePattern::new(440.0, &format, 0.5).must();

    // Fill to the budget: 2 × 256 accepted.
    assert_eq!(pump.push(&chunk(&mut pattern)).must().accepted, 256);
    assert_eq!(pump.push(&chunk(&mut pattern)).must().accepted, 256);
    assert!(pump.in_flight() <= HIGH_WATER);

    // The third buffer is refused — typed, whole-buffer, countable.
    let err = pump.push(&chunk(&mut pattern)).err().must();
    assert_eq!(err.context.code, "audio.pump_backpressure");
    assert!(matches!(
        err.kind,
        media_types::ErrorKind::ResourceLimit { .. }
    ));
    assert_eq!(pump.written_frames(), 512, "a refused push never counts");

    // The device (the clock's truth) consumes 256 frames — and the
    // very same buffer now fits.
    let mut out = vec![0.0f32; 256 * 2];
    let stats = device.render(&mut out).must();
    assert!(stats.fully_served());
    assert_eq!(pump.sink().sample_count(), 256);
    let outcome = pump.push(&chunk(&mut pattern)).must();
    assert_eq!(outcome.accepted, 256);
    assert_eq!(outcome.in_flight, HIGH_WATER);
    assert!(pump.in_flight() <= HIGH_WATER, "never over budget");
}

#[test]
fn the_cpal_seam_refuses_with_the_typed_marker() {
    assert!(!AUDIO_ADR005_ADMITTED, "the ADR-005 gate is recorded closed");
    let marker: BackendNotAdmitted = cpal::cpal_backend().err().must();
    assert_eq!(marker, BackendNotAdmitted::CPAL);
    assert!(marker.to_string().contains("ADR-005"));
}
