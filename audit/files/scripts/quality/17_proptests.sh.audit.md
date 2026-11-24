# AUDIT: scripts/quality/17_proptests.sh
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Date | 2026-11-24 |
| Lines / functions | 61 / 3 phases (17a discovery count, 17b candidate scan, 17c catalogue) |
| File verdict | REAL (reporter; honest about adding nothing) |
| Reachable from shipped entry point? | YES — scripts/quality/99_run_all.sh:37 `J 17_proptests.sh` (static tier, every PR per audit.yml:80). Not part of the release binary lane. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | grep proptest/quickcheck macros over crates/apps/tools/tests -> 0 hits; lock grep -> no proptest/quickcheck crate; dry re-implementation of the candidate scanner -> 157 candidates |

## 1. One-paragraph verdict
A clean, honest discovery reporter. Its headline claim — "the repo has zero property-based tests" — is true today (my grep over the same paths: 0 matches; no proptest/quickcheck crate in the lock), the candidate scanner is a real regex inventory (re-run dry: 157 invariant-rich pure functions found), and the 15-property catalogue is correctly labeled as what the fix program SHOULD add, not what exists. The only real criticism is structural: a quality script whose entire value is a prose wish-list pinned in a heredoc — nothing in this script can fail, so as a gate it is pure documentation.

## 2. What actually works (verified)
— verified by execution (read-only): `grep -rn --include=*.rs -E '#\[proptest\]|proptest!|quickcheck!' crates apps tools tests | wc -l` -> 0 — the script's zero-proptest premise holds on the current tree.
— verified by dry re-run of the 17b scanner logic (python, no writes): 157 candidate `pub fn`s match the interesting-keyword + no-&mut/dyn/impl filters — the catalogue generation works and is non-empty.
— verified by lock grep: no proptest/quickcheck packages — consistent with 0 property tests.
— bash -n clean (trivial).

## 3. Findings
F-001 [S4]
Where: scripts/quality/17_proptests.sh:43-60
Code:
```
head1 "17c. catalogue (the 15 properties the fix program should add)"
cat <<'TXT'
 1 media-clock        monotonic under any callback sequence
```
What is wrong: nothing functionally — but the script's exit behavior is a constant 0 with its only content being discovery output plus a static heredoc catalogue. As a tier step it can never signal a problem; the real finding (F-T17-01: zero property tests) is noted in prose at :10-11 rather than recorded into the tier's FAILED ledger or a gates list. Why it matters: the property-test gap is a repo-level weakness that this reporter reduces to a NOTE line. What correct looks like: after the zero-test note, append a machine-readable finding entry (not-run.txt style) or make the tier's report.py classify it. Confidence: HIGH — Verification: read + executed the discovery grep.

## 4. Per-function table (file <100 LOC; phases listed)
| Phase | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| 17a discovery | 6-12 | REAL | — | grep count; zero-note honest (verified 0 today) |
| 17b candidate scan | 14-41 | REAL | — | writes $OUT/proptest-candidates.txt; regex scanner verified equivalent (157 hits) |
| 17c catalogue | 43-60 | REAL (prose) | S4 | 15 properties = PAPER wishlist, self-labeled "should add" |

## 5. Test honesty
The script tests nothing and says so ("Discovery only: does not add tests", :2). The repo-level honesty it exposes: zero property-based tests in a media-timeline codebase with 157 invariant-rich candidates — a genuine coverage gap, honestly surfaced rather than hidden.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NOTE: the repo has zero property-based tests (matches the documented 'no-proptest status quo')" | 17_proptests.sh:9-12 | REAL | my grep -> 0 matches; no proptest/quickcheck in Cargo.lock |

## 7. Dead / duplicated / unreachable code
None. Catalogue text is static by design.

## 8. Dependencies & flags
grep + python via common.sh; writes $OUT/proptest-candidates.txt (docs/audit/reports — do-not-execute class for auditors).

## 9. Open questions for the next agent
- Were any of the 15 catalogue properties ever implemented since this script was written? My grep says no proptest/quickcheck macros exist — so 0/15 as of 2026-11-24. The fix program it names has not started.
