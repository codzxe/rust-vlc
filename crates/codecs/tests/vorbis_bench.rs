//! Vorbis decode throughput (card 5.B2 AC): the 5.A1 `flac_bench.rs`
//! convention — an ignored measurement test whose number goes into
//! the card's worklog entry, never the normal gate run.
//!
//! The measured shape is the **product contract path** (ogg demux →
//! `extra_data` configure → one packet per block → sink) over the
//! real corpus Vorbis `0085-audio-only.ogg` (8 kHz stereo, 126 data
//! packets, 32 000 granule samples) — with the committed 44.1 kHz
//! stereo sine fixture as the fallback lane when the /tmp corpus is
//! absent. Median of 7 repetitions, reported as inter-channel
//! samples/sec.

mod common;
mod vorbis;

use std::time::Instant;

use testkit::Must;
use vorbis::{corpus_dir, decode_through_ogg, fixture_bytes};

/// Repetitions per measurement (median reported, as in flac_bench).
const REPS: usize = 7;

#[test]
#[ignore = "5.B2 throughput measurement: cargo test -p codecs --release --ignored (number in the 5.B2 worklog entry)"]
fn vorbis_decode_throughput() {
    // The heaviest real input the card owns, else the fallback lane.
    let (name, data) = match corpus_dir() {
        Some(root) => {
            let path = root.join("media/0085-audio-only.ogg");
            if path.is_file() {
                ("corpus 0085-audio-only.ogg", std::fs::read(&path).must())
            } else {
                eprintln!("corpus 0085 absent — falling back to the 44.1 kHz fixture lane");
                (
                    "vorbis_sine_44100_stereo.ogg",
                    fixture_bytes("vorbis_sine_44100_stereo.ogg"),
                )
            }
        }
        None => {
            eprintln!(
                "corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — \
                 falling back to the 44.1 kHz fixture lane"
            );
            (
                "vorbis_sine_44100_stereo.ogg",
                fixture_bytes("vorbis_sine_44100_stereo.ogg"),
            )
        }
    };

    // Warm-up + coverage sanity (the decode is deterministic; every
    // rep is byte-identical, so only timing varies).
    let warm = decode_through_ogg(&data);
    let channels = warm.headers.identification.channels as usize;
    let total_samples: u64 = warm
        .sink
        .buffers
        .iter()
        .map(|b| (b.len() / (channels * 4)) as u64)
        .sum();
    assert!(
        warm.packets > 0 && total_samples > 0,
        "the lane must decode real audio"
    );
    assert_eq!(
        warm.sink.buffers.len(),
        warm.packets,
        "one packet = one block"
    );

    let mut samples_per_sec: Vec<f64> = Vec::with_capacity(REPS);
    for _ in 0..REPS {
        let start = Instant::now();
        let run = decode_through_ogg(&data);
        let elapsed = start.elapsed();
        assert_eq!(run.packets, warm.packets, "deterministic rep");
        let sps = total_samples as f64 / elapsed.as_secs_f64();
        samples_per_sec.push(sps);
    }
    samples_per_sec.sort_by(|a, b| a.total_cmp(b));
    let median = samples_per_sec[REPS / 2];
    eprintln!(
        "THROUGHPUT {name}: {total_samples} inter-channel samples, {} packets, \
         median {median:.0} samples/sec over {REPS} reps (all: {samples_per_sec:?})",
        warm.packets
    );
}
