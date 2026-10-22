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
