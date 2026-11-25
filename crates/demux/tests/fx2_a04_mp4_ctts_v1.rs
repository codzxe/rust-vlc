//! FX2-A04 — the ctts version-1 offset read (audit F-007, the
//! follow-up FX1-A03 recorded out of its clamp's scope).
//!
//! The defect: the v1 arm read the offset with `i64be` — 8 bytes at
//! the 4-byte offset field's position, i.e. the offset OR'd with the
//! NEXT entry's sample_count in the high 32 bits — so every ctts v1
//! file (ffmpeg's `-movflags +negative_cts_offsets` muxes) carried
//! astronomically wrong composition offsets. Entries are 8 bytes in
//! BOTH versions; per §8.6.1.3 v1 only changes the offset's
//! signedness (a 4-byte i32), so the fixed v1 read is the v0 lane's
//! own 4-byte read re-interpreted as i32.
//!
//! The fixture is the audit's executed ffmpeg ground truth
//! (negctts.mp4: `ctts v=1` with runs `(1, 0), (1, 2048),
//! (2, -1024)`), spliced byte-neutral over a well-formed progressive
//! file's own v0 ctts box (the builder writes one entry per sample,
//! offsets `as u32` — the v0 two's-complement lane). Both lanes must
//! read the same signed ladder; pre-fix the v1 lane produced pts
//! `[1, 8796093025210, -4398046505104, -4398046502104]` (verified
//! against a byte-exact python replication of the old misread — the
//! i64be read mixing offset and next-run bytes).

use std::sync::Arc;

use demux::mp4::Mp4Demuxer;
use demux::mp4::test_support::{SampleSpec, TrackSpec, mem_stream, progressive};
use media_types::time::Stamp;
use plugin_api::{DemuxEvent, Demuxer};

/// One ctts entry as big-endian bytes: (run, signed offset).
fn entry(run: u32, offset: i32) -> Vec<u8> {
    let mut v = Vec::with_capacity(8);
    v.extend_from_slice(&run.to_be_bytes());
    v.extend_from_slice(&(offset as u32).to_be_bytes());
    v
}

/// A full ctts box: header + version/flags + entry count + entries.
fn ctts_box(version: u8, count: u32, entries: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(16 + entries.len());
    v.extend_from_slice(&((16 + entries.len()) as u32).to_be_bytes());
    v.extend_from_slice(b"ctts");
    v.extend_from_slice(&[version, 0, 0, 0]);
    v.extend_from_slice(&count.to_be_bytes());
    v.extend_from_slice(entries);
    v
}

/// Splice `crafted` over the file's first `kind` box — byte-for-byte
/// length-neutral so the `stco` chunk offsets stay valid.
fn splice(file: &[u8], kind: &[u8; 4], crafted: &[u8]) -> Vec<u8> {
    let at = file
        .windows(4)
        .position(|w| w == &kind[..])
        .unwrap_or_else(|| unreachable!("no {kind:?} box in the fixture"));
    let head: [u8; 4] = file[at - 4..at]
        .try_into()
        .unwrap_or_else(|_| unreachable!("{kind:?}: box size field"));
    let size = u32::from_be_bytes(head) as usize;
    assert_eq!(crafted.len(), size, "{kind:?}: length-neutral splice");
    let mut out = file.to_vec();
    out[at - 4..at - 4 + size].copy_from_slice(crafted);
    out
}

/// One vide/avc1 track on the 9000 timescale.
fn video_track(durs: &[u64], cts: &[i64]) -> TrackSpec {
    TrackSpec {
        handler: *b"vide",
        entry: *b"avc1",
        timescale: 9000,
        edit: None,
        chap_ref: None,
        samples: durs
            .iter()
            .enumerate()
            .map(|(i, dur)| SampleSpec {
                bytes: vec![0x21; 4],
                dur: *dur,
                sync: true,
                cts: cts.get(i).copied().unwrap_or(0),
            })
            .collect(),
    }
}

/// Construct the demuxer over the crafted bytes (the in-memory seam).
fn open(file: impl Into<Arc<[u8]>>) -> Mp4Demuxer {
    let stream = match mem_stream(file) {
        Ok(stream) => stream,
        Err(err) => unreachable!("mem stream: {err}"),
    };
    Mp4Demuxer::new(stream).unwrap_or_else(|e| unreachable!("construct: {e}"))
}

/// Drain to EOS (bounded), returning the packets' PTS ticks.
fn drain_pts(mp4: &mut Mp4Demuxer) -> Vec<i64> {
    let mut pts = Vec::new();
    let mut events = 0usize;
    loop {
        events += 1;
        assert!(events < 64, "event budget");
        match mp4
            .next_event()
            .unwrap_or_else(|e| unreachable!("event: {e}"))
        {
            DemuxEvent::Packet(p) => pts.push(match &p.pts {
                Stamp::Present(ts) => ts.ticks,
                Stamp::Missing => unreachable!("pts missing"),
            }),
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
    }
    pts
}

/// The v1 lane: the ffmpeg ground-truth table (runs `(1, 0),
/// (1, 2048), (2, -1024)` — the negctts.mp4 shape) spliced as a
/// version-1 box. The signed offsets must read as i32: dts
/// [0, 3000, 6000, 9000] + cts [0, 2048, -1024, -1024] = the same
/// ladder the v0 lane reads. Pre-fix the 8-byte i64be read produced
/// pts [1, 8796093025210, -4398046505104, -4398046502104] — the
/// 4-byte offset OR'd with each next entry's sample_count.
#[test]
fn fx2_a04_ctts_v1_signed_offsets_read_as_i32() {
    let track = video_track(&[3000, 3000, 3000, 3000], &[0, 2048, -1024, -1024]);
    let file = progressive(&[track], b"isom");
    // Three real entries + one inert zero-entry of padding keeps the
    // splice byte-neutral over the builder's four-entry v0 box.
    let mut entries = entry(1, 0);
    entries.extend_from_slice(&entry(1, 2048));
    entries.extend_from_slice(&entry(2, -1024));
    entries.extend_from_slice(&[0u8; 8]);
    let crafted = ctts_box(1, 3, &entries);
    let file = splice(&file, b"ctts", &crafted);
    let mut mp4 = open(file);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks.len(), 1);
    let pts = drain_pts(&mut mp4);
    assert_eq!(pts, vec![0, 5048, 4976, 7976], "v1 signed i32 offsets");
}

/// The v0 control: the builder's own version-0 box with the same
/// offsets written `as u32` — the two's-complement lane the in-crate
/// `ctts_shifts_pts_off_dts` suite pins. The fixed v1 lane reads the
/// identical ladder; this control holds the v0 side of that equality
/// (and pins that the fix left v0 untouched).
#[test]
fn fx2_a04_ctts_v0_two_complement_lane_is_unchanged() {
    let track = video_track(&[3000, 3000, 3000, 3000], &[0, 2048, -1024, -1024]);
    let file = progressive(&[track], b"isom");
    let mut mp4 = open(file);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks.len(), 1);
    let pts = drain_pts(&mut mp4);
    assert_eq!(pts, vec![0, 5048, 4976, 7976], "v0 lane unchanged");
}
