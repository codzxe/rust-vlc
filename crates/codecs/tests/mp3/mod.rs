#![allow(dead_code)]
//! Shared MP3/MP2 test surface (card 5.A3): the crate-local fixture
//! loader, the reference-data parser, the ffmpeg-matching s16
//! conversion, and the stream decode driver used by the `mp3_*.rs`
//! suites (the 5.A1 `tests/flac/` pattern).
//!
//! Everything here is **test-side verification tooling** in the 2.B4
//! sense: its own correctness is pinned by the recorded expectations
//! it is checked against (ffprobe stream facts, ffmpeg PCM digests),
//! never trusted blind.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use codecs::audio::mp3::{DecodedMp3, decode_stream};
use testkit::Must;

/// The crate-local fixture directory (committed, budget-checked: the
/// card's < 200 KB mandate; total recorded in the reference file).
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

/// The reference-data file path.
fn reference_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/mp3_bitexact_reference.txt")
}

/// One `[section]` of the reference data: `key = value` pairs.
type Section = HashMap<String, String>;

/// Parsed reference data (lazily, once per test binary).
pub fn reference() -> &'static HashMap<String, Section> {
    static REF: OnceLock<HashMap<String, Section>> = OnceLock::new();
    REF.get_or_init(
        || match parse_reference(&std::fs::read(reference_path()).must()) {
            Ok(map) => map,
            Err(err) => unreachable!("reference data must parse: {err}"),
        },
    )
}

/// The `fixture.<name>` section (panics: recorded expectations are
/// part of the committed fixture set).
pub fn fixture_reference(name: &str) -> &'static Section {
    match reference().get(&format!("fixture.{name}")) {
        Some(section) => section,
        None => unreachable!("no [fixture.{name}] section in the reference data"),
    }
}

/// The `corpus.<name>` section, when recorded.
pub fn corpus_reference(name: &str) -> Option<&'static Section> {
    reference().get(&format!("corpus.{name}"))
}

/// Parse the ini-like reference format (`[section]` + `key = value`,
/// `#` comments).
fn parse_reference(data: &[u8]) -> Result<HashMap<String, Section>, String> {
    let text = std::str::from_utf8(data).map_err(|e| e.to_string())?;
    let mut map: HashMap<String, Section> = HashMap::new();
    let mut current = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[') {
            let name = name.strip_suffix(']').ok_or("unterminated [section]")?;
            current = name.to_string();
            map.entry(current.clone()).or_default();
            continue;
        }
        let (key, value) = line.split_once('=').ok_or("expected key = value")?;
        let section = map
            .get_mut(&current)
            .ok_or_else(|| format!("key outside a section: {key}"))?;
        section.insert(key.trim().to_string(), value.trim().to_string());
    }
    Ok(map)
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
/// rounding mode — then clip). This is the EXACT mapping the
/// differential compares with; any deviation would show up as an MD5
/// mismatch, never silently.
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

/// Interleaved f32 samples → ffmpeg-layout s16le bytes (little-endian
/// on every target the differential runs on; the byte order is pinned
/// by the recorded MD5s).
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
pub fn decode_s16le(data: &[u8]) -> Result<(DecodedMp3, Vec<u8>), media_types::EngineError> {
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

/// ffmpeg's own decode of `path` as interleaved s16le PCM (the exact
/// `av_clip_int16(lrintf(x·2¹⁵))` float→s16 rule our `f32_to_s16`
/// mirrors). Panics only on oracle failure (a test-env problem, not a
/// verdict). Uses `-map 0:a` per the card's differential mandate.
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
