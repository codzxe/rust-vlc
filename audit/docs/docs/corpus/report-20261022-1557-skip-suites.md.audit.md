# CLAIM AUDIT: docs/corpus/report-20261022-1557-skip-suites.md
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R4) |
| Claims extracted | 9 |
| REAL 5 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 4 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
An oracle-side-only corpus report whose honesty is the point: every suite
section is empty "by construction" and every Lumen cell says "not-emitted
(suite gap)". Its numbers are internally consistent (composition sums to 310;
family histograms sum to their populations; listed + "… more" rows reconcile
exactly) and cross-corroborated by the full-run twin report (1610, same seed
63117490995983871645529930018, same 310 files) which carries identical
oracle-side numbers and real "got" values where this one says suite-gap. The
media corpus and TSV ledger live outside the repo and are gone — those are
UNVERIFIED, nothing more.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "files: 310 (1095 MiB clean), rejected draws: 1" + seed 63117490995983871645529930018 | :3-4 | REAL (cross-report) | `rg 'seed:' docs/corpus/report-20261022-1610.md` -> :3 same seed; :4 "files: **310** (1095 MiB clean), rejected draws: 1". |
| 2 | composition classes summing to 310 | :5 | REAL | python3: 38+39+87+42+8+6+11+21+4+20+19+2+8+5 = 310 exactly; identical line in 1610:5. |
| 3 | "--skip-suites ... suite sections below carry no rows by construction" | :6-7,11 | REAL | All four suite tables in this doc are empty with 0/0 cells; explicit "skipped — not driven" markers at :7 and :127. |
| 4 | Oracle-side decoded-format histograms (68/69 dims; yuv420=46,yuv444=22; 45/47; u8=13,f32=13,s24=10,s16=9; channels 2=18,1=10,8=9,6=6,4=1,3=1; rates ...) | :65-73 | REAL | Same corpus full run report-20261022-1610.md:103-110 carries byte-identical values; each histogram sums to its population (46+22=68; 13+13+10+9=45; 18+10+9+6+1+1=45; rates sum 45). |
| 5 | "Lumen-side decoded format/dims: not-emitted (suite gap)" + registered follow-up (out_format/out_w/out_h columns) | :85 | REAL | This report + 1610 mismatch rows show "got not-emitted (suite gap)" here vs 1610:125-132 real "got 120x90 rgba8" values — the gap claim is exactly the difference between the two runs. |
| 6 | Mismatch listing counts (video 10 + "58 more" = 68; audio 10 + "35 more" = 45; drift "10 file(s)" = 8 + 2 more) | :89-123 | REAL | Arithmetic: 10+58=68 (= oracle-dims population), 10+35=45 (= pcm population), 8+2=10 (=:114 "10 file(s)"). |
| 7 | "harness wall: 36s" | :128 | UNVERIFIED | Run-time fact; no artifact in tree. |
| 8 | "decoded-rows ledger: /home/z/my-project/corpus-big/_harness/decoded-rows.tsv (310 rows, 348 suite-gap cells)" | :129 | UNVERIFIED | `ls /home/z/my-project/corpus-big` -> no such dir (external corpus gone). 310 rows would mean one row per file (plausible); cannot check. |
| 9 | "replay with gen_corpus.py --replay corpus/manifest.json" | :3 | REAL-UNPROVEN (mechanism) / UNVERIFIED (manifest) | tools/corpus/gen_corpus.py exists in repo (also audited: audit/files/tools/corpus/gen_corpus.py.audit.md); the committed docs/corpus/corpus-big-manifest-20261022.json is a DIFFERENT run (seed 58007..., 485 files) — this run's manifest lived in the external corpus dir and is gone. |

## 3. Contradictions
None. The committed manifest (seed 58007..., 485 files) does not contradict
this report — it documents the LATER 20:02 corpus, and this report never claims
its own manifest was committed.

## 4. Fabrication / inflation indicators
None — the opposite: the report systematically downgrades itself (0/0 cells,
"must be 100%" shown as 0/0 skipped, suite-gap admissions) and its numbers
reconcile both internally and against the twin full-run report. The only
soft spot is that ALL measured oracle numbers come from ffprobe runs over a
corpus that no longer exists; corroboration is documentary (1610 twin), not
re-executable.
