#!/usr/bin/env python3
"""Craft a single-program MPEG-TS whose audio PES carries one ADTS
header with frame_length == 0 — the apps/cli/src/containers/ts.rs
mint_adts_aus livelock/OOM lane (end = at, at = end, forever).
Ported from the demuxer's own test_support builders + the CLI ts.rs
test fixtures (synthetic_ts).
"""
import struct

def crc32_mpeg(data: bytes) -> int:
    crc = 0xFFFFFFFF
    for b in data:
        crc ^= b << 24
        for _ in range(8):
            crc = ((crc << 1) ^ 0x04C11DB7) & 0xFFFFFFFF if crc & 0x80000000 else (crc << 1) & 0xFFFFFFFF
    return crc

def be16(v): return struct.pack(">H", v)

def psi_section(table_id, tsid, version, body):
    section_length = 9 + len(body)
    out = bytearray()
    out.append(table_id)
    out.append(0xB0 | (section_length >> 8))
    out.append(section_length & 0xFF)
    out += be16(tsid)
    out.append(0xC0 | ((version << 1) & 0x3E) | 1)  # current
    out.append(0); out.append(0)
    out += body
    out += struct.pack(">I", crc32_mpeg(bytes(out)))
    return bytes(out)

def pat_body(entries):
    body = bytearray()
    for number, pid in entries:
        body += be16(number)
        body += be16((pid & 0x1FFF) | 0xE000)
    return bytes(body)

def pmt_body(pcr_pid, program_info, streams):
    body = bytearray()
    body += be16((pcr_pid & 0x1FFF) | 0xE000)
    body += be16((len(program_info) & 0x0FFF) | 0xF000)
    body += program_info
    for st, pid, es in streams:
        body.append(st)
        body += be16((pid & 0x1FFF) | 0xE000)
        body += be16((len(es) & 0x0FFF) | 0xF000)
        body += es
    return bytes(body)

def sdt_body(onid, services):
    body = bytearray()
    body += be16(onid)
    body.append(0xFF)
    for sid, provider, name in services:
        body += be16(sid)
        body.append(0xFC)
        d = bytearray([0x48, 0, 0x01, len(provider)])
        d += provider.encode()
        d.append(len(name))
        d += name.encode()
        d[1] = len(d) - 2
        body += be16((len(d) & 0x0FFF) | 0xF000)
        body += d
    return bytes(body)

def ts_packet(pid, pusi, cc, pcr, payload):
    af = bytearray()
    if pcr is not None:
        base, ext = pcr
        af.append(0x10)
        af.append((base >> 25) & 0xFF)
        af.append((base >> 17) & 0xFF)
        af.append((base >> 9) & 0xFF)
        af.append((base >> 1) & 0xFF)
        af.append((((base & 1) << 7) | ((ext >> 8) & 1) | 0x7E) & 0xFF)
        af.append(ext & 0xFF)
    out = bytearray(188)
    out[0] = 0x47
    out[1] = ((1 if pusi else 0) << 6) | ((pid >> 8) & 0x1F)
    out[2] = pid & 0xFF
    afc = 0b01 if not af else 0b11
    out[3] = (afc << 4) | (cc & 0x0F)
    at = 4
    if af:
        af.insert(0, len(af))
        out[4:4+len(af)] = af
        at = 4 + len(af)
    take = min(len(payload), 188 - at)
    out[at:at+take] = payload[:take]
    return bytes(out)

def psi_packets(pid, cc_start, sections):
    out = []
    cc = cc_start
    for s in sections:
        payload = b"\x00" + s
        for i, chunk in enumerate(payload[i:i+184] for i in range(0, len(payload), 184)):
            out.append(ts_packet(pid, i == 0, cc, None, chunk))
            cc = (cc + 1) & 0x0F
    return out

def encode_ts5(v, marker):
    return bytes([
        ((marker & 0x0F) << 4) | (((v >> 30) & 0x07) << 1) | 1,
        (v >> 22) & 0xFF,
        (((v >> 15) & 0x7F) << 1) | 1,
        (v >> 7) & 0xFF,
        ((v & 0x7F) << 1) | 1,
    ])

def pes_bytes(stream_id, pts, payload):
    after = 8 + len(payload)
    out = bytearray([0x00, 0x00, 0x01, stream_id])
    out += be16(after)
    out.append(0x80); out.append(0x80); out.append(5)
    out += encode_ts5(pts, 0b0010)
    out += payload
    return bytes(out)

def adts_frame_len0():
    # MPEG-4 LC 44.1kHz stereo, protection absent, frame_length == 0:
    # head[3]&0x03=0, head[4]=0, head[5]>>5=0.
    f = bytearray(b"\xBB" * 15)
    f[0] = 0xFF
    f[1] = 0xF1
    f[2] = (1 << 6) | (4 << 2)         # profile LC, sf_idx 4 (44100)
    f[3] = 0x00                          # channel_config bits + length hi = 0
    f[4] = 0x00                          # length mid = 0
    f[5] = 0x1F                          # length lo (3 bits) = 0, VBR fullness
    f[6] = 0xFC                           # fullness lo + 1 raw block
    return bytes(f)

def main():
    pat = psi_section(0x00, 1, 0, pat_body([(1, 0x1000)]))
    pmt = psi_section(0x02, 1, 0, pmt_body(0x100, b"", [(0x1B, 0x100, b""), (0x0F, 0x101, b"")]))
    sdt = psi_section(0x42, 1, 0, sdt_body(1, [(1, "FFmpeg", "Service01")]))
    packets = []
    packets += psi_packets(0x0011, 0, [sdt])
    packets += psi_packets(0x0000, 0, [pat])
    packets += psi_packets(0x1000, 0, [pmt])
    packets.append(ts_packet(0x100, True, 0, None, pes_bytes(0xE0, 126_000, b"\xAA" * 170)))
    packets.append(ts_packet(0x101, True, 0, None, pes_bytes(0xC0, 128_000, adts_frame_len0())))
    out = b"".join(packets)
    open("ts_adts_zero.ts", "wb").write(out)
    print("ts_adts_zero.ts:", len(out), "bytes")

main()
