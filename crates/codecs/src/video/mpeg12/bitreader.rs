//! MSB-first bit reader for MPEG-1/2 header syntax (ISO/IEC 11172-2
//! §6.2 / 13818-2 §6.2: every `uimsbf`/`simsbf`/`bslbf` element is
//! read most-significant bit first — the opposite convention from
//! H.264's Exp-Golomb reader, hence a sibling rather than a share).
//!
//! Total and bounds-checked: every read is either the requested
//! bits or a typed [`Mpeg12Error::Truncated`]; no panic, no
//! unbounded work on any input (the `fuzz_codec_mpeg2` target's
//! future guarantee, doc 04 §8.5).

use super::error::Mpeg12Error;

/// A bit-position cursor over a borrowed byte slice, MSB-first.
#[derive(Debug, Clone)]
pub struct BitReader<'a> {
    data: &'a [u8],
    /// Absolute bit offset from the slice start.
    bit_pos: usize,
}

impl<'a> BitReader<'a> {
    /// Start reading at the slice's first bit.
    #[must_use]
    pub fn new(data: &'a [u8]) -> BitReader<'a> {
        BitReader { data, bit_pos: 0 }
    }

    /// Bits not yet consumed.
    #[must_use]
    pub fn remaining_bits(&self) -> u32 {
        (self.data.len().saturating_sub(self.bit_pos / 8) * 8)
            .saturating_sub(self.bit_pos % 8)
            .min(u32::MAX as usize) as u32
    }

    /// Read `n` bits (1..=32) as an unsigned big-endian (MSB-first)
    /// integer.
    ///
    /// # Errors
    /// [`Mpeg12Error::Truncated`] when fewer than `n` bits remain;
    /// [`Mpeg12Error::Invalid`] for `n == 0 || n > 32`.
    pub fn read_bits(&mut self, n: u32) -> Result<u32, Mpeg12Error> {
        if n == 0 || n > 32 {
            return Err(Mpeg12Error::Invalid {
                code: "mpeg12.bits.width",
            });
        }
        if self.remaining_bits() < n {
            return Err(Mpeg12Error::Truncated {
                needed: n,
                have: self.remaining_bits(),
            });
        }
        let mut value: u32 = 0;
        for _ in 0..n {
            let byte = self.data[self.bit_pos / 8];
            let bit = (byte >> (7 - (self.bit_pos % 8))) & 1;
            value = (value << 1) | u32::from(bit);
            self.bit_pos += 1;
        }
        Ok(value)
    }

    /// Read one bit.
    ///
    /// # Errors
    /// [`Mpeg12Error::Truncated`] at end of data.
    pub fn read_bit(&mut self) -> Result<u32, Mpeg12Error> {
        self.read_bits(1)
    }

    /// Read a spec `marker_bit` (shall be 1).
    ///
    /// # Errors
    /// [`Mpeg12Error::Marker`] when the bit is 0; `Truncated` at end.
    pub fn read_marker(&mut self, code: &'static str) -> Result<(), Mpeg12Error> {
        if self.read_bit()? != 1 {
            return Err(Mpeg12Error::Marker { code });
        }
        Ok(())
    }

    /// Peek up to `n` bits (1..=32) without consuming (missing bits
    /// read as 0 — peeks only steer dispatch, never assert facts).
    #[must_use]
    pub fn peek_bits(&self, n: u32) -> u32 {
        let mut clone = self.clone();
        clone.read_bits(n).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    #[test]
    fn reads_msb_first() {
        let mut r = BitReader::new(&[0b1011_0010, 0b0100_0001]);
        assert_eq!(r.read_bits(4).must(), 0b1011);
        assert_eq!(r.read_bit().must(), 0);
        assert_eq!(r.read_bits(3).must(), 0b010);
        assert_eq!(r.read_bits(8).must(), 0b0100_0001);
    }

    #[test]
    fn truncation_is_typed() {
        let mut r = BitReader::new(&[0xff]);
        assert_eq!(r.read_bits(8), Ok(0xff));
        assert_eq!(
            r.read_bit(),
            Err(Mpeg12Error::Truncated { needed: 1, have: 0 })
        );
        let mut r2 = BitReader::new(&[0x00]);
        assert_eq!(
            r2.read_bits(9),
            Err(Mpeg12Error::Truncated { needed: 9, have: 8 })
        );
    }

    #[test]
    fn marker_bit_enforced() {
        let mut r = BitReader::new(&[0b0111_1111]);
        assert_eq!(
            r.read_marker("mpeg12.test.marker"),
            Err(Mpeg12Error::Marker {
                code: "mpeg12.test.marker"
            })
        );
        let mut ok = BitReader::new(&[0b1000_0000]);
        assert!(ok.read_marker("mpeg12.test.marker").is_ok());
    }

    #[test]
    fn width_bounds_rejected() {
        let mut r = BitReader::new(&[0xff; 8]);
        assert_eq!(
            r.read_bits(0),
            Err(Mpeg12Error::Invalid {
                code: "mpeg12.bits.width"
            })
        );
        assert_eq!(
            r.read_bits(33),
            Err(Mpeg12Error::Invalid {
                code: "mpeg12.bits.width"
            })
        );
    }

    #[test]
    fn remaining_counts_partial_bytes() {
        let mut r = BitReader::new(&[0xff; 3]);
        assert_eq!(r.remaining_bits(), 24);
        r.read_bits(5).must();
        assert_eq!(r.remaining_bits(), 19);
        r.read_bits(19).must();
        assert_eq!(r.remaining_bits(), 0);
    }

    #[test]
    fn empty_input_only_truncates() {
        let mut r = BitReader::new(&[]);
        assert!(matches!(r.read_bit(), Err(Mpeg12Error::Truncated { .. })));
        assert_eq!(r.peek_bits(8), 0);
    }
}
