# CLAIM AUDIT: docs/corpus/speed-baseline-20261022.md (credit: W19-A08, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Claims extracted | 6 |
| REAL 2 / REAL-UNPROVEN 1 / SHALLOW 2 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNREACHABLE 0 / UNVERIFIED 1 |
| Internal contradictions found | 2 |

## 1. Blunt verdict
A 36-file ffmpeg-only reference table that does its job as a "denominator" honestly (the
citing worklog confirms that role), but it carries statistical garbage in the p95 column
for low-N classes (p95 BELOW median in 4 rows) and its "ratio vs ffmpeg" column is 100%
empty — the title "Speed-parity benchmark" promises parity data the file does not contain.

## 2. Claim ledger
| # | Claim (verbatim) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "corpus: `corpus-big` \| files: 36 \| best-of-2 \| runners: ffmpeg" | :3 | REAL (subset) | class column sums to exactly 36 (8+7+1+2+1+2+1+5+2+1+2+2+1+1); corpus-big-manifest-20261022.json `count: 485` — 36 is a subset; worklog archive-2026-10.md:7827 confirms role: "ffmpeg reference per class ... the denominator table every future parity run compares against" |
| 2 | per-class med walls/fps/realtime | :7-20 | REAL-UNPROVEN | worklog:7827 quotes avi-video "med 97.6 ms/1311 fps", drift-av "190x realtime", vfr-av "37x" — all match doc rows byte-exactly; re-run impossible (485-file corpus media off-repo) |
| 3 | "p95 wall" values | :10,12,15,17 | SHALLOW | p95 BELOW median in 4 of 14 rows: drift-av 282.2 < med 405.7; fmp4 65.9 < 73.2; multivideo 60.4 < 65.7; ogv 56.4 < 75.6 — statistically meaningless at n=1-2 per class; publishing them implies a distribution the sample cannot support |
| 4 | "ratio vs ffmpeg" column | :5-20 | SHALLOW | every row is "-"; only the ffmpeg side exists — no Lumen numbers in this file |
| 5 | title "Speed-parity benchmark (decode-to-null)" | :1 | SHALLOW | nothing in the file is a parity (two-sided) measurement; PERF-VS-VLC.md:18 honestly discloses the caveat: "MEASURED — but against ffmpeg 7.1.5, not VLC, on a shared contended 2-core sandbox" |
| 6 | 14 class names correspond to corpus-big classes | :7-20 | REAL | all 14 names exist in corpus-big-manifest-20261022.json class census (python3 Counter over 485 entries) |

## 3. Contradictions
| This doc says | Other source says | Which is true |
|---|---|---|
| "files: 36" under corpus-big | manifest says `"count": 485` | both true — 36 is an unexplained-in-file subsample; the doc never states the sampling rule (S4 opacity) |
| "p95 wall 282.2 ms" (drift-av, n=2) | its own median 405.7 ms | p95 < median is impossible for any real distribution — the p95 column is noise at low N |

## 4. Fabrication / inflation indicators
No inflation of Lumen (no Lumen claims at all); the doc is a pure ffmpeg baseline. The
p95 column is statistical decoration, not evidence. Quotes byte-verbatim via
`sed -n '3p;10p;12p'` (output above).
