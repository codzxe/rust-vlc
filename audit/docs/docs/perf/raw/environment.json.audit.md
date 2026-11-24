# CLAIM AUDIT: docs/perf/raw/environment.json
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 2 |
| REAL 2 | |

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | Provenance block: binary md5/bytes/mtime, cpu model, kernel, git_head a63fe225, captured 2026-10-23 | REAL | valid JSON, all fields populated; kernel string matches the same sandbox family the baseline pin records (5.10.134-013.15.kangaroo) |
| 2 | "timing_note": "shared 2-core sandbox; other wave agents may run concurrently — medians+raw are the record, single rows are not" | REAL | the honest-noise disclosure that makes the whole perf/raw corpus interpretable; 2 logical cores recorded in cpu block |

## 1. Blunt verdict
Exemplary provenance: identifies the exact binary (md5), tree (git_head), hardware (2-core Xeon), and states its own measurement-noise caveat. This is what raw evidence should look like.
## 3. Contradictions: none. ## 4. Indicators: none.
