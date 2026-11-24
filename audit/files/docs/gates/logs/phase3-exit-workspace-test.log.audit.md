# AUDIT: docs/gates/logs/phase3-exit-workspace-test.log
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Date | 2026-10-30 |
| Lines / functions | 4057 / n/a (data log: 2 cargo test invocations) |
| File verdict | REAL (as data) — but the run it records FAILED both times |
| Reachable from shipped entry point? | N/A — committed gate-evidence artifact (docs/) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 2 |
| Commands run | wc -l (4057); rg "test result:" (211 lines); awk sums; sed -n spot checks (all exit 0) |

## 1. One-paragraph verdict
This is a genuine cargo `test` transcript (varying times, per-binary hashes, realistic soak timings) — not fabricated. But it is committed as *phase exit evidence* and it records TWO runs that BOTH end in `EXIT_CODE:101`. As gate evidence it does not prove a green workspace gate; `docs/gates/phase3-exit.md` then quotes only the middle (1939/0/36 across 163 binaries — which I verified is accurate) while asserting an off-log "targeted retry" that does not exist anywhere in this committed log.

## 2. What actually works (verified)
- `sed -n '994,4057p' … | grep -c '^\s*Running '` → 139; `grep -cP '^\s*Doc-tests '` → 24; 139+24 = **163 test binaries**, exactly the count phase3-exit.md claims (L50).
- awk sum over the 163 `test result:` lines of the RERUN: **passed=1939 failed=0 ignored=36** — matches phase3-exit.md row 1b verbatim.
- `sed -n '2995p'` → scenarios_b 23/23 in 5.53s; soak 11/11 in 120.06s (L3063 region) — matches the doc's claimed numbers.
- First run (L1-993) honestly records its own failure: run1 = 559 passed / 1 failed / 16 ignored, aborted fail-fast, `EXIT_CODE:101` at L993.

## 3. Findings
F-001 [S2]
Where: docs/gates/logs/phase3-exit-workspace-test.log:976-990
Code:
```
test committed_transcripts_match_fresh_runs ... FAILED
...
test result: FAILED. 2 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.01s
```
What is wrong: the first invocation of the phase-3 workspace gate fails on `committed_transcripts_match_fresh_runs` (lumen bin missing at the relative worktree path). Why it matters: the "committed transcripts match fresh runs" invariant — the evidence-integrity test the exit gate leans on — did not run to completion in run 1; it only passes in run 2 after an env fix applied BETWEEN runs (log L997 `Compiling lumen-cli`). The committed proof of evidence integrity is conditional on manual intervention. What correct looks like: a gate log where the recorded invocation passes without mid-run human fixes. Confidence: HIGH — Verification: read + awk sums.

F-002 [S2]
Where: docs/gates/logs/phase3-exit-workspace-test.log:3929-3936, 4055-4057
Code:
```
   Doc-tests first_vertical
error[E0463]: can't find crate for `lumen_cli`
  --> tests/integration/first-vertical/src/harness.rs:40:5
...
error: 1 target failed:
    `-p first-vertical --doc`
EXIT_CODE:101
```
What is wrong: the RERUN — the run the gate doc cites as its pass evidence — ends in failure: the first-vertical doctest cannot compile (E0463), so the overall exit is 101. Why it matters: the committed exit log's bottom line is a failure; any doc statement that "phase 3 workspace gate passed" is contradicted by this artifact. The rustdoc invocation at L3942 even passes `--extern lumen_cli=.../liblumen_cli-....rlib` yet the crate "can't be found" — consistent with a stale shared-target-dir artifact, i.e. the run was polluted by the environment, weakening all of it as evidence. What correct looks like: rerun until the recorded gate run is fully green, or record the failing exit as the gate result. Confidence: HIGH — Verification: read.

F-003 [S3]
Where: docs/gates/logs/phase3-exit-workspace-test.log:995-996
Code:
```
===== RERUN (7-GATE): cargo test --workspace --no-fail-fast =====
# context: first run (above) aborted at first-vertical evidence — the lumen bin was not at the test's relative path (worktree target symlink now points at the config-pinned shared dir; bin rebuilt from THIS worktree, provenance line above). --no-fail-fast for the full per-crate picture.
```
What is wrong: the narration cites a "provenance line above" — no such line exists anywhere before L995 in this log (L1-994 are raw cargo output ending at `EXIT_CODE:101`). The referenced evidence is absent from the file. Why it matters: gate narration pointing at evidence that isn't committed is exactly the pattern of uncheckable claims. Confidence: HIGH — Verification: read (rg for "provenance" → only L996).

F-004 [S4]
Where: docs/gates/logs/phase3-exit-workspace-test.log:1
Code:
```
   Compiling media-types v0.1.0 (/home/z/my-project/worktrees/w7-gate/crates/media-types)
```
What is wrong: the log has no invocation header for run 1 (command line, date, toolchain) — it starts mid-stream at "Compiling"; only the RERUN gets a `=====` header. No timestamp anywhere in 4057 lines (rg for dates → only test names). A gate log that can't say when or exactly how it was produced. Confidence: HIGH — Verification: read + rg.

F-005 [S4]
Where: whole file
What is wrong: mixed-provenance paths (compile lines reference `/home/z/my-project/worktrees/w7-gate/...`, all executed test binaries reference `/home/z/my-project/target/debug/deps/...`) — explained only by the L996 narration, never by recorded env/config. Why it matters: the log is not reproducible as-committed (depends on an unrecorded CARGO_TARGET_DIR/symlink state). Confidence: HIGH.

## 4. Per-function table
N/A (data log). Structure inventory instead:
| Section | Lines | Content | Verified |
|---|---|---|---|
| Run 1 (fail-fast) | 1-993 | 42 suites, 559/1/16, abort at first-vertical evidence | awk count |
| RERUN header + narration | 994-996 | `--no-fail-fast` context | sed |
| Run 2 body | 997-4054 | 163 binaries, 1939/0/36, scenarios_b 23@5.53s, soak 11@120.06s | awk count |
| Doctest failure + exit | 3929-4057 | E0463, `error: 1 target failed: -p first-vertical --doc`, EXIT_CODE:101 | sed |

## 5. Test honesty
N/A — the log is itself the test evidence; its honesty checked above (real timings, honest failure records).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "1939 passed / 0 failed / 36 ignored across 163 test binaries" | docs/gates/phase3-exit.md:50 | REAL | awk over L994-4057: 1939/0/36; 139 Running + 24 Doc-tests headers = 163 |
| "Exit 101 from exactly ONE target: -p first-vertical --doc" | docs/gates/phase3-exit.md:50 | REAL | L4055-4057 quoted above |
| "PASSES on immediate targeted retry (running 0 tests … ok)" | docs/gates/phase3-exit.md:50 | **FALSE-as-evidence / UNVERIFIED** — no retry section exists in this log; and "running 0 tests" is a vacuous pass | rg "retry" in log → no retry invocation recorded |
| "soak 11/11 green (120.06s)" | docs/gates/phase3-exit.md:50 | REAL | L3081: `test result: ok. 11 passed; 0 failed; ... finished in 120.06s` |

## 7. Dead / duplicated / unreachable code
Run 1's 42 suites are duplicated at the start of run 2 (same suites re-run) — expected for a rerun, not a defect; noted so no one counts run-1 suites twice (total distinct = 163).

## 8. Dependencies & flags
Log references toolchain 1.99.1 (L3942 rustdoc path), edition 2024, `--deny` lint set recorded only via the rustdoc invocation — good detail, only place it is captured.

## 9. Open questions for the next agent
- Does the Lead's promised re-run on main (phase3-exit.md "[VERIFY: Lead re-run on main]") exist anywhere? Not in this repo's committed logs — check before trusting phase 3 exit.
- The E0463-with---extern paradox (crate given but "can't find") is unexplained by any committed artifact.
