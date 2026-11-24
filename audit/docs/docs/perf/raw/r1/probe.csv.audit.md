# DATA AUDIT: docs/perf/raw/r1/probe.csv
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A09) |
| Type | CSV, 61 lines (header + 60 rows), 8 columns |
| Numeric verification | recomputed in python (csv + statistics) |
| Producer/consumer | tools/perf/baseline.py probe lane (`--raw-dir .../raw/r1`); cited by docs/perf/breadth.md:122,214 |

## 1. Blunt verdict
Clean refusal-lane measurement: 6 fixtures × 10 fresh sessions, every row
`typed-refusal` / exit ok. The medians I recomputed (2.418/2.442/2.49/2.503/
2.507/2.531 ms) bracket EXACTLY the "2.42–2.53 ms" range breadth.md:122
publishes for these lanes. Nothing invented.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | 6 lanes: 0085 ogg / 0042 wav / 0059 aac / 0094 m4a / 0034 mkv / 0014 mkv | breadth.md:122 | REAL | Counter: exactly those 6 fixtures, 10 rows each |
| 2 | "open → typed refusal `probe.unsupported`" | breadth.md:122 | REAL | outcome=typed-refusal ×60, exit=ok ×60 (no crash masquerading) |
| 3 | "N=10 fresh sessions each" | breadth.md:22 | REAL | run numbers 1..10 per fixture, no gaps |
| 4 | "2.42–2.53 ms" family range | breadth.md:122 | REAL | recomputed medians 2.418…2.531 — matches to the printed precision |
| 5 | "probe.csv (60 rows: 6 refusal lanes)" | breadth.md:214 | REAL | wc -l = 61 (header+60) |
| 6 | column `media_info_ms` populated | header | MISLEADING-EMPTY | 60/60 rows carry an empty value — declared measurement never taken (refusal never reaches media.info); header advertises a column with zero data (S4 hygiene) |

## 3. Contradictions
None numeric. The empty media_info_ms column vs its header presence is the
only blemish.

## 4. Fabrication / inflation indicators
None — refusal latencies are honest negative results. Findings: S4 x1
(never-populated column).
