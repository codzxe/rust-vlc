//! Baseline JPEG marker/segment parsing (ISO/IEC 10918-1 §B.2, cited
//! public specification — clean-room, AGENTS.md §3.1).
//!
//! Walks SOI → (APPn/COM/DQT/DHT/DRI) → SOF0 → (tables) → SOS →
//! entropy-coded data → EOI and produces the [`JpegInfo`] the scan
//! decoder consumes. Everything is bounds-checked; no panics on any
//! input (fuzz guarantee).

use super::FieldPolarity;
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
    /// AVI1 APP0 field polarity (card 5.H3): `None` for progressive
    /// frames (no AVI1 marker — the JFIF world), `Some` for the
    /// "AVI 1.1" field-based extension (OpenDML AVI M-JPEG).
    pub field_polarity: Option<FieldPolarity>,
    /// Byte offset just past the SOS segment: entropy data starts here.
    pub entropy_offset: usize,
    /// Whether an APP0 JFIF marker was seen (colorimetry default).
    pub jfif: bool,
    /// Scan components in SOS order, mapped to SOF indices (filled by
    /// the SOS parse; empty before it).
    pub scan: Vec<ScanComponent>,
}

/// Parsed marker-walk outcome (the stateless [`parse`] — now a
/// test/one-shot convenience; the decode context walks into its own
/// persistent scratch via [`parse_into`]).
#[cfg(test)]
pub(crate) struct Parsed {
    pub info: JpegInfo,
}

impl JpegInfo {
    /// The pre-parse state: capacity-free fields only (an `empty()`
    /// allocates nothing — `Vec::new()` never hits the heap), so the
    /// decode context can park one in place of the live scratch while
    /// a frame walks (decoder.rs, D-1).
    pub(crate) fn empty() -> JpegInfo {
        JpegInfo {
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
            field_polarity: None,
            entropy_offset: 0,
            jfif: false,
            scan: Vec::new(),
        }
    }

    /// Reset to the pre-parse state **keeping every Vec capacity**
    /// (D-1: a caller-held scratch — the decode context's persistent
    /// header storage — then rewalks in steady state without a single
    /// allocation). Resetting to the same shape `empty()` produces is
    /// what makes a failed walk harmless: the next `parse_into` starts
    /// from the same clean state a fresh `JpegInfo` would.
    pub(crate) fn reset(&mut self) {
        self.width = 0;
        self.height = 0;
        self.components.clear();
        self.quant_tables = [[1; 64]; 4];
        self.quant_present = [false; 4];
        self.huff_bits = [[[0; 17]; 4]; 2];
        for class in &mut self.huff_vals {
            for vals in class {
                vals.clear();
            }
        }
        self.huff_present = [[false; 4]; 2];
        self.restart_interval = 0;
        self.field_polarity = None;
        self.entropy_offset = 0;
        self.jfif = false;
        self.scan.clear();
    }
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

/// Walk the marker segments of one complete JPEG packet, stateless
/// (fresh scratch — the one-shot/test convenience; the decode context
/// uses [`parse_into`] on its persistent scratch instead).
#[cfg(test)]
pub(crate) fn parse(data: &[u8]) -> Result<Parsed, MjpegError> {
    let mut info = JpegInfo::empty();
    parse_into(&mut info, data)?;
    Ok(Parsed { info })
}

/// Walk the marker segments of one complete JPEG packet into a
/// caller-held scratch `JpegInfo` (the decode context's persistent
/// header storage — D-1: in steady state this walk allocates
/// **nothing**; every Vec is cleared and refilled in place,
/// capacities retained across frames. The stateless [`parse`] above
/// simply starts from an `empty()` scratch).
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
///
/// On error the scratch may hold a partially walked state — the next
/// call resets it first, so no damage can leak into a later frame.
pub(crate) fn parse_into(info: &mut JpegInfo, data: &[u8]) -> Result<(), MjpegError> {
    if data.len() < 2 {
        return Err(MjpegError::Truncated {
            expected: 2,
            found: data.len() as u64,
        });
    }
    if data[0] != 0xff || data[1] != marker::SOI {
        return Err(bad_marker("mjpeg.marker.no_soi", 0));
    }
    info.reset();

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
                // Fills `info.scan` in place (D-1: no per-frame Vec).
                info.entropy_offset = parse_sos(data, pos, info)?;
                // The scan decoder takes over from here; everything
                // after SOS is entropy data until a marker or the end
                // of the packet.
                return Ok(());
            }
            marker::DQT => {
                pos = parse_dqt(data, pos, &mut *info)?;
            }
            marker::DHT => {
                pos = parse_dht(data, pos, &mut *info)?;
            }
            marker::DRI => {
                pos = parse_dri(data, pos, &mut *info)?;
            }
            marker::SOF0 => {
                if seen_sof {
                    return Err(bad_marker("mjpeg.marker.duplicate_sof", pos));
                }
                parse_sof0(data, pos, &mut *info)?;
                seen_sof = true;
                pos = next_segment(data, pos)?;
            }
            m if (marker::APPN_FIRST..=marker::APPN_LAST).contains(&m) || m == marker::COM => {
                if m == marker::APP0 {
                    if is_jfif(data, pos) {
                        info.jfif = true;
                    } else if let Some(polarity) = parse_avi1(data, pos) {
                        info.field_polarity = Some(polarity?);
                    }
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

/// Parse the AVI1 APP0 of a field-based MJPEG frame (card 5.H3; the
/// "AVI 1.1" extension — OpenDML AVI File Format Extensions, Motion
/// JPEG DIB): payload `AVI1\0` + one polarity byte.
///
/// Returns `None` when the segment is not an AVI1 APP0 (some other
/// APP0 — JFIF checked first by the caller, anything else skipped per
/// §B.2), `Some(Ok(polarity))` when it is, `Some(Err(..))` when it is
/// an AVI1 segment with structural damage (cut short, or a polarity
/// byte outside the spec's 0..=2 vocabulary).
fn parse_avi1(data: &[u8], pos: usize) -> Option<Result<FieldPolarity, MjpegError>> {
    let payload = pos + 4;
    if data.len() < payload + 5 || &data[payload..payload + 5] != b"AVI1\0" {
        return None;
    }
    // The polarity byte must lie inside the declared segment: a
    // segment that ends at `AVI1\0` is damaged (never read the next
    // marker's bytes as polarity). A segment-length failure here
    // yields `None`; the caller's own `next_segment` then surfaces
    // the typed error.
    let end = next_segment(data, pos).ok()?;
    if payload + 6 > end {
        return Some(Err(bad_marker("mjpeg.app0.avi1.no_polarity", pos)));
    }
    Some(match data[payload + 5] {
        0 => Ok(FieldPolarity::FirstField),
        1 => Ok(FieldPolarity::SecondField),
        2 => Ok(FieldPolarity::Interleaved),
        _ => Err(bad_marker("mjpeg.app0.avi1.polarity", pos + 4)),
    })
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
    // Refill the persistent component list in place (D-1: no fresh
    // Vec — cleared by the pre-walk reset, capacity retained).
    info.components.reserve(count);
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
        info.components.push(ComponentInfo {
            id,
            h,
            v,
            quant_table,
        });
    }
    // Our honest baseline subset: sampling factors 1..=2 per axis (the
    // 4:4:4 / 4:2:2 / 4:4:0 / 4:2:0 family). Factors 3/4 are legal
    // baseline but exotic; they stay the 5.H3 polish card's territory.
    for comp in &info.components {
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
        // Zigzag input order → natural order (§B.2.4.1): payload byte
        // k is the quantizer of the k-th coefficient *in scan order*,
        // which is natural position ZIGZAG[k] — the destination, not
        // the source, of the walk. (The entropy path is the mirror:
        // there, scan position k carries the coefficient FROM natural
        // ZIGZAG[k]. Regression-tested with distinct per-position
        // values — flat tables make a permutation invisible, which is
        // exactly how this bug hid through the salvage's uniform-table
        // unit tests and the fixture luma plane.)
        for k in 0..64 {
            // Zero is out of spec (§B.2.4.1: 8-bit values are 1..255).
            let raw = data[p + 1 + k];
            if raw == 0 {
                return Err(bad_marker("mjpeg.dqt.zero_value", p + 1 + k));
            }
            table[ZIGZAG[k]] = u16::from(raw);
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
        // Refill the persistent HUFFVAL storage in place (D-1: no
        // per-frame `to_vec` — cleared by the pre-walk reset, capacity
        // retained; `total ≤ 256` above keeps the retained capacity
        // bounded).
        let vals = &mut info.huff_vals[tc as usize][td as usize];
        vals.clear();
        vals.extend_from_slice(&data[p + 17..p + 17 + total]);
        info.huff_bits[tc as usize][td as usize] = bits;
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
/// successive-approximation bits zero. Fills `info.scan` in place
/// (D-1: the persistent list is cleared and refilled — no per-frame
/// Vec) and returns the entropy-data start offset.
fn parse_sos(data: &[u8], pos: usize, info: &mut JpegInfo) -> Result<usize, MjpegError> {
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
    info.scan.clear();
    info.scan.reserve(ns);
    for i in 0..ns {
        let cs = data[payload + 1 + 2 * i];
        let tables = data[payload + 2 + 2 * i];
        let sof_index = info
            .components
            .iter()
            .position(|c| c.id == cs)
            .ok_or_else(|| bad_marker("mjpeg.sos.unknown_component", payload + 1 + 2 * i))?;
        // A component may appear only once in a scan (B.2.3).
        if info
            .scan
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
        info.scan.push(ScanComponent { sof_index, td, ta });
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
    Ok(end)
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
    fn parse_into_reuses_the_scratch_without_leaking_across_walks() {
        // Walk A: a 3-component stream whose DC table carries two
        // symbols; a failed walk in between; walk B (same scratch):
        // grayscale with a single-symbol DC table. B must see none of
        // A's components, scan entries, HUFFVAL bytes or table flags —
        // the reset keeps *capacities*, never contents (W2-06 made
        // the walk reuse persistent storage, so the reset is a
        // correctness property, not just hygiene) — and B's fields
        // must equal a fresh stateless parse byte-for-byte.
        let mut a = soi();
        a.extend(dqt());
        // DC table 0 with two symbols: BITS[1]=2, HUFFVAL=[0,5].
        let mut dht_dc2 = vec![0x00u8];
        dht_dc2.extend_from_slice(&[2u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        dht_dc2.extend_from_slice(&[0x00, 0x05]);
        a.extend(seg(0xc4, &dht_dc2));
        a.extend(dht_ac());
        // SOF0 with three components (4:2:0).
        let mut sof = vec![8u8];
        sof.extend_from_slice(&[0, 8, 0, 8, 3, 1, 0x22, 0, 2, 0x11, 0, 3, 0x11, 0]);
        a.extend(seg(0xc0, &sof));
        a.extend(seg(0xda, &[3, 1, 0x00, 2, 0x00, 3, 0x00, 0, 63, 0]));
        a.extend_from_slice(&[0xff, 0xd9]);

        let mut b = soi();
        b.extend(dqt());
        b.extend(dht());
        b.extend(sof0(8, 8));
        b.extend(sos());
        b.extend_from_slice(&[0xff, 0xd9]);

        let mut scratch = JpegInfo::empty();
        parse_into(&mut scratch, &a).unwrap_or_else(|e| unreachable!("walk A: {e:?}"));
        assert_eq!(scratch.components.len(), 3);
        assert_eq!(scratch.huff_vals[0][0], vec![0x00, 0x05]);
        assert_eq!(scratch.scan.len(), 3);

        // A failed walk: cut inside the SOS segment, after SOF/DQT/DHT
        // have already been applied — the scratch holds partial state.
        let mut broken = a.clone();
        broken.truncate(broken.len() - 10);
        assert!(parse_into(&mut scratch, &broken).is_err(), "cut walk fails");

        parse_into(&mut scratch, &b).unwrap_or_else(|e| unreachable!("walk B: {e:?}"));
        assert_eq!(scratch.components.len(), 1, "no leftover components");
        assert_eq!(scratch.scan.len(), 1, "no leftover scan entries");
        assert_eq!(scratch.huff_vals[0][0], vec![0x00], "HUFFVAL refilled");
        assert!(!scratch.huff_present[0][1], "no leftover table flags");
        // The reuse this whole shape exists for: A's capacities are
        // still behind B's smaller walk (nothing regrew).
        assert!(scratch.components.capacity() >= 3);
        assert!(scratch.huff_vals[0][0].capacity() >= 2);

        // And B's parse equals a fresh stateless parse on every field.
        let fresh = match parse(&b) {
            Ok(p) => p,
            Err(e) => unreachable!("fresh walk B: {e:?}"),
        };
        assert_eq!(scratch.components, fresh.info.components);
        assert_eq!(scratch.scan, fresh.info.scan);
        assert_eq!(scratch.huff_vals, fresh.info.huff_vals);
        assert_eq!(scratch.huff_bits, fresh.info.huff_bits);
        assert_eq!(scratch.huff_present, fresh.info.huff_present);
        assert_eq!(scratch.quant_tables, fresh.info.quant_tables);
        assert_eq!(scratch.quant_present, fresh.info.quant_present);
        assert_eq!(scratch.entropy_offset, fresh.info.entropy_offset);
        assert_eq!(scratch.restart_interval, fresh.info.restart_interval);
        assert_eq!(scratch.jfif, fresh.info.jfif);
        assert_eq!(
            (scratch.width, scratch.height),
            (fresh.info.width, fresh.info.height)
        );
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

    #[test]
    fn dqt_dezigzag_maps_scan_order_to_natural_positions() {
        // Regression (found via the 2.B4 ffmpeg-oracle cross-check):
        // the salvage's parse read payload[ZIGZAG[i]] into natural
        // slot i — the permutation's wrong direction. With distinct
        // per-position values the bug is visible: emit natural table
        // value (position + 1) in zigzag payload order, then require
        // the parsed natural table to match exactly. (The salvage's
        // uniform-value tables made any permutation invisible — the
        // fixture luma plane decoded byte-exact by luck of the
        // content's zero high-frequency coefficients; chroma did
        // not, which is what exposed this.)
        let natural: [u16; 64] = std::array::from_fn(|i| (i + 1) as u16);
        let mut payload = vec![0u8]; // Pq=0, Tq=0
        for &natural_index in ZIGZAG.iter() {
            payload.push(natural[natural_index] as u8);
        }
        let mut jpeg = soi();
        jpeg.extend(seg(0xdb, &payload));
        // Complete the minimal marker set so the walk reaches SOS.
        jpeg.extend(dht());
        jpeg.extend(sof0(8, 8));
        jpeg.extend(sos());
        jpeg.extend_from_slice(&[0x00, 0xff, 0xd9]);
        let parsed = match parse(&jpeg) {
            Ok(p) => p,
            Err(e) => unreachable!("parse failed: {e:?}"),
        };
        assert_eq!(parsed.info.quant_tables[0], natural);
    }

    #[test]
    fn avi1_polarity_parses_for_all_three_values() {
        // Card 5.H3: the "AVI 1.1" field-based MJPEG APP0. Polarity 0
        // = first field, 1 = second field, 2 = both fields
        // interleaved (OpenDML AVI M-JPEG).
        for (byte, want) in [
            (0u8, FieldPolarity::FirstField),
            (1, FieldPolarity::SecondField),
            (2, FieldPolarity::Interleaved),
        ] {
            let mut jpeg = soi();
            let mut app0 = b"AVI1\0".to_vec();
            app0.push(byte);
            jpeg.extend(seg(0xe0, &app0));
            jpeg.extend(dqt());
            jpeg.extend(dht());
            jpeg.extend(sof0(8, 4));
            jpeg.extend(sos());
            jpeg.extend_from_slice(&[0x00, 0xff, 0xd9]);
            let parsed = match parse(&jpeg) {
                Ok(p) => p,
                Err(e) => unreachable!("parse failed: {e:?}"),
            };
            assert_eq!(parsed.info.field_polarity, Some(want), "polarity {byte}");
            assert!(!parsed.info.jfif, "AVI1 is not JFIF");
        }
    }

    #[test]
    fn avi1_polarity_out_of_vocabulary_is_structural() {
        let mut jpeg = soi();
        let mut app0 = b"AVI1\0".to_vec();
        app0.push(3);
        jpeg.extend(seg(0xe0, &app0));
        jpeg.extend(dqt());
        jpeg.extend(dht());
        jpeg.extend(sof0(8, 8));
        let e = parse(&jpeg).err().unwrap_or_else(|| unreachable!());
        assert!(
            matches!(e, MjpegError::BadMarker { code, .. } if code == "mjpeg.app0.avi1.polarity")
        );
    }

    #[test]
    fn avi1_segment_without_the_polarity_byte_is_structural() {
        // The declared segment ends at `AVI1\0`: no polarity byte
        // inside the segment (the byte after it in the buffer belongs
        // to the next marker — it must not be read as polarity).
        let mut jpeg = soi();
        jpeg.extend(seg(0xe0, b"AVI1\0"));
        jpeg.extend(dqt());
        let e = parse(&jpeg).err().unwrap_or_else(|| unreachable!());
        assert!(
            matches!(e, MjpegError::BadMarker { code, .. } if code == "mjpeg.app0.avi1.no_polarity")
        );
    }

    #[test]
    fn avi1_field_polarity_resets_between_walks() {
        // The persistent-scratch contract: a walk that saw AVI1 must
        // not leak the polarity into the next progressive frame's
        // walk (the reset clears it like every other field).
        let mut with_marker = soi();
        let mut app0 = b"AVI1\0".to_vec();
        app0.push(2);
        with_marker.extend(seg(0xe0, &app0));
        with_marker.extend(dqt());
        with_marker.extend(dht());
        with_marker.extend(sof0(8, 8));
        with_marker.extend(sos());
        with_marker.extend_from_slice(&[0x00, 0xff, 0xd9]);
        let mut plain = soi();
        plain.extend(dqt());
        plain.extend(dht());
        plain.extend(sof0(8, 8));
        plain.extend(sos());
        plain.extend_from_slice(&[0x00, 0xff, 0xd9]);
        let mut scratch = JpegInfo::empty();
        parse_into(&mut scratch, &with_marker).unwrap_or_else(|e| unreachable!("{e:?}"));
        assert_eq!(scratch.field_polarity, Some(FieldPolarity::Interleaved));
        parse_into(&mut scratch, &plain).unwrap_or_else(|e| unreachable!("{e:?}"));
        assert_eq!(scratch.field_polarity, None, "no polarity leak");
    }
}
