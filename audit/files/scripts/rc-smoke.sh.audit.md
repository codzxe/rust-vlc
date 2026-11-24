# AUDIT: scripts/rc-smoke.sh
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / checks | 204 / 8 checks + preflight |
| File verdict | REAL — every check is a genuine executable assertion; the whole run CAN and DOES fail (baseline proof) |
| Reachable from shipped entry point? | NO (release harness script; invoked per docs/release-checklist.md §2) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | baseline rc-smoke.txt cross-check (8 checks); `bash -n` style via release-checklist claim; grep merge markers in ui/src/App.tsx -> 0; ls scripts/rc-smoke-web.mjs -> exists |

## 1. One-paragraph verdict
This is a real smoke harness with a real fail counter: `set -uo pipefail` + explicit `fail=1` on every branch, exit code 0 only if all 8 checks pass. The orchestrator baseline executed it on this tree: 7 of 8 checks PASS (engine-build/version/info, headless-decode with real frame-count arithmetic, command-layer-play, web-ui-build, browser-load) and wasm-web-build FAILS (missing wasm32 rust-std in the sandbox — environmental, passes once installed per the baseline ADDENDUM), overall RESULT: FAIL, exit 1. The script's claims vs the baseline are fully consistent; the only defect is a stale hard-coded failure note that tells users the UI is red when the baseline shows it green.

## 2. What actually works (verified)
— Baseline `audit/runs/baseline/rc-smoke.txt:119-211` (cited as executed evidence): every check ran for real; `=== RC SMOKE RESULT: FAIL ===` at L211 — honest failure reporting.
— Check 4's awk (L100-108) is a genuine assertion: `vf > 0 && vd == vf && af > 0 && ad == af` — every decoded frame count must equal the container count (baseline L131 PASS on the AVI fixture).
— Check 5 greps the CLI's real UTF-8 transition strings (`Buffering → Playing`, `Playing → Stopped`) — same strings the CLI's own conformance tests assert (apps/cli/tests/container_conformance.rs:481).

## 3. Findings
F-001 [S4] — Stale hard-coded failure note misinforms on failure.
Where: scripts/rc-smoke.sh:155-157
Code:
```
    echo "  (known main defects as of 2026-11-10: ui/src/App.tsx merge-conflict"
    echo "   markers + stale generated settings screen — docs/audit/docs-sweep-20261110.md F-1/F-2,"
    echo "   fixes owned outside lane E; re-run when they land)"
```
What is wrong: `grep -c '<<<<<<<\|>>>>>>>' ui/src/App.tsx` -> 0 (no merge markers today) and the baseline shows `web-ui-build: PASS` (rc-smoke.txt:204). A user hitting a UI-build failure today is told a false root cause from a hard-coded 2026-11-10 snapshot. Why it matters: failure triage points at the wrong defect. Correct: print the failing command and let docs/release-checklist.md carry history. Confidence: HIGH — Verification: read + grep + baseline.
F-002 [S4] — Preview-server kill does not kill vite's children.
Where: scripts/rc-smoke.sh:180-194
Code:
```
      (cd "$ROOT/ui" && "./node_modules/.bin/vite" preview --port "$PORT" --strictPort >/tmp/rc-smoke-preview.log 2>&1) &
      preview_pid=$!
```
and L193 `kill "$preview_pid" 2>/dev/null`
What is wrong: `kill` on the subshell PID does not signal the vite child (no process-group kill, e.g. `kill -- -$preview_pid` or `pkill -P`), so a slow-starting/leaked vite can hold port 4173 after the run on some systems. Bounded impact (one orphan process per run). Confidence: MEDIUM — Verification: read (no execution of check 8 here; baseline ran it with PASS and evidently no leak in that sandbox).

## 4. Per-check table
| Check | Lines | Can it fail? | Baseline result | Notes |
|---|---|---|---|---|
| preflight | 49-55 | YES (exit 2) | n/a | tool + fixture existence |
| 1 engine-build | 70-75 | YES | PASS | `--locked --release`, both shipped bins |
| 2 engine-version | 78-83 | YES | PASS (lumen 0.1.3) | regex `^lumen [0-9]+\.[0-9]+\.[0-9]+$` |
| 3 engine-info | 85-93 | YES | PASS | requires `version:` + `decoders:` keys |
| 4 headless-decode | 96-117 | YES | PASS | real arithmetic on BENCH counters |
| 5 command-layer-play | 119-126 | YES | PASS | state-machine walk via open |
| 6 wasm-web-build | 128-140 | YES | **FAILED** (env: no wasm32 rust-std; PASS after install per ADDENDUM) | size >1 MiB + `\0asm` magic (od) |
| 7 web-ui-build | 146-160 | YES | PASS | npm build; F-001's stale note lives here |
| 8 web-ui-browser-load | 162-201 | YES | PASS | port probe ≤15 s; BLOCKED also fails the run (L197-199) |

## 5. Test honesty
N/A (script). The release-checklist's own bash -n syntax sweep (docs/release-checklist.md:51) covers this file — consistent claim, not re-run by me.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "Exit code: 0 only when every check PASSes" | rc-smoke.sh:15-16 | REAL | exit $fail (L204); baseline FAIL -> exit 1 |
| "A check that is skipped ... is reported BLOCKED and ALSO fails the run" | rc-smoke.sh:15-17 | REAL | L197-199 sets fail=1 on BLOCKED |
| "known main defects ... App.tsx merge-conflict markers" | rc-smoke.sh:155 | FALSE today | grep -> 0 markers; baseline web-ui-build PASS |
| "all 8 checks PASS on the RC commit" (requirement) | docs/release-checklist.md:192 | UNMET on current tree | baseline rc-smoke RESULT: FAIL (wasm env gap) |

## 7. Dead / duplicated / unreachable code
None. All 8 checks are reachable via a single run.

## 8. Dependencies & flags
cargo/node/npm preflight; playwright chromium via discovery or RC_SMOKE_CHROMIUM; RC_SMOKE_SKIP_WEB escape hatch honestly labeled "NOT a full RC smoke".

## 9. Open questions for the next agent
- Does any CI workflow run rc-smoke? (checklist §2 says local-only; CI rows blocked by billing — consistent with the soak-nightly/R-16 story.)
