//! The MP2 codec fuzz harness (card 5.J3 `+FB`; doc 04 §8.5 registry
//! name `fuzz_codec_mp2`): one call = one untrusted byte buffer
//! through every decode surface this module owns.
//!
//! 1. the Layer II header parse + walk ([`super::header`]) — sync
//!    recovery, the layer gate, the CBR geometry gate;
//! 2. the CBR seek planners (frame arithmetic both directions);
//! 3. the full stream pipeline ([`super::decode_stream`]) — engine
//!    decode included;
//! 4. the codec-api contract path (configure → packets → sink →
//!    flush) over the walked frames.
//!
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — damage is expected to surface as a
//! typed error and is swallowed here; the AC suites pin the specific
//! outcomes (`tests/mp2_fuzz_survival.rs`).
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_mp2.rs` exactly as doc 04 §8.5
//! registers it; building it needs the `libfuzzer-sys`
//! dev-dependency plus the `fuzz/Cargo.toml` scaffolding — both
//! Lead-owned dependency decisions (AGENTS.md §3.2, C5) — so until
//! they land the harness logic is proven by the in-crate
//! deterministic smoke tests here and by the committed-fixture +
//! seeded-mutation survival run. (The same status as
//! `fuzz_codec_mp3` and its siblings.)
//!
//! # Bounds (MP §12)
//!
//! Memory is bounded by the input: `decode_stream`'s output is at
//! most `walked_frames × 1152 × 2 channels × 4 bytes` with the frame
//! count bounded by `input_len / frame_size` (frame_size ≥ 17 for
//! any Layer II table bitrate at the LSF floor — the 8 kbps/48 kHz
//! corner), a fixed number of passes per call. The contract path
//! pools 4 output slots of `1152 × channels × 4` bytes. There is no
//! attacker-controlled event count.

use codec_api::prelude::*;
use media_types::GenerationId;

use super::decoder::{DecodedMp2, Mp2Decoder, decode_stream};
use super::header::{geometry, parse_mp2_header, walk_mp2_frames};
use super::seek::{cbr_seek, duration_nanos, sample_at_byte};

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
    /// Frames the Layer II walk produced.
    pub frames: usize,
    /// Whether the walk + geometry both succeeded (a CBR-verified
    /// Layer II stream shape).
    pub cbr_geometry: bool,
    /// Whether the full decode produced samples.
    pub decoded: bool,
    /// Frames the contract path emitted.
    pub contract_frames: usize,
}

/// Run the harness (see the module docs). Never panics on any input;
/// every damage path answers a typed error that is swallowed here.
#[must_use]
pub fn fuzz_mp2_stream(data: &[u8]) -> FuzzOutcome {
    let mut outcome = FuzzOutcome {
        frames: 0,
        cbr_geometry: false,
        decoded: false,
        contract_frames: 0,
    };
    // 1. Layer II walk (the layer gate + family resync inside).
    let frames = match walk_mp2_frames(data, 0) {
        Ok(f) => f,
        Err(_) => return outcome,
    };
    outcome.frames = frames.len();
    if frames.is_empty() {
        return outcome;
    }
    // 2. Geometry + both-direction seek arithmetic on it.
    let geo = match geometry(&frames) {
        Ok(Some(g)) => g,
        _ => return outcome,
    };
    outcome.cbr_geometry = true;
    let plan = cbr_seek(&geo, geo.total_samples / 2);
    let _ = sample_at_byte(&geo, plan.byte_offset);
    let _ = duration_nanos(&geo);
    // Single-header parse under raw bytes (boundary input).
    if let Some(first) = frames.first() {
        let window = data.get(first.offset..first.offset + 4).unwrap_or(&[]);
        let _ = parse_mp2_header(window);
    }
    // 3. Full stream pipeline.
    if let Ok(DecodedMp2 { samples, .. }) = decode_stream(data) {
        outcome.decoded = !samples.is_empty();
    }
    // 4. Contract path over the walked frames.
    let mut decoder = Mp2Decoder::new();
    let generation = match GenerationId::FIRST.next() {
        Ok(g) => g,
        Err(_) => return outcome,
    };
    let configured = decoder.configure(&DecoderConfig::new(
        match media_types::ids::CodecDescriptor::new("mp2") {
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
                .get(frame.offset..frame.offset + frame.header.family.frame_size as usize)
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
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_garbage_inputs_never_panic() {
        assert_eq!(
            fuzz_mp2_stream(&[]),
            FuzzOutcome {
                frames: 0,
                cbr_geometry: false,
                decoded: false,
                contract_frames: 0
            }
        );
        let garbage = [0xFFu8; 4096];
        let o = fuzz_mp2_stream(&garbage);
        // Repeated 0xFF syncs fail family validation (bad bitrate
        // etc.) so the walk stays at zero frames — no hang, no panic.
        assert_eq!(o.frames, 0);
    }

    #[test]
    fn a_header_only_buffer_is_survived() {
        // Valid Layer II header, no frame body.
        let o = fuzz_mp2_stream(&[0xFF, 0xFD, 0xE0, 0x00]);
        assert_eq!(o.frames, 0);
    }

    #[test]
    fn a_layer_iii_frame_is_refused_not_decoded() {
        let o = fuzz_mp2_stream(&[0xFF, 0xFB, 0x90, 0x00]);
        assert_eq!(o.frames, 0);
        assert!(!o.cbr_geometry);
        assert!(!o.decoded);
    }
}
