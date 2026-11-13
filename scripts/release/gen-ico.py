#!/usr/bin/env python3
"""gen-ico.py — deterministic placeholder icon ICO (wave-L round 4).

The tauri-build Windows resource step requires `icons/icon.ico` beside
the crate ("required for generating a Windows Resource file during
tauri-build", release dry-run 43406493996, windows lane) — tauri.conf's
bundle.icon PNG list alone does not satisfy it. The honest choice is the
same one gen-icon.py made for the Linux surfaces: a GENERATED
placeholder, not a borrowed third-party icon (clean-room rule) and not a
silently omitted file.

What this generates: a Vista-style ICO holding ONE PNG-compressed
256x256 RGBA entry filled with the same brand accent_color as
gen-icon.py's raster (256 is the standard PNG-entry size; the entry
width/height bytes encode 256 as 0 per the ICO spec). Deterministic: no
timestamps, zlib level 9 on fixed scanlines — same input color, same
bytes, every run.

Stdlib only (zlib + struct), like gen-icon.py. The ICO wrapper adds no
new dependencies to any lane.

Usage:
  gen-ico.py <output.ico> <#rrggbb>
"""

import struct
import sys
import zlib


def die(msg: str) -> None:
    print(f"gen-ico: error: {msg}", file=sys.stderr)
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


def png_rgba_square(size: int, rgb: tuple[int, int, int]) -> bytes:
    r, g, b = rgb
    scanline = b"\x00" + bytes((r, g, b, 255)) * size
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)  # 8-bit RGBA
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(scanline * size, 9))
        + chunk(b"IEND", b"")
    )


def main() -> None:
    if len(sys.argv) != 3:
        die(f"usage: {sys.argv[0]} <output.ico> <#rrggbb>")
    out_path, color_spec = sys.argv[1], sys.argv[2]
    rgb = parse_color(color_spec)
    size = 256  # the standard PNG-entry size; width/height byte 0 == 256

    png = png_rgba_square(size, rgb)
    # ICONDIR (6 bytes) + one ICONDIRENTRY (16 bytes) + the PNG image.
    # width/height 0 encodes 256; planes 1 + bpp 32 are the conventional
    # PNG-entry values; colorcount 0 (PNG entries carry no palette).
    ico = (
        struct.pack("<HHH", 0, 1, 1)  # reserved, type=icon, count=1
        + struct.pack(
            "<BBBBHHII",
            0,  # width 256 (byte 0)
            0,  # height 256 (byte 0)
            0,  # palette entries (PNG entry)
            0,  # reserved
            1,  # color planes
            32,  # bits per pixel
            len(png),
            6 + 16,  # image offset: after ICONDIR + ICONDIRENTRY
        )
        + png
    )
    with open(out_path, "wb") as fh:
        fh.write(ico)
    print(f"gen-ico: wrote {out_path} ({size}x{size} PNG-in-ICO, fill {color_spec}, "
          f"{len(ico)} bytes)")


if __name__ == "__main__":
    main()
