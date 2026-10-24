//! FLAC decode throughput (card 5.A1): the W5-19 `pcm_bench.rs`
//! convention — an ignored measurement test whose number goes into the
//! card's worklog entry, never the normal gate run.
//!
//! The measured shape is the **product contract path** (configure +
//! one packet per frame + sink) over the real corpus FLAC
//! `0046-audio-only.flac` (88.2 kHz stereo 24-bit, 44 frames, 352 800
//! inter-channel samples) — the heaviest real input the card owns.
//! The corpus is /tmp-resident; when absent the test SKIPs with a
//! message (same honesty rule as `flac_bitexact.rs`).

mod common;
mod flac;

use std::time::Instant;

use flac::{corpus_dir, decode_all};
use testkit::Must;

/// Repetitions per measurement (median reported, as in pcm_bench).
const REPS: usize = 7;

#[test]
#[ignore = "5.A1 throughput measurement: cargo test -p codecs --release --ignored (number in the 5.A1 worklog entry)"]
fn flac_decode_throughput_corpus_0046() {
    let Some(root) = corpus_dir() else {
        eprintln!("corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP");
        return;
    };
    let data = std::fs::read(root.join("media/0046-audio-only.flac")).must();

    // Warm-up + facts (the decode is deterministic; every rep is
    // byte-identical, so only timing varies).
    let (layout, sink) = decode_all(&data);
    let total_samples: u64 = sink
        .first_samples
        .last()
        .zip(sink.buffers.last())
        .map(|(first, buf)| first + u64::try_from(buf.len()).must() / 6)
        .unwrap_or(0);
    assert_eq!(
        total_samples,
        layout.stream_info.total_samples.unwrap_or(0),
        "decoded coverage must equal the declared total"
    );

    let mut samples_per_sec: Vec<f64> = Vec::with_capacity(REPS);
    for _ in 0..REPS {
        let start = Instant::now();
        let (_, sink) = decode_all(&data);
        let elapsed = start.elapsed();
        assert!(!sink.buffers.is_empty(), "every rep decodes 44 frames");
        samples_per_sec.push(total_samples as f64 / elapsed.as_secs_f64());
    }
    samples_per_sec.sort_by(|a, b| a.total_cmp(b));
    let median = samples_per_sec[REPS / 2];
    let rate = f64::from(layout.stream_info.sample_rate);
    println!(
        "flac_decode_corpus_0046: {median:.0} inter-channel samples/s \
         (median of {REPS}; {:.1}x realtime at {} Hz, 24-bit stereo)",
        median / rate,
        layout.stream_info.sample_rate
    );
    // Honest floor: a software FLAC decoder is memory-light arithmetic;
    // release must sustain at least 5x realtime (debug: sanity only).
    let floor = if cfg!(debug_assertions) {
        50_000.0
    } else {
        500_000.0
    };
    assert!(
        median >= floor,
        "{median:.0} samples/s < {floor:.0} samples/s floor (N={REPS})"
    );
}
