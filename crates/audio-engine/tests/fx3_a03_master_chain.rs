//! FX3-A03 — the runtime master chain through the real pipeline
//! (wave-07's dead audio-controls surface, W07-A02/A06/A07/A08): the
//! audit proved zero production callers of `set_volume`/`Gain`/
//! `Eq3Band`, a mixer permanently at `UNITY`, and RBJ biquads wired
//! to nothing. These tests drive the activated surface end to end —
//! `process → mix → take_output` with controls set **between
//! quanta on the same pipeline object** (the runtime seam, never a
//! rebuild), plus the zero-behavior-change acceptance bar for the
//! uncontrolled default.
//!
//! Tone material follows the crate's own deterministic-generator
//! convention (integer-period sines + settled-tail RMS/peak probes —
//! eq.rs's discipline). The assignment's premise of "testkit NCO
//! tone plans" does not hold: testkit carries no signal generators
//! (its surface is Must-asserts, allocation counting, fixtures), so
//! the local deterministic generators here mirror the in-crate ones.

use audio_engine::{AudioBuffer, AudioPipeline, PipelineSpec, Volume};
use media_types::frame::{AudioFormat, AudioFrame, AudioLayout, SampleFormat};
use media_types::ids::GenerationId;
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, Timestamp};
use testkit::Must;

const RATE: u32 = 44_100;
const EQ_RATE: u32 = 48_000;

fn stereo_s16_frame(pairs: &[(i16, i16)], pts_ticks: i64, rate: u32) -> AudioFrame {
    let mut bytes = Vec::new();
    for (l, r) in pairs {
        bytes.extend_from_slice(&l.to_le_bytes());
        bytes.extend_from_slice(&r.to_le_bytes());
    }
    AudioFrame {
        format: AudioFormat {
            sample: SampleFormat::S16,
            channels: 2,
            layout: AudioLayout::Interleaved,
            rate,
        },
        samples: SharedBytes::from_vec(bytes),
        pts: Stamp::Present(Timestamp::from_parts(pts_ticks, 1, rate).must()),
        duration: None,
        generation: GenerationId::FIRST,
    }
}

/// An integer-period stereo tone pair (deterministic f32 sines
/// quantized to s16) — `n` chosen so both the quantum and the settled
/// tail (second half) hold whole periods, keeping RMS probes
/// phase-exact.
fn tone_pairs(n: usize, freq: f32, amp_l: f32, amp_r: f32, rate: u32) -> Vec<(i16, i16)> {
    (0..n)
        .map(|i| {
            let phase = 2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32;
            (
                (amp_l * phase.sin() * 32_767.0) as i16,
                (amp_r * phase.sin() * 32_767.0) as i16,
            )
        })
        .collect()
}

fn f32_pairs(buffer: &AudioBuffer) -> Vec<(f32, f32)> {
    buffer
        .data
        .as_slice()
        .as_chunks::<8>()
        .0
        .iter()
        .map(|c| {
            (
                f32::from_le_bytes([c[0], c[1], c[2], c[3]]),
                f32::from_le_bytes([c[4], c[5], c[6], c[7]]),
            )
        })
        .collect()
}

fn left_channel(buffer: &AudioBuffer) -> Vec<f32> {
    f32_pairs(buffer).into_iter().map(|(l, _)| l).collect()
}

fn right_channel(buffer: &AudioBuffer) -> Vec<f32> {
    f32_pairs(buffer).into_iter().map(|(_, r)| r).collect()
}

/// RMS of the settled tail (transients discarded — eq.rs's probe).
fn settled_rms(samples: &[f32]) -> f32 {
    let tail = &samples[samples.len() / 2..];
    (tail.iter().map(|s| s * s).sum::<f32>() / tail.len() as f32).sqrt()
}

/// One full quantum through the live pipeline (the pump's step).
fn run_quantum(pipeline: &mut AudioPipeline, frame: &AudioFrame) -> AudioBuffer {
    pipeline.process(0, frame).must();
    assert!(pipeline.mix().must());
    pipeline.take_output().must()
}

fn stereo_pipeline(rate: u32) -> AudioPipeline {
    AudioPipeline::new(PipelineSpec {
        rate,
        channels: 2,
        tracks: 1,
    })
    .must()
}

#[test]
fn fx3_a03_volume_changes_output_amplitude() {
    // The volume knob, live: unity is bit-exact, 0.5 halves every
    // sample exactly, and returning to UNITY restores the bit stream.
    let mut pipeline = stereo_pipeline(RATE);
    let frame = stereo_s16_frame(&[(16384, -16384)], 0, RATE);
    let unity = run_quantum(&mut pipeline, &frame);
    assert_eq!(f32_pairs(&unity), vec![(0.5, -0.5)]);

    // The runtime seam — same pipeline object, control between quanta.
    pipeline.set_master_volume(Volume::from_linear(0.5).must());
    assert_eq!(pipeline.master_volume().linear(), 0.5);
    let halved = run_quantum(&mut pipeline, &frame);
    assert_eq!(f32_pairs(&halved), vec![(0.25, -0.25)]);

    pipeline.set_master_volume(Volume::UNITY);
    assert_eq!(pipeline.master_volume(), Volume::UNITY);
    let restored = run_quantum(&mut pipeline, &frame);
    assert_eq!(restored.data.as_slice(), unity.data.as_slice());
}

#[test]
fn fx3_a03_mute_zeroes_output() {
    // Mute zeroes every sample of the mixed quantum and conserves the
    // frame count (the clock contract: mute is silence, not a skip);
    // unmuting restores the exact prior bytes (stateless ×0).
    let mut pipeline = stereo_pipeline(RATE);
    let frame = stereo_s16_frame(&tone_pairs(4410, 1000.0, 0.8, 0.8, RATE), 0, RATE);
    let audible = run_quantum(&mut pipeline, &frame);
    assert!(left_channel(&audible).iter().any(|s| *s != 0.0));

    pipeline.set_muted(true);
    assert!(pipeline.muted());
    let muted = run_quantum(&mut pipeline, &frame);
    assert_eq!(
        muted.frames().must(),
        4410,
        "mute emits silence, never a shorter quantum"
    );
    assert!(
        f32_pairs(&muted)
            .iter()
            .all(|(l, r)| *l == 0.0 && *r == 0.0)
    );

    pipeline.set_muted(false);
    assert!(!pipeline.muted());
    let restored = run_quantum(&mut pipeline, &frame);
    assert_eq!(restored.data.as_slice(), audible.data.as_slice());
}

#[test]
fn fx3_a03_eq3_attenuates_sine_at_band_center() {
    // A 1 kHz sine (the mid peak's exact center) through −12 dB mid:
    // the settled tail carries 10^(−12/20) ≈ 0.2512 of the reference
    // amplitude. The reference is the SAME pipeline's uncontrolled
    // quantum, so s16 quantization cancels — the ratio is the EQ's
    // alone.
    let mut pipeline = stereo_pipeline(EQ_RATE);
    let frame = stereo_s16_frame(&tone_pairs(24_000, 1000.0, 0.4, 0.4, EQ_RATE), 0, EQ_RATE);
    let reference = run_quantum(&mut pipeline, &frame);

    pipeline.set_eq(0.0, -12.0, 0.0, Volume::UNITY).must();
    assert_eq!(pipeline.eq_bands(), Some((0.0, -12.0, 0.0, Volume::UNITY)));
    let filtered = run_quantum(&mut pipeline, &frame);

    let ratio = settled_rms(&left_channel(&filtered)) / settled_rms(&left_channel(&reference));
    let want = 10.0_f32.powf(-12.0 / 20.0);
    assert!(
        (ratio - want).abs() / want < 0.05,
        "L ratio {ratio} vs expected {want}"
    );
    let ratio = settled_rms(&right_channel(&filtered)) / settled_rms(&right_channel(&reference));
    assert!(
        (ratio - want).abs() / want < 0.05,
        "R ratio {ratio} vs expected {want}"
    );
}

#[test]
fn fx3_a03_balance_shifts_l_r_energy() {
    // Hard left keeps L and kills R; a mid position splits the energy
    // by the constant-power law (p = +0.5 ⇒ L/R amplitude = cot(67.5°)
    // = 0.41421, energy 0.17157).
    let mut pipeline = stereo_pipeline(RATE);
    let frame = stereo_s16_frame(&tone_pairs(4410, 1000.0, 0.8, 0.2, RATE), 0, RATE);

    pipeline.set_balance(-1.0).must();
    assert_eq!(pipeline.balance(), Some(-1.0));
    let hard_left = run_quantum(&mut pipeline, &frame);
    let l = settled_rms(&left_channel(&hard_left));
    let r = settled_rms(&right_channel(&hard_left));
    assert!(
        (l - 0.8 / std::f32::consts::SQRT_2).abs() < 1e-3,
        "L kept: {l}"
    );
    assert!(r < 1e-6, "R silenced: {r}");

    // Mid position on identical L/R content: the split is the law's.
    pipeline.clear_balance();
    assert_eq!(pipeline.balance(), None);
    let even = stereo_s16_frame(&tone_pairs(4410, 1000.0, 0.5, 0.5, RATE), 4410, RATE);
    pipeline.set_balance(0.5).must();
    let split = run_quantum(&mut pipeline, &even);
    let l = settled_rms(&left_channel(&split));
    let r = settled_rms(&right_channel(&split));
    // p = +0.5 ⇒ θ = 1.5·π/4; L/R amplitude = cot θ ≈ 0.41421.
    let theta = 1.5 * std::f32::consts::FRAC_PI_4;
    let want = theta.cos() / theta.sin();
    let ratio = l / r;
    assert!(
        (ratio - want).abs() < 1e-4,
        "L/R split {ratio} vs the pan law's {want}"
    );
}

#[test]
fn fx3_a03_eq_updates_apply_between_quanta_without_rebuild() {
    // The "without rebuilding" acceptance: ONE pipeline binding, two
    // EQ regimes — −12 dB then −6 dB mid at the band center — the
    // settled ratio between the two is exactly the 6 dB delta, and the
    // configuration getter tracks the second regime.
    let mut pipeline = stereo_pipeline(EQ_RATE);
    let frame = stereo_s16_frame(&tone_pairs(24_000, 1000.0, 0.4, 0.4, EQ_RATE), 0, EQ_RATE);

    pipeline.set_eq(0.0, -12.0, 0.0, Volume::UNITY).must();
    let deep = run_quantum(&mut pipeline, &frame);
    pipeline.set_eq(0.0, -6.0, 0.0, Volume::UNITY).must();
    assert_eq!(pipeline.eq_bands().map(|b| b.1), Some(-6.0));
    let shallow = run_quantum(&mut pipeline, &frame);

    let ratio = settled_rms(&left_channel(&shallow)) / settled_rms(&left_channel(&deep));
    let want = 10.0_f32.powf(6.0 / 20.0);
    assert!(
        (ratio - want).abs() / want < 0.05,
        "regime ratio {ratio} vs expected {want}"
    );
}

#[test]
fn fx3_a03_uncontrolled_chain_is_bit_exact_unity() {
    // The acceptance bar's other half: zero behavior change when
    // uncontrolled. A fresh pipeline and one whose controls were
    // exercised and returned to the identity produce byte-identical
    // quanta — and the exact pre-FX3-A03 values.
    let mut plain = stereo_pipeline(RATE);
    let mut roundtrip = stereo_pipeline(RATE);
    // Exercise every setter, then restore the identity.
    roundtrip.set_master_volume(Volume::from_linear(0.25).must());
    roundtrip.set_muted(true);
    roundtrip.set_balance(0.25).must();
    roundtrip.set_eq(6.0, -6.0, 3.0, Volume::UNITY).must();
    roundtrip.set_master_volume(Volume::UNITY);
    roundtrip.set_muted(false);
    roundtrip.clear_balance();
    roundtrip.clear_eq();
    assert!(roundtrip.controls().is_identity());
    assert!(plain.controls().is_identity());

    for quantum in 0..3_i64 {
        let frame = stereo_s16_frame(
            &[(16384, -16384), (8192, -8192), (0, 32767)],
            quantum * 3,
            RATE,
        );
        let a = run_quantum(&mut plain, &frame);
        let b = run_quantum(&mut roundtrip, &frame);
        assert_eq!(a.data.as_slice(), b.data.as_slice());
        assert_eq!(
            f32_pairs(&a),
            vec![
                (0.5, -0.5),
                (8192.0 / 32_768.0, -0.25),
                (0.0, 32_767.0 / 32_768.0)
            ]
        );
    }
}

#[test]
fn fx3_a03_control_validation_is_typed() {
    // The control boundary rejects bad values typed (D-6) — at set
    // time, never mid-stream: balance positions, the balance stereo
    // gate, the EQ band bound and low-rate cliff, and the dB-gain
    // overflow W07-A02 F-001 proved executable.
    let mut pipeline = stereo_pipeline(RATE);
    let err = pipeline.set_balance(1.01).err().must();
    assert_eq!(err.context.code, "audio.effects_balance_position");
    let err = pipeline.set_balance(f32::NAN).err().must();
    assert_eq!(err.context.code, "audio.effects_balance_position");
    let err = pipeline.set_eq(41.0, 0.0, 0.0, Volume::UNITY).err().must();
    assert_eq!(err.context.code, "audio.effects_eq_params");
    let err = pipeline
        .set_eq(0.0, f32::NAN, 0.0, Volume::UNITY)
        .err()
        .must();
    assert_eq!(err.context.code, "audio.effects_eq_params");
    let err = pipeline.set_eq(0.0, 0.0, 40.5, Volume::UNITY).err().must();
    assert_eq!(err.context.code, "audio.effects_eq_params");

    // A mono session refuses the stereo-only balance stage at set
    // time (the stage's own gate, surfaced at the boundary).
    let mut mono = AudioPipeline::new(PipelineSpec {
        rate: RATE,
        channels: 1,
        tracks: 1,
    })
    .must();
    let err = mono.set_balance(0.0).err().must();
    assert_eq!(err.context.code, "audio.effects_balance_channels");

    // A rate too low for the fixed 4 kHz shelf corner refuses typed
    // (the honest capability cliff, W07-A06 F-005).
    let mut narrow = AudioPipeline::new(PipelineSpec {
        rate: 8_000,
        channels: 2,
        tracks: 1,
    })
    .must();
    let err = narrow.set_eq(0.0, 0.0, 0.0, Volume::UNITY).err().must();
    assert_eq!(err.context.code, "audio.effects_eq_params");

    // The dB-gain overflow (finite 771 dB used to store +inf).
    let err = Volume::from_db(771.0).err().must();
    assert_eq!(err.context.code, "audio.gain_db_overflow");
}

#[test]
fn fx3_a03_getters_report_the_live_config() {
    // Observability: the getters mirror every set and every clear —
    // the host's honest read-back of the controls it armed.
    let mut pipeline = stereo_pipeline(RATE);
    assert_eq!(pipeline.master_volume(), Volume::UNITY);
    assert!(!pipeline.muted());
    assert_eq!(pipeline.balance(), None);
    assert_eq!(pipeline.eq_bands(), None);
    assert!(pipeline.controls().is_identity());

    pipeline.set_master_volume(Volume::from_linear(0.5).must());
    pipeline.set_muted(true);
    pipeline.set_balance(-0.25).must();
    let preamp = Volume::from_db(-1.0).must();
    pipeline.set_eq(3.0, -3.0, 6.0, preamp).must();
    assert_eq!(pipeline.master_volume().linear(), 0.5);
    assert!(pipeline.muted());
    assert_eq!(pipeline.balance(), Some(-0.25));
    let (low, mid, high, got_preamp) = pipeline.eq_bands().must();
    assert_eq!((low, mid, high), (3.0, -3.0, 6.0));
    assert_eq!(got_preamp, preamp);
    assert!(!pipeline.controls().is_identity());

    pipeline.clear_balance();
    pipeline.clear_eq();
    assert_eq!(pipeline.balance(), None);
    assert_eq!(pipeline.eq_bands(), None);
    pipeline.set_master_volume(Volume::UNITY);
    pipeline.set_muted(false);
    assert!(pipeline.controls().is_identity());
}
