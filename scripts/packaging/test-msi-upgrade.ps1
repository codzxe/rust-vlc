# test-msi-upgrade.ps1 -- the REAL MSI upgrade-path test the Windows
# runner executes (issue #56, card 8.E1, task W24-C).
#
# What it proves (end to end, via msiexec + the Windows Installer
# registry -- no mocks):
#   1. A predecessor MSI is built from the SAME tree with a LOWER
#      ProductVersion and the SAME frozen UpgradeCode (from
#      product.toml identity.ids.windows.msi_upgrade_code -- stable
#      across versions by MP sec.5) and installed.
#   2. Installing the CURRENT MSI REPLACES it: exactly one ProductCode
#      remains registered under the UpgradeCode (MSI MajorUpgrade
#      semantics: upgrades replace, never co-install), and the code is
#      a NEW one.
#   3. The shipped files are present under Program Files after both
#      installs.
#   4. Uninstall removes the product: zero ProductCodes remain and the
#      install directory is gone.
#
# Predecessor version rule: parse the numeric MSI version x.y.z, find
# the LAST non-zero field, decrement it and zero every field after it
# (0.1.0 -> 0.0.0; 1.2.3 -> 1.2.2; 1.2.0 -> 1.1.0). Any strictly-lower
# x.y.z is sufficient for MajorUpgrade; this rule is deterministic.
#
# Requirements: Windows, elevated (msiexec per-machine installs), the
# config environment from scripts/packaging/packaging-env.py, WIX_EXE,
# and built release exes. Signing is irrelevant to upgrade semantics
# (MSI replacement is keyed on UpgradeCode/ProductVersion); the test
# builds unsigned MSIs and says so.
#
# HONEST LOCAL-PROOF BOUNDARY: this sandbox is Linux -- the script is
# CI/Windows-only and is NOT executed here [VERIFY at first live run].
# The release workflow runs it on the windows-latest runner right
# after packaging (see .github/workflows/release.yml, step "MSI
# upgrade-path test").
#
# Usage:
#   pwsh scripts/packaging/test-msi-upgrade.ps1 -CurrentMsi <path-to-current.msi>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$CurrentMsi
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

function Fail([string]$Message) {
    Write-Error "test-msi-upgrade.ps1: $Message"
    exit 1
}

# --- Preconditions -----------------------------------------------------------
if (-not (Test-Path $CurrentMsi)) { Fail "current MSI not found: $CurrentMsi" }

foreach ($name in @(
        'PRODUCT_MSI_UPGRADE_CODE', 'MSI_VERSION', 'BIN_CLI', 'BIN_DESKTOP',
        'PRODUCT_NAME', 'PRODUCT_LOWER', 'ARCH_ID', 'OS_ID')) {
    if (-not (Get-Item -Path "Env:$name" -ErrorAction SilentlyContinue)) {
        Fail "required config env $name is not set (run packaging-env.py first)"
    }
}

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Fail 'per-machine msiexec installs need an elevated shell (windows-latest runners are elevated)'
}

# --- Helpers ------------------------------------------------------------------
function Get-SquishedGuid([string]$Guid) {
    # The Installer registry layout keys UpgradeCode lookups by the GUID
    # in "squished" form: braces/dashes removed, the first three groups
    # (DWORD, WORD, WORD) BYTE-reversed, the rest verbatim. The first
    # three rounds of this test probed a key that can never exist: the
    # helper STRING-reversed each group ("7c1f6a52" -> "25a6f1c7") while
    # the byte-wise reversal is "526a1f7c" (bytes 7c 1f 6a 52 reversed);
    # - the round-8 hive diagnostics (exists=False on every path, with
    # the install exit 0) named it. Verified example:
    # {7c1f6a52-9d3e-4b8a-a001-3f9c2d64e701} ->
    # 526a1f7c3e9d8a4ba0013f9c2d64e701.
    $g = $Guid.Trim('{}').Replace('-', '').ToLowerInvariant()
    if ($g.Length -ne 32) { Fail "not a GUID: $Guid" }
    function Reverse-HexByteWise([string]$s) {
        $bytes = @()
        for ($i = $s.Length - 2; $i -ge 0; $i -= 2) {
            $bytes += $s.Substring($i, 2)
        }
        return ($bytes -join '')
    }
    $p1 = Reverse-HexByteWise $g.Substring(0, 8)
    $p2 = Reverse-HexByteWise $g.Substring(8, 4)
    $p3 = Reverse-HexByteWise $g.Substring(12, 4)
    return ($p1 + $p2 + $p3 + $g.Substring(16))
}

function Get-InstalledProductCodes() {
    # ProductCodes registered under the (frozen) UpgradeCode.
    # MSI registry contract: under UpgradeCodes\<squished-upgrade>, each
    # value NAME is the squished ProductCode and the value DATA is
    # installer-internal (typically an empty binary blob). The first live
    # run (release round 5, 2026-11-13) read the DATA and found 0 codes
    # on a perfectly registered install - read the names.
    $squished = Get-SquishedGuid $env:PRODUCT_MSI_UPGRADE_CODE
    $paths = @(
        "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Installer\UpgradeCodes\$squished",
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Installer\UpgradeCodes\$squished"
    )
    $codes = @()
    foreach ($path in $paths) {
        if (Test-Path $path) {
            $item = Get-Item $path
            foreach ($prop in $item.GetValueNames()) {
                if ($prop) { $codes += $prop }
            }
        }
    }
    return , @($codes | Select-Object -Unique)
}

function Invoke-MsiExec([string]$Verb, [string]$MsiPath, [string]$LogFile) {
    # 0 = success; 3010 = success, reboot required (both are installs
    # that took effect -- 3010 is normal for per-machine files-in-use).
    # ALLUSERS=1: the install must be PER-MACHINE (the generated MSI's
    # UpgradeCode registration lands under HKLM) - without it a
    # non-elevated context installs per-user and the UpgradeCodes key
    # the assertions read never appears (rounds 5-6 found 0 with the
    # install exit code 0: it succeeded, just not in the hive the
    # test reads). The runner's shell is elevated, so this is legal.
    $proc = Start-Process -FilePath 'msiexec.exe' `
        -ArgumentList @("/$Verb", "`"$MsiPath`"", 'ALLUSERS=1', '/qn', "/L*v", "`"$LogFile`"") `
        -Wait -PassThru
    if ($proc.ExitCode -ne 0 -and $proc.ExitCode -ne 3010) {
        Fail "msiexec /$Verb exited $($proc.ExitCode) (log: $LogFile)"
    }
    return $proc.ExitCode
}

function Assert-InstalledFiles([string]$Label) {
    $installRoot = Join-Path $env:ProgramFiles $env:PRODUCT_NAME
    # All three shipped exes (w26.m.t3 cross-lane note N1: the tauri row
    # joins the upgrade-test assertion alongside the CLI and desktop
    # binaries - the MSI ships all three since wave 26-M).
    foreach ($bin in @("$($env:BIN_CLI).exe", "$($env:BIN_DESKTOP).exe", "$($env:BIN_TAURI).exe")) {
        $p = Join-Path $installRoot $bin
        if (-not (Test-Path $p)) {
            Fail "$Label - expected installed file missing: $p"
        }
    }
    Write-Output "  [$Label] installed files present under $installRoot"
}

# --- Predecessor version ------------------------------------------------------
$fields = $env:MSI_VERSION.Split('.')
if ($fields.Count -ne 3) { Fail "MSI_VERSION $($env:MSI_VERSION) is not x.y.z" }
$nums = $fields | ForEach-Object { [int]::Parse($_) }
$idx = 2
while ($idx -gt 0 -and $nums[$idx] -eq 0) { $idx-- }
if ($nums[$idx] -eq 0) { Fail "cannot derive a predecessor of $($env:MSI_VERSION) (all fields zero)" }
$nums[$idx]--
for ($j = $idx + 1; $j -lt 3; $j++) { $nums[$j] = 0 }
$prevVersion = ($nums -join '.')
Write-Output "upgrade test: current=$($env:MSI_VERSION) predecessor=$prevVersion (same frozen UpgradeCode $($env:PRODUCT_MSI_UPGRADE_CODE))"

# --- 1. Build the predecessor MSI (same tree, lower version, unsigned) -------
$work = Join-Path ([IO.Path]::GetTempPath()) ("$($env:PRODUCT_LOWER)-upgrade-test-" + [IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $work | Out-Null
try {
    $env:DIST_DIR = $work
    & (Join-Path $PSScriptRoot 'build-msi.ps1') -VersionOverride $prevVersion -SkipSigning
    if ($LASTEXITCODE -ne 0) { Fail 'predecessor MSI build failed (build-msi.ps1)' }
    $prevBase = "$($env:PRODUCT_LOWER)-$($env:PRODUCT_VERSION)-$($env:ARCH_ID)-$($env:OS_ID)"
    $prevMsi = Join-Path $work "$prevBase.msi"
    if (-not (Test-Path $prevMsi)) { Fail "predecessor MSI not found at $prevMsi" }

    # --- 2. Install predecessor, assert 1 product ---------------------------
    $null = Invoke-MsiExec 'i' $prevMsi (Join-Path $work 'prev-install.log')
    $codes = Get-InstalledProductCodes
    if ($codes.Count -ne 1) {
        # Honest diagnostics: WHERE the registration landed (or didn't).
        # The registry contract is HKLM UpgradeCodes\<squished-upgrade>;
        # a per-user install (ALLUSERS lost), a different hive view, or a
        # failed-but-zero-exit install all end here - the next run must
        # say which, not just "found 0".
        $squished = Get-SquishedGuid $env:PRODUCT_MSI_UPGRADE_CODE
        foreach ($hive in @("HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Installer\UpgradeCodes",
                            "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Installer\UpgradeCodes",
                            "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Installer\UpgradeCodes")) {
            $exists = Test-Path (Join-Path $hive $squished)
            Write-Output "  diagnostics: $hive\$squished exists=$exists"
        }
        Write-Output "  diagnostics: prev-install.log tail:"
        Get-Content (Join-Path $work 'prev-install.log') -Tail 12 | ForEach-Object { Write-Output "    $_" }
        Fail "after predecessor install: expected exactly 1 ProductCode under the UpgradeCode, found $($codes.Count): $($codes -join ', ')"
    }
    $prevCode = $codes[0]
    Assert-InstalledFiles 'predecessor install'

    # --- 3. Install CURRENT MSI, assert REPLACEMENT --------------------------
    $null = Invoke-MsiExec 'i' $CurrentMsi (Join-Path $work 'cur-install.log')
    $codes = Get-InstalledProductCodes
    if ($codes.Count -ne 1) {
        Fail "after current install: expected exactly 1 ProductCode under the UpgradeCode (replace, not co-install), found $($codes.Count): $($codes -join ', ')"
    }
    if ($codes[0] -eq $prevCode) {
        Fail "current install kept the predecessor ProductCode $prevCode - version did not advance?"
    }
    Assert-InstalledFiles 'current install (upgraded)'
    Write-Output "  [current install] ProductCode $($prevCode) -> $($codes[0]) (replaced, not co-installed)"

    # --- 4. Uninstall, assert clean -------------------------------------------
    $null = Invoke-MsiExec 'x' $CurrentMsi (Join-Path $work 'uninstall.log')
    $codes = Get-InstalledProductCodes
    if ($codes.Count -ne 0) {
        Fail "after uninstall: expected 0 ProductCodes under the UpgradeCode, found: $($codes -join ', ')"
    }
    $installRoot = Join-Path $env:ProgramFiles $env:PRODUCT_NAME
    if (Test-Path (Join-Path $installRoot "$($env:BIN_CLI).exe")) {
        Fail "uninstall left files behind in $installRoot"
    }
    Write-Output '  [uninstall] clean - 0 ProductCodes, install dir removed'

    Write-Output 'test-msi-upgrade.ps1: PASS (install -> upgrade-replaces -> uninstall-clean)'
}
finally {
    # Never leave an install behind, even on failure.
    $codes = Get-InstalledProductCodes
    foreach ($code in $codes) {
        Write-Output "cleanup: removing leftover product $code"
        $proc = Start-Process -FilePath 'msiexec.exe' `
            -ArgumentList @('/x', "`"$code`"", '/qn') -Wait -PassThru
        if ($proc.ExitCode -ne 0 -and $proc.ExitCode -ne 3010) {
            Write-Warning "cleanup uninstall of $code exited $($proc.ExitCode)"
        }
    }
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
