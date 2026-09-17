# Install the Buraaq compiler onto PATH (Windows).
# One step. Rust is not required. Prefers the packaged dist\buraaq.exe.
# LLVM: scripts\ensure-llvm.ps1 if clang is missing.
# Usage: .\install.ps1 [-Prefix C:\Users\you\bin]
param(
    [string]$Prefix = (Join-Path $env:USERPROFILE "bin")
)

$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot
New-Item -ItemType Directory -Force -Path $Prefix | Out-Null

$prebuilt = Join-Path $Root "dist\buraaq.exe"
$already = Join-Path $Root "compiler\target\release\buraaq.exe"
$src = $null
if (Test-Path $prebuilt) {
    $src = $prebuilt
} elseif (Test-Path $already) {
    $src = $already
}
if (-not $src) {
    throw "Packaged compiler missing (dist\buraaq.exe). Rust is not required to use Buraaq. Get a packaged tree, or on a packager machine run: .\scripts\pack-dist.ps1"
}

Write-Host "Using packaged compiler $src (Rust is not required)"
$dst = Join-Path $Prefix "buraaq.exe"
$sidecar = Join-Path $Prefix "buraaq1.exe"
# Always refresh sidecar first (PowerShell prefers .exe over .cmd; LSP often locks buraaq.exe).
Copy-Item -Force $src $sidecar
$copied = $false
try {
    Copy-Item -Force $src $dst -ErrorAction Stop
    $copied = $true
} catch {
    $old = Join-Path $Prefix "buraaq.exe.old"
    Remove-Item -Force $old -ErrorAction SilentlyContinue
    Move-Item -Force $dst $old -ErrorAction SilentlyContinue
    try {
        Copy-Item -Force $src $dst -ErrorAction Stop
        $copied = $true
        Write-Host "Replaced locked buraaq.exe (previous saved as buraaq.exe.old)."
    } catch {
        Write-Host "Note: buraaq.exe still locked; buraaq.cmd -> buraaq1.exe has the new build."
    }
}
if ($copied) {
    Write-Host "Installed $dst"
}
@"
@echo off
setlocal
if exist "%~dp0buraaq1.exe" (
  "%~dp0buraaq1.exe" %*
) else (
  "%~dp0buraaq.exe" %*
)
"@ | Set-Content -Encoding ASCII (Join-Path $Prefix "buraaq.cmd")
Write-Host "Also installed buraaq.cmd / buraaq1.exe shim."

# discover_sysroot looks next to the exe: <prefix>/sysroot/src/io.bq
$sys = Join-Path $Prefix "sysroot"
$stdlib = Join-Path $Root "stdlib"
if (-not (Test-Path (Join-Path $stdlib "src\io.bq"))) {
    throw "stdlib missing under $stdlib (needed next to the installed compiler)"
}
New-Item -ItemType Directory -Force -Path $sys | Out-Null
foreach ($part in @("src", "runtime")) {
    $from = Join-Path $stdlib $part
    $to = Join-Path $sys $part
    if (-not (Test-Path $from)) { throw "missing $from" }
    if (Test-Path $to) { Remove-Item -Recurse -Force $to }
    Copy-Item -Recurse $from $to
}
$pkg = Join-Path $stdlib "buraaq.pkg"
if (Test-Path $pkg) { Copy-Item -Force $pkg (Join-Path $sys "buraaq.pkg") }
$rt = Join-Path $Root "compiler\runtime\buraaq_rt.c"
if (Test-Path $rt) { Copy-Item -Force $rt (Join-Path $sys "runtime\buraaq_rt.c") }
Write-Host "Installed sysroot $sys"

$ensure = Join-Path $Root "scripts\ensure-llvm.ps1"
if (Test-Path $ensure) {
    & $ensure
}

# Ensure User PATH contains the install prefix (Machine PATH is easy to miss in new shells).
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$parts = @()
if (-not [string]::IsNullOrWhiteSpace($userPath)) {
    $parts = $userPath -split ';' | Where-Object { $_ -and $_.Trim() -ne "" }
}
if ($parts -notcontains $Prefix) {
    $parts += $Prefix
    [Environment]::SetEnvironmentVariable("Path", ($parts -join ';'), "User")
    Write-Host "Added $Prefix to your User PATH."
} else {
    Write-Host "User PATH already includes $Prefix."
}
$env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [Environment]::GetEnvironmentVariable("Path", "User")

Write-Host ""
Write-Host "Open a new terminal, then:"
Write-Host "  buraaq --version"
Write-Host "  buraaq ai doctor"
Write-Host "  buraaq            # interactive shell"
Write-Host "Rust was not required."
