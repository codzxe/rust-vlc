//! The HEVC bitstream-parser fuzz harness (card 5.Q1; doc 04 §8.5
//! registry name `fuzz_codec_hevc` "(parser self-build)", table §4.1
//! row "HEVC/H.265").
//!
//! One call = one untrusted byte buffer through **every** parser
//! surface of the landed parser self-build (card 5.G1; decode stays
//! hw-first per ADR-001 and rides its own suites):
//!
//! 1. the Annex-B start-code walk
//!    ([`annexb_nals`](super::parser::nal::annexb_nals));
//! 2. per-NAL parameter-set parses — VPS + SPS + PPS, all via the
//!    reusable-RBSP `parse_with_buffer` entries;
//! 3. the `hvcC` record walk
//!    ([`HvcCConfig::parse`](super::parser::nal::HvcCConfig::parse))
//!    — the MP4 configuration surface (arrays-of-NALs included);
//! 4. the length-prefixed walk at every legal NAL length size
//!    (1/2/4).
//!
//! The slice-header seam needs active VPS/SPS/PPS contexts (its own
//! unit suites own that pass); every context-free surface is here.
//!
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — damage is expected to surface as a
//! typed error and is swallowed here.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_codec_hevc.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's parser conformance suites.
//!
//! # Bounds (MP §12)
//!
//! Memory is bounded by the input: NAL views are zero-copy, the RBSP
//! buffer is reused across NALs, a fixed number of passes per call.

/// Maximum NALs parsed per pass `[PROPOSAL: 4096 — far above any
/// sane AU's NAL count; a second belt over the input-length bound]`.
pub const FUZZ_NAL_BUDGET: usize = 4096;

/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates beyond input-derived bounds — by
/// construction of the bounded parser paths.
pub fn fuzz_codec_hevc(data: &[u8]) {
    // Pass 1 + 2: the Annex-B walk with per-NAL parameter-set parses.
    let nals = super::parser::nal::annexb_nals(data);
    let mut rbsp = Vec::new();
    for nal in nals.iter().take(FUZZ_NAL_BUDGET) {
        let _ = super::parser::vps::VideoParameterSet::parse_with_buffer(nal, &mut rbsp);
        let _ = super::parser::sps::SeqParameterSet::parse_with_buffer(nal, &mut rbsp);
        let _ = super::parser::pps::PicParameterSet::parse_with_buffer(nal, &mut rbsp);
    }

    // Pass 3: the hvcC record walk over the raw bytes.
    let _ = super::parser::nal::HvcCConfig::parse(data);

    // Pass 4: the length-prefixed walk at every legal length size.
    for length_size in [1u8, 2, 4] {
        if let Ok(mut nals) = super::parser::nal::LengthPrefixedNals::new(data, length_size) {
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
        fuzz_codec_hevc(&[]);
        fuzz_codec_hevc(&[0x00, 0x00, 0x00, 0x01]);
        // NAL type 32 (VPS) header byte after a start code.
        fuzz_codec_hevc(&[0x00, 0x00, 0x01, 0x40, 0x01]);
    }

    #[test]
    fn hostile_wide_inputs_survive() {
        fuzz_codec_hevc(&vec![0x00; 4096]);
        fuzz_codec_hevc(&vec![0xff; 4096]);
        // Start codes every 8 bytes with hostile VPS/SPS/PPS-typed
        // headers between.
        let mut flood = Vec::new();
        for _ in 0..256 {
            flood.extend_from_slice(&[0x00, 0x00, 0x01, 0x40, 0x01, 0xff, 0x80, 0x7f]);
        }
        fuzz_codec_hevc(&flood);
    }
}
