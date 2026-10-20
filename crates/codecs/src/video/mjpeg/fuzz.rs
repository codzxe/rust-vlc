//! The MJPEG codec fuzz harness (card 2.B4 `+FB`; doc 04 §8.5
//! registry name `fuzz_codec_mjpeg`, table §4 row "Motion-JPEG").
//!
//! One call = one full untrusted-JPEG pass through **both** decode
//! surfaces: the free [`decode_image`](super::decode_image) entry
//! (marker walk → entropy → IDCT → output assembly) and the
//! codec-api [`Decoder`] contract path (configure → packet →
//! sink → flush). **The harness contract is
//! absence-of-panic/hang/unbounded-allocation, nothing else** —
//! malformed input is *expected* to produce typed errors or a
//! concealed frame, both of which are swallowed here; the AC suite
//! (integration tests) pins the specific outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file lives at
//! `fuzz/fuzz_targets/fuzz_codec_mjpeg.rs` exactly as doc 04 §8.5
//! registers it. Building it needs `libfuzzer-sys` (an external
//! dev-dependency) plus the `fuzz/Cargo.toml` scaffolding — both are
//! Lead-owned dependency decisions (AGENTS.md §3.2, C5), so until
//! they land the harness logic is proven by the in-crate deterministic
//! smoke tests here and by the committed-corpus survival run
//! (`tests/fuzz_survival.rs`); the libFuzzer campaign itself is owed
//! CI wiring (same status as `fuzz_demux_avi`, worklog 2.B2).
//!
//! # Bounds (MP §12)
//!
//! Memory is bounded by the decoder's own ceilings
//! ([`MAX_PIXELS`](super::decoder::MAX_PIXELS) and the per-plane
//! budget) — this harness adds none. Runtime is structurally bounded:
//! one JPEG image per call, every loop in the decode either walks a
//! bounded structure or conceals-and-stops; no budget constant is
//! needed on top (unlike a container walk, there is no
//! attacker-controlled event count here — the MCU grid is derived
//! from the dimension cap).

use codec_api::prelude::*;
use media_types::GenerationId;

use super::MjpegDecoder;

/// A frame sink that only counts (the panic-free emission target of
/// the contract-path pass).
#[derive(Default)]
struct CountingSink {
    frames: usize,
}

impl FrameSink for CountingSink {
    fn accept(&mut self, _frame: Frame) -> Result<(), EngineError> {
        self.frames = self.frames.saturating_add(1);
        Ok(())
    }
}

/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates without a ceiling — by construction
/// of the decoder's bounds-checked, overflow-checked paths.
pub fn fuzz_codec_mjpeg(data: &[u8]) {
    // Pass 1: the free entry (whole pipeline, no contract state).
    let _ = super::decode_image(data);

    // Pass 2: the codec-api contract path. Configure with the current
    // generation, decode the bytes as one packet, flush. Outcomes and
    // errors are equally fine; only survivability is asserted.
    let Ok(descriptor) = media_types::ids::CodecDescriptor::new("mjpeg") else {
        return;
    };
    let Ok(generation) = GenerationId::FIRST.next() else {
        return;
    };
    let mut decoder = MjpegDecoder::new();
    let config = DecoderConfig::new(descriptor, generation, ThreadBudget::default(), None, None);
    if decoder.configure(&config).is_err() {
        return;
    }
    let packet = Packet {
        track: media_types::TrackId::new(0),
        pts: media_types::Stamp::Missing,
        dts: media_types::Stamp::Missing,
        duration: None,
        data: media_types::packet::SharedBytes::from_vec(data.to_vec()),
        keyframe: true,
        discontinuity: false,
        generation,
    };
    let mut sink = CountingSink::default();
    let _ = decoder.decode(packet, &mut sink);
    let _ = decoder.flush(&mut sink);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_survives_both_passes() {
        fuzz_codec_mjpeg(&[]);
    }

    #[test]
    fn soi_only_survives_both_passes() {
        fuzz_codec_mjpeg(&[0xff, 0xd8]);
    }

    #[test]
    fn hostile_marker_soup_survives_both_passes() {
        // Marker bytes with absurd declared lengths.
        fuzz_codec_mjpeg(&[0xff, 0xd8, 0xff, 0xdb, 0xff, 0xff, 0xff, 0x00]);
        // SOI + a shrinking tail of 0xff.
        let mut soup = vec![0xff, 0xd8];
        soup.extend(std::iter::repeat_n(0xff, 4096));
        soup.push(0x00);
        fuzz_codec_mjpeg(&soup);
    }

    #[test]
    fn truncated_valid_stream_survives_both_passes() {
        // A minimal grayscale stream with the entropy data cut short
        // (the concealment path, not the error path).
        let mut jpeg = vec![0xff, 0xd8];
        let seg = |jpeg: &mut Vec<u8>, marker: u8, payload: &[u8]| {
            jpeg.push(0xff);
            jpeg.push(marker);
            jpeg.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
            jpeg.extend_from_slice(payload);
        };
        let mut dqt = vec![0u8];
        dqt.extend(std::iter::repeat_n(16u8, 64));
        seg(&mut jpeg, 0xdb, &dqt);
        for class in [0x00u8, 0x10] {
            let mut dht = vec![class];
            dht.extend(std::iter::repeat_n(0u8, 16));
            dht[1] = 1;
            dht.push(0x00);
            seg(&mut jpeg, 0xc4, &dht);
        }
        let mut sof = vec![8u8];
        sof.extend_from_slice(&[0, 8, 0, 8, 1, 1, 0x11, 0]);
        seg(&mut jpeg, 0xc0, &sof);
        seg(&mut jpeg, 0xda, &[1, 1, 0x00, 0, 63, 0]);
        // Entropy: three bytes then the stream just ends.
        jpeg.extend_from_slice(&[0x7f, 0x30, 0x10]);
        fuzz_codec_mjpeg(&jpeg);
    }
}
