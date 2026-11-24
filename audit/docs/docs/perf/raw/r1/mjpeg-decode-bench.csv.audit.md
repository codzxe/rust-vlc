# DATA AUDIT: docs/perf/raw/r1/mjpeg-decode-bench.csv (credit: W19-A07, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Rows | 20 data rows (4 fixtures x 5 runs), NO header (fixture,wall_s?,second-column) |
| Cited by | docs/perf/breadth.md:212,216 — "mjpeg-decode-bench.csv (25 rows)" + "The CSVs are the truth; the tables above are their medians" |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
Timing data whose MEDIANS CROSS-DERIVE EXACTLY to the citing doc's table (verified with
python3 statistics.median): 0052 col1 [0.902,0.927,0.885,0.970,0.888] -> 0.902 ==
breadth.md:84 "0.902 s"; 0073 -> 0.760 == :85 "0.760 s"; 0124 -> 0.708 == :86 "0.708 s";
0087 -> 0.422 == :87 "0.422 s". The "median-of-5" derivation claim is TRUE for this file.
Single defect (S4): breadth.md:212 claims "(25 rows)" but `wc -l` = 20 data rows — the
row-count citation is off by 5 (sibling probe.csv's "60 rows" claim is correct at 60 data
rows, so this is an isolated error, not a systematic one). Run-to-run spread is retained
(0.885-0.970 for 0052) — real-measurement texture, not fabricated uniformity. No
provenance header (no tool/command in-file); fixtures are off-repo corpus media, so re-run
is UNVERIFIED. Quotes byte-verbatim: `sed -n '1p'` -> "0052-avi-video.avi,0.902,0.897".
