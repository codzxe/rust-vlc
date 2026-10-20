//! Baseline JPEG marker/segment parsing (ISO/IEC 10918-1 §B.2, cited
//! public specification — clean-room, AGENTS.md §3.1).
//!
//! Walks SOI → (APPn/COM/DQT/DHT/DRI) → SOF0 → (tables) → SOS →
//! entropy-coded data → EOI and produces the [`JpegInfo`] the scan
//! decoder consumes. Everything is bounds-checked; no panics on any
//! input (fuzz guarantee).

use super::error::{MjpegError, bad_marker, unsupported};

/// Zigzag order: natural (row-major) position of the i-th coefficient
/// in scan order (ISO/IEC 10918-1 Figure A.6) — identical table to the
/// 2.A3 encoder (`tools/xtask/src/cmd/fixtures.rs`), the shared ground
/// truth of this vertical.
pub(crate) const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, //
    17, 24, 32, 25, 18, 11, 4, 5, //
    12, 19, 26, 33, 40, 48, 41, 34, //
    27, 20, 13, 6, 7, 14, 21, 28, //
    35, 42, 49, 56, 57, 50, 43, 36, //
    29, 22, 15, 23, 30, 37, 44, 51, //
    58, 59, 52, 45, 38, 31, 39, 46, //
    53, 60, 61, 54, 47, 55, 62, 63,
];

/// One frame component as declared by SOF0 (§B.2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ComponentInfo {
    /// Component identifier (Ci; 1/2/3 in JFIF, arbitrary otherwise).
    pub id: u8,
    /// Horizontal sampling factor Hi (1..=4 by spec; 1..=2 supported).
    pub h: u8,
    /// Vertical sampling factor Vi (1..=4 by spec; 1..=2 supported).
    pub v: u8,
    /// Quantization table selector Tq (0..=3).
    pub quant_table: u8,
}

/// Everything a baseline scan decoder needs from the marker segments.
#[derive(Debug, Clone)]
pub(crate) struct JpegInfo {
    /// Frame width in pixels (X).
    pub width: u32,
    /// Frame height in pixels (Y).
    pub height: u32,
    /// Per-component sampling factors and quant-table selectors, in SOF
    /// order.
    pub components: Vec<ComponentInfo>,
    /// Quantization tables in natural order, indexed by Tq; a table is
    /// present when the flag bit is set.
    pub quant_tables: [[u16; 64]; 4],
    pub quant_present: [bool; 4],
    /// Huffman specifications per (class, destination): class 0 = DC,
    /// 1 = AC. Raw BITS/HUFFVAL, built lazily by `huffman.rs`.
    pub huff_bits: [[[u8; 17]; 4]; 2],
    pub huff_vals: [[Vec<u8>; 4]; 2],
    pub huff_present: [[bool; 4]; 2],
    /// Restart interval in MCUs (0 = no restart markers).
    pub restart_interval: u32,
    /// Byte offset just past the SOS segment: entropy data starts here.
    pub entropy_offset: usize,
    /// Whether an APP0 JFIF marker was seen (colorimetry default).
    pub jfif: bool,
    /// Scan components in SOS order, mapped to SOF indices (filled by
    /// the SOS parse; empty before it).
    pub scan: Vec<ScanComponent>,
}

/// Parsed marker-walk outcome.
pub(crate) struct Parsed {
    pub info: JpegInfo,
}

/// Read a big-endian u16 at `pos`, bounds-checked.
fn u16_at(data: &[u8], pos: usize) -> Option<u16> {
    let hi = *data.get(pos)?;
    let lo = *data.get(pos.checked_add(1)?)?;
    Some(u16::from_be_bytes([hi, lo]))
}

/// Marker codes we distinguish (§B.1): standalone (no payload) markers
/// and segment markers (2-byte big-endian length including itself).
mod marker {
    pub const SOI: u8 = 0xd8;
    pub const EOI: u8 = 0xd9;
    pub const SOS: u8 = 0xda;
    pub const DQT: u8 = 0xdb;
    pub const DHT: u8 = 0xc4;
    pub const DRI: u8 = 0xdd;
    pub const SOF0: u8 = 0xc0;
    pub const APP0: u8 = 0xe0;
    pub const APPN_FIRST: u8 = 0xe0;
    pub const APPN_LAST: u8 = 0xef;
    pub const COM: u8 = 0xfe;
    pub const TEM: u8 = 0x01;
}

/// Walk the marker segments of one complete JPEG packet.
///
/// Tolerances (documented, honest subset):
/// - SOI (`FFD8`) must open the packet; leading pad bytes are rejected
///   (the AVI writer pads chunks *outside* the frame bytes, so real
///   first-vertical packets carry no pad — 5.H3 may relax this for
///   field-based MJPEG).
/// - APPn/COM and unknown reserved segment markers with a length are
///   skipped per §B.2 conventions.
/// - A second SOF/SOI, or a missing SOS, is structural.
/// - Bytes after EOI are ignored (AVI chunk padding reality).
pub(crate) fn parse(data: &[u8]) -> Result<Parsed, MjpegError> {
    if data.len() < 2 {
        return Err(MjpegError::Truncated {
            expected: 2,
            found: data.len() as u64,
        });
    }
    if data[0] != 0xff || data[1] != marker::SOI {
        return Err(bad_marker("mjpeg.marker.no_soi", 0));
    }

    let mut info = JpegInfo {
        width: 0,
        height: 0,
        components: Vec::new(),
        quant_tables: [[1; 64]; 4],
        quant_present: [false; 4],
        huff_bits: [[[0; 17]; 4]; 2],
        huff_vals: [
            [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        ],
        huff_present: [[false; 4]; 2],
        restart_interval: 0,
        entropy_offset: 0,
        jfif: false,
        scan: Vec::new(),
    };

    let mut pos = 2usize;
    let mut seen_sof = false;
    loop {
        if pos + 2 > data.len() {
            return Err(MjpegError::Truncated {
                expected: (pos + 2) as u64,
                found: data.len() as u64,
            });
        }
        // Fill bytes (any number of 0xFF) may precede a marker
        // (§F.1.2.3); skip them.
        while pos < data.len() && data[pos] == 0xff && data.get(pos + 1) == Some(&0xff) {
            pos += 1;
        }
        if pos + 2 > data.len() {
            return Err(MjpegError::Truncated {
                expected: (pos + 2) as u64,
                found: data.len() as u64,
            });
        }
        if data[pos] != 0xff {
            return Err(bad_marker("mjpeg.marker.expected_ff", pos));
        }
        let code = data[pos + 1];
        match code {
            marker::SOI => return Err(bad_marker("mjpeg.marker.duplicate_soi", pos)),
            marker::EOI => {
                // EOI without a scan is a header-only JPEG.
                return Err(bad_marker("mjpeg.marker.eoi_before_sos", pos));
            }
            marker::SOS => {
                if !seen_sof {
                    return Err(bad_marker("mjpeg.marker.sos_before_sof", pos));
                }
                let (scan_start, scan) = parse_sos(data, pos, &info)?;
                info.entropy_offset = scan_start;
                info.scan = scan;
                // The scan decoder takes over from here; everything
                // after SOS is entropy data until a marker or the end
                // of the packet.
                return Ok(Parsed { info });
            }
            marker::DQT => {
                pos = parse_dqt(data, pos, &mut info)?;
            }
            marker::DHT => {
                pos = parse_dht(data, pos, &mut info)?;
            }
            marker::DRI => {
                pos = parse_dri(data, pos, &mut info)?;
            }
            marker::SOF0 => {
                if seen_sof {
                    return Err(bad_marker("mjpeg.marker.duplicate_sof", pos));
                }
                parse_sof0(data, pos, &mut info)?;
                seen_sof = true;
                pos = next_segment(data, pos)?;
            }
            m if (marker::APPN_FIRST..=marker::APPN_LAST).contains(&m) || m == marker::COM => {
                if m == marker::APP0 && is_jfif(data, pos) {
                    info.jfif = true;
                }
                pos = next_segment(data, pos)?;
            }
            marker::TEM => pos += 2,
            0x02..=0xbf => {
                return Err(unsupported(format!(
                    "reserved marker FF{code:02x} at byte {pos}"
                )));
            }
            0xc1 | 0xc2 | 0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf => {
                return Err(unsupported(format!(
                    "non-baseline SOF marker FF{code:02x} \
                     (extended/progressive/lossless/arithmetic JPEG)"
                )));
            }
            _ => {
                return Err(unsupported(format!(
                    "unknown marker FF{code:02x} at byte {pos}"
                )));
            }
        }
    }
}

/// Length of the segment starting at `pos` (`FF code len_hi len_lo`),
/// advanced past it.
fn next_segment(data: &[u8], pos: usize) -> Result<usize, MjpegError> {
    let Some(len) = u16_at(data, pos + 2) else {
        return Err(MjpegError::Truncated {
            expected: (pos + 4) as u64,
            found: data.len() as u64,
        });
    };
    let len = len as usize;
    if len < 2 {
        return Err(bad_marker("mjpeg.marker.bad_length", pos));
    }
    let end = pos
        .checked_add(2)
        .and_then(|p| p.checked_add(len))
        .ok_or_else(|| bad_marker("mjpeg.marker.length_overflow", pos))?;
    if end > data.len() {
        return Err(MjpegError::Truncated {
            expected: end as u64,
            found: data.len() as u64,
        });
    }
    Ok(end)
}

/// True when the APP0 at `pos` carries the JFIF identifier (§B.2.4).
fn is_jfif(data: &[u8], pos: usize) -> bool {
    let payload = pos + 4;
    data.len() >= payload + 5 && &data[payload..payload + 5] == b"JFIF\0"
}

/// SOF0 (§B.2.2): baseline frame header.
fn parse_sof0(data: &[u8], pos: usize, info: &mut JpegInfo) -> Result<(), MjpegError> {
    let seg_end = next_segment(data, pos)?;
    let payload = pos + 4; // past FF C0 len16
    if seg_end < payload + 6 {
        return Err(bad_marker("mjpeg.sof.too_short", pos));
    }
    let precision = data[payload];
    if precision != 8 {
        return Err(unsupported(format!(
            "sample precision {precision} (baseline is 8)"
        )));
    }
    let height = u16_at(data, payload + 1).unwrap_or(0);
    let width = u16_at(data, payload + 3).unwrap_or(0);
    if width == 0 || height == 0 {
        return Err(bad_marker("mjpeg.sof.zero_dimension", pos));
    }
    let count = data[payload + 5] as usize;
    if count == 0 || count > 4 {
        return Err(bad_marker("mjpeg.sof.component_count", pos));
    }
    if seg_end < payload + 6 + 3 * count {
        return Err(bad_marker("mjpeg.sof.truncated_components", pos));
    }
    let mut components = Vec::with_capacity(count);
    for i in 0..count {
        let base = payload + 6 + 3 * i;
        let id = data[base];
        let hv = data[base + 1];
        let h = hv >> 4;
        let v = hv & 0x0f;
        if !(1..=4).contains(&h) || !(1..=4).contains(&v) {
            return Err(bad_marker("mjpeg.sof.sampling_range", base));
        }
        let quant_table = data[base + 2];
        if quant_table > 3 {
            return Err(bad_marker("mjpeg.sof.quant_selector", base));
        }
        components.push(ComponentInfo {
            id,
            h,
            v,
            quant_table,
        });
    }
    // Our honest baseline subset: sampling factors 1..=2 per axis (the
    // 4:4:4 / 4:2:2 / 4:4:0 / 4:2:0 family). Factors 3/4 are legal
    // baseline but exotic; they stay the 5.H3 polish card's territory.
    for comp in &components {
        if comp.h > 2 || comp.v > 2 {
            return Err(unsupported(format!(
                "sampling factor h{}v{} on component {} \
                 (supported: 1..=2 per axis)",
                comp.h, comp.v, comp.id
            )));
        }
    }
    info.width = u32::from(width);
    info.height = u32::from(height);
    info.components = components;
    Ok(())
}

/// DQT (§B.2.4.1): one or more quantization tables (8-bit precision
/// only — baseline).
fn parse_dqt(data: &[u8], pos: usize, info: &mut JpegInfo) -> Result<usize, MjpegError> {
    let end = next_segment(data, pos)?;
    let mut p = pos + 4;
    while p < end {
        let pq_tq = data[p];
        let pq = pq_tq >> 4;
        let tq = pq_tq & 0x0f;
        if pq != 0 {
            return Err(unsupported("16-bit quantization precision (Pq=1)"));
        }
        if tq > 3 {
            return Err(bad_marker("mjpeg.dqt.table_id", p));
        }
        if p + 65 > end {
            return Err(bad_marker("mjpeg.dqt.too_short", p));
        }
        let mut table = [0u16; 64];
        for (i, slot) in table.iter_mut().enumerate() {
            // Zigzag input order → natural order (same walk as the
            // encoder's DQT emission). Zero is out of spec (§B.2.4.1:
            // 8-bit values are 1..255).
            let raw = data[p + 1 + ZIGZAG[i]];
            if raw == 0 {
                return Err(bad_marker("mjpeg.dqt.zero_value", p + 1 + ZIGZAG[i]));
            }
            *slot = u16::from(raw);
        }
        info.quant_tables[tq as usize] = table;
        info.quant_present[tq as usize] = true;
        p += 65;
    }
    if p != end {
        return Err(bad_marker("mjpeg.dqt.trailing_bytes", p));
    }
    Ok(end)
}

/// DHT (§B.2.4.2): one or more Huffman table specifications.
fn parse_dht(data: &[u8], pos: usize, info: &mut JpegInfo) -> Result<usize, MjpegError> {
    let end = next_segment(data, pos)?;
    let mut p = pos + 4;
    while p < end {
        let tc_td = data[p];
        let tc = tc_td >> 4;
        let td = tc_td & 0x0f;
        if tc > 1 {
            return Err(bad_marker("mjpeg.dht.class", p));
        }
        if td > 3 {
            return Err(bad_marker("mjpeg.dht.table_id", p));
        }
        if p + 17 > end {
            return Err(bad_marker("mjpeg.dht.too_short", p));
        }
        let mut bits = [0u8; 17];
        let mut total = 0usize;
        for (i, slot) in bits.iter_mut().enumerate().skip(1) {
            *slot = data[p + i];
            total = total.saturating_add(*slot as usize);
        }
        if total > 256 {
            return Err(bad_marker("mjpeg.dht.symbol_overflow", p));
        }
        // Canonical code-space check (Annex C.2): an over-subscribed
        // BITS histogram has no valid code assignment — reject it as
        // structural damage now rather than at first use.
        let mut code = 0usize;
        for (length, &count_u8) in bits.iter().enumerate().skip(1) {
            if count_u8 > 0 {
                let count = count_u8 as usize;
                let oversubscribed = code
                    .checked_add(count - 1)
                    .is_none_or(|last| last >= 1usize << length);
                if oversubscribed {
                    return Err(bad_marker("mjpeg.dht.oversubscribed", p));
                }
                code += count;
            }
            code <<= 1;
        }
        if p + 17 + total > end {
            return Err(bad_marker("mjpeg.dht.symbol_truncation", p));
        }
        let vals = data[p + 17..p + 17 + total].to_vec();
        info.huff_bits[tc as usize][td as usize] = bits;
        info.huff_vals[tc as usize][td as usize] = vals;
        info.huff_present[tc as usize][td as usize] = true;
        p += 17 + total;
    }
    if p != end {
        return Err(bad_marker("mjpeg.dht.trailing_bytes", p));
    }
    Ok(end)
}

/// DRI (§B.2.4.4): restart interval in MCUs.
fn parse_dri(data: &[u8], pos: usize, info: &mut JpegInfo) -> Result<usize, MjpegError> {
    let end = next_segment(data, pos)?;
    if pos + 6 != end {
        return Err(bad_marker("mjpeg.dri.length", pos));
    }
    info.restart_interval = u32::from(u16_at(data, pos + 4).unwrap_or(0));
    Ok(end)
}

/// One scan component: which frame component it is (by SOF index) and
/// its Huffman table selectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScanComponent {
    /// Index into `JpegInfo::components`.
    pub sof_index: usize,
    /// DC table selector Td.
    pub td: u8,
    /// AC table selector Ta.
    pub ta: u8,
}

/// SOS (§B.2.3): scan header. Baseline subset: exactly one interleaved
/// scan covering every frame component, full spectral range,
/// successive-approximation bits zero.
fn parse_sos(
    data: &[u8],
    pos: usize,
    info: &JpegInfo,
) -> Result<(usize, Vec<ScanComponent>), MjpegError> {
    let end = next_segment(data, pos)?;
    let payload = pos + 4;
    if end < payload + 1 {
        return Err(bad_marker("mjpeg.sos.too_short", pos));
    }
    let ns = data[payload] as usize;
    if ns != info.components.len() {
        return Err(unsupported(format!(
            "non-interleaved scan (Ns={ns} for {} frame components) — \
             only single interleaved scans are supported",
            info.components.len()
        )));
    }
    if end < payload + 1 + 2 * ns + 3 {
        return Err(bad_marker("mjpeg.sos.truncated", pos));
    }
    let mut scan = Vec::with_capacity(ns);
    for i in 0..ns {
        let cs = data[payload + 1 + 2 * i];
        let tables = data[payload + 2 + 2 * i];
        let sof_index = info
            .components
            .iter()
            .position(|c| c.id == cs)
            .ok_or_else(|| bad_marker("mjpeg.sos.unknown_component", payload + 1 + 2 * i))?;
        // A component may appear only once in a scan (B.2.3).
        if scan
            .iter()
            .any(|sc: &ScanComponent| sc.sof_index == sof_index)
        {
            return Err(bad_marker(
                "mjpeg.sos.duplicate_component",
                payload + 1 + 2 * i,
            ));
        }
        let comp = info.components[sof_index];
        let td = tables >> 4;
        let ta = tables & 0x0f;
        if td > 3 || ta > 3 {
            return Err(bad_marker("mjpeg.sos.table_selector", payload + 2 + 2 * i));
        }
        if !info.huff_present[0][td as usize] || !info.huff_present[1][ta as usize] {
            return Err(bad_marker(
                "mjpeg.sos.missing_huffman_table",
                payload + 2 + 2 * i,
            ));
        }
        if !info.quant_present[comp.quant_table as usize] {
            return Err(bad_marker(
                "mjpeg.sos.missing_quant_table",
                payload + 2 + 2 * i,
            ));
        }
        scan.push(ScanComponent { sof_index, td, ta });
    }
    let spectral_start = data[payload + 1 + 2 * ns];
    let spectral_end = data[payload + 2 + 2 * ns];
    let ah_al = data[payload + 3 + 2 * ns];
    if spectral_start != 0 || spectral_end != 63 || ah_al != 0 {
        return Err(unsupported(format!(
            "partial/spectral-selection scan \
             (Ss={spectral_start}, Se={spectral_end}, Ah/Al={ah_al})"
        )));
    }
    Ok((end, scan))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal well-formed header set for the parse tests (content is
    /// filled per test; the shape mirrors the 2.A3 encoder).
    fn soi() -> Vec<u8> {
        vec![0xff, 0xd8]
    }

    fn seg(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![0xff, marker];
        out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn dqt() -> Vec<u8> {
        let mut payload = vec![0u8]; // Pq=0, Tq=0
        payload.extend(std::iter::repeat_n(16u8, 64));
        seg(0xdb, &payload)
    }

    fn dht_dc() -> Vec<u8> {
        // Single DC symbol (category 0): BITS[1]=1, HUFFVAL=[0].
        let mut payload = vec![0x00]; // Tc=0 DC, Td=0
        payload.extend_from_slice(&[0u8; 16]);
        payload[1] = 1; // one code of length 1
        payload.push(0x00); // HUFFVAL
        seg(0xc4, &payload)
    }

    fn dht_ac() -> Vec<u8> {
        // Single AC symbol (EOB): BITS[1]=1, HUFFVAL=[0].
        let mut payload = vec![0x10]; // Tc=1 AC, Ta=0
        payload.extend_from_slice(&[0u8; 16]);
        payload[1] = 1; // one code of length 1
        payload.push(0x00); // HUFFVAL
        seg(0xc4, &payload)
    }

    fn dht() -> Vec<u8> {
        // Both tables a grayscale SOS needs (Td=0, Ta=0).
        let mut out = dht_dc();
        out.extend(dht_ac());
        out
    }

    fn sof0(w: u16, h: u16) -> Vec<u8> {
        let mut payload = vec![8];
        payload.extend_from_slice(&h.to_be_bytes());
        payload.extend_from_slice(&w.to_be_bytes());
        payload.push(1); // one component
        payload.extend_from_slice(&[1, 0x11, 0]); // Y only, h1v1, Tq0
        seg(0xc0, &payload)
    }

    fn sos() -> Vec<u8> {
        let payload = [1u8, 1, 0x00, 0, 63, 0];
        seg(0xda, &payload)
    }

    #[test]
    fn parses_the_minimal_grayscale_stream() {
        let mut jpeg = soi();
        jpeg.extend(dqt());
        jpeg.extend(dht());
        jpeg.extend(sof0(8, 8));
        jpeg.extend(sos());
        jpeg.extend_from_slice(&[0x00, 0xff, 0xd9]); // 1 pad bit + EOI
        let parsed = match parse(&jpeg) {
            Ok(p) => p,
            Err(e) => unreachable!("parse failed: {e:?}"),
        };
        assert_eq!(parsed.info.width, 8);
        assert_eq!(parsed.info.height, 8);
        assert_eq!(parsed.info.components.len(), 1);
        assert_eq!(parsed.info.components[0].h, 1);
        assert_eq!(parsed.info.components[0].v, 1);
        assert!(parsed.info.quant_present[0]);
        assert!(parsed.info.huff_present[0][0]);
        assert!(!parsed.info.jfif);
        assert_eq!(parsed.info.entropy_offset, jpeg.len() - 3);
        assert_eq!(parsed.info.scan.len(), 1);
        assert_eq!(parsed.info.scan[0].sof_index, 0);
        assert_eq!(parsed.info.scan[0].td, 0);
        assert_eq!(parsed.info.scan[0].ta, 0);
    }

    #[test]
    fn jfif_app0_is_detected() {
        let mut jpeg = soi();
        let mut app0 = b"JFIF\0".to_vec();
        app0.extend_from_slice(&[1, 1, 0, 0, 0, 0, 0, 0, 0]);
        jpeg.extend(seg(0xe0, &app0));
        jpeg.extend(dqt());
        jpeg.extend(dht());
        jpeg.extend(sof0(8, 8));
        jpeg.extend(sos());
        jpeg.extend_from_slice(&[0xff, 0xd9]);
        let parsed = match parse(&jpeg) {
            Ok(p) => p,
            Err(e) => unreachable!("parse failed: {e:?}"),
        };
        assert!(parsed.info.jfif);
    }

    #[test]
    fn missing_soi_is_structural() {
        let e = parse(&[0x00, 0x00]).err().unwrap_or_else(|| unreachable!());
        assert!(matches!(e, MjpegError::BadMarker { code, .. } if code == "mjpeg.marker.no_soi"));
    }

    #[test]
    fn short_input_is_truncation() {
        let e = parse(&[0xff]).err().unwrap_or_else(|| unreachable!());
        assert!(matches!(
            e,
            MjpegError::Truncated {
                expected: 2,
                found: 1
            }
        ));
    }

    #[test]
    fn zero_dimensions_are_structural() {
        let mut jpeg = soi();
        jpeg.extend(dqt());
        jpeg.extend(dht());
        jpeg.extend(sof0(0, 8));
        let e = parse(&jpeg).err().unwrap_or_else(|| unreachable!());
        assert!(
            matches!(e, MjpegError::BadMarker { code, .. } if code == "mjpeg.sof.zero_dimension")
        );
    }

    #[test]
    fn progressive_sof2_is_unsupported_not_corrupt() {
        let mut jpeg = soi();
        jpeg.extend(seg(0xc2, &[8, 0, 8, 0, 8, 1, 1, 0x11, 0]));
        let e = parse(&jpeg).err().unwrap_or_else(|| unreachable!());
        match e {
            MjpegError::UnsupportedFeature(detail) => {
                assert!(detail.contains("FFc2"), "{detail}");
            }
            other => unreachable!("expected UnsupportedFeature, got {other:?}"),
        }
    }

    #[test]
    fn segment_overrun_is_truncation() {
        let mut jpeg = soi();
        // Declare a DQT longer than the remaining bytes.
        jpeg.extend_from_slice(&[0xff, 0xdb, 0xff, 0xff]);
        let e = parse(&jpeg).err().unwrap_or_else(|| unreachable!());
        assert!(matches!(e, MjpegError::Truncated { .. }));
    }

    #[test]
    fn sampling_factor_three_is_unsupported() {
        let mut payload = vec![8];
        payload.extend_from_slice(&[0, 8, 0, 8, 1]);
        payload.extend_from_slice(&[1, 0x31, 0]); // h3v1 — out of subset
        let mut jpeg = soi();
        jpeg.extend(seg(0xc0, &payload));
        let e = parse(&jpeg).err().unwrap_or_else(|| unreachable!());
        assert!(matches!(e, MjpegError::UnsupportedFeature(_)));
    }

    #[test]
    fn sos_before_sof_is_structural() {
        let mut jpeg = soi();
        jpeg.extend(sos());
        let e = parse(&jpeg).err().unwrap_or_else(|| unreachable!());
        assert!(
            matches!(e, MjpegError::BadMarker { code, .. } if code == "mjpeg.marker.sos_before_sof")
        );
    }

    #[test]
    fn non_interleaved_scan_is_unsupported() {
        // Two-component SOF, one-component scan.
        let mut sof_payload = vec![8];
        sof_payload.extend_from_slice(&[0, 8, 0, 8, 2]);
        sof_payload.extend_from_slice(&[1, 0x11, 0]);
        sof_payload.extend_from_slice(&[2, 0x11, 0]);
        let mut jpeg = soi();
        jpeg.extend(dqt());
        jpeg.extend(dht());
        jpeg.extend(seg(0xc0, &sof_payload));
        // Ns=1 (only component 1).
        jpeg.extend(seg(0xda, &[1, 1, 0x00, 0, 63, 0]));
        let e = parse(&jpeg).err().unwrap_or_else(|| unreachable!());
        match e {
            MjpegError::UnsupportedFeature(detail) => {
                assert!(detail.contains("non-interleaved"), "{detail}");
            }
            other => unreachable!("expected UnsupportedFeature, got {other:?}"),
        }
    }

    #[test]
    fn oversubscribed_huffman_counts_are_structural() {
        // BITS summing past 256: 200 codes of length 1 is impossible.
        let mut payload = vec![0x00];
        payload.extend_from_slice(&[200u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        payload.extend(std::iter::repeat_n(0u8, 200));
        let mut jpeg = soi();
        jpeg.extend(seg(0xc4, &payload));
        let e = parse(&jpeg).err().unwrap_or_else(|| unreachable!());
        assert!(
            matches!(e, MjpegError::BadMarker { code, .. } if code == "mjpeg.dht.oversubscribed")
        );
    }

    #[test]
    fn eoi_before_sos_is_structural() {
        let mut jpeg = soi();
        jpeg.extend_from_slice(&[0xff, 0xd9]);
        let e = parse(&jpeg).err().unwrap_or_else(|| unreachable!());
        assert!(
            matches!(e, MjpegError::BadMarker { code, .. } if code == "mjpeg.marker.eoi_before_sos")
        );
    }

    #[test]
    fn two_byte_quant_table_ids_reject_the_upper_half() {
        // Tq = 4 is out of range.
        let mut payload = vec![0x04];
        payload.extend(std::iter::repeat_n(16u8, 64));
        let mut jpeg = soi();
        jpeg.extend(seg(0xdb, &payload));
        let e = parse(&jpeg).err().unwrap_or_else(|| unreachable!());
        assert!(matches!(e, MjpegError::BadMarker { code, .. } if code == "mjpeg.dqt.table_id"));
    }
}
