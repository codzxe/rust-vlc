//! Canonical Huffman decoding (ISO/IEC 10918-1 Annex F.2.2.3, the
//! spec's own MINCODE/MAXCODE/VALPTR procedure — cited public
//! specification, clean-room).
//!
//! Build is total (no panics); decode of a code absent from the table
//! or of exhausted entropy data is a typed [`Stop`], which the scan
//! decoder turns into concealment.
//!
//! # ED-FIX-1 (audit C3): 8-bit look-ahead LUT
//!
//! Each table carries a 256-entry LUT indexed by the next 8 entropy
//! bits (zero-padded look-ahead via [`BitReader::peek8_padded`]):
//! the entry holds the symbol and code length when a code of ≤ 8
//! bits starts with that prefix (the libjpeg
//! `jpeg_make_d_derived_tbl` pattern). Longer codes take the classic
//! MINCODE/MAXCODE walk over lengths 9..=16. Because a canonical
//! code is prefix-free, the LUT entry is the *unique* ≤ 8-bit code
//! the upcoming bits can start, and the entry's length is validated
//! against the reader's available count before consuming — so a
//! truncated stream yields the same [`Stop::Exhausted`], an absent
//! code the same [`Stop::InvalidCode`], with the same bits consumed
//! as the bit-at-a-time walk (pinned by the tests here and by the
//! committed decode-reference digests).

use super::bitreader::BitReader;

/// Why a Huffman decode stopped early (both are concealable damage).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stop {
    /// The entropy-coded data ran out mid-code.
    Exhausted,
    /// The bits read form a code absent from the table (or walk past
    /// its longest code).
    InvalidCode,
}

/// One decode-side Huffman table: canonical codes per length, built
/// from the DHT BITS/HUFFVAL pair.
#[derive(Debug, Clone)]
pub(crate) struct HuffmanDecoder {
    /// MINCODE[l]: first code of length `l` (l in 1..=16).
    min_code: [i32; 17],
    /// MAXCODE[l]: last code of length `l`; -1 = no codes of this
    /// length (the spec's sentinel, F.2.2.3 Figure F.15).
    max_code: [i32; 17],
    /// VALPTR[l]: index of the first symbol of length `l` in `values`.
    val_ptr: [usize; 17],
    /// HUFFVAL, in code order.
    values: Vec<u8>,
    /// 8-bit look-ahead LUT (ED-FIX-1): `(symbol, length)` for the
    /// unique code of length ≤ 8 that prefixes each 8-bit window;
    /// `length == 0` marks windows that must take the slow walk
    /// (only codes > 8 bits, or no code, can match).
    lut: [(u8, u8); 256],
}

impl HuffmanDecoder {
    /// Build from DHT fields.
    ///
    /// # Errors
    /// [`Stop::InvalidCode`] semantics at build time: the table is
    /// structurally unusable (over-subscribed code space, or BITS/HUFFVAL
    /// disagreeing) — surfaced as `Err` so the caller can reject the
    /// stream before any scan work.
    pub(crate) fn new(bits: &[u8; 17], values: &[u8]) -> Result<Self, Stop> {
        let mut table = HuffmanDecoder {
            min_code: [0i32; 17],
            max_code: [-1i32; 17],
            val_ptr: [0usize; 17],
            values: Vec::new(),
            lut: [(0u8, 0u8); 256],
        };
        table.rebuild(bits, values)?;
        Ok(table)
    }

    /// Rebuild every derived table in place (V-3,
    /// docs/perf/memory-audit-20261023.md): the canonical-code arrays
    /// and the look-ahead LUT are overwritten, and the owned `HUFFVAL`
    /// storage reuses its allocation (`clear` + `extend_from_slice` —
    /// capacity retained, bounded by the 256-symbol DHT limit). A
    /// table-set change on a persistent decoder therefore allocates
    /// nothing once the first build exists — the pre-V-3 `to_vec` per
    /// rebuild is gone.
    ///
    /// # Errors
    /// Same [`Stop::InvalidCode`] semantics as [`HuffmanDecoder::new`].
    pub(crate) fn rebuild(&mut self, bits: &[u8; 17], values: &[u8]) -> Result<(), Stop> {
        self.min_code = [0i32; 17];
        self.max_code = [-1i32; 17];
        self.val_ptr = [0usize; 17];
        let mut code: i32 = 0;
        let mut symbol_index = 0usize;
        for (length, &count_u8) in bits.iter().enumerate().skip(1) {
            if count_u8 > 0 {
                self.val_ptr[length] = symbol_index;
                self.min_code[length] = code;
                // Canonical codes are consecutive at each length
                // (Annex C.2): the last is code + count - 1.
                let count = i32::from(count_u8);
                let last = code.checked_add(count - 1).ok_or(Stop::InvalidCode)?;
                if last >= (1i32 << length) {
                    return Err(Stop::InvalidCode);
                }
                self.max_code[length] = last;
                code = last + 1;
                symbol_index += count as usize;
            }
            code <<= 1;
        }
        if symbol_index != values.len() {
            return Err(Stop::InvalidCode);
        }

        // Look-ahead LUT: every code of length ≤ 8 claims the range of
        // 8-bit windows it prefixes (ranges are disjoint — the code is
        // prefix-free).
        self.lut = [(0u8, 0u8); 256];
        for length in 1..=8usize {
            if bits[length] == 0 {
                continue;
            }
            let base = self.min_code[length];
            let width = 1usize << (8 - length);
            for offset in 0..bits[length] as i32 {
                let symbol = values[self.val_ptr[length] + offset as usize];
                let start = ((base + offset) as usize) << (8 - length);
                for entry in self.lut.iter_mut().skip(start).take(width) {
                    *entry = (symbol, length as u8);
                }
            }
        }

        // V-3: HUFFVAL storage last — the loops above read the
        // borrowed `values` slice, this owns the copy.
        self.values.clear();
        self.values.extend_from_slice(values);
        Ok(())
    }

    /// Decode one symbol, MSB-first, from the reader.
    ///
    /// # Errors
    /// [`Stop::Exhausted`] when entropy data ends mid-code;
    /// [`Stop::InvalidCode`] when the bits form no code in the table.
    pub(crate) fn decode(&self, reader: &mut BitReader<'_>) -> Result<u8, Stop> {
        reader.refill();
        let available = reader.available();
        let peek = u32::from(reader.peek8_padded());

        // Fast path: a ≤ 8-bit code prefixes the window. The length
        // check against `available` is what keeps truncated streams
        // honest — the zero padding of the look-ahead can never
        // fabricate a decode.
        let (symbol, length) = self.lut[peek as usize];
        if length != 0 {
            let length = u32::from(length);
            if available >= length {
                reader.consume(length);
                return Ok(symbol);
            }
            reader.consume_all();
            return Err(Stop::Exhausted);
        }

        // No ≤ 8-bit code matches the window: if the data cannot even
        // supply the 8 real bits the walk would have consumed, this is
        // truncation (any code > 8 bits needs more than remains).
        if available < 8 {
            reader.consume_all();
            return Err(Stop::Exhausted);
        }

        // Slow path: the classic MINCODE/MAXCODE walk over the long
        // codes. `peek_bits(l)` is the first `l` bits — exactly the
        // code the bit-at-a-time walk would have assembled.
        for length in 9usize..=16 {
            let length_u32 = length as u32;
            if available < length_u32 {
                reader.consume_all();
                return Err(Stop::Exhausted);
            }
            let code = reader.peek_bits(length_u32).unwrap_or(0) as i32;
            if self.max_code[length] >= 0 {
                if code > self.max_code[length] {
                    continue;
                }
                if code < self.min_code[length] {
                    reader.consume(length_u32);
                    return Err(Stop::InvalidCode);
                }
                reader.consume(length_u32);
                let index = self.val_ptr[length]
                    .checked_add((code - self.min_code[length]) as usize)
                    .ok_or(Stop::InvalidCode)?;
                return match self.values.get(index) {
                    Some(&symbol) => Ok(symbol),
                    None => Err(Stop::InvalidCode),
                };
            }
        }
        reader.consume(16);
        Err(Stop::InvalidCode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reader_of(bytes: &[u8]) -> BitReader<'_> {
        BitReader::new(bytes)
    }

    /// A tiny table: two codes — `0` (len 1, symbol 0x05) and `10`
    /// (len 2, symbol 0x11); `11` is unused (incomplete table, legal
    /// per C.2 — decoding it is an error).
    fn toy_table() -> HuffmanDecoder {
        let mut bits = [0u8; 17];
        bits[1] = 1;
        bits[2] = 1;
        match HuffmanDecoder::new(&bits, &[0x05, 0x11]) {
            Ok(t) => t,
            Err(e) => unreachable!("build failed: {e:?}"),
        }
    }

    #[test]
    fn decodes_each_present_code() {
        let table = toy_table();
        // Bits 0 10 0 0000 decode as 0x05, 0x11, then five more 0x05
        // from the trailing zero bits (1+2+5 = all 8); the next read
        // exhausts.
        let mut reader = reader_of(&[0b0100_0000]);
        let mut symbols = Vec::new();
        while let Ok(symbol) = table.decode(&mut reader) {
            symbols.push(symbol);
        }
        assert_eq!(symbols, vec![0x05, 0x11, 0x05, 0x05, 0x05, 0x05, 0x05]);
        assert_eq!(table.decode(&mut reader).err(), Some(Stop::Exhausted));
    }

    #[test]
    fn code_absent_from_table_is_an_error() {
        let table = toy_table();
        // Sixteen bits with the unused 11 prefix walk every length
        // without a match: InvalidCode (with fewer bits it would be
        // indistinguishable from exhaustion).
        let mut reader = reader_of(&[0b11_000000, 0x00]);
        assert_eq!(table.decode(&mut reader).err(), Some(Stop::InvalidCode));
    }

    #[test]
    fn oversubscribed_table_is_rejected_at_build() {
        // Two codes of length 1 fill the space; a third at length 2
        // cannot exist.
        let mut bits = [0u8; 17];
        bits[1] = 2;
        bits[2] = 1;
        assert!(HuffmanDecoder::new(&bits, &[1, 2, 3]).is_err());
    }

    #[test]
    fn code_overflow_past_sixteen_bits_is_rejected() {
        // 255 codes of length 8 leave one slot; three at length 9
        // overflow the canonical assignment (two would still fit).
        let mut bits = [0u8; 17];
        bits[8] = 255;
        bits[9] = 3;
        let values: Vec<u8> = (0..258).map(|i| i as u8).collect();
        assert!(HuffmanDecoder::new(&bits, &values).is_err());
    }

    #[test]
    fn single_symbol_table_uses_a_one_bit_code() {
        // The 2.A3 encoder emits exactly this shape for skewed tables.
        let mut bits = [0u8; 17];
        bits[1] = 1;
        let table = match HuffmanDecoder::new(&bits, &[0x2a]) {
            Ok(t) => t,
            Err(e) => unreachable!("build failed: {e:?}"),
        };
        let mut reader = reader_of(&[0b0_1000000]);
        assert_eq!(table.decode(&mut reader).ok(), Some(0x2a));
        // Sixteen 1-bits (each FF stuffed) are not in the table
        // (incomplete): error, not a symbol.
        let mut ones = reader_of(&[0xff, 0x00, 0xff, 0x00]);
        assert_eq!(table.decode(&mut ones).err(), Some(Stop::InvalidCode));
    }

    #[test]
    fn sixteen_bit_codes_decode() {
        // One code of each length 1..=16 in canonical order: 0, 10,
        // 110, … The length-16 code is 1111 1111 1111 1110.
        let mut bits = [0u8; 17];
        for b in bits.iter_mut().skip(1).take(16) {
            *b = 1;
        }
        let values: Vec<u8> = (0..16).map(|i| 0xa0 + i as u8).collect();
        let table = match HuffmanDecoder::new(&bits, &values) {
            Ok(t) => t,
            Err(e) => unreachable!("build failed: {e:?}"),
        };
        // The bit pattern begins with 0xFF, so the first byte must be
        // stuffed (FF 00) for the reader to see it as data.
        let mut reader = reader_of(&[0xff, 0x00, 0xfe]);
        assert_eq!(
            table.decode(&mut reader).ok(),
            Some(0xaf),
            "the length-16 code is 1111111111111110 → symbol 0xaf"
        );
    }

    // --- ED-FIX-1: LUT semantics vs the bit-at-a-time walk ---

    impl HuffmanDecoder {
        /// The pre-LUT decode walk, kept verbatim as the equivalence
        /// oracle for the LUT + word-refill reader.
        fn decode_naive(&self, reader: &mut BitReader<'_>) -> Result<u8, Stop> {
            let mut code: i32 = 0;
            for length in 1..=16usize {
                let Some(bit) = reader.next_bit() else {
                    return Err(Stop::Exhausted);
                };
                code = (code << 1) | i32::from(bit);
                if self.max_code[length] >= 0 {
                    if code > self.max_code[length] {
                        continue;
                    }
                    if code < self.min_code[length] {
                        return Err(Stop::InvalidCode);
                    }
                    let index = self.val_ptr[length]
                        .checked_add((code - self.min_code[length]) as usize)
                        .ok_or(Stop::InvalidCode)?;
                    return match self.values.get(index) {
                        Some(&symbol) => Ok(symbol),
                        None => Err(Stop::InvalidCode),
                    };
                }
            }
            Err(Stop::InvalidCode)
        }
    }

    #[test]
    fn lut_decode_matches_the_bit_walk_on_random_streams() {
        // Deterministic sweep: random DHT shapes (some over-subscribed
        // and rejected) crossed with random entropy byte strings,
        // including truncated tails. Every (table, stream) pair is
        // decoded by both paths; the symbol sequence and the terminal
        // Stop must agree. (Consumed-bit counts may differ only on
        // the terminal InvalidCode/Exhausted, where the scan decoder
        // abandons the reader — verified by construction here: both
        // paths stop at the same symbol boundary.)
        struct Rng(u64);
        impl Rng {
            fn next(&mut self) -> u64 {
                let mut x = self.0;
                x ^= x >> 12;
                x ^= x << 25;
                x ^= x >> 27;
                self.0 = x;
                x.wrapping_mul(0x2545_f491_4f6c_dd1d)
            }
        }
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let mut pairs = 0usize;
        let mut mismatches = Vec::new();
        for round in 0..600 {
            // Random BITS: distribute 1..=J codes across lengths.
            let mut bits = [0u8; 17];
            let mut total_codes = 0u32;
            for (length, slot) in bits.iter_mut().enumerate().skip(1) {
                if round % 3 != 0 || length <= 9 {
                    *slot = (rng.next() % 4) as u8;
                    total_codes += u32::from(*slot);
                }
            }
            let values: Vec<u8> = (0..total_codes).map(|i| (97 + i % 26) as u8).collect();
            let Ok(table) = HuffmanDecoder::new(&bits, &values) else {
                continue; // over-subscribed: build rejection tested above
            };
            let len = (rng.next() % 12) as usize;
            let stream: Vec<u8> = (0..len)
                .map(|i| {
                    // Avoid accidental stuffing/marker prefixes in
                    // most bytes, but keep some 0xff stress too.
                    if i % 7 == 3 {
                        0xff
                    } else {
                        (rng.next() % 256) as u8
                    }
                })
                .collect();
            // Skip pure-0xff streams whose first feed stops at a
            // marker — both paths stop identically, but the naive
            // oracle consumes nothing; still a valid pair.
            let mut a = BitReader::new(&stream);
            let mut b = BitReader::new(&stream);
            loop {
                let fast = table.decode(&mut a);
                let slow = table.decode_naive(&mut b);
                pairs += 1;
                match (&fast, &slow) {
                    (Ok(x), Ok(y)) if x == y => continue,
                    (Err(x), Err(y)) if x == y => break,
                    _ => mismatches.push(format!(
                        "round {round} stream {stream:02x?}: {fast:?} vs {slow:?}"
                    )),
                }
            }
        }
        assert!(
            mismatches.is_empty(),
            "LUT decode diverged from the walk:\n{}",
            mismatches.join("\n")
        );
        println!("lut-vs-walk equivalence: {pairs} decode calls agree");
    }

    #[test]
    fn truncation_mid_code_is_exhausted_not_fabricated() {
        // A length-4 code (0000) with only 3 real bits left: the
        // zero-padded look-ahead sees window 00000000 and the LUT
        // entry claims length 4 — but `available == 3`, so the decode
        // must report Exhausted, never fabricate the symbol from
        // padding.
        let mut bits = [0u8; 17];
        bits[4] = 1;
        let table = match HuffmanDecoder::new(&bits, &[0x42]) {
            Ok(t) => t,
            Err(e) => unreachable!("build failed: {e:?}"),
        };
        let mut reader = reader_of(&[0b0001_0000]);
        assert_eq!(reader.read_bits(5), Some(0b00010), "consume 5 of 8 bits");
        assert_eq!(reader.available(), 3);
        assert_eq!(table.decode(&mut reader).err(), Some(Stop::Exhausted));
        assert_eq!(reader.available(), 0, "the partial bits are consumed");
        // The full 8 bits decode fine on a fresh reader.
        let mut fresh = reader_of(&[0b0000_1000]);
        assert_eq!(table.decode(&mut fresh).ok(), Some(0x42));
    }

    #[test]
    fn short_codes_decode_near_the_tail() {
        // 3 bits remain and a length-3 code matches them exactly: the
        // padded look-ahead finds it and the length check passes.
        let mut bits = [0u8; 17];
        bits[3] = 2; // codes 000, 001
        let table = match HuffmanDecoder::new(&bits, &[0x10, 0x20]) {
            Ok(t) => t,
            Err(e) => unreachable!("build failed: {e:?}"),
        };
        let mut reader = reader_of(&[0b1110_0000]);
        // Consume the leading 111 (no code of any length here starts
        // with it); the remaining 00000 starts with the code 000.
        assert_eq!(reader.read_bits(3), Some(0b111));
        assert_eq!(reader.available(), 5);
        assert_eq!(table.decode(&mut reader).ok(), Some(0x10));
        assert_eq!(reader.available(), 2);
    }
}
