//! MP2 decode throughput (card 5.J3 → 5.X1's note): the 5.A3
//! `mp3_bench.rs` convention — an ignored measurement test whose
//! number goes into the card's worklog entry, never the normal gate
//! run.
//!
//! The measured shape is the whole-stream free surface
//! (`decode_stream`: ID3 skip + Layer II walk + CBR gate + engine
//! decode) over the real corpus MP2s `0119-audio-only.mp2`
//! (MPEG-1 L2 384 kbps 44.1 kHz stereo, 154 frames) and
//! `0005-audio-only.mp2` (MPEG-2 LSF L2 80 kbps 16 kHz mono,
//! 56 frames). The corpus is /tmp-resident; when absent the corpus
//! rows SKIP with a message (the fixture row is the always-on
//! measurement).

mod common;
mod mp2;

use std::time::Instant;

use mp2::{decode_s16le, fixture_bytes};

/// Repetitions per measurement (median reported).
const REPS: usize = 7;

fn bench(label: &str, data: &[u8]) {
    // Warm-up + decode-coverage fact.
    let (decoded, _) = match decode_s16le(data) {
        Ok(x) => x,
        Err(e) => unreachable!("{e}"),
    };
    let inter_channel: u64 = decoded.samples.len() as u64;
    let channels = u64::from(decoded.format.channels.max(1));
    let rate = u64::from(decoded.format.rate.max(1));
    let seconds = inter_channel as f64 / channels as f64 / rate as f64;
    let mut per_sec: Vec<f64> = Vec::with_capacity(REPS);
    for _ in 0..REPS {
        let start = Instant::now();
        let (d, _) = match decode_s16le(data) {
            Ok(x) => x,
            Err(e) => unreachable!("{e}"),
        };
        let _ = d.samples.len();
        let elapsed = start.elapsed().as_secs_f64();
        per_sec.push(inter_channel as f64 / elapsed);
    }
    per_sec.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = per_sec[REPS / 2];
    let realtime = median / (channels as f64 * rate as f64);
    eprintln!(
        "{label}: {inter_channel} inter-channel samples, median {median:.0} samples/s \
         = {realtime:.0}x realtime ({seconds:.3}s of audio)"
    );
}

#[test]
#[ignore = "5.J3 throughput measurement: cargo test -p codecs --release --test mp2_bench -- --ignored (number in the 5.J3 worklog entry)"]
fn mp2_decode_throughput() {
    bench("fixture mp2_48k.mp2", &fixture_bytes("mp2_48k.mp2"));
    let Some(corpus) = mp2::corpus_dir() else {
        eprintln!(
            "corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP corpus rows"
        );
        return;
    };
    for name in ["0119-audio-only.mp2", "0005-audio-only.mp2"] {
        let path = corpus.join("media").join(name);
        match std::fs::read(&path) {
            Ok(data) => bench(&format!("corpus {name}"), &data),
            Err(_) => eprintln!("corpus {name} absent — SKIP row"),
        }
    }
}
