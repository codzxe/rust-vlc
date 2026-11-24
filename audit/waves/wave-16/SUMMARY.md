# WAVE 16 SUMMARY — test quality across Rust crates, UI, e2e, visual

| Field | Value |
|---|---|
| Agents | A01–A10 rerun after sandbox recycle; adapter instability -> recovery RR1–RR5; 100% coverage (1312 rows) |
| Files audited | 756 distinct files (1312 rows incl. binary chunks) |
| Findings | S0:0 S1:1 S2:2+ S3:~60 S4:~120 |
| Tree purity | clean |

## The test-quality verdict
The Rust test culture is genuinely strong where it exists: model
checkers with independent oracles, exact typed-refusal expectation rows
(executed), byte-surgery fixtures with verified digests, zero-alloc
instruments with real counters, schema-driven UI vitest, anti-hardcoding
patterns (mock-catalog differentiators). BUT the evidence paper trail
overstates:

1. [S1] First-vertical evidence transcripts are STALE at HEAD:
   committed_transcripts_match_fresh_runs FAILED in the Wave-0 baseline
   (open.json drift + final_frame) — and the package's lib failure
   (exit 101) aborted the test package BEFORE journey.rs/recovery_matrix
   ever ran in the baseline. The "4744 passed / 2 failed" headline
   UNDERCOUNTS: wire_mode/journey/recovery_matrix/class_b binaries never
   executed. Matrix row 203 cites this stale tree (W16-A06).
2. [S2, recovered-run cross-check CONFIRMED by RR1] The lost-run reports
   (preserved in audit/runs/INCIDENT-LOG.md) said recovery_matrix
   22-pass/1-FAIL (matrix_mp4_trunc4 settles Playing vs expected Error)
   and journey 6-pass/1-FAIL (journey_seek pre-seek deliveries 90 vs 154)
   — RR1's re-derivation found the same 2 currency-test failures in the
   baseline and the fail-fast undercount. The prior-run findings are
   therefore corroborated (though not re-executed post-incident).
3. [S3 systemic] Fixture drift gate covers only 28/116 files under
   tests/fixtures/generated: ~88 ffmpeg-authored files (wav/aiff/ogg/
   mkv/webm/3gp/mov/HLS families) have NO hash pin; CORPUS.md's
   "authored in-tree / no external encoder output" is FALSE for the ogg
   and HLS families.
4. [S3 systemic] corpus-v4 absent in every environment -> ~10
   differential lanes are vacuous-green NO-OPs (counted as passes in the
   4744 headline; documented W04-W10).
5. [S3 systemic] Fuzz: 35 targets registered, harnesses real, campaign
   NEVER RUN anywhere; fuzz/README "SCAFFOLDED NOT WIRED" contradicts
   audit.yml's nightly fuzz job; several seeds are byte-copies of unit
   fixtures (zero novel coverage).
6. [S3] Visual lane: 13/19 baselines nightly-only (regression ships
   <=24h undetected); committed CI labels stale ("30/136" vs actual
   50/216); 4/5 browser-matrix lanes never executed anywhere; all
   -2x- baselines compared at CSS scale (DPI lane blind); web-matrix
   sole run record captured on a dirty branch tree ~2wk stale.
7. [S3] a11y suite accepts 3 shipped WCAG-AA contrast failures
   (1.57:1 / 1.75:1) behind its exclusion ledger.
8. [S3] evidence-quality liars: AccentColorPicker.test claims end-to-end
   proof but asserts two schema constants; scenarios.rs trail check
   verifies a 1-of-3 disjunction under a name that claims a sequence;
   ControlBar CSS "regression guards" regex the stylesheet source, not
   behavior; keymap tested against a hand-copied 3-binding excerpt.
9. [S4] ui/.vitest/json/output.json committed ad-hoc run output from a
   FOREIGN checkout (path leak, success:false, zero consumers) (W15-R3
   cross-ref).
