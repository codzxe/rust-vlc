//! H.264 software-decoder differential suite — the admission's
//! binding evidence (the `rusty_vp9` lane pattern, applied to
//! [`codecs::video::h264::H264SwDecoder`]): six ffmpeg-authored
//! lanes, decoded through the **wired** decoder on BOTH entry
//! forms — Annex-B elementary stream (the real `demux::es` h264
//! demuxer, in-band parameter sets) and MP4/AVCC samples (the real
//! `demux::mp4` demuxer, `avcC` `extra_data` seeding) — and
//! compared against the ffmpeg 7.1.5 `libx264` oracle.
//!
//! The lanes (libx264, testsrc2, 30 fps, 8-bit 4:2:0):
//!
//! | lane | profile | frames | size | notes |
//! |------|---------|--------|------|-------|
//! | A | Constrained Baseline (CAVLC, I/P) | 45 | 320x240 | the simplest real shape |
//! | B | Main (CABAC, I/P) | 45 | 320x240 | entropy-coding switch |
//! | C | Main + B-frames (bf=2) | 45 | 320x240 | display ≠ decode order; the PTS metas must survive reordering |
//! | D | High + B-frames | 45 | 640x360 | 8x8 transforms, weighted pred |
//! | E | Constrained Baseline, odd ask 127x95 | 30 | 126x94 | SPS crop (coded 128x96 → display 126x94) |
//! | F | single-IDR still | 1 | 320x240 | the flush-drain boundary |
//!
//! The ES and MP4 twins of every lane are the SAME bitstream (the
//! mp4 was encoded natively; the ES extracted with
//! `-bsf:v h264_mp4toannexb`), and ffmpeg decodes both byte-identically
//! (measured this session) — so both entry forms pin the SAME MD5
//! constant, re-verified live when ffmpeg is on PATH (the pinned
//! constants are that same command's output).
//!
//! The hostile lanes at the bottom (the `rusty_vp9` §H battery
//! through the wrapper): garbage packets are `Recovered`, the
//! off-by-one truncation sweep is panic-free, a garbage-then-real
//! stream recovers, and the contract's state rules (unconfigured
//! decode/flush, wrong descriptor) answer typed — never a panic.

mod common;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use codec_api::prelude::*;
use codecs::video::h264::H264SwDecoder;
use common::md5_hex;
use demux::mp4::Mp4Demuxer;
use media_types::frame::{Frame, FrameStorage};
use media_types::ids::{CodecDescriptor, MediaKind, Track};
use media_types::{GenerationId, Packet};
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

/// One lane's expected truth (ffmpeg 7.1.5.42 re-measured this
/// session: `ffprobe -count_frames` + `ffmpeg -f rawvideo
/// -pix_fmt yuv420p` decode MD5s, identical on both entry forms).
struct Lane {
    /// Fixture stem under `tests/fixtures/h264/`.
    name: &'static str,
    /// ffprobe `nb_read_frames`.
    frames: u32,
    /// ffprobe `width`/`height` (the cropped display size).
    size: (u32, u32),
    /// MD5 of ffmpeg's decoded yuv420p dump (both entry forms).
    yuv_md5: &'static str,
}

const LANES: [Lane; 6] = [
    Lane {
        name: "A_baseline_ip_320x240_45f",
        frames: 45,
        size: (320, 240),
        yuv_md5: "a0a9144b154d0403aa38429a85adce56",
    },
    Lane {
        name: "B_main_cabac_ip_320x240_45f",
        frames: 45,
        size: (320, 240),
        yuv_md5: "19520a80eb3f586eca053d04ae90b129",
    },
    Lane {
        name: "C_main_bframes_320x240_45f",
        frames: 45,
        size: (320, 240),
        yuv_md5: "532c57170f56fcf4d20243b1242b88d3",
    },
    Lane {
        name: "D_high_ipb_640x360_45f",
        frames: 45,
        size: (640, 360),
        yuv_md5: "412bd350f062af2e35cffb45cfec7f9b",
    },
    Lane {
        name: "E_odd_127x95_30f",
        frames: 30,
        size: (126, 94),
        yuv_md5: "d0fc84c9dc83f3df47e725db6d82c40f",
    },
    Lane {
        name: "F_still_1f_320x240",
        frames: 1,
        size: (320, 240),
        yuv_md5: "810d6f44c4f4e3b9eae14fd9aece2a76",
    },
];

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

fn fixture_path(name: &str, ext: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/h264")
        .join(format!("{name}.{ext}"))
}

/// A memory source stream (the demux crate's own test helper).
fn mem_stream(bytes: Vec<u8>) -> Box<dyn plugin_api::SourceStream> {
    demux::avi::test_support::mem_stream(bytes).must()
}

/// Demux the Annex-B ES lane to packets (the real `demux::es` h264
/// demuxer — AU-aware boundary detection, in-band parameter sets,
/// no test-side framing).
fn es_packets(name: &str) -> Vec<Packet> {
    let bytes = std::fs::read(fixture_path(name, "h264"))
        .unwrap_or_else(|e| unreachable!("committed fixture {name}: {e}"));
    let mut demuxer = demux::es::h264::open(mem_stream(bytes)).must();
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

/// One ISO-BMFF child box header (declared size, fourcc) at `pos`,
/// `None` past the entry's end (the CLI host's `mp4.rs` walker,
/// mirrored test-side).
fn child_header(entry: &[u8], pos: usize) -> Option<(usize, [u8; 4])> {
    let header = entry.get(pos..pos + 8)?;
    let size = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
    let kind = [header[4], header[5], header[6], header[7]];
    Some((size, kind))
}

/// Find a child box payload of a sample entry (the CLI host's
/// `find_child_box` convention: the Audio/Visual sample-entry fixed
/// prefixes 28 / 76 / 78 — a wrong prefix walks no well-formed child
/// and falls through).
fn find_child_box<'a>(entry: &'a [u8], magic: &[u8; 4]) -> Option<&'a [u8]> {
    for start in [28usize, 76, 78] {
        if entry.len() < start + 8 {
            continue;
        }
        let mut pos = start;
        while let Some((size, kind)) = child_header(entry, pos) {
            if kind == *magic {
                return entry.get(pos + 8..pos + size);
            }
            if size < 8 {
                break;
            }
            pos += size;
        }
    }
    None
}

/// Demux the MP4 lane to (packets, avcC `extra_data`) — the real
/// `demux::mp4` demuxer; the `avcC` record is harvested from the
/// video track's first sample entry exactly as the CLI host does
/// (the `config_blob` convention).
fn mp4_packets(name: &str) -> (Vec<Packet>, Option<SharedBytes>) {
    let bytes = std::fs::read(fixture_path(name, "mp4"))
        .unwrap_or_else(|e| unreachable!("committed fixture {name}: {e}"));
    let mut demuxer = Mp4Demuxer::new(mem_stream(bytes)).must();
    let tracks = demuxer.start().must();
    let video: &Track = tracks
        .iter()
        .find(|t| t.kind == MediaKind::Video)
        .unwrap_or_else(|| unreachable!("fixture must carry a video track"));
    let extra = demuxer
        .sample_entry_bytes(video.id)
        .and_then(|entry| find_child_box(entry, b"avcC"))
        .map(|body| SharedBytes::from_vec(body.to_vec()));
    let mut packets = Vec::new();
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(p)) if p.track == video.id => packets.push(p),
            Ok(DemuxEvent::Packet(_)) => {}
            Ok(DemuxEvent::Recovered { .. }) | Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
            Err(err) => unreachable!("clean fixture must demux: {}", err.context.code),
        }
    }
    (packets, extra)
}

/// One decoded run through the contract path (configure → every
/// packet → flush), returning the emitted frames and any typed
/// error codes.
fn decode_all(packets: &[Packet], extra: Option<SharedBytes>) -> (Vec<Frame>, Vec<String>) {
    let generation = GenerationId::FIRST.next().must();
    let mut decoder = H264SwDecoder::new();
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new("h264").must(),
            generation,
            ThreadBudget::default(),
            None,
            extra,
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
        FrameStorage::Cpu(b) => b.as_slice().to_vec(),
        FrameStorage::OpaqueHandle(_) => {
            unreachable!("software decoder emits CPU frames only")
        }
    };
    (v.size.0, v.size.1, bytes)
}

/// ffmpeg's own decode of a fixture, as raw yuv420p bytes (live
/// oracle; `None` when ffmpeg is not on PATH).
fn ffmpeg_reference(relative: &PathBuf) -> Option<Vec<u8>> {
    let status = Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    let out = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(relative)
        .args(["-f", "rawvideo", "-pix_fmt", "yuv420p", "-"])
        .output()
        .ok()?;
    Some(out.stdout)
}

/// One entry form's decoded run, cached per process (the debug-build
/// decode is the expensive part; the assertions all share one run).
enum Form {
    Es,
    Mp4,
}

/// The cached lane runs: (name, form) → (frames, typed errors).
type LaneRuns = HashMap<(&'static str, &'static str), (Vec<Frame>, Vec<String>)>;

fn cached_run(name: &'static str, form: Form) -> &'static (Vec<Frame>, Vec<String>) {
    static RUNS: OnceLock<LaneRuns> = OnceLock::new();
    let key = match form {
        Form::Es => "es",
        Form::Mp4 => "mp4",
    };
    RUNS.get_or_init(|| {
        let mut map = HashMap::new();
        for lane in LANES.iter() {
            let es = es_packets(lane.name);
            let (frames, errors) = decode_all(&es, None);
            map.insert((lane.name, "es"), (frames, errors));
            let (mp4, extra) = mp4_packets(lane.name);
            let (frames, errors) = decode_all(&mp4, extra);
            map.insert((lane.name, "mp4"), (frames, errors));
        }
        map
    })
    .get(&(name, key))
    .unwrap_or_else(|| unreachable!("lane {name} was staged"))
}

/// The shared lane body: frame count, per-frame size + format, the
/// concatenated tight-I420 MD5 vs the pinned constant, zero typed
/// errors, and — when ffmpeg is on PATH — the live-oracle re-check
/// (our dump == ffmpeg's dump, byte-identical).
fn assert_lane(name: &'static str, form: Form, ext: &str) {
    let lane = LANES.iter().find(|l| l.name == name).must();
    let (frames, errors) = cached_run(name, form);
    assert!(
        errors.is_empty(),
        "lane {name} ({ext}) must decode without typed errors: {errors:?}"
    );
    assert_eq!(
        frames.len(),
        lane.frames as usize,
        "lane {name} ({ext}): frame count"
    );
    let mut dump = Vec::new();
    for frame in frames {
        let (w, h, bytes) = video_bytes(frame);
        assert_eq!((w, h), lane.size, "lane {name} ({ext}): frame size");
        let Frame::Video(v) = frame else {
            unreachable!("checked in video_bytes");
        };
        assert_eq!(
            v.format,
            media_types::PixelFormat::I420,
            "lane {name} ({ext}): output format"
        );
        dump.extend_from_slice(&bytes);
    }
    assert_eq!(
        md5_hex(&dump),
        lane.yuv_md5,
        "lane {name} ({ext}): the bit-exact oracle (ffmpeg 7.1.5)"
    );
    // The live re-check (skipped loudly when ffmpeg is absent).
    if let Some(reference) = ffmpeg_reference(&fixture_path(name, ext)) {
        assert_eq!(
            md5_hex(&dump),
            md5_hex(&reference),
            "lane {name} ({ext}): live ffmpeg oracle re-check"
        );
        assert_eq!(
            md5_hex(&reference),
            lane.yuv_md5,
            "lane {name} ({ext}): the pinned constant vs live ffmpeg (re-pin!)"
        );
    } else {
        eprintln!("SKIP live-oracle re-check for {name} ({ext}): ffmpeg not on PATH");
    }
}

#[test]
fn lane_a_baseline_annexb() {
    assert_lane("A_baseline_ip_320x240_45f", Form::Es, "h264");
}

#[test]
fn lane_a_baseline_mp4() {
    assert_lane("A_baseline_ip_320x240_45f", Form::Mp4, "mp4");
}

#[test]
fn lane_b_main_cabac_annexb() {
    assert_lane("B_main_cabac_ip_320x240_45f", Form::Es, "h264");
}

#[test]
fn lane_b_main_cabac_mp4() {
    assert_lane("B_main_cabac_ip_320x240_45f", Form::Mp4, "mp4");
}

#[test]
fn lane_c_bframes_annexb() {
    // The B-frame lane: display order must be POC-sorted. Raw ES
    // files carry no timestamps (the demuxer's documented
    // `Stamp::Missing` posture), so the frames echo Missing stamps —
    // the pts-ladder assertion belongs to the MP4 twin.
    assert_lane("C_main_bframes_320x240_45f", Form::Es, "h264");
    let (frames, errors) = cached_run("C_main_bframes_320x240_45f", Form::Es);
    assert!(errors.is_empty(), "{errors:?}");
    for frame in frames {
        let Frame::Video(v) = frame else {
            unreachable!("video frames only");
        };
        assert!(matches!(v.pts, media_types::Stamp::Missing));
    }
}

#[test]
fn lane_c_bframes_mp4() {
    assert_lane("C_main_bframes_320x240_45f", Form::Mp4, "mp4");
    // MP4 pts are the presentation times: display order ⇒ strictly
    // increasing.
    let (frames, errors) = cached_run("C_main_bframes_320x240_45f", Form::Mp4);
    assert!(errors.is_empty(), "{errors:?}");
    let mut last_nanos: Option<u128> = None;
    for frame in frames {
        let Frame::Video(v) = frame else {
            unreachable!("video frames only");
        };
        if let Ok(ts) = v.pts.timestamp() {
            let nanos = ts.as_nanos().must().0 as u128;
            if let Some(prev) = last_nanos {
                assert!(nanos > prev, "mp4 display-order pts: {nanos} after {prev}");
            }
            last_nanos = Some(nanos);
        }
    }
    assert!(last_nanos.is_some(), "the lane carries pts stamps");
}

#[test]
fn lane_d_high_ipb_annexb() {
    assert_lane("D_high_ipb_640x360_45f", Form::Es, "h264");
}

#[test]
fn lane_d_high_ipb_mp4() {
    assert_lane("D_high_ipb_640x360_45f", Form::Mp4, "mp4");
}

#[test]
fn lane_e_odd_crop_annexb() {
    assert_lane("E_odd_127x95_30f", Form::Es, "h264");
}

#[test]
fn lane_e_odd_crop_mp4() {
    assert_lane("E_odd_127x95_30f", Form::Mp4, "mp4");
}

#[test]
fn lane_f_still_annexb() {
    // The single-frame boundary: the picture finalizes at flush.
    assert_lane("F_still_1f_320x240", Form::Es, "h264");
}

#[test]
fn lane_f_still_mp4() {
    assert_lane("F_still_1f_320x240", Form::Mp4, "mp4");
}

// ---------------------------------------------------------------------------
// The hostile battery (admission condition: panic-free on any byte
// string, damage counted, recovery after garbage).
// ---------------------------------------------------------------------------

/// A synthetic packet of `bytes` on the video track.
fn raw_packet(bytes: Vec<u8>) -> Packet {
    Packet {
        track: media_types::TrackId::new(0),
        pts: media_types::Stamp::Missing,
        dts: media_types::Stamp::Missing,
        duration: None,
        data: SharedBytes::from_vec(bytes),
        keyframe: false,
        discontinuity: false,
        generation: GenerationId::FIRST,
    }
}

fn configured() -> (H264SwDecoder, GenerationId) {
    let generation = GenerationId::FIRST.next().must();
    let mut decoder = H264SwDecoder::new();
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new("h264").must(),
            generation,
            ThreadBudget::default(),
            None,
            None,
        ))
        .must();
    (decoder, generation)
}

/// Garbage packets must not panic, must count damage, and must
/// leave the decoder usable (`Recovered` or `NeedMoreInput`, never
/// a typed `Err` — corrupt input is recovery, not failure).
#[test]
fn hostile_garbage_packets_recover() {
    let (mut decoder, generation) = configured();
    let mut sink = Sink::default();
    // Deterministic pseudo-random garbage (no RNG dependency).
    let mut state = 0x9E3779B97F4A7C15u64;
    let mut garbage = |n: usize| {
        let mut bytes = Vec::with_capacity(n);
        for _ in 0..n {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            bytes.push((state >> 33) as u8);
        }
        bytes
    };
    for size in [0, 1, 2, 3, 7, 64, 1000] {
        let mut packet = raw_packet(garbage(size));
        packet.generation = generation;
        match decoder.decode(packet, &mut sink) {
            Ok(DecodeOutcome::NeedMoreInput) => {}
            Ok(DecodeOutcome::Recovered { .. }) => {}
            other => unreachable!("garbage must recover, got {other:?}"),
        }
    }
    // Zero-length annexb packets: nothing to see.
    let mut packet = raw_packet(Vec::new());
    packet.generation = generation;
    let outcome = decoder.decode(packet, &mut sink).must();
    assert!(matches!(outcome, DecodeOutcome::NeedMoreInput));
    assert!(sink.frames.is_empty());
}

/// The off-by-one truncation sweep: every truncation of a real
/// packet must be panic-free (damage counted where it lands).
#[test]
fn hostile_truncation_sweep_is_panic_free() {
    let packets = es_packets("A_baseline_ip_320x240_45f");
    let first = packets.iter().find(|p| p.data.len() > 64).must();
    let bytes = first.data.as_slice();
    let (mut decoder, generation) = configured();
    let mut sink = Sink::default();
    for cut in 0..bytes.len() {
        let mut packet = raw_packet(bytes[..cut].to_vec());
        packet.generation = generation;
        // Any outcome is fine EXCEPT a panic or a process abort; the
        // typed `Unsupported` family is impossible on this lane
        // (baseline), so assert the recovery vocabulary.
        let outcome = decoder.decode(packet, &mut sink);
        assert!(
            outcome.is_ok()
                || matches!(
                    outcome,
                    Err(ref e) if e.context.code == "h264.sw.avc.length_size"
                ),
            "truncation at {cut}: {outcome:?}"
        );
    }
}

/// A garbage-then-real stream must recover: after leading garbage,
/// the real packets still decode to the pinned lane truth.
#[test]
fn hostile_garbage_then_real_recovers() {
    let packets = es_packets("A_baseline_ip_320x240_45f");
    let (frames, errors) = {
        let (mut decoder, generation) = configured();
        let mut sink = Sink::default();
        let mut garbage = raw_packet(vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x00, 0x01, 0xFF]);
        garbage.generation = generation;
        let _ = decoder.decode(garbage, &mut sink).must();
        for packet in &packets {
            let mut one = packet.clone();
            one.generation = generation;
            let _ = decoder.decode(one, &mut sink).must();
        }
        let _ = decoder.flush(&mut sink);
        (sink.frames, Vec::<String>::new())
    };
    assert!(errors.is_empty());
    assert_eq!(frames.len(), 45, "all 45 frames after leading garbage");
    let mut dump = Vec::new();
    for frame in &frames {
        let (_, _, bytes) = video_bytes(frame);
        dump.extend_from_slice(&bytes);
    }
    assert_eq!(md5_hex(&dump), "a0a9144b154d0403aa38429a85adce56");
}

/// The contract's state rules: decode/flush before configure is the
/// typed `BadState`, and a wrong descriptor refuses typed.
#[test]
fn contract_state_rules() {
    let mut decoder = H264SwDecoder::new();
    let (mut sink, packet) = (Sink::default(), raw_packet(Vec::new()));
    let err = decoder.decode(packet, &mut sink).err().must();
    assert_eq!(err.context.code, "h264.sw.not_configured");
    let err = decoder.flush(&mut sink).err().must();
    assert_eq!(err.context.code, "h264.sw.not_configured");

    let generation = GenerationId::FIRST.next().must();
    let err = decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new("mpeg4").must(),
            generation,
            ThreadBudget::default(),
            None,
            None,
        ))
        .err()
        .must();
    assert_eq!(err.context.code, "h264.sw.codec_descriptor");

    // A stale packet is dropped on sight (NeedMoreInput, zero frames).
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new("h264").must(),
            generation,
            ThreadBudget::default(),
            None,
            None,
        ))
        .must();
    let mut stale = raw_packet(Vec::new());
    stale.generation = GenerationId::FIRST;
    let outcome = decoder.decode(stale, &mut sink).must();
    assert!(matches!(outcome, DecodeOutcome::NeedMoreInput));
    // Flush on a fresh generation with nothing buffered: Drained.
    let outcome = decoder.flush(&mut sink).must();
    assert!(matches!(outcome, DecodeOutcome::Drained));
}

/// Damaged `avcC` extradata at configure is a typed error (the
/// stream is not decodable as configured), and the decoder accepts a
/// correct re-configure afterwards (generation reset).
#[test]
fn contract_damaged_extradata_refuses_typed() {
    let generation = GenerationId::FIRST.next().must();
    let mut decoder = H264SwDecoder::new();
    // `looks_like_avcc` true (version 1, reserved bits, one SPS) but
    // the SPS length runs past the record: truncated mid-SPS.
    let damaged = [0x01, 0x64, 0x00, 0x1E, 0xFF, 0xE1, 0x00, 0x40, 0x00];
    let err = decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new("h264").must(),
            generation,
            ThreadBudget::default(),
            None,
            Some(SharedBytes::from_vec(damaged.to_vec())),
        ))
        .err()
        .must();
    assert!(
        err.context.code.starts_with("h264.sw.extradata"),
        "typed extradata refusal, got {}",
        err.context.code
    );
    // Recovery: the same decoder configures cleanly without it.
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new("h264").must(),
            generation,
            ThreadBudget::default(),
            None,
            None,
        ))
        .must();
}
