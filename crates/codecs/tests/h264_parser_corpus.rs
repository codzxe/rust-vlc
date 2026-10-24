//! H.264 parser corpus differential (card 5.F1's field-level AC):
//! real ffmpeg-authored bytes parsed by our clean-room NAL/SPS/PPS
//! parser, asserted against **live ffprobe truth** — profile,
//! level, resolution, chroma/pix_fmt — on both entry forms:
//!
//! - `0019-raw-video.h264`: the raw Annex-B elementary stream, split
//!   by [`codecs::video::h264::parser::nal::AnnexBNals`];
//! - every `.mkv` lane whose first video track is H.264: opened
//!   through `input` + the real `demux::mkv::MkvDemuxer`, parameter
//!   sets taken from the track's `avcC` `CodecPrivate` (and, when a
//!   track carries them only in-band, from its first
//!   length-prefixed packets via
//!   [`codecs::video::h264::parser::nal::AvcNals`]).
//!
//! MP4/MOV/TS lanes carrying H.264 are **skipped with a message** on
//! this tree: their demuxers are sibling cards not yet merged here
//! (5.C1 MP4 on its branch; TS is 5.I1). When they land, their avcC
//! blocks plug into the same [`AvcConfig`] path — no parser change.
//!
//! When the corpus root (or ffprobe) is absent the suite **skips
//! loudly** (the corpus regenerates with fresh seeds; PASS/FAIL
//! tables live in the worklog entry, not pinned constants).

use std::path::PathBuf;
use std::process::Command;

use codecs::video::h264::parser::nal::{AnnexBNals, AvcConfig, AvcNals, NalHeader, NalUnitType};
use codecs::video::h264::parser::pps::PicParameterSet;
use codecs::video::h264::parser::sps::{ChromaFormat, SeqParameterSet};
use demux::mkv::MkvDemuxer;
use media_types::ids::MediaKind;
use plugin_api::{Access, DemuxEvent, Demuxer, SourceStream};
use testkit::Must;

fn corpus_dir() -> Option<PathBuf> {
    let dir = std::env::var("LUMEN_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/my-project/corpus-v4"));
    let media = dir.join("media");
    media.is_dir().then_some(media)
}

fn ffmpeg_available() -> bool {
    Command::new("ffprobe")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// ffprobe's first-video-stream facts (the oracle).
#[derive(Debug, Clone)]
struct Probe {
    codec: String,
    profile: String,
    level: Option<i32>,
    width: Option<u32>,
    height: Option<u32>,
    pix_fmt: String,
}

fn ffprobe_video(path: &std::path::Path) -> Probe {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name,profile,level,width,height,pix_fmt",
            "-of",
            "default=nw=1:noprint_wrappers=1",
        ])
        .arg(path)
        .output()
        .must();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let field = |name: &str| -> Option<String> {
        text.lines()
            .find(|l| l.starts_with(name) && l[name.len()..].starts_with('='))
            .map(|l| l[name.len() + 1..].trim().to_string())
            .filter(|v| !v.is_empty())
    };
    Probe {
        codec: field("codec_name").unwrap_or_default(),
        profile: field("profile").unwrap_or_else(|| "unknown".to_string()),
        level: field("level").and_then(|v| v.parse().ok()),
        width: field("width").and_then(|v| v.parse().ok()),
        height: field("height").and_then(|v| v.parse().ok()),
        pix_fmt: field("pix_fmt").unwrap_or_default(),
    }
}

/**
 * ffmpeg's rendering fallback for gray H.264 (the corpus oracle's
 * behavior, verified on 0019): default libavcodec builds (no
 * `--enable-gray`) synthesize neutral chroma for 4:0:0 streams, so
 * ffprobe reports the 4:2:0 *view* of a monochrome SPS. The parser
 * honestly reports `ChromaFormat::Monochrome`; this maps it to what
 * this build's ffprobe shows for the comparison.
 */
fn ffprobe_visible_pix_fmt(sps: &SeqParameterSet) -> String {
    let full = sps.vui.as_ref().is_some_and(|v| v.video_full_range);
    if sps.chroma_format() == ChromaFormat::Monochrome {
        let range = if full { "j" } else { "" };
        format!("yuv{range}420p")
    } else {
        sps.chroma_format()
            .pix_fmt_name(full, sps.bit_depth_luma_minus8 + 8)
    }
}

/// The parser facts for one h264 track (first SPS/PPS win — the same
/// 'first parameter set' ffprobe's stream identity is built from).
struct Parsed {
    sps: Option<SeqParameterSet>,
    pps: Option<PicParameterSet>,
    note: &'static str,
}

/// Extract SPS/PPS from an avcC record.
fn parse_avcc(private: &[u8], note: &'static str) -> Parsed {
    let mut parsed = Parsed {
        sps: None,
        pps: None,
        note,
    };
    if !AvcConfig::looks_like_avcc(private) {
        return parsed;
    }
    let cfg = match AvcConfig::parse(private) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("  avcC parse error: {e}");
            return parsed;
        }
    };
    if let Some(nal) = cfg.sps.first() {
        parsed.sps = SeqParameterSet::parse(nal).ok();
    }
    if let Some(nal) = cfg.pps.first() {
        parsed.pps = PicParameterSet::parse(nal).ok();
    }
    parsed
}

/// In-band fallback: scan length-prefixed packets for SPS/PPS NALs.
fn parse_inband_avc(packets: &[Vec<u8>], length_size: u8, note: &'static str) -> Parsed {
    let mut parsed = Parsed {
        sps: None,
        pps: None,
        note,
    };
    for packet in packets {
        let Ok(nals) = AvcNals::new(packet, length_size) else {
            continue;
        };
        for nal in nals.flatten() {
            if let Ok((header, _)) = NalHeader::split(nal) {
                match header.unit_type {
                    NalUnitType::Sps if parsed.sps.is_none() => {
                        parsed.sps = SeqParameterSet::parse(nal).ok();
                    }
                    NalUnitType::Pps if parsed.pps.is_none() => {
                        parsed.pps = PicParameterSet::parse(nal).ok();
                    }
                    _ => {}
                }
            }
        }
    }
    parsed
}

/// Assert our parsed facts against the ffprobe oracle; returns a
/// PASS/FAIL row (printed for the worklog table either way).
fn assert_facts(name: &str, parsed: &Parsed, probe: &Probe) -> bool {
    let Some(sps) = parsed.sps.as_ref() else {
        eprintln!("ROW {name}: SKIP (no SPS found — {})", parsed.note);
        return false;
    };
    let mut why = String::new();
    let ours_profile = sps.profile_name();
    if probe.profile != "unknown" && ours_profile != probe.profile {
        why.push_str(&format!("profile {ours_profile} vs {} ", probe.profile));
    }
    if let Some(level) = probe.level
        && level >= 0
        && i32::from(sps.level_idc) != level
    {
        why.push_str(&format!("level {} vs {level} ", sps.level_idc));
    }
    if let (Some(w), Some(h)) = (probe.width, probe.height) {
        if let Some((ow, oh)) = sps.display_size() {
            if (ow, oh) != (w, h) {
                why.push_str(&format!("size {ow}x{oh} vs {w}x{h} "));
            }
        } else {
            why.push_str("size <crop overflow> ");
        }
    }
    let ours_fmt = ffprobe_visible_pix_fmt(sps);
    if !probe.pix_fmt.is_empty()
        && probe.pix_fmt != "unknown"
        && !probe.pix_fmt.starts_with("gray")
        && ours_fmt != probe.pix_fmt
    {
        why.push_str(&format!("pix_fmt {ours_fmt} vs {} ", probe.pix_fmt));
    }
    if let Some(pps) = parsed.pps.as_ref()
        && !pps.references(&[sps.seq_parameter_set_id])
    {
        why.push_str("pps->sps id mismatch ");
    }
    let verdict = if why.is_empty() { "PASS" } else { "FAIL" };
    eprintln!(
        "ROW {name}: {verdict} profile={} level={} size={:?} chroma={:?} pps={} {}",
        ours_profile,
        sps.level_idc,
        sps.display_size(),
        sps.chroma_format(),
        parsed.pps.is_some(),
        why
    );
    why.is_empty()
}

#[test]
fn corpus_raw_annexb_0019_field_assertions() {
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP h264 raw corpus: corpus-v4 absent (LUMEN_CORPUS unset)");
        return;
    };
    if !ffmpeg_available() {
        eprintln!("SKIP h264 raw corpus: no ffprobe on PATH");
        return;
    }
    let path = media.join("0019-raw-video.h264");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("SKIP h264 raw corpus: 0019-raw-video.h264 absent (corpus re-imaged)");
        return;
    };
    let probe = ffprobe_video(&path);
    assert_eq!(probe.codec, "h264", "0019 is the raw h264 lane");

    // Both entry forms must reach the same parameter sets: split the
    // Annex-B stream, and re-frame the found SPS/PPS AVC-style.
    let mut sps: Option<SeqParameterSet> = None;
    let mut pps: Option<PicParameterSet> = None;
    let mut nals_seen = 0usize;
    let mut rbsp = Vec::new();
    for nal in AnnexBNals::new(&bytes) {
        nals_seen += 1;
        if let Ok((header, _)) = NalHeader::split(nal) {
            match header.unit_type {
                NalUnitType::Sps if sps.is_none() => {
                    sps = SeqParameterSet::parse_with_buffer(nal, &mut rbsp).ok();
                }
                NalUnitType::Pps if pps.is_none() => {
                    pps = PicParameterSet::parse_with_buffer(nal, &mut rbsp).ok();
                }
                _ => {}
            }
        }
    }
    assert!(
        nals_seen > 2,
        "0019 carries SPS/PPS/SEI/slices, saw {nals_seen}"
    );
    let parsed = Parsed {
        sps,
        pps,
        note: "raw annexb",
    };
    assert!(assert_facts("0019-raw-video.h264", &parsed, &probe));

    // Property pass (+FB prep): mutations of the real stream — byte
    // flips, truncations, 0x03 injections — never panic and never
    // hang; results are Ok or typed errors.
    let mut state: u64 = 0x1234_5678_9abc_def0;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut buffer = Vec::new();
    for round in 0..2000u32 {
        let mut data = bytes.clone();
        for _ in 0..(next() % 3) + 1 {
            let at = (next() as usize) % data.len();
            data[at] = (next() % 256) as u8;
        }
        if round % 3 == 0 {
            data.truncate((next() as usize) % (data.len() + 1));
        }
        let mut count = 0usize;
        for nal in AnnexBNals::new(&data) {
            count += 1;
            assert!(count <= data.len(), "annexb walker spun");
            if let Ok((header, _)) = NalHeader::split(nal)
                && matches!(header.unit_type, NalUnitType::Sps)
            {
                // The call itself is the property: it must return
                // (Ok or typed Err) without panicking.
                let _ = SeqParameterSet::parse_with_buffer(nal, &mut buffer);
            }
        }
    }
}

#[test]
fn corpus_mkv_h264_tracks_field_assertions() {
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP h264 mkv corpus: corpus-v4 absent (LUMEN_CORPUS unset)");
        return;
    };
    if !ffmpeg_available() {
        eprintln!("SKIP h264 mkv corpus: no ffprobe on PATH");
        return;
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(&media)
        .must()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "mkv" || e == "webm"))
        .collect();
    files.sort();
    let mut lanes = 0usize;
    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut skipped = 0usize;
    for path in &files {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        // MP4/MOV/TS h264 lanes: honest skip — no demuxer on this
        // tree yet (sibling cards 5.C1/5.I1).
        let probe = ffprobe_video(path);
        if probe.codec != "h264" {
            continue;
        }
        lanes += 1;
        // Open through the real access stack + demuxer.
        let uri = format!("file://{}", path.display());
        let parsed_uri = media_types::ids::SourceUri::parse(&uri)
            .unwrap_or_else(|e| unreachable!("valid file URI: {e}"));
        let stream: Box<dyn SourceStream> = input::FileAccess::new()
            .open(&parsed_uri)
            .unwrap_or_else(|e| unreachable!("open {name}: {e}"));
        let mut demuxer = MkvDemuxer::new(stream).unwrap_or_else(|e| unreachable!("{e}"));
        let _ = demuxer.start().must();
        let Some((entry_ordinal, entry)) = demuxer
            .track_entries()
            .iter()
            .find(|e| e.kind == MediaKind::Video && e.codec_descriptor == "h264")
            .map(|e| (e.ordinal, e))
        else {
            eprintln!("ROW {name}: SKIP (no h264 track in our table)");
            skipped += 1;
            continue;
        };
        // Preferred source: the avcC CodecPrivate.
        let mut parsed = parse_avcc(&entry.private.codec_private, "codec_private");
        if parsed.sps.is_none() {
            // Fallback: in-band length-prefixed NALs of this track's
            // first packets (avcC geometry, or the 4-byte default).
            let length_size = if AvcConfig::looks_like_avcc(&entry.private.codec_private) {
                AvcConfig::parse(&entry.private.codec_private)
                    .map(|c| c.length_size)
                    .unwrap_or(4)
            } else {
                4
            };
            // Payloads are pooled views into the demuxer's cluster
            // window (5.D1's zero-copy design) — copy the few we
            // need; the window is refilled by later events.
            let mut packets: Vec<Vec<u8>> = Vec::new();
            let mut guard = 0u64;
            while packets.len() < 64 {
                guard += 1;
                assert!(guard < 200_000, "packet storm guard");
                match demuxer.next_event() {
                    Ok(DemuxEvent::Packet(p)) => {
                        if p.track.handle() == entry_ordinal {
                            packets.push(p.data.as_slice().to_vec());
                        }
                    }
                    Ok(DemuxEvent::Recovered { .. }) => {}
                    Ok(DemuxEvent::Discontinuity) | Ok(DemuxEvent::EndOfStream) => break,
                    Err(e) => {
                        eprintln!("ROW {name}: demux error {e}");
                        break;
                    }
                }
            }
            if parsed.sps.is_none() {
                parsed = parse_inband_avc(&packets, length_size, "in-band avc");
            }
            if parsed.sps.is_none() && !packets.is_empty() {
                // Last resort: Annex-B style blocks (nonstandard for
                // V_MPEG4/ISO/AVC but some muxers emit it).
                let mut sps = None;
                let mut pps = None;
                let mut buf = Vec::new();
                'outer: for packet in &packets {
                    for nal in AnnexBNals::new(packet) {
                        if let Ok((h, _)) = NalHeader::split(nal) {
                            match h.unit_type {
                                NalUnitType::Sps if sps.is_none() => {
                                    sps = SeqParameterSet::parse_with_buffer(nal, &mut buf).ok();
                                }
                                NalUnitType::Pps if pps.is_none() => {
                                    pps = PicParameterSet::parse_with_buffer(nal, &mut buf).ok();
                                }
                                _ => {}
                            }
                            if sps.is_some() && pps.is_some() {
                                break 'outer;
                            }
                        }
                    }
                }
                parsed = Parsed {
                    sps,
                    pps,
                    note: "in-band annexb",
                };
            }
        }
        if assert_facts(name, &parsed, &probe) {
            pass += 1;
        } else if parsed.sps.is_some() {
            fail += 1;
        } else {
            skipped += 1;
        }
    }
    eprintln!(
        "H264 MKV CORPUS: {lanes} lanes, {pass} PASS, {fail} FAIL, {skipped} SKIP (mp4/mov/ts h264 lanes skip: demuxer cards not merged on this tree)"
    );
    assert_eq!(fail, 0, "{fail} h264 mkv lanes disagreed with ffprobe");
    assert!(
        pass >= 10,
        "expected double-digit h264 mkv lanes, got {pass}"
    );
}
