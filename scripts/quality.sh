#!/usr/bin/env bash
# دروازه کیفیت محلی — همان چک‌های CI (fmt + clippy + test)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
cd "$ROOT"

echo "=== rustfmt --check ==="
cargo fmt --all -- --check

echo "=== clippy -D warnings ==="
cargo clippy --workspace --all-targets -- -D warnings

echo "=== test ==="
cargo test --workspace -- --test-threads=2

echo "QUALITY OK"
