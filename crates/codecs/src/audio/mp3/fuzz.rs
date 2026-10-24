//! The MP3/MP2 codec fuzz harness (card 5.A3 `+FB`; doc 04 §8.5
//! registry name `fuzz_codec_mp3`, table §3.1 row "MP3 raw stream").
//!
//! One call = one untrusted byte buffer through the decode surfaces:
//!
//! 1. the ID3 walk ([`super::id3::parse_id3v2`] via the length
//!    probe) — the dirty-world parser under the dirtiest input;
//! 2. the frame header parse + walk ([`super::header`]) — sync
//!    recovery, tables, framing arithmetic;
//! 3. the Xing/Info + VBRI parsers over the first walked frame;
//! 4. the full stream pipeline ([`super::decode_stream`]) — engine
//!    decode included;
//! 5. the codec-api contract path (configure → packets → sink →
//!    flush) over the walked frames;
//! 6. the seek planners (CBR arithmetic + VBR TOC interpolation).
//!
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — damage is expected to surface as a
//! typed error and is swallowed here; the AC suites (integration
//! tests) pin the specific outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_mp3.rs` exactly as doc 04 §8.5
//! registers it. Building it needs the `libfuzzer-sys` dev-dependency
//! plus the `fuzz/Cargo.toml` scaffolding — both Lead-owned
//! dependency decisions (AGENTS.md §3.2, C5) — so until they land
//! the harness logic is proven by the in-crate deterministic smoke
//! tests here and by the committed-fixture + seeded-mutation
//! survival run (`crates/codecs/tests/mp3_fuzz_survival.rs`); the
//! libFuzzer campaign itself is owed CI wiring (the same status as
//! `fuzz_codec_flac` / `fuzz_codec_mjpeg` / `fuzz_codec_pcm`).
//!
//! # Bounds (MP §12)
//!
//! Memory is bounded by the input: `decode_stream`'s output is at
//! most `walked_frames × 1152 × 2 channels × 4 bytes` with the frame
//! count bounded by `input_len / frame_size` (frame_size ≥ 24 for
//! any table bitrate), a fixed number of passes per call. The
//! contract path pools 4 output slots of `1152 × channels × 4`
//! bytes. There is no attacker-controlled event count.

use codec_api::prelude::*;
use media_types::GenerationId;

use super::decoder::{DecodedMp3, Mp3Decoder, decode_stream};
use super::header::{parse_frame_header, walk_frames};
use super::id3::{id3v2_len, parse_id3v2};
use super::seek::{VbrToc, cbr_seek, vbr_toc_seek};
use super::xing::parse_xing;

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
    /// Bytes the walk consumed as frames.
    pub frames: usize,
    /// Whether an ID3 tag parsed.
    pub id3: bool,
    /// Whether a Xing/Info header parsed.
    pub xing: bool,
    /// Whether the full decode produced samples.
    pub decoded: bool,
    /// Frames the contract path emitted.
    pub contract_frames: usize,
}

/// Run the harness (see the module docs). Never panics on any input;
/// every damage path answers a typed error that is swallowed here.
#[must_use]
pub fn fuzz_mp3_stream(data: &[u8]) -> FuzzOutcome {
    let mut outcome = FuzzOutcome {
        frames: 0,
        id3: false,
        xing: false,
        decoded: false,
        contract_frames: 0,
    };
    // 1. ID3 walk.
    if let Ok(Some(len)) = id3v2_len(data) {
        outcome.id3 = true;
        let _ = parse_id3v2(data);
        let _ = len;
    }
    // 2. Header walk from post-ID3 (or 0).
    let start = match id3v2_len(data) {
        Ok(Some(len)) => len,
        _ => 0,
    };
    let frames = walk_frames(data, start.min(data.len()));
    outcome.frames = frames.len();
    if let Some(first) = frames.first() {
        if let Some(bytes) = data.get(first.offset..first.offset + first.header.frame_size as usize)
        {
            // 3. Xing (and its internal LAME parse).
            if let Ok(Some(x)) = parse_xing(bytes, first.offset as u64) {
                outcome.xing = true;
                if let Some(toc) = x.toc {
                    // 6. VBR seek interpolation with the parsed TOC.
                    let v = VbrToc {
                        toc,
                        first_frame_offset: x.frame_end,
                        total_bytes: u64::from(x.bytes.unwrap_or(1)),
                        total_frames: x.frames.unwrap_or(1),
                    };
                    let _ = vbr_toc_seek(&v, 12345, 1152);
                    let _ = vbr_toc_seek(&v, 0, 1152);
                }
            }
            // Single-header parse under raw bytes (boundary input).
            let _ = parse_frame_header(bytes);
        }
        // 6b. CBR plan from the real header.
        let _ = cbr_seek(&first.header, first.offset as u64, 999_999);
    }
    // 4. Full stream pipeline.
    if let Ok(DecodedMp3 { samples, .. }) = decode_stream(data) {
        outcome.decoded = !samples.is_empty();
    }
    // 5. Contract path over the walked frames.
    if let Some(first) = frames.first() {
        let codec = match first.header.layer {
            super::header::Layer::L1 => "mp1",
            super::header::Layer::L2 => "mp2",
            super::header::Layer::L3 => "mp3",
            super::header::Layer::Reserved => "mp3",
        };
        let mut decoder = Mp3Decoder::new();
        let generation = match GenerationId::FIRST.next() {
            Ok(g) => g,
            Err(_) => return outcome,
        };
        let configured = decoder.configure(&DecoderConfig::new(
            match media_types::ids::CodecDescriptor::new(codec) {
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
                    .get(frame.offset..frame.offset + frame.header.frame_size as usize)
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
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_garbage_inputs_never_panic() {
        assert_eq!(
            fuzz_mp3_stream(&[]),
            FuzzOutcome {
                frames: 0,
                id3: false,
                xing: false,
                decoded: false,
                contract_frames: 0
            }
        );
        let garbage = [0xFFu8; 4096];
        let o = fuzz_mp3_stream(&garbage);
        assert!(!o.id3);
        // Repeated 0xFF syncs fail validation (bad bitrate etc.) so
        // the walk stays at zero frames — no hang, no panic.
        assert_eq!(o.frames, 0);
    }

    #[test]
    fn a_header_only_buffer_is_survived() {
        // Valid header, no frame body.
        let o = fuzz_mp3_stream(&[0xFF, 0xFB, 0x90, 0x00]);
        assert_eq!(o.frames, 0);
    }
}
