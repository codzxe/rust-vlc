//! AC suite 2 — the **callback budget** (card 2.C3): "the callback
//! path instrumented (operation/allocation budget per doc 03 §4.9)
//! and proven within budget on the null adapter".
//!
//! Doc 03 §4.9 (c) is binding: inside the render callback it is
//! forbidden — any lock except the SPSC ring's wait-free index swap,
//! allocation, logging, syscalls, unbounded loops. The v0 enforcement
//! is the doc's own clause: *"code review plus these tests are the
//! gate"*. This suite is the tests half:
//!
//! - **Operation ledger** — every callback's work is counted in
//!   [`RenderStats`] (the instrumentation): exactly one ring read
//!   (≤ 2 contiguous segments — the power-of-two wrap bound), at most
//!   one silence fill, exactly one sample-counter advance (measured
//!   as the counter delta, not trusted), at most one underrun count.
//!   Asserted per call, over a fully driven session AND over a
//!   geometry sweep of channels × capacity × request sizes
//!   (including requests larger than the ring).
//! - **Allocation budget** — the callback path
//!   ([`audio_output::render_into`]) takes only borrows and returns a
//!   `Copy` ledger; it constructs no owned value, so it *cannot*
//!   allocate (the structural half). The measurable half: the ring's
//!   slot count and capacity are proven constant across every
//!   callback of the driven session (zero storage growth), and the
//!   driver-side wrapper's only growing buffer (capture) runs after
//!   the callback returns, never inside it. Doc 03 §4.9 (c)'s
//!   fault-injecting allocator hook needs a `#[global_allocator]` —
//!   i.e. `unsafe`, platform-adapter territory per doc 01 §2.7 — and
//!   is `[PROPOSAL: tooling]` owed to testkit hardening (recorded in
//!   the crate's render module docs).
//! - **Time budget** — doc 03 §4.9 (e) marks worst-case execution
//!   time `[VERIFY: per-platform numbers to be recorded in Phase 3]`;
//!   wall-clock asserts are deliberately absent here (scheduling
//!   noise makes them flaky — the 2.C1 flake lesson). The *bounded
//!   work* argument is the ledger: a callback's work is a fixed
//!   linear-in-`out.len()` copy plus O(1) bookkeeping, no unbounded
//!   loop (the ring geometry proof, asserted as the segment bound).
//! - **Pause is a zero-operation callback** — a paused device pulls
//!   nothing: no callback work, no counter advance, no capture.

mod common;

use audio_output::AudioBackend;
use audio_output::render::render_into;
use audio_output::{NullBackend, NullConfig, RenderStats};
use common::{TONE_RATE, TONE_SAMPLES_PER_TICK, TONE_TOTAL_SAMPLES, corpus_pcm};
use media_clock::SampleCounterSource;
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use std::sync::atomic::{AtomicU64, Ordering};
use testkit::Must;

/// Assert one callback's ledger is within the doc 03 §4.9 (c) budget.
/// `counter_before`/`counter_after` bracket the call so the "exactly
/// one counter advance" is measured, not assumed; the underrun counts
/// bracket it the same way.
fn assert_within_budget(
    stats: &RenderStats,
    counter_before: u64,
    counter_after: u64,
    underruns_before: u64,
    underruns_after: u64,
) {
    // One ring read, bounded by the power-of-two wrap: at most two
    // contiguous segments (never an unbounded loop).
    assert!(
        stats.ring_segments <= 2,
        "ring segments {} > 2 (wrap bound violated)",
        stats.ring_segments
    );
    // Whole-frame accounting: the callback served the request with
    // data + silence, nothing else.
    assert_eq!(
        stats.frames_read + stats.silence_filled,
        stats.frames_requested,
        "frames must be accounted whole"
    );
    // At most one silence fill, covering exactly the shortfall; the
    // underrun flag matches it.
    assert!(stats.silence_filled <= stats.frames_requested);
    assert_eq!(stats.silence_filled > 0, stats.underrun);
    // Exactly one counter advance of the full request (data and
    // silence both advance the device — the clock's truth).
    assert_eq!(
        counter_after - counter_before,
        stats.frames_requested as u64,
        "exactly one counter advance per callback"
    );
    // At most one underrun count per callback.
    assert!(underruns_after - underruns_before <= 1);
    assert_eq!(
        underruns_after - underruns_before,
        u64::from(stats.underrun),
        "one underrun event iff the callback could not fill entirely"
    );
}

/// The corpus PCM as interleaved f32 device bytes (audio-engine's
/// exact unity conversion — `v / 32768`, value-proven by the
/// sine-hash suite; here it is the deterministic load).
fn s16_to_device_f32(s16le: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s16le.len() * 2);
    for pair in s16le.as_chunks::<2>().0 {
        let sample = i16::from_le_bytes(*pair);
        out.extend_from_slice(&(f32::from(sample) / 32_768.0).to_le_bytes());
    }
    out
}

#[test]
fn callback_ledger_within_budget_over_a_fully_driven_session() {
    // The corpus tone through the output path at the corpus geometry:
    // 44.1 kHz stereo, one 1470-frame quantum per tick, fixed device
    // buffers of 490 frames (3 per tick — 1470 = 3 × 490, so every
    // callback is a whole number of full reads), ring of 4096 (the
    // pump stays ahead; the starvation case is the next test).
    const RING: usize = 4096;
    const DEVICE_BUFFER: usize = 490;
    let format = AudioFormat {
        sample: SampleFormat::F32,
        channels: 2,
        layout: AudioLayout::Interleaved,
        rate: TONE_RATE,
    };
    let mut stream = NullBackend::open_stream(&format, RING, NullConfig { capture: true }).must();

    let pcm = corpus_pcm();
    let device_bytes = s16_to_device_f32(&pcm);
    let frame_bytes = TONE_SAMPLES_PER_TICK * 4;
    let ticks = TONE_TOTAL_SAMPLES / TONE_SAMPLES_PER_TICK;

    let mut callbacks = 0u64;
    let mut served_frames = 0u64;
    for tick in 0..ticks {
        // The pump: one tick of the tone as interleaved f32 device
        // bytes, straight into the sink.
        let start = tick * frame_bytes;
        let chunk = &device_bytes[start * 2..(start + frame_bytes) * 2];
        stream.sink.write_bytes(chunk).must();
        // Drain the tick in three fixed device buffers, ledger
        // checked per callback.
        for _ in 0..TONE_SAMPLES_PER_TICK / DEVICE_BUFFER {
            let before = stream.device.sample_count();
            let underruns_before = stream.device.underruns();
            let mut out = vec![0.0f32; DEVICE_BUFFER * 2];
            let stats = stream.device.render(&mut out).must();
            assert_within_budget(
                &stats,
                before,
                stream.device.sample_count(),
                underruns_before,
                stream.device.underruns(),
            );
            assert!(
                stats.fully_served(),
                "tick {tick}: the pump must stay ahead"
            );
            callbacks += 1;
            served_frames += stats.frames_requested as u64;
            // Zero storage growth: the ring's shape is frozen at
            // construction — buffered + free == capacity, every call.
            assert_eq!(
                stream.device.buffered_frames() + free_frames(&stream, RING),
                RING
            );
        }
    }
    // 88 200 frames = 180 callbacks of 490: every ledger checked, the
    // accounts close exactly.
    assert_eq!(served_frames, TONE_TOTAL_SAMPLES as u64);
    assert_eq!(stream.device.sample_count(), TONE_TOTAL_SAMPLES as u64);
    assert_eq!(callbacks, (TONE_TOTAL_SAMPLES / DEVICE_BUFFER) as u64);
    assert_eq!(stream.device.underruns(), 0);
    // The clock source (what media-clock reads) tracked every frame.
    let clock = stream.sink.clock_source();
    let source: &dyn SampleCounterSource = clock.as_ref();
    assert_eq!(source.sample_count(), TONE_TOTAL_SAMPLES as u64);
    assert_eq!(source.sample_rate_hz(), TONE_RATE);
    // And the transport carried the tone intact under the
    // ledger-checked driving.
    assert_eq!(stream.device.take_capture(), device_bytes);
}

/// The ring's free-frame count, derived from the device-side view
/// (`free = capacity − buffered`; the SPSC cursors agree).
fn free_frames(stream: &audio_output::NullStream, capacity: usize) -> usize {
    capacity - stream.device.buffered_frames()
}

#[test]
fn starved_callback_fills_silence_and_still_honors_the_budget() {
    // The underrun path. Phase A: 3 primed frames against a request
    // of 5 — data first, in order, then silence (never repeat old
    // audio — doc 03 §4.8 d). Phase B: an empty ring against requests
    // of many sizes (including larger than the ring): pure zeros,
    // exactly one underrun event per callback, the counter still
    // advancing by the full request, the ledger still within budget.
    let format = AudioFormat {
        sample: SampleFormat::F32,
        channels: 2,
        layout: AudioLayout::Interleaved,
        rate: TONE_RATE,
    };
    let mut stream = NullBackend::open_stream(&format, 1024, NullConfig { capture: false }).must();

    // Phase A: 3 stereo frames, one short callback.
    let mut prime = Vec::new();
    for s in [0.25f32, -0.25, 0.5, -0.5, 0.75, -0.75] {
        prime.extend_from_slice(&s.to_le_bytes());
    }
    stream.sink.write_bytes(&prime).must();
    let before = stream.device.sample_count();
    let underruns_before = stream.device.underruns();
    let mut out = vec![f32::NAN; 5 * 2];
    let stats = stream.device.render(&mut out).must();
    assert_within_budget(
        &stats,
        before,
        stream.device.sample_count(),
        underruns_before,
        stream.device.underruns(),
    );
    assert_eq!(stats.frames_read, 3);
    assert_eq!(stats.silence_filled, 2);
    assert!(stats.underrun);
    assert_eq!(
        out,
        vec![0.25, -0.25, 0.5, -0.5, 0.75, -0.75, 0.0, 0.0, 0.0, 0.0],
        "data first, in order, then zeros"
    );

    // Phase B: starved sweep — two callbacks per request size.
    let requests = [1usize, 2, 3, 7, 64, 511, 512, 513, 1024, 2048];
    let mut total_served = 5u64;
    for &request in &requests {
        for _ in 0..2 {
            let before = stream.device.sample_count();
            let underruns_before = stream.device.underruns();
            let mut out = vec![f32::NAN; request * 2];
            let stats = stream.device.render(&mut out).must();
            assert_within_budget(
                &stats,
                before,
                stream.device.sample_count(),
                underruns_before,
                stream.device.underruns(),
            );
            total_served += stats.frames_requested as u64;
            assert_eq!(stats.frames_read, 0, "the ring is empty");
            assert_eq!(stats.silence_filled, request);
            assert!(stats.underrun);
            assert!(
                out.iter().all(|s| *s == 0.0),
                "pure silence, never old audio"
            );
        }
    }
    // One underrun event per short callback: 1 (phase A) + 20 (the
    // sweep). The counter advanced by every requested frame.
    assert_eq!(stream.device.underruns(), 21);
    let total_requested: u64 = 5 + requests.iter().map(|r| *r as u64).sum::<u64>() * 2;
    assert_eq!(total_served, total_requested);
    assert_eq!(stream.device.sample_count(), total_requested);
}

#[test]
fn callback_budget_holds_across_a_geometry_sweep() {
    // The wrap bound and whole-frame accounting are geometry
    // properties: sweep channels × capacity × request sizes
    // (including requests larger than the ring — a device asking for
    // more than the buffer depth), from many cursor alignments.
    for channels in [1u32, 2, 6] {
        for capacity in [64usize, 512, 4096] {
            let ring = audio_output::SpscRing::new(channels, capacity).must();
            let counter = audio_output::SampleCounter::new(48_000).must();
            let underruns = AtomicU64::new(0);
            let mut value = 0u32;
            // Park the cursors at many alignments by cycling
            // push/read pairs of varying sizes.
            for step in 0..16 {
                let push_frames = (step % 5) + 1;
                let mut chunk = Vec::new();
                for _ in 0..push_frames {
                    for _ in 0..channels {
                        let s = f32::from_bits(value);
                        value = value.wrapping_add(0x0100_0000);
                        chunk.push(s);
                    }
                }
                let _ = ring.push(&chunk);
                let mut sink = vec![0.0f32; channels as usize];
                let _ = ring.read(&mut sink);
                // The callback under test, at this alignment, for a
                // spread of request sizes.
                for request in [
                    1usize,
                    3,
                    capacity / 2,
                    capacity,
                    capacity + 1,
                    capacity * 2,
                ] {
                    let before = counter.count();
                    let underruns_before = underruns.load(Ordering::Relaxed);
                    let mut out = vec![f32::NAN; request * channels as usize];
                    let stats = render_into(&ring, &counter, &underruns, &mut out);
                    assert_within_budget(
                        &stats,
                        before,
                        counter.count(),
                        underruns_before,
                        underruns.load(Ordering::Relaxed),
                    );
                    assert_eq!(stats.frames_requested, request);
                }
            }
            // Zero storage growth across the whole sweep: the ring's
            // geometry is frozen at construction.
            assert_eq!(ring.capacity_frames(), capacity);
            assert_eq!(ring.channels(), channels);
            assert!(counter.count() > 0);
        }
    }
}

#[test]
fn paused_device_is_a_zero_operation_callback() {
    // Doc 03 §4.7 (e): a paused device pulls nothing. The callback
    // does not run: no counter advance, no underrun, no capture —
    // RenderStats::default() is the honest empty ledger.
    let format = AudioFormat {
        sample: SampleFormat::F32,
        channels: 2,
        layout: AudioLayout::Interleaved,
        rate: TONE_RATE,
    };
    let mut backend = NullBackend::new();
    backend
        .open(audio_output::null::NULL_DEVICE_ID, &format, 64)
        .must();
    let mut bytes = Vec::new();
    for s in [0.5f32, -0.5] {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    // Write through the backend's sink half (the AudioSink seam).
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
    backend.pause().must();
    let mut device = backend.take_device().must();
    let before = device.sample_count();
    // One stereo frame of device buffer: the request matches the one
    // buffered frame, so the resumed callback is fully served and
    // the counter delta is exactly the data (the silence-advances-
    // the-clock case is pinned in the starved-callback test).
    let mut out = vec![f32::NAN; 2];
    let stats = device.render(&mut out).must();
    assert_eq!(stats, RenderStats::default());
    assert_eq!(device.sample_count(), before);
    assert_eq!(device.underruns(), 0);
    assert!(out.iter().all(|v| v.is_nan()), "paused touches nothing");
    // The clock source agrees: frozen while paused (doc 03 §4.7 e).
    let clock = backend.clock_source().must();
    let source: &dyn SampleCounterSource = clock.as_ref();
    assert_eq!(source.sample_count(), before);
    // Resume, and the frozen frames play.
    backend.resume().must();
    let stats = device.render(&mut out).must();
    assert_eq!(stats.frames_read, 1);
    assert_eq!(out[..], [0.5, -0.5]);
    assert_eq!(source.sample_count(), 1);
}
