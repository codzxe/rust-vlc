#!/usr/bin/env python3
"""W16-A10 batch metadata: sizes, sha256, magic, container structure, PNG dims,
malformed-vs-parent byte diffs, oracle hash checks. One script, all data files."""
import hashlib, struct, zlib

ROOT = "/home/z/my-project/audit-workspace"

FILES = [
 "tests/fixtures/generated/valid/mp4_mjpeg_pcm.mp4",
 "tests/fixtures/generated/malformed/avi_mjpeg_pcm__noidx0.avi",
 "fuzz/corpus/fuzz_demux_wav/seed_wav_s16le.wav",
 "tests/fixtures/generated/malformed/wav_pcm_s16le_44k1_st__flip4",
 "tests/fixtures/generated/malformed/aiff_pcm_s16be_44k1_st__trunc3",
 "tests/fixtures/hls/ts_live/s00.ts",
 "tests/fixtures/hls/ts_vod/seg002.ts",
 "tests/fixtures/hls/ts_live/s01.ts",
 "fuzz/corpus/fuzz_codec_hevc/seed_crossformat_mp4.mp4",
 "tests/fixtures/generated/valid/mp4_qt.mov",
 "tests/fixtures/generated/valid/mkv_h264_flac.mkv",
 "fuzz/corpus/fuzz_demux_mpd/ffmpeg_av_template.mpd",
 "fuzz/corpus/fuzz_hls_playlist/fmp4_pl.m3u8",
 "fuzz/corpus/fuzz_playlist_pls/seed_dirty.pls",
 "tests/fixtures/generated/malformed/ogg_vorbis_8k_st__garbage0",
 "fuzz/corpus/fuzz_playlist_m3u/seed_basic_extended.m3u",
 "fuzz/corpus/fuzz_demux_ivf/seed_ivf_vp9.ivf",
 "fuzz/corpus/fuzz_http_headers/odd_headers.txt",
 "fuzz/corpus/fuzz_codec_alac/seed_cookie_badver.bin",
 "fuzz/corpus/fuzz_codec_alac/seed_alac_stereo16.m4a",
 "fuzz/corpus/fuzz_demux_avi/seed_avi_trunc0.avi",
 "tests/fixtures/generated/malformed/avi_mjpeg_pcm__trunc0.avi",
 "tests/fixtures/generated/malformed/wav_pcm_s16le_44k1_st__trunc0",
 "tests/fixtures/generated/valid/wav_pcm_u8_44k1_mono.wav",
 "tests/fixtures/generated/valid/avi_mjpeg_pcm.avi.oracle.sha256",
 "tests/rebrand/assets/logo.svg",
] + [
 "ui/e2e/visual/baselines/linux/"+n for n in [
  "player-music-1920x1080-1x-dark.png","settings-playback-1920x1080-1x-light.png",
  "settings-privacy-1280x720-2x-dark.png","settings-privacy-1280x720-1x-dark.png",
  "settings-playback-1280x720-1x-light.png","settings-library-1920x1080-2x-light.png",
  "settings-audio-1280x720-1x-light.png","settings-video-1920x1080-2x-light.png",
  "settings-network-1280x720-2x-dark.png","settings-performance-1280x720-2x-light.png",
  "discovery-1280x720-1x-light.png","settings-extensions-1280x720-2x-dark.png",
  "settings-extensions-1280x720-1x-dark.png","open-network-stream-1920x1080-1x-light.png",
  "onboarding-1280x720-1x-dark.png","playlist-1920x1080-1x-dark.png",
  "not-found-1280x720-2x-light.png","miniplayer-1920x1080-2x-light.png",
 ]
]

PAIRS = [
 ("tests/fixtures/generated/malformed/wav_pcm_s16le_44k1_st__flip4","tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav"),
 ("tests/fixtures/generated/malformed/wav_pcm_s16le_44k1_st__trunc0","tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav"),
 ("tests/fixtures/generated/malformed/aiff_pcm_s16be_44k1_st__trunc3","tests/fixtures/generated/valid/aiff_pcm_s16be_44k1_st.aiff"),
 ("tests/fixtures/generated/malformed/ogg_vorbis_8k_st__garbage0","tests/fixtures/generated/valid/ogg_vorbis_8k_st.ogg"),
 ("tests/fixtures/generated/malformed/avi_mjpeg_pcm__trunc0.avi","tests/fixtures/generated/valid/avi_mjpeg_pcm.avi"),
 ("tests/fixtures/generated/malformed/avi_mjpeg_pcm__noidx0.avi","tests/fixtures/generated/valid/avi_mjpeg_pcm.avi"),
]

def sha256(path):
    h = hashlib.sha256()
    with open(path,"rb") as f:
        for chunk in iter(lambda: f.read(1<<20), b""):
            h.update(chunk)
    return h.hexdigest()

def png_info(data):
    out = {}
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        return {"error": "not a PNG"}
    pos = 8
    idat = b""
    while pos+12 <= len(data):
        ln = struct.unpack(">I", data[pos:pos+4])[0]
        typ = data[pos+4:pos+8]
        body = data[pos+8:pos+8+ln]
        if typ == b"IHDR":
            w,h,bd,ct,cm,fm,il = struct.unpack(">IIBBBBB", body)
            out.update(width=w,height=h,bitdepth=bd,colortype=ct,interlace=il)
        elif typ == b"IDAT":
            idat += body
        pos += 12 + ln
        if typ == b"IEND":
            break
    if idat:
        try:
            raw = zlib.decompress(idat)
            out["raw_bytes"] = len(raw)
            ct = out.get("colortype"); w = out.get("width")
            ch = {0:1,2:3,3:1,4:2,6:4}.get(ct,4)
            stride = 1 + w*ch
            rows = len(raw)//stride if stride else 0
            out["rows_decoded"] = rows
            s = set(raw[i*stride:(i+1)*stride] for i in range(min(rows, 2000)))
            out["unique_rows(first2000)"] = len(s)
        except Exception as e:
            out["zlib_error"] = str(e)
    return out

def riff_chunks(data):
    pos = 12; chunks = []
    while pos+8 <= len(data):
        cid = data[pos:pos+4].decode("latin1","replace")
        csz = struct.unpack("<I", data[pos+4:pos+8])[0]
        chunks.append((cid, csz))
        pos += 8 + csz + (csz & 1)
        if len(chunks) > 60: break
    return chunks

def wav_info(data):
    out = {}
    pos = 12
    while pos+8 <= len(data):
        cid = data[pos:pos+4].decode("latin1","replace")
        csz = struct.unpack("<I", data[pos+4:pos+8])[0]
        out.setdefault("chunks", []).append((cid, csz))
        if cid == "fmt " and csz >= 16:
            fmt, ch, rate, byterate, align, bits = struct.unpack("<HHIIHH", data[pos+8:pos+8+16])
            out["fmt"] = dict(tag=fmt, channels=ch, rate=rate, bits=bits, align=align, byterate=byterate)
        if cid == "data" and csz == 0x7FFFFFFF:
            out["note"] = "data chunk claims 0x7FFFFFFF"
        pos += 8+csz+(csz&1)
    return out

def aiff_info(data):
    sz = struct.unpack(">I", data[4:8])[0]
    out = {"form_size": sz, "form": data[8:12].decode("latin1"), "match": sz+8 == len(data)}
    pos = 12; chunks=[]
    while pos+8 <= len(data):
        cid = data[pos:pos+4].decode("latin1","replace")
        csz = struct.unpack(">I", data[pos+4:pos+8])[0]
        chunks.append((cid,csz))
        pos += 8+csz
        if len(chunks) > 20: break
    out["chunks"] = chunks
    return out

def mp4_boxes(data, pos=0, end=None, maxn=25, depth=0):
    if end is None: end = len(data)
    boxes = []
    while pos+8 <= end and len(boxes) < maxn:
        ln = struct.unpack(">I", data[pos:pos+4])[0]
        typ = data[pos+4:pos+8].decode("latin1","replace")
        hdr = 8
        if ln == 1:
            ln = struct.unpack(">Q", data[pos+8:pos+16])[0]; hdr = 16
        elif ln == 0:
            ln = end - pos
        boxes.append(("  "*depth + typ, ln, pos))
        if ln < hdr or pos+ln > end:
            boxes.append(("  "*depth + "!TRUNC", ln, pos)); break
        if typ in ("moov","trak","mdia","minf","stbl","edts","dinf","udta"):
            boxes.extend(mp4_boxes(data, pos+hdr, pos+ln, 12, depth+1))
        pos += ln
    return boxes

def ogg_info(data):
    return {"pages": data.count(b"OggS"), "first_page_hdr": data[:28].hex()}

def mkv_info(data):
    # parse first EBML element sizes at top level
    return {"magic": data[:4].hex(), "magic_is_ebml": data[:4].hex() == "1a45dfa3"}

def ts_info(data):
    n = len(data)
    sync = sum(1 for i in range(0, n-187, 188) if data[i]==0x47)
    return {"size": n, "packet_aligned": n % 188 == 0, "sync_at_stride": sync, "pkts": n//188}

def ivf_info(data):
    if data[:4] != b"DKIF": return {"magic": data[:4]}
    ver, hlen = struct.unpack("<HH", data[4:8])
    cc = struct.unpack("<I", data[8:12])[0]
    w, h = struct.unpack("<HH", data[12:16])
    rate, scale, frames = struct.unpack("<III", data[16:28])
    return {"ver": ver, "hdrlen": hlen, "fourcc": data[8:12].decode("latin1"),
            "width": w, "height": h, "rate": rate, "scale": scale, "frames": frames,
            "size": len(data)}

def main():
    print("=== W16-A10 BATCH METADATA (single script) ===")
    for rel in FILES:
        path = f"{ROOT}/{rel}"
        try:
            with open(path,"rb") as f: data = f.read()
        except FileNotFoundError:
            print(f"\n## {rel}\n  MISSING"); continue
        h = sha256(path)
        print(f"\n## {rel}")
        print(f"  size={len(data)} sha256={h}")
        print(f"  magic={data[:16].hex()}")
        try:
            if rel.endswith(".png"):
                print(f"  png={png_info(data)}")
            elif data[:4] == b"RIFF":
                if data[8:12] == b"WAVE":
                    print(f"  wav={wav_info(data)}")
                    print(f"  riff_decl={struct.unpack('<I', data[4:8])[0]+8}")
                else:
                    sz = struct.unpack("<I", data[4:8])[0]
                    print(f"  riff form={data[8:12]!r} decl_size={sz+8} chunks={riff_chunks(data)[:25]}")
            elif data[:4] == b"FORM":
                print(f"  aiff={aiff_info(data)}")
            elif data[:4] == b"OggS":
                print(f"  ogg={ogg_info(data)}")
            elif data[:4] == b"\x1aE\xdf\xa3":
                print(f"  mkv={mkv_info(data)}")
            elif data[:4] == b"DKIF":
                print(f"  ivf={ivf_info(data)}")
            elif data[4:8] == b"ftyp":
                boxes = mp4_boxes(data)
                print(f"  mp4_boxes={[(t,l) for t,l,_ in boxes][:34]}")
            elif data[0] == 0x47 and len(data) > 188:
                print(f"  ts={ts_info(data)}")
            elif data[:1] in (b"<", b"#", b"[") or data[:1].isdigit() or data[:1] in (b"/", b" ", b"-"):
                txt = data.decode("utf-8","replace")
                print(f"  text lines={txt.count(chr(10))+1}")
                print(f"  head={txt[:500]!r}")
            else:
                txt = data.decode("utf-8","replace")
                print(f"  text lines={txt.count(chr(10))+1}")
                print(f"  head={txt[:400]!r}")
        except Exception as e:
            print(f"  PARSE_ERROR {type(e).__name__}: {e}")

    print("\n=== PAIRS: malformed vs valid parent ===")
    for m, v in PAIRS:
        try:
            a = open(f"{ROOT}/{m}","rb").read(); b = open(f"{ROOT}/{v}","rb").read()
        except FileNotFoundError as e:
            print(f"  {m}: MISSING {e}"); continue
        n = min(len(a), len(b))
        diffs = [i for i in range(n) if a[i]!=b[i]]
        print(f"  {m} vs {v}:")
        print(f"    sizes {len(a)} vs {len(b)} (delta {len(a)-len(b)}); common-prefix diff bytes={len(diffs)}; offsets[:10]={diffs[:10]}")

    print("\n=== ORACLE checks ===")
    for oracle, target in [
        ("tests/fixtures/generated/valid/avi_mjpeg_pcm.avi.oracle.sha256", "tests/fixtures/generated/valid/avi_mjpeg_pcm.avi"),
        ("tests/fixtures/generated/valid/mp4_mjpeg_pcm.mp4.oracle.sha256", "tests/fixtures/generated/valid/mp4_mjpeg_pcm.mp4"),
    ]:
        try:
            content = open(f"{ROOT}/{oracle}").read()
            actual = sha256(f"{ROOT}/{target}")
            lines = [l for l in content.splitlines() if l.strip()]
            print(f"  {oracle}: {len(lines)} entries; target={target}")
            print(f"    actual_sha256={actual}")
            print(f"    file-hash-in-oracle={any(actual in l for l in lines)}")
            for l in lines[:8]: print(f"      | {l[:130]}")
        except FileNotFoundError as e:
            print(f"  {oracle}: MISSING {e}")

if __name__ == "__main__":
    main()
