//! MJPEG decode throughput measurements (ED-FIX-1, audit M13 / Lead
//! decision ED-a: std-only, `#[ignore]` tests — run explicitly with
//! `cargo test -p codecs --release --ignored`; the numbers are
//! committed in the worklog ED-FIX-1 entry).
//!
//! Two probes, both through the **contract path** (`MjpegDecoder` +
//! sink, the real pipeline surface — persistent `DecodeContext`,
//! per-frame output handoff included):
//!
//! 1. `mjpeg_decode_throughput_contract_path` — the largest committed
//!    fixture (`tests/fixtures/generated/valid/avi_mjpeg_pcm.avi`,
//!    60 deterministic 320×240 4:2:0 frames, external-writer-authored
//!    by the 2.A3 generator with committed per-frame sha256), decoded
//!    N passes. Asserts the honest floor: ≥ 2× realtime for the
//!    fixture's 30 fps (i.e. ≥ 60 frames/s) in release builds.
//!
//! 2. `mjpeg_decode_throughput_dense_4k_budget` — a **synthetic**
//!    worst-case 3840×2160 4:2:0 frame (built here; perf-only
//!    synthetic, *not* a correctness fixture — every block carries 63
//!    non-zero ACs, defeating the zero-AC fast path, at Q=16): the
//!    content-density bound for the D-10 pre-commit question ("will
//!    this freeze on a 4K 60fps stream?"). Asserts ≥ 60 frames/s
//!    (realtime 4K60) in release builds.
//!
//! Debug builds assert only a sanity floor (≥ 1 frame/s): the
//! measurements and floors are release-profile claims, and pretending
//! an unoptimized build carries them would be dishonest.

mod common;

use codec_api::prelude::*;
use codecs::video::mjpeg::MjpegDecoder;
use common::{extract_avi_mjpeg_frames, fixture_bytes};
use media_types::{GenerationId, Packet, TrackId};

const AVI: &str = "generated/valid/avi_mjpeg_pcm.avi";
const FRAME_COUNT: usize = 60;
const FRAME_WIDTH: u64 = 320;
const FRAME_HEIGHT: u64 = 240;
const FIXTURE_FPS: f64 = 30.0;

/// A sink that drops frames after counting them (throughput shape:
/// the decode cost is what is measured, not retention).
struct DroppingSink {
    frames: usize,
}

impl FrameSink for DroppingSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        self.frames += 1;
        Ok(())
    }
}

fn configured() -> (MjpegDecoder, GenerationId) {
    let descriptor = match media_types::ids::CodecDescriptor::new("mjpeg") {
        Ok(d) => d,
        Err(e) => unreachable!("descriptor: {e}"),
    };
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => unreachable!("generation: {e}"),
    };
    let mut decoder = MjpegDecoder::new();
    match decoder.configure(&DecoderConfig::new(
        descriptor,
        generation,
        ThreadBudget::default(),
        None,
        None,
    )) {
        Ok(()) => (decoder, generation),
        Err(e) => unreachable!("configure: {e}"),
    }
}

fn packet_for(jpeg: &[u8], generation: GenerationId) -> Packet {
    Packet {
        track: TrackId::new(0),
        pts: media_types::Stamp::Missing,
        dts: media_types::Stamp::Missing,
        duration: None,
        data: media_types::packet::SharedBytes::from_vec(jpeg.to_vec()),
        keyframe: true,
        discontinuity: false,
        generation,
    }
}

/// Decode `jpegs` once through a fresh decoder (warm-up: allocator
/// and caches; the timed passes then measure steady state).
fn warm_up(jpegs: &[Vec<u8>]) {
    let (mut decoder, generation) = configured();
    let mut sink = DroppingSink { frames: 0 };
    for jpeg in jpegs {
        match decoder.decode(packet_for(jpeg, generation), &mut sink) {
            Ok(_) => {}
            Err(e) => unreachable!("warm-up decode: {e}"),
        }
    }
    assert_eq!(sink.frames, jpegs.len());
}

/// Timed passes; returns (total elapsed ns, frames decoded).
fn timed_passes(jpegs: &[Vec<u8>], passes: usize) -> (u128, usize) {
    let (mut decoder, generation) = configured();
    let mut sink = DroppingSink { frames: 0 };
    let start = std::time::Instant::now();
    for _ in 0..passes {
        for jpeg in jpegs {
            match decoder.decode(packet_for(jpeg, generation), &mut sink) {
                Ok(_) => {}
                Err(e) => unreachable!("timed decode: {e}"),
            }
        }
    }
    (start.elapsed().as_nanos(), sink.frames)
}

fn report(tag: &str, frames: usize, elapsed_ns: u128, width: u64, height: u64) -> f64 {
    let ns_per_frame = elapsed_ns as f64 / frames as f64;
    let fps = 1e9 / ns_per_frame;
    let mpx_per_s = fps * (width as f64 * height as f64) / 1e6;
    println!(
        "{tag}: {frames} frames in {elapsed_ns} ns — {ns_per_frame:.0} ns/frame, \
         {fps:.1} frames/s, {mpx_per_s:.1} Mpx/s ({width}x{height})"
    );
    fps
}

fn release_floor(release_fps: f64, debug_fps: f64) -> f64 {
    if cfg!(debug_assertions) {
        debug_fps
    } else {
        release_fps
    }
}

#[test]
#[ignore = "ED-FIX-1 M13 throughput measurement: cargo test -p codecs --release --ignored (numbers in worklog)"]
fn mjpeg_decode_throughput_contract_path() {
    let bytes = match fixture_bytes(AVI) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let jpegs = match extract_avi_mjpeg_frames(&bytes) {
        Ok(f) => f,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(jpegs.len(), FRAME_COUNT);

    warm_up(&jpegs);
    // ~600-6000 frames total: enough passes for a stable mean.
    let passes = (600 / FRAME_COUNT).max(1);
    let (elapsed, frames) = timed_passes(&jpegs, passes);
    let fps = report(
        "fixture 320x240 4:2:0 (largest committed fixture, 60 frames)",
        frames,
        elapsed,
        FRAME_WIDTH,
        FRAME_HEIGHT,
    );
    // Honest floor: ≥ 2x realtime for the fixture's 30 fps in
    // release (≥ 60 frames/s); debug builds only sanity-check.
    let floor = release_floor(2.0 * FIXTURE_FPS, 1.0);
    assert!(
        fps >= floor,
        "{fps:.1} frames/s < {floor:.1} frames/s floor (ns/frame {:.0})",
        elapsed as f64 / frames as f64
    );
}

/// Build the synthetic dense-AC 4K frame (see module docs). Entropy:
/// every 2-bit window of a `0x55` fill is a valid (code, amplitude)
/// pair — DC size 2 then 63× (run 0, size 1) per block — so every
/// block takes the full factored IDCT with 63 non-zero ACs and the
/// zero-AC fast path never fires.
fn dense_4k_jpeg() -> Vec<u8> {
    const WIDTH: u16 = 3840;
    const HEIGHT: u16 = 2160;
    let mut jpeg = vec![0xff, 0xd8];
    let seg = |jpeg: &mut Vec<u8>, marker: u8, payload: &[u8]| {
        jpeg.push(0xff);
        jpeg.push(marker);
        jpeg.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        jpeg.extend_from_slice(payload);
    };
    // DQT: Tq=0, all 16s.
    let mut dqt = vec![0u8];
    dqt.extend(std::iter::repeat_n(16u8, 64));
    seg(&mut jpeg, 0xdb, &dqt);
    // DHT DC0: two 1-bit codes, both size-2.
    let mut dht = vec![0x00u8, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    dht.extend_from_slice(&[0x02, 0x02]);
    seg(&mut jpeg, 0xc4, &dht);
    // DHT AC0: two 1-bit codes, both (run 0, size 1).
    let mut dht = vec![0x10u8, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    dht.extend_from_slice(&[0x01, 0x01]);
    seg(&mut jpeg, 0xc4, &dht);
    // SOF0: 2160x3840, 4:2:0.
    let mut sof = vec![8u8];
    sof.extend_from_slice(&HEIGHT.to_be_bytes());
    sof.extend_from_slice(&WIDTH.to_be_bytes());
    sof.push(3);
    sof.extend_from_slice(&[1, 0x22, 0, 2, 0x11, 0, 3, 0x11, 0]);
    seg(&mut jpeg, 0xc0, &sof);
    // SOS: all three components, Td=0 Ta=0.
    seg(&mut jpeg, 0xda, &[3, 1, 0x00, 2, 0x00, 3, 0x00, 0, 63, 0]);
    // Entropy: 129 bits per block x 194,400 blocks (4:2:0 MCU grid
    // 240x135, 6 blocks per MCU), rounded up to bytes.
    let blocks = 240usize * 135 * 6;
    let entropy_bytes = blocks * 129 / 8 + 1;
    jpeg.resize(jpeg.len() + entropy_bytes, 0x55);
    jpeg.extend_from_slice(&[0xff, 0xd9]);
    jpeg
}

#[test]
#[ignore = "ED-FIX-1 M13 4K dense-content budget probe: cargo test -p codecs --release --ignored (numbers in worklog)"]
fn mjpeg_decode_throughput_dense_4k_budget() {
    let jpeg = dense_4k_jpeg();
    // Sanity: it decodes cleanly (dense, nothing concealed) at 4K.
    let image = match codecs::video::mjpeg::decode_image(&jpeg) {
        Ok(image) => image,
        Err(e) => unreachable!("dense 4K frame decodes: {e}"),
    };
    assert_eq!((image.width, image.height), (3840, 2160));
    assert!(
        !image.concealed,
        "the dense frame must decode without concealment"
    );
    assert_eq!(image.data.len(), 3840 * 2160 * 3 / 2, "I420 layout");

    warm_up(std::slice::from_ref(&jpeg));
    let passes = if cfg!(debug_assertions) { 1 } else { 12 };
    let (elapsed, frames) = timed_passes(std::slice::from_ref(&jpeg), passes);
    let fps = report(
        "synthetic dense-AC 3840x2160 4:2:0 (worst case)",
        frames,
        elapsed,
        3840,
        2160,
    );
    // The 4K60 budget question (D-10 q2): realtime is 60 frames/s.
    // Release asserts realtime; debug only sanity-checks.
    let floor = release_floor(60.0, 1.0);
    assert!(
        fps >= floor,
        "{fps:.1} frames/s < {floor:.1} frames/s floor (ns/frame {:.0})",
        elapsed as f64 / frames as f64
    );
}
