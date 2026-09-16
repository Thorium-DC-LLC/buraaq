# Run the Buraaq engineer test suite.
# Usage: .\run.ps1 [-Strict]   # Strict fails on experimental compile errors too
param([switch]$Strict)

$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot
$Manifest = Join-Path $Root "manifest.tsv"
if (-not (Get-Command buraaq -ErrorAction SilentlyContinue)) {
    Write-Error "buraaq not on PATH. Run install.ps1 from the repo root."
}

$pass = 0
$fail = 0
$warn = 0
$rows = Get-Content $Manifest | Where-Object { $_ -and -not $_.StartsWith("#") }

foreach ($line in $rows) {
    $cols = $line -split "`t"
    if ($cols.Count -lt 5) { continue }
    $id, $rel, $kind, $mode, $expect = $cols[0..4]
    $path = Join-Path $Root $rel
    Write-Host ("`n=== {0} ({1}) ===" -f $id, $mode) -ForegroundColor Cyan

    try {
        if ($kind -eq "single") {
            Push-Location $Root
            if ($mode -eq "run") {
                $out = & buraaq run $path 2>&1 | Out-String
                if ($LASTEXITCODE -ne 0) { throw "run failed: $out" }
                if ($expect -and $out -notmatch [regex]::Escape($expect)) {
                    throw "stdout missing '$expect': $out"
                }
                Write-Host "PASS run" -ForegroundColor Green
                $pass++
            } elseif ($mode -eq "compile" -or $mode -eq "check") {
                try {
                    $out = & buraaq build $path 2>&1 | Out-String
                    if ($LASTEXITCODE -ne 0) {
                        if ($Strict) { throw "compile failed: $out" }
                        Write-Host "WARN compile (experimental)" -ForegroundColor Yellow
                        Write-Host $out
                        $warn++
                    } else {
                        Write-Host "PASS compile" -ForegroundColor Green
                        $pass++
                    }
                } catch {
                    if ($Strict) { throw }
                    Write-Host "WARN compile crash (experimental): $_" -ForegroundColor Yellow
                    $warn++
                }
            }
            Pop-Location
        } elseif ($kind -eq "project") {
            $proj = Join-Path $Root $rel
            Push-Location $proj
            if ($mode -eq "run") {
                $build = & buraaq run 2>&1 | Out-String
                if ($LASTEXITCODE -ne 0) { throw "run failed: $build" }
                if ($expect -and $build -notmatch [regex]::Escape($expect)) {
                    throw "output missing '$expect': $build"
                }
                Write-Host "PASS run" -ForegroundColor Green
                $pass++
            } else {
                $out = & buraaq check 2>&1 | Out-String
                if ($LASTEXITCODE -ne 0) { throw "check failed: $out" }
                Write-Host "PASS check" -ForegroundColor Green
                $pass++
            }
            Pop-Location
        }
    } catch {
        Write-Host "FAIL $_" -ForegroundColor Red
        $fail++
    }
}

Write-Host "`n--- SUMMARY ---" -ForegroundColor Cyan
Write-Host "pass=$pass fail=$fail experimental_warn=$warn"
if ($fail -gt 0) {
    Write-Host "Required run tests failed. Experimental compile warnings are OK unless you used -Strict." -ForegroundColor Yellow
    exit 1
}
