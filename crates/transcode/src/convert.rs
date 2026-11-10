//! Decoded-frame → WAV-payload sample conversion (card 7.E1).
//!
//! # The honest conversion matrix (v1)
//!
//! WAV is little-endian interleaved PCM; decoded frames are
//! native-endian [`SampleFormat`] in the decoder's own layout. Two
//! conversions are always legal and lossless:
//!
//! * **byte-order normalization** — native → little-endian (a no-op
//!   on little-endian hosts, a byte swap per sample otherwise);
//! * **layout normalization** — planar → interleaved (a permutation
//!   of the same samples, one frame at a time).
//!
//! Plus the **lossless widenings only**: u8→s16/s24/s32, s16→s24/s32,
//! s24→s32 (value-preserving left shifts) and f32→f64 (exact). Every
//! other pair — narrowing, int↔float, signedness flips — is a typed
//! [`wav_conversion_unsupported`](crate::error::wav_conversion_unsupported)
//! refusal: a transcode engine must never quantize silently.
//!
//! # Allocation policy (D-1)
//!
//! The identity pair (same sample kind, interleaved source,
//! little-endian host) is a **zero-copy passthrough** — the frame's
//! [`SharedBytes`] go to the sink untouched. Every other pair
//! converts into a caller-owned scratch [`Vec`] that is cleared and
//! reused per frame (zero allocations after the first, bounded by
//! one frame's converted size).

use demux::wav::format::PcmSample;
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::EngineError;

use crate::error::wav_conversion_unsupported;
use crate::graph::WavPcmTarget;

/// Whether this host is little-endian (a compile-time constant —
/// the byte-order normalization is chosen per target, never guessed).
const HOST_IS_LITTLE: bool = cfg!(target_endian = "little");

/// The WAV-vocabulary descriptor of a decoded sample format (the
/// error/report vocabulary: media-types' formats are native-endian,
/// the WAV container's are little-endian; on a little-endian host
/// the two are the same bytes).
#[must_use]
pub(crate) fn source_descriptor(source: SampleFormat) -> &'static str {
    match source {
        SampleFormat::U8 => "pcm_u8",
        SampleFormat::S16 => "pcm_s16le",
        SampleFormat::S24 => "pcm_s24le",
        SampleFormat::S32 => "pcm_s32le",
        SampleFormat::F32 => "pcm_f32le",
        SampleFormat::F64 => "pcm_f64le",
    }
}

/// The little-endian WAV sample a decoded format maps to identically.
#[must_use]
pub(crate) fn identity_target(source: SampleFormat) -> PcmSample {
    match source {
        SampleFormat::U8 => PcmSample::U8,
        SampleFormat::S16 => PcmSample::S16Le,
        SampleFormat::S24 => PcmSample::S24Le,
        SampleFormat::S32 => PcmSample::S32Le,
        SampleFormat::F32 => PcmSample::F32Le,
        SampleFormat::F64 => PcmSample::F64Le,
    }
}

/// Resolve the WAV sample kind a frame of `source` must be written
/// as, under the job's target selection.
///
/// # Errors
/// [`wav_conversion_unsupported`] for every pair that is not the
/// identity or a lossless widening (module docs matrix).
pub(crate) fn resolve_target(
    source: SampleFormat,
    target: &WavPcmTarget,
) -> Result<PcmSample, EngineError> {
    let wanted = match target {
        WavPcmTarget::Source => return Ok(identity_target(source)),
        WavPcmTarget::Sample(wanted) => *wanted,
    };
    if conversion_legal(source, wanted) {
        Ok(wanted)
    } else {
        Err(wav_conversion_unsupported(
            source_descriptor(source),
            wanted.descriptor(),
        ))
    }
}

/// Whether `source` → `target` is the identity or a lossless
/// widening (module docs matrix).
#[must_use]
pub(crate) fn conversion_legal(source: SampleFormat, target: PcmSample) -> bool {
    matches!(
        (source, target),
        // Identity (byte-order + layout normalization only).
        (SampleFormat::U8, PcmSample::U8)
            | (SampleFormat::S16, PcmSample::S16Le)
            | (SampleFormat::S24, PcmSample::S24Le)
            | (SampleFormat::S32, PcmSample::S32Le)
            | (SampleFormat::F32, PcmSample::F32Le)
            | (SampleFormat::F64, PcmSample::F64Le)
            // Lossless integer widenings (value-preserving shifts).
            | (SampleFormat::U8, PcmSample::S16Le)
            | (SampleFormat::U8, PcmSample::S24Le)
            | (SampleFormat::U8, PcmSample::S32Le)
            | (SampleFormat::S16, PcmSample::S24Le)
            | (SampleFormat::S16, PcmSample::S32Le)
            | (SampleFormat::S24, PcmSample::S32Le)
            // Exact float widening.
            | (SampleFormat::F32, PcmSample::F64Le)
    )
}

/// Whether the frame's bytes can go to the sink untouched: same
/// sample kind, interleaved layout, little-endian host (the
/// zero-copy path — module docs allocation policy).
#[must_use]
pub(crate) fn identity_passthrough(format: &AudioFormat, target: PcmSample) -> bool {
    HOST_IS_LITTLE && format.layout == AudioLayout::Interleaved && identity_target(format.sample) == target
}

/// Read one native-endian sample from `window` (`len` = the source
/// width). Sign/float interpretation per [`SampleFormat`].
fn read_native(window: &[u8], sample: SampleFormat) -> i64 {
    match sample {
        SampleFormat::U8 => i64::from(window[0]),
        SampleFormat::S16 => i64::from(i16::from_ne_bytes([window[0], window[1]])),
        SampleFormat::S24 => {
            let (lo, mid, hi) = if HOST_IS_LITTLE {
                (window[0], window[1], window[2])
            } else {
                (window[2], window[1], window[0])
            };
            let value = u32::from(lo) | (u32::from(mid) << 8) | (u32::from(hi) << 16);
            // Sign-extend from bit 23: park the 24 payload bits at
            // the top of i32, then arithmetic-shift back down (a
            // plain `value as i32` would leave the high byte zero —
            // unsigned, never sign-extended).
            i64::from(((value << 8) as i32) >> 8)
        }
        SampleFormat::S32 => i64::from(i32::from_ne_bytes([
            window[0], window[1], window[2], window[3],
        ])),
        // Float samples are read only for the f32→f64 widening; the
        // integer return is unused there (see `emit`).
        SampleFormat::F32 | SampleFormat::F64 => 0,
    }
}

/// Read one native-endian f32 sample.
fn read_f32(window: &[u8]) -> f32 {
    f32::from_ne_bytes([window[0], window[1], window[2], window[3]])
}

/// Append one converted sample to `out` in the target's
/// little-endian representation. The pair was legality-checked by
/// the caller (module docs matrix).
fn emit(window: &[u8], source: SampleFormat, target: PcmSample, out: &mut Vec<u8>) {
    match (source, target) {
        (SampleFormat::U8, PcmSample::U8) => out.push(window[0]),
        (SampleFormat::S16, PcmSample::S16Le) => {
            out.extend_from_slice(&i16::from_ne_bytes([window[0], window[1]]).to_le_bytes());
        }
        (SampleFormat::S24, PcmSample::S24Le) => {
            let value = read_native(window, SampleFormat::S24) as i32;
            out.extend_from_slice(&value.to_le_bytes()[..3]);
        }
        (SampleFormat::S32, PcmSample::S32Le) => {
            out.extend_from_slice(
                &i32::from_ne_bytes([window[0], window[1], window[2], window[3]]).to_le_bytes(),
            );
        }
        (SampleFormat::F32, PcmSample::F32Le) => {
            out.extend_from_slice(&read_f32(window).to_le_bytes());
        }
        (SampleFormat::F64, PcmSample::F64Le) => {
            out.extend_from_slice(
                &f64::from_ne_bytes([
                    window[0], window[1], window[2], window[3], window[4], window[5], window[6],
                    window[7],
                ])
                .to_le_bytes(),
            );
        }
        (SampleFormat::U8, PcmSample::S16Le) => {
            let widened = (i64::from(window[0]) - 128) << 8;
            out.extend_from_slice(&(widened as i16).to_le_bytes());
        }
        (SampleFormat::U8, PcmSample::S24Le) => {
            let widened = (i64::from(window[0]) - 128) << 16;
            out.extend_from_slice(&(widened as i32).to_le_bytes()[..3]);
        }
        (SampleFormat::U8, PcmSample::S32Le) => {
            let widened = (i64::from(window[0]) - 128) << 24;
            out.extend_from_slice(&(widened as i32).to_le_bytes());
        }
        (SampleFormat::S16, PcmSample::S24Le) => {
            let widened = read_native(window, SampleFormat::S16) << 8;
            out.extend_from_slice(&(widened as i32).to_le_bytes()[..3]);
        }
        (SampleFormat::S16, PcmSample::S32Le) => {
            let widened = read_native(window, SampleFormat::S16) << 16;
            out.extend_from_slice(&(widened as i32).to_le_bytes());
        }
        (SampleFormat::S24, PcmSample::S32Le) => {
            let widened = read_native(window, SampleFormat::S24) << 8;
            out.extend_from_slice(&(widened as i32).to_le_bytes());
        }
        (SampleFormat::F32, PcmSample::F64Le) => {
            out.extend_from_slice(&f64::from(read_f32(window)).to_le_bytes());
        }
        // Every other pair was refused by `conversion_legal` before
        // any byte moved; reaching here is a logic bug in this
        // module, and the honest answer is to skip nothing silently —
        // the writer's whole-frame check catches any length drift.
        _ => {}
    }
}

/// Convert one decoded frame's payload into `scratch` (cleared and
/// reused), little-endian interleaved in `target`'s representation.
///
/// The caller has already verified the payload is a whole number of
/// sample frames and the pair is legal
/// ([`resolve_target`]/[`conversion_legal`]).
pub(crate) fn convert_to_wav(
    samples: &[u8],
    format: &AudioFormat,
    target: PcmSample,
    scratch: &mut Vec<u8>,
) {
    let channels = format.channels as usize;
    let width = format.sample.bytes_per_sample() as usize;
    let frames = samples.len() / (channels * width);
    scratch.clear();
    let wanted = frames * channels * target.bytes_per_sample() as usize;
    if scratch.capacity() < wanted {
        scratch.reserve(wanted - scratch.len());
    }
    for frame in 0..frames {
        for channel in 0..channels {
            let at = match format.layout {
                AudioLayout::Interleaved => (frame * channels + channel) * width,
                AudioLayout::Planar => (channel * frames + frame) * width,
            };
            emit(&samples[at..at + width], format.sample, target, scratch);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
    use media_types::packet::SharedBytes;
    use testkit::{Must, assert_err};

    fn format(sample: SampleFormat, channels: u32, layout: AudioLayout) -> AudioFormat {
        AudioFormat {
            sample,
            channels,
            layout,
            rate: 48_000,
        }
    }

    #[test]
    fn target_resolution_matrix() {
        // Source selection is always the identity mapping.
        for (source, target) in [
            (SampleFormat::U8, PcmSample::U8),
            (SampleFormat::S16, PcmSample::S16Le),
            (SampleFormat::S24, PcmSample::S24Le),
            (SampleFormat::S32, PcmSample::S32Le),
            (SampleFormat::F32, PcmSample::F32Le),
            (SampleFormat::F64, PcmSample::F64Le),
        ] {
            let resolved = resolve_target(source, &WavPcmTarget::Source).must();
            assert_eq!(resolved, target);
        }
        // Every legal widening resolves.
        for (source, target) in [
            (SampleFormat::S16, PcmSample::S32Le),
            (SampleFormat::S24, PcmSample::S32Le),
            (SampleFormat::F32, PcmSample::F64Le),
            (SampleFormat::U8, PcmSample::S16Le),
        ] {
            let resolved = resolve_target(source, &WavPcmTarget::Sample(target)).must();
            assert_eq!(resolved, target);
        }
        // Narrowing and int↔float are refused typed.
        for (source, target) in [
            (SampleFormat::S32, PcmSample::S16Le),
            (SampleFormat::F64, PcmSample::F32Le),
            (SampleFormat::S16, PcmSample::F32Le),
            (SampleFormat::F32, PcmSample::S32Le),
        ] {
            let err = assert_err(
                resolve_target(source, &WavPcmTarget::Sample(target)),
                "lossy pair",
            );
            assert_eq!(err.context.code, "transcode.wav.sample-conversion");
        }
    }

    #[test]
    fn interleaved_identity_is_passthrough_on_little_endian_hosts() {
        let fmt = format(SampleFormat::S16, 2, AudioLayout::Interleaved);
        assert_eq!(
            identity_passthrough(&fmt, PcmSample::S16Le),
            HOST_IS_LITTLE
        );
        // Planar always converts (permutation), other targets always
        // convert.
        let planar = format(SampleFormat::S16, 2, AudioLayout::Planar);
        assert!(!identity_passthrough(&planar, PcmSample::S16Le));
        assert!(!identity_passthrough(&fmt, PcmSample::S32Le));
    }

    #[test]
    fn planar_to_interleaved_permutes_frame_major() {
        // Two channels, two frames of s16: planes [L0 L1] [R0 R1]
        // must come out interleaved [L0 R0 L1 R1].
        let fmt = format(SampleFormat::S16, 2, AudioLayout::Planar);
        let left = [0x0100i16, 0x0300];
        let right = [0x0200i16, 0x0400];
        let mut bytes = Vec::new();
        for v in left {
            bytes.extend_from_slice(&v.to_ne_bytes());
        }
        for v in right {
            bytes.extend_from_slice(&v.to_ne_bytes());
        }
        let mut scratch = Vec::new();
        convert_to_wav(&bytes, &fmt, PcmSample::S16Le, &mut scratch);
        let mut got = [0i16; 4];
        for (i, slot) in got.iter_mut().enumerate() {
            *slot = i16::from_le_bytes([scratch[2 * i], scratch[2 * i + 1]]);
        }
        assert_eq!(got, [0x0100, 0x0200, 0x0300, 0x0400]);
    }

    #[test]
    fn widening_conversions_are_exact() {
        let mut scratch = Vec::new();
        // s16 → s32: value-preserving << 16.
        let fmt = format(SampleFormat::S16, 1, AudioLayout::Interleaved);
        let samples: Vec<i16> = [-32768, -1, 0, 1, 32767].to_vec();
        let mut bytes = Vec::new();
        for v in &samples {
            bytes.extend_from_slice(&v.to_ne_bytes());
        }
        convert_to_wav(&bytes, &fmt, PcmSample::S32Le, &mut scratch);
        for (i, want) in samples.iter().enumerate() {
            let raw = [
                scratch[4 * i],
                scratch[4 * i + 1],
                scratch[4 * i + 2],
                scratch[4 * i + 3],
            ];
            assert_eq!(
                i32::from_le_bytes(raw),
                i32::from(*want) << 16,
                "s16→s32 sample {i}"
            );
        }
        // u8 → s16: (u − 128) << 8, silent-at-128 confirmed.
        let fmt_u8 = format(SampleFormat::U8, 1, AudioLayout::Interleaved);
        convert_to_wav(&[0, 128, 255], &fmt_u8, PcmSample::S16Le, &mut scratch);
        let got: Vec<i16> = scratch
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();
        assert_eq!(got, [-32768, 0, 32512]);
        // s24 → s32: sign-extended << 8 (a −1 s24 sample is
        // 0xFF FF FF little-endian).
        let fmt_s24 = format(SampleFormat::S24, 1, AudioLayout::Interleaved);
        convert_to_wav(&[0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00], &fmt_s24, PcmSample::S32Le, &mut scratch);
        let got: Vec<i32> = scratch
            .chunks_exact(4)
            .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert_eq!(got, [-256, 0]);
        // f32 → f64: exact widening.
        let fmt_f32 = format(SampleFormat::F32, 1, AudioLayout::Interleaved);
        let f = -0.5f32;
        convert_to_wav(&f.to_ne_bytes(), &fmt_f32, PcmSample::F64Le, &mut scratch);
        let raw: [u8; 8] = scratch[..8].try_into().unwrap_or([0; 8]);
        assert_eq!(f64::from_le_bytes(raw), f64::from(f));
    }

    #[test]
    fn native_identity_roundtrips_bytes() {
        // On a little-endian host the identity conversion is the
        // byte-for-byte copy; on big-endian it is the swap. Both are
        // pinned by re-reading with the matching endianness.
        let fmt = format(SampleFormat::S32, 2, AudioLayout::Interleaved);
        let samples: Vec<i32> = [i32::MIN, -1, 0, i32::MAX].to_vec();
        let mut bytes = Vec::new();
        for v in &samples {
            bytes.extend_from_slice(&v.to_ne_bytes());
        }
        let mut scratch = Vec::new();
        convert_to_wav(&bytes, &fmt, PcmSample::S32Le, &mut scratch);
        let got: Vec<i32> = scratch
            .chunks_exact(4)
            .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert_eq!(got, samples);
        assert_eq!(scratch.len(), bytes.len());
    }

    #[test]
    fn scratch_is_reused_not_accumulated() {
        let fmt = format(SampleFormat::S16, 2, AudioLayout::Planar);
        let bytes = vec![0u8; 8 * 4];
        let mut scratch = Vec::new();
        convert_to_wav(&bytes, &fmt, PcmSample::S16Le, &mut scratch);
        let capacity = scratch.capacity();
        for _ in 0..4 {
            convert_to_wav(&bytes, &fmt, PcmSample::S16Le, &mut scratch);
            assert_eq!(scratch.len(), bytes.len());
            assert_eq!(scratch.capacity(), capacity, "conversion scratch is reused");
        }
    }

    #[test]
    fn shared_bytes_deref_gives_the_payload_for_passthrough() {
        // The zero-copy path writes the frame's own bytes; prove the
        // view semantics the writer relies on (Deref = payload).
        let payload = SharedBytes::from_vec(vec![9u8; 6]);
        assert_eq!(payload.as_slice(), &[9u8; 6]);
        assert_eq!(payload.len(), 6);
    }
}
