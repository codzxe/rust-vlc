//! SWF / Shockwave Flash ADPCM (card 5.J2b): the 4-bit codec of SWF
//! `SoundStreamBlock`s and FLV audio tags — the `adpcm_swf` row of
//! doc 04 §3.1's ADPCM block.
//!
//! # The packet grammar (pinned against the ffmpeg 7.1.5 reference)
//!
//! Unlike the raw flavors, an SWF packet is **self-contained**: it
//! carries its own block headers, bit-packed MSB-first:
//!
//! ```text
//! [2 bits: code width]  nb_bits = code + 2  ∈ {2, 3, 4, 5}
//! per block, per channel:  predictor (signed 16) , step index (6)
//! then sample fields, nb_bits wide, channel-interleaved per instant
//! — the "stereo pair layout": left field, right field, left, right…
//! ```
//!
//! A packet may carry **several consecutive blocks** (each restarts
//! the per-channel state from its own header). Block capacity is
//! 4095 coded samples plus the seed (ffmpeg's `count < 4095`); the
//! seed sample is the header predictor itself, emitted first.
//!
//! The per-field reconstruction is the partial-sum IMA form
//! (ffmpeg's `k`-loop over `k0 = 1 << (nb_bits-2)`):
//!
//! ```text
//! vpdiff = (Σ_{k∈k0,k0/2,…,1} [delta & k] · step_k)  + step_after
//! ```
//!
//! which for the usual `nb_bits = 4` equals the classic
//! `(n&4?step:0)+(n&2?step>>1:0)+(n&1?step>>2:0)+step>>3` — the
//! odd-step rounding that the multiply form would hide.
//!
//! # Oracle
//!
//! ffmpeg 7.1.5 has both an `adpcm_swf` encoder and decoder: the
//! committed fixture is a stereo 440 Hz sine the reference encoder
//! produced (FLV-carried, tag payloads extracted verbatim), and this
//! decoder is bit-exact against the reference decoder's PCM —
//! `tests/adpcm_breadth_differential.rs`.
//!
//! # Damage model
//!
//! Block headers are in-band, so damage is detectable: a packet whose
//! bit stream ends mid-header or mid-field stops at the last whole
//! field (a typed concealed tail), and a packet too short for even
//! the 2-bit code + one header is skipped whole. The 6-bit header
//! index cannot exceed 63 (< 89), so no index validation is needed
//! beyond the 0..=88 clamp the expand applies anyway.

/// ffmpeg's `swf_index_tables` (padded shapes for code widths 2..5;
/// indexed by `nb_bits - 2`, addressed by the field's magnitude bits).
const SWF_INDEX_TABLES: [[i8; 16]; 4] = [
    [
        -1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
    ],
    [
        -1, -1, 2, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
    ],
    [
        -1, -1, -1, -1, 2, 4, 6, 8, 0, 0, 0, 0, 0, 0, 0, 0, //
    ],
    [
        -1, -1, -1, -1, -1, -1, -1, -1, 1, 2, 4, 6, 8, 10, 13, 16, //
    ],
];

/// The IMA 89-entry step table (shared with the IMA-WAV flavor; OKI's
/// is the truncated 49-entry ladder — see [`crate::audio::adpcm::oki`]).
const STEP_TABLE: [i16; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

/// An MSB-first checked bit reader (ffmpeg's `GetBitContext` for this
/// codec's exact usage: whole-field reads that must fit).
struct BitReader<'a> {
    data: &'a [u8],
    /// Bit position from the MSB of byte 0.
    pos: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> BitReader<'a> {
        BitReader { data, pos: 0 }
    }

    /// Read `bits` (1..=16) bits MSB-first; `None` when they do not fit
    /// before end of stream (the honest stop for damaged tails).
    fn u(&mut self, bits: u32) -> Option<u32> {
        let bits = bits as usize;
        if bits == 0 || self.pos + bits > self.data.len() * 8 {
            return None;
        }
        let mut value = 0u32;
        for _ in 0..bits {
            let byte = self.data[self.pos / 8];
            let bit = (byte >> (7 - (self.pos % 8))) & 1;
            value = (value << 1) | u32::from(bit);
            self.pos += 1;
        }
        Some(value)
    }

    /// Read a signed field (ffmpeg's `get_sbits`: sign-extend from the
    /// field width).
    fn s(&mut self, bits: u32) -> Option<i32> {
        let value = self.u(bits)? as i32;
        let width = bits as usize;
        if width < 32 && (value >> (width - 1)) & 1 != 0 {
            Some(value - (1 << width))
        } else {
            Some(value)
        }
    }
}

/// Per-channel state inside one packet (re-seeded by each block header
/// — nothing persists across packets).
#[derive(Debug, Clone, Copy, Default)]
struct SwfChannel {
    predictor: i32,
    step_index: usize,
}

/// What one `decode_swf_packet` call did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SwfStats {
    /// Sample instants appended (all channels interleaved).
    pub frames: u64,
    /// The packet was too short for the code bits plus one header.
    pub skipped_packet: bool,
    /// The bit stream ended mid-field; a prefix was decoded.
    pub concealed_tail: bool,
}

/// Decode one SWF packet into `out` (native-endian i16,
/// channel-interleaved). Mirrors ffmpeg's `adpcm_swf_decode` loop
/// structure exactly, including the `count < 4095` block capacity and
/// the bit-position loop guards. Never panics.
pub(crate) fn decode_swf_packet(data: &[u8], channels: usize, out: &mut Vec<u8>) -> SwfStats {
    let mut stats = SwfStats::default();
    let mut reader = BitReader::new(data);
    let Some(code) = reader.u(2) else {
        stats.skipped_packet = !data.is_empty();
        return stats;
    };
    let nb_bits = code + 2;
    let table = &SWF_INDEX_TABLES[(nb_bits - 2) as usize];
    let k0: u32 = 1 << (nb_bits - 2);
    let signmask: u32 = 1 << (nb_bits - 1);
    let size_bits = data.len() * 8;
    let header_bits = 22i64 * channels as i64;
    let field_bits = nb_bits as i64 * channels as i64;
    if (size_bits as i64 - reader.pos as i64) < header_bits {
        // Non-empty packet that cannot carry even one block header.
        stats.skipped_packet = !data.is_empty();
        return stats;
    }
    let mut states = vec![SwfChannel::default(); channels];
    // ffmpeg: while (get_bits_count(&gb) <= size - 22 * channels)
    while (size_bits as i64 - reader.pos as i64) >= header_bits {
        // Per-channel header: predictor (16, signed) then index (6).
        let mut seeded = true;
        for state in states.iter_mut() {
            match (reader.s(16), reader.u(6)) {
                (Some(predictor), Some(index)) => {
                    state.predictor = predictor;
                    state.step_index = index as usize;
                    push_sample(out, predictor as i16);
                    stats.frames += 1;
                }
                _ => {
                    seeded = false;
                    break;
                }
            }
        }
        if !seeded {
            stats.concealed_tail = true;
            return stats;
        }
        // ffmpeg: while (bits_left >= nb_bits*ch && count < 4095)
        let mut count = 0u32;
        while (size_bits as i64 - reader.pos as i64) >= field_bits && count < 4095 {
            for state in states.iter_mut() {
                let Some(delta) = reader.u(nb_bits) else {
                    stats.concealed_tail = true;
                    return stats;
                };
                let step = i32::from(STEP_TABLE[state.step_index.min(88)]);
                let mut vpdiff = 0i32;
                let mut k = k0;
                let mut scale = step;
                loop {
                    if delta & k != 0 {
                        vpdiff += scale;
                    }
                    scale >>= 1;
                    k >>= 1;
                    if k == 0 {
                        break;
                    }
                }
                vpdiff += scale;
                state.predictor = if delta & signmask != 0 {
                    state.predictor - vpdiff
                } else {
                    state.predictor + vpdiff
                };
                let magnitude = (delta & !signmask) as usize;
                state.step_index =
                    (state.step_index as i32 + i32::from(table[magnitude])).clamp(0, 88) as usize;
                state.predictor = state.predictor.clamp(-32768, 32767);
                push_sample(out, state.predictor as i16);
                stats.frames += 1;
            }
            count += 1;
        }
    }
    stats
}

/// Append one i16 sample in native byte order.
fn push_sample(out: &mut Vec<u8>, sample: i16) {
    out.extend_from_slice(&sample.to_ne_bytes());
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

    /// A hand-rolled MSB-first bit writer for vector construction
    /// (byte-granular packets, zero-padding at the tail — exactly what
    /// a real muxer emits).
    struct BitWriter {
        bytes: Vec<u8>,
        /// Bits used in the last byte (0..8).
        bit: u32,
    }

    impl BitWriter {
        fn new() -> BitWriter {
            BitWriter {
                bytes: Vec::new(),
                bit: 0,
            }
        }

        /// Push the low `width` bits of `value`, MSB first.
        fn put(&mut self, value: u32, width: u32) {
            for i in (0..width).rev() {
                if self.bit == 0 {
                    self.bytes.push(0);
                }
                let bit = (value >> i) & 1;
                if bit != 0 {
                    let last = self.bytes.len() - 1;
                    self.bytes[last] |= 1 << (7 - self.bit);
                }
                self.bit = (self.bit + 1) % 8;
            }
        }

        fn finish(self) -> Vec<u8> {
            self.bytes
        }
    }

    /// A minimal mono packet: code 2 (nb_bits 4), predictor, index,
    /// then the given 4-bit fields. Ends byte-aligned; tail pad bits
    /// become extra zero-fields, so vectors are chosen to end at a
    /// field boundary where the guard arithmetic is asserted anyway.
    fn mono_packet(predictor: i16, index: u8, fields: &[u8]) -> Vec<u8> {
        let mut w = BitWriter::new();
        w.put(0b10, 2);
        w.put(predictor as u16 as u32, 16);
        w.put(u32::from(index & 0x3F), 6);
        for &f in fields {
            w.put(u32::from(f & 0x0F), 4);
        }
        w.finish()
    }

    #[test]
    fn bit_writer_round_trips_the_reader() {
        let mut w = BitWriter::new();
        w.put(0b10, 2);
        w.put(0x1234, 16);
        w.put(0b100111, 6);
        let bytes = w.finish();
        let mut reader = BitReader::new(&bytes);
        assert_eq!(reader.u(2), Some(0b10));
        assert_eq!(reader.u(16), Some(0x1234));
        assert_eq!(reader.u(6), Some(0b100111));
    }

    #[test]
    fn bit_reader_is_msb_first_and_checked() {
        let mut reader = BitReader::new(&[0b10110000]);
        assert_eq!(reader.u(2), Some(0b10));
        assert_eq!(reader.u(2), Some(0b11));
        assert_eq!(reader.u(4), Some(0));
        assert_eq!(reader.u(1), None);
        // 0xFF00 sign-extended from 16 bits is −256 (−1 would be
        // 0xFFFF).
        let mut reader = BitReader::new(&[0xFF, 0x00]);
        assert_eq!(reader.s(16), Some(-256));
    }

    #[test]
    fn mono_block_seed_then_partial_sum_fields() {
        // predictor 100, index 0 (step 7): field 0xF → vpdiff =
        // 7+3+1+0 = 11, sign set → 89, index += table[7] = 8 (step 16);
        // field 0x0 → vpdiff = 16>>3 = 2 → 91. Two fields land the
        // stream at a byte boundary with 0 pad bits.
        let mut out = Vec::new();
        let stats = decode_swf_packet(&mono_packet(100, 0, &[0xF, 0x0]), 1, &mut out);
        assert_eq!(stats.frames, 3);
        assert_eq!(samples(&out), [100, 89, 91]);
        assert!(!stats.concealed_tail && !stats.skipped_packet);
    }

    #[test]
    fn odd_step_rounding_is_the_partial_sum_form() {
        // index 0 → step 7 (odd). Field 0b0111: vpdiff =
        // (n&4?7:0) + (n&2?3:0) + (n&1?1:0) + (7>>3 = 0) = 11 — the
        // multiply form ((2·7+1)·7)>>3 = 13 would disagree; this pin
        // holds the partial-sum side (ffmpeg's k-loop). The index
        // then moves by table[7] = +8 → step 16, and field 0 adds
        // 16>>3 = 2.
        let mut out = Vec::new();
        let stats = decode_swf_packet(&mono_packet(0, 0, &[0b0111, 0]), 1, &mut out);
        assert_eq!(stats.frames, 3);
        assert_eq!(samples(&out), [0, 11, 13]);
    }

    #[test]
    fn stereo_pair_layout_interleaves_fields() {
        // Hand-packed stereo: code 2, ch0 pred 100/idx 0, ch1 pred
        // −100/idx 0, then per instant: ch0 field, ch1 field (the
        // pair layout). 2+16+6+16+6+4+4 = 54 bits; 2 pad bits bring
        // the packet to 7 bytes, and 2 < nb_bits·ch = 8 stops the loop.
        let mut w = BitWriter::new();
        w.put(0b10, 2);
        w.put(100u16 as u32, 16);
        w.put(0, 6);
        w.put((-100i16) as u16 as u32, 16);
        w.put(0, 6);
        w.put(0xF, 4); // ch0
        w.put(0x0, 4); // ch1
        let packet = w.finish();
        assert_eq!(packet.len(), 7);
        let mut out = Vec::new();
        let stats = decode_swf_packet(&packet, 2, &mut out);
        // 4 SAMPLES: the seed pair (ch0+ch1) plus one field instant
        // (ch0+ch1) — `frames` counts samples, not instants.
        assert_eq!(stats.frames, 4, "seed pair + one instant");
        let got = samples(&out);
        assert_eq!(got[0], 100);
        assert_eq!(got[1], -100);
        // ch0 field 0xF at step 7: −11 → 89; ch1 field 0 at step 7:
        // +0 → −100.
        assert_eq!(got[2], 89);
        assert_eq!(got[3], -100);
    }

    #[test]
    fn packet_short_of_a_header_is_skipped() {
        // Two bytes: the 2-bit code fits, the 22-bit mono header does
        // not — typed skip, nothing decoded. (`concealed_tail` is the
        // defensive branch: the loop guards mirror ffmpeg's own
        // bit-budget arithmetic, so an in-bounds stream never fails a
        // read — it stays for fuzz safety.)
        let mut w = BitWriter::new();
        w.put(0b10, 2);
        w.put(0x1F, 6);
        let mut out = Vec::new();
        let stats = decode_swf_packet(&w.finish(), 1, &mut out);
        assert!(stats.skipped_packet);
        assert_eq!(stats.frames, 0);
        assert!(out.is_empty());
    }

    #[test]
    fn empty_and_tiny_packets_are_typed() {
        let mut out = Vec::new();
        assert_eq!(decode_swf_packet(&[], 1, &mut out), SwfStats::default());
        // One byte: the code fits, a stereo header cannot.
        let stats = decode_swf_packet(&[0x80], 2, &mut out);
        assert!(stats.skipped_packet);
        assert!(out.is_empty());
    }

    #[test]
    fn count_cap_chains_a_second_block_in_one_packet() {
        // The 4095-field capacity is the only way a second block
        // header fits in one packet (the field loop otherwise eats to
        // the last whole field). Build: block 1 header + 4095 fields
        // of 0xF, then block 2 header (predictor −1, index 0) + one
        // pad zero-field, and check the re-seed lands at instant 4096.
        let mut w = BitWriter::new();
        w.put(0b10, 2);
        w.put(100u16 as u32, 16);
        w.put(0, 6);
        for _ in 0..4095 {
            w.put(0xF, 4);
        }
        w.put((-1i16) as u16 as u32, 16);
        w.put(0, 6);
        let packet = w.finish();
        let mut out = Vec::new();
        let stats = decode_swf_packet(&packet, 1, &mut out);
        // 1 seed + 4095 fields + 1 seed + 1 pad field.
        assert_eq!(stats.frames, 4098);
        let got = samples(&out);
        assert_eq!(got[0], 100);
        assert_eq!(got[4096], -1, "block 2 seed restarts the state");
        assert_eq!(got[4097], -1, "zero-field at step 7 moves by 0");
    }
}
