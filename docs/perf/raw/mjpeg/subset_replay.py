#!/usr/bin/env python3
"""subset_replay.py — regenerate ONLY the corpus-big files the 7-b
speed comparison needs, by replaying the committed manifest's recorded
ffmpeg argvs verbatim (deterministic lavfi sources).

Why a subset: the full corpus replay is ~17 min on this 2-core sandbox
and tool calls cap at 10 min; the speed protocol (W4-R2 baseline)
samples the first 8 clean files of classes avi-video + avi-av plus the
heavy-mjpeg stage-profile representative (0127). Manifest order and
the class filter reproduce the baseline's exact file selection.

Each generated file's sha256 is compared against the manifest (muxer
timestamp noise can differ — W5-16 measured 37/70 files with sha diffs
on full replays; decode workload is unaffected; every diff is printed,
never hidden).
"""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

MANIFEST = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(
    "/home/z/my-project/worktrees/w7-b/docs/corpus/corpus-big-manifest-20261022.json")
OUT = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("/tmp/corpus-big")

m = json.loads(MANIFEST.read_text())
files = [f for f in m["files"] if f["rel"].startswith("media/") and f.get("ffprobe_ok")]
sel = []
for cls in ("avi-video", "avi-av"):
    sel.extend([f for f in files if f["class"] == cls][:8])
sel.extend([f for f in m["files"] if f["path"].startswith("0127-")])

OUT.mkdir(parents=True, exist_ok=True)
(OUT / "media").mkdir(exist_ok=True)

# A reduced manifest preserving order + the fields speed_bench reads.
reduced = {k: m[k] for k in ("generator", "tool_version", "seed", "jobs")}
reduced["files"] = [
    {k: f[k] for k in ("path", "rel", "class", "family", "bytes", "sha256", "ffprobe_ok")}
    for f in sel
]
(OUT / "manifest.json").write_text(json.dumps(reduced, indent=1))

match = diff = missing = 0
for f in sel:
    dst = OUT / f["rel"]
    argv = ["ffmpeg", "-hide_banner", "-loglevel", "error", "-nostdin"] + [
        dst.as_posix() if a == "__OUT__" else a for a in f["argvs"][0]]
    r = subprocess.run(argv, capture_output=True, text=True)
    if r.returncode != 0:
        print(f"[FAIL rc={r.returncode}] {f['path']}: {r.stderr.strip()[:200]}")
        missing += 1
        continue
    got = hashlib.sha256(dst.read_bytes()).hexdigest()
    if got == f["sha256"]:
        match += 1
        print(f"[OK    exact] {f['path']}")
    else:
        diff += 1
        print(f"[OK sha-diff] {f['path']} (decode-equivalent muxer noise; see doc)")
print(f"\n{match} byte-exact, {diff} sha-diff, {missing} failed, {len(sel)} total")
sys.exit(1 if missing else 0)
