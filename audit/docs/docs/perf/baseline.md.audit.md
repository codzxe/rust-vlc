# CLAIM AUDIT: docs/perf/baseline.md (236 lines, Phase-2/3 perf baseline, card 3.F1)
| Field | Value |
|---|---|
| Auditor | W19-A04 — RECOVERED by W19-R2 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 10 / 1 / 0 / 0 / 0 / 0 / 1 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Every table row in this baseline recomputes EXACTLY from the committed raw CSVs — I
re-derived medians and p95 for all 22 numeric rows (startup 4, probe 3, decode 4,
seek 8, memory 7) with the doc's own nearest-rank rule and got the printed values
cell-for-cell, including the refusal-path rows. The fixture byte sizes, the binary
size/md5, and the hardware manifest all match `raw/environment.json` byte-exact.
This is the honest-baseline pattern the repo's docs should be judged by.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Fixtures: `avi_mjpeg_pcm.avi` (425 516 B) / `avi_mjpeg_2pcm.avi` (497 160 B) / `mp4_mjpeg_pcm.mp4` (424 452 B) | :14 | REAL | `stat -c %s` on tests/fixtures/generated/valid/ -> 425516, 497160, 424452 — exact. |
| 2 | Binary 3 232 328 B, md5 `18e6b1ccdf8b16077b5f199d608856fe`, HEAD `a63fe22` | :13 | REAL-UNPROVEN (recorded) | raw/environment.json: `"bytes": 3232328, "md5": "18e6b1ccdf8b16077b5f199d608856fe", git_head "a63fe225…"` — the record matches the doc byte-for-byte; the binary itself is not in-tree to re-hash. |
| 3 | §2 startup table (help 1.138/1.350; version 0.537/0.605; cold 6.706/7.538; engine-info 0.546/0.694; N=25/25/5/25) | :52-57 | REAL | Recomputed from raw/startup.csv (80 rows): medians/p95 EXACT on all four variants; N counts match. |
| 4 | §3 probe table (1.074/2.347; 1.034/1.081; mp4 typed-refusal 0.520/0.548) | :77-81 | REAL | Recomputed from raw/probe.csv: EXACT; outcome column `typed-refusal` for mp4 rows verified in CSV. |
| 5 | §4 decode table (8 ms wall / 7 ms packet / 0.117 ms-f; 2pcm 3/3/0.150; mp4 DNF exit 65) | :98-106 | REAL | Recomputed from raw/decode.csv: medians EXACT; `status` values include `exit65` rows kept in the CSV as claimed. |
| 6 | §5 seek table (8 rows: 6.983/7.312, 6.992/7.320, 5.804/5.988, 5.810/6.002, 5.339/5.543, 5.351/5.550, 5.346/5.399, 5.352/5.407) | :125-134 | REAL | Recomputed from raw/seek.csv: all 8 medians AND p95s EXACT. |
| 7 | §6 memory table (7 rows, e.g. --help 3 224/3 348; session pcm 12 356/12 440; mp4 refusal 3 000/3 124) | :151-159 | REAL | Recomputed from raw/memory.csv: all 7 medians and maxima EXACT; `hwm_samples` column present as claimed. |
| 8 | §7 `system.get-stats` -> unknown-variant error (exit 64); `system.get-state-snapshot` -> `unsupported-for-host` (exit 0) | :177-183 | REAL | raw/stats-probe.jsonl: row 1 `exit64` + result-accept; row 2 `ok` + result.error — the two typed refusals recorded verbatim. |
| 9 | Hardware manifest (Xeon family 6/model 173, 2 cores, 4 138 564 kB, kernel 5.10.134-013.15, machine id) | :31-40 | REAL | environment.json `cpu{family:"6", logical_cores:2, model_id:"173", model_name:"Intel(R) Xeon(R) Processor"}`, `mem_total_kb: "4138564 kB"`, kernel + hostname strings byte-equal. |
| 10 | p95 = ceil(0.95·N)-th smallest sample ("with N=20 that is rank 19") | :24-26 | REAL | My recompute used exactly that rank and reproduced every printed p95 — the stated method is the one the numbers follow. |
| 11 | Harness `tools/perf/baseline.py` (Python 3.12, stdlib only) | :11 | REAL | File exists (audit/tools/perf/baseline.py.audit.md by an earlier wave cross-references it); environment.json records python 3.12.15. |
| 12 | Predecessor's relaunch fix ("mislabeled every ready-signal row `bad-response` … 24 values discarded") | :59-63 | UNVERIFIED | Session-history narrative; the discarded raw set is gone by design, so the incident is not re-derivable — but the DISCIPLINE (discard-and-rerun rather than patch numbers) is the honest direction. |

## 3. Contradictions
None. MP4 refusal rows are labeled DNF/exit-65 in every family, matching the D-018
no-MP4-demuxer posture (consistent with baseline engine-info: decoders mjpeg, pcm_s16le).

## 4. Fabrication / inflation indicators
None. The doc's posture line says "no targets claimed — a target before a baseline is
theater", every family that cannot be measured is DNF-with-evidence, and the per-frame
means are labeled "derived" rather than presented as measured TTF.
