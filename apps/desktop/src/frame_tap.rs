//! The presented-frame tap (w31.1, lane `luf1-channel`): the shell's
//! [`PresentTap`] — the seam that mirrors every frame the engine's
//! renderer presents into the bounded mailbox the pacer drains, so
//! the webview's canvas consumer rides the SAME presents the render
//! path takes (the variant-B design, `bridge.rs`'s reference
//! composition: [`CanvasBridge`] over [`MailboxTransport`]).
//!
//! # Why a tap, not a renderer replacement
//!
//! The CLI's video limb presents to its headless software renderer
//! (the evidence surface: `screenshot_png`, final-frame.png). The
//! desktop wants those SAME frames — without the pure CLI ever
//! constructing a tap (its behavior stays byte-identical with the
//! seam inert; `apps/cli`'s `set_present_tap` is the injector).
//! The tap is called on the pump thread immediately after a
//! successful present (a stale or failed present never taps), with
//! the scheduler's repeat bit, and must NEVER block the pump — the
//! mailbox's latest-wins bound absorbs a slow consumer by dropping
//! the older frame (doc 03 §4.10 c: the scheduler never stalls).
//!
//! # The drain side
//!
//! [`FrameTap::drain_records`] hands the consumer (the shell ticker,
//! once per pacer tick) every queued record re-encoded as a complete
//! LUF1 wire record (the UI's decoder is the `lumen-frames.mjs`
//! reference, byte-mirror of `wire.rs`): latest-wins means at most a
//! couple of records per tick — the ticker ships the newest and drops
//! the rest per the channel contract (one frame per tick keeps the
//! webview at the pacer's own cadence).

use std::sync::Mutex;
use std::vec::Vec;

use lumen_cli::pipeline_host::PresentTap;
use media_types::EngineError;
use video_engine::{PresentRequest, Presenter};

use crate::bridge::CanvasBridge;
use crate::mailbox::{MailboxConfig, ShippedFrame};
use crate::transport::{MailboxTransport, TransportError};
use crate::wire::{MIN_HEADER_LEN, encode_into};

/// The webview frame tap: [`CanvasBridge`] over [`MailboxTransport`]
/// behind one lock (the pump thread taps, the pacer thread drains).
pub struct FrameTap {
    bridge: Mutex<CanvasBridge<MailboxTransport>>,
}

impl FrameTap {
    /// The reference configuration (mailbox capacity 2 — one frame in
    /// flight plus one landing, `MailboxConfig`'s own default).
    ///
    /// # Errors
    /// [`MailboxError`] when the mailbox configuration is structurally
    /// invalid (never for the default).
    pub fn new() -> Result<FrameTap, TransportError> {
        let transport = MailboxTransport::new(MailboxConfig::default())?;
        Ok(FrameTap {
            bridge: Mutex::new(CanvasBridge::new(transport)),
        })
    }

    /// Drain every queued frame as complete LUF1 wire records,
    /// appended to `out` in mailbox order (oldest first; the consumer
    /// ships the LAST and drops the rest, per the one-frame-per-tick
    /// channel contract). Returns how many records were appended.
    ///
    /// Re-encoding (header + payload → record) is one memcpy — the
    /// mailbox stores decoded frames, the wire stores records, and
    /// the UI's decoder expects the record form (the
    /// `lumen-frames.mjs` byte-mirror).
    pub fn drain_records(&self, out: &mut Vec<Vec<u8>>) -> usize {
        let mut bridge = match self.bridge.lock() {
            Ok(guard) => guard,
            // A panic mid-present left the lock poisoned; the mailbox
            // state is still structurally consistent (the bridge's
            // invariants are per-record), so the drain proceeds —
            // frames must keep flowing even past a presenter bug.
            Err(poisoned) => poisoned.into_inner(),
        };
        let mailbox = bridge.transport_mut().mailbox_mut();
        let mut count = 0;
        while let Some(frame) = mailbox.draw_next() {
            if encode_record(&frame, out) {
                count += 1;
            }
        }
        count
    }

    /// The bridge's own accounting (diagnostics; `BridgeStats`).
    pub fn stats(&self) -> crate::bridge::BridgeStats {
        match self.bridge.lock() {
            Ok(guard) => guard.stats(),
            Err(poisoned) => poisoned.into_inner().stats(),
        }
    }
}

impl Default for FrameTap {
    fn default() -> Self {
        Self::new().unwrap_or_else(|err| unreachable!("the default config is valid: {err}"))
    }
}

impl PresentTap for FrameTap {
    fn tap_present(&self, request: &PresentRequest, repeat: bool) -> Result<(), EngineError> {
        let mut bridge = match self.bridge.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if repeat {
            bridge.present_repeat(request).map(|_| ())
        } else {
            bridge.present(request).map(|_| ())
        }
    }
}

/// One [`ShippedFrame`] as a complete LUF1 record appended to `out`.
/// A geometry/length inconsistency (a bridge-side bug — the encoder
/// produced the record the mailbox now holds) is SKIPPED, never
/// shipped: the consumer must not receive a record its decoder will
/// reject, and the drop is observable via the mailbox/bridge stats.
fn encode_record(frame: &ShippedFrame, out: &mut Vec<Vec<u8>>) -> bool {
    let mut record = Vec::with_capacity(frame.header.payload_len as usize + MIN_HEADER_LEN);
    match encode_into(&frame.header, &frame.payload, &mut record) {
        Ok(()) => {
            out.push(record);
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_clock::NANO_BASE;
    use media_types::GenerationId;
    use media_types::frame::{ColorDescription, FrameStorage, VideoFrame};
    use media_types::packet::SharedBytes;
    use media_types::time::{Stamp, Timestamp};
    use testkit::Must;

    /// A minimal opaque RGBA frame (4x4, red) at the given ns ticks —
    /// the presenter-arrival shape the CLI pipeline hands the renderer
    /// (host-converted RGBA; the bridge's format table is RGBA/BGRA
    /// only, the YUV family is the GPU shader path's).
    fn rgba_frame(pts_ns: i64, generation: GenerationId) -> VideoFrame {
        VideoFrame {
            format: media_types::PixelFormat::Rgba8,
            size: (4, 4),
            data: FrameStorage::Cpu(SharedBytes::from_vec([255, 0, 0, 255].repeat(4 * 4))),
            pts: Stamp::Present(Timestamp::new(pts_ns, NANO_BASE)),
            color: ColorDescription::BT709_LIMITED,
            interlaced: false,
            generation,
        }
    }

    fn request_at(pts_ns: i64, generation: GenerationId) -> PresentRequest {
        PresentRequest {
            frame: rgba_frame(pts_ns, generation),
            when: Timestamp::new(pts_ns, NANO_BASE),
            generation,
        }
    }

    /// The tap ships what the presenter presents: two taps → two LUF1
    /// records in mailbox order, each carrying the magic + header +
    /// payload (the record form the UI's `lumen-frames.mjs` decoder
    /// consumes).
    #[test]
    fn tapped_frames_drain_as_luf1_records() {
        let tap = FrameTap::new().must();
        let generation = GenerationId::FIRST;
        tap.tap_present(&request_at(100, generation), false).must();
        tap.tap_present(&request_at(200, generation), false).must();
        let mut records = Vec::new();
        assert_eq!(tap.drain_records(&mut records), 2, "both frames drain");
        for (i, record) in records.iter().enumerate() {
            assert!(
                record.starts_with(&crate::wire::MAGIC),
                "record {i} carries the LUF1 magic"
            );
            assert!(
                record.len() > MIN_HEADER_LEN,
                "record {i} carries header + payload"
            );
        }
        // Draining empties the mailbox (the drain is exhaustive).
        let mut again = Vec::new();
        assert_eq!(tap.drain_records(&mut again), 0);
    }

    /// The mailbox's latest-wins bound: a full mailbox (capacity 2)
    /// drops the OLDEST frame on offer, so a consumer that never
    /// drains still never blocks the pump.
    #[test]
    fn a_full_mailbox_drops_oldest_never_blocks() {
        let tap = FrameTap::new().must();
        let generation = GenerationId::FIRST;
        for pts_ns in 100..=106 {
            tap.tap_present(&request_at(pts_ns, generation), false)
                .must();
        }
        let mut records = Vec::new();
        let count = tap.drain_records(&mut records);
        assert_eq!(count, 2, "only the mailbox capacity survives");
        assert!(
            count < 7,
            "the pump was never blocked: all 7 offers returned"
        );
    }

    /// Repeat presents ship with the repeat flag (the header's flag
    /// byte, bit 0 — decoded through the wire's own table).
    #[test]
    fn repeat_taps_ship_the_repeat_flag() {
        let tap = FrameTap::new().must();
        let generation = GenerationId::FIRST;
        tap.tap_present(&request_at(300, generation), false).must();
        tap.tap_present(&request_at(300, generation), true).must();
        let mut records = Vec::new();
        assert_eq!(tap.drain_records(&mut records), 2);
        let mut decoder = crate::wire::WireDecoder::new();
        let first = decoder.decode_record(&records[0]).must();
        let second = decoder.decode_record(&records[1]).must();
        assert!(!first.header.flags.repeat, "the fresh present is unflagged");
        assert!(second.header.flags.repeat, "the repeat carries the flag");
    }
}
