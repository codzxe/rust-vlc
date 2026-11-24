#!/usr/bin/env python3
"""Craft a FLAC-in-Ogg (.oga) per the xiph mapping: BOS page with the
9-byte mapping header, second header packet = 4-byte METADATA_BLOCK_HEADER
+ 34-byte STREAMINFO (38 bytes total), then an audio page carrying a
FLAC frame. STREAMINFO: 48 kHz, 2 ch, 16-bit.
"""
import struct

def ogg_crc(data: bytes) -> int:
    crc = 0
    for b in data:
        crc ^= b << 24
        for _ in range(8):
            if crc & 0x80000000:
                crc = ((crc << 1) ^ 0x04C11DB7) & 0xFFFFFFFF
            else:
                crc = (crc << 1) & 0xFFFFFFFF
    return crc

assert ogg_crc(b"123456789") == 0x89A1897F, hex(ogg_crc(b"123456789"))

def page(header_type: int, granule: int, serial: int, seq: int, packets: list[bytes]) -> bytes:
    body = b"".join(packets)
    lacings = []
    for p in packets:
        n = len(p)
        while n >= 255:
            lacings.append(255)
            n -= 255
        lacings.append(n)  # < 255 terminates the packet
    seg_count = len(lacings)
    assert seg_count <= 255
    out = bytearray()
    out += b"OggS"
    out.append(0)                     # version
    out.append(header_type)
    out += struct.pack("<q", granule)
    out += struct.pack("<I", serial)
    out += struct.pack("<I", seq)
    out += b"\x00\x00\x00\x00"       # checksum placeholder
    out.append(seg_count)
    out += bytes(lacings)
    out += body
    crc = ogg_crc(bytes(out))
    out[22:26] = struct.pack("<I", crc)
    return bytes(out)

def streaminfo(rate=48000, ch=2, bits=16, total=4096,
               min_bs=4096, max_bs=4096):
    si = bytearray(34)
    si[0:2] = struct.pack(">H", min_bs)
    si[2:4] = struct.pack(">H", max_bs)
    # min/max frame size 3-byte each (0 = unknown)
    si[4:7] = b"\x00\x00\x00"
    si[7:10] = b"\x00\x00\x00"
    packed = (rate << 44) | ((ch - 1) << 41) | ((bits - 1) << 36) | total
    si[10:18] = struct.pack(">Q", packed)
    # md5 zero
    return bytes(si)

def build(path):
    serial = 0x1A2B3C4D
    # 1. mapping header: 0x7F "FLAC" 0x01 0x02 <BE16 header-packet count>
    mapping = b"\x7fFLAC" + bytes([0x01, 0x02]) + struct.pack(">H", 2)
    # 2. second header packet: metadata block header + STREAMINFO
    si = streaminfo()
    meta_header = bytes([0x80, 0x00, 0x00, len(si)])  # last + type 0, len 34
    second = meta_header + si
    # 3. audio: a frame-shaped packet (bytes lumen/ffmpeg will hand the decoder)
    frame = bytes([0xFF, 0xF8, 0x10, 0x00, 0x00, 0x00]) + b"\x00" * 20
    pages = [
        page(0x02, 0, serial, 0, [mapping]),            # BOS
        page(0x00, 0, serial, 1, [second]),              # STREAMINFO
        page(0x00, 4096, serial, 2, [frame]),            # audio
        page(0x04, 4096, serial, 3, []),                 # EOS (empty)
    ]
    data = b"".join(pages)
    open(path, "wb").write(data)
    print(path, len(data), "bytes")

build("flac_in_ogg.oga")
