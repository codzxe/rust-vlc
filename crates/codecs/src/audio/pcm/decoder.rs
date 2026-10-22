//! The PCM decoder core (card 2.B5): stream-facts negotiation
//! (descriptor + WAVEFORMATEX), the byte-level conversion engine, and
//! the [`codec_api::Decoder`] implementation.
//!
//! # Stream facts — how the decoder learns channels and rate
//!
//! The L0 [`Track`](media_types::ids) vocabulary deliberately carries
//! no codec parameters (doc 01 §3.1): the registry descriptor names
//! the codec, parameters ride the format-specific demuxer. Card 2.B2
//! exposed that seam as the demux crate's
//! `AviDemuxer::stream_format` (worklog 2.B2: "the decoder-config
//! seam for cards 2.B4/2.B5"). This decoder consumes the same facts
//! through the two sanctioned channels of
//! [`DecoderConfig`](codec_api::DecoderConfig):
//!
//! 1. **`extra_data` as WAVEFORMATEX** `[PROPOSAL]` — the container's
//!    own out-of-band init bytes, verbatim (AVI `strf` / WAV `fmt `:
//!    16-byte PCMWAVEFORMAT, 18-byte WAVEFORMATEX, or the ≥ 40-byte
//!    WAVEFORMATEXTENSIBLE whose SubFormat GUID names the real format
//!    — vendor tail bytes past offset 40 are legal per the MS schema
//!    and unread; the engine hands the blob on exactly as it hands AVC
//!    config records). Cross-checked against the descriptor's
//!    width/tag at `configure`.
//! 2. **[`PcmDecoder::with_format`]** — the typed-facts seam for
//!    containers whose init bytes are not WAVEFORMATEX (MP4's
//!    `AudioSampleEntry`: the adapter synthesizes the 16 bytes from
//!    channelcount/samplesize/samplerate — all present in the entry).
//!
//! When both are supplied they must agree (a conflict is a typed
//! negotiation failure, never a silent pick).
//!
//! # Damage model (doc 03 §4.5(d))
//!
//! Unpacked PCM has no entropy coding: a packet is either a whole
//! number of sample-frames or its tail is fewer bytes than one
//! sample-frame (channels × bytes-per-sample). The decodable unit is
//! the **whole-sample-frame prefix** — the largest `cut ≤ len` with
//! `cut % (channels × bytes_per_sample) == 0`. One expression serves
//! both layouts, and not by accident: for interleaved data it is the
//! largest whole number of sample-frames; for planar data it is
//! exactly the largest split into whole equal planes of whole
//! samples (a buffer that is a whole number of sample-frames is a
//! whole number of equal `bytes_per_sample`-aligned planes, and
//! conversely) — so appended tail bytes are dropped without
//! mislocating the plane boundaries.
//!
//! A non-empty prefix with a damaged tail is **concealed**: the
//! prefix is emitted and the call reports
//! [`DecodeOutcome::Recovered`](codec_api::DecodeOutcome::Recovered)
//! with [`RecoverReason::ConcealedFrame`](codec_api::RecoverReason::ConcealedFrame)
//! (output from partial data — the 2.B4 MJPEG precedent). A packet
//! with *no* decodable sample-frame is skipped whole:
//! `Recovered { skipped: 1, CorruptInput }`. Silence is never
//! fabricated to paper over missing bytes (doc 03 §4.5(a): the
//! unsupported-track rule — fabricated silence is a lie about the
//! timeline; an honest shorter frame is not).
//!
//! # Bounds (MP §12)
//!
//! The output is at most the input: every conversion is a width- and
//! order-preserving permutation of the packet's own bytes, so memory
//! is bounded by the packet (which the demuxer's chunk ceilings and
//! the packet queue already bound — no additional budget constant is
//! needed, unlike a container walk; the 2.B4 `fuzz.rs` rationale).

use crate::audio::pcm::error::{PcmError, unsupported};
use codec_api::prelude::*;
use media_types::Frame;
use media_types::frame::{AudioFormat, AudioFrame, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::{Duration, Nanoseconds};

use super::{PcmFormat, PcmSource};

/// `wFormatTag` for integer PCM (mulaw/etc are later codecs, not PCM
/// width variants).
const WAVE_FORMAT_PCM: u16 = 1;
/// `wFormatTag` for IEEE float PCM.
const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;
/// Maximum accepted `nChannels` in a WAVEFORMATEX/WAVEFORMATEXTENSIBLE
/// blob — stage agreement with the demuxer's own ceiling
/// (`demux::avi::header::MAX_PCM_CHANNELS`; the two crates keep their
/// own copy rather than share an interface, and the attack suites pin
/// the agreement). Beyond it a channel count is corruption, and the
/// block-align cross-check below could truncate through `u16` for
/// channels × width ≥ 65536 — refuse typed instead (W2-15 g1).
const MAX_PCM_CHANNELS: u16 = 64;

/// The validated decode plan, assembled once at `configure`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PcmPlan {
    /// Source sample layout (format + planarity + channels).
    source: PcmSource,
    /// Samples per second (nonzero, validated).
    rate: u32,
    /// Per-sample byte swap needed (stream byte order ≠ host).
    swap: bool,
    /// Bytes per sample-frame: `channels × bytes_per_sample`.
    frame_bytes: usize,
}

impl PcmPlan {
    /// Build and validate a plan.
    ///
    /// # Errors
    /// [`PcmError::Unsupported`] for zero channels or zero rate.
    pub(crate) fn new(source: PcmSource, rate: u32) -> Result<PcmPlan, PcmError> {
        if source.channels == 0 {
            return Err(unsupported(
                "pcm.channels.zero",
                "a PCM stream needs at least one channel",
            ));
        }
        if rate == 0 {
            return Err(unsupported(
                "pcm.rate.zero",
                "a PCM stream needs a nonzero sample rate",
            ));
        }
        let swap = source
            .sample
            .is_little_endian()
            .is_some_and(|stream_le| stream_le != cfg!(target_endian = "little"));
        // Saturating, never wrapping: `with_format` deliberately keeps
        // accepting extreme channel counts (every sane packet then
        // decodes to zero whole sample-frames and skips honestly), and
        // on a 32-bit target (wasm32 — an explicit project target) the
        // plain `*` would overflow: debug builds panic, release builds
        // wrap (W2-15 g3 / B5's pinned 64-bit behavior is unchanged —
        // the product only ever saturates when it already exceeded
        // every representable packet).
        let frame_bytes =
            (source.channels as usize).saturating_mul(source.sample.bytes_per_sample() as usize);
        Ok(PcmPlan {
            source,
            rate,
            swap,
            frame_bytes,
        })
    }

    /// The output [`AudioFormat`] this plan declares.
    pub(crate) fn output_format(&self) -> AudioFormat {
        AudioFormat {
            sample: self.source.sample.output_sample_format(),
            channels: self.source.channels,
            layout: if self.source.planar {
                AudioLayout::Planar
            } else {
                AudioLayout::Interleaved
            },
            rate: self.rate,
        }
    }

    /// Presentation duration of `instants` sample-frames, in
    /// nanoseconds (floor division — the recorded sub-ns remainder is
    /// dropped, never rounded up into a longer frame).
    fn duration_nanos(&self, instants: u64) -> u64 {
        instants
            .saturating_mul(1_000_000_000)
            .checked_div(u64::from(self.rate))
            .unwrap_or(u64::MAX)
    }
}

/// The result of the byte-level decode of one packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Payload {
    /// `None`: the packet's own bytes are the decoded samples (no
    /// swap, no truncation) — the caller may reuse the packet buffer
    /// zero-copy. `Some(bytes)`: a converted and/or truncated copy.
    pub converted: Option<Vec<u8>>,
    /// Whole sample-frames in the output (all channels).
    pub instants: u64,
    /// Trailing bytes that belonged to no complete sample-frame and
    /// were dropped (concealment observability).
    pub dropped_tail: usize,
}

/// The byte-level decode: whole-sample-frame prefix, per-sample
/// byte-order normalization (see module docs — the prefix rule is
/// layout-uniform: a whole number of sample-frames is exactly a whole
/// number of equal sample-aligned planes, and vice versa).
pub(crate) fn decode_payload(data: &[u8], plan: &PcmPlan) -> Payload {
    let bps = plan.source.sample.bytes_per_sample() as usize;
    // `frame_bytes` is ≥ 1 by construction (PcmPlan::new validates
    // channels ≥ 1 and the width ≥ 1); checked_div keeps the total
    // function shape explicit anyway.
    let instants = data.len().checked_div(plan.frame_bytes).unwrap_or(0);
    let cut = instants * plan.frame_bytes;
    let segment = &data[..cut];
    let converted = if plan.swap {
        Some(swapped(segment, bps))
    } else if cut != data.len() {
        Some(segment.to_vec())
    } else {
        None
    };
    Payload {
        converted,
        instants: instants as u64,
        dropped_tail: data.len() - cut,
    }
}

/// A swapped copy: every `bps`-byte sample group reversed (the
/// stream/host byte-order normalization — identical code for 2/3/4
/// byte samples; 1-byte samples never swap).
fn swapped(data: &[u8], bps: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for group in data.chunks(bps) {
        out.extend(group.iter().rev());
    }
    out
}

/// The stream facts a configuration carries: channel count and rate.
type Facts = (u32, u32);

/// `wFormatTag` for WAVEFORMATEXTENSIBLE — the 40-byte form whose
/// SubFormat GUID names the real format (the AVI demuxer resolves the
/// same GUID; the decoder accepts the verbatim container blob).
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;
/// `KSDATAFORMAT_SUBTYPE_PCM` — `{00000001-0000-0010-8000-00AA00389B71}`.
const SUBFORMAT_PCM_GUID: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];
/// `KSDATAFORMAT_SUBTYPE_IEEE_FLOAT` —
/// `{00000003-0000-0010-8000-00AA00389B71}`.
const SUBFORMAT_IEEE_FLOAT_GUID: [u8; 16] = [
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];

/// Parse a 16-byte PCMWAVEFORMAT, 18-byte WAVEFORMATEX or
/// WAVEFORMATEXTENSIBLE (any length ≥ 40: the GUID sits at the fixed
/// offset 24..40, and vendor tail bytes past 40 are legal per the MS
/// schema — `cbSize` counts the whole extension — and unread; the
/// bound is physical, mirroring the demuxer's own `strf_size >= 40`
/// rule at `header.rs:757`, so both stages accept the same blobs)
/// and cross-check it against the descriptor's format. The
/// extensible form resolves its SubFormat GUID to the plain PCM/float
/// tag first (the demuxer accepts the same blobs — the engine may
/// hand either crate's view the container's verbatim `strf`).
///
/// # Errors
/// [`PcmError::Unsupported`] for every length/tag/width/consistency
/// violation (distinct context codes; see the module docs).
fn parse_waveformatex(bytes: &[u8], format: PcmFormat) -> Result<Facts, PcmError> {
    if !matches!(bytes.len(), 16 | 18) && bytes.len() < 40 {
        return Err(unsupported(
            "pcm.waveformat.length",
            format!(
                "stream-facts blob must be a 16-byte PCMWAVEFORMAT, \
                 18-byte WAVEFORMATEX or >= 40-byte WAVEFORMATEXTENSIBLE, \
                 got {} bytes",
                bytes.len()
            ),
        ));
    }
    let u16le = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let u32le =
        |at: usize| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let tag = u16le(0);
    let channels = u16le(2);
    let rate = u32le(4);
    let block_align = u16le(12);
    let bits = u16le(14);

    if channels == 0 {
        return Err(unsupported(
            "pcm.waveformat.channels",
            "nChannels is zero — a PCM stream needs at least one channel",
        ));
    }
    // The channel ceiling (W2-15 g1): the demuxer refuses `nChannels`
    // beyond 64 before ever resolving the GUID; this seam must not be
    // bypassable by the same untrusted bytes its align cross-check
    // exists to validate (channels × width ≥ 65536 used to wrap the
    // expected align through `as u16`, so a crafted blob passed its
    // own "consistency" check).
    if channels > MAX_PCM_CHANNELS {
        return Err(unsupported(
            "pcm.waveformat.channels",
            format!(
                "nChannels {channels} is beyond the accepted ceiling \
                 ({MAX_PCM_CHANNELS}) — corruption, not a layout this \
                 engine can honor"
            ),
        ));
    }
    if rate == 0 {
        return Err(unsupported(
            "pcm.waveformat.rate",
            "nSamplesPerSec is zero — a PCM stream needs a nonzero rate",
        ));
    }
    // The extensible form names its real format in the SubFormat GUID
    // (byte 24); resolve it to the plain tag the descriptor check
    // understands. Anything short of the ≥ 40-byte form is a length
    // lie (the GUID cannot physically fit).
    let tag = if tag == WAVE_FORMAT_EXTENSIBLE {
        if bytes.len() < 40 {
            return Err(unsupported(
                "pcm.waveformat.extensible",
                "WAVE_FORMAT_EXTENSIBLE requires the >= 40-byte form \
                 carrying the SubFormat GUID",
            ));
        }
        if bytes[24..40] == SUBFORMAT_PCM_GUID {
            WAVE_FORMAT_PCM
        } else if bytes[24..40] == SUBFORMAT_IEEE_FLOAT_GUID {
            WAVE_FORMAT_IEEE_FLOAT
        } else {
            return Err(unsupported(
                "pcm.waveformat.subformat",
                "the SubFormat GUID names a format outside the PCM \
                 vocabulary",
            ));
        }
    } else {
        tag
    };
    let float = matches!(
        format.output_sample_format(),
        SampleFormat::F32 | SampleFormat::F64
    );
    let tag_ok = if float {
        tag == WAVE_FORMAT_IEEE_FLOAT
    } else {
        tag == WAVE_FORMAT_PCM
    };
    if !tag_ok {
        return Err(unsupported(
            "pcm.waveformat.tag",
            format!(
                "wFormatTag {tag} does not match the {} descriptor \
                 (expected {})",
                format.descriptor(),
                if float {
                    WAVE_FORMAT_IEEE_FLOAT
                } else {
                    WAVE_FORMAT_PCM
                }
            ),
        ));
    }
    let bits_expected = (format.bytes_per_sample() * 8) as u16;
    if bits != bits_expected {
        return Err(unsupported(
            "pcm.waveformat.bits",
            format!(
                "wBitsPerSample {bits} does not match the {} descriptor \
                 ({bits_expected})",
                format.descriptor()
            ),
        ));
    }
    // Checked, never truncating (W2-15 g1): the expected align is the
    // true product `channels × sample width`; a product that does not
    // fit `u16` means no declared `nBlockAlign` can be honest. (With
    // the channel ceiling above the product is ≤ 512 — the `try_from`
    // is defense in depth so the two guards stay independent.)
    let align_true = u32::from(channels) * format.bytes_per_sample();
    let align_expected = match u16::try_from(align_true) {
        Ok(align) => align,
        Err(_) => {
            return Err(unsupported(
                "pcm.waveformat.block_align",
                format!(
                    "channels × sample width is {align_true} — beyond \
                     nBlockAlign's u16 range, no declared align can be honest"
                ),
            ));
        }
    };
    if block_align != align_expected {
        return Err(unsupported(
            "pcm.waveformat.block_align",
            format!(
                "nBlockAlign {block_align} does not match channels × \
                 sample width ({align_expected})",
            ),
        ));
    }
    Ok((u32::from(channels), rate))
}

/// The PCM decoder behind the codec-api [`Decoder`] contract (card
/// 2.B5; doc 01 §2.5, doc 03 §4.5/§7).
///
/// Stateless between packets (unpacked PCM has no inter-packet
/// state); `configure` installs the negotiated format + facts, so
/// re-configuration after a seek is free.
///
/// # Example
///
/// ```
/// use codec_api::prelude::*;
/// use codecs::audio::pcm::PcmDecoder;
/// use media_types::{GenerationId, Stamp, TrackId};
///
/// // The AVI first vertical: s16le stereo 44100 — the container's
/// // `strf` (WAVEFORMATEX) rides `extra_data`.
/// let strf = [
///     0x01, 0x00, // wFormatTag = WAVE_FORMAT_PCM
///     0x02, 0x00, // nChannels = 2
///     0x44, 0xac, 0x00, 0x00, // nSamplesPerSec = 44100
///     0x10, 0xb1, 0x02, 0x00, // nAvgBytesPerSec
///     0x04, 0x00, // nBlockAlign = 4
///     0x10, 0x00, // wBitsPerSample = 16
/// ];
/// let mut decoder = PcmDecoder::new();
/// let generation = GenerationId::FIRST
///     .next()
///     .unwrap_or_else(|e| unreachable!("{e}"));
/// decoder
///     .configure(&DecoderConfig::new(
///         media_types::ids::CodecDescriptor::new("pcm_s16le")
///             .unwrap_or_else(|e| unreachable!("{e}")),
///         generation,
///         ThreadBudget::default(),
///         None,
///         Some(media_types::packet::SharedBytes::from_vec(strf.to_vec())),
///     ))
///     .unwrap_or_else(|e| unreachable!("{e}"));
///
/// // One stereo sample-frame: a quiet negative left, a positive right.
/// let packet = Packet {
///     track: TrackId::new(1),
///     pts: Stamp::Missing,
///     dts: Stamp::Missing,
///     duration: None,
///     data: media_types::packet::SharedBytes::from_vec(vec![
///         0x34, 0xf3, 0xdc, 0x0f,
///     ]),
///     keyframe: true,
///     discontinuity: false,
///     generation,
/// };
/// struct Sink;
/// impl FrameSink for Sink {
///     fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
///         Ok(())
///     }
/// }
/// let outcome = decoder.decode(packet, &mut Sink);
/// match outcome {
///     Ok(DecodeOutcome::Emitted { frames: 1 }) => {}
///     other => unreachable!("one clean frame: {other:?}"),
/// }
/// ```
#[derive(Debug, Default)]
pub struct PcmDecoder {
    /// The validated decode plan (None until `configure`).
    plan: Option<PcmPlan>,
    /// Out-of-band facts pre-seeded by the engine adapter (the
    /// `stream_format` seam) — cross-checked against `extra_data`.
    seeded: Option<Facts>,
    /// The configured stream generation (None until `configure`).
    generation: Option<GenerationId>,
    /// Codec descriptor accepted at configure time (diagnostics).
    codec: Option<String>,
}

impl PcmDecoder {
    /// A fresh decoder, unconfigured; the stream facts must arrive in
    /// `DecoderConfig::extra_data` (WAVEFORMATEX) at configure time.
    pub fn new() -> PcmDecoder {
        PcmDecoder::default()
    }

    /// A fresh decoder with the stream facts pre-seeded — the typed
    /// seam for engines that hold `AviDemuxer::stream_format`-style
    /// facts (card 2.B2's demuxer seam; or an MP4 `AudioSampleEntry`
    /// parsed by a later adapter) and do not have WAVEFORMATEX bytes.
    /// When `extra_data` is *also* supplied it must agree with these
    /// facts.
    ///
    /// # Errors
    /// [`ErrorKind::Unsupported`](media_types::ErrorKind::Unsupported)
    /// (code `pcm.channels.zero` / `pcm.rate.zero`) for zero values.
    pub fn with_format(channels: u32, rate: u32) -> Result<PcmDecoder, EngineError> {
        if channels == 0 {
            return Err(unsupported(
                "pcm.channels.zero",
                "a PCM stream needs at least one channel",
            )
            .into_engine_error());
        }
        if rate == 0 {
            return Err(
                unsupported("pcm.rate.zero", "a PCM stream needs a nonzero sample rate")
                    .into_engine_error(),
            );
        }
        Ok(PcmDecoder {
            seeded: Some((channels, rate)),
            ..PcmDecoder::default()
        })
    }

    /// Resolve the stream facts for a configuration.
    ///
    /// # Errors
    /// [`PcmError::Unsupported`] when facts are absent, malformed or
    /// conflicting (see the module docs).
    fn resolve_facts(
        &self,
        format: PcmFormat,
        extra: Option<&SharedBytes>,
    ) -> Result<Facts, PcmError> {
        match (self.seeded, extra) {
            (None, None) => Err(unsupported(
                "pcm.stream_format_absent",
                "no stream facts: pass WAVEFORMATEX bytes in \
                 DecoderConfig::extra_data, or build the decoder with \
                 PcmDecoder::with_format",
            )),
            (Some(seeded), None) => Ok(seeded),
            (seeded, Some(bytes)) => {
                let parsed = parse_waveformatex(bytes.as_slice(), format)?;
                if let Some(seeded) = seeded
                    && seeded != parsed
                {
                    return Err(unsupported(
                        "pcm.stream_format_conflict",
                        format!(
                            "with_format seeded {} channel(s) at {} Hz, \
                             extra_data declares {} channel(s) at {} Hz",
                            seeded.0, seeded.1, parsed.0, parsed.1
                        ),
                    ));
                }
                Ok(parsed)
            }
        }
    }
}

impl Decoder for PcmDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        let descriptor = cfg.codec.as_str();
        let (base, planar) = match descriptor.strip_suffix("_planar") {
            Some(base) => (base, true),
            None => (descriptor, false),
        };
        let Some(format) = PcmFormat::from_descriptor(base) else {
            return Err(unsupported(
                "pcm.codec_descriptor",
                format!("codec descriptor `{descriptor}` is not in the PCM vocabulary"),
            )
            .into_engine_error());
        };
        let (channels, rate) = self
            .resolve_facts(format, cfg.extra_data.as_ref())
            .map_err(PcmError::into_engine_error)?;
        let plan = PcmPlan::new(
            PcmSource {
                sample: format,
                planar,
                channels,
            },
            rate,
        )
        .map_err(PcmError::into_engine_error)?;
        self.plan = Some(plan);
        self.generation = Some(cfg.generation);
        self.codec = Some(descriptor.to_string());
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        let (generation, plan) = match (self.generation, self.plan.as_ref()) {
            (Some(generation), Some(plan)) => (generation, plan),
            _ => return Err(PcmError::BadState.into_engine_error()),
        };
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        let data = packet.data.as_slice();
        let payload = decode_payload(data, plan);
        if payload.instants == 0 {
            if data.is_empty() {
                // Consumed, nothing decodable, nothing dropped: the
                // honest "nothing to emit" report.
                return Ok(DecodeOutcome::NeedMoreInput);
            }
            // Too few bytes for even one sample-frame: the packet is
            // skipped whole (bounded, observable, never silent).
            return Ok(DecodeOutcome::Recovered {
                skipped: 1,
                reason: RecoverReason::CorruptInput,
            });
        }
        let samples = match payload.converted {
            None => packet.data.clone(), // zero-copy: the bytes ARE the frame
            Some(bytes) => SharedBytes::from_vec(bytes),
        };
        let frame = Frame::Audio(AudioFrame {
            format: plan.output_format(),
            samples,
            pts: packet.pts,
            duration: Some(Duration::Known(Nanoseconds(
                plan.duration_nanos(payload.instants),
            ))),
            generation: packet.generation,
        });
        out.accept(frame)?;
        if payload.dropped_tail > 0 {
            Ok(DecodeOutcome::Recovered {
                skipped: 0,
                reason: RecoverReason::ConcealedFrame,
            })
        } else {
            Ok(DecodeOutcome::Emitted { frames: 1 })
        }
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        // Unpacked PCM carries no inter-packet state: nothing is
        // buffered, nothing to drain.
        if self.generation.is_none() || self.plan.is_none() {
            return Err(PcmError::BadState.into_engine_error());
        }
        Ok(DecodeOutcome::Drained)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::ids::CodecDescriptor;
    use media_types::{Packet, Stamp, TrackId};

    /// A WAVEFORMATEX for s16le stereo at `rate`.
    fn waveformat(rate: u32) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        bytes.extend_from_slice(&WAVE_FORMAT_PCM.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&rate.to_le_bytes());
        bytes.extend_from_slice(&(rate * 4).to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes
    }

    /// One configured decoder for the given descriptor + facts blob.
    fn configured(descriptor: &str, extra: Option<Vec<u8>>) -> (PcmDecoder, GenerationId) {
        let descriptor = match CodecDescriptor::new(descriptor) {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        let generation = match GenerationId::FIRST.next() {
            Ok(g) => g,
            Err(e) => unreachable!("{e}"),
        };
        let mut decoder = PcmDecoder::new();
        match decoder.configure(&DecoderConfig::new(
            descriptor,
            generation,
            ThreadBudget::default(),
            None,
            extra.map(SharedBytes::from_vec),
        )) {
            Ok(()) => (decoder, generation),
            Err(e) => unreachable!("configure: {e}"),
        }
    }

    fn packet(bytes: Vec<u8>, generation: GenerationId) -> Packet {
        Packet {
            track: TrackId::new(1),
            pts: Stamp::Missing,
            dts: Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(bytes),
            keyframe: true,
            discontinuity: false,
            generation,
        }
    }

    #[derive(Default)]
    struct Capture {
        frames: Vec<AudioFrame>,
    }

    impl FrameSink for Capture {
        fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
            let Frame::Audio(audio) = frame else {
                unreachable!("PCM emits audio frames only");
            };
            self.frames.push(audio);
            Ok(())
        }
    }

    /// Decode one packet, capturing the emitted audio frame.
    fn decode_one(decoder: &mut PcmDecoder, bytes: Vec<u8>) -> (DecodeOutcome, Capture) {
        let generation = decoder.generation.unwrap_or(GenerationId::FIRST);
        let mut sink = Capture::default();
        let outcome = match decoder.decode(packet(bytes, generation), &mut sink) {
            Ok(o) => o,
            Err(e) => unreachable!("decode must not fail: {e}"),
        };
        (outcome, sink)
    }

    // ---- WAVEFORMATEX negotiation -----------------------------------------

    #[test]
    fn waveformatex_accepts_16_and_18_byte_forms() {
        let mut eighteen = waveformat(48_000);
        eighteen.push(0);
        eighteen.push(0); // cbSize = 0
        for bytes in [waveformat(48_000), eighteen] {
            let facts = parse_waveformatex(&bytes, PcmFormat::S16Le);
            assert_eq!(facts, Ok((2, 48_000)));
        }
    }

    #[test]
    fn waveformatextensible_length_ladder_pins_the_physical_bound() {
        // W2-15 (a4): the extensible bound is physical, not the
        // exact-40 shape — the GUID sits at the fixed offset 24..40, so
        // every length >= 40 carries it, and the vendor tail past 40 is
        // legal WAVEFORMATEXTENSIBLE (cbSize counts the whole
        // extension; the demuxer's own rule is `strf_size >= 40` and
        // ffprobe plays the 42-byte form — measured, W2-15 worklog).
        // 17 and 19..=39 are the refused margins: neither a plain form
        // nor a GUID-carrying one.
        let mut blob = waveformat(44_100);
        blob[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes());
        blob.extend_from_slice(&22u16.to_le_bytes()); // cbSize
        blob.extend_from_slice(&16u16.to_le_bytes()); // valid bits
        blob.extend_from_slice(&0x3u32.to_le_bytes()); // channel mask
        blob.extend_from_slice(&SUBFORMAT_PCM_GUID);
        assert_eq!(blob.len(), 40);
        for tail in [0usize, 1, 2, 8, 24] {
            let mut extended = blob.clone();
            extended.extend(std::iter::repeat_n(0xA5, tail));
            assert_eq!(
                parse_waveformatex(&extended, PcmFormat::S16Le),
                Ok((2, 44_100)),
                "vendor tail {tail}"
            );
        }
        for len in [17usize, 19, 20, 39] {
            assert_eq!(
                parse_waveformatex(&blob[..len], PcmFormat::S16Le)
                    .err()
                    .map(|e| e.into_engine_error().context.code),
                Some("pcm.waveformat.length".to_string()),
                "len {len}"
            );
        }
        // 16 and 18 with the extensible tag stay the extensible
        // refusal: the GUID cannot physically fit.
        for len in [16usize, 18] {
            assert_eq!(
                parse_waveformatex(&blob[..len], PcmFormat::S16Le)
                    .err()
                    .map(|e| e.into_engine_error().context.code),
                Some("pcm.waveformat.extensible".to_string()),
                "len {len}"
            );
        }
    }

    #[test]
    fn channel_counts_beyond_the_ceiling_are_typed_refusals() {
        // W2-15 (g1): the align cross-check used to truncate through
        // `as u16` — channels=32768 × s16 with a declared align of 0
        // passed its own "consistency" check (65536 wraps to 0). The
        // ceiling (stage agreement with the demuxer's 64) and the
        // checked product close the bypass; 64 itself is the boundary
        // and must stay accepted.
        for (channels, align, bits) in [
            (32768u16, 0u16, 16u16),
            (65535, 65534, 16),
            (16384, 0, 32),
            (8192, 0, 64),
            (65, 130, 16),
        ] {
            let mut blob = waveformat(8_000);
            blob[2..4].copy_from_slice(&channels.to_le_bytes());
            blob[8..12].copy_from_slice(&(u32::from(align) * 8_000).to_le_bytes());
            blob[12..14].copy_from_slice(&align.to_le_bytes());
            blob[14..16].copy_from_slice(&bits.to_le_bytes());
            assert_eq!(
                parse_waveformatex(&blob, PcmFormat::S16Le)
                    .err()
                    .map(|e| e.into_engine_error().context.code),
                Some("pcm.waveformat.channels".to_string()),
                "channels {channels} × width must not configure"
            );
        }
        let mut boundary = waveformat(8_000);
        boundary[2..4].copy_from_slice(&64u16.to_le_bytes());
        boundary[8..12].copy_from_slice(&(128u32 * 8_000).to_le_bytes());
        boundary[12..14].copy_from_slice(&128u16.to_le_bytes());
        assert_eq!(
            parse_waveformatex(&boundary, PcmFormat::S16Le),
            Ok((64, 8_000)),
            "the ceiling itself is accepted"
        );
    }

    #[test]
    fn waveformatex_rejects_short_long_and_wrong_tags() {
        // Short.
        assert_eq!(
            parse_waveformatex(&waveformat(44_100)[..15], PcmFormat::S16Le)
                .err()
                .map(|e| e.into_engine_error().context.code),
            Some("pcm.waveformat.length".to_string())
        );
        // Long (WAVEFORMATEXTENSIBLE-shape).
        let mut long = waveformat(44_100);
        long.extend_from_slice(&[0u8; 22]);
        assert_eq!(
            parse_waveformatex(&long, PcmFormat::S16Le)
                .err()
                .map(|e| e.into_engine_error().context.code),
            Some("pcm.waveformat.length".to_string())
        );
        // Tag mismatch: integer format with a float tag.
        let mut float_tag = waveformat(44_100);
        float_tag[0..2].copy_from_slice(&WAVE_FORMAT_IEEE_FLOAT.to_le_bytes());
        assert_eq!(
            parse_waveformatex(&float_tag, PcmFormat::S16Le)
                .err()
                .map(|e| e.into_engine_error().context.code),
            Some("pcm.waveformat.tag".to_string())
        );
        // Extensible tag in a blob too short to carry the GUID.
        let mut extensible = waveformat(44_100);
        extensible[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes());
        assert_eq!(
            parse_waveformatex(&extensible, PcmFormat::S16Le)
                .err()
                .map(|e| e.into_engine_error().context.code),
            Some("pcm.waveformat.extensible".to_string())
        );
    }

    #[test]
    fn waveformatextensible_resolves_guids_and_decodes() {
        // The 40-byte form ffmpeg's AVI muxer writes (the demuxer
        // accepts the same blobs — an engine may hand the verbatim
        // `strf` here as `extra_data`).
        let extensible = |guid: [u8; 16], channels: u16, rate: u32, bits: u16| {
            let mut v = waveformat(rate);
            v.truncate(16);
            v[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes());
            v[2..4].copy_from_slice(&channels.to_le_bytes());
            v[8..12].copy_from_slice(&(u32::from(channels) * u32::from(bits / 8)).to_le_bytes());
            v[12..14].copy_from_slice(&(channels * (bits / 8)).to_le_bytes());
            v[14..16].copy_from_slice(&bits.to_le_bytes());
            v.extend_from_slice(&22u16.to_le_bytes()); // cbSize
            v.extend_from_slice(&bits.to_le_bytes()); // valid bits
            v.extend_from_slice(&0x3u32.to_le_bytes()); // channel mask
            v.extend_from_slice(&guid);
            v
        };
        // Int GUID + int descriptor: parses, configures and decodes.
        let s24 = extensible(SUBFORMAT_PCM_GUID, 2, 48_000, 24);
        assert_eq!(parse_waveformatex(&s24, PcmFormat::S24Le), Ok((2, 48_000)));
        let (mut decoder, generation) = configured("pcm_s24le", Some(s24));
        let (outcome, sink) = decode_one(
            &mut decoder,
            vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xab], // one 2ch frame
        );
        assert_eq!(outcome, DecodeOutcome::Emitted { frames: 1 });
        assert_eq!(sink.frames[0].format.sample, SampleFormat::S24);
        assert_eq!(sink.frames[0].format.channels, 2);
        let _ = generation;
        // Float GUID + float descriptor.
        let f32le = extensible(SUBFORMAT_IEEE_FLOAT_GUID, 1, 44_100, 32);
        assert_eq!(
            parse_waveformatex(&f32le, PcmFormat::F32Le),
            Ok((1, 44_100))
        );
        // Float GUID + int descriptor: the resolved tag must match.
        assert_eq!(
            parse_waveformatex(&f32le, PcmFormat::S32Le)
                .err()
                .map(|e| e.into_engine_error().context.code),
            Some("pcm.waveformat.tag".to_string())
        );
        // A GUID outside the PCM vocabulary (μ-law).
        let mulaw = extensible(
            [
                0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38,
                0x9b, 0x71,
            ],
            2,
            44_100,
            16,
        );
        assert_eq!(
            parse_waveformatex(&mulaw, PcmFormat::S16Le)
                .err()
                .map(|e| e.into_engine_error().context.code),
            Some("pcm.waveformat.subformat".to_string())
        );
    }

    #[test]
    fn waveformatex_rejects_width_and_align_lies() {
        // Bits mismatch.
        let mut bits = waveformat(44_100);
        bits[14..16].copy_from_slice(&24u16.to_le_bytes());
        assert_eq!(
            parse_waveformatex(&bits, PcmFormat::S16Le)
                .err()
                .map(|e| e.into_engine_error().context.code),
            Some("pcm.waveformat.bits".to_string())
        );
        // Block-align mismatch.
        let mut align = waveformat(44_100);
        align[12..14].copy_from_slice(&8u16.to_le_bytes());
        assert_eq!(
            parse_waveformatex(&align, PcmFormat::S16Le)
                .err()
                .map(|e| e.into_engine_error().context.code),
            Some("pcm.waveformat.block_align".to_string())
        );
        // Zero channels / zero rate.
        let mut zero_ch = waveformat(44_100);
        zero_ch[2..4].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            parse_waveformatex(&zero_ch, PcmFormat::S16Le)
                .err()
                .map(|e| e.into_engine_error().context.code),
            Some("pcm.waveformat.channels".to_string())
        );
        let mut zero_rate = waveformat(44_100);
        zero_rate[4..8].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            parse_waveformatex(&zero_rate, PcmFormat::S16Le)
                .err()
                .map(|e| e.into_engine_error().context.code),
            Some("pcm.waveformat.rate".to_string())
        );
    }

    #[test]
    fn float_descriptors_demand_the_float_tag() {
        let mut int_tag = waveformat(48_000);
        int_tag[14..16].copy_from_slice(&32u16.to_le_bytes()); // 32-bit
        int_tag[12..14].copy_from_slice(&8u16.to_le_bytes()); // 2ch × 4B
        let mut float_ok = int_tag.clone();
        float_ok[0..2].copy_from_slice(&WAVE_FORMAT_IEEE_FLOAT.to_le_bytes());
        assert_eq!(
            parse_waveformatex(&float_ok, PcmFormat::F32Le),
            Ok((2, 48_000))
        );
        assert_eq!(
            parse_waveformatex(&int_tag, PcmFormat::F32Le)
                .err()
                .map(|e| e.into_engine_error().context.code),
            Some("pcm.waveformat.tag".to_string())
        );
    }

    // ---- configure-time vocabulary ----------------------------------------

    #[test]
    fn configure_without_facts_is_a_typed_unsupported() {
        let descriptor = match CodecDescriptor::new("pcm_s16le") {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        let mut decoder = PcmDecoder::new();
        let err = match decoder.configure(&DecoderConfig::new(
            descriptor,
            GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            None,
        )) {
            Ok(()) => unreachable!("facts are required"),
            Err(e) => e,
        };
        assert!(matches!(err.kind, ErrorKind::Unsupported { .. }), "{err}");
        assert_eq!(err.context.code, "pcm.stream_format_absent");
        assert_eq!(err.stage, media_types::PipelineStage::Decode);
    }

    #[test]
    fn with_format_seeds_facts_and_conflicts_are_typed() {
        let descriptor = match CodecDescriptor::new("pcm_s16le") {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        // Seeded facts alone configure fine.
        let mut decoder = match PcmDecoder::with_format(2, 44_100) {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        assert!(
            decoder
                .configure(&DecoderConfig::new(
                    descriptor.clone(),
                    GenerationId::FIRST,
                    ThreadBudget::default(),
                    None,
                    None
                ))
                .is_ok()
        );
        // Agreeing extra_data is fine.
        let mut agreeing = match PcmDecoder::with_format(2, 44_100) {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        assert!(
            agreeing
                .configure(&DecoderConfig::new(
                    descriptor.clone(),
                    GenerationId::FIRST,
                    ThreadBudget::default(),
                    None,
                    Some(SharedBytes::from_vec(waveformat(44_100)))
                ))
                .is_ok()
        );
        // Conflicting extra_data is a typed negotiation failure.
        let mut conflicting = match PcmDecoder::with_format(2, 48_000) {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        let err = match conflicting.configure(&DecoderConfig::new(
            descriptor,
            GenerationId::FIRST,
            ThreadBudget::default(),
            None,
            Some(SharedBytes::from_vec(waveformat(44_100))),
        )) {
            Ok(()) => unreachable!("conflict must fail"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "pcm.stream_format_conflict");
        // Zero-valued seeds are rejected at the constructor.
        assert_eq!(
            PcmDecoder::with_format(0, 44_100)
                .err()
                .map(|e| e.context.code),
            Some("pcm.channels.zero".to_string())
        );
        assert_eq!(
            PcmDecoder::with_format(2, 0).err().map(|e| e.context.code),
            Some("pcm.rate.zero".to_string())
        );
    }

    #[test]
    fn unconfigured_decode_and_flush_are_typed_bad_state() {
        let mut decoder = PcmDecoder::new();
        let generation = match GenerationId::FIRST.next() {
            Ok(g) => g,
            Err(e) => unreachable!("{e}"),
        };
        let err = match decoder.decode(packet(vec![0; 4], generation), &mut Capture::default()) {
            Ok(o) => unreachable!("must fail: {o:?}"),
            Err(e) => e,
        };
        assert!(
            matches!(
                err.kind,
                ErrorKind::Decoder {
                    cause: media_types::error::DecodeCause::BadState
                }
            ),
            "{err}"
        );
        let err = match decoder.flush(&mut Capture::default()) {
            Ok(o) => unreachable!("must fail: {o:?}"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "pcm.not_configured");
    }

    // ---- decode: the fixture format ---------------------------------------

    #[test]
    fn s16le_stereo_decodes_whole_frames_with_format_and_duration() {
        let (mut decoder, _generation) = configured("pcm_s16le", Some(waveformat(44_100)));
        // Two stereo sample-frames: (-3200, 4060), (1, -1).
        let mut bytes = Vec::new();
        for sample in [-3200i16, 4060, 1, -1] {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let (outcome, sink) = decode_one(&mut decoder, bytes);
        assert_eq!(outcome, DecodeOutcome::Emitted { frames: 1 });
        assert_eq!(sink.frames.len(), 1);
        let frame = &sink.frames[0];
        assert_eq!(
            frame.format,
            AudioFormat {
                sample: SampleFormat::S16,
                channels: 2,
                layout: AudioLayout::Interleaved,
                rate: 44_100,
            }
        );
        // 2 instants at 44100 Hz = 45351.47… ns → floor 45351.
        assert_eq!(frame.duration, Some(Duration::Known(Nanoseconds(45_351))));
        // On a little-endian host the decode is the identity; on a
        // big-endian host the values are identical in native order —
        // assert on VALUES, not bytes.
        let out = frame.samples.as_slice();
        assert_eq!(out.len(), 8);
        for (index, want) in [-3200i16, 4060, 1, -1].iter().enumerate() {
            let got = i16::from_ne_bytes([out[index * 2], out[index * 2 + 1]]);
            assert_eq!(got, *want, "sample {index}");
        }
    }

    #[test]
    fn native_endian_interleaved_decode_is_zero_copy() {
        // A format whose byte order matches the host needs no
        // conversion: the emitted frame must share the packet's
        // allocation (the same Arc, not a copy).
        let native = if cfg!(target_endian = "little") {
            "pcm_s16le"
        } else {
            "pcm_s16be"
        };
        let (mut decoder, generation) = configured(native, Some(waveformat(44_100)));
        let data = SharedBytes::from_vec(vec![0x11, 0x22, 0x33, 0x44]);
        let pointer = data.as_slice().as_ptr();
        let mut sink = Capture::default();
        let outcome = match decoder.decode(
            Packet {
                track: TrackId::new(1),
                pts: Stamp::Missing,
                dts: Stamp::Missing,
                duration: None,
                data: data.clone(),
                keyframe: true,
                discontinuity: false,
                generation,
            },
            &mut sink,
        ) {
            Ok(o) => o,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(outcome, DecodeOutcome::Emitted { frames: 1 });
        assert_eq!(
            sink.frames[0].samples.as_slice().as_ptr(),
            pointer,
            "the clean native-endian decode reuses the packet buffer"
        );
    }

    // ---- damage model ------------------------------------------------------

    #[test]
    fn partial_tail_is_concealed_not_dropped() {
        let (mut decoder, _generation) = configured("pcm_s16le", Some(waveformat(44_100)));
        // Three whole frames + a dangling byte.
        let mut bytes = Vec::new();
        for sample in [100i16, -100, 200, -200, 300, -300] {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes.push(0x42);
        let (outcome, sink) = decode_one(&mut decoder, bytes);
        assert_eq!(
            outcome,
            DecodeOutcome::Recovered {
                skipped: 0,
                reason: RecoverReason::ConcealedFrame
            }
        );
        assert_eq!(sink.frames.len(), 1, "the prefix is still emitted");
        assert_eq!(sink.frames[0].samples.len(), 12, "3 whole frames");
        // The decoder remains usable (contract: recover, keep going).
        let (outcome, _) = decode_one(&mut decoder, vec![0, 0, 0, 0]);
        assert_eq!(outcome, DecodeOutcome::Emitted { frames: 1 });
    }

    #[test]
    fn sub_frame_packet_is_skipped_whole() {
        let (mut decoder, _generation) = configured("pcm_s16le", Some(waveformat(44_100)));
        for length in 1..4 {
            let (outcome, sink) = decode_one(&mut decoder, vec![0xa5; length]);
            assert_eq!(
                outcome,
                DecodeOutcome::Recovered {
                    skipped: 1,
                    reason: RecoverReason::CorruptInput
                },
                "length {length}"
            );
            assert!(sink.frames.is_empty(), "nothing decodable, nothing emitted");
        }
    }

    #[test]
    fn empty_packet_is_need_more_input() {
        let (mut decoder, _generation) = configured("pcm_s16le", Some(waveformat(44_100)));
        let (outcome, sink) = decode_one(&mut decoder, Vec::new());
        assert_eq!(outcome, DecodeOutcome::NeedMoreInput);
        assert!(sink.frames.is_empty());
    }

    #[test]
    fn flush_drains_and_sticks() {
        let (mut decoder, _generation) = configured("pcm_s16le", Some(waveformat(44_100)));
        assert_eq!(
            decoder.flush(&mut Capture::default()).ok(),
            Some(DecodeOutcome::Drained)
        );
        assert_eq!(
            decoder.flush(&mut Capture::default()).ok(),
            Some(DecodeOutcome::Drained)
        );
    }

    // ---- planar ------------------------------------------------------------

    #[test]
    fn planar_truncation_keeps_plane_boundaries_at_whole_frames() {
        let (mut decoder, _generation) = configured("pcm_s16le_planar", Some(waveformat(44_100)));
        // Planar stereo: [L0 L1 L2 L3] || [R0 R1 R2 R3], then 3 bytes
        // of appended damage. The whole-sample-frame prefix is 16
        // bytes (4 instants × 4 bytes): both planes keep their true
        // boundaries (L at 0, R at 8) and the damage drops off the
        // end — a floor-stride split (19/2 = 9) would shift the R
        // plane into mid-sample garbage instead.
        let mut bytes = Vec::new();
        for sample in [10i16, 11, 12, 13] {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        for sample in [20i16, 21, 22, 23] {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes.extend_from_slice(&[0x99, 0x99, 0x99]);
        let (outcome, sink) = decode_one(&mut decoder, bytes);
        assert_eq!(
            outcome,
            DecodeOutcome::Recovered {
                skipped: 0,
                reason: RecoverReason::ConcealedFrame
            }
        );
        let frame = &sink.frames[0];
        assert_eq!(frame.format.layout, AudioLayout::Planar);
        let out = frame.samples.as_slice();
        assert_eq!(out.len(), 16, "4 whole instants × 4 bytes, 3 dropped");
        // Values, not bytes: the output is native-endian.
        let value = |at: usize| i16::from_ne_bytes([out[at * 2], out[at * 2 + 1]]);
        // L plane: first 8 bytes; R plane: bytes 8..16.
        assert_eq!((value(0), value(1), value(2), value(3)), (10, 11, 12, 13));
        assert_eq!((value(4), value(5), value(6), value(7)), (20, 21, 22, 23));
    }

    #[test]
    fn planar_clean_passes_through_with_planar_layout() {
        let (mut decoder, _generation) = configured("pcm_s16le_planar", Some(waveformat(44_100)));
        let mut bytes = Vec::new();
        for sample in [1i16, 2, 3] {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        for sample in [-1i16, -2, -3] {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let (outcome, sink) = decode_one(&mut decoder, bytes.clone());
        assert_eq!(outcome, DecodeOutcome::Emitted { frames: 1 });
        let frame = &sink.frames[0];
        assert_eq!(frame.format.layout, AudioLayout::Planar);
        assert_eq!(frame.format.channels, 2);
        if cfg!(target_endian = "little") {
            assert_eq!(frame.samples.as_slice(), bytes.as_slice());
        } else {
            assert_eq!(frame.samples.len(), bytes.len());
        }
    }

    // ---- duration math -----------------------------------------------------

    #[test]
    fn duration_floors_to_whole_nanoseconds() {
        let plan = match PcmPlan::new(
            PcmSource {
                sample: PcmFormat::S16Le,
                planar: false,
                channels: 2,
            },
            44_100,
        ) {
            Ok(p) => p,
            Err(e) => unreachable!("{e:?}"),
        };
        // The fixture tick: 1470 instants → 33_333_333.33 ns.
        assert_eq!(plan.duration_nanos(1470), 33_333_333);
        // Exact division: 44100 instants = 1 s.
        assert_eq!(plan.duration_nanos(44_100), 1_000_000_000);
        // Saturating: absurd instants never panic or wrap — the
        // saturating product divides down to a huge-but-finite span.
        assert_eq!(plan.duration_nanos(u64::MAX), u64::MAX / 44_100);
    }
}
