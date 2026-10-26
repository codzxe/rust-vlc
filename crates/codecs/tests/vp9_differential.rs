//! VP9 differential suite (card 6.M-vp9 AC, binding admission
//! condition b of `docs/dependencies/records/rusty_vp9-0.1.3.md`):
//! the record's six lanes, re-run through the **wired**
//! [`codecs::video::vp9::Vp9Decoder`] — demuxed by the landed
//! `demux::ivf`, decoded behind the codec-api contract, and
//! compared against the ffmpeg `libvpx-vp9` oracle.
//!
//! The fixtures ARE the admission record's lanes (byte-identical,
//! staged under `tests/fixtures/vp9/`; the reference decode MD5s
//! were re-measured against ffmpeg 7.1.5 `libvpx-vp9` this session
//! before committing — all six match the record's §D table). The
//! record's classification is **bit-exact** (not tolerance-class):
//! the assertion is whole-stream MD5 equality plus per-frame
//! identity, not a delta budget.
//!
//! Per lane (A–E, 8-bit 4:2:0): frame count == packet count ==
//! ffprobe truth, dimensions per frame, the pts ladder restored
//! from the demuxer's stamps, and the concatenated tight-I420
//! output MD5 == ffmpeg's own `-f rawvideo -pix_fmt yuv420p` dump
//! (computed live when ffmpeg is on PATH; the pinned constants are
//! that same command's output). Lane F (profile-2 10-bit) pins the
//! v0 output boundary: a typed `vp9.bitdepth` `Unsupported`
//! refusal — never a mis-decode, never silence (the av1 10-bit
//! fixture precedent); the engine-level 10-bit bit-exactness stays
//! the record's measurement, re-confirmable via `tools/vta/vp9`.
//!
//! The hostile lanes at the bottom re-run the record's §H battery
//! **through the wrapper**: garbage and zero-length packets are
//! `Recovered`, the off-by-one truncation sweep is panic-free, and
//! a garbage-then-real stream recovers (the record measured 10/10
//! frames after a leading garbage packet).

mod common;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use codec_api::prelude::*;
use codecs::video::vp9::Vp9Decoder;
use common::md5_hex;
use demux::ivf::IvfDemuxer;
use demux::ivf::test_support::mem_stream;
use media_types::frame::Frame;
use media_types::ids::CodecDescriptor;
use media_types::{GenerationId, Packet};
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

/// One lane's expected truth (the admission record's §D table,
/// re-verified this session: ffprobe counts + ffmpeg decode MD5s).
struct Lane {
    /// Fixture name under `tests/fixtures/vp9/`.
    name: &'static str,
    /// ffprobe `nb_read_frames` == the record's frame count.
    frames: u32,
    /// ffprobe `width`/`height` (lane C: libvpx coded render, the
    /// asked 127x95 does not exist in the file).
    size: (u32, u32),
    /// MD5 of ffmpeg's decoded yuv420p dump (the record's
    /// whole-stream digest, re-measured this session).
    yuv_md5: &'static str,
}

const LANES: [Lane; 5] = [
    Lane {
        name: "testsrc2_320x240_125f",
        frames: 125,
        size: (320, 240),
        yuv_md5: "7b5cbbc70b04132c8c74fcd07576fd48",
    },
    Lane {
        name: "testsrc2_640x480_g30_90f",
        frames: 90,
        size: (640, 480),
        yuv_md5: "356c84a819e60fcfc3bc73048de20c70",
    },
    Lane {
        name: "testsrc2_odd_126x94_75f",
        frames: 75,
        size: (126, 94),
        yuv_md5: "5d879407bc72e7ea105bc214c64c2256",
    },
    Lane {
        name: "corpus_reenc_vp8to9_480x360_41f",
        frames: 41,
        size: (480, 360),
        yuv_md5: "9621dcf981bfc3b52efa40c66c66a543",
    },
    Lane {
        name: "webm_remux_320x240_50f",
        frames: 50,
        size: (320, 240),
        yuv_md5: "88bb0b3ce56155bf0edb85bce4374ef4",
    },
];

/// The 10-bit profile-2 lane (record §D lane F: 50 frames, 320x240,
/// engine-level bit-exact, MD5 2391b4241617d81cf05d50efd54781d4 —
/// pinned at the engine seam by `tools/vta/vp9`; through THIS
/// wrapper it is the typed `vp9.bitdepth` refusal below).
const TENBIT_LANE: &str = "testsrc2_320x240_10bit_50f";

/// Collecting sink (frames in emission order).
#[derive(Default)]
struct Sink {
    frames: Vec<Frame>,
}

impl FrameSink for Sink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        self.frames.push(frame);
        Ok(())
    }
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/vp9")
        .join(format!("{name}.ivf"))
}

/// Demux one IVF fixture to packets (the landed demuxer, not a test
/// re-implementation — the full L2→L3 composition under test).
fn ivf_packets(name: &str) -> Vec<Packet> {
    let bytes = std::fs::read(fixture_path(name))
        .unwrap_or_else(|e| unreachable!("committed fixture {name}: {e}"));
    let mut demuxer = IvfDemuxer::new(mem_stream(bytes).must()).must();
    demuxer.start().must();
    let mut packets = Vec::new();
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(p)) => packets.push(p),
            Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => unreachable!("clean fixture must demux: {}", err.context.code),
        }
    }
    packets
}

/// One decoded run through the contract path (configure → every
/// packet → flush), returning the emitted frames and any typed
/// error codes.
fn decode_all(packets: &[Packet]) -> (Vec<Frame>, Vec<String>) {
    let generation = GenerationId::FIRST.next().must();
    let mut decoder = Vp9Decoder::new();
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new("vp9").must(),
            generation,
            ThreadBudget::default(),
            None,
            None,
        ))
        .must();
    let mut sink = Sink::default();
    let mut errors = Vec::new();
    for packet in packets {
        let mut one = packet.clone();
        one.generation = generation;
        if let Err(err) = decoder.decode(one, &mut sink) {
            errors.push(err.context.code.clone());
        }
    }
    let _ = decoder.flush(&mut sink);
    (sink.frames, errors)
}

/// The frame's I420 bytes (asserts the v0 output shape).
fn video_bytes(frame: &Frame) -> (u32, u32, Vec<u8>) {
    let Frame::Video(v) = frame else {
        unreachable!("video decoder emits video frames only");
    };
    let bytes = match &v.data {
        media_types::frame::FrameStorage::Cpu(b) => b.as_slice().to_vec(),
        media_types::frame::FrameStorage::OpaqueHandle(_) => {
            unreachable!("software decoder emits CPU frames only")
        }
    };
    (v.size.0, v.size.1, bytes)
}

/// One lane's decoded run: the emitted frames, any typed error
/// codes, and the demuxed packets.
type LaneRun = (Vec<Frame>, Vec<String>, Vec<Packet>);

/// Decode a lane once per process (the debug-build decode is the
/// expensive part; the assertions all share one run).
fn cached_run(name: &'static str) -> &'static LaneRun {
    static RUNS: OnceLock<HashMap<&'static str, LaneRun>> = OnceLock::new();
    RUNS.get_or_init(|| {
        let mut map = HashMap::new();
        let mut stage = |name: &'static str| {
            let packets = ivf_packets(name);
            let (frames, errors) = decode_all(&packets);
            map.insert(name, (frames, errors, packets));
        };
        for lane in LANES.iter() {
            stage(lane.name);
        }
        stage(TENBIT_LANE);
        map
    })
    .get(name)
    .unwrap_or_else(|| unreachable!("lane {name} was staged"))
}

/// Whether an `ffmpeg` binary is reachable (the live oracle).
fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// ffmpeg's own decode of the fixture as rawvideo yuv420p (the
/// oracle command family that produced the pinned constants).
fn ffmpeg_decode_yuv420p(name: &str) -> Vec<u8> {
    let out = std::process::Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-c:v",
            "libvpx-vp9",
            "-i",
        ])
        .arg(fixture_path(name))
        .args(["-f", "rawvideo", "-pix_fmt", "yuv420p"])
        .arg("-")
        .output()
        .must();
    if !out.status.success() {
        unreachable!(
            "ffmpeg failed on {name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    out.stdout
}

#[test]
fn differential_lanes_vs_ffmpeg_libvpx_vp9() {
    let live = ffmpeg_available();
    if !live {
        eprintln!("NOTE: no ffmpeg binary — pinning to the recorded constants");
    }
    for lane in LANES.iter() {
        let (frames, errors, packets) = cached_run(lane.name);
        // Geometry (no oracle needed).
        assert_eq!(
            packets.len() as u32,
            lane.frames,
            "{}: packet count",
            lane.name
        );
        assert!(
            errors.is_empty(),
            "{}: unexpected decode errors {errors:?}",
            lane.name
        );
        assert_eq!(
            frames.len() as u32,
            lane.frames,
            "{}: frame count (mine == ffprobe == record)",
            lane.name
        );
        let fsz = lane.size.0 as usize * lane.size.1 as usize * 3 / 2;
        let mut plane = Vec::with_capacity(fsz * lane.frames as usize);
        for (i, frame) in frames.iter().enumerate() {
            let (w, h, bytes) = video_bytes(frame);
            assert_eq!((w, h), lane.size, "{}: frame {i} size", lane.name);
            assert_eq!(bytes.len(), fsz, "{}: frame {i} I420 size", lane.name);
            // Presentation-time association: the engine carries the
            // coding packet's pts through to its frame (VP9 display
            // order == decode order on these lanes).
            let Frame::Video(v) = frame else {
                unreachable!("video frames only");
            };
            assert_eq!(
                v.pts, packets[i].pts,
                "{}: frame {i} pts must be the demuxer's stamp",
                lane.name,
            );
            plane.extend_from_slice(&bytes);
        }
        // Bit-exactness: whole-stream MD5, live oracle when present.
        let reference = if live {
            ffmpeg_decode_yuv420p(lane.name)
        } else {
            Vec::new()
        };
        let expected = if live {
            md5_hex(&reference)
        } else {
            lane.yuv_md5.to_string()
        };
        let ours = md5_hex(&plane);
        assert_eq!(
            ours, expected,
            "{}: pixel MD5 vs ffmpeg libvpx-vp9 dump",
            lane.name
        );
        // Per-frame identity (the record's per-lane metric): when
        // the live oracle ran, count frame-MD5-equal chunks.
        let mut identical = lane.frames as usize;
        if live {
            let ref_frames = reference.len() / fsz;
            assert_eq!(
                ref_frames, lane.frames as usize,
                "{}: ffmpeg's own frame count (window exactness cross-check)",
                lane.name
            );
            identical = (0..lane.frames as usize)
                .filter(|i| {
                    md5_hex(&plane[i * fsz..(i + 1) * fsz])
                        == md5_hex(&reference[i * fsz..(i + 1) * fsz])
                })
                .count();
            assert_eq!(
                identical, lane.frames as usize,
                "{}: every frame byte-identical (bit-exact class)",
                lane.name
            );
        }
        eprintln!(
            "{}: {} frames {}x{}, stream MD5 {ours} == record, {identical}/{} frames identical",
            lane.name, lane.frames, lane.size.0, lane.size.1, lane.frames,
        );
    }
}

#[test]
fn tenbit_lane_is_a_typed_refusal_not_a_misdecode() {
    let (frames, errors, packets) = cached_run(TENBIT_LANE);
    assert_eq!(packets.len(), 50, "lane F: packet count");
    // Every packet's 10-bit frame refuses at the v0 output boundary
    // — the av1 10-bit-fixture precedent: typed Unsupported, never
    // a silent skip, never a fabricated frame.
    assert!(
        errors.iter().all(|code| code == "vp9.bitdepth"),
        "lane F: every refusal is the bitdepth boundary, got {errors:?}"
    );
    assert_eq!(errors.len(), 50, "lane F: all 50 frames refused typed");
    assert!(
        frames.is_empty(),
        "lane F: no fabricated frames through the wrapper"
    );
    eprintln!(
        "lane F: 50/50 frames typed-refused vp9.bitdepth through the wrapper; \
         engine-level 10-bit bit-exactness (50/50, MD5 2391b4241617d81cf05d50efd54781d4) \
         is the admission record's measurement via tools/vta/vp9"
    );
}

#[test]
fn hostile_garbage_is_recovered_and_the_stream_survives() {
    // The record's §H garbage-then-real lane, through the wrapper:
    // a leading garbage packet is Recovered (never fatal), and the
    // real keyframe stream behind it decodes in full.
    let generation = GenerationId::FIRST.next().must();
    let mut decoder = Vp9Decoder::new();
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new("vp9").must(),
            generation,
            ThreadBudget::default(),
            None,
            None,
        ))
        .must();
    let mut sink = Sink::default();
    let garbage: Vec<u8> = (0..64u32)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
        .collect();
    let lead = Packet {
        track: media_types::TrackId::new(0),
        pts: media_types::Stamp::Missing,
        dts: media_types::Stamp::Missing,
        duration: None,
        data: media_types::packet::SharedBytes::from_vec(garbage),
        keyframe: true,
        discontinuity: false,
        generation,
    };
    let outcome = decoder.decode(lead, &mut sink).must();
    assert!(
        matches!(
            outcome,
            DecodeOutcome::Recovered {
                reason: RecoverReason::CorruptInput,
                ..
            }
        ),
        "garbage counts damage: {outcome:?}"
    );
    let (_, _, packets) = cached_run(LANES[0].name);
    for packet in packets {
        let mut one = packet.clone();
        one.generation = generation;
        let _ = decoder.decode(one, &mut sink).must();
    }
    let _ = decoder.flush(&mut sink).must();
    assert_eq!(
        sink.frames.len(),
        LANES[0].frames as usize,
        "garbage-then-real: the real stream decodes in full after recovery"
    );
}

#[test]
fn off_by_one_truncation_sweep_is_panic_free() {
    // The record's §H sweep, through the wrapper: every packet of
    // lane A truncated by one byte, decoded alone — every outcome is
    // a typed Ok (some decode: VP9's boolean decoder end-fills like
    // libvpx's concealment, the record's honest note), none panics
    // (a panic here fails the test by construction).
    let (_, _, packets) = cached_run(LANES[0].name);
    let generation = GenerationId::FIRST.next().must();
    let mut decoded_any = 0usize;
    let mut recovered = 0usize;
    for (i, packet) in packets.iter().enumerate() {
        let mut decoder = Vp9Decoder::new();
        decoder
            .configure(&DecoderConfig::new(
                CodecDescriptor::new("vp9").must(),
                generation,
                ThreadBudget::default(),
                None,
                None,
            ))
            .must();
        let mut sink = Sink::default();
        let data = packet.data.as_slice();
        if data.is_empty() {
            continue;
        }
        let mut cut = packet.clone();
        cut.generation = generation;
        cut.data = media_types::packet::SharedBytes::from_vec(data[..data.len() - 1].to_vec());
        match decoder.decode(cut, &mut sink) {
            Ok(DecodeOutcome::Emitted { .. }) => decoded_any += 1,
            Ok(DecodeOutcome::Recovered { .. }) => recovered += 1,
            Ok(DecodeOutcome::NeedMoreInput) | Ok(DecodeOutcome::Drained) => {}
            Err(err) => unreachable!("truncated packet {i} must be typed-Ok, got {err}"),
        }
    }
    eprintln!(
        "off-by-one sweep (lane A, {} packets): 0 panics, {decoded_any} concealed-decode, \
         {recovered} recovered",
        packets.len()
    );
}
