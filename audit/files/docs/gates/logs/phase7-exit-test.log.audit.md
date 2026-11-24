# AUDIT: docs/gates/logs/phase7-exit-test.log
| Field | Value |
|---|---|
| Auditor | W17-A01 |
| Date | 2026-11-23 |
| Lines / functions | 6399 (6400 w/ final NL) / 306 `test result:` blocks (269 unit+integration targets, 37 doctest blocks; 305 `Running`/`Doc-tests` headers — codec-api emits 2 result lines under 1 doctest header) |
| File verdict | REAL (committed gate-evidence data: a genuine, internally consistent, honestly-FAILING `cargo test --workspace` round-1 capture; exit 101 with 4 failing targets) |
| Reachable from shipped entry point? | N/A — evidence artifact; cited as §2 #3 evidence by docs/gates/phase7-exit.md:46 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | wc -l (6399); grep -c '^test result:' → 306; awk/python3 tallies → 4418/4/49; line-classification of ALL 6400 lines; 305-block integrity check → 0 mismatches; byte-array decode+diff of L6056/6057; baseline tallies → 315 blocks, 4744/2/51; baseline per-target greps; sed quote verification (all exit 0) |

## 1. One-paragraph verdict
This is a real, honestly-failing gate log, not a rigged one. It records one `cargo test --workspace` run (round 1, debug profile, worktree `wt/w24f`, dated 2026-11-10, tree `c339ee8`) that ran all 306 result blocks past 4 genuine failures and exited 101. Every arithmetic claim in its retro-annotated header and in the citing gate report row (4418 passed / 4 failed / 49 ignored, 306 blocks, 4 failing targets, TEST_EXIT=101) reproduces exactly from the body, and the per-test-line counts reconcile with the per-block result tallies to the last test. The four recorded failures carry real panic bodies (stale pins: 27-entry seed pin vs 32 actual, 6-area help pin vs 7 actual, Phase-1-stale README fixture in the rebrand no-op dest copy, pinned-literal drift in settings-schema integration) — i.e., the log proves the Phase-7 exit gate FAILED at `c339ee8`, which the parent report says outright. The only structural weakness is provenance: the header was retro-annotated by W24-K after the fact and its worktree/commit/no-alteration claims cannot be verified without git (forbidden) — UNVERIFIED.

## 2. What actually works (verified)
— `grep -c '^test result:' docs/gates/logs/phase7-exit-test.log` -> 306 (matches header L1 and report row 3).
— awk tally over all result lines -> **4418 passed / 4 failed / 49 ignored**; independent line-classification of all 6400 lines -> 4418 `... ok` test lines, 4 `... FAILED` lines, 49 `... ignored` lines — per-test lines equal result tallies exactly.
— python3 block parser -> 305 headers, `running N` == passed+failed+ignored in **all 305/305 blocks, 0 mismatches**.
— Tail `sed -n '6394,6399p'` -> `error: 4 targets failed:` + the 4 targets + `TEST_EXIT=101` — matches header L7 claim exactly.
— Body continues past every failure (next `Running` after L2794/L4768/L4802/L6064 errors) — corroborates the header's inferred `--no-fail-fast`.
— Byte-array decode of L6056/L6057 (python3) -> left 3569 B vs right 8032 B, identical prefix, first diff at offset 919 ("Phase 1 — Foundation planning" vs "Phase 6 — Network & streaming") — the rebrand failure is real byte evidence, not fabricated.
— Baseline cross-check: `audit/runs/baseline/cargo-test-workspace.txt` -> 315 blocks, **4744/2/51** (equals AGENT-SPEC §13 verbatim); identical per-target counts for audio_engine (86) and codecs lib (506); drift elsewhere is tree evolution over the 13-day gap (baseline markers show `started 2026-11-23T17:50:50Z` vs log 2026-11-10).
— Citing doc `docs/gates/phase7-exit.md:46` numbers match the log with zero discrepancy.

## 3. Findings
F-001 [S3]
Where: docs/gates/logs/phase7-exit-test.log:8-9
Code: (verbatim)
```
# Header retroactively annotated by W24-K (fix lane, review W24-R3 P2-1): round-1 logs were
# committed without capture-time headers; the body below was not altered in any way.
```
What is wrong: the header was written AFTER the run by a different agent, and every provenance claim (worktree `wt/w24f`, `a4e35e7 = c339ee8 + 4 round-1 logs`, "body not altered") is unverifiable without git history. Why it matters: gate evidence whose only chain-of-custody is a trust-me note can be retrofitted; a hostile reviewer cannot distinguish "annotated after" from "edited after". What correct looks like: capture-time headers (date, cmd, exit) written by the runner at commit time, or an orchestrator-side hash of the body recorded at capture. Confidence: HIGH — Verification: read; git check UNVERIFIED (git forbidden by AGENT-SPEC §2).
Mitigating cross-checks performed: worktree path leaks into the body independently (L4791-4792 reference `/home/z/my-project/wt/w24f/schemas/settings.toml`), and the round-1 failures are corroborated by the current tree's own comment in `crates/settings-schema/tests/integration.rs:109-112` ("phase-7 exit gate OF-2: c71cd5a's 27→32 growth stranded a pinned literal here") — the failures were real and later fixed (baseline: lumen-cli exit 0, settings-schema exit 0, `reverse_control...is_a_noop ... ok`).

F-002 [S3]
Where: docs/gates/phase7-exit.md:46 (claim about this log)
Code: (verbatim)
```
| 3 | `cargo test --workspace` (round 1, all 306 targets + doctests) | **FAIL** (exit 101): **4418 passed / 4 failed / 49 ignored** | `logs/phase7-exit-test.log` (`TEST_EXIT=101`; failing targets listed at end) |
```
What is wrong: "all 306 targets + doctests" misstates the body: there are 269 unit/integration targets plus 37 doctest blocks = 306 `test result:` LINES over 305 headers (codec-api's doctest block emits 2 result lines under one header). The counts (4418/4/49) are exact; the target count is a wording error in the citing gate report. Why it matters: gate reports get quoted downstream; "306 targets" will be counted as targets by future auditors and won't reconcile. What correct looks like: "306 test-result blocks (269 targets + 37 doctest blocks)". Confidence: HIGH — Verification: ran (block parse, 305 headers/306 result lines).

F-003 [S3]
Where: docs/gates/logs/phase7-exit-test.log:2431-2439 (evidence block) vs audit/runs/baseline/cargo-test-workspace.txt:2761-2793
Code: (verbatim, log side)
```
Running tests/evidence.rs (/home/z/my-project/rust-vlc/target/debug/deps/evidence-15fc9ed0b24f746a)
```
with result `test result: ok. 3 passed; 0 failed; 1 ignored` — while the orchestrator baseline (HEAD, 2026-11-23) records the same suite as `running 4 tests` → `test committed_transcripts_match_fresh_runs ... FAILED`, `test final_frame_is_current ... FAILED` (baseline L2763-2766), `test result: FAILED. 1 passed; 2 failed; 1 ignored`.
What is wrong: not wrong for its date — the log honestly records the first-vertical evidence suite PASSING at `c339ee8`. But gate report row 5 (`docs/gates/phase7-exit.md:48`, "first-vertical evidence suite … PASS (3 passed / 1 ignored)") is now stale against current HEAD reality: the same suite fails 2/4 today. Why it matters: anyone reading the Phase-7 exit report today believes first-vertical evidence is green; the orchestrator baseline proves it red. What correct looks like: gate report annotated with the HEAD regression (spec §13 records exactly these 2 failures). Confidence: HIGH — Verification: ran (grep/sed on baseline L2761-2793).

F-004 [S4]
Where: docs/gates/logs/phase7-exit-test.log:4-6
Code: (verbatim)
```
# Date: 2026-11-10, before 16:51:45Z (commit time of a4e35e7; exact capture time not recorded)
# Cmd: cargo test --workspace (report §2 #3); the body ran past all 4 failures through all
#   306 targets, i.e. --no-fail-fast behavior (inferred from the body; exact flag spelling not recorded)
```
What is wrong: capture metadata is missing (exact capture time) and the command line is reconstructed by inference, not recorded. Why it matters: evidence provenance is weaker than a raw capture; the inference is however corroborated by the body (all 306 blocks ran past 4 failures — verified). What correct looks like: capture `date -u` and the full command line in the header at run time. Confidence: HIGH — Verification: read + body corroboration.

F-005 [S4]
Where: docs/gates/logs/phase7-exit-test.log:106-107, 6039-6040 (representative)
Code: (verbatim, L6039-6040)
```
test rebrand_skeleton::about_model_renders_nova_logo ... ignored, boot tier: the unit-tier proof lives in tests/rebrand/rebrand_acceptance.rs::surface_about_facts_render_nova + ui/src/screens/settings/areas/updates-about.nova.test.tsx (card 4.H2); this placeholder keeps the booted-About/engineInfo assertion for the nightly tier — doc 07 §8.3 step 4
```
What is wrong: 49 tests are ignored in this run (vs 51 at HEAD baseline) with load-shedding annotations (throughput benches "numbers in the worklog entry", long fuzz runs, live-network tests, boot-tier placeholders in tools/xtask). The 4418-passed headline silently excludes all of them; each annotation names where the deferred proof lives, but those side-channels (worklog entries, nightly tier) are NOT in this log and are unverified here. Why it matters (limited): totals quoted from this log as "workspace tests pass" overstate coverage. Confidence: HIGH — Verification: read (line classification: 49 ignored lines) — the deferred side-channel evidence is UNVERIFIED.

## 4. Per-block table (data-audit substitute for per-function table; all 305 headers machine-checked: `running N` == p+f+i in 305/305)
| Block (log line) | run | p/f/i | Verdict | Notes |
|---|---|---|---|---|
| L2649 lumen_cli lib | 125 | 124/1/0 | FAILED | help::tests::areas_block_lists_the_six_areas (help.rs:310) |
| L4705 settings_schema lib | 43 | 42/1/0 | FAILED | seed entry count 32 vs 27 pin (lib.rs:155) |
| L4775 settings-schema --test integration | 7 | 6/1/0 | FAILED | committed_outputs_have_no_drift (integration.rs:117) |
| L6031 xtask --test cli | 15 | 11/1/3 | FAILED | rebrand reverse-control no-op, README.md byte drift |
| L11 audio_engine lib | 86 | 86/0/0 | ok | baseline identical (86) |
| L245 codecs lib | 506 | 506/0/0 | ok | baseline identical (506) |
| L2396-2484 first_vertical (7 blocks) | 10/2/5/4/7/24/1 | all ok | ok | evidence 3/0/1; journey/recovery_matrix/wire_mode ran here — baseline never reached them (per-package run aborted after evidence FAILED) |
| L4823-... streaming lib | 206 | 204/0/2 | ok | baseline 212/0/2 (+8 tests later) |
| 305 headers total | — | 4418/4/49 | — | codec-api doctest header (L6133) emits 2 result lines (L6140, L6145) |

## 5. Test honesty
| Recorded item | Would it fail if code were broken? | Reason |
|---|---|---|
| help::tests::areas_block_lists_the_six_areas | YES — it DID fail | real assert_eq on 7-vs-6 area list (L2783-2785) |
| tests::committed_seed_validates_against_committed_product | YES — it DID fail | real 32-vs-27 count assert (L4757-4759) |
| committed_outputs_have_no_drift | YES — it DID fail | stdout literally prints "no drift — outputs match (32 entries)" while the assert on the pinned count/exit fails (L4790-4792) — confusing but genuine; current code comment names this exact OF-2 root cause |
| rebrand_skeleton::reverse_control_real_product_toml_is_a_noop | YES — it DID fail | decoded byte arrays (3569 B vs 8032 B, first diff at offset 919: Phase-1 vs Phase-6 README) are real content, not constants |
The log proves a gate that FAILED and says so — no pass-rigging, no `|| true`, no redaction of failures.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cargo test --workspace (round 1 …) FAIL (exit 101): 4418 passed / 4 failed / 49 ignored" | docs/gates/phase7-exit.md:46 | REAL | tallies reproduce exactly (grep/awk on this log) |
| "TEST_EXIT=101; failing targets listed at end" | phase7-exit.md:46 | REAL | L6394-6399 verbatim: 4 targets + TEST_EXIT=101 |
| "first-vertical evidence suite … PASS (3 passed / 1 ignored-by-design regen)" | phase7-exit.md:48 | REAL for 2026-11-10 / STALE for HEAD | this log L2431-2439: 3/0/1; baseline L2761-2793: 1 passed 2 FAILED |
| Header provenance (worktree/commit/no-alteration) | phase7-exit-test.log:1-9 | REAL-UNPROVEN | body-internal corroborations only; git forbidden |
| "Date: 2026-11-10" | phase7-exit-test.log:4 | REAL-UNPROVEN | no in-body timestamp; baseline markers date 2026-11-23 |

## 7. Dead / duplicated / unreachable code
N/A (data file). Noted: all binary paths in `Running` lines point at `/home/z/my-project/rust-vlc/target/debug/deps/...` while panic bodies reference `/home/z/my-project/wt/w24f/...` — consistent with a worktree sharing the parent target dir (matches report header "worktree /home/z/my-project/wt/w24f"); no contradiction found in the body.

## 8. Dependencies & flags
`cargo test --workspace` (debug "test" profile, L10). No features asserted. 37 doctest blocks all ok. 2 slow-test notices (L853 av1_conformance >60s, finished 125.35s; L4031 soak_loops_stay_memory_bounded) — normal cargo output.

## 9. Open questions for the next agent
1. Verify via git (orchestrator only): a4e35e7 == c339ee8 + 4 round-1 logs; body of this file unaltered since a4e35e7.
2. Audit sibling logs `phase7-exit-test-rerun.log` (row 4) and `phase7-exit-evidence-rerun.log` (row 5) — same honesty standard.
3. The baseline's 2 first-vertical failures (committed_transcripts_match_fresh_runs, final_frame_is_current, baseline L2773-2792) post-date this log: who broke the transcripts between 11-10 and 11-23?
4. Baseline never ran first_vertical journey/recovery_matrix/wire_mode (per-package run aborted at evidence failure, no --no-fail-fast) — those 3 binaries' current status is UNVERIFIED at HEAD.
