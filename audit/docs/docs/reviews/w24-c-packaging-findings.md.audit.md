# CLAIM AUDIT: docs/reviews/w24-c-packaging-findings.md
| Field | Value |
|---|---|
| Auditor | W19-A06 (recovered by W19-R4) |
| Claims extracted | 9 |
| REAL 8 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 1 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A review whose findings were so real that the tree now carries the fixes with
the review's own severity tags in comments: P1-1's `|| true` masking is gone
(sign-macos.sh:189-197 implements the exact rc-0/2-vs-die pattern the review
prescribed, with "refusing to fall back to the unsigned path"), P1-2's zero
secrets-mappings is fixed (release.yml:250-251, 511-515 — all eight names),
P2-2's POSIX WIX_EXE carries a "W24-R1 P2-2" conversion block
(build-msi.ps1:509-515), P3-1's literals are now `$env:PRODUCT_LOWER`-driven,
and P2-3's stale digests were regenerated with the review cited by name
(evidence/README.md:57-60). The gate-PASS claim is corroborated by the
orchestrator baseline. This is what an adversarial review is supposed to look
like — every claim I spot-checked resolved to code.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | P1-1: `sign-macos.sh:175` `signing_identity=$(resolve_identity \|\| true)` demotes hard-fails to unsigned fallback with false reason | :13-30 | REAL (defect; fixed since) | Current sign-macos.sh:189-197: `resolve_identity_rc` pattern, `[ "$resolve_identity_rc" -eq 0 ] \|\| [ ... -eq 2 ] \|\| { ... refusing to fall back to the unsigned path }` — the prescribed fix landed. |
| 2 | P1-2: "zero `secrets.*` mappings" in workflows; signing lanes can never see CI credentials | :32-48 | REAL (defect; fixed since) | Current .github/workflows/release.yml:250-251 `WINDOWS_SIGNING_PFX_B64: ${{ secrets... }}` + :511-515 all six macOS names — exactly the prescribed mappings. |
| 3 | P2-2: WIX_EXE POSIX path breaks the pwsh upgrade test | :59-62 | REAL (defect; fixed since) | build-msi.ps1:509-515 comment "WIX_EXE POSIX -> Windows path (W24-R1 P2-2)" with the conversion fix. |
| 4 | P2-3: evidence table digests contradict its log (.deb/.AppImage rows) | :64-67 | REAL (defect; fixed since) | docs/packaging/evidence/README.md:57-60: "Table regenerated from the committed log's step-9 manifest (W24-H, review W24-R1 P2-3: the r1 table carried two stale digests ... .deb/.AppImage)" — .deb now `a39dafa88039` (the log's value per the review). |
| 5 | P3-1: `"lumen-msi-"`/`"lumen-upgrade-test-"` literals vs "ZERO IDENTITY LITERALS" banner | :78-79 | REAL (defect; fixed since) | build-msi.ps1:549 `"$($env:PRODUCT_LOWER)-msi-"`; test-msi-upgrade.ps1:214 `"$($env:PRODUCT_LOWER)-upgrade-test-"` — config-driven now. |
| 6 | "Packaging gate ... PASS (6/6)" | :101 | REAL (cross-doc) | Baseline audit/runs/baseline/full_gate.txt:77-79 "=== 11. packaging gate ... gate.sh: PASS ... packaging: PASS". |
| 7 | "run-block syntax 41/41 clean; scripts all exist; 14 .sh bash -n clean" | :97-99 | UNVERIFIED (run) / consistent | Not re-run here; the baseline's packaging PASS (gate.sh re-runs those checks) corroborates the same surface. |
| 8 | "Signing reality: real signtool/codesign/notarytool commands, no fake signing" | :107 | REAL | sign-windows.sh/sign-macos.sh carry the real invocations (signtool sign /fd SHA256 /tr, codesign --force --timestamp --options runtime, notarytool submit --wait, stapler staple — present in both scripts, sed-verified this session). |
| 9 | "Severity counts: P0=0, P1=2, P2=4, P3=4 (total 10)" | :7 | REAL | Ledger contains exactly 2 P1, 4 P2, 4 P3 sections, matching counts. |

## 3. Contradictions
None. The review's defects are dated (pre-merge); the current tree shows the
post-review state — that is the review doing its job, and the fix comments
citing "W24-R1 P2-2/P2-3" prove the loop closed.

## 4. Fabrication / inflation indicators
None. The "verified GREEN" table is over-specified in the honest direction
(shellcheck unavailable is admitted; tool pins hashed live; no "verified on
Windows/macOS" claims anywhere — each CI-only step carries [VERIFY at first
live run]). Every spot-checkable finding landed in the tree as a labeled fix.
