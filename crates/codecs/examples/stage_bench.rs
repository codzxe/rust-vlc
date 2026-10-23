//! MJPEG decode **stage attribution** benchmark (card 7-b, the
//! PROFILE-BEFORE-OPTIMIZING instrument): where does heavy-MJPEG
//! decode time actually go — Huffman entropy decode, dequant+IDCT,
//! the plane store, or the output assembly (color conversion +
//! upsampling / I420 crop)?
//!
//! # Method (ablation over the identical bitstream)
//!
//! The decoder's scan walk is generic over stage hooks
//! (`decoder.rs`, `ScanHooks`); the production path instantiates
//! `FullStages` (codegen-identical to the pre-seam body — pinned by
//! the in-file bit-exactness test). This example times the same walk
//! with stages stubbed out, per pass, arms interleaved so machine
//! drift hits all arms equally:
//!
//! | arm | runs | measures |
//! |---|---|---|
//! | `full` | parse + huffman + idct + store + assemble (`DecodeContext::decode`) | the whole decode |
//! | `scan` | parse + huffman + idct + store (`decode_scan`) | scan without assembly |
//! | `noidct` | parse + huffman + store | — |
//! | `nowrite` | parse + huffman + idct | — |
//! | `huffonly` | parse + huffman | — |
//! | `parse` | `parser::parse_into` alone | header walk |
//!
//! Stage shares (medians, one-stage-at-a-time subtraction — each stub
//! arm differs from the `huffonly` arm by exactly one stage):
//! - `parse` = parse arm (header walk),
//! - `huffman` = huffonly − parse (entropy decode),
//! - `idct` = nowrite − huffonly (dequant+IDCT; dequant folded in),
//! - `store` = noidct − huffonly (plane write traffic),
//! - `assemble` = full − scan (color conversion + upsampling for
//!   4:4:4/4:2:2/4:4:0 RGBA output; plain row crop for I420),
//! - residual = scan − (huffonly + idct + store) — the closure check
//!   (MCU loop + cursor bookkeeping; must be small vs `scan`).
//!
//! Because the Huffman walk consumes the identical bitstream in every
//! arm (it depends on the tables and the reader only), the deltas are
//! a valid attribution of the real walk's time. Known approximation:
//! stubbing a stage also removes its cache traffic, so each share
//! slightly understates that stage's standalone cost; the residual and
//! the `full` arm's agreement with `huffman_bench` (the contract-path
//! measurement) bound the error from above.
//!
//! The live sources are `#[path]`-included (the `color_bench`
//! precedent): the example always measures the exact shipped code.
//!
//! # Protocol
//!
//! `--passes N` (default 15) after 2 warm-up passes; per-pass arm
//! times on stderr, one `STAGEBENCH` summary line per file on stdout
//! (medians + shares). `/proc/loadavg` sampled around the measured
//! window (honest-numbers mandate).
//!
//! Run: `cargo run --release -p codecs --example stage_bench -- <a.mjpeg
//! b.mjpeg ...> [--passes N]`

use std::env;
use std::process::ExitCode;
use std::time::Instant;

// The live decoder sources, compiled into this example (the
// `#[path]`-include pattern of `color_bench.rs`: pub(crate) internals
// benched without widening the library API).
#[path = "../src/video/mjpeg/error.rs"]
mod error;
#[path = "../src/video/mjpeg/bitreader.rs"]
mod bitreader;
#[path = "../src/video/mjpeg/huffman.rs"]
mod huffman;
#[path = "../src/video/mjpeg/idct.rs"]
mod idct;
#[path = "../src/video/mjpeg/parser.rs"]
mod parser;
#[path = "../src/video/mjpeg/color.rs"]
mod color;
#[path = "../src/video/mjpeg/decoder.rs"]
mod decoder;

use decoder::{DecodeContext, FullStages, NoIdctNoWriteStages, NoIdctStages, NoWriteStages};

/// Usage text (stderr).
const USAGE: &str = "usage: stage_bench <file.mjpeg>... [--passes N]";

/// One frame's byte range in the raw stream (the huffman_bench
/// splitter, verbatim — same walk shape for the same fixtures).
struct FrameSpan {
    start: usize,
    end: usize,
}

fn split_frames(data: &[u8]) -> Vec<FrameSpan> {
    let mut frames = Vec::new();
    let mut pos = 0usize;
    while pos + 1 < data.len() {
        if data[pos] != 0xff || data[pos + 1] != 0xd8 {
            pos += 1;
            continue;
        }
        let start = pos;
        let mut p = pos + 2;
        let mut end = None;
        while p + 3 < data.len() {
            if data[p] != 0xff {
                p += 1;
                continue;
            }
            let code = data[p + 1];
            match code {
                0x00 => p += 2,        // stuffed FF (entropy)
                0xff => p += 1,        // fill byte
                0xd0..=0xd7 => p += 2, // restart marker (entropy)
                0x01 | 0xd8 => p += 2, // TEM / stray SOI
                0xd9 => {
                    end = Some(p + 2);
                    break;
                }
                0xda => {
                    let len = u16::from_be_bytes([data[p + 2], data[p + 3]]) as usize;
                    if len < 2 {
                        break;
                    }
                    p += 2 + len;
                    while p + 1 < data.len() {
                        let b = data[p];
                        if b == 0xff {
                            let c = data[p + 1];
                            if c == 0x00 {
                                p += 2;
                                continue;
                            }
                            if c == 0xff {
                                p += 1;
                                continue;
                            }
                            if (0xd0..=0xd7).contains(&c) {
                                p += 2;
                                continue;
                            }
                            break; // real marker: end of entropy
                        }
                        p += 1;
                    }
                    end = Some(p);
                    break;
                }
                _ => {
                    let len = u16::from_be_bytes([data[p + 2], data[p + 3]]) as usize;
                    if len < 2 {
                        break;
                    }
                    p += 2 + len;
                }
            }
        }
        match end {
            Some(end) if end > start + 2 => {
                frames.push(FrameSpan { start, end });
                pos = end;
            }
            _ => {
                pos = start + 2;
            }
        }
    }
    frames
}

fn loadavg() -> Option<String> {
    let text = std::fs::read_to_string("/proc/loadavg").ok()?;
    let first = text.split_whitespace().next()?;
    Some(first.to_string())
}

/// One arm's per-pass closure shape: returns elapsed ms for one pass
/// over all frames.
fn timed_pass(mut body: impl FnMut()) -> f64 {
    let started = Instant::now();
    body();
    started.elapsed().as_secs_f64() * 1e3
}

fn run_file(file: &str, passes: usize) -> Result<(), String> {
    let data = std::fs::read(file).map_err(|e| format!("cannot read {file}: {e}"))?;
    let frames = split_frames(&data);
    if frames.is_empty() {
        return Err(format!("no complete JPEG frames found in {file}"));
    }
    let spans: Vec<&[u8]> = frames.iter().map(|f| &data[f.start..f.end]).collect();

    // One shared context across arms: planes/tables are keyed and
    // every block of every plane is rewritten by every scan (parse
    // proves full coverage), so arm interleaving cannot poison a
    // later arm — and the single plane set keeps the cache footprint
    // identical across arms.
    let mut ctx = DecodeContext::new();
    let mut headers = parser::JpegInfo::empty();

    // Validation gate (printed, not asserted away): the staged-full
    // walk must byte-equal the production walk on every frame of this
    // fixture — the seam's license to attribute stage costs.
    {
        let mut staged = DecodeContext::new();
        for frame in &spans {
            let a = ctx
                .decode_scan(frame)
                .map_err(|e| format!("production scan: {e:?}"))?;
            let b = staged
                .decode_scan_with::<FullStages>(frame)
                .map_err(|e| format!("staged scan: {e:?}"))?;
            let len = ctx.frame_len().map_err(|e| format!("frame_len: {e:?}"))?;
            let mut out_a = vec![0u8; len];
            let mut out_b = vec![0u8; len];
            ctx.assemble_frame(&mut out_a)
                .map_err(|e| format!("assemble a: {e:?}"))?;
            staged.assemble_frame(&mut out_b)
                .map_err(|e| format!("assemble b: {e:?}"))?;
            if out_a != out_b || (a.width, a.height, a.concealed) != (b.width, b.height, b.concealed)
            {
                return Err(format!(
                    "staged-full walk diverged from the production path on {file} \
                     (seam invalid — do not trust these numbers)"
                ));
            }
        }
    }

    // Warm-up: 2 passes of the full arm (plane/LUT/pool sizing).
    for _ in 0..2 {
        for frame in &spans {
            ctx.decode(frame).map_err(|e| format!("warm-up: {e:?}"))?;
        }
    }

    let mut full_v = Vec::with_capacity(passes);
    let mut scan_v = Vec::with_capacity(passes);
    let mut noidct_v = Vec::with_capacity(passes);
    let mut nowrite_v = Vec::with_capacity(passes);
    let mut huffonly_v = Vec::with_capacity(passes);
    let mut parse_v = Vec::with_capacity(passes);

    let load_before = loadavg();
    for pass in 0..passes {
        // Arms interleaved within each pass (drift hits all equally).
        full_v.push(timed_pass(|| {
            for frame in &spans {
                let _ = ctx.decode(frame);
            }
        }));
        scan_v.push(timed_pass(|| {
            for frame in &spans {
                let _ = ctx.decode_scan(frame);
            }
        }));
        noidct_v.push(timed_pass(|| {
            for frame in &spans {
                let _ = ctx.decode_scan_with::<NoIdctStages>(frame);
            }
        }));
        nowrite_v.push(timed_pass(|| {
            for frame in &spans {
                let _ = ctx.decode_scan_with::<NoWriteStages>(frame);
            }
        }));
        huffonly_v.push(timed_pass(|| {
            for frame in &spans {
                let _ = ctx.decode_scan_with::<NoIdctNoWriteStages>(frame);
            }
        }));
        parse_v.push(timed_pass(|| {
            for frame in &spans {
                let _ = parser::parse_into(&mut headers, frame);
            }
        }));
        eprintln!(
            "pass {pass:>2}: full {:8.1} | scan {:8.1} | noidct {:8.1} | nowrite {:8.1} | \
             huffonly {:8.1} | parse {:8.1} ms",
            full_v[pass],
            scan_v[pass],
            noidct_v[pass],
            nowrite_v[pass],
            huffonly_v[pass],
            parse_v[pass],
        );
    }
    let load_after = loadavg();

    let median = |v: &[f64]| {
        let mut s = v.to_vec();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        s[s.len() / 2]
    };
    let minimum = |v: &[f64]| {
        v.iter()
            .copied()
            .fold(f64::INFINITY, |a, b| if b < a { b } else { a })
    };
    let full = median(&full_v);
    let scan = median(&scan_v);
    let noidct = median(&noidct_v);
    let nowrite = median(&nowrite_v);
    let huffonly = median(&huffonly_v);
    let parse = median(&parse_v);

    // One-stage-at-a-time deltas (each stub arm differs from the
    // huffonly arm by exactly one stage, so the deltas are additive
    // up to cache-interaction second-order effects — the `residual`
    // below is the honest closure check):
    //   huffonly = parse + huffman
    //   nowrite  = huffonly + idct
    //   noidct   = huffonly + store
    //   scan     ≈ huffonly + idct + store (+ MCU-loop residual)
    //   full     = scan + assemble
    let huffman = huffonly - parse;
    let idct = nowrite - huffonly;
    let store = noidct - huffonly;
    let assemble = full - scan;
    let residual = scan - (huffonly + idct + store);
    let pct = |x: f64| 100.0 * x / full;

    // Min-of-passes variants: under bursty external machine load the
    // minimum approximates the unloaded cost of each arm (recorded as
    // supplementary stability evidence, medians stay the headline).
    let full_min = minimum(&full_v);
    let scan_min = minimum(&scan_v);
    let noidct_min = minimum(&noidct_v);
    let nowrite_min = minimum(&nowrite_v);
    let huffonly_min = minimum(&huffonly_v);
    let parse_min = minimum(&parse_v);
    let huffman_min = huffonly_min - parse_min;
    let idct_min = nowrite_min - huffonly_min;
    let store_min = noidct_min - huffonly_min;
    let assemble_min = full_min - scan_min;

    // Geometry of the first decoded frame (all frames of a stream
    // share it; the fixture summary line says so).
    let meta = ctx
        .decode_scan(spans[0])
        .map_err(|e| format!("meta scan: {e:?}"))?;
    let format = match meta.format {
        decoder::OutputFormat::I420 => "I420",
        decoder::OutputFormat::Rgba8 => "RGBA",
    };

    eprintln!(
        "shares of full: parse {:.1}% | huffman {:.1}% | idct+dequant {:.1}% | store {:.1}% | \
         assemble(color/crop) {:.1}% | residual {residual:.1} ms",
        pct(parse),
        pct(huffman),
        pct(idct),
        pct(store),
        pct(assemble),
    );
    println!(
        "STAGEBENCH file={file} frames={} passes={passes} {format} {}x{} med_full_ms={full:.1} \
         med_scan_ms={scan:.1} parse_ms={parse:.1} huffman_ms={huffman:.1} idct_ms={idct:.1} \
         store_ms={store:.1} assemble_ms={assemble:.1} residual_ms={residual:.2} \
         pct_parse={:.1} pct_huffman={:.1} pct_idct={:.1} pct_store={:.1} pct_assemble={:.1} \
         min_full_ms={full_min:.1} min_huffman_ms={huffman_min:.1} min_idct_ms={idct_min:.1} \
         min_store_ms={store_min:.1} min_assemble_ms={assemble_min:.1} \
         load_before={} load_after={}",
        spans.len(),
        meta.width,
        meta.height,
        pct(parse),
        pct(huffman),
        pct(idct),
        pct(store),
        pct(assemble),
        load_before.as_deref().unwrap_or("?"),
        load_after.as_deref().unwrap_or("?"),
    );
    Ok(())
}

fn main() -> ExitCode {
    let mut files: Vec<String> = Vec::new();
    let mut passes = 15usize;
    let args: Vec<String> = env::args().skip(1).collect();
    let mut index = 0usize;
    while index < args.len() {
        let arg = args[index].as_str();
        if arg == "--passes" {
            let Some(value) = args.get(index + 1) else {
                eprintln!("--passes needs a value\n{USAGE}");
                return ExitCode::from(64);
            };
            match value.parse::<usize>() {
                Ok(n) if n >= 3 => passes = n,
                _ => {
                    eprintln!("bad --passes value {value:?} (need >= 3)\n{USAGE}");
                    return ExitCode::from(64);
                }
            }
            index += 2;
            continue;
        }
        if arg.starts_with("--") {
            eprintln!("unknown argument {arg:?}\n{USAGE}");
            return ExitCode::from(64);
        }
        files.push(args[index].clone());
        index += 1;
    }
    if files.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(64);
    }
    for file in &files {
        if let Err(e) = run_file(file, passes) {
            eprintln!("stage_bench: {e}");
            return ExitCode::from(70);
        }
    }
    ExitCode::SUCCESS
}
