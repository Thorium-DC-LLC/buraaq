# Fetch or locate LLVM for Buraaq (Windows). Does not commit LLVM into git.
# Installs a sidecar at %LOCALAPPDATA%\buraaq\llvm when clang is missing.
param(
    [switch]$Winget
)

$ErrorActionPreference = "Stop"
$Sidecar = Join-Path $env:LOCALAPPDATA "buraaq\llvm\bin\clang.exe"

function Test-Clang([string]$Path) {
    if (-not $Path) { return $false }
    if (-not (Test-Path $Path)) { return $false }
    & $Path --version 2>$null | Out-Null
    return $LASTEXITCODE -eq 0
}

$found = @(
    $env:BURAAQ_CLANG,
    $Sidecar,
    "${env:ProgramFiles}\LLVM\bin\clang.exe",
    "clang"
) | Where-Object { Test-Clang $_ } | Select-Object -First 1

if ($found) {
    Write-Host "clang: $found"
    exit 0
}

Write-Host "clang not found. Installing LLVM sidecar for Buraaq."
if ($Winget -or (Get-Command winget -ErrorAction SilentlyContinue)) {
    winget install --id LLVM.LLVM -e --accept-source-agreements --accept-package-agreements
    $pf = "${env:ProgramFiles}\LLVM\bin\clang.exe"
    if (Test-Clang $pf) {
        New-Item -ItemType Directory -Force -Path (Split-Path $Sidecar) | Out-Null
        # Point the sidecar at the real install without copying 400MB.
        Set-Content -Path (Join-Path (Split-Path $Sidecar) "BURAAQ_CLANG.txt") -Value $pf
        $env:BURAAQ_CLANG = $pf
        Write-Host "Set BURAAQ_CLANG=$pf (add this to your user environment)"
        exit 0
    }
}

Write-Error @"
Could not install LLVM automatically.
Install LLVM, then either:
  1. Add 'C:\Program Files\LLVM\bin' to PATH, or
  2. setx BURAAQ_CLANG `"C:\Program Files\LLVM\bin\clang.exe`"
"@
