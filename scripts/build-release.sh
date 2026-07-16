#!/usr/bin/env bash
# Build release into dist/<triple>/
# Usage: ./scripts/build-release.sh [--cross] [--lib] [--skip-test]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_TARGET_DIR="$ROOT/target"
cd "$ROOT"

CROSS=0
LIB=0
SKIP_TEST=0
for a in "$@"; do
  case "$a" in
    --cross) CROSS=1 ;;
    --lib) LIB=1 ;;
    --skip-test) SKIP_TEST=1 ;;
  esac
done

triple() {
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) echo linux-x86_64 ;;
    Linux-aarch64|Linux-arm64) echo linux-aarch64 ;;
    Darwin-arm64) echo darwin-aarch64 ;;
    Darwin-x86_64) echo darwin-x86_64 ;;
    MINGW*|MSYS*|CYGWIN*) echo windows-x86_64 ;;
    *) echo "unknown-$(uname -m)" ;;
  esac
}

T="$(triple)"
echo "=== native release ($T) ==="
if [[ "$SKIP_TEST" -eq 0 ]]; then
  cargo test -p scanner_core -p dns-cli -- --test-threads=2
fi
cargo build -p dns-cli --release
[[ "$LIB" -eq 1 ]] && cargo build -p scanner_core --release

DIST="$ROOT/dist/$T"
mkdir -p "$DIST"
cp -f "$ROOT/target/release/dns-cli" "$DIST/" 2>/dev/null || cp -f "$ROOT/target/release/dns-cli.exe" "$DIST/"
if [[ "$LIB" -eq 1 ]]; then
  for f in libscanner_core.so libscanner_core.dylib scanner_core.dll; do
    [[ -f "$ROOT/target/release/$f" ]] && cp -f "$ROOT/target/release/$f" "$DIST/"
  done
fi
echo "→ $DIST"

if [[ "$CROSS" -eq 1 ]]; then
  for tgt in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-pc-windows-gnu; do
    echo "=== cross $tgt ==="
    rustup target add "$tgt" || true
    if cargo build -p dns-cli --release --target "$tgt"; then
      name=dns-cli
      [[ "$tgt" == *windows* ]] && name=dns-cli.exe
      d="$ROOT/dist/$tgt"
      mkdir -p "$d"
      cp -f "$ROOT/target/$tgt/release/$name" "$d/" || true
      echo "→ $d"
    else
      echo "⚠️  skip $tgt"
    fi
  done
fi

echo "BUILD OK"
find "$ROOT/dist" -type f -printf '%p %s\n' 2>/dev/null || find "$ROOT/dist" -type f
