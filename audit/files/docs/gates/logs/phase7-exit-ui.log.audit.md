# AUDIT: docs/gates/logs/phase7-exit-ui.log (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines | 303 (read in full) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A — gate evidence log |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read (full), rg for conflict markers in ui/src/App.tsx (none), sed quotes, rg citations |

## 1. One-paragraph verdict
A genuine, honest FAILING gate log: UI typecheck exit 2 (merge-conflict
markers in App.tsx), lint exit 1, vitest 705/706 passed with 2 suites failing
to load (same markers) plus 1 settings-drift failure, l10n:check exit 0.
phase7-exit.md cites it accurately as FAIL (rows 16-18). The captured defects
(App.tsx conflict markers OF-1, settings drift OF-3) no longer exist on the
current tree — consistent with later fixes, so this log is period evidence,
not current evidence.

## 2. What actually works (verified)
— rg '^<<<<<<<|^=======$|^>>>>>>>' ui/src/App.tsx -> No matches (current tree
has no markers; the log's defect was since fixed, matching worklog claims).
— ui/src/App.test.tsx and ui/src/i18n/pseudo.render.test.tsx exist today, and
ui/src/screens/settings/generated.test.ts still contains the `--check`
assertion around L80 (verified the execFileSync harness). The log's named
files are real.
— Exit codes are recorded per command (`UI_EXIT[typecheck]=2`,
`UI_EXIT[lint]=1`, `UI_EXIT[test]=1`, `UI_EXIT[l10n:check]=0`), honest.

## 3. Findings
F-001 [S3]
Where: docs/gates/logs/phase7-exit-ui.log:2 and 224-227
Code:
```
# Worktree: /home/z/my-project/wt/w24f (branch w24/f @ a4e35e7)
```
```
 ❯ transformWithOxc ../../../rust-vlc/ui/node_modules/vite/dist/node/chunks/node.js:7618:19
```
What is wrong: the header names worktree /home/z/my-project/wt/w24f while the
stack traces resolve node_modules from a sibling checkout named `rust-vlc`
— the log's provenance (machine/checkout) is internally inconsistent with the
stated worktree path, so the log cannot be pinned to this repo layout by its
own content. Also "Snapshots 9 written" (L289) — the gate run WROTE
snapshots, so it is not a hermetic read-only check.
Confidence: MEDIUM — Verification: read.
F-002 [S4]
Where: docs/gates/logs/phase7-exit-ui.log:290-291
Code:
```
 Test Files  3 failed | 77 passed (80)
      Tests  1 failed | 705 passed (706)
```
What is wrong: 2 of the 3 "failed files" are uncollected suites (0 tests
run), so "706 tests" understates what a green run should collect; the
headline number mixes load-failures with assertion-failures. The gate doc
phase7-exit.md:60 discloses this honestly, so impact is documentation clarity
only.
Confidence: HIGH — Verification: read.

## 4. Data summary (no functions — log)
Commands: npm run typecheck / lint / test / l10n:check in worktree w24f
(node v24.22.2, npm 11.19.1, 2026-11-10T16:58:12Z). Results: 2/1/1/0.
Vitest: 80 files, 77 passed, 3 failed; 706 tests, 705 passed, 1 failed;
9 snapshots written; duration 75.37s. l10n: 611 keys, missing 0, dups 0,
orphans 0, placeholders 0.

## 5. Test honesty (of the gate the log feeds)
The settings-drift test that failed here (`passes settings:check against the
committed generated files`) is a mutation-honest test — its siblings
(lines 141-143) verify the checker FAILS on mutated copies. Real evidence of
drift at the time; today the equivalent suite passes (baseline ui-test.txt).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "UI npm run typecheck / lint — FAIL (TS1185/TS1005 parse errors; eslint 1 error) — OF-1" | docs/gates/phase7-exit.md:59 | REAL | log L7-27, L28 `UI_EXIT[typecheck]=2`, L91/L93 |
| "UI npm run test — FAIL: 705/706; 2 suites fail to load, 1 failure settings-drift (OF-3)" | docs/gates/phase7-exit.md:60 | REAL | log L290-291, L197/L231, L269 |
| "UI npm run l10n:check — PASS (611 keys...)" | docs/gates/phase7-exit.md:61 | REAL | log L301-303 |

## 7. Dead / duplicated code
N/A.

## 8. Dependencies & flags
N/A (log). Note: stack paths prove node_modules lived under the `rust-vlc`
checkout name.

## 9. Open questions for the next agent
Who rewrote the `# Worktree:` header lines (L1-4 are hand-added comments) —
the log is a curated paste, not a raw script capture; check gate scripts for
a logger that emits `UI_EXIT[...]` markers.
