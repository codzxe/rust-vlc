//! DVB subtitle pixel RLE (ETSI EN 300 743 §7.2.5, Table 9/10/11) —
//! the 2-bit, 4-bit and 8-bit code tables, exactly as the reference
//! decoders (ffmpeg `dvbsubdec`, the wave's differential oracle)
//! interpret them.
//!
//! # Code tables (transcribed from EN 300 743, cross-checked line by
//! line against ffmpeg's `dvbsub_read_{2,4,8}bit_string`)
//!
//! **2-bit**: `01|10|11` = one pixel of value 1/2/3; `001 nnn vv` =
//! 3..10 pixels of `vv`; `0001` = one pixel of 0; `000001` = two
//! pixels of 0; `000010 nnnn vv` = 12..27; `000011 nnnnnnnn vv` =
//! 29..284; `000000` = end of line (the zero-pixel codes sit at the
//! same tree position as the 4-bit table's, with the 2-bit table's
//! extra code space affording a single-zero-pixel form).
//!
//! **4-bit**: `1..15` = one pixel; after a `0000` nibble: `0 nnn`
//! (`nnn != 0`) = 2..9 pixels of 0; `10 nn vvvv` = 4..7 of `vvvv`;
//! `11 10 nnnn vvvv` = 9..24; `11 11 nnnnnnnn vvvv` = 25..280;
//! `11 01` = two of 0; `0 000` and `11 00` = end of line (the
//! continuation bits are read after the initial `0000` nibble —
//! ffmpeg's grammar, not a raw bitstring).
//!
//! **8-bit**: `vv != 0` = one pixel of `vv`; `00 0 rrrrrrr` = 1..127
//! pixels of 0; `00 1 rrrrrrr vvvvvvvv` = 1..127 pixels of `vvvvvvvv`;
//! `00 0 00000000` = end of line.
//!
//! A line ends at the EOL code (then the stream is byte-aligned —
//! the EOL code plus zero padding up to the next byte boundary, the
//! `0xF0` field-step sub-block follows). A row that fills exactly
//! does NOT end implicitly: the reader keeps consuming (clipped,
//! no writes past the row) until the EOL — EN 300 743 requires an
//! EOL per line, and the field walker needs it consumed to find
//! `0xF0`. Zero padding at the buffer end reads as EOL (harmless:
//! all-zero bits are the EOL code; the only stop reasons are EOL,
//! buffer-exhausted with the row full, or buffer-exhausted early).
//! A run that would cross the row end is **clipped** to the row
//! (EN 300 743 "pixels shall not be written outside the region") —
//! a typed damage event, never a panic.
//!
//! # Never
//!
//! Panics, allocations proportional to anything but the row length.

/// A big-endian MSB-first bit reader over a borrowed field buffer.
#[derive(Debug)]
pub(crate) struct BitReader<'a> {
    data: &'a [u8],
    /// Bit position from the start of `data`.
    pos: usize,
}

impl<'a> BitReader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> BitReader<'a> {
        BitReader { data, pos: 0 }
    }

    /// Read `n` bits (n <= 16), MSB first; returns 0 past the end
    /// (zero-extension, the reference decoders' behavior — a typed
    /// overrun is reported by the caller when the EOL never comes).
    pub(crate) fn read(&mut self, n: u32) -> u16 {
        debug_assert!(n <= 16);
        let mut v: u16 = 0;
        for _ in 0..n {
            let bit = if self.pos / 8 < self.data.len() {
                (self.data[self.pos / 8] >> (7 - (self.pos % 8))) & 1
            } else {
                0
            };
            v = (v << 1) | u16::from(bit);
            self.pos += 1;
        }
        v
    }

    /// Bytes consumed so far (rounded up — the line is byte-aligned).
    pub(crate) fn consumed_bytes(&self) -> usize {
        self.pos.div_ceil(8)
    }

    /// Discard the remaining bits of the current byte (byte-align).
    pub(crate) fn align(&mut self) {
        self.pos = self.pos.div_ceil(8) * 8;
    }

    pub(crate) fn bit_len(&self) -> usize {
        self.data.len() * 8
    }

    pub(crate) fn pos(&self) -> usize {
        self.pos
    }
}

/// Why a pixel line stopped early (damage data, never a panic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineStop {
    /// The EOL code was seen; the rest of the byte is padding.
    EndOfLine,
    /// The row filled up.
    RowFull,
    /// The field buffer ran out before EOL or a full row.
    BufferExhausted,
}

/// Decode one 2-bit-RLE line into `row[x_start..]` (indices at the
/// region's depth; `map` translates 2-bit codes when the region is
/// deeper; `non_mod` skips writing value-1 pixels — EN 300 743
/// non-modifying color). Returns the bytes consumed, the x after the
/// line, and why the line stopped.
pub(crate) fn decode_line_2bit(
    data: &[u8],
    row: &mut [u8],
    x_start: usize,
    map: Option<&[u8; 4]>,
    non_mod: bool,
) -> (usize, usize, LineStop) {
    let mut gb = BitReader::new(data);
    let mut x = x_start;
    let stop;
    loop {
        if gb.pos() >= gb.bit_len() {
            // Buffer exhausted: a full row is a complete line;
            // anything less is early exhaustion.
            stop = if x >= row.len() {
                LineStop::RowFull
            } else {
                LineStop::BufferExhausted
            };
            break;
        }
        let bits = gb.read(2);
        if bits != 0 {
            if x < row.len() && (!non_mod || bits != 1) {
                let v = map.map_or(u32::from(bits), |m| u32::from(m[bits as usize]));
                row[x] = v as u8;
            }
            x += 1;
            continue;
        }
        match gb.read(1) {
            1 => {
                // 001 nnn vv — 3..10 pixels.
                let run = u32::from(gb.read(3)) + 3;
                let v = gb.read(2);
                let v = map.map_or(u32::from(v), |m| u32::from(m[v as usize]));
                x = fill(row, x, run, v as u8, non_mod);
            }
            _ => match gb.read(1) {
                0 => match gb.read(2) {
                    2 => {
                        // 000010 nnnn vv — 12..27.
                        let run = u32::from(gb.read(4)) + 12;
                        let v = gb.read(2);
                        let v = map.map_or(u32::from(v), |m| u32::from(m[v as usize]));
                        x = fill(row, x, run, v as u8, non_mod);
                    }
                    3 => {
                        // 000011 nnnnnnnn vv — 29..284.
                        let run = u32::from(gb.read(8)) + 29;
                        let v = gb.read(2);
                        let v = map.map_or(u32::from(v), |m| u32::from(m[v as usize]));
                        x = fill(row, x, run, v as u8, non_mod);
                    }
                    1 => {
                        // 000001 — two pixels of 0.
                        x = fill(row, x, 2, map.map_or(0, |m| m[0]), false);
                    }
                    _ => {
                        stop = LineStop::EndOfLine;
                        break;
                    }
                },
                _ => {
                    // 0001 — one pixel of 0.
                    if x < row.len() {
                        row[x] = map.map_or(0, |m| m[0]);
                    }
                    x += 1;
                }
            },
        }
    }
    (gb.consumed_bytes().min(data.len()), x, stop)
}

/// Decode one 4-bit-RLE line (see the module docs for the table).
pub(crate) fn decode_line_4bit(
    data: &[u8],
    row: &mut [u8],
    x_start: usize,
    map: Option<&[u8; 16]>,
    non_mod: bool,
) -> (usize, usize, LineStop) {
    let mut gb = BitReader::new(data);
    let mut x = x_start;
    let stop;
    loop {
        if gb.pos() >= gb.bit_len() {
            stop = if x >= row.len() {
                LineStop::RowFull
            } else {
                LineStop::BufferExhausted
            };
            break;
        }
        let bits = gb.read(4);
        if bits != 0 {
            if x < row.len() && (!non_mod || bits != 1) {
                let v = map.map_or(u32::from(bits), |m| u32::from(m[bits as usize]));
                row[x] = v as u8;
            }
            x += 1;
            continue;
        }
        match gb.read(1) {
            0 => {
                let run = u32::from(gb.read(3));
                if run == 0 {
                    stop = LineStop::EndOfLine;
                    break;
                }
                x = fill(row, x, run + 2, map.map_or(0, |m| m[0]), false);
            }
            _ => match gb.read(1) {
                0 => {
                    // 0000 + 10 nn vvvv — 4..7.
                    let run = u32::from(gb.read(2)) + 4;
                    let v = gb.read(4);
                    let v = map.map_or(u32::from(v), |m| u32::from(m[v as usize]));
                    x = fill(row, x, run, v as u8, non_mod);
                }
                _ => match gb.read(2) {
                    2 => {
                        // 0000 + 11 10 nnnn vvvv — 9..24.
                        let run = u32::from(gb.read(4)) + 9;
                        let v = gb.read(4);
                        let v = map.map_or(u32::from(v), |m| u32::from(m[v as usize]));
                        x = fill(row, x, run, v as u8, non_mod);
                    }
                    3 => {
                        // 0000 + 11 11 nnnnnnnn vvvv — 25..280.
                        let run = u32::from(gb.read(8)) + 25;
                        let v = gb.read(4);
                        let v = map.map_or(u32::from(v), |m| u32::from(m[v as usize]));
                        x = fill(row, x, run, v as u8, non_mod);
                    }
                    1 => {
                        // 0000 + 11 01 — two of 0.
                        x = fill(row, x, 2, map.map_or(0, |m| m[0]), false);
                    }
                    _ => {
                        stop = LineStop::EndOfLine;
                        break;
                    }
                },
            },
        }
    }
    (gb.consumed_bytes().min(data.len()), x, stop)
}

/// Decode one 8-bit-RLE line (byte codes, see the module docs).
pub(crate) fn decode_line_8bit(
    data: &[u8],
    row: &mut [u8],
    x_start: usize,
    non_mod: bool,
) -> (usize, usize, LineStop) {
    let mut i = 0usize;
    let mut x = x_start;
    let stop;
    loop {
        if i >= data.len() {
            stop = if x >= row.len() {
                LineStop::RowFull
            } else {
                LineStop::BufferExhausted
            };
            break;
        }
        let v = data[i];
        i += 1;
        if v != 0 {
            if x < row.len() && (!non_mod || v != 1) {
                row[x] = v;
            }
            x += 1;
            continue;
        }
        // 00 — run code follows.
        let n = *data.get(i).unwrap_or(&0);
        i += 1;
        let run = u32::from(n & 0x7f);
        if n & 0x80 == 0 {
            if run == 0 {
                stop = LineStop::EndOfLine;
                break;
            }
            x = fill(row, x, run, 0, false);
        } else {
            let color = *data.get(i).unwrap_or(&0);
            i += 1;
            x = fill(row, x, run, color, non_mod);
        }
    }
    (i, x, stop)
}

/// Clip `run` pixels of `v` into `row` at `x` (EN 300 743: pixels
/// past the region edge are dropped, not wrapped). `non_mod`
/// suppresses writes of value 1 (the non-modifying color). `x` may
/// already be past the row end (single-pixel codes advance it
/// unclipped so the EOL search stays bit-accurate) — the slice is
/// clamped at both ends, never panicking (the fuzz-survival pin).
#[inline]
fn fill(row: &mut [u8], x: usize, run: u32, v: u8, non_mod: bool) -> usize {
    let start = x.min(row.len());
    let end = (x + run as usize).min(row.len());
    if !(non_mod && v == 1) {
        for slot in &mut row[start..end] {
            *slot = v;
        }
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assert a full row decode with expected pixels and stop reason.
    fn check_2bit(data: &[u8], width: usize, want: &[u8], want_stop: LineStop) {
        let mut row = vec![0xAA; width];
        let (used, _x, stop) = decode_line_2bit(data, &mut row, 0, None, false);
        assert_eq!(&row[..want.len()], want, "2-bit pixels");
        assert_eq!(stop, want_stop, "2-bit stop");
        assert!(used <= data.len() + 1, "consumed {used} of {}", data.len());
    }

    #[test]
    fn two_bit_single_pixel_codes() {
        // 01 10 11 -> 1,2,3 then 000000 EOL: b0 = 0b01_10_11_00, rest 0.
        check_2bit(&[0b01_10_11_00], 8, &[1, 2, 3], LineStop::EndOfLine);
    }

    #[test]
    fn two_bit_switch_runs_and_eol() {
        // 001 000 01 = run 3 of 1; then 0001 = one 0; then EOL
        // (the padding/zero bits read as the all-zero EOL code).
        check_2bit(
            &[0b00100001, 0b0001_0000],
            8,
            &[1, 1, 1, 0],
            LineStop::EndOfLine,
        );
    }

    #[test]
    fn two_bit_long_runs() {
        // 000010 0001 11 = 12+1=13 px of 3 (fills the row of 13;
        // the trailing padding zeros then read as EOL — clipped, no
        // writes past the row).
        check_2bit(
            &[0b00001000, 0b0111_0000],
            13,
            &[3; 13],
            LineStop::EndOfLine,
        );
        // 000011 00000010 10 = 29+2=31 px of 2 (row exactly full).
        let mut row = vec![0xAA; 31];
        let (_, _, stop) = decode_line_2bit(
            &[0b00001100, 0b0000_1010, 0b0000_0000],
            &mut row,
            0,
            None,
            false,
        );
        assert_eq!(row, vec![2u8; 31]);
        assert_eq!(stop, LineStop::EndOfLine);
    }

    #[test]
    fn two_bit_map_table_translates() {
        // 001 011 10 -> run 6 of code 2 -> mapped to 0x88 via map2to8.
        let map = [0x00u8, 0x77, 0x88, 0xff];
        let mut row = vec![0xAA; 6];
        let (used, _x, stop) =
            decode_line_2bit(&[0b00101110, 0b0000_0000], &mut row, 0, Some(&map), false);
        assert_eq!(row, vec![0x88u8; 6]);
        // EOL is in the second byte (bits 9-14): 2 bytes consumed.
        assert_eq!(stop, LineStop::EndOfLine);
        assert_eq!(used, 2);
    }

    #[test]
    fn two_bit_run_clipped_at_row_end() {
        // 000011 11111111 01 = 284 px of 1 in a 5-wide row: clipped.
        let mut row = vec![0xAA; 5];
        let (_, _, stop) =
            decode_line_2bit(&[0b00001111, 0b1111_1101, 0], &mut row, 0, None, false);
        assert_eq!(row, vec![1u8; 5]);
        assert_eq!(stop, LineStop::EndOfLine);
    }

    #[test]
    fn two_bit_run_after_x_passed_row_end_is_clipped_not_panicking() {
        // Singles 3,1,2 fill the 3-wide row, then one more single
        // pushes x to 4 (past the end — no write), then a 6-px run
        // of 2 lands entirely past the row: clipped no-op, no
        // slice-panic (the fuzz-survival pin). Bits:
        // 11 01 10 01 001 011 10 000000 -> 0xD9 0x2E 0x00.
        let mut row = vec![0xAA; 3];
        let (_, _, stop) =
            decode_line_2bit(&[0b1101_1001, 0b0010_1110, 0x00], &mut row, 0, None, false);
        assert_eq!(row, vec![3u8, 1, 2]);
        assert_eq!(stop, LineStop::EndOfLine);
    }

    #[test]
    fn two_bit_buffer_exhausted_is_typed() {
        // A code that needs more bits than remain: zero-extension
        // decodes as EOL-ish padding; the stop is BufferExhausted only
        // when even the code start is missing. One lone 0x00 byte at
        // bit level reads 00 -> 0 -> 0 -> 00 = EOL at 6 bits.
        let mut row = vec![0xAA; 4];
        let (used, _x, stop) = decode_line_2bit(&[0x00], &mut row, 0, None, false);
        assert_eq!(stop, LineStop::EndOfLine);
        assert_eq!(used, 1);
        // Truly empty: exhausted.
        let (_, _, stop) = decode_line_2bit(&[], &mut row, 0, None, false);
        assert_eq!(stop, LineStop::BufferExhausted);
    }

    #[test]
    fn four_bit_codes() {
        // Nibbles 1, 2, F = single pixels; then `0000` + `10` + `01`
        // + `1001` = run 5 (4+1) of color 9; then `0000 0 000` EOL
        // in the padding. MSB-first packed: 0001 0010 1111 0000 1001
        // 1001 0000 0000 = 0x12 0xF0 0x99 0x00.
        let mut row = vec![0xAA; 8];
        let (_, _, stop) = decode_line_4bit(&[0x12, 0xF0, 0x99, 0x00], &mut row, 0, None, false);
        assert_eq!(&row[..8], &[1, 2, 15, 9, 9, 9, 9, 9]);
        assert_eq!(stop, LineStop::EndOfLine);
    }

    #[test]
    fn four_bit_zero_runs_and_eol() {
        // `0000 0 011` = run 3+2 = 5 px of 0; then EOL in padding.
        let mut row = vec![0xAA; 5];
        let (_, _, stop) = decode_line_4bit(&[0b0000_0011, 0x00], &mut row, 0, None, false);
        assert_eq!(row, vec![0u8; 5]);
        assert_eq!(stop, LineStop::EndOfLine);
        // `0000 11 10 0001 0010` = run 9+1 = 10 px of 2 (row exactly
        // full, buffer exhausted — RowFull).
        let mut row = vec![0xAA; 10];
        let (_, _, stop) = decode_line_4bit(&[0b0000_1110, 0b0001_0010], &mut row, 0, None, false);
        assert_eq!(row, vec![2u8; 10]);
        assert_eq!(stop, LineStop::RowFull);
    }

    #[test]
    fn eight_bit_codes() {
        // 05 = one px 5; 00 82 07 = 2 px of 7; 00 03 = 3 px of 0; 00 00 EOL.
        let mut row = vec![0xAA; 6];
        let (used, _x, stop) = decode_line_8bit(
            &[0x05, 0x00, 0x82, 0x07, 0x00, 0x03, 0x00, 0x00],
            &mut row,
            0,
            false,
        );
        assert_eq!(row, vec![5, 7, 7, 0, 0, 0]);
        assert_eq!(stop, LineStop::EndOfLine);
        assert_eq!(used, 8);
        // 00 80 09 = 0 px of 9? No: run 0 with 0x80 set = run 0 (no-op).
        let mut row = vec![0xAA; 2];
        let (_, _, stop) = decode_line_8bit(&[0x01, 0x00, 0x80, 0x09, 0x02], &mut row, 0, false);
        assert_eq!(row, vec![1, 2]);
        assert_eq!(stop, LineStop::RowFull);
    }

    #[test]
    fn eight_bit_eol_and_exhaustion() {
        let mut row = vec![0xAA; 4];
        let (_, _, stop) = decode_line_8bit(&[0x00, 0x00], &mut row, 0, false);
        assert_eq!(stop, LineStop::EndOfLine);
        let (_, _, stop) = decode_line_8bit(&[], &mut row, 0, false);
        assert_eq!(stop, LineStop::BufferExhausted);
        // Truncated run (00 85 with no color byte): color reads 0,
        // clipped run, buffer exhausted — typed, no panic.
        let mut row = vec![0xAA; 3];
        let (_, _, stop) = decode_line_8bit(&[0x00, 0x85], &mut row, 0, false);
        assert_eq!(row, vec![0, 0, 0]);
        assert_eq!(stop, LineStop::RowFull);
    }
}
