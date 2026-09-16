# Record real elapsed fuzz time. Never invent a 7-day result.
# Usage: .\fuzz_duration.ps1 [-Seconds 60]
param(
    [int]$Seconds = 60
)

$ErrorActionPreference = "Stop"
$Compiler = Split-Path $PSScriptRoot -Parent
$OutDir = Join-Path $Compiler "fuzz-evidence"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$started = Get-Date
$deadline = $started.AddSeconds($Seconds)
$iters = 0
Push-Location $Compiler
try {
    while ((Get-Date) -lt $deadline) {
        cargo test --workspace fuzz_ -- --nocapture
        if ($LASTEXITCODE -ne 0) { throw "fuzz batch failed after $iters iterations" }
        $iters++
    }
} finally {
    Pop-Location
}
$ended = Get-Date
$elapsed = ($ended - $started).TotalSeconds
$report = @"
BURAAQ FUZZ DURATION EVIDENCE
started_utc=$($started.ToUniversalTime().ToString("o"))
ended_utc=$($ended.ToUniversalTime().ToString("o"))
elapsed_sec=$([math]::Round($elapsed, 3))
requested_sec=$Seconds
iterations=$iters
seven_days_elapsed=$($elapsed -ge (7 * 24 * 3600))
"@
$path = Join-Path $OutDir ("elapsed-{0}.txt" -f $started.ToString("yyyyMMdd-HHmmss"))
Set-Content -Encoding utf8 $path $report
Write-Host $report
Write-Host "wrote $path"
if ($elapsed -lt (7 * 24 * 3600)) {
    Write-Host "Gate D remains FAIL: 7 days have not elapsed."
}
