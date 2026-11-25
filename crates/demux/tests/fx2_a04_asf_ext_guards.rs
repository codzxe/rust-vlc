//! FX2-A04 — the ASF Header Extension child walk's unchecked adds
//! (W04-A10 F-002, the sibling FX1-A07 recorded in its risk section:
//! F0.4j fixed the header children walk's own size add, this twin was
//! left out of that scope).
//!
//! The defect: `walk_header_extension`'s bound check computed
//! `off as u64 + size` with a plain add and advanced the cursor with
//! `off += size as usize`. A crafted near-u64::MAX extension-child
//! size overflowed the comparison add (debug builds: panic at the
//! arithmetic; release builds: the wrapped sum slipped past the
//! `> end` bound guard and the wrapped cursor walked backwards — a
//! bounded re-read spin up to MAX_EXTENSION_CHILDREN). The fixed walk
//! computes one checked sum that feeds both the bound check and the
//! cursor advance: overflow is the typed `demux.asf.bad-object-size`
//! refusal — the sibling idiom the header children walk already
//! follows.
//!
//! The crafted blobs are the builder's minimal one-audio-stream ASF
//! with a Header Extension Object spliced in before the Data Object
//! (the Header Object's size QWORD, its declared child count and the
//! FPO's File Size patched to the grown totals). Each extension child
//! is a bare 24-byte object head — GUID + size QWORD — because the
//! walk is capture-and-ignore: the size QWORD is the only live field,
//! exactly the byte the defect lived on.
//!
//! * first child (ESP) size := `u64::MAX` — the overflow at the
//!   walk's opening iteration;
//! * a well-formed first child, then a second with size :=
//!   `u64::MAX` — the same class mid-walk, after one clean advance;
//! * a 1000-byte child inside a 24-byte data field — no u64 overflow,
//!   the pre-existing overrun note-and-break path, unchanged;
//! * an ESP child plus an unknown-GUID child — the control: both are
//!   counted (the unknown damage-noted) and the file still starts.

use std::sync::Arc;

use demux::asf::AsfDemuxer;
use demux::asf::test_support::{mem_stream, minimal_asf_audio, waveformatex};
use media_types::EngineError;
use plugin_api::Demuxer;

/// The Header Extension Object GUID (asf/guid.rs).
const HEADER_EXTENSION: [u8; 16] = [
    0xB5, 0x03, 0xBF, 0x5F, 0x2E, 0xA9, 0xCF, 0x11, 0x8E, 0xE3, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65,
];

/// The Extended Stream Properties GUID (asf/guid.rs) — the child the
/// walk counts silently; any other GUID is damage-noted.
const EXTENDED_STREAM_PROPERTIES: [u8; 16] = [
    0xEA, 0xCB, 0xF8, 0xC5, 0xAF, 0x5B, 0x77, 0x48, 0x84, 0x67, 0xAA, 0x8C, 0x44, 0xFA, 0x4C, 0xCA,
];

/// The minimal file's fixed layout: Header head 30 + File Properties
/// 104 + Stream Properties 96 — the Data Object starts at 230.
const DATA_OBJECT_AT: usize = 30 + 104 + 96;

/// One extension child: GUID + size QWORD — a bare 24-byte head (the
/// walk never slices a child payload; the size QWORD is the only live
/// field).
fn child_head(guid: &[u8; 16], size: u64) -> Vec<u8> {
    let mut head = Vec::with_capacity(24);
    head.extend_from_slice(guid);
    head.extend_from_slice(&size.to_le_bytes());
    head
}

/// The minimal one-audio-stream ASF with a Header Extension Object
/// spliced in before the Data Object, carrying `children` as its data
/// field: reserved GUID(16) + reserved WORD(2, observed 6) + data-size
/// DWORD(4) + children. The Header Object's size QWORD ([16..24]),
/// declared child count ([24..28], 2 → 3) and the FPO's File Size
/// QWORD ([70..78]) are patched to the grown totals.
fn with_header_extension(children: &[Vec<u8>]) -> Arc<[u8]> {
    let wfx = waveformatex(1, 1, 11_025, 2, 16);
    let mut bytes = minimal_asf_audio(&wfx).to_vec();
    let carried: usize = children.iter().map(Vec::len).sum();
    let mut payload = Vec::with_capacity(22 + carried);
    payload.extend_from_slice(&[0u8; 16]); // reserved GUID
    payload.extend_from_slice(&6u16.to_le_bytes()); // reserved WORD
    let data_size = match u32::try_from(carried) {
        Ok(v) => v,
        Err(err) => unreachable!("extension children fit a DWORD: {err}"),
    };
    payload.extend_from_slice(&data_size.to_le_bytes()); // data size DWORD
    for child in children {
        payload.extend_from_slice(child);
    }
    let mut ext = Vec::with_capacity(24 + payload.len());
    ext.extend_from_slice(&HEADER_EXTENSION);
    ext.extend_from_slice(&((24 + payload.len()) as u64).to_le_bytes());
    ext.extend_from_slice(&payload);
    let mut out = Vec::with_capacity(bytes.len() + ext.len());
    out.extend_from_slice(&bytes[..DATA_OBJECT_AT]);
    out.extend_from_slice(&ext);
    out.extend_from_slice(&bytes[DATA_OBJECT_AT..]);
    let header_size = (DATA_OBJECT_AT + ext.len()) as u64;
    out[16..24].copy_from_slice(&header_size.to_le_bytes());
    out[24..28].copy_from_slice(&3u32.to_le_bytes());
    let file_size = out.len() as u64;
    out[70..78].copy_from_slice(&file_size.to_le_bytes());
    Arc::from(out.as_slice())
}

/// Hand-crafted bytes to `start` and its typed error — a success is
/// itself the failure under test.
fn start_refusal(bytes: Arc<[u8]>) -> EngineError {
    let stream = match mem_stream(bytes) {
        Ok(stream) => stream,
        Err(err) => unreachable!("mem_stream: {err}"),
    };
    let mut asf = match AsfDemuxer::new(stream) {
        Ok(asf) => asf,
        Err(err) => unreachable!("new: {err}"),
    };
    match asf.start() {
        Ok(_) => unreachable!("crafted extension child size must be refused"),
        Err(err) => err,
    }
}

/// Hand-crafted bytes through `start` to its track list.
fn start_tracks(bytes: Arc<[u8]>) -> Vec<media_types::Track> {
    let stream = match mem_stream(bytes) {
        Ok(stream) => stream,
        Err(err) => unreachable!("mem_stream: {err}"),
    };
    let mut asf = match AsfDemuxer::new(stream) {
        Ok(asf) => asf,
        Err(err) => unreachable!("new: {err}"),
    };
    match asf.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("crafted file must start: {err}"),
    }
}

/// A crafted near-u64::MAX extension-child size is the typed refusal —
/// at the walk's first iteration and mid-walk after a clean advance.
/// Pre-fix: debug builds panicked at the plain `off + size` add;
/// release builds wrapped the sum past the bound guard.
#[test]
fn fx2_a04_asf_extension_child_size_overflow_is_typed_bad_object_size() {
    let err = start_refusal(with_header_extension(&[child_head(
        &EXTENDED_STREAM_PROPERTIES,
        u64::MAX,
    )]));
    assert_eq!(err.context.code, "demux.asf.bad-object-size");

    let err = start_refusal(with_header_extension(&[
        child_head(&EXTENDED_STREAM_PROPERTIES, 24),
        child_head(&EXTENDED_STREAM_PROPERTIES, u64::MAX),
    ]));
    assert_eq!(err.context.code, "demux.asf.bad-object-size");
}

/// A child size that overruns the data field WITHOUT overflowing the
/// sum: the pre-existing note-and-break path — damage-noted, the file
/// still starts. Pins that the checked add changed nothing for
/// non-overflowing sizes.
#[test]
fn fx2_a04_asf_extension_overrun_without_overflow_still_walks() {
    let tracks = start_tracks(with_header_extension(&[child_head(
        &EXTENDED_STREAM_PROPERTIES,
        1000,
    )]));
    assert_eq!(tracks.len(), 1, "one audio track");
    assert_eq!(tracks[0].codec.as_str(), "pcm_s16le");
}

/// Control: a well-formed ESP child and an unknown-GUID child — both
/// counted (the unknown damage-noted), the walk completes, the file
/// still starts.
#[test]
fn fx2_a04_asf_extension_well_formed_children_still_walk() {
    let tracks = start_tracks(with_header_extension(&[
        child_head(&EXTENDED_STREAM_PROPERTIES, 24),
        child_head(&[0x11u8; 16], 24),
    ]));
    assert_eq!(tracks.len(), 1, "one audio track");
    assert_eq!(tracks[0].codec.as_str(), "pcm_s16le");
}
