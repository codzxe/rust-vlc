# DATA AUDIT: docs/perf/raw/w6-03-color-bench-baseline.txt
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Size / shape | 2,121 lines TSV; 2,016 REP rows (96 groups × 21 reps) + 96 MEDIAN rows |
| File verdict | REAL (structure + all medians/bests recomputed exactly) / timing values REAL-UNPROVEN |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 recompute (exit 0); rg producer checks |

## 1. What it is / provenance
Raw per-rep benchmark capture of the MJPEG color pipeline (YCbCr→RGBA + chroma upsampling,
`video/mjpeg/color.rs`), arms legacy/current/scalar/float_ref, produced by the in-tree
harness `crates/codecs/examples/color_bench.rs`. Its own header (lines 1-6, byte-verified):
```
# cpu="Intel(R) Xeon(R) Processor" cores=2
# loadavg_start="1.27 0.90 0.45"
# seed=0x5eed000000000603 reps=21 warmup=2 interleaved
# REP columns: tag phase arm fill WxH sampling rep_idx ns
# MEDIAN columns: tag phase arm fill WxH sampling median_ns best_ns bitexact_vs_legacy
# phase=baseline-2
```
Producer cross-check: color_bench.rs:46-47 documents "2 untimed warm-up rounds, then 21
timed reps, arms **interleaved**"; :634 prints `println!("# seed=0x5eed000000000603
reps=21 warmup=2 interleaved")` — byte-identical to the artifact's header line.

## 2. Data verification (executed, python3)
- REP rows: 2,016; groups: 96; reps per group: exactly 21 (all groups).
- Arms: 24 groups per arm for legacy/current/scalar/float_ref (96 total).
- ALL 96 MEDIAN rows recomputed: median_ns and best_ns match my recomputation from the raw
  REP rows with 0 mismatches (python3 statistics.median + min).
- Sampling/fill census consistent (gray + random fills, 640x480 and other sizes).

## 3. Claim ledger
| # | Claim (verbatim) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "reps=21 warmup=2 interleaved" | :3 | REAL | color_bench.rs:46-47,634 protocol matches |
| 2 | "(honest-numbers mandate: medians, N≥15 recorded per-rep raw data" | producer doc-comment | REAL | 2,016 raw rows + 96 medians present, recomputed exact |
| 3 | loadavg printed by run | :2 | REAL-UNPROVEN | point-in-time machine state |
| 4 | MEDIAN rows' "bitexact_vs_legacy" column | all 96 | SHALLOW | constant string, see F-002 |
| 5 | timing values themselves | all rows | REAL-UNPROVEN | bench not re-run (cargo budget); numeric internal consistency proven instead |

## 4. Findings
F-001 [S4] Header/row column mismatch. The REP header names 8 fields
("tag phase arm fill WxH sampling rep_idx ns") but every REP row carries 7 — no `tag`
column exists (first field is `phase`, cf. "# phase=baseline-2"). A consumer indexing by
the documented schema mis-assigns every column. Confidence: HIGH — Verification: ran.
F-002 [S4] `bitexact_vs_legacy` carries the SAME string
("legacy=true,current=true,scalar=true") on all 96 MEDIAN rows — including the 24
`float_ref` rows where the comparison is semantically meaningless, and it is a constant
per-config string, not a measured column. Confidence: HIGH — Verification: ran
(distinct values = 1).
(Both reproduce dead W19-A09's findings exactly — independent re-derivation agrees.)

## 5. Consumers
docs/worklog/archive-2026-10.md (W6-03 session log, sole in-repo citation). No analysis
doc consumes it; the numbers stand alone as raw evidence.
