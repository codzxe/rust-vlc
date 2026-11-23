# AUDIT: crates/playlists/tests/pls_suite.rs
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-12 |
| Lines / functions | 386 / 14 tests + 2 fuzz helpers (SplitMix64, random_input) |
| File verdict | REAL (strong suite, one dishonest scoping choice) |
| Reachable from shipped entry point? | N/A — test artifact; the parser under test (crates/playlists/src/pls.rs) is UNREACHABLE from shipped binaries (see m3u.rs audit: only streaming+fuzz depend on playlists; CLI refuses playlist.*) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | full read, rg (pls.rs title/location handling, to_m3u_string, bridge doc), sed -n, baseline citation |

## 1. One-paragraph verdict
An honest, high-quality test suite: full structural `Entry` equality against spec-shaped goldens, an exact 10-warning pinned vector over the dirty corpus (line numbers and variant order), u64-overflow inputs, exact-at-cap/over-cap line boundaries with an exact Display-string redaction pin, UTF-8 garbage typing, and two real deterministic fuzz-lite harnesses (1000 random + 200 mutated inputs) with runtime byte-budget counters that match the documented ~98 KiB budget. All 14 tests pass in the baseline. The one real defect is a scoping lie in the cross-format test: `pls_bridges_losslessly_through_m3u` runs only on the two clean fixtures and quietly excludes the dirty corpus, whose kept-empty-location entry (which this same suite pins at L157: `location == ""`, "empty File3 kept") cannot survive an M3U roundtrip — `to_m3u_string` writes an empty location line, and the M3U parser skips blank lines (m3u.rs L169-171), so the entry silently vanishes. The pls.rs:52 doc claim "a parsed PLS therefore bridges losslessly through `Playlist::to_m3u_string`" is FALSE for that legal parsed shape, and the test's fixture selection hides the counterexample.

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt` (PACKAGE playlists): `tests/pls_suite.rs ... 14 passed; 0 failed; 0 ignored`.
— verified by read: `dirty_fixture_degrades_honestly` (L130-160) pins an exact ordered 10-warning vector — it fails on nearly any parser regression; `over_cap_line_is_a_typed_error` (L227-249) pins `(line, length) == (2, 65542)` and the exact redacted message; the fuzz harnesses assert generated-bytes budgets (`generated < 100_000`) matching the header's documented budget.

## 3. Findings
F-001 [S3]
Where: crates/playlists/tests/pls_suite.rs:254-266 (+ crates/playlists/src/pls.rs:52)
Code:
```
    fn pls_bridges_losslessly_through_m3u() {
        // PLS entries carry exactly the fields M3U models, so the canonical
        // M3U writer must round-trip them (documented in pls.rs module docs).
        for src in [BASIC, SPARSE] {
```
What is wrong: the test name and the pls.rs:52 claim ("a parsed PLS therefore bridges losslessly through [`Playlist::to_m3u_string`]") promise losslessness for any parsed PLS, but the loop runs only the two clean fixtures. DIRTY is excluded, and DIRTY contains the counterexample this suite itself pins: `File3=` keeps an entry with `location == ""` (L157 `assert_eq!(d.playlist.entries[1].location, "", "empty File3 kept")`). `to_m3u_string` → m3u `write` emits the location line as `"" + '\n'` (m3u.rs L381-382), and the M3U parser skips blank lines (m3u.rs L169-171 `if trimmed.is_empty() { continue; }`) — the entry disappears, so `parse(to_m3u_string(parse_pls(DIRTY))) != parse_pls(DIRTY)`. The empty-title case is safe (pls.rs:317 normalizes empty titles to `None`), the empty-location case is not. Why it matters: the "lossless bridge" is the documented justification for PLS mapping onto the shared vocabulary; it is false for a damage shape the parser deliberately supports (pls.rs:36 "the entry is kept with an empty location"). What correct looks like: either exclude empty locations from the lossless claim (document + assert the documented asymmetry like xspf does — xspf_suite.rs has a `documented_asymmetry` test), or drop empty-location entries at the bridge. Confidence: HIGH — Verification: read of both parsers + the write path; no execution (cargo-build-budget), mutation reasoning only.

F-002 [S4]
Where: crates/playlists/tests/pls_suite.rs:146-149
Code:
```
            PlsWarning::EntryCountMismatch {
                declared: 99,
                found: 3
            },
```
What is wrong: `EntryCountMismatch` is tested only in the declared > found direction. The declared < found direction (a lying-low count) is untested in this suite — the parser presumably still warns, but no test would catch a regression where a smaller declared count truncates or misclassifies. Confidence: MEDIUM — Verification: read; pls.rs unit tests not examined (not my file).

F-003 [S4]
Where: crates/playlists/tests/pls_suite.rs:330, 381
Code:
```
    assert!(ok_count > 200, "fuzz too weak: only {ok_count} parses");
```
What is wrong: the liveness thresholds (`> 200` of 1000, `> 100` of 200) are the only semantic checks in the fuzz tests; everything else is no-panic + loose sanity bounds (`playlist.len() <= buf.len()` is trivially true since each entry needs ≥ 6 input bytes). The fuzz's real value is the panic hunt, which is legitimate, but the liveness bars are so low they would not detect a parser that started rejecting half the corpus. Confidence: HIGH — Verification: read.

## 4. Per-test table
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| golden_basic_spec_shaped | YES | full Entry struct equality ×2 + field pins |
| golden_sparse_indexes_case_and_comments | YES | index-order (1,5) semantics |
| crlf_bom_and_case_tolerance | YES | BOM+CRLF+case keys |
| empty_and_comment_only_inputs | YES | 5 shapes, empty + warning-free |
| pre_section_keys_parse_with_one_warning | YES | exact single-warning vector |
| dirty_fixture_degrades_honestly | YES | exact 10-warning ordered vector + entry survival + last-wins |
| non_finite_and_negative_lengths | YES | -1 preserved; inf/NaN/1e999/"" typed |
| overflowing_index_and_count | YES | u64-overflow typed, empty playlist |
| value_with_equals_and_unknown_keys | YES | first-'=' split, unknown keys ignored |
| garbage_bytes_are_typed_invalid_utf8 | YES | variant match |
| over_cap_line_is_a_typed_error | YES | (line,length) pin + exact Display + at-cap accepted |
| pls_bridges_losslessly_through_m3u | PARTIAL | strong on clean fixtures; hides the empty-location counterexample (F-001) |
| fuzz_lite_random_bytes_no_panic | YES (panic class) | 1000 inputs, budget-asserted |
| fuzz_lite_mutations_of_valid_pls_no_panic | YES (panic class) | 200 mutated inputs, budget-asserted |

## 5. Test honesty
Covered in the per-test table. The suite as a whole is far above average: exact-warning-vector pinning and boundary/Display pins are the strongest patterns in the playlists crate.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a parsed PLS therefore bridges losslessly through Playlist::to_m3u_string" | crates/playlists/src/pls.rs:52 | FALSE (for the kept-empty-location shape) | F-001 chain: pls.rs:36/306 keeps empty location → m3u.rs:381-382 writes blank line → m3u.rs:169-171 skips it |
| "this file generates at most ~98 KiB ... pinned by runtime byte counters" | pls_suite.rs:7-10 | REAL | L332-334/L382-384 `generated < 100_000` asserts |
| "CI-able: no I/O beyond compile-time include_str!" | pls_suite.rs:5 | REAL | only include_str! fixtures + in-process fuzz |

## 7. Dead / duplicated / unreachable code
None in the file. (The parser under test is unreachable from shipped binaries — cross-reference m3u.rs audit F-001.)

## 8. Dependencies & flags
Dev-only: `playlists` crate itself (via workspace dev-dependency). No features. No I/O.

## 9. Open questions for the next agent
- Does pls.rs's own unit-test module cover EntryCountMismatch in the declared<found direction (F-002)?
- The xspf suite documents its bridge asymmetry explicitly (xspf_suite.rs `documented_asymmetry`); pls.rs should get the same treatment for empty locations — who owns that doc fix?
