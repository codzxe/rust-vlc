# AUDIT: scripts/quality/astgrep_scan.ps1
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / functions | 29 / main |
| File verdict | REAL manual Windows driver (unwired by design); not executable here (no pwsh) |
| Reachable from shipped entry point? | NO — manual Git-Bash/Windows escape hatch; the canonical CI path is scripts/quality/03_astgrep.sh (header says so); zero references to this file anywhere in the repo (grep) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `ls scripts/quality/rules/` -> 11 yml rules (incl. unwrap-outside-tests.yml); `command -v pwsh powershell` -> absent (execution UNVERIFIED); grep for references -> none |

## 1. One-paragraph verdict
A small, competent PowerShell mirror of the bash ast-grep sweep for Windows hosts where npx hits the Windows Store alias failure: pinned @ast-grep/cli@0.51.3 (same version pin as common.sh's sg_run), per-rule JSON + .err capture, a `count = -1` sentinel when JSON parsing fails (honest failure marking), UTF-8 no-BOM output, and a summary table + summary.json. It has no failure exit path of its own ($ErrorActionPreference = "Continue"; the script cannot exit non-zero) and is referenced by nothing — fine for a manual driver, worthless as a gate.

## 2. What actually works (verified)
— Read: rule enumeration `Get-ChildItem $rules -Filter *.yml` covers all 11 committed rules (verified: 11 files incl. unwrap-outside-tests.yml — my next file).
— Pin consistency: `npx --yes -p @ast-grep/cli@0.51.3` matches common.sh's sg_run pin (both 0.51.3) — no version drift between the two drivers.
— Honest sentinel: `try { $count = ($raw | ConvertFrom-Json).Count } catch { $count = -1 }` — a failed parse is recorded as -1 in the summary, not silently 0.

## 3. Findings
F-001 [S4] — No failure exit path; stderr only lands in per-rule .err files.
Where: scripts/quality/astgrep_scan.ps1:5,20
Code:
```
$ErrorActionPreference = "Continue"
...
    $raw = npx --yes -p @ast-grep/cli@0.51.3 ast-grep scan --rule $f.FullName --json @paths 2> $err
```
What is wrong: an npx/ast-grep invocation failure produces empty stdout; ConvertFrom-Json then throws -> count=-1 (good), but the script still exits 0 with no nonzero signal — a completely broken tool run looks identical to a completed sweep except for -1 rows. Acceptable for a manual driver; unacceptable if anyone ever wires it as a gate. Confidence: HIGH — Verification: read.
(Note: execution UNVERIFIED — no pwsh/powershell in this sandbox.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main | 5-28 | REAL | S4 | as above |

## 5. Test honesty
N/A. The count=-1 sentinel is the honesty mechanism.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "The bash script (03_astgrep.sh) is the canonical CI path" | L2-3 | REAL | no workflow references this ps1 (grep: zero hits) |

## 7. Dead / duplicated / unreachable code
None; unwired by design (manual driver).

## 8. Dependencies & flags
npx + @ast-grep/cli@0.51.3 (pinned); writes docs\audit\reports\astgrep (gitignored).

## 9. Open questions for the next agent
- Windows devs: does the REGDB_E_CLASSNOTREG workaround actually fire? Only answerable on a Windows host.
