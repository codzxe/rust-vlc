//! The Ogg page layer (card 5.B1): capture-pattern framing, the
//! 27-byte fixed header, the segment (lacing) table, and the Ogg
//! CRC-32 — verify and generate, native (C5: no new deps; the
//! polynomial and shift are specified by RFC 3533 §5).
//!
//! # Page grammar (RFC 3533 §6)
//!
//! ```text
//!  0..4   "OggS"        capture pattern
//!  4      version       0 (RFC 3533 §6.1 — anything else is damage)
//!  5      header_type   bit0 continued, bit1 BOS, bit2 EOS
//!  6..14  granulepos    i64 LE (position of the last packet ending
//!                        on this page; -1 = no position)
//! 14..18  serialno      u32 LE (logical stream id)
//! 18..22  sequenceno    u32 LE (per-stream page counter)
//! 22..26  CRC32         u32 LE over the whole page, field zeroed
//! 26      nsegs         1..=255
//! 27..27+nsegs  segment table (each 0..=255 lacing value)
//! then    body (sum of the lacing values, ≤ 255×255)
//! ```
//!
//! A lacing value < 255 terminates a packet; a run of 255s continues
//! the packet into the next page (the "continued" flag then marks the
//! page's first lacing run as the tail of the previous page's packet).

use media_types::EngineError;

use super::error;

/// The fixed header size before the segment table.
pub const HEADER_BYTES: usize = 27;

/// The capture pattern.
pub const CAPTURE: &[u8; 4] = b"OggS";

/// One parsed page header (the body is read separately — the demuxer
/// pools it; this layer stays allocation-light: one segment-table
/// copy).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageHeader {
    /// Absolute file offset of the capture pattern.
    pub offset: u64,
    /// The version byte (must be 0).
    pub version: u8,
    /// The continued-packet flag (header_type bit 0).
    pub continued: bool,
    /// The beginning-of-stream flag (bit 1).
    pub bos: bool,
    /// The end-of-stream flag (bit 2).
    pub eos: bool,
    /// The granule position (-1 = "no position").
    pub granule: i64,
    /// The logical-stream serial number.
    pub serial: u32,
    /// The per-stream page sequence number.
    pub seq: u32,
    /// The declared CRC-32 (as stored).
    pub crc: u32,
    /// The segment table (nsegs entries).
    pub segments: Vec<u8>,
    /// Sum of the segment table = body length.
    pub body_len: usize,
}

impl PageHeader {
    /// Total on-disk size of this page (header + table + body).
    pub fn total_len(&self) -> u64 {
        (HEADER_BYTES + self.segments.len() + self.body_len) as u64
    }

    /// The offset just past this page.
    pub fn end(&self) -> u64 {
        self.offset + self.total_len()
    }

    /// Which lacing runs complete on this page: returns
    /// `(run_start_in_body, run_len, terminal)` for every run.
    /// A run is terminal when its lacing value < 255 (the packet
    /// ends here); runs are the maximal groups of body bytes a
    /// single packet owns on this page.
    pub fn packet_runs(&self) -> Vec<(usize, usize, bool)> {
        let mut runs = Vec::with_capacity(self.segments.len());
        let mut at = 0usize;
        for &lace in &self.segments {
            runs.push((at, lace as usize, lace < 255));
            at += lace as usize;
        }
        runs
    }
}

/// Parse the fixed header + segment table out of `head`, which must
/// carry at least `HEADER_BYTES` bytes starting at the capture
/// pattern (the body is *not* needed here).
///
/// # Errors
/// Typed [`error::bad_capture`]-family shapes for: short header,
/// wrong capture, nonzero version, zero segment count, or a header +
/// table + body that runs past `stream_len` (the truncated page).
pub fn parse_header(head: &[u8], offset: u64, stream_len: u64) -> Result<PageHeader, EngineError> {
    if head.len() < HEADER_BYTES || &head[0..4] != CAPTURE {
        return Err(error::bad_capture(offset));
    }
    let version = head[4];
    if version != 0 {
        return Err(error::bad_version(offset, version));
    }
    let flags = head[5];
    let granule = i64::from_le_bytes(
        head[6..14]
            .try_into()
            // HEADER_BYTES (27) is checked above; the Err arm is
            // unreachable slicing residue — a malformed page, never
            // a panic (8.A1 sweep).
            .map_err(|_| error::bad_capture(offset))?,
    );
    let serial = u32::from_le_bytes(
        head[14..18]
            .try_into()
            // HEADER_BYTES (27) is checked above; the Err arm is
            // unreachable slicing residue — a malformed page, never
            // a panic (8.A1 sweep).
            .map_err(|_| error::bad_capture(offset))?,
    );
    let seq = u32::from_le_bytes(
        head[18..22]
            .try_into()
            // HEADER_BYTES (27) is checked above; the Err arm is
            // unreachable slicing residue — a malformed page, never
            // a panic (8.A1 sweep).
            .map_err(|_| error::bad_capture(offset))?,
    );
    let crc = u32::from_le_bytes(
        head[22..26]
            .try_into()
            // HEADER_BYTES (27) is checked above; the Err arm is
            // unreachable slicing residue — a malformed page, never
            // a panic (8.A1 sweep).
            .map_err(|_| error::bad_capture(offset))?,
    );
    let nsegs = head[26] as usize;
    if head.len() < HEADER_BYTES + nsegs {
        // The caller's window does not carry the segment table.
        return Err(error::bad_capture(offset));
    }
    if nsegs == 0 {
        // RFC 3533 §6: a page with zero segments is not valid.
        return Err(error::bad_structure(offset));
    }
    let table_end = (HEADER_BYTES + nsegs) as u64;
    if offset + table_end > stream_len {
        return Err(error::truncated_page(offset + table_end, stream_len));
    }
    let segments = head[HEADER_BYTES..HEADER_BYTES + nsegs].to_vec();
    let body_len: usize = segments.iter().map(|&s| s as usize).sum();
    let total = table_end + body_len as u64;
    if offset + total > stream_len {
        return Err(error::truncated_page(offset + total, stream_len));
    }
    Ok(PageHeader {
        offset,
        version,
        continued: flags & 0x01 != 0,
        bos: flags & 0x02 != 0,
        eos: flags & 0x04 != 0,
        granule,
        serial,
        seq,
        crc,
        segments,
        body_len,
    })
}

// ---------------------------------------------------------------------------
// CRC-32 (RFC 3533 §5): polynomial 0x04C11DB7, MSB-first, init 0, no
// final XOR, no reflection — the Ogg variant, natively table-driven.
// ---------------------------------------------------------------------------

/// The 256-entry table, built once (std-only; `OnceLock`, C5).
fn crc_table() -> &'static [u32; 256] {
    use std::sync::OnceLock;
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0u32; 256];
        for (i, slot) in table.iter_mut().enumerate() {
            // The byte enters at the TOP of the register (MSB-first):
            // entries are crc(byte-at-position-0), i.e. i << 24
            // shifted eight times — not a bare low-byte i.
            let mut r = (i as u32) << 24;
            for _ in 0..8 {
                r = if r & 0x8000_0000 != 0 {
                    (r << 1) ^ 0x04C1_1DB7
                } else {
                    r << 1
                };
            }
            *slot = r;
        }
        table
    })
}

/// The Ogg CRC-32 of `data` (see the table docs for the variant).
pub fn crc32(data: &[u8]) -> u32 {
    let table = crc_table();
    let mut crc = 0u32;
    for &byte in data {
        // MSB-first: the top byte of the register selects the entry.
        let idx = ((crc >> 24) as u8 ^ byte) as usize;
        crc = (crc << 8) ^ table[idx];
    }
    crc
}

/// Verify a whole in-memory page (header + table + body, CRC field
/// as stored): `true` iff the recomputed CRC matches.
pub fn crc_valid(page: &[u8]) -> bool {
    if page.len() < HEADER_BYTES {
        return false;
    }
    let (stored, rest) = page.split_at(HEADER_BYTES);
    let declared = match stored[22..26].try_into() {
        Ok(bytes) => u32::from_le_bytes(bytes),
        // stored is the 27-byte split prefix; the Err arm is
        // unreachable slicing residue — verification fails, never a
        // panic (8.A1 sweep).
        Err(_) => return false,
    };
    // The CRC field itself is zeroed for the computation (RFC 3533
    // §5) — the four zero bytes are INPUT, not a gap: fold them, then
    // the rest, never allocating a patched copy.
    let mut crc = crc32(&stored[0..22]);
    crc = crc32_cont(crc, &[0u8; 4]);
    crc = crc32_cont(crc, &stored[26..]);
    crc = crc32_cont(crc, rest);
    crc == declared
}

/// Continue a CRC-32 computation from a running register (the page
/// verify seam: the CRC field bytes are skipped between the halves).
fn crc32_cont(mut crc: u32, data: &[u8]) -> u32 {
    let table = crc_table();
    for &byte in data {
        let idx = ((crc >> 24) as u8 ^ byte) as usize;
        crc = (crc << 8) ^ table[idx];
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal hand-built page: one 4-byte packet, granule 9,
    /// serial 0x11223344, seq 7, flags none.
    fn page_bytes() -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(CAPTURE);
        p.push(0); // version
        p.push(0); // flags
        p.extend_from_slice(&9i64.to_le_bytes());
        p.extend_from_slice(&0x1122_3344u32.to_le_bytes());
        p.extend_from_slice(&7u32.to_le_bytes());
        p.extend_from_slice(&0u32.to_le_bytes()); // CRC patched below
        p.push(1); // nsegs
        p.push(4); // lacing
        p.extend_from_slice(b"abcd");
        let crc = crc32(&p);
        p[22..26].copy_from_slice(&crc.to_le_bytes());
        p
    }

    #[test]
    fn crc_round_trips_and_catches_flips() {
        let page = page_bytes();
        assert!(crc_valid(&page), "self-built page verifies");
        let mut broken = page.clone();
        broken[30] ^= 0x40;
        assert!(!crc_valid(&broken), "one flipped payload bit fails");
        let mut broken = page;
        broken[0] = b'X';
        assert!(!crc_valid(&broken), "broken capture fails");
    }

    #[test]
    fn crc_matches_the_ogg_variant_check_value() {
        // The Ogg CRC (RFC 3533 §5: poly 0x04C11DB7, MSB-first, init
        // 0, no reflection, no final XOR) — independently computed
        // (bitwise reference) and cross-checked against a real
        // corpus page by the integration suite.
        assert_eq!(crc32(b"123456789"), 0x89A1_897F);
        assert_eq!(crc32(&[0]), 0x0000_0000);
    }

    #[test]
    fn header_fields_parse_little_endian() {
        let page = page_bytes();
        let hdr = parse_header(&page, 0, page.len() as u64).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(hdr.granule, 9);
        assert_eq!(hdr.serial, 0x1122_3344);
        assert_eq!(hdr.seq, 7);
        assert!(!hdr.continued && !hdr.bos && !hdr.eos);
        assert_eq!(hdr.body_len, 4);
        assert_eq!(hdr.total_len(), 32);
    }

    #[test]
    fn lacing_runs_mark_packet_boundaries() {
        let mut p = Vec::new();
        p.extend_from_slice(CAPTURE);
        p.extend_from_slice(&[0, 0]); // version, flags
        p.extend_from_slice(&(-1i64).to_le_bytes()); // granulepos
        p.extend_from_slice(&1u32.to_le_bytes()); // serial
        p.extend_from_slice(&2u32.to_le_bytes()); // seq
        p.extend_from_slice(&[0u8; 4]); // CRC (not verified here)
        p.push(3); // nsegs
        p.extend_from_slice(&[255, 100, 7]); // one continuing run, two terminals
        // stream_len is a ceiling the body must fit under (the runs
        // promise 362 bytes of body).
        let hdr = parse_header(&p, 0, 4096).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            hdr.packet_runs(),
            vec![(0, 255, false), (255, 100, true), (355, 7, true)]
        );
    }

    #[test]
    fn nonzero_version_and_zero_segments_are_typed() {
        let mut p = page_bytes();
        p[4] = 1;
        assert!(parse_header(&p, 0, 64).is_err());
        let mut q = Vec::new();
        q.extend_from_slice(CAPTURE);
        q.extend_from_slice(&[0, 2]);
        q.extend_from_slice(&0i64.to_le_bytes());
        q.extend_from_slice(&[0; 8]);
        q.push(0); // nsegs = 0
        assert!(parse_header(&q, 0, 64).is_err());
    }
}
