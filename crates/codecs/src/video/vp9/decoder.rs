//! The rusty_vp9-backed VP9 decoder core (card 6.M-vp9): the engine
//! seam and the [`codec_api::Decoder`] implementation.
//!
//! # Engine admission (the record, not hope)
//!
//! **`rusty_vp9 = "=0.1.3"`, default features (an empty set)** —
//! `docs/dependencies/records/rusty_vp9-0.1.3.md`: pure Rust, zero
//! dependencies, no build.rs, no FFI, the only feature
//! (`prometheus-telemetry`) stays OFF. The record measured the
//! engine **bit-exact against ffmpeg `libvpx-vp9` on six lanes
//! (431/431 frames), hostile-input panic-free** — this card's
//! differential suite (card AC, binding admission condition b)
//! re-runs the same lanes through THIS wrapper.
//!
//! # The seam (all safe Rust)
//!
//! Unlike rav1d (whose only public surface is a C ABI — the av1
//! module's sanctioned `unsafe` exception), rusty_vp9's decode
//! surface is safe Rust end to end: `Vp9Decoder::new` /
//! `push(data, pts)` / `next_frame()` / `flush()`, with
//! `Error::Again`/`Error::Eof` mirroring ffmpeg's send/receive
//! convention. This module contains **zero `unsafe`**.
//!
//! # Push/pull model and damage philosophy
//!
//! One packet = one IVF frame (the demuxer's contract; Matroska
//! WebM `V_VP9` delivery re-frames the same way). `push` splits VP9
//! superframes internally and (libvpx convention) marks only the
//! top displayable layer for output; `next_frame` decodes one
//! queued frame and returns `Ok(frame)` when it is displayable, or
//! `Err(Again)` when it was consumed but hidden (an alt-ref, or a
//! lower spatial layer superseded inside its superframe).
//!
//! Frame-scoped isolation (doc 03 §4.5(d), the av1 model): a frame
//! the engine rejects has already been consumed from its queue —
//! the damage is counted (a `Recovered` outcome, reason
//! `CorruptInput`), the drain continues, and the next packet
//! decodes against whatever reference state survived (the admission
//! record's garbage-then-real lane measured this recovery: 10
//! frames out after a leading garbage packet). The one recorded
//! concealment-equivalence exception (record §H): a *truncated*
//! keyframe may decode to a full frame (the boolean decoder
//! end-fills, like libvpx's error concealment) — that is an honest
//! `Ok`, not detectable at this seam.
//!
//! # Output boundary (honest v0)
//!
//! 8-bit 4:2:0 → [`media_types::PixelFormat::I420`], planes copied
//! row-by-row from the engine's strided buffers into a pooled
//! tight slot. Bit depths ≠ 8 (`vp9.bitdepth`) and chroma ≠ 4:2:0
//! (`vp9.layout`) are typed refusals at the emit seam —
//! [`media_types::PixelFormat`] has no 10/12-bit planar vocabulary
//! yet (the av1 module's recorded follow-up); the engine itself
//! decodes 8/10/12-bit and 4:2:0/4:2:2/4:4:4 fine (the record's
//! lane F measured 10-bit profile-2 bit-exact at the engine
//! level). Colorimetry: the engine's `DecodedFrame` does not
//! surface the bitstream's color config, so frames carry the
//! conservative BT.709-limited default (the av1 module's documented
//! assumption for unsignaled color — the renderer owns the
//! fallback rendering).
//!
//! # Delivery plumbing
//!
//! The landed IVF demuxer (`demux::ivf`) maps `VP90` → descriptor
//! `vp9` → these packets; Matroska/WebM `V_VP9` maps to the same
//! descriptor (`demux::mkv` header table) — container wiring is the
//! parallel container cards' surface, reconciled by the Lead.

use codec_api::prelude::*;
use media_types::frame::{ColorDescription, Frame, FrameStorage, VideoFrame};
use media_types::packet::SharedBytes;
use media_types::{EngineError, GenerationId, PixelFormat, Stamp, TimeBase, Timestamp};

use super::error::{corrupt, unsupported, Vp9Error};

/// Pooled output slots (the MJPEG/av1 `FRAME_SLOTS` posture): enough
/// for a renderer holding a few frames while late frames chase an
/// alt-ref lag, without per-frame allocation in steady state.
const FRAME_SLOTS: usize = 4;

/// The engine type (aliased to keep the wrapper's own name clear).
type Engine = rusty_vp9::Vp9Decoder;

/// The VP9 decoder behind the codec-api [`Decoder`] contract.
///
/// Stateful (VP9 is inter-frame), damage-bounded per frame. `flush`
/// latches the engine's end-of-input and drains every frame still
/// queued (hidden alt-refs decode as they are consumed; only their
/// displayable successors emit).
#[derive(Default)]
pub struct Vp9Decoder {
    /// The configured stream generation (None until `configure`).
    generation: Option<GenerationId>,
    /// The engine (opened at configure).
    engine: Option<Engine>,
    /// The timeline base of the most recent stamped packet — frames
    /// emitted during a later packet's drain can carry an earlier
    /// packet's pts (alt-ref lag), but the timeline base is a
    /// per-stream constant in practice; `None` until a stamped
    /// packet is seen.
    base: Option<TimeBase>,
    /// Pooled output slots (the MJPEG/av1 pattern, module docs).
    frame_slots: Vec<SharedBytes>,
    /// Round-robin cursor into `frame_slots`.
    slot_next: usize,
}

impl Vp9Decoder {
    /// A fresh decoder, unconfigured.
    pub fn new() -> Vp9Decoder {
        Vp9Decoder::default()
    }

    /// The codec descriptors this decoder accepts (the demuxers'
    /// honest `vp9` name; `VP90` accepted as the fourcc spelling).
    fn accepts(descriptor: &str) -> bool {
        matches!(descriptor, "vp9" | "VP90")
    }

    /// Fold accumulated per-call damage into an outcome (the av1
    /// model: counted, resynced, never fatal).
    fn take_damage(outcome: &mut DecodeOutcome, damage: u32) {
        if damage > 0 {
            let mut skipped = damage;
            if let DecodeOutcome::Recovered { skipped: s, .. } = *outcome {
                skipped = skipped.saturating_add(s);
            }
            *outcome = DecodeOutcome::Recovered {
                skipped,
                reason: RecoverReason::CorruptInput,
            };
        }
    }

    /// Drain ready frames from the engine.
    ///
    /// Mid-stream (`stop_on_again` set), `Again` ends the drain:
    /// either the queue is empty (more input wanted) or a hidden
    /// frame was consumed — its displayable successor emits on the
    /// next packet's drain or at `flush`. After the engine's
    /// end-of-input latch, `Again` can only mark a consumed hidden
    /// frame (the empty queue reports `Eof` there), so the drain
    /// continues to `Eof`. Every other error is one damaged frame —
    /// already consumed — counted and skipped.
    fn drain(&mut self, stop_on_again: bool) -> (Vec<rusty_vp9::DecodedFrame>, u32) {
        let mut frames = Vec::new();
        let mut damage = 0u32;
        let Some(engine) = &mut self.engine else {
            return (frames, damage);
        };
        loop {
            match engine.next_frame() {
                Ok(frame) => frames.push(frame),
                Err(rusty_vp9::Error::Again) if stop_on_again => break,
                Err(rusty_vp9::Error::Again) => continue,
                Err(rusty_vp9::Error::Eof) => break,
                Err(_) => damage += 1,
            }
        }
        (frames, damage)
    }

    /// Copy one engine frame into a pooled tight-I420 slot and push
    /// the [`Frame::Video`] to `out` (the v0 output boundary, module
    /// docs: 8-bit 4:2:0 or a typed refusal).
    fn emit_frame(
        &mut self,
        frame: &rusty_vp9::DecodedFrame,
        out: &mut dyn FrameSink,
        generation: GenerationId,
    ) -> Result<(), EngineError> {
        let (w, h) = (frame.width as usize, frame.height as usize);
        if w == 0 || h == 0 {
            return Err(corrupt(
                "vp9.frame.geometry",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                format!("engine produced a {}x{} frame", frame.width, frame.height),
            )
            .into_engine_error());
        }
        // v0 output boundary (module docs): 8-bit 4:2:0 only —
        // everything else is a typed refusal, never a misread. The
        // engine decoded the frame; the wrapper cannot name the
        // layout in the media-types vocabulary yet.
        if frame.bit_depth != 8 {
            return Err(unsupported(
                "vp9.bitdepth",
                format!(
                    "{}-bit output: the media-types pixel-format vocabulary has no \
                     high-bit-depth planar variant yet (engine decodes it; the wrapper \
                     refuses to reinterpret it)",
                    frame.bit_depth
                ),
            )
            .into_engine_error());
        }
        if frame.subsampling_x != 1 || frame.subsampling_y != 1 {
            return Err(unsupported(
                "vp9.layout",
                format!(
                    "chroma subsampling {}:{}: only 4:2:0 maps onto \
                     media_types::PixelFormat::I420 in v0",
                    frame.subsampling_x, frame.subsampling_y
                ),
            )
            .into_engine_error());
        }
        if frame.planes.len() != 3 || frame.strides.len() != 3 {
            return Err(corrupt(
                "vp9.frame.planes",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                format!(
                    "engine frame carries {} planes / {} strides (expected 3/3)",
                    frame.planes.len(),
                    frame.strides.len()
                ),
            )
            .into_engine_error());
        }
        let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
        // Plane geometry must close against the declared dimensions
        // before any byte is copied (no panics on hostile geometry:
        // the stride arithmetic is checked, never wrapping).
        for plane in 0..3 {
            let (pw, ph) = if plane == 0 { (w, h) } else { (cw, ch) };
            let stride = frame.strides[plane];
            let needed = stride
                .checked_mul(ph - 1)
                .and_then(|rows| rows.checked_add(pw));
            if stride < pw || !needed.is_some_and(|need| frame.planes[plane].len() >= need) {
                return Err(corrupt(
                    "vp9.frame.plane_bounds",
                    media_types::error::CorruptDetail::InvalidStructure { offset: None },
                    format!(
                        "plane {plane}: {} bytes at stride {stride} cannot cover {pw}x{ph}",
                        frame.planes[plane].len()
                    ),
                )
                .into_engine_error());
            }
        }
        let len = w * h + 2 * cw * ch;
        let data = pooled_frame_slot(&mut self.frame_slots, &mut self.slot_next, len, |bytes| {
            for plane in 0..3 {
                let (pw, ph) = if plane == 0 { (w, h) } else { (cw, ch) };
                let stride = frame.strides[plane];
                for y in 0..ph {
                    let src = &frame.planes[plane][y * stride..y * stride + pw];
                    let at = match plane {
                        0 => y * w,
                        1 => w * h + y * cw,
                        _ => w * h + cw * ch + y * cw,
                    };
                    bytes[at..at + pw].copy_from_slice(src);
                }
            }
        })?;
        // The pts ticks rode into the engine with the packet that
        // coded this frame and come back on it (engine passthrough);
        // the timeline base is the stream's (module docs).
        let pts = match (frame.pts, self.base) {
            (Some(ticks), Some(base)) => Stamp::Present(Timestamp::new(ticks, base)),
            _ => Stamp::Missing,
        };
        let frame_out = Frame::Video(VideoFrame {
            format: PixelFormat::I420,
            size: (frame.width, frame.height),
            data: FrameStorage::Cpu(data),
            pts,
            color: ColorDescription::BT709_LIMITED,
            interlaced: false,
            generation,
        });
        out.accept(frame_out)
    }
}

impl Decoder for Vp9Decoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        let descriptor = cfg.codec.as_str();
        if !Self::accepts(descriptor) {
            return Err(unsupported(
                "vp9.codec_descriptor",
                format!("codec descriptor `{descriptor}` is not `vp9`"),
            )
            .into_engine_error());
        }
        // VP9 carries its whole configuration in-band (keyframe
        // headers); `extra_data` has nothing to say. The engine is
        // single-threaded (the admission record's honest gap — the
        // threaded fork `vp9-mt` is a separate candidate), so the
        // ThreadBudget is accepted and unused. A fresh generation
        // (seek/source switch) gets a fresh engine: stale reference
        // frames must never leak across generations.
        self.engine = Some(Engine::new());
        self.base = None;
        self.frame_slots.clear();
        self.slot_next = 0;
        self.generation = Some(cfg.generation);
        Ok(())
    }

    /// Decode one packet (one IVF frame, possibly a superframe).
    ///
    /// Frame-scoped isolation (doc 03 §4.5(d)): a `Recovered`
    /// outcome damaged at most the frames it counted — the engine
    /// consumed them, kept its reference state, and the next packet
    /// decodes normally.
    #[allow(clippy::missing_errors_doc)]
    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        let generation = match self.generation {
            Some(g) => g,
            None => return Err(Vp9Error::BadState.into_engine_error()),
        };
        if self.engine.is_none() {
            return Err(Vp9Error::BadState.into_engine_error());
        }
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        // Remember the timeline base for frame stamping (module
        // docs: emitted frames can lag their coding packet behind an
        // alt-ref, but the base is the stream's constant).
        if let Stamp::Present(ts) = packet.pts {
            self.base = Some(ts.base);
        }
        // The pts ticks ride into the engine with the packet and
        // come back on the frame they coded.
        let ticks = match packet.pts {
            Stamp::Present(ts) => Some(ts.ticks),
            Stamp::Missing => None,
        };
        if let Some(engine) = &mut self.engine {
            engine.push(packet.data.as_slice(), ticks).map_err(|err| {
                corrupt(
                    "vp9.packet.push",
                    media_types::error::CorruptDetail::InvalidStructure { offset: None },
                    format!("engine refused the packet: {err}"),
                )
                .into_engine_error()
            })?;
        }
        let (frames, damage) = self.drain(true);
        let mut emitted = 0usize;
        for frame in &frames {
            self.emit_frame(frame, out, generation)?;
            emitted += 1;
        }
        let mut outcome = if emitted == 0 {
            DecodeOutcome::NeedMoreInput
        } else {
            DecodeOutcome::Emitted {
                frames: u32::try_from(emitted).unwrap_or(u32::MAX),
            }
        };
        Self::take_damage(&mut outcome, damage);
        Ok(outcome)
    }

    fn flush(&mut self, out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        let generation = match self.generation {
            Some(g) => g,
            None => return Err(Vp9Error::BadState.into_engine_error()),
        };
        if let Some(engine) = &mut self.engine {
            // Latch end-of-input: the empty queue reports Eof from
            // here on (contract: further flush calls stay Drained).
            engine.flush();
        } else {
            return Err(Vp9Error::BadState.into_engine_error());
        }
        let (frames, damage) = self.drain(false);
        let mut emitted = 0usize;
        for frame in &frames {
            self.emit_frame(frame, out, generation)?;
            emitted += 1;
        }
        let mut outcome = if emitted == 0 {
            DecodeOutcome::Drained
        } else {
            DecodeOutcome::Emitted {
                frames: u32::try_from(emitted).unwrap_or(u32::MAX),
            }
        };
        Self::take_damage(&mut outcome, damage);
        Ok(outcome)
    }
}

/// Pooled output-slot rotation (the MJPEG/av1 `pooled_frame_slot`
/// pattern): steady-state decodes refill a slot in place; a slot
/// still owned downstream falls through to the next, and only an
/// all-busy rotation allocates.
fn pooled_frame_slot(
    slots: &mut Vec<SharedBytes>,
    next: &mut usize,
    len: usize,
    fill: impl FnOnce(&mut [u8]),
) -> Result<SharedBytes, EngineError> {
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != len {
            continue;
        }
        if let Some(bytes) = slots[idx].get_mut() {
            fill(bytes);
            *next = (idx + 1) % count;
            return Ok(slots[idx].clone());
        }
        // The slot still has downstream owners — try the next one.
    }
    let mut fresh = SharedBytes::from_vec(vec![0; len]);
    match fresh.get_mut() {
        Some(bytes) => fill(bytes),
        None => {
            // A freshly built `SharedBytes` has refcount 1 by
            // construction; `get_mut` failing here is impossible —
            // surface it loudly instead of fabricating an empty
            // frame.
            return Err(corrupt(
                "vp9.output.fresh_slot_not_exclusive",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                "a fresh slot must be exclusively owned",
            )
            .into_engine_error());
        }
    }
    if slots.len() < FRAME_SLOTS {
        slots.push(fresh.clone());
    }
    *next = 0;
    Ok(fresh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::ids::CodecDescriptor;
    use media_types::{Packet, TrackId};

    /// A collecting sink (no panics, counts accepts).
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

    fn configured() -> (Vp9Decoder, GenerationId) {
        let mut decoder = Vp9Decoder::new();
        let generation = GenerationId::FIRST
            .next()
            .unwrap_or_else(|e| unreachable!("{e}"));
        decoder
            .configure(&DecoderConfig::new(
                CodecDescriptor::new("vp9").unwrap_or_else(|e| unreachable!("{e}")),
                generation,
                ThreadBudget::default(),
                None,
                None,
            ))
            .unwrap_or_else(|e| unreachable!("configure: {e}"));
        (decoder, generation)
    }

    fn packet(generation: GenerationId, data: Vec<u8>) -> Packet {
        Packet {
            track: TrackId::new(1),
            pts: Stamp::Missing,
            dts: Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(data),
            keyframe: true,
            discontinuity: false,
            generation,
        }
    }

    #[test]
    fn wrong_descriptor_is_a_typed_refusal() {
        let mut decoder = Vp9Decoder::new();
        let generation = GenerationId::FIRST
            .next()
            .unwrap_or_else(|e| unreachable!("{e}"));
        let refused = decoder.configure(&DecoderConfig::new(
            CodecDescriptor::new("av1").unwrap_or_else(|e| unreachable!("{e}")),
            generation,
            ThreadBudget::default(),
            None,
            None,
        ));
        let err = refused.unwrap_err();
        assert_eq!(err.context.code, "vp9.codec_descriptor");
        assert_eq!(err.stage, media_types::PipelineStage::Decode);
    }

    #[test]
    fn decode_before_configure_is_typed_bad_state() {
        let mut decoder = Vp9Decoder::new();
        let generation = GenerationId::FIRST
            .next()
            .unwrap_or_else(|e| unreachable!("{e}"));
        let mut sink = Sink::default();
        let err = decoder
            .decode(packet(generation, vec![0x82]), &mut sink)
            .unwrap_err();
        assert_eq!(err.context.code, "vp9.not_configured");
        let err = decoder.flush(&mut sink).unwrap_err();
        assert_eq!(err.context.code, "vp9.not_configured");
    }

    #[test]
    fn garbage_packet_is_recovered_not_fatal() {
        let (mut decoder, generation) = configured();
        let mut sink = Sink::default();
        // 64 bytes of garbage: the engine consumes and rejects the
        // frame — a Recovered outcome, the decoder stays usable (the
        // admission record's hostile class, through the wrapper).
        let garbage: Vec<u8> = (0..64u32)
            .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
            .collect();
        let outcome = decoder
            .decode(packet(generation, garbage), &mut sink)
            .unwrap_or_else(|e| unreachable!("garbage must not be fatal: {e}"));
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
        // The decoder remains usable: flush answers Drained.
        let outcome = decoder
            .flush(&mut sink)
            .unwrap_or_else(|e| unreachable!("flush after garbage: {e}"));
        assert_eq!(outcome, DecodeOutcome::Drained);
    }

    #[test]
    fn zero_length_packet_is_recovered_not_fatal() {
        let (mut decoder, generation) = configured();
        let mut sink = Sink::default();
        let outcome = decoder
            .decode(packet(generation, Vec::new()), &mut sink)
            .unwrap_or_else(|e| unreachable!("zero-length must not be fatal: {e}"));
        assert!(
            matches!(
                outcome,
                DecodeOutcome::Recovered {
                    reason: RecoverReason::CorruptInput,
                    ..
                }
            ),
            "zero-length counts damage: {outcome:?}"
        );
    }
}
