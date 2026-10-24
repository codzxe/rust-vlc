#![allow(dead_code)]
//! Shared AAC test surface (card 5.C2a): the crate-local fixture
//! loader, the ffmpeg-matching s16 conversion, and the stream decode
//! driver used by the `aac_*.rs` suites (the 5.A3 `tests/mp3/`
//! pattern).
//!
//! Everything here is **test-side verification tooling** in the 2.B4
//! sense: its own correctness is pinned by the recorded expectations
//! it is checked against (ffprobe stream facts, ffmpeg PCM digests),
//! never trusted blind.

use std::path::PathBuf;

use codecs::audio::aac::{DecodedAac, decode_stream};

/// The crate-local fixture directory (committed, budget-checked: the
/// card's < 200 KB mandate).
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
/// 32768, round-ties-to-even, then clip) — the 5.A3 convention.
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

/// Decode a whole ADTS stream and convert to ffmpeg-layout s16le
/// bytes (the differential's unit of comparison).
pub fn decode_s16le(data: &[u8]) -> Result<(DecodedAac, Vec<u8>), media_types::EngineError> {
    let decoded = decode_stream(data)?;
    let bytes = to_s16le(&decoded.samples);
    Ok((decoded, bytes))
}

/// Compare two s16le byte streams sample-by-sample: returns
/// (differing_samples, max_abs_delta) — the honest tolerance report
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

/// ffmpeg's own decode of `path` as interleaved s16le PCM. Uses
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

/// Extract an MP4/M4A container's first audio track to ADTS with a
/// **stream copy** (`-c:a copy`: no re-encode — the AAC essence bytes
/// are the container's own, so the Lumen-vs-ffmpeg comparison stays
/// apples-to-apples). The native MP4 demuxer path arrives when 5.C1
/// merges; until then the extraction is the documented bridge.
pub fn ffmpeg_extract_adts(path: &std::path::Path) -> Vec<u8> {
    let out = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-map", "0:a:0", "-c:a", "copy", "-f", "adts", "-"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap_or_else(|e| unreachable!("ffmpeg spawn: {e}"));
    if !out.status.success() {
        unreachable!(
            "ffmpeg adts extraction failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    out.stdout
}

/// ffmpeg's decode of raw ADTS **bytes** (the extracted-essence side
/// of the m4a bridge): `-f aac` forces the raw-ADTS demuxer on stdin
/// (ffmpeg's format probe refuses short extracted streams — measured;
/// the forced input is the documented differential shape). The
/// container-level trim (edit list / media duration) is the MP4
/// demuxer's (5.C1) surface, deliberately OUT of this comparison.
pub fn ffmpeg_decode_s16le_adts_bytes(bytes: &[u8]) -> Vec<u8> {
    use std::io::Write as _;
    let mut child = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-f", "aac", "-i", "-"])
        .args(["-map", "0:a", "-c:a", "pcm_s16le", "-f", "s16le", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| unreachable!("ffmpeg spawn: {e}"));
    child
        .stdin
        .as_mut()
        .unwrap_or_else(|| unreachable!("stdin must be piped"))
        .write_all(bytes)
        .unwrap_or_else(|e| unreachable!("ffmpeg stdin: {e}"));
    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| unreachable!("ffmpeg wait: {e}"));
    if !out.status.success() {
        unreachable!(
            "ffmpeg failed on the extracted ADTS bytes: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    out.stdout
}

/// Read a corpus media file (SKIP-aware: `None` when absent).
pub fn corpus_media(corpus: &std::path::Path, name: &str) -> Option<Vec<u8>> {
    let path = corpus.join("media").join(name);
    std::fs::read(path).ok()
}
