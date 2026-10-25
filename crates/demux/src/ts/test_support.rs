//! In-crate byte builders for the TS tests (card 5.I1a).
//!
//! Public like [`crate::avi::test_support`] on purpose: rustdoc
//! examples and the integration/fuzz suites need deterministic
//! transport-stream shapes (including deliberately malformed ones).
//! Test-support surface, not player surface — nothing in the demuxer
//! uses it.

use std::sync::Arc;

use plugin_api::SourceStream;

use crate::ts::psi::{MAX_SECTION_BYTES, crc32_mpeg};

/// A boxed in-memory [`SourceStream`] over `bytes` (the demux crate's
/// shared test seam).
///
/// # Errors
/// Propagates the (never-failing) URI parse.
pub fn mem_stream(
    bytes: impl Into<Arc<[u8]>>,
) -> Result<Box<dyn SourceStream>, media_types::EngineError> {
    crate::avi::test_support::mem_stream(bytes)
}

/// Build one PSI section: `table_id`, the 16-bit header (syntax
/// indicator set, `section_length` computed), `body` (everything
/// after the 8 fixed header bytes), a correct CRC-32 appended. The
/// fixed bytes are `tsid`, `version|cni` (current by default) and
/// `section_number/last_section_number` — the shape PAT/PMT/SDT
/// share.
#[must_use]
pub fn psi_section(table_id: u8, tsid: u16, version: u8, body: &[u8]) -> Vec<u8> {
    psi_section_tuned(table_id, tsid, version, body, true)
}

/// [`psi_section`] with the current/next bit tunable (`false` forges
/// a "future" section).
#[must_use]
pub fn psi_section_tuned(
    table_id: u8,
    tsid: u16,
    version: u8,
    body: &[u8],
    current: bool,
) -> Vec<u8> {
    psi_section_full(table_id, tsid, version, body, current, true)
}

/// [`psi_section`] with the syntax indicator tunable too (`false`
/// forges a short-form section: no fixed header, no CRC — the body
/// starts right after the 3-byte section header).
#[must_use]
pub fn psi_section_full(
    table_id: u8,
    tsid: u16,
    version: u8,
    body: &[u8],
    current: bool,
    syntax: bool,
) -> Vec<u8> {
    if !syntax {
        let sl = body.len().min(MAX_SECTION_BYTES - 3);
        let mut out = Vec::with_capacity(3 + sl);
        out.push(table_id);
        out.push(0x70 | (sl >> 8) as u8);
        out.push((sl & 0xFF) as u8);
        out.extend_from_slice(&body[..sl]);
        return out;
    }
    // Long form: section_length counts everything after its own
    // field — 5 fixed bytes + body + the 4-byte CRC.
    let section_length = 9 + body.len();
    let mut out = Vec::with_capacity(3 + section_length);
    out.push(table_id);
    out.push(0xB0 | (section_length >> 8) as u8);
    out.push((section_length & 0xFF) as u8);
    out.extend_from_slice(&tsid.to_be_bytes());
    out.push(0xC0 | ((version << 1) & 0x3E) | u8::from(current));
    out.push(0x00); // section_number
    out.push(0x00); // last_section_number
    out.extend_from_slice(body);
    let crc = crc32_mpeg(&out);
    out.extend_from_slice(&crc.to_be_bytes());
    out
}

/// A PAT body: one `(program_number, pmt_pid)` entry per pair.
#[must_use]
pub fn pat_body(entries: &[(u16, u16)]) -> Vec<u8> {
    let mut body = Vec::new();
    for &(number, pid) in entries {
        body.extend_from_slice(&number.to_be_bytes());
        body.extend_from_slice(&((pid & 0x1FFF) | 0xE000).to_be_bytes());
    }
    body
}

/// A PMT body: `pcr_pid`, optional program descriptors, then the ES
/// entries `(stream_type, pid, es_info)`.
#[must_use]
pub fn pmt_body(pcr_pid: u16, program_info: &[u8], streams: &[(u8, u16, &[u8])]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&((pcr_pid & 0x1FFF) | 0xE000).to_be_bytes());
    body.extend_from_slice(&((program_info.len() & 0x0FFF) as u16 | 0xF000).to_be_bytes());
    body.extend_from_slice(program_info);
    for &(stream_type, pid, es_info) in streams {
        body.push(stream_type);
        body.extend_from_slice(&((pid & 0x1FFF) | 0xE000).to_be_bytes());
        body.extend_from_slice(&((es_info.len() & 0x0FFF) as u16 | 0xF000).to_be_bytes());
        body.extend_from_slice(es_info);
    }
    body
}

/// An SDT body (after the 8 fixed section bytes):
/// `original_network_id` plus one service
/// `(service_id, provider, name)` with its 0x48
/// descriptor. The service-loop header is **5 bytes** (the EN 300 468
/// §5.2.3 pin this card's differential re-verified).
#[must_use]
pub fn sdt_body(original_network_id: u16, services: &[(u16, &str, &str)]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&original_network_id.to_be_bytes());
    body.push(0xFF); // reserved
    for &(service_id, provider, name) in services {
        body.extend_from_slice(&service_id.to_be_bytes());
        body.push(0xFC); // reserved/EIT flags
        let mut descriptor = vec![0x48, 0, 0x01, provider.len() as u8];
        descriptor.extend_from_slice(provider.as_bytes());
        descriptor.push(name.len() as u8);
        descriptor.extend_from_slice(name.as_bytes());
        descriptor[1] = (descriptor.len() - 2) as u8;
        let dl = descriptor.len() & 0x0FFF;
        body.extend_from_slice(&(dl as u16 | 0xF000).to_be_bytes());
        body.extend_from_slice(&descriptor);
    }
    body
}

/// Build one TS packet carrying `payload` on `pid` with `pusi` set,
/// continuity counter `cc`, and (when `pcr` is `Some`) an adaptation
/// field carrying that raw PCR:
///
/// - `payload` — the packet's payload bytes (padded to 188).
/// - `pcr` — the raw `(base, extension)` pair when present.
#[must_use]
pub fn ts_packet(pid: u16, pusi: bool, cc: u8, pcr: Option<(u64, u16)>, payload: &[u8]) -> Vec<u8> {
    ts_packet_tuned(pid, pusi, cc, pcr, payload, false, false)
}

/// [`ts_packet`] with the transport-error and discontinuity bits
/// tunable (the damage-path tests).
#[must_use]
pub fn ts_packet_tuned(
    pid: u16,
    pusi: bool,
    cc: u8,
    pcr: Option<(u64, u16)>,
    payload: &[u8],
    tei: bool,
    af_discontinuity: bool,
) -> Vec<u8> {
    let mut af: Vec<u8> = Vec::new();
    if let Some((base, ext)) = pcr {
        af.push(if af_discontinuity { 0x80 | 0x10 } else { 0x10 });
        // 33-bit base, 6 reserved bits, 9-bit extension.
        af.push((base >> 25) as u8);
        af.push((base >> 17) as u8);
        af.push((base >> 9) as u8);
        af.push((base >> 1) as u8);
        af.push(((((base & 1) as u16) << 7) | ((ext >> 8) & 1) | 0x7E) as u8);
        af.push((ext & 0xFF) as u8);
    } else if af_discontinuity {
        af.push(0x80);
    }
    let mut out = vec![0u8; 188];
    out[0] = 0x47;
    out[1] = u8::from(tei) << 7 | u8::from(pusi) << 6 | ((pid >> 8) & 0x1F) as u8;
    out[2] = (pid & 0xFF) as u8;
    let afc = if af.is_empty() { 0b01 } else { 0b11 };
    out[3] = afc << 4 | (cc & 0x0F);
    let mut at = 4;
    if !af.is_empty() {
        af.insert(0, af.len() as u8);
        out[4..4 + af.len()].copy_from_slice(&af);
        at = 4 + af.len();
    }
    let take = payload.len().min(188 - at);
    out[at..at + take].copy_from_slice(&payload[..take]);
    out
}

/// Wrap complete sections into TS packets on `pid`: the first
/// packet of each section carries PUSI + pointer field 0, the rest
/// continue (continuity counters held across the whole call).
#[must_use]
pub fn psi_packets(pid: u16, cc_start: u8, sections: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut cc = cc_start;
    for section in sections {
        let mut payload = vec![0u8]; // pointer_field
        payload.extend_from_slice(section);
        for (index, chunk) in payload.chunks(184).enumerate() {
            let pusi = index == 0;
            out.push(ts_packet(pid, pusi, cc, None, chunk));
            cc = (cc + 1) & 0x0F;
        }
    }
    out
}

/// Concatenate packets into a stream.
#[must_use]
pub fn stream_of(packets: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in packets {
        out.extend_from_slice(p);
    }
    out
}

/// A minimal single-program TS: PAT → PMT → SDT → two PES-looking
/// packets on the video ES (counter continuity held throughout).
#[must_use]
pub fn single_program_ts() -> Vec<u8> {
    let pat = psi_section(0x00, 1, 0, &pat_body(&[(1, 0x1000)]));
    let pmt = psi_section(
        0x02,
        1,
        0,
        &pmt_body(0x100, &[], &[(0x1B, 0x100, &[]), (0x0F, 0x101, &[])]),
    );
    let sdt = psi_section(0x42, 1, 0, &sdt_body(1, &[(1, "FFmpeg", "Service01")]));
    let mut packets = Vec::new();
    packets.extend(psi_packets(0x0011, 0, &[sdt]));
    packets.extend(psi_packets(0x0000, 0, &[pat]));
    packets.extend(psi_packets(0x1000, 0, &[pmt]));
    packets.push(ts_packet(0x100, true, 0, Some((90_000, 0)), &[0x00; 160]));
    packets.push(ts_packet(0x100, false, 1, None, &[0x01; 160]));
    packets.push(ts_packet(0x101, true, 0, None, &[0x02; 160]));
    stream_of(&packets)
}
