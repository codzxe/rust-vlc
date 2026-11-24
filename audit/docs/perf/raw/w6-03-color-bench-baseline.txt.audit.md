# CLAIM AUDIT: docs/perf/raw/w6-03-color-bench-baseline.txt (L1-2121, chunks part1-3)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 7 / 3 / 1 / 0 / 0 / 1 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This raw bench dump is what the PERF-VS-VLC methodology should look like: the numbers are internally consistent to the last digit — all 96 MEDIAN rows recomputed from the 2016 REP rows match exactly (median and best), every group has exactly the claimed 21 timed reps with contiguous indices 0–20, and the producer binary's protocol (interleaved arms within each rep, warmups discarded, per-rep raw data) is verifiable in code. Two defects, neither material: the header column description advertises a `tag` column that does not exist (first data column is `phase`), and the `bitexact_vs_legacy` column is a per-configuration constant string repeated on every MEDIAN row — including float_ref rows, for which the tool's own docs say floats are NOT bit-exact — so the column never carries row-specific evidence.

## 2. Claim ledger
| # | Claim (verbatim, ≤25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | `# cpu="Intel(R) Xeon(R) Processor" cores=2` | :1 | REAL-UNPROVEN | Machine facts self-reported at run time by the tool (color_bench.rs:620-634 reads /proc + available_parallelism); not independently confirmable now. |
| 2 | `# seed=0x5eed000000000603 reps=21 warmup=2 interleaved` | :3 | REAL | Producer code: Rng(0x5eed_0000_0000_0603) (crates/codecs/examples/color_bench.rs:443), REPS+WARMUP loop with per-rep arm rotation (color_bench.rs:746-757), header line :634. File: 96 groups × 21 reps = 2016 REP rows; interleaving is in the run loop (file is printed grouped per arm AFTER timing — verified code, not file order). |
| 3 | REP columns include `rep_idx` 0..20 per group | all REP rows | REAL | Recount: 96/96 groups have exactly reps {0..20}, 2016 rows (python3). |
| 4 | MEDIAN rows = median of the 21 reps; best_ns = min | all 96 MEDIAN rows | REAL | Recomputed all 96 groups: 0 mismatches for median and best (python3 statistics.median / min). |
| 5 | Configs: 3 sizes × 4 samplings × 2 fills × 4 arms | file | REAL | 640x480/1280x720/3840x2160 × gray/420/422/444 × random/smooth × legacy/current/scalar/float_ref = 96 (verified by grouping). |
| 6 | `bitexact_vs_legacy` = "legacy=true,current=true,scalar=true" on every row | e.g. :30, :2984-area MEDIAN rows | SHALLOW/constant | All 96 MEDIAN rows carry the identical string (rg | awk uniq -c -> 96× one value), including float_ref rows. Producer builds `checks` once per config (color_bench.rs:735-741) and prints the same string on every arm's MEDIAN line — a config-wide summary, not per-row evidence. Header calls it a per-row column. |
| 7 | simd arm "absent from the baseline phase binary — recorded… explicit" (tool doc) | tool doc :25-28 | REAL | No simd rows in this baseline file (fills/arms recount shows exactly 4 arms); the tool documents the arm as conditional on CPU/build. |
| 8 | "floats are NOT bit-exact with the shipped fixed-point path… context, never a candidate" (tool doc) | color_bench.rs:29-31 | REAL | float_ref present as context arm; its bit-exactness is simply not asserted in the checks string (skipped), consistent with doc. |
| 9 | Legacy kernel "kept verbatim… Frozen; never edited" | color_bench.rs:18-21 | REAL-UNPROVEN | The legacy kernel is embedded in the example file; "verbatim from color.rs @ 62c48b6" is unverifiable without git (UNVERIFIED, git forbidden). |
| 10 | Warmup 2 rounds untimed | :3 + code | REAL-UNPROVEN | Code discards rep < WARMUP from times (color_bench.rs:752-755) — logic verified by read; the ACTUAL occurrence of 2 warmups in this run is not observable from the file (warmups unrecorded by design). |
| 11 | Header: "REP columns: tag phase arm fill WxH sampling rep_idx ns" | :4-5 | FALSE (schema description) | Actual REP data columns are phase, arm, fill, WxH, sampling, rep_idx, ns (7) — no `tag` column; producer prints only {phase} (color_bench.rs:762-765). MEDIAN same (8 claimed, 8 actual but `tag` absent, e.g. first MEDIAN row fields). |
| 12 | `# loadavg_start="1.27 0.90 0.45"` | :2 | REAL | Read from /proc/loadavg at start (color_bench.rs:625-631); the run footer `# loadavg_end="…"` IS present as the file's last line (python check: loadavg_end in file -> True; 7 header lines total). |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| bitexact_vs_legacy column implies per-row bit-exactness evidence | Tool docs: float_ref "NOT bit-exact… context, never a candidate" (color_bench.rs:29-31) | The column is a constant per-config string (verified 96 identical values) and does not assert anything about float_ref; header naming is misleading, tool docs are truthful. |

## 4. Fabrication / inflation indicators
None found: every number reconciles (medians, bests, rep counts, group cardinality, interleaved protocol in code). The one constant-string column (bitexact) is a presentation defect, not inflation — the underlying equality checks ARE computed per config (scratch == reference, color_bench.rs:735-741).
Commands run: python3 aggregation over 2016/96 rows (outputs above), rg over crates/codecs/examples/color_bench.rs, awk/uniq on MEDIAN flags. Exit 0.
Findings: S2: 0 · S3: 0 · S4: 2 (F-006 header schema mis-describes columns; F-007 bitexact column constant on all rows incl. float_ref).
