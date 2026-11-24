# CLAIM AUDIT: docs/perf/raw/mjpeg/speed-before-avi-av.md
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 1 benchmark table (raw artifact) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 0 / 0 / 0 / 0 / 1 (raw data record) / 0 / 1 (corpus /tmp/corpus-w7b absent; not re-runnable) |

## 1. Blunt verdict
Raw benchmark artifact ("before" lane of the mjpeg-simd card). Internal arithmetic
checks out (ratio 1.34x = 244.3/182.9 ms); nothing in-tree can re-run it — corpus
path `/tmp/corpus-w7b` does not exist (checked).

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | ffmpeg med 244.3 ms vs lumen med 182.9 ms → "ratio vs ffmpeg 1.34x" | table | REAL (arithmetic) | python3: 244.3/182.9 = 1.3358 → 1.34x correct |
| 2 | corpus /tmp/corpus-w7b, 8 files, best-of-1, runners ffmpeg+lumen | header | UNVERIFIED | ls /tmp/corpus-w7b → absent; no re-run possible; no script in-tree pins this protocol for avi-av (speed_bench.py exists but targets its own corpus) |
| 3 | fps/realtime columns (286.1 vs 349.1 fps; 21.73x vs 26.16x realtime) | table | UNVERIFIED | per-file frames/lengths not in the artifact; med-fps ratio (1.22) differs from wall ratio (1.34) — consistent with heterogeneous files, not independently checkable |

## 3. Contradictions
None within the artifact. Note: the fps-column ratio (1.22x) is materially lower than
the wall ratio (1.34x) the headline uses — a doc citing only "1.34x" would overstate
the fps-parity gain; this file is raw data, so no verdict beyond noting the spread.

## 4. Fabrication / inflation indicators
None — but also no evidence: best-of-1 (single pass, no repetition), 8 files, source
corpus gone. Any downstream doc quoting "1.34x faster than ffmpeg" from this inherits
a single unrepeatable measurement.
