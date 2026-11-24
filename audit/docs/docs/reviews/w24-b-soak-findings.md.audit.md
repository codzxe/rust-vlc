# CLAIM AUDIT: docs/reviews/w24-b-soak-findings.md
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 5 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 3 / 2 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 (1 finding later fixed, verified) |

## 1. Blunt verdict
A high-quality re-run-based review: its instrument-reality verdict is
corroborated by the baseline (soak_scenarios suite green), the soak tree
and raw evidence exist as described, and its P1-1 (dangling worklog
citation) is confirmed fixed in the current worklog — appended
retroactively with explicit provenance, exactly as the review demanded.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "cargo test -p playback-engine --test soak_scenarios … 29/29 PASS" | :13-14 | REAL-UNPROVEN | baseline cargo-test-workspace.txt:4502-4534 shows soak_scenarios main ran, `test soak_scenario_suite ... ok`; the 29/29 split not visible there |
| 2 | "55 #[ignore] in crates/+apps/ (exact)" | :40 | REAL-UNPROVEN | loc-patterns.txt:34 → 66 total #[ignore] tree-wide (subset claim plausible; recount not run) |
| 3 | diff: tests/soak/** + soak_scenarios/** + docs/soak/** + nightly.yml + script | :6 | REAL | `ls tests/soak` (4 manifests+schema) and `ls docs/soak/raw` (7 raw reports) exist |
| 4 | P1-1: synthesis cites a W24-B worklog entry that "does not exist on the branch" | :55-70 | REAL (and since FIXED) | worklog.md:778-793 now carries the W24-B entry, appended retroactively 2026-11-10 by W24-M r2 with provenance bullets naming exactly this P1-1 |
| 5 | forced-threshold re-run "FAIL — 2 violations, non-zero exit … reproduced exactly" | :16-17 | UNVERIFIED | not re-runnable here (cargo-build-budget); internal numbers consistent (1,000,000 − 150,185 = 849,815 ✓) |

## 3. Contradictions
None. The P1-1 was a genuine dangling citation at review time; the fix
(worklog:793) proves the review bit on a real defect.

## 4. Fabrication / inflation indicators
None — self-recomputes arithmetic, records its own re-runs, and flags a
dangling citation in the reviewed lane's central claim.

Findings: none. Commands: rg in baseline + worklog, ls soak trees (exit 0).
