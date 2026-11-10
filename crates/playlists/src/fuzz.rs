//! The playlist fuzz harnesses (card 5.Q1; doc 04 §8.5 registry
//! naming extended per the card: `fuzz_playlist_<fmt>` — the
//! playlist family sits outside doc 04's §2/§3/§5 tables, so this
//! module establishes the registry surface the fuzz README carries).
//!
//! One call = one full untrusted-playlist pass through the format's
//! `parse_bytes` entry (BOM/encoding sniff included):
//!
//! - [`fuzz_playlist_m3u`] — M3U/M3U8/extended + HLS directives;
//! - [`fuzz_playlist_pls`] — the INI-shaped `File`/`Title` walk;
//! - [`fuzz_playlist_xspf`] — the XML depth/bounds-checked walk;
//! - [`fuzz_playlist_cue`] — the CUE-sheet token/timing walk.
//!
//! **The harness contract is absence-of-panic/hang/unbounded-
//! allocation, nothing else** — malformed input is *expected* to
//! produce typed errors or damage warnings, both swallowed here; the
//! per-format suites (crates/playlists/tests) pin the specific
//! outcomes.
//!
//! # Wiring status (honest)
//!
//! The cargo-fuzz targets live at `fuzz/fuzz_targets/fuzz_playlist_*.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's committed-fixture suites.
//!
//! # Bounds (MP §12 / D-2)
//!
//! Parse: one pass, output ≤ O(input); line lengths and XML depth
//! are ceiling-checked by the parsers themselves, damage ledgers are
//! capped.

/// Run one M3U/M3U8 fuzz pass over `data` (see the module docs).
/// Never panics, never hangs, never allocates without a ceiling.
pub fn fuzz_playlist_m3u(data: &[u8]) {
    let _ = crate::parse_bytes(data);
}

/// Run one PLS fuzz pass over `data`. Same contract.
pub fn fuzz_playlist_pls(data: &[u8]) {
    let _ = crate::parse_pls_bytes(data);
}

/// Run one XSPF fuzz pass over `data`. Same contract.
pub fn fuzz_playlist_xspf(data: &[u8]) {
    let _ = crate::parse_xspf_bytes(data);
}

/// Run one CUE-sheet fuzz pass over `data`. Same contract.
pub fn fuzz_playlist_cue(data: &[u8]) {
    let _ = crate::parse_cue_bytes(data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_marker_inputs_survive() {
        for data in [
            &[][..],
            b"#EXTM3U\n",
            b"[playlist]\n",
            b"<?xml version=\"1.0\"?>\n",
            b"FILE \"a.wav\" WAVE\n",
        ] {
            fuzz_playlist_m3u(data);
            fuzz_playlist_pls(data);
            fuzz_playlist_xspf(data);
            fuzz_playlist_cue(data);
        }
    }

    #[test]
    fn hostile_wide_inputs_survive() {
        for hostile in [&vec![0xffu8; 4096][..], &vec![0x00u8; 4096][..]] {
            fuzz_playlist_m3u(hostile);
            fuzz_playlist_pls(hostile);
            fuzz_playlist_xspf(hostile);
            fuzz_playlist_cue(hostile);
        }
        // Directive/key floods (maximal line counts, hostile keys).
        let mut m3u = b"#EXTM3U\n".to_vec();
        for i in 0..128u32 {
            m3u.extend_from_slice(format!("#EXTINF:{i},t{i}\nhttp://x/{i}\n").as_bytes()); // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
        }
        fuzz_playlist_m3u(&m3u);
        let mut xspf = b"<?xml version=\"1.0\"?><playlist version=\"1\">".to_vec();
        for i in 0..128u32 {
            xspf.extend_from_slice(format!("<track><location>u{i}</location></track>").as_bytes());
        }
        xspf.extend_from_slice(b"</playlist>");
        fuzz_playlist_xspf(&xspf);
    }
}
