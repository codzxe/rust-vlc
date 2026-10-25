//! AV1 differential suite (card 5.E1 AC): real ffmpeg-authored
//! AV1-in-IVF fixtures, demuxed by the landed `demux::ivf` and
//! decoded by the rav1d-backed [`codecs::video::av1::Av1Decoder`] —
//! asserted against **ffprobe/ffmpeg truth**.
//!
//! The chain per fixture (commands recorded in the worklog, all run
//! 2026-10-26, ffmpeg 7.1.5-ish sandbox build):
//!
//! 1. `ffprobe` stream facts: codec `av1`, width/height, frame
//!    count — the decoder must emit exactly that many frames at
//!    exactly those dimensions;
//! 2. packet pts ladder: IVF frames are pts 0..n−1 in order — the
//!    decoder's presentation-time association must restore exactly
//!    that ladder (the `Dav1dDataProps::timestamp` → pending-map
//!    design in `video/av1/decoder.rs`);
//! 3. **pixel bit-exactness**: our concatenated I420 output MD5 ==
//!    ffmpeg's own `ffmpeg -i f.ivf -f rawvideo -pix_fmt yuv420p`
//!    dump MD5 (computed live when ffmpeg is on PATH; the recorded
//!    constants are the same command's output, so the pinned values
//!    and the live oracle are one and the same);
//! 4. the 10-bit fixture pins the v0 output boundary: a typed
//!    `av1.bitdepth` `Unsupported` refusal — never a mis-decode,
//!    never silence.
//!
//! The fuzz-lite `+FB` sweep lives at the bottom: random and mutated
//! inputs (< 200 KB each) through the OBU framer **and** the full
//! decode path; the invariant is "typed outcome or Ok, zero panics".
//!
//! Fixture provenance: generated this session with
//! `ffmpeg -f lavfi -i <src> -c:v libaom-av1 -b:v 0 -crf <n>
//! [-cpu-used 8 -row-mt 1] [-g 6] -pix_fmt yuv420p[10le] -f ivf`
//! (≈39 KB total — the crate's fixture budget convention; the g6
//! variant forces an independent frame every 6 packets for the
//! mid-stream resync proof).

mod common;

use codec_api::prelude::*;
use codecs::video::av1::Av1Decoder;
use common::md5_hex;
use demux::ivf::IvfDemuxer;
use demux::ivf::test_support::mem_stream;
use media_types::frame::Frame;
use media_types::ids::CodecDescriptor;
use media_types::{GenerationId, Packet};
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

/// One fixture's expected truth (ffprobe + ffmpeg, recorded above).
struct Truth {
    /// Fixture name under `tests/fixtures/av1/`.
    name: &'static str,
    /// ffprobe `nb_frames`.
    frames: u32,
    /// ffprobe `width`/`height`.
    size: (u32, u32),
    /// MD5 of ffmpeg's decoded yuv420p dump (live-recomputed when
    /// ffmpeg is available; pinned otherwise).
    yuv_md5: &'static str,
}

const FIXTURES: [Truth; 3] = [
    Truth {
        name: "testsrc2_320x240_60f",
        frames: 60,
        size: (320, 240),
        yuv_md5: "275082414fa880cc72a4c52f71875953",
    },
    Truth {
        name: "smpte_160x128_18f",
        frames: 18,
        size: (160, 128),
        yuv_md5: "fbedc17b688665d981b917380e83e2c1",
    },
    Truth {
        name: "rgb_192x108_6f",
        frames: 6,
        size: (192, 108),
        yuv_md5: "f153f1d34f1c8828a8c061fefc100d74",
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

/// Demux one IVF fixture to packets (the landed demuxer, not a test
/// re-implementation — the full L2→L3 composition under test).
fn ivf_packets(name: &str) -> Vec<Packet> {
    let bytes = std::fs::read(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/av1")
            .join(format!("{name}.ivf")),
    )
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

/// Configure + decode a whole packet run through the contract path.
fn decode_all(packets: &[Packet]) -> (Vec<Frame>, Vec<String>) {
    let generation = GenerationId::FIRST.next().must();
    let mut decoder = Av1Decoder::new();
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new("av1").must(),
            generation,
            ThreadBudget::default(),
            None,
            None,
        ))
        .must();
    let mut sink = Sink::default();
    let mut errors = Vec::new();
    for packet in packets {
        let mut one = Packet {
            track: packet.track,
            pts: packet.pts,
            dts: packet.dts,
            duration: packet.duration,
            data: packet.data.clone(),
            keyframe: packet.keyframe,
            discontinuity: packet.discontinuity,
            generation,
        };
        one.generation = generation;
        if let Err(err) = decoder.decode(one, &mut sink) {
            errors.push(err.context.code.clone());
        }
    }
    // Drain the reordering delay.
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

#[test]
fn differential_frame_count_dimensions_and_pts_vs_ffprobe() {
    for truth in FIXTURES.iter() {
        let packets = ivf_packets(truth.name);
        assert_eq!(
            packets.len() as u32,
            truth.frames,
            "{}: packet count",
            truth.name
        );
        let (frames, errors) = decode_all(&packets);
        assert!(
            errors.is_empty(),
            "{}: unexpected decode errors {errors:?}",
            truth.name
        );
        assert_eq!(
            frames.len() as u32,
            truth.frames,
            "{}: frame count",
            truth.name
        );
        for (i, frame) in frames.iter().enumerate() {
            let (w, h, bytes) = video_bytes(frame);
            assert_eq!((w, h), truth.size, "{}: frame {i} size", truth.name);
            let expected = w as usize * h as usize * 3 / 2;
            assert_eq!(bytes.len(), expected, "{}: frame {i} I420 size", truth.name);
            // Presentation-time association: the decoder must
            // restore the demuxer's own stamp for the frame's TU
            // (frames and packets pair 1:1 in these streams, in
            // order, after the flush drain).
            let Frame::Video(v) = frame else {
                unreachable!()
            };
            assert_eq!(
                v.pts, packets[i].pts,
                "{}: frame {i} pts must be the demuxer's stamp",
                truth.name,
            );
        }
    }
}

#[test]
fn differential_pixel_md5_against_ffmpeg_dump() {
    let ffmpeg = std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    for truth in FIXTURES.iter() {
        let packets = ivf_packets(truth.name);
        let (frames, _) = decode_all(&packets);
        let mut plane = Vec::new();
        for frame in &frames {
            let (_, _, bytes) = video_bytes(frame);
            plane.extend_from_slice(&bytes);
        }
        let ours = md5_hex(&plane);
        // Live oracle when ffmpeg is present: same command family
        // that produced the pinned constant.
        let expected = if ffmpeg {
            let out = std::process::Command::new("ffmpeg")
                .args(["-hide_banner", "-loglevel", "error", "-y"])
                .arg("-i")
                .arg(
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/fixtures/av1")
                        .join(format!("{}.ivf", truth.name)),
                )
                .args(["-f", "rawvideo", "-pix_fmt", "yuv420p"])
                .arg("-")
                .output()
                .must();
            md5_hex(&out.stdout)
        } else {
            truth.yuv_md5.to_string()
        };
        assert_eq!(ours, expected, "{}: pixel MD5 vs ffmpeg dump", truth.name);
    }
}

#[test]
fn tenbit_fixture_is_a_typed_refusal_not_a_misdecode() {
    let packets = ivf_packets("tenbit_128x96_5f");
    assert_eq!(packets.len(), 5);
    let generation = GenerationId::FIRST.next().must();
    let mut decoder = Av1Decoder::new();
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new("av1").must(),
            generation,
            ThreadBudget::default(),
            None,
            None,
        ))
        .must();
    let mut sink = Sink::default();
    let mut refused = 0usize;
    let mut first_code = String::new();
    for packet in packets {
        let mut one = packet.clone();
        one.generation = generation;
        match decoder.decode(one, &mut sink) {
            Ok(_) => {}
            Err(err) => {
                if first_code.is_empty() {
                    first_code = err.context.code.clone();
                }
                refused += 1;
                assert!(
                    matches!(err.kind, media_types::ErrorKind::Unsupported { .. }),
                    "10-bit refusal must be Unsupported, got {:?}",
                    err.kind
                );
            }
        }
    }
    assert_eq!(first_code, "av1.bitdepth", "the v0 boundary site");
    assert_eq!(refused, 5, "every 10-bit TU refuses, none silently decodes");
    assert!(sink.frames.is_empty(), "no fabricated frames");
}

#[test]
fn error_paths_are_typed_and_per_packet() {
    let generation = GenerationId::FIRST.next().must();

    // Not configured: typed BadState, never a panic.
    let mut virgin = Av1Decoder::new();
    let packet = Packet {
        track: media_types::TrackId::new(0),
        pts: media_types::Stamp::Missing,
        dts: media_types::Stamp::Missing,
        duration: None,
        data: media_types::packet::SharedBytes::from_vec(vec![0x12]),
        keyframe: true,
        discontinuity: false,
        generation,
    };
    let err = match virgin.decode(packet, &mut Sink::default()) {
        Ok(_) => unreachable!("unconfigured decode must fail typed"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "av1.not_configured");

    // Wrong descriptor: typed codec refusal (the doctest pins the
    // same site; this pins it through the contract call shape).
    let mut decoder = Av1Decoder::new();
    let refused = decoder.configure(&DecoderConfig::new(
        CodecDescriptor::new("vp9").must(),
        generation,
        ThreadBudget::default(),
        None,
        None,
    ));
    assert!(refused.is_err());

    // Bad OBU (forbidden bit) on a configured decoder: typed
    // av1.obu.* Corrupt, per-packet.
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new("av1").must(),
            generation,
            ThreadBudget::default(),
            None,
            None,
        ))
        .must();
    let bad = Packet {
        track: media_types::TrackId::new(0),
        pts: media_types::Stamp::Missing,
        dts: media_types::Stamp::Missing,
        duration: None,
        data: media_types::packet::SharedBytes::from_vec(vec![0x80, 0x00, 0x00]),
        keyframe: true,
        discontinuity: false,
        generation,
    };
    let err = match decoder.decode(bad, &mut Sink::default()) {
        Ok(_) => unreachable!("forbidden-bit OBU must fail typed"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "av1.obu.forbidden_bit");

    // Truncated TU: a sized FRAME OBU (6<<3 | has_size 0x02) whose
    // leb128 declares 32 payload bytes but only 2 remain.
    let truncated = Packet {
        track: media_types::TrackId::new(0),
        pts: media_types::Stamp::Missing,
        dts: media_types::Stamp::Missing,
        duration: None,
        data: media_types::packet::SharedBytes::from_vec(vec![0x32, 0x20, 0x01, 0x02]),
        keyframe: true,
        discontinuity: false,
        generation,
    };
    let err = match decoder.decode(truncated, &mut Sink::default()) {
        Ok(_) => unreachable!("truncated TU must fail typed"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "av1.obu.payload_truncated");

    // Empty packet: typed, not a stall.
    let empty = Packet {
        track: media_types::TrackId::new(0),
        pts: media_types::Stamp::Missing,
        dts: media_types::Stamp::Missing,
        duration: None,
        data: media_types::packet::SharedBytes::from_vec(Vec::new()),
        keyframe: true,
        discontinuity: false,
        generation,
    };
    let err = match decoder.decode(empty, &mut Sink::default()) {
        Ok(_) => unreachable!("empty packet must fail typed"),
        Err(err) => err,
    };
    assert_eq!(err.context.code, "av1.obu.empty_tu");

    // After all that damage the decoder still accepts a clean stream
    // end-to-end (per-frame isolation, the doc 03 §4.5(d) contract).
    let packets = ivf_packets("rgb_192x108_6f");
    let (frames, errors) = decode_all(&packets);
    assert!(errors.is_empty());
    assert_eq!(frames.len(), 6);

    // Decoder-init-failure site (`av1.init.engine`): unreachable
    // through a valid `DecoderConfig` by construction —
    // `ThreadBudget` validates 1..=256 and rav1d accepts 0..=256, so
    // no open-argument combination can fail. The site is pinned here
    // by name; its construction is code-reviewed, not
    // fault-injected. [recorded honestly, 5.E1]
}

#[test]
fn midstream_corruption_resyncs_and_recovers() {
    // The g6 fixture (keyframes every 6 frames — verified at
    // generation: independent frames at packets 0, 6, 12) proves the
    // FULL recovery contract:
    //
    // 1. packet 9's OBU framing is damaged (forbidden bit) → typed
    //    Corrupt refusal, the engine never sees the TU;
    // 2. the reference-state gate (decoder module docs — the rav1d
    //    1.1.0 missing-reference abort containment) skips packets
    //    10-11 as `Recovered { Discontinuity }` — their references
    //    chain through the dropped frame;
    // 3. packet 12 is an independent frame → the gate reopens and
    //    decoding RESUMES: packets 12-17 decode cleanly.
    //
    // Net: 15 of 18 frames survive (0-8 and 12-17), exactly one
    // typed error, zero aborts, every survivor at stream geometry.
    let packets = ivf_packets("testsrc2_g6_192x108_18f");
    assert_eq!(packets.len(), 18);
    let generation = GenerationId::FIRST.next().must();
    let mut decoder = Av1Decoder::new();
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new("av1").must(),
            generation,
            ThreadBudget::default(),
            None,
            None,
        ))
        .must();
    let mut sink = Sink::default();
    let mut typed_errors = 0usize;
    let mut gate_skips = 0usize;
    for (i, packet) in packets.iter().enumerate() {
        let mut one = packet.clone();
        one.generation = generation;
        if i == 9 {
            // Flip the first OBU byte's forbidden bit.
            let mut damaged = packet.data.as_slice().to_vec();
            damaged[0] |= 0x80;
            one.data = media_types::packet::SharedBytes::from_vec(damaged);
        }
        match decoder.decode(one, &mut sink) {
            Err(_) => typed_errors += 1,
            Ok(DecodeOutcome::Recovered {
                reason: RecoverReason::Discontinuity,
                ..
            }) => {
                gate_skips += 1;
            }
            Ok(_) => {}
        }
    }
    let _ = decoder.flush(&mut sink);
    assert_eq!(typed_errors, 1, "exactly the damaged packet refuses");
    assert_eq!(
        gate_skips, 2,
        "packets 10-11 are gate-skipped (broken refs)"
    );
    // 9 pre-damage + 6 post-resync frames; nothing concealed, nothing
    // fabricated, no abort.
    assert_eq!(
        sink.frames.len(),
        15,
        "resync at the next independent frame keeps 0-8 and 12-17"
    );
    let expected_pts: Vec<_> = packets
        .iter()
        .enumerate()
        .filter(|(i, _)| *i <= 8 || *i >= 12)
        .map(|(_, p)| p.pts)
        .collect();
    let got_pts: Vec<_> = sink
        .frames
        .iter()
        .map(|f| {
            let Frame::Video(v) = f else { unreachable!() };
            v.pts
        })
        .collect();
    assert_eq!(got_pts, expected_pts, "survivors restore their own pts");
    for frame in &sink.frames {
        let (w, h, _) = video_bytes(frame);
        assert_eq!((w, h), (192, 108));
    }
}

/// The `+FB` fuzz-lite sweep (the card's mandate): < 200 KB inputs
/// through the OBU framer **and** the full decode path (framer,
/// reference-state gate, engine feed for inputs that peek as
/// independent frames, outcome accounting), zero panics, every
/// outcome typed or Ok. The gate is what makes the engine-feed half
/// survivable: garbage that peeks dependent never reaches rav1d's
/// missing-reference abort path (decoder module docs — the recorded
/// 1.1.0 upstream bug).
#[test]
fn fuzz_lite_framing_and_decode_survival() {
    // Deterministic splitmix64 (the demux/playlist suites' seam).
    struct SplitMix64(u64);
    impl SplitMix64 {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^ (z >> 31)
        }
    }
    let mut rng = SplitMix64(0x5e1_d1ce_2026_1024);

    let generation = GenerationId::FIRST.next().must();
    let mut decoder = Av1Decoder::new();
    decoder
        .configure(&DecoderConfig::new(
            CodecDescriptor::new("av1").must(),
            generation,
            ThreadBudget::default(),
            None,
            None,
        ))
        .must();
    let mut sink = Sink::default();

    // Phase 1: random bytes (rare large cases stay < 200 KB) through
    // the full decode path.
    for i in 0..3000 {
        let len = match i % 5 {
            0 => 0,
            1 => (rng.next() % 64) as usize,
            2 => (rng.next() % 1024) as usize,
            3 => (rng.next() % 8192) as usize,
            _ => 60_000 + (rng.next() % 140_000) as usize, // < 200 KB
        };
        let bytes: Vec<u8> = (0..len).map(|_| (rng.next() >> 32) as u8).collect();
        let outcome = codecs::video::av1::obu::parse_temporal_unit(&bytes);
        if let Ok(tu) = &outcome {
            // Structural invariant: the census covers ≤ the input.
            assert!(tu.payload_bytes <= len);
        }
        let packet = Packet {
            track: media_types::TrackId::new(0),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: media_types::packet::SharedBytes::from_vec(bytes),
            keyframe: false,
            discontinuity: false,
            generation,
        };
        let _ = decoder.decode(packet, &mut sink);
        // Any sink content must still be well-formed I420.
        for frame in &sink.frames {
            let (w, h, bytes) = video_bytes(frame);
            assert_eq!(bytes.len(), w as usize * h as usize * 3 / 2);
        }
    }

    // Phase 2: mutations of a real fixture TU (structure-aware
    // damage — the interesting grammar edge cases).
    let packets = ivf_packets("rgb_192x108_6f");
    let base = packets[1].data.as_slice().to_vec();
    for _i in 0..3000 {
        let mut bytes = base.clone();
        let mutations = 1 + (rng.next() % 8) as usize;
        for _ in 0..mutations {
            if bytes.is_empty() {
                break;
            }
            let pos = (rng.next() as usize) % bytes.len();
            match rng.next() % 4 {
                0 => bytes[pos] ^= 1 << (rng.next() % 8),
                1 => bytes[pos] = (rng.next() >> 40) as u8,
                2 => {
                    let _ = bytes.remove(pos);
                }
                _ => bytes.insert(pos, (rng.next() >> 48) as u8),
            }
        }
        let packet = Packet {
            track: media_types::TrackId::new(0),
            pts: media_types::Stamp::Missing,
            dts: media_types::Stamp::Missing,
            duration: None,
            data: media_types::packet::SharedBytes::from_vec(bytes),
            keyframe: false,
            discontinuity: false,
            generation,
        };
        let _ = decoder.decode(packet, &mut sink);
    }
    // A final flush must leave the decoder in a sane, drained state.
    let _ = decoder.flush(&mut sink);
}
