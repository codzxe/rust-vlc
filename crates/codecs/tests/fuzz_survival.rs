//! Committed-corpus survival run for the `fuzz_codec_mjpeg` harness
//! (card 2.B4 `+FB`): every fixture file, every extracted frame, and
//! systematic truncations/mutations of a real frame pass through the
//! harness. The harness contract is absence-of-panic (its module
//! docs); specific outcomes are the AC suite's job, not this one.

mod common;

use codecs::video::mjpeg::fuzz::fuzz_codec_mjpeg;
use common::{extract_avi_mjpeg_frames, extract_mp4_mjpeg_frames, fixture_bytes};

/// Every fixture file in the corpus (both class-A parents and all
/// class-B derivatives): container bytes are arbitrary input to a
/// JPEG decoder — survival is the only claim.
fn all_fixture_files() -> Vec<String> {
    let mut files = vec![
        "generated/valid/avi_mjpeg_pcm.avi".to_string(),
        "generated/valid/mp4_mjpeg_pcm.mp4".to_string(),
    ];
    for name in [
        "avi_mjpeg_pcm__flip1.avi",
        "avi_mjpeg_pcm__flip4.avi",
        "avi_mjpeg_pcm__flip16.avi",
        "avi_mjpeg_pcm__garbage0.avi",
        "avi_mjpeg_pcm__noidx0.avi",
        "avi_mjpeg_pcm__trunc0.avi",
        "avi_mjpeg_pcm__trunc1.avi",
        "avi_mjpeg_pcm__trunc2.avi",
        "avi_mjpeg_pcm__trunc3.avi",
        "avi_mjpeg_pcm__trunc4.avi",
        "mp4_mjpeg_pcm__flip1.mp4",
        "mp4_mjpeg_pcm__flip4.mp4",
        "mp4_mjpeg_pcm__flip16.mp4",
        "mp4_mjpeg_pcm__garbage0.mp4",
        "mp4_mjpeg_pcm__stszlie0.mp4",
        "mp4_mjpeg_pcm__trunc0.mp4",
        "mp4_mjpeg_pcm__trunc1.mp4",
        "mp4_mjpeg_pcm__trunc2.mp4",
        "mp4_mjpeg_pcm__trunc3.mp4",
        "mp4_mjpeg_pcm__trunc4.mp4",
    ] {
        files.push(format!("generated/malformed/{name}"));
    }
    files
}

#[test]
fn every_fixture_file_survives_the_harness() {
    for relative in all_fixture_files() {
        let bytes = match fixture_bytes(&relative) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        // The pass is the return; panics/hangs are the only failures.
        fuzz_codec_mjpeg(&bytes);
    }
}

#[test]
fn every_extracted_frame_survives_the_harness() {
    for parent in [
        "generated/valid/avi_mjpeg_pcm.avi",
        "generated/valid/mp4_mjpeg_pcm.mp4",
    ] {
        let bytes = match fixture_bytes(parent) {
            Ok(b) => b,
            Err(e) => unreachable!("{e}"),
        };
        let frames = match if parent.ends_with(".avi") {
            extract_avi_mjpeg_frames(&bytes)
        } else {
            extract_mp4_mjpeg_frames(&bytes)
        } {
            Ok(f) => f,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(frames.len(), 60, "{parent}");
        for frame in &frames {
            fuzz_codec_mjpeg(frame);
        }
    }
}

#[test]
fn systematic_truncations_of_a_real_frame_survive() {
    let bytes = match fixture_bytes("generated/valid/avi_mjpeg_pcm.avi") {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let frames = match extract_avi_mjpeg_frames(&bytes) {
        Ok(f) => f,
        Err(e) => unreachable!("{e}"),
    };
    let frame = &frames[0];
    let mut cut = 1;
    while cut < frame.len() {
        fuzz_codec_mjpeg(&frame[..cut]);
        cut += 128;
    }
    // The empty prefix too (distinct damage class: nothing at all).
    fuzz_codec_mjpeg(&[]);
}

#[test]
fn systematic_single_byte_flips_of_a_real_frame_survive() {
    let bytes = match fixture_bytes("generated/valid/avi_mjpeg_pcm.avi") {
        Ok(b) => b,
        Err(e) => unreachable!("{e}"),
    };
    let frames = match extract_avi_mjpeg_frames(&bytes) {
        Ok(f) => f,
        Err(e) => unreachable!("{e}"),
    };
    let mut frame = frames[0].clone();
    let mut offset = 0;
    while offset < frame.len() {
        frame[offset] = frame[offset].wrapping_add(1);
        fuzz_codec_mjpeg(&frame);
        frame[offset] = frame[offset].wrapping_sub(1);
        offset += 64;
    }
}

#[test]
fn adversarial_geometries_survive_the_harness() {
    // A complete header set declaring the largest in-cap geometry
    // (8 MiP) with almost no entropy data: exercises the padded-plane
    // allocation and the concealment fill at their budget ceiling.
    let mut jpeg = vec![0xff, 0xd8];
    let seg = |jpeg: &mut Vec<u8>, marker: u8, payload: &[u8]| {
        jpeg.push(0xff);
        jpeg.push(marker);
        jpeg.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        jpeg.extend_from_slice(payload);
    };
    let mut dqt = vec![0u8];
    dqt.extend(std::iter::repeat_n(16u8, 64));
    seg(&mut jpeg, 0xdb, &dqt);
    for class in [0x00u8, 0x10] {
        let mut dht = vec![class];
        dht.extend(std::iter::repeat_n(0u8, 16));
        dht[1] = 1;
        dht.push(0x00);
        seg(&mut jpeg, 0xc4, &dht);
    }
    // 2896×2896 ≈ 8.39 MiP — just above the 8 MiP cap is rejected;
    // 2890×2900 ≈ 8.38 MiP is the near-ceiling survivor case.
    let mut sof = vec![8u8];
    sof.extend_from_slice(&2900u16.to_be_bytes());
    sof.extend_from_slice(&2890u16.to_be_bytes());
    sof.extend_from_slice(&[1, 1, 0x11, 0]);
    seg(&mut jpeg, 0xc0, &sof);
    seg(&mut jpeg, 0xda, &[1, 1, 0x00, 0, 63, 0]);
    jpeg.extend_from_slice(&[0x7f, 0xff, 0xd9]);
    fuzz_codec_mjpeg(&jpeg);

    // And one past the cap (8193×8193) — the typed rejection path.
    let mut oversize = vec![0xff, 0xd8];
    let mut sof = vec![8u8];
    sof.extend_from_slice(&[0x20, 0x01, 0x20, 0x01, 1, 1, 0x11, 0]);
    seg(&mut oversize, 0xdb, &dqt);
    for class in [0x00u8, 0x10] {
        let mut dht = vec![class];
        dht.extend(std::iter::repeat_n(0u8, 16));
        dht[1] = 1;
        dht.push(0x00);
        seg(&mut oversize, 0xc4, &dht);
    }
    seg(&mut oversize, 0xc0, &sof);
    seg(&mut oversize, 0xda, &[1, 1, 0x00, 0, 63, 0]);
    oversize.extend_from_slice(&[0x00, 0xff, 0xd9]);
    fuzz_codec_mjpeg(&oversize);
}
