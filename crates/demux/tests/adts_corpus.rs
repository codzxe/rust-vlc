//! The real-corpus ADTS/ADIF differential (card 5.C2b, the user's
//! standing mandate): the two raw-AAC lanes of corpus-v4
//! (ffmpeg-authored bytes) demuxed through the real access stack and
//! compared against the **measured ffmpeg truth** — stream identity
//! and packet counts from `ffprobe -show_packets`, the full payload
//! digest from `ffmpeg -map 0:a -c copy -f data -` (byte-identical to
//! the `-f adts` roundtrip dump), per-frame timestamps from ffprobe's
//! `pts_time` column.
//!
//! Every pinned value below was measured this session from the
//! corpus-v4 instance at `/tmp/my-project/corpus-v4/media` (ffmpeg
//! 7.1.5 / Lavf61.7.103, the same build that authored the corpus):
//! ffprobe reports `codec_name=aac, profile=LC` — the demuxer's
//! profile-honest descriptor is `aac-lc` (the mapping row lives in
//! the worklog). When the corpus is absent (and no `LUMEN_CORPUS`
//! override is set) this suite **skips loudly**; the per-file
//! PASS/FAIL table for this instance is in the worklog entry.
//!
//! # The dirty lane (measured, honestly divergent)
//!
//! `dirty/tail-garbage-003-0059-audio-only.aac` is 0059 verbatim plus
//! 39 757 random bytes. Measured ffprobe truth: 351 packets — ffmpeg
//! accepts mid-scan resync candidates without re-checking layer or
//! sampling index (the frame at +1299 bytes declares index 15, the
//! one at +7978 declares layer 2) and emits the final overrunning
//! frame as a **short 3405-byte packet**. This demuxer recovers the
//! same four complete garbage frames (the measured acceptance
//! predicate) but refuses the overrunning tail as the typed sticky
//! truncation (the Ogg precedent) — 350 packets, honestly recorded in
//! the worklog table as the one intentional divergence.
//!
//! # The ADIF lane (no ffmpeg reference exists)
//!
//! ffmpeg has neither an ADIF muxer nor demuxer (measured:
//! `-muxers`/`-demuxers` list only `adts`/`aac`), so the generated
//! ADIF differential is the **cross-container** parity of the unit
//! suite (`adts_and_adif_agree_on_the_same_payload`): the same
//! essence wrapped as ADTS frames and as a CBR ADIF stream must
//! demux to identical payload bytes. The honest note is in the
//! worklog.

use std::path::{Path, PathBuf};

mod common;

use common::{md5_hex, open_via_input};
use demux::adts::{AdtsDemuxer, SAMPLES_PER_BLOCK};
use media_types::time::Stamp;
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

/// One corpus file's ffmpeg-measured truth.
struct CorpusTruth {
    /// File name inside the corpus media dir.
    file: &'static str,
    /// The track's time base as (num, den) — one tick = one sample.
    base: (u32, u32),
    /// ffprobe `codec_name,profile` → the demuxer's descriptor.
    codec: &'static str,
    /// Demuxer-side channels (the ADTS header's declaration).
    channels: u16,
    /// `ffprobe -show_packets` packet count.
    packets: u64,
    /// `ffmpeg -map 0:a -c copy -f data -` (the raw frame dump; the
    /// `-f adts` roundtrip of both lanes is byte-identical to the
    /// source file, measured).
    md5: &'static str,
    /// ffprobe-measured `pts_time` spot values (seconds) at frames
    /// 0 / 100 / last — the per-frame series is the `i·1024/rate`
    /// invariant asserted below.
    pts_spots: [(u64, f64); 3],
    /// This lane's exact duration by frame arithmetic (ffprobe's own
    /// format-level duration is a bitrate *estimate*: 4.083052 vs the
    /// exact 4.017052 on 0059 — recorded in the worklog).
    duration_nanos: u64,
}

const TRUTHS: [CorpusTruth; 2] = [
    CorpusTruth {
        file: "0059-audio-only.aac",
        base: (1, 88_200),
        codec: "aac-lc",
        channels: 1,
        packets: 346,
        md5: "9c5542e88a9ff8baa2eb15ba488c8354",
        pts_spots: [(0, 0.0), (100, 1.160998), (345, 4.005442)],
        duration_nanos: 4_017_052_154, // 346 × 1024 / 88200 s
    },
    CorpusTruth {
        file: "0110-audio-only.aac",
        base: (1, 96_000),
        codec: "aac-lc",
        channels: 1,
        packets: 376,
        md5: "693bead87996e2d0aada5ee9f56ea4e7",
        pts_spots: [(0, 0.0), (200, 2.133333), (375, 4.0)],
        duration_nanos: 4_010_666_666, // 376 × 1024 / 96000 s
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

fn open_demuxer(path: &Path) -> AdtsDemuxer {
    let mut demuxer = AdtsDemuxer::new(open_via_input(path).must()).must();
    demuxer.start().must();
    demuxer
}

#[test]
fn corpus_adts_lanes_match_the_ffmpeg_truth() {
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "corpus(adts): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for the 2026-10-24 instance is in the worklog)"
        );
        return;
    };
    let mut table = Vec::new();
    for truth in &TRUTHS {
        let path = dir.join(truth.file);
        assert!(path.is_file(), "corpus file missing: {path:?}");
        let mut demuxer = open_demuxer(&path);

        // Stream identity: one audio track, profile-honest descriptor,
        // the header's rate/channels, the sample clock base.
        let tracks = demuxer.start().must();
        assert_eq!(tracks.len(), 1, "{}", truth.file);
        let track = &tracks[0];
        assert_eq!(track.kind, media_types::MediaKind::Audio, "{}", truth.file);
        assert_eq!(track.codec.as_str(), truth.codec, "{}", truth.file);
        assert_eq!(
            track.timing.base.as_parts(),
            truth.base,
            "{}: the sample clock",
            truth.file
        );
        let info = demuxer.stream_info().must();
        assert_eq!(
            (info.format, info.rate, info.channels),
            ("adts", truth.base.1, truth.channels),
            "{}: ffprobe-side rate/channels",
            truth.file
        );

        // Drain: payload parity with ffmpeg's raw dump, packet count
        // with ffprobe, the pts invariant for EVERY frame, clean walk.
        let mut payload = Vec::new();
        let mut pts_ticks = Vec::new();
        let mut reasons = Vec::new();
        let mut terminal = String::new();
        loop {
            match demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    match packet.pts {
                        Stamp::Present(t) => pts_ticks.push(t.ticks),
                        Stamp::Missing => unreachable!("{}: every frame is stamped", truth.file),
                    }
                    payload.extend_from_slice(&packet.data);
                }
                Ok(DemuxEvent::Recovered { reason, .. }) => reasons.push(reason),
                Ok(DemuxEvent::Discontinuity) => {}
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => {
                    terminal = err.context.code.to_owned();
                    break;
                }
            }
        }
        let digest = md5_hex(&payload);
        assert_eq!(
            digest, truth.md5,
            "{}: payload parity vs ffmpeg raw dump",
            truth.file
        );
        assert_eq!(
            pts_ticks.len() as u64,
            truth.packets,
            "{}: packet count",
            truth.file
        );
        assert_eq!(demuxer.frames_demuxed(), truth.packets, "{}", truth.file);
        assert!(
            terminal.is_empty(),
            "{}: clean EOS (got {terminal})",
            truth.file
        );
        assert!(
            reasons.is_empty(),
            "{}: no damage ({reasons:?})",
            truth.file
        );
        assert_eq!(demuxer.damage_total(), 0, "{}", truth.file);

        // The per-frame timestamp series: pts(i) = i · 1024 samples
        // (ffprobe's pts_time column is exactly this arithmetic — the
        // spot values pin the measured series at both ends + middle).
        for (i, ticks) in pts_ticks.iter().enumerate() {
            assert_eq!(
                *ticks,
                (i as u64 * SAMPLES_PER_BLOCK) as i64,
                "{}: frame {i} pts",
                truth.file
            );
        }
        for (frame, ffprobe_secs) in truth.pts_spots {
            let ticks = pts_ticks[frame as usize];
            let nanos =
                u128::from(ticks.unsigned_abs()) * 1_000_000_000u128 / u128::from(truth.base.1);
            let diff = (nanos as f64 / 1e9 - ffprobe_secs).abs();
            assert!(
                diff <= 0.000_001,
                "{}: frame {frame} pts {} vs ffprobe {}",
                truth.file,
                nanos as f64 / 1e9,
                ffprobe_secs
            );
        }
        assert_eq!(
            demuxer.track_duration().must(),
            media_types::time::Duration::Known(media_types::time::Nanoseconds(
                truth.duration_nanos
            )),
            "{}: exact frame arithmetic",
            truth.file
        );

        table.push(format!(
            "PASS {} codec={} rate={} ch={} frames={} md5={} bytes={}",
            truth.file,
            truth.codec,
            truth.base.1,
            truth.channels,
            pts_ticks.len(),
            digest,
            payload.len()
        ));
    }
    for row in &table {
        eprintln!("corpus(adts): {row}");
    }
    eprintln!(
        "corpus(adts): {}/{} lanes PASS the ffmpeg differential",
        table.len(),
        TRUTHS.len()
    );
}

#[test]
fn corpus_adts_seek_roundtrip() {
    let Some(dir) = corpus_dir() else {
        eprintln!("corpus(adts): corpus absent — SKIP (seek roundtrip)");
        return;
    };
    for truth in &TRUTHS {
        let mut demuxer = open_demuxer(&dir.join(truth.file));
        // Mid-stream target: 60% of the exact duration.
        let target_nanos = truth.duration_nanos * 6 / 10;
        let ticks = target_nanos as i64 * i64::from(truth.base.1) / 1_000_000_000;
        let target = media_types::time::TimeSpan {
            start: Stamp::Present(media_types::time::Timestamp::new(
                ticks,
                track_base(&mut demuxer),
            )),
            end: Stamp::Missing,
        };
        let landing = demuxer.seek(target).must();
        let landed = match landing.start {
            Stamp::Present(t) => t,
            Stamp::Missing => unreachable!("{}: landing stamped", truth.file),
        };
        assert!(
            landed.ticks <= ticks,
            "{}: at-or-before the target ({} > {})",
            truth.file,
            landed.ticks,
            ticks
        );
        demuxer.reanchor(target).must();
        let mut post = 0u64;
        let mut last_pts = landed.ticks;
        loop {
            match demuxer.next_event() {
                Ok(DemuxEvent::Packet(p)) => {
                    if let Stamp::Present(t) = p.pts {
                        assert!(t.ticks >= last_pts, "{}: monotonic after seek", truth.file);
                        last_pts = t.ticks;
                    }
                    post += 1;
                }
                Ok(DemuxEvent::EndOfStream) | Err(_) => break,
                Ok(_) => {}
            }
        }
        assert!(
            post > truth.packets / 4,
            "{}: real packets after the reanchor ({post})",
            truth.file
        );
        eprintln!(
            "corpus(adts): {} seek mid={ticks} landed={} post-packets={post}",
            truth.file, landed.ticks
        );
    }
}

fn track_base(demuxer: &mut AdtsDemuxer) -> media_types::time::TimeBase {
    let tracks = demuxer.start().must();
    tracks[0].timing.base
}

/// The dirty lane: 0059 + 39 757 random tail bytes. The 346 real
/// frames demux byte-exact; the garbage tail recovers ffmpeg's four
/// complete resync frames and refuses the overrunning fifth as the
/// typed sticky truncation (the one honest divergence — ffmpeg emits
/// it as a short 3405-byte packet; measured, recorded in the worklog).
#[test]
fn corpus_dirty_tail_garbage_is_honest() {
    let Some(dir) = corpus_dir() else {
        eprintln!("corpus(adts): corpus absent — SKIP (dirty lane)");
        return;
    };
    let dirty = dir
        .parent()
        .map(|root| root.join("dirty/tail-garbage-003-0059-audio-only.aac"))
        .filter(|p| p.is_file());
    let Some(path) = dirty else {
        eprintln!("corpus(adts): no dirty aac lane this instance — SKIP");
        return;
    };
    let mut demuxer = open_demuxer(&path);
    let mut payload = Vec::new();
    let mut recovered = Vec::new();
    let mut terminal = String::new();
    let mut packets = 0u64;
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(p)) => {
                packets += 1;
                payload.extend_from_slice(&p.data);
            }
            Ok(DemuxEvent::Recovered { skipped, reason }) => recovered.push((skipped, reason)),
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => {
                terminal = err.context.code.to_owned();
                break;
            }
        }
    }
    // The real audio is never corrupted: the first 346 frames are
    // byte-identical to the clean lane (0059's full-file digest).
    let clean_md5 = TRUTHS[0].md5;
    assert_eq!(md5_hex(&payload[..130_920]), clean_md5);
    assert_eq!(packets, 350, "346 real + 4 complete garbage frames");
    assert!(
        !recovered.is_empty(),
        "the garbage tail is recovered, not silent"
    );
    assert!(
        demuxer.damage_total() > 0,
        "damage is counted, never hidden"
    );
    assert_eq!(
        terminal.as_str(),
        "demux.adts.truncated",
        "the overrunning fifth garbage frame refuses typed"
    );
    // Sticky: the truncation replays.
    let again = demuxer.next_event();
    assert_eq!(
        again.err().map(|e| e.context.code),
        Some("demux.adts.truncated".to_string())
    );
    eprintln!(
        "corpus(adts): PASS dirty/tail-garbage-003 346-exact + 4 recovered, typed truncation \
         (ffprobe: 351 — the short-packet divergence, worklog)"
    );
}

/// Every corpus ADTS lane (and the dirty/garbage siblings) survives
/// the fuzz harness — the `+FB` no-panic/no-hang/bounded-memory
/// contract over real-world bytes — plus deterministic
/// truncation/flip mutation sweeps over the two real lanes (the
/// committed `fixtures/adts/` derivatives pin the same contract when
/// the corpus is absent).
#[test]
fn corpus_adts_survive_the_fuzz_harness() {
    let Some(dir) = corpus_dir() else {
        eprintln!("corpus(adts): corpus absent — SKIP (fuzz sweep)");
        return;
    };
    for truth in &TRUTHS {
        let bytes = std::fs::read(dir.join(truth.file)).must();
        demux::adts::fuzz::fuzz_demux_adts(&bytes);
        // Deterministic mutation sweeps over the real bytes: every
        // 4 KiB-stride truncation prefix and single-bit flip.
        for cut in (0..bytes.len()).step_by(4096) {
            demux::adts::fuzz::fuzz_demux_adts(&bytes[..cut]);
        }
        for off in (0..bytes.len()).step_by(4096) {
            let mut mutated = bytes.clone();
            mutated[off] ^= 0x01;
            demux::adts::fuzz::fuzz_demux_adts(&mutated);
        }
    }
    // The dirty/garbage siblings (any bytes are legal fuzz input).
    if let Some(root) = dir.parent() {
        for subdir in ["dirty", "garbage"] {
            let sub = root.join(subdir);
            let Ok(entries) = std::fs::read_dir(&sub) else {
                continue;
            };
            let mut swept = 0usize;
            for entry in entries {
                let Ok(entry) = entry else {
                    continue;
                };
                let path = entry.path();
                if path.is_file() {
                    let Ok(bytes) = std::fs::read(&path) else {
                        continue;
                    };
                    demux::adts::fuzz::fuzz_demux_adts(&bytes);
                    swept += 1;
                }
            }
            eprintln!("corpus(adts): fuzz sweep over {subdir}/: {swept} files");
        }
    }
}
