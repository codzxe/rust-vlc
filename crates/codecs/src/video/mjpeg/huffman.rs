//! Canonical Huffman decoding (ISO/IEC 10918-1 Annex F.2.2.3, the
//! spec's own MINCODE/MAXCODE/VALPTR procedure — cited public
//! specification, clean-room).
//!
//! Build is total (no panics); decode of a code absent from the table
//! or of exhausted entropy data is a typed [`Stop`], which the scan
//! decoder turns into concealment.

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
        let mut min_code = [0i32; 17];
        let mut max_code = [-1i32; 17];
        let mut val_ptr = [0usize; 17];
        let mut code: i32 = 0;
        let mut symbol_index = 0usize;
        for length in 1..=16usize {
            if bits[length] > 0 {
                val_ptr[length] = symbol_index;
                min_code[length] = code;
                // Canonical codes are consecutive at each length
                // (Annex C.2): the last is code + count - 1.
                let count = i32::from(bits[length]);
                let last = code.checked_add(count - 1).ok_or(Stop::InvalidCode)?;
                if last >= (1i32 << length) {
                    return Err(Stop::InvalidCode);
                }
                max_code[length] = last;
                code = last + 1;
                symbol_index += bits[length] as usize;
            }
            code <<= 1;
        }
        if symbol_index != values.len() {
            return Err(Stop::InvalidCode);
        }
        Ok(HuffmanDecoder {
            min_code,
            max_code,
            val_ptr,
            values: values.to_vec(),
        })
    }

    /// Decode one symbol, MSB-first, from the reader.
    ///
    /// # Errors
    /// [`Stop::Exhausted`] when entropy data ends mid-code;
    /// [`Stop::InvalidCode`] when the bits form no code in the table.
    pub(crate) fn decode(&self, reader: &mut BitReader<'_>) -> Result<u8, Stop> {
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
}
