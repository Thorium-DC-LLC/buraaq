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
Copy-Item -Force $src $dst
Write-Host "Installed $dst"

$ensure = Join-Path $Root "scripts\ensure-llvm.ps1"
if (Test-Path $ensure) {
    & $ensure
}

Write-Host "Add $Prefix to PATH if buraaq is not found."
Write-Host "Then run: buraaq doctor"
Write-Host "Rust was not required."
