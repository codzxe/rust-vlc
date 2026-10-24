//! HEVC parser corpus differential (card 5.G1's field-level AC):
//! real ffmpeg-authored bytes parsed by our clean-room
//! NAL/VPS/SPS/PPS/slice-prefix parser, asserted against **live
//! ffprobe truth** — profile, tier, level, resolution, chroma/pix_fmt
//! — on both entry forms:
//!
//! - `0044-raw-video.265`: the raw Annex-B elementary stream, split
//!   by [`codecs::video::hevc::parser::nal::AnnexBNals`];
//! - `0091-bframe-storm.ts`: the HEVC-in-TS lane. The TS demuxer is
//!   a sibling card not merged here, so the lane enters through the
//!   mission-sanctioned extraction (`ffmpeg -c copy -f hevc`) and
//!   its facts are compared against ffprobe of the original
//!   container.
//!
//! The corpus's two HEVC-in-AVI lanes (0038/0099) are **skipped with
//! a message**: ffprobe itself reports codec `unknown` for them (the
//! AVI+HEVC fourcc hack), so there is no oracle to differ against.
//!
//! When the corpus root (or ffmpeg) is absent the suite **skips
//! loudly** (the corpus regenerates with fresh seeds; PASS/FAIL
//! tables live in the worklog entry, not pinned constants).

use std::path::PathBuf;
use std::process::Command;

use codecs::video::hevc::parser::nal::{AnnexBNals, NalHeader, NalUnitType};
use codecs::video::hevc::parser::pps::PicParameterSet;
use codecs::video::hevc::parser::slice::{
    SliceSegmentHeader, SliceSegmentPpsInfo, SliceSegmentSpsInfo,
};
use codecs::video::hevc::parser::sps::SeqParameterSet;
use codecs::video::hevc::parser::vps::VideoParameterSet;
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

/// The parser facts for one HEVC lane (first parameter sets win —
/// the same 'first parameter set' ffprobe's stream identity is built
/// from).
struct Parsed<'a> {
    vps: Option<VideoParameterSet>,
    sps: Option<SeqParameterSet>,
    pps: Option<PicParameterSet>,
    /// The first VCL NAL (type + bytes) for the slice-prefix walk.
    first_vcl: Option<(NalUnitType, &'a [u8])>,
    /// The parsed slice-segment prefix of `first_vcl`.
    slice: Option<SliceSegmentHeader>,
    nals: usize,
}

fn parse_annexb(bytes: &'_ [u8]) -> Parsed<'_> {
    let mut parsed = Parsed {
        vps: None,
        sps: None,
        pps: None,
        first_vcl: None,
        slice: None,
        nals: 0,
    };
    for nal in AnnexBNals::new(bytes) {
        parsed.nals += 1;
        let Ok((h, _)) = NalHeader::split(nal) else {
            continue;
        };
        match h.unit_type {
            NalUnitType::VpsNut if parsed.vps.is_none() => {
                parsed.vps = VideoParameterSet::parse(nal).ok();
            }
            NalUnitType::SpsNut if parsed.sps.is_none() => {
                parsed.sps = SeqParameterSet::parse(nal).ok();
            }
            NalUnitType::PpsNut if parsed.pps.is_none() => {
                parsed.pps = PicParameterSet::parse(nal).ok();
            }
            _ if h.unit_type.is_vcl() && parsed.first_vcl.is_none() => {
                parsed.first_vcl = Some((h.unit_type, nal));
            }
            _ => {}
        }
    }
    // The first VCL NAL's slice-segment prefix over the derived
    // SPS/PPS seams.
    if let (Some(sps), Some(pps), Some((_, nal))) = (&parsed.sps, &parsed.pps, parsed.first_vcl)
        && let Ok(seam_sps) = SliceSegmentSpsInfo::from_sps(sps)
    {
        let seam_pps = SliceSegmentPpsInfo::from_pps(pps);
        parsed.slice = SliceSegmentHeader::parse(nal, &seam_sps, &seam_pps).ok();
    }
    parsed
}

/// Assert our parsed facts against the ffprobe oracle; returns a
/// PASS/FAIL row (printed for the worklog table either way).
/// `require_slice`: elementary-stream lanes must also yield the
/// slice-segment prefix; a bare `hvcC` record carries none.
fn assert_facts(name: &str, parsed: &Parsed, probe: &Probe, require_slice: bool) -> bool {
    let Some(sps) = parsed.sps.as_ref() else {
        eprintln!("ROW {name}: FAIL (no SPS found)");
        return false;
    };
    let mut why = String::new();
    let ptl = &sps.profile_tier_level;
    let ours_profile = ptl.profile_name();
    if probe.profile != "unknown" && ours_profile != probe.profile {
        why.push_str(&format!("profile {ours_profile} vs {} ", probe.profile));
    }
    if let Some(level) = probe.level
        && level >= 0
        && i32::from(ptl.level_idc) != level
    {
        why.push_str(&format!("level {} vs {level} ", ptl.level_idc));
    }
    if let (Some(w), Some(h)) = (probe.width, probe.height) {
        match sps.cropped_size() {
            Ok((ow, oh)) if (ow, oh) != (w, h) => {
                why.push_str(&format!("size {ow}x{oh} vs {w}x{h} "));
            }
            Err(e) => why.push_str(&format!("size <{e}> ")),
            Ok(_) => {}
        }
    }
    let full = sps.vui.as_ref().is_some_and(|v| v.video_full_range);
    let ours_fmt = sps
        .chroma_format
        .pix_fmt_name(full, sps.bit_depth_luma_minus8 + 8);
    if !probe.pix_fmt.is_empty() && probe.pix_fmt != "unknown" && ours_fmt != probe.pix_fmt {
        why.push_str(&format!("pix_fmt {ours_fmt} vs {} ", probe.pix_fmt));
    }
    let tier = if ptl.tier { "High" } else { "Main" };
    if parsed.pps.as_ref().is_none() {
        why.push_str("no PPS ");
    }
    if require_slice && parsed.slice.as_ref().is_none() {
        why.push_str("no slice prefix ");
    }
    let verdict = if why.is_empty() { "PASS" } else { "FAIL" };
    eprintln!(
        "ROW {name}: {verdict} profile={ours_profile} tier={tier} level={} size={:?} chroma={ours_fmt} depth={} poc_bits={} pps={} slice={:?} {}",
        ptl.level_idc,
        sps.cropped_size(),
        sps.bit_depth_luma_minus8 + 8,
        u32::from(sps.log2_max_pic_order_cnt_lsb_minus4) + 4,
        parsed.pps.is_some(),
        parsed
            .slice
            .as_ref()
            .map(|s| (s.first_slice_segment_in_pic, s.slice_type)),
        why
    );
    why.is_empty()
}

#[test]
fn corpus_raw_annexb_0044_ffprobe_field_assertions() {
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP hevc raw corpus: corpus-v4 absent (LUMEN_CORPUS unset)");
        return;
    };
    if !ffmpeg_available() {
        eprintln!("SKIP hevc raw corpus: no ffprobe on PATH");
        return;
    }
    let path = media.join("0044-raw-video.265");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("SKIP hevc raw corpus: 0044-raw-video.265 absent (corpus re-imaged)");
        return;
    };
    let probe = ffprobe_video(&path);
    assert_eq!(probe.codec, "hevc", "0044 is the raw hevc lane");
    let parsed = parse_annexb(&bytes);
    assert!(parsed.nals > 3, "0044 carries more than parameter sets");
    assert!(assert_facts("0044-raw-video.265", &parsed, &probe, true));
    // The honest-subset slice prefix on the first VCL NAL.
    let slice = parsed.slice.as_ref().must();
    assert!(
        slice.first_slice_segment_in_pic,
        "a stream opens with the first segment of a picture"
    );
    let pps = parsed.pps.as_ref().must();
    assert_eq!(slice.slice_pic_parameter_set_id, pps.pic_parameter_set_id);

    // Property pass (+FB prep): mutations of the real stream — byte
    // flips, truncations, 0x03 injections — never panic and never
    // hang; results are Ok or typed errors.
    let mut seed: u64 = 0xfeed_face_cafe_beef;
    let mut next = move || {
        seed = seed
            .wrapping_mul(0x2545_f491_4f6c_dd1d)
            .wrapping_add(0x9e37);
        seed ^ (seed >> 27)
    };
    let mut buffer = Vec::new();
    let mut sps_seen = 0usize;
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
                && matches!(
                    header.unit_type,
                    NalUnitType::SpsNut | NalUnitType::VpsNut | NalUnitType::PpsNut
                )
            {
                // The call itself is the property: it must return
                // (Ok or typed Err) without panicking.
                let _ = SeqParameterSet::parse_with_buffer(nal, &mut buffer);
                let _ = VideoParameterSet::parse_with_buffer(nal, &mut buffer);
                let _ = PicParameterSet::parse_with_buffer(nal, &mut buffer);
                sps_seen += 1;
            }
        }
    }
    assert!(sps_seen > 100, "mutations must keep hitting deep syntax");
}

#[test]
fn corpus_ts_0091_ffprobe_field_assertions_via_extraction() {
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP hevc ts corpus: corpus-v4 absent (LUMEN_CORPUS unset)");
        return;
    };
    if !ffmpeg_available() {
        eprintln!("SKIP hevc ts corpus: no ffprobe on PATH");
        return;
    }
    let path = media.join("0091-bframe-storm.ts");
    if !path.is_file() {
        eprintln!("SKIP hevc ts corpus: 0091-bframe-storm.ts absent (corpus re-imaged)");
        return;
    }
    let probe = ffprobe_video(&path);
    assert_eq!(probe.codec, "hevc", "0091 is the hevc-in-ts lane");
    // The TS demuxer is a sibling card: enter through the sanctioned
    // extraction instead (`ffmpeg -c copy -f hevc`).
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(&path)
        .args(["-c", "copy", "-f", "hevc", "pipe:1"])
        .output();
    let Ok(out) = out else {
        eprintln!("SKIP hevc ts corpus: ffmpeg could not run");
        return;
    };
    if !out.status.success() || out.stdout.is_empty() {
        eprintln!(
            "SKIP hevc ts corpus: extraction failed (status {:?}, {} bytes)",
            out.status,
            out.stdout.len()
        );
        return;
    }
    let parsed = parse_annexb(&out.stdout);
    assert!(parsed.nals > 3, "0091 extraction carries a slice train");
    assert!(assert_facts(
        "0091-bframe-storm.ts (extracted ES)",
        &parsed,
        &probe,
        true
    ));
    let slice = parsed.slice.as_ref().must();
    assert!(slice.first_slice_segment_in_pic);
}

#[test]
fn corpus_mp4_0100_hvcc_field_assertions_via_ffprobe() {
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP hevc hvcc corpus: corpus-v4 absent (LUMEN_CORPUS unset)");
        return;
    };
    if !ffmpeg_available() {
        eprintln!("SKIP hevc hvcc corpus: no ffprobe on PATH");
        return;
    }
    let path = media.join("0100-mp4-video.mp4");
    if !path.is_file() {
        eprintln!("SKIP hevc hvcc corpus: 0100-mp4-video.mp4 absent (corpus re-imaged)");
        return;
    }
    let probe = ffprobe_video(&path);
    assert_eq!(probe.codec, "hevc", "0100 is the hevc-in-mp4 lane");
    // The MP4 demuxer is a sibling card: pull the real `hvcC`
    // (extradata) through ffprobe's hex dump instead.
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=extradata",
            "-show_data",
            "-of",
            "default=nw=1",
        ])
        .arg(&path)
        .output()
        .must();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut record: Vec<u8> = Vec::new();
    for line in text.lines() {
        if let Some((_, rest)) = line.split_once(": ") {
            // The hex column comes before the double-space ASCII
            // column; its groups are two bytes each (4 hex chars).
            let hex_part = rest.split("  ").next().unwrap_or("");
            for group in hex_part.split(' ').filter(|p| !p.is_empty()) {
                match group.len() {
                    // Full groups are two bytes; the dump's final
                    // line can end in a single byte.
                    4 => {
                        if let (Ok(hi), Ok(lo)) = (
                            u8::from_str_radix(&group[..2], 16),
                            u8::from_str_radix(&group[2..], 16),
                        ) {
                            record.push(hi);
                            record.push(lo);
                        }
                    }
                    2 => {
                        if let Ok(b) = u8::from_str_radix(group, 16) {
                            record.push(b);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    assert!(
        record.len() > 23,
        "no hvcC extradata hex parsed from ffprobe ({} bytes)",
        record.len()
    );
    let cfg = codecs::video::hevc::parser::nal::HvcCConfig::parse(&record).must();
    // Field-exact vs the live oracle: Main / level 93 / 4:2:0 8-bit.
    assert_eq!(cfg.profile_space, 0);
    assert!(!cfg.tier);
    assert_eq!(cfg.profile_idc, 1);
    assert_eq!(cfg.level_idc, 93);
    assert_eq!(cfg.chroma_format, 1);
    assert_eq!(cfg.bit_depth_luma_minus8, 0);
    assert_eq!(cfg.bit_depth_chroma_minus8, 0);
    assert_eq!(cfg.length_size, 4);
    assert_eq!(cfg.num_temporal_layers, 1);
    // The real record carries 4 arrays: VPS, SPS, PPS, prefix SEI.
    assert_eq!(cfg.arrays.len(), 4);
    assert_eq!(
        cfg.arrays
            .iter()
            .map(|a| a.nal_unit_type)
            .collect::<Vec<_>>(),
        vec![32, 33, 34, 39]
    );
    assert!(cfg.arrays.iter().all(|a| a.nals.len() == 1));
    // The arrays must parse as the parameter sets they claim.
    let sps = SeqParameterSet::parse(&cfg.arrays[1].nals[0]).must();
    let pps = PicParameterSet::parse(&cfg.arrays[2].nals[0]).must();
    let vps = VideoParameterSet::parse(&cfg.arrays[0].nals[0]).must();
    assert_eq!(sps.cropped_size().must(), (1024, 768));
    assert_eq!(sps.profile_tier_level, vps.profile_tier_level);
    assert_eq!(pps.seq_parameter_set_id, sps.seq_parameter_set_id);
    assert!(assert_facts(
        "0100-mp4-video.mp4 (hvcC via ffprobe)",
        &Parsed {
            vps: Some(vps),
            sps: Some(sps),
            pps: Some(pps),
            first_vcl: None,
            slice: None,
            nals: 4,
        },
        &probe,
        false
    ));
}
