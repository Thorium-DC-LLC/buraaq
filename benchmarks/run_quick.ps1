# Quick cross-language performance comparison (CPU microbenchmarks)
# Usage: .\run_quick.ps1 [-Release] [-ReleaseFast] [-Bench name1,name2]
param(
    [switch]$Release,
    [switch]$ReleaseFast,
    [string]$Bench = "integer_sum,fib_iter,numerical_loop"
)

$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot
$Out = Join-Path $Root "results\quick"
New-Item -ItemType Directory -Force -Path $Out | Out-Null

$Opt = if ($ReleaseFast) { "-O3" } elseif ($Release) { "-O2" } else { "-O0" }
$Lto = if ($ReleaseFast) { @("-flto=thin") } else { @() }
$Benches = $Bench.Split(",") | ForEach-Object { $_.Trim() } | Where-Object { $_ }

$BuraaqCandidate = (Join-Path $Root "..\compiler\target\release\buraaq.exe")
if (-not (Test-Path -LiteralPath $BuraaqCandidate)) {
    Write-Error "Buraaq compiler not found at $BuraaqCandidate. Run: cd compiler; cargo build --release"
}
$Buraaq = (Get-Item -LiteralPath $BuraaqCandidate).FullName
$Clang = (Get-Command clang -ErrorAction SilentlyContinue).Source
if (-not $Clang) { $Clang = "C:\Program Files\LLVM\bin\clang.exe" }

$rows = New-Object System.Collections.Generic.List[object]
$Report = Join-Path $Out "report-$(Get-Date -Format 'yyyyMMdd-HHmmss').txt"
$lines = @(
    "Buraaq quick performance comparison",
    "opt=$Opt",
    "benches=$($Benches -join ', ')",
    ""
)

function Add-Line($s) { $script:lines += $s; Write-Host $s }

function Parse-BenchLine($line) {
    if ($line -notmatch '^BENCH name=(\S+) time_sec=([0-9.]+) ops=(\d+) ops_per_sec=([0-9.]+)') { return $null }
    [pscustomobject]@{
        Bench       = $Matches[1]
        Lang        = $null
        TimeSec     = [double]$Matches[2]
        Ops         = [int64]$Matches[3]
        OpsPerSec   = [double]$Matches[4]
    }
}

function Record-Bench($lang, $line) {
    $parsed = Parse-BenchLine $line
    if ($null -eq $parsed) { return }
    $parsed.Lang = $lang
    $script:rows.Add($parsed) | Out-Null
    Add-Line "  $lang : $line"
}

function Run-Cpp($name) {
    $src = Join-Path $Root "cpp\$name.cpp"
    if (-not (Test-Path $src)) { Add-Line "SKIP cpp/$name (no source)"; return }
    if (-not (Test-Path $Clang)) { Add-Line "SKIP cpp/$name (no clang)"; return }
    $exe = Join-Path $Out "$name-cpp.exe"
    & $Clang $Opt @Lto $src "-I$(Join-Path $Root suite)" "-o" $exe "-D_CRT_SECURE_NO_WARNINGS" 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { Add-Line "FAIL cpp/$name compile"; return }
    $out = & $exe 2>&1 | Out-String
    $out.Trim().Split("`n") | ForEach-Object { if ($_ -match "^BENCH") { Record-Bench "C++" $_ } }
}

function Run-Buraaq($name) {
    $src = Join-Path $Root "buraaq\$name.bq"
    if (-not (Test-Path $src)) { Add-Line "SKIP buraaq/$name (no source)"; return }
    if (-not (Test-Path $Buraaq)) { Add-Line "SKIP buraaq/$name (compiler not built)"; return }
    $flags = if ($ReleaseFast) { @("--release-fast") } elseif ($Release) { @("--release") } else { @() }
    $exe = Join-Path $Out "$name-bq.exe"
    Push-Location $Out
    try {
        $buildArgs = @("build") + $flags + @($src, "-o", $exe)
        $ErrorActionPreference = "Continue"
        & $Buraaq @buildArgs 2>&1 | Out-Null
        $ErrorActionPreference = "Stop"
        if ($LASTEXITCODE -ne 0) { Add-Line "FAIL buraaq/$name compile"; return }
        $out = & $exe 2>&1 | Out-String
        $out.Trim().Split("`n") | ForEach-Object { if ($_ -match "^BENCH") { Record-Bench "Buraaq" $_ } }
    } finally { Pop-Location }
}

function Run-Rust($name) {
    $dir = Join-Path $Root "rust\$name"
    if (-not (Test-Path $dir)) { Add-Line "SKIP rust/$name"; return }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { Add-Line "SKIP rust/$name (no cargo)"; return }
    Push-Location $dir
    try {
        $ErrorActionPreference = "Continue"
        $out = cargo run --release 2>&1 | Out-String
        $ErrorActionPreference = "Continue"
        $out.Trim().Split("`n") | ForEach-Object { if ($_ -match "^BENCH") { Record-Bench "Rust" $_ } }
    } finally { Pop-Location }
}

function Run-Go($name) {
    $dir = Join-Path $Root "go\$name"
    if (-not (Test-Path $dir)) { Add-Line "SKIP go/$name"; return }
    if (-not (Get-Command go -ErrorAction SilentlyContinue)) { Add-Line "SKIP go/$name (no go)"; return }
    Push-Location $dir
    try {
        $env:GOGC = "off"
        $out = go run -ldflags="-s -w" . 2>&1 | Out-String
        $out.Trim().Split("`n") | ForEach-Object { if ($_ -match "^BENCH") { Record-Bench "Go" $_ } }
    } finally { Pop-Location }
}

function Run-Python($name) {
    $src = Join-Path $Root "python\$name.py"
    if (-not (Test-Path $src)) { Add-Line "SKIP python/$name"; return }
    $py = (Get-Command python -ErrorAction SilentlyContinue).Source
    if (-not $py) { Add-Line "SKIP python/$name (no python)"; return }
    $out = & $py $src 2>&1 | Out-String
    $out.Trim().Split("`n") | ForEach-Object { if ($_ -match "^BENCH") { Record-Bench "Python" $_ } }
}

foreach ($b in $Benches) {
    Add-Line "=== $b ==="
    Run-Cpp $b
    Run-Buraaq $b
    Run-Rust $b
    Run-Go $b
    Run-Python $b
    Add-Line ""
}

Add-Line "=== Summary (ops/sec, higher is faster) ==="
foreach ($b in $Benches) {
    $group = $rows | Where-Object { $_.Bench -eq $b } | Sort-Object OpsPerSec -Descending
    if (-not $group) { continue }
    $best = $group[0].OpsPerSec
    Add-Line ""
    Add-Line "$b :"
    foreach ($r in $group) {
        $pct = if ($best -gt 0) { [math]::Round(100.0 * $r.OpsPerSec / $best, 1) } else { 0 }
        $line = "  $($r.Lang.PadRight(8)) $($r.OpsPerSec.ToString('N0').PadLeft(12)) ops/sec ($pct% of fastest)"
        Add-Line $line
    }
}

$lines | Set-Content -Encoding utf8 $Report
Add-Line ""
Add-Line "Report: $Report"
