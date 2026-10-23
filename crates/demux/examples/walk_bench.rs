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
//! # Allocation counting (`--allocs`, W6-04)
//!
//! `--allocs` swaps the timing protocol for a counting-allocator
//! protocol over the mem-mode walk (fresh demuxer per walk, warmups
//! discarded, median alloc events/bytes per walk + per packet). The
//! mem source's `read_into` is a zero-alloc memcpy, so every counted
//! event is the walk's own (header parse, pool, packet hand-off) —
//! the file mode would add the input layer's window refills and
//! belongs to the input crate's bench, not this one.
//!
//! # Payload-size census (`--sizes`, W6-04)
//!
//! `--sizes` walks the file once (mem mode) and prints, per track, the
//! payload-size distribution the payload-pool policy is derived from:
//! count, min/median/max, distinct-size count, the repeat-within-4
//! rate (the only reuse an exact-size slot rotation can harvest), and
//! how many payloads exceed 64 KiB (the poolable-class ceiling check).
//! Raw evidence for docs/perf/demux-spill-fix.md.
//!
//! # Instrument-cost honesty
//!
//! The counting allocator is installed unconditionally in this binary
//! (a `#[global_allocator]` cannot be mode-gated, and a paused
//! counter would hide allocations — D-5). Timing runs therefore carry
//! the counter's per-event cost (W5-15 measured 17.7–22.4 ns per
//! alloc+dealloc pair); with the post-W6-04 walk at ~0.9 alloc
//! events/packet that is ~0.01% of a walk — but state the protocol
//! you compare under: pristine-vs-pristine or counted-vs-counted.
//!
//! # Usage
//!
//! ```text
//! cargo run --release -p demux --example walk_bench -- \
//!     FILE [--walks N] [--warmup W] [--mode file|mem|both]
//!          [--allocs] [--sizes]
//! ```

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use demux::avi::AviDemuxer;
use demux::avi::test_support;
use media_types::EngineError;
use plugin_api::{Access, DemuxEvent, Demuxer, SourceStream};
use testkit::alloc_count::CountingGlobalAlloc;

/// The counting global allocator (W6-04): unconditional in this binary,
/// read by `--allocs` (see the instrument-cost note in the module docs).
#[global_allocator]
static ALLOC: CountingGlobalAlloc = CountingGlobalAlloc::new();

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

/// One measured walk's allocation delta (`--allocs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WalkAllocs {
    events: u64,
    bytes: u64,
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

/// One full walk under the counting allocator (`--allocs`): the same
/// protocol as [`walk`] with the delta bracketed. The stats walk and
/// the measured walk are the same pass — the timing is not reported
/// for alloc walks (they are not timing data points).
fn walk_counted(demuxer: &mut AviDemuxer) -> Result<(WalkStats, WalkAllocs), EngineError> {
    demuxer.start()?;
    let guard = ALLOC.state().guard();
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
    let delta = guard.delta();
    let damage = demuxer.damage_total();
    Ok((
        WalkStats {
            packets,
            payload_bytes,
            recovered,
            damage,
            micros: 0,
        },
        WalkAllocs {
            events: delta.alloc_calls,
            bytes: delta.total_bytes,
        },
    ))
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

/// The `--allocs` protocol: fresh demuxer per walk (mem mode — the
/// source itself allocates nothing, so every event is the walk's),
/// warmups discarded, median events/bytes per walk and per packet.
fn bench_allocs(path: &std::path::Path, bytes: &Arc<[u8]>, warmup: usize, walks: usize) {
    let mut stats: Option<WalkStats> = None;
    let mut events: Vec<u128> = Vec::with_capacity(walks);
    let mut alloc_bytes: Vec<u128> = Vec::with_capacity(walks);
    for index in 0..warmup + walks {
        let source = or_exit(test_support::mem_stream(Arc::clone(bytes)), "mem_stream");
        let mut demuxer = or_exit(AviDemuxer::new(source), "AviDemuxer::new");
        let (result, allocs) = match walk_counted(&mut demuxer) {
            Ok(pair) => pair,
            Err(err) => or_exit(Err(err), "walk"),
        };
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
                        "walk_bench: alloc walk {index} disagreed: {prior:?} vs {result:?} — refusing"
                    );
                    std::process::exit(4);
                }
            }
        }
        eprintln!(
            "walk_bench: allocs walk={index} events={} bytes={}",
            allocs.events, allocs.bytes
        );
        if index >= warmup {
            events.push(allocs.events as u128);
            alloc_bytes.push(allocs.bytes as u128);
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
        eprintln!("walk_bench: zero packets — refusing to measure");
        std::process::exit(3);
    }
    let med_events = median(&mut events);
    let med_bytes = median(&mut alloc_bytes);
    println!(
        "WALKALLOCS file={} walks={} warmup={} packets={} payload_bytes={} \
         median_alloc_events={med_events} median_alloc_bytes={med_bytes} \
         events_per_pkt={:.4} alloc_bytes_per_pkt={:.1} alloc_bytes_per_payload_byte={:.4}",
        path.display(),
        walks,
        warmup,
        stats.packets,
        stats.payload_bytes,
        med_events as f64 / stats.packets as f64,
        med_bytes as f64 / stats.packets as f64,
        med_bytes as f64 / stats.payload_bytes.max(1) as f64,
    );
}

/// One track's payload-size census row (`--sizes`).
struct SizeRow {
    track: u32,
    sizes: Vec<usize>,
}

/// The `--sizes` protocol: one mem-mode walk collecting every packet's
/// payload size per track, then the distribution summary the pool
/// policy is derived from.
fn bench_sizes(path: &std::path::Path, bytes: &Arc<[u8]>) {
    let source = or_exit(test_support::mem_stream(Arc::clone(bytes)), "mem_stream");
    let mut demuxer = or_exit(AviDemuxer::new(source), "AviDemuxer::new");
    if let Err(err) = demuxer.start() {
        eprintln!("walk_bench: start: {err}");
        std::process::exit(2);
    }
    let mut rows: Vec<SizeRow> = Vec::new();
    let mut ended = false;
    while !ended {
        let event = match demuxer.next_event() {
            Ok(event) => event,
            Err(err) => {
                eprintln!("walk_bench: walk: {err}");
                std::process::exit(2);
            }
        };
        match event {
            DemuxEvent::Packet(packet) => {
                let track = packet.track.handle();
                match rows.iter_mut().find(|row| row.track == track) {
                    Some(row) => row.sizes.push(packet.data.len()),
                    None => rows.push(SizeRow {
                        track,
                        sizes: vec![packet.data.len()],
                    }),
                }
            }
            DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => {}
            DemuxEvent::EndOfStream => ended = true,
        }
    }
    rows.sort_by_key(|row| row.track);
    for row in &rows {
        let mut sizes = row.sizes.clone();
        sizes.sort_unstable();
        let n = sizes.len();
        let distinct = sizes
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let med = sizes[n / 2];
        // Repeat-within-4: how often a packet's exact size appeared in
        // the previous 4 packets of the same track — the only reuse an
        // exact-size slot rotation can harvest.
        let mut hits = 0usize;
        let mut window: std::collections::VecDeque<usize> =
            std::collections::VecDeque::with_capacity(4);
        for &size in &row.sizes {
            if window.contains(&size) {
                hits += 1;
            }
            if window.len() == 4 {
                window.pop_front();
            }
            window.push_back(size);
        }
        let over_cap = sizes.iter().filter(|&&s| s > 64 * 1024).count();
        println!(
            "WALKSIZES file={} track={} packets={} min={} med={} max={} distinct={} \
             repeat_within_4={:.1}% over_64KiB={} ({:.1}%)",
            path.display(),
            row.track,
            n,
            sizes[0],
            med,
            sizes[n - 1],
            distinct,
            hits as f64 / n as f64 * 100.0,
            over_cap,
            over_cap as f64 / n as f64 * 100.0,
        );
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(file) = args.next() else {
        eprintln!(
            "usage: walk_bench FILE [--walks N] [--warmup W] [--mode file|mem|both] [--allocs] [--sizes]"
        );
        return ExitCode::from(64);
    };
    let path = PathBuf::from(&file);
    let (mut walks, mut warmup, mut mode) = (9usize, 2usize, String::from("both"));
    let (mut allocs, mut sizes) = (false, false);
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
            "--allocs" => allocs = true,
            "--sizes" => sizes = true,
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
    // The file's bytes, read once outside every measured protocol.
    let bytes: Arc<[u8]> = match std::fs::read(&path) {
        Ok(bytes) => bytes.into(),
        Err(err) => {
            eprintln!("walk_bench: read {}: {err}", path.display());
            return ExitCode::from(66);
        }
    };
    if sizes {
        bench_sizes(&path, &bytes);
    }
    if allocs {
        bench_allocs(&path, &bytes, warmup, walks);
        return ExitCode::SUCCESS;
    }
    if mode == "file" || mode == "both" {
        bench_mode("file", &path, file_bytes, warmup, walks, &|| {
            open_file(&path)
        });
    }
    if mode == "mem" || mode == "both" {
        // The mem mode is the I/O-floor shape (same dyn `SourceStream`
        // contract, no syscalls, no input-layer window refills).
        bench_mode("mem", &path, file_bytes, warmup, walks, &|| {
            test_support::mem_stream(Arc::clone(&bytes))
        });
    }
    ExitCode::SUCCESS
}
