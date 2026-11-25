//! FX3-A04 — the track-rate conversion resampler: wave-07's
//! engine-map gap ("Resampler: LINEAR, ±1000 ppm drift-only, NO
//! 44.1→48 conversion (convert.rs contains no SRC; pipeline refuses
//! rate mismatch)") activated as a REAL engine capability. The
//! [`RateResampler`](audio_engine::resample::RateResampler) is the
//! SRC path; wiring its call into the pipeline's rate-mismatch
//! refusal is the audio-output lane's later card, not this crate's.
//!
//! The count oracles are exact where the design is exact: the
//! emission rule (every stream position `k · num/den ≤ n − 1` emits
//! once, the eager exact-tail included) makes a finished `n`-sample
//! stream's output count `⌊(n−1) · den/num⌋ + 1` — no tolerance.
//! The spectrum check carries the tolerance instead (the linear
//! kernel's documented sinc² rolloff — resample.rs's fidelity
//! honesty, pinned here at a tolerance far above the kernel's error).

use std::f64::consts::PI;

use audio_engine::resample::RateResampler;
use testkit::Must;

/// Plane-view helper: build `&[Vec<f32>]` from rows (the in-module
/// resample tests' own helper, restated for the integration lane).
fn planes(rows: &[&[f32]]) -> Vec<Vec<f32>> {
    rows.iter().map(|row| row.to_vec()).collect()
}

/// Feed a mono source through the converter in `quantum`-sample
/// quanta (the streaming shape the pipeline wiring will use) and
/// collect the whole converted stream.
fn convert_stream(converter: &mut RateResampler, source: &[f32], quantum: usize) -> Vec<f32> {
    let mut collected = Vec::new();
    for chunk in source.chunks(quantum) {
        let input = planes(&[chunk]);
        let out = converter.process(&input, chunk.len());
        collected.extend_from_slice(&converter.planes()[0][..out]);
    }
    collected
}

/// The deterministic output count for a finished `n`-sample stream at
/// reduced ratio `num/den` (input samples per output sample): every
/// stream position `k · num/den ≤ n − 1` emits exactly once — the
/// eager exact-tail included — so the count is `⌊(n−1) · den/num⌋ + 1`.
fn converted_count(n: usize, num: u64, den: u64) -> u64 {
    (n as u64 - 1) * den / num + 1
}

/// Hann-windowed single-bin DFT magnitude at `freq` Hz — the
/// spectrum oracle (no FFT dependency for a single tone).
fn dft_magnitude(samples: &[f32], rate: u32, freq: f64) -> f64 {
    let n = samples.len();
    let mut re = 0.0_f64;
    let mut im = 0.0_f64;
    for (k, v) in samples.iter().enumerate() {
        let window = 0.5 - 0.5 * (2.0 * PI * k as f64 / (n as f64 - 1.0)).cos();
        let angle = 2.0 * PI * freq * k as f64 / f64::from(rate);
        re += f64::from(*v) * window * angle.cos();
        im -= f64::from(*v) * window * angle.sin();
    }
    (re * re + im * im).sqrt()
}

/// The monotone-hull oracle: every output of a linear kernel over a
/// monotone ramp is itself monotone and inside the ramp's hull (the
/// kernel is a convex combination — no overshoot, no fabrication).
fn assert_monotone_inside_hull(out: &[f32], first: f32, last: f32) {
    let mut previous = f32::MIN;
    for (i, v) in out.iter().enumerate() {
        assert!(*v >= previous, "not monotone at {i}: {v} < {previous}");
        assert!(
            (first..=last).contains(v),
            "escaped the hull at {i}: {v} not in [{first}, {last}]"
        );
        previous = *v;
    }
}

#[test]
fn fx3_a04_sine_44100_to_48000_keeps_ratio_and_spectrum() {
    // 0.1 s of a 440 Hz tone at amplitude 0.5, streamed in 1470-sample
    // quanta: 4410 source samples → the exact emission-rule count
    // ⌊4409 · 160/147⌋ + 1 = 4799 (the naive duration ratio says
    // 4800 — the −1 is the final position whose kernel right-sample
    // does not exist; the exact count is the oracle).
    let source: Vec<f32> = (0..4410)
        .map(|k| (2.0 * PI * 440.0 * k as f64 / 44_100.0).sin() as f32 * 0.5)
        .collect();
    let mut converter = RateResampler::new(1, 44_100, 48_000).must();
    let out = convert_stream(&mut converter, &source, 1470);
    assert_eq!(out.len() as u64, converted_count(4410, 147, 160));
    // Spectrum: resampling moves the sampling grid, not the waveform
    // — the tone stays 440 Hz at the destination rate, dominating
    // every off-tone bin by ≥5× (the 140/740 Hz bins are the rational
    // phase pattern's 300 Hz modulation sidebands — the linear
    // kernel's own small error, orders below the margin).
    let peak = dft_magnitude(&out, 48_000, 440.0);
    for freq in [
        140.0, 200.0, 300.0, 350.0, 380.0, 410.0, 470.0, 520.0, 640.0, 740.0, 800.0,
    ] {
        let other = dft_magnitude(&out, 48_000, freq);
        assert!(peak > 5.0 * other, "440 Hz {peak} vs {freq} Hz {other}");
    }
    // Amplitude: the linear kernel's response at 440/44100 is
    // sinc²(π · 440/44100) ≈ 0.9998 — the documented tolerance, with
    // 2 % headroom, is generous by design.
    let max = out.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
    assert!((0.45..=0.515).contains(&max), "amplitude drifted: {max}");
}

#[test]
fn fx3_a04_non_integer_upsample_22050_to_48000_hits_the_ratio() {
    // Ratio 320/147 (×2.18 upsample — far past the drift twin's
    // ±1000 ppm): 7350 ramp samples in 735-sample quanta → the exact
    // count, monotone and inside the hull.
    let source: Vec<f32> = (0..7350).map(|k| k as f32).collect();
    let mut converter = RateResampler::new(1, 22_050, 48_000).must();
    let out = convert_stream(&mut converter, &source, 735);
    assert_eq!(out.len() as u64, converted_count(7350, 147, 320));
    assert_monotone_inside_hull(&out, 0.0, 7349.0);
}

#[test]
fn fx3_a04_non_integer_downsample_48000_to_44100_hits_the_ratio() {
    // Ratio 147/160 (×0.92 downsample — the kernel SKIPS input
    // samples; no read may land outside the quantum that owns it):
    // 4800 ramp samples in 1600-sample quanta → the exact count.
    let source: Vec<f32> = (0..4800).map(|k| k as f32).collect();
    let mut converter = RateResampler::new(1, 48_000, 44_100).must();
    let out = convert_stream(&mut converter, &source, 1600);
    assert_eq!(out.len() as u64, converted_count(4800, 160, 147));
    assert_monotone_inside_hull(&out, 0.0, 4799.0);
}

#[test]
fn fx3_a04_one_sample_quanta_carry_the_phase_and_reset_restarts() {
    // The tail-prefix path at its extreme: 1-sample quanta make every
    // position behind a base a cross-quantum interpolation. 4096 ramp
    // samples at 44.1→48 → the exact whole-stream count (the phase
    // carry must make the quantumization invisible); every output
    // inside the hull. Then `reset()` drops the stream phase: the
    // next quantum's first output is exactly its first sample.
    let source: Vec<f32> = (0..4096).map(|k| k as f32).collect();
    let mut converter = RateResampler::new(1, 44_100, 48_000).must();
    let out = convert_stream(&mut converter, &source, 1);
    assert_eq!(out.len() as u64, converted_count(4096, 147, 160));
    assert_monotone_inside_hull(&out, 0.0, 4096.0);
    converter.reset();
    let input = planes(&[&[7.0, 8.0, 9.0]]);
    let out = converter.process(&input, 3);
    assert_eq!(out, 3);
    assert_eq!(converter.planes()[0][0], 7.0);
}

#[test]
fn fx3_a04_matching_rates_are_a_bit_exact_passthrough() {
    // Equal rates: the identity fast path — the count is conserved
    // AND the bits survive verbatim (−0.0 included: a copy, not the
    // interpolation's `a + (b−a)·0` sum, which would flush it).
    let specials = [
        -0.0f32,
        0.0,
        1.5,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    let mut converter = RateResampler::new(2, 48_000, 48_000).must();
    assert!(converter.is_passthrough());
    assert_eq!(converter.rates(), (48_000, 48_000));
    for quantum in 0..4 {
        let input: Vec<Vec<f32>> = (0..2)
            .map(|c| {
                (0..6)
                    .map(|i| specials[(quantum + i + c as usize) % specials.len()])
                    .collect()
            })
            .collect();
        let out = converter.process(&input, 6);
        assert_eq!(out, 6, "quantum {quantum}");
        for (c, plane) in converter.planes().iter().enumerate() {
            let got: Vec<u32> = plane.iter().map(|v| v.to_bits()).collect();
            let want: Vec<u32> = input[c].iter().map(|v| v.to_bits()).collect();
            assert_eq!(got, want, "channel {c} quantum {quantum}");
        }
    }
    // The identity holds at every matching rate pair, not just 48 k.
    let mut converter = RateResampler::new(1, 44_100, 44_100).must();
    let input = planes(&[&[0.25, -0.5, 0.75]]);
    assert_eq!(converter.process(&input, 3), 3);
    assert_eq!(converter.planes()[0], input[0]);
}

#[test]
fn fx3_a04_zero_channels_or_zero_rates_are_typed_refusals() {
    // A zero rate has no ratio: the same typed shape refusal the
    // pipeline itself gives a zero spec (`audio.pipeline_shape_invalid`).
    for (channels, src, dst) in [(0u32, 44_100, 48_000), (2, 0, 48_000), (2, 44_100, 0)] {
        let err = RateResampler::new(channels, src, dst).err().must();
        assert_eq!(err.context.code, "audio.pipeline_shape_invalid");
    }
}

#[test]
fn fx3_a04_ragged_or_wrong_shaped_planes_emit_zero_not_panic() {
    // The drift twin's audit F-002 class (a plane shorter than
    // `in_frames` panicked it) done right at birth: a channel-count
    // mismatch OR a short plane OR a zero-frame quantum produces zero
    // output, no panic, no sticky state — a well-formed quantum right
    // after the refusals still converts.
    let mut converter = RateResampler::new(2, 44_100, 48_000).must();
    let mono = planes(&[&[1.0, 2.0]]);
    assert_eq!(converter.process(&mono, 2), 0);
    let short = planes(&[&[1.0, 2.0], &[1.0]]);
    assert_eq!(converter.process(&short, 2), 0);
    let empty = planes(&[&[], &[]]);
    assert_eq!(converter.process(&empty, 0), 0);
    let good = planes(&[&[1.0, 2.0], &[3.0, 4.0]]);
    let out = converter.process(&good, 2);
    assert_eq!(out, 2);
    assert_eq!(converter.planes()[0][0], 1.0);
}

#[test]
fn fx3_a04_long_stream_ratio_is_exact_with_no_cumulative_drift() {
    // 1000 quanta × 1470 frames: the rational accumulator never
    // rounds — both directions land the exact closed-form count (the
    // 44.1→48 stream's 1 599 999 and the 48→44.1 stream's 1 350 562
    // — the .5 fraction of the naive ratio shows the accumulator is
    // not snapping to round numbers).
    let input = planes(&[&[0.5_f32; 1470]]);
    let mut converter = RateResampler::new(1, 44_100, 48_000).must();
    let mut total = 0u64;
    for _ in 0..1000 {
        total += converter.process(&input, 1470) as u64;
    }
    assert_eq!(total, converted_count(1_470_000, 147, 160));
    let mut converter = RateResampler::new(1, 48_000, 44_100).must();
    let mut total = 0u64;
    for _ in 0..1000 {
        total += converter.process(&input, 1470) as u64;
    }
    assert_eq!(total, converted_count(1_470_000, 160, 147));
}

#[test]
fn fx3_a04_channels_share_one_phase_by_construction() {
    // One phase for every channel (the drift twin's coherence proof,
    // re-pinned on the rate path): channel 1 = channel 0 / 10 through
    // the interpolation, to f32 rounding.
    let mut converter = RateResampler::new(2, 44_100, 48_000).must();
    let input = planes(&[&[10.0, 20.0, 30.0], &[1.0, 2.0, 3.0]]);
    let out = converter.process(&input, 3);
    assert_eq!(out, 3);
    for (x, y) in converter.planes()[0]
        .iter()
        .zip(converter.planes()[1].iter())
    {
        assert!((x / 10.0 - y).abs() < 2e-4, "{x} / 10 vs {y}");
    }
}

// ---- the differential oracle: an independent single-loop kernel ------------

/// The independent reference: the pre-split single-loop kernel
/// (the drift twin's `ReferenceResampler` shape, generalized from a
/// ppm correction to the rate pair). Emits while `floor(p) < last_abs`
/// or `p == last_abs` exactly — one loop, no prefix/body/tail split.
struct ReferenceRateResampler {
    channels: usize,
    step_num: u64,
    step_den: u64,
    cursor_abs: i128,
    frac_num: u64,
    tail_base: i128,
    tail: Vec<f32>,
    out: Vec<Vec<f32>>,
}

fn reference_gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

impl ReferenceRateResampler {
    fn new(channels: u32, src_rate: u32, dst_rate: u32) -> ReferenceRateResampler {
        let g = reference_gcd(u64::from(src_rate), u64::from(dst_rate));
        let channels = channels as usize;
        ReferenceRateResampler {
            channels,
            step_num: u64::from(src_rate) / g,
            step_den: u64::from(dst_rate) / g,
            cursor_abs: 0,
            frac_num: 0,
            tail_base: -1,
            tail: vec![0.0; channels],
            out: vec![Vec::new(); channels],
        }
    }

    fn reset(&mut self) {
        self.cursor_abs = 0;
        self.frac_num = 0;
        self.tail_base = -1;
        self.tail.fill(0.0);
    }

    fn process(&mut self, input: &[Vec<f32>], in_frames: usize) -> usize {
        if in_frames == 0 || input.len() != self.channels {
            return 0;
        }
        let step_num = self.step_num;
        let step_den = self.step_den;
        let base_abs = self.tail_base + 1;
        let last_abs = base_abs + in_frames as i128 - 1;
        let mut cursor = self.cursor_abs;
        let mut frac = self.frac_num;
        let tail = &self.tail;
        let sample = |c: usize, index: i128| -> f32 {
            if index >= base_abs {
                input[c]
                    .get((index - base_abs) as usize)
                    .copied()
                    .unwrap_or(0.0)
            } else {
                tail[c]
            }
        };
        for plane in &mut self.out {
            plane.clear();
        }
        let mut emitted = 0usize;
        while cursor <= last_abs && (frac == 0 || cursor < last_abs) {
            let weight = (frac as f64 / step_den as f64) as f32;
            let i0 = cursor;
            let exact_tail = frac == 0 && cursor == last_abs;
            for (c, plane) in self.out.iter_mut().enumerate() {
                let a = sample(c, i0);
                let b = if exact_tail { a } else { sample(c, i0 + 1) };
                plane.push(a + (b - a) * weight);
            }
            emitted += 1;
            frac += step_num;
            let whole = frac / step_den;
            frac -= whole * step_den;
            cursor += whole as i128;
        }
        self.cursor_abs = cursor;
        self.frac_num = frac;
        for (c, slot) in self.tail.iter_mut().enumerate() {
            *slot = input[c][in_frames - 1];
        }
        self.tail_base = last_abs;
        emitted
    }
}

#[test]
fn fx3_a04_split_kernel_is_bit_identical_to_a_single_loop_reference() {
    // The differential proof, in the drift twin's own style: the
    // production prefix/body/exact-tail split must emit bit-identical
    // samples to the independent single-loop reference over the whole
    // reachable space — up/down/non-integer/extreme ratios, quantum
    // lengths from 1-sample to 1471 (the prefix at 1, the body at
    // 1470, the boundary at 1471), 2 channel counts, multi-quantum
    // phase carry and a mid-stream reset.
    let mut seed = 0x0123_4567_89ab_cdef_u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for (src, dst) in [
        (44_100u32, 48_000u32),
        (48_000, 44_100),
        (22_050, 48_000),
        (96_000, 48_000),
        (8_000, 192_000),
        (16_000, 16_000),
    ] {
        for channels in [1u32, 2] {
            let mut mine = RateResampler::new(channels, src, dst).must();
            let mut reference = ReferenceRateResampler::new(channels, src, dst);
            for quantum in 0..14 {
                let in_frames = [1usize, 2, 3, 7, 147, 1470, 1471][quantum % 7];
                let input: Vec<Vec<f32>> = (0..channels)
                    .map(|c| {
                        (0..in_frames)
                            .map(|i| {
                                // Wide dynamic range; the generator
                                // never yields −0.0 (integer division
                                // then a positive offset), so the
                                // 16 k→16 k passthrough's copy-vs-sum
                                // sign flush cannot false-fail here.
                                let v = (next() % 20_001) as i32 - 10_000;
                                v as f32 / 512.0 + c as f32 + ((i % 7) as f32) * 0.125
                            })
                            .collect()
                    })
                    .collect();
                let out_mine = mine.process(&input, in_frames);
                let out_ref = reference.process(&input, in_frames);
                assert_eq!(
                    out_mine, out_ref,
                    "src {src} dst {dst} ch {channels} quantum {quantum}"
                );
                for (c, (a, b)) in mine.planes().iter().zip(reference.out.iter()).enumerate() {
                    assert_eq!(
                        a[..out_mine],
                        b[..out_ref],
                        "src {src} dst {dst} ch {channels} quantum {quantum} plane {c}"
                    );
                }
                if quantum == 9 {
                    mine.reset();
                    reference.reset();
                }
            }
        }
    }
}
