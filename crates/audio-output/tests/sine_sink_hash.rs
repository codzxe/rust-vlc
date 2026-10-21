//! AC suite 1 — the **sine fixture sink-buffer hash** (card 2.C3):
//! "deterministic PCM (fixtures' NCO tones or a spec'd sine) through
//! engine→output sink buffers, hashed, byte-stable (double-run
//! proof)".
//!
//! The path under test is the full audio v0 vertical:
//!
//! ```text
//! spec'd NCO s16le PCM          (== the committed corpus tone —
//!  ↕ digest-pinned                 audio_stream_sha256, both parents)
//! audio-engine AudioPipeline    (stage A s16→f32, unity volume,
//!  ↕                                identity map, single-track mix,
//!                                   stage D f32 interleave)
//! audio-output NullSink         (the AudioSink seam → SPSC ring)
//!  ↕
//! NullDevice renders            (the real-time callback, driven one
//!  ↕                                1470-frame device buffer/tick)
//! captured device output        → SHA-256, hashed and compared
//! ```
//!
//! Three independent anchors make the hash *meaningful*, not merely
//! stable:
//!
//! 1. **Tone provenance** — the regenerated PCM's SHA-256 equals the
//!    expectation TOML's recorded `audio_stream_sha256` (both
//!    parents): the test's sine IS the corpus's sine.
//! 2. **Bit-exact value truth** — a single track at unity volume is
//!    value-transparent through the engine (`v/32768` in, `v/32768`
//!    out — pinned by audio-engine's own unity test), so the captured
//!    bytes must equal the pinned f32 conversion of the PCM, byte for
//!    byte. The hash is therefore the digest of a known-good signal,
//!    not just of "whatever the code did".
//! 3. **Double-run stability** — two fully independent runs (fresh
//!    pipeline, fresh null stream) produce byte-identical captures
//!    and identical digests: the deterministic-mix argument
//!    (ascending-track accumulation, doc 03 §4.8 b) proven end to
//!    end.
//!
//! Clock truth is asserted on the way out: the device sample counter
//! — the media-clock truth source — reads exactly the 88 200
//! sample-instants the corpus defines, through the same
//! `SampleCounterSource` handle a `MediaClock` would hold.

mod common;

use audio_engine::{AudioPipeline, AudioSink, PipelineSpec, Volume};
use audio_output::{NullBackend, NullConfig};
use common::{
    TONE_RATE, TONE_SAMPLES_PER_TICK, TONE_TOTAL_SAMPLES, corpus_pcm, fixture_bytes,
    s16_stream_to_f32_bytes, sha256_hex, toml_string,
};
use media_types::frame::{AudioFormat, AudioFrame, AudioLayout, SampleFormat};
use media_types::ids::GenerationId;
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, Timestamp};
use testkit::Must;

/// One full engine→output run over the corpus tone: 60 ticks of 1470
/// frames, each processed through a fresh single-track pipeline at
/// unity volume, written to the null sink, and rendered by the null
/// device one device-buffer per tick. Returns everything the ACs
/// assert on.
fn run_tone_through_output(volume: Volume) -> ToneRun {
    let spec = PipelineSpec {
        rate: TONE_RATE,
        channels: 2,
        tracks: 1,
    };
    let mut pipeline = AudioPipeline::new(spec).must();
    pipeline.set_volume(0, volume).must();
    let format = pipeline.device_format();

    // The output ring holds a few ticks more than one device buffer:
    // the pump never hits backpressure in this run (the AC's underrun
    // count asserts exactly that).
    let buffer_frames = 8192;
    let mut stream =
        NullBackend::open_stream(&format, buffer_frames, NullConfig { capture: true }).must();

    let pcm = corpus_pcm();
    let decoded_format = AudioFormat {
        sample: SampleFormat::S16,
        channels: 2,
        layout: AudioLayout::Interleaved,
        rate: TONE_RATE,
    };
    let frame_bytes = TONE_SAMPLES_PER_TICK * 4;
    let ticks = TONE_TOTAL_SAMPLES / TONE_SAMPLES_PER_TICK;

    for tick in 0..ticks {
        let start = tick * frame_bytes;
        let chunk = &pcm[start..start + frame_bytes];
        let frame = AudioFrame {
            format: decoded_format,
            samples: SharedBytes::from_vec(chunk.to_vec()),
            pts: Stamp::Present(
                Timestamp::from_parts((tick * TONE_SAMPLES_PER_TICK) as i64, 1, TONE_RATE).must(),
            ),
            duration: None,
            generation: GenerationId::FIRST,
        };
        pipeline.process(0, &frame).must();
        assert!(pipeline.mix().must(), "tick {tick}: a quantum must mix");
        let buffer = pipeline.take_output().must();
        assert_eq!(buffer.frames().must() as usize, TONE_SAMPLES_PER_TICK);
        stream.sink.write(&buffer).must();

        // The device drains exactly one tick per callback period —
        // the shape an OS device thread would drive.
        let mut out = vec![0.0f32; TONE_SAMPLES_PER_TICK * 2];
        let stats = stream.device.render(&mut out).must();
        assert!(
            stats.fully_served(),
            "tick {tick}: underran — pump too slow"
        );
        assert_eq!(stats.silence_filled, 0);
    }

    let capture = stream.device.take_capture();
    let clock = stream.sink.clock_source();
    ToneRun {
        hash: sha256_hex(&capture),
        capture,
        sample_count: stream.device.sample_count(),
        underruns: stream.device.underruns(),
        clock_count: clock.sample_count(),
        clock_rate: clock.sample_rate_hz(),
        pcm_hash: sha256_hex(&pcm),
    }
}

/// Everything one run observes.
struct ToneRun {
    hash: String,
    capture: Vec<u8>,
    sample_count: u64,
    underruns: u64,
    clock_count: u64,
    clock_rate: u32,
    pcm_hash: String,
}

#[test]
fn specd_sine_is_the_committed_corpus_tone() {
    // Tone provenance: the spec'd NCO reconstruction (public math,
    // tests/common) is the committed corpus's PCM — pinned by the
    // recorded audio_stream_sha256 of BOTH parents (the AVI parent
    // and the MP4 sowt parent carry the same tone plan). This also
    // pins this suite's SHA-256 implementation against a committed
    // digest.
    let pcm = corpus_pcm();
    assert_eq!(pcm.len(), TONE_TOTAL_SAMPLES * 4, "88 200 stereo s16 pairs");
    let hash = sha256_hex(&pcm);
    for expectation in [
        "expectations/valid/avi_mjpeg_pcm.avi.toml",
        "expectations/valid/mp4_mjpeg_pcm.mp4.toml",
    ] {
        let toml = String::from_utf8(fixture_bytes(expectation).must()).must();
        let recorded = toml_string(&toml, "audio_stream_sha256").must();
        assert_eq!(
            hash, recorded,
            "{expectation}: tone plan drifted from the corpus"
        );
    }
}

#[test]
fn sine_fixture_sink_buffer_hash_is_byte_stable_across_runs() {
    // The AC's double-run proof, anchored on both value truth and
    // stability.
    let first = run_tone_through_output(Volume::UNITY);
    let second = run_tone_through_output(Volume::UNITY);

    // (1) Tone provenance held in both runs (redundant with the
    // dedicated test, cheap here, and it makes this test's evidence
    // self-contained).
    let expected_stream = s16_stream_to_f32_bytes(&corpus_pcm());
    assert_eq!(first.pcm_hash, second.pcm_hash);

    // (2) Value truth: unity single-track is value-transparent end to
    // end — the captured device output IS the pinned f32 conversion
    // of the corpus PCM, byte for byte.
    assert_eq!(first.capture.len(), TONE_TOTAL_SAMPLES * 2 * 4);
    assert_eq!(
        first.capture, expected_stream,
        "engine/output must be value-transparent at unity"
    );

    // (3) Byte-stability: two independent runs, identical bytes and
    // digest.
    assert_eq!(
        first.capture, second.capture,
        "double run must be byte-identical"
    );
    assert_eq!(first.hash, second.hash);
    assert_eq!(first.hash, sha256_hex(&expected_stream));

    // (4) Clock truth: the device consumed exactly the corpus's
    // sample-instants, the underrun count is zero (the pump kept
    // ahead), and the SampleCounterSource view media-clock holds
    // agrees — same counter, same rate.
    assert_eq!(first.sample_count, TONE_TOTAL_SAMPLES as u64);
    assert_eq!(first.clock_count, TONE_TOTAL_SAMPLES as u64);
    assert_eq!(first.clock_rate, TONE_RATE);
    assert_eq!(first.underruns, 0, "a fully-driven stream never underruns");
    assert_eq!(second.sample_count, TONE_TOTAL_SAMPLES as u64);
    assert_eq!(second.underruns, 0);

    // (5) Sensitivity: the digest is a real discriminator — half
    // volume changes the bytes (and would change the hash). This is
    // what makes assertions (2)-(4) non-vacuous.
    let attenuated = run_tone_through_output(Volume::from_linear(0.5).must());
    assert_ne!(
        attenuated.hash, first.hash,
        "the hash must track the samples"
    );
    assert_eq!(attenuated.sample_count, TONE_TOTAL_SAMPLES as u64);
}

#[test]
fn sink_buffer_hash_survives_ring_wrap_refusal_and_pacing() {
    // Same AC, adversarial geometry: a ring of 2048 frames (under two
    // ticks) against a pump that writes one quantum AHEAD of the
    // device — every iteration the ring refuses the write (the typed
    // `audio.ring_full` backpressure signal, MP §8), the device
    // drains two fixed 735-frame buffers to make room, and the write
    // retries. Every render fully served, the wrap cursor landing at
    // non-round offsets every tick — and the captured bytes must be
    // IDENTICAL to the aligned run: ring geometry and pacing are
    // transport, not content.
    let spec = PipelineSpec {
        rate: TONE_RATE,
        channels: 2,
        tracks: 1,
    };
    let mut pipeline = AudioPipeline::new(spec).must();
    let format = pipeline.device_format();
    let mut stream = NullBackend::open_stream(&format, 2048, NullConfig { capture: true }).must();

    let pcm = corpus_pcm();
    let decoded_format = AudioFormat {
        sample: SampleFormat::S16,
        channels: 2,
        layout: AudioLayout::Interleaved,
        rate: TONE_RATE,
    };
    let frame_bytes = TONE_SAMPLES_PER_TICK * 4;
    let ticks = TONE_TOTAL_SAMPLES / TONE_SAMPLES_PER_TICK;
    const DEVICE_BUFFER: usize = 735; // half a tick — fixed, as a real device

    let stage = |pipeline: &mut AudioPipeline, tick: usize| {
        let start = tick * frame_bytes;
        let frame = AudioFrame {
            format: decoded_format,
            samples: SharedBytes::from_vec(pcm[start..start + frame_bytes].to_vec()),
            pts: Stamp::Present(
                Timestamp::from_parts((tick * TONE_SAMPLES_PER_TICK) as i64, 1, TONE_RATE).must(),
            ),
            duration: None,
            generation: GenerationId::FIRST,
        };
        pipeline.process(0, &frame).must();
        assert!(pipeline.mix().must());
        pipeline.take_output().must()
    };
    fn render(stream: &mut audio_output::NullStream) {
        let mut out = vec![0.0f32; DEVICE_BUFFER * 2];
        let stats = stream.device.render(&mut out).must();
        assert!(stats.fully_served(), "the pump must keep the device fed");
        assert_eq!(stats.silence_filled, 0);
    }

    /// Write with backpressure: on the typed `audio.ring_full`
    /// refusal, drain one device buffer and retry; any other error is
    /// returned (the caller surfaces it via `must()` — the sanctioned
    /// vocabulary, no raw panics).
    fn write_with_backpressure(
        stream: &mut audio_output::NullStream,
        buffer: audio_engine::buffer::AudioBuffer,
        refusals: &mut usize,
    ) -> Result<(), media_types::EngineError> {
        let mut pending = Some(buffer);
        while let Some(buffer) = pending.take() {
            match stream.sink.write(&buffer) {
                Ok(()) => return Ok(()),
                Err(err) if err.context.code == "audio.ring_full" => {
                    *refusals += 1;
                    render(stream);
                    pending = Some(buffer);
                }
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }

    // Prime: quantum 0 in, before the device has pulled anything.
    let first = stage(&mut pipeline, 0);
    stream.sink.write(&first).must();
    // Steady state: stage the next quantum, run into the refusal,
    // drain two device buffers, retry the write.
    for tick in 1..ticks {
        let buffer = stage(&mut pipeline, tick);
        let mut refusals = 0;
        write_with_backpressure(&mut stream, buffer, &mut refusals).must();
        assert_eq!(
            refusals, 2,
            "the 2048-ring must refuse exactly twice per tick"
        );
    }
    // Flush the final quantum still buffered.
    render(&mut stream);
    render(&mut stream);
    assert_eq!(stream.device.buffered_frames(), 0);

    let capture = stream.device.take_capture();
    assert_eq!(
        capture,
        s16_stream_to_f32_bytes(&pcm),
        "wrap/refusal/pacing geometry must not alter content"
    );
    assert_eq!(stream.device.underruns(), 0);
    assert_eq!(stream.device.sample_count(), TONE_TOTAL_SAMPLES as u64);
}
