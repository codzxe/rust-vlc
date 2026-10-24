//! IMA ADPCM (WAV flavor) block decode (card 5.J2a): the 89-entry
//! step table, the 7-entry index table, 4-byte-per-channel nibble
//! granules with the **low-nibble-first** packing — every decision
//! pinned bit-exactly against the ffmpeg 7.1 reference on the real
//! corpus (0042, `adpcm_ima_wav`, see
//! `tests/adpcm_corpus_differential.rs`).
//!
//! # The block grammar (as the corpus bytes actually carry it)
//!
//! One `nBlockAlign`-byte block starts with a **4-byte header per
//! channel** (predictor i16 LE, step index u8, reserved u8 — headers
//! for all channels first), then nibble data in **8-sample groups**:
//! per group, 4 consecutive bytes belong to one channel (channel 0's
//! group, then channel 1's, …), each byte holding two samples of that
//! channel — **low nibble first**, then high. The block decodes
//! `1 + (block_align − 4·channels)·2 / channels` sample instants.
//!
//! # The diff rounding (the one non-obvious bit)
//!
//! ffmpeg computes the nibble diff as a single multiply-then-shift,
//! not as the shifted partial sums the original DVI spec's C sample
//! shows:
//!
//! ```text
//! m    = 1 + 8·(n&4) + 4·(n&2) + 2·(n&1)
//! diff = (step · m) >> 3
//! ```
//!
//! The multiply folds `step>>3` in before the shift, so odd steps
//! round differently than the partial-sum form (step 7, nibble 7:
//! 13 here, 11 there). The corpus differential found this the hard
//! way; the form above is what matches the reference bit-for-bit.
//!
//! # The DVI/raw flavor (documented gap)
//!
//! The headerless, raw-nibble stream (`adpcm_ima` in ffprobe's
//! vocabulary — DVI/ROQ/AMI variants and the 2-byte-header DK3/DK4)
//! is **not** this decoder: those flavors carry no per-block headers
//! and pack nibbles channel-major. The codec-api gate accepts only
//! `adpcm_ima_wav` (the WAV `wFormatTag 0x0011` shape the demuxer
//! emits); the raw flavor is a typed `Unsupported` at configure,
//! owed to whatever card owns demuxing raw DVI containers.
//!
//! # Damage model
//!
//! A block shorter than its 4·channels header is **block-skipping**
//! damage ([`ImaStats::skipped_blocks`]); a step index above 88 is
//! clamped to 88 with the concealment flagged ([`ImaStats::clamped_index`]
//! — ffmpeg's own healing, kept for reference parity); a short group
//! tail decodes its whole bytes and flags [`ImaStats::concealed_tail`].

/// `wFormatTag` for IMA ADPCM, the WAV flavor (the WAV fmt identity).
pub(crate) const TAG_WAVE_FORMAT: u16 = 0x0011;

/// The IMA step table (89 entries, `step_table[88] = 32767`).
pub(crate) const STEP_TABLE: [u16; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

/// The step-index adjustment per nibble (indexed by `nibble & 7`).
const INDEX_TABLE: [i8; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

/// Maximum legal step index (the table's last slot).
pub(crate) const MAX_STEP_INDEX: usize = 88;

/// Bytes of one block header (4 per channel: predictor + index +
/// reserved).
pub(crate) fn header_bytes(channels: u32) -> usize {
    4 * channels as usize
}

/// Per-channel decoder state.
struct ChannelState {
    predictor: i32,
    step_index: usize,
}

impl ChannelState {
    fn expand(&mut self, nibble: u8) -> i16 {
        let step = i32::from(STEP_TABLE[self.step_index]);
        let m = 1
            + if nibble & 4 != 0 { 8 } else { 0 }
            + if nibble & 2 != 0 { 4 } else { 0 }
            + if nibble & 1 != 0 { 2 } else { 0 };
        let diff = (step * m) >> 3;
        let predictor = if nibble & 8 != 0 {
            self.predictor - diff
        } else {
            self.predictor + diff
        }
        .clamp(-32768, 32767);
        self.predictor = predictor;
        let next = self.step_index as i32 + i32::from(INDEX_TABLE[usize::from(nibble & 7)]);
        self.step_index = next.clamp(0, MAX_STEP_INDEX as i32) as usize;
        predictor as i16
    }
}

/// What one `decode_ima_blocks` call did (the damage ledger the
/// codec-api layer maps onto `DecodeOutcome`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ImaStats {
    /// Sample instants (all channels interleaved) appended.
    pub frames: u64,
    /// Blocks dropped whole (shorter than 4·channels).
    pub skipped_blocks: u32,
    /// A header step index above 88 was clamped (concealment).
    pub clamped_index: bool,
    /// A partial group tail decoded its whole bytes.
    pub concealed_tail: bool,
}

/// The configure-time IMA facts (parsed WAVEFORMATEX slice).
#[derive(Debug, Clone)]
pub(crate) struct ImaFacts {
    /// Channel count.
    pub channels: u32,
}

/// Decode IMA-WAV blocks from `data` into `out` (native-endian i16
/// samples, channel-interleaved). Blocks are `block_align` bytes; a
/// short trailing block is prefix-decoded (concealed). Never panics;
/// every index is bounds-checked.
pub(crate) fn decode_ima_blocks(
    data: &[u8],
    facts: &ImaFacts,
    block_align: usize,
    out: &mut Vec<u8>,
) -> ImaStats {
    let channels = facts.channels as usize;
    let header = header_bytes(facts.channels);
    let mut stats = ImaStats::default();
    for block in data.chunks(block_align.max(1)) {
        if block.len() < header {
            if !block.is_empty() {
                stats.skipped_blocks += 1;
            }
            continue;
        }
        let mut states = Vec::with_capacity(channels);
        let mut clamped = false;
        for ch in 0..channels {
            let base = 4 * ch;
            let predictor = i16::from_le_bytes([block[base], block[base + 1]]) as i32;
            let index = usize::from(block[base + 2]);
            let index = if index > MAX_STEP_INDEX {
                clamped = true;
                MAX_STEP_INDEX
            } else {
                index
            };
            states.push(ChannelState {
                predictor,
                step_index: index,
            });
        }
        stats.clamped_index |= clamped;
        for state in &states {
            push_sample(out, state.predictor as i16);
        }
        stats.frames += 1;
        // Nibble payload in 8-sample groups: 4 bytes per channel per
        // group, low nibble first inside each byte. The group is the
        // channel-major unit; the output is sample-interleaved, so
        // each group is walked instant-major (two nibbles per byte
        // read back) with no intermediate buffering.
        let nibble_bytes = &block[header..];
        if block.len() < block_align {
            stats.concealed_tail = true;
        }
        let group = 4 * channels; // bytes per 8-sample group, all channels
        let mut g = 0;
        while g < nibble_bytes.len() {
            let group_end = (g + group).min(nibble_bytes.len());
            let group_bytes = &nibble_bytes[g..group_end];
            // Honest prefix of a damaged tail group: the sample count
            // every channel can still fully supply (channels whose
            // 4-byte slice is cut short bound the interleave).
            let per_group = (0..channels)
                .map(|ch| 2 * group_bytes.len().saturating_sub(4 * ch).min(4))
                .min()
                .unwrap_or(0);
            for j in 0..per_group {
                for ch in 0..channels {
                    let byte = group_bytes[4 * ch + j / 2];
                    let nibble = if j % 2 == 0 { byte & 0x0F } else { byte >> 4 };
                    push_sample(out, states[ch].expand(nibble));
                }
            }
            stats.frames += per_group as u64;
            g += group;
        }
    }
    stats
}

/// Append one i16 sample in native byte order (the
/// `SampleFormat::S16` contract).
fn push_sample(out: &mut Vec<u8>, sample: i16) {
    out.extend_from_slice(&sample.to_ne_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_table_is_the_published_89_entry_ladder() {
        assert_eq!(STEP_TABLE.len(), 89);
        assert_eq!(STEP_TABLE[0], 7);
        assert_eq!(STEP_TABLE[88], 32767);
        // Monotone non-decreasing (the decoder's clamp relies on it
        // only for honesty; the pin is the byte values themselves).
        for pair in STEP_TABLE.windows(2) {
            assert!(pair[0] <= pair[1], "{pair:?}");
        }
    }

    #[test]
    fn mono_block_decodes_the_pinned_ladder() {
        // Hand-verified against ffmpeg: predictor 2314, index 0,
        // nibbles 0x7F 0xFF — low nibble first: 15, 7, 15, 15.
        let mut block = Vec::new();
        block.extend_from_slice(&2314i16.to_le_bytes());
        block.push(0); // step index
        block.push(0); // reserved
        block.extend_from_slice(&[0x7F, 0xFF]);
        let mut out = Vec::new();
        let stats = decode_ima_blocks(&block, &ImaFacts { channels: 1 }, block.len(), &mut out);
        let samples: Vec<i16> = out
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_ne_bytes(*b))
            .collect();
        // 2314, then −13 (step 7 · m 15 >> 3), +30 (step 16), −63
        // (step 34), −136 (step 73): the multiply-before-shift ladder.
        assert_eq!(samples, [2314, 2301, 2331, 2268, 2132]);
        assert_eq!(stats.frames, 5);
        assert_eq!(stats.skipped_blocks, 0);
        assert!(!stats.clamped_index);
    }

    #[test]
    fn header_index_above_88_clamps_and_flags() {
        let mut block = Vec::new();
        block.extend_from_slice(&0i16.to_le_bytes());
        block.push(200); // out of table
        block.push(0);
        block.extend_from_slice(&[0x00]);
        let mut out = Vec::new();
        let stats = decode_ima_blocks(&block, &ImaFacts { channels: 1 }, block.len(), &mut out);
        assert!(stats.clamped_index);
        assert_eq!(stats.skipped_blocks, 0);
    }

    #[test]
    fn block_smaller_than_header_is_skipped_not_panicked() {
        let mut out = Vec::new();
        let stats = decode_ima_blocks(&[0u8; 3], &ImaFacts { channels: 1 }, 1024, &mut out);
        assert_eq!(stats.skipped_blocks, 1);
        assert!(out.is_empty());
        let stats = decode_ima_blocks(&[], &ImaFacts { channels: 2 }, 0, &mut out);
        assert_eq!(stats, ImaStats::default());
    }

    #[test]
    fn stereo_group_interleave_keeps_channels_separate() {
        // Two channels with opposite predictor seeds and a symmetric
        // nibble payload: the 4-byte granules must not cross wires.
        let mut block = Vec::new();
        block.extend_from_slice(&1000i16.to_le_bytes());
        block.extend_from_slice(&[0, 0]);
        block.extend_from_slice(&(-1000i16).to_le_bytes());
        block.extend_from_slice(&[0, 0]);
        // ch0 granule 0x7F ×4, ch1 granule 0x00 ×4.
        block.extend_from_slice(&[0x7F, 0x7F, 0x7F, 0x7F]);
        block.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        let mut out = Vec::new();
        let stats = decode_ima_blocks(&block, &ImaFacts { channels: 2 }, block.len(), &mut out);
        let samples: Vec<i16> = out
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_ne_bytes(*b))
            .collect();
        // Even indices = ch0 (predictor 1000 moves: nibble 15 at
        // step 7 is −13), odd = ch1 (nibble 0 at step 7 shifts by
        // (7·1)>>3 = 0 — it stays put).
        assert_eq!(samples[0], 1000);
        assert_eq!(samples[1], -1000);
        assert_eq!(samples[2], 987);
        assert_eq!(samples[3], -1000);
        assert_eq!(stats.frames, 9); // 1 seed + 8 group samples
    }
}
