//! Opus corpus differential (card 6.M-opus AC): the opus lanes of
//! the v4 corpus, decoded through the **real composition** (ogg
//! demux → `OpusHead ‖ OpusTags` as `extra_data` → the codec-api
//! `OpusDecoder`), checked against the ffmpeg 7.1.5 float oracle.
//!
//! The assertions, in the admission record's own tolerance class:
//!
//! 1. **Geometry** (hard, no oracle needed): packet count, the
//!    OpusHead pre-skip (312 on both lanes), the raw per-channel
//!    decode count (192,960 = 201 × 960 — pre-skip included), and the OUTPUT
//!    count = the last granule (192,000) — i.e. **pre-skip 312 +
//!    end-trim 648 exactness**, binding condition 2 measured.
//! 2. **Live differential** (when an ffmpeg binary is present): our
//!    f32 output vs ffmpeg's, per-sample — max |Δ| recorded, ZERO
//!    samples beyond one s16 LSB (1/32768 ≈ 3.05e-5). Opus is lossy
//!    and both sides are float decoders: bit-exactness is NOT the
//!    bar (the admission record's D note); s16-audible equivalence
//!    is.
//! 3. **Channel order** (the 5.1 lane): our output — permuted from
//!    RFC 7845 Vorbis order into the house WAV order at the decoder's
//!    output seam — compares against ffmpeg's NATIVE layout
//!    directly, with no reference-side permutation. A wrong
//!    permutation would blow the differential sky-high (the channels
//!    carry different signals), so the tolerance assertion IS the
//!    order assertion.

mod opus;

use std::path::Path;

use opus::{
    corpus_dir, decode_through_ogg, f32_max_delta, ffmpeg_available, ffmpeg_decode_f32le,
    output_samples, samples_beyond,
};

/// One s16 LSB in float (the tolerance-class bound).
const ONE_S16_LSB: f32 = 1.0 / 32_768.0;

/// One corpus lane's full assertion set.
fn assert_lane(lane: &str, file: &str, channels: u32, packets: usize, raw: u64, granule: i64, valid: u64) {
    let Some(root) = corpus_dir() else {
        eprintln!("SKIP {lane}: no corpus directory (LUMEN_CORPUS unset, /tmp default absent)");
        return;
    };
    let path = Path::new(&root).join("media").join(file);
    let data = match std::fs::read(&path) {
        Ok(data) => data,
        Err(err) => unreachable!("corpus lane {lane} must exist ({}): {err}", path.display()),
    };
    let run = decode_through_ogg(&data);

    // (1) Geometry — the recorded admission numbers.
    assert_eq!(run.format.channels, channels, "{lane}: channel count");
    assert_eq!(run.format.rate, 48_000, "{lane}: decode rate");
    assert_eq!(run.packets, packets, "{lane}: data packet count");
    assert_eq!(run.pre_skip, 312, "{lane}: pre-skip (the recorded 312)");
    assert_eq!(run.decoded_raw, raw, "{lane}: raw decode count (pre-skip included)");
    // The granule counts from the RAW stream head (pre-skip
    // included, RFC 7845 §4): valid = granule − pre-skip.
    assert_eq!(
        run.last_granule,
        Some(granule),
        "{lane}: last granule (pre-skip included, the RFC 7845 domain)"
    );
    assert_eq!(
        output_samples(&run),
        valid as usize,
        "{lane}: output samples = pre-skip-discarded, end-trimmed valid window"
    );
    // The first frame's pts is the valid-timeline zero (the pre-skip
    // consumed the head; end-trim owns the tail).
    let first_pts = run
        .sink
        .pts
        .first()
        .copied()
        .unwrap_or_else(|| unreachable!("{lane}: at least one emitted frame"));
    assert_eq!(first_pts, 0, "{lane}: first frame pts on the valid timeline");

    // (2) The live differential (when the oracle is present).
    if !ffmpeg_available() {
        eprintln!("SKIP {lane} live differential: no ffmpeg binary");
        return;
    }
    let reference = ffmpeg_decode_f32le(&path);
    let ours = opus::concatenated(&run.sink);
    let ref_samples = reference.len() / (channels as usize * 4);
    assert_eq!(
        ref_samples,
        valid as usize,
        "{lane}: ffmpeg's own sample count (the window exactness cross-check)"
    );
    let max_delta = f32_max_delta(&ours, &reference);
    let beyond = samples_beyond(&ours, &reference, ONE_S16_LSB);
    eprintln!(
        "{lane}: channels={channels} packets={packets} raw={raw} valid={valid} \
         pre_skip=312 end_trim={} max|Δ|={max_delta:.3e} samples>1LSB={beyond}",
        raw - granule as u64
    );
    assert_eq!(
        beyond, 0,
        "{lane}: s16-audible equivalence — zero samples beyond one s16 LSB"
    );
    // (3) The channel-order assertion rides the tolerance: a wrong
    // Vorbis→house permutation would fail `beyond` catastrophically
    // (the 5.1 channels are not interchangeable signals).
    assert!(
        max_delta < 1e-3,
        "{lane}: float divergence sanity bound (measured {max_delta:.3e})"
    );
}

/// Corpus 0047 — mono, VBR, 201 packets: the admission record's
/// first differential row (max |Δ| 1.378e-06, 0/192,000 beyond the
/// LSB).
#[test]
fn corpus_0047_mono() {
    assert_lane(
        "corpus.0047",
        "0047-audio-only.ogg",
        1,
        201,
        192_960,
        192_312,
        192_000,
    );
}

/// Corpus 0077 — 5.1 surround, mapping family 1, 201 packets: the
/// admission record's second row (max |Δ| 4.381e-06 after channel
/// order normalization, 0/1,152,000 beyond the LSB) — except OUR
/// decoder normalizes the order itself, so the reference is
/// ffmpeg's native layout compared DIRECTLY.
#[test]
fn corpus_0077_surround_51() {
    assert_lane(
        "corpus.0077",
        "0077-audio-only.ogg",
        6,
        201,
        192_960,
        192_312,
        192_000,
    );
}
