//! The committed-fixture MPEG-PS/VOB differential (card 5.I2): the
//! four ffmpeg-authored products under `fixtures/ps/` demuxed
//! through the real file-access stack and compared against the
//! **measured ffmpeg truth** — stream identity from `ffprobe
//! -show_entries stream=codec_name,codec_type`, packet counts from
//! `ffprobe -show_packets`, payload digests from `ffmpeg -map
//! -c copy` raw extractions (ffmpeg 7.1.5 / Lavf61.7.103, the same
//! build that authored the fixtures, measured this session).
//!
//! # The re-packetization divergence (pinned, not matched)
//!
//! ffmpeg's demuxer re-packetizes to codec access units: mpegvideo
//! picture-splitting (25 "packets" from 68 video PES on the VOBs)
//! and AC-3 frame alignment (32 packets from 7 PES). This demuxer's
//! container truth is the PES packet (one PES = one demuxed packet)
//! — the 5.I2 prior-session note. The table below pins OUR counts
//! and records ffprobe's alongside instead of matching them; the
//! *byte streams* are what must agree, and they do bit-for-bit on
//! video / MP2 / AC-3 / DTS.
//!
//! # The LPCM delta (measured, exact)
//!
//! ffmpeg's `pcm_dvd` extraction retains a 3-byte pcm_dvd AU header
//! per packet (96 144 B = 48 × (3 + 2 000)); this demuxer strips the
//! full 7-byte DVD private-1 prefix (substream + 6-byte LPCM
//! header), so its payload is the pure sample bytes (96 000 B).
//! The exact delta — `ffmpeg_size - ours == 3 × packets` — is
//! asserted below.
//!
//! # Nav packets (measured, honest)
//!
//! ffprobe declares the `dvd_nav_packet` Data stream but demuxes
//! **zero** packets for it (its `dvd` demuxer consumes private
//! stream 2 as navigation); this demuxer exposes the 4 nav PES as
//! Data-track packets (container truth) — recorded in the table.

use std::path::PathBuf;

mod common;

use common::{md5_hex, open_via_input};
use demux::ps::PsDemuxer;
use media_types::ids::MediaKind;
use media_types::time::{Stamp, TimeBase};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};
use testkit::Must;

/// One fixture's measured truth (both sides).
struct Lane {
    /// File name under `fixtures/ps/`.
    file: &'static str,
    /// `(ffprobe codec_name, our descriptor, kind)` per stream, in
    /// ffprobe's stream order (== our first-appearance order).
    streams: [(&'static str, &'static str, MediaKind); 3],
    /// Track count (2 for the MPG, 3 for the VOBs).
    track_count: usize,
    /// Our PES packet count per track index (container truth).
    ours: [u64; 3],
    /// ffprobe `-show_packets` count per stream index (the
    /// re-packetized truth; `[0; 3]` slots unused on the MPG lane).
    ffprobe: [u64; 3],
    /// md5 of our concatenated payload bytes per track index.
    digest: [&'static str; 3],
    /// Payload byte totals per track index (ours).
    bytes: [u64; 3],
    /// ffmpeg `-c copy` extraction size per track index (equal to
    /// `bytes` except the LPCM lane — the 3-byte AU-header delta).
    ffmpeg_bytes: [u64; 3],
    /// First PTS in 90 kHz ticks per track index (None = unstamped).
    first_pts: [Option<i64>; 3],
    /// Last PTS in 90 kHz ticks per track index (None = unstamped).
    last_pts: [Option<i64>; 3],
}

const LANES: [Lane; 4] = [
    // ffprobe: mpeg2video + mp2; packets 25 + 39.
    Lane {
        file: "ps-mp2-video-audio.mpg",
        streams: [
            ("mpeg2video", "mpeg2video", MediaKind::Video),
            ("mp2", "mp2", MediaKind::Audio),
            ("", "", MediaKind::Video),
        ],
        track_count: 2,
        ours: [32, 5, 0],
        ffprobe: [25, 39, 0],
        digest: [
            "b235981565a1b5f08783097bb9f40627",
            "903e70465aecd25c6e5cf6c9f38b3daf",
            "",
        ],
        bytes: [63_898, 8_150, 0],
        ffmpeg_bytes: [63_898, 8_150, 0],
        first_pts: [Some(48_600), Some(47_618), None],
        // The audio span end is the last *observed PES stamp*
        // (118 149); ffprobe interpolates the unstamped MP2 AUs to
        // 136 957 — the honest container-truth divergence.
        last_pts: [Some(135_000), Some(118_149), None],
    },
    // ffprobe: dvd_nav_packet + mpeg2video + ac3; packets 0 + 25 + 32.
    Lane {
        file: "ps-vob-ac3.vob",
        streams: [
            ("dvd_nav_packet", "dvd_nav_packet", MediaKind::Data),
            ("mpeg2video", "mpeg2video", MediaKind::Video),
            ("ac3", "ac3", MediaKind::Audio),
        ],
        track_count: 3,
        ours: [4, 68, 7],
        ffprobe: [0, 25, 32],
        digest: [
            "",
            "130873a40a8090129373005692be7269",
            "ca55023b9c0a551199cb45269e8d939c",
        ],
        bytes: [0, 135_397, 12_288],
        ffmpeg_bytes: [0, 135_397, 12_288],
        first_pts: [None, Some(48_754), Some(48_274)],
        // Audio span ends at the last observed PES stamp (126 034);
        // ffprobe interpolates the unstamped AC-3 AUs to 137 554 —
        // the honest container-truth divergence (video last stamp
        // 135 090; ffprobe prints N/A there — B-frame DTS-only tail).
        last_pts: [None, Some(135_090), Some(126_034)],
    },
    // ffprobe: dvd_nav_packet + mpeg2video + dts; packets 0 + 25 + 94.
    Lane {
        file: "ps-vob-dts.vob",
        streams: [
            ("dvd_nav_packet", "dvd_nav_packet", MediaKind::Data),
            ("mpeg2video", "mpeg2video", MediaKind::Video),
            ("dts", "dts", MediaKind::Audio),
        ],
        track_count: 3,
        ours: [4, 68, 24],
        ffprobe: [0, 25, 94],
        digest: [
            "",
            "130873a40a8090129373005692be7269",
            "de45412596ea9fc909b4553055e420b0",
        ],
        bytes: [0, 135_397, 48_128],
        ffmpeg_bytes: [0, 135_397, 48_128],
        first_pts: [None, Some(48_754), Some(48_754)],
        last_pts: [None, Some(135_090), Some(136_114)],
    },
    // ffprobe: dvd_nav_packet + mpeg2video + pcm_dvd; 0 + 25 + 48.
    Lane {
        file: "ps-vob-lpcm.vob",
        streams: [
            ("dvd_nav_packet", "dvd_nav_packet", MediaKind::Data),
            ("mpeg2video", "mpeg2video", MediaKind::Video),
            ("pcm_dvd", "pcm_dvd", MediaKind::Audio),
        ],
        track_count: 3,
        ours: [4, 68, 48],
        ffprobe: [0, 25, 48],
        digest: [
            "",
            "130873a40a8090129373005692be7269",
            "d7754037c4ff56d806ada9dc67877c59",
        ],
        bytes: [0, 135_397, 96_000],
        // ffmpeg keeps a 3-byte pcm_dvd AU header per packet:
        // 96 144 = 48 × (3 + 2 000) — the exact-delta assertion.
        ffmpeg_bytes: [0, 135_397, 96_144],
        first_pts: [None, Some(48_754), Some(48_754)],
        last_pts: [None, Some(135_090), Some(138_583)],
    },
];

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/ps")
        .join(name)
}

fn span_ticks(track: &media_types::Track, which: bool) -> Option<i64> {
    let stamp = if which {
        track.timing.span.start
    } else {
        track.timing.span.end
    };
    let Stamp::Present(ts) = stamp else {
        return None;
    };
    let base = TimeBase::new(1, 90_000).must();
    Some(ts.convert(&base).must().ticks)
}

/// The full differential walk for one lane.
fn run_lane(lane: &Lane) -> (Vec<media_types::Track>, Vec<(u32, Vec<u8>)>) {
    let stream = open_via_input(&fixture_path(lane.file)).must();
    let mut ps = PsDemuxer::new(stream).must();
    assert_eq!(ps.seekability(), SeekSupport::SlowLinear);
    let tracks = ps.start().must();
    assert_eq!(tracks.len(), lane.track_count, "{}", lane.file);
    for (i, (ffprobe_name, ours, kind)) in lane.streams.iter().take(lane.track_count).enumerate() {
        assert_eq!(
            tracks[i].kind, *kind,
            "{} track {i} kind (ffprobe calls it {ffprobe_name})",
            lane.file
        );
        assert_eq!(
            tracks[i].codec.as_str(),
            *ours,
            "{} track {i} descriptor (ffprobe: {ffprobe_name})",
            lane.file
        );
    }
    let mut packets: Vec<(u32, Vec<u8>)> = Vec::new();
    loop {
        match ps.next_event().must() {
            DemuxEvent::Packet(p) => packets.push((p.track.handle(), p.data.to_vec())),
            DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => {}
            DemuxEvent::EndOfStream => break,
        }
    }
    (tracks, packets)
}

#[test]
fn fixture_stream_tables_and_packet_counts_match_the_pins() {
    for lane in &LANES {
        let (tracks, packets) = run_lane(lane);
        for i in 0..lane.track_count {
            let mine = packets.iter().filter(|(t, _)| *t == i as u32).count() as u64;
            assert_eq!(
                mine, lane.ours[i],
                "{} track {i}: our PES count (ffprobe re-packetizes to {})",
                lane.file, lane.ffprobe[i]
            );
        }
        // PTS extents: the span start/end ticks per track.
        for (i, want) in lane.first_pts.iter().take(lane.track_count).enumerate() {
            assert_eq!(
                span_ticks(&tracks[i], true),
                *want,
                "{} track {i} first pts",
                lane.file
            );
        }
        for (i, want) in lane.last_pts.iter().take(lane.track_count).enumerate() {
            assert_eq!(
                span_ticks(&tracks[i], false),
                *want,
                "{} track {i} last pts",
                lane.file
            );
        }
    }
}

#[test]
fn fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit() {
    for lane in &LANES {
        let (_, packets) = run_lane(lane);
        for i in 0..lane.track_count {
            if lane.digest[i].is_empty() {
                continue; // the nav lane: unhashed control payload
            }
            let mut concat = Vec::new();
            for (t, data) in &packets {
                if *t == i as u32 {
                    concat.extend_from_slice(data);
                }
            }
            assert_eq!(
                concat.len() as u64,
                lane.bytes[i],
                "{} track {i} payload bytes",
                lane.file
            );
            assert_eq!(
                md5_hex(&concat),
                lane.digest[i],
                "{} track {i} payload digest",
                lane.file
            );
            // The ffmpeg-side size: identical everywhere except the
            // LPCM lane's 3-byte-per-AU pcm_dvd header delta.
            assert_eq!(
                lane.ffmpeg_bytes[i] - lane.bytes[i],
                if lane.file.ends_with("lpcm.vob") && i == 2 {
                    3 * lane.ours[i]
                } else {
                    0
                },
                "{} track {i}: exact ffmpeg size delta",
                lane.file
            );
        }
    }
}

#[test]
fn lpcm_packets_strip_the_full_dvd_prefix() {
    // The LPCM lane's 48 packets total 96 000 sample bytes with the
    // 7-byte DVD prefix (substream + 6-byte LPCM header) fully
    // stripped — measured sizes 2 008 … 1 424 (ffmpeg's own
    // extraction is ours + 3 B × 48 of retained pcm_dvd AU headers).
    let (_, packets) = run_lane(&LANES[3]);
    let lpcm: Vec<&Vec<u8>> = packets
        .iter()
        .filter(|(t, _)| *t == 2)
        .map(|(_, d)| d)
        .collect();
    assert_eq!(lpcm.len(), 48);
    assert_eq!(lpcm[0].len(), 2_008, "first lpcm packet");
    assert_eq!(lpcm[47].len(), 1_424, "final lpcm packet");
    let total: usize = lpcm.iter().map(|d| d.len()).sum();
    assert_eq!(total, 96_000);
}

#[test]
fn vob_nav_packets_are_real_and_zero_padded_by_ffmpeg() {
    // ffmpeg's `dvd` muxer zero-fills the PCI/DSI navigation payload;
    // the 4 nav PES come through whole (container truth) where
    // ffprobe's own demuxer consumes all of them (0 packets).
    let (_, packets) = run_lane(&LANES[1]);
    let nav: Vec<&Vec<u8>> = packets
        .iter()
        .filter(|(t, _)| *t == 0)
        .map(|(_, d)| d)
        .collect();
    assert_eq!(nav.len(), 4);
    // Measured private_stream_2 lengths alternate 980 / 1 018 (the
    // ffmpeg `dvd` muxer's PCI/DSI nav-pack pair); the 1 018-byte
    // DSI packs carry a single 0x01 lead byte, the rest is
    // zero-filled by ffmpeg's muxer (measured).
    for (i, want) in [980usize, 1018, 980, 1018].iter().enumerate() {
        assert_eq!(nav[i].len(), *want, "nav packet {i} size");
        let (lead, rest) = nav[i].split_first().must();
        if *want == 1018 {
            assert_eq!(*lead, 0x01, "nav packet {i} DSI lead byte");
        } else {
            assert_eq!(*lead, 0x00, "nav packet {i} PCI lead byte");
        }
        assert!(rest.iter().all(|b| *b == 0), "nav packet {i} zero-filled");
    }
}
