//! VTA harness for oxideav-ac3 0.0.11 (card 6.VTA-oxideav-ac3).
//!
//! HARNESS ONLY — never production. Not a workspace member (own
//! `[workspace]` table in Cargo.toml).
//!
//! Pipeline: (1) harness-only AC-3 frame syncwalker over a raw .ac3
//! elementary stream, (2) oxideav-ac3 decode to interleaved s16le PCM,
//! (3) differential vs `ffmpeg -c:a pcm_s16le -f s16le`, (4) hostile
//! inputs under catch_unwind.
//!
//! AC-3 frame-size formula (ATSC A/52B §5.4.1.4, Table 5.18):
//!   syncword 0x0B77 (2 bytes) + crc1 (2 bytes) + byte 4 = fscod (top
//!   2 bits: 0=48k, 1=44.1k, 2=32k, 3=reserved) | frmsizecod (low 6
//!   bits, 0..=37 valid). Frame length in 16-bit WORDS comes from
//!   Table 5.18; frame_bytes = words * 2. For 48 kHz the table
//!   collapses to the closed form words = 2 * nominal_kbps (e.g. 448k
//!   -> 896 words -> 1792 bytes); 32 kHz is words = 3 * nominal_kbps;
//!   44.1 kHz carries TWO rows per bitrate (alternating word counts,
//!   e.g. 192k -> 417/418 words) because 1536/44100-second frames have
//!   a fractional byte size — encoders alternate frmsizecod rows to
//!   track it (empirically confirmed on corpus: 834/836-byte strides).
//!   Table below is (w44_row_a, w44_row_b, w32, w48) per the 19
//!   nominal bitrates; frmsizecod = 2*idx selects row a, 2*idx+1 row b.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::Command;

use oxideav_ac3::decoder::make_decoder;
use oxideav_core::{CodecId, CodecParameters, Frame, Packet, TimeBase};

const DIR: &str = "/tmp/vta-ac3";

/// Table 5.18 word counts per (bitrate index): (w44 row_a, w44 row_b, w32, w48).
const T5_18: [[u16; 4]; 19] = [
    [69, 70, 96, 64],      // 32
    [87, 88, 120, 80],     // 40
    [104, 105, 144, 96],   // 48
    [121, 122, 168, 112],  // 56
    [139, 140, 192, 128],  // 64
    [174, 175, 240, 160],  // 80
    [208, 209, 288, 192],  // 96
    [243, 244, 336, 224],  // 112
    [278, 279, 384, 256],  // 128
    [348, 349, 480, 320],  // 160
    [417, 418, 576, 384],  // 192
    [487, 488, 672, 448],  // 224
    [557, 558, 768, 512],  // 256
    [696, 697, 960, 640],  // 320
    [835, 836, 1152, 768], // 384
    [975, 976, 1344, 896], // 448
    [1114, 1115, 1536, 1024], // 512
    [1253, 1254, 1728, 1152], // 576
    [1393, 1394, 1920, 1280], // 640
];

/// frame length in bytes per ATSC A/52 Table 5.18; None = reserved.
fn frame_len_bytes(fscod: u8, frmsizecod: u8) -> Option<usize> {
    if frmsizecod >= 38 || fscod > 2 {
        return None;
    }
    let idx = (frmsizecod / 2) as usize;
    let row = &T5_18[idx];
    let words = match fscod {
        0 => row[3],                          // 48 kHz: 2 * kbps
        1 => row[(frmsizecod % 2) as usize], // 44.1 kHz: alternating row
        2 => row[2],                          // 32 kHz: 3 * kbps
        _ => return None,
    };
    Some(words as usize * 2)
}

/// Harness-only syncwalker: split a raw .ac3 ES into syncframes.
/// Returns (frames, resync_events). Resyncs when the predicted frame
/// boundary does not carry the 0x0B77 syncword.
fn syncwalk(data: &[u8]) -> (Vec<Vec<u8>>, usize) {
    let mut frames = Vec::new();
    let mut resyncs = 0usize;
    let mut i = 0usize;
    while i + 5 <= data.len() {
        while i + 5 <= data.len() && (data[i] != 0x0B || data[i + 1] != 0x77) {
            i += 1; // scan to next syncword
        }
        if i + 5 > data.len() {
            break;
        }
        let fscod = data[i + 4] >> 6;
        let frmsizecod = data[i + 4] & 0x3F;
        let Some(len) = frame_len_bytes(fscod, frmsizecod) else {
            i += 2; // bad header — skip this pseudo-syncword
            resyncs += 1;
            continue;
        };
        if i + len > data.len() {
            frames.push(data[i..].to_vec()); // trailing truncated frame
            break;
        }
        let next = &data[i + len..];
        if next.is_empty() || (next[0] == 0x0B && next[1] == 0x77) {
            frames.push(data[i..i + len].to_vec());
            i += len;
        } else {
            i += 2; // false sync — step past and rescan
            resyncs += 1;
        }
    }
    (frames, resyncs)
}

/// Returns (pcm_s16le, dec_errors, resyncs, channels, sr, samples_per_ch).
fn decode_file(path: &str) -> (Vec<u8>, usize, usize, u32, u32, u64) {
    let data = std::fs::read(path).expect("read input");
    let (frames, resyncs) = syncwalk(&data);
    let params = CodecParameters::audio(CodecId::new("ac3")); // channels=None → passthrough
    let mut dec = make_decoder(&params).expect("make_decoder");
    let rate = match data.get(4).map(|b| b >> 6) {
        Some(0) => 48_000u32,
        Some(1) => 44_100,
        Some(2) => 32_000,
        _ => 0,
    };
    let tb = TimeBase::new(1, rate.max(1) as i64);
    let mut pcm = Vec::new();
    let mut errors = 0usize;
    let mut samples_per_ch = 0u64;
    let mut channels = 0u32;
    for (n, fr) in frames.iter().enumerate() {
        let pkt = Packet::new(0, tb, fr.clone());
        if let Err(e) = dec.send_packet(&pkt) {
            errors += 1;
            eprintln!("  frame {n}: send_packet Err: {e:?}");
            continue;
        }
        match dec.receive_frame() {
            Ok(Frame::Audio(a)) => {
                if n == 0 {
                    // 1536 samples/ch; data[0] = interleaved s16le single plane
                    channels = (a.data[0].len() / 2 / a.samples as usize) as u32;
                }
                samples_per_ch += a.samples as u64;
                pcm.extend_from_slice(&a.data[0]);
            }
            Ok(_) => errors += 1,
            Err(e) => {
                errors += 1;
                eprintln!("  frame {n}: receive_frame Err: {e:?}");
            }
        }
    }
    (pcm, errors, resyncs, channels, rate, samples_per_ch)
}

fn md5(bytes: &[u8]) -> String {
    let out = Command::new("md5sum")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write;
            c.stdin.as_mut().unwrap().write_all(bytes)?;
            c.wait_with_output()
        })
        .expect("md5sum");
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "hostile" {
        hostile();
        return;
    }
    if args.len() > 1 && args[1] == "cplprobe" {
        for p in &args[2..] {
            cplprobe(p);
        }
        return;
    }
    let lanes: Vec<(&str, String)> = vec![
        ("corpus_0001", format!("{DIR}/lanes/corpus_0001.ac3")),
        ("g_st192_44k", format!("{DIR}/gen/g_st192_44k.ac3")),
        ("g_st384_48k_noise", format!("{DIR}/gen/g_st384_48k_noise.ac3")),
        ("g_51_448_48k", format!("{DIR}/gen/g_51_448_48k.ac3")),
        ("g_mono96_48k", format!("{DIR}/gen/g_mono96_48k.ac3")),
        ("g_st96_48k", format!("{DIR}/gen/g_st96_48k.ac3")),
        ("g_51_256_48k", format!("{DIR}/gen/g_51_256_48k.ac3")),
        ("g_noise192_48k", format!("{DIR}/gen/g_noise192_48k.ac3")),
    ];
    println!("lane | smp/ch err resync ch sr | mine_B ff_B | exact diff maxLSB md5_mine md5_ff");
    for (name, path) in lanes {
        let (pcm, errors, resyncs, ch, sr, spc) = decode_file(&path);
        let mine_md5 = md5(&pcm);
        std::fs::write(format!("{DIR}/out_{name}.s16"), &pcm).unwrap();
        let ff = Command::new("ffmpeg")
            .args([
                "-v", "error", "-y", "-i", &path, "-map", "0:a", "-c:a", "pcm_s16le",
                "-f", "s16le", &format!("{DIR}/ff_{name}.s16"),
            ])
            .status()
            .expect("ffmpeg");
        assert!(ff.success());
        let ref_pcm = std::fs::read(format!("{DIR}/ff_{name}.s16")).unwrap();
        let ff_md5 = md5(&ref_pcm);
        let n = pcm.len().min(ref_pcm.len()) / 2;
        let (mut exact, mut diff, mut maxd) = (0u64, 0u64, 0i64);
        for k in 0..n {
            let a = i16::from_le_bytes([pcm[2 * k], pcm[2 * k + 1]]) as i64;
            let b = i16::from_le_bytes([ref_pcm[2 * k], ref_pcm[2 * k + 1]]) as i64;
            let d = (a - b).abs();
            if d == 0 {
                exact += 1
            } else {
                diff += 1
            }
            if d > maxd {
                maxd = d
            }
        }
        println!(
            "{name} | {spc} {errors} {resyncs} {ch} {sr} | {} {} | {exact} {diff} {maxd} | {} {}",
            pcm.len(),
            ref_pcm.len(),
            &mine_md5[..16],
            &ff_md5[..16]
        );
    }
}

fn hostile() {
    let data = std::fs::read(format!("{DIR}/lanes/corpus_0001.ac3")).unwrap();
    let (frames, _) = syncwalk(&data);
    let good = frames[0].clone();
    let params = CodecParameters::audio(CodecId::new("ac3"));
    let tb = TimeBase::new(1, 44_100);

    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("truncated_frame", good[..good.len() / 2].to_vec()),
        ("garbage_midstream", {
            let mut v = good.clone();
            for b in v.iter_mut().skip(40).take(200) {
                *b ^= 0xA5;
            }
            v
        }),
        ("zero_length", vec![]),
        ("corrupt_fscod_reserved", {
            let mut v = good.clone();
            v[4] = (v[4] & 0x3F) | 0xC0; // fscod = 3 (reserved)
            v
        }),
        ("corrupt_frmsizecod_63", {
            let mut v = good.clone();
            v[4] = (v[4] & 0xC0) | 0x3F; // frmsizecod = 63 (reserved)
            v
        }),
        ("garbage_only", {
            let mut v = vec![0x0Bu8, 0x77];
            v.extend(std::iter::repeat_n(0xEE, 4000));
            v
        }),
    ];
    let mut panics = 0;
    for (name, payload) in &cases {
        // warm decoder (real prior state), then the hostile packet;
        // fresh decoder per case so poisoned state can't leak.
        let mut dec = make_decoder(&params).expect("make_decoder");
        let _ = dec.send_packet(&Packet::new(0, tb, good.clone()));
        let _ = dec.receive_frame();
        let r = catch_unwind(AssertUnwindSafe(|| {
            match dec.send_packet(&Packet::new(0, tb, payload.clone())) {
                Err(e) => format!("Err({e:?})"),
                Ok(()) => match dec.receive_frame() {
                    Err(e) => format!("Err({e:?})"),
                    Ok(Frame::Audio(a)) => format!(
                        "Ok(AudioFrame samples={} bytes={})",
                        a.samples,
                        a.data[0].len()
                    ),
                    Ok(_) => "Ok(non-audio)".into(),
                },
            }
        }));
        match r {
            Ok(s) => println!("hostile {name}: {s}"),
            Err(_) => {
                panics += 1;
                println!("hostile {name}: PANIC!");
            }
        }
    }
    println!("hostile: {panics} panics / {} cases", cases.len());
}

/// Print per-block coupling usage (cplinu) using the crate's own
/// side-info parser — evidence for the coupling hypothesis.
fn cplprobe(path: &str) {
    let data = std::fs::read(path).expect("read");
    let mut off = 0usize;
    let mut cpl_blocks = 0usize;
    let mut dith_blocks = 0usize;
    let mut total_blocks = 0usize;
    let mut frames = 0usize;
    while off + 5 <= data.len() {
        let Ok(si) = oxideav_ac3::syncinfo::parse(&data[off..]) else { break };
        let flen = si.frame_length as usize;
        if off + flen > data.len() { break; }
        let Ok(b) = oxideav_ac3::bsi::parse(&data[off + 5..]) else { break; };
        let frame = &data[off..off + flen];
        if let Ok(side) = oxideav_ac3::audblk::parse_frame_side_info(&si, &b, frame) {
            for s in &side {
                total_blocks += 1;
                if s.cplinu { cpl_blocks += 1; }
                if s.dithflag.iter().take(b.nfchans as usize).any(|&d| d) { dith_blocks += 1; }
            }
        }
        frames += 1;
        off += flen;
    }
    println!("cplprobe {path}: {frames} frames, {cpl_blocks}/{total_blocks} cplinu {cpl_blocks}/{total_blocks}, dithflag {dith_blocks}/{total_blocks}");
}
