# AUDIT: docs/gates/logs/wave1-merge.test.log (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A10 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines | 15 (5-section gate summary + verdict) |
| File verdict | REAL (genuine Wave-1-era gate summary; corroborated exactly by the archived worklog) |
| Reachable from shipped entry point? | N/A — Wave-1 merge gate evidence log |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | archive worklog:4241 cross-read -> numbers match EXACTLY; `ls /tmp/w1-gate4.log` -> GONE; `ls target/architecture-report.json` -> ABSENT; fixture census today -> 164 files (vs 52) |

## 1. One-paragraph verdict
A real Wave-1 merge-gate summary whose every number reproduces the archived worklog entry verbatim (1493/0/15, fmt CLEAN, clippy CLEAN, arch 5/5, fixtures no-drift 52) and whose verdict line is embedded. It is summary-only — no per-test output, no binary hashes — and two of its pointers are dead today: the raw transcript it mirrors from (/tmp/w1-gate4.log) and the architecture report it cites (target/…) are both gone, and the fixtures line carries the pre-sandbox absolute path. Today's tree has 164 fixture files and 4744 passing tests (baseline) — this log describes a much earlier era, honestly.

## 2. What actually works (verified)
— verified by cross-reading docs/worklog/archive-2026-10.md:4241:
`- FINAL GATE: PASS — fmt CLEAN, clippy CLEAN, tests 1493 passed / 0 failed / 15 ignored (baseline ~1362 → +131), arch 5/5, fixtures no-drift 52. Raw log: /tmp/w1-gate4.log (mirrored into docs/gates/logs/wave1-merge.test.log by this commit).`
Every number in the log (fmt/clippy CLEAN, 1493/0/15, arch 5/5 pass list, 52 fixtures, PASS) transcribes faithfully, and the worklog explicitly names this exact file as the mirror.
— verified: `=== GATE RESULT: PASS ===` (:15) — the verdict is embedded in the log itself, not only in the citing doc.
— verified era consistency: 1493 tests at Wave-1 vs the worklog's stated baseline ~1362 → +131 growth arithmetic (1362+131=1493, exact).

## 3. Findings
F-001 [S4]
Where: docs/gates/logs/wave1-merge.test.log:1-15 (whole file)
Code: `=== 3. test ===
tests: 1493 passed / 0 failed / 15 ignored
test: CLEAN`
What is wrong: summary-only evidence — no per-test output, no test-binary hashes, no command/date capture header; the raw transcript (/tmp/w1-gate4.log, per archive:4241) is GONE. Contrast the Wave-2 sibling (wave2-merge.test.log, 2316-line full transcript, audited by W17-A05) — this log's auditability rests entirely on the worklog cross-reference.
Confidence: HIGH — Verification: ran (ls + cross-read).

F-002 [S4]
Where: docs/gates/logs/wave1-merge.test.log:10, 13
Code: `full log: target/architecture-report.json` / `result: no drift — 52 file(s) verified under /home/z/my-project/tests/fixtures`
What is wrong: both in-log pointers are dead today — target/architecture-report.json ABSENT (gitignored build dir, never committed), and the absolute path /home/z/my-project/tests/fixtures is the pre-sandbox checkout layout (today the tree lives at .../audit-workspace with 164 fixture files vs the 52 of that era). Evidence readers cannot follow either pointer.
Confidence: HIGH — Verification: ran (ls + find census).

## 4. Test honesty
No tests in the log itself; it is the record of a 1493-test run whose per-test evidence no longer exists. Today's tree: 4744 passed / 2 failed / 51 ignored (AGENT-SPEC §13 baseline) — the "0 failed" claim is era-true, not current-tree-true.

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "tests: 1493 passed / 0 failed / 15 ignored" | :6 | REAL-at-the-era | archive:4241 identical; growth arithmetic 1362+131 exact |
| "architecture: result=pass edges=pass cycles=pass unsafe-lints=pass static-mut=pass testkit-direction=pass" | :9 | REAL-at-the-era (5/5) | archive:4241 "arch 5/5" |
| "fixtures ... no drift — 52 file(s)" | :13 | REAL-at-the-era / STALE-now | today 164 fixture files (census) |
| "=== GATE RESULT: PASS ===" | :15 | REAL-at-the-era | all five sections CLEAN/PASS in this log |

## 6. Dead / duplicated / unreachable code
N/A (data artifact). Dead pointers covered by F-002.

## 7. Dependencies & flags
N/A — captured gate.sh-era summary output; consuming doc: docs/worklog/archive-2026-10.md:4241.

## 8. Open questions for the next agent
- None. No fabrication indicators; the worklog's mirror note makes provenance explicit.
