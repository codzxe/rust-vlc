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
//!
//! # ED-FIX-1 (audit C3): 32-bit accumulator, word-granular refill
//!
//! The accumulator holds up to 32 bits and is refilled in a
//! byte-per-iteration loop (up to 4 bytes per refill call) instead of
//! the old bit-at-a-time shifter, with [`BitReader::peek_bits`]/
//! [`BitReader::consume`] splitting look-ahead from consumption so the
//! Huffman layer can run its 8-bit LUT without touching the stream.
//! The observable state machine — bits returned, positions, unstuff
//! behavior, marker stops, partial-consumption on truncation — is
//! identical to the bit-at-a-time reader (pinned by the tests below
//! and by the committed decode-reference digests).

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
    /// Bit accumulator: the `nbits` most significant valid bits hold
    /// the stream window (MSB of the window = next bit to consume).
    acc: u32,
    /// Valid bits in `acc` (0..=32).
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

    /// Refill the accumulator towards 32 bits (byte granular — up to
    /// a word per call). Stops early at a marker or end of data.
    #[inline]
    pub(crate) fn refill(&mut self) {
        while self.nbits <= 24 {
            if !self.feed() {
                break;
            }
        }
    }

    /// Bits currently available for consumption.
    #[inline]
    pub(crate) fn available(&self) -> u32 {
        self.nbits
    }

    /// Look at the next `n ≤ 32` bits **without consuming** them.
    /// Returns `None` when fewer than `n` bits remain.
    #[inline]
    pub(crate) fn peek_bits(&mut self, n: u32) -> Option<u32> {
        debug_assert!(n <= 32);
        self.refill();
        if self.nbits < n {
            return None;
        }
        Some((self.acc >> (self.nbits - n)) & ((1u32 << n) - 1).max(if n == 32 {
            u32::MAX
        } else {
            0
        }))
    }

    /// The next 8 bits, zero-padded on the right when the data is
    /// exhausted (a look-ahead window for the Huffman LUT — padding
    /// never yields a decode because the code length is validated
    /// against the available count by the caller).
    #[inline]
    pub(crate) fn peek8_padded(&self) -> u8 {
        if self.nbits >= 8 {
            ((self.acc >> (self.nbits - 8)) & 0xff) as u8
        } else {
            ((self.acc << (8 - self.nbits)) & 0xff) as u8
        }
    }

    /// Consume `n ≤ available()` bits (the look-ahead counterpart of
    /// [`BitReader::peek_bits`]).
    #[inline]
    pub(crate) fn consume(&mut self, n: u32) {
        debug_assert!(n <= self.nbits);
        self.nbits -= n;
    }

    /// Discard every buffered bit (the truncation path: partial bits
    /// count as consumed, mirroring the old bit-at-a-time reader).
    #[inline]
    pub(crate) fn consume_all(&mut self) {
        self.nbits = 0;
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

    /// Read `n` bits MSB-first; `None` when the data ends before `n`
    /// bits are available. Partial bits are consumed (any `n` is
    /// accepted — a hostile DC size category beyond the baseline
    /// 0..=15 band simply consumes the tail and reports exhaustion,
    /// which the scan decoder conceals; no panic in any build).
    pub(crate) fn read_bits(&mut self, n: u32) -> Option<u32> {
        self.refill();
        if self.nbits < n {
            self.nbits = 0;
            return None;
        }
        let value = (self.acc >> (self.nbits - n)) & mask_bits(n);
        self.nbits -= n;
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

/// Low-`n` mask helper (`n ≤ 32`).
#[inline]
fn mask_bits(n: u32) -> u32 {
    if n == 0 {
        0
    } else if n >= 32 {
        u32::MAX
    } else {
        (1u32 << n) - 1
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

    // --- ED-FIX-1: word-refill accumulator semantics ---

    #[test]
    fn peek_bits_does_not_consume() {
        let mut r = BitReader::new(&[0xab, 0xcd, 0xef, 0x01, 0x23]);
        r.refill();
        assert_eq!(r.available(), 32, "a full word of look-ahead");
        assert_eq!(r.peek_bits(8), Some(0xab));
        assert_eq!(r.peek_bits(8), Some(0xab), "peek is idempotent");
        assert_eq!(r.peek_bits(16), Some(0xabcd));
        r.consume(8);
        assert_eq!(r.peek_bits(8), Some(0xcd));
        r.consume(4);
        assert_eq!(r.peek_bits(4), Some(0xd), "the low nibble of 0xcd");
        // 32 − 12 consumed; the second peek pulled the fifth byte in
        // (nbits had dropped to 24 ≤ 24), so the accumulator holds a
        // full word again.
        assert_eq!(r.available(), 28);
    }

    #[test]
    fn peek8_pads_with_zeros_past_the_end() {
        let mut r = BitReader::new(&[0b1010_0110]);
        r.refill();
        assert_eq!(r.available(), 8);
        r.consume(5);
        // The 3 remaining bits (110) sit at the top of the padded
        // byte window: 1100_0000.
        assert_eq!(r.peek8_padded(), 0b1100_0000);
        r.consume(3);
        assert_eq!(r.available(), 0);
        assert_eq!(r.peek8_padded(), 0, "fully exhausted pads to zero");
    }

    #[test]
    fn read_bits_spans_the_word_refill() {
        // 16-bit reads crossing the 4-byte refill boundary.
        let mut r = BitReader::new(&[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]);
        assert_eq!(r.read_bits(16), Some(0x1234));
        assert_eq!(r.read_bits(16), Some(0x5678));
        assert_eq!(r.read_bits(16), Some(0x9abc));
        assert_eq!(r.read_bits(1), None);
    }

    #[test]
    fn read_bits_zero_returns_zero() {
        let mut r = BitReader::new(&[]);
        assert_eq!(r.read_bits(0), Some(0), "s=0 extend: no bits, no error");
    }

    #[test]
    fn oversized_read_consumes_the_tail_without_panicking() {
        // A hostile DC size category > 16 (rejected at the table level
        // by the spec, but the reader must not panic on it): the
        // partial bits are consumed and the read reports exhaustion.
        let mut r = BitReader::new(&[0b0101_1000]);
        assert_eq!(r.read_bits(200), None);
        assert_eq!(r.available(), 0);
        assert_eq!(r.next_bit(), None);
    }

    #[test]
    fn stuffing_survives_the_word_refill() {
        // Stuffing pairs and fill bytes interleaved with data across
        // refills: the byte-level state machine is unchanged. (A fill
        // byte 0xFF 0xFF only precedes a real marker — FF 55 is a
        // marker prefix, not data.)
        let mut r = BitReader::new(&[0x0f, 0xff, 0x00, 0xf0, 0xff, 0xff, 0xd9]);
        assert_eq!(r.read_bits(8), Some(0x0f));
        assert_eq!(r.read_bits(8), Some(0xff), "stuffed literal FF");
        assert_eq!(r.read_bits(8), Some(0xf0));
        assert_eq!(r.read_bits(1), None);
        assert_eq!(r.stop(), Some(ReaderStop::Marker(0xd9)), "fill byte skipped");
    }
}

