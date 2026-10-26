//! DNxHD differential (card 6.M-dnx AC): the admission record's
//! five lanes (`docs/dependencies/records/filmcraft-dnx-0.9.0.md`
//! §D), regenerated with ffmpeg 7.1.5's own `dnxhd` encoder (the
//! record's exact commands — the corpus holds zero dnxhd lanes) and
//! decoded through the **wired `DnxDecoder`**, checked against
//! ffmpeg's own decode at the Lead-ratified tolerance
//! (6.VTA-DNX-RATIFY):
//!
//! 1. **Lane integrity** (hard): the regenerated lane's ffmpeg
//!    decode hashes to the record's committed per-lane MD5 — the
//!    deterministic lavfi sources + the same encoder reproduce the
//!    record's lanes byte-for-byte, so the §D numbers compare
//!    lane-for-lane.
//! 2. **Byte-count parity** (hard): our emitted byte total equals
//!    ffmpeg's rawvideo byte total.
//! 3. **Tolerance** (hard): per-plane max|Δ| ≤ 2 LSB (the 10-bit
//!    lane: 2 of 1023) — the IDCT-rounding class the record root-
//!    caused (exact orthonormal f32 IDCT vs ffmpeg's integer IDCT;
//!    bit-exactness unreachable in principle).
//! 4. **Sign balance** (hard): the ±1 bulk is balanced (neither
//!    sign dominates 3:1) and |Δ|≥2 outliers stay under 1% of the
//!    differing samples — the record's histogram shape
//!    (`{−1: 24848, +1: 29408, +2: 2}` on L1 frame 0).
//!
//! The 8-bit lanes run the full **trait path** (packet → `decode` →
//! `Frame::Video` in packed YUYV, unpacked by the exact inverse of
//! the wrapper's lossless repack). The 10-bit lane pins the trait
//! surface's typed refusal (the 8-bit pixel vocabulary gap, the
//! av1-recorded 16-bit follow-up seam) and then runs the same
//! differential through the wrapper's full-depth `DnxPicture` seam
//! — the exact engine call the trait path itself runs.
//!
//! Every number the suite prints is measured in this run; the
//! record's §D values are the cross-check, not the source.

mod dnx;

use codec_api::prelude::*;
use codecs::video::dnx::{DnxChroma, DnxDecoder};
use dnx::{
    assert_lane_md5, cbr_chunks, compare_plane, decode_lane, decode_lane_full_depth,
    ffmpeg_available, ffmpeg_rawvideo, generate_lane, lane_facts, split_yuv422p,
    split_yuv422p10le, unpack_yuyv, PlaneStats,
};
use media_types::packet::SharedBytes;

/// One 8-bit lane's full assertion set through the trait path.
fn assert_8bit_lane(
    lane: &str,
    source: &str,
    size: &str,
    bitrate: &str,
    width: usize,
    height: usize,
    frames: usize,
    reference_md5: &str,
) {
    if !ffmpeg_available() {
        eprintln!("SKIP {lane}: no ffmpeg binary (the live oracle is the lane generator)");
        return;
    }
    let path = generate_lane(lane, source, size, bitrate, "yuv422p", frames as u32);
    let data =
        std::fs::read(&path).unwrap_or_else(|e| unreachable!("read {}: {e}", path.display()));
    let reference = ffmpeg_rawvideo(&path);
    assert_lane_md5(&path, reference_md5);

    let facts = lane_facts(&data);
    eprintln!(
        "{lane}: engine facts cid={} {}×{} bit_depth={} interlaced={} par={:?} ({} bytes/frame)",
        facts.cid,
        facts.width,
        facts.height,
        facts.bit_depth,
        facts.interlaced,
        facts.par,
        data.len() / frames
    );

    // Geometry + per-frame differential through the trait path.
    let run = decode_lane(&data, frames);
    assert_eq!(run.sink.frames.len(), frames, "{lane}: frame count");
    let frame_bytes = width * height * 2; // YUYV: 2 bytes per sample
    let ref_frame_bytes = width * height + 2 * (width / 2) * height; // yuv422p rawvideo
    assert_eq!(
        reference.len(),
        ref_frame_bytes * frames,
        "{lane}: ffmpeg rawvideo byte total"
    );
    let mut whole = PlaneStats::default();
    for (index, (format, raster, bytes)) in run.sink.frames.iter().enumerate() {
        assert_eq!(format, "Yuyv422", "{lane} frame {index}: lossless 4:2:2 repack");
        assert_eq!(*raster, (width as u32, height as u32), "{lane} frame {index}: raster");
        assert_eq!(bytes.len(), frame_bytes, "{lane} frame {index}: packed byte count");
        let ref_slice = &reference[index * ref_frame_bytes..(index + 1) * ref_frame_bytes];
        let ours = unpack_yuyv(bytes, width, height);
        let theirs = split_yuv422p(ref_slice, width, height);
        let mut report = format!("{lane} frame {index}:");
        for ((plane, theirs_plane), name) in ours.iter().zip(theirs.iter()).zip(["Y", "Cb", "Cr"]) {
            let stats = compare_plane(plane, theirs_plane);
            report.push_str(&format!(" {}", stats.render(name)));
            assert!(
                stats.max_delta <= 2,
                "{lane} frame {index} {name}: maxΔ {} exceeds the ratified ±2 LSB",
                stats.max_delta
            );
            whole.sum(&stats);
        }
        eprintln!("{report}");
    }
    // (2) Whole-lane byte parity.
    assert_eq!(
        run.sink.frames.iter().map(|(_, _, b)| b.len()).sum::<usize>(),
        reference.len(),
        "{lane}: byte-count parity vs ffmpeg rawvideo"
    );
    balance_gate(lane, &whole);
}

/// The ratified balance gate: ±1 bulk balanced within 3:1 and |Δ|≥2
/// outliers under 1% of the differing samples (calibrated to the
/// record's own histogram: L1 frame 0 = {−1: 24848, +1: 29408,
/// +2: 2} — 0.845 sign ratio, 0.004% outliers).
fn balance_gate(lane: &str, whole: &PlaneStats) {
    eprintln!("{lane}: whole-lane histogram {whole:?}");
    if whole.differing == 0 {
        return; // a bit-exact lane trivially satisfies the class
    }
    assert!(
        whole.twos * 100 <= whole.differing,
        "{lane}: |Δ|≥2 outliers {} exceed 1% of {} differing samples",
        whole.twos,
        whole.differing
    );
    let (minor, major) = (
        whole.neg_one.min(whole.pos_one),
        whole.neg_one.max(whole.pos_one),
    );
    assert!(
        major == 0 || minor * 3 >= major,
        "{lane}: ±1 bulk unbalanced (−1:{}, +1:{}) — a systematic offset, not rounding noise",
        whole.neg_one,
        whole.pos_one
    );
}

/// L1 — testsrc2 1080p 175 Mbps 8-bit (the record's first row).
#[test]
fn lane_l1_1080p_175m() {
    assert_8bit_lane(
        "L1_1080p_175M",
        "testsrc2",
        "1920x1080",
        "175M",
        1920,
        1080,
        3,
        "f555a591a0bc733e18ba7937c212e543",
    );
}

/// L2 — smptehdbars 1080p 36 Mbps 8-bit (the static-content row:
/// the smallest delta fraction, 0.04%).
#[test]
fn lane_l2_1080p_36m() {
    assert_8bit_lane(
        "L2_1080p_36M",
        "smptehdbars",
        "1920x1080",
        "36M",
        1920,
        1080,
        3,
        "ee4523751e6d7ca98bfdd9d19d0b9975",
    );
}

/// L3 — mandelbrot 1080p 145 Mbps 8-bit (the worst-case content:
/// 5–6.6% of samples differ).
#[test]
fn lane_l3_1080p_145m() {
    assert_8bit_lane(
        "L3_1080p_145M",
        "mandelbrot",
        "1920x1080",
        "145M",
        1920,
        1080,
        3,
        "8e924a4f2a22f31cda1821254ed0ef2f",
    );
}

/// L4 — testsrc2 720p 90 Mbps 8-bit (the non-1080p raster).
#[test]
fn lane_l4_720p_90m() {
    assert_8bit_lane(
        "L4_720p_90M",
        "testsrc2",
        "1280x720",
        "90M",
        1280,
        720,
        3,
        "f6d1e56c6bce6f1d2856cfef1ee6f7c5",
    );
}

/// L5 — testsrc2 1080p 185 Mbps **10-bit**: the trait surface
/// refuses it typed (the 8-bit pixel vocabulary gap — pinned), and
/// the same differential runs through the wrapper's full-depth
/// `DnxPicture` seam (the exact engine call the trait path runs).
#[test]
fn lane_l5_1080p10_185m() {
    if !ffmpeg_available() {
        eprintln!("SKIP L5: no ffmpeg binary (the live oracle is the lane generator)");
        return;
    }
    let path = generate_lane(
        "L5_1080p10_185M",
        "testsrc2",
        "1920x1080",
        "185M",
        "yuv422p10le",
        3,
    );
    let data =
        std::fs::read(&path).unwrap_or_else(|e| unreachable!("read {}: {e}", path.display()));
    let reference = ffmpeg_rawvideo(&path);
    assert_lane_md5(&path, "76cb5352e80f969ac3254a2aaa4faee9");

    // (a) The trait path's typed refusal, per frame — the av1-style
    // pinned vocabulary gap (never a downshift, never a guess).
    let mut decoder = dnx_decoder();
    for chunk in cbr_chunks(&data, 3) {
        let packet = raw_es_packet(chunk);
        let mut sink = NullSink;
        match decoder.decode(packet, &mut sink) {
            Err(err) => assert_eq!(
                err.context.code, "codecs.dnx.output.bit_depth",
                "the 10-bit refusal code"
            ),
            Ok(_) => unreachable!("the 8-bit vocabulary must refuse 10-bit samples typed"),
        }
    }

    // (b) The full-depth differential: the record's L5 row.
    let pictures = decode_lane_full_depth(&data, 3);
    assert_eq!(pictures.len(), 3, "L5: frame count");
    let width = 1920usize;
    let height = 1080usize;
    let frame_bytes = 2 * (width * height + 2 * (width / 2) * height);
    assert_eq!(reference.len(), frame_bytes * 3, "L5: ffmpeg byte total");
    let mut whole = PlaneStats::default();
    for (index, picture) in pictures.iter().enumerate() {
        assert_eq!((picture.width, picture.height), (1920, 1080), "L5 raster");
        assert_eq!(picture.chroma, DnxChroma::Yuv422, "L5 subsampling");
        assert_eq!(picture.bit_depth, 10, "L5 bit depth");
        let ref_slice = &reference[index * frame_bytes..(index + 1) * frame_bytes];
        let theirs = split_yuv422p10le(ref_slice, width, height);
        let ours = [&picture.y, &picture.cb, &picture.cr];
        let mut report = format!("L5 frame {index} (cid={}):", picture.cid);
        for ((plane, theirs_plane), name) in ours.iter().zip(theirs.iter()).zip(["Y", "Cb", "Cr"]) {
            let stats = compare_plane(plane, theirs_plane);
            report.push_str(&format!(" {}", stats.render(name)));
            assert!(
                stats.max_delta <= 2,
                "L5 frame {index} {name}: maxΔ {} exceeds the ratified ±2 LSB (of 1023)",
                stats.max_delta
            );
            whole.sum(&stats);
        }
        eprintln!("{report}");
    }
    balance_gate("L5_1080p10_185M", &whole);
}

/// A truncated real frame must refuse typed, not conceal-decode
/// (the admission record's §H flag — the wrapper's CBR gate closes
/// the engine's measured silent-Ok window on half-truncated L1).
#[test]
fn truncated_real_frame_refuses_typed() {
    if !ffmpeg_available() {
        eprintln!("SKIP truncation gate: no ffmpeg binary");
        return;
    }
    let path = generate_lane("L1_1080p_175M", "testsrc2", "1920x1080", "175M", "yuv422p", 3);
    let data =
        std::fs::read(&path).unwrap_or_else(|e| unreachable!("read {}: {e}", path.display()));
    let frame = cbr_chunks(&data, 3)[0];
    let half = &frame[..frame.len() / 2];
    let mut decoder = dnx_decoder();
    let err = match decoder.decode_picture(half) {
        Ok(_) => unreachable!("a half-truncated CBR frame must not decode silently"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "codecs.dnx.frame.short");
    assert_eq!(
        err.kind,
        media_types::ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::Truncated {
                expected: 917_504,
                found: (frame.len() / 2) as u64
            }
        }
    );
    // The decoder stays usable for the next whole frame (the
    // frame-scoped isolation guarantee).
    let mut sink = NullSink;
    match decoder.decode(raw_es_packet(frame), &mut sink) {
        Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
        other => unreachable!("the next whole frame decodes: {other:?}"),
    }
}

/// A raw-ES packet: one complete CBR frame, no timing (the raw
/// `.dnxhd` lanes carry none — the ES host's contract).
fn raw_es_packet(chunk: &[u8]) -> Packet {
    Packet {
        track: media_types::TrackId::new(0),
        pts: media_types::Stamp::Missing,
        dts: media_types::Stamp::Missing,
        duration: None,
        data: SharedBytes::from_vec(chunk.to_vec()),
        keyframe: true,
        discontinuity: false,
        generation: media_types::GenerationId::FIRST,
    }
}

/// A configured decoder (the raw-ES packet contract).
fn dnx_decoder() -> DnxDecoder {
    let mut decoder = DnxDecoder::new();
    match decoder.configure(&DecoderConfig::new(
        match media_types::ids::CodecDescriptor::new("dnxhd") {
            Ok(d) => d,
            Err(e) => unreachable!("{e:?}"),
        },
        media_types::GenerationId::FIRST,
        ThreadBudget::default(),
        None,
        None,
    )) {
        Ok(()) => decoder,
        Err(err) => unreachable!("configure must accept dnxhd: {err}"),
    }
}

/// The sink that swallows (the refusal tests emit nothing).
struct NullSink;

impl FrameSink for NullSink {
    fn accept(&mut self, _frame: media_types::Frame) -> Result<(), media_types::EngineError> {
        Ok(())
    }
}
