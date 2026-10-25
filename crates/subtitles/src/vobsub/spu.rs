//! VobSub SPU (sub-picture unit) decode — the DVD subpicture packet
//! grammar (control sequences, dates, palette/alpha/coords commands,
//! the nibble-RLE interlaced bitmap) plus the MPEG-PS PES walker
//! that carries SPUs in a `.sub` file.
//!
//! Reference: ffmpeg `dvdsubdec.c` (this wave's differential
//! oracle) and the de-facto VobSub format documentation — the PS
//! demuxer (5.I2a) already routes `0xBD` + substream `0x20..=0x3F`
//! payloads as `"dvd_subtitle"`; this module is the standalone
//! idx/sub file path and the SPU decode both feed.
//!
//! # SPU layout (standard, non-HD)
//!
//! ```text
//! [0..2]  SPU total size (BE16)
//! [2..4]  offset of the first display-control command sequence
//! ...     RLE bitmap data (two interlaced fields)
//! at the offset: DCSQT = [date BE16][next DCSQT offset BE16]
//!               [commands...][0xFF]
//! ```
//!
//! Commands: `0x00` forced/menu, `0x01` start display, `0x02` end
//! date, `0x03` colormap (4×2-bit nibbles into the idx palette),
//! `0x04` alpha (4×4-bit), `0x05` coords (12-bit x1/x2/y1/y2),
//! `0x06` top/bottom field RLE offsets. `0x01`/`0x02` take NO
//! arguments — the display dates come from the DCSQT date fields
//! (the reference walks the chain and stamps whichever sequence the
//! command sits in). HD variants (`0x83..0x86`) are typed
//! unsupported. Dates are `(date << 10) / 90` milliseconds relative
//! to the PES PTS (the idx timestamp).
//!
//! # RLE (4-bit nibble codes)
//!
//! Nibbles accumulate while the value stays below the growing
//! threshold (`1, 4, 16, 64`): a 1-nibble code `n ≥ 4` is a run of
//! `n >> 2` pixels of color `n & 3`; longer codes prefix zero
//! nibbles and reach runs up to 16383; a final value `< 4` fills the
//! rest of the line. A run that would cross the line end is a typed
//! decode error (the reference rejects the packet); lines are
//! byte-aligned after each row.

use crate::dvb::rle::BitReader;

/// Typed SPU decode damage/errors (data, never a panic).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpuError {
    /// Fewer than 10 bytes — not an SPU header.
    TooSmall {
        got: usize,
    },
    /// `cmd_pos` points past the declared size — the packet is
    /// garbage (the reference discards it).
    InvalidCommandOffset {
        cmd_pos: usize,
        size: usize,
    },
    /// `cmd_pos` is past the data received so far but within the
    /// declared size — feed more PES fragments.
    NeedMoreData {
        have: usize,
        need: usize,
    },
    /// A command sequence overran the packet.
    CommandOverrun,
    /// A required command (0x05 coords / 0x06 offsets) never came.
    MissingCommands,
    /// A run crossed the line end — the bitmap decode failed.
    RleOverrun {
        x: usize,
        width: usize,
    },
    /// The HD-DVD command set (0x83..0x86) — unsupported, typed.
    HdUnsupported {
        command: u8,
    },
    /// The declared size exceeds the accumulation cap (128 KiB, the
    /// reference's reconstruction buffer).
    SizeOverCap {
        size: usize,
    },
}

impl std::fmt::Display for SpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpuError::TooSmall { got } => write!(f, "SPU packet too small: {got} bytes"),
            SpuError::InvalidCommandOffset { cmd_pos, size } => {
                write!(f, "SPU command offset {cmd_pos} past size {size} — discarded")
            }
            SpuError::NeedMoreData { have, need } => {
                write!(f, "SPU packet incomplete: {have} of {need} bytes — feed more PES")
            }
            SpuError::CommandOverrun => f.write_str("SPU command sequence overruns the packet"),
            SpuError::MissingCommands => f.write_str("SPU missing coords (0x05) or offsets (0x06)"),
            SpuError::RleOverrun { x, width } => {
                write!(f, "SPU RLE run crossed the line end at x={x} of {width}")
            }
            SpuError::HdUnsupported { command } => {
                write!(f, "HD-DVD SPU command 0x{command:02X} unsupported")
            }
            SpuError::SizeOverCap { size } => {
                write!(f, "SPU size {size} over the 128 KiB cap")
            }
        }
    }
}

impl std::error::Error for SpuError {}

/// The SPU accumulation cap (the reference decoder's reconstruction
/// buffer size).
pub const SPU_MAX_SIZE: usize = 128 * 1024;

/// One decoded SPU: the bitmap (4 color indices, interlaced rows
/// already woven), its coords, and the display dates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpuDecoded {
    /// Top-left x on the video canvas.
    pub x: u16,
    /// Top-left y.
    pub y: u16,
    pub width: usize,
    pub height: usize,
    /// `width * height` color indices 0..4 (palette applied later).
    pub indices: Vec<u8>,
    /// The 4 colormap entries (idx-palette indices).
    pub colormap: [u8; 4],
    /// The 4 alpha nibbles (0..15).
    pub alpha: [u8; 4],
    /// Display start, ms relative to the PES PTS (command 0x01).
    pub start_ms: i64,
    /// Display end, ms relative to the PES PTS (command 0x02);
    /// `None` when the stream never sent 0x02.
    pub end_ms: Option<i64>,
    /// The forced/menu flag (command 0x00).
    pub forced: bool,
    /// The declared SPU size (for PES accumulation).
    pub size: usize,
}

/// Try to decode one complete SPU packet.
///
/// # Errors
/// [`SpuError`] — see the variants. `NeedMoreData` is the
/// accumulate-more-PES signal.
pub fn decode_spu(data: &[u8]) -> Result<SpuDecoded, SpuError> {
    if data.len() < 10 {
        return Err(SpuError::TooSmall { got: data.len() });
    }
    let big_offsets = u16::from_be_bytes([data[0], data[1]]) == 0;
    let offset_size = if big_offsets { 4 } else { 2 };
    let read_offset = |at: usize| -> usize {
        if big_offsets {
            u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize
        } else {
            u16::from_be_bytes([data[at], data[at + 1]]) as usize
        }
    };
    let size = read_offset(if big_offsets { 2 } else { 0 });
    if size > SPU_MAX_SIZE {
        return Err(SpuError::SizeOverCap { size });
    }
    let cmd_pos_field = if big_offsets { 6 } else { 2 };
    let cmd_pos = read_offset(cmd_pos_field);
    if cmd_pos > data.len().saturating_sub(2 + offset_size) {
        if cmd_pos > size {
            return Err(SpuError::InvalidCommandOffset { cmd_pos, size });
        }
        return Err(SpuError::NeedMoreData {
            have: data.len(),
            need: cmd_pos + 2 + offset_size,
        });
    }

    // Walk the chained display-control sequences.
    let mut colormap = [0u8; 4];
    let mut alpha = [0u8; 4];
    let (mut x1, mut x2, mut y1, mut y2) = (0usize, 0usize, 0usize, 0usize);
    let mut offset1: i64 = -1;
    let mut offset2: i64 = -1;
    let mut start_ms: Option<i64> = None;
    let mut end_ms: Option<i64> = None;
    let mut forced = false;

    let mut seq = cmd_pos;
    while seq > 0 && seq < data.len().saturating_sub(2 + offset_size) {
        let date = u16::from_be_bytes([data[seq], data[seq + 1]]) as i64;
        let next = read_offset(seq + 2);
        let mut pos = seq + 2 + offset_size;
        loop {
            if pos >= data.len() {
                return Err(SpuError::CommandOverrun);
            }
            let cmd = data[pos];
            pos += 1;
            match cmd {
                0x00 => forced = true,
                0x01 => start_ms = Some((date << 10) / 90),
                0x02 => end_ms = Some((date << 10) / 90),
                0x03 => {
                    if data.len() - pos < 2 {
                        return Err(SpuError::CommandOverrun);
                    }
                    colormap[3] = data[pos] >> 4;
                    colormap[2] = data[pos] & 0x0F;
                    colormap[1] = data[pos + 1] >> 4;
                    colormap[0] = data[pos + 1] & 0x0F;
                    pos += 2;
                }
                0x04 => {
                    if data.len() - pos < 2 {
                        return Err(SpuError::CommandOverrun);
                    }
                    alpha[3] = data[pos] >> 4;
                    alpha[2] = data[pos] & 0x0F;
                    alpha[1] = data[pos + 1] >> 4;
                    alpha[0] = data[pos + 1] & 0x0F;
                    pos += 2;
                }
                0x05 | 0x85 => {
                    if data.len() - pos < 6 {
                        return Err(SpuError::CommandOverrun);
                    }
                    x1 = ((data[pos] as usize) << 4) | (data[pos + 1] as usize >> 4);
                    x2 = (((data[pos + 1] & 0x0F) as usize) << 8) | data[pos + 2] as usize;
                    y1 = ((data[pos + 3] as usize) << 4) | (data[pos + 4] as usize >> 4);
                    y2 = (((data[pos + 4] & 0x0F) as usize) << 8) | data[pos + 5] as usize;
                    if cmd == 0x85 {
                        return Err(SpuError::HdUnsupported { command: cmd });
                    }
                    pos += 6;
                }
                0x06 => {
                    if data.len() - pos < 4 {
                        return Err(SpuError::CommandOverrun);
                    }
                    offset1 = u16::from_be_bytes([data[pos], data[pos + 1]]) as i64;
                    offset2 = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as i64;
                    pos += 4;
                }
                0x86 | 0x83 | 0x84 => {
                    return Err(SpuError::HdUnsupported { command: cmd });
                }
                0xFF => break,
                _ => break, // Unknown command: stop this sequence.
            }
        }
        if next < seq {
            break; // Invalid chain: stop (reference behavior).
        }
        if next == seq {
            break;
        }
        seq = next;
    }

    if offset1 < 0 || offset2 < 0 || (x2 < x1 && y2 < y1) {
        return Err(SpuError::MissingCommands);
    }
    let (offset1, offset2) = (offset1 as usize, offset2 as usize);
    if offset1 >= data.len() || offset2 >= data.len() {
        return Err(SpuError::CommandOverrun);
    }

    let w = x2.saturating_sub(x1) + 1;
    let h = y2.saturating_sub(y1) + 1;
    if w == 0 || h <= 1 {
        return Err(SpuError::MissingCommands);
    }

    // Two interlaced fields into one woven bitmap: offset1 = top
    // (even rows), offset2 = bottom (odd rows).
    let mut indices = vec![0u8; w * h];
    let mut used_color = [false; 4];
    decode_field(&mut indices, data, offset1, w, (h + 1) / 2, &mut used_color)
        .map_err(|x| SpuError::RleOverrun { x, width: w })?;
    decode_field(&mut indices[w..], data, offset2, w, h / 2, &mut used_color)
        .map_err(|x| SpuError::RleOverrun { x, width: w })?;

    Ok(SpuDecoded {
        x: x1 as u16,
        y: y1 as u16,
        width: w,
        height: h,
        indices,
        colormap,
        alpha,
        start_ms: start_ms.unwrap_or(0),
        end_ms,
        forced,
        size: size.max(data.len()),
    })
}

/// Decode one interlaced field (every other row, `step = 2` rows in
/// the woven bitmap). `Err(x)` is the overrun position.
fn decode_field(
    dest: &mut [u8],
    data: &[u8],
    start: usize,
    w: usize,
    field_h: usize,
    used_color: &mut [bool; 4],
) -> Result<(), usize> {
    if start >= data.len() || w == 0 || field_h == 0 {
        return Ok(());
    }
    let mut gb = BitReader::new(&data[start..]);
    let mut x = 0usize;
    let mut y = 0usize;
    loop {
        if gb.pos() > gb.bit_len() {
            return Err(x);
        }
        let (len, color) = decode_run(&mut gb);
        let room = w - x;
        if len != usize::MAX && len > room {
            return Err(x);
        }
        let take = len.min(room);
        let row = y * 2 * w;
        for slot in &mut dest[row + x..row + x + take] {
            *slot = color as u8;
        }
        used_color[usize::from(color.min(3))] = true;
        x += take;
        if x >= w {
            y += 1;
            if y >= field_h {
                break;
            }
            x = 0;
            // Byte-align between lines (the reference behavior).
            gb.align();
        }
    }
    Ok(())
}

/// The nibble-RLE run decoder: accumulate nibbles while the value
/// stays under the growing threshold (1, 4, 16, 64 — max 4 nibbles).
/// A final value < 4 fills the rest of the line (`usize::MAX`).
fn decode_run(gb: &mut BitReader<'_>) -> (usize, u8) {
    let mut v: u32 = 0;
    let mut t: u32 = 1;
    while v < t && t <= 0x40 {
        v = (v << 4) | u32::from(gb.read(4));
        t <<= 2;
    }
    let color = (v & 3) as u8;
    if v < 4 {
        (usize::MAX, color)
    } else {
        ((v >> 2) as usize, color)
    }
}

/// Extract the PES payloads of the SPU substreams from a `.sub`
/// MPEG-PS byte stream, with each fragment's byte offset in the
/// file (the idx `filepos` targets).
///
/// The walk: scan for `00 00 01 BD` start codes (skipping
/// pack/system headers the way the reference's
/// `find_next_start_code` does), parse the PES length + optional
/// PTS, take the substream byte (`0x20..=0x3F`), and yield the
/// payload after it. Returns `(substream_id, offset, pts_90khz,
/// payload)` in file order.
pub fn extract_spu_pes(sub: &[u8]) -> Vec<(u8, usize, Option<i64>, &[u8])> {
    let mut out = Vec::new();
    let mut p = 0usize;
    while let Some(at) = find_start_code(sub, p, 0xBD) {
        let offset = at;
        let mut q = at + 4;
        if q + 2 > sub.len() {
            break;
        }
        let mut len = u16::from_be_bytes([sub[q], sub[q + 1]]) as usize;
        q += 2;
        let mut pts = None;
        // PES header (MPEG-2 style when the top bits are '10').
        if q < sub.len() {
            let c = sub[q];
            if c & 0xC0 == 0x80 && q + 2 < sub.len() {
                let flags = sub[q + 1];
                let header_len = sub[q + 2] as usize;
                let hp = q + 3;
                if flags & 0x80 != 0 && hp + 5 <= sub.len() {
                    pts = Some(decode_pts(&sub[hp..hp + 5]));
                }
                let _ = header_len;
                q += 3 + header_len.min(len.saturating_sub(3));
                len = len.saturating_sub(3 + header_len);
            } else if c & 0xE0 == 0x20 || c & 0xC0 == 0x40 {
                // MPEG-1 PTS variants: 5-byte PTS after the marker.
                if c & 0xE0 == 0x20 && q + 4 < sub.len() {
                    pts = Some(decode_pts(&sub[q..q + 5]));
                    q += 5;
                    len = len.saturating_sub(5);
                } else if c & 0xC0 == 0x40 {
                    q += 2;
                    len = len.saturating_sub(2);
                }
            } else if c != 0x0F {
                // Unknown header shape: give up on this start code.
                p = at + 4;
                continue;
            }
        }
        if q >= sub.len() {
            break;
        }
        let substream = sub[q];
        q += 1;
        len = len.saturating_sub(1);
        let end = (q + len).min(sub.len());
        if (0x20..=0x3F).contains(&substream) {
            out.push((substream, offset, pts, &sub[q..end]));
        }
        p = end.max(at + 6);
    }
    out
}

/// Find `00 00 01 <code>` from `from` (the linear scan the idx
/// filepos targets rely on — a sub-only region has no other PES).
fn find_start_code(data: &[u8], from: usize, code: u8) -> Option<usize> {
    if data.len() < 4 {
        return None;
    }
    let mut i = from;
    while i + 4 <= data.len() {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 && data[i + 3] == code {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// The 5-byte MPEG PTS (33 bits + marker bits).
fn decode_pts(b: &[u8]) -> i64 {
    (((b[0] >> 1) & 0x07) as i64) << 30
        | (b[1] as i64) << 22
        | (((b[2] >> 1) & 0x7F) as i64) << 15
        | (b[3] as i64) << 7
        | ((b[4] >> 1) & 0x7F) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One SPU packet from field data + commands. Two chained
    /// DCSQTs carry the dates — `0x01`/`0x02` take no arguments;
    /// the display start stamps the first sequence's date (0), the
    /// stop the second's (0x100 -> 2901 ms), like the reference
    /// streams.
    fn build_spu(top: &[u8], bottom: &[u8], x1: u16, y1: u16, w: u16, h: u16) -> Vec<u8> {
        let x2 = x1 + w - 1;
        let y2 = y1 + h - 1;
        let mut spu = Vec::new();
        spu.extend_from_slice(&[0, 0]); // size (patched)
        spu.extend_from_slice(&[0, 0]); // cmd offset (patched)
        let off1 = spu.len();
        spu.extend_from_slice(top);
        let off2 = spu.len();
        spu.extend_from_slice(bottom);
        let cmds = spu.len();
        // DCSQT 1: date 0, chained to DCSQT 2.
        let cmds2 = cmds + 4 + 1 + 3 + 3 + 7 + 5 + 1;
        spu.extend_from_slice(&[0x00, 0x00]); // date 0 -> start_ms 0
        spu.extend_from_slice(&(cmds2 as u16).to_be_bytes());
        spu.push(0x01); // start display (date from the DCSQT header)
        spu.push(0x03);
        spu.extend_from_slice(&[0x01, 0x23]); // colormap [0]=3,[1]=2,[2]=1,[3]=0
        spu.push(0x04);
        spu.extend_from_slice(&[0x0F, 0x00]); // alpha [0]=0,[1]=0,[2]=15,[3]=0
        spu.push(0x05);
        spu.extend_from_slice(&[
            (x1 >> 4) as u8,
            ((x1 & 0xF) << 4) as u8 | (x2 >> 8) as u8,
            (x2 & 0xFF) as u8,
            (y1 >> 4) as u8,
            ((y1 & 0xF) << 4) as u8 | (y2 >> 8) as u8,
            (y2 & 0xFF) as u8,
        ]);
        spu.push(0x06);
        spu.extend_from_slice(&(off1 as u16).to_be_bytes());
        spu.extend_from_slice(&(off2 as u16).to_be_bytes());
        spu.push(0xFF);
        debug_assert_eq!(spu.len(), cmds2, "cmds2 offset arithmetic");
        // DCSQT 2: date 0x100 -> end_ms (0x100 << 10) / 90; self-chained.
        spu.extend_from_slice(&[0x01, 0x00]);
        spu.extend_from_slice(&(cmds2 as u16).to_be_bytes());
        spu.push(0x02); // stop display
        spu.push(0xFF);
        let total = spu.len();
        spu[0..2].copy_from_slice(&(total as u16).to_be_bytes());
        spu[2..4].copy_from_slice(&(cmds as u16).to_be_bytes());
        spu
    }

    #[test]
    fn nibble_rle_runs() {
        // Direct table checks via a tiny buffer + decode_run.
        let mut gb = BitReader::new(&[0x53]); // nibbles 5, 3
        let (len, color) = decode_run(&mut gb);
        assert_eq!((len, color), (1, 1), "0x5 = run 1 color 1");
        let mut gb = BitReader::new(&[0xC8]); // nibbles C, 8
        let (len, color) = decode_run(&mut gb);
        assert_eq!((len, color), (3, 0), "0xC = run 3 color 0");
        // Nibbles 0, 2, F, 0: v accumulates while under the growing
        // threshold (4, 16, 64) -> v = 0x2F0 = run 188 color 0.
        let mut gb = BitReader::new(&[0x02, 0xF0]);
        let (len, color) = decode_run(&mut gb);
        assert_eq!((len, color), (188, 0));
        // 1 then 8: v = 0x18 = 24 -> run 6 color 0.
        let mut gb = BitReader::new(&[0x18]);
        let (len, color) = decode_run(&mut gb);
        assert_eq!((len, color), (6, 0));
        // Fill-to-end: 0 0 0 2 -> v = 2 < 4.
        let mut gb = BitReader::new(&[0x00, 0x02]);
        let (len, color) = decode_run(&mut gb);
        assert_eq!((len, color), (usize::MAX, 2));
    }

    #[test]
    fn full_spu_decode() {
        // 4x2 bitmap, one line per field. Top line nibbles: 5
        // (1 px c1), 0,0,0,2 (fill-to-end c2) — 5 nibbles padded to
        // 3 bytes: [0x50, 0x00, 0x20]. Bottom: same.
        let top = [0x50u8, 0x00, 0x20];
        let bottom = [0x50u8, 0x00, 0x20];
        let spu = build_spu(&top, &bottom, 100, 200, 4, 2);
        let d = decode_spu(&spu).unwrap();
        assert_eq!((d.x, d.y, d.width, d.height), (100, 200, 4, 2));
        // Row 0 (top field): c1 then 3x c2.
        assert_eq!(&d.indices[0..4], &[1, 2, 2, 2]);
        // Row 1 (bottom field): same.
        assert_eq!(&d.indices[4..8], &[1, 2, 2, 2]);
        assert_eq!(d.colormap, [3, 2, 1, 0]);
        assert_eq!(d.start_ms, 0);
        assert_eq!(d.end_ms, Some((0x100 << 10) / 90));
        assert!(!d.forced);
    }

    #[test]
    fn interlaced_weave_is_correct() {
        // 2x4: top field 2 lines, bottom field 2 lines. Line 1 of
        // each field: 1px cN + fill; line 2: fill c0. Line 1 = 5
        // nibbles -> [0xX0, 0x00, 0xY0]; line 2 = fill c0 = 4 zero
        // nibbles = [0x00, 0x00].
        let top = [0x50u8, 0x00, 0x20, 0x00, 0x00]; // c1 + fill c2; fill c0
        let bottom = [0x70u8, 0x00, 0x10, 0x00, 0x00]; // c3 + fill c1; fill c0
        let spu = build_spu(&top, &bottom, 0, 0, 2, 4);
        let d = decode_spu(&spu).unwrap();
        assert_eq!(d.height, 4);
        assert_eq!(&d.indices[0..2], &[1, 2], "row0 = top line 1");
        assert_eq!(&d.indices[2..4], &[3, 1], "row1 = bottom line 1");
        assert_eq!(&d.indices[4..6], &[0, 0], "row2 = top line 2");
        assert_eq!(&d.indices[6..8], &[0, 0], "row3 = bottom line 2");
    }

    #[test]
    fn overrun_is_typed_error() {
        // Nibbles 0,2,F,0: v = 0x2F0 = run 188 of color 0 in a
        // 2-wide row — the run crosses the line end, typed error.
        let top = [0x02, 0xF0];
        let spu = build_spu(&top, &top, 0, 0, 2, 2);
        assert!(matches!(decode_spu(&spu), Err(SpuError::RleOverrun { x: 0, width: 2 })));
    }

    #[test]
    fn truncated_and_invalid_are_typed() {
        assert!(matches!(decode_spu(&[0, 5]), Err(SpuError::TooSmall { got: 2 })));
        // cmd_pos > size -> discarded.
        let mut spu = vec![0u8; 12];
        spu[0..2].copy_from_slice(&10u16.to_be_bytes()); // size 10
        spu[2..4].copy_from_slice(&500u16.to_be_bytes()); // cmd offset 500
        assert!(matches!(
            decode_spu(&spu),
            Err(SpuError::InvalidCommandOffset { cmd_pos: 500, size: 10 })
        ));
        // cmd_pos inside size but past data -> NeedMoreData.
        let mut spu = vec![0u8; 12];
        spu[0..2].copy_from_slice(&600u16.to_be_bytes());
        spu[2..4].copy_from_slice(&500u16.to_be_bytes());
        assert!(matches!(
            decode_spu(&spu),
            Err(SpuError::NeedMoreData { have: 12, need: 504 })
        ));
    }

    #[test]
    fn pes_extraction_finds_substreams() {
        // One PES 0xBD, substream 0x20, payload [AA BB].
        let mut ps = Vec::new();
        ps.extend_from_slice(&[0x00, 0x00, 0x01, 0xBD]);
        ps.extend_from_slice(&7u16.to_be_bytes()); // PES len: hdr 3 + sub 1 + data 2... set below
        ps.extend_from_slice(&[0x81, 0x80, 0x00]); // '10', PTS flag, hdr len 0
        ps.push(0x20);
        ps.extend_from_slice(&[0xAA, 0xBB]);
        let total_pes = 3 + 1 + 2;
        ps[4..6].copy_from_slice(&(total_pes as u16).to_be_bytes());
        let found = extract_spu_pes(&ps);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, 0x20);
        assert_eq!(found[0].1, 0);
        assert_eq!(found[0].3, &[0xAA, 0xBB]);
    }

    #[test]
    fn pts_decode_90khz() {
        let pts = 90_000i64; // 1s
        let b = [
            (((pts >> 30) & 0x07) << 1) as u8 | 0x21,
            ((pts >> 22) & 0xFF) as u8,
            ((((pts >> 15) & 0x7F) << 1) as u8) | 0x01,
            ((pts >> 7) & 0xFF) as u8,
            (((pts & 0x7F) << 1) as u8) | 0x01,
        ];
        assert_eq!(decode_pts(&b), pts);
    }
}
