# CLAIM AUDIT: docs/packaging/evidence/w26m-t3-win-mac-payload.md
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Claims extracted | 10 |
| REAL 4 · REAL-UNPROVEN 1 · SHALLOW 0 · STUB 0 · PAPER-ONLY 0 · FALSE-on-current-tree 3 · resolved-later-notes 2 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A genuinely evidence-driven lane record: it reproduces its syntax proofs, runs hostile self-tests, records the bug its own harness caught (the `--`-in-XML-comment fix), and closes with an honest CI-boundary list. Its core deliverable — BIN_TAURI riding every payload lane — is still in the tree and verified today (all 7 owned scripts carry the wiring; gen-wxs.sh `--self-test` re-run by this auditor: PASS, exit 0). What is false is everything time-bound to the w26.m lane state: the tree has since moved through w30.8/w30.9 (desktop stub removed from MSI/.app; CFBundleExecutable re-pointed to lumen-tauri), so this doc's "5 File rows (3 exes)", "BinDesktop component", and "CFBundleExecutable=lumen-desktop ... AUXILIARY" claims contradict the current scripts.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "bash -n AND sh -n per script ... -> OK: all six (exit 0 on both parsers, every file)" | :15-23 | REAL (re-executed by auditor) | ran `bash -n` and `sh -n` on all six scripts today -> 12/12 OK (commands in run log) |
| 2 | "gen-wxs.sh: --self-test PASS (5 File rows parsed; hostile basenames ...)" | :38-42 | FALSE-on-current-tree (PASS holds; count stale) | auditor ran `sh scripts/release/gen-wxs.sh --self-test` -> exit 0, prints "**4 File rows** parsed; ... 2 shortcuts pinned: Start-Menu + Desktop, both tauri-targeted" — the desktop stub exe was removed from the MSI by w30.9 |
| 3 | generated shape: "Component/File/ComponentRef rows ... BinCli, BinDesktop, BinTauri, Docs ... 5 File rows total" | :68-76 | FALSE-on-current-tree | scripts/release/gen-wxs.sh:22 comment now: "BIN_CLI / BIN_TAURI -> the two shipped exe file names"; current components have no BinDesktop (ps1:347-348 asserts "BinDesktop ... must stay GONE (w30.9)") |
| 4 | `len(files) == 4` -> `== 5` ("expected 5 File rows (3 exes + 2 docs)") — the only numeric File-row count | :163-165 | FALSE-on-current-tree (stale count) | scripts/packaging/build-msi.ps1:310 today: `assert len(files) == 4, f"expected 4 File rows (2 exes + 2 docs), got {len(files)}"` — reverted-to-4 after w30.9 removed the desktop stub (2 exes = CLI + tauri) |
| 5 | build-zip.sh "30: BIN_TAURI ... not set (run packaging-env.py first) -> exit 2" (fail-loud guard) | :81-87 | REAL | scripts/packaging/build-zip.sh:33 `: "${BIN_TAURI:?build-zip.sh: BIN_TAURI not set (run packaging-env.py first)}"` — guard present verbatim-class; not re-executed (guard code read) |
| 6 | MSI lane with WIX_EXE=/bin/false -> "wix build failed (exit 1 — expected) ... .wxs KEPT" | :107-124 | REAL-UNPROVEN | run-claim from the lane session; the equivalent guard/generation path exists (gen-wxs.sh ran for this auditor under --self-test); full stubbed run not repeated (time budget) |
| 7 | "Info.plist CFBundleExecutable=lumen-desktop (the tauri exe is an AUXILIARY executable — CFBundleExecutable unchanged)" | :131-135 | FALSE-on-current-tree | scripts/release/make-app.sh:107 `<string>${BIN_TAURI}</string>` under CFBundleExecutable; :122 "MacOS/: ${BIN_TAURI} — the CFBundleExecutable, the bundle's only exe" — w30.8 re-pointed it; only ONE exe ships now |
| 8 | N1: test-msi-upgrade.ps1 should append BIN_TAURI to Assert-InstalledFiles | :188-192 | RESOLVED-later (doc's ask honored) | scripts/packaging/test-msi-upgrade.ps1:192 `foreach ($bin in @("$($env:BIN_CLI).exe", "$($env:BIN_TAURI).exe"))` |
| 9 | "the config gate lives in release.yml (`PACKAGING_WINDOWS_UPGRADE_TEST`, step "MSI upgrade-path test")" | :168-170 | REAL | .github/workflows/release.yml:381-384 step "MSI upgrade-path test (config-gated: packaging.toml windows.upgrade_test)" with `${PACKAGING_WINDOWS_UPGRADE_TEST:?}` guard |
| 10 | N3: docs/packaging/README.md payload table predates the third binary | :197-199 | RESOLVED-and-superseded | docs/packaging/README.md:55-58 now documents the CURRENT two-binary payload (`lumen` CLI + `lumen-tauri` GUI shell, stub gone, shortcuts tauri-targeted) — the table moved past this doc's era |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "5 File rows (3 exes + 2 docs)" (:163-165, :68-76) | build-msi.ps1:310 & gen-wxs.sh self-test output: 4 File rows (2 exes + 2 docs) | scripts — w30.9 removed the desktop stub; this evidence doc predates it |
| "CFBundleExecutable=lumen-desktop ... AUXILIARY" (:134-135) | make-app.sh:107/122: CFBundleExecutable = BIN_TAURI, only exe | make-app.sh — w30.8 re-point |

## 4. Fabrication / inflation indicators
None. The doc records a caught-and-fixed bug in its own change (3b), labels runner-only steps `[VERIFY at first live run]`, and its cross-lane notes ask for MORE coverage, not less. The File-row counts and bundle-executable statements are stale, not invented — the doc is dated (2026-11-13, branch w26.m/t3) and the tree has two later waves of packaging changes on top.

## 5. Matrix rows
Not a matrix chunk. Related row: packaging payload truth now lives in docs/packaging/README.md:55-58 (audited cross-check only).
