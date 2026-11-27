//! PGS (HDMV) embedded-subtitle decoding (FX9-A04, wave FX9; FIX-PLAN
//! §5 F3.1): one TS PES payload per packet → one PGS display set →
//! [`Frame::Subtitle`](media_types::Frame::Subtitle) overlay cues.
//!
//! # The reduce path
//!
//! The `subtitles` crate (L2) owns the whole format grammar; this
//! decoder is the engine adapter:
//!
//! 1. [`PgsDecoder::decode_display_set`](subtitles::pgs::PgsDecoder::decode_display_set)
//!    consumes the packet's bytes as one display-set payload. The
//!    segment framing (`.sup`-style PTS-prefixed or
//!    Matroska-stripped) is detected and stripped by the
//!    [`PgsFraming`](subtitles::pgs::segment::PgsFraming) walk inside
//!    that call — this adapter never parses segment headers itself.
//!    The packet's own PTS (in any time base) is rescaled to the 90
//!    kHz carriage base and handed in alongside.
//! 2. [`PgsCueBuilder::cues`](subtitles::pgs::PgsCueBuilder::cues)
//!    turns the composed display-set objects into compositing cues
//!    (exact-position layouts over the PCS video extent, PCS object
//!    order as z).
//! 3. Each cue reduces to a
//!    [`SubtitleCue::bitmap`](media_types::frame::SubtitleCue::bitmap):
//!    bitmap → [`SubtitleOverlay`](media_types::frame::SubtitleOverlay)
//!    (w/h/rgba verbatim; x/y recovered exactly from the anchor ×
//!    reference extent), span from the cue's nanosecond interval.
//!
//! # Timing: the held display set
//!
//! PGS has no explicit end times — a subtitle stays up until the next
//! display set replaces or clears it. A subtitle frame that carried
//! an open-ended span for EVERY set would push the closing decision
//! into the consumer; instead this decoder holds each display set
//! until the next set's PTS is known and emits it then, half-open
//! `[t0, next - 1 ns]` (the [`PgsCueBuilder`](subtitles::pgs::PgsCueBuilder)
//! rule). The trailing set is emitted open-ended by [`Decoder::flush`].
//!
//! # Damage model (doc 03 §4.5(d))
//!
//! Every malformation the L2 grammar can name is typed data in the
//! display set's damage ledger — never a panic, never a fatal error:
//! a damaged-but-composed set is held and reported as
//! [`Recovered`](codec_api::DecodeOutcome::Recovered) with
//! [`ConcealedFrame`](codec_api::RecoverReason::ConcealedFrame); a
//! dropped set (damage with zero composed objects, e.g. a missing
//! palette) reports `Recovered { 1, CorruptInput }` and holds
//! nothing (blank from its PTS on). A zero-object set with a clean
//! ledger is the composition's own clear — a real display set, held
//! and emitted like any other. Empty or truncated packets are the
//! same typed-damage path, never an error.
//!
//! # Bounds (MP §12)
//!
//! Bounded work per call: one packet in, at most one frame out, one
//! display set held. Memory is the L2 caps (object canvases, the
//! 64 MiB composed-RGBA retention cap, the damage ledger cap) plus
//! the one held set.
//!
//! # Never
//!
//! I/O, unsafe, panics, unbounded allocation (this crate's
//! crate-wide conventions; the L2 fuzz harness
//! `fuzz_sub_pgs` hammers the grammar underneath).

use codec_api::prelude::*;
use media_types::frame::{SubtitleCue, SubtitleFrame, SubtitleOverlay};
use media_types::time::{Stamp, TimeBase, TimeSpan, Timestamp};
use subtitles::compositing::bitmap::CompositedCue;
use subtitles::compositing::layout::Rational;
use subtitles::pgs::{PgsCueBuilder, PgsDecoder, PgsDisplaySet};

/// The codec descriptor this decoder answers to — exactly the mint
/// the TS demuxer produces for stream type `0x90`
/// (`demux::ts::TsStream::descriptor`, the FX9-A03 `decoder_for`
/// interlock).
pub const DESCRIPTOR: &str = "pgssub";

/// The 90 kHz base MPEG carriage stamps PGS in (exact: `num`/`den`
/// coprime and non-zero, so this literal is `TimeBase::new(1,
/// 90_000)`'s Ok value — the same construction doctrine as the
/// subtitles crate's nanosecond base below).
const KHZ_90: TimeBase = TimeBase {
    num: 1,
    den: 90_000,
};

/// The 1/1e9 nanosecond base the compositing-cue reduce speaks — the
/// L2 PGS builder's construction, reused verbatim.
const NANOS: TimeBase = TimeBase {
    num: 1,
    den: 1_000_000_000,
};

/// Typed decode-stage failures (the PCM taxonomy pattern): only
/// configuration-time refusals and contract misuse are fatal — all
/// byte-level damage inside a display set is typed ledger data,
/// surfaced as [`DecodeOutcome::Recovered`], never an error.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PgsError {
    /// A configuration outside this decoder's vocabulary (a codec
    /// descriptor other than [`DESCRIPTOR`]). `code` names the exact
    /// rule; `detail` rides in the error's `source` chain.
    Unsupported {
        code: &'static str,
        detail: String,
    },
    /// `decode`/`flush` before a successful `configure` (contract
    /// misuse — typed, never a panic).
    BadState,
    /// A display set the L2 cue builder refused to reduce (unreachable
    /// by construction: the decoder only hands it pre-validated
    /// objects; typed defensively, never a panic).
    Unbuildable {
        code: &'static str,
        detail: String,
    },
}

impl PgsError {
    /// Convert to the public contract error (stage `Decode`; the
    /// `PcmError::into_engine_error` pattern).
    fn into_engine_error(self) -> EngineError {
        let stage = media_types::PipelineStage::Decode;
        match self {
            PgsError::Unsupported { code, detail } => EngineError::new(
                stage,
                media_types::ErrorKind::Unsupported {
                    format: Some(media_types::error::FormatInfo {
                        name: Some(DESCRIPTOR.to_string()),
                        sniffed: Vec::new(),
                    }),
                },
                media_types::error::ErrorContext::new(code),
            )
            .with_source(Detail(detail)),
            PgsError::Unbuildable { code, detail } => EngineError::new(
                stage,
                media_types::ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::Bitstream,
                },
                media_types::error::ErrorContext::new(code),
            )
            .with_source(Detail(detail)),
            PgsError::BadState => EngineError::new(
                stage,
                media_types::ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState,
                },
                media_types::error::ErrorContext::new("pgs.not_configured"),
            ),
        }
    }
}

/// Chain-root carrier for the human-readable detail (keeps it in the
/// error's `source` without leaking into `Display`'s no-secrets
/// surface — the 2.B4 `FeatureDetail` pattern verbatim).
#[derive(Debug, Clone)]
struct Detail(String);

impl std::fmt::Display for Detail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Detail {}

/// The PGS display-set decoder for the embedded TS subtitle lane:
/// the L2 epoch state machine + cue builder behind the
/// [`Decoder`](codec_api::Decoder) contract, reduced to
/// [`Frame::Subtitle`] overlay cues.
///
/// One instance per subtitle limb; the engine recycles instances
/// across generations (the decoder cache), and [`Decoder::configure`]
/// resets the epoch so a recycled instance never presents the
/// previous stream's objects.
#[derive(Debug, Clone)]
pub struct PgsSubtitleDecoder {
    /// The L2 epoch state machine (display-set grammar + banks).
    pgs: PgsDecoder,
    /// The L2 compositing-cue builder (stateless; the timing rule's
    /// one home).
    builder: PgsCueBuilder,
    /// The configured generation (the staleness watermark).
    generation: Option<GenerationId>,
    /// The display set held while its closing PTS is unknown.
    pending: Option<PendingSet>,
}

/// A decoded display set awaiting the next set's PTS.
#[derive(Debug, Clone)]
struct PendingSet {
    /// The composed set (objects, extent, PTS).
    set: PgsDisplaySet,
    /// The generation of the packet that produced the set (the
    /// emitted frame's own stamp).
    generation: GenerationId,
}

impl Default for PgsSubtitleDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl PgsSubtitleDecoder {
    /// A fresh decoder, unconfigured.
    pub fn new() -> PgsSubtitleDecoder {
        PgsSubtitleDecoder {
            pgs: PgsDecoder::new(),
            builder: PgsCueBuilder::new(),
            generation: None,
            pending: None,
        }
    }

    /// Reduce one held display set to a subtitle frame, closing its
    /// span against `next_pts_ns` when known (see the module docs'
    /// timing model — `next_pts_ns` is the NEXT set's PTS, `None`
    /// leaves the span open-ended).
    fn build_frame(
        &self,
        pending: PendingSet,
        next_pts_ns: Option<i64>,
    ) -> Result<Frame, EngineError> {
        let cues = self
            .builder
            .cues(&pending.set, next_pts_ns, 0)
            .map_err(|err| {
                PgsError::Unbuildable {
                    code: "pgs.cue_build",
                    detail: format!("display set refused by the cue builder: {err}"),
                }
                .into_engine_error()
            })?;
        let cues = cues.into_iter().map(overlay_cue).collect::<Vec<_>>();
        Ok(Frame::Subtitle(SubtitleFrame {
            cues,
            pts: ns_stamp(pending.set.pts_ns),
            generation: pending.generation,
        }))
    }
}

impl Decoder for PgsSubtitleDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        let descriptor = cfg.codec.as_str();
        if descriptor != DESCRIPTOR {
            return Err(PgsError::Unsupported {
                code: "pgs.codec_descriptor",
                detail: format!(
                    "codec descriptor `{descriptor}` is not `{DESCRIPTOR}` (the TS demuxer's \
                     stream-type-0x90 mint)"
                ),
            }
            .into_engine_error());
        }
        // A configure marks a stream generation (fresh open, seek,
        // source switch): the previous epoch's banks and the held
        // set describe the OLD timeline — release both, so a recycled
        // decoder instance never presents stale objects (the L2
        // `PgsDecoder::reset` doctrine).
        self.pgs.reset();
        self.pending = None;
        self.generation = Some(cfg.generation);
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        let generation = match self.generation {
            Some(generation) => generation,
            None => return Err(PgsError::BadState.into_engine_error()),
        };
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        // A discontinuity inside the same generation (seek, splice):
        // the epoch's banks and the held set belong to the old
        // timeline — the held set's closing PTS would cite the wrong
        // clock, so drop it (counted) and rebuild the epoch from this
        // packet on.
        let discontinuity_skipped = if packet.discontinuity {
            let held = self.pending.take().is_some();
            self.pgs.reset();
            u32::from(held)
        } else {
            0
        };
        let pts_90khz = stamp_to_90khz(&packet.pts);
        let set = self.pgs.decode_display_set(packet.data.as_slice(), pts_90khz);
        let damaged = !set.damage.is_empty();
        // Damage with zero composed objects: the set was dropped whole
        // (missing palette, truncation past every segment) — blank
        // from its PTS on, nothing to hold. A zero-object set with a
        // CLEAN ledger is the composition's clear, held like any set.
        let dropped = damaged && set.objects.is_empty();
        let mut emitted = 0u32;
        if let Some(pending) = self.pending.take() {
            out.accept(self.build_frame(pending, set.pts_ns)?)?;
            emitted += 1;
        }
        if !dropped {
            self.pending = Some(PendingSet {
                set,
                generation: packet.generation,
            });
        }
        if discontinuity_skipped > 0 {
            return Ok(DecodeOutcome::Recovered {
                skipped: discontinuity_skipped,
                reason: RecoverReason::Discontinuity,
            });
        }
        if dropped {
            return Ok(DecodeOutcome::Recovered {
                skipped: 1,
                reason: RecoverReason::CorruptInput,
            });
        }
        if damaged {
            return Ok(DecodeOutcome::Recovered {
                skipped: 0,
                reason: RecoverReason::ConcealedFrame,
            });
        }
        if emitted > 0 {
            return Ok(DecodeOutcome::Emitted { frames: emitted });
        }
        Ok(DecodeOutcome::NeedMoreInput)
    }

    fn flush(&mut self, out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        if self.generation.is_none() {
            return Err(PgsError::BadState.into_engine_error());
        }
        // The trailing set's end is genuinely unknown (PGS has no
        // explicit end times): emit it open-ended, once — further
        // flushes stay Drained.
        match self.pending.take() {
            Some(pending) => {
                out.accept(self.build_frame(pending, None)?)?;
                Ok(DecodeOutcome::Emitted { frames: 1 })
            }
            None => Ok(DecodeOutcome::Drained),
        }
    }
}

/// A packet stamp to 90 kHz ticks (the base the PGS carriage stamps
/// in). `None` when the stamp is absent or cannot be rescaled — the
/// L2 layer then falls back to the PCS segment's own `.sup` PTS.
fn stamp_to_90khz(stamp: &Stamp) -> Option<i64> {
    stamp
        .map_timestamp(|ts| ts.convert(&KHZ_90).ok())
        .flatten()
        .map(|ts| ts.ticks)
}

/// Nanoseconds to a frame-vocabulary stamp (`Missing` when unknown).
fn ns_stamp(ns: Option<i64>) -> Stamp {
    match ns {
        Some(ns) => Stamp::Present(Timestamp::new(ns, NANOS)),
        None => Stamp::Missing,
    }
}

/// One compositing cue to one overlay cue: the bitmap verbatim, the
/// placement recovered from the cue's own authored extent, the z
/// carried as authored (PCS object order).
fn overlay_cue(cue: CompositedCue) -> SubtitleCue {
    let CompositedCue {
        bitmap,
        layout,
        z,
        start,
        end,
    } = cue;
    // The PGS builder authors every cue against the PCS video extent
    // (`reference: Some(video)`); a reference-less cue has no
    // authored pixel space, so its overlay pins to the frame origin.
    let (x, y) = match layout.reference {
        Some((video_w, video_h)) => (
            anchor_px(layout.anchor.u, video_w),
            anchor_px(layout.anchor.v, video_h),
        ),
        None => (0, 0),
    };
    let overlay = SubtitleOverlay {
        width: bitmap.width,
        height: bitmap.height,
        rgba: SharedBytes::from_vec(bitmap.rgba),
        x,
        y,
        z,
    };
    SubtitleCue::bitmap(
        TimeSpan {
            start: ns_stamp(start),
            end: ns_stamp(end),
        },
        overlay,
    )
}

/// The pixel coordinate a layout anchor pins in its own reference
/// extent: `num * extent / den`, exact — the anchor was built as the
/// reduced `coordinate / extent`, and reduction divides both sides by
/// the same `gcd`, so the multiply-then-divide round-trips the
/// original coordinate (never a float, never a guess; a degenerate
/// zero denominator — only reachable through the plain pub fields —
/// reads as the origin, never a panic).
fn anchor_px(rational: Rational, extent: u32) -> i32 {
    if rational.den == 0 {
        return 0;
    }
    let px = (u64::from(rational.num) * u64::from(extent)) / u64::from(rational.den);
    i32::try_from(px).unwrap_or(i32::MAX)
}
