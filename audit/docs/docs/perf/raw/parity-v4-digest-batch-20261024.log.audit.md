# DATA AUDIT: docs/perf/raw/parity-v4-digest-batch-20261024.log (credit: W19-A07, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | cargo test transcript: bench-vs-vlc lumen_pcm `corpus_batch_via_env` — 1 passed, 9 filtered out, 0.20s |
| Cited by | pairs with docs/perf/raw/parity-v4-lumen-digests-20261024.json (the run's output) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
A genuine-looking cargo-test run record whose every structural number reconciles with the
repo's code:
- `corpus_batch_via_env` EXISTS: tools/bench-vs-vlc/tests/lumen_pcm.rs:532; the file's
  header (:12) documents it as "the batch entry" marked `#[ignore]` — matching a
  `-- --ignored` invocation.
- Test census: `#[test]` count in lumen_pcm.rs = 10; the log says "1 passed; ...
  9 filtered out" — 10 total, 1 selected, 9 filtered: EXACT match.
- The output schema is bound in code: lumen_pcm.rs:504 asserts the emitted JSON contains
  `"schema": "bench-vs-vlc/lumen-pcm-digests/1"` — the exact schema string in
  parity-v4-lumen-digests-20261024.json (my file-25 audit), so this log records the run
  that produced that JSON. The pair is internally coherent.
- Runtime 0.20s for 9 PCM digests (max ~2 MB per file) is plausible.
S4 only: build paths point at the dead worktree `/home/z/my-project/worktrees/w7-d/`
(same provenance-rot class as the soak manifests' w24b path). Not re-run (cargo-build
budget; corpus off-repo). Quotes byte-verbatim from Read output.
