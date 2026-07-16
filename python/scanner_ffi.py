#!/usr/bin/env python3
"""ctypes wrapper around scanner_core shared library (DLL/SO).

Build first:
  cargo build -p scanner_core --release
  # Windows: target/release/scanner_core.dll
  # Linux:   target/release/libscanner_core.so

Usage:
  python python/scanner_ffi.py testdata/dns_sample.txt
"""
from __future__ import annotations

import ctypes
import json
import platform
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def lib_path() -> Path:
    rel = ROOT / "target" / "release"
    dist = ROOT / "dist"
    sysname = platform.system().lower()
    if sysname.startswith("win"):
        names = ["scanner_core.dll"]
        triples = ["windows-x86_64"]
    elif sysname == "darwin":
        names = ["libscanner_core.dylib"]
        triples = ["darwin-aarch64", "darwin-x86_64"]
    else:
        names = ["libscanner_core.so"]
        triples = ["linux-x86_64", "linux-aarch64"]
    for n in names:
        p = rel / n
        if p.is_file():
            return p
        for t in triples:
            p2 = dist / t / n
            if p2.is_file():
                return p2
    raise FileNotFoundError(
        "scanner_core shared lib not found. Run:\n"
        "  cargo build -p scanner_core --release\n"
        "  or .\\scripts\\build-release.ps1 -Lib"
    )


def scan_file(input_path: str | Path) -> dict:
    lib = ctypes.CDLL(str(lib_path()))
    lib.scanner_run_from_file.argtypes = [ctypes.c_char_p]
    lib.scanner_run_from_file.restype = ctypes.c_void_p
    lib.scanner_free_string.argtypes = [ctypes.c_void_p]
    lib.scanner_free_string.restype = None

    path_b = str(Path(input_path).resolve()).encode("utf-8")
    ptr = lib.scanner_run_from_file(path_b)
    if not ptr:
        raise RuntimeError("scanner_run_from_file returned null")
    try:
        raw = ctypes.cast(ptr, ctypes.c_char_p).value
        if raw is None:
            raise RuntimeError("null C string")
        return json.loads(raw.decode("utf-8"))
    finally:
        lib.scanner_free_string(ptr)


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: scanner_ffi.py <ips.txt>", file=sys.stderr)
        return 2
    data = scan_file(sys.argv[1])
    print(json.dumps(data, indent=2, ensure_ascii=False)[:4000])
    print("...")
    print("keys:", list(data.keys()) if isinstance(data, dict) else type(data))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
