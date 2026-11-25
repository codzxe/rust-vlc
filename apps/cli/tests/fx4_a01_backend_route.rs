//! FX4-A01 — the backend-routing AC at the session seam: the
//! factory posture (both grammar values route the instrumented
//! null backend in this build), the FX3-A04 rate-mismatch
//! resampler's wiring (a sink clocked at another rate than the
//! track), and the FX3-A03 master chain reached through the
//! host-facing `audio_controls()` entry — volume, mute, balance and
//! the EQ observable in the samples the null device actually plays
//! (FX3-A05's ledger is the accounting cross-check).
//!
//! The composition under test is `open_audio_session[_for]` — the
//! production session constructor — driven directly (the sync
//! harness's discipline: virtual time only, the pump's render loop
//! replaced by the test's own renders, zero sleeps).

use std::f32::consts::PI;

use audio_engine::Volume;
use audio_output::cpal_linked;
use lumen_cli::pipeline_host::{
    open_audio_session, open_audio_session_for, AudioBackendPosture, AudioSession,
    RoutedAudioBackend, RENDER_QUANTUM,
};
use media_clock::clock::PlaybackClock;
use media_types::frame::{AudioFormat, AudioFrame, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::Stamp;
use media_types::GenerationId;
use testkit::Must;

/// The first vertical's rate.
const RATE: u32 = 44_100;

/// One interleaved S16 quantum at `rate` with every sample at the
/// constant `level` (s16 8192 → the exact f32 0.25 the pipeline
/// stages — the byte-exact lane's reference value).
fn constant_quantum(channels: u32, rate: u32, frames: usize, level: i16) -> AudioFrame {
    let mut bytes = Vec::with_capacity(frames * channels as usize * 2);
    for _ in 0..frames * channels as usize {
        bytes.extend_from_slice(&level.to_le_bytes());
    }
    AudioFrame {
        format: AudioFormat {
            sample: SampleFormat::S16,
            channels,
            layout: AudioLayout::Interleaved,
            rate,
        },
        samples: SharedBytes::from_vec(bytes),
        pts: Stamp::Missing,
        duration: None,
        generation: GenerationId::FIRST,
    }
}

/// `quanta` interleaved S16 frames of a globally-continuous `freq`
/// tone at `level` amplitude (both channels the same tone — the
/// chain's DSP is per-channel anyway; continuity across quanta is
/// what makes the settled-tail probe clean).
fn tone_quanta(
    channels: u32,
    rate: u32,
    quanta: usize,
    quantum: usize,
    freq: f32,
    level: f32,
) -> Vec<AudioFrame> {
    let mut frames = Vec::with_capacity(quanta);
    let mut sample: usize = 0;
    for _ in 0..quanta {
        let mut bytes = Vec::with_capacity(quantum * channels as usize * 2);
        for _ in 0..quantum {
            let value = ((2.0 * PI * freq * sample as f32 / rate as f32).sin() * level * 32_767.0)
                .round() as i16;
            for _ in 0..channels {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            sample += 1;
        }
        frames.push(AudioFrame {
            format: AudioFormat {
                sample: SampleFormat::S16,
                channels,
                layout: AudioLayout::Interleaved,
                rate,
            },
            samples: SharedBytes::from_vec(bytes),
            pts: Stamp::Missing,
            duration: None,
            generation: GenerationId::FIRST,
        });
    }
    frames
}

/// Render the whole ring out through the device half, collecting the
/// interleaved f32 samples the "device" played — the null backend's
/// observable discard (each render sized to the residue, so the
/// underrun silence-fill never contaminates the capture).
fn render_out(session: &mut AudioSession) -> Vec<f32> {
    let mut played = Vec::new();
    while session.limb.buffered_frames() > 0 {
        let take = session.limb.buffered_frames().min(RENDER_QUANTUM);
        let mut buffer = vec![0.0_f32; take * session.channels as usize];
        let _stats = session.device.render(&mut buffer).must();
        played.extend_from_slice(&buffer);
    }
    played
}

/// The RMS over the stream's settled tail (the last `tail` samples,
/// interleaved view — both channels ride the same tone).
fn settled_tail_rms(samples: &[f32], tail: usize) -> f32 {
    let window = &samples[samples.len() - tail..];
    let sum = window.iter().map(|sample| sample * sample).sum::<f32>();
    (sum / window.len() as f32).sqrt()
}

/// The factory posture routes the instrumented null backend in this
/// build — under both values of the grammar — and the driven session
/// still works end-to-end: the Auto default and the forced Null both
/// construct the driven halves, the staged bytes land in the ring
/// byte-exact (equal rates: no resampler seam), the device renders
/// them back out, and the FX3-A05 ledger counts exactly what played.
#[test]
fn fx4_a01_factory_posture_routes_the_instrumented_null_backend() {
    // The build truth the posture consults (FX3-A01's
    // admission-companion query): this build carries no real device
    // backend — apps/cli enables no audio-output feature.
    assert!(!cpal_linked(), "this build carries no real device backend");

    // The Auto default (open_audio_session's own posture)…
    let mut auto = open_audio_session(2, RATE, 0).must();
    assert_eq!(auto.routed, RoutedAudioBackend::Null);
    // …and the forced Null posture route the same backend.
    let mut forced = open_audio_session_for(2, RATE, RATE, 0, AudioBackendPosture::Null).must();
    assert_eq!(forced.routed, RoutedAudioBackend::Null);

    for session in [&mut auto, &mut forced] {
        session
            .limb
            .stage(&constant_quantum(2, RATE, RENDER_QUANTUM, 8192))
            .must();
        assert_eq!(session.limb.written_frames(), RENDER_QUANTUM as u64);
        let played = render_out(session);
        // The byte-exact lane: s16 8192 → f32 0.25 in every slot, both
        // channels — no resampler seam at equal rates, so the samples
        // the ring holds are the mix's own.
        assert_eq!(played.len(), RENDER_QUANTUM * 2);
        assert!(
            played.iter().all(|sample| *sample == 0.25),
            "the passthrough ring carries the mixed samples verbatim"
        );
        // FX3-A05's ledger: exactly what the device played, one
        // buffer per render, no silence.
        let stats = session.device.stats();
        assert_eq!(stats.frames(), RENDER_QUANTUM as u64);
        assert_eq!(stats.samples(), (RENDER_QUANTUM * 2) as u64);
        assert_eq!(stats.bytes(), (RENDER_QUANTUM * 2 * 4) as u64);
        assert_eq!(stats.silence_frames(), 0);
        assert_eq!(stats.data_frames(), RENDER_QUANTUM as u64);
        // The deterministic clock: 1470 frames at 44.1 kHz = exactly
        // one render quantum of media time (media-clock's
        // round-half-up ns conversion: 1_470_000_000_000 / 44_100).
        assert_eq!(session.device.sample_count(), RENDER_QUANTUM as u64);
        assert_eq!(session.clock.now().ticks, 33_333_333);
    }
}

/// The rate-mismatch seam: a sink clocked at 48 kHz under a 44.1 kHz
/// track wires FX3-A04's RateResampler into the chain at the session
/// seam — the mixed quantum crosses the sampling grids before the
/// write, the ring counts the exact emission-rule closed form, and
/// the audio-master clock's arithmetic rides the SINK's rate.
#[test]
fn fx4_a01_rate_mismatch_resampler_wires_the_output_clock() {
    let mut session = open_audio_session_for(2, RATE, 48_000, 0, AudioBackendPosture::Auto).must();
    assert_eq!(session.routed, RoutedAudioBackend::Null);
    session
        .limb
        .stage(&constant_quantum(2, RATE, RENDER_QUANTUM, 8192))
        .must();
    // FX3-A04's closed form for a finished 1470-sample stream at the
    // reduced ratio 147/160 (input per output): ⌊(1470−1)·160/147⌋+1
    // = 1599 sink frames (the naive duration ratio says 1600 — the
    // exact count is the oracle).
    assert_eq!(session.limb.written_frames(), 1599);
    let played = render_out(&mut session);
    assert_eq!(
        played.len(),
        1599 * 2,
        "every converted frame reached the ring"
    );
    // The kernel's exactness on a constant quantum: every interpolated
    // slot is the constant itself (a + (b − a)·weight with a == b).
    assert!(
        played.iter().all(|sample| *sample == 0.25),
        "the linear kernel passes a constant bit-exactly"
    );
    // The audio-master clock rides the SINK's counter: 1599 frames at
    // 48 kHz = exactly 33.3125 ms of media time (1_599_000_000_000 /
    // 48_000 is exact — no rounding to pin).
    assert_eq!(session.device.sample_count(), 1599);
    assert_eq!(session.clock.now().ticks, 33_312_500);
}

/// The frozen contract: `audio_controls()` on the host-facing audio
/// session types yields live control of the FX3-A03 master chain —
/// the engine's own [`AudioControls`] handed out mutably (the shape
/// FX4-A02's host verbs and FX4-A05's session tests drive) — volume,
/// mute and balance observable in the samples the device plays, with
/// the session-level and limb-level entries answering one live chain.
#[test]
fn fx4_a01_audio_controls_volume_mute_balance_reach_the_ring() {
    let mut session = open_audio_session(2, RATE, 0).must();
    let quantum = || constant_quantum(2, RATE, RENDER_QUANTUM, 8192);

    // Volume: set through the SESSION handle, read through the LIMB's
    // (one live chain, two reachable entries).
    session
        .audio_controls()
        .set_volume(Volume::from_linear(0.5).must());
    assert_eq!(
        session.limb.audio_controls().volume(),
        Volume::from_linear(0.5).must()
    );
    session.limb.stage(&quantum()).must();
    let halved = render_out(&mut session);
    assert!(
        halved.iter().all(|sample| *sample == 0.125),
        "0.25 × 0.5 in every slot"
    );

    // Mute zeroes every mixed quantum (silence, not a skip — the frame
    // count is conserved); unmute restores the prior bytes.
    session.audio_controls().set_muted(true);
    assert!(session.limb.audio_controls().muted());
    session.limb.stage(&quantum()).must();
    let muted = render_out(&mut session);
    assert_eq!(muted.len(), RENDER_QUANTUM * 2);
    assert!(muted.iter().all(|sample| *sample == 0.0));
    session.audio_controls().set_muted(false);
    session.limb.stage(&quantum()).must();
    let unmuted = render_out(&mut session);
    assert_eq!(unmuted, halved, "unmute restores the signal bit-exactly");

    // Balance (at the identity volume restored): hard left keeps L
    // bit-exactly and silences R (the constant-power law's exact
    // (1.0, 0.0) gains at p = −1); clear restores the identity split.
    session.audio_controls().set_volume(Volume::UNITY);
    session.audio_controls().set_balance(-1.0).must();
    assert_eq!(session.limb.audio_controls().balance(), Some(-1.0));
    session.limb.stage(&quantum()).must();
    let hard_left = render_out(&mut session);
    for pair in hard_left.chunks_exact(2) {
        assert_eq!(pair[0], 0.25, "L kept");
        assert_eq!(pair[1], 0.0, "R silenced");
    }
    session.audio_controls().clear_balance();
    assert_eq!(session.limb.audio_controls().balance(), None);

    // The raw `&mut` hand-off's one moved gate (the seam doc's honest
    // cost): a MONO session's `set_balance` takes the `Ok` — the
    // chain cannot see the session's channel count — and the stereo
    // gate fires at process time instead: the next `stage` refuses
    // typed (`audio.effects_balance_channels`), never a silent
    // mislabel. The pipeline's own surface pre-validates at set time;
    // this seam answers loudly at mix time.
    let mut mono = open_audio_session(1, RATE, 0).must();
    mono.audio_controls().set_balance(0.0).must();
    let error = mono
        .limb
        .stage(&constant_quantum(1, RATE, 4, 8192))
        .err()
        .must();
    assert_eq!(error.context.code, "audio.effects_balance_channels");
}

/// The EQ is real DSP the session seam reaches: a −12 dB mid cut on a
/// 1 kHz tone (the band's exact center) settles at 10^(−12/20) of the
/// same input's uncontrolled output — the reference is the same
/// session shape uncontrolled, so the s16 quantization cancels in the
/// ratio and what remains is the EQ's response alone.
#[test]
fn fx4_a01_eq_attenuates_the_band_center_through_the_session_seam() {
    let quanta = 8;
    let tail = 2 * RENDER_QUANTUM; // the settled tail (the last two quanta)

    let mut plain = open_audio_session(2, RATE, 0).must();
    let mut filtered = open_audio_session(2, RATE, 0).must();
    filtered
        .audio_controls()
        .set_eq(0.0, -12.0, 0.0, Volume::UNITY)
        .must();
    assert_eq!(
        filtered.audio_controls().eq_bands(),
        Some((0.0, -12.0, 0.0, Volume::UNITY))
    );

    for frame in tone_quanta(2, RATE, quanta, RENDER_QUANTUM, 1000.0, 0.5) {
        plain.limb.stage(&frame).must();
        filtered.limb.stage(&frame).must();
    }
    let reference = render_out(&mut plain);
    let shaped = render_out(&mut filtered);
    for (name, output) in [("reference", &reference), ("filtered", &shaped)] {
        assert_eq!(
            output.len(),
            quanta * RENDER_QUANTUM * 2,
            "{name}: every quantum crossed the ring"
        );
    }

    let ratio = settled_tail_rms(&shaped, tail) / settled_tail_rms(&reference, tail);
    let want = 10.0_f32.powf(-12.0 / 20.0);
    assert!(
        ((ratio - want) / want).abs() < 0.05,
        "the mid cut settles at {want} ±5%, got {ratio}"
    );
}
