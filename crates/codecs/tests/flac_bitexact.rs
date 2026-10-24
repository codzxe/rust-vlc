//! FLAC bit-exact differential suite (card 5.A1 AC): every committed
//! fixture and every real-corpus FLAC decodes **byte-exactly** against
//! the recorded ffmpeg 7.1.5 reference (`tests/data/
//! flac_bitexact_reference.txt` — generation commands, dump commands,
//! MD5s, ffprobe frame counts and packet pts all recorded there).
//!
//! The comparison chain per file:
//!
//! 1. the metadata walk agrees with ffprobe's stream facts
//!    (channels, rate, bits, total samples);
//! 2. the frame walker splits exactly `frames` packets with exactly
//!    the recorded `first_samples` (ffprobe packet pts, the tail
//!    frame included — the spec fixed-blocksize conversion);
//! 3. the contract path (`FlacDecoder`: configure with the verbatim
//!    STREAMINFO, one packet per frame) emits one audio frame per
//!    packet whose concatenated PCM MD5-matches ffmpeg's own decoded
//!    dump (s16le for 16-bit; for 24-bit the S24 output lifted by
//!    ≪ 8 against ffmpeg's s32le dump);
//! 4. the concatenated PCM at the stream's native width MD5-matches
//!    the STREAMINFO `md5` field — the *encoder's own* integrity
//!    signature, an end-to-end check that needs no oracle at all;
//! 5. the free surface (`decode_frame`) produces the identical bytes.
//!
//! The real corpus (`LUMEN_CORPUS` env or the sandbox default) adds
//! the three corpus rows; when absent the corpus test SKIPs with a
//! message (the fixtures above are the always-on coverage).

mod common;
mod flac;

use codecs::audio::flac::{decode_frame, frame_header_at, is_flac, parse_stream};
use common::md5_hex;
use flac::{
    concatenated, corpus_dir, decode_all, fixture_bytes, fixture_reference, lift_s24_to_s32le,
    reference_list, to_native_le, walk_frames,
};
use media_types::ErrorKind;
use testkit::Must;

/// The committed fixture set (names map to `fixtures/<name>.flac` and
/// `[fixture.<name>]` reference sections).
const FIXTURES: [&str; 4] = [
    "flac_s16_stereo_44100",
    "flac_mono_s16_44100",
    "flac_s24_stereo_48000",
    "flac_s16_stereo_44100_seektable",
];

#[test]
fn fixtures_decode_byte_exact_against_the_ffmpeg_reference() {
    for name in FIXTURES {
        let data = fixture_bytes(&format!("{name}.flac"));
        let expect = fixture_reference(name);
        let (layout, sink) = decode_all(&data);
        let facts = &layout.stream_info;

        // 1. The walk agrees with ffprobe's stream facts.
        assert_eq!(
            facts.channels,
            expect["channels"].parse::<u32>().must(),
            "{name}"
        );
        assert_eq!(
            facts.sample_rate,
            expect["rate"].parse::<u32>().must(),
            "{name}"
        );
        assert_eq!(
            facts.bits_per_sample,
            expect["bps"].parse::<u32>().must(),
            "{name}"
        );
        assert_eq!(
            facts.total_samples,
            Some(expect["total_samples"].parse::<u64>().must()),
            "{name}"
        );

        // 2. The frame split matches ffprobe's packet count and pts.
        let walked = walk_frames(&data, facts, layout.frames_start);
        assert_eq!(
            walked.len(),
            expect["frames"].parse::<usize>().must(),
            "{name}"
        );
        let want_samples = reference_list(expect, "first_samples");
        let got_samples: Vec<u64> = walked.iter().map(|f| f.first_sample).collect();
        assert_eq!(got_samples, want_samples, "{name}: per-frame first samples");
        assert_eq!(
            sink.first_samples, want_samples,
            "{name}: decoded pts per frame"
        );
        // The block sizes sum to the declared total; the tail frame is
        // the one shorter than the stream's fixed block size.
        if let Some(total) = facts.total_samples.as_ref() {
            let covered: u64 = walked.iter().map(|f| u64::from(f.block_size)).sum();
            assert_eq!(covered, *total, "{name}: block sizes sum to total samples");
            assert!(
                walked
                    .last()
                    .is_some_and(|f| u64::from(f.block_size) < u64::from(facts.max_block_size)),
                "{name}: the final block is the short tail"
            );
        }

        // 3. Byte-exact PCM vs the ffmpeg dump (width-lifted for 24-bit).
        let pcm = concatenated(&sink);
        let pcm_md5 = md5_hex(&pcm);
        match facts.bits_per_sample {
            bps if bps <= 16 => {
                assert_eq!(pcm_md5, expect["pcm_s16le_md5"], "{name}: s16le PCM");
            }
            bps if bps <= 24 => {
                let lifted = md5_hex(&lift_s24_to_s32le(&pcm));
                assert_eq!(lifted, expect["pcm_s32le_md5"], "{name}: s24→s32le PCM");
            }
            _ => unreachable!("{name}: no 32-bit fixture in this set"),
        }
        // 4. The encoder's own STREAMINFO signature (native-width LE).
        let width = usize::try_from(facts.bits_per_sample / 8).must();
        assert_eq!(
            md5_hex(&to_native_le(&pcm, width)),
            expect["streaminfo_md5"],
            "{name}: streaminfo md5"
        );

        // 5. The free surface produces the identical bytes per frame.
        for (index, frame) in walked.iter().enumerate() {
            let decoded = match decode_frame(&frame.packet, facts) {
                Ok(decoded) => decoded,
                Err(err) => unreachable!("{name} frame {index} free decode: {err}"),
            };
            assert_eq!(decoded.samples, sink.buffers[index], "{name} frame {index}");
            assert_eq!(
                decoded.first_sample, frame.first_sample,
                "{name} frame {index}"
            );
            assert_eq!(decoded.block_size, frame.block_size, "{name} frame {index}");
        }
    }
}

#[test]
fn seektable_carrier_exposes_six_verified_points() {
    let data = fixture_bytes("flac_s16_stereo_44100_seektable.flac");
    let expect = fixture_reference("flac_s16_stereo_44100_seektable");
    let layout = match parse_stream(&data) {
        Ok(layout) => layout,
        Err(err) => unreachable!("{err}"),
    };
    let index = layout.seek_index.as_ref().must();
    assert_eq!(
        index.points.len(),
        expect["seektable_points"].parse::<usize>().must()
    );
    // Every point's offset must resolve to a frame header whose
    // first sample equals the point (the resolution verify rule).
    for point in &index.points {
        let header = frame_header_at(&data, layout.frames_start + point.offset).must();
        assert_eq!(header.first_sample(&layout.stream_info), point.sample);
    }
}

/// The fuzz-found panic class, pinned with a DECODABLE frame: a mono
/// frame plus facts declaring 2 channels must be the typed
/// `flac.frame.channels` corruption — never a panic in the engine's
/// channel-plane indexing (the gate now runs before any plane access).
#[test]
fn channel_mismatch_on_a_decodable_frame_is_typed_not_panic() {
    let data = fixture_bytes("flac_mono_s16_44100.flac");
    let layout = match parse_stream(&data) {
        Ok(layout) => layout,
        Err(err) => unreachable!("{err}"),
    };
    let walked = walk_frames(&data, &layout.stream_info, layout.frames_start);
    assert_eq!(walked.len(), 3);
    let mut doctored = layout.stream_info;
    doctored.channels = 2;
    for (index, frame) in walked.iter().enumerate() {
        match decode_frame(&frame.packet, &doctored) {
            Err(err) => {
                assert_eq!(
                    err.context.code, "flac.frame.channels",
                    "frame {index}: {err}"
                );
                assert!(
                    matches!(err.kind, ErrorKind::Corrupt { .. }),
                    "frame {index}"
                );
            }
            Ok(decoded) => unreachable!("frame {index} must not decode: {decoded:?}"),
        }
    }
}

/// The three real corpus FLACs (skip with a message when the corpus is
/// absent — /tmp does not survive re-imaging; a hang would not be
/// honest, a skip is).
#[test]
fn corpus_flacs_decode_byte_exact_or_reject_typed() {
    let Some(root) = corpus_dir() else {
        eprintln!("corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP");
        return;
    };
    let reference = flac::reference();

    // 0046: the real differential row (88.2 kHz, 24-bit, 44 frames).
    let data = std::fs::read(root.join("media/0046-audio-only.flac")).must();
    let section = reference.get("corpus.0046-audio-only").must();
    let (layout, sink) = decode_all(&data);
    let facts = &layout.stream_info;
    assert_eq!(facts.sample_rate, 88_200);
    assert_eq!(facts.channels, 2);
    assert_eq!(facts.bits_per_sample, 24);
    assert_eq!(facts.total_samples, Some(352_800));
    assert_eq!(
        sink.buffers.len(),
        section["frames"].parse::<usize>().must()
    );
    let pcm = concatenated(&sink);
    // ffmpeg's s32le dump, and the encoder's own STREAMINFO signature
    // at the native 24-bit width.
    assert_eq!(
        md5_hex(&lift_s24_to_s32le(&pcm)),
        section["pcm_s32le_md5"],
        "0046: s24→s32le PCM vs ffmpeg"
    );
    assert_eq!(
        md5_hex(&to_native_le(&pcm, 3)),
        section["streaminfo_md5"],
        "0046: STREAMINFO md5"
    );

    // 0106: metadata-only (cover art, zero frames — ffmpeg itself
    // decodes 0 audio packets here; the honest expectation is 0).
    let data = std::fs::read(root.join("media/0106-cover-audio.flac")).must();
    let (layout, sink) = decode_all(&data);
    assert_eq!(layout.seek_index, None);
    assert_eq!(layout.stream_info.total_samples, None);
    assert!(
        sink.buffers.is_empty(),
        "no frames in a metadata-only stream"
    );

    // 0071: an ID3/MP3 file with a .flac extension — typed rejection.
    let data = std::fs::read(root.join("media/0071-cover-audio.flac")).must();
    assert!(!is_flac(&data));
    let err = match parse_stream(&data) {
        Err(err) => err,
        Ok(layout) => unreachable!("an ID3 file must not parse as FLAC: {layout:?}"),
    };
    assert_eq!(err.context.code, "flac.probe.magic");
    assert!(matches!(err.kind, ErrorKind::Unsupported { .. }));
}
