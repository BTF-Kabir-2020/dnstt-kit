# Nightly/manual source+data backup wrapper for Task Scheduler / cron.
# Example (Windows Task Scheduler every 6h):
#   powershell -File ...\scripts\backup-scheduled.ps1

param(
    [ValidateSet("kit","data","full")]
    [string]$Mode = "kit",
    [int]$Keep = 20,
    [switch]$IncludeSecrets,
    [switch]$IncludeRuns
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$env:CARGO_TARGET_DIR = Join-Path $Root "target"
Set-Location $Root

$bin = Join-Path $Root "target\release\dns-cli.exe"
if (-not (Test-Path $bin)) { $bin = Join-Path $Root "target\debug\dns-cli.exe" }
if (-not (Test-Path $bin)) {
    cargo build -p dns-cli --release
    $bin = Join-Path $Root "target\release\dns-cli.exe"
}

$args = @("backup","create","--mode",$Mode,"--keep",$Keep,"--label","scheduled")
if ($IncludeSecrets) { $args += "--include-secrets" }
if ($IncludeRuns) { $args += "--include-runs" }

& $bin @args
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "scheduled backup OK"
