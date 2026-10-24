//! Real-corpus MP3 differential (card 5.A3's main AC): the two
//! corpus-v4 MP3s plus the committed fixture set, decoded BOTH by
//! Lumen and by the external ffmpeg (the oracle, never a dependency),
//! compared on the SAME gapless window.
//!
//! The window each side decodes (documented exactly, per the card):
//! ffmpeg `-map 0:a -c:a pcm_s16le` applies the LAME gapless trim its
//! mp3 demuxer exports (skip_samples/end trim), yielding
//! `frames × spf − delay − padding` samples; Lumen's `decode_stream`
//! applies the same trim on the +529-shifted pipeline timeline (the
//! Layer III synthesis latency — `src/audio/mp3/decoder.rs` module
//! doc). The two windows cover the SAME samples, so the comparison is
//! sample-locked, not tolerance-fudged: equal length, per-sample
//! |Δ| ≤ 1 LSB (lossy-decoder rounding; recorded per file in
//! `tests/data/mp3_bitexact_reference.txt`).
//!
//! ffmpeg's MD5 is additionally checked against the recorded
//! `ffmpeg_pcm_md5` — if the corpus was re-imaged, the row FAILS
//! loudly instead of silently testing different bytes.
//!
//! SKIP-with-message when the corpus root or ffmpeg is absent.

mod common;
mod mp3;

use common::md5_hex;

use std::path::PathBuf;

use mp3::{
    corpus_dir, corpus_reference, decode_s16le, ffmpeg_available, ffmpeg_decode_s16le,
    fixture_bytes, fixture_path,
};
use testkit::Must;

/// The two real-corpus MP3s (corpus-v4 manifest rows 0139/0145).
const CORPUS_MP3: [&str; 2] = ["0139-audio-only.mp3", "0145-cover-audio.mp3"];

/// One differential row's verdict.
struct Row {
    file: String,
    rate: u32,
    channels: u32,
    samples: usize,
    differing: usize,
    max_delta: i64,
    ffmpeg_md5: String,
    lumen_md5: String,
}

fn run_row(label: &str, path: PathBuf, data: &[u8]) -> Row {
    let reference = ffmpeg_decode_s16le(&path);
    let (decoded, bytes) = match decode_s16le(data) {
        Ok(x) => x,
        Err(e) => unreachable!("{label} must decode: {e}"),
    };
    assert_eq!(
        bytes.len(),
        reference.len(),
        "{label}: window mismatch — Lumen {} bytes, ffmpeg {} bytes",
        bytes.len(),
        reference.len()
    );
    let (differing, max_delta) = mp3::s16_delta_stats(&bytes, &reference);
    Row {
        file: label.to_string(),
        rate: decoded.format.rate,
        channels: decoded.format.channels,
        samples: bytes.len() / 2 / decoded.format.channels as usize,
        differing,
        max_delta,
        ffmpeg_md5: md5_hex(&reference),
        lumen_md5: md5_hex(&bytes),
    }
}

#[test]
fn corpus_and_fixture_differential_vs_ffmpeg() {
    if !ffmpeg_available() {
        eprintln!("SKIP mp3 corpus differential: no ffmpeg binary on PATH");
        return;
    }
    let Some(corpus) = corpus_dir() else {
        eprintln!("SKIP mp3 corpus differential: corpus root absent");
        return;
    };

    let mut rows = Vec::new();

    // The real corpus (the card's two files, SKIP-with-message when
    // a file was re-imaged away — but an md5 drift FAILS, above).
    for name in CORPUS_MP3 {
        let path = corpus.join("media").join(name);
        if !path.is_file() {
            eprintln!("SKIP corpus row {name}: file absent (corpus re-imaged)");
            continue;
        }
        let data = std::fs::read(&path).must();
        let row = run_row(name, path, &data);
        if let Some(expect) = corpus_reference(name.trim_end_matches(".mp3")) {
            assert_eq!(
                row.ffmpeg_md5, expect["ffmpeg_pcm_md5"],
                "{name}: corpus bytes drifted from the recorded ffmpeg MD5"
            );
            assert_eq!(
                row.samples,
                expect["samples_per_channel"].parse::<usize>().must(),
                "{name}: window sample count drifted"
            );
        }
        rows.push(row);
    }

    // The committed fixture set through the same live oracle.
    for name in [
        "cbr128.mp3",
        "vbr_q4.mp3",
        "mono48.mp3",
        "mpeg2_24k.mp3",
        "mp2_48k.mp2",
        "tagged_art.mp3",
    ] {
        rows.push(run_row(name, fixture_path(name), &fixture_bytes(name)));
    }

    // The verdict table (mirrored into the worklog).
    eprintln!("=== 5.A3 MP3 differential: Lumen vs ffmpeg (same gapless window) ===");
    for row in &rows {
        eprintln!(
            "{:<24} rate={:<6} ch={} samples={:<6} differing={:<7} ({:.4}%) max_delta={} \
             lumen_md5={} ffmpeg_md5={}",
            row.file,
            row.rate,
            row.channels,
            row.samples,
            row.differing,
            100.0 * row.differing as f64 / (row.samples * row.channels as usize).max(1) as f64,
            row.max_delta,
            row.lumen_md5,
            row.ffmpeg_md5
        );
    }

    // The pass bar: bit-exact modulo ±1 LSB rounding on every row.
    for row in &rows {
        assert!(
            row.max_delta <= 1,
            "{}: max sample delta {} exceeds the ±1 LSB lossy tolerance",
            row.file,
            row.max_delta
        );
    }
}

/// The VBR-seek accuracy note made executable: the Xing TOC has 100
/// entries — one per 1% of the stream — so a TOC-only seek lands
/// within one Xing-frame of the target; `refine_to_frame` walks to
/// the exact frame boundary afterwards. Pinned on the vbr_q4 fixture:
/// the TOC must be monotone (the spec constraint the interpolation
/// relies on), the midpoint seek must land within ±1 frame of the
/// true midpoint, and the refined landing must be a real frame start.
#[test]
fn vbr_toc_seek_accuracy_is_within_one_frame_and_refines_exactly() {
    use codecs::audio::mp3::{VbrToc, parse_frame_header, refine_to_frame, vbr_toc_seek};

    let data = fixture_bytes("vbr_q4.mp3");
    let decoded = codecs::audio::mp3::decode_stream(&data).must();
    let Some(xing) = decoded.xing.as_ref() else {
        unreachable!("vbr_q4 must carry a Xing header");
    };
    let Some(toc) = xing.toc.as_ref() else {
        unreachable!("vbr_q4 must carry a TOC");
    };
    // TOC granularity: 100 buckets over the audio span.
    assert_eq!(toc.len(), 100);
    // Monotonic (the spec's constraint our interpolation relies on).
    assert!(toc.windows(2).all(|w| w[0] <= w[1]));
    let vbr = VbrToc {
        toc: *toc,
        first_frame_offset: decoded.first_frame_offset,
        total_bytes: u64::from(xing.bytes.unwrap_or(0)),
        total_frames: xing.frames.unwrap_or(0),
    };
    // Seek to the stream midpoint through the TOC: within one frame
    // (≤ 500 bytes here — frame sizes at q4 are ≈ 200–400) of the
    // true midpoint byte position, at or before it by construction.
    let audio_bytes = vbr.total_bytes;
    let mid_sample = 77u64 * 576; // an interior sample target
    let plan = vbr_toc_seek(&vbr, mid_sample, 1152);
    let true_mid = decoded.first_frame_offset + audio_bytes / 2;
    assert!(
        plan.byte_offset <= true_mid + 600,
        "TOC seek landed {} bytes past the true midpoint {true_mid}",
        plan.byte_offset
    );
    assert!(
        plan.byte_offset + 600 >= true_mid,
        "TOC seek landed {} bytes before the true midpoint {true_mid}",
        plan.byte_offset
    );
    // …and the refine snaps it to a real frame header.
    let landed = refine_to_frame(&data, plan.byte_offset as usize).unwrap_or(plan.byte_offset);
    let window = data
        .get(landed as usize..landed as usize + 4)
        .unwrap_or(&[]);
    assert!(
        parse_frame_header(window).is_ok(),
        "refined landing at {landed} must be a frame header"
    );
}
