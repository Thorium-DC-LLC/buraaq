# Prefer the toolchain command (same as flash.ps1 historically).
param([string]$Board = "pico_w")
$ErrorActionPreference = "Stop"
if (-not (Get-Command buraaq -ErrorAction SilentlyContinue)) {
    Write-Error "buraaq not on PATH"
}
buraaq flash --board $Board
