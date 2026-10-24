//! OKI / Dialogic ADPCM (card 5.J2b): the 4-bit codec of Dialogic VOX
//! telephony files and the OKI MSM6295-style silicon — the
//! `adpcm_ima_oki` row of doc 04 §3.1's ADPCM block.
//!
//! # The nibble law (pinned against the ffmpeg 7.1.5 reference)
//!
//! A **headerless continuous nibble stream** like Yamaha's, but with
//! three OKI-specific twists (ffmpeg's `adpcm_ima_oki_expand_nibble`,
//! exact):
//!
//! ```text
//! step       = oki_step_table[step_index]        // 49-entry, max 1552
//! step_index = clamp(step_index + index_table[nibble], 0, 48)
//! diff       = ((2·(nibble & 7) + 1) · step) >> 3
//! predictor  = clamp[pred ± diff, -2048, 2047]   // 12-bit accumulator
//! output     = predictor · 16                    // scaled to 16 bits
//! ```
//!
//! The accumulator is **12-bit** and the output is the predictor
//! shifted up four places — a VOX decoder speaks a 12-bit world into a
//! 16-bit bus. Channel packing is the mirror of Yamaha's: **high
//! nibble first** (channel 0 = `v >> 4`, channel 1 = `v & 0x0F`).
//!
//! # The lettered gap (`adpcm_oki`, 32 kHz / 24 kHz variants)
//!
//! The card's scope names `adpcm_oki` (the 32k/24k "lettered" OKI
//! variants). **ffmpeg 7.1.5 ships neither a decoder nor an encoder
//! for them** (verified this session: `-decoders`/`-encoders` list
//! only `adpcm_ima_oki`), so no differential oracle exists on this
//! reference build. Implementing unpublished tables from memory would
//! violate the bit-exact discipline this module is held to — the
//! flavor is a typed `Unsupported` at the gate
//! (`adpcm.oki.no_reference`) with this note as the honest ledger
//! entry, owed to whatever session can pin real vectors.
//!
//! # Oracle (what IS verified)
//!
//! The committed fixtures are hand-crafted vectors (a 512-byte
//! deterministic ramp — every nibble value, both channels) and their
//! PCM was measured through ffmpeg's forced decoder
//! (`-c:a adpcm_ima_oki -f s16le`): this decoder is bit-exact on both
//! paths — `tests/adpcm_breadth_differential.rs`.

/// ffmpeg's `oki_step_table` (49 entries, `step_table[48] = 1552`).
const OKI_STEP_TABLE: [i16; 49] = [
    16, 17, 19, 21, 23, 25, 28, 31, 34, 37, //
    41, 45, 50, 55, 60, 66, 73, 80, 88, 97, //
    107, 118, 130, 143, 157, 173, 190, 209, 230, 253, //
    279, 307, 337, 371, 408, 449, 494, 544, 598, 658, //
    724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552,
];

/// ffmpeg's `ff_adpcm_index_table` (16 entries; OKI indexes it with
/// the full nibble and clamps to 48 — the table's own length).
const INDEX_TABLE: [i8; 16] = [
    -1, -1, -1, -1, 2, 4, 6, 8, //
    -1, -1, -1, -1, 2, 4, 6, 8,
];

/// Maximum legal OKI step index (the table's last slot).
const MAX_STEP_INDEX: usize = 48;

/// Per-channel continuous decoder state (persists across packets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct OkiChannel {
    /// Running 12-bit predicted sample.
    pub predictor: i32,
    /// Running step index (0..=48).
    pub step_index: usize,
}

/// Expand one nibble (ffmpeg's `adpcm_ima_oki_expand_nibble`, exact).
pub(crate) fn expand(state: &mut OkiChannel, nibble: u8) -> i16 {
    let step = i32::from(OKI_STEP_TABLE[state.step_index]);
    let next = (state.step_index as i32 + i32::from(INDEX_TABLE[usize::from(nibble)]))
        .clamp(0, MAX_STEP_INDEX as i32) as usize;
    let delta = i32::from(nibble & 7);
    let diff = ((2 * delta + 1) * step) >> 3;
    let predictor = if nibble & 8 != 0 {
        state.predictor - diff
    } else {
        state.predictor + diff
    };
    // av_clip_intp2(predictor, 11): the 12-bit VOX accumulator.
    state.predictor = predictor.clamp(-2048, 2047);
    state.step_index = next;
    (state.predictor * 16) as i16
}

/// What one `decode_ima_oki` call appended.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct OkiStats {
    /// Sample instants appended (all channels interleaved).
    pub frames: u64,
}

/// Decode a continuous OKI payload into `out` (native-endian i16,
/// channel-interleaved). `state` is the cross-packet channel state and
/// is updated in place. Channel packing: **high nibble first** —
/// channel 0 = `v >> 4`, channel 1 = `v & 0x0F`. Never panics.
pub(crate) fn decode_ima_oki(
    data: &[u8],
    channels: usize,
    state: &mut [OkiChannel],
    out: &mut Vec<u8>,
) -> OkiStats {
    let st = channels.saturating_sub(1);
    let mut frames = 0u64;
    for &byte in data {
        let high = expand(&mut state[0], byte >> 4);
        let low = expand(&mut state[st], byte & 0x0F);
        out.extend_from_slice(&high.to_ne_bytes());
        out.extend_from_slice(&low.to_ne_bytes());
        frames += 1;
    }
    OkiStats { frames }
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
    fn step_table_is_the_pinned_49_entry_ladder() {
        assert_eq!(OKI_STEP_TABLE.len(), 49);
        assert_eq!(OKI_STEP_TABLE[0], 16);
        // The ladder is the IMA table's 8..57 window: OKI[10] is 41
        // (the IMA table's own 41-slot), and 107 only arrives at 20 —
        // not at IMA's 28, not at a memory-guessed 10.
        assert_eq!(OKI_STEP_TABLE[10], 41);
        assert_eq!(OKI_STEP_TABLE[20], 107);
        assert_eq!(OKI_STEP_TABLE[48], 1552, "OKI tops out at 1552, not 32767");
        for pair in OKI_STEP_TABLE.windows(2) {
            assert!(pair[0] <= pair[1], "{pair:?}");
        }
    }

    #[test]
    fn first_nibble_ladder_is_hand_verified() {
        // Fresh state: step 16 (index 0), predictor 0.
        // nibble 0: diff = (1·16)>>3 = 2 → 2; output 2·16 = 32;
        //   index 0 + (-1) → clamped to 0.
        // nibble 7: diff = (15·16)>>3 = 30 → 32; output 512; index → 8.
        let mut state = [OkiChannel::default()];
        assert_eq!(expand(&mut state[0], 0), 32);
        assert_eq!(state[0].step_index, 0);
        assert_eq!(expand(&mut state[0], 7), 512);
        assert_eq!(state[0].step_index, 8);
        assert_eq!(state[0].predictor, 32);
    }

    #[test]
    fn accumulator_is_twelve_bit_and_output_scaled() {
        // Drive the predictor to the rail: big steps down from a high
        // start saturate at −2048; a further −nibble stays pinned there
        // and the output is −32768, never a wrap. A +nibble at max step
        // (1552) lifts it by ((15)·1552)>>3 = 2910 → −2048+2910 = 862
        // → output 862·16 = 13792.
        let mut state = [OkiChannel {
            predictor: 2000,
            step_index: MAX_STEP_INDEX,
        }];
        for _ in 0..64 {
            expand(&mut state[0], 0x0F);
        }
        assert_eq!(state[0].predictor, -2048);
        assert_eq!(expand(&mut state[0], 0x0F), -32768);
        let mut state = [OkiChannel {
            predictor: -2048,
            step_index: MAX_STEP_INDEX,
        }];
        assert_eq!(expand(&mut state[0], 0x07), 13792);
    }

    #[test]
    fn index_clamps_at_both_rails() {
        let mut state = [OkiChannel {
            predictor: 0,
            step_index: 46,
        }];
        expand(&mut state[0], 0x07); // +8 → 54 → clamped to 48
        assert_eq!(state[0].step_index, 48);
        for _ in 0..48 {
            expand(&mut state[0], 0x00); // −1 each, floor 0
        }
        assert_eq!(state[0].step_index, 0);
    }

    #[test]
    fn channel_order_is_high_nibble_first() {
        // Byte 0x18: the HIGH nibble is 1 and the LOW nibble is 8 —
        // channel 0 takes the high (1: +6 → 96), channel 1 the low
        // (8: −2 → −32) — the mirror of Yamaha's packing.
        let mut state = [OkiChannel::default(), OkiChannel::default()];
        let mut out = Vec::new();
        let stats = decode_ima_oki(&[0x18], 2, &mut state, &mut out);
        assert_eq!(stats.frames, 1);
        let got = samples(&out);
        // ch0: nibble 1 → ((2·1+1)·16)>>3 = 6 → 6·16 = 96;
        // ch1: nibble 8 → −((1)·16)>>3 = −2 → −2·16 = −32.
        assert_eq!(got[0], 96);
        assert_eq!(got[1], -32);
    }

    #[test]
    fn state_persists_across_calls() {
        let mut state = [OkiChannel::default()];
        let mut out = Vec::new();
        decode_ima_oki(&[0x77], 1, &mut state, &mut out);
        decode_ima_oki(&[0x77], 1, &mut state, &mut out);
        // 2 samples per byte, mono; the OUTPUT is the 12-bit
        // accumulator scaled ×16, so the pin compares the scaled
        // predictor against the last sample (523·16 = 8368).
        assert_eq!(samples(&out).len(), 4);
        assert_eq!((state[0].predictor * 16) as i16, samples(&out)[3]);
    }
}
