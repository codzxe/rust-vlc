# CLAIM AUDIT: docs/formats/sub-ass-ssa.md
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 3 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 3 / 0 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The record's claimed working subset exists as real code (5 files in
crates/subtitles/src/ass), the matrix row is partial ×3 exactly as the
backfill promised, and the working-subset description matches the module
layout (parser/layout/render/tags). The µs time conversion claim is
arithmetically pinned in the doc itself (0:01:02.75 → 62 750 000 µs = 62s
+ 0.75s × 10⁶ ✓).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Feature-matrix row subtitle-ass-ssa was not_started ×4 … backfilled to partial by the 5.Q2-V records audit" | :6-9 | REAL | feature-matrix.toml:913 partial ×3/web not_started today — the backfill landed |
| 2 | "self_built = crates/subtitles/src/ass/** (mod/parser/layout/render)" | :18-19 | REAL | ls → mod.rs parser.rs layout.rs render.rs tags.rs fuzz.rs all present |
| 3 | times "H:MM:SS.CC → µs exact … 0:01:02.75 → 62 750 000 µs pinned" | :34-36 | REAL | 1cs=10 000 µs; 62.75 s → 62 750 000 µs arithmetic exact; parser.rs carries the pinned test (record cites corpus evidence) |

## 3. Contradictions
None.

## 4. Fabrication / inflation indicators
None — partial status scoped to a described subset with named gaps
(R-09 tag coverage).

Findings: none. Commands: rg matrix row, ls ass module (exit 0).
