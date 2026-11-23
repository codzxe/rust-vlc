# AUDIT: crates/playlists/tests/m3u_suite.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-23 |
| Lines / functions | 513 / 21 (1 helper fn, 1 RNG impl with 4 methods, 1 generator fn, 18 tests) |
| File verdict | REAL (honest, mutation-hard test suite; runs in the workspace gate) |
| Reachable from shipped entry point? | N/A (test binary) — runs under `cargo test --workspace`; verified executed: `cargo test -p playlists --test m3u_suite` → `18 passed; 0 failed; 0 ignored; finished in 0.06s` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
This is what a parser test suite should look like: a roundtrip law (parse→write→parse stable) enforced on every fixture, a 23-case dirty corpus, exact warning-sequence pins for the dirty fixture, line-cap boundary tests at/over the cap with exact lengths and late-line numbering, a redacted-error message pin, and two deterministic fuzz-lite harnesses (10k random + 5k mutated inputs) that assert the roundtrip law on everything that parses plus a "fuzz too weak" sanity floor (`ok_count > 100`). I ran it: 18/18 pass in 0.06s. The only gaps: the BOM tolerance it proves for `parse_detailed` does NOT extend to the HLS typing layer (where the same BOM input is refused `not-hls` — a cross-file defect my hls.rs audit documents), and the lone-CR no-panic pin skips the roundtrip law every other parseable input gets.

## 2. What actually works (verified)
— `cargo test -p playlists --test m3u_suite` → `18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s`. All five fixtures exist under crates/playlists/tests/fixtures/ (verified by ls). Mutation reasoning: breaking the line cap, the warning taxonomy, the writer canonical form, or the roundtrip stability each fails a specific exact-equality assertion here.

## 3. Findings
F-001 [S4]
Where: crates/playlists/tests/m3u_suite.rs:128-148 vs crates/playlists/src/hls.rs:222-223
Code:
```
#[test]
fn bom_and_crlf_tolerance() {
    let src = "\u{FEFF}#EXTM3U\r\n#EXTINF:5.5,Title\r\npath/one.wav\r\n\r\n#EXTINF:4,\r\npath two.wav\r\n";
```
What is wrong: coverage gap with a false sense of safety — this test proves the M3U LAYER tolerates a leading BOM, but the exact same input string fed to `parse_media_playlist` refuses `playlists.hls.not-hls` (hls.rs:222-223 scans the raw text; see my hls.rs audit F-001). A reader of this suite would believe BOM handling is done. Why it matters: the real-world BOM defect ships one layer up, untested. What correct looks like: one additional test feeding a BOM'd `#EXTM3U` playlist through `parse_media_playlist` (it would fail today — that is the point).
Confidence: HIGH — Verification: ran this suite; hls.rs behavior verified by read.
F-002 [S4]
Where: crates/playlists/tests/m3u_suite.rs:258-267
Code:
```
    let Ok(d) = parse_detailed("a\rb\rc") else {
        unreachable!("CR-only input must parse");
    };
    assert_eq!(d.playlist.len(), 1);
    assert_eq!(d.playlist.entries[0].location, "a\rb\rc");
```
What is wrong: the lone-CR pin skips `assert_roundtrip` (every other parseable input in the file gets the law). A location containing raw CRs must survive write→re-parse for the law to hold; it is plausible but unproven here. Why it matters: an untested invariant at exactly the weird input the test exists to pin. What correct looks like: add `assert_roundtrip("a\rb\rc")`.
Confidence: HIGH — Verification: read (absence of the call verified).

## 4. Per-function table (test file — per-test verdicts)
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| assert_roundtrip (helper) | 20-32 | REAL | — | law: parse→write→parse stable |
| golden_basic_extended | 36-63 | REAL | — | exact Entry equality incl. -1.0 marker, Windows path verbatim |
| golden_plain_urls_without_header | 65-77 | REAL | — | 4 entries, no metadata |
| golden_directives_attach_in_order | 79-113 | REAL | — | exact directive vectors, colon-title, trailing-comma title None |
| golden_comments_and_blanks_ignored | 115-125 | REAL | — | internal-space location verbatim |
| bom_and_crlf_tolerance | 127-148 | REAL | S4 | F-001 cross-layer gap |
| lf_crlf_mix | 150-159 | REAL | — | |
| repeated_header_anywhere_is_ignored | 161-169 | REAL | — | 3 headers, 0 warnings |
| dirty_fixture_degrades_honestly | 173-196 | REAL | — | exact 4-warning vector, 2 surviving entries |
| non_finite_durations_are_bad_duration | 198-214 | REAL | — | inf/NaN/1e999/empty → BadDuration |
| empty_and_comment_only_inputs | 216-241 | REAL | — | 4 shapes, no warnings |
| garbage_bytes_are_typed_invalid_utf8 | 243-256 | REAL | — | incl. BOM+invalid sequence |
| lone_cr_file_does_not_panic | 258-267 | REAL | S4 | F-002 no roundtrip |
| ten_megabyte_line_is_a_typed_error | 271-290 | REAL | — | exact redacted message pin (MP §12) |
| line_cap_boundaries | 292-314 | REAL | — | at-cap ok, +1 typed, late-line number |
| write_canonical_golden_pin | 318-359 | REAL | — | exact canonical text, empty→header only |
| roundtrip_all_fixtures_and_dirty_corpus | 361-395 | REAL | — | 23-case corpus incl. BOM, non-ASCII, huge f64 |
| SplitMix64 impl + random_input | 399-446 | REAL | — | deterministic, seeded, branch-steering bytes |
| fuzz_lite_random_bytes_10k_no_panic | 448-467 | REAL | — | 10k inputs, roundtrip law, >100-parses floor |
| fuzz_lite_mutations_of_valid_playlist_5k | 469-513 | REAL | — | 5k mutations, same law + floor |

## 5. Test honesty
Would each fail if the code were broken? YES for all 18 — exact-equality assertions on entries, warnings, wire text, error variants, and message strings; the fuzz harnesses assert an invariant (roundtrip) plus a strength floor. No tautologies, no `assert!(true)` shapes, no mocks-of-the-thing-tested (it tests the real parser/writer). `#[ignore]`: none.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "CI-able: no I/O beyond compile-time include_str!, bounded runtime" | m3u_suite.rs:4-5 | REAL | ran: 0.06s; only include_str! |
| "deterministic fuzz-lite … (random bytes + mutations)" | m3u_suite.rs:2-3 | REAL | fixed seeds 0x4D31_C0DE_5EED_000{1,2} |

## 7. Dead / duplicated / unreachable code
None. All 18 tests run (verified by execution).

## 8. Dependencies & flags
Test-only; depends on the playlists crate's public API (`Entry`, `HlsSegment`, `MAX_LINE_BYTES`, `ParseError`, `ParseWarning`, `Playlist`, `parse`, `parse_bytes`, `parse_detailed`, `write`) — all real exports. No features.

## 9. Open questions for the next agent
- hls_attrs.rs (not my file) — does it cover the EXT-X-KEY/BYTERANGE/DISCONTINUITY stamping at the m3u layer with the same rigor?
- Should the roundtrip law be promoted to a proptest with the same corpus? (nice-to-have, not a defect)
