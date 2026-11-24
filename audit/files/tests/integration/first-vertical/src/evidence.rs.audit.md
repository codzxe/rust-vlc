# AUDIT: tests/integration/first-vertical/src/evidence.rs
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 392 / 10 fns (7 pub) + 3 tests |
| File verdict | REAL (library + executed unit tests, all passing in baseline; vectors independently re-verified) |
| Reachable from shipped entry point? | NO (integration-suite library, used by the evidence tests + regen writer) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |

## 1. One-paragraph verdict
The single evidence-assembly point (transcripts, final frame, ledger) with an
honest mock/PENDING labeling story and a refusal-to-write-non-green regen. Its
unit tests ran green in the baseline; the ISO-8601 vectors are correct — I
re-derived all four with python and they match exactly. Two real defects: a
corrupted short_sha fallback, and the ledger's per-run `started_at`/run_id being
per-run values inside a byte-compared artifact (a determinism smell that bites
the currency suite).

## 2. What actually works (verified)
- Baseline (cargo-test-workspace.txt:2729-2734): `test evidence::tests::
  artifact_paths_are_stable ... ok / iso_utc_known_instants ... ok /
  iso_utc_monotonic ... ok` (the 10 lib unittests).
- iso_utc vectors re-verified with python: 0 -> 1970-01-01T00:00:00Z,
  951825600 -> 2000-02-29T12:00:00Z, 1792368000 -> 2026-10-19T00:00:00Z,
  1735689599 -> 2024-12-31T23:59:59Z — all four match the asserts at
  evidence.rs:368-374 verbatim.
- regen_all (296-347): refuses to write when any scenario check fails
  (Err naming the failed claims) — evidence only commits green runs; verified
  by code path read.

## 3. Findings
F-001 [S3] Where: evidence.rs:88-98 + 192-195.
Code: `pub fn git_head() -> String { match ... _ => "(git unavailable)".to_string(), }` then `let short_sha = { let head = git_head(); head.chars().take(9).collect::<String>() };`
What is wrong: when git is absent the placeholder "(git unavailable)" is
truncated to "((git unav" and embedded in `run_id = "2e1-local-((git unav"` —
a corrupted identifier in the committed ledger for any tree regenerated
without git (sandboxes, source exports). What correct looks like: branch on
the placeholder explicitly. Confidence: HIGH — read + python `(git
unavailable)[:9]` == "((git unav". Blast radius: cosmetic ledger field.
F-002 [S4] Where: evidence.rs:244 `("started_at", Json::str(iso_utc_now()))`
and :230 run_id with short_sha.
What is wrong: the ledger embeds per-run wall-clock time and the regenerating
commit's sha — both change on every regen, so ledger.json byte-diff noise is
intrinsic; the currency test works around it by digest-matching only the
artifact table. Mild but real determinism smell in a byte-compared evidence
system. Confidence: HIGH — read.
F-003 [S4] Where: evidence.rs:56-59 `unwrap_or(0)` on the pre-epoch clock,
:68 `unwrap_or(0)` on the u64->i64 cast — silent zero-fallbacks for
impossible-in-practice values; the formatter then renders 1970. Acceptable,
noted per checklist 6.2. Confidence: HIGH — read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| evidence_dir / transcript_path / final_frame_path / ledger_path | 29-51 | REAL | — | CARGO_MANIFEST_DIR-relative, tested |
| iso_utc_now | 55-61 | REAL | — | wall-clock (per-run value) |
| iso_utc | 67-83 | REAL | — | civil_from_days port; vectors python-verified |
| git_head | 88-98 | REAL | S3 | placeholder corruption F-001 |
| hardware_labels | 103-185 | REAL | — | honest mock/PENDING columns; matches 2.E1 txt |
| build_ledger | 190-287 | REAL | S4 | per-run fields (F-002) |
| regen_all | 296-347 | REAL | — | refuses non-green; writes 3 artifact classes |
| final_png_of | 352-356 | REAL | — | run-to-end png extraction |
| tests::iso_utc_known_instants / _monotonic / artifact_paths_are_stable | 366-391 | REAL | — | green in baseline; vectors verified |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| iso_utc_known_instants | YES | 4 exact instants incl. leap day + year boundary (I re-derived all 4 — match) |
| iso_utc_monotonic | YES (weakly) | day-boundary ordering only |
| artifact_paths_are_stable | YES (weakly) | ends_with checks on 3 paths |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the workflow files exist ... no runner has executed them" | :175-179 | REAL | .github/workflows-disabled/ naming matches (baseline parity: 0 runs) |
| "no wall-clock sleeps anywhere, hence the byte-determinism proofs" (clock label) | :156-160 | FALSE-adjacent | the open/run-to-end transcripts drifted at baseline (evidence.rs audit F-001) — determinism does NOT hold in practice |
## 7. Dead code: none.
## 8. Dependencies: crate::harness ScenarioRun, crate::json, crate::digest, std::process (git).
## 9. Open questions: harness.rs owns normalize_run_specific (duplicate-key bug, see tests/evidence.rs audit F-002) — flag to that owner agent.
