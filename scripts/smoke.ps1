# Smoke tests (Windows)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$env:CARGO_TARGET_DIR = Join-Path $Root "target"
Set-Location $Root

cargo test -p scanner_core
cargo test -p dns-cli
cargo build -p dns-cli

$bin = Join-Path $Root "target\debug\dns-cli.exe"
& $bin init
& $bin info
& $bin slipnet which
& $bin slipnet probe
& $bin scan testdata\dns_sample.txt --preset low --limit 3 --quiet --run-id smoke_scan
& $bin pipeline run --input testdata\dns_sample.txt --profile mame --preset low --skip-slipnet --limit 3 --auto-backup --run-id smoke_pipe
& $bin backup create --mode kit --keep 10 --label smoke
& $bin backup list
& $bin doctor

Write-Host "SMOKE OK"
