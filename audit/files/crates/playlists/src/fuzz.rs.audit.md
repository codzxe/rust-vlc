# AUDIT: crates/playlists/src/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-21 |
| Lines / functions | 95 / 6 items: 4 fuzz harness fns (`fuzz_playlist_m3u/pls/xspf/cue`) + 2 tests (`empty_and_marker_inputs_survive`, `hostile_wide_inputs_survive`) |
| File verdict | REAL (fuzz-infrastructure by design; harnesses executed in baseline; cargo-fuzz shims verified wired) — production-unreachable |
| Reachable from shipped entry point? | NO production path calls it (callers: `fuzz/fuzz_targets/fuzz_playlist_*.rs` libFuzzer shims — verified present, one per format; `crates/streaming/tests/netfuzz_sweep.rs:351` (the HLS sweep, test); the module's own tests). `pub mod fuzz` at lib.rs:113 ships it in the rlib; call-unreachable in binaries. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read tool full file; `sed -n` verification of the quote; `ls fuzz/fuzz_targets \| rg playlist` (4 shims exist); `cat fuzz/fuzz_targets/fuzz_playlist_cue.rs` (shim calls the in-crate harness, as documented); `rg parse_pls_bytes/parse_xspf_bytes` lib.rs:90,96 (re-exports real); `rg "BOM\|sniff\|encoding"` in m3u/pls/xspf (encoding-sniff claim check — no sniffing exists; m3u.rs:68 "the parser never guesses an encoding"); `grep -n "playlists::fuzz"` repo-wide (caller census) + od byte-verification of one hit (display-pipeline artifact ruled out); `cat scripts/fuzz_registry_check.sh` (registry gate logic: bins==files==README==harness fns + corpus); baseline: `orchrun-test-workspace-nofailfast.log.txt:4262, :4267` (fuzz::tests 2/2 ok), per-format fuzz-lite all ok (m3u_suite :4393-region, pls :4393, xspf :4414, cue_suite :4321/:4336), netfuzz_sweep :5250-5255 |

## 1. One-paragraph verdict
This is the playlist fuzz-harness module: four one-line wrappers, each driving one full untrusted byte slice through a format's `parse_*_bytes` entry and swallowing the result — the documented absence-of-panic/hang/unbounded-allocation contract, honestly stated as "nothing else." The wiring story checks out completely: all four `fuzz/fuzz_targets/fuzz_playlist_*.rs` libFuzzer shims exist and call exactly these harnesses; the registry gate (baseline #18, PASS — 35 targets, diff empty) mechanically enforces bins==files==README==harness-fn plus non-empty seed corpora; and the "proven on stable meanwhile" claim is backed by executed deterministic sweeps at three levels — this module's own 2 tests, the per-format suites' fuzz-lite runs (random bytes + mutations, all four formats), and the streaming netfuzz sweep that additionally drives `fuzz_playlist_m3u` through the HLS interpretation stack. The single defect is a doc inaccuracy: the module header claims each entry pass includes "BOM/encoding sniff" — BOM tolerance is real (one leading U+FEFF stripped in every text parser), but **no format sniffs encodings**: all four `parse_*_bytes` are `from_utf8`-or-typed-`InvalidUtf8`, and m3u's own docs state "the parser never guesses an encoding." Half the parenthetical describes machinery that does not exist.

## 2. What actually works (verified)
— Both in-module tests executed ok in baseline: `fuzz::tests::empty_and_marker_inputs_survive` (:4262), `fuzz::tests::hostile_wide_inputs_survive` (:4267) — 5 marker inputs × 4 formats, plus 0xff/0x00 4 KiB floods × 4 formats, a 128-entry M3U directive flood, and a 128-track XSPF flood. Any parser panic propagates and fails the test.
— The deeper per-format sweeps this module's docs lean on all ran green: `m3u_suite` `fuzz_lite_random_bytes_10k_no_panic` + `fuzz_lite_mutations_of_valid_playlist_5k`; `pls_suite` `fuzz_lite_random_bytes_no_panic` + `fuzz_lite_mutations_of_valid_pls_no_panic` (:4393); `xspf_suite` random + mutations (:4414); `cue_suite` `fuzz_lite_random_bytes_10k_no_panic` + `fuzz_lite_mutations_of_valid_sheet_5k` — so every harness's parser is exercised far beyond the in-module smoke set.
— All four harness entry points resolve: `parse_bytes` (lib.rs:83), `parse_pls_bytes` (lib.rs:90), `parse_xspf_bytes` (lib.rs:96), `parse_cue_bytes` (cue.rs:844, audited this session).
— The cargo-fuzz shims exist and are shaped exactly as documented (read `fuzz_playlist_cue.rs`: `fuzz_target!(\|data: &[u8]\| { playlists::fuzz::fuzz_playlist_cue(data); })`), the registry gate script mechanically verifies the shim→harness call for every target, and baseline #18 records it PASS with an empty diff.

## 3. Findings

### F-001 [S4]
Where: crates/playlists/src/fuzz.rs:6-8
Code:
```
//! One call = one full untrusted-playlist pass through the format's
//! `parse_bytes` entry (BOM/encoding sniff included):
//!
```
What is wrong: "encoding sniff included" is false for every format. All four byte entries are strict UTF-8: m3u `parse_bytes` = `std::str::from_utf8(input).map_err(|_| ParseError::InvalidUtf8)?` (m3u.rs, verified); XSPF's refusal says outright "encoding is the caller's policy" (xspf.rs:88); m3u's module docs state "the parser never guesses an encoding" (m3u.rs:68). Only the BOM half of the parenthetical is real (one leading U+FEFF stripped in m3u/xspf/cue).
Why it matters: A contributor reading the harness contract to decide what the fuzz corpus should contain (e.g., "code-page-1252 .m3u files are sniffed, so no need for an encoding corpus") is misled: such inputs are typed refusals, never parsed. The doc describes a robustness property the parsers deliberately do not have (a documented design decision elsewhere in the same crate).
What correct looks like: "(BOM tolerance included; encoding is the caller's policy — non-UTF-8 is a typed refusal)".
Confidence: HIGH — Verification: read of all four `parse_*_bytes` bodies + the two contradicting doc lines in m3u.rs/xspf.rs.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_playlist_m3u | 35-37 | REAL | — | One-line pass-through; result swallowed by contract. Executed via fuzz::tests, m3u_suite fuzz-lite, and netfuzz_sweep:351. |
| fuzz_playlist_pls | 40-42 | REAL | — | Same shape; pls_suite fuzz-lite green. |
| fuzz_playlist_xspf | 45-47 | REAL | — | Same shape; xspf_suite fuzz-lite green. |
| fuzz_playlist_cue | 50-52 | REAL | — | Same shape; cue_suite fuzz-lite green. |
| tests::empty_and_marker_inputs_survive | 59-72 | REAL | — | 5 format markers × 4 harnesses; panic = fail. |
| tests::hostile_wide_inputs_survive | 75-94 | REAL | — | 0xff/0x00 floods + structured 128-entry floods; panic = fail. `lumen-allow-literal` annotation on the test-double URL is correctly applied. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| empty_and_marker_inputs_survive | YES (for panics) | Panic propagates through the wrapper. The contract is explicitly absence-of-panic only — outcome correctness is (correctly) delegated to the per-format suites, which assert typed outcomes. |
| hostile_wide_inputs_survive | YES (for panics) | Same mechanism; the floods cover the classic parser-killer shapes (0xff = invalid UTF-8 path, 0x00 flood, directive floods). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "One call = one full untrusted-playlist pass through the format's parse_bytes entry (BOM/encoding sniff included)" | fuzz.rs:6-8 | PARTIALLY FALSE | F-001: BOM ✓, encoding sniff ✗ (all four entries are UTF-8-or-typed-refusal). |
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:14-18 | REAL | All four wrappers swallow results; per-format suites own outcome assertions (executed, baseline-cited). |
| "cargo-fuzz targets live at fuzz/fuzz_targets/fuzz_playlist_*.rs (Lead-owned nightly wiring)" | fuzz.rs:22-23 | REAL | 4 target files present; shim bodies call these harnesses; registry gate PASS (baseline #18). |
| "the harness logic is meanwhile proven on stable by the deterministic sweeps below and the crate's committed-fixture suites" | fuzz.rs:24-25 | REAL | In-module 2/2 + per-format fuzz-lite suites green (baseline lines cited in header). |
| "Parse: one pass, output ≤ O(input); line lengths and XML depth are ceiling-checked by the parsers themselves" | fuzz.rs:29-31 | REAL (cross-file) | MAX_LINE_BYTES fatal in m3u/cue (verified this session: m3u.rs:157, cue part2 :497-502); depth/bounds claims are the xspf/pls owners' to re-verify (noted below). |

## 7. Dead / duplicated / unreachable code
- No dead code: every harness has a caller (shim, sweep, or suite) and every test ran.
- No duplication: the wrappers are the registry surface; per-format mutation logic lives in the suites.
- Production-unreachable by design (fuzz infrastructure); compiled into the rlib via `pub mod fuzz` (lib.rs:113).

## 8. Dependencies & flags
Crate-internal only (the four `parse_*_bytes` re-exports); no I/O, no threads, no unsafe, no feature gates. Tests use fixed byte vectors (no RNG — deterministic by design).

## 9. Open questions for the next agent
- XSPF/PLS owners (xspf.rs part2 / pls.rs if assigned): the module doc's "XML depth … ceiling-checked" claim is yours to pin — I verified the BOM/UTF-8 halves only.
- Lead/nightly wiring owner: the shims compile-on-stable claim ("cargo check in fuzz/") has no baseline evidence (fuzz/ dir not built in any recorded run); the registry gate checks file presence, not compilation. REQUEST-ORCH-RUN (informational, low priority): `cargo check --manifest-path fuzz/Cargo.toml` to pin the stable-compile claim.
