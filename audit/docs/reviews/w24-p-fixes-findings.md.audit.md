# CLAIM AUDIT: docs/reviews/w24-p-fixes-findings.md (W24-R10 scout report, 209 lines)
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 4 / 2 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
A review-of-a-fix-lane with unusually high evidence density: exact commit hashes, blob
hashes, recomputed row counts, string-level edit fidelity checks, and an explicit
un-audited-worktree caveat. The verifiable-in-sandbox subset (matrix file + raw rows +
counts) checks out. Git-level claims (blob identity, commit order) are UNVERIFIED here
because agents in this audit never run git — the review's own evidence stands as the
only record.

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | "the 166-row matrix counts" | REAL | `rg -c '' docs/corpus/compat-matrix-20261022...raw-rows.jsonl` -> 166 lines exactly (compat-matrix-20261110.raw-rows.jsonl) |
| 2 | compat-matrix-20261110.md (+surgical rewrites) exists with provenance blocks | REAL | file exists (333 lines); the md/json artifacts cited all present (ls docs/corpus) |
| 3 | "cargo test in tests/compat — 11/11 pass ... 0 ignored" | REAL-UNPROVEN (this session) | tests/compat workspace exists; baseline cargo-test-workspace.txt covers aggregate (4744 pass); not re-run (build budget) |
| 4 | "clippy 0 warnings under mirrored deny lints; fmt clean" | REAL-UNPROVEN | consistent with baseline clippy CLEAN / fmt FAILED workspace-wide — fmt-clean claim is scoped to tests/compat standalone workspace, plausible, not re-run |
| 5 | "blob 469f246… identical on origin/w24/d and 022d679" (raw-rows immutability) | REAL-UNPROVEN | requires git; raw rows file present and 166 rows consistent |
| 6 | verdict headline "0 P0 / 0 P1 / 1 P2 / 2 P3" with IN-8 unaddressed | REAL | the finding structure (finish-line items) is self-consistent; P2s/P1 declared "genuinely, testably fixed" with the test names quoted (report::tests::*, 7 listed by name) |

## 3-5. Contradictions / fabrication / matrix rows
None / none — the review's own caveats (NOT pushed branch, uncommitted worktree NOT
audited, snapshot verdict) are the anti-fabrication markers. Commands: rg -c raw rows; ls
tests/compat + docs/corpus.
