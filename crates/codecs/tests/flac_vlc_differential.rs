//! FLAC differential-vs-VLC (card 5.A1 AC): Lumen's decoded PCM vs
//! the pinned external VLC 3.0.23's decoded PCM, fixture by fixture —
//! the lossless-comparable paths only (the 3.E3 `bench-vs-vlc` doc 10
//! §6.6 rules, applied to the FLAC set).
//!
//! # Status (honest, AGENTS §3.4)
//!
//! **PENDING INFRA.** The sandbox carries no `vlc`/`cvlc` binary
//! (verified this session: `which vlc cvlc` → nothing), so the VLC
//! half of every row is recorded as `PENDING-VLC` and the test SKIPs
//! the comparison with a message — the Phase-3/4 pattern (the 4h-soak
//! and 3-OS gates carry the same status). What is NOT pending:
//!
//! - the Lumen-side digests are computed for real here and asserted
//!   against the ffmpeg oracle recorded in
//!   `tests/data/flac_bitexact_reference.txt` (the always-on half of
//!   the differential: FLAC is lossless, so ffmpeg's and VLC's dumps
//!   of the same stream must be byte-identical — the moment the
//!   oracle exists, all three digests must agree);
//! - the VLC invocation plumbing is the 3.E3 harness's pinned command,
//!   ready to run the moment a binary exists (the `VLC` env override
//!   points at one).
//!
//! # Comparability (doc 10 §6.6)
//!
//! VLC's `afile` output speaks `u8|s16|float32` — there is **no s24
//! passthrough**, so 24-bit FLAC (the s24 fixture, corpus 0046) is
//! recorded as a non-comparable row with that reason (a converted
//! dump is a transcode, not a lossless comparison). The 16-bit
//! streams compare through headerless native-endian `s16` dumps, MD5
//! per row, on the little-endian host the pinned harness assumes.

mod common;
mod flac;

use std::path::PathBuf;
use std::process::Command;

use common::md5_hex;
use flac::{
    concatenated, corpus_dir, decode_all, fixture_path, fixture_reference, lift_s24_to_s32le,
};
use testkit::Must;

/// The pinned oracle version (the 3.E3 harness's pin; drift aborts the
/// comparison — a different VLC may transcode differently).
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

/// The pinned dump invocation (3.E3 baseline, transcribed from the
/// pinned build's own `--help`): headless, no video out, file audio
/// out, headerless native-endian raw PCM.
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
        "s16".into(),
        "--audiofile-file".into(),
        out.display().to_string(),
        input.display().to_string(),
    ]
}

/// One differential row.
struct Row {
    /// Display name (fixture or corpus path).
    name: String,
    /// The Lumen decode's MD5 (computed here, always).
    lumen_md5: Option<String>,
    /// The comparability verdict.
    comparable: Result<(), String>,
    /// The oracle digest when the run happened.
    vlc_md5: Option<String>,
}

/// Dump VLC's PCM for `input` and return its MD5 (the real oracle
/// path). `Err` carries the reason — never a fabricated digest.
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
        "lumen-flac-vlc-{}.s16",
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
    Ok(md5_hex(&bytes))
}

#[test]
fn flac_differential_vs_vlc_or_pending_infra() {
    let oracle = vlc_binary();

    // The comparable fixture rows (16-bit → VLC s16 passthrough).
    let comparable: [(&str, PathBuf); 3] = [
        (
            "flac_s16_stereo_44100",
            fixture_path("flac_s16_stereo_44100.flac"),
        ),
        (
            "flac_mono_s16_44100",
            fixture_path("flac_mono_s16_44100.flac"),
        ),
        (
            "flac_s16_stereo_44100_seektable",
            fixture_path("flac_s16_stereo_44100_seektable.flac"),
        ),
    ];
    let mut rows: Vec<Row> = Vec::new();

    for (name, path) in comparable {
        // Lumen side: decode through the product path, digest.
        let data = std::fs::read(&path).must();
        let (.., sink) = decode_all(&data);
        let lumen = md5_hex(&concatenated(&sink));
        // The always-on half: the ffmpeg oracle must agree with the
        // Lumen decode (both recorded in the reference data).
        let expect = fixture_reference(name);
        assert_eq!(
            &lumen, &expect["pcm_s16le_md5"],
            "{name}: Lumen decode vs the ffmpeg oracle"
        );
        // VLC side: a real dump when the oracle exists, else pending.
        let vlc_md5 = match &oracle {
            Some(binary) => match vlc_oracle_md5(binary, &path) {
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
            lumen_md5: Some(lumen),
            comparable: Ok(()),
            vlc_md5,
        });
    }

    // The 24-bit rows: no s24 passthrough in VLC's afile — recorded
    // non-comparable with the reason (never silently dropped).
    let s24_path = fixture_path("flac_s24_stereo_48000.flac");
    let data = std::fs::read(&s24_path).must();
    let (.., sink) = decode_all(&data);
    // The digest base is the decoder's S24 output lifted to ffmpeg's
    // s32le convention (`<< 8`) — the same bytes the s32le oracle
    // dumps, so the recorded row is comparable the day VLC grows an
    // s24/s32 passthrough; today it is informational.
    rows.push(Row {
        name: "flac_s24_stereo_48000".to_string(),
        lumen_md5: Some(md5_hex(&lift_s24_to_s32le(&concatenated(&sink)))),
        comparable: Err(
            "VLC afile has no s24 passthrough (u8|s16|float32 only) — a converted \
             dump is a transcode, not a lossless comparison"
                .to_string(),
        ),
        vlc_md5: None,
    });
    if let Some(root) = corpus_dir() {
        let path = root.join("media/0046-audio-only.flac");
        if path.is_file() {
            let data = std::fs::read(&path).must();
            let (.., sink) = decode_all(&data);
            rows.push(Row {
                name: "corpus 0046-audio-only (24-bit)".to_string(),
                lumen_md5: Some(md5_hex(&lift_s24_to_s32le(&concatenated(&sink)))),
                comparable: Err("VLC afile has no s24 passthrough (see above)".to_string()),
                vlc_md5: None,
            });
        }
    } else {
        eprintln!("corpus absent — the 24-bit corpus row SKIPs (see flac_bitexact.rs)");
    }

    // Report every row honestly; assert the real comparisons.
    let mut compared = 0usize;
    for row in &rows {
        match (&row.comparable, &row.vlc_md5) {
            (Ok(()), Some(vlc)) => {
                assert_eq!(
                    row.lumen_md5.as_deref(),
                    Some(vlc.as_str()),
                    "{}: Lumen vs VLC {PINNED_VLC} PCM digest",
                    row.name
                );
                compared += 1;
                eprintln!("{}: Lumen == VLC == ffmpeg ({})", row.name, vlc);
            }
            (Ok(()), None) => eprintln!(
                "{}: Lumen {} — VLC digest PENDING-VLC (no usable oracle)",
                row.name,
                row.lumen_md5.as_deref().unwrap_or("-")
            ),
            (Err(reason), _) => eprintln!(
                "{}: Lumen digest recorded — NOT COMPARABLE: {reason}",
                row.name
            ),
        }
    }
    if oracle.is_none() {
        eprintln!(
            "PENDING-VLC: no vlc/cvlc on PATH and $VLC unset — the comparable rows \
             above carry Lumen's digests (each already asserted against the ffmpeg \
             oracle) and the exact pinned comparison to run; nothing is fabricated. \
             The 3.E3 harness (tools/bench-vs-vlc) carries the same status."
        );
    }
    // The always-on half ran regardless: three comparable rows' Lumen
    // digests were asserted against the ffmpeg oracle above.
    assert!(rows.len() >= 4, "3 comparable + >=1 non-comparable rows");
    assert_eq!(compared, if oracle.is_some() { 3 } else { 0 });
}
