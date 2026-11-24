# AUDIT: scripts/release/package-windows.sh
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / functions | 98 / main + die() |
| File verdict | REAL (guards verified, sh -n clean) — but UNWIRED legacy: no current workflow invokes it |
| Reachable from shipped entry point? | NO — superseded by scripts/packaging/build-zip.sh + build-msi.sh in release.yml:235-291 (windows lane); only docs/release-checklist.md:101 still cites it (stale) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `sh -n scripts/release/package-windows.sh` -> OK; `grep build-zip/build-msi .github/workflows/release.yml` -> L235-291; `grep -rn package-windows .github/ scripts/packaging/` -> zero invocations |

## 1. One-paragraph verdict
A well-guarded wave-22 packager (env-required-not-defaulted posture, input existence checks, staging with trap, stdlib zipfile creation, optional WiX MSI path with its own guards) — but it is dead in the current release pipeline: release.yml's windows lane packages via `scripts/packaging/build-zip.sh` (flat zip, L235) and `scripts/packaging/build-msi.sh` (which reuses only `gen-wxs.sh`). Nothing invokes package-windows.sh; the release-checklist §3 row that still names it describes a pipeline that no longer exists.

## 2. What actually works (verified)
— `sh -n` -> exit 0.
— Guard inventory (L29-43): 8 `:?` required vars (BIN_TAURI required — GUI shell shipped, "the exit-86 lumen-desktop stub left every user payload" per w30.8/w30.12); optional WIX_EXE path.
— Input checks L56-58: cli/tauri exes + LICENSE + README.md.
— Prior execution: worklog.md:644 "package-windows.sh zip mode verified (4 flat entries)" (session evidence; not re-runnable here — no .exe binaries without a windows toolchain).

## 3. Findings
F-001 [S3] — Superseded/dead packaging script; its checklist citation is stale.
Where: scripts/release/package-windows.sh:1-3 vs .github/workflows/release.yml:235-291 and docs/release-checklist.md:101
Code:
```
# package-windows.sh — stage + package the Windows release artifacts
# (card 7.w22g, issue #56 release half).
```
vs release.yml:
```
      - name: Package flat zip (scripts/packaging/build-zip.sh — exes + [shipped] docs from config)
        run: scripts/packaging/build-zip.sh
      - name: Package MSI (scripts/packaging/build-msi.sh — config-driven .wxs + WiX + sign-windows.sh)
```
and docs/release-checklist.md:101:
```
| windows | windows-latest | `.zip` + `.msi` (WiX v5.0.2, SHA-verified nupkg; `UpgradeCode` frozen from product.toml → upgrades replace, never co-install) | `scripts/release/package-windows.sh` + `gen-wxs.sh` |
```
What is wrong: `grep -rn package-windows .github/ scripts/` -> zero invocations; the live lane is scripts/packaging/*. The release-checklist tells a release operator to reason about (and re-run) a script the actual workflow never executes — a real doc-vs-workflow drift on the RELEASE path, which is exactly where silent divergence hurts. Why it matters: an operator following the checklist audits/validates the wrong artifact chain. Correct: either delete the legacy script or update the checklist row to build-zip.sh/build-msi.sh. Confidence: HIGH — Verification: ran greps.
F-002 [S4] — die() exit code inconsistent across the release family (1 here, 2 in make-appimage.sh) — cosmetic contract drift for CI triage. Confidence: HIGH — Verification: read + comparison.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| die | 48 | REAL | — | exit 1 |
| main | 29-97 | REAL | — | zip path verified in-session (worklog 644); MSI path session-verified via gen-wxs/build flow |

## 5. Test honesty
N/A (script). Checklist §1.2's `bash -n` loop covers this file (docs/release-checklist.md:51).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| windows lane uses `scripts/release/package-windows.sh + gen-wxs.sh` | docs/release-checklist.md:101 | FALSE (stale) | release.yml:235-291 calls build-zip.sh/build-msi.sh |
| "zip made with `python -m zipfile` ... One tool, three images" | package-windows.sh:20-23 | REAL (design) | same approach in build-zip.sh's lineage; runner-portability rationale sound |

## 7. Dead / duplicated / unreachable code
The whole script is dead in the live pipeline (F-001) — kept as wave-22 history. Its MSI path duplicates build-msi.sh's role.

## 8. Dependencies & flags
python3 stdlib zipfile; optional wix.exe; env from product-env.py bridge. POSIX sh for git-bash.

## 9. Open questions for the next agent
- Confirm with the release-checklist owner whether §3's lane table predates the scripts/packaging wave (W24-C) and needs the same rewrite make-appimage/package-deb rows need.
