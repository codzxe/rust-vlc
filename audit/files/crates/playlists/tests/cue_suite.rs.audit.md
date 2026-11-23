# AUDIT: crates/playlists/tests/cue_suite.rs
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2025-06-14 |
| Lines / functions | 723 / 20 tests + 4 helpers (SplitMix64, random_input, assert_cue_invariants, fixtures) |
| File verdict | REAL (test file; 20/20 executed pass; hand-truth independently re-computed by me) |
| Reachable from shipped entry point? | N/A (test file). The parser it tests is NOT reachable: `parse_cue` has zero production callers (grep over crates/+apps/: only playlists crate internals, this suite, fuzz/); playback-engine has "no playlist model in the engine (Phase 4, 4.D1)" (events.rs:732); desktop routes `.cue` to `playlist.load` (apps/desktop/src/files.rs:54) which the engine does not serve. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/cue_suite-cb05466f7df8bf83` -> "test result: ok. 20 passed; 0 failed; 0 ignored; finished in 0.15s", exit 0 |

## 1. One-paragraph verdict
This is one of the honest test suites in the tree: three handcrafted AC sheets with window truth asserted to the nanosecond (I re-derived every hand-computed value — 13675 fr → 182_333_333_333 ns, 175 fr → 2_333_333_333 ns, 449999 fr → 5_999_986_666_667 ns, AC-3's 33/750/900/950-frame chain — all correct), dirty-world cases asserting the EXACT ordered warning vector (not just "has warnings"), the D-1 line-cap fatal + at-cap-accepted pair, and two deterministic fuzz-lite harnesses (10k random inputs + 5k mutations of a valid sheet, seeded splitmix64, structural invariants on every successful parse, plus a liveness floor `ok_count > 100` that catches a fuzz-too-weak regression). All 20 pass in 0.15s. The problem is upstream of the file: this suite is the sole evidence string for feature-matrix row `plfmt-cue` status `implemented` on three platforms, while no shipped entry point can parse a CUE sheet — the desktop shell routes `.cue` drops to a `playlist.load` command the engine does not serve.

## 2. What actually works (verified)
— `./target/debug/deps/cue_suite-cb05466f7df8bf83` -> 20 passed (incl. `fuzz_lite_random_bytes_10k_no_panic`, `fuzz_lite_mutations_of_valid_sheet_5k`, `line_cap_is_fatal_and_redacted`, `large_valid_sheet_under_200kb`), exit 0.
— Hand-truth re-computation (independent, by me): AC-1 03:02:25 = 13675 fr = 182.333..s ✓; 07:11:00 = 32325 fr = 431 s ✓; T3 pregap 65 fr = 866_666_667 ns ✓. AC-3 chain 33/750/900/950 fr ✓. Boundary 99:59:74 = 449999 fr → 5_999_986_666_667 ns (round-to-nearest) ✓. The comments' arithmetic is genuine, not generated.
Baseline cross-check: audit/runs/baseline/cargo-test-workspace.txt (cue_suite suite, ok).

## 3. Findings
F-001 [S3]
Where: crates/playlists/tests/cue_suite.rs:1-5 (module doc) + docs/feature-matrix.toml:1230
Code: `//! Card 5.O2 test suite: the three handcrafted AC sheets with\n//! hand-computed window truth, dirty-world tolerance (bad times, missing\n//! INDEX 01, BOM/CRLF/case, structural damage), the D-1 line-cap pin, and\n//! the two deterministic fuzz-lite harnesses (random bytes + mutations of\n//! a valid sheet, all inputs < 200 KB). No I/O, bounded runtime.`
What is wrong: not the suite itself — its evidence role. Matrix row `plfmt-cue` (feature-matrix.toml:1222-1231) claims `status = { windows = "implemented", linux = "implemented", macos = "implemented" }` with `evidence = "crates/playlists/src/cue.rs (card 5.O2: 3 handcrafted AC sheets vs hand-computed frame->ns truth, ...)"`. The parser is real and this suite proves it in isolation — but `parse_cue` has zero production callers: the shipped open path never parses a CUE sheet. The desktop shell's `.cue` recognition (apps/desktop/src/files.rs:54) dispatches to `playlist.load`, and the playback engine has no playlist model at all (playback-engine/src/events.rs:732 "no playlist model in the engine (Phase 4, 4.D1)"). A user dropping an album.cue gets nothing. Why it matters: "implemented" on three platforms is a library-level status dressed as a feature-level one; per the matrix's own rule (feature-matrix.toml:85 "status REQUIRES evidence resolving to an existing artifact") the artifact proves the component, not the VLC-parity capability (`vlc_name = "CUE sheets"`). What correct looks like: status `partial`/`not_started` at the product level, or wire `playlist.load` engine-side. Confidence: HIGH — Verification: ran (grep sweeps: parse_cue callers; desktop files.rs; playback-engine events.rs).
F-002 [S4]
Where: crates/playlists/tests/cue_suite.rs:481
Code: `assert_eq!(sheet.windows[0].start_ns, playlists::frames_to_ns(50));`
What is wrong: the expected value is computed by the code under test (`frames_to_ns`), a tautology-adjacent assert — if `frames_to_ns` were wrong in the same way the window builder uses it, this line cannot fail. Every other numeric assert in the suite uses hand constants; this one should be `666_666_667` (50/75 s). Blast radius: one line; `frames_to_ns` is independently pinned elsewhere in the suite by hand constants. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/playlists/tests/cue_suite.rs:653-666
Code: `fn assert_cue_invariants(sheet: &playlists::CueSheet) {\n    assert_eq!(sheet.windows.len(), sheet.tracks.len());`
What is wrong: the fuzz invariants are structural only (index alignment, file-reference range, synthetic⇒Some). No timing invariants on fuzzed parses: start_ns ordering within a FILE, `duration_ns` consistency with the next window's start, pregap ≤ start. A mutation that corrupts window arithmetic without breaking structure would pass both fuzz harnesses. Minor: the dirty-world tests do pin the arithmetic on hand-crafted shapes. Confidence: HIGH — Verification: read.

## 4. Per-function table (test file)
| Test / helper | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ac1_single_file_gapless_windows | 34-104 | REAL | — | hand truth verified by me |
| ac2_multi_file_windows_open_ended | 125-162 | REAL | — | open-ended across FILEs pinned |
| ac3_pregap_postgap_arithmetic | 183-227 | REAL | — | synthetic vs in-file pregap pinned |
| bad_times_are_typed_and_tracks_degrade | 231-273 | REAL | — | exact ordered warning vector |
| more_bad_time_shapes | 275-299 | REAL | — | 5 bad shapes + 99:59:74 boundary |
| pregap_postgap_bad_times_and_contexts | 301-336 | REAL | — | TimeContext discrimination |
| bom_crlf_lowercase_and_indentation | 338-350 | REAL | — | BOM+CRLF+case |
| track_without_file_and_outside_track_commands | 352-376 | REAL | — | |
| malformed_numbers_and_missing_arguments | 378-418 | REAL | — | 11-warning exact vector |
| unknown_command_and_flags | 420-443 | REAL | — | unknown flags skipped, known kept |
| non_monotonic_index_empties_window | 445-465 | REAL | — | emptied-not-negative pinned |
| bad_pregap_arithmetic_is_typed | 467-482 | REAL | S4 | F-002 tautology-adjacent assert |
| quoted_unquoted_and_verbatim_values | 484-510 | REAL | — | no-escape verbatim semantics pinned |
| empty_and_minimal_inputs | 512-532 | REAL | — | |
| garbage_bytes_are_typed_invalid_utf8 | 534-541 | REAL | — | |
| line_cap_is_fatal_and_redacted | 543-559 | REAL | — | exact Display string, at-cap accepted |
| lone_cr_is_one_weird_line | 561-569 | REAL | — | CR-only documented behavior |
| large_valid_sheet_under_200kb | 573-599 | REAL | — | 4600 tracks, equal-start joins |
| SplitMix64 / random_input / INTERESTING | 603-650 | REAL | — | deterministic, biased-to-valid corpus |
| assert_cue_invariants | 653-666 | REAL | S4 | F-003 structural-only |
| fuzz_lite_random_bytes_10k_no_panic | 668-681 | REAL | — | liveness floor >100 |
| fuzz_lite_mutations_of_valid_sheet_5k | 683-723 | REAL | — | byte-sub/del/ins mutations |

## 5. Test honesty
All 20 tests would fail if the corresponding code were broken: values are hand constants (except F-002's one line), warnings asserted as exact ordered vectors, boundaries tested on both sides of the cap. The fuzz harnesses assert no-panic + invariants + a liveness floor — the correct shape for fuzz-lite. No `#[ignore]`, no tautological `assert!(true)` shapes, no mock of the thing tested (the suite tests the real `parse_cue`/`parse_cue_bytes`).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Parse CUE sheets (REM/INDEX/gaps) with album-oriented gapless handoff" — status implemented (win/linux/macos) | docs/feature-matrix.toml:1225-1231 | SHALLOW (component-true, product-dead) | suite executed (20 pass); parse_cue zero production callers; engine has no playlist model (events.rs:732) — F-001 |
| "3 handcrafted AC sheets vs hand-computed frame->ns truth, zero-gap joins pinned exactly, 13 typed warnings, 4600-track stress" (evidence string) | docs/feature-matrix.toml:1230 | REAL | all present in this file; math re-derived by me |
| "pregap excluded from duration, exposed separately" | docs/feature-matrix.toml:1231 | REAL | ac1/ac3 assert pregap_ns separate from duration arithmetic |

## 7. Dead / duplicated / unreachable code
None in the file. The tested parser (`crates/playlists/src/cue.rs`) is production-dead today (F-001) — audited by whoever owns cue.rs; this audit covers the suite only.

## 8. Dependencies & flags
Dev-deps only: `playlists`, std. No I/O (module doc claim "No I/O" holds — all inputs are in-memory strings/byte vectors). Deterministic seeds — reproducible failures as claimed.

## 9. Open questions for the next agent
- cue.rs (parser) audit: confirm the 13-warning vocabulary and windows builder against this suite's expectations; the suite is strong enough to be a mutation-testing oracle.
- If/when `playlist.load` lands engine-side, the matrix row should move from library-implemented to feature-implemented with a NEW evidence string (end-to-end drop→play), not this suite.
