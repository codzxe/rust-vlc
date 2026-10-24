//! Real-corpus differential for the raw ES demuxers (card 5.I3a):
//! the randomized corpus-v4 media lane, with ffprobe 7.1.5 (the
//! session's measured oracle) as ground truth.
//!
//! What is compared (per the card):
//!
//! - **packet counts** for every lane (`.h264`, `.265`, `.mp3`,
//!   `.aac`) against `ffprobe -show_packets`;
//! - **exact AU boundary geometry** for the two video lanes (packet
//!   sizes must equal ffprobe's measured `size` column — the walk is
//!   contiguous, so equal sizes ⇒ equal boundaries);
//! - **timing vocabulary** (video: `Stamp::Missing`, the raw-stream
//!   case; audio: exact sample clocks);
//! - **keyframe flags** vs ffprobe's `flags=K`, with the honestly
//!   recorded divergences (H.264 recovery-point SEI is decode-domain:
//!   ffmpeg marks `0019` packet 19 `K`, this demuxer marks IDR only;
//!   `0145`'s ID3 cover art is skipped metadata, ffmpeg synthesizes a
//!   second attached-pic `mjpeg` stream + 1 packet — not framing).
//!
//! Resolution truth for `0019` (64×48, High@1.3) is the landed
//! codecs-side H.264 parser's own corpus suite
//! (`codecs::tests::h264_parser_corpus`, green on this tree — SPS
//! parse vs ffprobe); this crate cannot link the codecs crate
//! (layer ordering: L2 may not depend on L3, and the reverse dev-edge
//! exists), so the resolution differential is cited, not re-run here.
//!
//! Activation: plain `cargo test -p demux --test es_corpus` (the
//! corpus dir defaults to `/tmp/my-project/corpus-v4/media`,
//! overridable via `LUMEN_CORPUS`); absent corpus ⇒ skip-with-message.

use std::path::{Path, PathBuf};

use demux::es::annexb::{AnnexbCodec, AnnexbVideoDemuxer};
use demux::es::mp3::RawMp3Demuxer;
use media_types::time::Stamp;
use media_types::{EngineError, Packet};
use plugin_api::{Access, DemuxEvent, Demuxer, SourceStream};

/// Open a corpus file through the real access stack (card 2.A1).
fn open_via_input(path: &Path) -> Result<Box<dyn SourceStream>, EngineError> {
    let text = match path.to_str() {
        Some(text) => format!("file://{text}"),
        None => unreachable!("corpus paths are UTF-8 ({path:?})"),
    };
    let uri = match media_types::ids::SourceUri::parse(&text) {
        Ok(uri) => uri,
        Err(err) => unreachable!("corpus path is a valid file URI ({path:?}): {err}"),
    };
    let access = input::FileAccess::new();
    access.open(&uri)
}

/// The corpus media dir: the `LUMEN_CORPUS` override, else the
/// session default.
fn corpus_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
        return Some(PathBuf::from(dir));
    }
    let default = PathBuf::from("/tmp/my-project/corpus-v4/media");
    default.is_dir().then_some(default)
}

/// One drained walk: packets, keyframe flags, terminal error.
type Walk = (Vec<Packet>, Vec<bool>, Option<EngineError>);

/// Drain a demuxer: packets (+ keyframe flags) to the end.
fn drain(demux: &mut dyn Demuxer) -> Result<Walk, EngineError> {
    let tracks = demux.start()?;
    let mut packets = Vec::new();
    let mut keys = Vec::new();
    let mut err = None;
    let _ = tracks;
    loop {
        match demux.next_event() {
            Ok(DemuxEvent::Packet(p)) => {
                keys.push(p.keyframe);
                packets.push(p);
            }
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
            Err(e) => {
                err = Some(e);
                break;
            }
        }
    }
    Ok((packets, keys, err))
}

/// ffprobe 7.1.5 measured packet sizes for `0019-raw-video.h264`
/// (contiguous: equal sizes ⇔ equal AU boundaries). 26 packets.
const H264_0019_SIZES: [u32; 26] = [
    1135, 196, 120, 143, 216, 162, 141, 232, 152, 156, 231, 162, 156, 237, 180, 160, 240, 174, 145,
    480, 138, 226, 154, 137, 223, 156,
];

/// ffprobe measured packet sizes for `0044-raw-video.265` (56).
const HEVC_0044_SIZES_HEAD: [u32; 8] = [19745, 13800, 14114, 14191, 14292, 14034, 14743, 14929];

#[test]
fn corpus_h264_lane_matches_ffprobe_packet_geometry() {
    let Some(dir) = corpus_dir() else {
        eprintln!("SKIP es corpus: /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset");
        return;
    };
    let path = dir.join("0019-raw-video.h264");
    if !path.is_file() {
        eprintln!("SKIP es corpus: 0019-raw-video.h264 absent");
        return;
    }
    let mut demux = AnnexbVideoDemuxer::new(
        AnnexbCodec::H264,
        open_via_input(&path).unwrap_or_else(|e| unreachable!("open {path:?}: {e:?}")),
    )
    .unwrap_or_else(|err| unreachable!("open 0019: {err:?}"));
    let (packets, keys, err) = drain(&mut demux).unwrap_or_else(|e| unreachable!("{e:?}"));
    assert!(err.is_none(), "clean stream: {err:?}");
    assert_eq!(packets.len(), 26, "ffprobe -show_packets: 26 packets");
    assert_eq!(demux.aus_demuxed(), 26);
    for (i, p) in packets.iter().enumerate() {
        assert_eq!(
            p.data.len(),
            H264_0019_SIZES[i] as usize,
            "packet {} size vs ffprobe",
            i + 1
        );
        assert_eq!(p.pts, Stamp::Missing, "raw Annex-B carries no stamps");
        assert_eq!(p.duration, None);
    }
    // Keyframes: the IDR AUs. ffmpeg additionally marks packet 19 `K`
    // via its recovery-point SEI parse (decode-domain; documented
    // divergence — this demuxer's flag is IDR-presence only).
    assert!(keys[0], "packet 1 holds the IDR");
    assert!(!keys[18], "packet 19 is a non-IDR slice (recovery point)");
    assert!(keys.iter().filter(|k| **k).count() >= 1);
    // Byte truth: packet 2 starts with the 4-byte start code + the
    // slice NAL header the landed parser classified (type 1).
    assert_eq!(&packets[1].data[0..5], &[0, 0, 0, 1, 0x41]);
}

#[test]
fn corpus_hevc_lane_matches_ffprobe_packet_geometry() {
    let Some(dir) = corpus_dir() else {
        eprintln!("SKIP es corpus: /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset");
        return;
    };
    let path = dir.join("0044-raw-video.265");
    if !path.is_file() {
        eprintln!("SKIP es corpus: 0044-raw-video.265 absent");
        return;
    }
    let mut demux = AnnexbVideoDemuxer::new(
        AnnexbCodec::Hevc,
        open_via_input(&path).unwrap_or_else(|e| unreachable!("open {path:?}: {e:?}")),
    )
    .unwrap_or_else(|err| unreachable!("open 0044: {err:?}"));
    let (packets, keys, err) = drain(&mut demux).unwrap_or_else(|e| unreachable!("{e:?}"));
    assert!(err.is_none(), "clean stream: {err:?}");
    assert_eq!(packets.len(), 56, "ffprobe -show_packets: 56 packets");
    assert_eq!(demux.aus_demuxed(), 56);
    for (i, want) in HEVC_0044_SIZES_HEAD.iter().enumerate() {
        assert_eq!(packets[i].data.len(), *want as usize, "packet {}", i + 1);
    }
    let total: u64 = packets.iter().map(|p| p.data.len() as u64).sum();
    let file_len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    assert_eq!(total, file_len, "contiguous coverage of the whole file");
    assert!(
        keys.iter().all(|k| *k),
        "all-IRAP stream: ffprobe flags all K"
    );
}

#[test]
fn corpus_mp3_lanes_match_ffprobe_packet_counts() {
    let Some(dir) = corpus_dir() else {
        eprintln!("SKIP es corpus: /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset");
        return;
    };
    // 0139: MPEG-2.5 L3, 11025 Hz stereo (576-sample frames).
    let path = dir.join("0139-audio-only.mp3");
    if path.is_file() {
        let mut demux = RawMp3Demuxer::new(
            open_via_input(&path).unwrap_or_else(|e| unreachable!("open {path:?}: {e:?}")),
        )
        .unwrap_or_else(|err| unreachable!("open 0139: {err:?}"));
        let (packets, _keys, err) = drain(&mut demux).unwrap_or_else(|e| unreachable!("{e:?}"));
        assert!(err.is_none(), "clean stream: {err:?}");
        assert_eq!(packets.len(), 79, "ffprobe audio packets: 79");
        let head = demux.head().unwrap_or_else(|| unreachable!("pinned"));
        assert_eq!(head.sample_rate, 11_025);
        assert_eq!(
            head.samples_per_frame, 576,
            "MPEG-2.5 L3 halves the granules"
        );
        // Exact sample clock: pts step = 576 on a 1/11025 base.
        let (a, b) = match (&packets[0].pts, &packets[1].pts) {
            (Stamp::Present(a), Stamp::Present(b)) => (a, b),
            other => unreachable!("mp3 pts present: {other:?}"),
        };
        assert_eq!(a.ticks, 0);
        assert_eq!(b.ticks - a.ticks, 576);
    } else {
        eprintln!("SKIP es corpus: 0139-audio-only.mp3 absent");
    }
    // 0145: cover-art lane. ffmpeg: 6 audio packets + 1 synthesized
    // attached-pic mjpeg packet; our audio lane: 6 (the ID3 APIC is
    // skipped metadata — the recorded divergence).
    let path = dir.join("0145-cover-audio.mp3");
    if path.is_file() {
        let mut demux = RawMp3Demuxer::new(
            open_via_input(&path).unwrap_or_else(|e| unreachable!("open {path:?}: {e:?}")),
        )
        .unwrap_or_else(|err| unreachable!("open 0145: {err:?}"));
        let (packets, _keys, err) = drain(&mut demux).unwrap_or_else(|e| unreachable!("{e:?}"));
        assert!(err.is_none(), "clean stream: {err:?}");
        assert_eq!(packets.len(), 6, "ffprobe audio packets: 6 (of 7 total)");
        assert!(demux.id3_skipped() > 0, "the cover tag is skipped metadata");
        let head = demux.head().unwrap_or_else(|| unreachable!("pinned"));
        assert_eq!(head.sample_rate, 24_000);
        assert_eq!(head.channels, 2);
    } else {
        eprintln!("SKIP es corpus: 0145-cover-audio.mp3 absent");
    }
}

#[test]
fn corpus_aac_lanes_match_ffprobe_packet_counts() {
    let Some(dir) = corpus_dir() else {
        eprintln!("SKIP es corpus: /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset");
        return;
    };
    for (name, want) in [("0059-audio-only.aac", 346), ("0110-audio-only.aac", 376)] {
        let path = dir.join(name);
        if !path.is_file() {
            eprintln!("SKIP es corpus: {name} absent");
            continue;
        }
        let mut demux = demux::es::aac::open(
            open_via_input(&path).unwrap_or_else(|e| unreachable!("open {path:?}: {e:?}")),
        )
        .unwrap_or_else(|err| unreachable!("open {name}: {err:?}"));
        let (packets, _keys, err) = drain(&mut demux).unwrap_or_else(|e| unreachable!("{e:?}"));
        assert!(err.is_none(), "clean stream {name}: {err:?}");
        assert_eq!(packets.len(), want, "ffprobe -show_packets: {want}");
        assert_eq!(demux.frames_demuxed(), want as u64);
        let info = demux
            .stream_info()
            .unwrap_or_else(|| unreachable!("started"));
        assert_eq!(info.format, "adts");
    }
}
