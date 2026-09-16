# Reproducible concurrency benchmark runner (Windows / cross-platform via clang)
# Usage: .\run.ps1 [-Release]
param([switch]$Release)

$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot
$Harness = Join-Path $Root "harness"
$StdRuntime = Join-Path $Root "..\stdlib\runtime"
$Out = Join-Path $Root "results"
New-Item -ItemType Directory -Force -Path $Out | Out-Null

$Opt = if ($Release) { "-O2" } else { "-O0" }
$Clang = $null
foreach ($c in @("clang", "clang-18", "clang-17", "gcc", "cc")) {
    if (Get-Command $c -ErrorAction SilentlyContinue) { $Clang = $c; break }
}
if (-not $Clang) { Write-Error "no C compiler (clang/gcc) found on PATH"; exit 1 }

function Compile-BuraaqBench($name, $src) {
    $exe = Join-Path $Out "$name.exe"
    & $Clang $Opt $src (Join-Path $StdRuntime "buraaq_runtime.c") (Join-Path $StdRuntime "buraaq_std.c") "-I$Harness" "-I$StdRuntime" "-o" $exe
    if ($LASTEXITCODE -ne 0) { throw "compile failed: $name" }
    return $exe
}

Write-Host "=== Buraaq runtime benchmarks ($Opt) ===" -ForegroundColor Cyan
$results = @()

foreach ($b in @(
    @{ n = "task_spawn"; s = "buraaq_task_spawn.c" },
    @{ n = "channel"; s = "buraaq_channel.c" },
    @{ n = "mutex"; s = "buraaq_mutex.c" },
    @{ n = "context_switch"; s = "buraaq_context_switch.c" }
)) {
    $exe = Compile-BuraaqBench $b.n (Join-Path $Harness $b.s)
    $line = & $exe
    Write-Host $line
    $results += $line
}

Write-Host "`n=== Reference: Rust ===" -ForegroundColor Cyan
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Push-Location (Join-Path $Root "refs\rust\task_spawn")
    cargo run --release 2>$null | Select-String "BENCH" | ForEach-Object { Write-Host $_; $results += $_.Line }
    Pop-Location
    Push-Location (Join-Path $Root "refs\rust\channel")
    cargo run --release 2>$null | Select-String "BENCH" | ForEach-Object { Write-Host $_; $results += $_.Line }
    Pop-Location
}

Write-Host "`n=== Reference: Go ===" -ForegroundColor Cyan
if (Get-Command go -ErrorAction SilentlyContinue) {
    Push-Location (Join-Path $Root "refs\go\task_spawn")
    go run . | ForEach-Object { Write-Host $_; $results += $_ }
    Pop-Location
    Push-Location (Join-Path $Root "refs\go\channel")
    go run . | ForEach-Object { Write-Host $_; $results += $_ }
    Pop-Location
}

Write-Host "`n=== Reference: C++ ===" -ForegroundColor Cyan
if (Get-Command g++ -ErrorAction SilentlyContinue) {
    $cppExe = Join-Path $Out "task_spawn_cpp.exe"
    & g++ -std=c++17 $(if ($Release) { "-O2" } else { "-O0" }) (Join-Path $Root "refs\cpp\task_spawn.cpp") -o $cppExe
    & $cppExe | ForEach-Object { Write-Host $_; $results += $_ }
}

Write-Host "`n=== Reference: Zig ===" -ForegroundColor Cyan
if (Get-Command zig -ErrorAction SilentlyContinue) {
    Push-Location (Join-Path $Root "refs\zig")
    $opt = if ($Release) { "-O ReleaseFast" } else { "-O Debug" }
    zig c++ $opt task_spawn.zig -o (Join-Path $Out "task_spawn_zig.exe")
    & (Join-Path $Out "task_spawn_zig.exe") | ForEach-Object { Write-Host $_; $results += $_ }
    Pop-Location
}

$stamp = Get-Date -Format "yyyy-MM-dd_HHmmss"
$outFile = Join-Path $Out "run_$stamp.txt"
$results | Set-Content $outFile
Write-Host "`nResults saved to $outFile" -ForegroundColor Green
