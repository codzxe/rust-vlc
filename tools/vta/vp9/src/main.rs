//! 6.VTA-rusty-vp9 differential harness (SPIKE).
//!
//! Decodes an .ivf VP9 file through OUR ivf demuxer
//! (`crates/demux/src/ivf`) into rusty_vp9 0.1.3, writing tightly
//! packed I420 rawvideo + one `frame_md5` line per decoded frame on
//! stdout. Reference side is `ffmpeg -c:v libvpx-vp9 -f rawvideo`
//! (see README). `--hostile` runs the no-panic battery.

use std::panic::{catch_unwind, AssertUnwindSafe};

use demux::avi::test_support::mem_stream;
use demux::ivf::IvfDemuxer;
use plugin_api::{DemuxEvent, Demuxer};
use rusty_vp9::{DecodedFrame, Vp9Decoder};

/// Tightly packed I420 (ffmpeg rawvideo yuv420p layout, 8-bit).
fn frame_to_i420(f: &DecodedFrame) -> Vec<u8> {
    assert_eq!(f.bit_depth, 8, "harness compares 8-bit lanes only");
    let (w, h) = (f.width as usize, f.height as usize);
    let w_uv = (w + f.subsampling_x as usize) >> 1;
    let h_uv = (h + f.subsampling_y as usize) >> 1;
    let mut out = Vec::with_capacity(w * h + 2 * w_uv * h_uv);
    for p in 0..3 {
        let (pw, ph) = if p == 0 { (w, h) } else { (w_uv, h_uv) };
        let stride = f.strides[p];
        let plane = &f.planes[p];
        for y in 0..ph {
            out.extend_from_slice(&plane[y * stride..y * stride + pw]);
        }
    }
    out
}

/// Tightly packed high-bit-depth I420, u16 LE per sample.
fn frame_to_i420_hbd(f: &DecodedFrame) -> Vec<u8> {
    let (w, h) = (f.width as usize, f.height as usize);
    let w_uv = (w + f.subsampling_x as usize) >> 1;
    let h_uv = (h + f.subsampling_y as usize) >> 1;
    let mut out = Vec::with_capacity(2 * (w * h + 2 * w_uv * h_uv));
    for p in 0..3 {
        let (pw, ph) = if p == 0 { (w, h) } else { (w_uv, h_uv) };
        let stride = f.strides[p];
        let plane = &f.planes[p];
        for y in 0..ph {
            let row = &plane[y * stride..y * stride + pw * 2];
            // high-bit-depth planes are u16 samples stored LE in the u8 vec
            for sm in row.chunks(2) {
                out.extend_from_slice(sm);
            }
        }
    }
    out
}

fn demux_packets(path: &str) -> Result<Vec<Vec<u8>>, String> {
    let data = std::fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    let mut d =
        IvfDemuxer::new(mem_stream(data).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let _tracks = d.start().map_err(|e| e.to_string())?;
    let mut pkts = Vec::new();
    loop {
        match d.next_event().map_err(|e| e.to_string())? {
            DemuxEvent::Packet(p) => pkts.push(p.data.to_vec()),
            DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => {}
            DemuxEvent::EndOfStream => break,
        }
    }
    Ok(pkts)
}

fn decode_all(pkts: &[Vec<u8>]) -> Result<Vec<DecodedFrame>, String> {
    let mut dec = Vp9Decoder::new();
    for p in pkts {
        dec.push(p, None).map_err(|e| format!("push: {e}"))?;
    }
    dec.flush();
    let mut frames = Vec::new();
    loop {
        match dec.next_frame() {
            Ok(f) => frames.push(f),
            Err(rusty_vp9::Error::Eof) => break,
            Err(rusty_vp9::Error::Again) => continue,
            Err(e) => return Err(format!("next_frame after {} frames: {e}", frames.len())),
        }
    }
    Ok(frames)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: vta-vp9 <in.ivf> [--out F] | --hostile <in.ivf>");
        std::process::exit(2);
    }
    let path = &args[1];
    let pkts = demux_packets(path).unwrap_or_else(|e| {
        eprintln!("DEMUX-ERROR: {e}");
        std::process::exit(1);
    });
    println!(
        "packets {} total_bytes {}",
        pkts.len(),
        pkts.iter().map(|p| p.len()).sum::<usize>()
    );
    if args.iter().any(|a| a == "--hostile") {
        hostile(&pkts);
        return;
    }
    let frames = decode_all(&pkts).unwrap_or_else(|e| {
        eprintln!("DECODE-ERROR: {e}");
        std::process::exit(1);
    });
    let mut raw: Vec<u8> = Vec::new();
    let mut dims = (0u32, 0u32);
    for (i, f) in frames.iter().enumerate() {
        dims = (f.width, f.height);
        if f.bit_depth != 8 {
            // 10/12-bit: planes are u16 LE (DecodedFrame contract) — emit
            // the same layout ffmpeg rawvideo yuv420p10le/yuv420p12le uses.
            let hi = frame_to_i420_hbd(f);
            println!("frame {} {}x{} bd{}", i, f.width, f.height, f.bit_depth);
            raw.extend_from_slice(&hi);
            continue;
        }
        let i420 = frame_to_i420(f);
        // per-frame digests are computed externally (md5sum/python) over
        // this stream's frame-sized chunks — see tools/vta/vp9/README.md
        println!("frame {} {}x{} bd{}", i, f.width, f.height, f.bit_depth);
        raw.extend_from_slice(&i420);
    }
    println!("DIMS {} {}", dims.0, dims.1);
    if let Some(o) = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|o| args.get(o + 1))
    {
        std::fs::write(o, &raw).expect("write out");
        eprintln!("wrote {} bytes to {o}", raw.len());
    }
}

fn try_feed(label: &str, data: &[u8]) {
    let r = catch_unwind(AssertUnwindSafe(|| {
        let mut dec = Vp9Decoder::new();
        let push = dec.push(data, None);
        dec.flush();
        let outcome = match push {
            Err(e) => format!("push-Err({e})"),
            Ok(()) => match dec.next_frame() {
                Ok(f) => format!("frame {}x{}", f.width, f.height),
                Err(e) => format!("next-Err({e})"),
            },
        };
        format!("{outcome}")
    }));
    match r {
        Ok(outcome) => println!("hostile {label}: ok — {outcome}"),
        Err(_) => println!("hostile {label}: PANIC — FAIL"),
    }
}

fn hostile(pkts: &[Vec<u8>]) {
    let first = &pkts[0];
    // 1. truncated packet
    try_feed("truncated", &first[..first.len() / 2]);
    // 2. garbage bytes
    let garbage: Vec<u8> = (0..64u32)
        .map(|i| (i.wrapping_mul(2654435761) >> 24) as u8)
        .collect();
    try_feed("garbage64", &garbage);
    // 3. zero-length
    try_feed("zerolen", &[]);
    // 4. corrupted frame header (break the VP9 frame marker)
    let mut h = first.clone();
    h[0] ^= 0xff;
    h[1] ^= 0xff;
    try_feed("hdr-corrupt", &h);
    // 5. corrupted mid-bitstream on a later packet
    if pkts.len() > 2 {
        let mut m = pkts[2].clone();
        let mid = m.len() / 2;
        m[mid] ^= 0xa5;
        m[mid + 1] ^= 0x5a;
        try_feed("mid-corrupt", &m);
    }
    // 6. whole-file battery: every packet truncated by 1 byte, one at a time
    let mut panics = 0usize;
    for (i, p) in pkts.iter().enumerate() {
        if p.is_empty() {
            continue;
        }
        let r = catch_unwind(AssertUnwindSafe(|| {
            let mut dec = Vp9Decoder::new();
            let _ = dec.push(&p[..p.len() - 1], None);
            dec.flush();
            while !matches!(dec.next_frame(), Err(rusty_vp9::Error::Eof)) {}
        }));
        if r.is_err() {
            panics += 1;
            println!("hostile off-by-one pkt {i}: PANIC — FAIL");
        }
    }
    println!(
        "hostile off-by-one sweep: {}/{} packets panicked",
        panics,
        pkts.len()
    );
    // 7. keyframe-first-garbage stream: garbage packet then real packets
    let r = catch_unwind(AssertUnwindSafe(|| {
        let mut dec = Vp9Decoder::new();
        let _ = dec.push(&garbage, None);
        for p in pkts.iter().take(10) {
            let _ = dec.push(p, None);
        }
        dec.flush();
        let mut n = 0;
        loop {
            match dec.next_frame() {
                Ok(_) => n += 1,
                Err(rusty_vp9::Error::Eof) => break,
                Err(_) => continue,
            }
        }
        n
    }));
    match r {
        Ok(n) => println!("hostile garbage-then-real: ok — {n} frames out"),
        Err(_) => println!("hostile garbage-then-real: PANIC — FAIL"),
    }
}
