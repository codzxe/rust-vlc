//! The real-corpus MP4/MOV/M4A differential (card 5.C1, the standing
//! mandate): every ISO-BMFF file in the randomized real corpus
//! (corpus-v4, ffmpeg-authored bytes — 24 `.mp4` + 5 `.mov` + 2 `.m4a`)
//! demuxed through the real access stack and compared against the
//! **measured ffmpeg truth** — `ffprobe -count_packets` stream codecs
//! and packet counts, plus the first video stream's keyframe-flagged
//! packet count (`ffprobe -show_packets packet=flags`, `K`-flagged).
//!
//! Every pinned value below was measured this session from the
//! corpus-v4 instance at `/tmp/my-project/corpus-v4/media` (ffprobe
//! 7.1.5 / Lavf61.7.103, the same build that authored the corpus).
//! The corpus regenerates with fresh seeds: when it is absent (and no
//! `LUMEN_CORPUS` override is set) this suite **skips loudly** — the
//! per-file PASS/FAIL table for this instance lives in the worklog
//! entry, exactly as measured.
//!
//! # Honest scope (recorded gaps, not smoothed)
//!
//! - `0128-param-midchange.mp4` is MPEG-2 video (`mp2v`): the stsd
//!   entry is outside the registry's codec vocabulary, so the demuxer
//!   refuses the file typed (`no-tracks`) — recorded [PENDING] for
//!   the MPEG-2 codec card, not pinned here.
//! - `0029-cover-audio.m4a`'s cover art is a real attached-pic video
//!   trak — it is compared like any other stream.
//! - ffprobe `mpeg4` is the registry's `mpeg4video` (name mapping,
//!   not a behavior difference).

use std::path::PathBuf;

use plugin_api::{DemuxEvent, Demuxer};

use common::open_via_input;
use demux::mp4::Mp4Demuxer;

mod common;

/// One corpus file's ffmpeg-measured truth.
struct CorpusTruth {
    file: &'static str,
    /// (registry codec, packet count) per stream, ffprobe order.
    streams: &'static [(&'static str, u64)],
    /// `K`-flagged packets on the first video stream (0 when the
    /// first video stream is absent).
    keyframes: u64,
}

const TRUTHS: [CorpusTruth; 30] = [
    // 0008: h264 all-keyframe extremes track — 345 packets, 2 flagged.
    CorpusTruth {
        file: "0008-fps-extremes.mp4",
        streams: &[("h264", 345)],
        keyframes: 2,
    },
    // 0012: the 24 h still — one packet, one keyframe.
    CorpusTruth {
        file: "0012-still-24h.mp4",
        streams: &[("h264", 1)],
        keyframes: 1,
    },
    CorpusTruth {
        file: "0021-vbv-burst.mp4",
        streams: &[("h264", 86)],
        keyframes: 3,
    },
    CorpusTruth {
        file: "0025-meta-bomb-av.mp4",
        streams: &[("h264", 111), ("aac", 44)],
        keyframes: 2,
    },
    CorpusTruth {
        file: "0027-mp4-video.mp4",
        streams: &[("h264", 42)],
        keyframes: 42,
    },
    // 0032: fragmented (moof ladder) h264.
    CorpusTruth {
        file: "0032-fmp4-video.mp4",
        streams: &[("h264", 77)],
        keyframes: 20,
    },
    CorpusTruth {
        file: "0045-mp4-video.mp4",
        streams: &[("mpeg4video", 37)],
        keyframes: 9,
    },
    CorpusTruth {
        file: "0049-muxed-av.mp4",
        streams: &[("h264", 42), ("aac", 35)],
        keyframes: 1,
    },
    CorpusTruth {
        file: "0058-rot-display.mp4",
        streams: &[("h264", 28)],
        keyframes: 1,
    },
    CorpusTruth {
        file: "0060-drift-av.mp4",
        streams: &[("h264", 1412), ("aac", 2029)],
        keyframes: 12,
    },
    CorpusTruth {
        file: "0062-mp4-video.mp4",
        streams: &[("h264", 3)],
        keyframes: 1,
    },
    CorpusTruth {
        file: "0069-muxed-av.mp4",
        streams: &[("h264", 25), ("aac", 18)],
        keyframes: 2,
    },
    CorpusTruth {
        file: "0074-bframe-storm.mp4",
        streams: &[("h264", 16)],
        keyframes: 1,
    },
    CorpusTruth {
        file: "0082-vfr-av.mp4",
        streams: &[("h264", 17), ("aac", 32)],
        keyframes: 1,
    },
    // 0089: fragmented HEVC.
    CorpusTruth {
        file: "0089-fmp4-video.mp4",
        streams: &[("hevc", 39)],
        keyframes: 1,
    },
    CorpusTruth {
        file: "0093-param-midchange.mp4",
        streams: &[("mpeg4video", 63)],
        keyframes: 5,
    },
    CorpusTruth {
        file: "0097-mp4-video.mp4",
        streams: &[("mpeg4video", 37)],
        keyframes: 2,
    },
    CorpusTruth {
        file: "0100-mp4-video.mp4",
        streams: &[("hevc", 49)],
        keyframes: 3,
    },
    CorpusTruth {
        file: "0105-multi-av.mp4",
        streams: &[("h264", 84), ("aac", 62), ("mp3", 55)],
        keyframes: 1,
    },
    CorpusTruth {
        file: "0109-odd-dims.mp4",
        streams: &[("h264", 44)],
        keyframes: 1,
    },
    CorpusTruth {
        file: "0112-packed-bs.mp4",
        streams: &[("h264", 25)],
        keyframes: 1,
    },
    CorpusTruth {
        file: "0115-fps-extremes.mp4",
        streams: &[("mjpeg", 8)],
        keyframes: 8,
    },
    // 0128 (mpeg2video) is the recorded vocabulary gap — see module
    // docs; it refuses typed and is [PENDING] the MPEG-2 codec card.
    CorpusTruth {
        file: "0140-muxed-av.mp4",
        streams: &[("hevc", 82), ("aac", 101)],
        keyframes: 2,
    },
    // ---- MOV (QuickTime authored) ----
    CorpusTruth {
        file: "0011-vfr-av.mov",
        streams: &[("h264", 117), ("mp3", 66)],
        keyframes: 2,
    },
    CorpusTruth {
        file: "0051-vfr-av.mov",
        streams: &[("mjpeg", 46), ("pcm_s16le", 93)],
        keyframes: 46,
    },
    CorpusTruth {
        file: "0061-muxed-av.mov",
        streams: &[("h264", 61), ("aac", 46)],
        keyframes: 61,
    },
    CorpusTruth {
        file: "0079-mov-video.mov",
        streams: &[("mjpeg", 60)],
        keyframes: 60,
    },
    CorpusTruth {
        file: "0122-packed-bs.mov",
        streams: &[("h264", 22)],
        keyframes: 1,
    },
    // ---- M4A ----
    // 0029: cover art is an attached-pic mjpeg trak (1 packet).
    CorpusTruth {
        file: "0029-cover-audio.m4a",
        streams: &[("aac", 2), ("mjpeg", 1)],
        keyframes: 1,
    },
    CorpusTruth {
        file: "0094-audio-only.m4a",
        streams: &[("aac", 48)],
        keyframes: 0,
    },
];

/// The corpus media dir: the `LUMEN_CORPUS` override, else the
/// sandbox default. `None` when neither exists (the loud skip).
fn corpus_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
        return Some(PathBuf::from(dir));
    }
    let default = PathBuf::from("/tmp/my-project/corpus-v4/media");
    default.is_dir().then_some(default)
}

#[test]
fn corpus_mp4_family_matches_the_ffmpeg_truth() {
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "corpus(mp4): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for the 2026-10-24 instance is in the worklog)"
        );
        return;
    };
    for truth in &TRUTHS {
        let path = dir.join(truth.file);
        let stream =
            open_via_input(&path).unwrap_or_else(|e| unreachable!("{}: open: {e}", truth.file));
        let mut mp4 =
            Mp4Demuxer::new(stream).unwrap_or_else(|e| unreachable!("{}: new: {e}", truth.file));
        let tracks = mp4
            .start()
            .unwrap_or_else(|e| unreachable!("{}: start: {e}", truth.file));
        assert_eq!(
            tracks.len(),
            truth.streams.len(),
            "{}: track count",
            truth.file
        );
        for (track, (codec, _)) in tracks.iter().zip(truth.streams) {
            assert_eq!(track.codec.as_str(), *codec, "{}: codec", truth.file);
        }
        // Drain: per-track packet counts + first-video keyframes.
        let mut counts: Vec<u64> = vec![0; tracks.len()];
        let mut keyframes = 0u64;
        let video_handle = tracks
            .iter()
            .position(|t| t.kind == media_types::MediaKind::Video)
            .map(|i| i as u32);
        let mut events = 0u64;
        loop {
            events += 1;
            assert!(events < 2_000_000, "{}: event budget", truth.file);
            match mp4
                .next_event()
                .unwrap_or_else(|e| unreachable!("{}: event: {e}", truth.file))
            {
                DemuxEvent::Packet(p) => {
                    counts[p.track.handle() as usize] += 1;
                    if Some(p.track.handle()) == video_handle && p.keyframe {
                        keyframes += 1;
                    }
                }
                DemuxEvent::EndOfStream => break,
                _ => {}
            }
        }
        for (i, (_, want)) in truth.streams.iter().enumerate() {
            assert_eq!(counts[i], *want, "{}: track {i} packet count", truth.file);
        }
        assert_eq!(
            keyframes, truth.keyframes,
            "{}: video keyframes",
            truth.file
        );
    }
}

/// The seek AC over the corpus: both paths (progressive stss+stco and
/// fMP4 moof-scan) land at or before the target and resume.
#[test]
fn corpus_mp4_seek_lands_and_resumes() {
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "corpus(mp4): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (seek differential for the 2026-10-24 instance is in the worklog)"
        );
        return;
    };
    for probe in [
        "0027-mp4-video.mp4",  // progressive, all-sync
        "0049-muxed-av.mp4",   // progressive A+V
        "0032-fmp4-video.mp4", // fragmented
        "0011-vfr-av.mov",     // QuickTime VFR
        "0094-audio-only.m4a", // audio-only M4A
    ] {
        let path = dir.join(probe);
        let stream = open_via_input(&path).unwrap_or_else(|e| unreachable!("{probe}: open: {e}"));
        let mut mp4 = Mp4Demuxer::new(stream).unwrap_or_else(|e| unreachable!("{probe}: new: {e}"));
        mp4.start()
            .unwrap_or_else(|e| unreachable!("{probe}: start: {e}"));
        let span = media_types::time::TimeSpan {
            start: media_types::time::Stamp::Present(
                media_types::time::Timestamp::from_parts(250, 1, 1_000)
                    .unwrap_or_else(|e| unreachable!("{e}")),
            ),
            end: media_types::time::Stamp::Missing,
        };
        let landed = mp4
            .seek(span)
            .unwrap_or_else(|e| unreachable!("{probe}: seek: {e}"));
        let start = landed
            .start
            .timestamp()
            .unwrap_or_else(|e| unreachable!("{probe}: {e}"));
        // Compare in nanoseconds — the landing is in the track's own
        // time base, the target was authored in milliseconds.
        let nanos = start
            .as_nanos()
            .ok()
            .map(|n| n.0)
            .unwrap_or_else(|| unreachable!("{probe}: landing not representable"));
        assert!(
            nanos <= 250_000_000,
            "{probe}: landed after target ({nanos} ns)"
        );
        // The walk resumes and finishes.
        let mut seen = 0u64;
        loop {
            match mp4
                .next_event()
                .unwrap_or_else(|e| unreachable!("{probe}: event: {e}"))
            {
                DemuxEvent::EndOfStream => break,
                DemuxEvent::Packet(_) => seen += 1,
                _ => {}
            }
            seen += 1;
            assert!(seen < 2_000_000, "{probe}: budget");
        }
    }
}
