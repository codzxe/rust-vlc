#!/usr/bin/env python3
"""Minimal EBML/Matroska writer + block inspector for W20-A01 finding 2/3.

Spec-convention under test: the lacing head byte (frames-1 per the
Matroska spec) vs literal frame count (the lumen reading).
"""
import struct, sys

def vint_size(n):
    """EBML size vint for value n: marker bit + value bits, big-endian."""
    for length in range(1, 9):
        limit = (1 << (7 * length)) - 1
        if n < limit:
            first = (1 << (8 - length)) | (n >> (8 * (length - 1)))
            if length == 1:
                return bytes([first])
            mask = (1 << (8 * (length - 1))) - 1
            return bytes([first]) + (n & mask).to_bytes(length - 1, "big")
    raise ValueError("too big")

def el(eid_bytes, payload):
    return eid_bytes + vint_size(len(payload)) + payload

def uint(v):
    if v == 0: return b"\x80"
    b = v.to_bytes((v.bit_length() + 7) // 8, "big")
    return bytes([0x80 | b[0]]) + b[1:] if False else b"\x80" + b  # simplest: vint with marker
def mkv_uint(v):
    # data-size handled by caller; here encode as big-endian bytes
    if v == 0: return b"\x00"
    return v.to_bytes((v.bit_length() + 7) // 8, "big")

def f32(x): return struct.pack(">f", x)

ID_EBML      = bytes.fromhex("1A45DFA3")
ID_SEGMENT   = bytes.fromhex("18538067")
ID_INFO      = bytes.fromhex("1549A966")
ID_TRACKS    = bytes.fromhex("1654AE6B")
ID_CLUSTER   = bytes.fromhex("1F43B675")
ID_TIMECODE  = bytes.fromhex("E7")
ID_SIMPLEBLK = bytes.fromhex("A3")
ID_TSENTRY   = bytes.fromhex("AE")
ID_TRACKNUM  = bytes.fromhex("D7")
ID_TRACKUID  = bytes.fromhex("73E5")
ID_TRACKTYPE = bytes.fromhex("83")
ID_CODECID   = bytes.fromhex("86")
ID_AUDIO     = bytes.fromhex("E1")
ID_SAMPLING  = bytes.fromhex("B5")
ID_CHANNELS  = bytes.fromhex("9F")
ID_BITDEPTH  = bytes.fromhex("6264")
ID_TSCALE    = bytes.fromhex("2AD7B1")
ID_MUXAPP    = bytes.fromhex("4D80")
ID_WRAPP     = bytes.fromhex("5741")
ID_DOCTYPE   = bytes.fromhex("4282")
ID_DTVER     = bytes.fromhex("4287")
ID_DTREAD    = bytes.fromhex("4285")
ID_EBMLVER   = bytes.fromhex("4286")
ID_EBMLRVER  = bytes.fromhex("42F7")
ID_MAXID     = bytes.fromhex("42F2")
ID_MAXSZ     = bytes.fromhex("42F3")

def ebml_header():
    payload = (el(ID_EBMLVER, mkv_uint(1)) + el(ID_EBMLRVER, mkv_uint(1))
               + el(ID_MAXID, mkv_uint(4)) + el(ID_MAXSZ, mkv_uint(8))
               + el(ID_DOCTYPE, b"matroska") + el(ID_DTVER, mkv_uint(4))
               + el(ID_DTREAD, mkv_uint(2)))
    return el(ID_EBML, payload)

def audio_track():
    audio = (el(ID_SAMPLING, f32(8000.0)) + el(ID_CHANNELS, mkv_uint(1))
             + el(ID_BITDEPTH, mkv_uint(16)))
    entry = (el(ID_TRACKNUM, mkv_uint(1)) + el(ID_TRACKUID, mkv_uint(1))
             + el(ID_TRACKTYPE, mkv_uint(2)) + el(ID_CODECID, b"A_PCM/INT/LIT")
             + el(ID_AUDIO, audio))
    return el(ID_TRACKS, el(ID_TSENTRY, entry))

def info():
    return el(ID_INFO, el(ID_TSCALE, mkv_uint(1000000))
              + el(ID_MUXAPP, b"a") + el(ID_WRAPP, b"a"))

def simple_block(track, rel_tc, flags, body):
    return el(ID_SIMPLEBLK, bytes([0x80 | track]) + struct.pack(">h", rel_tc)
             + bytes([flags]) + body)

def cluster(tc, blocks_payload):
    return el(ID_CLUSTER, el(ID_TIMECODE, mkv_uint(tc)) + blocks_payload)

UNKNOWN8 = bytes.fromhex("01FFFFFFFFFFFFFF")

def build_mkv(cluster_payload, seg_children_sizes_known=True):
    out = ebml_header()
    seg_children = info() + audio_track() + cluster_payload
    out += el(ID_SEGMENT, seg_children)
    return out

# ---------------- xiph lacing files ----------------
def xiph_spec_file(path):
    # SPEC reading: 3 frames -> head byte = 2 (frames-1), sizes for the
    # first two frames (100, 100), last frame is the remainder (60).
    f1 = b"\x11" * 100
    f2 = b"\x22" * 100
    f3 = b"\x33" * 60
    body = bytes([2, 100, 100]) + f1 + f2 + f3
    blk = simple_block(1, 0, 0x80 | 0x02, body)   # flags: keyframe + Xiph
    data = build_mkv(cluster(0, blk))
    open(path, "wb").write(data)
    return len(data)

def xiph_code_file(path):
    # LUMEN reading: 3 frames -> head byte = 3, sizes for first two
    # frames, remainder last (this is what cluster.rs's own test writes).
    f1 = b"\x11" * 100
    f2 = b"\x22" * 100
    f3 = b"\x33" * 60
    body = bytes([3, 100, 100]) + f1 + f2 + f3
    blk = simple_block(1, 0, 0x80 | 0x02, body)
    data = build_mkv(cluster(0, blk))
    open(path, "wb").write(data)
    return len(data)

def fixed_spec_file(path):
    # FIXED lacing, spec reading: 4 frames -> head byte = 3.
    body = bytes([3]) + b"\x44" * (4 * 40)
    blk = simple_block(1, 0, 0x80 | 0x04, body)   # 0b10<<1 = fixed
    data = build_mkv(cluster(0, blk))
    open(path, "wb").write(data)
    return len(data)

if __name__ == "__main__":
    cmd = sys.argv[1]
    if cmd == "xiph_spec":
        print("wrote", xiph_spec_file("mk_xiph_spec.mkv"), "bytes")
    elif cmd == "xiph_code":
        print("wrote", xiph_code_file("mk_xiph_code.mkv"), "bytes")
    elif cmd == "fixed_spec":
        print("wrote", fixed_spec_file("mk_fixed_spec.mkv"), "bytes")
