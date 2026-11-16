# scripts/quality/astgrep_scan.ps1
# Windows driver for the ast-grep rules. The bash script (03_astgrep.sh) is the
# canonical CI path; this exists because npx under Git-Bash hits the Windows
# Store app-alias exec failure (REGDB_E_CLASSNOTREG).
$ErrorActionPreference = "Continue"
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$rules = Join-Path $PSScriptRoot "rules"
$outd  = Join-Path $root "docs\audit\reports\astgrep"
New-Item -ItemType Directory -Force -Path $outd | Out-Null
Set-Location $root

$paths = @("crates", "apps", "tools", "tests")
foreach ($d in @("plugins")) { if (Test-Path $d) { $paths += $d } }

$summary = @()
foreach ($f in Get-ChildItem $rules -Filter *.yml) {
    $id = $f.BaseName
    $json = Join-Path $outd "$id.json"
    $err  = Join-Path $outd "$id.err"
    $raw = npx --yes -p @ast-grep/cli@0.51.3 ast-grep scan --rule $f.FullName --json @paths 2> $err
    [System.IO.File]::WriteAllText($json, ($raw -join "`n"), (New-Object System.Text.UTF8Encoding($false)))
    $count = 0
    try { $count = ($raw | ConvertFrom-Json).Count } catch { $count = -1 }
    $summary += [pscustomobject]@{ rule = $id; hits = $count }
    Write-Output ("rule={0,-24} hits={1}" -f $id, $count)
}
$summary | Format-Table -AutoSize
$summary | ConvertTo-Json | Out-File -FilePath (Join-Path $outd "summary.json") -Encoding utf8
