# build-msi.ps1 -- native Windows MSI builder (issue #56, card 8.E1, task
# W24-C; doc-basename XML escaping + the hostile-fixture -SelfTest
# added in W24-U for the W24-T audit's F-1 -- the .ps1 twin of
# gen-wxs.sh's W24-Q fix, per the twins' change-both-together rule).
#
# The PowerShell twin of scripts/packaging/build-msi.sh (the CI path;
# the release workflow runs bash by default). This script is the
# Windows-native developer path and the engine behind the MSI
# upgrade-path test (test-msi-upgrade.ps1): stage -> generate the WiX
# v4/v5 .wxs from the config environment -> `wix build` per-arch.
#
# CONFIG-DRIVEN, ZERO IDENTITY LITERALS: every identity field (product
# name, publisher, description, numeric MSI version, the frozen
# UpgradeCode, all three exe names) is read from $env: variables
# exported by scripts/packaging/packaging-env.py (which validates
# packaging/packaging.toml against its schema and re-exports the
# product.toml identity resolved by scripts/release/product-env.py).
# When the environment is missing, this script bootstraps it by running
# packaging-env.py itself (python3/python required).
#
# THIRD SHIPPED BINARY (w26.m.t3): BIN_TAURI (the workspace-excluded
# tauri admission crate's bin, built by CI's release workflow
# separately) rides exactly like BIN_CLI/BIN_DESKTOP -- staged beside
# them and carried by its own BinTauri Component/ComponentRef pair.
# The tauri exe renders through the WebView2 Evergreen Runtime, a
# SYSTEM component on Win10/11 (preinstalled on Win11; distributed by
# the Evergreen setup on Win10): no runtime is bundled or bootstrapped
# here and no extra dependency row is needed -- the assumption is
# pinned in the generated XML beside the BinTauri component.
#
# The .wxs shape MIRRORS scripts/release/gen-wxs.sh (the repo's one
# bash generator, used by build-msi.sh). Change both together -- the
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
#                     bash (git-bash) -- the ONE signing implementation.
#   -SkipSigning      explicit opt-out (CI signs in its own step).
#   -SelfTest         hostile-fixture regression test for the .wxs
#                     generation (no config env, no wix, no binaries
#                     needed; needs python3/python for the XML proof --
#                     the same interpreter CI provides for
#                     packaging-env.py). Mirrors gen-wxs.sh --self-test.
#
# HONEST LOCAL-PROOF BOUNDARY: authored and reviewed on Linux; executed
# on Windows only [VERIFY at first live run]. The env assertions run
# locally under gate.sh (step 1 exercises the same packaging-env.py
# bridge this script bootstraps), and the .wxs generation carries its
# own executable -SelfTest -- `wix build` itself needs Windows + .NET
# and stays CI-only.

[CmdletBinding()]
param(
    [string]$WixExe = $env:WIX_EXE,
    [string]$VersionOverride,
    [switch]$Sign,
    [switch]$SkipSigning,
    [switch]$SelfTest
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

function Fail([string]$Message) {
    Write-Error "build-msi.ps1: $Message"
    exit 1
}

# --- XML escaping: the FULL set, mirroring gen-wxs.sh ------------------------
# Every value interpolated into an XML attribute goes through XmlEscape
# -- the identity fields always did, and since W24-U (W24-T audit F-1)
# the doc BASENAMES in File/@Source do too (a doc named `a&b.md` used
# to yield malformed XML, exactly as W24-R6 P4-2 described for the
# bash twin). The set is the full one (& < > " ', ' as &apos;
# included), identical to gen-wxs.sh's xml_escape; change both together.
function XmlEscape([string]$Text) {
    return $Text.Replace('&', '&amp;').Replace('<', '&lt;').
        Replace('>', '&gt;').Replace('"', '&quot;').Replace("'", '&apos;')
}

# --- .wxs generation (config -> XML; mirrors gen-wxs.sh) ---------------------
# The ONE generation path, a function so -SelfTest exercises the REAL
# code (never a copy that could drift from it) -- the same discipline
# as gen-wxs.sh --self-test. The FIRST doc is the component KeyPath
# (deterministic; exactly one File per component may be KeyPath).
# File/@Id must be a legal identifier: non-word characters collapse to
# '_' and the positional prefix keeps ids collision-free. File/@Source
# carries the BASENAME, XML-escaped like every other attribute value --
# a doc named `a&b<c>"d".md` must produce well-formed XML. Source
# basenames resolve from the staging dir (the wix build working
# directory), so two configured docs sharing a basename would silently
# clobber in staging -- refused loudly here instead (mirrors gen-wxs.sh
# -- change both together).
function New-MsiWxs {
    param(
        [string]$Name,
        [string]$Manufacturer,
        [string]$Description,
        [string]$Version,
        [string]$UpgradeCode,
        [string]$CliExe,
        [string]$DesktopExe,
        [string]$TauriExe,
        [string[]]$DocPaths
    )
    $escName = XmlEscape $Name
    $escManufacturer = XmlEscape $Manufacturer
    $escDescription = XmlEscape $Description

    $docsRows = @()
    $seenBase = @{}
    foreach ($doc in $DocPaths) {
        $base = [IO.Path]::GetFileName($doc)
        if ($seenBase.ContainsKey($base)) {
            Fail "packaging.toml shipped.docs lists two docs with the same basename '$base' ($doc and $($seenBase[$base])) - the MSI staging dir is flat"
        }
        $seenBase[$base] = $doc
        $id = 'Doc_' + ($docsRows.Count + 1) + '_' + ($base -replace '[^0-9A-Za-z_]', '_')
        $escBase = XmlEscape $base
        if ($docsRows.Count -eq 0) {
            $docsRows += "          <File Id=`"$id`" Source=`"$escBase`" KeyPath=`"yes`" />"
        }
        else {
            $docsRows += "          <File Id=`"$id`" Source=`"$escBase`" />"
        }
    }
    if ($docsRows.Count -eq 0) { Fail 'packaging.toml [shipped] docs is empty - the MSI Docs component needs at least one file' }
    $docsXml = $docsRows -join "`n"

    $wxs = @"
<?xml version="1.0" encoding="UTF-8"?>
<!-- Generated by scripts/packaging/build-msi.ps1 - DO NOT HAND-EDIT.
     All identity fields come from product.toml and the docs list from
     packaging.toml [shipped] docs (both via packaging-env.py);
     regenerate instead of editing (MP 5 single source of truth).
     Shape mirrors scripts/release/gen-wxs.sh - change both together. -->
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">
  <Package Name="$escName"
           Manufacturer="$escManufacturer"
           Version="$Version"
           UpgradeCode="$UpgradeCode"
           Scope="perMachine"
           Language="1033">
    <MajorUpgrade DowngradeErrorMessage="A newer version of [ProductName] is already installed." />
    <MediaTemplate EmbedCab="yes" />
    <StandardDirectory Id="ProgramFiles64Folder">
      <Directory Id="INSTALLDIR" Name="$escName">
        <Component Id="BinCli" Guid="*">
          <File Id="BinCliExe" Source="$CliExe" KeyPath="yes" />
        </Component>
        <Component Id="BinDesktop" Guid="*">
          <File Id="BinDesktopExe" Source="$DesktopExe" KeyPath="yes" />
        </Component>
        <!-- WebView2 assumption: the tauri exe renders through the
             WebView2 Evergreen Runtime, a system component on
             Win10/11 (preinstalled on Win11; distributed by the
             Evergreen setup on Win10). No runtime is bundled or
             bootstrapped here. Kept in lockstep with the twin
             generator (change both together). -->
        <Component Id="BinTauri" Guid="*">
          <File Id="BinTauriExe" Source="$TauriExe" KeyPath="yes" />
        </Component>
        <Component Id="Docs" Guid="*">
$docsXml
        </Component>
      </Directory>
    </StandardDirectory>
    <Feature Id="Main" Level="1" Title="$escName">
      <ComponentRef Id="BinCli" />
      <ComponentRef Id="BinDesktop" />
      <ComponentRef Id="BinTauri" />
      <ComponentRef Id="Docs" />
    </Feature>
  </Package>
</Wix>
"@
    return $wxs
}

# --- -SelfTest: hostile fixtures must yield WELL-FORMED XML ------------------
# Regression test for W24-T F-1 (the .ps1 twin of gen-wxs.sh --self-test,
# added in W24-Q for W24-R6 P4-2): doc basenames used to be interpolated
# into File/@Source UNESCAPED, producing output no XML parser accepts.
# Runs the REAL New-MsiWxs with hostile identity values + hostile doc
# basenames, then proves with python's XML parser that the result parses
# AND that the hostile values round-trip. The raw-text assertions pin the
# escape SET itself: parse + round-trip alone cannot catch a reverted
# &apos; (a bare ' parses fine inside a double-quoted attribute).
if ($SelfTest) {
    $py = Get-Command python3 -ErrorAction SilentlyContinue
    if (-not $py) { $py = Get-Command python -ErrorAction SilentlyContinue }
    if (-not $py) {
        Fail '-SelfTest needs python3/python (the XML well-formedness proof - the same interpreter CI provides for packaging-env.py)'
    }
    $work = Join-Path ([IO.Path]::GetTempPath()) ('build-msi-selftest-' + [IO.Path]::GetRandomFileName())
    New-Item -ItemType Directory -Path $work | Out-Null
    $rc = 1
    try {
        # Hostile fixtures (test data only - never real identity
        # values): every XML metacharacter in the identity fields and
        # BOTH doc basenames (' included, per the escaping contract) --
        # the same fixtures as gen-wxs.sh --self-test.
        $wxs = New-MsiWxs -Name 'A&B "C" <D>' -Manufacturer 'P&Q' `
            -Description "it's <a> & `"b`" > c" -Version '1.2.3' `
            -UpgradeCode '11111111-2222-3333-4444-555555555555' `
            -CliExe 'selftest-cli.exe' -DesktopExe 'selftest-desktop.exe' `
            -TauriExe 'selftest-tauri.exe' `
            -DocPaths @('a&b<c>"d".md', "it's.md")
        $wxsPath = Join-Path $work 'hostile.wxs'
        [IO.File]::WriteAllText($wxsPath, $wxs)

        $proof = @'
import sys
import xml.etree.ElementTree as ET

path = sys.argv[1]
raw = open(path, encoding="utf-8").read()
root = ET.parse(path).getroot()  # malformed XML -> parser death = test failure
ns = "{http://wixtoolset.org/schemas/v4/wxs}"
files = root.findall(f".//{ns}File")
assert files, "no File elements generated"
assert len(files) == 5, f"expected 5 File rows (3 exes + 2 docs), got {len(files)}"

sources = [f.get("Source") for f in files]
for hostile in ('a&b<c>"d".md', "it's.md"):
    assert hostile in sources, (
        f"doc basename did not round-trip through the XML escaping: "
        f"{hostile!r} not in {sources!r}")

# The three shipped exe rows (CLI, desktop, tauri) must all be present,
# each as its component's KeyPath (the w26.m.t3 payload added BinTauri;
# mirrors gen-wxs.sh --self-test).
exe_rows = ('selftest-cli.exe', 'selftest-desktop.exe', 'selftest-tauri.exe')
for exe in exe_rows:
    assert exe in sources, f"shipped exe row missing from the generated .wxs: {exe!r}"
for f in files:
    if f.get("Source") in exe_rows:
        assert f.get("KeyPath") == "yes", (
            f"exe File row is not its component's KeyPath: {f.get('Source')!r}")

# Every component must be referenced by the Feature, by Id= (a RefId=
# typo would silently ship a component outside the feature).
comp_ids = {c.get("Id") for c in root.findall(f".//{ns}Component")}
ref_ids = {r.get("Id") for r in root.findall(f".//{ns}ComponentRef")}
assert comp_ids, "no Component elements generated"
assert comp_ids == ref_ids, (
    f"Feature ComponentRefs do not cover every component: "
    f"components={sorted(comp_ids)!r} refs={sorted(ref_ids)!r}")
for r in root.findall(f".//{ns}ComponentRef"):
    assert r.get("Id") and "RefId" not in r.attrib, (
        "ComponentRef must carry Id= (never a RefId= attribute)")
assert "BinTauri" in comp_ids, "the BinTauri component row is missing"

# The escape SET pinned on the raw text (the one proof that catches a
# reverted &apos; -- a bare ' parses fine in a double-quoted attribute,
# so parse + round-trip alone cannot).
assert 'Source="a&amp;b&lt;c&gt;&quot;d&quot;.md"' in raw, (
    "raw .wxs does not carry the fully escaped doc basename "
    "(expected a&amp;b&lt;c&gt;&quot;d&quot;.md)")
assert 'Source="it&apos;s.md"' in raw, (
    "raw .wxs does not escape ' as &apos; -- the .ps1 escape set must "
    "mirror gen-wxs.sh's full set")

ids = [f.get("Id") for f in files]
assert len(ids) == len(set(ids)), f"File ids collide: {ids!r}"
for i in ids:
    assert i and all(c.isalnum() or c == "_" for c in i), (
        f"File id not sanitized to a legal identifier: {i!r}")

pkg = root.find(f"{ns}Package")
assert pkg.get("Name") == 'A&B "C" <D>', "hostile identity Name did not round-trip"
assert pkg.get("Manufacturer") == "P&Q", "hostile identity Manufacturer did not round-trip"

print(f"build-msi.ps1: -SelfTest PASS ({len(files)} File rows parsed; "
      f"hostile basenames + identity round-trip; escape set pinned on "
      f"raw text; ids sanitized; exe rows KeyPath; Feature refs cover "
      f"every component by Id=)")
'@
        $proofPath = Join-Path $work 'proof.py'
        [IO.File]::WriteAllText($proofPath, $proof)

        & $py.Source $proofPath $wxsPath
        $rc = $LASTEXITCODE
    }
    finally {
        Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
    }
    if ($rc -ne 0) { Fail '-SelfTest: the XML proof failed (see the python output above)' }
    exit 0
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
        'BIN_CLI', 'BIN_DESKTOP', 'BIN_TAURI', 'OS_ID', 'ARCH_ID',
        'PACKAGING_WINDOWS_BINDIR', 'PACKAGING_DOCS')) {
    if (-not (Get-Item -Path "Env:$name" -ErrorAction SilentlyContinue)) {
        Fail "required config env $name is not set (run packaging-env.py first)"
    }
}
$msiVersion = if ($VersionOverride) { $VersionOverride } else { $env:MSI_VERSION }
if (-not $msiVersion) { Fail 'MSI_VERSION is not set (run packaging-env.py first)' }

if ($env:OS_ID -ne 'windows') { Fail "this is the Windows lane (OS_ID=$($env:OS_ID))" }
if (-not $WixExe) { Fail 'WixExe (or $env:WIX_EXE) is required -- install the pinned WiX nupkg first' }

# --- WIX_EXE POSIX -> Windows path (W24-R1 P2-2) ---------------------------
# The release workflow installs WiX under bash (git-bash on the Windows
# runner) and exports WIX_EXE=/d/a/<repo>/<repo>/.tools/... -- a POSIX
# path. PowerShell Test-Path resolves a leading '/' against the CURRENT
# drive (D:\d\a\...), which does not exist, so the MSI upgrade-path
# test (which calls this script with -WixExe omitted, defaulting to
# $env:WIX_EXE) would die at its first live run. Convert the git-bash
# drive-letter form (/d/... -> D:\...); anything else passes through
# untouched. [VERIFY at first live run]
if ($WixExe -match '^/([A-Za-z])/(.+)$') {
    $WixExe = $Matches[1].ToUpper() + ':\' + (($Matches[2] -split '/') -join '\')
    Write-Output "build-msi.ps1: converted POSIX WIX_EXE (git-bash) to Windows form: $WixExe"
}

if (-not (Test-Path $WixExe)) { Fail "WixExe not found: $WixExe" }

# Per-arch platform flag -- the MSI platform must match its binaries.
$wixArch = switch ($env:ARCH_ID) {
    'x86_64' { 'x64' }
    'aarch64' { 'arm64' }
    default { Fail "cannot map ARCH_ID=$($env:ARCH_ID) to a WiX platform" }
}

$targetDir = if ($env:TARGET_DIR) { $env:TARGET_DIR } else { 'target' }
$distDir = if ($env:DIST_DIR) { $env:DIST_DIR } else { 'dist' }

$cliExe = "$($env:BIN_CLI).exe"
$desktopExe = "$($env:BIN_DESKTOP).exe"
$tauriExe = "$($env:BIN_TAURI).exe"
foreach ($exe in @($cliExe, $desktopExe, $tauriExe)) {
    $path = Join-Path $targetDir "release\$exe"
    if (-not (Test-Path $path)) {
        Fail "missing packaging input: $path (build first: cargo build --release --locked)"
    }
}

# --- Stage -------------------------------------------------------------------
$stage = New-Item -ItemType Directory -Path ([IO.Path]::GetTempPath()) `
    -Name ("$($env:PRODUCT_LOWER)-msi-" + [IO.Path]::GetRandomFileName()) | Select-Object -ExpandProperty FullName
try {
    Copy-Item (Join-Path $targetDir "release\$cliExe") $stage
    Copy-Item (Join-Path $targetDir "release\$desktopExe") $stage
    Copy-Item (Join-Path $targetDir "release\$tauriExe") $stage

    # --- Docs: stage the CONFIG list (W24-R1 P2-1) ---------------------------
    # The Docs component is generated from packaging.toml [shipped] docs
    # ($env:PACKAGING_DOCS -- space-separated repo-relative paths), NOT a
    # hardcoded pair: a docs-list change in the TOML must reach the MSI
    # exactly as it reaches tar.gz/zip/deb/rpm/AppImage. The .wxs rows
    # themselves come from New-MsiWxs below -- the basename, duplicate
    # and XML-escaping decisions live there, mirroring gen-wxs.sh
    # (change both together).
    $docPaths = @($env:PACKAGING_DOCS -split '\s+' | Where-Object { $_ })
    foreach ($doc in $docPaths) {
        $src = Join-Path $repoRoot $doc
        if (-not (Test-Path $src)) { Fail "packaging.toml shipped.docs lists $doc - not found" }
        Copy-Item $src $stage
    }

    # --- .wxs generation: the REAL path (-SelfTest exercises it) ------------
    $wxs = New-MsiWxs -Name $env:PRODUCT_NAME -Manufacturer $env:PRODUCT_PUBLISHER `
        -Description $env:PRODUCT_DESCRIPTION -Version $msiVersion `
        -UpgradeCode $env:PRODUCT_MSI_UPGRADE_CODE -CliExe $cliExe `
        -DesktopExe $desktopExe -TauriExe $tauriExe -DocPaths $docPaths

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
