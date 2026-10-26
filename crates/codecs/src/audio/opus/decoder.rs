//! The Opus decoder core (card 6.M-opus): the `ruopus`-backed packet
//! decode, the granule-domain valid-timeline window (pre-skip +
//! end-trim), and the [`codec_api::Decoder`] implementation.
//!
//! # Engine (the 6.M-opus admission decision)
//!
//! **`ruopus` 0.1.2, exact-pinned, `default-features = false,
//! features = ["std"]`** — the admission record's binding condition 1
//! (the zero-dependency shape; default features would drag a
//! 23-package spectrogram/ndarray graph and the `python` feature
//! pyo3). Purity verified in
//! `docs/dependencies/records/ruopus-0.1.2.md` before the Cargo.toml
//! edit (pure Rust, SIMD-kernel `unsafe` only, no build.rs, no FFI).
//! The engine owns exactly one job: the CELT/SILK decode of ONE
//! framed Opus packet (TOC included) → interleaved f32 at 48 kHz.
//! The OpusHead grammar, the granule arithmetic and the error
//! taxonomy are ours.
//!
//! # The two binding conditions (admission record, honored here)
//!
//! 1. **The pre-constructor panic gate**: the ruopus constructors
//!    panic on channels∉{1,2} (mono engine) and rate∉{8..48}k —
//!    [`super::head`] closes the whole geometry from OpusHead
//!    *before* any engine is built, and the decode rate is the
//!    constant 48 kHz (RFC 7845 §4.2: granules and decode both count
//!    48 kHz samples; the head's `input_rate` is advisory).
//! 2. **The valid-timeline window**: Opus decodes MORE than the
//!    valid timeline at both ends. The head carries **pre-skip**
//!    (samples to discard at the head — the encoder look-ahead,
//!    measured 312 on every corpus lane) and the container's last
//!    granulepos bounds the tail (**end-trim**, measured 648 on every
//!    corpus lane). This decoder keeps a raw emission counter
//!    (pre-skip-included, granule-domain) and windows every packet's
//!    output to `[max(raw, pre-skip) .. min(raw+len, granule)]` —
//!    the granule arrives on the packet's pts whenever the packet
//!    completes an Ogg page (the demuxer stamps only page-completing
//!    packets), which is exactly the RFC 7845 §4 end-trim seam. A
//!    `discontinuity` (seek landing) re-anchors the counter from the
//!    landing stamp; stamped positions also heal counter drift after
//!    dropped-damage packets (monotonic max).
//!
//! # Channel order (the output seam)
//!
//! Mapping family 1 output is permuted from ruopus's RFC 7845
//! Vorbis order into the house WAV order at the copy boundary
//! ([`super::order`]) — the same order every other decoder here
//! emits (the PCM passthrough convention; ffmpeg's native layout).
//! Family 0 is identity; family 255 (discrete) passes through,
//! honestly undefined.
//!
//! # Output gain
//!
//! OpusHead's Q7.8 output gain is applied as a linear multiplication
//! (`sample × gain/256`) on the copy path (RFC 7845 §4.2). Every
//! corpus lane measures gain = 0 (unity) — the multiplication is a
//! no-op there, and the gain≠0 reading is recorded as an untested
//! honest gap in the worklog.
//!
//! # Output format (codec-native F32)
//!
//! Interleaved native-endian F32 at 48 kHz, `channels` wide — the
//! same shape as the Vorbis/MP3/FLAC paths. Lossy codec: the
//! differential suite pins OUR output against ffmpeg's float decoder
//! within the recorded tolerance class (max |Δ| ≤ one s16 LSB, zero
//! samples beyond it — two independent float decoders are not
//! bit-exact, by the admission record's own measurement).
//!
//! # Allocation discipline (D-1)
//!
//! The pooled-slot rotation of the Vorbis pattern, verbatim: slots
//! sized once at configure to the stream maximum (120 ms × 48 kHz ×
//! channels × 4 bytes — the RFC 6716 §3 frame-duration ceiling),
//! reused round-robin, exact-length slices handed downstream.

use super::error::{OpusError, corrupt, unsupported};
use super::head::{DECODE_RATE, OpusHead, parse_head};
use super::order::vorbis_to_house;
use codec_api::prelude::*;
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::time::{Nanoseconds, TimeBase, Timestamp};
use media_types::{Duration, Stamp};
use ruopus::MultistreamDecoder as RuopusMultistream;
use ruopus::OpusDecoder as RuopusMono;

/// Output-rotation depth (the MJPEG/FLAC/Vorbis `FRAME_SLOTS` bound).
const SAMPLE_SLOTS: usize = 4;

/// The maximum samples per channel one Opus packet can decode: the
/// RFC 6716 §3 duration ceiling (120 ms) at 48 kHz (code-3 packets,
/// up to 48 × 2.5 ms frames; DTX concealment stays under it).
const MAX_PACKET_SAMPLES: usize = 5760;

/// The engine half (the mono/stereo and multistream constructors
/// share the decode surface; the head's mapping family selected one
/// at configure).
enum Engine {
    /// Mapping family 0 (1–2 channels, unmapped).
    Mono(RuopusMono),
    /// Mapping family ≥ 1 (streams + coupled + mapping, validated).
    Multi(RuopusMultistream),
}

impl Engine {
    /// Decode one packet → interleaved f32 at 48 kHz (the engine's
    /// typed `PacketError` is the hostile-input contract — never a
    /// panic, per the admission record's 6-case measurement).
    fn decode(&mut self, data: &[u8]) -> Result<Vec<f32>, ruopus::PacketError> {
        match self {
            Engine::Mono(engine) => engine.decode_packet(data),
            Engine::Multi(engine) => engine.decode_packet(data),
        }
    }
}

/// Build the engine from a validated head (the pre-constructor gate
/// has already closed every panic window — see [`super::head`]).
fn build_engine(head: &OpusHead) -> Result<Engine, OpusError> {
    if head.mapping_family == 0 {
        // channels ∈ {1,2} is proven by parse_head; the rate is the
        // DECODE_RATE constant (48 kHz — in the accepted set).
        Ok(Engine::Mono(RuopusMono::with_rate(
            DECODE_RATE,
            usize::from(head.channels),
        )))
    } else {
        // streams ≥ 1, coupled ≤ streams, streams+coupled = channels
        // ≤ 255, mapping entries in-slot-or-silent: all proven by
        // parse_head before this call.
        Ok(Engine::Multi(RuopusMultistream::with_rate(
            DECODE_RATE,
            usize::from(head.streams),
            usize::from(head.coupled),
            &head.mapping,
        )))
    }
}

/// The output-slot size a stream can ever need (the pooled rotation
/// is sized to this once at configure).
fn max_output_len(head: &OpusHead) -> usize {
    MAX_PACKET_SAMPLES * usize::from(head.channels) * 4
}

/// Opus decoder behind the codec-api [`Decoder`] contract (card
/// 6.M-opus; doc 04 §3.1 row "Opus").
///
/// `configure` installs the OpusHead (via `extra_data`, the Ogg
/// host's `OpusHead ‖ OpusTags` verbatim join), `decode` turns one
/// audio packet into one [`Frame::Audio`] whose pts is the
/// valid-timeline sample index (pre-skip-trimmed, granule-clamped),
/// `flush` is immediately drained (each packet is independently
/// coded — the valid tail was trimmed by the last granule).
///
/// # Example
///
/// ```
/// use codec_api::prelude::*;
/// use codecs::audio::opus::OpusDecoder;
/// use media_types::{GenerationId, packet::SharedBytes};
///
/// # fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let mut decoder = OpusDecoder::new();
/// let generation = GenerationId::FIRST.next()?;
/// // Without the OpusHead, configure is a typed rejection — Opus
/// // cannot decode a single packet without its channel geometry.
/// let err = decoder
///     .configure(&DecoderConfig::new(
///         media_types::ids::CodecDescriptor::new("opus")?,
///         generation,
///         ThreadBudget::default(),
///         None,
///         None,
///     ))
///     .expect_err("the OpusHead is mandatory");
/// assert_eq!(err.context.code, "opus.head.missing");
/// # Ok(())
/// # }
/// # run().unwrap_or_else(|e| unreachable!("{e}"));
/// ```
#[derive(Default)]
pub struct OpusDecoder {
    /// The validated OpusHead (None until `configure`).
    head: Option<OpusHead>,
    /// The configured stream generation (None until `configure`).
    generation: Option<GenerationId>,
    /// Codec descriptor accepted at configure time (diagnostics).
    codec: Option<String>,
    /// The ruopus engine (rebuilt at each configure).
    engine: Option<Engine>,
    /// The output-channel permutation (Vorbis → house order;
    /// `None` = identity / discrete passthrough).
    permutation: Option<Vec<u8>>,
    /// The output gain multiplier (OpusHead Q7.8; 1.0 = unity).
    gain: f32,
    /// Pooled output slots, each sized to the stream maximum.
    sample_pool: Vec<SharedBytes>,
    /// Round-robin cursor into `sample_pool`.
    slot_next: usize,
    /// The raw emission counter: total per-channel samples decoded
    /// since configure, in the pre-skip-included granule domain (the
    /// valid-timeline position is `raw − pre_skip`).
    raw_next: u64,
    /// Packets decoded since configure (diagnostics).
    packet_index: u64,
}

impl OpusDecoder {
    /// A fresh decoder, unconfigured.
    #[must_use]
    pub fn new() -> OpusDecoder {
        OpusDecoder::default()
    }

    /// The codec descriptors this decoder accepts (`opus`; the
    /// registry vocabulary of doc 04 §3.1).
    fn accepts(descriptor: &str) -> bool {
        descriptor == "opus"
    }

    /// The negotiated audio shape a configured stream will emit (the
    /// registry/adapters' preview of [`AudioFormat`] without decoding).
    #[must_use]
    pub fn output_format(&self) -> Option<AudioFormat> {
        self.head.as_ref().map(|head| AudioFormat {
            sample: SampleFormat::F32,
            channels: u32::from(head.channels),
            layout: AudioLayout::Interleaved,
            rate: DECODE_RATE,
        })
    }

    /// The pre-skip the stream head declared (samples; the
    /// valid-timeline head-trim coordinate).
    #[must_use]
    pub fn pre_skip(&self) -> u16 {
        self.head.as_ref().map_or(0, |head| head.pre_skip)
    }

    /// Total raw per-channel samples decoded since configure (the
    /// pre-skip-included counter; the differential's geometry check).
    #[must_use]
    pub fn decoded_total(&self) -> u64 {
        self.raw_next
    }
}

impl Decoder for OpusDecoder {
    fn configure(&mut self, cfg: &DecoderConfig) -> Result<(), EngineError> {
        let descriptor = cfg.codec.as_str();
        if !Self::accepts(descriptor) {
            return Err(unsupported(
                "opus.codec_descriptor",
                format!("codec descriptor `{descriptor}` is not `opus`"),
            )
            .into_engine_error());
        }
        let blob = cfg
            .extra_data
            .as_ref()
            .map(|d| d.as_slice())
            .ok_or_else(|| {
                unsupported(
                    "opus.head.missing",
                    "no extra_data: the decoder requires the OpusHead ‖ OpusTags header chain \
                     (the Ogg host's verbatim join) and refuses to guess the channel geometry",
                )
                .into_engine_error()
            })?;
        let head = parse_head(blob)?;
        let engine = build_engine(&head)?;
        let permutation = if head.mapping_family == 1 {
            vorbis_to_house(head.channels).map(|perm| perm.to_vec())
        } else {
            // Family 0 is order-identity; family 255 is discrete
            // (undefined — passthrough, typed honestly in order.rs).
            None
        };
        self.sample_pool.clear();
        self.slot_next = 0;
        self.raw_next = 0;
        self.packet_index = 0;
        self.gain = f32::from(head.output_gain) / 256.0;
        self.permutation = permutation;
        self.head = Some(head);
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
            None => return Err(OpusError::BadState.into_engine_error()),
        };
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
        let head = match self.head.clone() {
            Some(head) => head,
            None => return Err(OpusError::BadState.into_engine_error()),
        };
        let base = TimeBase::new(1, DECODE_RATE).map_err(|_| {
            unsupported("opus.rate.zero", "the 48 kHz decode base is a constant").into_engine_error()
        })?;
        // The container stamp, in the granule domain, on the packets
        // that complete an Ogg page (the demuxer's granule-ownership
        // rule): the valid-timeline END of this packet — the end-trim
        // coordinate. Foreign bases / absent stamps carry no trim
        // information (emit everything; the window is honest).
        let granule: Option<i64> = match packet.pts {
            Stamp::Present(ts) if ts.base == base && ts.ticks >= 0 => Some(ts.ticks),
            _ => None,
        };
        let channels = usize::from(head.channels);
        // 1. The engine decode (typed errors, never a panic).
        let decoded = match self.engine.as_mut() {
            Some(engine) => engine.decode(packet.data.as_slice()),
            None => return Err(OpusError::BadState.into_engine_error()),
        }
        .map_err(|err| {
            corrupt(
                "opus.packet.decode",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                format!("engine rejected the audio packet: {err:?}"),
            )
            .into_engine_error()
        })?;
        if decoded.len() % channels != 0 {
            return Err(corrupt(
                "opus.packet.channel_multiple",
                media_types::error::CorruptDetail::InvalidStructure { offset: None },
                format!(
                    "engine emitted {} samples for a {channels}-channel stream (not a \
                     whole-sample multiple)",
                    decoded.len()
                ),
            )
            .into_engine_error());
        }
        let packet_samples = decoded.len() / channels;
        // 2. Positioning: the raw counter, re-anchored at a seek
        // landing (a stamped discontinuity packet ends at its
        // granule) and monotonic against every stamp (dropped-damage
        // healing — the counter never lags the container).
        let mut raw_before = self.raw_next;
        if packet.discontinuity && let Some(end) = granule {
            raw_before = (u64::try_from(end).unwrap_or(0)).saturating_sub(packet_samples as u64);
        }
        let raw_after = raw_before + packet_samples as u64;
        // 3. The valid-timeline window: head-trim at pre-skip,
        // tail-trim at the stamped granule (binding condition 2).
        let start = raw_before.max(u64::from(head.pre_skip));
        let end = granule
            .map(|g| raw_after.min(u64::try_from(g).unwrap_or(raw_after)))
            .unwrap_or(raw_after);
        let window = end.saturating_sub(start);
        // The window is bounded by this packet's own sample count —
        // a usize by construction, never a truncation.
        let window = usize::try_from(window).unwrap_or(usize::MAX);
        self.raw_next = match granule {
            Some(g) => raw_after.max(u64::try_from(g).unwrap_or(raw_after)),
            None => raw_after,
        };
        self.packet_index += 1;
        if window == 0 {
            // Fully inside the pre-skip (a head packet shorter than
            // the look-ahead) or fully past the last granule: the
            // packet was consumed, nothing valid to emit.
            return Ok(DecodeOutcome::NeedMoreInput);
        }
        // 4. The copy: window + channel-order permutation + output
        // gain, into a pooled slot (D-1).
        let slot_len = max_output_len(&head);
        let permutation = self.permutation.clone();
        let gain = self.gain;
        let scratch_offset = usize::try_from(start - raw_before).unwrap_or(0) * channels;
        let fill = |dst: &mut [u8]| -> Result<usize, OpusError> {
            let exact = window * channels * 4;
            debug_assert!(exact <= dst.len());
            if dst.len() < exact {
                return Err(corrupt(
                    "opus.output.slot_size",
                    media_types::error::CorruptDetail::InvalidStructure { offset: None },
                    format!("output slot is {} bytes, packet needs {exact}", dst.len()),
                ));
            }
            for frame in 0..window {
                for (channel, target) in (0..channels).enumerate() {
                    let source = match &permutation {
                        Some(perm) => usize::from(perm[channel]),
                        None => channel,
                    };
                    let sample = if gain == 1.0 {
                        decoded[scratch_offset + frame * channels + source]
                    } else {
                        decoded[scratch_offset + frame * channels + source] * gain
                    };
                    let at = (frame * channels + target) * 4;
                    dst[at..at + 4].copy_from_slice(&sample.to_ne_bytes());
                }
            }
            Ok(window)
        };
        let PooledWindow { bytes } =
            pooled_sample_slot(&mut self.sample_pool, &mut self.slot_next, slot_len, fill)?;
        let exact = window * channels * 4;
        let pts = Timestamp::new(
            i64::try_from(start - u64::from(head.pre_skip)).unwrap_or(i64::MAX),
            base,
        );
        let frame = Frame::Audio(media_types::frame::AudioFrame {
            format: AudioFormat {
                sample: SampleFormat::F32,
                channels: u32::from(head.channels),
                layout: AudioLayout::Interleaved,
                rate: DECODE_RATE,
            },
            samples: bytes.slice(0..exact),
            pts: Stamp::Present(pts),
            duration: Some(Duration::Known(Nanoseconds(
                u64::try_from(u128::from(window as u64) * 1_000_000_000u128 / u128::from(DECODE_RATE))
                    .unwrap_or(u64::MAX),
            ))),
            generation: packet.generation,
        });
        out.accept(frame)?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        if self.generation.is_none() || self.engine.is_none() {
            return Err(OpusError::BadState.into_engine_error());
        }
        // Each packet is independently coded: the valid tail was
        // trimmed by the last stamped granule; nothing is buffered.
        Ok(DecodeOutcome::Drained)
    }
}

/// One pooled-slot fill result: the slot view (the caller slices the
/// exact window).
struct PooledWindow {
    bytes: SharedBytes,
}

/// The pooled output rotation (the Vorbis `pooled_sample_slot`
/// pattern, verbatim shape): reuse a slot whose bytes are
/// exclusively owned, else allocate a fresh one and enter the
/// rotation (bounded at [`SAMPLE_SLOTS`], then evicting).
fn pooled_sample_slot(
    slots: &mut Vec<SharedBytes>,
    next: &mut usize,
    slot_len: usize,
    fill: impl FnOnce(&mut [u8]) -> Result<usize, OpusError>,
) -> Result<PooledWindow, OpusError> {
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != slot_len {
            continue;
        }
        if let Some(bytes) = slots[idx].get_mut() {
            fill(bytes)?;
            *next = (idx + 1) % count;
            return Ok(PooledWindow {
                bytes: slots[idx].clone(),
            });
        }
    }
    let mut fresh = SharedBytes::from_vec(vec![0; slot_len]);
    match fresh.get_mut() {
        Some(bytes) => fill(bytes),
        None => Err(corrupt(
            "opus.output.fresh_slot_not_exclusive",
            media_types::error::CorruptDetail::InvalidStructure { offset: None },
            "fresh output slot is not exclusively owned",
        )),
    }?;
    if slots.len() < SAMPLE_SLOTS {
        slots.push(fresh.clone());
        *next = slots.len() - 1;
    } else {
        *next = (*next + 1) % SAMPLE_SLOTS;
        slots[*next] = fresh.clone();
    }
    Ok(PooledWindow { bytes: fresh })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal family-0 stereo head + OpusTags tail, the Ogg
    /// host's extra_data shape.
    fn extra_v0() -> Vec<u8> {
        let mut blob = b"OpusHead".to_vec();
        blob.extend_from_slice(&[1, 2]); // version 1, stereo
        blob.extend_from_slice(&312u16.to_le_bytes()); // pre-skip
        blob.extend_from_slice(&48_000u32.to_le_bytes()); // input rate
        blob.extend_from_slice(&0i16.to_le_bytes()); // gain 0
        blob.push(0); // mapping family 0
        blob.extend_from_slice(b"OpusTags\x00\x00"); // comment tail (unvalidated)
        blob
    }

    fn config(extra: Option<Vec<u8>>) -> DecoderConfig {
        DecoderConfig::new(
            match media_types::ids::CodecDescriptor::new("opus") {
                Ok(d) => d,
                Err(e) => unreachable!("{e:?}"),
            },
            match GenerationId::FIRST.next() {
                Ok(g) => g,
                Err(e) => unreachable!("{e:?}"),
            },
            ThreadBudget::default(),
            None,
            extra.map(media_types::packet::SharedBytes::from_vec),
        )
    }

    #[test]
    fn descriptor_gate_accepts_only_opus() {
        assert!(OpusDecoder::accepts("opus"));
        assert!(!OpusDecoder::accepts("Opus"));
        assert!(!OpusDecoder::accepts("vorbis"));
        assert!(!OpusDecoder::accepts(""));
    }

    #[test]
    fn configure_requires_and_validates_the_head() {
        let mut decoder = OpusDecoder::new();
        let e = match decoder.configure(&config(None)) {
            Err(e) => e,
            Ok(()) => unreachable!("the OpusHead is mandatory"),
        };
        assert_eq!(e.context.code, "opus.head.missing");
        // A family-0 head with 3 channels: the constructor panic
        // window, refused typed at configure.
        let mut bad = extra_v0();
        bad[9] = 3;
        let e = match decoder.configure(&config(Some(bad))) {
            Err(e) => e,
            Ok(()) => unreachable!("3-channel family-0 head must refuse"),
        };
        assert_eq!(e.context.code, "opus.head.family0_channels");
        // The good head configures and negotiates 48 kHz stereo F32.
        decoder
            .configure(&config(Some(extra_v0())))
            .unwrap_or_else(|e| unreachable!("{e}"));
        let format = decoder.output_format().unwrap_or_else(|| unreachable!("configured"));
        assert_eq!(format.channels, 2);
        assert_eq!(format.rate, 48_000);
        assert_eq!(format.sample, SampleFormat::F32);
        assert_eq!(decoder.pre_skip(), 312);
    }

    #[test]
    fn decode_before_configure_is_typed_bad_state() {
        let mut decoder = OpusDecoder::new();
        let packet = Packet {
            track: media_types::TrackId::new(0),
            pts: Stamp::Missing,
            dts: Stamp::Missing,
            duration: None,
            data: media_types::packet::SharedBytes::from_vec(vec![0x38]),
            keyframe: true,
            discontinuity: false,
            generation: match GenerationId::FIRST.next() {
                Ok(g) => g,
                Err(e) => unreachable!("{e:?}"),
            },
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
        assert_eq!(e.context.code, "opus.not_configured");
        assert!(matches!(e.kind, media_types::ErrorKind::Decoder { .. }));
    }

    #[test]
    fn hostile_packets_never_panic_and_isolate_to_the_packet() {
        // The admission record's 6-case hostile class, through the
        // CONTRACT path (the corpus lanes warm the decoder first;
        // here a freshly configured one exercises the cold path):
        // every case is an Ok (consumed, possibly zero-window) or a
        // typed Corrupt — never a panic, and the decoder keeps
        // decoding afterwards.
        let mut decoder = OpusDecoder::new();
        decoder
            .configure(&config(Some(extra_v0())))
            .unwrap_or_else(|e| unreachable!("{e}"));
        struct Sink;
        impl FrameSink for Sink {
            fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
                Ok(())
            }
        }
        let cases: [(&str, Vec<u8>); 6] = [
            ("truncated_toc_body", vec![0xFB, 0xFF, 0xE0]),
            (
                "garbage",
                vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11, 0x22, 0x33],
            ),
            ("zero_length", Vec::new()),
            ("one_byte_toc", vec![0x38]),
            ("huge_ff", vec![0xFF; 4096]),
            ("huge_zeros", vec![0x00; 4096]),
        ];
        let generation = match GenerationId::FIRST.next() {
            Ok(g) => g,
            Err(e) => unreachable!("{e:?}"),
        };
        for (name, data) in cases {
            let packet = Packet {
                track: media_types::TrackId::new(0),
                pts: Stamp::Missing,
                dts: Stamp::Missing,
                duration: None,
                data: media_types::packet::SharedBytes::from_vec(data),
                keyframe: true,
                discontinuity: false,
                generation,
            };
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                decoder.decode(packet, &mut Sink)
            }))
            .unwrap_or_else(|_| unreachable!("hostile case {name} must not panic"));
            match outcome {
                Ok(_) | Err(_) => {}
            }
        }
        // The decoder still decodes after the barrage (packet-scoped
        // isolation): a DTX one-byte packet emits a concealment frame.
        let packet = Packet {
            track: media_types::TrackId::new(0),
            pts: Stamp::Missing,
            dts: Stamp::Missing,
            duration: None,
            data: media_types::packet::SharedBytes::from_vec(vec![0x38]),
            keyframe: true,
            discontinuity: false,
            generation,
        };
        let outcome = decoder.decode(packet, &mut Sink);
        assert!(outcome.is_ok(), "decoder survives the hostile barrage");
    }
}
