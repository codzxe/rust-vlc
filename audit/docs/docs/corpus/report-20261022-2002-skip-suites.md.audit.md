# CLAIM AUDIT: docs/corpus/report-20261022-2002-skip-suites.md
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R3; replaces a prior audit at the wrong path audit/docs/corpus/… missing the `docs/` segment) |
| Claims extracted | 8 |
| REAL 6 (internal arithmetic, all recomputed) / UNVERIFIED 2 (absent ledger + oracle-side session values) | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The cleanest kind of skipped-suites report: an oracle-side validation run whose suite
sections carry explicit "0/0 — no rows by construction" rows instead of silently
reused stale numbers (contrast: the 15:13 sibling reused persisted TSVs). Every
arithmetic identity recomputes EXACTLY (485 composition = per-class table; pix/sample
families 75/52; mismatch listings 10+65=75, 10+42=52; drift 8+4=12). The per-case
ledger lives under corpus-big/_harness — absent from this snapshot.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "files: 485 (1292 MiB clean), rejected draws: 19" | :4 | REAL | python sum of the 24-class composition line = 485 EXACT |
| 2 | per-class coverage table files column | :13-38 | REAL | 24-row table equals the composition map entry-for-entry (python dict compare: True); absent surfaces listed as zero-class rows, not dropped |
| 3 | "the Rust suites were NOT driven ... the suite sections below carry no rows by construction" | :6-7 | REAL | every suite metric printed as **0/0** with "section intentionally empty" banners (:42, :53, :73, :83); suite health :162 rc=skipped x4 |
| 4 | "mjpeg AVIs with oracle dims 75/76 ... pix_fmt families yuv444=29, yuv420=25, yuv422=21" | :100-103 | REAL | families sum 29+25+21 = 75 = the 75/76 population; sample families 26+11+9+6=52; channel layouts 16+14+8+6+5+3=52; sample rates sum=52 — all recomputed EXACT |
| 5 | mismatch listing "10 video rows + 65 more / 10 audio + 42 more" | :124-145 | REAL | 10+65=75 video (=75 dims population), 10+42=52 audio (=52 sample population) |
| 6 | "12 file(s) where oracle facts disagree with generator ... 8 listed + 4 more" | :149-158 | REAL | 8+4=12 EXACT; dims-vs-pix drift split with named files |
| 7 | "decoded-rows ledger: /home/z/my-project/corpus-big/_harness/decoded-rows.tsv (485 rows, 429 suite-gap cells)" | :164 | UNVERIFIED | ledger absent from this snapshot (corpus-big/ not present); 429/485x1 cell-space arithmetic not checkable |
| 8 | "harness wall: 52s" | :163 | REAL-UNPROVEN | oracle-side session claim; plausible for a 485-file ffprobe sweep with zero suites |

## 3. Contradictions
None. Notably honest details: ffprobe_ok "-" markers for dirty/garbage classes (:11) and
the generator-defect framing ("a false there is a real defect signal"); the audio
packet-count granularity footnote (:69) documents why count-parity is undefined and
payload-bytes is the judge.

## 4. Fabrication / inflation indicators
None. Zero-value rows are printed, not omitted; the one prior-run cross-reference
(:67, the 13:33 duration under-report) documents the error it fixed.

## 5. Findings
F-001 [S3-evidence] :164 — cited per-case ledger (corpus-big/_harness/decoded-rows.tsv)
absent from this tree; per-file claims not re-derivable repo-side. Same
missing-evidence-path class as the 15:13 sibling (baseline xtask feature-audit FAILURE).

## 6. Reachability
N/A (measurement report). Commands: python3 recomputation (composition sum, table
identity, family sums, mismatch-listing arithmetic), find for corpus-big (absent).
