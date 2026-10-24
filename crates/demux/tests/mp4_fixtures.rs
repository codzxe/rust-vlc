//! The MP4/MOV/3GP generated-fixture suites (card 5.C1): the
//! ffmpeg-authored valid fixtures (h264+aac progressive, QuickTime
//! `.mov`, fMP4 with and without `sidx`, chapters, `elst` edit lists,
//! 3GP H.263) demux through the real access stack, and the
//! deterministic malformed derivatives (trunc/flip/garbage) run
//! through both the `+FB` fuzz harness and a full typed pass —
//! **no panic, no hang, no unbounded allocation, nothing fake**.
//!
//! The parents were authored with the sandbox ffmpeg (Lavf61) —
//! `-movflags +faststart` progressive shapes and `-movflags
//! +frag_keyframe+empty_moov` fragmented shapes — by the prior
//! session's generator; these suites are the wiring the card's AC 5
//! requires (fixtures on disk are only half the contract).

use std::path::Path;

use plugin_api::{DemuxEvent, Demuxer};

use common::open_via_input;
use demux::mp4::{Mp4Demuxer, fuzz};
use testkit::Must;

mod common;

/// One valid fixture's expectations.
struct Valid {
    fixture: &'static str,
    /// (codec, kind) per exposed track, moov order.
    codecs: &'static [(&'static str, media_types::MediaKind)],
    fragmented: bool,
    chapters: usize,
}

const VALID: [Valid; 7] = [
    Valid {
        fixture: "generated/valid/mp4_h264_aac.mp4",
        codecs: &[
            ("h264", media_types::MediaKind::Video),
            ("aac", media_types::MediaKind::Audio),
        ],
        fragmented: false,
        chapters: 0,
    },
    Valid {
        fixture: "generated/valid/mp4_qt.mov",
        codecs: &[
            ("h264", media_types::MediaKind::Video),
            ("aac", media_types::MediaKind::Audio),
        ],
        fragmented: false,
        chapters: 0,
    },
    Valid {
        fixture: "generated/valid/mp4_frag.mp4",
        codecs: &[
            ("h264", media_types::MediaKind::Video),
            ("aac", media_types::MediaKind::Audio),
        ],
        fragmented: true,
        chapters: 0,
    },
    Valid {
        fixture: "generated/valid/mp4_frag_sidx.mp4",
        codecs: &[
            ("h264", media_types::MediaKind::Video),
            ("aac", media_types::MediaKind::Audio),
        ],
        fragmented: true,
        chapters: 0,
    },
    Valid {
        fixture: "generated/valid/mp4_chapters.mp4",
        // The QuickTime text chapter trak is tref/chap-referenced and
        // hidden from the exposed list (ffprobe shows it as `bin_data`).
        codecs: &[
            ("h264", media_types::MediaKind::Video),
            ("aac", media_types::MediaKind::Audio),
        ],
        fragmented: false,
        chapters: 3,
    },
    Valid {
        fixture: "generated/valid/mp4_edts.mp4",
        codecs: &[
            ("h264", media_types::MediaKind::Video),
            ("aac", media_types::MediaKind::Audio),
        ],
        fragmented: false,
        chapters: 0,
    },
    Valid {
        fixture: "generated/valid/mp4_3gp_h263.3gp",
        codecs: &[("h263", media_types::MediaKind::Video)],
        fragmented: false,
        chapters: 0,
    },
];

/// Open a fixture through the real access stack and start it.
fn started(path: &Path) -> (Mp4Demuxer, Vec<media_types::Track>) {
    let stream = open_via_input(path).unwrap_or_else(|e| unreachable!("open: {e}"));
    let mut mp4 = Mp4Demuxer::new(stream).unwrap_or_else(|e| unreachable!("new: {e}"));
    let tracks = mp4.start().unwrap_or_else(|e| unreachable!("start: {e}"));
    (mp4, tracks)
}

/// Drain every event (bounded), returning per-track packet counts and
/// the first video track's keyframe count.
fn drain(mp4: &mut Mp4Demuxer) -> (Vec<u64>, u64, u64) {
    let mut packets: Vec<u64> = Vec::new();
    let mut keyframes = 0u64;
    let mut events = 0u64;
    loop {
        events += 1;
        assert!(events < 500_000, "event budget");
        match mp4
            .next_event()
            .unwrap_or_else(|e| unreachable!("event: {e}"))
        {
            DemuxEvent::Packet(p) => {
                let handle = p.track.handle() as usize;
                if handle >= packets.len() {
                    packets.resize(handle + 1, 0);
                }
                packets[handle] += 1;
                if p.keyframe {
                    keyframes += 1;
                }
                assert!(!p.data.is_empty(), "empty payload");
            }
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
    }
    let total: u64 = packets.iter().sum();
    (packets, keyframes, total)
}

#[test]
fn valid_mp4_fixtures_demux_fully() {
    for spec in &VALID {
        let path = testkit::fixture_path(spec.fixture).must();
        let (mut mp4, tracks) = started(&path);
        assert_eq!(
            tracks.len(),
            spec.codecs.len(),
            "{}: track count",
            spec.fixture
        );
        for (track, (codec, kind)) in tracks.iter().zip(spec.codecs) {
            assert_eq!(track.codec.as_str(), *codec, "{}: codec", spec.fixture);
            assert_eq!(track.kind, *kind, "{}: kind", spec.fixture);
        }
        assert_eq!(
            mp4.is_fragmented(),
            spec.fragmented,
            "{}: fragmented",
            spec.fixture
        );
        assert_eq!(
            mp4.chapters().len(),
            spec.chapters,
            "{}: chapters",
            spec.fixture
        );
        let (packets, _key, total) = drain(&mut mp4);
        assert!(total > 0, "{}: no packets", spec.fixture);
        assert_eq!(
            packets.len(),
            spec.codecs.len(),
            "{}: packet tracks",
            spec.fixture
        );
    }
}

/// The chapter fixture's marker titles and movie-timeline starts.
#[test]
fn chapters_fixture_surfaces_titled_markers() {
    let path = testkit::fixture_path("generated/valid/mp4_chapters.mp4").must();
    let (mp4, _tracks) = started(&path);
    let chapters = mp4.chapters();
    assert_eq!(chapters.len(), 3);
    assert!(chapters.iter().all(|c| !c.title.is_empty()));
    // Monotone movie-timeline starts.
    assert!(chapters.windows(2).all(|w| w[0].start.0 < w[1].start.0));
}

/// The `elst` fixture applies its edit timeline (non-zero first PTS
/// on the video track — the empty-edit/media-time shape).
#[test]
fn edts_fixture_shifts_the_first_pts() {
    let path = testkit::fixture_path("generated/valid/mp4_edts.mp4").must();
    let (mut mp4, _tracks) = started(&path);
    let mut first_pts: Option<i64> = None;
    loop {
        match mp4
            .next_event()
            .unwrap_or_else(|e| unreachable!("event: {e}"))
        {
            DemuxEvent::Packet(p) => {
                if p.track.handle() == 0 && first_pts.is_none() {
                    first_pts = Some(
                        p.pts
                            .timestamp()
                            .unwrap_or_else(|e| unreachable!("{e}"))
                            .ticks,
                    );
                }
            }
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
    }
    let Some(pts) = first_pts else {
        unreachable!("no video packet")
    };
    assert!(
        pts != 0,
        "the edts fixture's first video PTS must be shifted, got {pts}"
    );
}

const MALFORMED: [&str; 14] = [
    "generated/malformed/mp4_h264_aac__flip1.mp4",
    "generated/malformed/mp4_h264_aac__flip4.mp4",
    "generated/malformed/mp4_h264_aac__flip16.mp4",
    "generated/malformed/mp4_h264_aac__garbage0.mp4",
    "generated/malformed/mp4_h264_aac__trunc0.mp4",
    "generated/malformed/mp4_h264_aac__trunc1.mp4",
    "generated/malformed/mp4_h264_aac__trunc2.mp4",
    "generated/malformed/mp4_qt__trunc0.mov",
    "generated/malformed/mp4_3gp_h263__flip1.3gp",
    "generated/malformed/mp4_3gp_h263__trunc0.3gp",
    "generated/malformed/mp4_chapters__flip1.mp4",
    "generated/malformed/mp4_edts__trunc0.mp4",
    "generated/malformed/mp4_frag__flip1.mp4",
    "generated/malformed/mp4_frag__trunc0.mp4",
];

/// AC 5 (`+FB`): every malformed derivative through the fuzz harness
/// — the contract is absence of panic/hang/unbounded allocation.
#[test]
fn malformed_mp4_fixtures_survive_the_fuzz_harness() {
    for fixture in MALFORMED {
        let bytes = testkit::fixture_bytes(fixture).must();
        fuzz::fuzz_demux_mp4(&bytes);
    }
}

/// The same derivatives through a full typed pass: `start` either
/// refuses typed or exposes tracks; the walk either drains to EOS or
/// fails typed — never a panic, never a fake packet.
#[test]
fn malformed_mp4_fixtures_only_ever_refuse_or_demux_typed() {
    for fixture in MALFORMED {
        let path = testkit::fixture_path(fixture).must();
        let stream = open_via_input(&path).unwrap_or_else(|e| unreachable!("open: {e}"));
        let mut mp4 = Mp4Demuxer::new(stream).unwrap_or_else(|e| unreachable!("new: {e}"));
        let mut events = 0u64;
        if let Err(err) = mp4.start() {
            // Typed refusal — the stage and code must be ours.
            assert_eq!(
                err.stage,
                media_types::PipelineStage::Demux,
                "{fixture}: {err}"
            );
            assert!(
                err.context.code.as_str().starts_with("demux.mp4."),
                "{fixture}: {err}"
            );
            continue;
        }
        loop {
            events += 1;
            assert!(events < 500_000, "{fixture}: event budget");
            match mp4.next_event() {
                Ok(DemuxEvent::EndOfStream) | Err(_) => break,
                Ok(DemuxEvent::Packet(p)) => assert!(!p.data.is_empty(), "{fixture}: fake packet"),
                Ok(_) => {}
            }
        }
    }
}
