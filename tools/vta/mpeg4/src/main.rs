//! 6.VTA-oxideav-mpeg4video differential harness (SPIKE).
//!
//! Demuxes AVI / MP4 through OUR demuxers (`crates/demux/src/avi`,
//! `crates/demux/src/mp4`), feeds the mpeg4 video track's packets to
//! `oxideav-mpeg4video` 0.1.8 (`Mpeg4VideoDecoder`, literal-spec mode
//! unless `--ecosystem`), and writes tightly packed display-dims I420
//! rawvideo (Y then Cb then Cr rows — byte-identical layout to ffmpeg
//! rawvideo `yuv420p`) plus a `FRAME` line per output frame on stdout
//! (coding type, display dims, container pts, §6.3.5 tick time).
//! Reference side is `ffmpeg -map 0:v:0 -f rawvideo -pix_fmt yuv420p`
//! + `-f framehash` (see README). `--hostile` runs the no-panic battery.

use std::panic::{catch_unwind, AssertUnwindSafe};

use demux::avi::test_support::mem_stream;
use demux::avi::AviDemuxer;
use demux::mp4::Mp4Demuxer;
use media_types::ids::MediaKind;
use oxideav_mpeg4video::decoder::Mpeg4VideoDecoder;
use plugin_api::{DemuxEvent, Demuxer};

struct Lane {
    packets: Vec<Vec<u8>>,
    pts: Vec<Option<i64>>,
    keyframes: Vec<bool>,
}

/// Demux the first video track through OUR demuxers (AVI or MP4).
fn demux_lane(path: &str) -> Result<Lane, String> {
    let data = std::fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    let stream = mem_stream(data).map_err(|e| e.to_string())?;
    let is_mp4 = path.to_ascii_lowercase().ends_with(".mp4");
    let mut dyn_dem: Box<dyn Demuxer> = if is_mp4 {
        Box::new(Mp4Demuxer::new(stream).map_err(|e| e.to_string())?)
    } else {
        Box::new(AviDemuxer::new(stream).map_err(|e| e.to_string())?)
    };
    let tracks = dyn_dem.start().map_err(|e| e.to_string())?;
    let vid = tracks
        .iter()
        .find(|t| t.kind == MediaKind::Video)
        .ok_or_else(|| format!("{path}: no video track"))?
        .id;
    let mut lane = Lane {
        packets: Vec::new(),
        pts: Vec::new(),
        keyframes: Vec::new(),
    };
    loop {
        match dyn_dem.next_event().map_err(|e| e.to_string())? {
            DemuxEvent::Packet(p) => {
                if p.track == vid {
                    let pts = p.pts.timestamp().ok().map(|ts| ts.ticks);
                    lane.packets.push(p.data.to_vec());
                    lane.pts.push(pts);
                    lane.keyframes.push(p.keyframe);
                }
            }
            DemuxEvent::Recovered { .. } | DemuxEvent::Discontinuity => {}
            DemuxEvent::EndOfStream => break,
        }
    }
    Ok(lane)
}

/// Crop one decoded frame (macroblock-padded planes) to the VOL display
/// dims, tightly packed — ffmpeg rawvideo yuv420p layout.
fn frame_to_i420(f: &oxideav_mpeg4video::framestore::DecodedFrame, dw: usize, dh: usize) -> Vec<u8> {
    let (pw, ph) = (f.width(), f.height());
    assert!(dw <= pw && dh <= ph, "display {dw}x{dh} > padded {pw}x{ph}");
    let cw = dw / 2;
    let ch = dh / 2;
    let mut out = Vec::with_capacity(dw * dh + 2 * cw * ch);
    for y in 0..dh {
        out.extend_from_slice(&f.luma_samples()[y * pw..y * pw + dw]);
    }
    for y in 0..ch {
        out.extend_from_slice(&f.cb_samples()[y * (pw / 2)..y * (pw / 2) + cw]);
    }
    for y in 0..ch {
        out.extend_from_slice(&f.cr_samples()[y * (pw / 2)..y * (pw / 2) + cw]);
    }
    out
}

fn run(path: &str, ecosystem: bool, out_path: Option<&str>) -> Result<(), String> {
    let lane = demux_lane(path)?;
    println!(
        "packets {} total_bytes {} keyframes {}",
        lane.packets.len(),
        lane.packets.iter().map(|p| p.len()).sum::<usize>(),
        lane.keyframes.iter().filter(|k| **k).count()
    );
    let mut out_file = out_path.map(|p| std::fs::File::create(p).map_err(|e| e.to_string()));
    let mut dec = if ecosystem {
        Mpeg4VideoDecoder::with_options(oxideav_mpeg4video::compat::DecodeOptions::ecosystem())
    } else {
        Mpeg4VideoDecoder::new()
    };
    let mut frames_out = 0usize;
    let mut errors = 0usize;
    let mut monotone_pts = true;
    let mut last_pts: Option<i64> = None;
    let mut emit = |frames: &mut dyn Iterator<Item = oxideav_mpeg4video::framestore::DecodedFrame>,
                    dec: &Mpeg4VideoDecoder,
                    out_file: &mut Option<Result<std::fs::File, String>>,
                    frames_out: &mut usize,
                    monotone: &mut bool,
                    last_pts: &mut Option<i64>|
     -> Result<(), String> {
        for f in frames {
            let (dw, dh) = match dec.vol() {
                Some(v) => (v.width as usize, v.height as usize),
                None => return Err("frame without an active VOL".to_string()),
            };
            let i420 = frame_to_i420(&f, dw, dh);
            if let Some(Ok(fh)) = out_file.as_mut() {
                use std::io::Write;
                fh.write_all(&i420).map_err(|e| e.to_string())?;
            }
            if let Some(p) = f.pts() {
                if let Some(lp) = *last_pts {
                    if p < lp {
                        *monotone = false;
                    }
                }
                *last_pts = Some(p);
            }
            println!(
                "FRAME {} {:?} {}x{} pts={:?} ticks={:?}",
                *frames_out, f.coding_type(), dw, dh, f.pts(), f.pts_ticks()
            );
            *frames_out += 1;
        }
        Ok(())
    };
    for (i, pkt) in lane.packets.iter().enumerate() {
        match dec.decode_with_pts(pkt, lane.pts[i]) {
            Ok(frames) => {
                let mut it = frames.into_iter();
                emit(
                    &mut it,
                    &dec,
                    &mut out_file,
                    &mut frames_out,
                    &mut monotone_pts,
                    &mut last_pts,
                )?;
            }
            Err(e) => {
                errors += 1;
                if errors <= 3 {
                    eprintln!("decode error at packet {i} ({} bytes): {e}", pkt.len());
                }
            }
        }
    }
    let flushed = dec.flush();
    let mut it = flushed.into_iter();
    emit(
        &mut it,
        &dec,
        &mut out_file,
        &mut frames_out,
        &mut monotone_pts,
        &mut last_pts,
    )?;
    if let Some(Err(e)) = out_file {
        return Err(e);
    }
    println!(
        "frames {} errors {} pts_monotone_nondecreasing {}",
        frames_out, errors, monotone_pts
    );
    Ok(())
}

fn run_case(name: &str, seq: Vec<Vec<u8>>, cases: &mut usize, panics: &mut usize) {
    *cases += 1;
    let mut dec = Mpeg4VideoDecoder::new();
    let mut frames = 0usize;
    let mut errs = 0usize;
    let r = catch_unwind(AssertUnwindSafe(|| {
        for p in &seq {
            match dec.decode(p) {
                Ok(fs) => frames += fs.len(),
                Err(_) => errs += 1,
            }
        }
        frames += dec.flush().len();
    }));
    match r {
        Ok(()) => println!("hostile {name}: Ok frames={frames} errs={errs}"),
        Err(_) => {
            *panics += 1;
            println!("hostile {name}: PANIC");
        }
    }
}

/// Hostile battery on a warm decoder: every case must be a typed Err or
/// an Ok frame count — never a panic (catch_unwind + exit code).
fn hostile(lane: &Lane) {
    let mut cases = 0usize;
    let mut panics = 0usize;

    if lane.packets.is_empty() {
        println!("hostile: no packets, skipping");
        return;
    }
    let pkt0 = lane.packets[0].clone();
    // 1. truncated first keyframe (every prefix length 0..len).
    {
        let mut dec = Mpeg4VideoDecoder::new();
        let mut p = 0usize;
        for n in 0..pkt0.len() {
            let r = catch_unwind(AssertUnwindSafe(|| {
                let _ = dec.decode(&pkt0[..n]);
            }));
            if r.is_err() {
                p += 1;
            }
        }
        cases += 1;
        if p > 0 {
            panics += 1;
        }
        println!("hostile truncate-sweep k0: {}/{} prefixes panicked", p, pkt0.len());
    }
    // 2. zero-length packet.
    run_case("zero-length", vec![Vec::new()], &mut cases, &mut panics);
    // 3. garbage 64B.
    let garbage: Vec<u8> = (0..64u32).map(|i| (i * 7 + 0xEE) as u8).collect();
    run_case("garbage-64B", vec![garbage.clone()], &mut cases, &mut panics);
    // 4. corrupted VOP header: first keyframe with the byte after the
    //    first start code flipped.
    let mut corrupt = pkt0.clone();
    let pos = corrupt
        .windows(3)
        .position(|w| w == [0, 0, 1])
        .map(|p| p + 3)
        .unwrap_or(0);
    if pos < corrupt.len() {
        corrupt[pos] ^= 0xFF;
    }
    run_case("vop-header-corrupt", vec![corrupt], &mut cases, &mut panics);
    // 5. garbage then real packets (recovery).
    let mut seq = vec![garbage];
    seq.extend(lane.packets.iter().take(6).cloned());
    run_case("garbage-then-real", seq, &mut cases, &mut panics);
    // 6. truncated mid-stream: real first half, then a cut packet.
    let mut seq: Vec<Vec<u8>> = lane.packets.iter().take(4).cloned().collect();
    if lane.packets.len() > 4 {
        let cut = &lane.packets[4];
        let n = cut.len() / 2;
        seq.push(cut[..n].to_vec());
    }
    run_case("truncated-midstream", seq, &mut cases, &mut panics);
    println!("hostile-summary cases {cases} panics {panics}");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: vta-mpeg4 <in.avi|in.mp4> [--out F.raw] [--ecosystem] [--hostile]"
        );
        std::process::exit(2);
    }
    let path = args[1].clone();
    let out = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let ecosystem = args.iter().any(|a| a == "--ecosystem");
    let hostile_mode = args.iter().any(|a| a == "--hostile");
    if hostile_mode {
        let lane = demux_lane(&path).unwrap_or_else(|e| {
            eprintln!("DEMUX-ERROR: {e}");
            std::process::exit(1);
        });
        hostile(&lane);
        return;
    }
    run(&path, ecosystem, out.as_deref()).unwrap_or_else(|e| {
        eprintln!("ERROR: {e}");
        std::process::exit(1);
    });
}
