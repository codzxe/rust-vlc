//! Audio-pipeline throughput measurements (card W5-19, D-10
//! measure-first): std-only, `#[ignore]` tests — run explicitly with
//! `cargo test -p audio-engine --release --ignored`; the committed
//! numbers live in the W5-19 worklog entry.
//!
//! Two probes over the **committed fixture set** (the class-A parent
//! AVI's own 60 × 1470-frame stereo s16le chunks — real
//! writer-authored media; the walker below is pinned by chunk count,
//! per-chunk size and an FNV-1a digest of the extracted stream, and
//! the same bytes are digest-pinned end-to-end by the codecs
//! `pcm_fixtures` suite against the writer's recorded sha256):
//!
//! 1. `pipeline_throughput_bypass` — the full per-quantum pump path
//!    `process → mix → take_output → drop` at zero correction: stage
//!    A s16→f32 conversion, identity channel map, unity single-track
//!    mix, stage D f32 interleave into the pooled output slot (the
//!    drift-resampler bypass — the common playback shape).
//! 2. `pipeline_throughput_drift_corrected` — the same path with a
//!    fixed +600 ppm correction in force: the mixed planes route
//!    through the micro-resampler between mix and stage D (the
//!    3.B1 corrected shape).
//!
//! Protocol: one warm-up pass, then `REPS` timed passes over all 60
//! quanta (each pass = 352 800 input bytes); every pass's MB/s is
//! printed (raw data on stderr), with the median and p95 on the one
//! machine-parseable stdout line. Debug builds assert only a sanity
//! floor — the measurements are release-profile claims.

use audio_engine::{AudioPipeline, PipelineSpec};
use media_types::frame::{AudioFormat, AudioFrame, AudioLayout, SampleFormat};
use media_types::ids::GenerationId;
use media_types::packet::SharedBytes;
use media_types::time::{Stamp, Timestamp};

/// The committed class-A parent fixture (read-only; the audio
/// extraction is pinned below).
const AVI: &str = "generated/valid/avi_mjpeg_pcm.avi";
const RATE: u32 = 44_100;
const CHUNKS: usize = 60;
const CHUNK_BYTES: usize = 5880;
const QUANTUM_FRAMES: usize = 1470;
const PASS_BYTES: f64 = (CHUNKS * CHUNK_BYTES) as f64;
/// Timed passes (N ≥ 10 for median + p95).
const REPS: usize = 15;
/// The drift correction of the corrected probe (inside the ±1000 ppm
/// design bound; the magnitude the zero-alloc resample proof uses).
const DRIFT_PPM: i64 = 600;

/// The extraction walker's digest pin (FNV-1a 64 over the
/// concatenated 01wb stream — computed from the committed fixture;
/// any fixture or walker change must re-pin it deliberately).
const STREAM_FNV1A64: u64 = 0x12f1_a740_97ed_28ca;

fn u32le(data: &[u8], at: usize) -> Option<u32> {
    data.get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Extract the movi list's `01wb` payloads (the test-side walker
/// pattern of codecs' `common` module — pinned by the digest below,
/// the same discipline its own suite uses).
fn extract_pcm_chunks(data: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"AVI " {
        return Err("not an AVI RIFF file".to_string());
    }
    let riff_end = 8usize + u32le(data, 4).ok_or("truncated RIFF size")? as usize;
    let mut pos = 12usize;
    let mut movi: Option<(usize, usize)> = None;
    while pos + 8 <= data.len() && pos < riff_end {
        let id = &data[pos..pos + 4];
        let size = u32le(data, pos + 4).ok_or("truncated chunk size")? as usize;
        let body = pos + 8;
        if id == b"LIST" && data.get(body..body + 4) == Some(b"movi") {
            movi = Some((body + 4, body + size));
            break;
        }
        let advanced = body + size + (size & 1);
        if advanced <= pos {
            return Err(format!("non-advancing top-level chunk at {pos}"));
        }
        pos = advanced;
    }
    let (start, end) = movi.ok_or("no movi list")?;
    let mut chunks = Vec::new();
    let mut cursor = start;
    while cursor + 8 <= end && cursor + 8 <= data.len() {
        let id = &data[cursor..cursor + 4];
        let size = u32le(data, cursor + 4).ok_or("truncated chunk size")? as usize;
        let body = cursor + 8;
        if id == b"01wb" {
            let payload = data
                .get(body..body + size)
                .ok_or("truncated 01wb payload")?;
            chunks.push(payload.to_vec());
        }
        cursor = body + size + (size & 1);
    }
    Ok(chunks)
}

/// The fixture's PCM chunks, digest-pinned.
fn fixture_chunks() -> Vec<Vec<u8>> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(AVI);
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(e) => unreachable!("fixture {}: {e}", path.display()),
    };
    let chunks = match extract_pcm_chunks(&data) {
        Ok(c) => c,
        Err(e) => unreachable!("walker: {e}"),
    };
    assert_eq!(chunks.len(), CHUNKS, "the walker must find the 60 ticks");
    assert!(chunks.iter().all(|c| c.len() == CHUNK_BYTES));
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for chunk in &chunks {
        for byte in chunk {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    assert_eq!(
        hash, STREAM_FNV1A64,
        "the extracted stream's digest changed — re-pin deliberately"
    );
    chunks
}

/// One fixture chunk as a decoded-contract `AudioFrame` (what the
/// pump hands `process`: s16le interleaved at the mixer rate).
fn quantum_frame(chunk: &[u8], index: usize) -> AudioFrame {
    AudioFrame {
        format: AudioFormat {
            sample: SampleFormat::S16,
            channels: 2,
            layout: AudioLayout::Interleaved,
            rate: RATE,
        },
        samples: SharedBytes::from_vec(chunk.to_vec()),
        pts: Stamp::Present(
            Timestamp::from_parts((index * QUANTUM_FRAMES) as i64, 1, RATE)
                .unwrap_or_else(|e| unreachable!("{e}")),
        ),
        duration: None,
        generation: GenerationId::FIRST,
    }
}

/// The shared bench body: warm up one pass, then time `REPS` passes of
/// the full pump path. Returns the per-pass MB/s samples.
fn bench(tag: &str, drift_ppm: i64) -> Vec<f64> {
    let chunks = fixture_chunks();
    let frames: Vec<AudioFrame> = chunks
        .iter()
        .enumerate()
        .map(|(index, chunk)| quantum_frame(chunk, index))
        .collect();
    let mut pipeline = match AudioPipeline::new(PipelineSpec {
        rate: RATE,
        channels: 2,
        tracks: 1,
    }) {
        Ok(p) => p,
        Err(e) => unreachable!("{e}"),
    };
    if drift_ppm != 0 {
        pipeline.set_drift(media_clock::DriftCorrection::new(drift_ppm));
    }
    // Warm-up: one pass sizes the stage-A scratch, the staged slot,
    // the mixer planes, the resampler's planes and the pooled output
    // slots (doc 03 §4.8 c).
    for frame in &frames {
        match pipeline.process(0, frame) {
            Ok(()) => {}
            Err(e) => unreachable!("warm-up process: {e}"),
        }
        match pipeline.mix() {
            Ok(true) => {}
            other => unreachable!("warm-up mix: {other:?}"),
        }
        drop(pipeline.take_output());
    }
    let mut samples = Vec::with_capacity(REPS);
    for _ in 0..REPS {
        let start = std::time::Instant::now();
        for frame in &frames {
            match pipeline.process(0, frame) {
                Ok(()) => {}
                Err(e) => unreachable!("timed process: {e}"),
            }
            match pipeline.mix() {
                Ok(true) => {}
                other => unreachable!("timed mix: {other:?}"),
            }
            drop(pipeline.take_output());
        }
        let elapsed = start.elapsed().as_secs_f64();
        samples.push(PASS_BYTES / elapsed / 1e6);
    }
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

#[test]
#[ignore = "W5-19 throughput measurement: cargo test -p audio-engine --release --ignored (numbers in the W5-19 worklog entry)"]
fn pipeline_throughput_bypass() {
    let samples = bench("pipeline-bypass", 0);
    let median = report("pipeline_bypass", &samples);
    // Honest floor: the bypass path (convert + map + mix + interleave,
    // ~5 linear passes over the quantum) must run far past realtime —
    // release expects ≥ 200 MB/s (≈ 1135× the 176.4 KB/s realtime
    // stereo stream). Debug: sanity only.
    let floor = if cfg!(debug_assertions) { 20.0 } else { 200.0 };
    assert!(
        median >= floor,
        "{median:.1} MB/s < {floor:.1} MB/s floor (N={REPS})"
    );
}

#[test]
#[ignore = "W5-19 throughput measurement: cargo test -p audio-engine --release --ignored (numbers in the W5-19 worklog entry)"]
fn pipeline_throughput_drift_corrected() {
    let samples = bench("pipeline-drift-corrected", DRIFT_PPM);
    let median = report("pipeline_drift_corrected", &samples);
    // Honest floor: the corrected path adds the linear-kernel
    // resampler between mix and stage D — release expects ≥ 150 MB/s.
    // Debug: sanity only.
    let floor = if cfg!(debug_assertions) { 15.0 } else { 150.0 };
    assert!(
        median >= floor,
        "{median:.1} MB/s < {floor:.1} MB/s floor (N={REPS})"
    );
}
