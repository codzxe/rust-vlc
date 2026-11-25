//! FX3-A05 — the null backend's discard, made observable (the FX4
//! end-to-end seam). The audit could prove "decodes into null sinks"
//! and nothing more; from this wave the sink's consumption is a
//! ledger ([`audio_output::SinkStats`]): every served device buffer
//! counts its samples, frames and bytes — data and underrun silence
//! split apart — readable from either session half or the backend,
//! from outside this crate, with the audio path otherwise untouched
//! (it still discards; the no-blocking contract's callback core is
//! unchanged; the ledger is recorded driver-side, after the callback,
//! where capture already lived).
//!
//! Oracles, one test each:
//! - **exact counts:** N samples pushed through the ring and rendered
//!   → the ledger reads exactly N samples, N/channels frames,
//!   N×4 bytes, and a paused device records nothing;
//! - **the pre-existing accounting is unaffected:** underrun events,
//!   the sample counter (data + silence), backpressure refusals;
//! - **the SPSC discipline under real concurrency:** producer and
//!   consumer threads on the two halves, and the ledger agrees with
//!   the threads' own local counts exactly, whatever the interleaving
//!   (the ring is `Sync` by construction — the documented contract,
//!   exercised here from the test side only; the crate itself still
//!   spawns no threads).

use audio_engine::AudioSink;
use audio_engine::buffer::AudioBuffer;
use audio_output::null::NULL_DEVICE_ID;
use audio_output::{AudioBackend, NullBackend, NullConfig, RenderStats};
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::Stamp;
use testkit::Must;

const RATE: u32 = 44_100;

fn f32_format(channels: u32, rate: u32) -> AudioFormat {
    AudioFormat {
        sample: SampleFormat::F32,
        channels,
        layout: AudioLayout::Interleaved,
        rate,
    }
}

fn buffer_of(format: AudioFormat, samples: &[f32]) -> AudioBuffer {
    let mut data = Vec::new();
    for s in samples {
        data.extend_from_slice(&s.to_le_bytes());
    }
    AudioBuffer {
        format,
        data: SharedBytes::from_vec(data),
        pts: Stamp::Missing,
    }
}

#[test]
fn fx3_a05_stats_report_exactly_the_pushed_samples() {
    // N interleaved f32 samples through the ring, rendered back out
    // in one device buffer: the ledger reads exactly N — samples,
    // frames, bytes, one buffer, zero silence, all data. N is the
    // 1470-frame corpus tick shape (stereo).
    let format = f32_format(2, RATE);
    let mut stream = NullBackend::open_stream(&format, 2048, NullConfig { capture: false }).must();
    const N: usize = 2_940;
    let mut bytes = Vec::with_capacity(N * 4);
    for i in 0..N {
        let v = ((i % 251) as f32 - 125.0) / 128.0;
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    stream.sink.write_bytes(&bytes).must();
    let mut out = vec![0.0f32; N];
    let call = stream.device.render(&mut out).must();
    assert!(call.fully_served());

    let stats = stream.device.stats();
    assert_eq!(stats.buffers(), 1);
    assert_eq!(stats.frames(), (N / 2) as u64);
    assert_eq!(stats.samples(), N as u64);
    assert_eq!(stats.bytes(), (N * 4) as u64);
    assert_eq!(stats.silence_frames(), 0);
    assert_eq!(stats.data_frames(), (N / 2) as u64);
    // One shared ledger, two half views (the counter's pattern) — and
    // the ledger agrees with the pre-existing accounting: same
    // consumed instants, no underruns, everything written consumed.
    assert_eq!(
        stream.sink.stats().samples(),
        stream.device.stats().samples()
    );
    assert_eq!(stream.sink.stats().frames(), stream.device.sample_count());
    assert_eq!(stream.device.underruns(), 0);
    assert_eq!(stream.sink.written_frames(), (N / 2) as u64);

    // Paused renders consume nothing: the ledger freezes with the
    // counter (doc 03 §4.7 e). Trait mode — the backend owns pause,
    // the driver is taken out, and the ledger stays readable from the
    // backend afterwards (shared session state, not driver state).
    let mut backend = NullBackend::new();
    assert!(
        backend.stats().is_none(),
        "no session, no ledger — honest state"
    );
    backend.open(NULL_DEVICE_ID, &format, 8).must();
    backend.write(&buffer_of(format, &[0.5, -0.5])).must();
    backend.pause().must();
    let mut device = backend.take_device().must();
    let mut out = vec![f32::NAN; 2];
    let call = device.render(&mut out).must();
    assert_eq!(call, RenderStats::default());
    assert_eq!(backend.stats().must().buffers(), 0);
    assert_eq!(backend.stats().must().samples(), 0);
    backend.resume().must();
    device.render(&mut out).must();
    assert_eq!(backend.stats().must().buffers(), 1);
    assert_eq!(backend.stats().must().frames(), 1);
    assert_eq!(backend.stats().must().samples(), 2);
    assert_eq!(backend.stats().must().bytes(), 8);
    assert_eq!(device.sample_count(), 1);
}

#[test]
fn fx3_a05_underrun_accounting_is_unaffected_by_stats() {
    // One data frame, then a starved callback (1 data + 2 silence):
    // the underrun accounting reads exactly as it did before the
    // ledger existed — one event per starved callback, the counter
    // counting data + silence — while the ledger splits the same
    // consumption into data vs silence (the null-backend audit's
    // F-002 arithmetic, made a field). Refused writes count nothing.
    let format = f32_format(1, RATE);
    let mut stream = NullBackend::open_stream(&format, 8, NullConfig { capture: false }).must();
    stream.sink.write_bytes(&0.5f32.to_le_bytes()).must();
    // A refused write never touches any accounting.
    let err = stream.sink.write_bytes(&[0, 0, 0]).err().must();
    assert_eq!(err.context.code, "audio.partial_frame");
    let mut big = Vec::new();
    for _ in 0..8 {
        big.extend_from_slice(&0.0f32.to_le_bytes());
    }
    let err = stream.sink.write_bytes(&big).err().must();
    assert_eq!(err.context.code, "audio.ring_full");
    assert_eq!(stream.device.stats().samples(), 0, "no render yet");

    let mut out = vec![f32::NAN; 3];
    let call = stream.device.render(&mut out).must();
    assert!(call.underrun);
    assert_eq!(call.silence_filled, 2);
    assert_eq!(out, vec![0.5, 0.0, 0.0]);

    let stats = stream.device.stats();
    assert_eq!(stats.buffers(), 1);
    assert_eq!(stats.frames(), 3);
    assert_eq!(stats.samples(), 3);
    assert_eq!(stats.bytes(), 12);
    assert_eq!(stats.silence_frames(), 2);
    assert_eq!(stats.data_frames(), 1);
    // The pre-existing accounts, unchanged by the instrumentation:
    assert_eq!(stream.device.underruns(), 1);
    assert_eq!(stream.sink.underruns(), 1);
    assert_eq!(
        stream.device.sample_count(),
        3,
        "the counter still counts data + silence"
    );
    assert_eq!(stream.device.sample_count(), stats.frames());

    // A second, fully-starved callback: one more event, all silence —
    // the data count never inflates.
    let mut out = vec![f32::NAN; 2];
    let call = stream.device.render(&mut out).must();
    assert!(call.underrun);
    assert_eq!(call.silence_filled, 2);
    assert_eq!(stream.device.underruns(), 2);
    let stats = stream.device.stats();
    assert_eq!(stats.buffers(), 2);
    assert_eq!(stats.frames(), 5);
    assert_eq!(stats.samples(), 5);
    assert_eq!(stats.silence_frames(), 4);
    assert_eq!(stats.data_frames(), 1);
    assert_eq!(stream.device.sample_count(), 5);
}

#[test]
fn fx3_a05_concurrent_spsc_producer_consumer_counts_exactly() {
    // The SPSC discipline under real concurrency: the sink half on the
    // producer thread (writes, backpressure retries on a full ring),
    // the device half on the consumer thread (fixed-quanta renders) —
    // the split the module docs prescribe. Whatever the interleaving,
    // the shared ledger must agree with the threads' own local counts
    // exactly: every served buffer counted once (no lost atomic
    // updates), every underrun event counted once, all W written data
    // frames conserved into `data_frames`, silence never mixed in.
    let format = f32_format(2, RATE);
    let mut stream = NullBackend::open_stream(&format, 64, NullConfig { capture: false }).must();
    // Frames written in total (a multiple of CHUNK).
    const W: u64 = 4_096;
    // Frames per producer write (ring capacity 64 → real wrap churn).
    const CHUNK: usize = 16;
    // Frames per consumer render.
    const QUANTUM: usize = 16;

    let mut sink = stream.sink;
    let mut device = stream.device;
    let mut bytes = Vec::with_capacity(CHUNK * 2 * 4);
    for i in 0..CHUNK {
        let v = i as f32 * 0.25 - 2.0;
        bytes.extend_from_slice(&v.to_le_bytes());
        bytes.extend_from_slice(&(-v).to_le_bytes());
    }

    let producer = std::thread::spawn(move || {
        let mut written = 0u64;
        while written < W {
            // Backpressure: a full ring refuses the whole write; the
            // pump's shape is retry (never blocks).
            if sink.write_bytes(&bytes).is_ok() {
                written += CHUNK as u64;
            } else {
                std::thread::yield_now();
            }
        }
        (sink, written)
    });

    let consumer = std::thread::spawn(move || {
        let mut out = vec![0.0f32; QUANTUM * 2];
        let mut calls = 0u64;
        let mut underrun_calls = 0u64;
        let mut silence = 0u64;
        // Drive until every written data frame is consumed as data:
        // the producer's writes are monotonic and all-or-nothing, so
        // `data_frames == W` exactly when the ring's data is drained.
        let mut spins = 0u64;
        while device.stats().data_frames() < W {
            let call = device.render(&mut out).must();
            calls += 1;
            if call.underrun {
                underrun_calls += 1;
            }
            silence += call.silence_filled as u64;
            spins += 1;
            assert!(spins < 10_000_000, "consumer never drained data == {W}");
        }
        (device, calls, underrun_calls, silence)
    });

    let joined = producer.join();
    let (sink, written) = match joined {
        Ok(counts) => counts,
        Err(_) => unreachable!("producer thread panicked"),
    };
    let joined = consumer.join();
    let (device, calls, underrun_calls, silence) = match joined {
        Ok(counts) => counts,
        Err(_) => unreachable!("consumer thread panicked"),
    };
    assert_eq!(written, W);

    let stats = device.stats();
    assert_eq!(
        stats.buffers(),
        calls,
        "every served buffer counted exactly once"
    );
    assert_eq!(stats.frames(), calls * QUANTUM as u64);
    assert_eq!(stats.samples(), stats.frames() * 2);
    assert_eq!(stats.bytes(), stats.samples() * 4);
    assert_eq!(
        stats.silence_frames(),
        silence,
        "the ledger's silence is the per-callback sum"
    );
    assert_eq!(
        stats.data_frames(),
        W,
        "data conservation: written == consumed as data"
    );
    assert_eq!(stats.frames(), stats.data_frames() + stats.silence_frames());
    // The pre-existing accounts under the same concurrency —
    // unaffected by the instrumentation:
    assert_eq!(
        device.underruns(),
        underrun_calls,
        "one underrun event per starved callback, no lost updates"
    );
    assert_eq!(
        device.sample_count(),
        stats.frames(),
        "counter and ledger agree"
    );
    assert_eq!(device.buffered_frames(), 0, "the ring is drained");
    // The two halves read one ledger (the same shared session state).
    assert_eq!(sink.stats().samples(), stats.samples());
    assert_eq!(sink.stats().data_frames(), W);
    assert_eq!(sink.written_frames(), W);
    assert_eq!(sink.underruns(), device.underruns());
}
