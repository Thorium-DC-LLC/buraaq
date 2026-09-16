# Cross-language performance suite (honest comparison — reports all results)
# Usage: .\run_suite.ps1 [-Release] [-ReleaseFast]
param(
    [switch]$Release,
    [switch]$ReleaseFast
)

$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot
$Out = Join-Path $Root "results\suite"
New-Item -ItemType Directory -Force -Path $Out | Out-Null

$Opt = if ($ReleaseFast) { "-O3" } elseif ($Release) { "-O2" } else { "-O0" }
$Lto = if ($ReleaseFast) { @("-flto=thin") } else { @() }
$Report = Join-Path $Out "report-$(Get-Date -Format 'yyyyMMdd-HHmmss').txt"
$lines = @("Buraaq benchmark suite", "opt=$Opt", "")

function Add-Line($s) { $script:lines += $s; Write-Host $s }

function Run-Exe($path) {
    if (-not (Test-Path $path)) { return $null }
    $out = & $path 2>&1 | Out-String
    $out.Trim().Split("`n") | ForEach-Object { if ($_ -match "^BENCH") { Add-Line $_ } }
}

function Compile-Cpp($name) {
    $src = Join-Path $Root "cpp\$name.cpp"
    if (-not (Test-Path $src)) { return }
    $exe = Join-Path $Out "$name.exe"
    $clang = (Get-Command clang, clang-18, clang-17, gcc -ErrorAction SilentlyContinue | Select-Object -First 1).Source
    if (-not $clang) { Add-Line "SKIP cpp/$name — no clang"; return }
    & $clang $Opt @Lto $src "-I$(Join-Path $Root suite)" "-o" $exe
    if ($LASTEXITCODE -ne 0) { Add-Line "FAIL cpp/$name compile"; return }
    Add-Line "=== cpp/$name ==="
    Run-Exe $exe
}

function Compile-Buraaq($name) {
    $src = Join-Path $Root "buraaq\$name.bq"
    if (-not (Test-Path $src)) { return }
    $buraaq = (Get-Command buraaq -ErrorAction SilentlyContinue).Source
    if (-not $buraaq) {
        $local = Join-Path $Root "..\compiler\target\release\buraaq.exe"
        if (Test-Path $local) { $buraaq = $local } else { Add-Line "SKIP buraaq/$name — buraaq not on PATH"; return }
    }
    $flags = if ($ReleaseFast) { @("--release-fast") } elseif ($Release) { @("--release") } else { @() }
    $exe = Join-Path $Out "$name-bq.exe"
    Push-Location (Split-Path $src)
    try {
        & $buraaq build @flags $src "-o" $exe 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { Add-Line "FAIL buraaq/$name compile"; return }
        Add-Line "=== buraaq/$name ==="
        Run-Exe $exe
    } finally { Pop-Location }
}

Add-Line "=== C++ equivalent workloads (-O2 when -Release) ==="
foreach ($b in @("numerical_loop", "integer_sum", "nested_loop", "float_saxpy", "fib_iter")) {
    Compile-Cpp $b
}

Add-Line ""
Add-Line "=== Buraaq equivalent workloads (same n as C++) ==="
foreach ($b in @("numerical_loop", "integer_sum", "nested_loop", "float_saxpy", "fib_iter")) {
    Compile-Buraaq $b
}

Add-Line ""
Add-Line "=== Rust (release) ==="
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Push-Location (Join-Path $Root "rust\numerical_loop")
    cargo run --release 2>$null | ForEach-Object { if ($_ -match "^BENCH") { Add-Line $_ } }
    Pop-Location
}

Add-Line ""
Add-Line "=== Go ==="
if (Get-Command go -ErrorAction SilentlyContinue) {
    Push-Location (Join-Path $Root "go\numerical_loop")
    go run . | ForEach-Object { if ($_ -match "^BENCH") { Add-Line $_ } }
    Pop-Location
}

Add-Line ""
Add-Line "=== Zig ==="
if (Get-Command zig -ErrorAction SilentlyContinue) {
    $zexe = Join-Path $Out "numerical_loop-zig.exe"
    $zopt = if ($Release -or $ReleaseFast) { "ReleaseFast" } else { "Debug" }
    zig build-exe (Join-Path $Root "zig\numerical_loop.zig") -O $zopt -femit-bin=$zexe
    Run-Exe $zexe
}

$lines | Set-Content -Encoding utf8 $Report
Add-Line ""
Add-Line "Report written to $Report"
