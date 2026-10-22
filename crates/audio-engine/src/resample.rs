//! The bounded micro-resampler (card 3.B1; doc 03 §4.7 e / §4.8 e-f) —
//! **consumer #1 of [`DriftCorrection`]**, the piece audit M12 named
//! absent ("DriftCorrection ±1000ppm limiter: ZERO consumers").
//!
//! Doc 03 §4.7(e) assigns audio-side drift (device clock vs media
//! rate) to "a bounded micro-resampler in audio-engine: correction
//! rate limited to ±1000 ppm" — this module is that resampler. The
//! resample ratio is derived exactly from the stored correction:
//! `out = in × (1e6 + ppm) / 1e6` — a positive correction (the
//! follower lags the master, [`DriftCorrection`]'s convention) emits
//! *more* device samples per media sample, refilling a draining ring.
//!
//! # Interpolation
//!
//! Linear between adjacent input samples `[PROPOSAL per W1-18 §3.2:
//! linear is inaudible at ≤1000 ppm; cubic is a later quality knob,
//! not a correctness need]`. One shared phase across channels —
//! channel-coherent by construction. The phase is an exact `i128`
//! rational accumulator (input position advanced by
//! `1e6 / (1e6 + ppm)` per output sample, remainder carried across
//! quanta): **no cumulative rounding drift, no floats on the control
//! path** (doc 01 §3.1 discipline — the sample *data* is f32, the
//! *positions* are exact integers).
//!
//! # Bypass
//!
//! [`DriftCorrection::ZERO`] is a **bit-exact bypass by ownership, not
//! by copy**: the pipeline's `mix` does not call this resampler at
//! all while `ppm == 0` (no interpolation, no copy — the mixed planes
//! feed stage D directly, pinned by test). A non-zero correction
//! routes the mixed planes through [`MicroResampler::process`] into
//! the resampler's own reused plane set.
//!
//! # Discipline (D-1 / D-8)
//!
//! - Planes and the one-sample-per-channel history are preallocated
//!   at construction; `process` only `push`es into reserved capacity
//!   (no allocation after warm-up — measured by the counting
//!   allocator in `tests/zero_alloc_quantum.rs`).
//! - `out_len ≤ in_len + in_len/1000 + 1` for any `|ppm| ≤ 1000` —
//!   capacity reserved per call, bounded by construction.
//! - No `unsafe`, no floats on the control path, no panics: a
//!   zero-frame quantum is a no-op; a geometry mismatch produces no
//!   output (the pipeline is the only caller and guarantees the
//!   shape — the guard is defensive, not reachable).
//!
//! # Why not a general resampler
//!
//! Track-rate conversion (a 48 kHz track in a 44.1 kHz session) is a
//! different, later card: it changes the *negotiated* rate, not the
//! device-vs-nominal ratio. This type's whole contract is the
//! ±1000 ppm *correction* bound — the limiter is the invariant
//! (drift.rs:23-25), which is why the ratio derives from
//! [`DriftCorrection`] and never from a free-form number.

use media_clock::DriftCorrection;

/// The ppm scale: 1 ppm = 1e-6 of the nominal rate (drift.rs's own
/// constant, restated for the phase arithmetic below).
const PPM_SCALE: i64 = 1_000_000;

/// The bounded linear micro-resampler over f32 planes.
///
/// Streaming shape: [`MicroResampler::process`] takes one quantum's
/// mixed planes and emits the resampled quantum into the resampler's
/// own reused planes, returning the output frame count. State is
/// O(channels) fixed-size (the phase accumulator plus one history
/// sample per channel) — nothing grows with stream length (D-1).
#[derive(Debug, Clone)]
pub struct MicroResampler {
    /// Channel count (the mixer shape; fixed at construction).
    channels: usize,
    /// The correction in force (the ratio's source — the limiter's
    /// guarantee rides on [`DriftCorrection`]'s constructor).
    correction: DriftCorrection,
    /// The absolute whole-input-sample cursor (input samples consumed
    /// since the stream origin — `i128`: ~1e14 samples ≈ 7 years at
    /// 44.1 kHz before it even approaches half-range).
    cursor_abs: i128,
    /// The fractional part of the input position, as a numerator over
    /// `step_den` (`0 ≤ frac < step_den`, the rational carry).
    frac_num: i64,
    /// The absolute index of the previous quantum's **last** input
    /// sample (the tail's stream position); `−1` before the first
    /// quantum, so the first quantum's base is index 0.
    tail_base: i128,
    /// One history sample per channel: the previous quantum's last
    /// input sample (the only cross-quantum read the linear kernel
    /// can make — see the invariant note on `process`).
    tail: Vec<f32>,
    /// The output planes (reused; the pipeline interleaves from
    /// these). `out[c]` is cleared and refilled per quantum.
    out: Vec<Vec<f32>>,
}

impl MicroResampler {
    /// Build for a mixer shape (channels) at zero correction — the
    /// dormant state every pipeline starts in.
    #[must_use]
    pub fn new(channels: u32) -> MicroResampler {
        let channels = channels as usize;
        MicroResampler {
            channels,
            correction: DriftCorrection::ZERO,
            cursor_abs: 0,
            frac_num: 0,
            tail_base: -1,
            tail: vec![0.0; channels],
            out: vec![Vec::new(); channels],
        }
    }

    /// The correction in force.
    #[must_use]
    pub fn correction(&self) -> DriftCorrection {
        self.correction
    }

    /// Set the correction (the governor's actuator input). The phase
    /// carry is renormalized against the new denominator so the
    /// `0 ≤ frac < step_den` invariant survives ratio changes
    /// mid-stream (the cursor absorbs the whole-sample part — no
    /// discontinuity, no reset).
    pub fn set_correction(&mut self, correction: DriftCorrection) {
        if correction.ppm() == self.correction.ppm() {
            return;
        }
        self.correction = correction;
        let step_den = PPM_SCALE + correction.ppm();
        while self.frac_num >= step_den {
            self.frac_num -= step_den;
            self.cursor_abs += 1;
        }
    }

    /// Drop all stream state (a new regime: seek / generation / anchor
    /// epoch). The next quantum starts a fresh phase at its first
    /// sample; the correction itself is the caller's to keep or drop
    /// (the governor decides — the resampler is stateless about *why*).
    pub fn reset(&mut self) {
        self.cursor_abs = 0;
        self.frac_num = 0;
        self.tail_base = -1;
        for slot in &mut self.tail {
            *slot = 0.0;
        }
    }

    /// The output planes of the last [`MicroResampler::process`] call
    /// (the stage-D interleave source while a correction is in force).
    /// Empty before the first call; `out[c].len()` equals the returned
    /// frame count for every channel.
    #[must_use]
    pub fn planes(&self) -> &[Vec<f32>] {
        &self.out
    }

    /// Resample one quantum: read `input[c][0..in_frames]`, emit into
    /// the reused output planes, answer the output frame count.
    ///
    /// # Phase math (exact, no floats)
    ///
    /// The input position advances `step = 1e6 / (1e6 + ppm)` input
    /// samples per output sample, carried as `cursor_abs + frac_num /
    /// step_den` across quanta. The whole-sample cursor is **absolute
    /// over the whole stream**; `tail` holds the previous quantum's
    /// last sample (the only sample behind the current quantum's base
    /// the kernel can read: the loop's exit guarantees the entry
    /// cursor is exactly `base − 1` or `base`, so the deepest
    /// cross-quantum read is `base − 1` — one sample).
    ///
    /// # Bounds
    ///
    /// Emits while the kernel's right sample (`cursor + 1`) is inside
    /// the current quantum, so `out_frames ∈ [in·ratio − 1,
    /// in·ratio + 1]` and always `≤ in + in/1000 + 1` for the legal
    /// correction range. Zero `in_frames` is a no-op returning 0.
    pub fn process(&mut self, input: &[Vec<f32>], in_frames: usize) -> usize {
        if in_frames == 0 || input.len() != self.channels {
            return 0;
        }
        let step_num = PPM_SCALE;
        let step_den = PPM_SCALE + self.correction.ppm();
        // This quantum's absolute placement: right after the previous
        // quantum's last sample (tail_base), which the tail holds.
        let base_abs = self.tail_base + 1;
        // The absolute index of this quantum's last input sample.
        let last_abs = base_abs + in_frames as i128 - 1;

        // Local phase copies: the kernel reads the tail (immutable)
        // while the output planes mutate — field-disjoint borrows.
        let mut cursor = self.cursor_abs;
        let mut frac = self.frac_num;
        let tail = &self.tail;

        // The interpolated read for one absolute index. `index <
        // base_abs` happens only for `base_abs − 1` (the entry-cursor
        // invariant) — the one-sample tail.
        let sample = |c: usize, index: i128| -> f32 {
            if index >= base_abs {
                // The loop guard keeps index ≤ last_abs, so this is
                // in-bounds by construction; the `get` is the honest
                // defensive backstop, never a fabricated value.
                input[c].get((index - base_abs) as usize).copied().unwrap_or(0.0)
            } else {
                tail[c]
            }
        };

        // Emit: clear the reused planes, reserve the worst-case
        // capacity once (after warm-up a no-op — D-1).
        let cap = in_frames + in_frames / 1000 + 1;
        for plane in &mut self.out {
            plane.clear();
            plane.reserve(cap);
        }
        let mut emitted = 0usize;
        // Emit while the kernel's right sample lies inside the
        // quantum, OR the phase sits exactly on a whole sample at
        // the quantum's last index (weight 0 — no right neighbor
        // needed, so a ppm == 0 quantum conserves its count exactly;
        // the deferred sample otherwise waits for the next quantum).
        while cursor <= last_abs && (frac == 0 || cursor + 1 <= last_abs) {
            // Both operands ≤ 1_001_000 < 2^24: exact in f32, and the
            // division is the only float on this path — it scales
            // sample *data*, never a position.
            let weight = frac as f32 / step_den as f32;
            let i0 = cursor;
            let exact_tail = frac == 0 && cursor == last_abs;
            for (c, plane) in self.out.iter_mut().enumerate() {
                let a = sample(c, i0);
                let b = if exact_tail { a } else { sample(c, i0 + 1) };
                plane.push(a + (b - a) * weight);
            }
            emitted += 1;
            // Advance the exact rational phase: at most two whole
            // wraps per output sample (step_num < 2·step_den for any
            // |ppm| ≤ 1000).
            frac += step_num;
            while frac >= step_den {
                frac -= step_den;
                cursor += 1;
            }
        }

        // Commit the phase and roll the tail: this quantum's last
        // sample becomes the cross-quantum history; the next quantum
        // starts right after it.
        self.cursor_abs = cursor;
        self.frac_num = frac;
        for (c, slot) in self.tail.iter_mut().enumerate() {
            *slot = input[c][in_frames - 1];
        }
        self.tail_base = last_abs;
        emitted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plane-view helper: build `&[Vec<f32>]` from rows.
    fn planes(rows: &[&[f32]]) -> Vec<Vec<f32>> {
        rows.iter().map(|row| row.to_vec()).collect()
    }

    #[test]
    fn zero_correction_emits_the_input_count_exactly() {
        // Not the bypass path (that skips the resampler wholly) — the
        // arithmetic path at ppm = 0 must still be exact: step = 1,
        // every output lands on an input sample.
        let mut r = MicroResampler::new(1);
        r.set_correction(DriftCorrection::ZERO);
        let input = planes(&[&[0.0, 1.0, 2.0, 3.0, 4.0]]);
        let out = r.process(&input, 5);
        assert_eq!(out, 5);
        assert_eq!(r.planes()[0], vec![0.0, 1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn ratio_is_exact_over_long_streams() {
        // +1000 ppm for 1000 quanta of 1470: out must be
        // 1000 × 1470 × 1.001 = 1_471_470 ± 1 (the ±1 is the
        // sub-sample phase straddling the final quantum boundary).
        let mut r = MicroResampler::new(2);
        r.set_correction(DriftCorrection::new(1000));
        let mut total = 0u64;
        let input = planes(&[&[0.5_f32; 1470], &[0.5_f32; 1470]]);
        for _ in 0..1000 {
            total += r.process(&input, 1470) as u64;
        }
        let want = 1000_u64 * 1470 * 1001 / 1000;
        assert!(
            total.abs_diff(want) <= 1,
            "out {total} vs exact {want}"
        );
        // −1000 ppm symmetric.
        let mut r = MicroResampler::new(2);
        r.set_correction(DriftCorrection::new(-1000));
        let mut total = 0u64;
        for _ in 0..1000 {
            total += r.process(&input, 1470) as u64;
        }
        let want = 1000_u64 * 1470 * 999 / 1000;
        assert!(total.abs_diff(want) <= 1, "out {total} vs exact {want}");
    }

    #[test]
    fn phase_carries_exactly_across_quanta() {
        // +500 ppm: step = 1e6/1000500. Feed 1-sample quanta of a
        // ramp and check every emitted sample is a convex combination
        // of its two neighbors (linear kernel, monotone ramp).
        let mut r = MicroResampler::new(1);
        r.set_correction(DriftCorrection::new(500));
        let mut value = 0.0_f32;
        let mut all = Vec::new();
        for _ in 0..4096 {
            let input = planes(&[&[value]]);
            value += 1.0;
            let n = r.process(&input, 1);
            for k in 0..n {
                all.push(r.planes()[0][k]);
            }
        }
        // 4096 inputs × 1.0005 → ~4098 outputs; all within the ramp.
        assert!(all.len() >= 4096 && all.len() <= 4099, "{}", all.len());
        let mut prev = f32::MIN;
        for (i, v) in all.iter().enumerate() {
            assert!(v >= &prev, "not monotone at {i}: {v} < {prev}");
            assert!(*v >= 0.0 && *v <= 4096.0, "escaped ramp at {i}: {v}");
            prev = *v;
        }
    }

    #[test]
    fn linear_interpolation_values_are_exact() {
        // ppm = +500 ⇒ step = 1e6/1_000_500. First output at phase 0
        // (== input 0), second at 1e6/1000500 ≈ 0.9995 — between
        // samples 0 and 1 with weight (1e6 − 999500.xxx)/… — verify
        // against the closed form for the first few outputs of a
        // known ramp with a LARGE quantum (no boundary effects).
        let mut r = MicroResampler::new(1);
        r.set_correction(DriftCorrection::new(500));
        let input: Vec<Vec<f32>> = planes(&[&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]]);
        let out = r.process(&input, 8);
        assert_eq!(out, 8, "8 × 1.0005 → 8.004 ⇒ 8 outputs");
        // out[1] = lerp(0, 1, 1e6/1000500) = 0.9995002… — the closed
        // form in f64, compared at f32 precision.
        let step = 1.0_f64 / 1.0005_f64;
        for (k, v) in r.planes()[0].iter().enumerate() {
            let want = (step * k as f64) as f32;
            assert!(
                (v - want).abs() < 1e-5,
                "out[{k}] = {v} vs closed form {want}"
            );
        }
    }

    #[test]
    fn channel_coherence_by_construction() {
        // Same phase for every channel: distinct constant planes
        // interpolate to the same convex blend per channel.
        let mut r = MicroResampler::new(3);
        r.set_correction(DriftCorrection::new(250));
        let input = planes(&[&[10.0, 20.0], &[1.0, 2.0], &[-5.0, 5.0]]);
        let out = r.process(&input, 2);
        assert_eq!(out, 2);
        let [a, b] = [&r.planes()[0], &r.planes()[1]];
        // Channel 1 = channel 0 / 10 exactly (linear kernel scales).
        for (x, y) in a.iter().zip(b.iter()) {
            assert!((x / 10.0 - y).abs() < 1e-6);
        }
        // Channel 3's spread is 10× channel 1's — same weights.
        let c = &r.planes()[2];
        assert!((c[1] - c[0]).abs() > 9.0);
    }

    #[test]
    fn correction_change_midstream_renormalizes_the_phase() {
        let mut r = MicroResampler::new(1);
        r.set_correction(DriftCorrection::new(1000));
        let input = planes(&[&[0.0, 1.0, 2.0, 3.0]]);
        let _ = r.process(&input, 4);
        // Change the correction: the carried fraction is renormalized
        // against the new denominator (the cursor absorbs the whole
        // part) — no panic, no position loss, processing continues.
        r.set_correction(DriftCorrection::new(-1000));
        let out = r.process(&input, 4);
        assert!(out >= 2 && out <= 5, "{out}");
        // And back.
        r.set_correction(DriftCorrection::new(500));
        let out = r.process(&input, 4);
        assert!(out >= 3 && out <= 6, "{out}");
    }

    #[test]
    fn reset_starts_a_fresh_phase() {
        let mut r = MicroResampler::new(1);
        r.set_correction(DriftCorrection::new(1000));
        let input = planes(&[&[7.0, 8.0, 9.0]]);
        let _ = r.process(&input, 3);
        r.reset();
        // After reset the next quantum's first output is exactly its
        // first sample (phase 0), like a fresh stream.
        let out = r.process(&input, 3);
        assert_eq!(out, 3);
        assert_eq!(r.planes()[0][0], 7.0);
    }

    #[test]
    fn zero_frame_quantum_is_a_no_op() {
        let mut r = MicroResampler::new(2);
        r.set_correction(DriftCorrection::new(300));
        let empty = planes(&[&[], &[]]);
        assert_eq!(r.process(&empty, 0), 0);
        let input = planes(&[&[1.0, 2.0], &[3.0, 4.0]]);
        assert_eq!(r.process(&input, 2), 2);
        assert_eq!(r.planes()[0][0], 1.0);
    }

    #[test]
    fn out_never_exceeds_the_design_bound() {
        // out ≤ in + in/1000 + 1 for the extreme correction, over
        // adversarial in lengths (D-1 capacity contract).
        for in_frames in [1usize, 2, 3, 7, 1470, 1471, 4096] {
            let mut r = MicroResampler::new(1);
            r.set_correction(DriftCorrection::new(1000));
            let row = vec![0.0_f32; in_frames];
            let input = planes(&[&row]);
            let out = r.process(&input, in_frames);
            assert!(
                out <= in_frames + in_frames / 1000 + 1,
                "in {in_frames} → out {out}"
            );
        }
    }

    #[test]
    fn the_stream_never_fabricates_samples_past_the_kernel_window() {
        // The interpolation weight is always in [0, 1): every output
        // is inside the convex hull of its two source samples. A
        // bounded input stream therefore produces bounded outputs —
        // no ringing, no overshoot (the linear-kernel guarantee).
        let mut r = MicroResampler::new(1);
        r.set_correction(DriftCorrection::new(-777));
        let input = planes(&[&[0.0, 100.0, -100.0, 0.0, 100.0, 0.0]]);
        for _ in 0..100 {
            let out = r.process(&input, 6);
            for v in &r.planes()[0][..out] {
                assert!(*v >= -100.0 && *v <= 100.0, "escaped hull: {v}");
            }
        }
    }
}
