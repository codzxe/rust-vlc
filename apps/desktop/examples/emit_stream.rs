//! `emit_stream` — the prototype's measurement + golden-stream tool
//! (card 2.D3; `docs/prototypes/video-in-webview.md` §4's evidence
//! generator).
//!
//! Emits a deterministic frame stream in the v1 wire format plus the
//! JSON expectation file the Node conformance harness
//! (`prototype/harness.mjs`) checks, and measures the encode/decode
//! wall-clock cost of the protocol on this machine (recorded
//! evidence — the 2.D1 staging precedent: measured, printed, never
//! asserted by tests).
//!
//! ```text
//! cargo run -p desktop --example emit_stream -- --out target/proto
//! node apps/desktop/prototype/harness.mjs target/proto
//! cargo run -p desktop --example emit_stream -- --measure --frames 240
//! ```
//!
//! Determinism: the synthetic pattern (renderer-wgpu's, fully
//! specified), fixed size schedule, 30 fps PTS grid, fixed virtual
//! emit stamps (slot + 5 ms — the pipeline test's ENCODE_COST), FNV-1a
//! payload digests. Two runs produce byte-identical outputs (the
//! digest self-check in `--measure` mode re-proves it).

use std::path::PathBuf;
use std::time::Instant;

use desktop::wire::{FormatCode, FrameFlags, FrameHeader, WireDecoder, encode_into};
use renderer_wgpu::params::Extent2D;
use renderer_wgpu::synthetic::synthetic_pattern;

/// FNV-1a 64-bit — a *conformance* digest (collision-fast, not
/// cryptographic; it pins "the JS decoder saw these exact bytes",
/// which is its only job here).
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The deterministic size schedule (the `--big` flag appends 1080p).
const SIZES: [(u32, u32); 3] = [(64, 48), (320, 180), (640, 360)];

/// The virtual encode cost folded into every emit stamp (kept equal
/// to the pipeline suite's `ENCODE_COST` so the quoted numbers line
/// up).
const ENCODE_COST: u64 = 5_000_000;

/// One stream entry: the record's header + the payload it carried.
struct Entry {
    header: FrameHeader,
    payload: Vec<u8>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut out_dir = PathBuf::from("target/proto");
    let mut frames = 90usize;
    let mut measure = false;
    let mut big = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                out_dir = PathBuf::from(args.next().ok_or("--out needs a value")?);
            }
            "--frames" => {
                frames = args
                    .next()
                    .ok_or("--frames needs a value")?
                    .parse()
                    .map_err(|e| format!("--frames: {e}"))?;
            }
            "--measure" => measure = true,
            "--big" => big = true,
            other => return Err(format!("unknown argument {other:?}").into()),
        }
    }

    // ---- compose the deterministic stream --------------------------------
    let mut sizes: Vec<(u32, u32)> = SIZES.to_vec();
    if big {
        sizes.push((1920, 1080));
    }
    let mut entries: Vec<Entry> = Vec::with_capacity(frames);
    let mut stream = Vec::new();
    for i in 0..frames {
        // Every 7th delivery is the wiring's RepeatLast: the previous
        // request re-presented (same payload, same slot, new record).
        let repeat = i > 0 && i % 7 == 0;
        let (width, height) = if repeat {
            let prev = &entries[i - 1];
            (prev.header.width, prev.header.height)
        } else {
            sizes[i % sizes.len()]
        };
        let extent = Extent2D { width, height };
        let mut payload = synthetic_pattern(extent);
        // The marker pixel (5,5) carries the frame index (the 2.D1
        // convention — content identity, not counters).
        let at = (5 * width as usize + 5) * 4;
        let marker = if repeat { (i - 1) % 256 } else { i % 256 };
        payload[at..at + 4].copy_from_slice(&[marker as u8, 0xEE, 0x77, 255]);
        let (pts_ns, emitted_ns) = if repeat {
            let prev = &entries[i - 1];
            (
                prev.header.pts_ns,
                (i as u64 - 1) * 33_333_334 + ENCODE_COST + 33_333_334,
            )
        } else {
            (
                (i as i64) * 33_333_334,
                (i as u64) * 33_333_334 + ENCODE_COST,
            )
        };
        let header = FrameHeader {
            format: FormatCode::Rgba8,
            flags: FrameFlags { repeat },
            width,
            height,
            payload_len: payload.len() as u32,
            frame_id: i as u64,
            pts_ns,
            emitted_ns,
        };
        encode_into(&header, &payload, &mut stream)?;
        entries.push(Entry { header, payload });
    }

    // ---- write the golden pair -------------------------------------------
    std::fs::create_dir_all(&out_dir)?;
    std::fs::write(out_dir.join("stream.bin"), &stream)?;
    let mut json = String::new();
    json.push_str(
        "{\n  \"protocol\": { \"magic\": \"LUF1\", \"version\": 1, \"headerLen\": 48 },\n",
    );
    json.push_str(&format!(
        "  \"streamBytes\": {},\n  \"frameCount\": {},\n  \"frames\": [\n",
        stream.len(),
        entries.len()
    ));
    for (i, entry) in entries.iter().enumerate() {
        let h = &entry.header;
        json.push_str(&format!(
            "    {{ \"frameId\": {}, \"width\": {}, \"height\": {}, \"formatCode\": {}, \"repeat\": {}, \"ptsNs\": {}, \"emittedNs\": {}, \"payloadLen\": {}, \"fnv1a64\": \"{:016x}\", \"head\": \"{}\", \"tail\": \"{}\" }}{}\n",
            h.frame_id,
            h.width,
            h.height,
            h.format.code(),
            h.flags.repeat,
            h.pts_ns,
            h.emitted_ns,
            h.payload_len,
            fnv1a64(&entry.payload),
            entry
                .payload
                .iter()
                .take(8)
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            entry
                .payload
                .iter()
                .rev()
                .take(8)
                .rev()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            if i + 1 == entries.len() { "" } else { "," }
        ));
    }
    json.push_str("  ]\n}\n");
    std::fs::write(out_dir.join("expected.json"), json)?;

    let total_payload: u64 = entries
        .iter()
        .map(|e| u64::from(e.header.payload_len))
        .sum();
    println!(
        "emit_stream: {} frames, {} stream bytes ({} payload + {} header), sizes {:?}{}",
        entries.len(),
        stream.len(),
        total_payload,
        entries.len() * 48,
        sizes,
        if big { " (+1080p)" } else { "" }
    );
    println!(
        "golden pair written to {} (stream.bin + expected.json)",
        out_dir.display()
    );

    // ---- measurement (recorded evidence, never test-asserted) ------------
    if measure {
        // Encode: rebuild the stream from the source entries.
        let mut sink = Vec::with_capacity(stream.len());
        let encode_start = Instant::now();
        for entry in &entries {
            encode_into(&entry.header, &entry.payload, &mut sink)?;
        }
        let encode_ns = encode_start.elapsed().as_nanos() as u64;
        assert_eq!(sink.len(), stream.len(), "encode determinism (length)");
        let mut identical = true;
        for (a, b) in sink.iter().zip(stream.iter()) {
            identical &= a == b;
        }
        println!(
            "encode:  {} bytes in {} ns -> {:.1} MiB/s (byte-identical rebuild: {})",
            sink.len(),
            encode_ns,
            sink.len() as f64 / 1_048_576.0 / (encode_ns as f64 / 1e9),
            identical
        );

        // Decode: full-stream pass through the incremental decoder.
        let decode_start = Instant::now();
        let mut decoder = WireDecoder::new();
        let mut decoded = 0usize;
        // Chunked like a real IPC feed (the deterministic LCG split
        // the Node harness mirrors).
        let mut offset = 0usize;
        let mut lcg: u64 = 0x853c_49e6_748f_ea9b;
        while offset < stream.len() {
            lcg = lcg
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let chunk = ((lcg >> 33) as usize % 4096 + 1).min(stream.len() - offset);
            decoder.feed(&stream[offset..offset + chunk]);
            offset += chunk;
            while decoder.next_frame()?.is_some() {
                decoded += 1;
            }
        }
        decoder.finish()?;
        let decode_ns = decode_start.elapsed().as_nanos() as u64;
        println!(
            "decode:  {} frames / {} bytes in {} ns -> {:.1} MiB/s, {:.0} ns/frame",
            decoded,
            stream.len(),
            decode_ns,
            stream.len() as f64 / 1_048_576.0 / (decode_ns as f64 / 1e9),
            decode_ns as f64 / decoded as f64
        );
        println!(
            "header overhead: 48 B/frame = {:.4}% of payload at 1080p, {:.4}% at 720p-class 640x360",
            48.0 / (1920.0 * 1080.0 * 4.0 + 48.0) * 100.0,
            48.0 / (640.0 * 360.0 * 4.0 + 48.0) * 100.0
        );
    }
    Ok(())
}
