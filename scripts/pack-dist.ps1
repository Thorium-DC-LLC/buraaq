# Pack the full compiler into dist\ for user install (Rust is not required on the user machine).
# Contributors only: this script may invoke Cargo on a packager machine.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Exe = Join-Path $Root "compiler\target\release\buraaq.exe"
if (-not (Test-Path $Exe)) {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "pack-dist needs an already-built compiler or Cargo on the packager machine."
    }
    Push-Location (Join-Path $Root "compiler")
    try {
        cargo build -p buraaq --release
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    } finally {
        Pop-Location
    }
}
$Dist = Join-Path $Root "dist"
New-Item -ItemType Directory -Force -Path $Dist | Out-Null
Copy-Item -Force $Exe (Join-Path $Dist "buraaq.exe")
Write-Host "Packed $(Join-Path $Dist 'buraaq.exe')"
Write-Host "Users install with: .\install.ps1  (Rust is not required)"
