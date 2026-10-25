//! The misc ADPCM pair (card 5.J2b): Apple QuickTime IMA4
//! (`adpcm_ima_qt`) and Westwood Studios IMA (`adpcm_ima_ws`) — the
//! last two rows of this card's doc 04 §3.1 ADPCM block.
//!
//! # Apple IMA4 (`adpcm_ima_qt`) — 34-byte channel chunks
//!
//! QuickTime carries IMA in **chunks of 34 bytes per channel** = 64
//! samples: a 2-byte big-endian header followed by 32 nibble bytes.
//! The header packs the seed into one 16-bit word:
//!
//! ```text
//! be16 = (predictor << 7) | step_index      // top 9 bits / low 7
//! ```
//!
//! and — the non-obvious bit — ffmpeg applies a **continuation
//! heuristic**: when the chunk's header index equals the running index
//! *and* the header predictor is within 0x7F of the running predictor,
//! the running state is kept and the header values are ignored (a
//! glitch-tolerant seam between consecutive chunks). Only on a real
//! jump does the header re-seed the state. Nibbles are low-first, and
//! the reconstruction is the **partial-sum** IMA form — which rounds
//! odd steps differently from the multiply form (see [`swf`], whose
//! table this shares).
//!
//! Oracle: ffmpeg 7.1.5 has an `adpcm_ima_qt` encoder — the committed
//! fixture is a stereo 440 Hz sine the reference encoder produced
//! (MOV-carried, per-chunk payloads extracted verbatim), bit-exact
//! through this decoder (`tests/adpcm_breadth_differential.rs`).
//!
//! # Westwood IMA (`adpcm_ima_ws`) — the AUD2 plain flavor
//!
//! A **headerless continuous nibble stream** (like Yamaha/OKI) with
//! the standard IMA 89-entry step table and the multiply-form
//! reconstruction `((2·delta+1)·step) >> 3`. The channel packing is
//! per-instant-pair: for each pair of instants, one byte per channel
//! — low nibble = the earlier instant, high = the later.
//!
//! ffmpeg's decoder has a second flavor selected by container
//! extradata (`vqa_version == 3`, VQA's planar variant with a
//! +0x1000 predictor bias); the extradata seam belongs to the future
//! VQA/AUD demuxer card, so this decoder implements the plain AUD2
//! flavor and the vqa3 shape is a documented gap.
//!
//! Oracle: no container in ffmpeg 7.1.5 muxes `adpcm_ima_ws` (the
//! encoder exists but has no muxer), so the committed vectors are
//! hand-crafted (the same 512-byte deterministic ramp as OKI's) and
//! verified bit-exact against ffmpeg's **forced decoder**
//! (`-c:a adpcm_ima_ws -f s16le`) on both mono and stereo paths.
//!
//! # Damage model
//!
//! An IMA4 packet shorter than `34·channels` is skipped whole
//! (ffmpeg refuses it with `AVERROR_INVALIDDATA`; here the refusal is
//! the typed `Recovered` concealment — the block-skip precedent). A
//! header index above 88 is the same skip (ffmpeg's hard error). The
//! Westwood stream has no headers to damage — every byte decodes.

/// The IMA 89-entry step table (shared identity with the IMA-WAV and
/// SWF flavors; duplicated here deliberately — each flavor module is
/// self-contained and independently pinnable, the J2a module map).
pub(crate) const STEP_TABLE: [i16; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

/// ffmpeg's `ff_adpcm_index_table` (16 entries).
const INDEX_TABLE: [i8; 16] = [
    -1, -1, -1, -1, 2, 4, 6, 8, //
    -1, -1, -1, -1, 2, 4, 6, 8,
];

/// Maximum legal IMA step index.
const MAX_STEP_INDEX: usize = 88;

/// Bytes of one IMA4 chunk per channel (2-byte header + 32 nibble
/// bytes = 64 samples).
const QT_CHUNK_BYTES: usize = 34;

/// Expand one IMA4 nibble (ffmpeg's `adpcm_ima_qt_expand_nibble`:
/// the partial-sum form).
fn qt_expand(state: &mut WsChannel, nibble: u8) -> i16 {
    let step = i32::from(STEP_TABLE[state.step_index]);
    let mut diff = step >> 3;
    if nibble & 4 != 0 {
        diff += step;
    }
    if nibble & 2 != 0 {
        diff += step >> 1;
    }
    if nibble & 1 != 0 {
        diff += step >> 2;
    }
    let predictor = if nibble & 8 != 0 {
        state.predictor - diff
    } else {
        state.predictor + diff
    };
    state.predictor = predictor.clamp(-32768, 32767);
    state.step_index = (state.step_index as i32 + i32::from(INDEX_TABLE[usize::from(nibble)]))
        .clamp(0, 88) as usize;
    state.predictor as i16
}

/// Expand one Westwood nibble (ffmpeg's `adpcm_ima_expand_nibble`
/// with `shift = 3`: the multiply form).
fn ws_expand(state: &mut WsChannel, nibble: u8) -> i16 {
    let step = i32::from(STEP_TABLE[state.step_index]);
    let delta = i32::from(nibble & 7);
    let diff = ((2 * delta + 1) * step) >> 3;
    let predictor = if nibble & 8 != 0 {
        state.predictor - diff
    } else {
        state.predictor + diff
    };
    state.predictor = predictor.clamp(-32768, 32767);
    state.step_index = (state.step_index as i32 + i32::from(INDEX_TABLE[usize::from(nibble)]))
        .clamp(0, 88) as usize;
    state.predictor as i16
}

/// Per-channel state (IMA4's cross-packet continuation heuristic and
/// Westwood's continuous stream both live in the decoder).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct WsChannel {
    /// Running predicted sample.
    pub predictor: i32,
    /// Running step index (0..=88).
    pub step_index: usize,
}

/// What one misc decode call did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MiscStats {
    /// Sample instants appended (all channels interleaved).
    pub frames: u64,
    /// Packets dropped whole (short chunk, or header index > 88).
    pub skipped_packets: u32,
}

/// Decode one IMA4 packet (a 34-byte chunk per channel; trailing
/// bytes beyond `34·channels` are ignored, as in the reference) into
/// `out`. `state` carries each channel's running predictor/index
/// across packets for the continuation heuristic. Never panics.
pub(crate) fn decode_ima_qt_packet(
    data: &[u8],
    channels: usize,
    state: &mut [WsChannel],
    out: &mut Vec<u8>,
) -> MiscStats {
    let mut stats = MiscStats::default();
    let chunk_total = QT_CHUNK_BYTES * channels;
    if data.len() < chunk_total {
        stats.skipped_packets = u32::from(!data.is_empty());
        return stats;
    }
    // Per-channel 64-sample chunks, written into a scratch and then
    // interleaved (the reference writes planar; the codec-api frame is
    // interleaved).
    let mut scratch = vec![[0i16; 64]; channels];
    for (ch, chunk) in scratch.iter_mut().enumerate() {
        let base = QT_CHUNK_BYTES * ch;
        let header = u16::from_be_bytes([data[base], data[base + 1]]);
        // The seed word: top 9 bits predictor, low 7 bits index.
        let predictor = (header as i32).sign_extend(16) & !0x7F;
        let step_index = usize::from(header & 0x7F);
        // ffmpeg's continuation heuristic: keep the running state
        // unless the header signals a real jump.
        if state[ch].step_index == step_index {
            let diff = (predictor - state[ch].predictor).abs();
            if diff > 0x7F {
                state[ch].step_index = step_index;
                state[ch].predictor = predictor;
            }
        } else {
            state[ch].step_index = step_index;
            state[ch].predictor = predictor;
        }
        if state[ch].step_index > MAX_STEP_INDEX {
            // The reference refuses the whole packet; here the typed
            // concealment (block-skip precedent).
            stats.skipped_packets = 1;
            return stats;
        }
        for m in 0..32 {
            let byte = data[base + 2 + m];
            chunk[2 * m] = qt_expand(&mut state[ch], byte & 0x0F);
            chunk[2 * m + 1] = qt_expand(&mut state[ch], byte >> 4);
        }
    }
    for instant in 0..64 {
        for chunk in &scratch {
            out.extend_from_slice(&chunk[instant].to_ne_bytes());
        }
    }
    stats.frames = 64;
    stats
}

/// Decode one Westwood payload into `out` (native-endian i16,
/// channel-interleaved). `state` is the cross-packet channel state.
/// Packing: per instant pair, one byte per channel — low nibble is the
/// earlier instant, high the later. Never panics.
pub(crate) fn decode_ima_ws(
    data: &[u8],
    channels: usize,
    state: &mut [WsChannel],
    out: &mut Vec<u8>,
) -> MiscStats {
    let mut stats = MiscStats::default();
    // One byte per channel per pair of instants (low nibble = earlier
    // instant, high = later): 512 mono bytes → 1024 samples, 512
    // stereo bytes → 512 instants × 2ch (ffmpeg's own counts,
    // measured on the forced decoder this session).
    let pairs = data.len() / channels.max(1);
    let mut lows = vec![0i16; channels];
    let mut highs = vec![0i16; channels];
    for pair in 0..pairs {
        let base = pair * channels;
        for (ch, channel) in state.iter_mut().enumerate() {
            let byte = data[base + ch];
            lows[ch] = ws_expand(channel, byte & 0x0F);
            highs[ch] = ws_expand(channel, byte >> 4);
        }
        for sample in &lows {
            out.extend_from_slice(&sample.to_ne_bytes());
        }
        for sample in &highs {
            out.extend_from_slice(&sample.to_ne_bytes());
        }
        stats.frames += 2;
    }
    stats
}

/// Sign-extend a `bits`-wide value held in an i32 (the `s(16)` read
/// of the header word).
trait SignExtend {
    fn sign_extend(self, bits: u32) -> i32;
}

impl SignExtend for i32 {
    fn sign_extend(self, bits: u32) -> i32 {
        if bits == 0 || bits >= 32 {
            return self;
        }
        let shift = 32 - bits;
        (self << shift) >> shift
    }
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
    fn sign_extend_helper_matches_c_semantics() {
        assert_eq!(0i32.sign_extend(16), 0);
        assert_eq!(0x7FFF_i32.sign_extend(16), 0x7FFF);
        assert_eq!(0xFFFF_i32.sign_extend(16), -1);
        assert_eq!(0x8000_i32.sign_extend(16), -32768);
    }

    #[test]
    fn partial_and_multiply_forms_disagree_on_odd_steps() {
        // step 7, nibble 7: partial-sum gives 7+3+1+0 = 11; multiply
        // gives ((2·7+1)·7)>>3 = 13. The two flavors must keep their
        // own forms (ffmpeg does; so do we).
        let mut qt = [WsChannel::default()];
        let mut ws = [WsChannel::default()];
        assert_eq!(qt_expand(&mut qt[0], 7), 11);
        assert_eq!(ws_expand(&mut ws[0], 7), 13);
    }

    /// One mono IMA4 chunk from a raw header word: bits 15..7 are the
    /// seed predictor, bits 6..0 the step index; then 32 nibble bytes.
    fn qt_chunk(word: u16, nibble: u8) -> Vec<u8> {
        let mut v = word.to_be_bytes().to_vec();
        v.extend_from_slice(&[nibble; 32]);
        v
    }

    #[test]
    fn qt_header_word_splits_predictor_and_index() {
        // Header word 0x2001: predictor 0x2000 (top 9 bits), index 1.
        // Fresh state (0, 0): index differs → adopt. First nibble 0xF
        // at step STEP_TABLE[1] = 8 moves the predictor by
        // −(1 + 8 + 4 + 2) = −15; the running state keeps evolving
        // through the 63 remaining nibbles, so the adoption is pinned
        // on the first output sample, not the post-decode state.
        let chunk = qt_chunk(0x2001, 0x0F);
        let mut state = [WsChannel::default()];
        let mut out = Vec::new();
        let stats = decode_ima_qt_packet(&chunk, 1, &mut state, &mut out);
        assert_eq!(stats.frames, 64);
        let got = samples(&out);
        assert_eq!(got[0], 0x2000 - 15, "seed adopted, then −15");
        assert_eq!(got.len(), 64);
    }

    #[test]
    fn qt_continuation_heuristic_keeps_running_state() {
        // Chunk 1 (word 0x2000: predictor 0x2000, index 0) with a
        // first nibble byte of 0x11: each nibble-1 adds STEP_TABLE[0]
        // >> 2 = 1 (index stays clamped at 0), so the running
        // predictor ends at 0x2002 after the pair and the 0x00 tail
        // adds nothing. Chunk 2 repeats the SAME header word — index
        // matches and |0x2000 − 0x2002| ≤ 0x7F — so the running state
        // (0x2002) is kept and the header's 0x2000 is ignored. Chunk
        // 3 carries a distant word (0x4000) and re-seeds.
        let mut chunk1 = qt_chunk(0x2000, 0x00);
        chunk1[2] = 0x11;
        let mut state = [WsChannel::default()];
        let mut out = Vec::new();
        decode_ima_qt_packet(&chunk1, 1, &mut state, &mut out);
        assert_eq!(state[0].predictor, 0x2002);
        assert_eq!(state[0].step_index, 0);
        decode_ima_qt_packet(&qt_chunk(0x2000, 0x00), 1, &mut state, &mut out);
        assert_eq!(state[0].predictor, 0x2002, "header ignored: continuation");
        decode_ima_qt_packet(&qt_chunk(0x4000, 0x00), 1, &mut state, &mut out);
        assert_eq!(state[0].predictor, 0x4000, "distant header re-seeds");
    }

    #[test]
    fn qt_short_packet_and_wild_index_are_skipped() {
        let mut state = [WsChannel::default()];
        let mut out = Vec::new();
        let stats = decode_ima_qt_packet(&[0; 10], 1, &mut state, &mut out);
        assert_eq!(stats.skipped_packets, 1);
        assert!(out.is_empty());
        // Header index 89 (> 88): word 0 | 89.
        let stats = decode_ima_qt_packet(&qt_chunk(89, 0), 1, &mut state, &mut out);
        assert_eq!(stats.skipped_packets, 1);
    }

    #[test]
    fn qt_stereo_chunks_are_per_channel_then_interleaved() {
        // ch0 chunk (predictor 0x0200 → 512, nibble 0x8), ch1 chunk
        // (word 0xFE00 → predictor −512, nibble 0x0): the output
        // alternates channels even though the chunks are consecutive.
        let packet = [qt_chunk(0x0200, 0x08), qt_chunk(0xFE00, 0x00)].concat();
        let mut state = [WsChannel::default(); 2];
        let mut out = Vec::new();
        let stats = decode_ima_qt_packet(&packet, 2, &mut state, &mut out);
        assert_eq!(stats.frames, 64);
        let got = samples(&out);
        assert_eq!(got.len(), 128);
        assert_eq!(got[0], 512, "ch0 nibble 8 at step 7: −(7>>3) = 0");
        assert_eq!(got[1], -512, "ch1 nibble 0 at step 7: +0");
        assert_eq!(got[2], 512, "ch0 instant 1");
        assert_eq!(got[3], -512, "ch1 instant 1");
    }

    #[test]
    fn ws_mono_is_two_samples_per_byte_low_first() {
        let mut state = [WsChannel::default()];
        let mut out = Vec::new();
        let stats = decode_ima_ws(&[0x71], 1, &mut state, &mut out);
        assert_eq!(stats.frames, 2);
        // nibble 1 (low): ((2·1+1)·7)>>3 = 2 → 2; index 0−1 → 0 (step
        // stays 7); nibble 7 (high): ((2·7+1)·7)>>3 = 13 → 15; index
        // 0+8 → 8.
        assert_eq!(samples(&out), [2, 15]);
    }

    #[test]
    fn ws_stereo_packs_one_byte_per_channel_per_pair() {
        let mut state = [WsChannel::default(); 2];
        let mut out = Vec::new();
        let stats = decode_ima_ws(&[0x11, 0x77], 2, &mut state, &mut out);
        assert_eq!(stats.frames, 2);
        // Instant 0 = both low nibbles; instant 1 = both highs. ch1's
        // nibble-7 pushes its index to 8 (step 16), so its high
        // nibble moves by ((15)·16)>>3 = 30.
        let got = samples(&out);
        assert_eq!(got[0], 2, "ch0 low");
        assert_eq!(got[1], 13, "ch1 low");
        assert_eq!(got[2], 4, "ch0 high");
        assert_eq!(got[3], 43, "ch1 high");
    }

    #[test]
    fn ws_state_persists_across_calls() {
        let mut state = [WsChannel::default()];
        let mut out = Vec::new();
        decode_ima_ws(&[0x77], 1, &mut state, &mut out);
        decode_ima_ws(&[0x77], 1, &mut state, &mut out);
        assert_eq!(samples(&out).len(), 4);
        assert_eq!(state[0].predictor as i16, samples(&out)[3]);
    }

    #[test]
    fn ws_trailing_byte_of_an_odd_stereo_payload_is_left_unconsumed() {
        // 3 bytes, stereo: one full pair (2 bytes) decodes; the third
        // byte is transport damage the loop guard leaves alone (the
        // reference's `bytestream2` overread check does the same).
        let mut state = [WsChannel::default(); 2];
        let mut out = Vec::new();
        let stats = decode_ima_ws(&[0x11, 0x22, 0x33], 2, &mut state, &mut out);
        assert_eq!(stats.frames, 2);
        assert_eq!(samples(&out).len(), 4);
    }
}
