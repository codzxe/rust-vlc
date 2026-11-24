# DATA AUDIT: docs/perf/raw/r1/codec-benches-20261025.txt (credit: W19-A09, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | per-codec ignored-bench transcript, release build, 21 lines |
| Cited by | docs/perf/breadth.md:216 "codec-benches-20261025.txt (audio+av1 bench transcripts)" |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
HIGH-QUALITY raw transcript with full provenance (cmd, env, loadavg) and arithmetic that
survives every check I ran (python3):
- flac: 46421260/88200 = 526.32 -> "526.3x realtime" ✓
- mp3/mp2/aac/ogg realtime factors: all recomputed EXACTLY against each file's own
  inter-channel rate (e.g. 71675908/22050 = 3251.0 -> "3251x"; 99516548/16000 = 6220 ->
  "6220x"; 122633953/96000 = 1277.4 -> "1277x") ✓ x8 rows
- alac: 201600 B / 1.51486 ms = 133.08 MB/s ✓; 123480 B / 1.205112 ms = 102.46 MB/s ✓
- av1: 33.0 fps x 2.0736 Mpix = 68.4 Mpix/s -> "68.41" ✓; 33.0/112.9 = 0.2923 -> "0.292x" ✓
- vorbis median: the 7 raw reps are printed; median = 24152607.10 == the reported
  "24152607 samples/sec" ✓ — the one row where self-verification is possible from raw
  values PASSES.
- The bench tests exist: crates/codecs/tests/flac_bench.rs:59 contains the exact print
  format "flac_decode_corpus_0046: {median:.0} inter-channel samples/s"; *_bench.rs files
  present for mp3/mp2/aac/alac/adpcm/vorbis (with #[ignore] counts, matching the
  `-- --ignored` invocation in the header).
S4 only: run itself not re-executed (cargo-build-budget; corpus at /tmp/my-project/corpus-v4
is off-repo) — numbers UNVERIFIED by re-run but arithmetically and format-wise consistent.
Quotes byte-verbatim via `sed -n '2p;4p;10p'` (output above).
