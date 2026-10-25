#![allow(dead_code)]
//! Shared MP2 test surface (card 5.J3): the crate-local fixture
//! loader, the corpus root, the ffmpeg-matching s16 conversion, and
//! the stream decode driver used by the `mp2_*.rs` suites (the 5.A3
//! `tests/mp3/` pattern).
//!
//! Everything here is **test-side verification tooling** in the 2.B4
//! sense: its own correctness is pinned by the recorded expectations
//! it is checked against (ffprobe stream facts, ffmpeg PCM digests),
//! never trusted blind.

use std::path::PathBuf;

use codecs::audio::mp2::{DecodedMp2, decode_stream};

/// The crate-local fixture directory (committed; the card's < 200 KB
/// fuzz-lite budget — this card adds one 19 KB Layer II LSF fixture
/// beside 5.A3's 36 KB `mp2_48k.mp2`).
pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Read a committed fixture (panics: fixtures are committed bytes —
/// absence is a repo-state error, not a test verdict).
pub fn fixture_bytes(name: &str) -> Vec<u8> {
    match std::fs::read(fixture_path(name)) {
        Ok(bytes) => bytes,
        Err(err) => unreachable!("committed fixture {name}: {err}"),
    }
}

/// The corpus root: the `LUMEN_CORPUS` env override when set, else the
/// sandbox's known default. `None` (→ the caller SKIPs with a message)
/// when neither exists — the corpus is /tmp-resident and may be
/// re-imaged away.
pub fn corpus_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
        let path = PathBuf::from(dir);
        return path.is_dir().then_some(path);
    }
    let default = PathBuf::from("/tmp/my-project/corpus-v4");
    default.is_dir().then_some(default)
}

/// Convert one f32 sample to s16 with ffmpeg's own float→s16 rule
/// (libswresample's `av_clip_int16(lrintf(x * (1<<15)))`: scale by
/// 32768, round-ties-to-even — `round_ties_even`, the lrint default
/// rounding mode — then clip). The EXACT mapping the differential
/// compares with; any deviation shows up as an MD5 mismatch, never
/// silently. (The 5.A3 helper, verbatim — the whole family shares the
/// conversion.)
#[must_use]
pub fn f32_to_s16(x: f32) -> i16 {
    let scaled = x * 32_768.0;
    let rounded = scaled.round_ties_even();
    if rounded >= 32_767.0 {
        32_767
    } else if rounded <= -32_768.0 {
        -32_768
    } else {
        rounded as i16
    }
}

/// Interleaved f32 samples → ffmpeg-layout s16le bytes.
#[must_use]
pub fn to_s16le(samples: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 2);
    for v in samples {
        out.extend_from_slice(&f32_to_s16(*v).to_le_bytes());
    }
    out
}

/// Decode a whole stream and convert to ffmpeg-layout s16le bytes
/// (the differential's unit of comparison).
pub fn decode_s16le(data: &[u8]) -> Result<(DecodedMp2, Vec<u8>), media_types::EngineError> {
    let decoded = decode_stream(data)?;
    let bytes = to_s16le(&decoded.samples);
    Ok((decoded, bytes))
}

/// Compare two s16le byte streams sample-by-sample: returns
/// (differing_samples, max_abs_delta). The honest tolerance report
/// behind every differential row.
#[must_use]
pub fn s16_delta_stats(a: &[u8], b: &[u8]) -> (usize, i64) {
    let mut differing = 0usize;
    let mut max_delta: i64 = 0;
    let n = a.len().min(b.len()) / 2;
    for i in 0..n {
        let sa = i16::from_le_bytes([a[2 * i], a[2 * i + 1]]);
        let sb = i16::from_le_bytes([b[2 * i], b[2 * i + 1]]);
        let d = i64::from(sa) - i64::from(sb);
        if d != 0 {
            differing += 1;
            max_delta = max_delta.max(d.abs());
        }
    }
    (differing, max_delta)
}

/// Whether an `ffmpeg` binary is reachable (the live differential's
/// oracle; absence → the caller SKIPs with a message).
pub fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// ffmpeg's own decode of `path` as interleaved s16le PCM. Panics
/// only on oracle failure (a test-env problem, not a verdict). Uses
/// `-map 0:a` per the card's differential mandate.
pub fn ffmpeg_decode_s16le(path: &std::path::Path) -> Vec<u8> {
    let out = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-map", "0:a", "-c:a", "pcm_s16le", "-f", "s16le", "-"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap_or_else(|e| unreachable!("ffmpeg spawn: {e}"));
    if !out.status.success() {
        unreachable!(
            "ffmpeg failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    out.stdout
}

/// The card's exact oracle form: ffmpeg's PCM MD5 of `path` via
/// `-map 0:a -f md5` (with `-c:a pcm_s16le` naming the PCM the hash
/// runs over — a bare `-f md5` would hash whatever default encoder
/// the muxer picks, which is not a PCM digest). Panics only on oracle
/// failure.
pub fn ffmpeg_pcm_md5(path: &std::path::Path) -> String {
    let out = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-map", "0:a", "-c:a", "pcm_s16le", "-f", "md5", "-"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap_or_else(|e| unreachable!("ffmpeg spawn: {e}"));
    if !out.status.success() {
        unreachable!(
            "ffmpeg md5 failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    // "MD5=..." on stdout.
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    match text.strip_prefix("MD5=") {
        Some(hex) => hex.to_string(),
        None => unreachable!("unexpected ffmpeg md5 output: {text}"),
    }
}
