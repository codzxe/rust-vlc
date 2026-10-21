//! The 2.C3 seam proof: **the audio output's sample counter is the
//! media-clock's truth** (doc 03 §4.7 e: "the device sample counter is
//! the only trusted progress source. Video never advances the
//! clock."; §4.9: the open session implements
//! [`media_clock::SampleCounterSource`]).
//!
//! A null session is opened in trait mode (the backend owns pause),
//! its clock source handed to a real [`MediaClock::audio_master`],
//! and the session is driven — the clock's media position must track
//! the device's consumption exactly: 44 100 instants per media
//! second, monotonic, frozen across a device pause, resumed with no
//! discontinuity (the 2.C1 anchor semantics exercised through the
//! production audio-output adapter, not a test fake).

mod common;

use audio_output::null::NullDevice;
use audio_output::{AudioBackend, NullBackend, RenderStats};
use common::{TONE_RATE, TONE_SAMPLES_PER_TICK, TONE_TOTAL_SAMPLES};
use media_clock::{ClockMaster, ClockState, MediaClock, NANO_BASE, PlaybackClock};
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::time::{Rate, Timestamp};
use testkit::Must;

#[test]
fn the_device_sample_counter_drives_the_audio_master_clock() {
    let format = AudioFormat {
        sample: SampleFormat::F32,
        channels: 2,
        layout: AudioLayout::Interleaved,
        rate: TONE_RATE,
    };
    // Trait mode: the backend owns pause/resume; the driver is taken
    // out and driven manually, exactly as the null-backend docs
    // prescribe.
    let mut backend = NullBackend::new();
    backend
        .open(audio_output::null::NULL_DEVICE_ID, &format, 8192)
        .must();
    let mut device = backend.take_device().must();

    // The production wiring: the open session's counter is the
    // audio-master's truth source, anchored at media time zero,
    // playing at rate 1.
    let clock = MediaClock::audio_master(
        backend.clock_source().must(),
        Timestamp::new(0, NANO_BASE),
        Rate::new(1, 1).must(),
    )
    .must();
    assert_eq!(clock.master(), ClockMaster::AudioOutput);
    assert_eq!(clock.snapshot().state, ClockState::Running);

    // Drive one tick: write one tick of interleaved f32 bytes through
    // the sink half, render it back out through the device half.
    let ticks = TONE_TOTAL_SAMPLES / TONE_SAMPLES_PER_TICK;
    let drive_tick = |backend: &mut NullBackend, device: &mut NullDevice, tick: usize| {
        let mut bytes = Vec::with_capacity(TONE_SAMPLES_PER_TICK * 2 * 4);
        // Deterministic load: a small integer ramp per tick (content
        // is irrelevant to the clock; the sine-hash suite owns value
        // truth).
        for i in 0..TONE_SAMPLES_PER_TICK {
            let v = ((tick + i) % 7) as f32 * 0.125 - 0.375;
            bytes.extend_from_slice(&v.to_le_bytes());
            bytes.extend_from_slice(&(-v).to_le_bytes());
        }
        use audio_engine::AudioSink;
        use audio_engine::buffer::AudioBuffer;
        use media_types::packet::SharedBytes;
        use media_types::time::Stamp;
        backend
            .write(&AudioBuffer {
                format,
                data: SharedBytes::from_vec(bytes),
                pts: Stamp::Missing,
            })
            .must();
        let mut out = vec![0.0f32; TONE_SAMPLES_PER_TICK * 2];
        let stats = device.render(&mut out).must();
        assert!(stats.fully_served());
    };

    // Drive half the session: 30 ticks × 1470 = 44 100 instants =
    // exactly 1.000 s of media time. Monotonicity at every step.
    let mut last = clock.now().ticks;
    for tick in 0..ticks / 2 {
        drive_tick(&mut backend, &mut device, tick);
        let now = clock.now();
        assert!(now.ticks >= last, "clock regressed at tick {tick}");
        last = now.ticks;
    }
    assert_eq!(
        clock.now(),
        Timestamp::new(1_000_000_000, NANO_BASE),
        "44 100 instants at 44 100 Hz is exactly one second"
    );

    // Pause the device: renders become no-ops, the counter freezes,
    // and the clock holds the frozen position (doc 03 §4.7 e).
    backend.pause().must();
    for _ in 0..3 {
        let mut out = vec![0.0f32; 64];
        let stats = device.render(&mut out).must();
        assert_eq!(stats, RenderStats::default());
    }
    assert_eq!(
        clock.now(),
        Timestamp::new(1_000_000_000, NANO_BASE),
        "a paused device freezes the clock"
    );

    // Resume and finish: the remaining 30 ticks land exactly on
    // 2.000 s — no discontinuity across the pause (the counter never
    // rewound; the clock never jumped).
    backend.resume().must();
    for tick in ticks / 2..ticks {
        drive_tick(&mut backend, &mut device, tick);
        let now = clock.now();
        assert!(now.ticks >= last, "clock regressed at tick {tick}");
        last = now.ticks;
    }
    assert_eq!(
        clock.now(),
        Timestamp::new(2_000_000_000, NANO_BASE),
        "88 200 instants at 44 100 Hz is exactly two seconds"
    );
    // The counter itself reads the same truth the clock scaled.
    assert_eq!(device.sample_count(), TONE_TOTAL_SAMPLES as u64);
    // try_now is sound across the whole run (no regression ever
    // happened — the typed read agrees with the infallible one).
    assert_eq!(clock.try_now().must().ticks, 2_000_000_000);
    // Latency stays honest through the session: everything written
    // was consumed.
    assert_eq!(device.buffered_frames(), 0);
    assert_eq!(backend.underruns(), 0);
}
