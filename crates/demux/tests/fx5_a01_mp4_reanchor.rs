//! FX5-A01 (FIX-PLAN §3 F1.3) — the MP4 re-anchor, the acting half of
//! a seek (wave-05's S0, audit F-001).
//!
//! The defect: `Mp4Demuxer` advertised `SeekSupport::Fast` while the
//! engine's only seek entry — the trait's `reanchor` — hit the
//! default typed refusal `plugin.demux.reanchor.unsupported`: every
//! in-session MP4 seek failed (`TimingSeekFailed`, wave-05's executed
//! F-001), and the open-time `--start-time` clamp's `seek()` machinery
//! was the only code that ever moved the position. Post-fix these
//! suites pin the landed half against the same in-tree behavioral
//! models the reference demuxers pin: the landing + continuation
//! semantics of AVI's
//! `reanchor_moves_the_stream_and_packets_continue_from_the_landing`,
//! the pending-flush / event re-prime of WAV and AVI, the typed
//! out-of-range refusal with D4 position preservation, and the two
//! shapes the fix itself exists for — the progressive emission-cursor
//! rewind (audit F-004: a backward landing silently dropped samples)
//! and the from-start fragment scan (a backward landing previously
//! scanned from the walk position and refused).
//!
//! Fixtures come from `mp4::test_support` (the in-crate byte builders:
//! progressive tables and a 4-moof fragmented file, both with sync
//! flags at samples 0 and 4).

use std::sync::Arc;

use demux::mp4::Mp4Demuxer;
use demux::mp4::test_support::{
    SampleSpec, TrackSpec, fragmented, mem_stream, progressive, progressive_with_cover,
};
use media_types::time::{Stamp, TimeSpan, Timestamp};
use plugin_api::{DemuxEvent, Demuxer};

/// The shared video shape: `n` samples × `dur` ticks on the 9000
/// media timescale, syncs at samples 0 and 4 (all-sync when `n` ≤ 4).
fn video_track(n: usize, dur: u64) -> TrackSpec {
    TrackSpec {
        handler: *b"vide",
        entry: *b"avc1",
        timescale: 9000,
        edit: None,
        chap_ref: None,
        samples: (0..n)
            .map(|i| SampleSpec {
                bytes: vec![0x40 + i as u8; 4],
                dur,
                sync: i == 0 || i == 4 || n <= 4,
                cts: 0,
            })
            .collect(),
    }
}

/// A target span at `nanos` (the engine's NANO_BASE seek shape —
/// what `pipeline_seek` hands the demuxer).
fn span_at(nanos: u64) -> TimeSpan {
    TimeSpan {
        start: Stamp::Present(
            Timestamp::from_parts(nanos as i64, 1, 1_000_000_000)
                .unwrap_or_else(|e| unreachable!("ts: {e}")),
        ),
        end: Stamp::Missing,
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

/// The landing span's start tick (in the reference track's base).
fn landing_ticks(span: &TimeSpan) -> i64 {
    match span.start.timestamp() {
        Ok(ts) => ts.ticks,
        Err(_) => unreachable!("landing start present"),
    }
}

/// Drain to EOS: `(track handle, pts ticks, keyframe)` per packet,
/// plus the count of `Recovered` damage records seen.
fn drain(mp4: &mut Mp4Demuxer) -> (Vec<(u32, i64, bool)>, usize) {
    let mut out = Vec::new();
    let mut recovered = 0usize;
    let mut events = 0usize;
    loop {
        events += 1;
        assert!(events < 256, "event budget");
        match mp4
            .next_event()
            .unwrap_or_else(|e| unreachable!("event: {e}"))
        {
            DemuxEvent::Packet(p) => {
                let ticks = match p.pts {
                    Stamp::Present(ts) => ts.ticks,
                    Stamp::Missing => unreachable!("pts present"),
                };
                out.push((p.track.handle(), ticks, p.keyframe));
            }
            DemuxEvent::Recovered { .. } => recovered += 1,
            DemuxEvent::EndOfStream => break,
            DemuxEvent::Discontinuity => {}
        }
    }
    (out, recovered)
}

/// Consume exactly `count` packets (container signals ignored) —
/// the AVI reference suite's `drain_through`.
fn drain_through(mp4: &mut Mp4Demuxer, count: usize) {
    let mut got = 0usize;
    let mut events = 0usize;
    while got < count {
        events += 1;
        assert!(events < 256, "event budget");
        match mp4
            .next_event()
            .unwrap_or_else(|e| unreachable!("event: {e}"))
        {
            DemuxEvent::Packet(_) => got += 1,
            DemuxEvent::EndOfStream => unreachable!("needed {count} packets"),
            _ => {}
        }
    }
}

/// The from-start truth of a fresh drain: `(handle, pts ticks)` per
/// packet.
fn from_start_truth(file: &[u8]) -> Vec<(u32, i64)> {
    let mut mp4 = open(file.to_vec());
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let (packets, _) = drain(&mut mp4);
    packets.into_iter().map(|(h, t, _)| (h, t)).collect()
}

/// The `(handle, pts ticks)` projection of a drain.
fn pts_only(packets: Vec<(u32, i64, bool)>) -> Vec<(u32, i64)> {
    packets.into_iter().map(|(h, t, _)| (h, t)).collect()
}

/// (a) The AVI behavioral model: consume, re-anchor mid-stream, and
/// the packets continue from the landing with the from-start
/// timestamps — the landing sample itself is a keyframe.
#[test]
fn fx5_a01_progressive_reanchor_lands_on_keyframe_and_packets_continue() {
    // 6 samples × 1000 ticks @ 9000 Hz; syncs at 0 and 4.
    let file = progressive(&[video_track(6, 1000)], b"isom");
    let truth = from_start_truth(&file);
    assert_eq!(
        truth,
        vec![
            (0, 0),
            (0, 1000),
            (0, 2000),
            (0, 3000),
            (0, 4000),
            (0, 5000)
        ]
    );

    let mut mp4 = open(file.clone());
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    drain_through(&mut mp4, 2);
    // 580 ms → 5220 ticks: the last sample ≤ it is 5, and the
    // previous keyframe is sample 4 (ticks 4000).
    let landed = mp4
        .reanchor(span_at(580_000_000))
        .unwrap_or_else(|e| unreachable!("reanchor: {e}"));
    assert_eq!(
        landing_ticks(&landed),
        4000,
        "lands on the keyframe at/below the target"
    );
    let (packets, _) = drain(&mut mp4);
    assert!(
        packets.first().is_some_and(|p| p.2),
        "the landing sample is the keyframe"
    );
    assert_eq!(pts_only(packets), vec![(0, 4000), (0, 5000)], "truth tail");
}

/// (b) Re-anchor to 0 after partial play: the emission cursor rewinds
/// and the whole stream replays with the from-start timestamps (the
/// pre-fix shape dropped everything between the landing and the old
/// cursor — audit F-004).
#[test]
fn fx5_a01_reanchor_to_zero_replays_from_start() {
    let file = progressive(&[video_track(6, 1000)], b"isom");
    let truth = from_start_truth(&file);

    let mut mp4 = open(file.clone());
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    drain_through(&mut mp4, 3);
    let landed = mp4
        .reanchor(span_at(0))
        .unwrap_or_else(|e| unreachable!("reanchor: {e}"));
    assert_eq!(landing_ticks(&landed), 0);
    let (packets, _) = drain(&mut mp4);
    assert_eq!(pts_only(packets), truth, "the full from-start replay");
}

/// A re-anchor from EOS revives the exhausted stream (the WAV/AVI
/// precedent: a seek is not a one-way door).
#[test]
fn fx5_a01_reanchor_from_eof_revives_the_stream() {
    let file = progressive(&[video_track(6, 1000)], b"isom");
    let truth = from_start_truth(&file);

    let mut mp4 = open(file.clone());
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let (packets, _) = drain(&mut mp4);
    assert_eq!(pts_only(packets), truth);
    match mp4.next_event() {
        Ok(DemuxEvent::EndOfStream) => {}
        other => unreachable!("EOS after drain, got {other:?}"),
    }
    // Backward to the start: the stream plays again.
    let landed = mp4
        .reanchor(span_at(0))
        .unwrap_or_else(|e| unreachable!("reanchor from EOS: {e}"));
    assert_eq!(landing_ticks(&landed), 0);
    let (packets, _) = drain(&mut mp4);
    assert_eq!(pts_only(packets), truth, "the stream plays again");
}

/// (c) A target past the extent is the typed refusal (the WAV
/// house precedent — never a silent clamp), and D4 holds: the
/// demuxer continues from its pre-seek position with the original
/// timestamps.
#[test]
fn fx5_a01_past_end_reanchor_is_typed_and_position_survives_d4() {
    // 4 samples × 1000 ticks @ 9000 → extent 444.4 ms.
    let file = progressive(&[video_track(4, 1000)], b"isom");
    let mut mp4 = open(file.clone());
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    drain_through(&mut mp4, 1);
    let err = match mp4.reanchor(span_at(999_999_999)) {
        Ok(_) => unreachable!("past-end reanchor must refuse"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.mp4.seek.out-of-range");
    // D4: nothing moved — the remaining packets arrive with their
    // from-start timestamps.
    let (packets, _) = drain(&mut mp4);
    assert_eq!(
        pts_only(packets),
        vec![(0, 1000), (0, 2000), (0, 3000)],
        "the pre-seek position survives"
    );
}

/// At the exact extent the re-anchor lands the last keyframe (AVI's
/// `reanchor_at_end_lands_last_and_past_end_is_typed` shape); landing
/// at the end is legal — the next event is EOS.
#[test]
fn fx5_a01_at_end_reanchor_lands_the_last_keyframe() {
    let file = progressive(&[video_track(4, 1000)], b"isom");
    let mut mp4 = open(file.clone());
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    // 4000 ticks @ 9000 ≈ 444.444.444 ns: in range, at the end.
    let landed = mp4
        .reanchor(span_at(444_444_444))
        .unwrap_or_else(|e| unreachable!("at-end reanchor: {e}"));
    assert_eq!(landing_ticks(&landed), 3000, "the last keyframe");
    let (packets, _) = drain(&mut mp4);
    assert_eq!(pts_only(packets), vec![(0, 3000)]);
}

/// (d) Re-anchors in a row: forward, then backward immediately (no
/// packets consumed between), then forward again after the replay.
/// Each landing is the keyframe ≤ target and each continuation is the
/// from-start truth tail.
#[test]
fn fx5_a01_reanchor_twice_in_a_row() {
    let file = progressive(&[video_track(6, 1000)], b"isom");
    let truth = from_start_truth(&file);

    let mut mp4 = open(file.clone());
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    drain_through(&mut mp4, 1);
    // Forward: 580 ms → the keyframe at tick 4000.
    let landed = mp4
        .reanchor(span_at(580_000_000))
        .unwrap_or_else(|e| unreachable!("reanchor 1: {e}"));
    assert_eq!(landing_ticks(&landed), 4000);
    // Backward, immediately: 0 → the keyframe at tick 0.
    let landed = mp4
        .reanchor(span_at(0))
        .unwrap_or_else(|e| unreachable!("reanchor 2: {e}"));
    assert_eq!(landing_ticks(&landed), 0);
    let (packets, _) = drain(&mut mp4);
    assert_eq!(pts_only(packets), truth, "full replay from tick 0");
    // Forward once more after the full replay (EOS → revived → seek).
    let landed = mp4
        .reanchor(span_at(580_000_000))
        .unwrap_or_else(|e| unreachable!("reanchor 3: {e}"));
    assert_eq!(landing_ticks(&landed), 4000);
    let (packets, _) = drain(&mut mp4);
    assert_eq!(pts_only(packets), vec![(0, 4000), (0, 5000)]);
}

/// (e) The pending flush: pre-seek queued events die with the old
/// position. A zero-size sample damage-notes a `Recovered` record
/// that surfaces ahead of the next packet; a re-anchor drops it —
/// no stale damage replay after the seek.
#[test]
fn fx5_a01_reanchor_flushes_pending_damage_no_stale_replay() {
    let track = TrackSpec {
        handler: *b"vide",
        entry: *b"avc1",
        timescale: 9000,
        edit: None,
        chap_ref: None,
        samples: vec![
            SampleSpec {
                bytes: Vec::new(),
                dur: 1000,
                sync: true,
                cts: 0,
            },
            SampleSpec {
                bytes: vec![0x41; 4],
                dur: 1000,
                sync: false,
                cts: 0,
            },
            SampleSpec {
                bytes: vec![0x42; 4],
                dur: 1000,
                sync: false,
                cts: 0,
            },
            SampleSpec {
                bytes: vec![0x43; 4],
                dur: 1000,
                sync: false,
                cts: 0,
            },
            SampleSpec {
                bytes: vec![0x44; 4],
                dur: 1000,
                sync: true,
                cts: 0,
            },
            SampleSpec {
                bytes: vec![0x45; 4],
                dur: 1000,
                sync: false,
                cts: 0,
            },
        ],
    };
    let file = progressive(&[track], b"isom");

    // Control (a fresh walk over the same bytes): after the first
    // packet the damage record is the next event — the queue shape
    // this test re-anchors through.
    {
        let mut probe = open(file.clone());
        probe.start().unwrap_or_else(|e| unreachable!("start: {e}"));
        match probe.next_event() {
            Ok(DemuxEvent::Packet(p)) => match p.pts {
                Stamp::Present(ts) => assert_eq!(ts.ticks, 1000),
                Stamp::Missing => unreachable!("pts present"),
            },
            other => unreachable!("first packet, got {other:?}"),
        }
        match probe.next_event() {
            Ok(DemuxEvent::Recovered { .. }) => {}
            other => unreachable!("damage record, got {other:?}"),
        }
    }

    // The real instance: one packet consumed leaves the damage record
    // in the pending queue; the re-anchor must drop it.
    let mut mp4 = open(file.clone());
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    match mp4.next_event() {
        Ok(DemuxEvent::Packet(p)) => match p.pts {
            Stamp::Present(ts) => assert_eq!(ts.ticks, 1000),
            Stamp::Missing => unreachable!("pts present"),
        },
        other => unreachable!("first packet, got {other:?}"),
    }
    let landed = mp4
        .reanchor(span_at(580_000_000))
        .unwrap_or_else(|e| unreachable!("reanchor: {e}"));
    assert_eq!(landing_ticks(&landed), 4000);
    let (packets, recovered) = drain(&mut mp4);
    assert_eq!(recovered, 0, "no stale damage record after the seek");
    assert_eq!(pts_only(packets), vec![(0, 4000), (0, 5000)]);
}

/// (e, the header-time half) The attached-picture packet rides the
/// pending queue at `start`; a re-anchor before it is consumed drops
/// it (the stale purge; a later seek does not re-deliver it — the
/// ffmpeg attached-pic precedent).
#[test]
fn fx5_a01_reanchor_drops_pending_cover_packet() {
    let file = progressive_with_cover(&[video_track(4, 1000)], b"isom", 13, &[0xC0, 0xFF, 0xEE]);
    // Control: the cover packet is the first event of a fresh walk
    // (attached-picture track, ordinal 1).
    {
        let mut probe = open(file.clone());
        let tracks = probe.start().unwrap_or_else(|e| unreachable!("start: {e}"));
        assert_eq!(tracks.len(), 2, "media + attached picture");
        match probe.next_event() {
            Ok(DemuxEvent::Packet(p)) => assert_eq!(p.track.handle(), 1),
            other => unreachable!("cover packet first, got {other:?}"),
        }
    }
    // The real instance: re-anchor before consuming — only the media
    // samples follow.
    let mut mp4 = open(file.clone());
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let landed = mp4
        .reanchor(span_at(0))
        .unwrap_or_else(|e| unreachable!("reanchor: {e}"));
    assert_eq!(landing_ticks(&landed), 0);
    let (packets, _) = drain(&mut mp4);
    assert_eq!(
        pts_only(packets),
        vec![(0, 0), (0, 1000), (0, 2000), (0, 3000)],
        "media samples only — the cover packet is gone with the old position"
    );
}

/// The fragmented lane, backward: the scan runs from the file start
/// (the pre-fix scan started at the walk position and a backward
/// target was an out-of-range lie). A partially consumed moof batch
/// rides the pending queue — the stale purge must drop it (no
/// duplicate 5000-tick packet in the replay).
#[test]
fn fx5_a01_fragmented_reanchor_backward_to_zero_replays() {
    let file = fragmented(&[video_track(8, 1000)], 4, 2);
    let truth = from_start_truth(&file);
    assert_eq!(
        truth,
        vec![
            (0, 0),
            (0, 1000),
            (0, 2000),
            (0, 3000),
            (0, 4000),
            (0, 5000),
            (0, 6000),
            (0, 7000)
        ]
    );

    let mut mp4 = open(file.clone());
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    // 5 packets = moofs 0-1 plus the first sample of moof 2 (its
    // sibling — ticks 5000 — sits in the pending queue).
    drain_through(&mut mp4, 5);
    let landed = mp4
        .reanchor(span_at(0))
        .unwrap_or_else(|e| unreachable!("reanchor: {e}"));
    assert_eq!(landing_ticks(&landed), 0, "backward to fragment 0");
    let (packets, _) = drain(&mut mp4);
    assert_eq!(
        pts_only(packets),
        truth,
        "the full ladder, no stale fragment-batch duplicate"
    );
    // And from EOS: the fragment stream revives too.
    let landed = mp4
        .reanchor(span_at(0))
        .unwrap_or_else(|e| unreachable!("reanchor from EOS: {e}"));
    assert_eq!(landing_ticks(&landed), 0);
    let (packets, _) = drain(&mut mp4);
    assert_eq!(pts_only(packets), truth, "the stream plays again");
}

/// The fragmented lane, forward: the landing is the moof whose
/// reference track starts a sync sample ≤ target, and the packets
/// continue from there (the pure-query test's committed twin).
#[test]
fn fx5_a01_fragmented_reanchor_forward_scans_and_continues() {
    let file = fragmented(&[video_track(8, 1000)], 4, 2);
    let mut mp4 = open(file.clone());
    mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    drain_through(&mut mp4, 2);
    // 450 ms → 4050 ticks: fragment 2 (samples 4-5, dts 4000) holds
    // the second sync.
    let landed = mp4
        .reanchor(span_at(450_000_000))
        .unwrap_or_else(|e| unreachable!("reanchor: {e}"));
    assert_eq!(landing_ticks(&landed), 4000);
    let (packets, _) = drain(&mut mp4);
    assert_eq!(
        pts_only(packets),
        vec![(0, 4000), (0, 5000), (0, 6000), (0, 7000)]
    );
}

/// The AVI agreement model: on a forward target the pure query and
/// the acting half report the same landing (ticks and base) — both
/// lanes.
#[test]
fn fx5_a01_reanchor_landing_agrees_with_the_pure_query() {
    let file = progressive(&[video_track(6, 1000)], b"isom");
    let mut a = open(file.clone());
    a.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let queried = a
        .seek(span_at(580_000_000))
        .unwrap_or_else(|e| unreachable!("query: {e}"));
    let mut b = open(file.clone());
    b.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let landed = b
        .reanchor(span_at(580_000_000))
        .unwrap_or_else(|e| unreachable!("reanchor: {e}"));
    let (q, l) = match (queried.start.timestamp(), landed.start.timestamp()) {
        (Ok(q), Ok(l)) => (q, l),
        _ => unreachable!("both landings present"),
    };
    assert_eq!(q.ticks, l.ticks);
    assert_eq!(q.base, l.base);

    let ffile = fragmented(&[video_track(8, 1000)], 4, 2);
    let mut c = open(ffile.clone());
    c.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let queried = c
        .seek(span_at(450_000_000))
        .unwrap_or_else(|e| unreachable!("query: {e}"));
    let mut d = open(ffile.clone());
    d.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    let landed = d
        .reanchor(span_at(450_000_000))
        .unwrap_or_else(|e| unreachable!("reanchor: {e}"));
    let (q, l) = match (queried.start.timestamp(), landed.start.timestamp()) {
        (Ok(q), Ok(l)) => (q, l),
        _ => unreachable!("both landings present"),
    };
    assert_eq!(q.ticks, l.ticks);
    assert_eq!(q.base, l.base);
}

/// The typed-refusal discipline: `reanchor` before `start` is the
/// demuxer's own `not-started` refusal, never a panic.
#[test]
fn fx5_a01_reanchor_before_start_is_typed() {
    let file = progressive(&[video_track(2, 1000)], b"isom");
    let mut mp4 = open(file.clone());
    let err = match mp4.reanchor(span_at(0)) {
        Ok(_) => unreachable!("reanchor before start must refuse"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "demux.mp4.not-started");
}
