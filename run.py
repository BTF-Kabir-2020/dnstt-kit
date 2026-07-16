#!/usr/bin/env python3
"""Thin launcher: pick native dns-cli for this OS/arch, else print build help.

Does NOT reimplement scanning — only selects the right binary beside this script.
"""
from __future__ import annotations

import os
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def triple() -> str:
    sysname = platform.system().lower()
    machine = platform.machine().lower()
    if sysname.startswith("win"):
        return "windows-x86_64"
    if "aarch64" in machine or machine == "arm64":
        return "linux-aarch64"
    return "linux-x86_64"


def find_bin() -> Path | None:
    t = triple()
    names = ["dns-cli.exe", "dns-cli"] if t.startswith("windows") else ["dns-cli"]
    candidates = [
        ROOT / "target" / "release" / names[0],
        ROOT / "dist" / t / names[0],
        ROOT / names[0],
    ]
    if len(names) > 1:
        candidates.append(ROOT / "target" / "release" / names[1])
    for c in candidates:
        if c.is_file():
            return c
    return None


def main() -> int:
    bin_path = find_bin()
    if not bin_path:
        print(
            "dns-cli binary not found. Build with:\n"
            "  cargo build --release -p dns-cli\n"
            f"Expected under target/release/ or dist/{triple()}/",
            file=sys.stderr,
        )
        return 1
    return subprocess.call([str(bin_path), *sys.argv[1:]], cwd=str(ROOT))


if __name__ == "__main__":
    raise SystemExit(main())
