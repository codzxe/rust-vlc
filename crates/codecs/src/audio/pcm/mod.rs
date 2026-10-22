//! Unpacked-PCM audio decoder (card 2.B5; doc 04 §3.1 row "PCM",
//! doc 03 §7 first vertical — the audio path of the AVI+MJPEG+PCM
//! slice).
//!
//! # The 2.B5 dependency decision (recorded here, evidence in worklog)
//!
//! The card offered `symphonia-codec-pcm` or a self-build. The
//! decision is **in-house**, extending D-018's zero-new-dependency
//! logic from the container pick to the audio codec, on the same
//! three grounds the Lead's MJPEG ratification recorded (worklog
//! 2.B4):
//!
//! 1. **C5 discipline**: `symphonia-codec-pcm` would be the `codecs`
//!    crate's first external engine dependency — a full doc 11 §1.3
//!    intake (F1–F7 tarball audit, R1–R10 record, Lead-ratified
//!    allowlist row) to buy functionality that is byte-order +
//!    width normalization + planar/interleaved accounting: a few
//!    hundred lines of std-only, panic-free Rust with no entropy
//!    coding to get subtly wrong.
//! 2. **Ground truth in-repo**: the 2.A3 fixture writer's
//!    NCO-generated s16le stream and its committed
//!    `audio_stream_sha256` pin the decoder's output bit-exactly —
//!    a stronger conformance anchor than a third-party crate's own
//!    test suite (which we cannot audit without the intake anyway).
//! 3. **Output-format control**: the media-types vocabulary is
//!    native-endian per sample width with an explicit
//!    interleaved/planar flag — the in-house decoder normalizes to
//!    exactly that (and can pass a clean native-endian packet
//!    through zero-copy, which a generic crate has no reason to
//!    promise).
//!
//! Doc 04 §3.1's row lists `symphonia-codec-pcm` as the candidate and
//! "none" under self-build (a crate exists); the Lead's D-018
//! precedent overrides "a crate exists" with "the first vertical
//! takes zero new dependencies". The external crate remains a future
//! fallback, owed a full audit before any adoption `[VERIFY: not
//! audited in this card — the in-house path made it moot]`.
//!
//! # What "PCM" means here (the format matrix)
//!
//! Unpacked PCM is a width + signedness + byte-order + layout tuple
//! (doc 04 §3.2 "PCM variants"). The v0 vocabulary ([`PcmFormat`] ×
//! layout) covers the card's matrix — **s16/s24/s32/f32/f64, each
//! little- and big-endian, interleaved and planar** — plus u8 (the
//! demuxer's AVI descriptor vocabulary already emits `pcm_u8` for 8-bit
//! streams, so the decoder accepts what its sibling crate promises):
//!
//! | descriptor | sample | width | notes |
//! |---|---|---|---|
//! | `pcm_u8` | U8 | 1 | unsigned, byte-order-free |
//! | `pcm_s16le` / `pcm_s16be` | S16 | 2 | the fixture format is `pcm_s16le` |
//! | `pcm_s24le` / `pcm_s24be` | S24 | 3 | 3-byte groups, no padding |
//! | `pcm_s32le` / `pcm_s32be` | S32 | 4 | |
//! | `pcm_f32le` / `pcm_f32be` | F32 | 4 | IEEE 754 |
//! | `pcm_f64le` / `pcm_f64be` | F64 | 8 | IEEE 754 (the AVI float-64 path) |
//!
//! Any of the above with a `_planar` suffix is the planar variant
//! (one plane per channel, concatenated in channel order). Honest
//! gaps, typed [`ErrorKind::Unsupported`](media_types::ErrorKind::Unsupported)
//! at `configure`: the bare `pcm` descriptor (the historical demuxer
//! fallback for unknown widths, never emitted since the demuxer's
//! sample vocabulary closed — the decoder still refuses to guess a
//! layout), WAVE_FORMAT_EXTENSIBLE channel masks (doc 04's "rare
//! channel orders" gap — the demuxer resolves the SubFormat GUID to
//! a width, the mask→layout mapping is future work), and mid-stream
//! format changes (doc 03 §4.5(e) `FormatChanged` is a `[PROPOSAL]`
//! the engine owns).
//!
//! # Output format (codec-native, the MJPEG precedent)
//!
//! The decoder emits [`AudioFrame`]s in the **source's own layout**
//! (interleaved stays interleaved, planar stays planar — layout
//! conversion is the audio-engine's job, doc 03 §4.8) with per-sample
//! byte order **normalized to the host** (media-types' native-endian
//! `SampleFormat` semantics). No depth conversion, no rate
//! conversion, no channel remix: every decoded byte is a permuted
//! input byte, which is what makes the fixture AC a byte-identity
//! hash match on the recorded little-endian corpus.
//!
//! # Module map
//!
//! - [`PcmDecoder`] — the codec-api [`Decoder`](codec_api::Decoder)
//!   implementation (negotiation + damage model documented there).
//! - [`PcmFormat`] / [`PcmSource`] / [`decode_pcm`] — the free
//!   surface: the full byte-level pipeline with no contract state
//!   (the `fuzz_codec_pcm` entry, the `decode_image` pattern of
//!   2.B4).

pub(crate) mod decoder;
pub(crate) mod error;
/// Fuzz harness (doc 04 §8.5 registry name `fuzz_codec_pcm`, table
/// §3.1 row "PCM"): `pub` because the registered target file and the
/// in-tree survival tests call it from outside the crate (the 2.B2/2.B4
/// precedent).
pub mod fuzz;

use codec_api::prelude::*;
use media_types::frame::{AudioFormat, SampleFormat};

pub use decoder::PcmDecoder;

/// One unpacked-PCM sample representation (width + signedness + byte
/// order — the descriptor vocabulary of the module table above).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PcmFormat {
    /// Unsigned 8-bit (byte-order-free).
    U8,
    /// Signed 16-bit, little-endian stream order.
    S16Le,
    /// Signed 16-bit, big-endian stream order.
    S16Be,
    /// Signed 24-bit (3-byte groups), little-endian stream order.
    S24Le,
    /// Signed 24-bit (3-byte groups), big-endian stream order.
    S24Be,
    /// Signed 32-bit, little-endian stream order.
    S32Le,
    /// Signed 32-bit, big-endian stream order.
    S32Be,
    /// IEEE 754 single precision, little-endian stream order.
    F32Le,
    /// IEEE 754 single precision, big-endian stream order.
    F32Be,
    /// IEEE 754 double precision, little-endian stream order.
    F64Le,
    /// IEEE 754 double precision, big-endian stream order.
    F64Be,
}

impl PcmFormat {
    /// The full vocabulary, descriptor order (test and fuzz sweeps
    /// iterate this).
    pub const ALL: [PcmFormat; 11] = [
        PcmFormat::U8,
        PcmFormat::S16Le,
        PcmFormat::S16Be,
        PcmFormat::S24Le,
        PcmFormat::S24Be,
        PcmFormat::S32Le,
        PcmFormat::S32Be,
        PcmFormat::F32Le,
        PcmFormat::F32Be,
        PcmFormat::F64Le,
        PcmFormat::F64Be,
    ];

    /// Bytes per sample (1/2/3/4/8).
    #[must_use]
    pub fn bytes_per_sample(self) -> u32 {
        match self {
            PcmFormat::U8 => 1,
            PcmFormat::S16Le | PcmFormat::S16Be => 2,
            PcmFormat::S24Le | PcmFormat::S24Be => 3,
            PcmFormat::S32Le | PcmFormat::S32Be | PcmFormat::F32Le | PcmFormat::F32Be => 4,
            PcmFormat::F64Le | PcmFormat::F64Be => 8,
        }
    }

    /// The registry descriptor string (doc 04 §3.1 table names; the
    /// 2.B2 demuxer's `StreamFormat::codec_descriptor` emits the same
    /// vocabulary for AVI).
    #[must_use]
    pub fn descriptor(self) -> &'static str {
        match self {
            PcmFormat::U8 => "pcm_u8",
            PcmFormat::S16Le => "pcm_s16le",
            PcmFormat::S16Be => "pcm_s16be",
            PcmFormat::S24Le => "pcm_s24le",
            PcmFormat::S24Be => "pcm_s24be",
            PcmFormat::S32Le => "pcm_s32le",
            PcmFormat::S32Be => "pcm_s32be",
            PcmFormat::F32Le => "pcm_f32le",
            PcmFormat::F32Be => "pcm_f32be",
            PcmFormat::F64Le => "pcm_f64le",
            PcmFormat::F64Be => "pcm_f64be",
        }
    }

    /// Parse a base descriptor (no `_planar` suffix —
    /// [`PcmDecoder`](crate::audio::pcm::PcmDecoder) strips it before
    /// calling). Returns `None` for anything outside the vocabulary.
    #[must_use]
    pub fn from_descriptor(descriptor: &str) -> Option<PcmFormat> {
        PcmFormat::ALL
            .iter()
            .copied()
            .find(|format| format.descriptor() == descriptor)
    }

    /// Whether the stream byte order is little-endian (`None` for the
    /// byte-order-free U8).
    #[must_use]
    pub fn is_little_endian(self) -> Option<bool> {
        match self {
            PcmFormat::U8 => None,
            PcmFormat::S16Le
            | PcmFormat::S24Le
            | PcmFormat::S32Le
            | PcmFormat::F32Le
            | PcmFormat::F64Le => Some(true),
            PcmFormat::S16Be
            | PcmFormat::S24Be
            | PcmFormat::S32Be
            | PcmFormat::F32Be
            | PcmFormat::F64Be => Some(false),
        }
    }

    /// The media-types output sample format (native-endian
    /// normalization happens in the byte engine, not the vocabulary).
    #[must_use]
    pub fn output_sample_format(self) -> SampleFormat {
        match self {
            PcmFormat::U8 => SampleFormat::U8,
            PcmFormat::S16Le | PcmFormat::S16Be => SampleFormat::S16,
            PcmFormat::S24Le | PcmFormat::S24Be => SampleFormat::S24,
            PcmFormat::S32Le | PcmFormat::S32Be => SampleFormat::S32,
            PcmFormat::F32Le | PcmFormat::F32Be => SampleFormat::F32,
            PcmFormat::F64Le | PcmFormat::F64Be => SampleFormat::F64,
        }
    }
}

/// The source layout a PCM decode runs against: sample format,
/// planarity and channel count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcmSource {
    /// Width + signedness + byte order.
    pub sample: PcmFormat,
    /// `true`: one plane per channel, concatenated. `false`:
    /// channels interleaved sample-frame by sample-frame.
    pub planar: bool,
    /// Channel count (≥ 1 — zero is a configuration error, not media
    /// damage).
    pub channels: u32,
}

impl PcmSource {
    /// The descriptor string including the planar suffix, as
    /// [`PcmDecoder`](crate::audio::pcm::PcmDecoder)'s configure
    /// parses it.
    #[must_use]
    pub fn descriptor(&self) -> String {
        if self.planar {
            format!("{}_planar", self.sample.descriptor())
        } else {
            self.sample.descriptor().to_string()
        }
    }
}

/// One decoded PCM buffer (the free-surface result;
/// [`PcmDecoder`](crate::audio::pcm::PcmDecoder) lifts the same bytes
/// into the contract's `Frame::Audio`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPcm {
    /// The declared output shape (native-endian samples, the source's
    /// layout, the `rate` passed to [`decode_pcm`]).
    pub format: AudioFormat,
    /// The decoded bytes: whole sample-frames only, byte order
    /// normalized to the host.
    pub samples: Vec<u8>,
    /// Trailing bytes that belonged to no complete sample-frame and
    /// were dropped (concealment observability — the damage model is
    /// documented on the decoder module).
    pub dropped_tail_bytes: usize,
}

/// Decode one PCM buffer (the free entry point: the whole byte-level
/// pipeline with no contract state — the `decode_image` pattern of
/// card 2.B4, and the first surface of `fuzz_codec_pcm`).
///
/// Damage never fails the call: the whole-sample-frame prefix is the
/// honest output and the dropped tail is reported in
/// [`DecodedPcm::dropped_tail_bytes`].
///
/// # Errors
/// A typed [`ErrorKind::Unsupported`](media_types::ErrorKind::Unsupported)
/// only for a broken *configuration* (zero channels / zero rate) —
/// never for byte-level damage.
pub fn decode_pcm(data: &[u8], source: PcmSource, rate: u32) -> Result<DecodedPcm, EngineError> {
    let plan = decoder::PcmPlan::new(source, rate).map_err(error::PcmError::into_engine_error)?;
    let payload = decoder::decode_payload(data, &plan);
    Ok(DecodedPcm {
        format: plan.output_format(),
        samples: payload.converted.unwrap_or_else(|| data.to_vec()),
        dropped_tail_bytes: payload.dropped_tail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_vocabulary_roundtrips_every_format() {
        for format in PcmFormat::ALL {
            assert_eq!(
                PcmFormat::from_descriptor(format.descriptor()),
                Some(format),
                "{}",
                format.descriptor()
            );
            assert_eq!(format.bytes_per_sample(), {
                match format {
                    PcmFormat::U8 => 1,
                    PcmFormat::S16Le | PcmFormat::S16Be => 2,
                    PcmFormat::S24Le | PcmFormat::S24Be => 3,
                    PcmFormat::F64Le | PcmFormat::F64Be => 8,
                    _ => 4,
                }
            });
        }
        // Out-of-vocabulary descriptors stay out.
        for foreign in [
            "pcm",
            "mjpeg",
            "",
            "s16le",
            "pcm_s20le",
            "pcm_f80le",
            "pcm_u16le",
        ] {
            assert_eq!(PcmFormat::from_descriptor(foreign), None, "{foreign}");
        }
    }

    #[test]
    fn planar_suffix_descriptor_shape() {
        let source = PcmSource {
            sample: PcmFormat::S24Be,
            planar: true,
            channels: 6,
        };
        assert_eq!(source.descriptor(), "pcm_s24be_planar");
        let plain = PcmSource {
            sample: PcmFormat::S16Le,
            planar: false,
            channels: 2,
        };
        assert_eq!(plain.descriptor(), "pcm_s16le");
    }

    #[test]
    fn every_format_decodes_a_clean_buffer_value_exactly() {
        // A buffer holding a distinguishable value in every width is
        // decoded under EVERY format and layout; values are asserted
        // semantically (parsed as native-endian after decode), so the
        // test proves both byte-order directions on any host.
        for format in PcmFormat::ALL {
            for planar in [false, true] {
                for channels in [1u32, 2, 3] {
                    let source = PcmSource {
                        sample: format,
                        planar,
                        channels,
                    };
                    // Two whole sample-frames per channel.
                    let frame = channels as usize * format.bytes_per_sample() as usize;
                    let bytes: Vec<u8> = (0..frame * 2)
                        .map(|i| (0x11 + (i % 13) * 7) as u8)
                        .collect();
                    let decoded = match decode_pcm(&bytes, source, 48_000) {
                        Ok(d) => d,
                        Err(e) => unreachable!("{}: {e}", source.descriptor()),
                    };
                    assert_eq!(decoded.dropped_tail_bytes, 0, "{}", source.descriptor());
                    assert_eq!(decoded.samples.len(), bytes.len());
                    assert_eq!(decoded.format.channels, channels);
                    assert_eq!(decoded.format.rate, 48_000);
                    // The value round-trip: re-encode each output
                    // sample's bytes as the native form of the input
                    // value. For u8/s24 (no CPU-native width) compare
                    // against the byte groups after reordering the
                    // input the same way.
                    let bps = format.bytes_per_sample() as usize;
                    let swapped = format
                        .is_little_endian()
                        .is_some_and(|le| le != cfg!(target_endian = "little"));
                    for (index, group) in decoded.samples.chunks(bps).enumerate() {
                        let expected: Vec<u8> = if swapped {
                            bytes[index * bps..index * bps + bps]
                                .iter()
                                .rev()
                                .copied()
                                .collect()
                        } else {
                            bytes[index * bps..index * bps + bps].to_vec()
                        };
                        assert_eq!(
                            group,
                            expected.as_slice(),
                            "{} sample {index}",
                            source.descriptor()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn decode_pcm_rejects_broken_configurations_typed() {
        let err = match decode_pcm(
            &[0; 8],
            PcmSource {
                sample: PcmFormat::S16Le,
                planar: false,
                channels: 0,
            },
            44_100,
        ) {
            Ok(d) => unreachable!("must fail: {d:?}"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "pcm.channels.zero");
        let err = match decode_pcm(
            &[0; 8],
            PcmSource {
                sample: PcmFormat::S16Le,
                planar: false,
                channels: 2,
            },
            0,
        ) {
            Ok(d) => unreachable!("must fail: {d:?}"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "pcm.rate.zero");
    }

    #[test]
    fn s24_groups_reverse_as_three_byte_units() {
        // 24-bit samples have no CPU-native width; the normalization
        // is a 3-byte group reverse — pinned here byte for byte.
        let source = PcmSource {
            sample: PcmFormat::S24Be,
            planar: false,
            channels: 1,
        };
        let decoded = match decode_pcm(&[0x01, 0x23, 0x45, 0x67, 0x89, 0xab], source, 8_000) {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        let expected: Vec<Vec<u8>> = if cfg!(target_endian = "little") {
            // Host is LE: each BE group reverses.
            vec![vec![0x45, 0x23, 0x01], vec![0xab, 0x89, 0x67]]
        } else {
            vec![vec![0x01, 0x23, 0x45], vec![0x67, 0x89, 0xab]]
        };
        let got: Vec<Vec<u8>> = decoded.samples.chunks(3).map(<[u8]>::to_vec).collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn f32_values_survive_the_byte_order_normalization() {
        let source = PcmSource {
            sample: PcmFormat::F32Le,
            planar: false,
            channels: 2,
        };
        let mut bytes = Vec::new();
        for value in [0.5f32, -0.25] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let decoded = match decode_pcm(&bytes, source, 48_000) {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        let left = f32::from_ne_bytes([
            decoded.samples[0],
            decoded.samples[1],
            decoded.samples[2],
            decoded.samples[3],
        ]);
        let right = f32::from_ne_bytes([
            decoded.samples[4],
            decoded.samples[5],
            decoded.samples[6],
            decoded.samples[7],
        ]);
        assert_eq!(left, 0.5);
        assert_eq!(right, -0.25);
        assert_eq!(decoded.format.sample, SampleFormat::F32);
    }

    #[test]
    fn f64_values_survive_the_byte_order_normalization() {
        // The AVI float-64 path: 8-byte IEEE groups normalized to the
        // host order, values asserted through a native re-read.
        let source = PcmSource {
            sample: PcmFormat::F64Le,
            planar: false,
            channels: 2,
        };
        let mut bytes = Vec::new();
        for value in [0.25f64, -0.125] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let decoded = match decode_pcm(&bytes, source, 48_000) {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        let left = f64::from_ne_bytes(
            decoded.samples[0..8]
                .try_into()
                .unwrap_or_else(|_| unreachable!("8 bytes")),
        );
        let right = f64::from_ne_bytes(
            decoded.samples[8..16]
                .try_into()
                .unwrap_or_else(|_| unreachable!("8 bytes")),
        );
        assert_eq!(left, 0.25);
        assert_eq!(right, -0.125);
        assert_eq!(decoded.format.sample, SampleFormat::F64);
        assert_eq!(decoded.format.frame_bytes(), 16);
    }

    #[test]
    fn u8_is_identity_and_byte_order_free() {
        let source = PcmSource {
            sample: PcmFormat::U8,
            planar: false,
            channels: 2,
        };
        let bytes = [0u8, 128, 255, 42];
        let decoded = match decode_pcm(&bytes, source, 8_000) {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(decoded.samples, bytes);
        assert_eq!(decoded.format.sample, SampleFormat::U8);
        assert_eq!(decoded.format.frame_bytes(), 2);
    }

    #[test]
    fn damaged_tail_is_reported_not_failed() {
        let source = PcmSource {
            sample: PcmFormat::S16Le,
            planar: false,
            channels: 2,
        };
        let decoded = match decode_pcm(&[0x01, 0x02, 0x03, 0x04, 0x05], source, 44_100) {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(decoded.samples.len(), 4, "the whole frame prefix");
        assert_eq!(decoded.dropped_tail_bytes, 1);
    }
}
