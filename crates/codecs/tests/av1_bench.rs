//! AV1 decode throughput, speed-honesty per doc 12 §1.3 (card 5.E2):
//! **report the gap honestly** — rav1d 1.1.0 is admitted with `asm`
//! OFF (purity, `docs/dependencies/records/rav1d-1.1.0.md`), so this
//! card measures what that costs against ffmpeg's `libdav1d` (asm
//! on) on the same file, same machine, same session.
//!
//! The `mp3_bench.rs`/`aac_bench.rs` convention: an **ignored
//! measurement test** whose number goes into the format record
//! (`docs/formats/video-av1.md`) and the 5.E2 worklog entry, never
//! the normal gate run:
//!
//! ```text
//! cargo test -p codecs --all-features --release --test av1_bench -- --ignored --nocapture
//! ```
//!
//! Surface measured, stated exactly (doc 12 §6.4 attribution):
//!
//! - **ours**: in-process full decode surface — IVF demux (packet
//!   walk) + rav1d-asm-off decode + I420 pooled copy-out, over the
//!   whole file; median of 5 runs after a warm-up (doc 12 §3.6:
//!   fixed run-selection, median, never re-run until liked);
//! - **ffmpeg**: wall clock of `ffmpeg -c:v libdav1d -i f.ivf
//!   -f null -` (process spawn + demux + libdav1d-asm decode +
//!   null mux), median of 5 — an *asymmetric* full-pipeline number
//!   (doc 12 §2.3), labeled as such wherever it is cited.
//!
//! The fixture is generated live (deterministic command, module
//! docs of `av1_conformance.rs`): `testsrc2` 1920x1080 @10 fps,
//! libaom-av1 crf 45 cpu-used 8 → 12 frames; ffmpeg absent ⇒ the
//! row SKIPs with a message.

mod common;

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use codec_api::prelude::*;
use codecs::video::av1::Av1Decoder;
use demux::ivf::IvfDemuxer;
use demux::ivf::test_support::mem_stream;
use media_types::GenerationId;
use media_types::ids::CodecDescriptor;
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

/// Median repetitions (doc 12 §3.6 shape, scaled to the fixture cost).
const REPS: usize = 5;
/// Frames of the 1080p fixture (12 s × ... no: 1.2 s @ 10 fps).
const DURATION: f64 = 1.2;

#[derive(Default)]
struct Sink {
    frames: usize,
    pixels: u64,
}

impl FrameSink for Sink {
    fn accept(&mut self, frame: media_types::frame::Frame) -> Result<(), EngineError> {
        let media_types::frame::Frame::Video(v) = frame else {
            unreachable!("video decoder emits video frames only");
        };
        self.frames += 1;
        self.pixels += v.size.0 as u64 * v.size.1 as u64;
        Ok(())
    }
}

/// The full Lumen surface: demux + configure + decode + flush.
fn decode_file(bytes: &[u8]) -> (usize, u64) {
    let generation = GenerationId::FIRST.next().must();
    let mut demuxer = IvfDemuxer::new(mem_stream(bytes.to_vec()).must()).must();
    demuxer.start().must();
    let mut decoder = Av1Decoder::new();
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new("av1").must(),
            generation,
            ThreadBudget::default(),
            None,
            None,
        ))
        .must();
    let mut sink = Sink::default();
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(mut p)) => {
                p.generation = generation;
                if let Err(err) = decoder.decode(p, &mut sink) {
                    unreachable!("clean fixture must decode: {}", err.context.code);
                }
            }
            Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => unreachable!("clean fixture must demux: {}", err.context.code),
        }
    }
    let _ = decoder.flush(&mut sink);
    (sink.frames, sink.pixels)
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    values[values.len() / 2]
}

#[test]
#[ignore = "5.E2 speed-honesty measurement: cargo test -p codecs --all-features --release --test av1_bench -- --ignored --nocapture (numbers in docs/formats/video-av1.md + the 5.E2 worklog entry)"]
fn av1_decode_throughput_1080p_vs_ffmpeg_libdav1d() {
    let ok = Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if !ok {
        eprintln!("av1_bench: ffmpeg absent — row SKIP (fixture generation is the oracle)");
        return;
    }
    let version = String::from_utf8_lossy(
        &Command::new("ffmpeg")
            .arg("-version")
            .output()
            .must()
            .stdout,
    )
    .lines()
    .next()
    .unwrap_or("ffmpeg ?")
    .to_string();
    eprintln!("oracle: {version} (decoder pinned: -c:v libdav1d)");

    let dir = std::env::temp_dir().join(format!("lumen-av1-bench-{}", std::process::id()));
    std::fs::create_dir_all(&dir).must();
    let path: PathBuf = dir.join("bench_1080p.ivf");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=1920x1080:rate=10",
            "-t",
            &format!("{DURATION}"),
            "-c:v",
            "libaom-av1",
            "-b:v",
            "0",
            "-crf",
            "45",
            "-cpu-used",
            "8",
            "-pix_fmt",
            "yuv420p",
            "-f",
            "ivf",
        ])
        .arg(&path)
        .status()
        .must();
    assert!(status.success(), "fixture generation failed");

    let bytes = std::fs::read(&path).must();
    let (frames, pixels) = decode_file(&bytes); // warm-up + coverage fact
    assert_eq!(frames, (DURATION * 10.0) as usize, "frame count");
    let mpix = pixels as f64 / 1e6;

    // Ours: full in-process surface, median of REPS.
    let mut fps_ours = Vec::new();
    for _ in 0..REPS {
        let start = Instant::now();
        let (f, _) = decode_file(&bytes);
        let elapsed = start.elapsed().as_secs_f64();
        fps_ours.push(f as f64 / elapsed);
    }
    let fps_ours = median(&mut fps_ours);
    let mpix_ours = fps_ours * mpix / frames as f64;

    // ffmpeg: full-pipeline wall clock (asymmetric — see module
    // docs), decoder pinned to libdav1d, median of REPS.
    let mut fps_ffmpeg = Vec::new();
    for _ in 0..REPS {
        let start = Instant::now();
        let status = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error"])
            .arg("-c:v")
            .arg("libdav1d")
            .arg("-i")
            .arg(&path)
            .args(["-f", "null"])
            .arg("-")
            .status()
            .must();
        assert!(status.success());
        fps_ffmpeg.push(frames as f64 / start.elapsed().as_secs_f64());
    }
    let fps_ffmpeg = median(&mut fps_ffmpeg);

    std::fs::remove_dir_all(&dir).must();
    let ratio = fps_ours / fps_ffmpeg;
    eprintln!(
        "av1 1080p ({frames}f, {mpix:.2} Mpix total):\n\
         \x20 ours   rav1d 1.1.0 asm-off, in-process demux+decode+copy-out: \
         {fps_ours:.1} fps = {mpix_ours:.2} Mpix/s\n\
         \x20 ffmpeg {version}: libdav1d (asm on), full pipeline incl. spawn: \
         {fps_ffmpeg:.1} fps\n\
         \x20 asm-off cost ratio (ours/ffmpeg): {ratio:.3}x — doc 12 §1.3 honest-gap record"
    );
}
