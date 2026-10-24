//! MP3 decode throughput (card 5.A3 → 5.X1's note): the W5-19
//! `pcm_bench.rs` / 5.A1 `flac_bench.rs` convention — an ignored
//! measurement test whose number goes into the card's worklog entry,
//! never the normal gate run.
//!
//! The measured shape is the whole-stream free surface
//! (`decode_stream`: ID3 skip + frame walk + Xing/gapless + engine
//! decode + trim) over the real corpus MP3 `0139-audio-only.mp3`
//! (11.025 kHz MPEG-2.5 stereo, 88 200 inter-channel samples) and the
//! heaviest committed fixture `cbr128.mp3` (44.1 kHz MPEG-1 stereo,
//! 176 400 inter-channel samples). The corpus is /tmp-resident; when
//! absent the corpus row SKIPs with a message (the fixture row is the
//! always-on measurement).

mod common;
mod mp3;

use std::time::Instant;

use mp3::{decode_s16le, fixture_bytes};
use testkit::Must;

/// Repetitions per measurement (median reported).
const REPS: usize = 7;

fn bench(label: &str, data: &[u8]) {
    // Warm-up + decode-coverage fact.
    let (decoded, _) = decode_s16le(data).must();
    let inter_channel: u64 = decoded.samples.len() as u64;
    let channels = u64::from(decoded.format.channels.max(1));
    let rate = u64::from(decoded.format.rate.max(1));
    let seconds = inter_channel as f64 / channels as f64 / rate as f64;
    let mut per_sec: Vec<f64> = Vec::with_capacity(REPS);
    for _ in 0..REPS {
        let start = Instant::now();
        let (d, _) = decode_s16le(data).must();
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
#[ignore = "5.A3 throughput measurement: cargo test -p codecs --release --test mp3_bench -- --ignored (number in the 5.A3 worklog entry)"]
fn mp3_decode_throughput() {
    bench("fixture cbr128.mp3", &fixture_bytes("cbr128.mp3"));
    let corpus = mp3::corpus_dir();
    match corpus {
        Some(root) => {
            let path = root.join("media").join("0139-audio-only.mp3");
            match std::fs::read(&path) {
                Ok(data) => bench("corpus 0139-audio-only.mp3", &data),
                Err(_) => eprintln!("corpus 0139 absent — SKIP corpus row"),
            }
        }
        None => eprintln!(
            "corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP corpus row"
        ),
    }
}
