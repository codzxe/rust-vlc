# AUDIT: crates/playlists/src/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2025-06-14 |
| Lines / functions | 95 / 4 harness fns + 2 tests |
| File verdict | REAL (thin, honest survival harnesses; executed 2/2 pass; the contract wording here is the CORRECT version that netfuzz/fuzz.rs lacks) |
| Reachable from shipped entry point? | N/A (fuzz surface, `pub` in the playlists crate). Consumers: 4 libFuzzer shims (`fuzz/fuzz_targets/fuzz_playlist_{m3u,pls,xspf,cue}.rs` — all four exist, verified), the netfuzz sweep (`fuzz_playlist_m3u`), and the crate's four format suites' own fuzz-lite harnesses. The parsers underneath (`parse_bytes`, `parse_pls_bytes`, `parse_xspf_bytes`, `parse_cue_bytes`) are crate-public library code. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/playlists-c262b72361161a1c fuzz` -> "test result: ok. 3 passed" (2 in this module + mpd's), exit 0; `ls fuzz/fuzz_targets | grep playlist` -> 4 shims; `grep -c fuzz_lite` over the 4 format suites -> 2 each |

## 1. One-paragraph verdict
Four one-line survival harnesses, each a single swallowed `parse_*_bytes` call — and unlike the streaming crate's netfuzz harness, this module's doc states the contract HONESTLY: "The harness contract is absence-of-panic/hang/unbounded-allocation, **nothing else** — malformed input is *expected* to produce typed errors or damage warnings, both swallowed here; the per-format suites pin the specific outcomes" (:14-18). That is exactly right, and it is the phrasing netfuzz/fuzz.rs:34-36 should have used. All four cargo-fuzz shims exist; all four format suites carry their own fuzz-lite (m3u/pls/xspf/cue — verified 2 fuzz-lite references each); the two in-file tests are honest smoke (empty/marker inputs, 4096-byte 0xFF/0x00 floods, 128-entry directive floods). The only defects are wording-level: "the deterministic sweeps below" points at suites that live in other files, and the in-file tests themselves are far too weak to deserve the word "sweeps".

## 2. What actually works (verified)
— `./target/debug/deps/playlists-c262b72361161a1c fuzz` -> `fuzz::tests::empty_and_marker_inputs_survive ... ok`, `fuzz::tests::hostile_wide_inputs_survive ... ok`, exit 0.
— All four shims exist: `fuzz_playlist_cue.rs`, `fuzz_playlist_m3u.rs`, `fuzz_playlist_pls.rs`, `fuzz_playlist_xspf.rs` (ls, fuzz/fuzz_targets/).
— Cross-crate use verified: netfuzz_sweep.rs:351 calls `playlists::fuzz::fuzz_playlist_m3u` (executed earlier — 3000 iterations).

## 3. Findings
F-001 [S4]
Where: crates/playlists/src/fuzz.rs:22-25
Code (verbatim, 4 lines):

        //! The cargo-fuzz targets live at `fuzz/fuzz_targets/fuzz_playlist_*.rs`
        //! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
        //! logic is meanwhile proven on stable by the deterministic sweeps
        //! below and the crate's committed-fixture suites.

What is wrong: "the deterministic sweeps below" — there are no sweeps below; this file contains two 5-input smoke tests (:58-94). The actual deterministic fuzz-lite harnesses live in four OTHER files (tests/m3u_suite.rs, pls_suite.rs, xspf_suite.rs, cue_suite.rs — verified 2 fuzz-lite references each). The claim's substance is true (stable-toolchain fuzz-lite proof exists for all four formats), but the pointer is wrong. What correct looks like: "in the per-format suites under tests/". Confidence: HIGH — Verification: ran (grep -c over the suites) + read.
F-002 [S4]
Where: crates/playlists/src/fuzz.rs:58-66
Code (verbatim, first lines of the test):

        #[test]
        fn empty_and_marker_inputs_survive() {
            for data in [
                &[][..],
                b"#EXTM3U\n",

What is wrong: the in-file tests are smoke-only (5 marker inputs + 2 wide floods + 2 floods of well-formed entries) — nothing hostile about them beyond length; no mutations, no random corpus. Fine as a compile-and-don't-crash gate, weak as "proven". Mitigated by the real fuzz-lite in the format suites. Confidence: HIGH — Verification: ran + read.

## 4. Per-function table (file ≤100 LOC — table included anyway for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_playlist_m3u | 35-37 | REAL | — | one swallowed parse_bytes call |
| fuzz_playlist_pls | 40-42 | REAL | — | |
| fuzz_playlist_xspf | 45-47 | REAL | — | |
| fuzz_playlist_cue | 50-52 | REAL | — | |
| tests: empty_and_marker_inputs_survive | 58-72 | REAL | S4 | smoke (F-002) |
| tests: hostile_wide_inputs_survive | 74-94 | REAL | S4 | floods well-formed entries; "hostile" is generous |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_marker_inputs_survive | YES (panic only) | any panic on the 5 inputs × 4 formats fails |
| hostile_wide_inputs_survive | YES (panic only) | 0xFF/0x00 floods + 128-entry floods |
Both are correctly scoped by the module doc's "nothing else" contract — no overclaim inside this file.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call = one full untrusted-playlist pass through the format's parse_bytes entry (BOM/encoding sniff included)" | fuzz.rs:6-7 | REAL | all four harnesses call the parse_bytes family; the m3u one exercised 3000× by the netfuzz sweep (ran) |
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else — … both swallowed here" | fuzz.rs:14-18 | REAL (honest) | the bodies really do swallow everything (:36,:41,:46,:51) — the doc says so plainly |
| "the harness logic is meanwhile proven on stable by the deterministic sweeps below" | fuzz.rs:24-25 | PARTIAL | sweeps exist but not "below" — F-001 |
| "line lengths and XML depth are ceiling-checked by the parsers themselves, damage ledgers are capped" | fuzz.rs:29-31 | REAL-UNPROVEN (parsers are other agents' files; the cue line-cap is pinned by cue_suite's `line_cap_is_fatal_and_redacted`, which I executed) | |

## 7. Dead / duplicated / unreachable code
None. All four harnesses have live consumers (shims + suites + the cross-crate sweep).

## 8. Dependencies & flags
Crate-internal only (the four parse entry points). No deps, no unsafe, no feature gates. `#[cfg(test)]` correctly scopes the smoke tests.

## 9. Open questions for the next agent
- m3u.rs/pls.rs/xspf.rs/cue.rs parser owners: the survival contract's "unbounded allocation" half rests on your line caps and ledger caps — pin them per format the way cue_suite pins the 65536-byte line cap (executed by me).
- mpd.rs has its own fuzz-lite (`mpd::tests::fuzz_lite_mutations_never_panic`, seen in my test run) but NO harness in this module and NO `fuzz_playlist_mpd` shim — deliberate (mpd is dash-scoped?) or gap? UNVERIFIED.
