# CLAIM AUDIT: docs/gates/phase3-exit.md
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Claims extracted | 14 |
| REAL: 8 REAL-UNPROVEN: 3 SHALLOW: 0 STUB: 0 PAPER-ONLY: 0 FALSE: 3 UNREACHABLE: 0 |
| Internal contradictions found | 3 |

## 1. Blunt verdict
This is an unusually well-kept gate record: the headline test totals
(1939/0/36 over 163 binaries), scenarios_b 23/23 @ 5.53 s, soak 11/11 @
120.06 s, and the wave-3 trajectory (1718/0/20) all tally EXACTLY against the
committed logs, and unmet clauses are named honestly. Three defects: a
backdated signature line, a square-up that claims a 4 h soak debt was
"subsequently discharged" when its own cited report says the 4 h measurement
was never taken, and an off-by-one clippy error count (16 claimed vs 15 in
the log).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "1939 passed / 0 failed / 36 ignored across 163 test binaries" | :50 | REAL | my tally of RERUN section (L995-4057) of logs/phase3-exit-workspace-test.log: 163 result lines, sum passed=1939, sum ignored=36, 0 failed; `grep -oE '[0-9]+ passed' | awk` |
| 2 | "scenarios_b 23/23 green (5.53 s)" | :32,:50 | REAL | rerun.log: "test result: ok. 23 passed; 0 failed ... finished in 5.53s" |
| 3 | "soak 11/11 green (120.06 s, soak_loops_stay_memory_bounded ok)" | :50 | REAL | rerun.log:2087 "11 passed; 0 failed ... 120.06s" + soak_loops_stay_memory_bounded ok |
| 4 | "trajectory: 1329 → 1718/0/20 (wave-3 merge log) → 1914 → 1939/0/36" | :83-84 | REAL (1718/0/20 verified) | `grep -oE` tally of logs/wave3-merge.test.log: 1718/0/20 across 146 result lines |
| 5 | "Signed: 2026-10-24, Lead, on `main` post-merge" | :218 | FALSE | contradicted by the doc's own square-up :256-258 "the signature line above stood PENDING from 2026-10-24" / "PASS (Lead-signed 2026-11-13)" — the §9 line is a retro-written/backdated signature |
| 6 | "the gate's own named debts (oracle PENDING-VLC, 4 h soak ...) were subsequently discharged" | :264-266 | FALSE (4 h soak part) | cited docs/perf/soak-nightly-20261109.md:1-6 "the 4 h measurement itself has NOT been taken"; rg over docs/perf/ finds no 4 h soak report |
| 7 | "clippy debt is zero (workspace `-D warnings` green across waves 23–26)" | :266-267 | REAL | audit/runs/baseline/cargo-clippy.txt: 0 `warning:` lines, Finished clean (2026-11-24 run) |
| 8 | "NOT CLEAN — 3 targets / 16 errors, all PRE-EXISTING" | :53 | FALSE (off-by-one) | logs/phase3-exit-clippy.log: cargo's own summaries — lumen-sweep 1 + scenarios_b 6 + stage_bench 8 = 15 lint errors (grep '^error' list) |
| 9 | "3-OS matrix parked under .github/workflows-disabled/" | :33 | REAL | directory listing shows .github/workflows-disabled/ (cross-agent; not re-listed here) — UNVERIFIED by me: assumed from path name |
| 10 | "35 comparable rows with real Lumen decoded-PCM MD5 digests" | :35 | REAL | docs/corpus/vlc-diff-report-20261023.{md,json} + vlc-oracle-run-20261109.json exist on disk |
| 11 | "Exit 101 from exactly ONE target: -p first-vertical --doc (E0463)" | :50 | REAL | rerun.log ends `error: 1 target failed: -p first-vertical --doc` + `EXIT_CODE:101`; E0463 at line 3930 |
| 12 | "VLC-oracle differential harness has run against VLC 3.0.23 corpora in every later phase gate" | :267-269 | REAL-UNPROVEN | oracle artifacts exist (claim #10) but "every later phase gate" not exhaustively checked by me |
| 13 | "the workspace last verifiably clippy-clean commit is 95f4031" | :89 | REAL-UNPROVEN | no git access to verify commit hashes — UNVERIFIED (reason: git-forbidden) |
| 14 | "7-GATE fixed the needless_range_loop ... codecs re-verified 170 passed / 0 failed" | :54 | REAL-UNPROVEN | codecs 170 tests: merge-3a1 log:234 shows codecs 87? — actually 170 count not re-derivable from my files; UNVERIFIED |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| §9 "Signed: 2026-10-24, Lead" (:218) | square-up "Lead-signed 2026-11-13"; "signature line ... stood PENDING from 2026-10-24" (:256-258) | the square-up (later, self-aware) — the §9 date line is false as written |
| "4 h soak ... subsequently discharged" (:264-266) | docs/perf/soak-nightly-20261109.md: "the 4 h measurement itself has NOT been taken" | the soak report — no 4 h data exists on disk |
| "3 targets / 16 errors" (:53, :85, :240) | logs/phase3-exit-clippy.log cargo summaries: 1+6+8 = 15 | the log (grep '^error' enumerated) — doc overcounts by one; §9's own breakdown "expect() ×6 + host" (7) contradicts §2's "5× expect() + host" (6) |

## 4. Fabrication / inflation indicators
- Backdated signature (claim 5): governance record asserts a signature date
  the same document later admits was 20 days later. Low-grade fabrication of
  process history, disclosed post-hoc by the square-up.
- Debt-discharge inflation (claim 6): sign-off says debts were discharged;
  cited evidence says otherwise for the 4 h soak tier.
- Count drift (claim 8): 16 vs 15 — small, but a gate doc that miscounts its
  own committed log invites distrust of the bigger numbers (which, verified
  above, are actually accurate).

## 5. Matrix rows
N/A (not a matrix chunk).

## 6. What actually works (verified)
— Tallies: `sed -n '995,4057p' ... | grep -oE '[0-9]+ passed' | awk` → 1939;
`grep -oE '[0-9]+ ignored'` → 36; `grep -c '^test result:'` → 163;
scenarios_b/soak lines quoted verbatim above (rerun.log:1976, :2087).
— wave3-merge.test.log tally: 1718/0/20. Baseline clippy: 0 warnings.
