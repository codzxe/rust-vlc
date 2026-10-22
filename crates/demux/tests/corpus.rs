//! Randomized real-media corpus suite — the AVI demux surface (ED-FIX-5;
//! the user's randomized-corpus directive: real ffmpeg-authored AVIs at
//! randomized codecs/resolutions/fps, plus corrupted and garbage
//! siblings, with ffprobe as the external oracle).
//!
//! For every AVI-container case the demuxer is driven exactly the way the
//! engine drives it: open through `input`'s `FileAccess`, `start`,
//! `next_event` to end. The suite records **measured numbers** per file —
//! track count, per-kind packet counts and byte totals, recovery events,
//! damage totals, first/last video PTS span, wall time — plus the typed
//! error code when the walk ends in a failure. Judgement against the
//! oracle (ffprobe's stream/packet/duration truth) is the aggregator's
//! job; this suite fails only on contract violations (panic, missing
//! corpus file, empty handed case list).
//!
//! Dirty cases carry the D-3 burden of proof: a corrupted AVI must walk
//! to a typed terminal (bounded damage, recovery events, or a clean
//! `Corrupt` error) — never a panic, never a hang.
//!
//! Activation (the harness driver runs exactly this):
//!
//! ```text
//! LUMEN_CORPUS=<dir> LUMEN_CORPUS_CASES=<tsv> LUMEN_CORPUS_RESULTS=<tsv> \
//!   cargo test --release -p demux --test corpus -- --ignored --nocapture
//! ```

use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use demux::avi::AviDemuxer;
use media_types::EngineError;
use media_types::ids::TrackId;
use media_types::packet::Packet;
use plugin_api::{Access, DemuxEvent, Demuxer, SourceStream};
use testkit::Must;

/// Open a corpus file through the real access stack (card 2.A1's seam).
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

/// One row of the harness case list (TSV).
struct AviCase {
    rel: String,
    class: String,
    truth_duration_ms: String,
    truth_streams: String,
    truth_v_packets: String,
    truth_a_packets: String,
}

fn parse_cases(text: &str) -> Vec<AviCase> {
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 6 {
            continue;
        }
        out.push(AviCase {
            rel: f[0].to_owned(),
            class: f[1].to_owned(),
            truth_duration_ms: f[2].to_owned(),
            truth_streams: f[3].to_owned(),
            truth_v_packets: f[4].to_owned(),
            truth_a_packets: f[5].to_owned(),
        });
    }
    out
}

/// A packet's PTS in nanoseconds (0 when absent — recorded, not guessed).
fn pts_nanos(packet: &Packet) -> i128 {
    match &packet.pts {
        media_types::Stamp::Present(ts) => match ts.as_nanos() {
            Ok(ns) => i128::from(ns.0),
            Err(_) => 0,
        },
        media_types::Stamp::Missing => 0,
    }
}

/// The measured walk over one AVI. Returns the result-row fields.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn walk_one(
    path: &Path,
) -> (
    String,
    u32,
    u64,
    u64,
    u64,
    u64,
    u64,
    u64,
    i128,
    i128,
    String,
    u128,
) {
    let t0 = Instant::now();
    let stream = match open_via_input(path) {
        Ok(stream) => stream,
        Err(err) => {
            return (
                "open-error".into(),
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                err.context.code.to_owned(),
                t0.elapsed().as_micros(),
            );
        }
    };
    let mut dem = match AviDemuxer::new(stream) {
        Ok(dem) => dem,
        Err(err) => {
            return (
                "construct-error".into(),
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                err.context.code.to_owned(),
                t0.elapsed().as_micros(),
            );
        }
    };
    let tracks = match dem.start() {
        Ok(tracks) => tracks,
        Err(err) => {
            return (
                "start-error".into(),
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                err.context.code.to_owned(),
                t0.elapsed().as_micros(),
            );
        }
    };
    let kind_of = |id: TrackId| -> u8 {
        // 1 = video, 2 = audio, 0 = other/unknown
        for track in &tracks {
            if track.id == id {
                return match track.kind {
                    media_types::MediaKind::Video => 1,
                    media_types::MediaKind::Audio => 2,
                    _ => 0,
                };
            }
        }
        0
    };
    let (mut v_pkts, mut a_pkts, mut v_bytes, mut a_bytes) = (0u64, 0u64, 0u64, 0u64);
    let (mut recovered, mut damage) = (0u64, 0u64);
    let (mut first_v_pts, mut last_v_pts) = (i128::MAX, i128::MIN);
    let mut terminal = "eos".to_owned();
    let mut err_code = String::new();
    loop {
        match dem.next_event() {
            Ok(DemuxEvent::Packet(p)) => {
                let pts = pts_nanos(&p);
                match kind_of(p.track) {
                    1 => {
                        v_pkts += 1;
                        v_bytes += p.data.len() as u64;
                        if pts < first_v_pts {
                            first_v_pts = pts;
                        }
                        if pts > last_v_pts {
                            last_v_pts = pts;
                        }
                    }
                    2 => {
                        a_pkts += 1;
                        a_bytes += p.data.len() as u64;
                    }
                    _ => {}
                }
            }
            Ok(DemuxEvent::Recovered { skipped, .. }) => {
                recovered += 1;
                damage += skipped;
            }
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => {
                terminal = "error".into();
                err_code = err.context.code.to_owned();
                break;
            }
        }
    }
    if first_v_pts == i128::MAX {
        first_v_pts = 0;
    }
    if last_v_pts == i128::MIN {
        last_v_pts = 0;
    }
    (
        terminal,
        tracks.len() as u32,
        v_pkts,
        a_pkts,
        v_bytes,
        a_bytes,
        recovered,
        damage,
        first_v_pts,
        last_v_pts,
        err_code,
        t0.elapsed().as_micros(),
    )
}

/// The env-gated corpus run (harness-activated only).
#[test]
#[ignore = "randomized-corpus harness: run via tools/corpus/run_harness.py"]
fn corpus_avi_walk_records_every_file() {
    let (Some(corpus), Some(cases_path), Some(results_path)) = (
        std::env::var("LUMEN_CORPUS").ok(),
        std::env::var("LUMEN_CORPUS_CASES").ok(),
        std::env::var("LUMEN_CORPUS_RESULTS").ok(),
    ) else {
        eprintln!("corpus: LUMEN_CORPUS env trio not set — no-op");
        return;
    };
    let corpus = PathBuf::from(corpus);
    let cases = parse_cases(&std::fs::read_to_string(&cases_path).must());
    assert!(
        !cases.is_empty(),
        "harness handed an empty case list ({cases_path:?})"
    );
    let mut out = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&results_path)
        .must();
    writeln!(out, "rel\tclass\ttruth_duration_ms\ttruth_streams\ttruth_v_packets\ttruth_a_packets\tterminal\ttracks\tv_pkts\ta_pkts\tv_bytes\ta_bytes\trecovered\tdamage\tfirst_v_pts_ns\tlast_v_pts_ns\terr_code\telapsed_us").must();
    for case in &cases {
        let path = corpus.join(&case.rel);
        assert!(path.is_file(), "corpus file missing: {path:?}");
        let (
            terminal,
            tracks,
            v_pkts,
            a_pkts,
            v_bytes,
            a_bytes,
            recovered,
            damage,
            f_pts,
            l_pts,
            err,
            elapsed,
        ) = walk_one(&path);
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            case.rel,
            case.class,
            case.truth_duration_ms,
            case.truth_streams,
            case.truth_v_packets,
            case.truth_a_packets,
            terminal,
            tracks,
            v_pkts,
            a_pkts,
            v_bytes,
            a_bytes,
            recovered,
            damage,
            f_pts,
            l_pts,
            err,
            elapsed
        )
        .must();
    }
    eprintln!(
        "corpus(demux): {} cases recorded -> {results_path:?}",
        cases.len()
    );
}

/// One manifest entry (the fields the audio matrix needs).
struct ManifestEntry {
    rel: String,
    /// The record class (kept for the results TSV's future columns;
    /// the parser only emits `avi-av` records).
    #[allow(dead_code)]
    class: String,
    audio_codec: String,
}

/// Parse the corpus manifest's clean avi-av entries (files are picked
/// by parameters, not names — the corpus is regenerated with fresh
/// random seeds, so name-pinning would rot).
fn parse_manifest_clean_avi_av(text: &str) -> Vec<ManifestEntry> {
    // A tiny std-only JSON field scan (the manifest is flat
    // ffmpeg-authored records; the harness owns full parsing). The
    // record-level `class` ("avi-av") must not be confused with the
    // nested `params.class` ("av") — fields are attributed by the
    // `params` object's brace extent.
    let mut out = Vec::new();
    let mut rel = String::new();
    let mut class = String::new();
    let mut audio_codec = String::new();
    let mut in_params = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("\"params\":") {
            in_params = true;
        } else if in_params && trimmed.starts_with('}') {
            in_params = false;
        } else if trimmed.starts_with('}') {
            // Record close: a file entry (only `files` entries carry
            // `rel`; `rejected_draws` entries do not).
            if !rel.is_empty() && class == "avi-av" {
                out.push(ManifestEntry {
                    rel: rel.clone(),
                    class: class.clone(),
                    audio_codec: audio_codec.clone(),
                });
            }
            rel.clear();
            class.clear();
            audio_codec.clear();
        } else if in_params {
            if let Some(rest) = trimmed.strip_prefix("\"audio_codec\":") {
                audio_codec = unquote(rest);
            }
        } else if let Some(rest) = trimmed.strip_prefix("\"rel\":") {
            rel = unquote(rest);
        } else if let Some(rest) = trimmed.strip_prefix("\"class\":") {
            class = unquote(rest);
        }
    }
    out
}

/// The first double-quoted value of a `: "value",` remainder.
fn unquote(rest: &str) -> String {
    let rest = rest.trim_start();
    let Some(start) = rest.find('"') else {
        return String::new();
    };
    let tail = &rest[start + 1..];
    match tail.find('"') {
        Some(end) => tail[..end].to_string(),
        None => String::new(),
    }
}

/// The env-gated PCM recognition matrix (W1-02): every **clean** avi-av
/// file in the corpus must demux an audio track and deliver at least
/// one audio packet — grouped by the manifest's audio codec, with
/// per-group packet totals recorded to the results TSV. The oracle
/// comparison (ffprobe's exact packet counts) stays with the harness
/// aggregator; this suite pins the recognition contract that was the
/// registered follow-up's root cause.
#[test]
#[ignore = "randomized-corpus harness: run via tools/corpus/run_harness.py (audio matrix)"]
fn corpus_avi_audio_matrix_recognizes_every_pcm_format() {
    let (Some(corpus), Some(results_path)) = (
        std::env::var("LUMEN_CORPUS").ok(),
        std::env::var("LUMEN_CORPUS_RESULTS").ok(),
    ) else {
        eprintln!("corpus: LUMEN_CORPUS env pair not set — no-op");
        return;
    };
    let corpus = PathBuf::from(corpus);
    let manifest = std::fs::read_to_string(corpus.join("manifest.json")).must();
    let entries = parse_manifest_clean_avi_av(&manifest);
    assert!(
        !entries.is_empty(),
        "corpus manifest has no clean avi-av entries"
    );
    let mut out = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&results_path)
        .must();
    writeln!(
        out,
        "audio_codec\tfiles\tfiles_with_audio_track\tfiles_with_audio_packets\taudio_packets_total"
    )
    .must();
    // Group by audio codec, preserving first-seen order.
    let mut codecs: Vec<String> = Vec::new();
    let mut per: Vec<(usize, usize, u64)> = Vec::new();
    for entry in &entries {
        let index = match codecs.iter().position(|c| *c == entry.audio_codec) {
            Some(index) => index,
            None => {
                codecs.push(entry.audio_codec.clone());
                per.push((0, 0, 0));
                codecs.len() - 1
            }
        };
        let path = corpus.join(&entry.rel);
        assert!(path.is_file(), "corpus file missing: {path:?}");
        let (terminal, tracks, _v, a_pkts, _vb, _ab, _rec, _dmg, _fp, _lp, err, _us) =
            walk_one(&path);
        let has_audio_track = walk_audio_track(&path);
        if has_audio_track {
            per[index].0 += 1;
        }
        if a_pkts > 0 {
            per[index].1 += 1;
        }
        per[index].2 += a_pkts;
        if !has_audio_track || a_pkts == 0 || terminal != "eos" {
            eprintln!(
                "audio-matrix MISS: {} ({}) terminal={terminal} tracks={tracks} a_pkts={a_pkts} err={err}",
                entry.rel, entry.audio_codec
            );
        }
    }
    for (index, codec) in codecs.iter().enumerate() {
        let files = entries.iter().filter(|e| &e.audio_codec == codec).count();
        let (tracks, packets, total) = per[index];
        writeln!(out, "{codec}\t{files}\t{tracks}\t{packets}\t{total}").must();
        eprintln!(
            "corpus(audio-matrix): {codec}: {tracks}/{files} with audio track, {packets}/{files} with packets, {total} packets"
        );
    }
    // The recognition contract: every clean avi-av file yields an
    // audio track and at least one audio packet (the demuxer-side AC
    // of the pcm stream-recognition follow-up).
    for (index, codec) in codecs.iter().enumerate() {
        let files = entries.iter().filter(|e| &e.audio_codec == codec).count();
        assert_eq!(
            per[index].0, files,
            "{codec}: every clean avi-av file must expose an audio track"
        );
        assert_eq!(
            per[index].1, files,
            "{codec}: every clean avi-av file must deliver audio packets"
        );
    }
}

/// Whether the file's `start()` track table carries an Audio track
/// (recognition, independent of the walk).
fn walk_audio_track(path: &Path) -> bool {
    let Ok(stream) = open_via_input(path) else {
        return false;
    };
    let Ok(mut dem) = AviDemuxer::new(stream) else {
        return false;
    };
    match dem.start() {
        Ok(tracks) => tracks
            .iter()
            .any(|t| matches!(t.kind, media_types::MediaKind::Audio)),
        Err(_) => false,
    }
}
