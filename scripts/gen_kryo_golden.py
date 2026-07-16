#!/usr/bin/env python3
"""Generate Kryo golden bytes for one fixed bean — compare with Rust later.

Requires no third-party packages. Mirrors dnstt_ssh_share_link.py encoding.
"""
from __future__ import annotations

import base64
import json
import zlib
from io import BytesIO
from pathlib import Path


class KryoOutput:
    def __init__(self) -> None:
        self._buf = BytesIO()

    def getvalue(self) -> bytes:
        return self._buf.getvalue()

    def write_int(self, value: int) -> None:
        v = value & 0xFFFFFFFF
        self._buf.write(bytes([v & 0xFF, (v >> 8) & 0xFF, (v >> 16) & 0xFF, (v >> 24) & 0xFF]))

    def write_var_int_flag(self, flag: bool, value: int, optimize_positive: bool) -> None:
        if not optimize_positive:
            value = ((value << 1) ^ (value >> 31)) & 0xFFFFFFFF
        first = (value & 0x3F) | (0x80 if flag else 0)
        if value >> 6 == 0:
            self._buf.write(bytes([first & 0xFF]))
            return
        if value >> 13 == 0:
            self._buf.write(bytes([(first | 0x40) & 0xFF, (value >> 6) & 0xFF]))
            return
        if value >> 20 == 0:
            self._buf.write(
                bytes([(first | 0x40) & 0xFF, ((value >> 6) | 0x80) & 0xFF, (value >> 13) & 0xFF])
            )
            return
        if value >> 27 == 0:
            self._buf.write(
                bytes(
                    [
                        (first | 0x40) & 0xFF,
                        ((value >> 6) | 0x80) & 0xFF,
                        ((value >> 13) | 0x80) & 0xFF,
                        (value >> 20) & 0xFF,
                    ]
                )
            )
            return
        self._buf.write(
            bytes(
                [
                    (first | 0x40) & 0xFF,
                    ((value >> 6) | 0x80) & 0xFF,
                    ((value >> 13) | 0x80) & 0xFF,
                    ((value >> 20) | 0x80) & 0xFF,
                    (value >> 27) & 0xFF,
                ]
            )
        )

    def write_string(self, value: str | None) -> None:
        if value is None:
            self._buf.write(b"\x80")
            return
        char_count = len(value)
        if char_count == 0:
            self._buf.write(bytes([1 | 0x80]))
            return
        if 1 < char_count <= 32:
            try:
                value.encode("ascii")
            except UnicodeEncodeError:
                pass
            else:
                data = bytearray(value.encode("latin-1"))
                data[-1] |= 0x80
                self._buf.write(bytes(data))
                return
        self.write_var_int_flag(True, char_count + 1, True)
        for c in value:
            o = ord(c)
            if o <= 0x7F:
                self._buf.write(bytes([o]))
            elif o > 0x07FF:
                self._buf.write(
                    bytes([0xE0 | ((o >> 12) & 0x0F), 0x80 | ((o >> 6) & 0x3F), 0x80 | (o & 0x3F)])
                )
            else:
                self._buf.write(bytes([0xC0 | ((o >> 6) & 0x1F), 0x80 | (o & 0x3F)]))


def serialize() -> bytes:
    out = KryoOutput()
    out.write_int(3)
    out.write_string("127.0.0.1")
    out.write_int(53)
    out.write_string("8.8.8.8:53")
    out.write_string("example.com")
    out.write_string("aa")
    out.write_int(0)
    out.write_string("")
    out.write_string("")
    out.write_string("")
    out.write_int(0)
    out.write_string("")
    out.write_int(22)
    out.write_string("")
    out.write_string("")
    out.write_int(1)
    out.write_string("t")
    out.write_string("")
    out.write_string("")
    return out.getvalue()


def main() -> None:
    raw = serialize()
    compressed = zlib.compress(raw, level=9)
    link = "sn://dnstt?" + base64.urlsafe_b64encode(compressed).decode("ascii").rstrip("=")
    out_dir = Path(__file__).resolve().parents[1] / "testdata"
    (out_dir / "kryo_golden_raw.hex").write_text(raw.hex() + "\n", encoding="utf-8")
    (out_dir / "kryo_golden_link.txt").write_text(link + "\n", encoding="utf-8")
    meta = {"raw_len": len(raw), "link_prefix": link[:40]}
    (out_dir / "kryo_golden_meta.json").write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
    print("wrote", out_dir / "kryo_golden_raw.hex")
    print(link[:80] + "...")


if __name__ == "__main__":
    main()
