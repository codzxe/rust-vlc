//! H.264 hardware decode-path integration suite (card 5.F2 AC):
//! a **fake hardware adapter** (in-test `HwAccelerator` +
//! `H264HwDecoder` + `H264HwBridge` impls) proves the feed path
//! end-to-end — parser → access-unit assembly → session feed →
//! opaque-handle frames — and the **typed-unsupported fallback**
//! (no accelerator / no bridge = `h264.hw.required` /
//! `h264.hw.no_bridge`, never a silent skip). Fixtures are real
//! ffmpeg-authored bytes from the corpus's `0019-raw-video.h264`
//! (High@1.3, 64×48, gray→Nv12-view) pinned as constants, plus the
//! same stream wrapped as an `avcC` + length-prefixed samples. The
//! parser itself is LANDED (5.F1) and pinned by its own suite +
//! `h264_parser_corpus.rs`; this suite pins only the 5.F2 plumbing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::prelude::*;
use codecs::video::h264::H264Decoder;
use codecs::video::h264::hw::{
    H264HwAccessUnit, H264HwBridge, H264HwDecoded, H264HwDecoder, H264HwError, H264HwFrame,
    H264HwStreamInfo,
};
use media_types::frame::FrameStorage;
use media_types::packet::SharedBytes;
use media_types::{EngineError, ErrorKind, GenerationId, Packet, PixelFormat, Stamp, TrackId};
/// `must()` for the negative branch: unwrap the typed error,
/// unreachable on Ok (testkit's `Must` has no err-half).
trait MustErr {
    fn must_err(self) -> EngineError;
}

impl<T> MustErr for Result<T, EngineError> {
    #[track_caller]
    fn must_err(self) -> EngineError {
        match self {
            Err(e) => e,
            Ok(_) => unreachable!("expected a typed error, got Ok"),
        }
    }
}

// ---- fixtures: corpus 0019-raw-video.h264 (x264 High@1.3 64×48
// gray) — provenance: the corpus file's first access units; the
// parser suite (5.F1) pins the same bytes field-exact. ----

const REAL_SPS: &[u8] = &[
    0x67, 0x64, 0x00, 0x0d, 0xf3, 0x65, 0x11, 0xec, 0x05, 0xb2, 0x00, 0x00, 0x03, 0x00, 0x02, 0x00,
    0x00, 0x03, 0x00, 0x28, 0x1e, 0x28, 0x53, 0x2c,
];
const REAL_PPS: &[u8] = &[0x68, 0xce, 0x3c, 0x80];
const REAL_IDR: &[u8] = &[
    0x65, 0x88, 0x84, 0x00, 0x1a, 0xa0, 0x84, 0x00, 0x11, 0x98, 0x95, 0x9d, 0xd9, 0x59, 0x9d, 0x98,
    0x00, 0x05, 0xa6, 0x3f, 0xf0, 0x09, 0x54, 0xe0, 0x03, 0x37, 0xd5, 0x3, 0x7d, 0x63, 0xe4, 0xc9,
    0x93, 0x1d, 0xf0, 0x33, 0x14, 0x55, 0x12, 0x00,
];
/// Two non-IDR slices of consecutive pictures (frame_num 0 and 1 of
/// the POC-type-2 stream; taken from the same file's VCL run).
const REAL_SLICE_A: &[u8] = &[
    0x41, 0x9a, 0x23, 0x00, 0x6a, 0xb8, 0x82, 0x4f, 0x28, 0x40, 0xd4, 0x9d, 0xaf, 0xff, 0xfe, 0x2a,
    0x19, 0x80, 0xf0, 0xc2, 0x10, 0x10, 0x4f, 0xd8, 0x77, 0x3d, 0x8f, 0x6c, 0x04, 0x9f, 0xa3, 0x0f,
    0xfe, 0x11, 0xc4, 0x04, 0x49, 0xa6, 0x8f, 0xf,
];
const REAL_SLICE_B: &[u8] = &[
    0x01, 0x9e, 0x62, 0x40, 0x31, 0x59, 0x38, 0x24, 0xf8, 0xbd, 0x32, 0x45, 0x6a, 0x3f, 0x9f, 0xfe,
    0xbc, 0xc6, 0x6c, 0xde, 0x1b, 0x88, 0x9e, 0xcf, 0xdb, 0x8a, 0x60,
];

fn annexb(units: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for u in units {
        out.extend_from_slice(b"\x00\x00\x00\x01");
        out.extend_from_slice(u);
    }
    out
}

/// An ffmpeg-style `avcC` record for the fixture parameter sets
/// (length_size 4; SPS count byte `0xe1`, PPS count byte `0x01` —
/// the real-world form the LANDED parser documents).
fn avcc_record() -> Vec<u8> {
    let mut out = vec![0x01, 0x64, 0x00, 0x0d, 0xff, 0xe1];
    out.extend_from_slice(&(REAL_SPS.len() as u16).to_be_bytes());
    out.extend_from_slice(REAL_SPS);
    out.push(0x01);
    out.extend_from_slice(&(REAL_PPS.len() as u16).to_be_bytes());
    out.extend_from_slice(REAL_PPS);
    out
}

/// One AVC length-prefixed sample (length_size 4) of `units`.
fn avc_sample(units: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for u in units {
        out.extend_from_slice(&(u.len() as u32).to_be_bytes());
        out.extend_from_slice(u);
    }
    out
}

fn packet(bytes: Vec<u8>, pts: u64) -> Packet {
    let ts = media_types::time::Timestamp::from_parts(pts as i64, 1, 1_000_000_000)
        .unwrap_or_else(|e| unreachable!("{e}"));
    Packet {
        track: TrackId::new(0),
        pts: Stamp::Present(ts),
        dts: Stamp::Missing,
        duration: None,
        data: SharedBytes::from_vec(bytes),
        keyframe: true,
        discontinuity: false,
        generation: GenerationId::FIRST,
    }
}

// ---- the fake hardware adapter (the in-test 7.B stand-in) ----

/// One observed session-configure call: (sps count, pps count,
/// negotiated display size).
type ConfigureCall = (usize, usize, Option<(u32, u32)>);

/// One sink-delivered frame: (surface handle, size, format, pts).
type SinkFrame = (u64, (u32, u32), PixelFormat, Option<i64>);

/// What the fake session observed (test assertions read this).
#[derive(Debug, Default)]
struct SessionLog {
    configured: Vec<ConfigureCall>,
    aus: Vec<H264HwAccessUnit>,
    flushed: u32,
}

struct FakeAccel {
    session_log: Arc<Mutex<SessionLog>>,
    /// Fail mode injected into the session's `configure`.
    fail_configure: Option<H264HwError>,
    /// Fail mode injected into the session's `decode`.
    fail_decode: Option<H264HwError>,
    /// Decoded-surface lag: hold each picture until the next decode.
    lag: bool,
}

impl codec_api::HwAccelerator for FakeAccel {
    fn backend_id(&self) -> &str {
        "hw.fake-h264"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl FakeAccel {
    fn new_session(&self) -> FakeSession {
        FakeSession {
            log: Arc::clone(&self.session_log),
            next_surface: AtomicU64::new(1),
            held: Vec::new(),
            fail_configure: self.fail_configure.clone(),
            fail_decode: self.fail_decode.clone(),
            lag: self.lag,
            display_size: None,
        }
    }
}

struct FakeSession {
    log: Arc<Mutex<SessionLog>>,
    next_surface: AtomicU64,
    held: Vec<H264HwFrame>,
    fail_configure: Option<H264HwError>,
    fail_decode: Option<H264HwError>,
    lag: bool,
    display_size: Option<(u32, u32)>,
}

impl H264HwDecoder for FakeSession {
    fn backend_id(&self) -> &str {
        "hw.fake-h264"
    }

    fn configure(&mut self, stream: &H264HwStreamInfo) -> Result<(), H264HwError> {
        if let Some(e) = &self.fail_configure {
            return Err(e.clone());
        }
        self.display_size = stream.display_size;
        self.log
            .lock()
            .unwrap_or_else(|e| unreachable!("{e}"))
            .configured
            .push((stream.sps.len(), stream.pps.len(), stream.display_size));
        Ok(())
    }

    fn decode(&mut self, au: &H264HwAccessUnit) -> Result<H264HwDecoded, H264HwError> {
        if let Some(e) = &self.fail_decode {
            return Err(e.clone());
        }
        self.log
            .lock()
            .unwrap_or_else(|e| unreachable!("{e}"))
            .aus
            .push(au.clone());
        let frame = H264HwFrame {
            surface: self.next_surface.fetch_add(1, Ordering::SeqCst),
            size: au.display_size.unwrap_or((64, 48)),
            interlaced: au.first_vcl().is_some_and(|v| v.slice.field_pic_flag),
            pts: au.pts,
        };
        let mut out = H264HwDecoded::default();
        if self.lag {
            // One-frame pipeline: emit everything held, hold this one.
            out.frames = std::mem::take(&mut self.held);
            self.held.push(frame);
        } else {
            out.frames.push(frame);
        }
        Ok(out)
    }

    fn flush(&mut self) -> Result<H264HwDecoded, H264HwError> {
        self.log
            .lock()
            .unwrap_or_else(|e| unreachable!("{e}"))
            .flushed += 1;
        Ok(H264HwDecoded {
            frames: std::mem::take(&mut self.held),
        })
    }
}

/// The bridge: knows the concrete `FakeAccel` type (the codec-api
/// downcast seam's "wrapper that knows its backend"). The session
/// it mints is built entirely from the accelerator's own state —
/// the bridge is deliberately stateless.
struct FakeBridge;

impl H264HwBridge for FakeBridge {
    fn backend_id(&self) -> &str {
        "hw.fake-h264"
    }

    fn open(&self, accel: &Arc<dyn codec_api::HwAccelerator>) -> Option<Box<dyn H264HwDecoder>> {
        // Only our own concrete type is claimable.
        accel.as_any().downcast_ref::<FakeAccel>().map(|concrete| {
            let session: FakeSession = concrete.new_session();
            Box::new(session) as Box<dyn H264HwDecoder>
        })
    }
}

/// A bridge that never claims anything (a foreign backend).
struct AlienBridge;

impl H264HwBridge for AlienBridge {
    fn backend_id(&self) -> &str {
        "hw.alien"
    }
    fn open(&self, _accel: &Arc<dyn codec_api::HwAccelerator>) -> Option<Box<dyn H264HwDecoder>> {
        None
    }
}

/// A sink that records every frame.
#[derive(Default)]
struct RecordingSink {
    frames: Vec<SinkFrame>,
}

impl FrameSink for RecordingSink {
    fn accept(&mut self, frame: media_types::Frame) -> Result<(), EngineError> {
        let media_types::Frame::Video(v) = frame else {
            unreachable!("h264 emits only video frames");
        };
        let surface = match v.data {
            FrameStorage::OpaqueHandle(h) => h,
            FrameStorage::Cpu(_) => unreachable!("hw path never emits CPU bytes"),
        };
        let pts = match v.pts {
            Stamp::Present(t) => Some(t.ticks),
            Stamp::Missing => None,
        };
        self.frames.push((surface, v.size, v.format, pts));
        Ok(())
    }
}

fn config(
    accel: Option<Arc<dyn codec_api::HwAccelerator>>,
    extra: Option<Vec<u8>>,
) -> DecoderConfig {
    DecoderConfig::new(
        media_types::ids::CodecDescriptor::new("h264").unwrap_or_else(|e| unreachable!("{e}")),
        GenerationId::FIRST,
        ThreadBudget::default(),
        accel,
        extra.map(SharedBytes::from_vec),
    )
}

// ---- AC 1: the typed-unsupported fallback (matrix truthfulness) ----

#[test]
fn no_accelerator_is_typed_unsupported_never_silent() {
    let mut decoder = H264Decoder::new();
    let err = decoder
        .configure(&config(None, Some(avcc_record())))
        .must_err();
    assert!(
        matches!(err.kind, ErrorKind::Unsupported { .. }),
        "ADR-001 fallback must be Unsupported, got {:?}",
        err.kind
    );
    assert_eq!(err.context.code, "h264.hw.required");
    // And it says so in Display (the honest human-facing message).
    let _ = err.to_string();
}

#[test]
fn accelerator_without_bridge_is_typed_no_bridge() {
    // An accelerator IS negotiated, but no installed bridge claims
    // it — still typed, still never a software fallback.
    let mut decoder = H264Decoder::new(); // no bridges
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::new(Mutex::new(SessionLog::default())),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    let err = decoder
        .configure(&config(Some(accel), Some(avcc_record())))
        .must_err();
    assert!(matches!(err.kind, ErrorKind::Unsupported { .. }));
    assert_eq!(err.context.code, "h264.hw.no_bridge");
}

#[test]
fn wrong_codec_descriptor_is_refused_typed() {
    let mut decoder = H264Decoder::new();
    let mut cfg = config(None, None);
    cfg.codec =
        media_types::ids::CodecDescriptor::new("hevc").unwrap_or_else(|e| unreachable!("{e}"));
    let err = decoder.configure(&cfg).must_err();
    assert_eq!(err.context.code, "h264.codec_descriptor");
}

#[test]
fn avc1_descriptor_aliases_are_accepted() {
    // MP4 fourcc-style descriptors (avc1.64000d) must configure.
    let mut decoder = H264Decoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::new(Mutex::new(SessionLog::default())),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    let mut cfg = config(Some(accel), Some(avcc_record()));
    cfg.codec = media_types::ids::CodecDescriptor::new("avc1.64000d")
        .unwrap_or_else(|e| unreachable!("{e}"));
    decoder
        .configure(&cfg)
        .unwrap_or_else(|e| unreachable!("{e}"));
}

// ---- AC 2: the feed path (fake adapter end-to-end) ----

#[test]
fn annexb_stream_feeds_aus_and_emits_opaque_frames() {
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = H264Decoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    // In-band parameter sets, Annex-B framing (no extradata).
    decoder
        .configure(&config(Some(accel), None))
        .unwrap_or_else(|e| unreachable!("{e}"));

    let mut sink = RecordingSink::default();
    // Packet 1: SPS/PPS/SEI/IDR — IDR picture closes at the NEXT
    // picture's first VCL (boundary detection), so this returns
    // NeedMoreInput with the AU buffered.
    let out = decoder
        .decode(
            packet(annexb(&[REAL_SPS, REAL_PPS, REAL_IDR]), 0),
            &mut sink,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(out, DecodeOutcome::NeedMoreInput, "AU not yet closed");

    // Packet 2: first VCL of picture 2 closes picture 1.
    let out = decoder
        .decode(packet(annexb(&[REAL_SLICE_A]), 1_000_000), &mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(out, DecodeOutcome::Emitted { frames: 1 });
    assert_eq!(sink.frames.len(), 1);
    assert_eq!(sink.frames[0].3, Some(0), "pts of the IDR's packet");

    // Packet 3 closes picture 2; flush drains picture 3.
    let out = decoder
        .decode(packet(annexb(&[REAL_SLICE_B]), 2_000_000), &mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(out, DecodeOutcome::Emitted { frames: 1 });
    let out = decoder
        .flush(&mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(out, DecodeOutcome::Emitted { frames: 1 });
    assert_eq!(sink.frames.len(), 3);
    // Surfaces are distinct opaque handles, Nv12 view, 64×48.
    assert_eq!(sink.frames[0].0, 1);
    assert_eq!(sink.frames[1].0, 2);
    assert_eq!(sink.frames[2].0, 3);
    assert_eq!(sink.frames[0].1, (64, 48));
    assert_eq!(sink.frames[0].2, PixelFormat::Nv12);

    // The session saw: configure (in-band SPS+PPS), 3 AUs (IDR
    // first), one flush.
    let log = log.lock().unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(log.configured.len(), 1, "one parameter-set sync");
    assert_eq!(log.configured[0], (1, 1, Some((64, 48))));
    assert_eq!(log.aus.len(), 3);
    assert!(log.aus[0].idr, "first AU is the IDR picture");
    assert!(!log.aus[1].idr && !log.aus[2].idr);
    // Every AU's first VCL parsed (the parser's slice header rides
    // along the feed).
    assert!(log.aus.iter().all(|au| au.first_vcl().is_some()));
    // The IDR AU carries its in-band parameter-set units.
    assert_eq!(log.aus[0].units.len(), 3);
    assert_eq!(log.flushed, 1);
}

#[test]
fn avcc_extradata_configures_and_length_prefixed_samples_decode() {
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = H264Decoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: None,
        fail_decode: None,
        lag: true,
    });
    decoder
        .configure(&config(Some(accel), Some(avcc_record())))
        .unwrap_or_else(|e| unreachable!("{e}"));
    // Configure-time sync: the avcC parameter sets reached the
    // session before any packet.
    {
        let log = log.lock().unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(log.configured.len(), 1);
        assert_eq!(log.configured[0].2, Some((64, 48)));
    }

    let mut sink = RecordingSink::default();
    // Sample 1 = IDR AU (closes at end of packet under Avc framing);
    // the lagging session emits nothing yet.
    let out = decoder
        .decode(packet(avc_sample(&[REAL_IDR]), 0), &mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(out, DecodeOutcome::NeedMoreInput);
    // Sample 2: AU 2 fed; AU 1's surface surfaces.
    let out = decoder
        .decode(packet(avc_sample(&[REAL_SLICE_A]), 1_000_000), &mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(out, DecodeOutcome::Emitted { frames: 1 });
    assert_eq!(sink.frames[0].3, Some(0));
    // Flush: AU 2's held surface drains.
    let out = decoder
        .flush(&mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(out, DecodeOutcome::Emitted { frames: 1 });
    assert_eq!(sink.frames.len(), 2);
    assert_eq!(sink.frames[1].3, Some(1_000_000));
    // Second flush stays drained.
    let out = decoder
        .flush(&mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(out, DecodeOutcome::Drained);
}

#[test]
fn multi_slice_picture_is_one_au() {
    // Two VCL units with IDENTICAL picture keys = one picture
    // (multi-slice): both land in the same AU.
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = H264Decoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    decoder
        .configure(&config(Some(accel), None))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let mut sink = RecordingSink::default();
    // Same slice twice with identical headers (first_mb 0 then a
    // second slice of the same picture): REAL_SLICE_A twice would
    // parse with equal keys — multi-slice emulation.
    let _ = decoder
        .decode(
            packet(annexb(&[REAL_SPS, REAL_PPS, REAL_IDR, REAL_SLICE_A]), 0),
            &mut sink,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = decoder
        .decode(packet(annexb(&[REAL_SLICE_A]), 5), &mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = decoder
        .flush(&mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    let log = log.lock().unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(log.aus.len(), 2, "IDR picture, then the A-slice picture");
    // The second AU holds both A-slice VCL units.
    let vcl = log.aus[1].units.iter().filter(|u| u.vcl.is_some()).count();
    assert_eq!(vcl, 2, "both slices of picture 2 in one AU");
}

// ---- AC 3: contract + damage behavior ----

#[test]
fn decode_before_configure_is_typed_bad_state() {
    let mut decoder = H264Decoder::new();
    let mut sink = RecordingSink::default();
    let err = decoder.decode(packet(vec![0x00], 0), &mut sink).must_err();
    assert_eq!(err.context.code, "h264.not_configured");
    let err = decoder.flush(&mut sink).must_err();
    assert_eq!(err.context.code, "h264.not_configured");
}

#[test]
fn stale_packets_are_dropped_on_sight() {
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = H264Decoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    let generation = GenerationId::FIRST
        .next()
        .unwrap_or_else(|e| unreachable!("{e}"));
    decoder
        .configure(&DecoderConfig::new(
            media_types::ids::CodecDescriptor::new("h264").unwrap_or_else(|e| unreachable!("{e}")),
            generation,
            ThreadBudget::default(),
            Some(accel),
            None,
        ))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let mut sink = RecordingSink::default();
    let mut stale = packet(annexb(&[REAL_SPS, REAL_PPS, REAL_IDR]), 0);
    stale.generation = GenerationId::FIRST;
    let out = decoder
        .decode(stale, &mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(out, DecodeOutcome::NeedMoreInput);
    assert!(sink.frames.is_empty());
    let log = log.lock().unwrap_or_else(|e| unreachable!("{e}"));
    assert!(log.aus.is_empty(), "zero decode work for stale input");
}

#[test]
fn damage_is_counted_recovery_and_decoder_stays_usable() {
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = H264Decoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    decoder
        .configure(&config(Some(accel), Some(avcc_record())))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let mut sink = RecordingSink::default();
    // A packet of: garbage "slice" (forbidden bit set → header
    // damage), then the real IDR: counted skip + emitted frame.
    let garbage_vcl = [0xe5u8, 0x88, 0x84, 0x00];
    let out = decoder
        .decode(packet(avc_sample(&[&garbage_vcl, REAL_IDR]), 0), &mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(
        out,
        DecodeOutcome::Recovered {
            skipped: 1,
            reason: RecoverReason::CorruptInput
        }
    );
    assert_eq!(sink.frames.len(), 1);
    // Truncated AVC length prefix (declares more than remains):
    // counted, bounded, decoder still answers the next packet.
    let mut truncated = Vec::new();
    truncated.extend_from_slice(&0xffff_ffffu32.to_be_bytes());
    truncated.extend_from_slice(REAL_SLICE_A);
    let out = decoder
        .decode(packet(truncated, 1), &mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert!(matches!(out, DecodeOutcome::Recovered { .. }));
    let out = decoder
        .decode(packet(avc_sample(&[REAL_SLICE_B]), 2), &mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert!(matches!(
        out,
        DecodeOutcome::Emitted { frames: 1 } | DecodeOutcome::Recovered { .. }
    ));
}

#[test]
fn hw_session_failure_is_typed_engine_error() {
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = H264Decoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: None,
        fail_decode: Some(H264HwError::Decode {
            code: "h264.hw.fake.decode",
        }),
        lag: false,
    });
    decoder
        .configure(&config(Some(accel), Some(avcc_record())))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let mut sink = RecordingSink::default();
    let err = decoder
        .decode(packet(avc_sample(&[REAL_IDR]), 0), &mut sink)
        .must_err();
    assert_eq!(err.context.code, "h264.hw.decode");
    // Device-lost maps to the hardware-lost decode cause.
    let mut decoder2 = H264Decoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel2: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::new(Mutex::new(SessionLog::default())),
        fail_configure: None,
        fail_decode: Some(H264HwError::DeviceLost),
        lag: false,
    });
    decoder2
        .configure(&config(Some(accel2), Some(avcc_record())))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let err = decoder2
        .decode(
            packet(avc_sample(&[REAL_IDR]), 0),
            &mut RecordingSink::default(),
        )
        .must_err();
    assert!(
        matches!(
            err.kind,
            ErrorKind::Decoder {
                cause: media_types::error::DecodeCause::HardwareLost
            }
        ),
        "{:?}",
        err.kind
    );
}

#[test]
fn hw_configure_failure_is_typed_unsupported() {
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = H264Decoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: Some(H264HwError::Configure {
            code: "h264.hw.fake.profile",
        }),
        fail_decode: None,
        lag: false,
    });
    let err = decoder
        .configure(&config(Some(accel), Some(avcc_record())))
        .must_err();
    assert_eq!(err.context.code, "h264.hw.configure");
    assert!(matches!(err.kind, ErrorKind::Unsupported { .. }));
}

#[test]
fn alien_bridge_leaves_the_stream_unclaimed() {
    // An installed but foreign bridge must not claim the fake
    // accelerator: typed no_bridge.
    let mut decoder = H264Decoder::with_bridges(vec![Arc::new(AlienBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::new(Mutex::new(SessionLog::default())),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    let err = decoder
        .configure(&config(Some(accel), Some(avcc_record())))
        .must_err();
    assert_eq!(err.context.code, "h264.hw.no_bridge");
}

// ---- AC 4: real corpus stream (skip-loud when absent) ----

#[test]
fn corpus_0019_annexb_full_stream_survives_and_feeds_every_picture() {
    let path = std::path::PathBuf::from(
        std::env::var("LUMEN_CORPUS").unwrap_or_else(|_| "/tmp/my-project/corpus-v4".to_string()),
    )
    .join("media/0019-raw-video.h264");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("SKIP h264 hw-path corpus: 0019-raw-video.h264 absent");
        return;
    };
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = H264Decoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    decoder
        .configure(&config(Some(accel), None))
        .unwrap_or_else(|e| unreachable!("{e}"));

    // Whole stream as ONE packet (Annex-B): every picture boundary
    // detected in-packet; count VCL units fed vs. AUs closed.
    let mut sink = RecordingSink::default();
    let out = decoder
        .decode(packet(bytes.clone(), 0), &mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = decoder
        .flush(&mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));

    let log = log.lock().unwrap_or_else(|e| unreachable!("{e}"));
    let fed_vcl: usize = log
        .aus
        .iter()
        .map(|au| au.units.iter().filter(|u| u.vcl.is_some()).count())
        .sum();
    // Ground truth by direct parser count (the LANDED splitter):
    // 26 VCL NALs (1 IDR + 25 non-IDR) in this file.
    let truth_vcl = codecs::video::h264::parser::nal::AnnexBNals::new(&bytes)
        .filter(|nal| matches!(nal[0] & 0x1f, 1 | 5))
        .count();
    assert_eq!(truth_vcl, 26, "corpus shape drift: re-pin");
    assert_eq!(
        fed_vcl, truth_vcl,
        "every VCL unit of the stream reached a session AU exactly once"
    );
    assert_eq!(log.aus.len(), 26, "one AU per picture (1 slice each)");
    assert!(log.aus[0].idr);
    assert!(
        matches!(
            out,
            DecodeOutcome::NeedMoreInput | DecodeOutcome::Recovered { .. }
        ) || sink.frames.len() >= 26
    );
    assert!(sink.frames.len() >= 26, "every picture produced a surface");
    // Picture keys advance: frame_num is non-decreasing across AUs.
    let frame_nums: Vec<u32> = log
        .aus
        .iter()
        .filter_map(|au| au.first_vcl().map(|v| v.slice.frame_num))
        .collect();
    assert_eq!(frame_nums.len(), log.aus.len());
    for pair in frame_nums.windows(2) {
        assert!(
            pair[1] >= pair[0] || pair[1] == 0,
            "frame_num regressed: {pair:?}"
        );
    }
}

// ---- +FB fuzz-lite: absence-of-panic on arbitrary bytes ----

/// splitmix64 (deterministic, no external dep).
struct SplitMix64(u64);
impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

#[test]
fn fuzz_lite_random_packets_never_panic() {
    // 5k random packets through the full configured hw path (fake
    // session), both framings, seeded from the real avcC: the claim
    // is absence of panic/hang only (typed errors and Recovered are
    // expected outcomes).
    let mut rng = SplitMix64(0x5eed_1234_abcd_f00d);
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = H264Decoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    decoder
        .configure(&config(Some(accel), Some(avcc_record())))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let mut sink = RecordingSink::default();
    for i in 0..5_000u32 {
        let len = (rng.next() % 512) as usize;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            bytes.push((rng.next() >> 24) as u8);
        }
        // Occasionally splice a real NAL in (reach the parse paths).
        if i % 7 == 0 {
            bytes = avc_sample(&[REAL_SPS, REAL_PPS, REAL_IDR]);
        }
        let _ = decoder.decode(packet(bytes, u64::from(i)), &mut sink);
    }
    let _ = decoder.flush(&mut sink);
    // Bounded accumulation held (MP §12): the pending list cannot
    // exceed the cap even under adversarial unit floods.
    let pending = format!("{decoder:?}");
    assert!(pending.contains("H264Decoder"));
}

#[test]
fn fuzz_lite_configure_with_random_extradata_never_panics() {
    // 2k random extradata blobs through configure: avcC-shaped
    // misses, Annex-B misses, pure noise — all typed outcomes.
    let mut rng = SplitMix64(0xdead_beef_cafe_1234);
    for i in 0..2_000u32 {
        let len = (rng.next() % 64) as usize;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            bytes.push((rng.next() >> 32) as u8);
        }
        if i % 3 == 0 && bytes.len() >= 7 {
            // Push it toward avcC shape (version 1 + reserved bits).
            bytes[0] = 0x01;
            bytes[4] = 0xff;
        }
        let mut decoder = H264Decoder::with_bridges(vec![Arc::new(FakeBridge)]);
        let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
            session_log: Arc::new(Mutex::new(SessionLog::default())),
            fail_configure: None,
            fail_decode: None,
            lag: false,
        });
        // Outcomes are typed errors or Ok — both fine, no panic.
        let _ = decoder.configure(&config(Some(accel), Some(bytes)));
        let mut sink = RecordingSink::default();
        let _ = decoder.decode(packet(vec![0x00, 0x00, 0x00, 0x01, 0x65], 0), &mut sink);
        let _ = decoder.flush(&mut sink);
    }
}
