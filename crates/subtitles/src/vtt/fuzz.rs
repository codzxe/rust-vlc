//! The WebVTT fuzz harness (card 5.Q1; doc 04 §8.5 registry name
//! `fuzz_sub_vtt`, table §5 row "WebVTT").
//!
//! One call = one full untrusted-VTT pass: lossy byte decode, BOM
//! handling, the header/`WEBVTT` signature gate, cue timing and
//! settings parses, the damage ledger — the whole
//! [`parse`](super::parse) surface. **The harness contract is
//! absence-of-panic/hang/unbounded-allocation, nothing else** —
//! malformed input is *expected* to produce typed errors or damage
//! events, both swallowed here; the AC suites pin the specific
//! outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_sub_vtt.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's VTT suites.
//!
//! # Bounds (MP §12 / D-2)
//!
//! Parse: one pass, output ≤ O(input); the damage ledger is capped
//! and the cue count is bounded by the line count.

/// Run one fuzz pass over `data` (see the module docs). Never
/// panics, never hangs, never allocates without a ceiling — by
/// construction of the parser's bounds-checked paths.
pub fn fuzz_sub_vtt(data: &[u8]) {
    let text = String::from_utf8_lossy(data);
    let _ = super::parse(&text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_marker_inputs_survive() {
        fuzz_sub_vtt(&[]);
        fuzz_sub_vtt(b"WEBVTT\n\n");
        fuzz_sub_vtt(b"WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nhello\n");
    }

    #[test]
    fn hostile_wide_inputs_survive() {
        fuzz_sub_vtt(&vec![0xff; 4096]);
        fuzz_sub_vtt(&vec![0x00; 4096]);
        // Signature + hostile cue-timing floods.
        let mut hostile = b"WEBVTT\n\n".to_vec();
        for i in 0..64u32 {
            hostile.extend_from_slice(
                format!("00:00:{:02}.{:03} --> 00:00:{:02}.000 align:start line:0%\n", i % 60, i, i % 60)
                    .as_bytes(),
            );
        }
        fuzz_sub_vtt(&hostile);
    }
}
