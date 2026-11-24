# CLAIM AUDIT: docs/corpus/report-20261022-1417-skip-suites.md
| Field | Value |
|---|---|
| Auditor | W19-A07 (recovered by W19-R4) |
| Claims extracted | 8 |
| REAL 4 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 4 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Same honest pattern as its sibling skip-suites reports: every Rust-suite section
is empty by construction ("skipped — not driven"), every Lumen cell is a
suite-gap admission, and the oracle-side histograms reconcile internally
(composition sums to 486; every histogram sums to its population; listed +
"… more" rows reconcile exactly). The later 15:13 re-aggregation run over the
SAME corpus (seed 6033317083842721047527680438) carries byte-identical oracle
numbers — independent corroboration. The corpus media and TSV ledger are
external and gone: UNVERIFIED residue.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "files: 486 (1514 MiB clean), rejected draws: 2" + seed 6033317083842721047527680438 | :3-4 | REAL (cross-doc) | report-20261022-1334.md:4-5 identical seed/files/MiB/rejected; 1513 report same seed. |
| 2 | composition classes summing to 486 | :5 | REAL | python3: 63+57+134+62+16+13+11+31+6+33+26+4+16+14 = 486 exactly; identical line in 1334:5. |
| 3 | "--skip-suites ... suite sections below carry no rows by construction" | :6-7 | REAL | All suite tables empty with 0/0 cells and explicit "skipped — not driven" markers at :7,:124. |
| 4 | Oracle-side histograms (99/99 dims; yuv420=63,yuv444=36; 65/67 sample_fmt; s16=19,u8=19,s24=14,f32=13; channels 2=16,1=14,6=13,4=12,8=6,3=4; rates 11025=10...) | :62-70 | REAL (cross-doc) | report-20261022-1513-skip-suites.md:110-117 carries byte-identical values; sums: 63+36=99, 19+19+14+13=65, 16+14+13+12+6+4=65, rates sum=65. |
| 5 | Mismatch listing reconciliation (video 10 + "89 more" = 99; audio 10 + "55 more" = 65; drift 8 + "19 more" = 27) | :86-120 | REAL | Arithmetic exact vs populations (:62-66, :111 "27 file(s)"); same counts in 1513:144,155,168. |
| 6 | "harness wall: 168s" | :125 | UNVERIFIED | Run-time fact; no artifact. |
| 7 | "decoded-rows ledger: /home/z/my-project/corpus/_harness/decoded-rows.tsv (486 rows, 498 suite-gap cells)" | :126 | UNVERIFIED | `ls /home/z/my-project/corpus` -> no such dir (external corpus gone). |
| 8 | Suite emission follow-up (out_format/out_w/out_h; out_desc/out_rate/out_ch) | :82 | REAL (registration) | Same registered follow-up text present in 1513 and in the 1557 sibling; consistent harness contract across reports. |

## 3. Contradictions
None. 1334 (full run, same seed) lacks the decoded-format section (added later),
so no overlap conflicts; 1513's re-aggregation matches every overlapping value.

## 4. Fabrication / inflation indicators
None. Numbers downgrade themselves (0/0 cells, suite-gap admissions, "must be
100%" shown skipped). Corroboration is documentary (two sibling reports), not
re-executable — the standing limit of this family.
