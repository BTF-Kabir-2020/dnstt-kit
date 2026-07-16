# Build release artifacts into dist/<triple>/
# Native host + optional cross targets.
# Usage:
#   .\scripts\build-release.ps1
#   .\scripts\build-release.ps1 -Cross
#   .\scripts\build-release.ps1 -Lib

param(
    [switch]$Cross,
    [switch]$Lib,
    [switch]$SkipTest
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$env:CARGO_TARGET_DIR = Join-Path $Root "target"
Set-Location $Root

function Triple {
    if ($IsWindows -or $env:OS -match "Windows") { return "windows-x86_64" }
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLower()
    if ($arch -match "arm") { return "linux-aarch64" }
    return "linux-x86_64"
}

$t = Triple
Write-Host "=== native release ($t) ==="
if (-not $SkipTest) {
    cargo test -p scanner_core -p dns-cli -- --test-threads=2
}
cargo build -p dns-cli --release
if ($Lib) {
    cargo build -p scanner_core --release
}

$dist = Join-Path $Root "dist\$t"
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$exe = Join-Path $Root "target\release\dns-cli.exe"
if (-not (Test-Path $exe)) { $exe = Join-Path $Root "target\release\dns-cli" }
Copy-Item $exe $dist -Force
if ($Lib) {
    $dll = Join-Path $Root "target\release\scanner_core.dll"
    $so = Join-Path $Root "target\release\libscanner_core.so"
    $dylib = Join-Path $Root "target\release\libscanner_core.dylib"
    foreach ($f in @($dll, $so, $dylib)) {
        if (Test-Path $f) { Copy-Item $f $dist -Force }
    }
}
Write-Host "→ $dist"

if ($Cross) {
    $targets = @(
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu"
    )
    foreach ($tgt in $targets) {
        Write-Host "=== cross $tgt ==="
        rustup target add $tgt 2>$null
        try {
            cargo build -p dns-cli --release --target $tgt
            $outDir = Join-Path $Root "target\$tgt\release"
            $name = if ($tgt -match "windows") { "dns-cli.exe" } else { "dns-cli" }
            $src = Join-Path $outDir $name
            if (Test-Path $src) {
                $d = Join-Path $Root "dist\$tgt"
                New-Item -ItemType Directory -Force -Path $d | Out-Null
                Copy-Item $src $d -Force
                Write-Host "→ $d"
            }
        } catch {
            Write-Host "⚠️  skip $tgt : $_"
        }
    }
}

Write-Host "BUILD OK"
Get-ChildItem -Recurse (Join-Path $Root "dist") | Select-Object FullName, Length
