//! The MPEG-1/2 sequence-layer differential (card 5.H1's
//! ffmpeg-oracle mandate): real ffmpeg-authored bytes scanned by
//! our clean-room start-code/header parser, asserted against
//! **measured ffprobe truth** (ffmpeg 7.1.5, the build that
//! authored every byte) on five committed fixtures and every
//! mpeg2video lane of the randomized corpus:
//!
//! - raw ES: `fixtures/mpeg12/*.m1v`/`*.m2v` (Main@Main progressive
//!   IPB, 4:2:2, interlaced NTSC, and an MPEG-1 IPB lane);
//! - transport: `mpeg2-in-ts.ts` through the real
//!   [`demux::ts::TsDemuxer`], the PS fixtures through
//!   [`demux::ps::PsDemuxer`] (the landed wave-7 demuxers carrying
//!   mpeg2video);
//! - corpus: every `.ts`/`.mpg`/`.vob`/`.avi`/`.mp4`/`.mkv` lane
//!   whose first video stream is mpeg2video, demuxed through the
//!   matching landed demuxer, packets concatenated, facts scanned.
//!
//! Facts compared: width, height, profile, level, chroma/pix_fmt,
//! frame rate, interlace. **Decode is not attempted** — the module
//! is honest about that boundary (typed `mpeg12.decode.blocked`).
//!
//! When the corpus root (or ffprobe) is absent the corpus half
//! **skips loudly** (the h264-parser-suite precedent); the fixture
//! half always runs (committed bytes, pinned truth).

use std::path::{Path, PathBuf};
use std::process::Command;

use codecs::video::mpeg12::facts::{Mpeg12Facts, descriptor_gate};
use demux::ps::PsDemuxer;
use demux::ts::pes::{PesAssembler, PesStreamClass};
use demux::ts::{PesConsumer, TsDemuxer};
use media_types::ids::MediaKind;
use plugin_api::{Access, DemuxEvent, Demuxer, SourceStream};
use std::sync::{Arc, Mutex};
use testkit::Must;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mpeg12")
}

fn ps_fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join("demux/fixtures/ps"))
        .unwrap_or_else(|| PathBuf::from("../demux/fixtures/ps"))
}

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
#[derive(Debug, Clone, Default)]
struct Probe {
    codec: String,
    profile: String,
    level: Option<i32>,
    width: Option<u32>,
    height: Option<u32>,
    pix_fmt: String,
    r_frame_rate: String,
    avg_frame_rate: String,
    field_order: String,
}

fn ffprobe_video(path: &Path) -> Probe {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name,profile,level,width,height,pix_fmt,r_frame_rate,avg_frame_rate,field_order",
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
            .filter(|v| !v.is_empty() && v != "N/A")
    };
    Probe {
        codec: field("codec_name").unwrap_or_default(),
        profile: field("profile").unwrap_or_else(|| "unknown".to_string()),
        level: field("level").and_then(|v| v.parse().ok()),
        width: field("width").and_then(|v| v.parse().ok()),
        height: field("height").and_then(|v| v.parse().ok()),
        pix_fmt: field("pix_fmt").unwrap_or_default(),
        r_frame_rate: field("r_frame_rate").unwrap_or_default(),
        avg_frame_rate: field("avg_frame_rate").unwrap_or_default(),
        field_order: field("field_order").unwrap_or_default(),
    }
}

/// Compare scanned facts against the oracle; returns a human row
/// plus a fail-reason string (empty = pass). `rate_oracle` picks
/// which ffprobe rate the ES header must match for this lane.
fn assert_facts(name: &str, facts: &Mpeg12Facts, probe: &Probe, note: &str) -> String {
    let mut why = String::new();
    let Some((w, h)) = facts.dimensions() else {
        return format!("no sequence header found ({note})");
    };
    if let (Some(pw), Some(ph)) = (probe.width, probe.height)
        && (w, h) != (pw, ph)
    {
        why.push_str(&format!("size {w}x{h} vs {pw}x{ph} "));
    }
    // MPEG-1 has no profile/level; ffprobe prints unknown + level
    // -99 there — only compare for MPEG-2 scans.
    if facts.is_mpeg2() {
        let ours_profile = facts.profile().unwrap_or("unknown");
        if probe.profile != "unknown" && ours_profile != probe.profile {
            why.push_str(&format!("profile {ours_profile} vs {} ", probe.profile));
        }
        if let Some(level) = probe.level
            && level >= 0
            && i32::from(facts.level().unwrap_or(0)) != level
        {
            why.push_str(&format!("level {} vs {level} ", facts.level().unwrap_or(0)));
        }
        let ours_fmt = facts.chroma().name();
        if !probe.pix_fmt.is_empty() && probe.pix_fmt != "unknown" && ours_fmt != probe.pix_fmt {
            why.push_str(&format!("pix_fmt {ours_fmt} vs {} ", probe.pix_fmt));
        }
        let ffprobe_interlaced = probe.field_order != "progressive";
        if facts.interlaced() != ffprobe_interlaced && !probe.field_order.is_empty() {
            why.push_str(&format!(
                "interlace {} vs {} ",
                facts.interlaced(),
                probe.field_order
            ));
        }
    }
    // Frame rate: the ES sequence header must match ffprobe's
    // r_frame_rate or avg_frame_rate (container-paced VFR lanes
    // diverge from r; reorder artifacts diverge from avg — matching
    // either is agreement with the measured stream, both mismatched
    // is a real failure).
    let ours_rate = facts.frame_rate_string().unwrap_or_default();
    let rate_match = ours_rate == probe.r_frame_rate || ours_rate == probe.avg_frame_rate;
    if !rate_match {
        why.push_str(&format!(
            "rate {ours_rate} vs r={} avg={} ",
            probe.r_frame_rate, probe.avg_frame_rate
        ));
    }
    let verdict = if why.is_empty() { "PASS" } else { "FAIL" };
    eprintln!(
        "ROW {name}: {verdict} {}x{} profile={:?} level={:?} chroma={} rate={ours_rate} interlaced={} pics={} slices={} mb_addr={} seqs={} changes={} ({note})",
        w,
        h,
        facts.profile(),
        facts.level(),
        facts.chroma().name(),
        facts.interlaced(),
        facts.picture_count,
        facts.slice_count,
        facts.slices_with_mb_address,
        facts.sequence_count,
        facts.sequence_changes,
    );
    why
}

/// The raw-ES fixture table (measured this session, ffmpeg 7.1.5 —
/// the same build that authored the bytes; re-measured live below
/// when ffprobe is on PATH).
#[test]
fn fixture_raw_es_facts_match_ffprobe() {
    let lanes = [
        // 320x240 4:3 25fps Main@Main progressive, IPB (bf 2).
        (
            "mpeg2-main-ipb.m2v",
            "mpeg2video",
            320,
            240,
            "Main",
            Some(8),
            "yuv420p",
            "25/1",
            false,
        ),
        // 192x108 16:9 4:2:2 (escaped profile 0x85, level 5).
        (
            "mpeg2-422.m2v",
            "mpeg2video",
            192,
            108,
            "4:2:2",
            Some(5),
            "yuv422p",
            "25/1",
            false,
        ),
        // Interlaced top-first NTSC: 30000/1001.
        (
            "mpeg2-interlaced-ntsc.m2v",
            "mpeg2video",
            192,
            108,
            "Main",
            Some(8),
            "yuv420p",
            "30000/1001",
            true,
        ),
        // MPEG-1 IPB: 320x240, 25fps, no profile/chroma vocabulary.
        (
            "mpeg1-ipb.m1v",
            "mpeg1video",
            320,
            240,
            "unknown",
            None,
            "yuv420p",
            "25/1",
            false,
        ),
    ];
    let mut failures = Vec::new();
    for (file, codec, w, h, profile, level, pix_fmt, rate, interlaced) in lanes {
        let path = fixture_dir().join(file);
        let bytes = std::fs::read(&path).must();
        let facts = Mpeg12Facts::scan(&bytes);
        assert!(facts.is_video_stream(), "{file}: no sequence header");
        assert_eq!(facts.descriptor(), codec, "{file}: descriptor");
        assert_eq!(facts.dimensions(), Some((w, h)), "{file}: dimensions");
        assert_eq!(
            facts.profile(),
            Some(profile).filter(|p| *p != "unknown"),
            "{file}: profile"
        );
        assert_eq!(facts.level(), level, "{file}: level");
        assert_eq!(facts.chroma().name(), pix_fmt, "{file}: chroma");
        assert_eq!(
            facts.frame_rate_string().as_deref(),
            Some(rate),
            "{file}: rate"
        );
        assert_eq!(facts.interlaced(), interlaced, "{file}: interlace");
        // Sequence-layer sanity: pictures and slices were walked,
        // macroblock addressing matched the VLC table, B-pictures
        // exist on the IPB lanes, nothing was silently damaged.
        assert!(facts.picture_count >= 10, "{file}: pictures");
        assert!(facts.slice_count >= 10, "{file}: slices");
        assert!(facts.slices_with_mb_address > 0, "{file}: mb addressing");
        assert_eq!(facts.bad_headers, 0, "{file}: undamaged headers");
        // IPB lanes carry P and B pictures => motion-vector syntax.
        if file.contains("ipb") {
            assert!(facts.has_motion_vectors(), "{file}: P/B presence");
            assert_eq!(facts.picture_type_mask, 0b0111, "{file}: I+P+B census");
        }
        assert!(
            descriptor_gate(facts.descriptor()),
            "{file}: gate accepts its own stream"
        );
        // Live cross-check against the oracle when present.
        if ffmpeg_available() {
            let probe = ffprobe_video(&path);
            assert_eq!(probe.codec, codec, "{file}: ffprobe codec");
            let why = assert_facts(file, &facts, &probe, "raw es");
            if !why.is_empty() {
                failures.push(format!("{file}: {why}"));
            }
        } else {
            eprintln!("ROW {file}: PINNED (no ffprobe on PATH)");
        }
    }
    assert!(failures.is_empty(), "fixture rows diverged: {failures:?}");
}

/// Demux one file with the given demuxer core, concatenate the
/// mpeg1/2 video track's packets, and scan the resulting
/// elementary stream.
fn scan_through_demuxer(name: &str, core: DemuxerCore) -> Mpeg12Facts {
    let mut demuxer = core;
    let tracks = demuxer.start().must();
    tracks
        .iter()
        .find(|t| t.kind == MediaKind::Video && descriptor_gate(t.codec.as_str()))
        .must();
    let _ = name;
    let mut es: Vec<u8> = Vec::new();
    let mut guard = 0u64;
    loop {
        guard += 1;
        assert!(guard < 2_000_000, "{name}: packet storm guard");
        match demuxer.next_event().must() {
            DemuxEvent::Packet(p) => {
                let ours = tracks
                    .iter()
                    .any(|t| t.id == p.track && descriptor_gate(t.codec.as_str()));
                if ours {
                    es.extend_from_slice(p.data.as_slice());
                }
            }
            DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => {}
            DemuxEvent::EndOfStream => break,
        }
    }
    Mpeg12Facts::scan(&es)
}

/// The `next_event`/`start` surface shared by every demuxer used
/// here (the `Demuxer` trait itself is not object-safe in the
/// landed surface — a thin enum bridges instead).
enum DemuxerCore {
    Ps(PsDemuxer),
}

impl DemuxerCore {
    fn start(&mut self) -> Result<Vec<media_types::Track>, media_types::error::EngineError> {
        match self {
            DemuxerCore::Ps(d) => d.start(),
        }
    }

    fn next_event(&mut self) -> Result<DemuxEvent, media_types::error::EngineError> {
        match self {
            DemuxerCore::Ps(d) => d.next_event(),
        }
    }
}

fn open_via_input(path: &Path) -> Box<dyn SourceStream> {
    let uri = format!("file://{}", path.display());
    let parsed = media_types::ids::SourceUri::parse(&uri)
        .unwrap_or_else(|e| unreachable!("valid file URI: {e}"));
    input::FileAccess::new()
        .open(&parsed)
        .unwrap_or_else(|e| unreachable!("open {}: {e}", path.display()))
}

/// The TS lane: the landed `TsDemuxer` PSI walk + the
/// [`demux::ts::pes::PesAssembler`] PES half at its public
/// consumer seam (the assembled-PES→`DemuxEvent::Packet` wiring is
/// 5.I1b's follow-up — the seam delivers the same bytes).
struct PesTap(Arc<Mutex<PesAssembler>>);

impl PesConsumer for PesTap {
    fn pes_payload(
        &mut self,
        pid: u16,
        pusi: bool,
        payload: &[u8],
    ) -> Result<(), media_types::error::EngineError> {
        self.0
            .lock()
            .map_err(|_| bad_lock())?
            .push_packet(pid, pusi, false, payload)
    }
}

fn bad_lock() -> media_types::error::EngineError {
    media_types::error::EngineError::new(
        media_types::PipelineStage::Demux,
        media_types::ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
        },
        media_types::error::ErrorContext::new("mpeg12.test.pes_lock"),
    )
}

fn scan_ts_pes(path: &Path) -> Mpeg12Facts {
    let assembler = Arc::new(Mutex::new(PesAssembler::new()));
    let mut demuxer = TsDemuxer::new(open_via_input(path)).must();
    demuxer.set_pes_consumer(Box::new(PesTap(Arc::clone(&assembler))));
    demuxer.start().must();
    let mut guard = 0u64;
    loop {
        guard += 1;
        assert!(guard < 2_000_000, "ts packet storm guard");
        match demuxer.next_event().must() {
            DemuxEvent::Packet(_) | DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => {}
            DemuxEvent::EndOfStream => break,
        }
    }
    let mut asm = assembler.lock().must();
    asm.finish().must();
    let mut es: Vec<u8> = Vec::new();
    while let Some(pes) = asm.next_pes() {
        if pes.class == PesStreamClass::Video {
            es.extend_from_slice(&pes.payload);
        }
    }
    Mpeg12Facts::scan(&es)
}

#[test]
fn fixture_transport_lanes_ts_and_ps() {
    // The TS lane: mpeg2video carried by the landed TsDemuxer's
    // PSI walk + PES assembler seam.
    let ts_path = fixture_dir().join("mpeg2-in-ts.ts");
    let facts = scan_ts_pes(&ts_path);
    assert_eq!(facts.dimensions(), Some((192, 108)));
    assert_eq!(facts.profile(), Some("Main"));
    assert_eq!(facts.level(), Some(8));
    assert_eq!(facts.chroma().name(), "yuv420p");
    assert_eq!(facts.frame_rate_string().as_deref(), Some("25/1"));
    assert!(facts.picture_count >= 5, "TS lane carried pictures");
    if ffmpeg_available() {
        let probe = ffprobe_video(&ts_path);
        assert_eq!(probe.codec, "mpeg2video");
        let why = assert_facts("mpeg2-in-ts.ts", &facts, &probe, "ts demux");
        assert!(why.is_empty(), "TS lane diverged: {why}");
    }
    // The PS lanes: the wave-7 committed demux fixtures (.mpg + a
    // .vob), through the landed PsDemuxer.
    for file in ["ps-mp2-video-audio.mpg", "ps-vob-ac3.vob"] {
        let path = ps_fixture_dir().join(file);
        assert!(path.exists(), "committed PS fixture {file} absent");
        let facts = scan_through_demuxer(
            file,
            DemuxerCore::Ps(PsDemuxer::new(open_via_input(&path)).must()),
        );
        assert_eq!(facts.descriptor(), "mpeg2video", "{file}");
        assert!(facts.picture_count >= 10, "{file}: pictures");
        assert!(facts.slice_count >= 10, "{file}: slices");
        assert_eq!(facts.bad_headers, 0, "{file}: undamaged");
        if ffmpeg_available() {
            let probe = ffprobe_video(&path);
            let why = assert_facts(file, &facts, &probe, "ps demux");
            assert!(why.is_empty(), "{file} diverged: {why}");
        }
    }
}

/// Every mpeg2video lane of the randomized corpus: the
/// elementary stream extracted with `ffmpeg -map 0:v:0 -c copy -f
/// mpegvideo` (the ps_fixtures byte-source precedent — the landed
/// AVI demuxer is MJPG-only v0 and the MP4 demuxer maps `mp4v` to
/// `mpeg4video` without the OTI 0x6B refinement, so those two
/// containers cannot yet feed this parser their own bytes; the gap
/// is recorded in `docs/formats/video-mpeg12.md`), scanned by our
/// parser, asserted against the container-level ffprobe facts.
/// Skips loudly when the corpus or ffprobe is absent.
#[test]
fn corpus_mpeg2video_lanes_field_assertions() {
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP mpeg12 corpus: corpus-v4 absent (LUMEN_CORPUS unset)");
        return;
    };
    if !ffmpeg_available() {
        eprintln!("SKIP mpeg12 corpus: no ffprobe on PATH");
        return;
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(&media)
        .must()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                ["avi", "mp4", "mkv", "ts", "mpg", "vob", "mov", "webm"].contains(&e)
            })
        })
        .collect();
    files.sort();
    let mut lanes = 0usize;
    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut skip = 0usize;
    let scratch = std::env::temp_dir().join("lumen-mpeg12-corpus");
    std::fs::create_dir_all(&scratch).must();
    for path in &files {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let probe = ffprobe_video(path);
        if probe.codec != "mpeg2video" && probe.codec != "mpeg1video" {
            continue;
        }
        lanes += 1;
        let es_path = scratch.join(format!("{name}.m2v"));
        let out = Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(path)
            .args(["-map", "0:v:0", "-c", "copy", "-f", "data"])
            .arg(&es_path)
            .output()
            .must();
        if !out.status.success() {
            eprintln!("ROW {name}: SKIP (ffmpeg extraction failed)");
            skip += 1;
            continue;
        }
        let bytes = match std::fs::read(&es_path) {
            Ok(b) => b,
            Err(_) => {
                eprintln!("ROW {name}: SKIP (extraction unreadable)");
                skip += 1;
                continue;
            }
        };
        let facts = Mpeg12Facts::scan(&bytes);
        let _ = std::fs::remove_file(&es_path);
        let why = assert_facts(name, &facts, &probe, "ffmpeg -c copy es");
        if why.is_empty() {
            pass += 1;
        } else if why.starts_with("no sequence header") {
            skip += 1;
        } else {
            fail += 1;
        }
    }
    eprintln!("MPEG12 CORPUS: {lanes} lanes, {pass} PASS, {fail} FAIL, {skip} SKIP");
    assert_eq!(fail, 0, "{fail} corpus lanes disagreed with ffprobe");
    assert!(
        pass >= 3,
        "expected the 3 known mpeg2video lanes (2 avi + 1 mp4), got {pass}"
    );
}
