//! The W4-R1 regression suite — the `strl` advisory-children contract
//! (the W3-16 corpus audit's one real defect, fixed alongside this
//! suite).
//!
//! ffmpeg authors a video `strl` as strh + strf + JUNK + **vprp** on
//! every clean file — measured across corpus-big (6/6 sampled
//! avi-video files: strh(56) + strf(40) + JUNK(4120) + vprp(68)).
//! Pre-fix, `vprp` fell into the unknown-chunk catch-all, so every
//! clean supported corpus row reported recovered=1/damage=68: the
//! damage channel cried wolf on each of the 57 clean rows, and the
//! "dirty AVIs that recovered damage mid-walk" metric actually measured
//! "walks that got past start()".
//!
//! The contract this suite pins, both directions:
//!
//! 1. **Clean is clean** — the exact ffmpeg strl shape (plus the
//!    `strd`/`strn` advisory peers) walks with zero `Recovered`
//!    events and `damage_total() == 0`.
//! 2. **Unknown is still damage** (the D-3 dirty-world guarantee the
//!    fix must not weaken) — an unknown child (`XXXX`) is noted
//!    exactly once, with its payload byte size, and the walk
//!    survives it to EOS with every frame intact.

use demux::avi::AviDemuxer;
use demux::avi::test_support;
use media_types::Stamp;
use plugin_api::{DemuxEvent, Demuxer};

// ---------------------------------------------------------------------------
// A tiny AVI builder — the same byte rules the adversarial-gaps suite
// and the xtask fixture writer use (WORD-aligned chunks, patched
// LIST/RIFF sizes, idx1 offsets relative to the `movi` form FourCC).
// The one axis it adds: arbitrary children inside the `strl`.
// ---------------------------------------------------------------------------

fn chunk(out: &mut Vec<u8>, fourcc: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(fourcc);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    if payload.len() & 1 == 1 {
        out.push(0);
    }
}

fn end_sized(out: &mut [u8], size_pos: usize) {
    let size = out.len() - size_pos - 4;
    out[size_pos..size_pos + 4].copy_from_slice(&(size as u32).to_le_bytes());
}

fn avih(streams: u32, total_frames: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(56);
    for value in [
        33_333u32, // dwMicroSecPerFrame
        0,         // dwMaxBytesPerSec
        0,         // dwPaddingGranularity
        0x110,     // AVIF_HASINDEX | AVIF_ISINTERLEAVED
        total_frames,
        0, // dwInitialFrames
        streams,
        0, // dwSuggestedBufferSize
        320,
        240,
    ] {
        v.extend_from_slice(&value.to_le_bytes());
    }
    v.extend(&[0u8; 16]);
    v
}

fn strh_video(rate: u32, length: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(56);
    v.extend_from_slice(b"vids");
    v.extend_from_slice(b"MJPG");
    v.extend_from_slice(&[0u8; 12]); // flags, priority, language, initial
    v.extend_from_slice(&1u32.to_le_bytes()); // dwScale
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes()); // dwStart
    v.extend_from_slice(&length.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes()); // dwSuggestedBufferSize
    v.extend_from_slice(&0u32.to_le_bytes()); // dwQuality
    v.extend_from_slice(&0u32.to_le_bytes()); // dwSampleSize (video: per-chunk)
    v.extend_from_slice(&[0u8; 8]); // rcFrame
    v
}

fn strf_video(width: u32, height: i32) -> Vec<u8> {
    let mut v = Vec::with_capacity(40);
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&width.to_le_bytes());
    v.extend_from_slice(&height.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&24u16.to_le_bytes());
    v.extend_from_slice(b"MJPG");
    v.extend_from_slice(&[0u8; 20]);
    v
}

/// One frame's distinctive payload (length and content vary per frame;
/// the digests must differ so payload truth is byte-exact).
fn frame_payload(index: usize) -> Vec<u8> {
    vec![(index % 251) as u8; 5 + index]
}

/// The exact 68-byte `vprp` payload ffmpeg 7.1.5 authors (copied from
/// corpus-big's 0021-avi-av.avi — our own generated corpus, measured
/// not transcribed from any foreign source). The demuxer never parses
/// it; the bytes are real so the regression cannot pass on a
/// coincidentally-empty stand-in.
const VPRP_FFMPEG: [u8; 68] = [
    0x00, 0x00, 0x00, 0x00, // dwVideoFormatToken
    0x00, 0x00, 0x00, 0x00, // dwControlFlags
    0x0c, 0x00, 0x00, 0x00, // dwPictAspectRatioX
    0x68, 0x01, 0x00, 0x00, // dwPictAspectRatioY
    0x80, 0x02, 0x00, 0x00, // dwEditRate
    0x10, 0x00, 0x09, 0x00, // dwSeqNum
    0x68, 0x01, 0x00, 0x00, // VideoFieldDescription[]
    0x80, 0x02, 0x00, 0x00, //
    0x01, 0x00, 0x00, 0x00, //
    0x80, 0x02, 0x00, 0x00, //
    0x68, 0x01, 0x00, 0x00, //
    0x80, 0x02, 0x00, 0x00, //
    0x68, 0x01, 0x00, 0x00, //
    0x00, 0x00, 0x00, 0x00, //
    0x00, 0x00, 0x00, 0x00, //
    0x00, 0x00, 0x00, 0x00, //
    0x00, 0x00, 0x00, 0x00, //
];

/// ffmpeg's measured inter-strl padding (4120 zero bytes, corpus-big).
fn junk_ffmpeg() -> Vec<u8> {
    vec![0u8; 4120]
}

/// Build a one-video-stream AVI (30 fps, one tick per chunk, all
/// chunks `00dc`, all keyframes, idx1 in file order) whose `strl`
/// carries `strh + strf + extra` — `extra` is where the advisory and
/// hostile shapes live.
const FRAMES: usize = 6;

fn build_avi_with_strl_children(extra: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"RIFF");
    let riff_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"AVI ");

    // LIST hdrl: avih + one LIST strl (video MJPG) with extra children.
    out.extend_from_slice(b"LIST");
    let hdrl_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"hdrl");
    chunk(&mut out, b"avih", &avih(1, FRAMES as u32));
    out.extend_from_slice(b"LIST");
    let strl_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"strl");
    chunk(&mut out, b"strh", &strh_video(30, FRAMES as u32));
    chunk(&mut out, b"strf", &strf_video(320, 240));
    for (fourcc, payload) in extra {
        chunk(&mut out, fourcc, payload);
    }
    end_sized(&mut out, strl_size_pos);
    end_sized(&mut out, hdrl_size_pos);

    // LIST movi: one chunk per frame (position + length kept for idx1).
    let movi_form_pos = (out.len() + 8) as u64;
    out.extend_from_slice(b"LIST");
    let movi_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"movi");
    let mut entries: Vec<(u64, usize)> = Vec::new();
    for index in 0..FRAMES {
        let payload = frame_payload(index);
        entries.push((out.len() as u64, payload.len()));
        chunk(&mut out, b"00dc", &payload);
    }
    end_sized(&mut out, movi_size_pos);

    // idx1 in file order (dwOffset relative to the movi form FourCC).
    let mut idx = Vec::new();
    for &(position, payload_len) in &entries {
        idx.extend_from_slice(b"00dc");
        idx.extend_from_slice(&0x10u32.to_le_bytes()); // AVIIF_KEYFRAME
        idx.extend_from_slice(&((position - movi_form_pos) as u32).to_le_bytes());
        idx.extend_from_slice(&(payload_len as u32).to_le_bytes());
    }
    chunk(&mut out, b"idx1", &idx);
    end_sized(&mut out, riff_size_pos);
    out
}

// ---------------------------------------------------------------------------
// Drivers
// ---------------------------------------------------------------------------

fn demuxer_over_bytes(bytes: Vec<u8>) -> AviDemuxer {
    let stream = match test_support::mem_stream(bytes) {
        Ok(stream) => stream,
        Err(err) => unreachable!("mem stream: {err}"),
    };
    match AviDemuxer::new(stream) {
        Ok(demuxer) => demuxer,
        Err(err) => unreachable!("demuxer over memory: {err}"),
    }
}

/// One packet's observable row: ticks on the stream's 1/30 base,
/// keyframe flag, raw payload bytes.
type PacketRow = (i64, bool, Vec<u8>);

/// One surfaced damage record: skipped bytes, pinned reason.
type DamageRow = (u64, &'static str);

/// Walk to EOS, collecting every packet row and damage record (six
/// frames — raw byte truth beats a digest at this scale). Returns
/// only at EOS: these walks are bounded and well-formed by
/// construction, so any error is a contract violation, not a case.
fn walk(avi: &mut AviDemuxer) -> (Vec<PacketRow>, Vec<DamageRow>) {
    let mut rows = Vec::new();
    let mut reasons = Vec::new();
    loop {
        match avi.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                let ticks = match packet.pts {
                    Stamp::Present(ts) => ts.ticks,
                    Stamp::Missing => unreachable!("stamps present by construction"),
                };
                rows.push((ticks, packet.keyframe, packet.data.as_slice().to_vec()));
            }
            Ok(DemuxEvent::Recovered { skipped, reason }) => reasons.push((skipped, reason)),
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => unreachable!("the strl walk is bounded and clean: {err}"),
        }
    }
    (rows, reasons)
}

/// The canonical six-frame truth every walk must deliver, unchanged:
/// ticks 0..FRAMES, all keyframes, per-frame payloads.
fn canonical_rows() -> Vec<PacketRow> {
    (0..FRAMES)
        .map(|index| (index as i64, true, frame_payload(index)))
        .collect()
}

// ---------------------------------------------------------------------------
// 1. Clean is clean
// ---------------------------------------------------------------------------

/// The exact ffmpeg-authored shape (measured, corpus-big): strh +
/// strf + JUNK(4120) + vprp(68). Pre-fix this file reported
/// `Recovered { skipped: 68, reason: "unknown-chunk-in-strl" }` on the
/// first `next_event` — the defect this card exists for.
#[test]
fn clean_ffmpeg_strl_shape_walks_with_zero_damage() {
    let bytes =
        build_avi_with_strl_children(&[(b"JUNK", junk_ffmpeg()), (b"vprp", VPRP_FFMPEG.to_vec())]);
    let mut avi = demuxer_over_bytes(bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1, "one video track"),
        Err(err) => unreachable!("clean ffmpeg shape starts: {err}"),
    }
    let (rows, reasons) = walk(&mut avi);
    assert!(
        reasons.is_empty(),
        "a clean ffmpeg-authored file surfaces zero damage events: {reasons:?}"
    );
    assert_eq!(avi.damage_total(), 0, "the damage ledger stays empty");
    assert_eq!(rows, canonical_rows(), "every frame walks, byte-exact");
}

/// The full advisory set — `strd` (codec-private data), `strn` (stream
/// name) and `vprp` alongside `JUNK` — is silently skipped: none of
/// them carries anything the v0 stream model consumes, and none of
/// them is damage.
#[test]
fn full_advisory_set_is_silently_skipped() {
    let bytes = build_avi_with_strl_children(&[
        (b"strd", vec![0xa5u8; 32]),
        (b"strn", b"W4-R1 corpus-shaped video\0".to_vec()),
        (b"JUNK", junk_ffmpeg()),
        (b"vprp", VPRP_FFMPEG.to_vec()),
    ]);
    let mut avi = demuxer_over_bytes(bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1, "one video track"),
        Err(err) => unreachable!("advisory children never block start: {err}"),
    }
    let (rows, reasons) = walk(&mut avi);
    assert!(
        reasons.is_empty(),
        "advisory children are not damage: {reasons:?}"
    );
    assert_eq!(avi.damage_total(), 0, "the damage ledger stays empty");
    assert_eq!(rows, canonical_rows(), "every frame walks, byte-exact");
}

// ---------------------------------------------------------------------------
// 2. Unknown is still damage (the guarantee the fix must not weaken)
// ---------------------------------------------------------------------------

/// An unknown `strl` child is noted exactly once with its payload byte
/// size — the D-3 catch-all the VPRP addition must leave intact. The
/// 37-byte odd payload also forces the WORD-pad advance path. The
/// damage record surfaces on the first `next_event` after `start()`
/// (the damage-record-first contract), and the walk survives the
/// unknown chunk to EOS with every frame intact.
#[test]
fn unknown_strl_child_is_damage_counted_not_fatal() {
    let hostile = vec![0xde_u8; 37];
    let bytes = build_avi_with_strl_children(&[(b"XXXX", hostile)]);
    let mut avi = demuxer_over_bytes(bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1, "the stream still resolves"),
        Err(err) => unreachable!("an unknown child never blocks start: {err}"),
    }
    // Damage-record-first: the Recovered row leads the event stream.
    match avi.next_event() {
        Ok(DemuxEvent::Recovered { skipped, reason }) => {
            assert_eq!(skipped, 37, "the note carries the payload byte size");
            assert_eq!(
                reason, "unknown-chunk-in-strl",
                "the reason string is the pinned catch-all"
            );
        }
        other => unreachable!("the damage record leads: {other:?}"),
    }
    let (rows, reasons) = walk(&mut avi);
    assert!(
        reasons.is_empty(),
        "exactly one note — no double counting after the surfaced record: {reasons:?}"
    );
    assert_eq!(avi.damage_total(), 37, "the ledger carries the note");
    assert_eq!(
        rows,
        canonical_rows(),
        "every frame still walks, byte-exact"
    );
}

/// `vprp` joining the silent-skip set must not mask unknown
/// neighbors: a vprp followed by an unknown chunk yields exactly the
/// unknown chunk's note — nothing more, nothing less.
#[test]
fn vprp_does_not_mask_unknown_neighbors() {
    let bytes = build_avi_with_strl_children(&[
        (b"vprp", VPRP_FFMPEG.to_vec()),
        (b"XXXX", vec![0xeeu8; 9]),
    ]);
    let mut avi = demuxer_over_bytes(bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1, "one video track"),
        Err(err) => unreachable!("mixed strl starts: {err}"),
    }
    let (rows, reasons) = walk(&mut avi);
    assert_eq!(
        reasons,
        vec![(9, "unknown-chunk-in-strl")],
        "only the unknown chunk is noted"
    );
    assert_eq!(avi.damage_total(), 9, "only its bytes are damage");
    assert_eq!(rows, canonical_rows(), "every frame walks, byte-exact");
}
