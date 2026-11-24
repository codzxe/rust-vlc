#!/usr/bin/env python3
"""W16-RR3 batch metadata: identity, size, sha256, PNG dims + unique rows, media headers."""
import hashlib, os, struct, zlib, sys

ROOT = "/home/z/my-project/audit-workspace"
FILES = [
    "ui/e2e/visual/baselines/linux/settings-playback-1920x1080-2x-light.png",
    "ui/e2e/visual/baselines/linux/keyboard-shortcuts-1920x1080-1x-light.png",
    "ui/e2e/visual/baselines/linux/settings-network-1280x720-1x-dark.png",
    "ui/e2e/visual/baselines/linux/settings-network-1920x1080-1x-light.png",
    "ui/e2e/visual/baselines/linux/settings-recording-1280x720-1x-light.png",
    "ui/e2e/visual/baselines/linux/settings-network-1280x720-2x-light.png",
    "ui/e2e/visual/baselines/linux/convert-1280x720-2x-light.png",
    "ui/e2e/visual/baselines/linux/open-network-stream-1280x720-1x-dark.png",
    "ui/e2e/visual/baselines/linux/open-network-stream-1920x1080-2x-dark.png",
    "ui/e2e/visual/baselines/linux/onboarding-1280x720-1x-dark-blue.png",
    "ui/e2e/visual/baselines/linux/dev-tokens-1920x1080-1x-dark.png",
    "ui/e2e/visual/baselines/linux/onboarding-1920x1080-2x-dark.png",
    "fuzz/corpus/fuzz_demux_flac/seed_flac_s24.flac",
    "fuzz/corpus/fuzz_codec_mjpeg/seed_avi_mjpeg_422.avi",
    "tests/fixtures/hls/adts_vod/seg002.aac",
    "tests/fixtures/generated/malformed/mp4_h264_aac__flip4.mp4",
    "tests/fixtures/generated/malformed/mp4_frag__flip1.mp4",
    "tests/fixtures/generated/valid/mp4_3gp_h263.3gp",
    "tests/fixtures/generated/valid/mp4_mjpeg_pcm.mp4.oracle.sha256",
    "fuzz/corpus/fuzz_codec_mp2/seed_crossformat_mp3.mp3",
    "tests/fixtures/hls/adts_vod/pl.m3u8",
    "tests/fixtures/generated/malformed/ogg_vorbis_8k_st__trunc3",
    "fuzz/corpus/fuzz_http_headers/ranged_206.txt",
    "fuzz/corpus/fuzz_codec_alac/seed_cookie_badch.bin",
    "tests/fixtures/generated/malformed/webm_vp9_opus__trunc2.webm",
    "tests/fixtures/generated/malformed/ogg_vorbis_8k_st__trunc0",
]

def png_info(b):
    assert b[:8] == b"\x89PNG\r\n\x1a\n"
    w = h = None; bitdepth = ctyp = None; idat = b""; chunks = []
    i = 8
    while i + 8 <= len(b):
        ln, typ = struct.unpack(">I4s", b[i:i+8]); i += 8
        dat = b[i:i+ln]
        if typ == b"IHDR":
            w, h, bitdepth, ctyp = struct.unpack(">IIBB", dat[:10])
        elif typ == b"IDAT":
            idat += dat
        chunks.append(typ.decode("latin1"))
        i += ln + 4
    raw = zlib.decompress(idat)
    bpp = {0:1,2:3,3:1,4:2,6:4}[ctyp]
    stride = w * bpp + 1
    rows = {raw[r*stride:(r+1)*stride] for r in range(h)}
    return w, h, bitdepth, ctyp, len(raw), len(rows), "+".join(chunks[:8]) + ("…" if len(chunks) > 8 else "")

def hexdump(b, n=64):
    return " ".join(f"{x:02x}" for x in b[:n])

out = []
for rel in FILES:
    p = os.path.join(ROOT, rel)
    b = open(p, "rb").read()
    sha = hashlib.sha256(b).hexdigest()
    hdr = hexdump(b)
    line = f"== {rel}\n   size={len(b)} sha256={sha}\n   head={hdr}"
    if b[:8] == b"\x89PNG\r\n\x1a\n":
        w, h, bd, ct, rawlen, urows, chunks = png_info(b)
        line += f"\n   PNG IHDR: {w}x{h} bitdepth={bd} colortype={ct}; idat-decompressed={rawlen}; unique-scanlines={urows}/{h}; chunks={chunks}"
    out.append(line)
    print(line)

with open(os.path.join(ROOT, "audit/scratch/W16-RR3/batchmeta.txt"), "w") as f:
    f.write("\n".join(out) + "\n")
print("\nwrote audit/scratch/W16-RR3/batchmeta.txt")
