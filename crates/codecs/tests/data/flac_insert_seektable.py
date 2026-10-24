#!/usr/bin/env python3
"""Insert a SEEKTABLE metadata block into an ffmpeg-authored FLAC (5.A1).

ffmpeg 7.1.5's flac muxer writes no SEEKTABLE (blocks: STREAMINFO,
VORBIS_COMMENT, PADDING). This script inserts one computed by scanning
frame headers with the spec sec.6 rules -- number first, then the
block-size/sample-rate escapes, then CRC-8 (poly 0x07, init 0) -- the
same rules crates/codecs/src/audio/flac/seek.rs implements. The
PADDING block is dropped; the SEEKTABLE becomes the last metadata
block. Points: every 8th frame, always frame 0.
"""
import struct
import sys


def crc8(data):
    crc = 0
    for b in data:
        crc ^= b
        for _ in range(8):
            crc = ((crc << 1) ^ 0x07) & 0xFF if crc & 0x80 else (crc << 1) & 0xFF
    return crc


def parse_header(data, off):
    if off + 4 > len(data) or data[off] != 0xFF:
        return None
    if (data[off + 1] & 0b11111100) != 0b11111000:
        return None
    variable = (data[off + 1] & 1) == 1
    bs_code = data[off + 2] >> 4
    sr_code = data[off + 2] & 0xF
    if bs_code == 0 or sr_code == 0b1111:
        return None
    b3 = data[off + 3]
    if (b3 >> 4) > 0b1010 or ((b3 >> 1) & 7) in (0b011, 0b111) or (b3 & 1):
        return None
    cur = 4
    first = data[off + cur]
    lo = 0
    x = first
    while x & 0x80:
        lo += 1
        x = (x << 1) & 0xFF
    if lo == 1 or lo > 6:
        return None
    add = max(lo - 1, 0)
    if add >= 1 and (first & (0x7F >> add)) == 0 and (first & 0x80):
        return None
    number = (first & (0x7F >> add)) << (6 * add)
    for i in range(add):
        b = data[off + cur + 1 + i]
        if (b & 0b11000000) != 0b10000000:
            return None
        number |= (b & 0x3F) << (6 * (add - 1 - i))
    cur += 1 + add
    if not variable and number > 0x7FFFFFFF:
        return None
    if bs_code == 0b0001:
        bs = 192
    elif bs_code == 0b0110:
        bs = data[off + cur] + 1
        cur += 1
    elif bs_code == 0b0111:
        raw = (data[off + cur] << 8) | data[off + cur + 1]
        if raw == 0xFFFF:
            return None
        bs = raw + 1
        cur += 2
    elif bs_code < 0b0110:
        bs = 576 << (bs_code - 2)
    else:
        bs = 256 << (bs_code - 8)
    if sr_code == 0b1100:
        cur += 1
    elif sr_code in (0b1101, 0b1110):
        cur += 2
    if crc8(data[off:off + cur]) != data[off + cur]:
        return None
    return {"number": number, "variable": variable, "bs": bs, "hlen": cur + 1}


def main(src, dst):
    data = open(src, "rb").read()
    assert data[:4] == b"fLaC"
    pos = 4
    blocks = []
    while True:
        last = (data[pos] & 0x80) != 0
        btype = data[pos] & 0x7F
        blen = int.from_bytes(data[pos + 1:pos + 4], "big")
        blocks.append((btype, data[pos + 4:pos + 4 + blen]))
        pos += 4 + blen
        if last:
            break
    frames_start = pos
    si = blocks[0][1]
    fixed_bs = int.from_bytes(si[2:4], "big")
    assert int.from_bytes(si[0:2], "big") == fixed_bs, "not a fixed-blocksize stream"
    frames = []
    scan = frames_start
    while scan < len(data):
        h = parse_header(data, scan)
        if h is not None:
            frames.append((scan, h))
        scan += 1
    points = []
    step = 8
    for idx, (off, h) in enumerate(frames):
        if idx % step != 0:
            continue
        first_sample = h["number"] if h["variable"] else h["number"] * fixed_bs
        points.append((first_sample, off - frames_start))
    table = bytearray()
    for i, (sample, off) in enumerate(points):
        end_frame = min(i + step, len(frames))
        table += struct.pack(">QQH", sample, off, end_frame - i)
    out = bytearray(b"fLaC")
    for btype, body in blocks:
        if btype == 1:  # drop PADDING
            continue
        last = False
        out += bytes([0x80 if last else 0x00 | btype]) + len(body).to_bytes(3, "big") + body
    out += bytes([0x80 | 3]) + len(table).to_bytes(3, "big") + table
    out += data[frames_start:]
    open(dst, "wb").write(out)
    print(f"frames={len(frames)} points={len(points)} "
          f"seektable_bytes={len(table)} out_bytes={len(out)}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
