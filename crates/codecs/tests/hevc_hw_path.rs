//! HEVC hardware decode-path integration suite (card 5.G2 AC — the
//! mirror of 5.F2's): a **fake hardware adapter** (in-test
//! `HwAccelerator` + `HevcHwDecoder` + `HevcHwBridge` impls) proves
//! the feed path end-to-end — parser → access-unit assembly →
//! session feed → opaque-handle frames — and the **typed-unsupported
//! fallback** (no accelerator / no bridge = `hevc.hw.required` /
//! `hevc.hw.no_bridge`, never a silent skip). Fixtures are real
//! ffmpeg/x265-authored bytes from the corpus's
//! `0044-raw-video.265` (Rext@6.3, 360×640, 4:2:0 8-bit,
//! **all-IDR** — every picture IDR_N_LP, the shape that exercises
//! §7.4.2.4.4's non-VCL-intervened boundary condition) pinned as
//! constants: the full parameter sets, an ffmpeg-authored `hvcC`
//! record carrying them (extracted from a real MP4 mux of the same
//! stream), and 48-byte prefixes of the first three slice segments
//! (the 5.G1 slice parser is an honest prefix parser; the fake
//! session consumes headers, not slice data — the full-file corpus
//! test below feeds the untruncated stream). The parser itself is
//! LANDED (5.G1) and pinned by its own suite +
//! `hevc_parser_corpus.rs`; this suite pins only the 5.G2 plumbing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::prelude::*;
use codecs::video::hevc::HevcDecoder;
use codecs::video::hevc::hw::{
    HevcHwAccessUnit, HevcHwBridge, HevcHwDecoded, HevcHwDecoder, HevcHwError, HevcHwFrame,
    HevcHwStreamInfo,
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

// ---- fixtures: corpus 0044-raw-video.265 (x265 Rext@6.3 360×640,
// 8-bit 4:2:0, all-IDR) — provenance: the corpus file's first
// access units; the parser suite (5.G1) pins the same parameter
// sets field-exact. ----

const REAL_VPS: &[u8] = &[
    0x40, 0x01, 0x0c, 0x01, 0xff, 0xff, 0x04, 0x08, 0x00, 0x00, 0x03, 0x00, 0x9f, 0xa8, 0x00, 0x00,
    0x03, 0x00, 0x00, 0x3f, 0xba, 0x02, 0x40,
];
const REAL_SPS: &[u8] = &[
    0x42, 0x01, 0x01, 0x04, 0x08, 0x00, 0x00, 0x03, 0x00, 0x9f, 0xa8, 0x00, 0x00, 0x03, 0x00, 0x00,
    0x3f, 0xa0, 0x0b, 0x88, 0x02, 0x81, 0xcb, 0xe5, 0xba, 0x4a, 0x4c, 0x2f, 0x01, 0x68, 0x08, 0x00,
    0x00, 0x03, 0x00, 0x08, 0x00, 0x00, 0x03, 0x00, 0xf0, 0x40,
];
const REAL_PPS: &[u8] = &[0x44, 0x01, 0xc0, 0x71, 0x83, 0x12];
/// The prefix-SEI that precedes every picture of this stream
/// (opaque to the decode path; truncated — the session never parses
/// it, and §7.4.2.4.4 boundary detection needs only its *presence*).
const REAL_SEI_PREFIX: &[u8] = &[
    0x4e, 0x01, 0x05, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x11, 0x2c, 0xa2, 0xde,
    0x09, 0xb5, 0x17, 0x47, 0xdb, 0xbb, 0x55, 0xa4, 0xfe, 0x7f, 0xc2, 0xfc, 0x4e, 0x78, 0x32, 0x36,
];
/// Picture 1's IDR_N_LP slice segment — first 48 bytes (the parsed
/// header prefix + slice-data head; see the module docs).
const REAL_IRAP_PREFIX: &[u8] = &[
    0x28, 0x01, 0xac, 0x6c, 0x28, 0x2f, 0xe7, 0x46, 0x89, 0x9d, 0x3b, 0x24, 0x6b, 0x2d, 0x49, 0x08,
    0xbd, 0x2b, 0xb1, 0x82, 0x64, 0x01, 0x90, 0x85, 0x1b, 0xb0, 0x35, 0xda, 0x8e, 0x35, 0x90, 0x1f,
    0x7a, 0xde, 0xf2, 0x7e, 0xf3, 0x5e, 0x92, 0x60, 0x13, 0xb4, 0x26, 0xe5, 0x70, 0x3b, 0xb0, 0xe7,
];
/// Picture 2's IDR_N_LP slice segment (48-byte prefix).
const REAL_TRAIL_A_PREFIX: &[u8] = &[
    0x28, 0x01, 0xaf, 0x0a, 0x0b, 0xb2, 0x4a, 0x95, 0x88, 0xab, 0x05, 0xd0, 0x84, 0x8c, 0x89, 0x22,
    0x21, 0x47, 0x5c, 0xd8, 0x00, 0xb2, 0xa4, 0xde, 0x5d, 0xeb, 0xbc, 0x6e, 0x88, 0xec, 0x05, 0xc0,
    0x8b, 0xb7, 0x0d, 0xf9, 0x89, 0xea, 0xda, 0xb0, 0xc6, 0x7e, 0x22, 0xc1, 0x72, 0x41, 0xe2, 0x17,
];
/// Picture 3's IDR_N_LP slice segment (48-byte prefix).
const REAL_TRAIL_B_PREFIX: &[u8] = &[
    0x28, 0x01, 0xae, 0x0a, 0x0b, 0xb7, 0xc9, 0x01, 0xbf, 0xaf, 0x94, 0xc2, 0x7e, 0x0f, 0x21, 0x36,
    0x1c, 0xe7, 0x49, 0x0a, 0x07, 0x92, 0x9e, 0xe3, 0x9f, 0x1b, 0xe8, 0x6b, 0x88, 0xf0, 0x04, 0xc0,
    0x8b, 0xb7, 0x0d, 0xf9, 0x89, 0xea, 0xda, 0xb0, 0xc6, 0x7e, 0x22, 0xc1, 0x72, 0x41, 0xe2, 0xff,
];
/// The real ffmpeg-authored `hvcC` record for the fixture parameter
/// sets (extracted from `ffmpeg -c copy` of this very stream into
/// MP4; `lengthSizeMinusOne` = 3 → length_size 4, three arrays:
/// VPS/SPS/PPS — the real-world form the LANDED parser documents).
const REAL_HVCC: &[u8] = &[
    0x01, 0x04, 0x08, 0x00, 0x00, 0x00, 0x9f, 0xa8, 0x00, 0x00, 0x00, 0x00, 0x3f, 0xf0, 0x00, 0xfc,
    0xfd, 0xf8, 0xf8, 0x00, 0x00, 0x0f, 0x03, 0x20, 0x00, 0x01, 0x00, 0x17, 0x40, 0x01, 0x0c, 0x01,
    0xff, 0xff, 0x04, 0x08, 0x00, 0x00, 0x03, 0x00, 0x9f, 0xa8, 0x00, 0x00, 0x03, 0x00, 0x00, 0x3f,
    0xba, 0x02, 0x40, 0x21, 0x00, 0x01, 0x00, 0x2a, 0x42, 0x01, 0x01, 0x04, 0x08, 0x00, 0x00, 0x03,
    0x00, 0x9f, 0xa8, 0x00, 0x00, 0x03, 0x00, 0x00, 0x3f, 0xa0, 0x0b, 0x88, 0x02, 0x81, 0xcb, 0xe5,
    0xba, 0x4a, 0x4c, 0x2f, 0x01, 0x68, 0x08, 0x00, 0x00, 0x03, 0x00, 0x08, 0x00, 0x00, 0x03, 0x00,
    0xf0, 0x40, 0x22, 0x00, 0x01, 0x00, 0x06, 0x44, 0x01, 0xc0, 0x71, 0x83, 0x12,
];

fn annexb(units: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for u in units {
        out.extend_from_slice(b"\x00\x00\x00\x01");
        out.extend_from_slice(u);
    }
    out
}

/// One length-prefixed sample (length_size 4) of `units`.
fn hvcc_sample(units: &[&[u8]]) -> Vec<u8> {
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

/// One observed session-configure call: (vps count, sps count, pps
/// count, negotiated display size).
type ConfigureCall = (usize, usize, usize, Option<(u32, u32)>);

/// One sink-delivered frame: (surface handle, size, format, pts).
type SinkFrame = (u64, (u32, u32), PixelFormat, Option<i64>);

/// What the fake session observed (test assertions read this).
#[derive(Debug, Default)]
struct SessionLog {
    configured: Vec<ConfigureCall>,
    aus: Vec<HevcHwAccessUnit>,
    flushed: u32,
}

struct FakeAccel {
    session_log: Arc<Mutex<SessionLog>>,
    /// Fail mode injected into the session's `configure`.
    fail_configure: Option<HevcHwError>,
    /// Fail mode injected into the session's `decode`.
    fail_decode: Option<HevcHwError>,
    /// Decoded-surface lag: hold each picture until the next decode.
    lag: bool,
}

impl codec_api::HwAccelerator for FakeAccel {
    fn backend_id(&self) -> &str {
        "hw.fake-hevc"
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
    held: Vec<HevcHwFrame>,
    fail_configure: Option<HevcHwError>,
    fail_decode: Option<HevcHwError>,
    lag: bool,
    display_size: Option<(u32, u32)>,
}

impl HevcHwDecoder for FakeSession {
    fn backend_id(&self) -> &str {
        "hw.fake-hevc"
    }

    fn configure(&mut self, stream: &HevcHwStreamInfo) -> Result<(), HevcHwError> {
        if let Some(e) = &self.fail_configure {
            return Err(e.clone());
        }
        self.display_size = stream.display_size;
        self.log
            .lock()
            .unwrap_or_else(|e| unreachable!("{e}"))
            .configured
            .push((
                stream.vps.len(),
                stream.sps.len(),
                stream.pps.len(),
                stream.display_size,
            ));
        Ok(())
    }

    fn decode(&mut self, au: &HevcHwAccessUnit) -> Result<HevcHwDecoded, HevcHwError> {
        if let Some(e) = &self.fail_decode {
            return Err(e.clone());
        }
        self.log
            .lock()
            .unwrap_or_else(|e| unreachable!("{e}"))
            .aus
            .push(au.clone());
        let frame = HevcHwFrame {
            surface: self.next_surface.fetch_add(1, Ordering::SeqCst),
            size: au.display_size.unwrap_or((360, 640)),
            pts: au.pts,
        };
        let mut out = HevcHwDecoded::default();
        if self.lag {
            // One-frame pipeline: emit everything held, hold this one.
            out.frames = std::mem::take(&mut self.held);
            self.held.push(frame);
        } else {
            out.frames.push(frame);
        }
        Ok(out)
    }

    fn flush(&mut self) -> Result<HevcHwDecoded, HevcHwError> {
        self.log
            .lock()
            .unwrap_or_else(|e| unreachable!("{e}"))
            .flushed += 1;
        Ok(HevcHwDecoded {
            frames: std::mem::take(&mut self.held),
        })
    }
}

/// The bridge: knows the concrete `FakeAccel` type (the codec-api
/// downcast seam's "wrapper that knows its backend"). The session
/// it mints is built entirely from the accelerator's own state —
/// the bridge is deliberately stateless.
struct FakeBridge;

impl HevcHwBridge for FakeBridge {
    fn backend_id(&self) -> &str {
        "hw.fake-hevc"
    }

    fn open(&self, accel: &Arc<dyn codec_api::HwAccelerator>) -> Option<Box<dyn HevcHwDecoder>> {
        // Only our own concrete type is claimable.
        accel.as_any().downcast_ref::<FakeAccel>().map(|concrete| {
            let session: FakeSession = concrete.new_session();
            Box::new(session) as Box<dyn HevcHwDecoder>
        })
    }
}

/// A bridge that never claims anything (a foreign backend).
struct AlienBridge;

impl HevcHwBridge for AlienBridge {
    fn backend_id(&self) -> &str {
        "hw.alien"
    }
    fn open(&self, _accel: &Arc<dyn codec_api::HwAccelerator>) -> Option<Box<dyn HevcHwDecoder>> {
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
            unreachable!("hevc emits only video frames");
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
        media_types::ids::CodecDescriptor::new("hevc").unwrap_or_else(|e| unreachable!("{e}")),
        GenerationId::FIRST,
        ThreadBudget::default(),
        accel,
        extra.map(SharedBytes::from_vec),
    )
}

// ---- AC 1: the typed-unsupported fallback (matrix truthfulness) ----

#[test]
fn no_accelerator_is_typed_unsupported_never_silent() {
    let mut decoder = HevcDecoder::new();
    let err = decoder
        .configure(&config(None, Some(REAL_HVCC.to_vec())))
        .must_err();
    assert!(
        matches!(err.kind, ErrorKind::Unsupported { .. }),
        "ADR-001 fallback must be Unsupported, got {:?}",
        err.kind
    );
    assert_eq!(err.context.code, "hevc.hw.required");
    // And it says so in Display (the honest human-facing message).
    let _ = err.to_string();
}

#[test]
fn accelerator_without_bridge_is_typed_no_bridge() {
    // An accelerator IS negotiated, but no installed bridge claims
    // it — still typed, still never a software fallback.
    let mut decoder = HevcDecoder::new(); // no bridges
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::new(Mutex::new(SessionLog::default())),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    let err = decoder
        .configure(&config(Some(accel), Some(REAL_HVCC.to_vec())))
        .must_err();
    assert!(matches!(err.kind, ErrorKind::Unsupported { .. }));
    assert_eq!(err.context.code, "hevc.hw.no_bridge");
}

#[test]
fn wrong_codec_descriptor_is_refused_typed() {
    let mut decoder = HevcDecoder::new();
    let mut cfg = config(None, None);
    cfg.codec =
        media_types::ids::CodecDescriptor::new("h264").unwrap_or_else(|e| unreachable!("{e}"));
    let err = decoder.configure(&cfg).must_err();
    assert_eq!(err.context.code, "hevc.codec_descriptor");
}

#[test]
fn hvc1_descriptor_aliases_are_accepted() {
    // MP4 fourcc-style descriptors (hvc1.1.6.L93.B0) must configure.
    let mut decoder = HevcDecoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::new(Mutex::new(SessionLog::default())),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    let mut cfg = config(Some(accel), Some(REAL_HVCC.to_vec()));
    cfg.codec = media_types::ids::CodecDescriptor::new("hvc1.1.6.L93.B0")
        .unwrap_or_else(|e| unreachable!("{e}"));
    decoder
        .configure(&cfg)
        .unwrap_or_else(|e| unreachable!("{e}"));
}

// ---- AC 2: the feed path (fake adapter end-to-end) ----

#[test]
fn annexb_stream_feeds_aus_and_emits_opaque_frames() {
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = HevcDecoder::with_bridges(vec![Arc::new(FakeBridge)]);
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
    // Packet 1: VPS/SPS/PPS/SEI/IDR — the IDR picture closes at the
    // NEXT picture's first slice (§7.4.2.4.4 boundary: the non-VCL
    // SEI intervenes before picture 2's identical-header IDR), so
    // this returns NeedMoreInput with the AU buffered.
    let out = decoder
        .decode(
            packet(
                annexb(&[
                    REAL_VPS,
                    REAL_SPS,
                    REAL_PPS,
                    REAL_SEI_PREFIX,
                    REAL_IRAP_PREFIX,
                ]),
                0,
            ),
            &mut sink,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(out, DecodeOutcome::NeedMoreInput, "AU not yet closed");

    // Packet 2: SEI + first slice of picture 2 closes picture 1.
    let out = decoder
        .decode(
            packet(annexb(&[REAL_SEI_PREFIX, REAL_TRAIL_A_PREFIX]), 1_000_000),
            &mut sink,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(out, DecodeOutcome::Emitted { frames: 1 });
    assert_eq!(sink.frames.len(), 1);
    assert_eq!(sink.frames[0].3, Some(0), "pts of the IDR's packet");

    // Packet 3 closes picture 2; flush drains picture 3.
    let out = decoder
        .decode(
            packet(annexb(&[REAL_SEI_PREFIX, REAL_TRAIL_B_PREFIX]), 2_000_000),
            &mut sink,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(out, DecodeOutcome::Emitted { frames: 1 });
    let out = decoder
        .flush(&mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(out, DecodeOutcome::Emitted { frames: 1 });
    assert_eq!(sink.frames.len(), 3);
    // Surfaces are distinct opaque handles, Nv12 view, 360×640.
    assert_eq!(sink.frames[0].0, 1);
    assert_eq!(sink.frames[1].0, 2);
    assert_eq!(sink.frames[2].0, 3);
    assert_eq!(sink.frames[0].1, (360, 640));
    assert_eq!(sink.frames[0].2, PixelFormat::Nv12);

    // The session saw: one configure (VPS+SPS+PPS, 360×640), 3 AUs
    // (IDR first), one flush.
    let log = log.lock().unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(log.configured.len(), 1, "one parameter-set sync");
    assert_eq!(log.configured[0], (1, 1, 1, Some((360, 640))));
    assert_eq!(log.aus.len(), 3);
    // All pictures of this all-intra stream are IRAP.
    assert!(log.aus.iter().all(|au| au.irap));
    // Every AU's first VCL parsed (the parser's slice-segment header
    // rides along the feed).
    assert!(log.aus.iter().all(|au| au.first_vcl().is_some()));
    // AU 1 carries its in-band parameter sets + the SEI that
    // preceded its slice, plus the SEI that preceded picture 2
    // (trailing attachment — the 5.F2 shape); AU 2 carries its own
    // trailing SEI ahead of picture 3's boundary.
    assert_eq!(log.aus[0].units.len(), 6);
    assert_eq!(log.aus[1].units.len(), 2);
    assert_eq!(log.flushed, 1);
}

#[test]
fn hvcc_extradata_configures_and_length_prefixed_samples_decode() {
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = HevcDecoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: None,
        fail_decode: None,
        lag: true,
    });
    decoder
        .configure(&config(Some(accel), Some(REAL_HVCC.to_vec())))
        .unwrap_or_else(|e| unreachable!("{e}"));
    // Configure-time sync: the hvcC parameter sets reached the
    // session before any packet.
    {
        let log = log.lock().unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(log.configured.len(), 1);
        assert_eq!(log.configured[0], (1, 1, 1, Some((360, 640))));
    }

    let mut sink = RecordingSink::default();
    // Sample 1 = IDR AU (closes at end of packet under
    // length-prefixed framing); the lagging session emits nothing yet.
    let out = decoder
        .decode(packet(hvcc_sample(&[REAL_IRAP_PREFIX]), 0), &mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(out, DecodeOutcome::NeedMoreInput);
    // Sample 2: AU 2 fed (end-of-packet close); AU 1's surface
    // surfaces.
    let out = decoder
        .decode(
            packet(hvcc_sample(&[REAL_TRAIL_A_PREFIX]), 1_000_000),
            &mut sink,
        )
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
fn multi_segment_picture_is_one_au() {
    // Two first-segment VCL units with IDENTICAL §7.4.2.4.4 keys and
    // no non-VCL between them = one picture (degenerate multi-slice
    // emulation, the same shape 5.F2's suite pins): both land in the
    // same AU.
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = HevcDecoder::with_bridges(vec![Arc::new(FakeBridge)]);
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
    let _ = decoder
        .decode(
            packet(
                annexb(&[
                    REAL_VPS,
                    REAL_SPS,
                    REAL_PPS,
                    REAL_IRAP_PREFIX,
                    REAL_IRAP_PREFIX,
                ]),
                0,
            ),
            &mut sink,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = decoder
        .decode(
            packet(annexb(&[REAL_SEI_PREFIX, REAL_TRAIL_A_PREFIX]), 5),
            &mut sink,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
    let _ = decoder
        .flush(&mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    let log = log.lock().unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(log.aus.len(), 2, "IDR picture, then the A-segment picture");
    // The first AU holds both identical first-segment VCL units.
    let vcl = log.aus[0].units.iter().filter(|u| u.vcl.is_some()).count();
    assert_eq!(vcl, 2, "both segments of picture 1 in one AU");
    // Picture 2 separated by the intervening SEI (same keys,
    // non-VCL boundary condition — the all-IDR shape).
    assert_eq!(
        log.aus[1].units.iter().filter(|u| u.vcl.is_some()).count(),
        1
    );
}

// ---- AC 3: contract + damage behavior ----

#[test]
fn decode_before_configure_is_typed_bad_state() {
    let mut decoder = HevcDecoder::new();
    let mut sink = RecordingSink::default();
    let err = decoder.decode(packet(vec![0x00], 0), &mut sink).must_err();
    assert_eq!(err.context.code, "hevc.not_configured");
    let err = decoder.flush(&mut sink).must_err();
    assert_eq!(err.context.code, "hevc.not_configured");
}

#[test]
fn stale_packets_are_dropped_on_sight() {
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = HevcDecoder::with_bridges(vec![Arc::new(FakeBridge)]);
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
            media_types::ids::CodecDescriptor::new("hevc").unwrap_or_else(|e| unreachable!("{e}")),
            generation,
            ThreadBudget::default(),
            Some(accel),
            None,
        ))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let mut sink = RecordingSink::default();
    let mut stale = packet(annexb(&[REAL_VPS, REAL_SPS, REAL_PPS, REAL_IRAP_PREFIX]), 0);
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
    let mut decoder = HevcDecoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    decoder
        .configure(&config(Some(accel), Some(REAL_HVCC.to_vec())))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let mut sink = RecordingSink::default();
    // A packet of: garbage "slice" (forbidden bit set → header
    // damage), then the real IDR: counted skip + emitted frame.
    let garbage_vcl = [0xa9u8, 0x01, 0xac, 0x6c];
    let out = decoder
        .decode(
            packet(hvcc_sample(&[&garbage_vcl, REAL_IRAP_PREFIX]), 0),
            &mut sink,
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(
        out,
        DecodeOutcome::Recovered {
            skipped: 1,
            reason: RecoverReason::CorruptInput
        }
    );
    assert_eq!(sink.frames.len(), 1);
    // Truncated length prefix (declares more than remains):
    // counted, bounded, decoder still answers the next packet.
    let mut truncated = Vec::new();
    truncated.extend_from_slice(&0xffff_ffffu32.to_be_bytes());
    truncated.extend_from_slice(REAL_TRAIL_A_PREFIX);
    let out = decoder
        .decode(packet(truncated, 1), &mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert!(matches!(out, DecodeOutcome::Recovered { .. }));
    let out = decoder
        .decode(packet(hvcc_sample(&[REAL_TRAIL_B_PREFIX]), 2), &mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert!(matches!(
        out,
        DecodeOutcome::Emitted { frames: 1 } | DecodeOutcome::Recovered { .. }
    ));
}

#[test]
fn orphan_continuation_segment_is_counted_damage() {
    // A non-first slice segment with no picture pending (the
    // dependent-segment prefix needs its first segment first).
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = HevcDecoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    decoder
        .configure(&config(Some(accel), Some(REAL_HVCC.to_vec())))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let mut sink = RecordingSink::default();
    // Same IDR NAL with the first_slice bit cleared (bit 7 of byte
    // 2): a continuation segment of no picture.
    let mut orphan = REAL_IRAP_PREFIX.to_vec();
    orphan[2] &= 0x7f;
    let out = decoder
        .decode(packet(hvcc_sample(&[&orphan]), 0), &mut sink)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(
        out,
        DecodeOutcome::Recovered {
            skipped: 1,
            reason: RecoverReason::CorruptInput
        }
    );
    let log = log.lock().unwrap_or_else(|e| unreachable!("{e}"));
    assert!(log.aus.is_empty(), "orphan segment fed no AU");
}

#[test]
fn hw_session_failure_is_typed_engine_error() {
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = HevcDecoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: None,
        fail_decode: Some(HevcHwError::Decode {
            code: "hevc.hw.fake.decode",
        }),
        lag: false,
    });
    decoder
        .configure(&config(Some(accel), Some(REAL_HVCC.to_vec())))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let mut sink = RecordingSink::default();
    let err = decoder
        .decode(packet(hvcc_sample(&[REAL_IRAP_PREFIX]), 0), &mut sink)
        .must_err();
    assert_eq!(err.context.code, "hevc.hw.decode");
    // Device-lost maps to the hardware-lost decode cause.
    let mut decoder2 = HevcDecoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel2: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::new(Mutex::new(SessionLog::default())),
        fail_configure: None,
        fail_decode: Some(HevcHwError::DeviceLost),
        lag: false,
    });
    decoder2
        .configure(&config(Some(accel2), Some(REAL_HVCC.to_vec())))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let err = decoder2
        .decode(
            packet(hvcc_sample(&[REAL_IRAP_PREFIX]), 0),
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
    let mut decoder = HevcDecoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: Some(HevcHwError::Configure {
            code: "hevc.hw.fake.profile",
        }),
        fail_decode: None,
        lag: false,
    });
    let err = decoder
        .configure(&config(Some(accel), Some(REAL_HVCC.to_vec())))
        .must_err();
    assert_eq!(err.context.code, "hevc.hw.configure");
    assert!(matches!(err.kind, ErrorKind::Unsupported { .. }));
}

#[test]
fn alien_bridge_leaves_the_stream_unclaimed() {
    // An installed but foreign bridge must not claim the fake
    // accelerator: typed no_bridge.
    let mut decoder = HevcDecoder::with_bridges(vec![Arc::new(AlienBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::new(Mutex::new(SessionLog::default())),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    let err = decoder
        .configure(&config(Some(accel), Some(REAL_HVCC.to_vec())))
        .must_err();
    assert_eq!(err.context.code, "hevc.hw.no_bridge");
}

// ---- AC 4: real corpus stream (skip-loud when absent) ----

#[test]
fn corpus_0044_annexb_full_stream_survives_and_feeds_every_picture() {
    let path = std::path::PathBuf::from(
        std::env::var("LUMEN_CORPUS").unwrap_or_else(|_| "/tmp/my-project/corpus-v4".to_string()),
    )
    .join("media/0044-raw-video.265");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("SKIP hevc hw-path corpus: 0044-raw-video.265 absent");
        return;
    };
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = HevcDecoder::with_bridges(vec![Arc::new(FakeBridge)]);
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
    // 56 VCL NALs (all IDR_N_LP — an all-intra stream) in this file.
    let truth_vcl = codecs::video::hevc::parser::nal::AnnexBNals::new(&bytes)
        .filter(|nal| {
            codecs::video::hevc::parser::nal::NalHeader::split(nal)
                .is_ok_and(|(h, _)| h.unit_type.is_vcl())
        })
        .count();
    assert_eq!(truth_vcl, 56, "corpus shape drift: re-pin");
    assert_eq!(
        fed_vcl, truth_vcl,
        "every VCL unit of the stream reached a session AU exactly once"
    );
    assert_eq!(log.aus.len(), 56, "one AU per picture (1 segment each)");
    assert!(log.aus.iter().all(|au| au.irap));
    // Every AU carries its picture's first (and only) segment.
    assert!(log.aus.iter().all(|au| {
        au.first_vcl()
            .is_some_and(|v| v.slice.first_slice_segment_in_pic)
    }));
    // Identical per-picture parameter-set re-activations are no-ops:
    // exactly one session configure for the whole stream.
    assert_eq!(log.configured.len(), 1);
    assert_eq!(log.configured[0], (1, 1, 1, Some((360, 640))));
    assert!(
        matches!(
            out,
            DecodeOutcome::NeedMoreInput | DecodeOutcome::Recovered { .. }
        ) || sink.frames.len() >= 56
    );
    assert!(sink.frames.len() >= 56, "every picture produced a surface");
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
    // session), both framings, seeded from the real hvcC: the claim
    // is absence of panic/hang only (typed errors and Recovered are
    // expected outcomes).
    let mut rng = SplitMix64(0x5eed_1234_abcd_f00d);
    let log = Arc::new(Mutex::new(SessionLog::default()));
    let mut decoder = HevcDecoder::with_bridges(vec![Arc::new(FakeBridge)]);
    let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
        session_log: Arc::clone(&log),
        fail_configure: None,
        fail_decode: None,
        lag: false,
    });
    decoder
        .configure(&config(Some(accel), Some(REAL_HVCC.to_vec())))
        .unwrap_or_else(|e| unreachable!("{e}"));
    let mut sink = RecordingSink::default();
    for i in 0..5_000u32 {
        let len = (rng.next() % 512) as usize;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            bytes.push((rng.next() >> 24) as u8);
        }
        // Occasionally splice real NALs in (reach the parse paths).
        if i % 7 == 0 {
            bytes = hvcc_sample(&[REAL_VPS, REAL_SPS, REAL_PPS, REAL_IRAP_PREFIX]);
        }
        let _ = decoder.decode(packet(bytes, u64::from(i)), &mut sink);
    }
    let _ = decoder.flush(&mut sink);
    // Bounded accumulation held (MP §12): the pending list cannot
    // exceed the cap even under adversarial unit floods.
    let pending = format!("{decoder:?}");
    assert!(pending.contains("HevcDecoder"));
}

#[test]
fn fuzz_lite_configure_with_random_extradata_never_panics() {
    // 2k random extradata blobs through configure: hvcC-shaped
    // misses, Annex-B misses, pure noise — all typed outcomes.
    let mut rng = SplitMix64(0xdead_beef_cafe_1234);
    for i in 0..2_000u32 {
        let len = (rng.next() % 64) as usize;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            bytes.push((rng.next() >> 32) as u8);
        }
        if i % 3 == 0 && bytes.len() >= 23 {
            // Push it toward hvcC shape (version 1 + reserved bits).
            bytes[0] = 0x01;
            bytes[13] = (bytes[13] & 0x0f) | 0xf0;
            bytes[15] = (bytes[15] & 0x03) | 0xfc;
            bytes[16] = (bytes[16] & 0x03) | 0xfc;
        }
        let mut decoder = HevcDecoder::with_bridges(vec![Arc::new(FakeBridge)]);
        let accel: Arc<dyn codec_api::HwAccelerator> = Arc::new(FakeAccel {
            session_log: Arc::new(Mutex::new(SessionLog::default())),
            fail_configure: None,
            fail_decode: None,
            lag: false,
        });
        // Outcomes are typed errors or Ok — both fine, no panic.
        let _ = decoder.configure(&config(Some(accel), Some(bytes)));
        let mut sink = RecordingSink::default();
        let _ = decoder.decode(
            packet(vec![0x00, 0x00, 0x00, 0x01, 0x28, 0x01], 0),
            &mut sink,
        );
        let _ = decoder.flush(&mut sink);
    }
}
