#!/usr/bin/env python3
"""gen-icon.py — deterministic placeholder icon PNG (card 7.w22g).

product.toml [brand] points at assets/brand/icon.{ico,icns,png-512}, but
those brand assets do not exist yet in this repository (config-validator
existence checks are a [PROPOSAL] follow-up; see docs/RELEASE.md
"known limits"). Until they land, Linux packaging surfaces (.deb hicolor
icon, AppImage .DirIcon) need SOME square raster, and the honest choice
is a generated placeholder — not a borrowed third-party icon (clean-room
rule: no VLC assets) and not a silently omitted icon (AppImage desktop
integration degrades without one).

What this generates: a flat 512x512 RGBA PNG filled with brand
accent_color from product.toml. Deterministic: no timestamps, no
compression variance beyond zlib level 9 on fixed scanlines — same input
color, same bytes, every run (verified locally; see worklog 7.w22g).

Stdlib only (zlib + struct): no Pillow dependency, no new Rust deps,
runs on every CI image with Python >= 3.6.

Usage:
  gen-icon.py <output.png> <#rrggbb>
"""

import struct
import sys
import zlib


def die(msg: str) -> None:
    print(f"gen-icon: error: {msg}", file=sys.stderr)
    raise SystemExit(1)


def parse_color(spec: str) -> tuple[int, int, int]:
    s = spec.strip()
    if not (len(s) == 7 and s[0] == "#"):
        die(f"color {spec!r} is not #rrggbb (brand.accent_color format)")
    try:
        return tuple(int(s[i : i + 2], 16) for i in (1, 3, 5))  # type: ignore[return-value]
    except ValueError:
        die(f"color {spec!r} has non-hex digits")


def chunk(tag: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + tag
        + data
        + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    )


def main() -> None:
    if len(sys.argv) != 3:
        die(f"usage: {sys.argv[0]} <output.png> <#rrggbb>")
    out_path, color_spec = sys.argv[1], sys.argv[2]
    r, g, b = parse_color(color_spec)
    size = 512

    # One scanline: filter byte 0 + `size` RGBA pixels.
    scanline = b"\x00" + bytes((r, g, b, 255)) * size
    raw = scanline * size

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)  # 8-bit RGBA
    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )
    with open(out_path, "wb") as fh:
        fh.write(png)
    print(f"gen-icon: wrote {out_path} ({size}x{size} RGBA, fill {color_spec}, "
          f"{len(png)} bytes)")


if __name__ == "__main__":
    main()
