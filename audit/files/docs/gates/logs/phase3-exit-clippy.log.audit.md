# AUDIT: docs/gates/logs/phase3-exit-clippy.log (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines | 168 (read in full) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A — gate evidence log |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | Read (full); grep '^error' enumeration; cross-check vs docs/gates/phase3-exit.md and baseline cargo-clippy.txt |

## 1. One-paragraph verdict
Genuine `cargo clippy --workspace --all-targets -- -D warnings` output from
worktree w7-gate (paths L1-2), ending EXIT_CODE:101. It records exactly 15
lint errors on 3 targets: lumen-sweep 1 (assert-always-false), playback-engine
scenarios_b 6 (1 dead_code host.rs:535 + 5 expect() uses), codecs example
stage_bench 8 (dead_code from the example's `#[path]`-include of lib
sources). The log is honest failing-state evidence; the gate doc that cites
it miscounts it as 16.

## 2. What actually works (verified)
— grep '^error' over the file: 15 lint errors + 3 "could not compile"
summary lines; cargo's own per-target tallies at :14, :104, :167 say
1 / 6 / 8 — my enumeration matches cargo's own arithmetic exactly.
— Baseline cargo-clippy.txt (2026-11-24): 0 warnings — all 15 errors
were subsequently fixed; the phase3-exit.md square-up's clippy-debt-zero
claim is corroborated.
— Span text is real clippy output format (rust-1.99.0 lint links, `-D
clippy::expect_used` request provenance at :44).

## 3. Findings
F-001 [S3]
Where: docs/gates/logs/phase3-exit-clippy.log:3-14, 17-24, 29-103, 105-167
vs docs/gates/phase3-exit.md:53
Code (the log's own tally lines):
```
error: could not compile `lumen-sweep` (bin "lumen-sweep" test) due to 1 previous error
```
```
error: could not compile `playback-engine` (test "scenarios_b") due to 6 previous errors
```
```
error: could not compile `codecs` (example "stage_bench") due to 8 previous errors
```
What is wrong: the citing gate doc (phase3-exit.md:53, :85, :240) claims
"3 targets / 16 errors"; the log's arithmetic is 1+6+8 = 15. The doc also
mischaracterizes scenarios_b's 6 as "expect() ×6 + host" in §9 (:241),
which would be 7 — contradicting both the log and the doc's own §2 row 4
parenthetical.
Why it matters: a gate evidence doc that overcounts its committed log by
one, and disagrees with itself about the breakdown, degrades the record's
reliability; the log itself is correct.
What correct looks like: "3 targets / 15 errors (1 sweep, 6 scenarios_b
= 5 expect + 1 dead_code, 8 stage_bench dead_code)".
Confidence: HIGH — Verification: read + grep enumeration.

## 4. Data summary
Targets and errors:
- tools/sweep (lumen-sweep): assert!(false) always-false at worker.rs:553.
- playback-engine scenarios_b: dead_code host.rs:535
  last_pts_ns_of_track; expect() at scenario_02_play.rs:44/78/84 and
  scenario_03_pause_resume.rs:45/92.
- codecs example stage_bench: 8 dead_code (DecodedImage, cause,
  take_output, FRAME_SLOTS, pooled_frame_slot, decode,
  into_engine_error, FeatureDetail) — all from the example `#[path]`
  including lib sources.
Exit 101. No warnings survived to the current tree (baseline clippy 0
warnings).

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "3 targets / 16 errors, all PRE-EXISTING on main b8cb904" | phase3-exit.md:53 | FALSE count / TRUE classification | log shows 15 (1+6+8); pre-existence not checkable (no git) — UNVERIFIED |
| "stage_bench 8× dead_code" | phase3-exit.md:53 | REAL | log :105-167, exactly 8 |
| "lumen-sweep 1× assert!(false) worker.rs:553" | phase3-exit.md:53 | REAL | log :3-14, exact |

## 6. Dead / duplicated code
The 8 codecs dead_code items and host.rs:535 method were dead at gate
time (per the log) — subsequently removed/fixed (baseline clean).

## 7. Dependencies & flags
clippy with -D warnings + -D clippy::expect_used (command-line request at
log :44) — i.e., the workspace clippy.toml-level expect/unwrap bans were
already active at phase-3 time.

## 8. Open questions
Was the 16-vs-15 miscount ever corrected in phase3-exit.md (it stands
uncorrected as of this audit)?
