# AUDIT: scripts/quality/21_perf.sh
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 38 / 5 sections (21a bloat, 21b llvm-lines, 21c harness census, 21d iai, 21e dhat census) |
| File verdict | REAL (reporter-tier: J in dynamic tier of 99_run_all.sh; honest about its own inability to detect perf regressions) |
| Reachable from shipped entry point? | YES as tooling — dynamic tier of scripts/quality/99_run_all.sh |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |

## 1. One-paragraph verdict
A perf-baseline REPORTER whose most valuable output is its own honest confession: I re-ran its two census greps (read-only) — the repo contains ZERO benchmark-harness crates (criterion/iai-callgrind/divan) and ZERO dhat usages, so the script's "no benchmark harness exists: perf regressions are currently undetectable (finding F-T21-01)" is TRUE on the current tree. The bloat/llvm-lines sections use real tools (with not_run discipline when absent). It cannot fail by design (J tier) — correct for a baseline recorder, and the undetectability finding is recorded rather than hidden.

## 2. What actually works (verified)
— EXECUTED the script's census logic by hand (read-only equivalents):
  - `grep -rln --include=Cargo.toml -E 'criterion|iai-callgrind|divan' crates apps tools | wc -l` -> **0** (ran)
  - `grep -rn --include=*.rs -E 'dhat::' crates apps tools | wc -l` -> **0** (ran)
  — both confirm the script's honest-gap claims against today's tree.
— 21a/21b/21d use `need`/`not_run` — no invented results when tools are absent.
— Note: 21b pipes `head -25 | tee` — tee sees a head-truncated stream (file gets 25 lines, not full output) — see F-002.

## 3. Findings

F-001 [S3]
Where: scripts/quality/21_perf.sh:25-27 (the finding it records)
Code:
```
if [ "$n" -eq 0 ]; then
  say "-> no benchmark harness exists: perf regressions are currently undetectable (finding F-T21-01)"
fi
```
What is wrong: not the script — the REPO STATE it reports: zero benchmark harness, zero dhat. Perf regressions in this codebase are undetectable by any in-tree mechanism (this reporter is the only perf instrument and it measures absence, not performance). Why it matters: any perf claim anywhere in docs (e.g. decode throughput, startup) has no regression tripwire. What correct looks like: land one criterion bench on the decode path. Confidence: HIGH — Verification: ran the census greps myself (both 0). Severity S3 for the domain (quality-lane gap, disclosed).

F-002 [S4]
Where: scripts/quality/21_perf.sh:17
Code:
```
  cargo llvm-lines -p playback-engine 2>&1 | head -25 | tee "$OUT/perf/llvm-lines.txt"
```
What is wrong: `head -25 | tee` — the tee'd FILE receives only 25 lines (head truncates upstream of tee), unlike 21a where tee precedes head. Inconsistent artifact completeness across sections (21a full, 21b truncated-by-design-accident). Confidence: HIGH — read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| 21a bloat | 8-13 | REAL | - | tee full then tail display — complete artifact |
| 21b llvm-lines | 15-20 | REAL | S4 | F-002 truncated artifact |
| 21c harness census | 22-27 | REAL | S3 | executed-equivalent -> 0; honest undetectability finding (F-001) |
| 21d iai | 29-34 | REAL | - | valgrind presence probe + not_run |
| 21e dhat census | 36-38 | REAL | - | executed-equivalent -> 0; D-1 directive unmeasured (disclosed) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| (reporter; no tests) | n/a | its census claims re-executed by me and TRUE |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no benchmark harness exists: perf regressions are currently undetectable" (conditional output) | L25-27 | REAL | census grep -> 0 (ran) |
| "0 = zero-allocation directive D-1 is unmeasured" | L38 | REAL | dhat grep -> 0 (ran) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
cargo-bloat / cargo-llvm-lines / valgrind / grep censuses. Writes under docs/audit/reports/perf/ (not executed by this audit — cargo sections + repo writes).

## 9. Open questions for the next agent
1. Does any doc claim measurable throughput ("539x realtime" in the flac record, perf-vs-VLC plans) without any bench harness? flac's 539x was a one-off measurement — no tripwire exists (F-001).
