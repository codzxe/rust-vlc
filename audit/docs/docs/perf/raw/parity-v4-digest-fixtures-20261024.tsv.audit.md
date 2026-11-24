# CLAIM AUDIT: docs/perf/raw/parity-v4-digest-fixtures-20261024.tsv
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 2 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 1 / 1 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
An 8-row fixture map (rel → /tmp/corpus-v4-smoke path) for the digest
differential. The rel names are consistent with the corpus-v4 naming
convention used by the 2026-10-23 diff report; the /tmp corpus is absent
from this sandbox so the digests themselves are not re-executable.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | 8 fixtures: 2 dirty (riff-size-lie, dup-range) + 6 media AVI lanes | whole file | REAL | 8 rows exactly; naming matches corpus classes (dirty/* + media/*-avi) |
| 2 | these were the digested fixtures of the parity-v4 smoke | implied | REAL-UNPROVEN | /tmp/corpus-v4-smoke absent; not re-runnable |

## 3. Contradictions
None.

## 4. Fabrication / inflation indicators
None — includes DIRTY (corrupt) fixtures in the digest set rather than
hiding them.

Findings: F-001 [S4] evidence paths point outside the repo (/tmp).
Commands: wc/read (exit 0).
