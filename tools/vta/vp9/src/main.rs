//! 6.VTA-rusty-vp9 differential harness (SPIKE).
//!
//! Decodes an .ivf VP9 file through OUR ivf demuxer
//! (`crates/demux/src/ivf`) into rusty_vp9 0.1.3, writing tightly
//! packed I420 rawvideo + one `frame_md5` line per decoded frame on
//! stdout. Reference side is `ffmpeg -c:v libvpx-vp9 -f rawvideo`
//! (see README). `--hostile` runs the no-panic battery.

use std::fmt::Write as _;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use demux::avi::test_support::mem_stream;
use demux::ivf::IvfDemuxer;
use plugin_api::{DemuxEvent, Demuxer};
use rusty_vp9::{DecodedFrame, Vp9Decoder};

fn md5_hex(data: &[u8]) -> String {
    // Minimal MD5 (RFC 1321) — no deps in the harness.
    let mut a: u32 = 0x6745_2301;
    let mut b: u32 = 0xefcd_ab89;
    let mut c: u32 = 0x98ba_dcfe;
    let mut d: u32 = 0x1032_5476;
    let s = [
        7u32, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14,
        20, 5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16,
        23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10,
        15, 21, 6, 10, 15, 21,
    ];
    let k: [u32; 64] = [
        0xd76a_a478, 0xe8c7_b756, 0x2420_70db, 0xc1bd_ceee, 0xf57c_0faf, 0x4787_c62a, 0xa830_4613,
        0xfd46_9501, 0x6980_98d8, 0x8b44_f7af, 0xffff_5bb1, 0x895c_d7be, 0x6b90_1122, 0xfd98_7193,
        0xa679_438e, 0x49b4_0821, 0xf61e_2562, 0xc040_b340, 0x265e_5a51, 0xe9b6_c7aa, 0xd62f_105d,
        0x0244_1453, 0xd8a1_e681, 0xe7d3_fbc8, 0x21e1_cde6, 0xc337_07d6, 0xf4d5_0d87, 0x455a_14ed,
        0xa9e3_e905, 0xfcef_a3f8, 0x676f_02d9, 0x8d2a_4c8a, 0xfffa_3942, 0x8771_f681, 0x6d9d_6122,
        0xfde5_380c, 0xa4be_ea44, 0x4bde_cfa9, 0xf6bb_4b60, 0xbebf_bc70, 0x289b_7ec6, 0xeaa1_27fa,
        0xd4ef_3085, 0x0488_1d05, 0xd9d4_d039, 0xe6db_99e5, 0x1fa2_7cf8, 0xc4ac_5665, 0xf429_2244,
        0x432a_ff97, 0xab94_23a7, 0xfc93_a039, 0x655b_59c3, 0x8f0c_cc92, 0xffef_f47d, 0x8584_5dd1,
        0x6fa8_7e4f, 0xfe2c_e6e0, 0xa301_4314, 0x4e08_11a1, 0xf753_7e82, 0xbd3a_f235, 0x2ad7_d2bb,
        0xeb86_d391,
    ];
    let mut msg = data.to_vec();
    let bitlen = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_le_bytes());
    for chunk in msg.chunks(64) {
        let mut m = [0u32; 16];
        for (i, w) in m.iter_mut().enumerate() {
            *w = u32::from_le_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        let (mut aa, mut bb, mut cc, mut dd) = (a, b, c, d);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((bb & cc) | (!bb & dd), i),
                1 => ((dd & bb) | (!dd & cc), (5 * i + 1) % 16),
                2 => (bb ^ cc ^ dd, (3 * i + 5) % 16),
                _ => (cc ^ (bb | !dd), (7 * i) % 16),
            };
            let tmp = dd;
            dd = cc;
            cc = bb;
            let x = aa
                .wrapping_add(f)
                .wrapping_add(k[i])
                .wrapping_add(m[g]);
            bb = bb.wrapping_add(x.rotate_left(s[i]));
            aa = tmp;
        }
        a = a.wrapping_add(aa);
        b = b.wrapping_add(bb);
        c = c.wrapping_add(cc);
        d = d.wrapping_add(dd);
    }
    let mut out = String::with_capacity(32);
    for w in [a, b, c, d] {
        let _ = write!(out, "{:08x}", w);
    }
    out
}

/// Tightly packed I420 (ffmpeg rawvideo yuv420p layout, 8-bit).
fn frame_to_i420(f: &DecodedFrame) -> Vec<u8> {
    assert_eq!(f.bit_depth, 8, "harness compares 8-bit lanes only");
    let (w, h) = (f.width as usize, f.height as usize);
    let w_uv = (w + f.subsampling_x as usize) >> 1;
    let h_uv = (h + f.subsampling_y as usize) >> 1;
    let mut out =
        Vec::with_capacity(w * h + 2 * w_uv * h_uv);
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

fn demux_packets(path: &str) -> Result<Vec<Vec<u8>>, String> {
    let data = std::fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    let mut d = IvfDemuxer::new(mem_stream(data).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
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
        let i420 = frame_to_i420(f);
        println!(
            "frame_md5 {} {}x{} bd{} {}",
            i,
            f.width,
            f.height,
            f.bit_depth,
            md5_hex(&i420)
        );
        dims = (f.width, f.height);
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
    let garbage: Vec<u8> = (0..64u32).map(|i| (i.wrapping_mul(2654435761) >> 24) as u8).collect();
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
    println!("hostile off-by-one sweep: {}/{} packets panicked", panics, pkts.len());
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
