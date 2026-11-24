# AUDIT: docs/gates/logs/phase7-exit-fmt.log (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines | 5 (header + `FMT_EXIT=0`) |
| File verdict | REAL as a historical snapshot — STALE vs current tree: fmt now FAILS |
| Reachable from shipped entry point? | N/A (gate evidence log) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | baseline cross-check: audit/runs/baseline/cargo-fmt.txt (`# cmd: cargo fmt --check (workspace)`, 2 `Diff in` sections, exit 1 per AGENT-SPEC §13); citation check docs/gates/phase7-exit.md:44 |

## 1. One-paragraph verdict
The log records `FMT_EXIT=0` at worktree w24/f (a4e35e7, 2026-11-10) with a proper header (worktree, date, cmd) — and it is provably stale: the orchestrator baseline on the CURRENT tree shows `cargo fmt --check` FAILED with 2 real diffs (crates/plugin-api/tests/testkit_wiring.rs and one more), exit 1. Together with the wave3-merge test log (0 failed then, 2 failed now), this is the core answer to the release/CI question: the committed gate logs and the current tree disagree — the tree regressed (fmt + first-vertical evidence drift) after the gates were cut, and no committed fmt-gate log reflects the current failure.

## 2. What actually works (verified)
— Log has provenance (worktree/branch/commit, ISO date, cmd) — better than clippy-r1's retro-annotated header.
— Citation consistent: docs/gates/phase7-exit.md:44 cites it as "PASS (exit 0, no diffs)" — true for the snapshot.
— Baseline contradiction captured: audit/runs/baseline/cargo-fmt.txt line 1 `# cmd: cargo fmt --check (workspace)` and `Diff in .../crates/plugin-api/tests/testkit_wiring.rs:40:` — 2 Diff sections; AGENT-SPEC §13 records fmt FAILED (exit 1).

## 3. Findings
F-001 [S3] — Gate log contradicts current-tree reality (stale-snapshot class, same as wave3-merge.test.log).
Where: docs/gates/logs/phase7-exit-fmt.log:5 vs audit/runs/baseline/cargo-fmt.txt
Code (both sides, verbatim):
```
# Date: 2026-11-10T16:56:26Z
# Cmd: cargo fmt --check
FMT_EXIT=0
```
vs baseline:
```
# cmd: cargo fmt --check (workspace)
Diff in /home/z/my-project/audit-workspace/crates/plugin-api/tests/testkit_wiring.rs:40:
```
What is wrong: fmt was clean at a4e35e7 (2026-11-10) and fails on the current tree (baseline, 2026-11-23): formatting drift landed post-gate with no committed re-run. The gate doc's own instruction `cargo fmt --check  # expect 0` (phase7-exit.md:310) now yields exit 1. Why it matters: any "fmt clean" claim citing this log is false for the current tree; the drift is invisible to anyone trusting committed gate evidence. Mitigation: the live ci.yml `lint` job (fmt #1, ci.yml:34) would catch this on any PR since the CI unblock — the committed-evidence lane is what's stale, not necessarily the live gate. Confidence: HIGH — Verification: read + baseline cross-check (no rustfmt execution here — no cargo/rustc in this sandbox).

## 4. Claim cross-references
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "PASS (exit 0, no diffs)" | docs/gates/phase7-exit.md:44 | TRUE-at-a4e35e7 / FALSE-now | baseline cargo-fmt.txt: 2 diffs, exit 1 |
| "round 1's fmt log was empty with no exit marker, so fmt was re-run in r2 with a recorded header" | phase7-exit.md:69-70 | REAL (this r2 log is the one with the header) | header present L1-4 |

## 5. Open questions for the next agent
- Which commit introduced the formatting drift after 2026-11-10? Git-lane question; flag for the fmt owner. The 2 diffs are in crates/plugin-api/tests/testkit_wiring.rs (+1 more section).
