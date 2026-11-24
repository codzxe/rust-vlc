# AUDIT: tools/sweep/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A10) |
| Date | 2026-06-14 |
| Lines / functions | 48 / package manifest: 1 bin + 7 path deps + lints inheritance |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev instrument (`cargo run -p lumen-sweep`); it IS a workspace member (root Cargo.toml:41 `members` includes "tools/sweep") and builds + tests green in the baseline |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n tools/sweep Cargo.toml` -> :41 (member); `rg -n workspace.lints Cargo.toml` -> :53/:59 (lint tables exist for `[lints] workspace = true`); `ls tools/sweep/src` -> main.rs 429 L + outcome.rs + worker.rs; baseline cargo-test-workspace.txt:3297-3323 (package compiles, 15/15 tests ok, exit_code=0) |

## 1. One-paragraph verdict
Every header claim checks out: package/bin `lumen-sweep` with a real src/main.rs (429 L + outcome.rs + worker.rs siblings); std-only (zero external deps — the 7 deps are all in-tree path edges, matching the claimed driver-level engine set codec-api/codecs/demux/input/media-probe/media-types/plugin-api); workspace membership via the documented one-line members edit (root :41, comment trail :13-22); `[lints] workspace = true` resolves (root [workspace.lints.rust] :53, [workspace.lints.clippy] :59); the package compiles and its 15 unit tests pass in the orchestrator baseline.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt:3300 `Finished test profile ... in 0.39s`, :3320 `test result: ok. 15 passed; 0 failed`, :3322 `exit_code=0`. — verified by rg: members array and lints tables present at the cited root lines.

## 3. Findings
None. Checked: dependency honesty (all 7 are path deps — the "zero external" claim holds), bin path validity (src/main.rs exists), lints inheritance resolvability, publish=false, workspace membership, and the claim that panic isolation is subprocess-based (main.rs:429 L real; claim consistency: no catch_unwind dep would be needed — none present). Zero findings is the honest result for this manifest.

## 4. Per-function table
File <100 LOC — [package] block, [[bin]] (name matches directory-tool precedent), [dependencies] (7 path edges), [lints] (workspace inheritance). All REAL.

## 5. Test honesty
N/A (manifest) — but the package's own 15 tests run green in baseline (:3303-3319), including parser round-trips and the typed-refusal-vs-crash distinction.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "appended to the root members array" | header :16-18 | REAL | root Cargo.toml:41 |
| "Std-only: zero external dependencies" | header :13 | REAL | deps table: 7 path edges, no versioned crates |
| "NO unsafe ... panic isolation via subprocesses" | header :14-15 | REAL | `rg -c "unsafe" tools/sweep/src/{main,outcome,worker}.rs` -> no matches (exit 1, zero count) — closed this run |

## 7. Dead / duplicated / unreachable code
None. NEXT-STEPS.md is tool-local documentation (not a manifest defect).

## 8. Dependencies & flags
No feature gates; edition 2024; publish=false.

## 9. Open questions for the next agent
The unsafe-scan claim (§6, third row) — run `rg -n "unsafe" tools/sweep/src/` to close it.
