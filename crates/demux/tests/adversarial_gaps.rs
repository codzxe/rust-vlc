//! The M16 adversarial-gap families (card W1-15; D-5 — escape the
//! synthetic echo chamber): four behaviors the clean-path suites never
//! exercise, pinned against the real fixture corpus and builder-built
//! degenerate siblings.
//!
//! 1. **Out-of-order `idx1`** — a valid AVI whose index entries are
//!    stored shuffled (seeded) or fully reversed. The `movi` walk is
//!    the packet-extraction truth (index order cannot change it);
//!    the seek table must come out **file-ordered with file-order
//!    timing** (the `index.rs` entry-order rule — the regression this
//!    family shipped with: pre-fix, the ladder ran in stored order,
//!    in-range seeks mis-landed or refused, and the reorder was
//!    silent).
//! 2. **Non-monotone seek ladders** — seek/reanchor over tables whose
//!    stored offsets violate monotonicity (a reversed run mid-table,
//!    duplicated `dwOffset`s), plus the defense-in-depth contract of
//!    [`search_index`](demux::avi::index::search_index) itself on a
//!    hand-built time-backwards table: typed results, keyframe-safe
//!    landings, no panic, no hang.
//! 3. **Dribble I/O** — a source whose reads return 1..n bytes
//!    (seeded chunking, down to a 1-byte maximum). A FULL walk to EOS
//!    through it must be semantically identical to the non-dribble
//!    walk of the same bytes (short reads are data, not errors — the
//!    `SourceStream` contract), with bounded read-loop behavior: no
//!    EOF spin, no quadratic re-read.
//! 4. **Wrong container (the B5 race)** — MP4-signatured bytes handed
//!    to the AVI demuxer path and AVI bytes behind a lying `.mp4`
//!    name: content truth decides. The demuxer's refusal is typed
//!    (`demux.avi.not-avi`), never a panic, never a speculative
//!    mis-walk into foreign bytes.
//!
//! Scope note (recorded deviation): the probe half of the probe→demux
//! handoff lives in `media-probe`, which is not in this crate's
//! dev-dependency allowlist (boundaries.toml — a root-config change is
//! Lead-only). The end-to-end refusal is pinned by the CLI suites
//! (`apps/cli` unsupported_path: "format `mp4` has no demuxer");
//! this family pins the **demux-side backstop** — what happens when
//! probe and demux disagree, whichever stage was wrong.

mod common;

use std::sync::{Arc, Mutex};

use common::{open_via_input, parse_expectation, sha256_hex};
use demux::avi::AviDemuxer;
use demux::avi::index::{IndexEntry, IndexTable, search_index};
use demux::avi::test_support;
use media_types::ids::SourceUri;
use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};
use media_types::{EngineError, ErrorKind, PipelineStage};
use plugin_api::{DemuxEvent, Demuxer, SeekOrigin, SeekSupport, SourcePosition, SourceStream};
use testkit::Must;

const PARENT: &str = "generated/valid/avi_mjpeg_pcm.avi";
const PARENT_EXPECTATION: &str = "expectations/valid/avi_mjpeg_pcm.avi.toml";
const MP4_PARENT: &str = "generated/valid/mp4_mjpeg_pcm.mp4";
const NOIDX0: &str = "generated/malformed/avi_mjpeg_pcm__noidx0.avi";

// ---------------------------------------------------------------------------
// A tiny AVI builder (the same byte rules the in-crate unit suite and
// the xtask fixture writer use: WORD-aligned chunks, patched LIST/RIFF
// sizes, idx1 offsets relative to the `movi` form FourCC). The one
// addition: the idx1 entries can be written in a caller-chosen order
// and a JUNK payload can be injected — the two degenerate shapes this
// suite exists for.
// ---------------------------------------------------------------------------

struct BuiltAvi {
    bytes: Vec<u8>,
    /// Absolute position of each movi chunk, in file order.
    chunk_positions: Vec<u64>,
}

fn chunk(out: &mut Vec<u8>, fourcc: &[u8; 4], payload: &[u8]) -> usize {
    let offset = out.len();
    out.extend_from_slice(fourcc);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    if payload.len() & 1 == 1 {
        out.push(0);
    }
    offset
}

fn end_sized(out: &mut [u8], size_pos: usize) {
    let size = out.len() - size_pos - 4;
    out[size_pos..size_pos + 4].copy_from_slice(&(size as u32).to_le_bytes());
}

fn avih(flags: u32, streams: u32, total_frames: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(56);
    for value in [
        33_333u32, // dwMicroSecPerFrame
        0,         // dwMaxBytesPerSec
        0,         // dwPaddingGranularity
        flags,
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
/// the digests must differ so walk-equality is byte-truth).
fn frame_payload(index: usize) -> Vec<u8> {
    vec![(index % 251) as u8; 5 + index]
}

/// Build a one-video-stream AVI (30 fps, one tick per chunk, all
/// chunks `00dc`) with an idx1 written in `order` (`None` = file
/// order; an `order[i]` of `j` stores frame `j`'s entry at idx1
/// position `i` — including duplicates, so lying tables can be
/// expressed). `junk`, when present, becomes a top-level `JUNK`
/// chunk's payload between `hdrl` and `movi`.
fn build_avi(
    frames: usize,
    keyframes: &[bool],
    order: Option<&[usize]>,
    junk: Option<&[u8]>,
) -> BuiltAvi {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"RIFF");
    let riff_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"AVI ");

    // LIST hdrl: avih + one LIST strl (video MJPG).
    out.extend_from_slice(b"LIST");
    let hdrl_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"hdrl");
    chunk(&mut out, b"avih", &avih(0x110, 1, frames as u32));
    out.extend_from_slice(b"LIST");
    let strl_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"strl");
    chunk(&mut out, b"strh", &strh_video(30, frames as u32));
    chunk(&mut out, b"strf", &strf_video(320, 240));
    end_sized(&mut out, strl_size_pos);
    end_sized(&mut out, hdrl_size_pos);

    if let Some(junk) = junk {
        chunk(&mut out, b"JUNK", junk);
    }

    // LIST movi: one chunk per frame.
    let movi_offset = out.len();
    out.extend_from_slice(b"LIST");
    let movi_size_pos = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"movi");
    let movi_form_pos = (movi_offset + 8) as u64;
    let mut chunk_positions = Vec::new();
    let mut payload_lens = Vec::new();
    for index in 0..frames {
        let payload = frame_payload(index);
        payload_lens.push(payload.len());
        chunk_positions.push(chunk(&mut out, b"00dc", &payload) as u64);
    }
    end_sized(&mut out, movi_size_pos);

    // idx1: entries in the caller's order (dwOffset relative to the
    // movi form FourCC, the spec base).
    let mut idx = Vec::new();
    let sequence: Vec<usize> = match order {
        Some(order) => order.to_vec(),
        None => (0..frames).collect(),
    };
    for &index in &sequence {
        idx.extend_from_slice(b"00dc");
        idx.extend_from_slice(&if keyframes[index] { 0x10u32 } else { 0u32 }.to_le_bytes());
        idx.extend_from_slice(&((chunk_positions[index] - movi_form_pos) as u32).to_le_bytes());
        idx.extend_from_slice(&(payload_lens[index] as u32).to_le_bytes());
    }
    chunk(&mut out, b"idx1", &idx);
    end_sized(&mut out, riff_size_pos);

    BuiltAvi {
        bytes: out,
        chunk_positions,
    }
}

// ---------------------------------------------------------------------------
// Seeded PRNG (splitmix64 — public-domain shape, std-only per C5) for
// the shuffle and the dribble chunking. Fixed seeds: a regression
// suite is deterministic by design; the OS-entropy dimension is the
// randomized corpus harness's job (D-11).
// ---------------------------------------------------------------------------

struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Seeded Fisher-Yates.
fn shuffle(items: &mut [usize], rng: &mut SplitMix64) {
    if items.len() < 2 {
        return;
    }
    let mut i = items.len() - 1;
    while i > 0 {
        let j = (rng.next() % (i as u64 + 1)) as usize;
        items.swap(i, j);
        i -= 1;
    }
}

/// How many idx1 entries the parser counts as misplaced for a stored
/// `order` (its own running-max rule: an entry is misplaced when its
/// chunk position precedes the highest position stored before it).
fn misplaced_entries(order: &[usize], chunk_positions: &[u64]) -> u64 {
    let mut highest: Option<u64> = None;
    let mut count = 0u64;
    for &index in order {
        let pos = chunk_positions[index];
        if highest.is_some_and(|seen| pos < seen) {
            count += 1;
        }
        highest = Some(highest.map_or(pos, |seen| seen.max(pos)));
    }
    count
}

// ---------------------------------------------------------------------------
// Demuxer drivers
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

fn demuxer_over_stream(stream: Box<dyn SourceStream>) -> AviDemuxer {
    match AviDemuxer::new(stream) {
        Ok(demuxer) => demuxer,
        Err(err) => unreachable!("demuxer over stream: {err}"),
    }
}

/// One packet's observable accounting row (the differential-equality
/// vocabulary: track, ladder ticks, keyframe, payload digest).
#[derive(Debug, Clone, PartialEq, Eq)]
struct PacketRow {
    track: u32,
    pts_ticks: i64,
    keyframe: bool,
    digest: String,
}

type DrainOutcome = (
    Vec<PacketRow>,
    Vec<(u64, &'static str)>,
    Result<(), EngineError>,
);

fn drain(avi: &mut AviDemuxer) -> DrainOutcome {
    let mut packets = Vec::new();
    let mut reasons = Vec::new();
    let terminal = loop {
        match avi.next_event() {
            Ok(DemuxEvent::Packet(packet)) => {
                let ts = match packet.pts {
                    Stamp::Present(ts) => ts,
                    Stamp::Missing => unreachable!("stamps present by construction (D1)"),
                };
                packets.push(PacketRow {
                    track: packet.track.handle(),
                    pts_ticks: ts.ticks,
                    keyframe: packet.keyframe,
                    digest: sha256_hex(packet.data.as_slice()),
                });
            }
            Ok(DemuxEvent::Recovered { skipped, reason }) => reasons.push((skipped, reason)),
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break Ok(()),
            Err(err) => break Err(err),
        }
    };
    (packets, reasons, terminal)
}

fn span_at_nanos(nanos: i64) -> TimeSpan {
    let base = match TimeBase::new(1, 1_000_000_000) {
        Ok(base) => base,
        Err(err) => unreachable!("{err}"),
    };
    TimeSpan {
        start: Stamp::Present(Timestamp::new(nanos, base)),
        end: Stamp::Missing,
    }
}

/// The landing start in ticks on the seek stream's own base.
fn landing_ticks(span: &TimeSpan) -> i64 {
    match span.start.timestamp() {
        Ok(ts) => ts.ticks,
        Err(_) => unreachable!("landing start present"),
    }
}

// ---------------------------------------------------------------------------
// Family 1 — out-of-order idx1
// ---------------------------------------------------------------------------

/// The shared family-1 spine: a canonical file and the same file with
/// its idx1 entries stored in `order` must demux identically, seek
/// identically (the absolute landing ticks `expected`, one per target
/// in [0, 20, 50, 100, 150, 200] ms), and reanchor to the canonical
/// tail. Returns the disordered demuxer after the walk (exhausted) for
/// follow-up probes.
fn assert_order_invariant(
    name: &str,
    frames: usize,
    keyframes: &[bool],
    order: &[usize],
    misplaced: u64,
    expected_ticks: &[i64],
) -> AviDemuxer {
    let canonical = build_avi(frames, keyframes, None, None);
    let disordered = build_avi(frames, keyframes, Some(order), None);

    // The movi walk is the packet-extraction truth.
    let mut plain = demuxer_over_bytes(canonical.bytes);
    match plain.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1, "{name}: one video track"),
        Err(err) => unreachable!("{name}: canonical start: {err}"),
    }
    let (plain_rows, plain_reasons, plain_terminal) = drain(&mut plain);
    assert!(plain_terminal.is_ok(), "{name}: canonical drains to EOS");
    assert!(plain_reasons.is_empty(), "{name}: canonical is clean");
    assert_eq!(plain_rows.len(), frames, "{name}: every frame walks");

    let mut avi = demuxer_over_bytes(disordered.bytes.clone());
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1, "{name}: one video track"),
        Err(err) => unreachable!("{name}: disordered start: {err}"),
    }
    assert_eq!(
        avi.seekability(),
        SeekSupport::Fast { indexed: true },
        "{name}: the disordered index is still a real index"
    );
    let (rows, reasons, terminal) = drain(&mut avi);
    assert!(terminal.is_ok(), "{name}: disordered drains to EOS");
    assert_eq!(
        rows, plain_rows,
        "{name}: walk rows are index-order-invariant"
    );

    // The reorder is damage-counted, never silent: one event, the byte
    // total carrying the misplaced-entry count (16 bytes each).
    assert_eq!(
        reasons,
        vec![(misplaced * 16, "idx1-out-of-order-entries")],
        "{name}: the reorder is observable"
    );
    assert_eq!(avi.damage_total(), misplaced * 16, "{name}: damage total");

    // Seek: every in-range target lands on the same keyframe the
    // canonical index lands on — pinned absolutely (the semantics)
    // and differentially (the invariance).
    for (index, target_ms) in [0, 20, 50, 100, 150, 200].into_iter().enumerate() {
        let want = match plain.seek(span_at_nanos(target_ms * 1_000_000)) {
            Ok(span) => landing_ticks(&span),
            Err(err) => unreachable!("{name}: canonical seek {target_ms}ms: {err}"),
        };
        assert_eq!(
            want, expected_ticks[index],
            "{name}: canonical seek {target_ms}ms lands on the recorded keyframe"
        );
        let got = match avi.seek(span_at_nanos(target_ms * 1_000_000)) {
            Ok(span) => landing_ticks(&span),
            Err(err) => unreachable!("{name}: disordered seek {target_ms}ms: {err}"),
        };
        assert_eq!(got, want, "{name}: seek {target_ms}ms lands identically");
    }

    // Reanchor from the exhausted stream (the revive path): the
    // resumed packets are exactly the canonical tail from the landing.
    let landing = match avi.reanchor(span_at_nanos(100_000_000)) {
        Ok(span) => span,
        Err(err) => unreachable!("{name}: reanchor 100ms: {err}"),
    };
    assert_eq!(
        landing_ticks(&landing),
        3,
        "{name}: reanchor lands on tick 3"
    );
    let (resumed, resumed_reasons, resumed_terminal) = drain(&mut avi);
    assert!(
        resumed_terminal.is_ok(),
        "{name}: resumed stream reaches EOS"
    );
    assert!(resumed_reasons.is_empty(), "{name}: no re-surfaced damage");
    assert_eq!(
        resumed,
        plain_rows[3..],
        "{name}: the resumed stream is the canonical tail"
    );
    avi
}

#[test]
fn shuffled_idx1_walks_and_seeks_like_the_canonical_file() {
    const FRAMES: usize = 8;
    // Keyframe pattern with non-keyframes inside: [K, K, n, K, n, K, K, n].
    let keyframes = [true, true, false, true, false, true, true, false];
    let mut rng = SplitMix64(0xD15E_A5E5_5EED_0042);
    let mut order: Vec<usize> = (0..FRAMES).collect();
    shuffle(&mut order, &mut rng);
    let moved = (0..FRAMES).filter(|&index| order[index] != index).count();
    assert!(
        moved >= FRAMES / 2,
        "the seeded shuffle must actually disorder ({moved} of {FRAMES} moved): {order:?}"
    );
    let disordered = build_avi(FRAMES, &keyframes, Some(&order), None);
    let misplaced = misplaced_entries(&order, &disordered.chunk_positions);
    assert!(
        misplaced > 0,
        "the shuffle must misplace entries: {order:?}"
    );
    // Absolute landing ticks on the 1/30 base: 0ms→f0, 20ms→tick 1→f1,
    // 50ms→tick 2→f1 (f2 is not a keyframe), 100ms→f3, 150ms→tick 5→f5,
    // 200ms→f6.
    let _ = assert_order_invariant(
        "shuffled",
        FRAMES,
        &keyframes,
        &order,
        misplaced,
        &[0, 1, 1, 3, 5, 6],
    );
}

#[test]
fn fully_reversed_idx1_walks_and_seeks_like_the_canonical_file() {
    const FRAMES: usize = 8;
    let keyframes = [true, true, false, true, false, true, true, false];
    let order: Vec<usize> = (0..FRAMES).rev().collect();
    // Every entry after the first is stored before its file-order
    // predecessor: FRAMES - 1 misplaced entries.
    assert_order_invariant(
        "reversed",
        FRAMES,
        &keyframes,
        &order,
        FRAMES as u64 - 1,
        &[0, 1, 1, 3, 5, 6],
    );
}

// ---------------------------------------------------------------------------
// Family 2 — non-monotone seek ladders (time-backwards entries)
// ---------------------------------------------------------------------------

#[test]
fn search_index_on_time_backwards_entries_stays_keyframe_safe() {
    // A table no parser path can produce anymore (parse assigns pts in
    // file order, by construction): hand-built time-backwards entries
    // exercise the search's own defense-in-depth contract — typed,
    // keyframe-safe, terminating. The early-break may return a
    // suboptimal landing on such a table (documented degraded
    // behavior); it must never return one ABOVE the target.
    let entry = |chunk_pos, size, stream, keyframe, pts| IndexEntry {
        chunk_pos,
        size,
        stream,
        keyframe,
        pts_ticks: pts,
    };
    let table = IndexTable {
        entries: vec![
            entry(100, 10, 0, true, 0),
            entry(200, 10, 0, true, 5),
            entry(300, 10, 0, true, 2), // time-backwards
            entry(400, 10, 0, true, 8),
            entry(500, 10, 0, false, 1), // non-keyframe below everything
            entry(600, 10, 1, true, 4),  // another stream's ladder
        ],
    };
    for target in [-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 100] {
        if let Some(landing) = search_index(&table, 0, target) {
            assert!(
                landing.pts_ticks <= target,
                "target {target}: keyframe-safe at-or-below, got {}",
                landing.pts_ticks
            );
            assert_ne!(
                landing.chunk_pos, 500,
                "target {target}: never a non-keyframe entry"
            );
        }
    }
    // The early-break on the backwards run: target 3 stops at the pts-5
    // entry and answers the pts-0 keyframe (degraded, still safe).
    assert_eq!(
        search_index(&table, 0, 3).map(|landing| landing.pts_ticks),
        Some(0)
    );
    // Past the run: the later keyframes still answer.
    assert_eq!(
        search_index(&table, 0, 8).map(|landing| landing.pts_ticks),
        Some(8)
    );
    // Streams answer from their own ladders only.
    assert_eq!(
        search_index(&table, 1, 4).map(|landing| (landing.pts_ticks, landing.chunk_pos)),
        Some((4, 600))
    );
    assert!(search_index(&table, 1, 3).is_none());
}

#[test]
fn mid_table_reversed_offset_run_seeks_and_reanchors_typed() {
    // Offsets violating monotonicity MID-table (a partially rewritten
    // index): entries for chunks 0,1,2,5,4,3,6,7 — the reversed run
    // 5,4,3 is the corruption; everything else is intact.
    const FRAMES: usize = 8;
    let keyframes = [true, true, false, true, false, true, true, false];
    let order = [0, 1, 2, 5, 4, 3, 6, 7];
    let disordered = build_avi(FRAMES, &keyframes, Some(&order), None);
    let misplaced = misplaced_entries(&order, &disordered.chunk_positions);
    assert_eq!(misplaced, 2, "the reversed run's 5 and 3 are misplaced");
    assert_order_invariant(
        "mid-run",
        FRAMES,
        &keyframes,
        &order,
        misplaced,
        &[0, 1, 1, 3, 5, 6],
    );
}

#[test]
fn duplicate_offset_entries_seek_typed_and_never_loop() {
    // Duplicated dwOffsets stored with an out-of-order run (entries
    // [0, 1, 3, 1, 2, 3] over 4 frames): the walk is positional truth
    // (no packet is ever duplicated), the seek answers typed, and a
    // re-anchored stream terminates at EOS with strictly
    // non-decreasing per-stream time. The landing SPAN may diverge
    // from the actual resume pts here — the documented lying-index
    // divergence (bounded, observable) — so this test pins the safety
    // properties, not exactness. (Monotone-stored duplicates —
    // [0, 1, 1, 2, 3, 3] — currently surface NO damage note at all;
    // registered as a hardening candidate in the W1-15 worklog.)
    const FRAMES: usize = 4;
    let keyframes = [true, true, true, true];
    let order = [0, 1, 3, 1, 2, 3];
    let canonical = build_avi(FRAMES, &keyframes, None, None);
    let duplicated = build_avi(FRAMES, &keyframes, Some(&order), None);

    let mut plain = demuxer_over_bytes(canonical.bytes);
    match plain.start() {
        Ok(_) => {}
        Err(err) => unreachable!("canonical start: {err}"),
    }
    let (plain_rows, _, plain_terminal) = drain(&mut plain);
    assert!(plain_terminal.is_ok());

    let mut avi = demuxer_over_bytes(duplicated.bytes);
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 1),
        Err(err) => unreachable!("duplicated entries are salvage, not an error: {err}"),
    }
    assert_eq!(avi.seekability(), SeekSupport::Fast { indexed: true });
    let (rows, reasons, terminal) = drain(&mut avi);
    assert!(terminal.is_ok(), "the walk terminates at EOS");
    assert_eq!(
        rows, plain_rows,
        "duplicate index entries never duplicate packets"
    );
    assert_eq!(
        reasons,
        vec![(2 * 16, "idx1-out-of-order-entries")],
        "the out-of-order duplicate run is counted"
    );

    // Seek grid: typed answers, keyframe-safe.
    for target_ms in [0, 20, 50, 100] {
        match avi.seek(span_at_nanos(target_ms * 1_000_000)) {
            Ok(span) => {
                let ticks = landing_ticks(&span);
                let target_ticks = ((target_ms * 30) + 999) / 1000;
                assert!(ticks <= target_ticks, "seek {target_ms}ms: at-or-below");
            }
            Err(err) => unreachable!("in-range seek {target_ms}ms must answer: {err}"),
        }
    }

    // Reanchor mid-file and drain: terminates, time never runs
    // backwards, every payload is a real frame.
    let landing = match avi.reanchor(span_at_nanos(50_000_000)) {
        Ok(span) => span,
        Err(err) => unreachable!("reanchor over duplicated entries: {err}"),
    };
    assert!(landing_ticks(&landing) <= 2, "landing at-or-below tick 2");
    let (resumed, _, resumed_terminal) = drain(&mut avi);
    assert!(
        resumed_terminal.is_ok(),
        "resumed stream reaches EOS (no loop)"
    );
    let canonical_digests: Vec<String> = plain_rows.iter().map(|row| row.digest.clone()).collect();
    let mut last_ticks = i64::MIN;
    for row in &resumed {
        assert!(
            row.pts_ticks >= last_ticks,
            "per-stream time never runs backwards: {row:?} after {last_ticks}"
        );
        last_ticks = row.pts_ticks;
        assert!(
            canonical_digests.contains(&row.digest),
            "every resumed payload is a real frame: {}",
            row.digest
        );
    }
    assert!(resumed.len() <= FRAMES, "no packet duplication post-seek");
}

// ---------------------------------------------------------------------------
// Family 3 — dribble I/O (short reads are data, not errors)
// ---------------------------------------------------------------------------

/// The I/O side-effect ledger (shared with the test through an
/// `Arc<Mutex>` — the stream itself moves into the demuxer).
#[derive(Default)]
struct IoStats {
    reads: u64,
    /// Calls that returned at least one byte.
    payload_reads: u64,
    bytes_served: u64,
    /// Calls that returned empty (legal only at end of stream).
    empty_reads: u64,
    seeks: u64,
}

/// A seekable source whose `read` returns a seeded-random 1..=cap
/// bytes per call — the dirty-world I/O shape (network stacks, slow
/// disks, buffered pipes). `Ok(0)` happens only at true end of stream
/// (the `SourceStream` contract); seeks are exact.
struct DribbleStream {
    uri: SourceUri,
    data: Arc<[u8]>,
    pos: u64,
    rng: SplitMix64,
    cap: usize,
    stats: Arc<Mutex<IoStats>>,
}

impl DribbleStream {
    fn new(data: Arc<[u8]>, seed: u64, cap: usize, stats: Arc<Mutex<IoStats>>) -> DribbleStream {
        let uri = match SourceUri::parse("memory://lumen-w1-15-dribble") {
            Ok(uri) => uri,
            Err(err) => unreachable!("fixed memory URI: {err}"),
        };
        DribbleStream {
            uri,
            data,
            pos: 0,
            rng: SplitMix64(seed),
            cap: cap.max(1),
            stats,
        }
    }
}

impl SourceStream for DribbleStream {
    fn uri(&self) -> &SourceUri {
        &self.uri
    }

    fn size(&self) -> Option<u64> {
        Some(self.data.len() as u64)
    }

    fn supports_seek(&self) -> bool {
        true
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let avail = self.data.len() as u64 - self.pos.min(self.data.len() as u64);
        let take = if len == 0 || avail == 0 {
            0
        } else {
            // 1..=bound: never zero mid-stream, never more than asked.
            let bound = (len as u64).min(avail).min(self.cap as u64);
            1 + self.rng.next() % bound
        };
        let start = self.pos as usize;
        let out = self.data[start..start + take as usize].to_vec();
        self.pos += take;
        let mut stats = match self.stats.lock() {
            Ok(stats) => stats,
            Err(poisoned) => poisoned.into_inner(),
        };
        stats.reads += 1;
        if take == 0 {
            stats.empty_reads += 1;
        } else {
            stats.payload_reads += 1;
            stats.bytes_served += take;
        }
        Ok(out)
    }

    fn seek(&mut self, origin: SeekOrigin) -> Result<SourcePosition, EngineError> {
        let target = match origin {
            SeekOrigin::Start(to) => to,
            SeekOrigin::Current(delta) => {
                let base = i128::from(self.pos);
                let sum = base + i128::from(delta);
                if sum < 0 || sum > i128::from(u64::MAX) {
                    return Err(out_of_range());
                }
                sum as u64
            }
        };
        if target > self.data.len() as u64 {
            return Err(out_of_range());
        }
        self.pos = target;
        let mut stats = match self.stats.lock() {
            Ok(stats) => stats,
            Err(poisoned) => poisoned.into_inner(),
        };
        stats.seeks += 1;
        Ok(SourcePosition(target))
    }

    fn position(&self) -> SourcePosition {
        SourcePosition(self.pos)
    }
}

/// Typed out-of-range seek for [`DribbleStream`].
fn out_of_range() -> EngineError {
    EngineError::new(
        PipelineStage::Open,
        ErrorKind::Inaccessible {
            reason: media_types::error::AccessDenied::NotFound,
        },
        media_types::error::ErrorContext::new("demux.test.dribble.seek-out-of-range"),
    )
}

fn stats_of(stats: &Arc<Mutex<IoStats>>) -> IoStats {
    match stats.lock() {
        Ok(stats) => IoStats {
            reads: stats.reads,
            payload_reads: stats.payload_reads,
            bytes_served: stats.bytes_served,
            empty_reads: stats.empty_reads,
            seeks: stats.seeks,
        },
        Err(_) => unreachable!("stats mutex never panics"),
    }
}

/// The no-read-loop-pathology assertions (family 3's second half):
/// every non-empty read advanced the position, EOF was probed a
/// bounded number of times, and the total bytes moved stay linear in
/// the file size (a per-packet rescan would blow the ceiling).
fn assert_no_read_loop_pathology(name: &str, stats: &IoStats, file_len: u64) {
    assert!(
        stats.payload_reads <= stats.bytes_served,
        "{name}: every non-empty read serves at least one byte ({} reads, {} bytes)",
        stats.payload_reads,
        stats.bytes_served
    );
    assert!(
        stats.empty_reads <= 64,
        "{name}: no end-of-stream spin ({} empty reads)",
        stats.empty_reads
    );
    assert!(
        stats.bytes_served <= 4 * file_len + 65_536,
        "{name}: bytes served stay linear ({} bytes for a {}-byte file)",
        stats.bytes_served,
        file_len
    );
}

#[test]
fn dribbled_parent_walk_matches_the_plain_walk() {
    // The REAL parent fixture (externally generated by the xtask
    // fixture writer, not this suite) driven to EOS through a dribbling
    // source: packet accounting — track, ladder ticks, keyframe flags,
    // per-packet sha256 — must be IDENTICAL to the non-dribble walk of
    // the same bytes. Two runs: a seeded 1..=16-byte dribble, and the
    // degenerate 1-byte dribble (maximal short-read stress).
    let bytes = testkit::fixture_bytes(PARENT).must();
    let file_len = bytes.len() as u64;
    let expect = parse_expectation(PARENT_EXPECTATION);

    let mut plain = demuxer_over_bytes(bytes.clone());
    let tracks = match plain.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("plain parent start: {err}"),
    };
    assert_eq!(tracks.len() as u64, expect.track_count);
    let (plain_rows, plain_reasons, plain_terminal) = drain(&mut plain);
    assert!(plain_terminal.is_ok(), "the plain parent walk reaches EOS");
    assert!(plain_reasons.is_empty(), "the parent is a clean container");
    assert_eq!(
        plain_rows.len() as u64,
        expect.video_sample_count * 2,
        "60 video frames + 60 audio ticks"
    );
    assert_eq!(plain.damage_total(), 0);

    for (seed, cap, name) in [
        (0xD15E_A5E5_0000_0001u64, 16usize, "dribble-16"),
        (0x5EED_0042_0000_0002, 1, "dribble-1"),
    ] {
        let stats = Arc::new(Mutex::new(IoStats::default()));
        let stream = DribbleStream::new(Arc::from(bytes.clone()), seed, cap, Arc::clone(&stats));
        let mut avi = demuxer_over_stream(Box::new(stream));
        let tracks = match avi.start() {
            Ok(tracks) => tracks,
            Err(err) => unreachable!("{name}: start through the dribble: {err}"),
        };
        assert_eq!(tracks.len() as u64, expect.track_count, "{name}");
        let (rows, reasons, terminal) = drain(&mut avi);
        assert!(terminal.is_ok(), "{name}: the dribbled walk reaches EOS");
        assert_eq!(
            rows, plain_rows,
            "{name}: short reads must not change semantics"
        );
        assert_eq!(reasons, plain_reasons, "{name}: no invented damage");
        assert_eq!(avi.damage_total(), 0, "{name}");
        assert_no_read_loop_pathology(name, &stats_of(&stats), file_len);
    }
}

#[test]
fn dribbled_parent_reanchor_revives_and_resumes_the_canonical_tail() {
    // The acting half of a seek through the dribble: reanchor to 1 s
    // (video tick 30) after exhaustion, then drain — the resumed
    // packets must be exactly the from-start tail from the landing
    // (the ladder replay is exact for a truthful index), all through
    // 1..=16-byte reads.
    let bytes = testkit::fixture_bytes(PARENT).must();
    let file_len = bytes.len() as u64;

    let mut plain = demuxer_over_bytes(bytes.clone());
    match plain.start() {
        Ok(_) => {}
        Err(err) => unreachable!("plain parent start: {err}"),
    }
    let (plain_rows, _, plain_terminal) = drain(&mut plain);
    assert!(plain_terminal.is_ok());

    let stats = Arc::new(Mutex::new(IoStats::default()));
    let stream = DribbleStream::new(Arc::from(bytes), 0xC0FF_EE00_0000_0003, 16, stats.clone());
    let mut avi = demuxer_over_stream(Box::new(stream));
    match avi.start() {
        Ok(_) => {}
        Err(err) => unreachable!("dribbled parent start: {err}"),
    }
    let (_, _, terminal) = drain(&mut avi);
    assert!(terminal.is_ok(), "exhaust the stream first");

    let landing = match avi.reanchor(span_at_nanos(1_000_000_000)) {
        Ok(span) => span,
        Err(err) => unreachable!("dribbled reanchor to 1s: {err}"),
    };
    assert_eq!(landing_ticks(&landing), 30, "lands on video tick 30");
    let (resumed, _, resumed_terminal) = drain(&mut avi);
    assert!(resumed_terminal.is_ok(), "the revived stream reaches EOS");
    assert_eq!(
        resumed,
        plain_rows[60..],
        "the resumed stream is the from-start tail from the landing"
    );
    assert_no_read_loop_pathology("dribble-reanchor", &stats_of(&stats), file_len);
}

#[test]
fn dribbled_no_index_scan_seeks_and_lands_within_tolerance() {
    // The linear-scan seek fallback (no idx1) through the dribble: the
    // scan peeks its way across the movi with seek+read+restore per
    // chunk header — the position-restoration logic under sustained
    // short reads. Every recorded target lands within the recorded
    // tolerance, at-or-below the target, and the subsequent walk is
    // the full 120-packet drain.
    let bytes = testkit::fixture_bytes(NOIDX0).must();
    let file_len = bytes.len() as u64;
    let expect = parse_expectation(PARENT_EXPECTATION);
    assert!(
        !expect.seek_targets_us.is_empty(),
        "the parent records a seek grid"
    );

    let stats = Arc::new(Mutex::new(IoStats::default()));
    let stream = DribbleStream::new(Arc::from(bytes), 0xBADF_00D0_0000_0004, 16, stats.clone());
    let mut avi = demuxer_over_stream(Box::new(stream));
    match avi.start() {
        Ok(tracks) => assert_eq!(tracks.len(), 2),
        Err(err) => unreachable!("noidx0 starts through the dribble: {err}"),
    };
    assert_eq!(avi.seekability(), SeekSupport::SlowLinear);

    let nanos = match TimeBase::new(1, 1_000_000_000) {
        Ok(base) => base,
        Err(err) => unreachable!("{err}"),
    };
    for target_us in &expect.seek_targets_us {
        let target = TimeSpan {
            start: Stamp::Present(Timestamp::new(*target_us as i64 * 1_000, nanos)),
            end: Stamp::Missing,
        };
        let landing = match avi.seek(target) {
            Ok(span) => span,
            Err(err) => unreachable!("dribbled linear seek {target_us}µs: {err}"),
        };
        let landed_us = match landing.start.timestamp() {
            Ok(ts) => match ts.as_nanos() {
                Ok(ns) => ns.0 / 1_000,
                Err(err) => unreachable!("{err}"),
            },
            Err(_) => unreachable!("landing start present"),
        };
        let delta = landed_us.abs_diff(*target_us);
        assert!(
            delta <= expect.seek_tolerance_us,
            "dribbled seek {target_us}µs landed at {landed_us}µs (delta {delta}µs > tolerance {}µs)",
            expect.seek_tolerance_us
        );
        assert!(landed_us <= *target_us, "keyframe-safe at-or-below");
    }

    let (rows, reasons, terminal) = drain(&mut avi);
    assert!(terminal.is_ok());
    assert_eq!(rows.len(), 120, "the full noidx0 walk after the scans");
    assert!(reasons.is_empty());
    assert_no_read_loop_pathology("dribble-scan", &stats_of(&stats), file_len);
}

// ---------------------------------------------------------------------------
// Family 4 — wrong container (the B5 probe-vs-demux race)
// ---------------------------------------------------------------------------

#[test]
fn mp4_signatured_bytes_refuse_typed_in_the_avi_path() {
    // The committed MP4 parent fixture — the bytes media-probe's MP4
    // signature claims at full score — handed to the AVI demuxer path
    // (the misroute a lying extension or a probe/host disagreement
    // produces). The refusal is typed `not-avi` carrying the sniffed
    // prefix (the probe's own anchor bytes), sticky on every replay,
    // never a panic, never a packet.
    let bytes = testkit::fixture_bytes(MP4_PARENT).must();
    assert!(bytes.len() > 12, "the mp4 parent is a real container");
    let mut avi = demuxer_over_bytes(bytes.clone());
    let err = match avi.start() {
        Ok(_) => unreachable!("mp4 bytes are not an AVI"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.not-avi");
    assert_eq!(err.stage, PipelineStage::Demux);
    match err.kind {
        ErrorKind::Unsupported { format } => {
            let info = match format {
                Some(info) => info,
                None => unreachable!("the refusal names the format"),
            };
            assert_eq!(info.name.as_deref(), Some("avi"));
            assert_eq!(
                info.sniffed,
                bytes[..12],
                "the sniffed prefix is the mp4 ftyp anchor"
            );
        }
        other => unreachable!("unsupported flavor, got {other:?}"),
    }
    // Sticky replays: the same typed error, never a panic.
    let replay = match avi.start() {
        Ok(_) => unreachable!("sticky"),
        Err(err) => err,
    };
    assert_eq!(replay.context.code, "demux.avi.not-avi");
    let event = match avi.next_event() {
        Ok(_) => unreachable!("sticky"),
        Err(err) => err,
    };
    assert_eq!(event.context.code, "demux.avi.not-avi");
    assert_eq!(avi.seekability(), SeekSupport::None);
}

#[test]
fn avi_bytes_behind_a_lying_mp4_name_demux_fully() {
    // The mirror race: AVI bytes behind a `.mp4` name. Content truth
    // decides — the access stack and the demuxer never consult the
    // extension, so the file demuxes fully (the wrong-container race
    // cannot cause a mis-walk at this layer).
    let bytes = testkit::fixture_bytes(PARENT).must();
    let expect = parse_expectation(PARENT_EXPECTATION);
    let path = std::env::temp_dir().join("lumen-w1-15-wrong-name.mp4");
    std::fs::write(&path, &bytes).must();
    let stream = open_via_input(&path).must();
    let mut avi = demuxer_over_stream(stream);
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("content, not the name, decides: {err}"),
    };
    assert_eq!(tracks.len() as u64, expect.track_count);
    let (rows, reasons, terminal) = drain(&mut avi);
    assert!(terminal.is_ok());
    assert_eq!(
        rows.len() as u64,
        expect.video_sample_count * 2,
        "the full parent walk behind the lying name"
    );
    assert!(reasons.is_empty());
    assert_eq!(avi.damage_total(), 0);
    std::fs::remove_file(&path).ok();
}

#[test]
fn ftyp_magic_with_an_embedded_avi_body_is_refused_not_rescued() {
    // The adversarial hybrid: a well-formed ftyp box at 0 with a
    // complete valid AVI appended after it. The AVI demuxer anchors
    // magic at 0 and refuses typed — scanning ahead to "rescue" the
    // embedded AVI would be a speculative mis-walk (the walk would
    // misattribute offsets and pads to a structure the file denies).
    let canonical = build_avi(4, &[true; 4], None, None);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&24u32.to_be_bytes()); // box size (BE)
    bytes.extend_from_slice(b"ftyp");
    bytes.extend_from_slice(b"isom");
    bytes.extend_from_slice(&[0, 0, 0, 0]); // version + flags
    bytes.extend_from_slice(b"isom");
    bytes.extend_from_slice(b"mp41");
    assert_eq!(bytes.len(), 24, "a well-formed ftyp box");
    bytes.extend_from_slice(&canonical.bytes);

    let mut avi = demuxer_over_bytes(bytes.clone());
    let err = match avi.start() {
        Ok(_) => unreachable!("an ftyp anchor is not an AVI"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.not-avi");
    match err.kind {
        ErrorKind::Unsupported { format } => {
            let info = match format {
                Some(info) => info,
                None => unreachable!("format info present"),
            };
            assert_eq!(info.name.as_deref(), Some("avi"));
            assert_eq!(
                info.sniffed,
                bytes[..12],
                "the sniffed prefix names the actual leading bytes"
            );
        }
        other => unreachable!("unsupported flavor, got {other:?}"),
    }
    // And the refusal is sticky, never a mid-walk surprise.
    let replay = match avi.start() {
        Ok(_) => unreachable!("sticky"),
        Err(err) => err,
    };
    assert_eq!(replay.context.code, "demux.avi.not-avi");
}

#[test]
fn avi_with_mp4_bytes_as_junk_payload_demuxes_as_avi() {
    // The other hybrid: valid AVI magic with mp4-flavored bytes inside
    // a top-level JUNK chunk. Those bytes are opaque payload — the
    // container claim is the RIFF magic, and the walk delivers every
    // real frame with the mp4 bytes untouched inside the skipped JUNK.
    let canonical = build_avi(4, &[true; 4], None, None);
    let mut junk = Vec::new();
    junk.extend_from_slice(&24u32.to_be_bytes());
    junk.extend_from_slice(b"ftyp");
    junk.extend_from_slice(b"isom");
    junk.extend_from_slice(&[0, 0, 0, 0]);
    junk.extend_from_slice(b"isom");
    junk.extend_from_slice(b"mp41");
    junk.extend_from_slice(b"moov");
    junk.extend_from_slice(&96u32.to_be_bytes());
    junk.extend(&[0xEE; 88]);
    let hybrid = build_avi(4, &[true; 4], None, Some(&junk));

    let mut plain = demuxer_over_bytes(canonical.bytes);
    match plain.start() {
        Ok(_) => {}
        Err(err) => unreachable!("canonical start: {err}"),
    }
    let (plain_rows, _, plain_terminal) = drain(&mut plain);
    assert!(plain_terminal.is_ok());

    let mut avi = demuxer_over_bytes(hybrid.bytes);
    let tracks = match avi.start() {
        Ok(tracks) => tracks,
        Err(err) => unreachable!("mp4 bytes as JUNK payload are still an AVI: {err}"),
    };
    assert_eq!(tracks.len(), 1);
    let (rows, reasons, terminal) = drain(&mut avi);
    assert!(terminal.is_ok());
    assert_eq!(
        rows, plain_rows,
        "the walk is byte-identical to the canonical"
    );
    assert!(
        reasons.is_empty(),
        "a top-level JUNK chunk is a silent skip, not damage"
    );
    assert_eq!(avi.damage_total(), 0);
}

#[test]
fn probe_hit_demux_refuse_disagreement_is_typed() {
    // The true disagreement shape: the AVI signature anchors on
    // RIFF + `AVI ` magic alone (score 70), while the demuxer needs
    // real structure. Magic followed by garbage is a typed demux-stage
    // refusal (never a panic) — the host surfaces it as a demux error,
    // exactly the probe-vs-demux split B5 names.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    let riff_size_pos = bytes.len();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(b"AVI ");
    bytes.extend(&[0xFFu8; 512]);
    end_sized(&mut bytes, riff_size_pos);
    let mut avi = demuxer_over_bytes(bytes);
    let err = match avi.start() {
        Ok(_) => unreachable!("magic without structure cannot start"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.avi.no-header");
    assert!(matches!(err.kind, ErrorKind::Corrupt { .. }));
    assert_eq!(err.stage, PipelineStage::Demux);

    // Wrong RIFF forms — a WAVE file, an RIFX file — refuse as
    // not-avi (probe and demux agree there; the demuxer's magic check
    // is the wrong-container backstop whatever the probe said).
    for (magic, form) in [(b"RIFF", b"WAVE"), (b"RIFX", b"AVI ")] {
        let mut foreign = Vec::new();
        foreign.extend_from_slice(magic);
        let size_pos = foreign.len();
        foreign.extend_from_slice(&0u32.to_le_bytes());
        foreign.extend_from_slice(form);
        foreign.extend(&[0x11u8; 64]);
        end_sized(&mut foreign, size_pos);
        let mut avi = demuxer_over_bytes(foreign);
        let err = match avi.start() {
            Ok(_) => unreachable!("{} must not start as AVI", String::from_utf8_lossy(form)),
            Err(err) => err,
        };
        assert_eq!(err.context.code, "demux.avi.not-avi");
        assert!(matches!(err.kind, ErrorKind::Unsupported { .. }));
    }
}
