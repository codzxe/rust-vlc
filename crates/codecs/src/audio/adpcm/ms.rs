//! MS-ADPCM block decode (card 5.J2a): the 7-pair coefficient table,
//! per-channel block headers, nibble expansion with the multiplicative
//! delta adaptation, and the stereo nibble interleave — every packing
//! decision pinned bit-exactly against the ffmpeg 7.1 reference on the
//! real corpus (0102, `adpcm_ms`, see `tests/adpcm_corpus_differential.rs`).
//!
//! # The block grammar (as the corpus bytes actually carry it)
//!
//! One `nBlockAlign`-byte block decodes `(block_align − 7·channels)`
//! nibble bytes → that many extra sample instants, plus 2 header
//! samples per channel. The header is **field-interleaved per channel**
//! (the layout ffmpeg and libsndfile both read):
//!
//! ```text
//! bPredictor[ch]      (u8 each channel, first)
//! iDelta[ch]          (i16 LE each channel)
//! iSamp1[ch]          (i16 LE each channel)   ← newest
//! iSamp2[ch]          (i16 LE each channel)   ← oldest
//! ```
//!
//! Output order: sample2 (the older seed), sample1, then nibble
//! samples, all channel-interleaved. The nibble payload packs **one
//! instant per byte** — high nibble channel 0, low nibble channel 1
//! (mono: high then low of the same channel).
//!
//! # Nibble expansion (ffmpeg `adpcm_ms_expand_nibble` parity)
//!
//! ```text
//! predictor = (samp1·coef1 + samp2·coef2) / 256          (trunc → 0)
//!           ± ((nibble ≥ 8) ? nibble−16 : nibble) · delta
//! clamp to i16
//! delta = max(delta · ADAPTATION[nibble] / 256, 16)      (trunc → 0)
//! ```
//!
//! Damage model: a predictor index above 6, or a block shorter than
//! its own header, is **block-skipping** damage (the block is dropped,
//! counted in [`MsStats::skipped_blocks`]); a short nibble tail decodes
//! its whole-byte prefix and flags [`MsStats::concealed_tail`]. Both
//! are concealment at the codec-api layer ([`DecodeOutcome::Recovered`]
//! shapes), never panics and never silent output.

/// `wFormatTag` for MS ADPCM (the WAV fmt identity).
pub(crate) const TAG_WAVE_FORMAT: u16 = 0x0002;

/// The 7 adaptation-coefficient pairs, MS-ADPCM's fixed table (the
/// corpus fmt chunk carries exactly these bytes — read from the file
/// per ffmpeg, cross-checked against the constant at configure).
pub(crate) const STANDARD_COEFFICIENTS: [(i32, i32); 7] = [
    (256, 0),
    (512, -256),
    (0, 0),
    (192, 64),
    (240, 0),
    (460, -208),
    (392, -232),
];

/// The nibble adaptation table (`iDelta` update numerator).
const ADAPTATION: [i32; 16] = [
    230, 230, 230, 230, 307, 409, 512, 614, 768, 614, 512, 409, 307, 230, 230, 230,
];

/// Maximum legal `bPredictor` index (7 coefficient pairs ⇒ 0..=6).
pub(crate) const MAX_PREDICTOR_INDEX: usize = 6;

/// Bytes of one block header (7 per channel: predictor + delta +
/// sample1 + sample2).
pub(crate) fn header_bytes(channels: u32) -> usize {
    7 * channels as usize
}

/// Per-channel decoder state (the block-local predictor/delta ladder).
struct ChannelState {
    coef: (i32, i32),
    delta: i32,
    sample1: i32,
    sample2: i32,
}

impl ChannelState {
    fn expand(&mut self, nibble: u8) -> i16 {
        let signed = if nibble >= 8 {
            i32::from(nibble) - 16
        } else {
            i32::from(nibble)
        };
        // i64 intermediates: every operand is i16-bounded except
        // `delta`, whose ×3-per-nibble growth is unbounded in the
        // reference (a C `int` that wraps — UB we refuse to mirror);
        // the clamp below is our typed concealment bound instead.
        let predictor = (i64::from(self.sample1) * i64::from(self.coef.0)
            + i64::from(self.sample2) * i64::from(self.coef.1))
            / 256
            + i64::from(signed) * i64::from(self.delta);
        let predictor = predictor.clamp(-32768, 32767);
        self.sample2 = self.sample1;
        self.sample1 = predictor as i32;
        // C-truncation semantics: `/` truncates toward zero, the
        // same as ffmpeg's integer divide — a negative header delta
        // heals to the 16 floor here. The upper clamp (32767) is
        // panic-freedom on pathological growth: ffmpeg's `int`
        // silently wraps there; we bound. Clean streams never reach
        // it (the corpus and fixtures stay four orders below).
        let delta = (i64::from(self.delta) * i64::from(ADAPTATION[usize::from(nibble)])) / 256;
        self.delta = delta.clamp(16, 32_767) as i32;
        predictor as i16
    }
}

/// What one `decode_ms_blocks` call did (the damage ledger the
/// codec-api layer maps onto `DecodeOutcome`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MsStats {
    /// Sample instants (all channels interleaved) appended.
    pub frames: u64,
    /// Blocks dropped whole: header shorter than 7·channels, or a
    /// predictor index above 6 (out of the coefficient table).
    pub skipped_blocks: u32,
    /// A final partial block decoded its whole-nibble prefix.
    pub concealed_tail: bool,
}

/// The configure-time MS facts (parsed WAVEFORMATEX slice).
#[derive(Debug, Clone)]
pub(crate) struct MsFacts {
    /// Channel count.
    pub channels: u32,
    /// The file's own coefficient table (7 pairs, validated equal to
    /// [`STANDARD_COEFFICIENTS`] range-wise at configure).
    pub coefficients: [(i32, i32); 7],
}

/// Decode MS-ADPCM blocks from `data` into `out` (native-endian i16
/// samples, channel-interleaved). Blocks are `block_align` bytes; a
/// short trailing block is prefix-decoded (concealed), a block too
/// short for its header — or carrying an out-of-table predictor — is
/// skipped and counted. Never panics; every index is bounds-checked.
pub(crate) fn decode_ms_blocks(
    data: &[u8],
    facts: &MsFacts,
    block_align: usize,
    out: &mut Vec<u8>,
) -> MsStats {
    let channels = facts.channels as usize;
    let header = header_bytes(facts.channels);
    let mut stats = MsStats::default();
    for block in data.chunks(block_align.max(1)) {
        if block.len() < header {
            if !block.is_empty() {
                stats.skipped_blocks += 1;
            }
            continue;
        }
        // Field-interleaved per-channel header.
        let mut states = Vec::with_capacity(channels);
        for ch in 0..channels {
            let predictor = block[ch];
            if usize::from(predictor) > MAX_PREDICTOR_INDEX {
                // Out of the coefficient table: ffmpeg errors the
                // frame; we conceal by skipping the block (bounded,
                // counted, decode continues).
                stats.skipped_blocks += 1;
                states.clear();
                break;
            }
            states.push(ChannelState {
                coef: facts.coefficients[usize::from(predictor)],
                delta: i16::from_le_bytes([block[channels + 2 * ch], block[channels + 2 * ch + 1]])
                    as i32,
                sample1: 0,
                sample2: 0,
            });
        }
        if states.len() != channels {
            continue;
        }
        let mut off = 3 * channels;
        for state in &mut states {
            state.sample1 = i16::from_le_bytes([block[off], block[off + 1]]) as i32;
            off += 2;
        }
        for state in &mut states {
            state.sample2 = i16::from_le_bytes([block[off], block[off + 1]]) as i32;
            off += 2;
        }
        // Seed samples, oldest first, channel-interleaved.
        for state in &states {
            push_sample(out, state.sample2 as i16);
        }
        for state in &states {
            push_sample(out, state.sample1 as i16);
        }
        stats.frames += 2;
        // Nibble payload: one instant per byte (hi = ch0, lo = ch1;
        // mono: hi then lo of the single channel). A short trailing
        // block contributes exactly the whole bytes it has.
        if block.len() < block_align {
            stats.concealed_tail = true;
        }
        let nibble_bytes = &block[header..];
        match channels {
            1 => {
                for byte in nibble_bytes {
                    push_sample(out, states[0].expand(byte >> 4));
                    push_sample(out, states[0].expand(byte & 0x0F));
                }
                stats.frames += 2 * nibble_bytes.len() as u64;
            }
            _ => {
                for byte in nibble_bytes {
                    push_sample(out, states[0].expand(byte >> 4));
                    push_sample(out, states[1].expand(byte & 0x0F));
                }
                stats.frames += nibble_bytes.len() as u64;
            }
        }
    }
    stats
}

/// Append one i16 sample in native byte order (the `SampleFormat::S16`
/// contract; equals little-endian on every host the differential ran
/// on).
fn push_sample(out: &mut Vec<u8>, sample: i16) {
    out.extend_from_slice(&sample.to_ne_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> MsFacts {
        MsFacts {
            channels: 1,
            coefficients: STANDARD_COEFFICIENTS,
        }
    }

    #[test]
    fn standard_table_matches_the_published_pairs() {
        // The corpus 0102 fmt chunk bytes (pinned this session).
        assert_eq!(
            STANDARD_COEFFICIENTS,
            [
                (256, 0),
                (512, -256),
                (0, 0),
                (192, 64),
                (240, 0),
                (460, -208),
                (392, -232)
            ]
        );
    }

    #[test]
    fn mono_block_decodes_the_pinned_ladder() {
        // Hand-verified against ffmpeg: predictor 0 (coef 256/0),
        // delta 16, samp1 367, samp2 −226, nibbles 8,8,7,7 (0x88 0x77).
        let mut block = vec![0u8];
        block.extend_from_slice(&16i16.to_le_bytes());
        block.extend_from_slice(&367i16.to_le_bytes());
        block.extend_from_slice(&(-226i16).to_le_bytes());
        block.extend_from_slice(&[0x88, 0x77]);
        let mut out = Vec::new();
        let stats = decode_ms_blocks(&block, &facts(), block.len(), &mut out);
        // samp2, samp1, then nibble samples 239, −145, 863, 3278 —
        // the multiplicative delta (16→48→144→345) is the ladder.
        let samples: Vec<i16> = out
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_ne_bytes(*b))
            .collect();
        assert_eq!(samples, [-226, 367, 239, -145, 863, 3278]);
        assert_eq!(stats.frames, 6);
        assert_eq!(stats.skipped_blocks, 0);
        assert!(!stats.concealed_tail);
    }

    #[test]
    fn predictor_out_of_table_skips_the_block_typed() {
        let mut block = vec![7u8]; // one past the 7-pair table
        block.extend_from_slice(&16i16.to_le_bytes());
        block.extend_from_slice(&0i16.to_le_bytes());
        block.extend_from_slice(&0i16.to_le_bytes());
        block.extend_from_slice(&[0x88, 0x77]);
        let mut out = Vec::new();
        let stats = decode_ms_blocks(&block, &facts(), block.len(), &mut out);
        assert_eq!(stats.skipped_blocks, 1);
        assert_eq!(stats.frames, 0);
        assert!(out.is_empty());
    }

    #[test]
    fn block_smaller_than_header_is_skipped_not_panicked() {
        let mut out = Vec::new();
        let stats = decode_ms_blocks(&[0u8; 4], &facts(), 1024, &mut out);
        assert_eq!(stats.skipped_blocks, 1);
        assert!(out.is_empty());
        // Zero-length input: nothing at all, no hang.
        let stats = decode_ms_blocks(&[], &facts(), 0, &mut out);
        assert_eq!(stats, MsStats::default());
    }

    #[test]
    fn negative_header_delta_heals_to_the_floor() {
        // delta −300: the first expand clamps the update to 16.
        let mut block = vec![0u8];
        block.extend_from_slice(&(-300i16).to_le_bytes());
        block.extend_from_slice(&1000i16.to_le_bytes());
        block.extend_from_slice(&1000i16.to_le_bytes());
        block.extend_from_slice(&[0x00, 0x00]);
        let mut out = Vec::new();
        let _ = decode_ms_blocks(&block, &facts(), block.len(), &mut out);
        let samples: Vec<i16> = out
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_ne_bytes(*b))
            .collect();
        // nibble 0 with delta −300: 1000·256/256 + 0·(−300) = 1000.
        assert_eq!(samples, [1000, 1000, 1000, 1000, 1000, 1000]);
    }
}
