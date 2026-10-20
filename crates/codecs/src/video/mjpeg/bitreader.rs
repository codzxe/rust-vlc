//! MSB-first entropy bit reader with byte unstuffing and marker
//! detection (ISO/IEC 10918-1 §F.1.2.3 / F.2.2.4, cited public spec).
//!
//! Behavior:
//! - `0xFF 0x00` in the entropy stream is a stuffed literal `0xFF`
//!   (F.1.2.3).
//! - `0xFF D0..D7` is a restart marker: the reader stops feeding bits
//!   at the `0xFF` and records the pending marker; the scan decoder
//!   consumes it at the MCU boundary via [`BitReader::take_restart`].
//! - Any other `0xFF xx` stops feeding (a real marker: end of scan).
//! - `0xFF 0xFF` before a marker is a fill byte (F.1.2.3): skipped.
//! - Past the end of the data, no bits are fabricated: the Huffman
//!   layer turns absence into concealment, never into zero bits.

/// Restart-marker band (RST0..RST7).
const RST_FIRST: u8 = 0xd0;
const RST_LAST: u8 = 0xd7;

/// Why the reader has no more bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReaderStop {
    /// Entropy data simply ended.
    EndOfData,
    /// A restart marker (RSTm) is pending at the current position.
    Restart(u8),
    /// A non-restart marker terminates the scan here.
    Marker(u8),
}

/// Bit reader over one entropy-coded segment.
pub(crate) struct BitReader<'a> {
    data: &'a [u8],
    /// Next unread *byte* position.
    pos: usize,
    /// Bit accumulator (MSB of the remaining `nbits` is the next bit).
    acc: u32,
    nbits: u32,
    /// Set once feeding stopped at a marker or the end of data.
    stop: Option<ReaderStop>,
}

impl<'a> BitReader<'a> {
    /// Reader over the entropy bytes (starting wherever SOS ended).
    pub(crate) fn new(data: &'a [u8]) -> BitReader<'a> {
        BitReader {
            data,
            pos: 0,
            acc: 0,
            nbits: 0,
            stop: None,
        }
    }

    /// Feed one unstuffed byte into the accumulator; returns `false`
    /// at a marker or the end of data (idempotent after that).
    fn feed(&mut self) -> bool {
        if self.stop.is_some() {
            return false;
        }
        loop {
            let Some(&byte) = self.data.get(self.pos) else {
                self.stop = Some(ReaderStop::EndOfData);
                return false;
            };
            if byte != 0xff {
                self.pos += 1;
                self.acc = (self.acc << 8) | u32::from(byte);
                self.nbits += 8;
                return true;
            }
            let Some(&next) = self.data.get(self.pos + 1) else {
                // Truncated inside/before a stuffing pair: end of data.
                self.stop = Some(ReaderStop::EndOfData);
                return false;
            };
            match next {
                0x00 => {
                    // Stuffed literal 0xFF (F.1.2.3).
                    self.pos += 2;
                    self.acc = (self.acc << 8) | 0xff;
                    self.nbits += 8;
                    return true;
                }
                0xff => {
                    // Fill byte before a marker (F.1.2.3): skip it.
                    self.pos += 1;
                }
                RST_FIRST..=RST_LAST => {
                    self.stop = Some(ReaderStop::Restart(next - RST_FIRST));
                    return false;
                }
                other => {
                    self.stop = Some(ReaderStop::Marker(other));
                    return false;
                }
            }
        }
    }

    /// Read one bit; `None` when no bits remain (marker or end).
    pub(crate) fn next_bit(&mut self) -> Option<bool> {
        if self.nbits == 0 && !self.feed() {
            return None;
        }
        if self.nbits == 0 {
            return None;
        }
        self.nbits -= 1;
        Some((self.acc >> self.nbits) & 1 == 1)
    }

    /// Read `n` bits MSB-first (`n <= 16`); `None` when the data ends
    /// before `n` bits are available. Partial bits are consumed.
    pub(crate) fn read_bits(&mut self, n: u32) -> Option<u32> {
        debug_assert!(n <= 16);
        let mut value = 0u32;
        for _ in 0..n {
            let bit = self.next_bit()?;
            value = (value << 1) | u32::from(bit);
        }
        Some(value)
    }

    /// The stop cause, if feeding has halted.
    pub(crate) fn stop(&self) -> Option<ReaderStop> {
        self.stop
    }

    /// Consume the restart marker the reader stopped at, verifying the
    /// sequence number `expected_m` (RSTm cycles mod 8, §F.2.2.4).
    ///
    /// On success, discards any padding bits still buffered (the
    /// encoder pads to a byte boundary before the marker) and consumes
    /// the two marker bytes. `Err(())` — with the reader left exactly
    /// as it was — when no marker sits here or the number is wrong
    /// (both concealable damage; the stop cause is preserved).
    pub(crate) fn take_restart(&mut self, expected_m: u8) -> Result<(), ()> {
        match self.stop {
            Some(ReaderStop::Restart(m)) if m == expected_m => {
                self.acc = 0;
                self.nbits = 0;
                self.stop = None;
                self.pos += 2; // consume FF Dm
                Ok(())
            }
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_msb_first_in_order() {
        let mut r = BitReader::new(&[0b1010_1100, 0b0110_0000]);
        let bits: Vec<bool> = (0..11).map(|_| r.next_bit().unwrap_or(false)).collect();
        let want = [
            true, false, true, false, true, true, false, false, false, true, true,
        ];
        assert_eq!(bits, want.to_vec());
    }

    #[test]
    fn stuffed_ff_yields_literal_bits() {
        // FF 00 in the stream is a literal FF byte: 1111 1111.
        let mut r = BitReader::new(&[0xff, 0x00, 0b0_0000001]);
        let mut bits = Vec::new();
        while let Some(b) = r.next_bit() {
            bits.push(b);
        }
        assert_eq!(
            bits.len(),
            16,
            "8 literal bits from the stuffed FF + 8 from the last byte"
        );
        assert!(bits[..8].iter().all(|&b| b), "stuffed FF is all ones");
    }

    #[test]
    fn restart_marker_stops_feeding_and_can_be_taken() {
        let mut r = BitReader::new(&[0b1010_0000, 0xff, 0xd1, 0xff, 0xd2]);
        // The eight bits of the first byte are all real.
        for want in [true, false, true, false, false, false, false, false] {
            assert_eq!(r.next_bit(), Some(want));
        }
        // The next read attempts to feed and stops at the RST1.
        assert_eq!(r.next_bit(), None);
        assert_eq!(r.stop(), Some(ReaderStop::Restart(1)));
        assert!(r.take_restart(1).is_ok());
        // Next bits come after the marker.
        assert_eq!(r.next_bit(), None, "0xff 0xd2 is a marker, not data");
        assert_eq!(r.stop(), Some(ReaderStop::Restart(2)));
    }

    #[test]
    fn wrong_restart_number_is_rejected() {
        let mut r = BitReader::new(&[0xff, 0xd3]);
        assert_eq!(r.next_bit(), None);
        assert_eq!(r.stop(), Some(ReaderStop::Restart(3)));
        assert!(r.take_restart(0).is_err(), "RST3 is not RST0");
        assert_eq!(r.stop(), Some(ReaderStop::Restart(3)), "cause preserved");
    }

    #[test]
    fn non_restart_marker_stops_the_scan() {
        let mut r = BitReader::new(&[0b1_0000000, 0xff, 0xd9]);
        assert_eq!(r.next_bit(), Some(true));
        for _ in 0..7 {
            assert_eq!(r.next_bit(), Some(false));
        }
        assert_eq!(r.next_bit(), None);
        assert_eq!(r.stop(), Some(ReaderStop::Marker(0xd9)));
    }

    #[test]
    fn truncated_stuffing_pair_is_end_of_data() {
        let mut r = BitReader::new(&[0x42, 0xff]);
        assert_eq!(r.read_bits(8), Some(0x42));
        assert_eq!(r.next_bit(), None);
        assert_eq!(r.stop(), Some(ReaderStop::EndOfData));
    }

    #[test]
    fn read_bits_returns_none_on_partial_availability() {
        // NB: the byte must not be 0xFF (that would be a stuffing
        // prefix); 0x77 carries seven set bits.
        let mut r = BitReader::new(&[0b0111_0111]);
        assert_eq!(r.read_bits(3), Some(0b011));
        assert_eq!(r.read_bits(16), None, "only 5 bits remain");
    }

    #[test]
    fn fill_bytes_before_a_restart_are_tolerated() {
        // FF FF D0: the first FF is a fill byte (F.1.2.3).
        let mut r = BitReader::new(&[0xff, 0xff, 0xd0]);
        assert_eq!(r.next_bit(), None);
        assert_eq!(r.stop(), Some(ReaderStop::Restart(0)));
        assert!(r.take_restart(0).is_ok());
        assert_eq!(r.next_bit(), None);
        assert_eq!(r.stop(), Some(ReaderStop::EndOfData));
    }

    #[test]
    fn take_restart_without_a_pending_marker_fails() {
        let mut r = BitReader::new(&[0b0_0000000]);
        // Bits remain; no marker pending.
        assert!(r.take_restart(0).is_err());
        assert_eq!(r.next_bit(), Some(false), "reader still usable");
    }
}
