# CLAIM AUDIT: docs/perf/raw/speed-v4-20261024-avi-av.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 2 |
| REAL 1 / REAL-UNPROVEN 1 | |

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | "runners: ffmpeg, lumen ... avi-av 8 files: ffmpeg med wall 81.7ms / lumen 73.6ms ... ratio 1.11x" | REAL-UNPROVEN (dated) | corpus /tmp/corpus-w7b ephemeral; the table structure and the derived-ratio math (81.7/73.6 = 1.11) check out |
| 2 | File is the human summary of the same campaign as environment.json/seek.csv | REAL | provenance chain present in the sibling files (same corpus string, same date) |

## 1. Blunt verdict
Speed-parity summary showing lumen 11% faster than ffmpeg on the AVI A/V class — a favorable number presented with the full raw complement (medians + p95). The p95 honesty: lumen's p95 (482ms) is WORSE than ffmpeg's (219ms) — both columns committed.
## 3. Contradictions: none. ## 4. Indicators: none.
