# AUDIT: docs/gates/logs/phase7-exit-test-rerun.log (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A09 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Lines / functions | 43 lines (very long lines — cargo output blocks up to 36 KB) |
| File verdict | REAL data (authentic cargo-test output; every count re-verified exact) |
| Reachable from shipped entry point? | N/A (evidence log); cited by docs/gates/phase7-exit.md:47 and :98 as the r2 re-confirmation evidence |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | awk re-sum over the round-1 log (306/4418/4/49 — EXACT); python decode of the two assertion byte-arrays; rg test-name status vs baseline; sed quote verifications; ls worktree |

## 1. One-paragraph verdict
Authentic raw cargo-test output, and every number in it re-derives exactly:
the header's citation of round-1 (phase7-exit-test.log: "306 targets: 4418
passed / 4 failed / 49 ignored; exit 101") re-summed by awk to 306 / 4418 /
4 / 49 — byte-exact; the rerun reproduces exactly the 4 failing targets
(lumen-cli --lib 124+1, settings-schema --lib 42+1, xtask --test cli
11+1+3i, settings-schema --test integration 6+1); both runs' exit codes
are honestly embedded (TEST_RERUN_EXIT=101 :32, TEST_RERUN_INTEGRATION_EXIT=101
:43) with an explicit methodology note that the pipe was filtered and the
unfiltered exit is recorded (:31). The 4 failures are historical facts that
are all RESOLVED on HEAD (3 pass in the orchestrator baseline; the areas
test was replaced by a seed-derived version whose comment cites this very
gate: help.rs:305-312 "a seed that legitimately grows (c71cd5a grew it 27
entries / 6→7 screens) can never strand this test stale again (phase-7 gate
OF-2)"). Three S4 data-quality notes: one failure's evidence is internally
ambiguous, one's root cause requires byte-array decoding, and the worktree
anchor no longer exists.

## 2. What actually works (verified)
- Arithmetic (awk over docs/gates/logs/phase7-exit-test.log): blocks 306,
  passed 4418, failed 4, ignored 49 — EXACTLY the header's citation.
- The 4 failing targets' inline results (all quoted, sed-verified):
  :11 `test result: FAILED. 124 passed; 1 failed; 0 ignored ... 6.38s`
  :18 `test result: FAILED. 42 passed; 1 failed ... 0.01s`
  :28 `test result: FAILED. 11 passed; 1 failed; 3 ignored ... 0.06s`
  :42 `test result: FAILED. 6 passed; 1 failed ... 0.02s`
- Embedded exits: :32 `TEST_RERUN_EXIT=101`, :43
  `TEST_RERUN_INTEGRATION_EXIT=101` — cargo's honest 101 for test failure.
- Honest methodology note: :30-31 `error: 3 targets failed:` +
  `RERUN_PIPESTATUS note: filtered; unfiltered exit below`.
- Failure content decoded (python, byte-arrays -> text):
  - :9-10 areas: left has 7 areas (with "extensions") vs right 6 — matches
    the c71cd5a seed growth the current help.rs:305-312 comment describes.
  - :26-27 README no-op copy: left = 3,569-byte file '# Lumen Media Player
    (working name)...' vs right = 8,032-byte same-prefix file — the no-op
    dest copy was left TRUNCATED (not content-drifted).
- Ignore reasons (:20-23) carry real cross-refs to the boot-tier proof tests
  (rebrand_acceptance.rs::surface_*) — honest annotations, and the 4th
  rebrand_skeleton test passes in the same log.
- Current status of the 4 named failures (baseline cargo-test-workspace.txt):
  committed_seed_validates_against_committed_product ... ok (:5359);
  reverse_control_real_product_toml_is_a_noop ... ok (:6792);
  committed_outputs_have_no_drift ... ok (:5379); and
  areas_block_lists_the_six_areas is GONE — replaced by
  areas_block_lists_every_screen_the_seed_declares (help.rs:312).

## 3. Findings
F-001 [S4]
Where: docs/gates/logs/phase7-exit-test-rerun.log:39-42
Code:
```
thread 'committed_outputs_have_no_drift' (22725) panicked at crates/settings-schema/tests/integration.rs:117:5:
stdout: schema-gen: no drift — outputs match /home/z/my-project/wt/w24f/schemas/settings.toml (32 entries)
```
What is wrong: the log's own content for failure #4 is internally ambiguous — the captured stdout says "no drift — outputs match" while the test FAILED at :117; no assertion values are recorded, so a reader of this log alone cannot tell what the failing assertion compared (the stdout lines belong to the schema-gen phase of the test, not the failing compare). Honest raw capture, incomplete evidence. Confidence: HIGH — Verification: read + decoded.
F-002 [S4]
Where: docs/gates/logs/phase7-exit-test-rerun.log:24-27
What is wrong: failure #3's root cause is invisible without byte-decoding the two arrays — left (the no-op dest README) is a 3,569-byte TRUNCATED prefix of the 8,032-byte right (source); the log records the arrays but annotates nothing (a truncation-in-copy signature consistent with the documented disk-full era of that worktree wave). Now green; the data is real but unannotated. Confidence: HIGH — Verification: decoded both arrays with python.
F-003 [S4]
Where: docs/gates/logs/phase7-exit-test-rerun.log:2
Code:
```
# Worktree: /home/z/my-project/wt/w24f (branch w24/f @ a4e35e7)
```
What is wrong: provenance anchor unverifiable — `ls /home/z/my-project/wt/w24f` -> No such file or directory (the worktree is gone; sibling logs phase7-exit-xtask.log carry the same anchor). The commit hash a4e35e7 is not checkable without git. UNVERIFIED provenance, dated and disclosed as a worktree — acceptable for an evidence log, flagged for the record. Confidence: HIGH — Verification: ls.

## 4. Data-integrity checklist (what was checked)
- Header citation arithmetic vs round-1 log: EXACT (306/4418/4/49, awk).
- Per-target result lines: 4 targets, counts internally consistent
  (124+1, 42+1, 11+1+3i, 6+1); duplicated summary lines :33-35 match :11/:18/:28.
- Exit codes embedded twice, matching cargo's failure code 101.
- Assertions decode to real content (areas list, README text) — not
  fabricated output.
- Timestamps/provenance present (:2-3); cmd lines recorded (:6, :38).
- Cross-citations valid: phase7-exit.md:47/:98 cite this log as r2 evidence
  with matching verdict (FAIL, exit 101).
- Current-tree reconciliation: all 4 failures resolved (3 green in
  baseline; 1 replaced by a seed-derived test that documents the fix
  rationale citing "phase-7 gate OF-2").

## 5. Test honesty (the log's tests)
The 3 ignored rebrand_skeleton tests carry honest tier-reasons with
cross-file pointers (rebrand_acceptance.rs::surface_*) — the log is the
evidence that they were ignored for cause, not to hide failures. The 4
failing tests were failing HONESTLY (each failure message names a real
drift: seed growth x2, truncated copy x1, unannotated compare x1) — this
log is why phase 7's gate recorded FAIL before the fixes landed.

## 6. Open questions for the next agent
- Why did the no-op dest README end up truncated (F-002)? If the w24f
  worktree hit ENOSPC mid-copy, the rebrand reverse-control test was
  failing for an environmental reason — worth one annotation line in
  phase7-exit.md's account of failure #3, since "reproduced" (its :47)
  suggests it was believed deterministic.
- integration.rs:117's failing compare (F-001) — the passing stdout vs
  failed assertion gap: check the phase7-exit.md:98 narrative for which
  drift class it assigns to failure #4.
