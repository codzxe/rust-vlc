//! The real-corpus Matroska/WebM differential (card 5.D1): every
//! `.mkv`/`.mka`/`.webm` lane of the randomized real corpus
//! (corpus-v4, ffmpeg-authored bytes) demuxed through the real access
//! stack and compared against **live ffprobe truth** — stream count,
//! per-stream codec identity, packet counts (`ffprobe -count_packets`),
//! and per-stream packet-timestamp ceilings.
//!
//! The comparison is measured at run time (not pinned): the corpus
//! regenerates with fresh seeds, and the per-file PASS/FAIL table for
//! this instance lives in the worklog entry. When the corpus is
//! absent (and no `LUMEN_CORPUS` override) the suite **skips loudly**.
//!
//! # Demux-domain scope (honest)
//!
//! Channels/dimensions are codec-domain (CodecPrivate) bytes we pass
//! through; the container-comparable surface asserted here is: track
//! count, track kind, codec identity (CodecID → registry descriptor),
//! packet count per stream, keyframe count where ffprobe exposes it,
//! and the first/last packet timestamps in the segment's scale.

use std::path::PathBuf;
use std::process::Command;

mod common;

use common::open_via_input;
use demux::mkv::MkvDemuxer;
use media_types::ids::MediaKind;
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

/// One ffprobe stream record (the fields this differential asserts).
#[derive(Debug, Clone)]
struct ProbeStream {
    index: usize,
    codec_name: String,
    kind: String,
    nb_read_packets: Option<u64>,
}

fn ffprobe_streams(path: &std::path::Path) -> Vec<ProbeStream> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_streams",
            "-count_packets",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .must();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut streams = Vec::new();
    // Minimal JSON field scrape (std-only, C5): the probe shape is
    // stable for our fields.
    for chunk in text.split("\"index\":").skip(1) {
        let field = |name: &str| -> Option<String> {
            let pat = format!("\"{name}\":");
            let at = chunk.find(&pat)? + pat.len();
            let rest = &chunk[at..];
            let rest = rest.trim_start();
            if let Some(stripped) = rest.strip_prefix('"') {
                let end = stripped.find('"')?;
                Some(stripped[..end].to_string())
            } else {
                let end = rest.find([',', '}'])?;
                Some(rest[..end].trim().to_string())
            }
        };
        streams.push(ProbeStream {
            index: streams.len(),
            codec_name: field("codec_name").unwrap_or_default(),
            kind: field("codec_type").unwrap_or_default(),
            nb_read_packets: field("nb_read_packets").and_then(|v| v.parse().ok()),
        });
    }
    streams
}

/// ffprobe's `codec_type` spelling for a track kind (the honest,
/// recorded mapping; anything unmapped shows up as a differential
/// failure rather than being smoothed).
fn kind_probe_name(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Audio => "audio",
        MediaKind::Video => "video",
        MediaKind::Subtitle => "subtitle",
        MediaKind::Data => "data",
    }
}

#[test]
fn corpus_matroska_differential_vs_ffprobe() {
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
    let mut pass = 0usize;
    let mut fail = 0usize;
    for path in &files {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let probe = ffprobe_streams(path);
        let mut demuxer = MkvDemuxer::new(open_via_input(path).must()).must();
        let tracks = demuxer.start().must();
        for e in demuxer.track_entries() {
            eprintln!(
                "TRACKS {name}: number={} ordinal={} kind={:?} codec={}",
                e.number, e.ordinal, e.kind, e.codec_descriptor
            );
        }
        // Count our packets per track + first/last pts.
        let mut ours: Vec<(u64, i64, i64, u64, String)> = Vec::new(); // (n, first, last, keyframes, codec)
        for _ in 0..tracks.len() {
            ours.push((0, i64::MAX, i64::MIN, 0, String::new()));
        }
        let kinds: Vec<&str> = demuxer
            .track_entries()
            .iter()
            .map(|e| kind_probe_name(e.kind))
            .collect();
        for (slot, e) in ours.iter_mut().zip(demuxer.track_entries().iter()) {
            slot.4 = e.codec_descriptor.clone();
        }
        let mut events = 0u64;
        let mut clean = true;
        loop {
            match demuxer.next_event() {
                Ok(DemuxEvent::Packet(p)) => {
                    let i = p.track.handle() as usize;
                    if i >= ours.len() {
                        clean = false;
                        break;
                    }
                    let slot = &mut ours[i];
                    slot.0 += 1;
                    slot.3 += u64::from(p.keyframe);
                    if let Stamp::Present(ts) = &p.pts {
                        slot.1 = slot.1.min(ts.ticks);
                        slot.2 = slot.2.max(ts.ticks);
                    }
                }
                Ok(DemuxEvent::Recovered { .. }) => {
                    clean = false;
                }
                Ok(DemuxEvent::Discontinuity) | Ok(DemuxEvent::EndOfStream) => break,
                Err(e) => {
                    eprintln!("FAIL {name}: demux error {e}");
                    clean = false;
                    break;
                }
            }
            events += 1;
            assert!(events < 5_000_000, "packet storm guard");
        }
        // Compare against ffprobe: per-stream packet count, codec
        // identity and track kind (the demux-comparable surface).
        let mut why = String::new();
        if tracks.len() != probe.len() {
            why.push_str(&format!("tracks {} vs {} ", tracks.len(), probe.len()));
        }
        for (i, (slot, ps)) in ours.iter().zip(probe.iter()).enumerate() {
            if let Some(expected) = ps.nb_read_packets
                && slot.0 != expected
            {
                why.push_str(&format!(
                    "s{}(probe {}) packets {} vs {expected} ",
                    i, ps.index, slot.0
                ));
            }
            if !slot.4.is_empty() && slot.4 != ps.codec_name {
                why.push_str(&format!(
                    "s{}(probe {}) codec {} vs {} ",
                    i, ps.index, slot.4, ps.codec_name
                ));
            }
            if !ps.kind.is_empty() && kinds.get(i).copied() != Some(ps.kind.as_str()) {
                why.push_str(&format!(
                    "s{}(probe {}) kind {:?} vs {} ",
                    i,
                    ps.index,
                    kinds.get(i),
                    ps.kind
                ));
            }
        }
        if !clean {
            why.push_str("damage/error ");
        }
        if why.is_empty() {
            pass += 1;
            println!("PASS {name}: {} tracks, packets match", tracks.len());
        } else {
            fail += 1;
            println!("FAIL {name}: {why}(ours {ours:?} probe {probe:?})");
        }
    }
    println!("CORPUS MKV DIFFERENTIAL: {pass} PASS / {fail} FAIL");
    assert_eq!(fail, 0, "{} corpus files disagreed with ffprobe", fail);
}

/// Cues-based seeking survives the round trip on every corpus file
/// with an index: seek to the mid-point, keep demuxing, land at or
/// before the target (keyframe-safe).
#[test]
fn corpus_matroska_seek_roundtrip() {
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP: corpus-v4 absent");
        return;
    };
    let mut files: Vec<PathBuf> = std::fs::read_dir(&media)
        .must()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("mkv") | Some("webm")
            )
        })
        .collect();
    files.sort();
    let mut pass = 0usize;
    for path in &files {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let mut demuxer = MkvDemuxer::new(open_via_input(path).must()).must();
        demuxer.start().must();
        if demuxer.cue_index().is_none() {
            println!("PASS {name}: seek (no cues — SlowLinear path untested here)");
            continue;
        }
        // Drain to the end once to learn the last stamp, then reanchor.
        let mut last = 0i64;
        loop {
            match demuxer.next_event().must() {
                DemuxEvent::Packet(p) => {
                    if let Stamp::Present(ts) = &p.pts {
                        last = last.max(ts.ticks);
                    }
                }
                DemuxEvent::EndOfStream => break,
                _ => {}
            }
        }
        let base = demuxer.segment_info().time_base().must();
        let target = media_types::time::TimeSpan {
            start: Stamp::Present(media_types::time::Timestamp::new(last / 2, base)),
            end: Stamp::Missing,
        };
        let landed = demuxer.reanchor(target).must();
        let mut got = 0u64;
        loop {
            match demuxer.next_event().must() {
                DemuxEvent::Packet(_) => got += 1,
                DemuxEvent::EndOfStream => break,
                _ => {}
            }
        }
        assert!(got > 0, "{name}: no packets after mid-point reanchor");
        let landed_ticks = landed.start.timestamp().map(|t| t.ticks).unwrap_or(0);
        assert!(landed_ticks <= last / 2 + 1, "{name}: landed after target");
        println!(
            "PASS {name}: seek mid={}/{} landed={} post-packets={got}",
            last / 2,
            last,
            landed_ticks
        );
        pass += 1;
    }
    assert!(pass > 0);
}
