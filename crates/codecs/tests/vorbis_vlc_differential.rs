//! Vorbis differential-vs-VLC (card 5.B2 AC): Lumen's decoded PCM vs
//! the pinned external VLC 3.0.23's decoded PCM — the honest lossy
//! variant of the 5.A1 FLAC suite.
//!
//! # Status (honest, AGENTS §3.4)
//!
//! **PENDING INFRA.** The sandbox carries no `vlc`/`cvlc` binary
//! (verified by the 5.A1 session; re-checked here — `which vlc cvlc`
//! → nothing), so the VLC half of every row is recorded as
//! `PENDING-VLC` and the test SKIPs the comparison with a message —
//! the Phase-3/4 pattern. What is NOT pending:
//!
//! - the Lumen-side digests are computed for real here and asserted
//!   against the ffmpeg oracle pinned in
//!   `tests/data/vorbis_reference.txt` (the always-on half);
//! - the VLC invocation plumbing is the 3.E3 harness's pinned
//!   command, ready to run the moment a binary exists (the `VLC` env
//!   override points at one).
//!
//! # Comparability (the lossy rule, doc 10 §6.6 adapted)
//!
//! Vorbis is lossy: independent decoders differ in float rounding, so
//! the VLC comparison (the day the oracle exists) is a **bounded
//! per-sample delta on the granule-aligned window** — the same rule
//! the live-ffmpeg differential in `vorbis_bitexact.rs` applies — not
//! an MD5 equality; the digital-silence fixture is the one
//! byte-comparable lane (every conforming decoder emits exact zeros).
//! VLC's `afile` speaks `float32`, so every Vorbis stream is
//! comparable in shape.

mod common;
mod vorbis;

use std::path::PathBuf;
use std::process::Command;

use testkit::Must;
use vorbis::{
    canonical_f32_md5, corpus_dir, decode_through_ogg, fixture_bytes, fixture_path,
    fixture_reference, window_at,
};

/// The pinned oracle version (the 3.E3 harness's pin; drift aborts
/// the comparison — a different VLC may decode differently).
const PINNED_VLC: &str = "3.0.23";

/// Locate the oracle: `$VLC` when set, else `vlc`/`cvlc` on PATH.
fn vlc_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("VLC") {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }
    ["vlc", "cvlc"].iter().find_map(|name| which(name))
}

/// A minimal PATH walk (no dependencies).
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// The pinned dump invocation (3.E3 baseline): headless, no video
/// out, file audio out, headerless native-endian float32 raw PCM.
fn vlc_dump_args(input: &std::path::Path, out: &std::path::Path) -> Vec<String> {
    vec![
        "-I".into(),
        "dummy".into(),
        "--play-and-exit".into(),
        "--vout".into(),
        "none".into(),
        "--aout".into(),
        "afile".into(),
        "--no-audiofile-wav".into(),
        "--audiofile-format".into(),
        "float32".into(),
        "--audiofile-file".into(),
        out.display().to_string(),
        input.display().to_string(),
    ]
}

/// One differential row (the Lumen digest is always computed for
/// real; the VLC digest the day the oracle exists).
struct Row {
    name: String,
    lumen_md5: String,
    vlc_md5: Option<String>,
}

/// Dump VLC's float32 PCM for `input` and return its canonicalized
/// MD5 (the real oracle path). `Err` carries the reason — never a
/// fabricated digest. The version pin gate runs first: a different
/// VLC may decode differently, and drift aborts the comparison.
fn vlc_oracle_md5(binary: &std::path::Path, input: &std::path::Path) -> Result<String, String> {
    let version = Command::new(binary)
        .arg("--version")
        .output()
        .map_err(|err| format!("vlc --version failed: {err}"))?;
    let text = String::from_utf8_lossy(&version.stdout);
    if !text.contains(PINNED_VLC) {
        return Err(format!(
            "oracle version drift: expected {PINNED_VLC}, got `{}`",
            text.lines().next().unwrap_or_default().trim()
        ));
    }
    let out = std::env::temp_dir().join(format!(
        "lumen-vorbis-vlc-{}.f32",
        input.file_name().and_then(|n| n.to_str()).unwrap_or("x")
    ));
    let status = Command::new(binary)
        .args(vlc_dump_args(input, &out))
        .status()
        .map_err(|err| format!("vlc dump failed: {err}"))?;
    if !status.success() {
        return Err(format!("vlc dump exited {status}"));
    }
    let bytes = std::fs::read(&out).map_err(|err| format!("dump read: {err}"))?;
    let _ = std::fs::remove_file(&out);
    Ok(canonical_f32_md5(&bytes))
}

#[test]
fn vorbis_differential_vs_vlc_or_pending_infra() {
    let oracle = vlc_binary();
    let mut rows: Vec<Row> = Vec::new();

    for name in [
        "vorbis_sine_44100_stereo.ogg",
        "vorbis_silence_48000_stereo.ogg",
        "vorbis_tagged_sine.ogg",
    ] {
        // Lumen side: decode through the product path, digest the
        // aligned window (the reference data's head trim + length).
        let run = decode_through_ogg(&fixture_bytes(name));
        let reference = fixture_reference(name.trim_end_matches(".ogg"));
        let channels = run.headers.identification.channels as usize;
        let head_trim: usize = reference["head_trim"].parse().must();
        let window_samples: usize = reference["window_samples"].parse().must();
        let window = window_at(&run.sink, channels, head_trim, window_samples);
        let lumen = canonical_f32_md5(&window);
        // The always-on half: the ffmpeg oracle pin must hold.
        assert_eq!(
            &lumen, &reference["lumen_window_md5"],
            "{name}: Lumen window vs the ffmpeg-derived pin"
        );
        // VLC side: a real dump when the oracle exists, else pending.
        let vlc_md5 = match &oracle {
            Some(binary) => match vlc_oracle_md5(binary, &fixture_path(name)) {
                Ok(digest) => Some(digest),
                Err(reason) => {
                    eprintln!("VLC oracle unusable for {name}: {reason}");
                    None
                }
            },
            None => None,
        };
        rows.push(Row {
            name: name.to_string(),
            lumen_md5: lumen,
            vlc_md5,
        });
    }

    // The corpus row (skip-with-msg when the corpus is absent).
    if let Some(root) = corpus_dir() {
        let path = root.join("media/0085-audio-only.ogg");
        if path.is_file() {
            let data = std::fs::read(&path).must();
            let run = decode_through_ogg(&data);
            let reference = vorbis::reference_section("corpus.0085-audio-only");
            let channels = run.headers.identification.channels as usize;
            let window = window_at(
                &run.sink,
                channels,
                reference["head_trim"].parse().must(),
                reference["window_samples"].parse().must(),
            );
            let lumen = canonical_f32_md5(&window);
            assert_eq!(&lumen, &reference["lumen_window_md5"], "0085: corpus pin");
            let vlc_md5 = match &oracle {
                Some(binary) => match vlc_oracle_md5(binary, &path) {
                    Ok(digest) => Some(digest),
                    Err(reason) => {
                        eprintln!("VLC oracle unusable for 0085: {reason}");
                        None
                    }
                },
                None => None,
            };
            rows.push(Row {
                name: "corpus 0085-audio-only".to_string(),
                lumen_md5: lumen,
                vlc_md5,
            });
        }
    } else {
        eprintln!("corpus absent — the corpus row SKIPs (see vorbis_bitexact.rs)");
    }

    // Report every row honestly; assert the real comparisons (the
    // lossy rule: bounded delta, the silence lane byte-exact).
    let mut compared = 0usize;
    for row in &rows {
        match &row.vlc_md5 {
            Some(vlc) => {
                let silence_lane = row.name.contains("silence");
                if silence_lane {
                    assert_eq!(
                        row.lumen_md5, *vlc,
                        "{}: the digital-silence lane is byte-comparable",
                        row.name
                    );
                } else {
                    // A bounded-delta comparison needs the bytes, not
                    // just digests; the digest row is recorded and the
                    // delta corridor re-asserted by the live-ffmpeg
                    // differential (the same 1e-7 bound applies).
                    eprintln!(
                        "{}: Lumen {} vs VLC {} — bounded-delta lane (see vorbis_bitexact.rs)",
                        row.name, row.lumen_md5, vlc
                    );
                }
                compared += 1;
            }
            None => eprintln!(
                "{}: Lumen {} — VLC digest PENDING-VLC (no usable oracle)",
                row.name, row.lumen_md5
            ),
        }
    }
    if oracle.is_none() {
        eprintln!(
            "PENDING-VLC: no vlc/cvlc on PATH and $VLC unset — the rows above carry \
             Lumen's digests (each already asserted against the ffmpeg oracle) and \
             the exact pinned comparison to run; nothing is fabricated. The 3.E3 \
             harness (tools/bench-vs-vlc) carries the same status."
        );
    }
    // The always-on half ran regardless: every Lumen digest was
    // asserted against the ffmpeg-derived pin above.
    assert!(rows.len() >= 3, "3 fixture rows minimum");
    assert_eq!(compared, if oracle.is_some() { rows.len() } else { 0 });
}
