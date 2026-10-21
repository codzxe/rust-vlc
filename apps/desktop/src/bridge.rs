//! The variant-B canvas bridge: the [`Presenter`] that turns the
//! engine's presentation decisions into wire records (card 2.D3;
//! doc 08 §7 candidate B; ADR-003 decision 2).
//!
//! The scheduler still owns *which frame is on screen when* — the
//! bridge is a transport, not a policy: `present()` serializes the
//! request's frame and hands the record to the
//! [`FrameTransport`], never reordering, never blocking, never
//! deciding. The canvas side (`prototype/lumen-frames.mjs`) is the
//! other half of the same contract.
//!
//! # Repeats: two entry points, one reason
//!
//! The [`Presenter`] trait receives an undifferentiated
//! `PresentRequest` — the repeat vocabulary lives in the wiring's
//! [`PresentAction`](video_engine::PresentAction) match. Guessing
//! "repeat" from slot equality would be exactly that (a guess: real
//! streams carry duplicate PTS). So the bridge exposes both entries:
//! [`Presenter::present`] for fresh slots and
//! [`CanvasBridge::present_repeat`] for `PresentAction::RepeatLast`
//! — the desktop wiring (and the harness) match on the action and
//! call the honest one. v1 re-ships the payload on repeats (correct
//! everywhere, no differential records); the wire flag lets the
//! consumer skip re-uploading identical pixels
//! `[PROPOSAL: v2 repeat-without-payload record]`.
//!
//! # Timestamp honesty
//!
//! [`PresentInfo::presented_at`] echoes the request's scheduled slot,
//! exactly like 2.D1's renderer: the *actual* draw instant is the
//! webview compositor's fact, observed downstream through the
//! mailbox's accounting (`emitted_ns` → `drawn_ns`) — never invented
//! here. The record's `emitted_ns` comes from the injected
//! [`EmitClock`]: virtual in the harness, monotonic wall in
//! production ([`MonotonicEmitClock`]).

use std::time::Instant;

use media_types::EngineError;
use media_types::frame::{FrameStorage, VideoFrame};
use video_engine::{PresentInfo, PresentRequest, Presenter};

use crate::error::WireError;
use crate::transport::{FrameTransport, TransportError};
use crate::wire::{FormatCode, FrameFlags, FrameHeader, encode_into};

/// The bridge's serialize clock (latency accounting origin).
pub trait EmitClock: Send {
    /// The current instant, nanoseconds on this clock's own base.
    fn emit_ns(&self) -> u64;
}

/// The production emit clock: [`Instant`] nanoseconds since bridge
/// construction (monotonic, process-local — the cross-boundary origin
/// handshake with the webview's `performance.now()` is protocol v2
/// `[PROPOSAL]`).
#[derive(Debug)]
pub struct MonotonicEmitClock {
    origin: Instant,
}

impl Default for MonotonicEmitClock {
    fn default() -> MonotonicEmitClock {
        MonotonicEmitClock {
            origin: Instant::now(),
        }
    }
}

impl MonotonicEmitClock {
    /// A clock anchored *now*.
    #[must_use = "pure constructor"]
    pub fn new() -> MonotonicEmitClock {
        MonotonicEmitClock::default()
    }
}

impl EmitClock for MonotonicEmitClock {
    fn emit_ns(&self) -> u64 {
        // Saturating cast: 2^64 ns ≈ 584 years of monotonic uptime —
        // the clamp is unreachable in practice and documented, not
        // panicked over.
        u64::try_from(self.origin.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

/// Observable bridge accounting (throughput evidence).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BridgeStats {
    /// Records shipped (presents + repeats).
    pub frames_shipped: u64,
    /// Records carrying the repeat flag.
    pub repeats: u64,
    /// Bytes shipped (headers + payloads).
    pub bytes_shipped: u64,
}

/// The variant-B presenter (see the module docs).
pub struct CanvasBridge<T: FrameTransport> {
    transport: T,
    clock: Box<dyn EmitClock>,
    next_frame_id: u64,
    stats: BridgeStats,
    /// Reused encode buffer (one record's worth of scratch — the
    /// transport copies what it keeps; the hot path allocates once).
    scratch: Vec<u8>,
}

impl<T: FrameTransport> std::fmt::Debug for CanvasBridge<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CanvasBridge")
            .field("transport", &self.transport.name())
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl<T: FrameTransport> CanvasBridge<T> {
    /// Build a bridge over a transport, with the production
    /// monotonic emit clock.
    #[must_use = "pure constructor"]
    pub fn new(transport: T) -> CanvasBridge<T> {
        CanvasBridge::with_clock(transport, Box::new(MonotonicEmitClock::new()))
    }

    /// Build with an injected emit clock (the harness's virtual
    /// time).
    #[must_use = "pure constructor"]
    pub fn with_clock(transport: T, clock: Box<dyn EmitClock>) -> CanvasBridge<T> {
        CanvasBridge {
            transport,
            clock,
            next_frame_id: 0,
            stats: BridgeStats::default(),
            scratch: Vec::new(),
        }
    }

    /// The transport (for consumer-side access — e.g. the
    /// [`FrameMailbox`](crate::mailbox::FrameMailbox) behind a
    /// [`MailboxTransport`](crate::transport::MailboxTransport)).
    #[must_use = "the seam's other half"]
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Mutable transport access (consumer side).
    #[must_use = "the seam's other half"]
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// The accounting so far.
    #[must_use = "observable evidence"]
    pub fn stats(&self) -> BridgeStats {
        self.stats
    }

    /// The emit clock's current reading (diagnostics).
    #[must_use = "observable evidence"]
    pub fn now_ns(&self) -> u64 {
        self.clock.emit_ns()
    }

    /// The `PresentAction::RepeatLast` entry: ship the previous slot
    /// again, flag set (see the module docs — the trait entry cannot
    /// know, the wiring does).
    ///
    /// # Errors
    /// The [`Presenter::present`] family (serialization + transport,
    /// stage `Present`).
    pub fn present_repeat(&mut self, request: &PresentRequest) -> Result<PresentInfo, EngineError> {
        self.ship(request, true)
    }

    /// Serialize + ship one record (the shared hot path).
    fn ship(&mut self, request: &PresentRequest, repeat: bool) -> Result<PresentInfo, EngineError> {
        self.serialize(request, repeat).map_err(wire_to_engine)?;
        self.stats.frames_shipped += 1;
        if repeat {
            self.stats.repeats += 1;
        }
        self.stats.bytes_shipped += self.scratch.len() as u64;
        self.transport
            .send(&self.scratch)
            .map_err(transport_to_engine)?;
        // Slot echo: the draw instant is downstream's fact (module
        // docs — timestamp honesty).
        Ok(PresentInfo {
            presented_at: request.when,
        })
    }

    /// Serialize one request into `self.scratch`.
    fn serialize(
        &mut self,
        request: &PresentRequest,
        repeat: bool,
    ) -> Result<FrameHeader, WireError> {
        let frame: &VideoFrame = &request.frame;
        let (format, payload): (FormatCode, &[u8]) = match (&frame.data, frame.format) {
            (FrameStorage::Cpu(bytes), format) => {
                let code =
                    FormatCode::for_pixel_format(format).ok_or(WireError::UnknownFormatCode {
                        // The YUV family: the code table mirrors the
                        // renderer's upload path; converting here would
                        // duplicate the GPU shader card's job. Hardware
                        // handles need the renderer's import path
                        // (variant A territory); the bridge is the
                        // CPU-frame transport of v1.
                        found: 0,
                    })?;
                (code, bytes.as_slice())
            }
            (FrameStorage::OpaqueHandle(_), _) => {
                return Err(WireError::UnknownFormatCode { found: 0 });
            }
        };
        let payload_len =
            u32::try_from(payload.len() as u64).map_err(|_| WireError::BadGeometry {
                reason: "payload exceeds the u32 length field",
            })?;
        let header = FrameHeader {
            format,
            flags: FrameFlags { repeat },
            width: frame.size.0,
            height: frame.size.1,
            payload_len,
            frame_id: self.next_frame_id,
            pts_ns: request.when.ticks,
            emitted_ns: self.clock.emit_ns(),
        };
        self.scratch.clear();
        encode_into(&header, payload, &mut self.scratch)?;
        self.next_frame_id = self.next_frame_id.wrapping_add(1);
        Ok(header)
    }
}

impl<T: FrameTransport> Presenter for CanvasBridge<T> {
    fn present(&mut self, request: &PresentRequest) -> Result<PresentInfo, EngineError> {
        self.ship(request, false)
    }
}

/// Map the wire codec's data errors into the engine vocabulary
/// (stage `Present`, typed codes — the same discipline as 2.D1's
/// renderer).
fn wire_to_engine(err: WireError) -> EngineError {
    use media_types::error::{CorruptDetail, ErrorContext, ErrorKind, PipelineStage};
    let code: &'static str = match &err {
        WireError::UnknownFormatCode { .. } => "render.stream.format-unsupported",
        WireError::BadGeometry { .. } | WireError::PayloadLengthMismatch { .. } => {
            "render.stream.geometry"
        }
        WireError::BadMagic { .. }
        | WireError::UnsupportedVersion { .. }
        | WireError::BadHeaderLen { .. }
        | WireError::ReservedNotZero { .. }
        | WireError::Truncated { .. } => "render.stream.malformed",
    };
    EngineError::new(
        PipelineStage::Present,
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
        ErrorContext::new(code),
    )
    .with_source(err)
}

/// Map transport failures into the engine vocabulary (device-flavored:
/// the webview/IPC channel is the device here).
fn transport_to_engine(err: TransportError) -> EngineError {
    use media_types::error::{ErrorContext, ErrorKind, PipelineStage};
    EngineError::new(
        PipelineStage::Present,
        ErrorKind::Device {
            which: media_types::error::DeviceSlot::Display,
            cause: media_types::error::DeviceCause::Disconnected,
        },
        ErrorContext::new("render.stream.transport"),
    )
    .with_source(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::CaptureTransport;
    use crate::wire::WireDecoder;
    use media_types::PixelFormat;
    use media_types::frame::ColorDescription;
    use media_types::packet::SharedBytes;
    use media_types::time::{Stamp, Timestamp};
    use media_types::{GenerationId, error::CorruptDetail};
    use testkit::{Must, assert_err};

    /// A fixed virtual emit clock (deterministic stamps).
    struct FixedClock(u64);
    impl EmitClock for FixedClock {
        fn emit_ns(&self) -> u64 {
            self.0
        }
    }

    fn request(pts_ns: i64, w: u32, h: u32) -> PresentRequest {
        let pixels: Vec<u8> = (0..w * h * 4).map(|i| (i % 253) as u8).collect();
        PresentRequest {
            frame: VideoFrame {
                format: PixelFormat::Rgba8,
                size: (w, h),
                data: FrameStorage::Cpu(SharedBytes::from_vec(pixels)),
                pts: Stamp::Present(Timestamp::new(pts_ns, video_engine::NANO_BASE)),
                color: ColorDescription::BT709_LIMITED,
                interlaced: false,
                generation: GenerationId::FIRST,
            },
            when: Timestamp::new(pts_ns, video_engine::NANO_BASE),
            generation: GenerationId::FIRST,
        }
    }

    #[test]
    fn present_ships_a_decodable_record_and_echoes_the_slot() {
        let transport = CaptureTransport::new();
        let mut bridge = CanvasBridge::with_clock(transport, Box::new(FixedClock(1_234)));
        let req = request(5_000_000, 2, 2);
        let info = bridge.present(&req).must();
        assert_eq!(info.presented_at, req.when);

        let records = bridge.transport().records();
        assert_eq!(records.len(), 1);
        let mut decoder = WireDecoder::new();
        decoder.feed(&records[0]);
        let frame = decoder.next_frame().must().must();
        assert_eq!(frame.header.frame_id, 0);
        assert_eq!(frame.header.pts_ns, 5_000_000);
        assert_eq!(frame.header.emitted_ns, 1_234);
        assert!(!frame.header.flags.repeat);
        // Payload is the frame bytes, byte-exact.
        match &req.frame.data {
            FrameStorage::Cpu(bytes) => assert_eq!(frame.payload, bytes.as_slice()),
            FrameStorage::OpaqueHandle(_) => unreachable!("test builds CPU frames"),
        }
        let stats = bridge.stats();
        assert_eq!(stats.frames_shipped, 1);
        assert_eq!(stats.repeats, 0);
        assert_eq!(stats.bytes_shipped, records[0].len() as u64);
    }

    #[test]
    fn repeat_entry_flags_the_record_and_reships_the_payload() {
        let transport = CaptureTransport::new();
        let mut bridge = CanvasBridge::with_clock(transport, Box::new(FixedClock(10)));
        bridge.present(&request(10_000, 2, 1)).must();
        // The wiring's RepeatLast path: same request, flag set.
        bridge.present_repeat(&request(10_000, 2, 1)).must();
        let stats = bridge.stats();
        assert_eq!(stats.frames_shipped, 2);
        assert_eq!(stats.repeats, 1);
        let records = bridge.transport().records();
        let mut decoder = WireDecoder::new();
        decoder.feed(&records[1]);
        let frame = decoder.next_frame().must().must();
        assert!(frame.header.flags.repeat);
        assert_eq!(frame.header.frame_id, 1);
        assert_eq!(frame.payload.len(), 8);
    }

    #[test]
    fn yuv_and_opaque_frames_are_typed_refusals() {
        let mut bridge = CanvasBridge::new(CaptureTransport::new());
        let mut yuv = request(1_000, 2, 2);
        yuv.frame.format = PixelFormat::Nv12;
        yuv.frame.data = FrameStorage::Cpu(SharedBytes::from_vec(vec![0; 6]));
        let err = assert_err(bridge.present(&yuv), "Nv12 is the GPU shader path");
        assert_eq!(err.context.code, "render.stream.format-unsupported");

        let mut hw = request(1_000, 2, 2);
        hw.frame.data = FrameStorage::OpaqueHandle(9);
        let err = assert_err(
            bridge.present(&hw),
            "hardware handles need the renderer import",
        );
        assert_eq!(err.context.code, "render.stream.format-unsupported");
        // Nothing shipped for either.
        assert_eq!(bridge.stats().frames_shipped, 0);
    }

    #[test]
    fn truncated_frames_are_refused_not_padded() {
        let mut bridge = CanvasBridge::new(CaptureTransport::new());
        let mut short = request(1_000, 4, 4);
        short.frame.data = FrameStorage::Cpu(SharedBytes::from_vec(vec![0; 8]));
        let err = assert_err(bridge.present(&short), "decode corruption, never guessed");
        assert_eq!(err.context.code, "render.stream.geometry");
        assert!(matches!(
            err.kind,
            media_types::ErrorKind::Corrupt {
                detail: CorruptDetail::InvalidStructure { .. }
            }
        ));
    }

    #[test]
    fn monotonic_emit_clock_advances() {
        let clock = MonotonicEmitClock::new();
        let a = clock.emit_ns();
        // Two consecutive reads are non-decreasing (monotonic by
        // construction; the assert is the documented contract).
        let b = clock.emit_ns();
        assert!(b >= a, "monotonic emit clock went backwards: {a} -> {b}");
    }
}
