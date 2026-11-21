# AUDIT: ui/.vitest/json/output.json (data-dump — provenance / reproducibility)
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / size | 1 JSON document (single line), 32,112 bytes |
| File verdict | STALE-ARTIFACT (committed, orphaned, red-at-birth — characterized below; as DATA it is REAL and internally consistent) |
| Reachable from shipped entry point? | N/A (test-report artifact) — consumed by NOTHING: `rg -ln "\.vitest"` over ui/, .github/, docs/ returns zero references; no package.json script emits it (`"test": "vitest run"` uses the default reporter); CI (.github/workflows/ui.yml:120-122 `npm run test`) neither produces nor reads it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | python3 json parse (full structure: 12 summary keys + testResults); suite/test count cross-check (status counts 11 passed + 8 failed FILES; Σassertions 73 == numPassedTests 73 — payload internally consistent; numTotalTestSuites 36 counts describe-blocks, not files); failed-suite message extraction (`Failed to resolve import "./controls" from "src/screens/settings/areas/index.tsx"`); startTime decode → **2026-11-11T14:48:31 UTC** (10 days before baseline, 10 days before this workspace's checkout); path provenance: every testResults[].name is absolute under **/home/z/my-project/rust-vlc/** — the Lead's original checkout, NOT this audit workspace; `rg -ln "\.vitest"` (zero consumers); `cat ui/.gitignore` (11 lines — `.vitest` NOT ignored) + root .gitignore (only a comment about committed DOM snapshots); existence check: all 19 suite files exist at HEAD; `rg -n '"./controls"' src/screens/settings/areas/index.tsx` → :49 `import { ControlsArea } from "./controls"` and controls.tsx exists (the historical failure resolved at HEAD); baseline #23 comparison (136 files / 1705 passed vs this artifact's 19 files / 73 tests / 8 failed). |

## 1. One-paragraph verdict
This is not baseline evidence and was never meant to be: it is a developer's ad-hoc `vitest --reporter=json --outputFile` run from **2026-11-11T14:48:31 UTC** (ten days before the baseline and the audit checkout), executed in the Lead's original tree (`/home/z/my-project/rust-vlc/...` — every suite path proves it), covering only the settings lane (19 of the 136 test files), and it was RED at birth: `success: false` with 8 settings suites failing collection on a mid-wave import error (`Failed to resolve import "./controls" from "src/screens/settings/areas/index.tsx"`) that is resolved at HEAD. As a data artifact it is well-formed and internally consistent (73 assertions in the payload = 73 numPassedTests; the 36/28 suite counters count describe-blocks, not files — not a corruption). As REPOSITORY CONTENT it is a defect: a stale, failing, ten-day-older test report, tracked by git (not in .gitignore; present in this fresh checkout), produced by no script, consumed by no tool, silently contradicting the tree's actual green state (baseline #23: 136/136 files, 1705 passed).

## 2. What actually works (verified)
— Verified by python3 parse: the JSON is valid and self-consistent — 19 testResults entries (11 passed files with 73 passing assertions, 8 failed files with zero assertions and a resolution-error message each), Σassertions == numPassedTests, snapshot block all-zero (no snapshot activity in that run — consistent: the run excludes the snapshot suites).
— Verified by path decode: provenance is unambiguous — absolute paths under `/home/z/my-project/rust-vlc/ui/`, startTime 2026-11-11T14:48:31Z. This artifact documents a REAL moment of the w26 development history: the settings area barrel importing `./controls` before that file landed, killing 8 suites at collection time.
— Verified by rg + read: at HEAD the historical failure is gone — areas/index.tsx:49 imports `./controls`, controls.tsx exists, and baseline #23 runs all 136 files green (including all 19 suites named in this artifact, e.g. advanced.test.tsx appears passing in the baseline log's sample lines).

## 3. Findings

### F-001 [S3]
Where: ui/.vitest/json/output.json (the artifact as committed; summary `success: false` + the 8 failed testResults entries)
Code:
```json
"message": "Failed to resolve import \"./controls\" from \"src/screens/settings/areas/index.tsx\". Does the file exist?"
```
(attributed to `src/screens/settings/SettingsScreen.test.tsx` and 7 sibling suites in the payload)
What is wrong: the repository ships a test report that says the settings test suites FAIL (`success: false`, 8 failed suites) — stale by ten days, partial (19/136 files), and from a different working tree — while the actual tree at HEAD is green (baseline #23). Anyone or anything reading this file as "the UI test run" (a naive CI summary scraper, an AI agent summarizing the repo, a future auditor) concludes 8 suites are broken.
Why it matters: misleading committed evidence — the exact class of artifact a hostile audit exists to catch. It also leaks the Lead's local checkout path (`/home/z/my-project/rust-vlc/`) into the repo, the same hard-coded-path smell the baseline INDEX already flagged for full_gate.sh (Wave-17 item).
What correct looks like: delete the file and add `.vitest/` to ui/.gitignore (reporter output is derived data); if a json report is ever wanted, wire the reporter in vite.config.ts's test block + a package.json script so it is regenerated deterministically instead of committed once by accident.
Confidence: HIGH — Verification: full parse + counts + message extraction + gitignore read + zero-consumer rg + baseline #23 comparison. Tracked-ness inferred from presence at checkout time (Nov 21 16:48, clone timestamp) with no .gitignore entry — git itself not run (session-2 deviation).

### F-002 [S4]
Where: ui/.vitest/json/output.json (its orphaned production chain) + ui/package.json:14
Code:
```
14:    "test": "vitest run",
```
What is wrong: nothing in the repository can regenerate this file: no script passes `--reporter=json --outputFile`, vite.config.ts's `test` block sets no reporters, CI runs plain `npm run test`. The artifact was produced by an unrecorded manual invocation — the definition of non-reproducible provenance — and nothing consumes it either.
Why it matters: violates the data-artifact contract both ways (no producer, no consumer); the only way it changes is another accidental manual run overwriting it.
What correct looks like: see F-001 — ignore it or wire it; the current state (tracked + unproducible + unread) is the worst of the three options.
Confidence: HIGH — Verification: package.json scripts read, vite.config.ts test block read, .github/workflows/ui.yml:120-122 read, repo-wide rg zero refs.

## 4. Per-function (field-group) table
| Field group | Content | Verdict | Severity | Notes |
|---|---|---|---|---|
| summary counters | 36 suites (28 pass/8 fail), 73 tests (73 pass/0 fail/0 todo), success:false | REAL (self-consistent) | F-001 | "Suites" = describe-blocks; failed files count as failed suites. Counts match payload exactly. |
| snapshot block | all zeros, didUpdate:false | REAL | — | No snapshot activity in that run. |
| startTime | 1794408511179 → 2026-11-11T14:48:31Z | REAL | — | Provenance anchor; 10 days pre-baseline. |
| testResults (19 files) | 11 passed (73 assertions) + 8 failed (0 assertions, import-resolution message) | REAL (historical) | F-001 | All 19 files exist at HEAD and pass in baseline #23; the failure cause (missing ./controls) is fixed at HEAD (index.tsx:49). |

## 5. Test honesty
Not applicable (this IS a test output). The artifact's own honesty is good in one specific way: it records a real failure with the real resolver message rather than massaged counts — as historical data it is truthful; as committed repo content it is misleading (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| (none — no doc references it) | — | — | rg over ui/, docs/, .github/ finds zero mentions; the artifact is undocumented — no way to tell design from accident, which is itself the finding. |
| (potential misreading) "this file shows the UI test status" | a naive reader | FALSE-at-HEAD | success:false + 8 failed suites vs baseline #23's 136 files / 1705 passed, 0 failed. The artifact describes 2026-11-11, not HEAD. |

## 7. Dead / duplicated / unreachable code
- The file is pure dead weight: orphan artifact, zero producers, zero consumers, gitignored nowhere, superseded by every later green run.

## 8. Dependencies & flags
- Vitest 5.0.5 json-reporter schema (jest-compatible shape). No flags. No config references.

## 9. Open questions for the next agent
- If the team ever wants json reports in CI, wire `.vitest/` as the artifact dir (F-002's fix) and un-track the stale file in the same commit; otherwise F-001's delete+ignore stands.
- The Lead's checkout path `/home/z/my-project/rust-vlc` leaking into a tracked file joins the baseline's full_gate.sh hard-coded-path finding (INDEX, Wave-17 item) — worth one consolidated "local paths in committed artifacts" cross-wave finding.
