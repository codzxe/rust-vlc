# AUDIT: scripts/packaging/test-msi-upgrade.ps1
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines / functions | 352 / 5 (Fail, Get-SquishedGuid, Get-InstalledProductCodes, Invoke-MsiExec, Assert-InstalledFiles) + main flow |
| File verdict | REAL-UNPROVEN (real, wired, non-stub; Windows-only — cannot execute in Linux sandbox, and no execution log in tree) |
| Reachable from shipped entry point? | N/A (CI/release infra) — wired from .github/workflows/release.yml:381-395 `pwsh scripts/packaging/test-msi-upgrade.ps1 -CurrentMsi "dist/${base}.msi"`, config-gated ON by packaging/packaging.toml:107 `upgrade_test = true` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | python3 squish verification (2 GUIDs + involution); rg fixture + receipt strings + release.yml wiring; ps1-sanity.py PASS exit 0 |

## 1. One-paragraph verdict
A genuinely engineered MSI upgrade-path test: builds a predecessor MSI from the
same tree with a decremented version and the frozen UpgradeCode, installs it,
installs the current MSI, asserts MajorUpgrade REPLACEMENT (exactly 1 squished
ProductCode under the UpgradeCode key, and a NEW code), launch-smokes the
installed lumen-tauri.exe (10 s survival + failure-receipt scan + process-tree
kill), uninstalls, and asserts 0 ProductCodes + no leftover files. The hard
parts check out by execution where Linux allows: the squished-GUID algorithm
reproduces BOTH worked examples in its comments byte-for-byte and is a verified
involution (which the `finally` cleanup depends on); the predecessor-version
decrement rule is correct on all three documented cases; the failure-receipt
regexes match real strings in apps/desktop-tauri/src/main.rs:59 and app.rs:166;
the fixture path exists; the release lane wiring is real and currently ENABLED.
Cannot run msiexec here — the script's own header honestly states the boundary.
Verdict REAL-UNPROVEN: everything static+algorithmic verified, the end-to-end
Windows run has no in-tree log [VERIFY at first live run].

## 2. What actually works (verified)
- Squish algorithm: `python3` re-implementation of the order array (7,6,...,30) -> `{01C5A10F-AD9B-405B-853A-6659841A1242}` => `f01a5c10b9dab50458a3669548a12124` and `7c1f6a52-9d3e-4b8a-a001-3f9c2d64e701` => `25a6f1c7e3d9a8b40a10f3c9d2467e10` — both match the comment's claimed outputs EXACTLY; involution property TRUE (squish(squish(x))==x), so the cleanup's re-dash logic is sound.
- Predecessor rule: 0.1.0->0.0.0, 1.2.3->1.2.2, 1.2.0->1.1.0 — hand-traced the loop, all correct; all-zero input Fails.
- Wiring: release.yml:395 invokes this script; packaging/packaging.toml:107 `upgrade_test = true` (gate currently OPEN).
- Filename consistency NON-bug: build-msi.ps1:578 names the MSI `$PRODUCT_LOWER-$PRODUCT_VERSION-$ARCH_ID-$OS_ID.msi` regardless of `-VersionOverride` — so test line 220's `$prevBase` (also PRODUCT_VERSION-based) matches the built file. The version difference lives only in the WXS Version table, which is what MajorUpgrade keys on. Consistent.
- Receipt strings: `admission shell failed` (main.rs:59), `event emit failed` (app.rs:166) exist in real code; fixture `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` exists (425516 bytes).
- `python3 scripts/packaging/ps1-sanity.py scripts/packaging/test-msi-upgrade.ps1 scripts/packaging/build-msi.ps1` -> `ps1-sanity: PASS (ASCII-only, delimiters balanced, param first)` exit 0.
- `$ErrorActionPreference='Stop'` + `Set-StrictMode -Version Latest` + `[CmdletBinding()] param` first + `finally` cleanup on all paths including failure — defensive posture is real.

## 3. Findings
F-001 [S4]
Where: scripts/packaging/test-msi-upgrade.ps1:304-310
Code:
```
    # Cleanup: kill the whole tree -- WebView2 child processes first
    # (Win32_Process ParentProcessId), then the parent -- so the GUI
    # exe never outlives the test. A Force-kill exit code is recorded,
    # not asserted.
    Get-CimInstance Win32_Process -Filter "ParentProcessId=$($p.Id)" |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
```
What is wrong: "kill the whole tree" is one level deep — WebView2's renderer/gpu
subprocesses are grandchildren (children of the browser process), which survive
the force-kill and get re-parented. On a throwaway CI runner this is cosmetic;
the comment overstates the guarantee. Confidence: HIGH — Verification: read
(Win32_Process filter is single-level by construction).

F-002 [S4]
Where: scripts/packaging/test-msi-upgrade.ps1:145-146
Code:
```
        "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Installer\UpgradeCodes\$squished",
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Installer\UpgradeCodes\$squished"
```
What is wrong: the comment itself (lines 137-141) documents these as "belt-and-
braces fallbacks" for paths the per-machine tables are NOT written to — dead
probing that can never contribute a code on a healthy install. Harmless but
noise; if a code EVER appears only there, the squish or hive assumption is broken
and the count assertions would misfire confusingly. Confidence: MEDIUM —
Verification: read (registry behavior UNVERIFIED on Windows).

F-003 [S4]
Where: scripts/packaging/test-msi-upgrade.ps1:297-302
Code:
```
    $receipts = @(Select-String -Path $smokeErr -Pattern 'admission shell failed|event emit failed|panicked' -ErrorAction SilentlyContinue)
```
What is wrong: the launch smoke's failure detection is stderr-receipt-only; a
silently-blank webview (GUI renders nothing, no error printed) passes the smoke.
The header calls this out ("An empty stderr is the healthy GUI-subsystem case"),
so it is an honest boundary, not a lie — but "launches" (comment line 16) is
proven only as "process alive 10 s with no printed failure". Confidence: HIGH —
Verification: read.

## 4. Per-function table (mandatory; file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Fail | 57-60 | REAL | — | Write-Error + exit 1; used consistently |
| Get-SquishedGuid | 89-123 | REAL (algorithm executed in python re-implementation) | — | both worked examples + involution verified; length-32 guard; hand-derivation typos documented |
| Get-InstalledProductCodes | 125-158 | REAL-UNPROVEN | S4 | 4 hive paths, 2 dead fallbacks (F-002); reads value NAMES (the documented round-5 lesson); returns wrapped unique array |
| Invoke-MsiExec | 160-183 | REAL-UNPROVEN | — | accepts 0/3010; bounded 20-line log tail on failure; ALLUSERS=1 with documented reason |
| Assert-InstalledFiles | 185-199 | REAL-UNPROVEN | — | asserts both BIN_CLI and BIN_TAURI under Program Files |
| main flow | 201-352 | REAL-UNPROVEN | S4 | predecessor build -> install -> replace-assert -> launch smoke -> uninstall-clean; finally-cleanup with squish involution; kill one level (F-001) |

## 5. Test honesty
This script IS a test. Mutation reasoning: if MajorUpgrade semantics broke
(co-install instead of replace), step 3's `$codes.Count -ne 1` fails (two codes
registered). If the MSI stopped installing the exes, Assert-InstalledFiles fails.
If the uninstall leaves residue, step 4 fails. If the app crashes on launch
within 10 s, the survival loop fails. Each failure prints diagnostics (hive
exists= / log tails) rather than bare counts. The asserts are honest — no
tautologies found. Real run evidence: NONE in tree (Windows-only) — REAL-UNPROVEN.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the REAL MSI upgrade-path test the Windows runner executes" | ps1:1-2 | TRUE | release.yml:381-395 runs it; packaging.toml:107 upgrade_test=true |
| "squished GUID ... Verified worked example" | ps1:98-111 | TRUE | python re-implementation reproduces both examples exactly |
| "squish index map is an involution" | ps1:336-338 | TRUE | squish(squish(x))==x verified computationally |
| "the release workflow runs it on the windows-latest runner right after packaging" | ps1:40-42 | TRUE | release.yml step order: packaging steps precede line 381 step |
| "this sandbox is Linux -- the script is NOT executed here" | ps1:38-39 | TRUE | `which pwsh` -> not found |

## 7. Dead / duplicated / unreachable code
- 2 dead hive fallbacks (F-002). No duplication. No stubs, no `|| true`-class
  cannot-fail steps: every msiexec call is exit-code-checked; build-msi.ps1
  call checks $LASTEXITCODE.

## 8. Dependencies & flags
- Depends on: packaging-env.py env vars (8 checked by name), build-msi.ps1
  (-VersionOverride/-SkipSigning params exist at build-msi.ps1:78), WIX_EXE,
  msiexec, elevated shell (asserted, not assumed), tests fixture. All present
  in tree.

## 9. Open questions for the next agent
- Has release.yml's windows job ever actually executed this to PASS? No in-tree
  log; release-checklist.md may claim one — check for a first-live-run log.
- Does the MSI's INSTALLDIR always equal `$env:ProgramFiles\$env:PRODUCT_NAME`
  (Assert-InstalledFiles assumption)? Verify against build-msi.ps1's WXS
  DIRECTORY table when auditing that file.
