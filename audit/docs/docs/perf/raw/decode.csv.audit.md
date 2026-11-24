# DATA AUDIT: docs/perf/raw/decode.csv (credit: W19-A07, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | CSV benchmark capture (37 lines): 12 runs x 3 fixtures; header `fixture,run,status,wall_ext_ms,bench_wall_s,bench_decode_s,v_frames,v_decoded,a_frames,a_decoded,bytes,refusal` |
| Live re-verified? | YES — 2/3 fixtures re-benched today via the release CLI; every field reproduces byte-exact |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## Verdict
Real, reproducible benchmark data with one dated-historical section:
- LIVE re-run `lumen decode-bench tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` ->
  `bytes=425516 v_frames=60 v_decoded=60 a_frames=60 a_decoded=60 wall_s=0.008
  decode_s=0.008` — EVERY field of the csv's 12 ok-rows reproduces exactly (exit 0);
  fixture on disk is 425516 B ✓.
- LIVE `avi_mjpeg_2pcm.avi` -> `bytes=497160 v_frames=20 v_decoded=20 a_frames=20
  a_decoded=20 wall_s=0.003 decode_s=0.003` — again byte-exact vs its 12 rows;
  fixture is 497160 B ✓.
- The 12 `exit65` rows for mp4_mjpeg_pcm.mp4 ("format `mp4` has no demuxer in this
  build (first vertical: AVI, D-018)") are HISTORICAL: today the same command returns
  exit 0 and decodes 60/60 (the MP4 demuxer has since landed). An honest negative
  record of the then-build, retained as raw evidence.
- Cross-citation: docs/perf/baseline.md:100-104 cites `raw/decode.csv` for exactly the
  AVI rows' medians (8 ms wall / 7-8 ms decode; 3 ms; 12 runs) — consistent.
S4 #1: the file carries no date/provenance header (the exit65-vs-now divergence is
undiscoverable from the file alone). S4 #2: as raw negative evidence of a superseded
build limitation it can mislead a present-day reader (mp4 decode works today).
No fraud: values are genuine and re-measurable.

Commands run: bin/lumen decode-bench on all 3 fixtures (exit codes 0/0/0 today);
ls -la fixtures (425516/497160 B); rg decode.csv in docs/perf/baseline.md.
