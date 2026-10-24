//! The ASS/SSA fuzz harness (card 5.Q1; doc 04 §8.5 registry name
//! `fuzz_sub_ass`, table §5 row "ASS/SSA").
//!
//! One call = one full untrusted-ASS pass: lossy byte decode, the
//! section/line walk, style/timing/margin parses, event-field
//! splitting — the whole [`parse`](super::parser::parse) surface —
//! plus a style lookup over whatever survived. **The harness
//! contract is absence-of-panic/hang/unbounded-allocation, nothing
//! else** — malformed input is *expected* to produce the parser's
//! damage ledger or empty sections, both swallowed here; the AC
//! suites pin the specific outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_sub_ass.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's ASS differential suites.
//!
//! # Bounds (MP §12 / D-2)
//!
//! Parse: one pass, output ≤ O(input); the damage ledger is capped
//! and the cue count is bounded by the line count.

/// Run one fuzz pass over `data` (see the module docs). Never
/// panics, never hangs, never allocates without a ceiling — by
/// construction of the parser's bounds-checked paths.
pub fn fuzz_sub_ass(data: &[u8]) {
    let text = String::from_utf8_lossy(data);
    let doc = super::parser::parse(&text);
    // Observable-surface probes (bounded lookups, not walks).
    let _ = doc.title();
    let _ = doc.style("Default");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_marker_inputs_survive() {
        fuzz_sub_ass(&[]);
        fuzz_sub_ass(b"[Script Info]\n");
        fuzz_sub_ass(b"[Events]\nFormat: Layer, Start, End, Style, Text\n");
    }

    #[test]
    fn hostile_wide_inputs_survive() {
        // All-0xFF (invalid UTF-8 everywhere → the lossy path) and
        // all-0x00 (embedded NULs / zero-length fields).
        fuzz_sub_ass(&vec![0xff; 4096]);
        fuzz_sub_ass(&vec![0x00; 4096]);
        // A plausible header with hostile dialogue lines after it.
        let mut hostile = b"[Events]\nFormat: Layer, Start, End, Style, Text\n".to_vec();
        for i in 0..64u32 {
            hostile.extend_from_slice(
                format!("Dialogue: 0,0:0:{i:02}.{:02},0:0:{:02}.00,Default,,0,0,0,,", i % 60, i % 60)
                    .as_bytes(),
            );
        }
        fuzz_sub_ass(&hostile);
    }
}
