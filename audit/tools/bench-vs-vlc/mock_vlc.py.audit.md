# AUDIT: tools/bench-vs-vlc/mock_vlc.py
| Field | Value |
|---|---|
| Auditor | W19-A04 (recovered by W19-R4) |
| Date | 2026-11-24 |
| Lines / functions | 121 / 1 (main) |
| File verdict | REAL (executed) |
| Reachable from shipped entry point? | NO — dev/bench tooling, not in the lumen-cli/desktop release lane; reached from tools/bench-vs-vlc/run.py `--oracle mock` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 mock_vlc.py --version (rc 0); 2x dump runs + cmp (identical); LUMEN_MOCK_VLC_VERSION=4.0.0 run; LUMEN_MOCK_VLC_ECHO run; no-args run (stderr msg); rg over run.py + baselines |

## 1. One-paragraph verdict
The mock oracle does exactly what its header promises and nothing more: it
emulates the CLI shape, produces deterministic synthetic PCM, labels itself
MOCK in every channel (version line, stderr, argparse prog), and exposes the
two documented test hooks. I executed every documented behavior and all
verified. No findings — it is honest-by-construction tooling.

## 2. What actually works (verified)
- `python3 mock_vlc.py --version` -> `VLC version 3.0.23 MOCK ORACLE (tools/bench-vs-vlc/mock_vlc.py — NOT the real binary, NOT a measurement)` (rc 0).
- Determinism: two runs over the same input -> `cmp` IDENTICAL, 4096 B dump (matches `max(4096, size//16)` for a tiny input).
- `LUMEN_MOCK_VLC_VERSION=4.0.0` -> version line becomes `VLC version 4.0.0 MOCK ORACLE ...` (drift-hook works).
- `LUMEN_MOCK_VLC_ECHO=/tmp/mvecho.txt` -> output file contains the echoed bytes verbatim (agreement-hook works).
- Missing args -> stderr `mock_vlc: --audiofile-file and an input are required` + exit path (return 1, :83).
- run.py:169-171 pin check requires the `VLC version ` prefix — the mock satisfies it (harness end-to-end exercise works).
- Clean-room sources exist: docs/baselines/raw/linux/{14-options-full-help.txt,04-vlc-version.txt} (ls verified).
- run.py:24-25 documents the mechanical `[MOCK]` label + `oracle.kind = "mock"` JSON envelope.

## 3. Findings
None. (Checked: argument surface vs the documented flags at run.py vlc_dump_command; determinism; hook behavior; exit codes; output size math at :104 `max(4096, st_size // 16)`; LCG `state = seed & 0xFFFFFFFF or 1` zero-guard at :105; path handling; no env leakage; no network; no VLC source consulted — header cites only this repo's own baselines.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main | 50-117 | REAL | — | Full behavior executed this session (see §2); argparse surface matches run.py's invocation flags. |

## 5. Test honesty
Tested indirectly by tools/bench-vs-vlc/test_run.py (sibling audits cover it);
its 30 unit tests were verified by the W19-A08 README audit (30/30 pass, 0.697s,
cited there). Not re-run here (no node_modules/pytest context needed — plain
python3 sufficed for direct execution).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "produces [MOCK]-labeled artifacts, never a measurement" | mock_vlc.py:2-12 header + run.py:21-25 | REAL | Executed: version line + stderr carry MOCK markers; envelope field in run.py. |
| "deterministic synthetic PCM: LCG seeded from SHA-256" | :98-110 | REAL | Two-run cmp identical (see §2). |

## 7. Dead / duplicated / unreachable code
None; single-purpose file, one entry point, both env hooks are live test seams
consumed by test_run.py (rg: LUMEN_MOCK_VLC_ECHO/VERSION in test_run.py —
verified by the sibling audit's coverage notes).

## 8. Dependencies & flags
Stdlib only (argparse, hashlib, os, sys, pathlib). No external deps; shebang
`#!/usr/bin/env python3` correct; __pycache__ present in the tool dir (build
artifact of test runs, harmless).

## 9. Open questions for the next agent
run.py's real-oracle lane is UNVERIFIABLE in this sandbox (no VLC binary) —
only the mock path is exercised; anyone claiming "bench-vs-vlc real oracle
PASS" in a doc should be challenged for a transcript.
