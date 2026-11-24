# AUDIT: tools/perf/r1_ff_lanes.py
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Lines / functions | 46 / 2 (bench, main) |
| File verdict | REAL (executed: `python3 tools/perf/r1_ff_lanes.py` → header + exit 0) |
| Reachable from shipped entry point? | NO — dev measurement tool (card 5.R1 breadth snapshot), invoked ad hoc by the perf lanes |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read; `python3 tools/perf/r1_ff_lanes.py` (no args) → `lane,n,med_s,min_s,max_s`, exit 0 |

## 1. One-paragraph verdict
A real, minimal ffmpeg reference-timing script: fresh process per run,
warm-up discarded, median-of-5, wall clock via time.monotonic around the
subprocess, DNF row on nonzero rc. It does what its docstring says; two
hygiene defects only.

## 2. What actually works (verified)
— `python3 tools/perf/r1_ff_lanes.py` → prints CSV header, exit 0
(behavior matches docstring for the zero-file case).

## 3. Findings
F-001 [S4]
Where: r1_ff_lanes.py:33-42
Code: `return 0` (main) — unconditionally.
What is wrong: DNF lanes (ffmpeg rc != 0) still yield process exit 0; a
driver gating on exit code sees success on total failure. What correct
looks like: nonzero exit if any lane DNF'd, or a summary line.
Confidence: HIGH — Verification: read + executed no-arg path.
F-002 [S4]
Where: r1_ff_lanes.py:21-23
Code: `subprocess.run(..., timeout=300)` without try/except.
What is wrong: a timeout raises TimeoutExpired → traceback, no DNF row.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| bench | 17-30 | REAL | — | warm-up discarded correctly (i>0); rc!=0 → [] + stderr note |
| main | 33-42 | REAL | S4 | always returns 0 (F-001); DNF rows printed |

## 5. Test honesty
No tests in file (ad hoc tool). The header-only no-arg behavior verified
by execution.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "card 5.R1 breadth snapshot … median-of-5" | docstring | REAL | code matches (REPS=5+1 warmup, statistics.median) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
stdlib only (statistics, subprocess, sys, time). No ffmpeg presence check
(ffmpeg absent → FileNotFoundError traceback; same class as F-002).

## 9. Open questions
None.
