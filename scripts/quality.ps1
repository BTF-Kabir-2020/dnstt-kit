# دروازه کیفیت محلی — همان چک‌های CI (fmt + clippy + test)
# Usage: .\scripts\quality.ps1
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$env:CARGO_TARGET_DIR = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root "target" }
Set-Location $Root

Write-Host "=== rustfmt --check ==="
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) {
    Write-Host "Format failed. Fix with: cargo fmt --all"
    exit $LASTEXITCODE
}

Write-Host "=== clippy -D warnings ==="
cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "=== test ==="
cargo test --workspace -- --test-threads=2
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "QUALITY OK"
