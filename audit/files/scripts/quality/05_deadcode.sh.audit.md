# AUDIT: scripts/quality/05_deadcode.sh
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines / functions | 35 / 1 script (sections 5a-5e) |
| File verdict | REAL (clean, tool-gated discovery) |
| Reachable from shipped entry point? | NO — dev quality script; called by scripts/quality/99_run_all.sh:32 (J, non-gate) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read (full); rg for callers; ls of scripts/quality/ |

## 1. One-paragraph verdict
A small, honest discovery script: five sections each gated on its pinned
tool being present (machete/shear/modules/udeps+nightly), every skip recorded
via not_run, and a cross-script dependency (03_astgrep's allow-attr.json)
handled with an explicit fallback message instead of a silent pass. Nothing
to execute here (all four cargo tools absent in the sandbox). The only
defect is that tool exit codes are wholly discarded — the script's own exit
status is that of the last census line, so a tool that fails mid-sweep is
indistinguishable from a clean sweep in the rc-based consumer.

## 2. What actually works (verified)
— Guard pattern verified consistent with siblings: `if need <tool>` around
every cargo invocation; `not_run` records the nightly-toolchain blocker
B3 for udeps (:26).
— 5e uses `[ -s file ]` (exists AND non-empty) with an honest instruction
fallback (:33-34) — no fail-open masquerade.
— Caller wiring verified: 99_run_all.sh:32 `J 05_deadcode.sh` (J records
rc into FAILED but returns 0 — see 99_run_all's own pattern).

## 3. Findings
F-001 [S4]
Where: scripts/quality/05_deadcode.sh:8,13,18,24
Code:
```
  cargo machete . 2>&1 | tee "$OUT/machete.txt" | tail -40
```
What is wrong: the four tools' exit codes are never captured or reported —
a failing machete/shear/modules/udeps run is invisible beyond whatever
text it emitted; the script exits with the status of the LAST command (5e),
so rc-based consumers cannot tell "tool failed" from "no dead code found".
For a discovery (non-gate) script this is a soft defect, but it makes the
sweep's rc meaningless as a summary of section health.
What correct looks like: capture rc per section into the FAILED list (as
99_run_all's J does at the script level).
Confidence: HIGH — Verification: read.

## 4. Per-section table
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| 5a machete | 6-9 | REAL-UNPROVEN | — | tool absent here; declared-but-unimported deps census |
| 5b shear | 11-14 | REAL-UNPROVEN | — | metadata-accurate unused deps+features |
| 5c modules orphans | 16-19 | REAL-UNPROVEN | — | unreferenced-module census |
| 5d udeps | 21-28 | REAL-UNPROVEN | — | nightly-gated with honest B3 not_run |
| 5e allow census | 30-35 | REAL | — | reads 03_astgrep output; fallback instruction |

## 5. Test honesty
N/A (discovery). Note: outputs land in gitignored docs/audit/reports/ —
like all T-scripts, results are never committed (systemic evidence gap
already recorded in the 12_mutants audit).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "T05 - dead code, orphans, unused dependencies and features" | :2 | REAL | five matching sections present |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
cargo-machete / cargo-shear / cargo-modules / cargo-udeps(+nightly) /
ast-grep (via 03_astgrep.sh) — all pinned in 00_install_tools.sh's PKGS
except udeps (nightly-only; honest blocker note).

## 9. Open questions
Has 03_astgrep.sh's allow-attr.json census ever been run with the
121-#[allow] figure the Wave-0 baseline cites? No census output is
committed; the number comes only from the baseline's own loc-patterns
scan.
