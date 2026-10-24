//! AAC-LC bit-exact suite (card 5.C2a): the two committed fixtures
//! decode against the recorded ffmpeg 7.x reference
//! (`tests/data/aac_bitexact_reference.txt` — provenance, the exact
//! dump/md5 commands, the all-complete-frames window contract and
//! the ±1 LSB lossy tolerance all recorded there).
//!
//! Per fixture the chain is:
//!
//! 1. the decode agrees with ffprobe's stream facts (rate, channels
//!    — reference rows);
//! 2. the walked frame count and the sample count equal the recorded
//!    `frames` / `samples_per_channel`;
//! 3. Lumen's s16le PCM MD5-equals the recorded `lumen_pcm_md5`
//!    (the exact regression pin: a lossy decoder's MD5 is not
//!    ffmpeg's, but it must never drift);
//! 4. with a live ffmpeg available, the per-sample delta against
//!    ffmpeg's own decode stays within the recorded
//!    `max_sample_delta` (the honest lossy tolerance; ±1 LSB).
//!
//! The real-corpus rows (0059/0110 .aac + the m4a bridge) live in
//! `aac_corpus_differential.rs`.

mod aac;
mod common;

use std::collections::HashMap;
use std::sync::OnceLock;

use aac::{
    decode_s16le, ffmpeg_available, ffmpeg_decode_s16le, fixture_bytes, fixture_path,
    s16_delta_stats,
};
use common::md5_hex;
use testkit::Must;

/// The committed fixture set (name → `fixtures/<name>` + a
/// `[fixture.<name>]` reference section).
const FIXTURES: [&str; 2] = ["sine441_lc_stereo.aac", "sine48_lc_mono.aac"];

type Section = HashMap<String, String>;

fn reference() -> &'static HashMap<String, Section> {
    static REF: OnceLock<HashMap<String, Section>> = OnceLock::new();
    REF.get_or_init(|| {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/aac_bitexact_reference.txt");
        parse_reference(&std::fs::read(&path).must()).unwrap_or_else(|e| unreachable!("{e}"))
    })
}

fn fixture_reference(name: &str) -> &'static Section {
    reference()
        .get(&format!("fixture.{name}"))
        .unwrap_or_else(|| unreachable!("no [fixture.{name}] section"))
}

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
            current = name
                .strip_suffix(']')
                .ok_or("unterminated [section]")?
                .to_string();
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

#[test]
fn fixtures_decode_the_recorded_window_exactly() {
    for name in FIXTURES {
        let data = fixture_bytes(name);
        let expect = fixture_reference(name.trim_end_matches(".aac"));
        let (decoded, bytes) = match decode_s16le(&data) {
            Ok(x) => x,
            Err(e) => unreachable!("{name} must decode: {e}"),
        };

        // 1. ffprobe's stream facts.
        assert_eq!(
            decoded.format.rate,
            expect["rate"].parse::<u32>().must(),
            "{name}: rate"
        );
        assert_eq!(
            decoded.format.channels,
            expect["channels"].parse::<u32>().must(),
            "{name}: channels"
        );

        // 2. The walked frames and the window length.
        assert_eq!(
            decoded.frames,
            expect["frames"].parse::<usize>().must(),
            "{name}: frame count"
        );
        let per_channel = decoded.samples.len() / decoded.format.channels as usize;
        assert_eq!(
            per_channel,
            expect["samples_per_channel"].parse::<usize>().must(),
            "{name}: samples per channel"
        );

        // 3. The exact s16le regression pin.
        assert_eq!(
            md5_hex(&bytes),
            expect["lumen_pcm_md5"],
            "{name}: Lumen s16le MD5 drifted"
        );

        // 4. Live ffmpeg tolerance check (±1 LSB on the same window).
        if ffmpeg_available() {
            let reference = ffmpeg_decode_s16le(&fixture_path(name));
            assert_eq!(
                reference.len(),
                bytes.len(),
                "{name}: ffmpeg decoded a different window length"
            );
            let tolerance = expect["max_sample_delta"].parse::<i64>().must();
            let (differing, max_delta) = s16_delta_stats(&bytes, &reference);
            assert!(
                max_delta <= tolerance,
                "{name}: max sample delta {max_delta} > recorded tolerance {tolerance} \
                 ({differing} differing samples)"
            );
        } else {
            eprintln!("SKIP live-ffmpeg tolerance for {name}: no ffmpeg binary");
        }
    }
}

/// The 1024-samples-per-frame constant of AAC-LC, pinned on real
/// streams (every fixture's `samples_per_channel == frames × 1024`).
#[test]
fn lc_frames_are_exactly_1024_samples() {
    for name in FIXTURES {
        let decoded = match aac::decode_s16le(&fixture_bytes(name)) {
            Ok((d, _)) => d,
            Err(e) => unreachable!("{name} must decode: {e}"),
        };
        let per_channel = decoded.samples.len() / decoded.format.channels as usize;
        assert_eq!(per_channel, decoded.frames * 1024, "{name}");
    }
}
