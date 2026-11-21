# AUDIT: crates/codecs/src/audio/flac/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-21 |
| Lines / functions | 130 / 5 (3 fns + 1 test + module docs/re-exports) |
| File verdict | UNREACHABLE (its three public functions are real and tested, but no shipped entry point calls any of them) |
| Reachable from shipped entry point? | NO for this file's functions: `frame_header_at` — only test callers (`crates/codecs/tests/flac_seek.rs:70`, `tests/flac_bitexact.rs:159`); `decode_frame_with_streaminfo` — only the in-file test (mod.rs:128); `output_format` — only the in-file test (mod.rs:123). Repo-wide rg finds zero production callers. (The module's actual shipped lane is `FlacDecoder` in decoder.rs — W07-A01's file, reachable via `pipeline_host.rs:253-259` — which calls `SeekPlanner::parse_header_at` directly, bypassing this hub.) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | repo-wide `rg -n "frame_header_at\|decode_frame_with_streaminfo\|output_format\("` (callers enumerated above; ivf's same-named method is unrelated); `sed -n '319,395p' flac/seek.rs` (independent re-verification of the reserved-bit mask and the `.min(6)` UTF-8 cap behind F-001); read of sibling audit `audit/files/crates/codecs/src/audio/flac/seek.rs.audit.md` (W07-A07 F-002/F-003 — re-verified in source by me, not trusted); baseline orchrun log:345 (`output_format_matches_the_decode_mapping ... ok`), :997 (flac_vlc_differential runs). |

## 1. One-paragraph verdict
This hub file's own code is three small, correct, tested composition functions — and every one of them is dead in the shipped build: the FLAC production path (`FlacDecoder`) reaches into `decoder.rs`/`seek.rs` directly, so the hub's "public navigation surface" is a test-only façade. Worse, the hub's doc for `frame_header_at` makes a verification claim that is false in the code it points at: I independently re-verified (mask arithmetic on seek.rs:333-336, `.min(6)` on :362) that the frame-header parse does NOT check the reserved bit and cannot parse the spec's 7-byte/36-bit number coding, contradicting "sync, reserved rules, UTF-8 number, escapes and CRC-8 all verified" — and the in-code comment at seek.rs:331-332 even asserts "reserved bit must be 0" of a mask that leaves that bit free.

## 2. What actually works (verified)
- Baseline orchrun log:345: `audio::flac::tests::output_format_matches_the_decode_mapping ... ok` — the test's STREAMINFO bit-packing (48000≪44 | 1≪41 | 23≪36 | 1000) re-derived by me: rate 48 kHz, 2 channels, 24 bps, 1000 total — the assertions (S24/2ch/48k/interleaved, empty-packet typed error) are genuine and would fail on a mapping regression.
- The re-export surface resolves (all named items exist — the baseline build compiles it; `flac::tests::the_public_surface_reexports_resolve`-style check equivalent: build #1 exit 0).
- VLC-differential honesty claim verified: `tests/flac_vlc_differential.rs` exists, header documents PENDING-INFRA with the always-on Lumen-side ffmpeg-oracle digests computed for real (baseline log:997 ran it).

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/audio/flac/mod.rs:68-71
Code:
```rust
/// Parse the FLAC frame header at `offset` (the public navigation
/// surface over [`seek::SeekPlanner::parse_header_at`]): sync,
/// reserved rules, UTF-8 number, escapes and CRC-8 all verified.
```
What is wrong: two of the five listed verifications are not performed by the delegated parser. (a) Reserved bit: seek.rs checks `b1 & 0b1111_1100 != 0b1111_1000` — the mask leaves bit 1 (the spec's reserved bit) free; a header with the reserved bit set (e.g. byte-1 `0xFB`, since `0xFB & 0xFC == 0xF8`) passes the sync gate. The code comment directly above the mask ("then reserved bit must be 0", seek.rs:331-332) asserts what the mask does not do. (b) UTF-8 number: `additional = first.leading_ones().min(6)` (seek.rs:362) caps the coding at 6-byte forms, so the spec's 7-byte/36-bit sample-number coding (leading byte 0xFE) is misparsed as a 6-byte form instead of refused or decoded.
Why it matters: this doc is the public API contract for `frame_header_at`; a consumer (a demuxer card, a validator) reading "reserved rules … all verified" will trust spec-strictness that does not exist. Reserved-bit-set headers are spec-invalid and a strict counterpart (ffmpeg/claxon reject them) — accepting them makes this parser the lenient outlier. Cross-ref: W07-A07's seek.rs audit F-002/F-003 (I re-verified both in source before writing this, per the trust-nothing rule).
What correct looks like: fix the parser (mask `0b1111_1110` vs `0b1111_1000` for the reserved bit; a 7-byte branch for 0xFE) or weaken this doc to name exactly which rules are enforced.
Confidence: HIGH — Verification: read + independent mask arithmetic; sibling audit cross-checked.

### F-002 [S3]
Where: crates/codecs/src/audio/flac/mod.rs:75-106
Code:
```rust
pub fn frame_header_at(data: &[u8], offset: u64) -> Option<ParsedFrameHeader> {
    SeekPlanner::parse_header_at(data, offset)
}
```
(plus `decode_frame_with_streaminfo` :86-92 and `output_format` :98-106)
What is wrong: all three public functions in this file are test-only surface. Repo-wide rg: `frame_header_at` called only by `tests/flac_seek.rs:70` and `tests/flac_bitexact.rs:159`; `decode_frame_with_streaminfo` and `output_format` called only by the in-file test. The production FLAC lane calls `SeekPlanner::parse_header_at` directly from decoder.rs (per the call chain in the sibling audits).
Why it matters: the docs pitch these as live seams ("the public navigation surface", "the registry/adapters' preview of AudioFormat", "convenience composition … for callers that hold a raw STREAMINFO blob" — no such callers exist); dead public API is drift bait, and the same tree-shake argument as mp2/seek.rs applies (`codec-flac` gates the module, but within it these stay exported).
What correct looks like: either wire the intended consumers (native-FLAC demuxer card is matrix-`not_started` per the sibling audit) or move these helpers to a `#[cfg(test)]`/test-support module and fix the docs.
Confidence: HIGH — Verification: repo-wide rg for each name; only test files match.

### F-003 [S4]
Where: crates/codecs/src/audio/flac/mod.rs:24-26
Code:
```rust
//! - [`decode_frame`] — the free surface: one packet + facts → one
//!   decoded block with no contract state (the `decode_image` pattern
//!   of 2.B4; the [`fuzz`] entry).
```
What is wrong: minor naming debt — the doc references `decode_frame` as "the `decode_image` pattern of 2.B4", but 2.B4's precedent is named `decode_image` in the MJPEG module while the FLAC twin is `decode_frame`; and the listed "consumers" of the free surface are the fuzz entry alone (no shipped caller), which the module map does not disclose.
Why it matters: doc precision on a hub file sets expectations for five sibling files; the unreachability of the free surface should be as honestly stated here as it is in the seek.rs sibling audit.
What correct looks like: one clause: "free surface: test/fuzz-only until the native-FLAC demuxer card lands".
Confidence: HIGH — Verification: read + the rg evidence of F-002.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-48 | REAL (one FALSE claim) | S3 | F-001 (verification list), F-003 (naming/disclosure); conventions claim (no unsafe/no panic) holds for this file's own code — verified by reading: pure delegates, no indexing. |
| module decls + re-exports | 50-66 | REAL | — | All items exist (build exit 0); `pub mod fuzz` justified by out-of-crate callers (fuzz shim + survival tests). |
| `frame_header_at` | 74-77 | REAL (unreachable) | S3 | Thin delegate; doc claim false (F-001); test-only callers (F-002). |
| `decode_frame_with_streaminfo` | 86-92 | REAL (unreachable) | S3 | Composition of two audited-sibling functions; in-file-test-only (F-002). |
| `output_format` | 98-106 | REAL (unreachable) | S3 | Same left-justify mapping as decode (cross-pinned by the in-file test, baseline:345); test-only (F-002). |
| test `output_format_matches_the_decode_mapping` | 112-129 | REAL | — | Genuine: hand-packed STREAMINFO bits verified by me; asserts S24/2ch/48k + typed error on empty packet. Would fail on mapping drift. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `output_format_matches_the_decode_mapping` | Yes | Exact field asserts on a hand-packed STREAMINFO (bit layout re-derived: 20-bit rate ≪44, 3-bit channels−1 ≪41, 5-bit bps−1 ≪36, 36-bit total); plus the empty-packet typed-error assert exercises the composition's error path. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "sync, reserved rules, UTF-8 number, escapes and CRC-8 all verified" (:69-70) | fn doc | FALSE | Reserved bit unchecked + 36-bit form misparsed — independently re-verified in seek.rs:333-336/:362 (F-001). |
| "the registry/adapters' preview of AudioFormat without decoding" (:94-96) | fn doc | FALSE (implied consumers) | Zero production callers of `output_format` (F-002). |
| "The VLC differential hook … is registered and honestly pending infra (no VLC in the sandbox)" (:46-48) | module doc | REAL | `tests/flac_vlc_differential.rs` header verified: PENDING-INFRA, Lumen-side digests computed against the ffmpeg oracle; ran in baseline (log:997). |
| "no panic on any input — every decode/parse/seek path is bounds-checked" (:33-35) | module doc | REAL for this file | Read: delegates + composition only; the underlying paths' panic-freedom is the sibling files' verdict (fuzz survival passes in baseline). |

## 7. Dead / duplicated / unreachable code
`frame_header_at`, `decode_frame_with_streaminfo`, `output_format` — all three dead in the shipped build (F-002); no duplication (delegation only).

## 8. Dependencies & flags
Rides `codec-flac` (default-on, `claxon` =0.4.3 optional dep). No unsafe, no panics in this file.

## 9. Open questions for the next agent
- W07-A01 owns `flac/decoder.rs`: confirm whether `FlacDecoder` duplicates any of this hub's composition (a `decode_frame_with_streaminfo`-shaped path inside the contract decoder would make the hub's third function redundant duplication rather than mere dead code).
