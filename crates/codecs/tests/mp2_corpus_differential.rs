//! Real-corpus MP2 differential (card 5.J3's main AC): the two
//! corpus-v4 MP2s plus the committed fixture set, decoded BOTH by
//! Lumen and by the external ffmpeg (the oracle, never a dependency),
//! compared on the identical `frames × 1152` window.
//!
//! Layer II has no gapless construction (no Xing/LAME/VBRI — those
//! are Layer III encoder side-channels), so the window is the whole
//! decode on both sides: ffmpeg emits exactly `frames × 1152`
//! per-channel samples of an MP2 and so does
//! `codecs::audio::mp2::decode_stream`. The comparison is therefore
//! sample-locked with NO window arithmetic: equal length, per-sample
//! |Δ| tolerance, whole-decode PCM MD5s recorded per file (the
//! `ffmpeg -map 0:a -f md5` form of the card's mandate).
//!
//! The corpus rows' ffmpeg PCM MD5s are additionally checked against
//! the recorded expectations — if the corpus was re-imaged, the row
//! FAILS loudly instead of silently testing different bytes.
//!
//! SKIP-with-message when the corpus root or ffmpeg is absent.

mod common;
mod mp2;

use common::md5_hex;
use std::path::PathBuf;

use mp2::{
    corpus_dir, decode_s16le, ffmpeg_available, ffmpeg_decode_s16le, ffmpeg_pcm_md5, fixture_bytes,
    fixture_path,
};
use testkit::Must;

/// The two real-corpus MP2s (corpus-v4 rows 0005/0119) with their
/// recorded ffmpeg PCM MD5s (measured 2026-10-24, ffmpeg 7.1.5).
const CORPUS_MP2: [(&str, &str); 2] = [
    ("0005-audio-only.mp2", "fe0d209790e35a2a21155c26ab028bb4"),
    ("0119-audio-only.mp2", "d5c54678b63272cee5715789ca93870c"),
];

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
    let ffmpeg_md5 = ffmpeg_pcm_md5(&path);
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
    let (differing, max_delta) = mp2::s16_delta_stats(&bytes, &reference);
    Row {
        file: label.to_string(),
        rate: decoded.format.rate,
        channels: decoded.format.channels,
        samples: bytes.len() / 2 / decoded.format.channels as usize,
        differing,
        max_delta,
        ffmpeg_md5,
        lumen_md5: md5_hex(&bytes),
    }
}

#[test]
fn corpus_and_fixture_differential_vs_ffmpeg() {
    if !ffmpeg_available() {
        eprintln!("SKIP mp2 corpus differential: no ffmpeg binary on PATH");
        return;
    }

    let mut rows = Vec::new();

    // The real corpus (the card's two files; SKIP-with-message when a
    // file was re-imaged away — but an MD5 drift FAILS, below).
    let corpus = corpus_dir();
    for (name, expect_md5) in CORPUS_MP2 {
        let Some(root) = corpus.as_ref() else {
            eprintln!("SKIP corpus row {name}: corpus root absent");
            continue;
        };
        let path = root.join("media").join(name);
        if !path.is_file() {
            eprintln!("SKIP corpus row {name}: file absent (corpus re-imaged)");
            continue;
        }
        let data = std::fs::read(&path).must();
        let row = run_row(name, path, &data);
        assert_eq!(
            row.ffmpeg_md5, expect_md5,
            "{name}: corpus bytes drifted from the recorded ffmpeg MD5"
        );
        rows.push(row);
    }

    // The committed fixture set through the same live oracle: 5.A3's
    // MPEG-1 Layer II 192 kbps 48 kHz stereo fixture plus this card's
    // generated MPEG-2 LSF Layer II 96 kbps 22.05 kHz stereo fixture
    // (ffmpeg `-c:a mp2`, sine source, committed).
    for name in ["mp2_48k.mp2", "mp2_lsf_22k.mp2"] {
        rows.push(run_row(name, fixture_path(name), &fixture_bytes(name)));
    }

    // The verdict table (mirrored into the worklog).
    eprintln!("=== 5.J3 MP2 differential: Lumen vs ffmpeg (identical frames×1152 window) ===");
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

    // The pass bar: bit-exact modulo ±1 LSB rounding on every row
    // (the family's lossy tolerance; the table records the truth).
    for row in &rows {
        assert!(
            row.max_delta <= 1,
            "{}: max sample delta {} exceeds the ±1 LSB lossy tolerance",
            row.file,
            row.max_delta
        );
    }
}

/// The CBR seek arithmetic made executable on the real padded corpus
/// stream: 0119 is 384 kbps 44.1 kHz (fractional nominal frame size,
/// padding slots sprinkled by the encoder), so a nominal landing
/// point can sit slots early — `refine_to_frame` must snap it forward
/// onto a real frame header, and the snapped frame must be the target
/// frame (never past it by more than the padding slack).
#[test]
fn cbr_seek_on_the_padded_corpus_stream_refines_exactly() {
    use codecs::audio::mp2::{cbr_seek, parse_mp2_header, refine_to_frame, walk_mp2_frames};

    let Some(root) = corpus_dir() else {
        eprintln!("SKIP mp2 seek pin: corpus root absent");
        return;
    };
    let path = root.join("media").join("0119-audio-only.mp2");
    if !path.is_file() {
        eprintln!("SKIP mp2 seek pin: 0119 absent (corpus re-imaged)");
        return;
    }
    let data = std::fs::read(&path).must();
    let frames = walk_mp2_frames(&data, 0).must();
    let decoded = codecs::audio::mp2::decode_stream(&data).must();
    let geo = decoded.geometry;
    // The geometry matches the walk.
    assert_eq!(geo.total_frames, frames.len() as u64);
    assert_eq!(geo.sample_rate, 44_100);
    assert_eq!(geo.channels, 2);
    // Seek to the last quarter's first sample: nominal arithmetic
    // lands within the padding slack (a few slots) of the real frame.
    let target_frame = geo.total_frames / 4;
    let plan = cbr_seek(&geo, target_frame * 1152);
    let real = frames[target_frame as usize].offset as u64;
    assert!(
        plan.byte_offset <= real,
        "nominal landing {} ran PAST the real frame start {}",
        plan.byte_offset,
        real
    );
    assert!(
        real - plan.byte_offset < 64,
        "padding drift {} bytes exceeds the documented slack",
        real - plan.byte_offset
    );
    let snapped = refine_to_frame(&data, plan.byte_offset as usize).must();
    assert_eq!(snapped, real);
    let window = data
        .get(snapped as usize..snapped as usize + 4)
        .unwrap_or(&[]);
    assert!(parse_mp2_header(window).is_ok());
}

/// The Layer II unit probe on a generated file, pinned in-repo: the
/// committed LSF fixture must walk, decode and expose the geometry
/// the ffmpeg encode defined (96 kbps 22.05 kHz stereo MPEG-2 LSF).
#[test]
fn the_generated_lsf_fixture_walks_and_decodes_with_declared_geometry() {
    let data = fixture_bytes("mp2_lsf_22k.mp2");
    let decoded = match codecs::audio::mp2::decode_stream(&data) {
        Ok(d) => d,
        Err(e) => unreachable!("the committed fixture must decode: {e}"),
    };
    assert_eq!((decoded.format.rate, decoded.format.channels), (22_050, 2));
    assert_eq!(decoded.geometry.bitrate_bps, 96_000);
    assert_eq!(
        decoded.geometry.version,
        codecs::audio::mp3::MpegVersion::V2
    );
    // 1152 samples per frame, every frame decoded.
    assert_eq!(decoded.samples.len(), decoded.frames * 1152 * 2);
    assert_eq!(
        decoded.geometry.total_samples,
        (decoded.frames as u64) * 1152
    );
}
