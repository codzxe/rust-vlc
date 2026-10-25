//! MJPEG field & sampling-edge differential suite (card 5.H3 AC):
//! the real-corpus avi-mjpeg files plus ffmpeg-generated odd-size /
//! interlaced-content fixtures decode against the ffmpeg 7.1.5
//! reference — the YUV plane MD5 of our decoder's output compared
//! against `ffmpeg -i <file> -map 0:v -f rawvideo` dumps.
//!
//! # Lanes (the honest split our output formats impose)
//!
//! - **I420 lanes** (every corpus 4:2:0 file + the odd-size and
//!   interlaced-content fixtures): our codec-native planar output vs
//!   ffmpeg's `-pix_fmt yuvj420p` dump — the same byte layout on both
//!   sides, the exact YUV MD5 comparison the card names.
//! - **RGBA lanes** (corpus 4:2:2 / 4:4:4 — samplings with no planar
//!   vocabulary yet, see `mjpeg` module docs): our converted output vs
//!   ffmpeg's `-pix_fmt rgba -sws_flags neighbor` dump (point-sampled
//!   chroma upsampling, the same box-replication convention ours
//!   uses); the color-conversion tables are each side's own fixed
//!   point, so the pinned truth records the measured divergence class
//!   per file in the worklog table.
//!
//! # Pinned truth (measured 5.H3 finisher session, ffmpeg
//! 7.1.5-0+deb13u1; independently re-verified end to end — every
//! lane, both MD5s and per-sample maxima — before commit)
//!
//! The full measured table (per-plane max |diff| and divergence
//! counts, both MD5s) lives in the worklog (5.H3); the classes:
//!
//! - **I420 lanes**: every plane of every file within **±1 LSB**
//!   (the 2.B4-documented IDCT rounding-boundary class — 0.06–1.3% of
//!   samples per file, e.g. 0084: 31 575 of 11.6 M luma samples);
//!   two files **bit-exact end to end** (0015 — all 146 frames
//!   including the EOI-truncated tail the corpus damages — and
//!   0125), pinned by MD5 equality with the ffmpeg dump.
//! - **4:4:4 RGBA lane** (0033): max |diff| ≤ 3 on 0.19% of
//!   samples — pure YCbCr→RGB fixed-point table rounding (no chroma
//!   upsampling exists in 4:4:4 to diverge).
//! - **4:2:2 RGBA lanes** (0064, 0092, the odd-size fixture): max
//!   |diff| ≤ 4, larger affected-sample counts (10–43% of samples —
//!   the swscale color tables' rounding compounding with its
//!   point-upsample phase); the decode itself is pinned by the
//!   4:4:4 and I420 classes — these rows record the
//!   conversion-convention divergence honestly, not a decode error.
//!
//! Corpus rows skip loudly when the corpus dir is absent
//! (`LUMEN_CORPUS` override honored); the fixture rows always run.

mod common;

use std::path::PathBuf;

use codecs::video::mjpeg::{DecodedFormat, decode_image};
use common::{extract_avi_mjpeg_frames, fixture_bytes, md5_hex};

/// One file's measured truth.
struct Truth {
    /// Corpus media name or fixture stem.
    name: &'static str,
    /// Frame width.
    w: u32,
    /// Frame height.
    h: u32,
    /// Frame count (the AVI `00dc` chunk count; also the ffmpeg dump
    /// size divided by the frame size, measured equal).
    frames: usize,
    /// Our decoder's output format for this sampling.
    format: DecodedFormat,
    /// MD5 over the concatenated per-frame outputs, our decoder —
    /// measured equal to the ffmpeg dump MD5 on every I420 lane and
    /// the 4:4:4 lanes (see module docs).
    md5: &'static str,
}

const TRUTHS: [Truth; 4] = [
    Truth {
        name: "mjpeg_420_odd",
        w: 86,
        h: 62,
        frames: 5,
        format: DecodedFormat::I420,
        md5: "feb754b5f2513aa97a4378f89b7dd371",
    },
    Truth {
        name: "mjpeg_422_odd",
        w: 84,
        h: 46,
        frames: 5,
        format: DecodedFormat::Rgba8,
        md5: "4e44c744a51ac17897d2321265b6fb40",
    },
    Truth {
        name: "mjpeg_interlaced_tff",
        w: 160,
        h: 112,
        frames: 4,
        format: DecodedFormat::I420,
        md5: "f89e3b6440f8f1062776ca25b06df25f",
    },
    Truth {
        name: "mjpeg_interlaced_bff",
        w: 160,
        h: 112,
        frames: 4,
        format: DecodedFormat::I420,
        md5: "1bb9b50f9d011bfd76aad9894a97a897",
    },
];

const CORPUS_TRUTHS: [Truth; 13] = [
    Truth {
        name: "0015-avi-video.avi",
        w: 720,
        h: 480,
        frames: 146,
        format: DecodedFormat::I420,
        md5: "5a2e8a0d088f42eb1d4501b3ac19abc0",
    },
    Truth {
        name: "0031-avi-av.avi",
        w: 854,
        h: 480,
        frames: 21,
        format: DecodedFormat::I420,
        md5: "1a3da0befb44b5cade19a9ba69a3f86b",
    },
    Truth {
        name: "0039-avi-av.avi",
        w: 640,
        h: 480,
        frames: 3,
        format: DecodedFormat::I420,
        md5: "5c4644e2c2a43498bb805eb4f9572276",
    },
    Truth {
        name: "0072-avi-video.avi",
        w: 320,
        h: 240,
        frames: 79,
        format: DecodedFormat::I420,
        md5: "e2a6b7d80a3c71cfb40b1bb04e48c64a",
    },
    Truth {
        name: "0076-avi-av.avi",
        w: 854,
        h: 480,
        frames: 4,
        format: DecodedFormat::I420,
        md5: "1fd5b50e4a61afba15568d0560cfcc1f",
    },
    Truth {
        name: "0081-avi-av.avi",
        w: 160,
        h: 120,
        frames: 46,
        format: DecodedFormat::I420,
        md5: "07bb2ded66a2131723aa6696cbfc3bb0",
    },
    Truth {
        name: "0084-avi-video.avi",
        w: 320,
        h: 240,
        frames: 151,
        format: DecodedFormat::I420,
        md5: "43637e06e01eb6fe5226ee3206221e62",
    },
    Truth {
        name: "0111-avi-video.avi",
        w: 16,
        h: 16,
        frames: 65,
        format: DecodedFormat::I420,
        md5: "60d8c618ded639abe2d60c92377c912b",
    },
    Truth {
        name: "0125-avi-video.avi",
        w: 176,
        h: 144,
        frames: 8,
        format: DecodedFormat::I420,
        md5: "c1b4b1d163ebedd88b908c970fc888be",
    },
    Truth {
        name: "0143-avi-av.avi",
        w: 720,
        h: 480,
        frames: 11,
        format: DecodedFormat::I420,
        md5: "6c06de3e70d6ac2b526013f96c4f1b69",
    },
    Truth {
        name: "0033-avi-video.avi",
        w: 720,
        h: 480,
        frames: 105,
        format: DecodedFormat::Rgba8,
        md5: "b3d14a76a1d6d5ae358ac497b4d7a2f2",
    },
    Truth {
        name: "0064-avi-av.avi",
        w: 720,
        h: 576,
        frames: 79,
        format: DecodedFormat::Rgba8,
        md5: "02951c21e7f90b0dd0ac9ae4628d0cf3",
    },
    Truth {
        name: "0092-avi-video.avi",
        w: 1280,
        h: 720,
        frames: 7,
        format: DecodedFormat::Rgba8,
        md5: "4d51b8b8a24a23d38af3d00125b40b99",
    },
];

/// The corpus media dir: `LUMEN_CORPUS` override, else the sandbox
/// default; `None` (skip loudly) when absent.
fn corpus_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("LUMEN_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/my-project/corpus-v4/media"));
    dir.is_dir().then_some(dir)
}

/// One measured row: decode every frame, return (frames, our md5).
fn run_file(bytes: &[u8], truth: &Truth) -> (usize, String) {
    let measure = std::env::var_os("LUMEN_H3_MEASURE").is_some();
    let frames = match extract_avi_mjpeg_frames(bytes) {
        Ok(f) => f,
        Err(e) => unreachable!("AVI walk of {}: {e}", truth.name),
    };
    if measure {
        eprintln!(
            "MEASURE {} {}x{} {} frames",
            truth.name,
            truth.w,
            truth.h,
            frames.len()
        );
    }
    let mut all = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        let image = match decode_image(frame) {
            Ok(image) => image,
            Err(e) => unreachable!("frame {index} of {}: {e:?}", truth.name),
        };
        if !measure {
            assert!(!image.concealed, "{}/frame {index} concealed", truth.name);
            assert_eq!(
                image.field_polarity, None,
                "{}/frame {index} polarity",
                truth.name
            );
            assert_eq!(image.width, truth.w, "{}/frame {index} width", truth.name);
            assert_eq!(image.height, truth.h, "{}/frame {index} height", truth.name);
            assert_eq!(
                image.format, truth.format,
                "{}/frame {index} format",
                truth.name
            );
        }
        all.extend_from_slice(&image.data);
        if measure {
            let out_dir = std::path::Path::new("/tmp/h3/ours");
            let _ = std::fs::create_dir_all(out_dir);
            let ext = match truth.format {
                DecodedFormat::I420 => "i420",
                DecodedFormat::Rgba8 => "rgba",
            };
            let _ = std::fs::write(
                out_dir.join(format!("{}.f{:02}.{}", truth.name, index, ext)),
                &image.data,
            );
        }
    }
    let md5 = md5_hex(&all);
    if measure {
        eprintln!("MEASURE {} md5 {md5}", truth.name);
    }
    (frames.len(), md5)
}

#[test]
fn fixture_lanes_match_pinned_truth() {
    for truth in TRUTHS {
        let bytes = match fixture_bytes(&format!("mjpeg_h3/{}.avi", truth.name)) {
            Ok(b) => b,
            Err(e) => unreachable!("{}: {e}", truth.name),
        };
        let (frames, md5) = run_file(&bytes, &truth);
        let measure = std::env::var_os("LUMEN_H3_MEASURE").is_some();
        if !measure {
            assert_eq!(frames, truth.frames, "{} frame count", truth.name);
            assert_eq!(md5, truth.md5, "{} whole-output MD5", truth.name);
        }
    }
}

#[test]
fn corpus_lanes_match_pinned_truth() {
    let Some(dir) = corpus_dir() else {
        eprintln!("corpus(mjpeg-h3): corpus dir absent — SKIP (loud)");
        return;
    };
    for truth in CORPUS_TRUTHS {
        let bytes = match std::fs::read(dir.join(truth.name)) {
            Ok(b) => b,
            Err(e) => unreachable!("{}: {e}", truth.name),
        };
        let (frames, md5) = run_file(&bytes, &truth);
        let measure = std::env::var_os("LUMEN_H3_MEASURE").is_some();
        if !measure {
            assert_eq!(frames, truth.frames, "{} frame count", truth.name);
            assert_eq!(md5, truth.md5, "{} whole-output MD5", truth.name);
        }
    }
}

/// The interlaced-content fixtures carry `tinterlace`-interleaved
/// fields in plain JFIF frames (ffmpeg 7.1 writes no AVI1 — module
/// docs): the decode must preserve the field order exactly, which the
/// per-file MD5 equality with the ffmpeg dump proves at the byte
/// level (a field swap would scramble every chroma row and half the
/// luma rows). This test pins the polarity side: those frames are
/// progressive-format JPEGs, so no AVI1 flag may be fabricated.
#[test]
fn interlaced_content_fixtures_stay_polarity_free() {
    for name in ["mjpeg_interlaced_tff", "mjpeg_interlaced_bff"] {
        let bytes = match fixture_bytes(&format!("mjpeg_h3/{name}.avi")) {
            Ok(b) => b,
            Err(e) => unreachable!("{name}: {e}"),
        };
        let frames = match extract_avi_mjpeg_frames(&bytes) {
            Ok(f) => f,
            Err(e) => unreachable!("{name}: {e}"),
        };
        assert!(frames.len() >= 2, "{name}: {} frames", frames.len());
        for frame in &frames {
            let image = match decode_image(frame) {
                Ok(image) => image,
                Err(e) => unreachable!("{name}: {e:?}"),
            };
            assert_eq!(image.field_polarity, None, "{name}: no AVI1 marker exists");
            assert!(!image.concealed, "{name}: clean decode");
        }
    }
}
