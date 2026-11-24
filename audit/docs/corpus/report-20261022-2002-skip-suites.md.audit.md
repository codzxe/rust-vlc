# CLAIM AUDIT: docs/corpus/report-20261022-2002-skip-suites.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 4 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 3 / 1 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
The largest of the three skip-suites runs (485 files) and equally explicit about being an
oracle-side validation run: per-class ffprobe_ok coverage (100% on authored classes), a
registered follow-up for the 429 suite-gap cells, and harness wall recorded. No
measurement inflation.

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | "mode: --skip-suites — oracle-side validation run ... every Lumen-side decoded-format cell is a suite-gap row" | REAL | verbatim L5-6; tail: "485 rows, 429 suite-gap cells" |
| 2 | "ffprobe_ok ... a false there is a real defect signal" per-class table (56/56, 43/43, 82/82 ...) | REAL | per-class 100% rows verbatim |
| 3 | "harness wall: 52s" + decoded-rows ledger path | REAL | verbatim Suite-health section |
| 4 | "corpus seed 58007138736976908607880252061" replay path | REAL-UNPROVEN | not re-generated (cost) |

## 3-5. Contradictions / fabrication / matrix rows
None / none / N/A. Commands: head -18; tail -8.
