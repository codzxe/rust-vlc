# AUDIT: docs/gates/logs/phase7-exit-parity.log (DATA AUDIT — log)
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines | 73 |
| File verdict | REAL (authentic xtask parity-report + feature-audit output; arithmetic exact; exit codes embedded) |
| Reachable from shipped entry point? | N/A (log data; cited by docs/gates/phase7-exit.md:62-63,243) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | python3 arithmetic re-verification (rows/categories/totals/cells); rg citation check |

## 1. One-paragraph verdict
Genuine double-tool run log: parity-report (21 categories, 252 rows,
windows/linux/macos 64/252 = 25.4% each, web 0/252, verified-only 0/252
EVERYWHERE) + feature-audit (result PASS, 252 rows, per-phase and per-status
cell counts). Arithmetic is exact under recomputation: 21 categories per
table sum to 252 rows each; windows counts sum to 64; the status cells
(133+192+505+142+36) sum to exactly 1008 = 252×4. Unlike the playwright log,
this one EMBEDS exit codes (`PARITY_EXIT=0`, `FEATURE_AUDIT_EXIT=0`) — honest.
Two hostile observations: (1) the feature-audit run executed in worktree
`/home/z/my-project/wt/w24f` (line 67), not the main tree — the log's PASS
describes a worktree's matrix state; (2) the current baseline's feature-audit
is a FAILURE with 2 missing evidence paths, so this log's "structure +
evidence resolution OK" no longer describes the shipped tree — evidence
drifted between 2026-11-10 and the 2026-11-23 baseline, and NO gate re-ran it
(feature-audit is invoked by no CI workflow, per my ci.yml audit).

## 2. What actually works (verified)
- python3 recomputation: 21 categories/table; row sums = 252 per table (504 across both); windows sum = 64 (128 across both tables / 2); status cells 133+192+505+142+36 = 1008 = 252×4 EXACT.
- Percentages spot-checked: 9/19=47.4% ✓, 64/252=25.4% ✓, 9/11=81.8% ✓.
- Exit codes present: `PARITY_EXIT=0` (line 63), `FEATURE_AUDIT_EXIT=0` (line 73).
- Citations accurate: phase7-exit.md:62-63 quotes both PASS rows from this log.

## 3. Findings
F-001 [S3]
Where: docs/gates/logs/phase7-exit-parity.log:65-73 vs audit/runs/baseline/xtask-feature-audit.txt
Code:
```
## Cmd: cargo run -q -p xtask -- feature-audit
=== LUMEN FEATURE-AUDIT REPORT ===
matrix: docs/feature-matrix.toml (root /home/z/my-project/wt/w24f)
```
What is wrong: two things. First, provenance: the feature-audit ran against
worktree `wt/w24f`, not the main tree — a phase-7 "exit gate" log whose
feature-audit half describes a worktree's matrix. Second, drift: the current
baseline runs the same command and gets FAILURE with 2 evidence paths
missing (xtask-feature-audit.txt), while phase7-exit.md:63 still cites this
log as "PASS (252 rows, structure + evidence resolution OK)". The PASS is
dated truth; the shipped tree fails the same check TODAY, and nothing in CI
would notice (no workflow invokes feature-audit — verified in my ci.yml
audit). Confidence: HIGH — Verification: read both + baseline file.
F-002 [S4]
Where: docs/gates/logs/phase7-exit-parity.log:62-63
Code:
```
parity: rows=252 windows=64/252 linux=64/252 macos=64/252 web=0/252
PARITY_EXIT=0
```
What is wrong: parity-report exit 0 means "the tool ran"; the web column is
0/252 — a 0% web parity that still exits 0. The tool has no threshold; the
"web parity honesty" framing is the doc's, not an enforced gate.
Confidence: HIGH — Verification: read (exit-code semantics from the output).

## 4. Data integrity checks
- 21 categories; 252 rows; totals 64/64/64/0; verified-only all zeros — all recomputed EXACT.
- Cells partition 1008 = 4×252 exactly. No missing/extra status vocabulary in the sum.
- Cross-time: current baseline parity = windows 66/252, linux 61/252 (spec §13) vs this log's 64/64/64 — per-platform counts diverged after 2026-11-10 (platform-specific flips landed); verified-only remains 0/252 — consistent story: implemented grew, verified never moved.

## 5. Provenance
Dated 2026-11-10T17:05:29Z; feature-audit root = worktree wt/w24f (F-001).
Not re-runnable in this sandbox (no cargo). Treated as data per rules.
