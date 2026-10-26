#!/usr/bin/env python3
"""6.VTA-oxideav-mpeg4video differential driver (SPIKE).

Per lane: ffprobe extradata -> hex file (CONFIG ONLY — the video samples
still come through OUR mp4 demuxer); harness decode -> mine.raw + FRAME
lines; ffmpeg 7.1.5 reference (-map 0:v:0 default mpeg4 decoder) ->
ref.raw + framehash md5 lines; per-frame MD5 + pixel delta comparison
(mine parsed by the FRAME dims walk, ref sliced by the same walk), plus
a whole-stream md5 cross-check.

Usage: python3 diff.py LANE [LANE...]   (LANE.mp4 under $LANES dir)
Env:   HARNESS (default /tmp/vta-mpeg4-target/debug/vta-mpeg4),
       LANES (default /tmp/vta-mpeg4/lanes), OUT (default /tmp/vta-mpeg4/out)
"""
import hashlib, json, os, subprocess, sys

HARNESS = os.environ.get("HARNESS", "/tmp/vta-mpeg4-target/debug/vta-mpeg4")
LANES = os.environ.get("LANES", "/tmp/vta-mpeg4/lanes")
OUT = os.environ.get("OUT", "/tmp/vta-mpeg4/out")
os.makedirs(OUT, exist_ok=True)


def run_lane(name, path):
    st = {}
    # extradata via ffprobe (config only; samples come from OUR demuxer)
    ed = os.path.join(OUT, name + ".extradata.txt")
    r = subprocess.run(["ffprobe", "-v", "error", "-select_streams", "v:0",
                        "-show_data", "-show_entries", "stream=extradata",
                        "-of", "json", path],
                       capture_output=True, text=True)
    import re
    m = re.search(r'"extradata": "((?:[^"\\]|\\.)*)"', r.stdout, re.S)
    hexs = ""
    if m:
        # ffprobe -show_data prints a hexdump: "00000000: 0000 01b0 ...  ascii"
        for hline in m.group(1).split("\\n"):
            if ": " in hline:
                hpart = hline.split(": ", 1)[1]
                for tok in hpart.split()[:8]:
                    if re.fullmatch(r"[0-9a-fA-F]{2,8}", tok) and len(tok) % 2 == 0:
                        hexs += tok
    open(ed, "w").write(hexs)
    st["extradata_bytes"] = len(hexs) // 2
    # our side
    mine_raw = os.path.join(OUT, name + ".mine.raw")
    cmd = [HARNESS, path, "--out", mine_raw, "--extradata", ed]
    r = subprocess.run(cmd, capture_output=True, text=True)
    open(os.path.join(OUT, name + ".mine.log"), "w").write(r.stdout + r.stderr)
    frames = []
    for line in r.stdout.splitlines():
        if line.startswith("packets "):
            st["packets_line"] = line
        elif line.startswith("frames "):
            st["summary_line"] = line
        elif line.startswith("FRAME "):
            parts = line.split()
            frames.append({"idx": int(parts[1]), "type": parts[2],
                           "dims": parts[3], "pts": parts[4], "ticks": parts[5]})
    st["my_frames"] = len(frames)
    if r.returncode != 0:
        st["harness_rc"] = r.returncode
        return st, frames
    # reference side
    ref_raw = os.path.join(OUT, name + ".ref.raw")
    r2 = subprocess.run(["ffmpeg", "-y", "-v", "error", "-i", path, "-map", "0:v:0",
                         "-f", "rawvideo", "-pix_fmt", "yuv420p", ref_raw],
                        capture_output=True, text=True)
    if r2.returncode != 0:
        st["ref_rc"] = r2.returncode
        return st, frames
    r3 = subprocess.run(["ffmpeg", "-v", "error", "-i", path, "-map", "0:v:0",
                         "-f", "framehash", "-hash", "md5", "-"],
                        capture_output=True, text=True)
    ref_md5s = [l.split(",")[-1] for l in r3.stdout.splitlines() if l.startswith("0,")]
    st["ref_frames"] = len(ref_md5s)
    # per-frame compare: walk my dims
    mine = open(mine_raw, "rb").read()
    ref = open(ref_raw, "rb").read()
    identical = 0
    maxdelta = 0
    ndiff = 0
    pos_m = 0
    pos_r = 0
    for i, f in enumerate(frames):
        w, h = f["dims"].split("x")
        w, h = int(w), int(h)
        n = w * h + 2 * ((w + 1) // 2) * ((h + 1) // 2)
        fm = mine[pos_m:pos_m + n]
        fr = ref[pos_r:pos_r + n]
        pos_m += n
        pos_r += n
        hm = hashlib.md5(fm).hexdigest()
        hr = hashlib.md5(fr).hexdigest() if len(fr) == n else "<short>"
        same = hm == hr
        d = 0
        pixdiff = 0
        if not same and len(fr) == n:
            for a, b in zip(fm, fr):
                if a != b:
                    pixdiff += 1
                    dd = abs(a - b)
                    if dd > d:
                        d = dd
            ndiff += 1
        if same:
            identical += 1
        if d > maxdelta:
            maxdelta = d
        f["md5_match"] = same
        f["max_delta"] = d
        f["pixdiff"] = pixdiff
    st["identical_frames"] = identical
    st["divergent_frames"] = ndiff
    st["max_pixel_delta"] = maxdelta
    st["bytes_mine"] = len(mine)
    st["bytes_ref"] = len(ref)
    st["stream_md5_equal"] = hashlib.md5(mine).hexdigest() == hashlib.md5(ref).hexdigest()
    return st, frames


if __name__ == "__main__":
    lanes = sys.argv[1:]
    results = {}
    for name in lanes:
        path = os.path.join(LANES, name + ".mp4")
        st, frames = run_lane(name, path)
        results[name] = st
        print(f"=== {name} ===")
        for k in ("packets_line", "extradata_bytes", "my_frames", "ref_frames",
                  "identical_frames", "divergent_frames", "max_pixel_delta",
                  "bytes_mine", "bytes_ref", "stream_md5_equal", "summary_line",
                  "harness_rc", "ref_rc"):
            if k in st:
                print(f"  {k}: {st[k]}")
        if st.get("my_frames") == st.get("ref_frames"):
            mism = [i for i, f in enumerate(frames) if not f.get("md5_match", False)]
            print(f"  md5_mismatch_frames: {mism[:10]}{'...' if len(mism) > 10 else ''} ({len(mism)} total)")
    slim = {k: {kk: vv for kk, vv in v.items() if kk != "per_frame"} for k, v in results.items()}
    open(os.path.join(OUT, "results.json"), "w").write(json.dumps(slim, indent=1, default=str))
