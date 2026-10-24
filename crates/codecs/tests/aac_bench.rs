//! AAC decode throughput (card 5.C2a → 5.X1's note): the 5.A3
//! `mp3_bench.rs` convention — an ignored measurement test whose
//! number goes into the card's worklog entry, never the normal gate
//! run.
//!
//! The measured shape is the whole-stream free surface
//! (`decode_stream`: ADTS walk + header gates + engine decode) over
//! the real corpus lanes `0059-audio-only.aac` (88.2 kHz mono) and
//! `0110-audio-only.aac` (96 kHz mono) — the corpus is /tmp-resident;
//! when absent the rows SKIP with a message.

mod aac;

use std::time::Instant;

use aac::{corpus_dir, corpus_media, decode_s16le};
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
#[ignore = "5.C2a throughput measurement: cargo test -p codecs --release --test aac_bench -- --ignored (number in the 5.C2a worklog entry)"]
fn aac_decode_throughput() {
    match corpus_dir() {
        Some(corpus) => {
            for name in ["0059-audio-only.aac", "0110-audio-only.aac"] {
                match corpus_media(&corpus, name) {
                    Some(data) => bench(&format!("corpus {name}"), &data),
                    None => eprintln!("corpus {name} absent — SKIP row"),
                }
            }
        }
        None => {
            eprintln!("corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP rows")
        }
    }
    bench(
        "fixture sine441_lc_stereo.aac",
        &aac::fixture_bytes("sine441_lc_stereo.aac"),
    );
}
