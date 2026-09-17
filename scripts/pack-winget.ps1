# Build a portable Windows zip for winget / GitHub Releases.
# Layout (sysroot beside exe — discover_sysroot looks for <exeDir>/sysroot):
#   buraaq.exe
#   sysroot/src/...
#   sysroot/runtime/...
#   sysroot/buraaq.pkg
#
# Usage (packager machine with release CLI already built, or Cargo available):
#   .\scripts\pack-winget.ps1
#   .\scripts\pack-winget.ps1 -Version 1.0.1
param(
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot

if (-not $Version) {
    $ws = Get-Content (Join-Path $Root "compiler\Cargo.toml") -Raw
    if ($ws -match '(?m)^version\s*=\s*"([^"]+)"') { $Version = $Matches[1] } else { $Version = "1.0.0" }
}

# Ensure dist\buraaq.exe
& (Join-Path $PSScriptRoot "pack-dist.ps1")

$Exe = Join-Path $Root "dist\buraaq.exe"
$Stdlib = Join-Path $Root "stdlib"
if (-not (Test-Path (Join-Path $Stdlib "src\io.bq"))) {
    throw "stdlib missing under $Stdlib"
}

$Stage = Join-Path $Root "dist\winget-stage"
$Name = "buraaq-$Version-windows-x64"
$StagePkg = Join-Path $Stage $Name
if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Force -Path $StagePkg | Out-Null

Copy-Item -Force $Exe (Join-Path $StagePkg "buraaq.exe")

$sys = Join-Path $StagePkg "sysroot"
New-Item -ItemType Directory -Force -Path $sys | Out-Null
foreach ($part in @("src", "runtime")) {
    Copy-Item -Recurse (Join-Path $Stdlib $part) (Join-Path $sys $part)
}
$pkgFile = Join-Path $Stdlib "buraaq.pkg"
if (Test-Path $pkgFile) { Copy-Item -Force $pkgFile (Join-Path $sys "buraaq.pkg") }
$rt = Join-Path $Root "compiler\runtime\buraaq_rt.c"
if (Test-Path $rt) { Copy-Item -Force $rt (Join-Path $sys "runtime\buraaq_rt.c") }

$Zip = Join-Path $Root "dist\$Name.zip"
if (Test-Path $Zip) { Remove-Item -Force $Zip }
# Zip root = buraaq.exe + sysroot/ (winget portable RelativeFilePath: buraaq.exe)
Compress-Archive -Path (Join-Path $StagePkg '*') -DestinationPath $Zip -CompressionLevel Optimal

$hash = (Get-FileHash -Algorithm SHA256 $Zip).Hash.ToLowerInvariant()
$notes = Join-Path $Root "dist\winget-$Version.sha256.txt"
@"
File: $Name.zip
SHA256: $hash
Upload to: https://github.com/ThoriumDC/buraaq/releases/download/v$Version/$Name.zip
Then set InstallerSha256 in packaging/winget/$Version/ThoriumDC.Buraaq.installer.yaml
Submit PR to https://github.com/microsoft/winget-pkgs
"@ | Set-Content -Path $notes -Encoding utf8

Write-Host "Packed $Zip"
Write-Host "SHA256 $hash"
Write-Host "Notes  $notes"

$installerYaml = Join-Path $Root "packaging\winget\$Version\ThoriumDC.Buraaq.installer.yaml"
if (Test-Path $installerYaml) {
    $y = Get-Content $installerYaml -Raw
    $y = $y -replace 'InstallerSha256:\s*\S+', "InstallerSha256: $hash"
    $y = $y -replace 'InstallerUrl:\s*\S+', "InstallerUrl: https://github.com/ThoriumDC/buraaq/releases/download/v$Version/$Name.zip"
    Set-Content -Path $installerYaml -Value $y.TrimEnd() -Encoding utf8
    Write-Host "Updated $installerYaml"
}

Write-Host "Users (after winget-pkgs merge): winget install buraaq"
