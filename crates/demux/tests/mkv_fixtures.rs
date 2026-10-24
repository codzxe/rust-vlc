//! MKV/WebM fixture suite (card 5.D1): the committed ffmpeg-authored
//! fixtures, the **hand-constructed lacing fixture** (ffmpeg's muxer
//! writes zero laced blocks in every file we generate — measured:
//! all corpus + fixture SimpleBlocks carry lacing bits `00` — so the
//! card's fallback applies: a minimal spec-built laced SimpleBlock
//! set, Xiph + EBML + fixed, constructed byte-by-byte here), the
//! malformed derivative family, and the demuxer-level error paths.
//!
//! ffmpeg-authored fixtures (lavfi sources, ffmpeg 7.1.5, all < 32
//! KiB committed):
//! - `mkv_h264_aac.mkv`, `mkv_h264_flac.mkv` — muxed A/V lanes
//! - `mkv_multitrack.mkv` — h264 + aac + flac + subrip (4 tracks)
//! - `webm_vp9_opus.webm` — the WebM branch (VP9 video demuxed,
//!   decode out of scope by assignment; Opus audio)

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

mod common;

use common::{md5_hex, open_via_input};
use demux::avi::test_support::mem_stream;
use demux::mkv::MkvDemuxer;
use media_types::time::Stamp;
use plugin_api::{DemuxEvent, Demuxer};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/generated/valid").join(name)
}

/// ffprobe's per-stream packet count (live measurement; the fixtures
/// are committed so the truth is stable).
fn probe_packets(path: &std::path::Path) -> Vec<(String, u64)> {
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-count_packets", "-show_streams", "-of", "json"])
        .arg(path)
        .output()
        .expect("ffprobe installed");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut streams = Vec::new();
    for chunk in text.split("\"index\":").skip(1) {
        let field = |name: &str| -> Option<String> {
            let pat = format!("\"{name}\":");
            let at = chunk.find(&pat)? + pat.len();
            let rest = chunk[at..].trim_start();
            if let Some(stripped) = rest.strip_prefix('"') {
                Some(stripped[..stripped.find('"')?].to_string())
            } else {
                Some(rest[..rest.find(|c: char| c == ',' || c == '}')?].trim().to_string())
            }
        };
        streams.push((
            field("codec_name").unwrap_or_default(),
            field("nb_read_packets").and_then(|v| v.parse().ok()).unwrap_or(0),
        ));
    }
    streams
}

/// ffmpeg's raw payload digest for one stream kind (a|v).
fn ffmpeg_stream_md5(path: &std::path::Path, kind: &str) -> String {
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-map", &format!("0:{kind}"), "-c", "copy", "-f", "md5", "-"])
        .output()
        .expect("ffmpeg installed");
    if !out.status.success() {
        panic!(
            "ffmpeg digest failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn walk_all(demuxer: &mut MkvDemuxer) -> (u64, Vec<(u64, u64)>) {
    let mut total = 0u64;
    let mut keyframes = Vec::new();
    loop {
        match demuxer.next_event().expect("walk clean") {
            DemuxEvent::Packet(p) => {
                let i = p.track.handle() as usize;
                while keyframes.len() <= i {
                    keyframes.push((0, 0));
                }
                keyframes[i].0 += 1;
                keyframes[i].1 += u64::from(p.keyframe);
                total += 1;
            }
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
    }
    (total, keyframes)
}

#[test]
fn valid_fixtures_match_ffprobe_packet_counts() {
    for (name, kinds) in [
        ("mkv_h264_aac.mkv", 2usize),
        ("mkv_h264_flac.mkv", 2),
        ("mkv_multitrack.mkv", 4),
        ("webm_vp9_opus.webm", 2),
    ] {
        let path = fixture(name);
        assert!(path.exists(), "{name} missing");
        let probe = probe_packets(&path);
        assert_eq!(probe.len(), kinds, "{name}: ffprobe stream count");
        let mut demuxer = MkvDemuxer::new(open(&path)).expect("construct {name}");
        let tracks = demuxer.start().expect("start {name}");
        assert_eq!(tracks.len(), kinds, "{name}: track count");
        let (total, per) = walk_all(&mut demuxer);
        let expected: u64 = probe.iter().map(|(_, n)| n).sum();
        assert_eq!(total, expected, "{name}: packet count vs ffprobe");
        for (i, (got, _)) in per.iter().enumerate() {
            assert_eq!(*got, probe[i].1, "{name}: stream {i} packets");
        }
        println!("PASS {name}: {kinds} tracks, {total} packets (ffprobe-matched)");
    }
}

#[test]
fn audio_payload_is_bit_exact_vs_ffmpeg() {
    for name in [
        "mkv_h264_aac.mkv",
        "mkv_h264_flac.mkv",
        "webm_vp9_opus.webm",
        "mkv_multitrack.mkv",
    ] {
        let path = fixture(name);
        let mut demuxer = MkvDemuxer::new(open(&path)).expect("construct");
        demuxer.start().expect("start");
        // Every audio track's payload bytes, in order, compared with
        // ffmpeg's per-stream raw dump (multi-audio files compare per
        // stream, not the -map 0:a interleave).
        let audio_ordinals: Vec<usize> = demuxer
            .track_entries()
            .iter()
            .enumerate()
            .filter(|(_, e)| e.kind == media_types::MediaKind::Audio)
            .map(|(i, _)| i)
            .collect();
        assert!(!audio_ordinals.is_empty(), "{name}: audio tracks");
        let mut per_track: Vec<Vec<u8>> = audio_ordinals.iter().map(|_| Vec::new()).collect();
        loop {
            match demuxer.next_event().expect("walk") {
                DemuxEvent::Packet(p) => {
                    if let Some(slot) = audio_ordinals
                        .iter()
                        .position(|&i| p.track.handle() as usize == i)
                    {
                        per_track[slot].extend_from_slice(&p.data);
                    }
                }
                DemuxEvent::EndOfStream => break,
                _ => {}
            }
        }
        for (slot, &ordinal) in audio_ordinals.iter().enumerate() {
            let truth = ffmpeg_stream_md5(&path, &ordinal.to_string());
            let ours = format!("MD5={}", md5_hex(&per_track[slot]));
            assert_eq!(ours, truth, "{name}: audio track {ordinal} payload md5");
            println!("PASS {name}: audio track {ordinal} payload md5 {ours}");
        }
    }
}

// ---- The hand-constructed lacing fixture (spec-built) -----------

/// Append an element: id vint (as raw bytes — every id below fits its
/// marker form), size vint, payload.
fn el(out: &mut Vec<u8>, id: &[u8], payload: &[u8]) {
    out.extend_from_slice(id);
    // Size vint: minimal width for the payload length.
    let len = payload.len();
    let mut vint = Vec::new();
    if len < 0x7F {
        vint.push(0x80 | len as u8);
    } else if len < 0x3FFF {
        vint.push(0x40 | (len >> 8) as u8);
        vint.push((len & 0xFF) as u8);
    } else {
        vint.push(0x20 | (len >> 16) as u8);
        vint.push((len >> 8) as u8);
        vint.push((len & 0xFF) as u8);
    }
    out.extend_from_slice(&vint);
    out.extend_from_slice(payload);
}

fn uint_payload(value: u64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let first = bytes.iter().position(|&b| b != 0).unwrap_or(7);
    bytes[first..].to_vec()
}

/// A minimal one-track matroska file whose three SimpleBlocks carry
/// Xiph, EBML and fixed lacing (2/3/3 frames).
fn laced_fixture() -> Arc<[u8]> {
    let mut out = Vec::new();
    // EBML header.
    let mut eh = Vec::new();
    el(&mut eh, &[0x42, 0x86], &[1]); // EBMLVersion
    el(&mut eh, &[0x42, 0xF7], &[1]); // EBMLReadVersion
    el(&mut eh, &[0x42, 0xF2], &[4]); // EBMLMaxIDLength
    el(&mut eh, &[0x42, 0xF3], &[8]); // EBMLMaxSizeLength
    el(&mut eh, &[0x42, 0x82], b"matroska"); // DocType
    el(&mut eh, &[0x42, 0x87], &[2]); // DocTypeVersion
    el(&mut eh, &[0x42, 0x85], &[2]); // DocTypeReadVersion
    el(&mut out, &[0x1A, 0x45, 0xDF, 0xA3], &eh);
    // Segment.
    let mut seg = Vec::new();
    // Info.
    let mut info = Vec::new();
    el(&mut info, &[0x2A, 0xD7, 0xB1], &uint_payload(1_000_000)); // TimestampScale
    el(&mut seg, &[0x15, 0x49, 0xA9, 0x66], &info);
    // Tracks: one audio track, lacing allowed.
    let mut entry = Vec::new();
    el(&mut entry, &[0xD7], &uint_payload(1)); // TrackNumber
    el(&mut entry, &[0x83], &uint_payload(2)); // TrackType = audio
    el(&mut entry, &[0x86], b"A_OPUS"); // CodecID
    el(&mut entry, &[0x9C], &uint_payload(1)); // FlagLacing
    let mut tracks = Vec::new();
    el(&mut tracks, &[0xAE], &entry);
    el(&mut seg, &[0x16, 0x54, 0xAE, 0x6B], &tracks);
    // Cluster: Timecode 0 + three laced SimpleBlocks.
    let mut cluster = Vec::new();
    el(&mut cluster, &[0xE7], &uint_payload(0)); // Timecode
    // Xiph: 3 frames — two size bytes (4, 5), the last frame is
    // the payload remainder (6).
    let mut xiph_body = vec![3u8, 4, 5];
    xiph_body.extend_from_slice(&[0xA1, 0xA2, 0xA3, 0xA4]);
    xiph_body.extend_from_slice(&[0xB1, 0xB2, 0xB3, 0xB4, 0xB5]);
    xiph_body.extend_from_slice(&[0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6]);
    let mut xiph_block = vec![0x81, 0x00, 0x00, 0x80 | 0x02]; // track 1, tc 0, keyframe+Xiph
    xiph_block.extend_from_slice(&xiph_body);
    el(&mut cluster, &[0xA3], &xiph_block);
    // EBML: 3 frames, sizes 3/3/remainder 2.
    let mut ebml_body = vec![3u8, 0x82, 0xBF];
    ebml_body.extend_from_slice(&[0xAA; 3]);
    ebml_body.extend_from_slice(&[0xBB; 3]);
    ebml_body.extend_from_slice(&[0xCC; 2]);
    let mut ebml_block = vec![0x81, 0x00, 0x08, 0x80 | 0x06]; // rel tc 8, keyframe+EBML
    ebml_block.extend_from_slice(&ebml_body);
    el(&mut cluster, &[0xA3], &ebml_block);
    // Fixed: 3 frames of 2 bytes each.
    let mut fixed_body = vec![3u8];
    fixed_body.extend_from_slice(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
    let mut fixed_block = vec![0x81, 0x00, 0x10, 0x80 | 0x04]; // rel tc 16, keyframe+fixed
    fixed_block.extend_from_slice(&fixed_body);
    el(&mut cluster, &[0xA3], &fixed_block);
    el(&mut seg, &[0x1F, 0x43, 0xB6, 0x75], &cluster);
    el(&mut out, &[0x18, 0x53, 0x80, 0x67], &seg);
    out.into()
}

#[test]
fn hand_built_laced_fixture_decodes_all_three_codings() {
    let bytes = laced_fixture();
    let mut demuxer = MkvDemuxer::new(mem_stream(bytes.clone()).expect("mem"))
        .expect("construct");
    let tracks = demuxer.start().expect("start");
    assert_eq!(tracks.len(), 1);
    assert_eq!(demuxer.doctype(), "matroska");
    let (total, per) = walk_all(&mut demuxer);
    assert_eq!(total, 9, "3 xiph + 3 ebml + 3 fixed frames");
    assert_eq!(per.len(), 1);
    assert_eq!(per[0].1, 9, "every frame a keyframe");
    assert_eq!(demuxer.damage_bytes(), 0, "no damage in the clean fixture");
}

#[test]
fn laced_frame_payloads_are_byte_exact() {
    let bytes = laced_fixture();
    let mut demuxer = MkvDemuxer::new(mem_stream(bytes.clone()).expect("mem"))
        .expect("construct");
    demuxer.start().expect("start");
    let mut payloads = Vec::new();
    let mut stamps = Vec::new();
    loop {
        match demuxer.next_event().expect("walk") {
            DemuxEvent::Packet(p) => {
                payloads.push(p.data.to_vec());
                if let Stamp::Present(ts) = &p.pts {
                    stamps.push(ts.ticks);
                }
            }
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
    }
    let expect: Vec<&[u8]> = vec![
        &[0xA1, 0xA2, 0xA3, 0xA4],
        &[0xB1, 0xB2, 0xB3, 0xB4, 0xB5],
        &[0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6],
        &[0xAA, 0xAA, 0xAA],
        &[0xBB, 0xBB, 0xBB],
        &[0xCC, 0xCC],
        &[0x11, 0x22],
        &[0x33, 0x44],
        &[0x55, 0x66],
    ];
    assert_eq!(payloads.len(), expect.len());
    for (got, want) in payloads.iter().zip(expect.iter()) {
        assert_eq!(got, want);
    }
    // Relative timecodes ride the block stamp (no DefaultDuration
    // declared → laced frames share it; the honest absence).
    assert!(stamps.iter().all(|&t| t == 0 || t == 8 || t == 16), "{stamps:?}");
}

// ---- Derivatives: fuzz-lite (never panic, never hang) -----------

#[test]
fn malformed_derivatives_are_typed_or_recover_never_panics() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/generated/malformed");
    let mut checked = 0usize;
    for name in [
        "mkv_h264_aac__trunc0.mkv",
        "mkv_h264_aac__trunc1.mkv",
        "mkv_h264_aac__trunc2.mkv",
        "mkv_h264_aac__trunc3.mkv",
        "mkv_h264_aac__trunc4.mkv",
        "mkv_h264_aac__flip1.mkv",
        "mkv_h264_aac__flip4.mkv",
        "mkv_h264_aac__flip16.mkv",
        "mkv_h264_aac__garbage0.mkv",
        "webm_vp9_opus__trunc0.webm",
        "webm_vp9_opus__trunc1.webm",
        "webm_vp9_opus__trunc2.webm",
        "webm_vp9_opus__trunc3.webm",
        "webm_vp9_opus__trunc4.webm",
        "webm_vp9_opus__flip1.webm",
        "webm_vp9_opus__flip4.webm",
        "webm_vp9_opus__flip16.webm",
        "webm_vp9_opus__garbage0.webm",
    ] {
        let path = dir.join(name);
        assert!(path.exists(), "{name} missing");
        let bytes: Arc<[u8]> = std::fs::read(&path).expect("read").into();
        let mut demuxer =
            MkvDemuxer::new(mem_stream(bytes.clone()).expect("mem")).expect("construct");
        match demuxer.start() {
            Err(e) => {
                // A typed refusal at open (trunc0's empty file, the
                // garbage head, a broken EBML header): stage Demux,
                // stable code, no panic.
                assert_eq!(e.stage, media_types::PipelineStage::Demux, "{name}: {e}");
                let code = e.context.code.as_str();
                assert!(code.starts_with("demux."), "{name}: {code}");
                println!("{name}: typed open refusal {code}");
            }
            Ok(tracks) => {
                let mut events = 0u64;
                let outcome = loop {
                    match demuxer.next_event() {
                        Ok(DemuxEvent::EndOfStream) => break "eos".to_string(),
                        Ok(_) => {}
                        Err(e) => {
                            let code = e.context.code.clone();
                            break code; // owned: outlives the loop arm
                        }
                    }
                    events += 1;
                    assert!(events < 1_000_000, "{name}: event storm");
                };
                println!(
                    "{name}: {} tracks, walk end={outcome}, damage={} events={events}",
                    tracks.len(),
                    demuxer.damage_bytes(),
                );
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 18);
}

// ---- The card's error paths at the demuxer level ----------------

#[test]
fn unknown_doctype_is_the_typed_refusal() {
    let mut bytes: Vec<u8> = laced_fixture().to_vec();
    // Rewrite the DocType payload in place: "matroska" → "ebml\0\0\0\0"
    // (same length, so every size stays valid).
    let at = bytes
        .windows(8)
        .position(|w| w == b"matroska")
        .expect("doctype payload");
    bytes[at..at + 8].copy_from_slice(b"ebml\0\0\0\0");
    let mut demuxer = MkvDemuxer::new(mem_stream(bytes).expect("mem"))
        .expect("construct");
    let err = demuxer.start().expect_err("unknown doctype refused");
    assert_eq!(err.context.code, "demux.mkv.unknown-doctype");
    assert_eq!(demuxer.doctype(), "");
}

#[test]
fn zero_head_is_the_typed_bad_vint() {
    let bytes = laced_fixture();
    let mut corrupt = vec![0x00];
    corrupt.extend_from_slice(&bytes[1..]);
    let mut demuxer =
        MkvDemuxer::new(mem_stream(corrupt.clone()).expect("mem")).expect("construct");
    let err = demuxer.start().expect_err("zero vint head refused");
    assert_eq!(err.context.code, "demux.mkv.not-matroska");
}

#[test]
fn truncated_cluster_is_the_typed_truncation() {
    let bytes = laced_fixture();
    // Cut mid-SimpleBlock payload (inside the Xiph block's data).
    let at = bytes
        .windows(3)
        .position(|w| w == &[0xA1, 0xA2, 0xA3])
        .expect("xiph frame data");
    let cut = &bytes[..at + 1];
    let mut demuxer = MkvDemuxer::new(mem_stream(cut.to_vec()).expect("mem"))
        .expect("construct");
    demuxer.start().expect("start survives (header intact)");
    let mut saw_typed = false;
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(_) => {}
            Err(e) => {
                assert_eq!(e.context.code, "demux.mkv.truncated");
                saw_typed = true;
                break;
            }
        }
    }
    assert!(saw_typed, "the cut block must surface, not vanish");
}

#[test]
fn probe_matches_the_ebml_magic() {
    assert!(demux::mkv::probe(&[0x1A, 0x45, 0xDF, 0xA3]));
    assert!(!demux::mkv::probe(&[0x1A, 0x45, 0xDF]));
    assert!(!demux::mkv::probe(b"RIFF"));
}

fn open(path: &std::path::Path) -> Box<dyn plugin_api::SourceStream> {
    open_via_input(path).expect("open fixture via input")
}
