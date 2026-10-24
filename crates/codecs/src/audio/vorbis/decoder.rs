//! The Vorbis decoder core (card 5.B2): the symphonia-backed block
//! decode, the granule-domain sample clock, and the
//! [`codec_api::Decoder`] implementation.
//!
//! # Engine choice (the 5.B2 dependency decision)
//!
//! **`symphonia-codec-vorbis` 0.6.1, exact-pinned**, under the Lead's
//! C5 conditional approval, purity-verified BEFORE any Cargo.toml edit
//! in `docs/dependencies/records/symphonia-vorbis-0.6.1.md`
//! (`#![forbid(unsafe_code)]` in the codec crate, zero unsafe tokens
//! in its new transitive nodes, no build.rs, no native code; 4 new
//! pure-Rust lock nodes). The card's alternative spelling
//! `symphonia-bundle-vorbis` does not exist on crates.io — the record's
//! R1 documents the finding (the mirror image of 5.A3's mp3 case).
//! The engine owns exactly one job: the entropy+DSP decode of ONE
//! already-framed audio packet (codebooks, floors, residues, MDCT
//! overlap-add → F32 planes). The header chain, Vorbis comments,
//! granule arithmetic and the error taxonomy are ours — a future
//! engine swap stays inside this module.
//!
//! # Packet model (the codec-api contract)
//!
//! One packet = **one Vorbis audio block** (the Ogg demuxer's
//! laced-packet framing; a first bit of `0` per spec §4.3.1 — header
//! bytes never reach `decode`, the demuxer routes them to
//! [`super::headers`]). Vorbis is MDCT overlap-add: each packet emits
//! `(prev_n + n) / 4` inter-channel samples (half the block, the
//! overlapping half joining the next block), so per-packet isolation
//! (doc 03 §4.5(d)) is **overlap-scoped**: after a typed `Corrupt` the
//! engine's overlap state is reset and decoding resumes with the NEXT
//! packet (one poisoned half-block at the seam, stated, never a
//! poisoned stream, never a panic).
//!
//! # Facts channel ([PROPOSAL], the FLAC STREAMINFO precedent)
//!
//! [`DecoderConfig::extra_data`] carries the three header packets
//! concatenated — identification ‖ comment ‖ setup — the verbatim
//! container bytes ([`super::headers::VorbisHeaders::from_extra_data`]
//! validates and splits them). Configure without facts is a typed
//! `vorbis.headers.missing`, never a guess.
//!
//! # The sample clock (granule arithmetic, the card's seek surface)
//!
//! Vorbis granules count **PCM samples at the identification rate**
//! (the Ogg `Clock::Samples` convention), on the *valid*
//! (delay/padding-trimmed) timeline. The decoder keeps a raw emission
//! counter and a measured **timeline shift** (see
//! [`super::seek::measure_shift`]): `pts = raw_first − shift`, with
//! the shift re-measured at every container stamp. At the stream head
//! the shift is the encoder delay (the first packets' pts go
//! negative — the same coordinates ffmpeg's ogg demuxer produces);
//! before the first stamp the pts is honestly `Missing` (a gap is
//! exposed, never invented, MP §8). A `discontinuity` packet resets
//! the engine's overlap state (seek reanchor) and re-arms the anchor
//! hunt; the landing page's first stamp re-derives the shift (see
//! [`super::seek::landing_first_sample`]).
//!
//! # Output format (codec-native F32)
//!
//! Vorbis decodes to IEEE F32; output is interleaved native-endian F32
//! (`AudioLayout::Interleaved`), no rate or channel conversion — the
//! same shape as the MP3 path. Bit-exactness: Vorbis is lossy, but
//! decoder outputs are deterministic per implementation — the
//! differential suite pins OUR output against ffmpeg's on identical
//! fixtures (see `tests/vorbis_bitexact.rs` for the verdicts and the
//! honest note about float-identical vs. merely-close).
//!
//! # Allocation discipline (D-1)
//!
//! The engine reuses its internal `AudioBuffer` across packets; our
//! interleaved copy rides a reused `Vec<f32>` scratch and lands in a
//! pooled [`SharedBytes`] rotation sized to the maximum block
//! (`blocksize_long / 2 × channels × 4` bytes) once at configure —
//! steady-state decodes refill a slot in place, zero per-block
//! allocation after warm-up (the 5.A1 `pooled_sample_slot` pattern).

use super::error::{VorbisError, corrupt, unsupported};
use super::headers::{Identification, VorbisHeaders};
use super::seek::measure_shift;
use codec_api::prelude::*;
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::{Nanoseconds, TimeBase, Timestamp};
use media_types::{Duration, Stamp};
use symphonia_codec_vorbis::VorbisDecoder as Engine;
use symphonia_core::codecs::audio::well_known::CODEC_ID_VORBIS;
use symphonia_core::codecs::audio::{AudioCodecParameters, AudioDecoder, AudioDecoderOptions};
use symphonia_core::packet::PacketRef;
use symphonia_core::units::{Duration as SymDuration, Timestamp as SymTimestamp};

/// Output-rotation depth (the MJPEG/FLAC `FRAME_SLOTS` bound).
const SAMPLE_SLOTS: usize = 4;

/// The maximum channels symphonia's Vorbis channel mapper knows (its
/// `vorbis_channels_to_channels` covers 1–8; beyond is a typed
/// refusal here with OUR code, before the engine is ever built).
const MAX_MAPPED_CHANNELS: u8 = 8;

/// One decoded Vorbis block, format-independent form (the free
/// surface's result; the contract path lifts the same bytes into
/// `Frame::Audio`).
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedVorbis {
    /// Inter-channel sample index of the block's first sample.
    pub first_sample: u64,
    /// Block output length (inter-channel samples, half the MDCT
    /// block — the overlap-add emission).
    pub samples: usize,
    /// The output shape (F32, interleaved, native channels/rate).
    pub format: AudioFormat,
    /// Interleaved F32 bytes.
    pub data: Vec<u8>,
}

/// Build the engine from a validated header chain (shared by the
/// contract path and the free surface). `gapless` stays OFF: the Ogg
/// granule clock — not the engine — owns Vorbis positioning, and the
/// engine's gapless trimming would need trim metadata the container
/// never carries.
fn build_engine(headers: &VorbisHeaders) -> Result<Engine, VorbisError> {
    let ident = headers.identification;
    if ident.channels > MAX_MAPPED_CHANNELS {
        return Err(unsupported(
            "vorbis.ident.channels_unmapped",
            format!(
                "identification header declares {} channels; the engine maps at most \
                 {MAX_MAPPED_CHANNELS}",
                ident.channels
            ),
        ));
    }
    let mut params = AudioCodecParameters::new();
    params.codec = CODEC_ID_VORBIS;
    params.sample_rate = Some(ident.sample_rate);
    params.max_frames_per_packet = Some(u64::from(ident.blocksize_long) / 2);
    params.extra_data = Some(headers.engine_extra_data().into_boxed_slice());
    let mut opts = AudioDecoderOptions::default();
    // Gapless stays OFF: the Ogg granule clock — not the engine —
    // owns Vorbis positioning (see the module docs).
    opts.gapless = false;
    match Engine::try_new(&params, &opts) {
        Ok(engine) => Ok(engine),
        Err(err) => Err(corrupt(
            "vorbis.engine.init",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            format!("engine refused the header chain: {err}"),
        )),
    }
}

/// Decode one audio packet through a configured engine: interleaved
/// F32 samples land in `scratch` (reused), the byte-encoded form at
/// the front of `dst` (the reusable-fill form behind both surfaces).
///
/// Returns the block's sample count. `dst` must hold
/// [`max_output_len`] bytes — the pooled-slot caller pre-sizes it; a
/// short slot is a typed error, never a partial write.
pub(crate) fn decode_block_into(
    engine: &mut Engine,
    packet: &[u8],
    scratch: &mut Vec<f32>,
    dst: &mut [u8],
) -> Result<usize, VorbisError> {
    let pkt = PacketRef::new(0, SymTimestamp::ZERO, SymDuration::ZERO, packet);
    let decoded = match engine.decode_ref(&pkt) {
        Ok(buf) => buf,
        Err(err) => {
            // One-block poison boundary: drop the overlap state so the
            // next packet decodes from a clean window (doc 03 §4.5(d),
            // the overlap-scoped honest variant).
            engine.reset();
            return Err(corrupt(
                "vorbis.packet.decode",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                format!("engine rejected the audio packet: {err}"),
            ));
        }
    };
    let frames = decoded.frames();
    let channels = decoded.spec().channels().count();
    let need = frames * channels;
    if scratch.len() < need {
        scratch.resize(need, 0.0);
    }
    decoded.copy_to_slice_interleaved::<f32, _>(&mut scratch[..need]);
    let byte_len = need * 4;
    if dst.len() < byte_len {
        return Err(corrupt(
            "vorbis.output.slot_size",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            format!("output slot is {} bytes, block needs {byte_len}", dst.len()),
        ));
    }
    for (cursor, value) in scratch[..need].iter().enumerate() {
        dst[cursor * 4..cursor * 4 + 4].copy_from_slice(&value.to_ne_bytes());
    }
    Ok(frames)
}

/// Decode one audio packet on a fresh engine (the free surface: the
/// whole pipeline with no contract state — the `decode_frame` pattern
/// of 5.A1, and the first surface of the `fuzz_codec_vorbis`
/// harness).
///
/// # Errors
/// Typed [`EngineError`]s from the header chain or the decode stage
/// (see the module map). The block is the engine's cold-start shape:
/// `blocksize / 2` samples, the left half windowed against silence.
pub fn decode_block(packet: &[u8], headers: &VorbisHeaders) -> Result<DecodedVorbis, EngineError> {
    let ident = headers.identification;
    let mut engine = build_engine(headers).map_err(VorbisError::into_engine_error)?;
    let mut scratch = Vec::new();
    let mut out = vec![0u8; max_output_len(&ident)];
    let samples = decode_block_into(&mut engine, packet, &mut scratch, &mut out)
        .map_err(VorbisError::into_engine_error)?;
    let exact = samples * usize::from(ident.channels) * 4;
    out.truncate(exact);
    Ok(DecodedVorbis {
        first_sample: 0,
        samples,
        format: AudioFormat {
            sample: SampleFormat::F32,
            channels: u32::from(ident.channels),
            layout: AudioLayout::Interleaved,
            rate: ident.sample_rate,
        },
        data: out,
    })
}

/// The output-slot size a stream can ever need: half the long block ×
/// channels × 4 bytes (the pooled rotation is sized to this once at
/// configure).
pub(crate) fn max_output_len(ident: &Identification) -> usize {
    ident.blocksize_long as usize / 2 * usize::from(ident.channels) * 4
}

/// Vorbis decoder behind the codec-api [`Decoder`] contract (card
/// 5.B2; doc 04 §3.1 row "Vorbis").
///
/// `configure` installs the header chain (via `extra_data`,
/// identification ‖ comment ‖ setup), `decode` turns one audio packet
/// into one [`Frame::Audio`] whose pts is the granule-domain sample
/// index, `flush` is immediately drained (the overlap tail is the
/// container's business — the last half-block was already emitted).
///
/// # Example
///
/// ```
/// use codec_api::prelude::*;
/// use codecs::audio::vorbis::VorbisDecoder;
/// use media_types::{GenerationId, packet::SharedBytes};
///
/// # fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let mut decoder = VorbisDecoder::new();
/// let generation = GenerationId::FIRST.next()?;
/// // Without the header chain, configure is a typed rejection —
/// // Vorbis cannot decode a single block without its codebooks.
/// let err = decoder
///     .configure(&DecoderConfig::new(
///         media_types::ids::CodecDescriptor::new("vorbis")?,
///         generation,
///         ThreadBudget::default(),
///         None,
///         None,
///     ))
///     .expect_err("the header chain is mandatory");
/// assert_eq!(err.context.code, "vorbis.headers.missing");
/// # Ok(())
/// # }
/// # run().unwrap_or_else(|e| unreachable!("{e}"));
/// ```
#[derive(Default)]
pub struct VorbisDecoder {
    /// The validated header chain (None until `configure`).
    headers: Option<VorbisHeaders>,
    /// The configured stream generation (None until `configure`).
    generation: Option<GenerationId>,
    /// Codec descriptor accepted at configure time (diagnostics).
    codec: Option<String>,
    /// The symphonia engine (rebuilt at each configure).
    engine: Option<Engine>,
    /// Reused interleaved F32 scratch (the engine's planes → one
    /// buffer, handed back per call — D-1).
    scratch: Vec<f32>,
    /// Pooled output slots, each sized to the stream maximum (the
    /// FLAC `pooled_sample_slot` rotation; `SharedBytes::slice` cuts
    /// the exact per-block view).
    sample_pool: Vec<SharedBytes>,
    /// Round-robin cursor into `sample_pool`.
    slot_next: usize,
    /// The raw emission counter: total inter-channel samples emitted
    /// since configure (counts the encoder delay at the head — the
    /// granule-timeline position is `raw − shift`).
    raw_next: u64,
    /// The measured raw→granule timeline shift (the encoder delay at
    /// the stream head; re-derived after each seek landing). `None`
    /// until the first container stamp.
    shift: Option<i64>,
    /// Packets decoded since configure (diagnostics).
    packet_index: u64,
}

impl VorbisDecoder {
    /// A fresh decoder, unconfigured.
    #[must_use]
    pub fn new() -> VorbisDecoder {
        VorbisDecoder::default()
    }

    /// The codec descriptors this decoder accepts (`vorbis`; the
    /// registry vocabulary of doc 04 §3.1).
    fn accepts(descriptor: &str) -> bool {
        descriptor == "vorbis"
    }

    /// The negotiated audio shape a configured stream will emit (the
    /// registry/adapters' preview of [`AudioFormat`] without decoding).
    #[must_use]
    pub fn output_format(&self) -> Option<AudioFormat> {
        self.headers.as_ref().map(|headers| {
            let ident = headers.identification;
            AudioFormat {
                sample: SampleFormat::F32,
                channels: u32::from(ident.channels),
                layout: AudioLayout::Interleaved,
                rate: ident.sample_rate,
            }
        })
    }

    /// The granule-timeline position of the next block's first sample
    /// (`raw − shift`; the seek landing surface's observable).
    #[must_use]
    pub fn next_first_sample(&self) -> u64 {
        self.raw_next
            .saturating_sub_signed(self.shift.unwrap_or(0).max(0))
    }

    /// The measured raw→granule timeline shift (the encoder delay at
    /// the stream head; `None` until the first container stamp — the
    /// playback layer's head-trim coordinate).
    #[must_use]
    pub fn timeline_shift(&self) -> Option<i64> {
        self.shift
    }

    /// Total raw inter-channel samples emitted since configure.
    #[must_use]
    pub fn emitted_total(&self) -> u64 {
        self.raw_next
    }
}

impl Decoder for VorbisDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        let descriptor = cfg.codec.as_str();
        if !Self::accepts(descriptor) {
            return Err(unsupported(
                "vorbis.codec_descriptor",
                format!("codec descriptor `{descriptor}` is not `vorbis`"),
            )
            .into_engine_error());
        }
        let body = cfg
            .extra_data
            .as_ref()
            .map(|d| d.as_slice())
            .ok_or_else(|| {
                unsupported(
                    "vorbis.headers.missing",
                    "no extra_data: the decoder requires the stream's three header packets \
                     (identification ‖ comment ‖ setup) and refuses to guess the codebooks",
                )
                .into_engine_error()
            })?;
        let headers = VorbisHeaders::from_extra_data(body)?;
        let engine = build_engine(&headers).map_err(VorbisError::into_engine_error)?;
        // The pooled output rotation is sized once per configuration
        // (bounded: SAMPLE_SLOTS × the stream maximum).
        self.sample_pool.clear();
        self.scratch.clear();
        self.slot_next = 0;
        self.raw_next = 0;
        self.shift = None;
        self.packet_index = 0;
        self.headers = Some(headers);
        self.engine = Some(engine);
        self.generation = Some(cfg.generation);
        self.codec = Some(descriptor.to_string());
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        let generation = match self.generation {
            Some(g) => g,
            None => return Err(VorbisError::BadState.into_engine_error()),
        };
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        let headers = match self.headers.as_ref() {
            Some(headers) => headers,
            None => return Err(VorbisError::BadState.into_engine_error()),
        };
        if packet.data.is_empty() {
            return Ok(DecodeOutcome::NeedMoreInput);
        }
        // A discontinuity (seek reanchor or damage gap): drop the
        // overlap-add state before decoding — the landing packet must
        // not be smeared through a stale window.
        if packet.discontinuity
            && let Some(engine) = self.engine.as_mut()
        {
            engine.reset();
        }
        let ident = headers.identification;
        let slot_len = max_output_len(&ident);
        let raw_first = self.raw_next;
        let shift_now = self.shift;
        let VorbisDecoder {
            engine,
            scratch,
            sample_pool,
            slot_next,
            ..
        } = self;
        let fill = |dst: &mut [u8]| -> Result<usize, VorbisError> {
            let engine = engine.as_mut().ok_or(VorbisError::BadState)?;
            decode_block_into(engine, packet.data.as_slice(), scratch, dst)
        };
        let PooledBlock { emitted, bytes } =
            pooled_sample_slot(sample_pool, slot_next, slot_len, fill)
                .map_err(VorbisError::into_engine_error)?;
        let exact = emitted * usize::from(ident.channels) * 4;
        // The granule clock: pts is this block's first sample on the
        // VALID timeline (`raw_first − shift`); before the first
        // container stamp the position is honestly unknown, and the
        // stamp at the end of this packet re-measures the shift (see
        // `super::seek`).
        let base = TimeBase::new(1, ident.sample_rate).map_err(|_| {
            unsupported("vorbis.ident.rate", "configured facts carry rate 0").into_engine_error()
        })?;
        let pts = shift_now.map(|shift| {
            Stamp::Present(Timestamp::new(
                i64::try_from(raw_first).unwrap_or(i64::MAX) - shift,
                base,
            ))
        });
        let frame = Frame::Audio(media_types::frame::AudioFrame {
            format: AudioFormat {
                sample: SampleFormat::F32,
                channels: u32::from(ident.channels),
                layout: AudioLayout::Interleaved,
                rate: ident.sample_rate,
            },
            samples: bytes.slice(0..exact),
            pts: pts.unwrap_or(Stamp::Missing),
            duration: Some(Duration::Known(Nanoseconds(
                u64::try_from(
                    u128::from(emitted as u64) * 1_000_000_000u128 / u128::from(ident.sample_rate),
                )
                .unwrap_or(u64::MAX),
            ))),
            generation: packet.generation,
        });
        out.accept(frame)?;
        self.raw_next = raw_first.saturating_add(emitted as u64);
        if let Some(fresh) = measure_shift(self.raw_next, packet.pts, &base) {
            self.shift = Some(fresh);
        }
        self.packet_index += 1;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        if self.generation.is_none() || self.engine.is_none() {
            return Err(VorbisError::BadState.into_engine_error());
        }
        // The overlap tail was already emitted with the last block
        // (each packet emits its own half); nothing is buffered.
        Ok(DecodeOutcome::Drained)
    }
}

/// One pooled-slot fill result: the emitted sample count plus the
/// bytes (the slot view, exact-frame slice pending at the call site).
struct PooledBlock {
    emitted: usize,
    bytes: SharedBytes,
}

/// The pooled output rotation (the 5.A1 `pooled_sample_slot` pattern,
/// f32 path): reuse a slot whose bytes are exclusively owned, else
/// allocate a fresh one and enter the rotation (bounded at
/// [`SAMPLE_SLOTS`], then evicting). Every slot is `slot_len` bytes —
/// the stream maximum; `decode_block_into` writes the actual block at
/// the front and the caller slices.
fn pooled_sample_slot(
    slots: &mut Vec<SharedBytes>,
    next: &mut usize,
    slot_len: usize,
    fill: impl FnOnce(&mut [u8]) -> Result<usize, VorbisError>,
) -> Result<PooledBlock, VorbisError> {
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != slot_len {
            continue;
        }
        if let Some(bytes) = slots[idx].get_mut() {
            let emitted = fill(bytes)?;
            *next = (idx + 1) % count;
            return Ok(PooledBlock {
                emitted,
                bytes: slots[idx].clone(),
            });
        }
        // The slot still has downstream owners — try the next one.
    }
    // Cold path: allocate, fill in place, then enter the rotation.
    let mut fresh = SharedBytes::from_vec(vec![0; slot_len]);
    let emitted = match fresh.get_mut() {
        Some(bytes) => fill(bytes),
        None => {
            // A freshly built `SharedBytes` has refcount 1 by
            // construction; `get_mut` failing here is impossible —
            // surface it loudly instead of fabricating a frame.
            return Err(corrupt(
                "vorbis.output.fresh_slot_not_exclusive",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                "fresh output slot is not exclusively owned",
            ));
        }
    }?;
    if slots.len() < SAMPLE_SLOTS {
        slots.push(fresh.clone());
        *next = slots.len() - 1;
    } else {
        *next = (*next + 1) % SAMPLE_SLOTS;
        slots[*next] = fresh.clone();
    }
    Ok(PooledBlock {
        emitted,
        bytes: fresh,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_gate_accepts_only_vorbis() {
        assert!(VorbisDecoder::accepts("vorbis"));
        assert!(!VorbisDecoder::accepts("Vorbis"));
        assert!(!VorbisDecoder::accepts("opus"));
        assert!(!VorbisDecoder::accepts("flac"));
        assert!(!VorbisDecoder::accepts(""));
    }

    #[test]
    fn decode_before_configure_is_typed_bad_state() {
        let mut decoder = VorbisDecoder::new();
        let generation = match GenerationId::FIRST.next() {
            Ok(g) => g,
            Err(e) => unreachable!("{e:?}"),
        };
        let packet = Packet {
            track: media_types::TrackId::new(0),
            pts: Stamp::Missing,
            dts: Stamp::Missing,
            duration: None,
            data: SharedBytes::from_vec(Vec::new()),
            keyframe: true,
            discontinuity: false,
            generation,
        };
        struct Sink;
        impl FrameSink for Sink {
            fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
                Ok(())
            }
        }
        let e = match decoder.decode(packet, &mut Sink) {
            Err(e) => e,
            Ok(_) => unreachable!("must be BadState"),
        };
        assert_eq!(e.context.code, "vorbis.not_configured");
        assert!(matches!(e.kind, media_types::ErrorKind::Decoder { .. }));
    }

    #[test]
    fn configure_rejects_foreign_descriptors_and_short_chains() {
        let mut decoder = VorbisDecoder::new();
        let generation = match GenerationId::FIRST.next() {
            Ok(g) => g,
            Err(e) => unreachable!("{e:?}"),
        };
        let e = match decoder.configure(&DecoderConfig::new(
            match media_types::ids::CodecDescriptor::new("opus") {
                Ok(d) => d,
                Err(e) => unreachable!("{e:?}"),
            },
            generation,
            ThreadBudget::default(),
            None,
            None,
        )) {
            Err(e) => e,
            Ok(()) => unreachable!("opus must be refused"),
        };
        assert_eq!(e.context.code, "vorbis.codec_descriptor");
        let e = match decoder.configure(&DecoderConfig::new(
            match media_types::ids::CodecDescriptor::new("vorbis") {
                Ok(d) => d,
                Err(e) => unreachable!("{e:?}"),
            },
            generation,
            ThreadBudget::default(),
            None,
            Some(SharedBytes::from_vec(vec![1, 2, 3])),
        )) {
            Err(e) => e,
            Ok(()) => unreachable!("a 3-byte chain must be refused"),
        };
        assert_eq!(e.context.code, "vorbis.ident.truncated");
    }
}
