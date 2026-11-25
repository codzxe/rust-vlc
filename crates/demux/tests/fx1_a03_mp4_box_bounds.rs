//! FX1-A03 (FIX-PLAN §2 F0.4b) — `stts`/`ctts` entry-count bounds.
//!
//! The audit's crafted shape (W05-A01 F-003, S1): a small `stts`/`ctts`
//! whose header declares `entry_count = 0xFFFF_FFFF` while the box body
//! carries one or two entries. Before the clamp the loops iterated the
//! raw header count (~4.3e9 failing reads per box — a minutes-scale CPU
//! spin on open, repeatable per trak) while the sibling tables
//! (`stsz`/`stco`) already clamped their walks; after it, the walk honors
//! only the entries the body actually carries, the parse completes with
//! fewer entries (or none), and the unbacked remainder surfaces as a
//! `Recovered` damage note — never a panic, never a hang.
//!
//! The fixtures are well-formed files from `mp4::test_support::progressive`
//! with the `stts`/`ctts` box spliced in place at equal byte length (the
//! `stco` chunk offsets stay valid — the only lie in each file is the
//! entry count). Pre-fix, these suites fail on the damage-note assertion
//! after the unbounded spin; post-fix they complete in microseconds.
//! The minimal zero-run `stts` is the one exception: zero deltas leave
//! the progressive moov with no samples at all, so its honest outcome
//! is the demuxer's typed `no-sample-table` refusal — asserted as
//! such (bounded, no panic).

use demux::mp4::Mp4Demuxer;
use demux::mp4::test_support::{SampleSpec, TrackSpec, mem_stream, progressive};
use media_types::time::Stamp;
use plugin_api::{DemuxEvent, Demuxer};

/// The lying entry count the crafted boxes declare.
const LYING_COUNT: u32 = 0xFFFF_FFFF;

/// One `stts`/`ctts` entry as big-endian bytes: (run, delta-or-offset).
fn entry(run: u32, value: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(8);
    v.extend_from_slice(&run.to_be_bytes());
    v.extend_from_slice(&value.to_be_bytes());
    v
}

/// A full box: header + version/flags + the declared count + entries.
fn crafted_box(kind: &[u8; 4], count: u32, entries: &[u8]) -> Vec<u8> {
    let body = 8 + entries.len();
    let mut v = Vec::with_capacity(8 + body);
    v.extend_from_slice(&(8 + body as u32).to_be_bytes());
    v.extend_from_slice(kind);
    v.extend_from_slice(&[0u8; 4]); // version 0, flags 0
    v.extend_from_slice(&count.to_be_bytes());
    v.extend_from_slice(entries);
    v
}

/// Splice `crafted` over the file's first `kind` box — byte-for-byte
/// length-neutral so the `stco` offsets stay valid.
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
fn open(file: Vec<u8>) -> Mp4Demuxer {
    let stream = mem_stream(file).unwrap_or_else(|e| unreachable!("mem stream: {e}"));
    Mp4Demuxer::new(stream).unwrap_or_else(|e| unreachable!("construct: {e}"))
}

/// Drain to EOS (bounded), returning the packets' PTS ticks and the
/// `Recovered` damage notes.
fn drain(mp4: &mut Mp4Demuxer) -> (Vec<i64>, Vec<(u64, &'static str)>) {
    let mut pts = Vec::new();
    let mut notes = Vec::new();
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
            DemuxEvent::Recovered { skipped, reason } => notes.push((skipped, reason)),
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
    }
    (pts, notes)
}

/// `stts` declares 0xFFFFFFFF entries over a body carrying one run —
/// the walk must honor the one run (two samples), complete, and
/// damage-note the unbacked remainder.
#[test]
fn fx1_a03_stts_entry_count_clamped_to_box_body() {
    // Uniform durations: the builder's stts is one run, a 24-byte box.
    let track = video_track(&[3000, 3000, 3000, 3000], &[]);
    let file = progressive(&[track], b"isom");
    // One complete entry (run 2, delta 3000): the body carries one of
    // the declared 4 billion.
    let crafted = crafted_box(b"stts", LYING_COUNT, &entry(2, 3000));
    let file = splice(&file, b"stts", &crafted);
    let mut mp4 = open(file);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks.len(), 1);
    let (pts, notes) = drain(&mut mp4);
    // sizes.len() == 4, deltas.len() == 2 → the track completes with
    // fewer entries: two packets, 3000 ticks apart.
    assert_eq!(pts, vec![0, 3000], "completes with fewer entries");
    assert_eq!(
        notes,
        vec![(u64::from(LYING_COUNT) - 1, "stts-count-over-ceiling")],
        "the unbacked remainder is damage-noted"
    );
}

/// The audit's minimal crafted box: a lying count whose one present
/// entry is a zero-run. The walk honors no entries — zero deltas — and
/// the per-sample expansion is `min(sizes, deltas)` long, so the trak
/// builds zero samples and the progressive moov is left with nothing
/// to demux: the demuxer's honest answer is its typed
/// `no-sample-table` refusal (the same contract the in-crate
/// `zero_sample_tables_in_progressive_is_typed` suite pins for empty
/// stsz/stco tables) — a bounded parse and a typed error, never a
/// panic, and never the pre-fix ~4.3e9-iteration spin before the
/// same refusal arrives.
#[test]
fn fx1_a03_stts_minimal_box_honors_no_entries() {
    let track = video_track(&[3000, 3000, 3000, 3000], &[]);
    let file = progressive(&[track], b"isom");
    let crafted = crafted_box(b"stts", LYING_COUNT, &entry(0, 0));
    let file = splice(&file, b"stts", &crafted);
    let mut mp4 = open(file);
    // Zero deltas → zero samples → the moov carries no demuxable
    // sample table: the typed refusal, not a successful parse.
    match mp4.start() {
        Err(err) => assert_eq!(err.context.code, "demux.mp4.no-sample-table"),
        Ok(_) => unreachable!("zero-entry stts: the typed refusal was required"),
    }
}

/// `ctts` declares 0xFFFFFFFF entries over a body carrying two runs —
/// two composition offsets are honored, the rest default to zero, and
/// the unbacked remainder is damage-noted.
#[test]
fn fx1_a03_ctts_entry_count_clamped_to_box_body() {
    // Non-zero cts values make the builder emit a ctts (one entry per
    // sample: a 48-byte box).
    let track = video_track(&[3000, 3000, 3000, 3000], &[2048, 2048, 2048, 2048]);
    let file = progressive(&[track], b"isom");
    // Two complete entries (1, 2048) and (1, 1024); the trailing zeros
    // are inert zero-run padding that keeps the splice length-neutral.
    let mut entries = entry(1, 2048);
    entries.extend_from_slice(&entry(1, 1024));
    entries.extend_from_slice(&[0u8; 16]);
    let crafted = crafted_box(b"ctts", LYING_COUNT, &entries);
    let file = splice(&file, b"ctts", &crafted);
    let mut mp4 = open(file);
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    assert_eq!(tracks.len(), 1);
    let (pts, notes) = drain(&mut mp4);
    // The stts/stsz tables are intact (4 samples, 3000 ticks apart);
    // only two cts entries exist: [2048, 1024, 0, 0].
    assert_eq!(
        pts,
        vec![2048, 3000 + 1024, 6000, 9000],
        "truncated ctts completes with fewer entries"
    );
    assert_eq!(
        notes,
        vec![(u64::from(LYING_COUNT) - 4, "ctts-count-over-ceiling")],
        "the unbacked remainder is damage-noted"
    );
}
