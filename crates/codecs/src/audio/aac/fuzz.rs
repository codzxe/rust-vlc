//! The AAC codec fuzz harness (card 5.C2a `+FB`; doc 04 §3.1
//! registry name `fuzz_codec_aac`).
//!
//! One call = one untrusted byte buffer through the decode surfaces:
//!
//! 1. the ADTS fixed-header parse ([`super::adts::parse_adts_header`])
//!    — sync, layer, profile/index/config tables, framing arithmetic;
//! 2. the resync walker ([`super::adts::walk_adts_frames`]);
//! 3. the full stream pipeline ([`super::decode_stream`]) — engine
//!    decode included;
//! 4. the codec-api contract path (configure → ADTS packets → sink →
//!    flush) over the walked frames.
//!
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — damage is expected to surface as a
//! typed error and is swallowed here; the AC suites (integration
//! tests) pin the specific outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_aac.rs` exactly as doc 04 §3.1
//! registers it. Building it needs the `libfuzzer-sys` dev-dependency
//! plus the `fuzz/Cargo.toml` scaffolding — both Lead-owned
//! dependency decisions (AGENTS.md §3.2, C5) — so until they land the
//! harness logic is proven by the in-crate deterministic smoke tests
//! here and by the committed-fixture + seeded-mutation survival run
//! (`crates/codecs/tests/aac_fuzz_survival.rs`); the libFuzzer
//! campaign itself is owed CI wiring (the same status as
//! `fuzz_codec_flac` / `fuzz_codec_mp3` / `fuzz_codec_mjpeg` /
//! `fuzz_codec_pcm`).
//!
//! # Bounds (MP §12)
//!
//! Memory is bounded by the input: `decode_stream`'s output is at
//! most `walked_frames × 1024 × 2 channels × 4 bytes` with the frame
//! count bounded by `input_len / 7` (minimum frame = header), a fixed
//! number of passes per call. The contract path pools 4 output slots
//! of `1024 × 2 × 4` bytes. There is no attacker-controlled event
//! count.

use codec_api::prelude::*;
use media_types::GenerationId;

use super::adts::{parse_adts_header, walk_adts_frames};
use super::decoder::{AacDecoder, DecodedAac, decode_stream};

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

/// One fuzz pass's outcome digest (observable, assertable absence).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuzzOutcome {
    /// Frames the walk found.
    pub frames: usize,
    /// Whether the full decode produced samples.
    pub decoded: bool,
    /// Frames the contract path emitted.
    pub contract_frames: usize,
}

/// Run the harness (see the module docs). Never panics on any input;
/// every damage path answers a typed error that is swallowed here.
#[must_use]
pub fn fuzz_aac_stream(data: &[u8]) -> FuzzOutcome {
    let mut outcome = FuzzOutcome {
        frames: 0,
        decoded: false,
        contract_frames: 0,
    };
    // 1. Single-header parse under raw bytes (boundary input).
    let _ = parse_adts_header(data);
    // 2. Header walk (resync included).
    let frames = walk_adts_frames(data, 0);
    outcome.frames = frames.len();
    // 3. Full stream pipeline.
    if let Ok(DecodedAac { samples, .. }) = decode_stream(data) {
        outcome.decoded = !samples.is_empty();
    }
    // 4. Contract path over the walked frames.
    if let Some(first) = frames.first() {
        let mut decoder = AacDecoder::new();
        let generation = match GenerationId::FIRST.next() {
            Ok(g) => g,
            Err(_) => return outcome,
        };
        let configured = decoder.configure(&DecoderConfig::new(
            match media_types::ids::CodecDescriptor::new("aac") {
                Ok(d) => d,
                Err(_) => return outcome,
            },
            generation,
            ThreadBudget::default(),
            None,
            None,
        ));
        if configured.is_ok() {
            let mut sink = CountingSink::default();
            for frame in frames.iter().take(64) {
                let bytes = data
                    .get(frame.offset..frame.offset + frame.header.frame_len)
                    .unwrap_or(&[]);
                let packet = Packet {
                    track: media_types::TrackId::new(0),
                    pts: media_types::Stamp::Missing,
                    dts: media_types::Stamp::Missing,
                    duration: None,
                    data: media_types::packet::SharedBytes::from_vec(bytes.to_vec()),
                    keyframe: true,
                    discontinuity: false,
                    generation,
                };
                if decoder.decode(packet, &mut sink).is_err() {
                    break;
                }
            }
            let _ = decoder.flush(&mut sink);
            outcome.contract_frames = sink.frames;
        }
        let _ = first;
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_garbage_inputs_never_panic() {
        assert_eq!(
            fuzz_aac_stream(&[]),
            FuzzOutcome {
                frames: 0,
                decoded: false,
                contract_frames: 0
            }
        );
        let garbage = [0xFFu8; 4096];
        let o = fuzz_aac_stream(&garbage);
        // Repeated 0xFF syncs fail validation (layer/index) so the
        // walk stays at zero frames — no hang, no panic.
        assert_eq!(o.frames, 0);
    }

    #[test]
    fn a_header_only_buffer_is_survived() {
        // Valid 7-byte header, no frame body.
        let o = fuzz_aac_stream(&[0xFF, 0xF1, 0x50, 0x80, 0x08, 0x1F, 0xFC]);
        assert_eq!(o.frames, 0);
    }
}
