//! MJPEG decode-core benchmark (card W6-02) — the honest-numbers
//! instrument for the heavy-mjpeg 0.16x-vs-ffmpeg target.
//!
//! Input: a raw Motion-JPEG elementary stream (concatenated complete
//! JPEG frames, e.g. `ffmpeg -f lavfi -i
//! testsrc2=size=1280x720:rate=30 -t 10 -c:v mjpeg -q:v 5 -f mjpeg
//! out.mjpeg`). The bench splits the stream into frames with a
//! marker-walking splitter (segment lengths + entropy-marker scan —
//! the same walk shape as the decoder's parser, test-side), then
//! decodes every frame through the real codec-api contract path
//! (`MjpegDecoder::decode` → sink), the surface the decode-to-null
//! speed-parity metric measures.
//!
//! Protocol (medians, N, load honesty — Wave-6 brief):
//! - `--passes N` (default 21): after 2 warm-up passes, N measured
//!   passes over all frames; per-pass wall time on stderr, one
//!   machine-parseable summary line on stdout (median / p95 / min,
//!   fps, MB/s of compressed input).
//! - The 300 packet shells are built once (one `SharedBytes` per
//!   frame); each measured pass re-feeds them by `Arc` clone, so the
//!   bench's own per-pass overhead is refcount bumps, not copies of
//!   frame bytes.
//! - Load conditions: `/proc/loadavg` is sampled before and after the
//!   measured window and printed on the summary line (other
//!   cargo/ffmpeg processes must be recorded by the operator).
//!
//! Exit codes: 0 ok, 64 usage, 66 unreadable input, 70 decode
//! failure (a frame that refuses to decode fails the bench — the
//! fixture must be decodable by construction). Allocation counting
//! is deliberately NOT this binary's job (a `#[global_allocator]`
//! cannot be switched on at runtime): the steady-state allocs/frame
//! claims live in the counting-allocator test suites, per the
//! `tests/zero_alloc_decode.rs` precedent.

use std::env;
use std::process::ExitCode;
use std::time::Instant;

use codec_api::prelude::*;
use codecs::video::mjpeg::MjpegDecoder;
use media_types::packet::SharedBytes;
use media_types::{GenerationId, Packet, TrackId};

/// Usage text (stderr).
const USAGE: &str = "usage: huffman_bench <file.mjpeg> [--passes N]";

/// A sink that drops frames after counting them (decode-to-null: the
/// handoff's real cost is the ownership transfer, not retention).
struct NullSink {
    frames: usize,
}

impl FrameSink for NullSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        self.frames += 1;
        Ok(())
    }
}

/// One frame's byte range in the raw stream.
struct FrameSpan {
    start: usize,
    end: usize,
}

/// Split a concatenated-JPEG stream into frame spans.
///
/// Walks SOI → segments by their 16-bit lengths → SOS → entropy scan
/// to the first real marker (FF followed by neither 00, nor FF fill,
/// nor RST0..7). Anything between frames that is not an SOI is
/// skipped byte-wise (defensive: the raw-mjpeg muxer emits nothing
/// between frames, but the splitter must not fabricate frames from
/// garbage either).
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
                    // EOI: frame complete.
                    end = Some(p + 2);
                    break;
                }
                0xda => {
                    // SOS: the entropy scan ends the header walk.
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
                    // Segment with a 16-bit length.
                    let len = u16::from_be_bytes([data[p + 2], data[p + 3]]) as usize;
                    if len < 2 {
                        break; // structurally broken: abandon this SOI
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
                pos = start + 2; // not a real frame: resync past this SOI
            }
        }
    }
    frames
}

/// One line of `/proc/loadavg` (the 1-minute average), if readable.
fn loadavg() -> Option<String> {
    let text = std::fs::read_to_string("/proc/loadavg").ok()?;
    let first = text.split_whitespace().next()?;
    Some(first.to_string())
}

/// Configure the contract decoder (the real decode surface).
fn configured() -> Result<(MjpegDecoder, GenerationId), String> {
    let descriptor = media_types::ids::CodecDescriptor::new("mjpeg")
        .map_err(|e| format!("descriptor: {e:?}"))?;
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => return Err(format!("generation: {e:?}")),
    };
    let mut decoder = MjpegDecoder::new();
    decoder
        .configure(&DecoderConfig::new(
            descriptor,
            generation,
            ThreadBudget::default(),
            None,
            None,
        ))
        .map_err(|e| format!("configure: {e:?}"))?;
    Ok((decoder, generation))
}

fn main() -> ExitCode {
    let mut file = None;
    let mut passes = 21usize;
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
        if file.is_none() {
            file = Some(args[index].clone());
            index += 1;
            continue;
        }
        eprintln!("unexpected extra argument {arg:?}\n{USAGE}");
        return ExitCode::from(64);
    }
    let Some(file) = file else {
        eprintln!("{USAGE}");
        return ExitCode::from(64);
    };

    let data = match std::fs::read(&file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("huffman_bench: cannot read {file}: {e}");
            return ExitCode::from(66);
        }
    };
    let frames = split_frames(&data);
    if frames.is_empty() {
        eprintln!("huffman_bench: no complete JPEG frames found in {file}");
        return ExitCode::from(66);
    }
    let total_bytes = frames.iter().map(|f| f.end - f.start).sum::<usize>();
    eprintln!(
        "huffman_bench: {file}: {} bytes, {} frames (splitter walk), {:.1} MiB of JPEG data",
        data.len(),
        frames.len(),
        total_bytes as f64 / (1024.0 * 1024.0)
    );

    let (mut decoder, generation) = match configured() {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("huffman_bench: {e}");
            return ExitCode::from(70);
        }
    };

    // Packet shells built once (the same ownership shape the demuxer
    // hands the decoder); each pass re-feeds them by Arc clone, so
    // the measured loop contains no frame-byte copies.
    let mut packets = Vec::with_capacity(frames.len());
    for span in &frames {
        packets.push(Packet {
            track: TrackId::new(0),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(data[span.start..span.end].to_vec()),
            keyframe: true,
            discontinuity: false,
            generation,
        });
    }

    let mut sink = NullSink { frames: 0 };

    // Warm-up (2 passes): plane/LUT/pool sizing, branch/cache warm.
    for _ in 0..2 {
        for packet in &packets {
            if let Err(e) = decoder.decode(packet.clone(), &mut sink) {
                eprintln!("huffman_bench: warm-up decode failed: {e:?}");
                return ExitCode::from(70);
            }
        }
    }

    let load_before = loadavg();
    let mut durations = Vec::with_capacity(passes);
    for pass in 0..passes {
        let started = Instant::now();
        for packet in &packets {
            if let Err(e) = decoder.decode(packet.clone(), &mut sink) {
                eprintln!("huffman_bench: pass {pass} decode failed: {e:?}");
                return ExitCode::from(70);
            }
        }
        let elapsed = started.elapsed();
        durations.push(elapsed.as_secs_f64() * 1e3);
        eprintln!(
            "pass {pass:>2}: {:8.1} ms ({} frames, {} emitted)",
            durations[pass],
            frames.len(),
            sink.frames
        );
    }
    let load_after = loadavg();

    durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = durations[passes / 2];
    let p95_idx = ((passes as f64 * 0.95) as usize).min(passes - 1);
    let p95 = durations[p95_idx];
    let min = durations[0];
    let fps = frames.len() as f64 / (median / 1e3);
    let mbs = total_bytes as f64 / 1e6 / (median / 1e3);

    eprintln!(
        "sink accepted {} frames total ({} passes x {} frames + warm-up)",
        sink.frames,
        passes,
        frames.len()
    );
    println!(
        "HUFFMANBENCH file={file} frames={} passes={passes} med_ms={median:.1} p95_ms={p95:.1} \
         min_ms={min:.1} med_fps={fps:.1} med_mbs={mbs:.1} input_mb={:.1} \
         load_before={} load_after={}",
        frames.len(),
        total_bytes as f64 / 1e6,
        load_before.as_deref().unwrap_or("?"),
        load_after.as_deref().unwrap_or("?"),
    );
    ExitCode::SUCCESS
}
