#!/usr/bin/env python3
"""Craft a fragmented MP4 whose trun carries ONLY a data offset (no
per-sample fields) and whose tfhd sets no defaults, so every sample's
duration/size/flags come from mvex/trex — the lane where lumen's
+4-skewed trex read shows. Trex written per ISO 14496-12 8.8.4:

  body: [version+flags(4)] track_ID(4) default_sample_description_index(4)
        default_sample_duration(4) default_sample_size(4)
        default_sample_flags(4)   -> offsets 0/4/8/12/16/20
"""
import struct

def u32b(v): return struct.pack(">I", v)
def u64b(v): return struct.pack(">Q", v)

def boxb(kind, payload):
    return u32b(8 + len(payload)) + kind + payload

def fullbox(kind, version, payload):
    return boxb(kind, bytes([version, 0, 0, 0]) + payload)

def fullbox_f(kind, version, flags, payload):
    return boxb(kind, bytes([version, (flags >> 16) & 0xFF, (flags >> 8) & 0xFF, flags & 0xFF]) + payload)

def ftyp():
    body = b"isom" + u32b(0) + b"isom" + b"iso2" + b"mp41" + b"mp42"
    return boxb(b"ftyp", body)

def mvhd():
    b = u32b(0) + u32b(0) + u32b(1000) + u32b(0)
    b += u32b(0x00010000) + bytes([0, 1]) + bytes(10) + bytes(36) + bytes(24) + u32b(0xFFFFFFFF)
    return fullbox(b"mvhd", 0, b)

def tkhd(track_id):
    b = u32b(0) + u32b(0) + u32b(track_id) + u32b(0) + u32b(0)
    b += bytes(8) + bytes(4) + bytes([0, 0]) + bytes(2) + bytes(36) + u32b(0) + u32b(0)
    return boxb(b"tkhd", bytes([0, 0, 0, 3]) + b)

def mdhd():
    b = u32b(0) + u32b(0) + u32b(1000) + u32b(0) + u32b(0x55C4) + u32b(0)
    return fullbox(b"mdhd", 0, b)

def hdlr():
    return fullbox(b"hdlr", 0, u32b(0) + b"vide" + bytes(12) + bytes(1))

def stsd_entry(kind=b"jpeg"):
    b = bytes(28)
    return boxb(kind, b)

def stbl_empty():
    stsd_body = u32b(1) + stsd_entry()
    stbl = fullbox(b"stsd", 0, stsd_body)
    stbl += fullbox(b"stts", 0, u32b(0))
    stbl += fullbox(b"stsc", 0, u32b(0))
    stbl += fullbox(b"stsz", 0, bytes(8))
    stbl += fullbox(b"stco", 0, u32b(0))
    return boxb(b"stbl", stbl)

def trak():
    t = tkhd(1)
    mdia = mdhd() + hdlr() + boxb(b"minf", fullbox(b"vmhd", 0, bytes(8)) + stbl_empty())
    t += boxb(b"mdia", mdia)
    return boxb(b"trak", t)

def trex(duration, size, flags):
    body = u32b(1) + u32b(1) + u32b(duration) + u32b(size) + u32b(flags)
    return fullbox(b"trex", 0, body)

def build(duration, size, flags, path):
    moov = boxb(b"moov", mvhd() + trak() + boxb(b"mvex", trex(duration, size, flags)))
    mdat_payload = b"\x11" * 50 + b"\x22" * 50 + b"\x33" * 50 + b"\x44" * 50

    # two-pass: moof's trun data_offset depends on layout
    def assemble(data_offset):
        trun_body = u32b(4) + struct.pack(">i", data_offset)
        trun = fullbox_f(b"trun", 0, 0x1, trun_body)  # flags: data-offset only
        tfhd = fullbox_f(b"tfhd", 0, 0x020000, u32b(1))  # default-base-is-moof only
        tfdt = fullbox(b"tfdt", 0, u32b(0))
        traf = boxb(b"traf", tfhd + tfdt + trun)
        moof = boxb(b"moof", fullbox(b"mfhd", 0, u32b(1)) + traf)
        return ftyp() + moov + moof + boxb(b"mdat", mdat_payload)

    f = assemble(0)
    # find offsets: moof starts after ftyp+moov; mdat payload starts after its header
    ftyp_len = len(ftyp())
    moov_len = len(moov)
    moof_len = len(assemble(0)) - ftyp_len - moov_len - 8 - len(mdat_payload)
    moof_start = ftyp_len + moov_len
    mdat_payload_start = moof_start + moof_len + 8
    f = assemble(mdat_payload_start - moof_start)
    open(path, "wb").write(f)
    print(path, len(f), "bytes; moof@", moof_start, "mdat_payload@", mdat_payload_start)

build(100, 50, 100, "frag_trexA.mp4")            # variant A: flags=100 (small)
build(100, 50, 0x02000000, "frag_trexB.mp4")    # variant B: realistic flags 0x02000000
