//! The PCM codec fuzz harness (card 2.B5 `+FB`; doc 04 §8.5 registry
//! name `fuzz_codec_pcm`, table §3.1 row "PCM").
//!
//! One call = one untrusted byte buffer through **both** decode
//! surfaces under a deterministic spread of the configuration space:
//! the free [`decode_pcm`](super::decode_pcm) entry (layout analysis
//! + byte-order normalization) and the codec-api [`Decoder`]
//! contract path (configure → packet → sink → flush). **The harness
//! contract is absence-of-panic/hang/unbounded-allocation, nothing
//! else** — damage is *expected* to surface as a concealed prefix or
//! a skipped packet, both swallowed here; the AC suites (integration
//! tests) pin the specific outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file lives at
//! `fuzz/fuzz_targets/fuzz_codec_pcm.rs` exactly as doc 04 §8.5
//! registers it. Building it needs the `libfuzzer-sys`
//! dev-dependency plus the `fuzz/Cargo.toml` scaffolding — both
//! Lead-owned dependency decisions (AGENTS.md §3.2, C5), so until
//! they land the harness logic is proven by the in-crate
//! deterministic smoke tests here and by the committed-corpus +
//! seeded-mutation survival run
//! (`crates/codecs/tests/pcm_fuzz_survival.rs`); the libFuzzer
//! campaign itself is owed CI wiring (same status as
//! `fuzz_demux_avi` / `fuzz_codec_mjpeg`, worklogs 2.B2/2.B4).
//!
//! # Bounds (MP §12)
//!
//! Memory is bounded by the input itself: every PCM conversion is a
//! width- and order-preserving permutation of the packet's own bytes
//! (output ≤ input; see the decoder module docs) — this harness adds
//! no buffers. Runtime is structurally bounded: one pass over the
//! bytes per configuration, a fixed number of configurations per
//! call; there is no attacker-controlled event count (no container
//! walk here).

use codec_api::prelude::*;
use media_types::GenerationId;

use super::{PcmDecoder, PcmFormat, PcmSource};

/// Rates the harness cycles through (all legal; one of them is the
/// fixture's 44100).
const RATES: [u32; 3] = [8_000, 44_100, 48_000];

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

/// The configurations one fuzz pass exercises: the fixture format
/// (always), a data-derived format, and two fixed stress shapes (an
/// odd-width planar surround case and a big-endian float case).
fn configurations(data: &[u8]) -> Vec<PcmSource> {
    let mut configs = vec![
        // The first vertical's format, unconditionally.
        PcmSource {
            sample: PcmFormat::S16Le,
            planar: false,
            channels: 2,
        },
    ];
    let a = u32::from(data.first().copied().unwrap_or(0));
    let b = data.get(1).copied().unwrap_or(0);
    let derived = PcmSource {
        sample: PcmFormat::ALL[(a % PcmFormat::ALL.len() as u32) as usize],
        planar: b & 0x01 == 1,
        channels: 1 + u32::from(b >> 1) % 8,
    };
    if derived != configs[0] {
        configs.push(derived);
    }
    configs.push(PcmSource {
        sample: PcmFormat::S24Be,
        planar: true,
        channels: 3,
    });
    configs.push(PcmSource {
        sample: PcmFormat::F32Be,
        planar: false,
        channels: 1,
    });
    configs
}

/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates beyond the input — by construction of
/// the decoder's bounded, permutation-only paths.
pub fn fuzz_codec_pcm(data: &[u8]) {
    let rate = RATES[usize::from(data.first().copied().unwrap_or(0)) % RATES.len()];

    // Pass 1: the free entry across the configuration spread.
    for source in configurations(data) {
        let _ = super::decode_pcm(data, source, rate);
    }

    // Pass 2: the codec-api contract path, one full configure →
    // decode → flush cycle per configuration. Outcomes and errors are
    // equally fine; only survivability is asserted.
    for source in configurations(data) {
        let Ok(descriptor) = media_types::ids::CodecDescriptor::new(&source.descriptor()) else {
            continue;
        };
        let Ok(generation) = GenerationId::FIRST.next() else {
            return;
        };
        let Ok(mut decoder) = PcmDecoder::with_format(source.channels, rate) else {
            continue;
        };
        let config =
            DecoderConfig::new(descriptor, generation, ThreadBudget::default(), None, None);
        if decoder.configure(&config).is_err() {
            continue;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_survives_both_passes() {
        fuzz_codec_pcm(&[]);
    }

    #[test]
    fn sub_frame_lengths_survive_both_passes() {
        for length in 1..=9 {
            fuzz_codec_pcm(&vec![0xa5; length]);
        }
    }

    #[test]
    fn fixture_shaped_buffer_survives_both_passes() {
        // One AVI audio chunk's worth (5880 bytes of s16le stereo),
        // plus a damaged variant with a dangling tail.
        let clean: Vec<u8> = (0..5880).map(|i| (i % 251) as u8).collect();
        fuzz_codec_pcm(&clean);
        let mut damaged = clean;
        damaged.truncate(5_879);
        damaged.push(0xff);
        fuzz_codec_pcm(&damaged);
    }

    #[test]
    fn hostile_wide_input_survives_both_passes() {
        // All-0xff and all-0x00 buffers: the two byte patterns that
        // maximize sign bits / zero-length degeneracy.
        fuzz_codec_pcm(&vec![0xff; 4096]);
        fuzz_codec_pcm(&vec![0x00; 4096]);
    }

    #[test]
    fn configurations_spread_is_deterministic_and_bounded() {
        for data in [&[][..], &[0u8][..], &[1u8, 7][..], &[9u8, 2, 3][..]] {
            let configs = configurations(data);
            assert!(
                configs.len() >= 3 && configs.len() <= 4,
                "{:?}",
                configs.len()
            );
            assert_eq!(configs[0].sample, PcmFormat::S16Le, "fixture format first");
            // Determinism: same input, same spread.
            assert_eq!(configs, configurations(data));
        }
    }
}
