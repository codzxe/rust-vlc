//! The test-pattern generator: an exact, table-free sine source for
//! pipeline proofs (wave 14, issue #1 — the PCM pipeline evidence
//! behind the ADR-005 seam; std-only, no `unsafe`, no allocation).
//!
//! # Exactness contract
//!
//! Sample instant `n` is `amplitude · sin(2π · freq · n / rate)`,
//! computed in **f64 from first principles** (no phase accumulator, no
//! lookup table) and cast to f32. Because every sample is a pure
//! function of its instant, the generator and any independently
//! written oracle agree **bit-for-bit by construction** — the property
//! the PCM-pipeline proof asserts against the sink's captured stream.
//! A frame carries the same value on every channel (a mono-compatible
//! test tone; channel *mapping* is downstream negotiation, doc 03).
//!
//! # Never
//!
//! Device semantics, timing, or drift modeling — this is a value
//! oracle, not a clock (the [`crate::counter::SampleCounter`] is the
//! clock truth).

use media_types::EngineError;
use media_types::frame::AudioFormat;

use crate::error;

/// A deterministic sine test pattern (module docs).
#[derive(Debug, Clone)]
pub struct SinePattern {
    freq_hz: f64,
    rate_hz: f64,
    channels: usize,
    amplitude: f32,
    next_instant: u64,
}

impl SinePattern {
    /// A pattern of `freq_hz` at the format's rate, `amplitude`
    /// (0, 1] on every channel.
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.pattern_invalid`: a non-finite or non-positive
    /// frequency, a frequency at/above Nyquist (`rate / 2`), a zero
    /// rate/channels, or an amplitude outside (0, 1].
    pub fn new(
        freq_hz: f64,
        format: &AudioFormat,
        amplitude: f32,
    ) -> Result<SinePattern, EngineError> {
        let valid = freq_hz.is_finite()
            && freq_hz > 0.0
            && format.rate > 0
            && format.channels > 0
            && 2.0 * freq_hz < f64::from(format.rate)
            && amplitude.is_finite()
            && amplitude > 0.0
            && amplitude <= 1.0;
        if !valid {
            return Err(error::unsupported("audio.pattern_invalid"));
        }
        Ok(SinePattern {
            freq_hz,
            rate_hz: f64::from(format.rate),
            channels: format.channels as usize,
            amplitude,
            next_instant: 0,
        })
    }

    /// The exact value at sample instant `n` — the oracle (a pure
    /// function of the instant; independent of generation state).
    pub fn sample_at(&self, instant: u64) -> f32 {
        let phase = std::f64::consts::TAU * self.freq_hz * (instant as f64) / self.rate_hz;
        (f64::from(self.amplitude) * phase.sin()) as f32
    }

    /// Fill `out` with the next whole frames, advancing the instant
    /// cursor. An unaligned tail (not a whole frame) is left
    /// **untouched** — the whole-frames discipline every other half of
    /// the pipeline enforces. Returns the frames filled.
    pub fn fill(&mut self, out: &mut [f32]) -> usize {
        let frames = out.len() / self.channels;
        for frame in 0..frames {
            let value = self.sample_at(self.next_instant + frame as u64);
            for channel in 0..self.channels {
                out[frame * self.channels + channel] = value;
            }
        }
        self.next_instant += frames as u64;
        frames
    }

    /// Sample instants emitted so far (monotonic).
    pub fn instants(&self) -> u64 {
        self.next_instant
    }

    /// Channels per frame.
    pub fn channels(&self) -> usize {
        self.channels
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::frame::{AudioLayout, SampleFormat};
    use testkit::Must;

    fn stereo_44k() -> AudioFormat {
        AudioFormat {
            sample: SampleFormat::F32,
            channels: 2,
            layout: AudioLayout::Interleaved,
            rate: 44_100,
        }
    }

    #[test]
    fn quarter_rate_tone_hits_the_exact_cardinal_values() {
        // freq = rate/4: phase = n·π/2 — the only points where sin is
        // exactly representable: 0, 1, 0, -1 (sin(π) is a tiny f64
        // residual, so only the odd/even zeros at multiples of π/2·odd
        // are asserted where sin is exact).
        let format = stereo_44k();
        let pattern = SinePattern::new(11_025.0, &format, 1.0).must();
        assert_eq!(pattern.sample_at(0), 0.0);
        assert_eq!(pattern.sample_at(1), 1.0);
        assert_eq!(pattern.sample_at(3), -1.0);
        assert_eq!(pattern.sample_at(5), 1.0);
    }

    #[test]
    fn fill_emits_whole_frames_and_leaves_the_tail_untouched() {
        let format = stereo_44k();
        let mut pattern = SinePattern::new(440.0, &format, 0.5).must();
        // 5 samples = 2 whole stereo frames + 1 unaligned slot.
        let mut out = [f32::NAN; 5];
        assert_eq!(pattern.fill(&mut out), 2);
        assert_eq!(pattern.instants(), 2);
        assert!(out[4].is_nan(), "the unaligned tail is untouched");
        // Oracle identity: both channels of frame n carry sample_at(n).
        assert_eq!(out[0], pattern.sample_at(0));
        assert_eq!(out[1], pattern.sample_at(0));
        assert_eq!(out[2], pattern.sample_at(1));
        assert_eq!(out[3], pattern.sample_at(1));
        // The next fill continues exactly where this one stopped.
        let mut next = [f32::NAN; 2];
        assert_eq!(pattern.fill(&mut next), 1);
        assert_eq!(next[0], pattern.sample_at(2));
        assert_eq!(next[1], pattern.sample_at(2));
    }

    #[test]
    fn the_oracle_is_a_pure_function_of_the_instant() {
        let format = stereo_44k();
        let mut a = SinePattern::new(440.0, &format, 0.5).must();
        let b = a.clone();
        let mut scratch = vec![0.0f32; 300];
        a.fill(&mut scratch);
        a.fill(&mut scratch);
        // However far `a` has generated, its oracle still names the
        // same absolute instants as a fresh pattern's.
        for instant in [0u64, 1, 42, 299, 599] {
            assert_eq!(a.sample_at(instant), b.sample_at(instant));
        }
    }

    #[test]
    fn invalid_shapes_are_typed() {
        let format = stereo_44k();
        for (freq, amp) in [
            (0.0, 0.5),
            (-440.0, 0.5),
            (22_050.0, 0.5), // Nyquist itself is invalid
            (30_000.0, 0.5),
            (440.0, 0.0),
            (440.0, 2.0),
        ] {
            let err = SinePattern::new(freq, &format, amp).err().must();
            assert_eq!(err.context.code, "audio.pattern_invalid", "{freq}Hz/{amp}");
        }
        let zero_rate = AudioFormat {
            rate: 0,
            ..stereo_44k()
        };
        assert!(SinePattern::new(440.0, &zero_rate, 0.5).is_err());
    }
}
