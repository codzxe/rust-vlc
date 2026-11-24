# AUDIT: scripts/packaging/build-msi.ps1
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Date | 2026-11-24 |
| Lines / functions | 613 / 3 (Fail, XmlEscape, New-MsiWxs) + -SelfTest block + main flow |
| File verdict | REAL-UNPROVEN (generation logic proven twin-identical offline; PowerShell execution never run anywhere) |
| Reachable from shipped entry point? | YES (CI: release.yml:395 pwsh test-msi-upgrade.ps1 -> its L218 `& build-msi.ps1 -VersionOverride ...`; gate.sh:218 -SelfTest when pwsh) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 5 |
| Commands run | python3 scripts/twin_parity.py -> exit 0; python3 scripts/packaging/ps1-sanity.py scripts/packaging/build-msi.ps1 -> exit 0; rg/sed (provenance) |

## 1. One-paragraph verdict
The Windows-native MSI builder: config-driven (all identity from $env: exported by
packaging-env.py/product-env.py; verified no identity literals — the "ZERO IDENTITY
LITERALS" header claim holds), full XML escaping for doc basenames, loud refusals
(missing env, missing binaries, duplicate doc basenames, empty docs, unknown ARCH_ID),
explicit `exit 0`/Fail-exit discipline. Its .wxs generation half is VERIFIED: I ran
`scripts/twin_parity.py` (exit 0), which executes the bash twin `gen-wxs.sh` for real,
simulates this script's here-string interpolation, and proves semantic XML identity
plus the 3 stable component GUIDs re-derived from product.toml's frozen
msi_upgrade_code. What is NOT proven anywhere: actual PowerShell execution — no pwsh
exists in this sandbox, gate.sh:220-223 skips it ("no pwsh on PATH"), and no log in the
tree shows a live Windows run (header L68-73 admits "authored and reviewed on Linux;
executed on Windows only [VERIFY at first live run]"). PowerShell semantics reviewed
by reading: param quoting, EAP=Stop + StrictMode Latest, $LASTEXITCODE checks on all
native calls (python, wix, bash) are correct.

## 2. What actually works (verified)
- `python3 scripts/twin_parity.py` -> "PASS -- gen-wxs.sh (executed for real) and the build-msi.ps1 here-string simulation produce SEMANTICALLY IDENTICAL .wxs XML ... AppMenuShortcut=44134d88-..., DesktopShortcut=5364ccd7-..., Docs=798918a8-..." exit 0. The 3 GUID literals at lines 192/208/231 of this script are exactly the re-derived ones.
- `python3 scripts/packaging/ps1-sanity.py scripts/packaging/build-msi.ps1` -> "PASS (ASCII-only, delimiters balanced, param first)" exit 0.
- XmlEscape order (L102-103) escapes `&` FIRST — correct (no double-escape).
- Every native call checks $LASTEXITCODE: L483 (packaging-env.py), L465+470 (selftest proof), L586 (wix build), L601 (sign-windows.sh).

## 3. Findings
F-001 [S4]
Where: scripts/packaging/build-msi.ps1:94-99 vs 164-165,173,186,210,233
Code (claim, L95-96):
```
# Every value interpolated into an XML attribute goes through XmlEscape
```
Code (reality, L164-165):
```
           Version="$Version"
           UpgradeCode="$UpgradeCode"
```
What is wrong: Version, UpgradeCode, CliExe and TauriExe are interpolated UNESCAPED;
only Name/Manufacturer/Description/doc-basenames go through XmlEscape. Why it matters:
the comment overstates the safety property; a free-text `-VersionOverride` (dev param,
L55-58) yields malformed XML (fails loudly at wix build, so no shipped-path breakage;
MSI_VERSION is derived numeric-only from display_version via product-env.py:37,194 and
UpgradeCode is the frozen UUID in product.toml:43). Twin-consistent: gen-wxs.sh:187-188
does the same. What correct looks like: escape them or scope the comment to the fields
that actually are escaped. Confidence: HIGH — Verification: read + rg.
F-002 [S4]
Where: scripts/packaging/build-msi.ps1:89-92
Code:
```
function Fail([string]$Message) {
    Write-Error "build-msi.ps1: $Message"
    exit 1
}
```
What is wrong: under `$ErrorActionPreference = 'Stop'` (L84) `Write-Error` throws a
terminating error, so `exit 1` is unreachable dead code; the script still exits nonzero
via the uncaught error, so behavior is preserved. What correct looks like: `exit 1`
first, or throw a typed terminating error. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: scripts/packaging/build-msi.ps1:551-552 vs 136-141
Code:
```
    Copy-Item (Join-Path $targetDir "release\$cliExe") $stage
    Copy-Item (Join-Path $targetDir "release\$tauriExe") $stage
```
What is wrong: docs get a duplicate-basename loud refusal (L138-140) but the two exe
staging copies get none — if BIN_CLI == BIN_TAURI the second copy silently clobbers the
first and both components reference one file. Config-controlled (packaging/product
identity, currently distinct names) so unreachable today. Confidence: HIGH — read.
F-004 [S4]
Where: scripts/packaging/build-msi.ps1:562
Code:
```
    $docPaths = @($env:PACKAGING_DOCS -split '\s+' | Where-Object { $_ })
```
What is wrong: whitespace split breaks doc paths containing spaces (refused loudly at
L565 "not found", not silently) — same contract as the bash twin's space-separated
list. Cosmetic asymmetry: loud failure for a config the schema permits in principle.
Confidence: HIGH — read.
F-005 [S4]
Where: scripts/packaging/build-msi.ps1:68-73 and 518-521
Code:
```
# HONEST LOCAL-PROOF BOUNDARY: authored and reviewed on Linux; executed
# on Windows only [VERIFY at first live run].
```
What is wrong: nothing false — the script honestly flags that its own live execution
is unproven; recorded here so the Devil Audit headline reflects it: the PowerShell
param/exit-code/wix-invocation paths have ZERO execution evidence in the repo (no pwsh
in sandbox; CI lane is config-gated `upgrade_test = true`, packaging.toml:107, but no
committed run log of the .ps1 under pwsh exists in docs/). Confidence: HIGH — read.
UNVERIFIED: any live run of this script.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Fail | 89-92 | REAL-UNPROVEN | S4 | exit 1 unreachable under EAP=Stop (F-002); net exit code still nonzero |
| XmlEscape | 101-104 | REAL | — | correct escape set + order; proven by twin_parity + selftest raw-text pins |
| New-MsiWxs | 119-267 | REAL | — | semantic output proven identical to executed bash twin (twin_parity exit 0); duplicate-basename + empty-docs refusals; Version/UpgradeCode unescaped (F-001) |
| -SelfTest block | 278-472 | REAL-UNPROVEN | — | hostile fixtures + python XML proof; never executed under pwsh (F-005); pin coverage is excellent (raw-text &apos; pin, ComponentRef Id= pin, GUID pins) |
| main flow | 474-613 | REAL-UNPROVEN | — | env bootstrap, 12 required keys each named on failure, POSIX WIX_EXE conversion (L518-521, honest [VERIFY]), arch map switch, staging in temp + finally cleanup, MSI non-empty check, sign via bash twin, explicit exit 0 |

## 5. Test honesty (the -SelfTest)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| XML parse of hostile fixtures | YES | python ET.parse dies on malformed XML |
| Round-trip of hostile basenames/identity | YES | assert in list (L313-316, L451-452) |
| Escape-set raw-text pins (L437-442) | YES | catches reverted &apos; — the one mutation parse-only tests miss |
| Component/ComponentRef coverage (L334-343) | YES | Id-vs-RefId typo would fail |
| Shortcut/GUID/keypath pins (L353-415) | YES | literal pins fail on any GUID/key drift |
| Would fail if XmlEscape dropped &? | YES | L437 pin is on raw text |
Weakness: all of this only ever runs under pwsh — in this repo's evidence it has NEVER run (no committed pass log; gate.sh skips without pwsh). The twin-parity simulation ran INSTEAD (PASS, exit 0).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "CONFIG-DRIVEN, ZERO IDENTITY LITERALS" | header L12-14 | REAL | every identity field read from $env: (L475-504, 570-573); no product identity string in body; rg confirmed only selftest fixtures + pinned GUIDs |
| "Every value interpolated into an XML attribute goes through XmlEscape" | L95 | FALSE-in-part | Version/UpgradeCode/exe names raw (F-001); limited blast radius |
| ".wxs shape MIRRORS gen-wxs.sh ... change both together" | L43-49 | REAL | twin_parity.py exit 0 proves semantic identity incl. 3 GUIDs |
| "executed on Windows only [VERIFY at first live run]" | L68-69 | HONEST | no pwsh in sandbox; no run log exists |

## 7. Dead / duplicated / unreachable code
- `exit 1` in Fail (L91) — unreachable (F-002).
- No dead blocks otherwise; every step feeds the build.

## 8. Dependencies & flags
- Calls: packaging-env.py (bootstrap, L481), python3/python (selftest proof), wix.exe
  (L585), sign-windows.sh via bash (L600), gen-wxs.sh twin (parity constraint).
- Consumers: test-msi-upgrade.ps1:218 (`-VersionOverride ... -SkipSigning`),
  gate.sh:218 (-SelfTest, pwsh-gated). release.yml runs the .sh twin for the real
  release MSI (L291) — the .ps1 is the upgrade-test + dev path.

## 9. Open questions for the next agent
- release.yml agent: has ANY tagged release actually executed the upgrade-test lane
  (packaging.toml `upgrade_test = true`) on a Windows runner? If not, F-005 hardens
  from "honest gap" to "lane never exercised".
- tools/vta/ruopus/Cargo.toml agent (mine): none related.
- Who owns ps1-sanity.py? It passed but only checks ASCII/balance/param-position —
  a shallow floor; noted here so its owner knows the real parse is still unproven.
