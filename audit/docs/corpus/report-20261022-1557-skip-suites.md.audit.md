# CLAIM AUDIT: docs/corpus/report-20261022-1557-skip-suites.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 4 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 3 / 1 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Same honest skip-suites pattern: 310 files, oracle-side validation run, suite sections
intentionally empty with the reason stated per-section ("suite skipped ... no rows;
section intentionally empty").

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | "the Rust suites were NOT driven (oracle-side validation run): suite sections carry no rows by construction" | REAL | verbatim header L5-6 |
| 2 | "files: 310 (1095 MiB clean), rejected draws: 1" + seed 6311... | REAL-UNPROVEN | manifest provenance; corpus not regenerated here |
| 3 | composition table (audio-only=38, avi-video=87, ...) | REAL | generator-intent join, self-consistent totals = 310 |
| 4 | "suite exit codes: probe=skipped demux=skipped ..." | REAL | verbatim L7 |

## 3-5. Contradictions / fabrication / matrix rows
None / none / N/A. Commands: head -14.
