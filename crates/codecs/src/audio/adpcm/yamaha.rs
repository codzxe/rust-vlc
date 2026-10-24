//! Yamaha ADPCM (card 5.J2b): the 4-bit codec of Sega Dreamcast `.adp`
//! and a family of game containers — a **headerless continuous nibble
//! stream** with per-channel decoder state carried across packets, the
//! `adpcm_yamaha` row of doc 04 §3.1's ADPCM block.
//!
//! # The nibble law (pinned against the ffmpeg 7.1.5 reference)
//!
//! Every byte carries two nibbles. **Stereo:** the low nibble expands
//! channel 0 and the high nibble channel 1 — one sample instant per
//! byte. **Mono:** both nibbles expand channel 0, low first. The state
//! (`predictor`, `step`) starts at zero; the first nibble of the
//! stream hits ffmpeg's `if (!step)` reset (`predictor = 0`,
//! `step = 127`), so a fresh decoder and a just-reset one agree.
//!
//! ```text
//! predictor += (step · difflookup[nibble]) / 8      // C trunc-toward-0
//! predictor  = clamp_i16(predictor)
//! step       = (step · indexscale[nibble]) >> 8
//! step       = clamp(step, 127, 24576)
//! ```
//!
//! The `/8` (not `>>3`) matters: the product is negative for the high
//! half of the table and C division truncates toward zero while a
//! shift floors — Rust's `/` on integers truncates toward zero too, so
//! the port is exact (`div_euclid` would NOT be).
//!
//! # Oracle
//!
//! ffmpeg 7.1.5 has both an `adpcm_yamaha` encoder and decoder: the
//! committed fixtures are a 440 Hz sine the reference encoder produced
//! (NUT-carried, payload extracted verbatim), and this decoder is
//! bit-exact against the reference decoder's PCM on both the mono and
//! stereo paths — `tests/adpcm_breadth_differential.rs`.
//!
//! # Damage model
//!
//! There are no headers and no block structure: every byte is a lawful
//! nibble pair, so *damage cannot be detected, only decoded* — the
//! honest posture is to decode exactly what arrived (the J2a
//! block-concealment ledger degenerates to "frames appended"). A
//! truncated payload decodes its whole bytes (no partial nibble can
//! exist at byte granularity).

/// ffmpeg's `ff_adpcm_yamaha_difflookup`: the signed 4-bit magnitude
/// ladder (truncating division partner of `step`).
const DIFF_LOOKUP: [i8; 16] = [1, 3, 5, 7, 9, 11, 13, 15, -1, -3, -5, -7, -9, -11, -13, -15];

/// ffmpeg's `ff_adpcm_yamaha_indexscale`: the multiplicative step
/// update (note the symmetric halves — the published table differs
/// from the IMA one and from the half-remembered variants; pinned from
/// the 7.1.5 source).
const INDEX_SCALE: [i16; 16] = [
    230, 230, 230, 230, 307, 409, 512, 614, //
    230, 230, 230, 230, 307, 409, 512, 614,
];

/// Per-channel continuous decoder state (persists across packets; the
/// `step == 0` sentinel reproduces ffmpeg's first-nibble reset).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct YamahaChannel {
    /// Running predicted sample.
    pub predictor: i32,
    /// Running quantizer step (127 floor; 0 = fresh state).
    pub step: i32,
}

/// Expand one nibble (ffmpeg's `adpcm_yamaha_expand_nibble`, exact).
pub(crate) fn expand(state: &mut YamahaChannel, nibble: u8) -> i16 {
    if state.step == 0 {
        state.predictor = 0;
        state.step = 127;
    }
    state.predictor += (state.step * i32::from(DIFF_LOOKUP[usize::from(nibble)])) / 8;
    state.predictor = state.predictor.clamp(-32768, 32767);
    state.step = (state.step * i32::from(INDEX_SCALE[usize::from(nibble)])) >> 8;
    state.step = state.step.clamp(127, 24576);
    state.predictor as i16
}

/// What one `decode_yamaha` call appended.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct YamahaStats {
    /// Sample instants appended (all channels interleaved).
    pub frames: u64,
}

/// Decode a continuous Yamaha payload into `out` (native-endian i16,
/// channel-interleaved). `state` is the cross-packet channel state and
/// is updated in place. Channel packing: low nibble = channel 0, high
/// nibble = channel `channels-1` (mono: both, low first) — exactly
/// ffmpeg's `c->status[0]` / `c->status[st]` pair. Never panics.
pub(crate) fn decode_yamaha(
    data: &[u8],
    channels: usize,
    state: &mut [YamahaChannel],
    out: &mut Vec<u8>,
) -> YamahaStats {
    let st = channels.saturating_sub(1);
    let mut frames = 0u64;
    for &byte in data {
        let low = expand(&mut state[0], byte & 0x0F);
        let high = expand(&mut state[st], byte >> 4);
        out.extend_from_slice(&low.to_ne_bytes());
        out.extend_from_slice(&high.to_ne_bytes());
        frames += 1;
    }
    YamahaStats { frames }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples(out: &[u8]) -> Vec<i16> {
        out.as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_ne_bytes(*b))
            .collect()
    }

    #[test]
    fn tables_are_the_pinned_7_1_5_pair() {
        assert_eq!(DIFF_LOOKUP[0], 1);
        assert_eq!(DIFF_LOOKUP[8], -1);
        assert_eq!(DIFF_LOOKUP[15], -15);
        assert_eq!(INDEX_SCALE[4], 307);
        assert_eq!(INDEX_SCALE[7], 614);
        assert_eq!(INDEX_SCALE[8], 230, "the symmetric half is real");
        assert_eq!(INDEX_SCALE[11], 230);
        assert_eq!(INDEX_SCALE[15], 614);
    }

    #[test]
    fn fresh_state_seeds_127_and_zero() {
        // First nibble of a stream: predictor 0, step 127 — nibble 1
        // (difflookup 3): (127·3)/8 = 47 (trunc); step update
        // (127·230)>>8 = 114, but the 127 FLOOR clamps it back up —
        // the floor is why a fresh stream's step barely moves.
        let mut state = [YamahaChannel::default()];
        let sample = expand(&mut state[0], 1);
        assert_eq!(sample, 47);
        assert_eq!(state[0].step, 127, "(127·230)>>8 = 114 floors back to 127");
        // Nibble 8 (difflookup −1): (127·−1)/8 = −15 (trunc toward
        // zero) → 32; step again floors at 127.
        let sample = expand(&mut state[0], 8);
        assert_eq!(sample, 32);
        assert_eq!(state[0].step, 127);
    }

    #[test]
    fn truncating_division_is_the_negative_case() {
        // step 129, nibble 15 (−15): (129·−15)/8 = −241 (trunc toward
        // zero); a flooring shift would give −242 — the C-semantics pin.
        let mut state = [YamahaChannel {
            predictor: 0,
            step: 129,
        }];
        assert_eq!(expand(&mut state[0], 15), -241);
    }

    #[test]
    fn mono_two_samples_per_byte_low_first() {
        let mut state = [YamahaChannel::default()];
        let mut out = Vec::new();
        let stats = decode_yamaha(&[0x12], 1, &mut state, &mut out);
        assert_eq!(stats.frames, 1);
        // 2 samples for 1 mono byte.
        assert_eq!(samples(&out).len(), 2);
    }

    #[test]
    fn stereo_pair_keeps_channels_separate() {
        // Byte 0x18: the LOW nibble is 8 and the HIGH is 1 — channel
        // 0 takes the low (8: (127·−1)/8 = −15), channel 1 the high
        // (1: +47), and neither state leaks into the other.
        let mut state = [YamahaChannel::default(), YamahaChannel::default()];
        let mut out = Vec::new();
        let stats = decode_yamaha(&[0x18], 2, &mut state, &mut out);
        assert_eq!(stats.frames, 1);
        let got = samples(&out);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], -15, "low nibble (8) → channel 0");
        assert_eq!(got[1], 47, "high nibble (1) → channel 1");
    }

    #[test]
    fn state_persists_across_calls() {
        let mut state = [YamahaChannel::default()];
        let mut out = Vec::new();
        decode_yamaha(&[0x11], 1, &mut state, &mut out);
        let after_first = state[0];
        decode_yamaha(&[0x11], 1, &mut state, &mut out);
        assert_ne!(after_first, YamahaChannel::default());
        // The second byte starts from the first byte's state, not zero.
        assert_eq!(samples(&out).len(), 4);
        assert_eq!(state[0].predictor as i16, samples(&out)[3]);
    }
}
