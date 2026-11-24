# AUDIT: docs/gates/logs/phase4-exit-ui-vitest.log (DATA AUDIT — log)
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines | 83 |
| File verdict | REAL (authentic vitest run log; arithmetic verified exactly) |
| Reachable from shipped entry point? | N/A (log data; cited by docs/gates/phase4-exit.md:58,204) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 regex sum (59 files / 557 tests — matches summary); rg citation check |

## 1. One-paragraph verdict
Genuine vitest v5.0.5 run from worktree `p4-gate`: 59 test files, 557 tests,
ALL green ("Test Files 59 passed (59)" / "Tests 557 passed (557)"). The
arithmetic is exact — I summed the 59 per-file test counts and got 557,
matching the summary line. The citing doc's claim (phase4-exit.md:58 "59
files / 557 tests passed, 0 failed") is accurate. Two honest-data blemishes:
"Snapshots 9 written" means 9 snapshots were CREATED this run (first-run
generation, not pinned-snapshot verification), and the log captures a real
React DOM warning (`Invalid DOM property 'stroke-width'. Did you mean
'strokeWidth'?`) leaking from the Icon component's tests — a live code smell
preserved in evidence. No exit-code line is embedded (vitest list output).

## 2. What actually works (verified)
- `python3` re-parse: 59 `✓` file lines, `total tests: 557` — EXACT match to the summary "557 passed (557)".
- Citation: phase4-exit.md:58 quotes "59 files / 557 tests passed, 0 failed" — accurate against the log.
- Cross-time consistency: current baseline UI vitest = 1705 passed / 1 skipped (spec §13) — clean evolution from 557; no regression signal in the numbers.

## 3. Findings
F-001 [S4]
Where: docs/gates/logs/phase4-exit-ui-vitest.log:50-53
Code:
```
stderr | src/components/icon/Icon.test.tsx > Icon (1.5-stroke set, doc 08 §1.4/§1.5) > every registry name renders a glyph
Invalid DOM property `stroke-width`. Did you mean `strokeWidth`?
```
What is wrong: the Icon component emits a DOM property React rejects
(`stroke-width` instead of `strokeWidth`) — the warning leaks on every glyph
render; the test still passes because the warning is non-fatal. Why it
matters: real component defect visible in committed evidence (console noise
in every app run that renders icons). Confidence: HIGH — Verification: read
(the log preserved the stderr).
F-002 [S4]
Where: docs/gates/logs/phase4-exit-ui-vitest.log:74
Code:
```
  Snapshots  9 written
```
What is wrong: "written" = created, not compared — the run generated 9 new
snapshots (first-run artifact); this is snapshot-SEEDING evidence, weaker
than snapshot-verification evidence. phase4-exit.md:58 presents the run as
"the full UI unit gate" without noting the 9 fresh snapshots.
Confidence: HIGH — Verification: read (vitest semantics).

## 4. Data integrity checks
- 59 file lines; sum(per-file tests) = 557 == summary. CONSISTENT.
- 0 failures, 0 skipped in this log (skipped appears only in the LATER baseline, 1 skipped).
- Provenance: worktree p4-gate (path prefix), vitest v5.0.5, duration 63.53s
  with environment breakdown — plausible formatting, no anomalies.
- Not re-runnable by me (npm test budget); log treated as data per rules.

## 5. Provenance note
Produced in `/home/z/my-project/worktrees/p4-gate/ui` — the phase-4 gate
worktree, consistent with the sibling phase4-exit-playwright.log's origin.
