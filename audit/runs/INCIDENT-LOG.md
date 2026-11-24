# Incident Log — orchestrator environment

## 2026-11-24: full sandbox recycle during Wave 16
The sandbox was recycled (server shutdown) mid-Wave-16. All local state was
lost: the working tree, target/ binaries, rust toolchain, samples, and the
orchestrator scripts. Recovery: the audit branch on GitHub (pushed
continuously after every wave) was re-cloned; waves 0-15 and all baseline
run outputs were recovered 100% (1,620 audit files).

Casualty: Wave 16's three completed agents (A05, A07, A10) had written
~225 audit files to disk but had NOT yet been committed by the orchestrator
(commits happen at wave gate) — that work is lost and Wave 16 is re-run in
full. Findings from the lost runs that were reported back to the
orchestrator (recovered from its transcript) are preserved below so the
re-run can cross-check against them:

- [lost-run W16-A07 report] recovery_matrix 22 pass / 1 FAIL
  (matrix_mp4_trunc4: settle "Playing" vs expected "Error",
  recovery_matrix.rs:489); journey 6/1 FAIL (journey_seek: pre-seek
  deliveries 90 vs 154); the first-vertical package lib failure
  (exit 101) meant these binaries never ran in the baseline —
  "4744 passed / 2 failed" UNDERCOUNTS failures.
- [lost-run W16-A10 report] fixture drift gate covers only 28/116 files
  under tests/fixtures/generated/; CORPUS.md "authored in-tree" FALSE for
  88 ffmpeg-authored files; first-vertical evidence transcripts STALE
  (currency tests RED in baseline); oracle_check.sh invoked by nothing;
  13/19 visual baselines nightly-only; 4/5 browser-matrix lanes never
  executed.
- [lost-run W16-A05 report] corpus_builders.py "odd" dims claim inflated;
  a11y suite accepts 3 shipped WCAG-AA contrast failures (1.57:1/1.75:1);
  ffmpeg-authored fixture families carry no hash pin.

The re-run must verify or refute these independently (they are now
prior-agent claims, not audited findings, until re-derived).
