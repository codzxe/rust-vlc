//! PCM decode throughput measurements (card W5-19, D-10 measure-first):
//! std-only, `#[ignore]` tests — run explicitly with
//! `cargo test -p codecs --release --ignored`; the committed numbers
//! live in the W5-19 worklog entry.
//!
//! Two probes, both through the **contract path** (`PcmDecoder` +
//! sink, the real pipeline surface — configure, per-packet decode,
//! frame handoff included), both over the **committed fixture set**
//! (the class-A parent AVI's own 60 × 1470-frame stereo s16le chunks
//! — real writer-authored media, digest-pinned by `pcm_fixtures`):
//!
//! 1. `pcm_decode_throughput_s16le_passthrough` — the zero-copy
//!    shape (native byte order, whole packets): decode = prefix
//!    arithmetic + the SharedBytes handoff. This is the floor every
//!    real s16le playback takes.
//! 2. `pcm_decode_throughput_s16be_converted` — the
//!    conversion-carrying shape (the same samples byte-swapped): the
//!    pooled-slot group-swap path.
//!
//! Protocol: one warm-up pass, then `REPS` timed passes over all 60
//! chunks (each pass = 352 800 bytes); every pass's MB/s is printed
//! (raw data on stderr), with the median and p95 on the one
//! machine-parseable stdout line. Debug builds assert only a sanity
//! floor — the measurements are release-profile claims.

mod common;

use codec_api::prelude::*;
use codecs::audio::pcm::PcmDecoder;
use common::{extract_avi_audio_strf, extract_avi_pcm_chunks, fixture_bytes};
use media_types::ids::GenerationId;
use media_types::packet::SharedBytes;
use media_types::{Packet, TrackId};

/// The committed class-A parent fixture: 60 chunks × 5880 bytes.
const AVI: &str = "generated/valid/avi_mjpeg_pcm.avi";
const CHUNKS: usize = 60;
const CHUNK_BYTES: usize = 5880;
const PASS_BYTES: f64 = (CHUNKS * CHUNK_BYTES) as f64;
/// Timed passes (N ≥ 10 for median + p95).
const REPS: usize = 15;

/// A sink that drops frames after counting them (throughput shape).
struct DroppingSink {
    frames: usize,
}

impl FrameSink for DroppingSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        self.frames += 1;
        Ok(())
    }
}

fn configured(descriptor: &str, extra: Vec<u8>) -> (PcmDecoder, GenerationId) {
    let descriptor = match media_types::ids::CodecDescriptor::new(descriptor) {
        Ok(d) => d,
        Err(e) => unreachable!("{e}"),
    };
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(e) => unreachable!("{e}"),
    };
    let mut decoder = PcmDecoder::new();
    match decoder.configure(&DecoderConfig::new(
        descriptor,
        generation,
        ThreadBudget::default(),
        None,
        Some(SharedBytes::from_vec(extra)),
    )) {
        Ok(()) => (decoder, generation),
        Err(e) => unreachable!("configure: {e}"),
    }
}

fn packet_for(bytes: &[u8], generation: GenerationId) -> Packet {
    Packet {
        track: TrackId::new(1),
        pts: media_types::Stamp::Missing,
        dts: media_types::Stamp::Missing,
        duration: None,
        data: SharedBytes::from_vec(bytes.to_vec()),
        keyframe: true,
        discontinuity: false,
        generation,
    }
}

/// The shared bench body: warm up one pass, then time `REPS` passes.
/// Returns the per-pass MB/s samples (raw data, also on stderr).
fn bench(tag: &str, descriptor: &str, chunks: &[Vec<u8>], strf: &[u8]) -> Vec<f64> {
    let (mut decoder, generation) = configured(descriptor, strf.to_vec());
    let mut sink = DroppingSink { frames: 0 };
    // Warm-up: allocator, plan, (for converted shapes) the slot pool.
    for chunk in chunks {
        match decoder.decode(packet_for(chunk, generation), &mut sink) {
            Ok(_) => {}
            Err(e) => unreachable!("warm-up decode: {e}"),
        }
    }
    assert_eq!(sink.frames, CHUNKS);
    // The timed passes: packets pre-built outside the timing loops.
    let packets: Vec<Packet> = chunks
        .iter()
        .map(|chunk| packet_for(chunk, generation))
        .collect();
    let mut samples = Vec::with_capacity(REPS);
    for _ in 0..REPS {
        let start = std::time::Instant::now();
        for packet in &packets {
            match decoder.decode(packet.clone(), &mut sink) {
                Ok(_) => {}
                Err(e) => unreachable!("timed decode: {e}"),
            }
        }
        let elapsed = start.elapsed().as_secs_f64();
        samples.push(PASS_BYTES / elapsed / 1e6);
    }
    assert_eq!(sink.frames, CHUNKS * (REPS + 1));
    eprintln!(
        "[{tag}] raw per-pass MB/s (N={REPS}): {}",
        samples
            .iter()
            .map(|v| format!("{v:.1}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    samples
}

/// Median + p95 of the samples (linear interpolation between order
/// statistics — the honest small-N estimator).
fn stats(samples: &[f64]) -> (f64, f64) {
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let at = |q: f64| -> f64 {
        let pos = q * (sorted.len() - 1) as f64;
        let low = pos.floor() as usize;
        let high = pos.ceil() as usize;
        sorted[low] + (sorted[high] - sorted[low]) * (pos - low as f64)
    };
    (at(0.5), at(0.95))
}

fn report(tag: &str, samples: &[f64]) -> f64 {
    let (median, p95) = stats(samples);
    println!(
        "BENCH {tag}: median={median:.1} MB/s p95={p95:.1} MB/s N={} passes={}bytes",
        samples.len(),
        PASS_BYTES as u64
    );
    median
}

fn fixture_material() -> (Vec<u8>, Vec<Vec<u8>>) {
    let bytes = match fixture_bytes(AVI) {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let strf = match extract_avi_audio_strf(&bytes) {
        Ok(s) => s,
        Err(e) => unreachable!("{e}"),
    };
    let chunks = match extract_avi_pcm_chunks(&bytes) {
        Ok(c) => c,
        Err(e) => unreachable!("{e}"),
    };
    assert_eq!(chunks.len(), CHUNKS);
    assert!(chunks.iter().all(|c| c.len() == CHUNK_BYTES));
    (strf, chunks)
}

#[test]
#[ignore = "W5-19 throughput measurement: cargo test -p codecs --release --ignored (numbers in the W5-19 worklog entry)"]
fn pcm_decode_throughput_s16le_passthrough() {
    let (strf, chunks) = fixture_material();
    let samples = bench("pcm-s16le-passthrough", "pcm_s16le", &chunks, &strf);
    let median = report("pcm_decode_s16le_passthrough", &samples);
    // Honest floor: the passthrough must beat a plain memcpy of the
    // same bytes by staying within a small factor of memory speed —
    // in release, ≥ 500 MB/s (the shape is prefix arithmetic + an
    // Arc bump; anything slower is a regression). Debug: sanity only.
    let floor = if cfg!(debug_assertions) { 50.0 } else { 500.0 };
    assert!(
        median >= floor,
        "{median:.1} MB/s < {floor:.1} MB/s floor (N={REPS})"
    );
}

#[test]
#[ignore = "W5-19 throughput measurement: cargo test -p codecs --release --ignored (numbers in the W5-19 worklog entry)"]
fn pcm_decode_throughput_s16be_converted() {
    let (strf, chunks) = fixture_material();
    // The byte-swapped twin: the same samples as an s16be stream (the
    // conversion-carrying shape — pooled-slot group swap).
    let swapped: Vec<Vec<u8>> = chunks
        .iter()
        .map(|chunk| {
            let mut out = chunk.clone();
            for pair in out.as_chunks_mut::<2>().0 {
                pair.reverse();
            }
            out
        })
        .collect();
    let samples = bench("pcm-s16be-converted", "pcm_s16be", &swapped, &strf);
    let median = report("pcm_decode_s16be_converted", &samples);
    // Honest floor: the converted path writes every byte once more
    // than the passthrough (the in-place group swap) — release
    // expects ≥ 250 MB/s. Debug: sanity only.
    let floor = if cfg!(debug_assertions) { 25.0 } else { 250.0 };
    assert!(
        median >= floor,
        "{median:.1} MB/s < {floor:.1} MB/s floor (N={REPS})"
    );
}
