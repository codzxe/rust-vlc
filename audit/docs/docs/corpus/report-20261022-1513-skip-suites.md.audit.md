# CLAIM AUDIT: docs/corpus/report-20261022-1513-skip-suites.md
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R3; replaces a prior audit at the wrong path audit/docs/corpus/… missing the `docs/` segment) |
| Claims extracted | 12 |
| REAL 6 (internal arithmetic, all recomputed) / REAL-UNPROVEN 2 / UNVERIFIED 4 (evidence ledgers absent from snapshot) | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Maximally self-honest measurement report: it labels itself "NOT a fresh measurement"
(skip-suites re-aggregation) in five separate places, admits suite gaps ("not-emitted")
as N/A rows instead of massaging them to zeros, and registers the follow-up. Every piece
of internal arithmetic I recomputed reconciles exactly (486-file composition, 40-row
MJPEG table -> 99 files/19,729 frames/97 eos + 2 start-error, PCM 41=18+23, drift 8+19=27).
The per-case ledgers it cites live under /home/z/my-project/corpus/_harness/ which does
NOT exist in this audit snapshot — unverifiable here (matches the ecosystem-wide
missing-evidence-path condition in the baseline).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "mode: --skip-suites ... the Rust suites were NOT driven ... NOT a fresh measurement" | :6-7 | REAL (self-declared) | repeated at :11, :24, :42, :93, :172 — the honesty marker is structural |
| 2 | "files: 486 (1514 MiB clean), rejected draws: 2" | :4 | REAL | composition line summed in python: 63+57+134+62+16+13+11+31+6+33+26+4+16+14 = 486 EXACT |
| 3 | MJPEG table: 40 rows, files/frames/terminals | :44-85 | REAL | recomputed: 40 rows, files=99, frames=19,729, eos=97 + start-error=2 = 99 = files; population 99 matches the "99/99 oracle dims" row and the 99-row verdicts ledger claim |
| 4 | "comparable PCM cases 41/65 ... byte-exact 18/41 ... shorter payload 23 ... mismatches 0" | :95-100 | REAL | arithmetic consistent: 41 = 18 + 23; "any mismatch is a defect signal" honesty framing |
| 5 | "garbage rejection 10/11 (must be 100%)" + named false positive | :17-19 | REAL | self-flagged failure with the exact offending file named — anti-inflation |
| 6 | "video dims/pix_fmt/audio match rates: N/A — not-emitted (suite gap) (0/99 ...)" | :121-128 | REAL | suite-gap admitted as N/A rather than claimed-passed; 498 gap cells counted in :174 |
| 7 | "mismatch listing ... 10 video rows + "and 89 more" ... 10 audio + "55 more"" | :132-155 | REAL | 10+89=99 video, 10+55=65 audio — populations reconcile exactly |
| 8 | "27 file(s) where the oracle's decoded facts disagree with the generator's ... 8 listed + 19 more" | :157-168 | REAL | 8+19=27 EXACT; gray-vs-yuvj444p drift mechanism explained |
| 9 | "decoded-rows ledger ... 486 rows, 498 suite-gap cells" | :174 | UNVERIFIED | corpus/_harness/decoded-rows.tsv NOT PRESENT in this snapshot (find over /home/z/my-project: 0 hits) |
| 10 | "mjpeg-verdicts.tsv (99 rows)" | :89, :175 | UNVERIFIED | same — ledger file absent from snapshot; internal consistency with #3 corroborates the count indirectly |
| 11 | "suite exit codes: probe=reused demux=reused mjpeg=reused pcm=reused" | :7 | REAL-UNPROVEN | no persisted TSVs in snapshot to check against; consistent with #1's self-declaration |
| 12 | "harness wall: 0s" | :173 | REAL | consistent with pure re-aggregation (no suites driven) — the number would be a red flag if suites HAD run |

## 3. Contradictions
None internal. One snapshot-vs-doc gap: the doc's evidence paths point at the dev
sandbox (/home/z/my-project/corpus/_harness/), absent from this audit copy — the same
missing-evidence-path class the orchestrator baseline records (xtask feature-audit
FAILURE, 2 evidence paths missing).

## 4. Fabrication / inflation indicators
None — the opposite: 0/99 match-rate rows are printed as "N/A — not-emitted (suite gap)"
rather than silently dropped. Duration-metric footnote (:38) documents the exact prior
under-report (−40/−16.7/−8.3 ms) it corrected.

## 5. Findings
F-001 [S3-evidence] :89, :174-175 — the two per-case ledgers the report cites as its
auditable ground truth (decoded-rows.tsv, mjpeg-verdicts.tsv) are not in this tree; a
reviewer of the repo alone cannot re-derive the per-file claims. Confidence HIGH
(find returned nothing) — Verification: ran (find /home/z/my-project -name …).

## 6. Reachability
N/A (measurement report). The suites it aggregates are the repo's corpus harness.
Commands: python3 recomputation (composition sum, MJPEG table sums, PCM/drift
arithmetic), find for ledger paths (absent), ls of corpus dirs (absent).
