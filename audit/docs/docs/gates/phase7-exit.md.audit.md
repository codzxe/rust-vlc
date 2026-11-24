# CLAIM AUDIT: docs/gates/phase7-exit.md
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Claims extracted | 14 |
| REAL 6 · REAL-UNPROVEN 3 · SHALLOW 1 · STUB 0 · PAPER-ONLY 0 · FALSE 1 · UNREACHABLE 0 · STALE-at-capture-but-now-false 3 (counted in FALSE/SHALLOW above) |
| Internal contradictions found | 0 (self-consistent; contradicts CURRENT tree in 3 places — §3) |

## 1. Blunt verdict
This doc does NOT claim "all gates passed" — its own §1 verdict is "**NOT PASSED at `c339ee8` — 5 open findings**" (L10-11), backed by a battery table with 4 FAIL rows and committed logs that match (`TEST_EXIT=101` at log L6399). It is the most honest gate doc audited so far in this wave: it records a retraction (OF-5), an environmental root cause (OF-4), and its own evidence gaps. Its defects are all STALENESS: three concrete claims are false against the current tree (2026-11-24), including the claim that the first-vertical evidence suite passes — that suite fails 2 tests today per the orchestrator baseline. Anyone citing phase7-exit.md as "everything was green by end of Phase 7" is misquoting it; anyone using it as current state is wrong on 3 claims.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "NOT PASSED at c339ee8 — 5 open findings (OF-1…OF-5)" | phase7-exit.md:10-11 | REAL | sed -n '8,27p' -> quote verified; battery rows 3,4,16,17 FAIL; log `TEST_EXIT=101` (phase7-exit-test.log:6399) |
| 2 | "18 of 22 battery checks green" | :14 | REAL | counted §2 table L44-65: PASS rows = 18, FAIL rows = 4 (#3,#4,#16,#17) |
| 3 | workspace test "**FAIL** (exit 101): **4418 passed / 4 failed / 49 ignored**" | :46 | REAL | docs/gates/logs/phase7-exit-test.log:6399 `TEST_EXIT=101`; log exists (11 phase7-* logs present) |
| 4 | first-vertical evidence suite "**PASS** (3 passed / 1 ignored-by-design `regen`)" | :48 | FALSE-on-current-tree (REAL at capture) | evidence-rerun.log body: `final_frame_is_current ... ok` + `committed_transcripts_match_fresh_runs ... ok` (binary evidence-2a22fc243c498b87, worktree wt/w24f); CURRENT baseline audit/runs/baseline/cargo-test-workspace.txt:2764/2767: both FAILED (transcript drift, evidence.rs:47:9/:162:5). The PASS does not reproduce today. |
| 5 | OF-1: "committed merge-conflict markers in `ui/src/App.tsx`" | :78-94 | REAL-at-capture / FIXED now | grep '^<<<<<<<' ui/src/App.tsx -> no hits; routes wired at ui/src/App.tsx:308-309 (`/discovery`, `/convert`) — fix 70ed69c landed |
| 6 | OF-2 pins (schema.len()==27 at lib.rs:155 etc.) | :96-112 | REAL-at-capture / FIXED now | crates/settings-schema/src/lib.rs:155-160 now derives counts ("DERIVED here ... not pinned as a second literal"); apps/cli/src/help.rs:312 test renamed `areas_block_lists_every_screen_the_seed_declares` |
| 7 | OF-5: matrix note claimed "compile-verified MF FFI that does not exist" | :157-180 | REAL | feature-matrix.toml H.264 row now reads "Media Foundation (Windows) NOT IMPLEMENTED — no MF lane exists (retracted in the module truth table...)"; hw/mod.rs:51-56 "NOT IMPLEMENTED — no MF lane exists ... retracted"; `ls crates/codecs/src/video/h264/hw/` -> capability.rs feed.rs frame_pool.rs mod.rs vaapi videotoolbox (no mediafoundation.rs) |
| 8 | 7.B "codecs suite green: 506 unit" | :227 | REAL | phase7-exit-test.log:753 `test result: ok. 506 passed; 0 failed; 0 ignored; ... finished in 2.88s` |
| 9 | parity "windows 64/252 · linux 64/252 · macos 64/252 · web 0/252" | :245-246 | SHALLOW (stale snapshot) | current baseline audit/runs/baseline/xtask-parity-report.txt:32/61: `windows=66/252 linux=61/252 macos=66/252 web=0/252` — numbers moved since 2026-11-10 |
| 10 | "verified-only (MP §1 strict): **0/252 on every platform**" | :246-248 | REAL | baseline xtask-parity-report.txt:59/61: TOTAL 0/252 on all four platforms |
| 11 | 7.K1: "`scripts/full_gate.sh` has no web-budget step" | :238 | FALSE-on-current-tree | scripts/full_gate.sh:70-74 has `web-budget --check` step ("=== 10. web-budget --check (issue #49 / doc 09 §2.4 ...)"); baseline full_gate.txt shows web-budget section red — the step was added after this doc |
| 12 | cargo-deny pin "SHA-256 `9f12ed4c…c8d5f` == `.github/workflows/ci.yml:469` pin" | :58 | REAL (substance) / stale line # | .github/workflows/ci.yml:484: `sha256 9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f` — hash matches doc prefix/suffix; line is 484, not 469 (drift) |
| 13 | #6 wasm32 build PASS, "lumen_web.wasm 5,998,961 B" | :49 | REAL-UNPROVEN | log exists (phase7-exit-wasm.log); corroborated by baseline rc-smoke.txt (wasm build passes); not rebuilt (cargo-build-budget) |
| 14 | #15 cargo-deny "advisories ok, bans ok, licenses ok, sources ok" | :58 | REAL-UNPROVEN | phase7-exit-deny.log exists; not re-run (cargo-deny binary was /tmp, session-bound); UNVERIFIED today |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| #5 evidence suite PASS (3 passed) | baseline cargo-test-workspace.txt:2764-2767 both tests FAILED | Baseline — current tree fails them (transcript drift); doc's PASS is from worktree wt/w24f @ a4e35e7, a different source state |
| 7.K1 "full_gate.sh has no web-budget step" | scripts/full_gate.sh:70-74 implements `cargo xtask web-budget --check` | full_gate.sh — step exists now; doc snapshot predates it |
| §5 parity 64/252 (win/linux/mac) | baseline parity windows=66, linux=61, macos=66 | Baseline — current-tree numbers differ; doc is a dated snapshot |

## 4. Fabrication / inflation indicators
None found. Opposite: the doc records a retraction of a prior false claim (OF-5), records environmental failure causes (OF-4 shared target dir), and labels un-run things `[VERIFY]`/`[PROPOSAL]` instead of claiming them. One soft spot: battery #22 (`xtask rebrand` reverse control) has "no log committed" — its PASS rests on a manual run plus a review doc on another branch (`w24/r3`), not on committed evidence (self-flagged at :65).

## 5. Matrix rows
Not a matrix chunk — no rows. Related row checked under claim #7 (feature-matrix.toml H.264 hw row: note now honest; status cells partial/partial/partial/not_started — consistent with hw/mod.rs truth table).
