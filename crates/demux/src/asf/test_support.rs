//! In-crate byte builders for the ASF tests (card 6.ASF-a).
//!
//! Public like [`crate::wav::test_support`] on purpose: rustdoc
//! examples cannot see `#[cfg(test)]` items, and the fuzz harness
//! needs deterministic shapes. This is test-support surface, not
//! player surface — nothing in the demuxer itself uses it.

use std::sync::Arc;

use plugin_api::SourceStream;

/// A boxed in-memory [`SourceStream`] over `bytes` — the demux
/// crate's shared test seam (the AVI module's implementation, reused
/// verbatim: one `SourceStream`, not two).
///
/// # Errors
/// Propagates [`crate::avi::test_support::BytesStream::new`]'s
/// (never-failing) URI parse.
pub fn mem_stream(bytes: Arc<[u8]>) -> Result<Box<dyn SourceStream>, media_types::EngineError> {
    crate::avi::test_support::mem_stream(bytes)
}

/// Append a GUID's wire bytes.
fn push_guid(out: &mut Vec<u8>, guid: [u8; 16]) {
    out.extend_from_slice(&guid);
}

/// Append a little-endian QWORD.
fn push_qword(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Append a little-endian DWORD.
fn push_dword(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Append a little-endian WORD.
fn push_word(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// The File Properties Object payload (the 80-byte layout this unit
/// pinned — see `header`'s module docs).
fn file_properties_payload(flags: u32, preroll_ms: u64, packet_size: u32, packets: u64) -> Vec<u8> {
    let mut p = Vec::with_capacity(80);
    p.extend_from_slice(&[0u8; 16]); // File ID (zeros — ffmpeg's own)
    push_qword(&mut p, 1_024); // File Size (patched by the builder)
    push_qword(&mut p, 116_444_736_000_000_000); // Creation Date constant
    push_qword(&mut p, packets); // Data Packets Count
    push_qword(&mut p, 1_000_000); // Play Duration (100 ns)
    push_qword(&mut p, 1_000_000); // Send Duration
    push_qword(&mut p, preroll_ms); // Preroll
    push_dword(&mut p, flags); // Flags
    push_dword(&mut p, packet_size); // Min packet size
    push_dword(&mut p, packet_size); // Max packet size
    push_dword(&mut p, 64_000); // Max bitrate
    p
}

/// An audio Stream Properties payload: audio type, audio-spread EC
/// type, stream `number`, the WAVEFORMATEX `tsd` as type-specific
/// data.
fn audio_stream_properties_payload(number: u16, tsd: &[u8]) -> Vec<u8> {
    let mut p = Vec::with_capacity(54 + tsd.len());
    push_guid(
        &mut p,
        [
            0x40, 0x9E, 0x69, 0xF8, 0x4D, 0x5B, 0xCF, 0x11, 0xA8, 0xFD, 0x00, 0x80, 0x5F, 0x5C,
            0x44, 0x2B,
        ],
    );
    push_guid(
        &mut p,
        [
            0x50, 0xCD, 0xC3, 0xBF, 0x8F, 0x61, 0xCF, 0x11, 0x8B, 0xB2, 0x00, 0xAA, 0xB4, 0xE2,
            0x20, 0xC0,
        ],
    );
    push_qword(&mut p, 0); // Time Offset
    push_dword(&mut p, tsd.len() as u32); // TSD length
    push_dword(&mut p, 0); // EC data length
    push_word(&mut p, number); // Stream number
    push_word(&mut p, 0); // Language index
    push_word(&mut p, 0); // The 2 unpinned flag bytes
    p.extend_from_slice(tsd);
    p
}

/// Wrap `payload` in a header-child object frame (GUID + size).
fn child(guid: [u8; 16], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(24 + payload.len());
    push_guid(&mut out, guid);
    push_qword(&mut out, 24 + payload.len() as u64);
    out.extend_from_slice(payload);
    out
}

/// A minimal valid ASF: Header Object (File Properties with
/// seekable-only flags + one audio Stream Properties carrying `wfx`
/// as its type-specific data) + an empty-area Data Object. The
/// smallest shape every walk path must accept; `wfx` is the caller's
/// WAVEFORMATEX (e.g. the 18-byte PCM form for tag 1 / 16-bit).
#[must_use]
pub fn minimal_asf_audio(wfx: &[u8]) -> Arc<[u8]> {
    let mut children = child(
        [
            0xA1, 0xDC, 0xAB, 0x8C, 0x47, 0xA9, 0xCF, 0x11, 0x8E, 0xE4, 0x00, 0xC0, 0x0C, 0x20,
            0x53, 0x65,
        ],
        &file_properties_payload(0x2, 0, 3200, 1),
    );
    children.extend(child(
        [
            0x91, 0x07, 0xDC, 0xB7, 0xB7, 0xA9, 0xCF, 0x11, 0x8E, 0xE6, 0x00, 0xC0, 0x0C, 0x20,
            0x53, 0x65,
        ],
        &audio_stream_properties_payload(1, wfx),
    ));

    let mut out = Vec::new();
    push_guid(
        &mut out,
        [
            0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62,
            0xCE, 0x6C,
        ],
    );
    push_qword(&mut out, 30 + children.len() as u64);
    push_dword(&mut out, 2); // Declared child count (matches the walk)
    out.extend_from_slice(&[0x01, 0x02]); // The 2 reserved bytes
    out.extend_from_slice(&children);

    // Data Object: GUID + size + File ID + count + reserved 01 01.
    let data_start = out.len();
    push_guid(
        &mut out,
        [
            0x36, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62,
            0xCE, 0x6C,
        ],
    );
    push_qword(&mut out, 50); // Size (patched below)
    out.extend_from_slice(&[0u8; 16]); // File ID
    push_qword(&mut out, 0); // Total data packets
    out.extend_from_slice(&[0x01, 0x01]); // Reserved
    let data_size = out.len() - data_start;
    let end = out.len();
    out[data_start + 16..data_start + 24].copy_from_slice(&(data_size as u64).to_le_bytes());
    // Patch the FPO's File Size to the real total.
    let header_size = 30 + children.len();
    let fpo_payload_at = 30 + 24;
    let total = end as u64;
    out[fpo_payload_at + 16..fpo_payload_at + 24].copy_from_slice(&total.to_le_bytes());
    debug_assert_eq!(header_size, 30 + children.len());
    let _ = header_size;
    Arc::from(out)
}

/// A minimal 18-byte WAVEFORMATEX (the PCM forms the doctest and the
/// unit tests use).
#[must_use]
pub fn waveformatex(tag: u16, channels: u16, rate: u32, align: u16, bits: u16) -> Vec<u8> {
    let mut wfx = Vec::with_capacity(18);
    push_word(&mut wfx, tag);
    push_word(&mut wfx, channels);
    push_dword(&mut wfx, rate);
    push_dword(&mut wfx, u32::from(align) * rate); // Avg bytes/sec
    push_word(&mut wfx, align);
    push_word(&mut wfx, bits);
    push_word(&mut wfx, 0); // cbSize
    wfx
}

/// One Simple Index Object entry spec for [`asf_with_packets`]:
/// `(packet number, packet count)` — the ffmpeg-measured 6-byte
/// entry form (DWORD packet number, WORD count; the number is
/// non-decreasing across every measured file — the seek fact).
pub type IndexEntry = (u32, u16);

/// The Simple Index Object spec for [`asf_with_packets`]: the
/// interval (100 ns units; ffmpeg writes 10,000,000 = 1 s) and the
/// entries. The object is appended after the Data Object exactly
/// where the ffmpeg muxer puts it, in the measured layout:
/// GUID + size + file id (16) + interval QWORD + the measured
/// DWORD (0/1 observed, carried raw) + entry count + 6-byte entries.
pub struct SimpleIndexSpec {
    /// The per-entry interval in 100 ns units.
    pub interval_100ns: u64,
    /// `(packet number, packet count)` per interval bucket.
    pub entries: Vec<IndexEntry>,
}

/// The ASF-b/c end-to-end builder: the minimal header (one audio
/// stream carrying `wfx`, FPO with `preroll_ms` and the fixed
/// `packet_size`) + a Data Object holding `packets` verbatim + an
/// optional trailing Simple Index Object. Every size field (FPO
/// packet count, Data Object size/count, Simple Index size/count) is
/// patched to the real totals — the shape the walk must accept.
#[must_use]
pub fn asf_with_packets(
    wfx: &[u8],
    preroll_ms: u64,
    packet_size: u32,
    packets: &[Vec<u8>],
    simple_index: Option<&SimpleIndexSpec>,
) -> Arc<[u8]> {
    let mut children = child(
        [
            0xA1, 0xDC, 0xAB, 0x8C, 0x47, 0xA9, 0xCF, 0x11, 0x8E, 0xE4, 0x00, 0xC0, 0x0C, 0x20,
            0x53, 0x65,
        ],
        &file_properties_payload(
            0x2,
            preroll_ms,
            packet_size,
            packets.len() as u64,
        ),
    );
    children.extend(child(
        [
            0x91, 0x07, 0xDC, 0xB7, 0xB7, 0xA9, 0xCF, 0x11, 0x8E, 0xE6, 0x00, 0xC0, 0x0C, 0x20,
            0x53, 0x65,
        ],
        &audio_stream_properties_payload(1, wfx),
    ));

    let mut out = Vec::new();
    push_guid(
        &mut out,
        [
            0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62,
            0xCE, 0x6C,
        ],
    );
    push_qword(&mut out, 30 + children.len() as u64);
    push_dword(&mut out, 2); // Declared child count
    out.extend_from_slice(&[0x01, 0x02]); // Reserved
    out.extend_from_slice(&children);

    // Data Object with the packets verbatim (size/count patched).
    let data_start = out.len();
    push_guid(
        &mut out,
        [
            0x36, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62,
            0xCE, 0x6C,
        ],
    );
    push_qword(&mut out, 50); // Size (patched below)
    out.extend_from_slice(&[0u8; 16]); // File ID
    push_qword(&mut out, packets.len() as u64); // Total data packets
    out.extend_from_slice(&[0x01, 0x01]); // Reserved
    for packet in packets {
        out.extend_from_slice(packet);
    }
    let data_size = out.len() - data_start;
    out[data_start + 16..data_start + 24].copy_from_slice(&(data_size as u64).to_le_bytes());

    // Optional trailing Simple Index Object (the measured layout).
    if let Some(spec) = simple_index {
        let index_start = out.len();
        push_guid(
            &mut out,
            [
                0x90, 0x08, 0x00, 0x33, 0xB1, 0xE5, 0xCF, 0x11, 0x89, 0xF4, 0x00, 0xA0, 0xC9,
                0x03, 0x49, 0xCB,
            ],
        );
        push_qword(&mut out, 0); // Size (patched below)
        out.extend_from_slice(&[0u8; 16]); // File ID
        push_qword(&mut out, spec.interval_100ns);
        push_dword(&mut out, 0); // The measured DWORD (0/1 observed)
        push_dword(&mut out, spec.entries.len() as u32);
        for (number, count) in &spec.entries {
            push_dword(&mut out, *number);
            push_word(&mut out, *count);
        }
        let index_size = out.len() - index_start;
        out[index_start + 16..index_start + 24]
            .copy_from_slice(&(index_size as u64).to_le_bytes());
    }

    // Patch the FPO's File Size to the real total.
    let fpo_payload_at = 30 + 24;
    let total = out.len() as u64;
    out[fpo_payload_at + 16..fpo_payload_at + 24].copy_from_slice(&total.to_le_bytes());
    Arc::from(out)
}
