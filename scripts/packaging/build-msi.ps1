# build-msi.ps1 — native Windows MSI builder (issue #56, card 8.E1, task
# W24-C).
#
# The PowerShell twin of scripts/packaging/build-msi.sh (the CI path;
# the release workflow runs bash by default). This script is the
# Windows-native developer path and the engine behind the MSI
# upgrade-path test (test-msi-upgrade.ps1): stage -> generate the WiX
# v4/v5 .wxs from the config environment -> `wix build` per-arch.
#
# CONFIG-DRIVEN, ZERO IDENTITY LITERALS: every identity field (product
# name, publisher, description, numeric MSI version, the frozen
# UpgradeCode, both exe names) is read from $env: variables exported by
# scripts/packaging/packaging-env.py (which validates
# packaging/packaging.toml against its schema and re-exports the
# product.toml identity resolved by scripts/release/product-env.py).
# When the environment is missing, this script bootstraps it by running
# packaging-env.py itself (python3/python required).
#
# The .wxs shape MIRRORS scripts/release/gen-wxs.sh (the repo's one
# bash generator, used by build-msi.sh). Change both together — the
# upgrade-path test cross-checks installed ProductVersion/UpgradeCode
# and would catch identity drift between them.
#
# Usage:
#   pwsh scripts/packaging/build-msi.ps1 [-WixExe <path>] `
#        [-VersionOverride <x.y.z>] [-SkipSigning] [-Sign]
#   -WixExe           path to wix.exe (or $env:WIX_EXE). REQUIRED.
#   -VersionOverride  build with this numeric x.y.z instead of the
#                     configured MSI version (used ONLY by the
#                     upgrade-path test to synthesize the predecessor
#                     build; never for real releases).
#   -Sign             also run scripts/packaging/sign-windows.sh via
#                     bash (git-bash) — the ONE signing implementation.
#   -SkipSigning      explicit opt-out (CI signs in its own step).
#
# HONEST LOCAL-PROOF BOUNDARY: authored and reviewed on Linux; executed
# on Windows only [VERIFY at first live run]. All env assertions and
# the .wxs generation logic are shared with the bash path that gate.sh
# exercises locally.

[CmdletBinding()]
param(
    [string]$WixExe = $env:WIX_EXE,
    [string]$VersionOverride,
    [switch]$Sign,
    [switch]$SkipSigning
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

function Fail([string]$Message) {
    Write-Error "build-msi.ps1: $Message"
    exit 1
}

# --- Ensure the config environment ----------------------------------------
if (-not $env:PRODUCT_NAME) {
    $py = Get-Command python3 -ErrorAction SilentlyContinue
    if (-not $py) { $py = Get-Command python -ErrorAction SilentlyContinue }
    if (-not $py) {
        Fail "product config env is absent and no python3/python found to bootstrap it. Run: python3 scripts/packaging/packaging-env.py --cli-package lumen-cli --desktop-package desktop --shell | Invoke-Expression (or export the variables yourself)."
    }
    $print = & $py.Source (Join-Path $repoRoot 'scripts\packaging\packaging-env.py') `
        --cli-package lumen-cli --desktop-package desktop --print
    if ($LASTEXITCODE -ne 0) { Fail "packaging-env.py failed" }
    foreach ($line in $print) {
        $idx = $line.IndexOf('=')
        if ($idx -lt 1) { Fail "packaging-env.py emitted an unparseable line: $line" }
        $key = $line.Substring(0, $idx)
        $val = $line.Substring($idx + 1)
        Set-Item -Path "Env:$key" -Value $val
    }
}

# --- Required config (every failure names the config key) ------------------
foreach ($name in @(
        'PRODUCT_NAME', 'PRODUCT_PUBLISHER', 'PRODUCT_DESCRIPTION',
        'PRODUCT_MSI_UPGRADE_CODE', 'PRODUCT_LOWER', 'PRODUCT_VERSION',
        'BIN_CLI', 'BIN_DESKTOP', 'OS_ID', 'ARCH_ID',
        'PACKAGING_WINDOWS_BINDIR')) {
    if (-not (Get-Item -Path "Env:$name" -ErrorAction SilentlyContinue)) {
        Fail "required config env $name is not set (run packaging-env.py first)"
    }
}
$msiVersion = if ($VersionOverride) { $VersionOverride } else { $env:MSI_VERSION }
if (-not $msiVersion) { Fail 'MSI_VERSION is not set (run packaging-env.py first)' }

if ($env:OS_ID -ne 'windows') { Fail "this is the Windows lane (OS_ID=$($env:OS_ID))" }
if (-not $WixExe) { Fail 'WixExe (or $env:WIX_EXE) is required — install the pinned WiX nupkg first' }
if (-not (Test-Path $WixExe)) { Fail "WixExe not found: $WixExe" }

# Per-arch platform flag — the MSI platform must match its binaries.
$wixArch = switch ($env:ARCH_ID) {
    'x86_64' { 'x64' }
    'aarch64' { 'arm64' }
    default { Fail "cannot map ARCH_ID=$($env:ARCH_ID) to a WiX platform" }
}

$targetDir = if ($env:TARGET_DIR) { $env:TARGET_DIR } else { 'target' }
$distDir = if ($env:DIST_DIR) { $env:DIST_DIR } else { 'dist' }

$cliExe = "$($env:BIN_CLI).exe"
$desktopExe = "$($env:BIN_DESKTOP).exe"
foreach ($exe in @($cliExe, $desktopExe)) {
    $path = Join-Path $targetDir "release\$exe"
    if (-not (Test-Path $path)) {
        Fail "missing packaging input: $path (build first: cargo build --release --locked)"
    }
}

# --- Stage -------------------------------------------------------------------
$stage = New-Item -ItemType Directory -Path ([IO.Path]::GetTempPath()) `
    -Name ("lumen-msi-" + [IO.Path]::GetRandomFileName()) | Select-Object -ExpandProperty FullName
try {
    Copy-Item (Join-Path $targetDir "release\$cliExe") $stage
    Copy-Item (Join-Path $targetDir "release\$desktopExe") $stage

    # --- .wxs generation (config -> XML; mirrors gen-wxs.sh) ---------------
    function XmlEscape([string]$Text) {
        return $Text.Replace('&', '&amp;').Replace('<', '&lt;').
            Replace('>', '&gt;').Replace('"', '&quot;')
    }
    $escName = XmlEscape $env:PRODUCT_NAME
    $escManufacturer = XmlEscape $env:PRODUCT_PUBLISHER
    $escDescription = XmlEscape $env:PRODUCT_DESCRIPTION

    $wxs = @"
<?xml version="1.0" encoding="UTF-8"?>
<!-- Generated by scripts/packaging/build-msi.ps1 - DO NOT HAND-EDIT.
     All identity fields come from product.toml (via packaging-env.py);
     regenerate instead of editing (MP 5 single source of truth).
     Shape mirrors scripts/release/gen-wxs.sh - change both together. -->
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">
  <Package Name="$escName"
           Manufacturer="$escManufacturer"
           Version="$msiVersion"
           UpgradeCode="$($env:PRODUCT_MSI_UPGRADE_CODE)"
           Scope="perMachine"
           Language="1033">
    <MajorUpgrade DowngradeErrorMessage="A newer version of [ProductName] is already installed." />
    <MediaTemplate EmbedCab="yes" />
    <StandardDirectory Id="ProgramFiles64Folder">
      <Directory Id="INSTALLDIR" Name="$escName">
        <Component Id="BinCli" Guid="*">
          <File Id="BinCliExe" Source="$cliExe" KeyPath="yes" />
        </Component>
        <Component Id="BinDesktop" Guid="*">
          <File Id="BinDesktopExe" Source="$desktopExe" KeyPath="yes" />
        </Component>
        <Component Id="Docs" Guid="*">
          <File Id="LicenseFile" Source="LICENSE" KeyPath="yes" />
          <File Id="ReadmeFile" Source="README.md" />
        </Component>
      </Directory>
    </StandardDirectory>
    <Feature Id="Main" Level="1" Title="$escName">
      <ComponentRef RefId="BinCli" />
      <ComponentRef RefId="BinDesktop" />
      <ComponentRef RefId="Docs" />
    </Feature>
  </Package>
</Wix>
"@

    # Docs ride beside the exes in the staging dir (File/@Source
    # basenames resolve from the wix build working directory).
    foreach ($doc in ($env:PACKAGING_DOCS -split '\s+' | Where-Object { $_ })) {
        $src = Join-Path $repoRoot $doc
        if (-not (Test-Path $src)) { Fail "packaging.toml shipped.docs lists $doc - not found" }
        Copy-Item $src $stage
    }

    $wxsPath = Join-Path $stage 'installer.wxs'
    [IO.File]::WriteAllText($wxsPath, $wxs)

    New-Item -ItemType Directory -Force -Path $distDir | Out-Null
    $distAbs = (Resolve-Path $distDir).Path
    $base = "$($env:PRODUCT_LOWER)-$($env:PRODUCT_VERSION)-$($env:ARCH_ID)-$($env:OS_ID)"
    $msiPath = Join-Path $distAbs "$base.msi"

    Push-Location $stage
    try {
        & $WixExe build -arch $wixArch installer.wxs -o $msiPath
        if ($LASTEXITCODE -ne 0) { Fail 'wix build failed' }
    }
    finally { Pop-Location }

    if (-not (Test-Path $msiPath) -or (Get-Item $msiPath).Length -eq 0) {
        Fail "MSI is missing or empty: $msiPath"
    }
    Copy-Item $wxsPath (Join-Path $distAbs "$base.wxs")
    Write-Output "build-msi.ps1: wrote $msiPath (arch=$wixArch, upgrade-code=$($env:PRODUCT_MSI_UPGRADE_CODE))"

    # --- Signing: ONE implementation (sign-windows.sh, via git-bash) -------
    if ($Sign) {
        $bash = Get-Command bash -ErrorAction SilentlyContinue
        if (-not $bash) { Fail '-Sign requested but no bash on PATH (git-bash) to run scripts/packaging/sign-windows.sh' }
        & $bash.Source (Join-Path $repoRoot 'scripts\packaging\sign-windows.sh') $msiPath
        if ($LASTEXITCODE -ne 0) { Fail 'sign-windows.sh failed' }
    }
    elseif (-not $SkipSigning) {
        Write-Output 'build-msi.ps1: signing not requested (pass -Sign, or -SkipSigning to silence this note)'
    }
}
finally {
    Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
}

# Explicit success exit so a caller relying on $LASTEXITCODE after `&
# build-msi.ps1` (test-msi-upgrade.ps1 does) never sees a stale value.
exit 0
