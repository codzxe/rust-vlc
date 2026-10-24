//! The H.264 bitstream-parser fuzz harness (card 5.Q1; doc 04 §8.5
//! registry name `fuzz_codec_h264` "(parser self-build)", table §4.1
//! row "H.264/AVC").
//!
//! One call = one untrusted byte buffer through **every** parser
//! surface of the landed parser self-build (card 5.F1; decode itself
//! is the hw-first ADR-001 half and rides its own suites):
//!
//! 1. the Annex-B start-code walk
//!    ([`annexb_nals`](super::parser::nal::annexb_nals)) — the
//!    zero-copy NAL splitter under sync-code floods;
//! 2. per-NAL parameter-set parses (SPS + PPS, both via the
//!    reusable-RBSP `parse_with_buffer` entry) and the SEI walk
//!    ([`parse_sei`](super::parser::sei::parse_sei)) over the
//!    unescaped RBSP;
//! 3. the `avcC` record walk
//!    ([`AvcConfig::parse`](super::parser::nal::AvcConfig::parse)) —
//!    the MP4 configuration surface;
//! 4. the length-prefixed walk at every legal NAL length size
//!    (1/2/4) — the hvcC/avc1 carry format.
//!
//! The slice-header seam needs *active* SPS/PPS contexts (its own
//! unit suites own that pass); every context-free surface is here.
//!
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — damage is expected to surface as a
//! typed error and is swallowed here.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_codec_h264.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's parser conformance suites.
//!
//! # Bounds (MP §12)
//!
//! Memory is bounded by the input: NAL views are zero-copy, the RBSP
//! buffer is reused across NALs, a fixed number of passes per call.

/// Maximum NALs parsed per pass `[PROPOSAL: 4096 — far above any
/// sane AU's NAL count; the take() cap is a second belt over the
/// input-length bound]`.
pub const FUZZ_NAL_BUDGET: usize = 4096;

/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates beyond input-derived bounds — by
/// construction of the bounded parser paths.
pub fn fuzz_codec_h264(data: &[u8]) {
    // Pass 1 + 2: the Annex-B walk with per-NAL parameter-set/SEI
    // parses (reusable RBSP buffer — steady state performs no
    // allocation per NAL).
    let nals = super::parser::nal::annexb_nals(data);
    let mut rbsp = Vec::new();
    for nal in nals.iter().take(FUZZ_NAL_BUDGET) {
        let _ = super::parser::sps::SeqParameterSet::parse_with_buffer(nal, &mut rbsp);
        let _ = super::parser::pps::PicParameterSet::parse_with_buffer(nal, &mut rbsp);
        if let Ok(rbsp_bytes) = super::parser::nal::unescape_rbsp(nal) {
            let _ = super::parser::sei::parse_sei(&rbsp_bytes);
        }
    }

    // Pass 3: the avcC record walk over the raw bytes.
    let _ = super::parser::nal::AvcConfig::parse(data);

    // Pass 4: the length-prefixed walk at every legal length size.
    for length_size in [1u8, 2, 4] {
        if let Ok(mut nals) = super::parser::nal::AvcNals::new(data, length_size) {
            while nals.next().is_some() {
                // Walk to exhaustion — the iterator's own bounds
                // (input length) terminate it.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_marker_inputs_survive() {
        fuzz_codec_h264(&[]);
        fuzz_codec_h264(&[0x00, 0x00, 0x00, 0x01]);
        fuzz_codec_h264(&[0x00, 0x00, 0x01, 0x67]);
    }

    #[test]
    fn hostile_wide_inputs_survive() {
        // All-0x00: a start-code flood (maximal NAL splits).
        fuzz_codec_h264(&vec![0x00; 4096]);
        // All-0xFF: maximal header/profile false positives.
        fuzz_codec_h264(&vec![0xff; 4096]);
        // Start codes every 8 bytes with hostile NAL bytes between.
        let mut flood = Vec::new();
        for _ in 0..256 {
            flood.extend_from_slice(&[0x00, 0x00, 0x01, 0x67, 0xff, 0xfe, 0x80, 0x7f]);
        }
        fuzz_codec_h264(&flood);
    }
}
