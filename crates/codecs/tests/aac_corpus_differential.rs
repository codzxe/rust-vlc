//! Real-corpus AAC differential (card 5.C2a's main AC): the two
//! corpus-v4 ADTS files plus the m4a audio tracks and the committed
//! fixtures, decoded BOTH by Lumen and by the external ffmpeg (the
//! oracle, never a dependency), compared on the SAME frame window.
//!
//! The window each side decodes: all complete ADTS frames (trailing
//! partial frames dropped on both sides — measured: ffmpeg decodes
//! exactly `frames × 1024` samples of each stream). The comparison is
//! sample-locked: equal byte length, per-sample |Δ| ≤ 1 LSB
//! (lossy-decoder rounding; recorded per file in
//! `tests/data/aac_bitexact_reference.txt`).
//!
//! The m4a rows cross a documented bridge: Lumen has no MP4 demuxer
//! on main yet (5.C1 is branch-pending), so the AAC essence is
//! extracted with a ffmpeg **stream copy** (`-c:a copy`, no
//! re-encode) and both decoders decode the SAME extracted bytes
//! (`-f aac` forced input — ffmpeg's own probe refuses short
//! extracted streams, a measured quirk recorded here). The
//! container-level trim (0029's edit list: 0.085 s media duration)
//! is the MP4 demuxer's surface, deliberately OUT of this codec
//! comparison.
//!
//! 0094-audio-only.m4a is a 6-channel AAC-LC stream: the engine
//! implements mono/stereo only (honest partial, module docs) — the
//! row asserts the TYPED refusal instead of comparing PCM.
//!
//! SKIP-with-message when the corpus root or ffmpeg is absent.

mod aac;
mod common;

use std::path::PathBuf;

use aac::{
    corpus_dir, corpus_media, decode_s16le, ffmpeg_available, ffmpeg_decode_s16le,
    ffmpeg_decode_s16le_adts_bytes, ffmpeg_extract_adts, fixture_path, s16_delta_stats,
};
use common::md5_hex;

/// One differential row's verdict.
struct Row {
    file: String,
    rate: u32,
    channels: u32,
    samples: usize,
    differing: usize,
    max_delta: i64,
    lumen_md5: String,
    ffmpeg_md5: String,
}

fn run_row(label: &str, path: PathBuf, data: &[u8]) -> Row {
    let reference = ffmpeg_decode_s16le(&path);
    run_row_bytes(label, data, &reference)
}

fn run_row_bytes(label: &str, data: &[u8], reference: &[u8]) -> Row {
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
    let (differing, max_delta) = s16_delta_stats(&bytes, reference);
    Row {
        file: label.to_string(),
        rate: decoded.format.rate,
        channels: decoded.format.channels,
        samples: bytes.len() / 2 / decoded.format.channels as usize,
        differing,
        max_delta,
        lumen_md5: md5_hex(&bytes),
        ffmpeg_md5: md5_hex(reference),
    }
}

/// The recorded reference values the corpus rows are pinned against
/// (drift FAILS loudly instead of silently testing other bytes).
fn expect(row: &Row, key: &str) -> String {
    match row.file.as_str() {
        "0059-audio-only.aac" => [
            ("rate", "88200"),
            ("channels", "1"),
            ("samples", "354304"),
            ("lumen_pcm_md5", "c18f85b069732afa99fd082049495048"),
            ("ffmpeg_pcm_md5", "6329d50395e2abe9ce9f222a376dd6a6"),
        ],
        "0110-audio-only.aac" => [
            ("rate", "96000"),
            ("channels", "1"),
            ("samples", "385024"),
            ("lumen_pcm_md5", "c1bb4de65fd61536a5960d2c031d2f39"),
            ("ffmpeg_pcm_md5", "1dfcbf725dc4ab6c5c5210079788d49b"),
        ],
        "0029-cover-audio.m4a" => [
            ("rate", "12000"),
            ("channels", "2"),
            ("samples", "2048"),
            ("lumen_pcm_md5", "6becaacf21207259393989b5927589ae"),
            ("ffmpeg_pcm_md5", "294366b2c371795b67da0b5368a8858f"),
        ],
        _ => return String::new(),
    }
    .iter()
    .find(|(k, _)| *k == key)
    .map(|(_, v)| (*v).to_string())
    .unwrap_or_default()
}

#[test]
fn corpus_differential_vs_ffmpeg() {
    if !ffmpeg_available() {
        eprintln!("SKIP aac corpus differential: no ffmpeg binary on PATH");
        return;
    }
    let Some(corpus) = corpus_dir() else {
        eprintln!("SKIP aac corpus differential: corpus root absent");
        return;
    };

    let mut rows = Vec::new();

    // The real corpus ADTS lanes (the card's two files).
    for name in ["0059-audio-only.aac", "0110-audio-only.aac"] {
        let Some(data) = corpus_media(&corpus, name) else {
            eprintln!("SKIP corpus row {name}: file absent (corpus re-imaged)");
            continue;
        };
        let path = corpus.join("media").join(name);
        let row = run_row(name, path, &data);
        assert_eq!(
            row.ffmpeg_md5,
            expect(&row, "ffmpeg_pcm_md5"),
            "{name}: corpus bytes drifted"
        );
        assert_eq!(
            row.lumen_md5,
            expect(&row, "lumen_pcm_md5"),
            "{name}: Lumen MD5 drifted"
        );
        assert_eq!(
            row.samples,
            expect(&row, "samples").parse::<usize>().unwrap_or(0)
        );
        rows.push(row);
    }

    // The m4a lanes through the stream-copy bridge. 0029 (cover-art
    // m4a, stereo LC) compares PCM; 0094 (6-channel LC) asserts the
    // typed partial.
    let m4a = corpus.join("media").join("0029-cover-audio.m4a");
    if m4a.is_file() {
        let extracted = ffmpeg_extract_adts(&m4a);
        let reference = ffmpeg_decode_s16le_adts_bytes(&extracted);
        let row = run_row_bytes("0029-cover-audio.m4a", &extracted, &reference);
        assert_eq!(row.ffmpeg_md5, expect(&row, "ffmpeg_pcm_md5"));
        assert_eq!(row.lumen_md5, expect(&row, "lumen_pcm_md5"));
        rows.push(row);
    } else {
        eprintln!("SKIP m4a row 0029: file absent (corpus re-imaged)");
    }
    let m4a = corpus.join("media").join("0094-audio-only.m4a");
    if m4a.is_file() {
        let extracted = ffmpeg_extract_adts(&m4a);
        let err = match aac::decode_s16le(&extracted) {
            Ok(_) => unreachable!("6-channel AAC must be refused typedly"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "aac.channel_count");
        eprintln!(
            "0094-audio-only.m4a: TYPED partial — {err} (engine caps at stereo; ffmpeg decodes 6ch)"
        );
    } else {
        eprintln!("SKIP m4a row 0094: file absent (corpus re-imaged)");
    }

    // The committed fixture set through the same live oracle.
    for name in ["sine441_lc_stereo.aac", "sine48_lc_mono.aac"] {
        rows.push(run_row(name, fixture_path(name), &aac::fixture_bytes(name)));
    }

    // The verdict table (mirrored into the worklog).
    eprintln!("=== 5.C2a AAC-LC differential: Lumen vs ffmpeg (same frame window) ===");
    for row in &rows {
        eprintln!(
            "{:<28} rate={:<6} ch={} samples={:<7} differing={:<6} ({:.4}%) max_delta={} \
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
