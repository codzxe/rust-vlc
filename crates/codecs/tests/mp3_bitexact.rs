//! MP3/MP2 bit-exact suite (card 5.A3): the six committed fixtures
//! decode against the recorded ffmpeg 7.1.5 reference
//! (`tests/data/mp3_bitexact_reference.txt` — provenance, the exact
//! dump/md5 commands, the gapless-window contract and the ±1 LSB
//! lossy tolerance all recorded there).
//!
//! Per fixture the chain is:
//!
//! 1. the decode agrees with ffprobe's stream facts (codec, rate,
//!    channels — reference rows);
//! 2. the walked audio-frame count and the gapless-window sample
//!    count equal the recorded `frames` / `samples_per_channel`
//!    (the window: Xing/Info frame excluded, LAME delay/padding
//!    trimmed on the +529-shifted pipeline timeline — the same
//!    window ffmpeg decodes, see the reference header);
//! 3. Lumen's s16le PCM MD5-equals the recorded `lumen_pcm_md5`
//!    (the exact regression pin: a lossy decoder's MD5 is not
//!    ffmpeg's, but it must never drift);
//! 4. with a live ffmpeg available, the aligned-window per-sample
//!    delta against ffmpeg's own decode stays within the recorded
//!    `max_sample_delta` (the honest lossy tolerance; ±1 LSB).
//!
//! The real-corpus rows (2 MP3s) live in `mp3_corpus_differential.rs`.

mod common;
mod mp3;

use common::md5_hex;
use mp3::{
    decode_s16le, ffmpeg_available, ffmpeg_decode_s16le, fixture_bytes, fixture_path,
    fixture_reference, s16_delta_stats,
};
use testkit::Must;

/// The committed fixture set (name → `fixtures/<name>` + a
/// `[fixture.<name>]` reference section).
const FIXTURES: [&str; 6] = [
    "cbr128.mp3",
    "vbr_q4.mp3",
    "mono48.mp3",
    "mpeg2_24k.mp3",
    "mp2_48k.mp2",
    "tagged_art.mp3",
];

#[test]
fn fixtures_decode_the_recorded_window_exactly() {
    for name in FIXTURES {
        let data = fixture_bytes(name);
        let expect = fixture_reference(name.trim_end_matches(".mp3").trim_end_matches(".mp2"));
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

        // 2. The walked audio frames and the gapless-window length.
        assert_eq!(
            decoded.frames,
            expect["frames"].parse::<usize>().must(),
            "{name}: audio frames (Xing/Info header excluded)"
        );
        let per_channel = decoded.samples.len() / decoded.format.channels as usize;
        assert_eq!(
            per_channel,
            expect["samples_per_channel"].parse::<usize>().must(),
            "{name}: gapless-window samples per channel"
        );

        // 3. The exact s16le regression pin.
        assert_eq!(
            md5_hex(&bytes),
            expect["lumen_pcm_md5"],
            "{name}: Lumen s16le MD5 drifted"
        );

        // 4. Live ffmpeg tolerance check (±1 LSB on the SAME window).
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

#[test]
fn tagged_art_is_cbr128_behind_a_tag() {
    // The ID3v2.3 + embedded-art fixture carries the same PCM essence
    // as cbr128: both sides' MD5s identical — tag skip is neutral.
    let (_, plain) = decode_s16le(&fixture_bytes("cbr128.mp3")).must();
    let (_, tagged) = decode_s16le(&fixture_bytes("tagged_art.mp3")).must();
    assert_eq!(md5_hex(&plain), md5_hex(&tagged));
    let expect = fixture_reference("tagged_art");
    assert_eq!(md5_hex(&tagged), expect["lumen_pcm_md5"]);
    // And the tag itself was parsed, not just skipped.
    let decoded = codecs::audio::mp3::decode_stream(&fixture_bytes("tagged_art.mp3")).must();
    assert!(decoded.id3.is_some(), "the ID3 tag must be parsed");
    assert!(
        decoded.id3.as_ref().is_some_and(|t| t
            .frames
            .iter()
            .any(|f| matches!(f, codecs::audio::mp3::Id3Frame::Picture { .. }))),
        "the embedded art frame must be exposed"
    );
}
