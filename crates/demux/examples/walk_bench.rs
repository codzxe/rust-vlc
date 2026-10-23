//! The AVI walk-rate bench (W5-14, D-10 measure-first): drives the real
//! `AviDemuxer` through the public `Demuxer` contract over a real file
//! and reports packets/second, MB/second and per-walk wall times.
//!
//! # What it measures
//!
//! One **walk** = open the source (through `input`'s `FileAccess` — the
//! production byte path — or over the file's bytes in memory, the
//! I/O-floor mode) → `AviDemuxer::new` → `start` → `next_event` to
//! `EndOfStream`. Packets are counted and dropped immediately (no
//! decode, no queues) — this is the demux-only cost, the Lumen analog
//! of `ffmpeg -i FILE -map 0 -c copy -f null -` (which likewise pays
//! per-packet allocation + interleave on top of its own demux).
//!
//! # Methodology (honest numbers)
//!
//! - `--warmup W` walks (default 2) run first and are discarded: they
//!   warm the OS page cache and the payload pool, so the measured walks
//!   see a hot cache — the same condition the ffmpeg comparator runs
//!   under.
//! - `--walks N` measured walks (default 9); every walk's wall time is
//!   printed to stderr (raw data), the median/min/max to stdout.
//! - Median is exact: middle element for odd N, mean of the two middle
//!   elements for even N.
//! - The comparator's process-startup overhead is NOT in our number
//!   (in-process loop) — compare against the ffmpeg side's
//!   startup-subtracted wall, or against `--mode mem` for the pure
//!   walk CPU floor.
//!
//! # Usage
//!
//! ```text
//! cargo run --release -p demux --example walk_bench -- \
//!     FILE [--walks N] [--warmup W] [--mode file|mem|both]
//! ```

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use demux::avi::AviDemuxer;
use demux::avi::test_support;
use media_types::EngineError;
use plugin_api::{Access, DemuxEvent, Demuxer, SourceStream};

/// One completed walk's counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WalkStats {
    packets: u64,
    payload_bytes: u64,
    recovered: u64,
    /// `damage_total()` — the damage ledger's byte sum (0-byte notes
    /// are decisions; see the W5-14 idx1 scope fix for why this is a
    /// bench output, not just a test concern).
    damage: u64,
    micros: u128,
}

/// `Result` → value or exit-with-error (the no-unwrap lint discipline,
/// CLI flavor: diagnostics on stderr, failure exit code).
fn or_exit<T>(result: Result<T, EngineError>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(err) => {
            eprintln!("walk_bench: {context}: {err}");
            std::process::exit(2);
        }
    }
}

/// Open a real file through the production access stack.
fn open_file(path: &std::path::Path) -> Result<Box<dyn SourceStream>, EngineError> {
    let text = match path.to_str() {
        Some(text) => format!("file://{text}"),
        None => {
            return Err(EngineError::new(
                media_types::PipelineStage::Open,
                media_types::ErrorKind::Unsupported { format: None },
                media_types::error::ErrorContext::new("walk-bench.path-not-utf8"),
            ));
        }
    };
    let uri = media_types::ids::SourceUri::parse(&text)?;
    input::FileAccess::new().open(&uri)
}

/// One full walk over an already-constructed demuxer. Every event is
/// consumed to `EndOfStream` (a walk that ends any other way is a bench
/// failure, not a data point).
fn walk(demuxer: &mut AviDemuxer) -> Result<WalkStats, EngineError> {
    let t0 = Instant::now();
    demuxer.start()?;
    let (mut packets, mut payload_bytes, mut recovered) = (0u64, 0u64, 0u64);
    loop {
        match demuxer.next_event()? {
            DemuxEvent::Packet(packet) => {
                payload_bytes += packet.data.len() as u64;
                packets += 1;
            }
            DemuxEvent::Recovered { .. } => recovered += 1,
            DemuxEvent::Discontinuity => {}
            DemuxEvent::EndOfStream => break,
        }
    }
    let damage = demuxer.damage_total();
    Ok(WalkStats {
        packets,
        payload_bytes,
        recovered,
        damage,
        micros: t0.elapsed().as_micros(),
    })
}

/// Exact median of a non-empty slice (middle element; mean of the two
/// middles for even counts).
fn median(values: &mut [u128]) -> u128 {
    values.sort_unstable();
    let len = values.len();
    if len % 2 == 1 {
        values[(len - 1) / 2]
    } else {
        (values[len / 2 - 1] + values[len / 2]) / 2
    }
}

/// Run `walks` measured walks (after `warmup` discarded ones) over a
/// fresh source per walk, printing the raw per-walk data to stderr and
/// the machine line to stdout.
fn bench_mode(
    label: &str,
    path: &std::path::Path,
    file_bytes: u64,
    warmup: usize,
    walks: usize,
    open: &dyn Fn() -> Result<Box<dyn SourceStream>, EngineError>,
) {
    let mut stats: Option<WalkStats> = None;
    let mut times: Vec<u128> = Vec::with_capacity(walks);
    for index in 0..warmup + walks {
        let mut demuxer = or_exit(
            AviDemuxer::new(or_exit(open(), "open source")),
            "AviDemuxer::new",
        );
        let result = match walk(&mut demuxer) {
            Ok(stats) => stats,
            Err(err) => or_exit(Err(err), "walk"),
        };
        // Every walk of one file must agree on its counters.
        match &stats {
            None => stats = Some(result),
            Some(prior) => {
                if (
                    prior.packets,
                    prior.payload_bytes,
                    prior.recovered,
                    prior.damage,
                ) != (
                    result.packets,
                    result.payload_bytes,
                    result.recovered,
                    result.damage,
                ) {
                    eprintln!(
                        "walk_bench: walk {index} disagreed: {prior:?} vs {result:?} — refusing"
                    );
                    std::process::exit(4);
                }
            }
        }
        eprintln!(
            "walk_bench: mode={label} walk={index} micros={}",
            result.micros
        );
        if index >= warmup {
            times.push(result.micros);
        }
    }
    let stats = match stats {
        Some(stats) => stats,
        None => {
            eprintln!("walk_bench: no walks ran");
            std::process::exit(5);
        }
    };
    if stats.packets == 0 {
        // The bench's files always carry packets; zero means a refusal
        // or an empty movi slipped through as a "successful" walk.
        eprintln!("walk_bench: zero packets — refusing to measure");
        std::process::exit(3);
    }
    let med = median(&mut times);
    let min = times.iter().min().copied().unwrap_or(med);
    let max = times.iter().max().copied().unwrap_or(med);
    let seconds = med as f64 / 1_000_000.0;
    println!(
        "WALKBENCH file={} mode={} walks={} warmup={} packets={} payload_bytes={} recovered={} damage={} \
         median_us={med} min_us={min} max_us={max} mb_per_s={:.2} packets_per_s={:.1}",
        path.display(),
        label,
        walks,
        warmup,
        stats.packets,
        stats.payload_bytes,
        stats.recovered,
        stats.damage,
        file_bytes as f64 / (1024.0 * 1024.0) / seconds,
        stats.packets as f64 / seconds,
    );
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(file) = args.next() else {
        eprintln!("usage: walk_bench FILE [--walks N] [--warmup W] [--mode file|mem|both]");
        return ExitCode::from(64);
    };
    let path = PathBuf::from(&file);
    let (mut walks, mut warmup, mut mode) = (9usize, 2usize, String::from("both"));
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--walks" => match args.next().and_then(|v| v.parse::<usize>().ok()) {
                Some(value) if value > 0 => walks = value,
                _ => {
                    eprintln!("walk_bench: --walks needs a positive integer");
                    return ExitCode::from(64);
                }
            },
            "--warmup" => match args.next().and_then(|v| v.parse::<usize>().ok()) {
                Some(value) => warmup = value,
                _ => {
                    eprintln!("walk_bench: --warmup needs a non-negative integer");
                    return ExitCode::from(64);
                }
            },
            "--mode" => {
                let Some(value) = args.next() else {
                    eprintln!("walk_bench: --mode needs a value (file, mem or both)");
                    return ExitCode::from(64);
                };
                if !matches!(value.as_str(), "file" | "mem" | "both") {
                    eprintln!("walk_bench: --mode must be file, mem or both, got {value}");
                    return ExitCode::from(64);
                }
                mode = value;
            }
            other => {
                eprintln!("walk_bench: unknown argument {other}");
                return ExitCode::from(64);
            }
        }
    }
    if !path.is_file() {
        eprintln!("walk_bench: not a file: {}", path.display());
        return ExitCode::from(66);
    }
    let file_bytes = match std::fs::metadata(&path) {
        Ok(meta) => meta.len(),
        Err(err) => {
            eprintln!("walk_bench: stat {}: {err}", path.display());
            return ExitCode::from(66);
        }
    };
    if mode == "file" || mode == "both" {
        bench_mode("file", &path, file_bytes, warmup, walks, &|| {
            open_file(&path)
        });
    }
    if mode == "mem" || mode == "both" {
        // The file's bytes, read once outside the measured walks: the
        // mem mode is the I/O-floor shape (same dyn `SourceStream`
        // contract, no syscalls, no input-layer window refills).
        let bytes: Arc<[u8]> = match std::fs::read(&path) {
            Ok(bytes) => bytes.into(),
            Err(err) => {
                eprintln!("walk_bench: read {}: {err}", path.display());
                return ExitCode::from(66);
            }
        };
        bench_mode("mem", &path, file_bytes, warmup, walks, &|| {
            test_support::mem_stream(Arc::clone(&bytes))
        });
    }
    ExitCode::SUCCESS
}
