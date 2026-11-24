# DATA AUDIT: docs/perf/raw/memory.csv (credit: W19-A09, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | CSV memory capture (36 lines): 7 scenarios x 5 runs; header `scenario,run,peak_rss_kib,hwm_samples,wall_ms,status` |
| Live re-verified? | PARTIAL — exit/wall reproduce; RSS is build-era-dependent (see below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## Verdict
Real raw data, cross-derived by the perf baseline exactly:
- docs/perf/baseline.md:153-158 cites `raw/memory.csv` for all 7 rows with
  min/median pairs (help 3224/3348; decode-bench 3532/3676, 3380/3548, refusal
  3000/3124; session 12356/12440, 11796/11868) — every pair re-derives from the
  csv's 5-run sets ✓ (medians recomputed here).
- Cross-consistent with decode.csv (same capture session): decode-bench
  avi_mjpeg_pcm wall ~8.7ms here vs 8.4-8.6ms there; 2pcm ~3.77 vs 3.6-3.8 ✓;
  mp4 exit65 refusal at ~0.67ms in both files ✓ — the refusal-path rows agree
  across the two raw files.
- The `session:mp4` rows show status=ok at ~0.7ms (the session command layer
  completes with the typed refusal handled in-band) while decode-bench
  propagates exit 65 — an accurate, non-contradictory distinction.
- Outliers are retained raw (2pcm decode run 3: 31.49ms wall / 813 samples;
  session 2pcm run 5: 40.89ms) — no cherry-picking (the same honest-retention
  pattern W19-R7 credited in the r1-avi probe audit).
S4 #1: no date/provenance header — the file is undated. S4 #2: dated-data drift —
a LIVE `lumen --help` re-run today gives exit 0 / 3.5ms wall (both consistent)
but peak RSS ~12.9 MB via rusage vs the csv's ~3.2 MB: the csv describes the
capture-era binary; current-binary RSS is not comparable from this file alone.

Commands run: rg memory.csv in baseline.md (7 citations); python3 median recompute;
live `bin/lumen --help` with RUSAGE_CHILDREN (exit 0, 3.53 ms, ru_maxrss 12932 KiB).
