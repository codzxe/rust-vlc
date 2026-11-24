# CLAIM AUDIT: docs/perf/raw/mjpeg/speed-before-avi-video.md (raw evidence, 8 lines)
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 3 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 3 / 0 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Small raw before-benchmark: 8 avi-video files, ffmpeg vs lumen, honest n-split (lumen
runs only 5 of 8 — the 3 it cannot decode are excluded by class, recorded in the n
column, not hidden). Lumen med 0.30x ffmpeg.

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | "corpus /tmp/corpus-w7b \| files: 8 \| best-of-1 \| runners: ffmpeg, lumen" | REAL | verbatim header (cat) |
| 2 | "lumen med wall 1507.8 ms ... ratio 0.30x vs ffmpeg" | REAL | table row verbatim; raw JSON sibling (speed-before-avi-video.json) present in same dir |
| 3 | n=8 for ffmpeg vs n=5 for lumen (excluded classes recorded) | REAL | the n columns differ honestly — non-decodable lanes not counted as measurements |

## 3-5. Contradictions / fabrication / matrix rows
None / none / N/A (raw evidence). Commands: cat (whole file).
