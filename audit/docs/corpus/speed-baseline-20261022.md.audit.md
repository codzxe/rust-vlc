# CLAIM AUDIT: docs/corpus/speed-baseline-20261022.md (20 lines)
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 3 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 3 / 0 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
A pure ffmpeg-only baseline (36 files, best-of-2). Honest by omission-avoidance: the
runner column lists ONLY ffmpeg — no Lumen numbers are claimed, so it cannot inflate
Lumen speed. It is the before-numbers the later x1a lanes compare against.

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | "corpus: corpus-big | files: 36 | best-of-2 | runners: ffmpeg" | REAL | verbatim header |
| 2 | per-class ffmpeg med/p95 wall rows (avi-video 97.6 ms etc.) | REAL | table verbatim; raw sibling JSONs under docs/perf/raw/ |
| 3 | (implicit) no Lumen claims | REAL | no lumen runner/rows anywhere in the file — the honest absence |

## 3-5. Contradictions / fabrication / matrix rows
None / none / N/A (raw evidence). Commands: cat (whole file).
