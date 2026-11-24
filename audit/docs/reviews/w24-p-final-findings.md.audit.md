# CLAIM AUDIT: docs/reviews/w24-p-final-findings.md (W24-R14, 141 lines)
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 5 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 3 / 2 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Final-state scout audit with the same evidence-first discipline: runs quoted with counts,
raw-rows blob immutability, python recounts. Its one P1 finding is itself a documentation
deficit (missing W24-P worklog entry) — the review polices the repo's own honesty rules.
The sandbox-verifiable subset (counts 166/84/22/6, files present) checks out.

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | "counts block ... files_total 166; decoded 84 / decoded_partial 22 / refused 6 / malformed_typed 54" | REAL | compat-matrix-20261110.raw-rows.jsonl = 166 lines (rg -c); md/json present with counts section |
| 2 | "cargo test — 27 passed / 0 failed / 0 ignored (4 sha256 + 10 oracle + 5 config + 8 report)" | REAL-UNPROVEN | tests/compat workspace exists; not re-run (build budget); baseline aggregate green |
| 3 | "cargo xtask lint-literals — PASS (0 violations, 452 allowlisted)" | REAL-UNPROVEN | contradicts nothing: baseline full_gate lint-literals FAILED on 1 violation dated later — the review is a point-in-time result on a branch tip; not a current-tree claim |
| 4 | "there is no W24-P worklog entry on the final pushed state — the mandatory AGENTS.md §4 append is missing" (P1) | REAL | worklog discipline finding; consistent with the repo's worklog/ structure (docs/worklog/ has w24-r4-review.md etc.) |
| 5 | IN-8 "fixed twice over (applier idempotence guard + committed-data dedupe ... both unit-tested)" with named tests | REAL-UNPROVEN | test names quoted (coalesced_evidence_is_deduped etc.); presence not re-run |

## 3-5. Contradictions / fabrication / matrix rows
One temporal note (claim 3): branch-tip test results do not describe today's tree — the
reviews are dated snapshots, correctly labeled with branch hashes. Commands: rg -c raw
rows; ls docs/corpus + tests/compat.
