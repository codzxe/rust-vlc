#!/usr/bin/env python3
"""Craft a minimal FLAC-in-WAV (WAVEFORMATEXTENSIBLE + FLAC SubFormat GUID
+ 34-byte STREAMINFO + data) to test the engine-vs-CLI seekable divergence.
Layout per crates/demux/src/wav/format.rs: GUID at fmt[24..40], STREAMINFO at
fmt[40..74] (FLAC_WAV_BYTES = 74)."""
import struct, sys

def streaminfo(rate=48000, ch=2, bps=16, total=48000):
    out = bytearray()
    out += struct.pack(">HH", 4096, 4096)      # min/max blocksize
    out += (0).to_bytes(3, "big")              # min framesize
    out += (0).to_bytes(3, "big")              # max framesize
    packed = (rate << 44) | ((ch - 1) << 41) | ((bps - 1) << 36) | total
    out += packed.to_bytes(8, "big")           # rate/ch/bps/total packed
    out += b"\x00" * 16                        # MD5
    assert len(out) == 34, len(out)
    return bytes(out)

FLAC_GUID = bytes([0xAC,0xF1,0x00,0x00,0x00,0x00,0x10,0x00,0x80,0x00,0x00,0xAA,0x00,0x38,0x9B,0x71])

fmt = bytearray()
fmt += struct.pack("<HHIIHH", 0xFFFE, 2, 48000, 48000*4, 4, 16)   # [0..16]
fmt += struct.pack("<HHI", 56, 16, 0x3)                            # [16..24] cbSize, validBits, mask
fmt += FLAC_GUID                                                    # [24..40]
fmt += streaminfo()                                                 # [40..74]
assert len(fmt) == 74, len(fmt)

data = b"\x00" * 4096

def chunk(tag, payload):
    return tag + struct.pack("<I", len(payload)) + payload

out = b"RIFF" + struct.pack("<I", 4 + 8 + len(fmt) + 8 + len(data)) + b"WAVE"
out += chunk(b"fmt ", bytes(fmt))
out += chunk(b"data", data)
path = sys.argv[1] if len(sys.argv) > 1 else "flac-in-wav.wav"
with open(path, "wb") as f:
    f.write(out)
print(f"wrote {path}: {len(out)} bytes")
