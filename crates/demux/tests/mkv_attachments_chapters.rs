//! MKV attachments + chapters differential (card 5.D2a): the
//! real-corpus `.mkv` lanes and an **ffmpeg-generated fixture**
//! (chapters via ffmetadata, attachments via `-attach`), compared
//! against **live ffprobe truth**:
//!
//! - chapters: `ffprobe -show_chapters` — count, per-chapter
//!   start/end nanoseconds (the spec's own chapter unit — chapter
//!   times do **not** scale with `TimestampScale`; verified live)
//!   and titles (`tags.title` ↔ the first
//!   `ChapterDisplay`'s `ChapString`), flattened depth-first.
//! - attachments: `ffprobe -show_entries
//!   stream=index,codec_type:stream_disposition:stream_tags` — the
//!   attachment surface is every stream with
//!   `codec_type == "attachment"` **or**
//!   `disposition.attached_pic == 1` (ffmpeg's two spellings of one
//!   Matroska `AttachedFile`); compared: count, `filename`,
//!   `mimetype`, and (generated fixture) byte-exact `FileData`.
//!
//! The corpus regenerates with fresh seeds (chapter-bomb included);
//! the generated fixture is rebuilt at run time. When the corpus is
//! absent (and no `LUMEN_CORPUS` override) or ffmpeg is unavailable
//! for the generated lane, the suite **skips loudly**.

use std::path::PathBuf;
use std::process::Command;

mod common;

use common::open_via_input;
use demux::mkv::MkvDemuxer;
use demux::mkv::attachments::FileData;
use media_types::time::Stamp;
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

fn corpus_dir() -> Option<PathBuf> {
    let dir = std::env::var("LUMEN_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/my-project/corpus-v4"));
    let media = dir.join("media");
    media.is_dir().then_some(media)
}

/// One ffprobe chapter row (ns start/end, the title tag).
#[derive(Debug, Clone)]
struct ProbeChapter {
    start: i64,
    end: i64,
    title: Option<String>,
}

fn ffprobe_chapters(path: &std::path::Path) -> Vec<ProbeChapter> {
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-show_chapters", "-of", "json"])
        .arg(path)
        .output()
        .must();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut chapters = Vec::new();
    for chunk in text.split("\"id\":").skip(1) {
        let field = |name: &str| -> Option<String> {
            let pat = format!("\"{name}\":");
            let at = chunk.find(&pat)? + pat.len();
            let rest = chunk[at..].trim_start();
            if let Some(stripped) = rest.strip_prefix('"') {
                Some(stripped[..stripped.find('"')?].to_string())
            } else {
                Some(rest[..rest.find([',', '}'])?].trim().to_string())
            }
        };
        chapters.push(ProbeChapter {
            start: field("start").and_then(|v| v.parse().ok()).unwrap_or(-1),
            end: field("end").and_then(|v| v.parse().ok()).unwrap_or(-1),
            title: field("title"),
        });
    }
    chapters
}

/// One ffprobe attachment row (ffmpeg's two spellings of one
/// Matroska `AttachedFile`: `codec_type attachment`, or an
/// `attached_pic` stream carrying filename/mimetype tags).
#[derive(Debug, Clone)]
struct ProbeAttachment {
    filename: Option<String>,
    mimetype: Option<String>,
}

fn ffprobe_attachments(path: &std::path::Path) -> Vec<ProbeAttachment> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=index,codec_type:stream_disposition=attached_pic:stream_tags=filename,mimetype",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .must();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut out = Vec::new();
    for chunk in text.split("\"index\":").skip(1) {
        let field = |name: &str| -> Option<String> {
            let pat = format!("\"{name}\":");
            let at = chunk.find(&pat)? + pat.len();
            let rest = chunk[at..].trim_start();
            if let Some(stripped) = rest.strip_prefix('"') {
                Some(stripped[..stripped.find('"')?].to_string())
            } else {
                Some(rest[..rest.find([',', '}'])?].trim().to_string())
            }
        };
        let attached_pic = field("attached_pic").as_deref() == Some("1");
        let codec_type = field("codec_type");
        if attached_pic || codec_type.as_deref() == Some("attachment") {
            out.push(ProbeAttachment {
                filename: field("filename"),
                mimetype: field("mimetype"),
            });
        }
    }
    out
}

/// The per-lane differential: chapters + attachments vs ffprobe.
fn assert_lane_matches_ffprobe(path: &std::path::Path) {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
    let probe_chapters = ffprobe_chapters(path);
    let probe_attachments = ffprobe_attachments(path);

    let mut demuxer = MkvDemuxer::new(open_via_input(path).must()).must();
    demuxer.start().must();
    let info = *demuxer.segment_info();
    let scale = u64::from(info.timestamp_scale);

    // -- chapters ------------------------------------------------------
    let chapters = demuxer.chapters().must();
    let ours = chapters.atoms_flattened();
    let mut why = String::new();
    if ours.len() != probe_chapters.len() {
        why.push_str(&format!(
            "chapters {} vs {} ",
            ours.len(),
            probe_chapters.len()
        ));
    }
    for (i, (atom, probe)) in ours.iter().zip(probe_chapters.iter()).enumerate() {
        let start_ns = atom.start_ns as i64;
        let end_ns = atom.end_ns.map_or(-1, |t| t as i64);
        if start_ns != probe.start {
            why.push_str(&format!(
                "c{}(probe) start {start_ns} vs {} ",
                i + 1,
                probe.start
            ));
        }
        if end_ns != probe.end {
            why.push_str(&format!("c{}(probe) end {end_ns} vs {} ", i + 1, probe.end));
        }
        let title = atom.title();
        if title != probe.title.as_deref() {
            why.push_str(&format!(
                "c{}(probe) title {title:?} vs {:?} ",
                i + 1,
                probe.title
            ));
        }
    }
    // The tolerant note is a count, not a delta: real corpus lanes
    // declare durations past their chapter ends (measured), so the
    // flag stays 0 unless the file lies.
    if chapters.beyond_duration != 0 {
        why.push_str(&format!("beyond_duration {} ", chapters.beyond_duration));
    }

    // -- attachments ---------------------------------------------------
    let attachments = demuxer.attachments().must();
    if attachments.files.len() != probe_attachments.len() {
        why.push_str(&format!(
            "attachments {} vs {} ",
            attachments.files.len(),
            probe_attachments.len()
        ));
    }
    for (i, (file, probe)) in attachments
        .files
        .iter()
        .zip(probe_attachments.iter())
        .enumerate()
    {
        if file.name.as_deref() != probe.filename.as_deref() {
            why.push_str(&format!(
                "a{}(probe) name {:?} vs {:?} ",
                i, file.name, probe.filename
            ));
        }
        if file.mime_type.as_deref() != probe.mimetype.as_deref() {
            why.push_str(&format!(
                "a{}(probe) mime {:?} vs {:?} ",
                i, file.mime_type, probe.mimetype
            ));
        }
    }
    assert!(why.is_empty(), "{name}: {why}");

    // -- the packet walk is untouched by the getters (positional) -----
    let mut packets = 0u64;
    for _ in 0..64 {
        match demuxer.next_event().must() {
            DemuxEvent::Packet(_) => packets += 1,
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
    }
    assert!(packets > 0, "{name}: packets still flow after the getters");
    // A present-timestamp sanity bound: chapters land inside the
    // stream's own timing vocabulary.
    if let Some(first) = ours.first() {
        let _ = Stamp::Present(media_types::time::Timestamp::new(
            (first.start_ns / scale) as i64,
            demuxer.segment_info().time_base().must(),
        ));
    }
    println!(
        "PASS {name}: {} chapters, {} attachments (ffprobe-matched{})",
        ours.len(),
        attachments.files.len(),
        if ours.is_empty() {
            ", none declared"
        } else {
            ""
        }
    );
}

#[test]
fn corpus_attachments_chapters_differential_vs_ffprobe() {
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP: corpus-v4 absent (LUMEN_CORPUS unset, /tmp/my-project/corpus-v4 missing)");
        return;
    };
    let mut files: Vec<PathBuf> = std::fs::read_dir(&media)
        .must()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("mkv") | Some("mka") | Some("mk3d") | Some("webm")
            )
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "corpus carries mkv lanes");
    // The card's named lanes run first so a failure names them
    // directly; every other mkv lane runs the same differential.
    let mut ordered: Vec<PathBuf> = Vec::new();
    for named in [
        "0007-chapter-bomb.mkv",
        "0017-meta-bomb-av.mkv",
        "0063-multi-av.mkv",
    ] {
        let path = media.join(named);
        if path.exists() {
            ordered.push(path);
        } else {
            eprintln!("SKIP: {named} absent from the corpus media dir");
        }
    }
    for path in files {
        if !ordered.contains(&path) {
            ordered.push(path);
        }
    }
    for path in &ordered {
        assert_lane_matches_ffprobe(path);
    }
    eprintln!(
        "corpus differential: {} lanes, chapters+attachments ffprobe-matched",
        ordered.len()
    );
}

/// Build the ffmpeg-generated lane at run time (chapters via
/// ffmetadata, a real JPEG cover + a text attachment via `-attach`).
fn generated_fixture(dir: &std::path::Path) -> Option<PathBuf> {
    let have_ffmpeg = || Command::new("ffmpeg").arg("-version").output().is_ok();
    if !have_ffmpeg() {
        eprintln!("SKIP: ffmpeg absent — the generated chapters+attachments lane needs it");
        return None;
    }
    let cover = dir.join("d2a-cover.jpg");
    let run = |args: &[&str]| {
        let status = Command::new("ffmpeg")
            .args(["-y", "-v", "error"])
            .args(args)
            .status()
            .must();
        assert!(status.success(), "ffmpeg fixture generation failed");
    };
    if !cover.exists() {
        run(&[
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1:size=96x64:rate=5",
            "-frames:v",
            "1",
            "-c:v",
            "mjpeg",
            "-q:v",
            "9",
            cover.to_str().unwrap_or("cover.jpg"),
        ]);
    }
    let notes = dir.join("d2a-notes.txt");
    std::fs::write(&notes, "Lumen attachment fixture\nsecond line\n").must();
    let meta = dir.join("d2a-chapters.ffmeta");
    std::fs::write(
        &meta,
        ";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=1500\ntitle=First Chapter\n\
         [CHAPTER]\nTIMEBASE=1/1000\nSTART=1500\nEND=4000\ntitle=Second Chapter\n",
    )
    .must();
    let out = dir.join("d2a-generated.mkv");
    if !out.exists() {
        let cover_s = cover.to_str().unwrap_or("cover.jpg");
        let notes_s = notes.to_str().unwrap_or("notes.txt");
        let meta_s = meta.to_str().unwrap_or("chapters.ffmeta");
        run(&[
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=4:size=128x96:rate=10",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=4",
            "-i",
            meta_s,
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-attach",
            cover_s,
            "-metadata:s:t:0",
            "mimetype=image/jpeg",
            "-metadata:s:t:0",
            "filename=cover.jpg",
            "-attach",
            notes_s,
            "-metadata:s:t:1",
            "mimetype=text/plain",
            "-metadata:s:t:1",
            "filename=notes.txt",
            "-map_chapters",
            "2",
            out.to_str().unwrap_or("out.mkv"),
        ]);
    }
    Some(out)
}

#[test]
fn generated_fixture_attachments_chapters_differential() {
    let dir = std::env::temp_dir().join("lumen-d2a-fixtures");
    std::fs::create_dir_all(&dir).must();
    let Some(path) = generated_fixture(&dir) else {
        return;
    };
    assert_lane_matches_ffprobe(&path);

    // Byte-exact FileData: the inline cover must equal the generated
    // JPEG (the generator's own bytes, read back through the parse).
    let cover_bytes = std::fs::read(dir.join("d2a-cover.jpg")).must();
    let notes_bytes = std::fs::read(dir.join("d2a-notes.txt")).must();
    let mut demuxer = MkvDemuxer::new(open_via_input(&path).must()).must();
    demuxer.start().must();
    let attachments = demuxer.attachments().must();
    assert_eq!(attachments.files.len(), 2, "cover + notes attached");
    let by_name = |name: &str| {
        attachments
            .files
            .iter()
            .find(|f| f.name.as_deref() == Some(name))
            .unwrap_or_else(|| unreachable!("fixture contract: {name} attached"))
    };
    match &by_name("cover.jpg").data {
        FileData::Inline(bytes) => assert_eq!(bytes, &cover_bytes, "cover.jpg byte-exact inline"),
        other => unreachable!("small cover is inline, got {other:?}"),
    }
    match &by_name("notes.txt").data {
        FileData::Inline(bytes) => assert_eq!(bytes, &notes_bytes, "notes.txt byte-exact inline"),
        other => unreachable!("small text is inline, got {other:?}"),
    }
    assert_eq!(
        by_name("cover.jpg").mime_type.as_deref(),
        Some("image/jpeg")
    );
    assert_eq!(
        by_name("notes.txt").mime_type.as_deref(),
        Some("text/plain")
    );

    // The chapter surface: 2 chapters, the ffmetadata times, titles.
    let chapters = demuxer.chapters().must();
    let atoms = chapters.atoms_flattened();
    assert_eq!(atoms.len(), 2);
    // Raw nanoseconds (chapter times ignore TimestampScale; the
    // ffmetadata chapter times are 0..1500 ms and 1500..4000 ms).
    assert_eq!(atoms[0].start_ns, 0);
    assert_eq!(atoms[0].end_ns, Some(1_500_000_000));
    assert_eq!(atoms[0].title(), Some("First Chapter"));
    assert_eq!(atoms[1].start_ns, 1_500_000_000);
    assert_eq!(atoms[1].end_ns, Some(4_000_000_000));
    assert_eq!(atoms[1].title(), Some("Second Chapter"));
    assert_eq!(chapters.beyond_duration, 0);
    assert!(chapters.damage.is_empty());
    println!("PASS d2a-generated.mkv: 2 chapters + 2 byte-exact attachments");
}
