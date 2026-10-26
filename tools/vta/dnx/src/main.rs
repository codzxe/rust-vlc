//! VTA differential harness for filmcraft-dnx 0.9.0 (card 6.VTA-filmcraft-dnx).
//!
//! Usage:
//!   vta-dnx --frames N IN.dnxhd OUT.raw   decode N CBR frames, pack rawvideo
//!   vta-dnx --hostile IN.dnxhd            hostile-input battery on frame 0
//!
//! Input is a raw DNxHD/VC-3 elementary stream (ffmpeg `-f dnxhd` muxer).
//! DNxHD is intra-only CBR: every frame occupies filesize/N bytes, so the
//! frame count (from ffprobe) is the only splitting metadata needed.
//!
//! Output is ffmpeg-rawvideo-compatible planar layout for the frame's
//! chroma/bit-depth: 8-bit -> yuv422p/yuv444p byte planes, >8-bit ->
//! yuv422p10le-style u16LE planes (Y plane then Cb then Cr, no padding).

use filmcraft_dnx::{ChromaFormat, Frame, Interlace};
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--hostile" {
        hostile(&args[2]);
        return;
    }
    if args.len() == 5 && args[1] == "--frames" {
        let n = args[2].parse::<usize>().unwrap();
        decode_stream(&args[3], n, &args[4]);
        return;
    }
    eprintln!("usage: vta-dnx --frames N IN.dnxhd OUT.raw | vta-dnx --hostile IN.dnxhd");
    std::process::exit(2);
}

fn decode_stream(path: &str, n: usize, out_path: &str) {
    let data = std::fs::read(path).expect("read input");
    assert!(n > 0, "frame count must be > 0");
    assert!(
        data.len() % n == 0,
        "file {} not divisible into {} CBR frames",
        data.len(),
        n
    );
    let fsz = data.len() / n;
    let mut out: Vec<u8> = Vec::new();
    let mut decoded = 0usize;
    for i in 0..n {
        let chunk = &data[i * fsz..(i + 1) * fsz];
        match filmcraft_dnx::decode_frame(chunk) {
            Ok(frame) => {
                pack_rawvideo(&frame, &mut out);
                decoded += 1;
            }
            Err(e) => eprintln!("frame {}: DECODE ERROR: {:?}", i, e),
        }
    }
    std::fs::write(out_path, &out).expect("write output");
    eprintln!(
        "decoded {}/{} frames, {} raw bytes, frame_size={}",
        decoded,
        n,
        out.len(),
        fsz
    );
}

/// Pack a Frame into ffmpeg rawvideo planar layout (Y, then Cb, then Cr).
fn pack_rawvideo(f: &Frame, out: &mut Vec<u8>) {
    assert!(
        !f.rgb,
        "RGB (4:4:4 CID 1256-style) lane not handled by this harness"
    );
    let w = f.width as usize;
    let h = f.height as usize;
    let cw = f.chroma.chroma_width(f.width) as usize;
    let ch = f.chroma.chroma_height(f.height) as usize;
    // Planes are tightly packed (no row padding, per the crate's docs):
    // rawvideo layout = Y plane (w*h) then Cb (cw*ch) then Cr (cw*ch).
    if f.bit_depth == 8 {
        for p in [&f.y, &f.cb, &f.cr] {
            out.extend(p.iter().map(|&s| s as u8));
        }
    } else {
        for p in [&f.y, &f.cb, &f.cr] {
            for &s in p.iter() {
                out.extend_from_slice(&(s as u16).to_le_bytes());
            }
        }
    }
    // sanity: expected sizes
    let expect = if f.bit_depth == 8 {
        w * h + 2 * cw * ch
    } else {
        2 * (w * h + 2 * cw * ch)
    };
    assert_eq!(out.len() % expect.max(1), 0, "packing size mismatch");
    let _ = Interlace::Progressive;
    let _ = ChromaFormat::Yuv422;
}

fn hostile(path: &str) {
    let data = std::fs::read(path).expect("read input");
    let fsz = if data.len() >= 2 { data.len() / 2 } else { data.len() };
    let f0 = &data[..fsz];
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("zero-length", Vec::new()),
        ("truncated-frame(1/2)", f0[..f0.len() / 2].to_vec()),
        ("truncated-frame(64B)", f0[..64.min(f0.len())].to_vec()),
        ("garbage-0xAA", vec![0xAAu8; fsz]),
        ("garbage-random", {
            // deterministic LCG garbage
            let mut x = 0x12345678u32;
            (0..fsz)
                .map(|_| {
                    x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                    (x >> 24) as u8
                })
                .collect()
        }),
        ("hdr-ok-payload-zero", {
            // keep first 640 B of a real frame (header + first scan lines),
            // zero the rest
            let mut v = f0.to_vec();
            let skip = 640.min(v.len());
            for b in v[skip..].iter_mut() {
                *b = 0;
            }
            v
        }),
    ];
    let mut panics = 0usize;
    for (name, input) in &cases {
        let r = std::panic::catch_unwind(|| filmcraft_dnx::decode_frame(input));
        match r {
            Ok(Ok(f)) => println!(
                "hostile {:<24} -> Ok({}x{} cid {} depth {})",
                name, f.width, f.height, f.cid, f.bit_depth
            ),
            Ok(Err(e)) => println!("hostile {:<24} -> Err({:?})", name, e),
            Err(_) => {
                println!("hostile {:<24} -> PANIC!", name);
                panics += 1;
            }
        }
    }
    // sampled off-by-one truncation sweep on frame 0. A full sweep is a
    // whole-frame decode per cut point (DNxHD frames are ~1 MB); we sample
    // every `step`-th cut point, capped at 256 decodes (~13 s worst case).
    let mut sweep_panics = 0usize;
    let mut sweep_ok = 0usize;
    let mut sweep_err = 0usize;
    let len = f0.len().max(1);
    let step = (len / 256).max(1);
    let mut cuts = 0usize;
    let mut cut = 1usize;
    while cut < len && cuts < 256 {
        let r = std::panic::catch_unwind(|| filmcraft_dnx::decode_frame(&f0[..cut]));
        match r {
            Err(_) => sweep_panics += 1,
            Ok(Ok(_)) => sweep_ok += 1,
            Ok(Err(_)) => sweep_err += 1,
        }
        cuts += 1;
        cut += step;
    }
    println!(
        "sweep: {} sampled truncation points (step {}), {} panics, {} Ok-frames, {} typed Err",
        cuts, step, sweep_panics, sweep_ok, sweep_err
    );
    let mut log = std::fs::File::create("/tmp/vta-dnx-hostile.log").unwrap();
    let _ = writeln!(log, "panics={} sweep_panics={}", panics, sweep_panics);
    println!("TOTAL PANICS: {}", panics + sweep_panics);
}
