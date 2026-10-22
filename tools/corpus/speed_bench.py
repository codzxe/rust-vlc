#!/usr/bin/env python3
"""speed_bench.py — decode-speed benchmark: Lumen vs ffmpeg (parity target).

The user target: "add comparison to ffmpeg — we need to reach their speed."
This tool makes that a MEASURED number, not an aspiration:

  * Baseline: ffmpeg decode-to-null (`-i FILE -f null -`) per corpus file,
    best-of-N wall time + child CPU time (rusage) + frames/s + realtime
    multiple.  ffmpeg IS the reference decoder speed.
  * Challenger(s): any number of `--runner NAME='CMD {file}'` hooks (the
    Lumen decode CLI when it lands; today the codecs corpus suite emits
    per-case decode timings that can be wired in).
  * Output: per-class aggregate tables (median/p95 decode speed, frames/s,
    realtime factor) + per-runner RATIO vs the ffmpeg baseline row-by-row
    where the runner produced a time.  JSON sidecar for trend tracking.

Honesty rules: warmup decode discarded; each timing is a fresh process;
timeouts recorded as DNF (did not finish), never averaged in; classes with
fewer than 3 measured files are marked low-N.

Usage:
  speed_bench.py --corpus corpus-big [--count 60] [--repeat 3]
                 [--runner lumen='lumen decode-null {file}']
                 [--json out.json] [--md out.md]
"""

from __future__ import annotations

import argparse
import json
import os
import resource
import statistics
import subprocess
import sys
import time
from collections import defaultdict
from pathlib import Path

FFMPEG = os.environ.get("FFMPEG", "ffmpeg")
FFPROBE = os.environ.get("FFPROBE", "ffprobe")


def sh(cmd: list[str], timeout: int):
    return subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)


def child_cpu() -> float:
    ru = resource.getrusage(resource.RUSAGE_CHILDREN)
    return ru.ru_utime + ru.ru_stime


def decode_frames_and_duration(path: Path) -> tuple[int, float]:
    """(frames, seconds) via ffprobe — the denominator of the fps metric."""
    p = sh([FFPROBE, "-v", "error", "-select_streams", "v:0",
            "-count_packets", "-show_entries",
            "stream=nb_read_packets:format=duration", "-of", "json",
            str(path)], 120)
    try:
        j = json.loads(p.stdout)
        n = int(j["streams"][0].get("nb_read_packets") or 0)
        d = float(j["format"].get("duration") or 0)
        return n, d
    except Exception:
        return 0, 0.0


def bench_one(cmd_template: str, path: Path, repeat: int, timeout: int,
              warmup: bool = True) -> dict:
    """Best-of-N wall + per-child CPU time for one command on one file."""
    runs = []
    # The runner command is a shell template; warmup decode is discarded.
    for i in range(repeat + (1 if warmup else 0)):
        cmd = cmd_template.replace("{file}", str(path))
        c0, c1 = child_cpu(), None
        t0 = time.monotonic()
        try:
            p = subprocess.run(["bash", "-c", cmd], capture_output=True,
                               timeout=timeout)
            rc = p.returncode
        except subprocess.TimeoutExpired:
            rc = 124
        wall = time.monotonic() - t0
        c1 = child_cpu()
        if i == 0 and warmup:
            continue  # discard the warmup (page cache, JIT-ish effects)
        runs.append({"wall": wall, "cpu": c1 - c0, "rc": rc})
    ok = [r for r in runs if r["rc"] == 0]
    if not ok:
        return {"ok": False, "dnf": True, "rc": runs[0]["rc"] if runs else -1}
    best = min(r["wall"] for r in ok)
    return {"ok": True, "wall_best": best,
            "wall_median": statistics.median(r["wall"] for r in ok),
            "cpu_best": min(r["cpu"] for r in ok), "n": len(ok)}


def fmt_row(cells: list[str], widths: list[int]) -> str:
    return "| " + " | ".join(c.ljust(w) for c, w in zip(cells, widths)) + " |"


def main() -> int:
    ap = argparse.ArgumentParser(description="Lumen speed-parity benchmark")
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--count", type=int, default=60)
    ap.add_argument("--stride", type=int, default=1,
                    help="sample every Nth clean media file")
    ap.add_argument("--repeat", type=int, default=3)
    ap.add_argument("--timeout", type=int, default=180)
    ap.add_argument("--runner", action="append", default=[],
                    help="NAME='shell cmd with {file}' (repeatable)")
    ap.add_argument("--json", default=None)
    ap.add_argument("--md", default=None)
    ap.add_argument("--classes", default=None,
                    help="comma filter, e.g. avi-video,mkv-video")
    args = ap.parse_args()

    corpus = Path(args.corpus)
    manifest = json.loads((corpus / "manifest.json").read_text())
    clean = [f for f in manifest["files"]
             if f["rel"].startswith("media/") and f.get("ffprobe_ok")]
    if args.classes:
        keep = set(args.classes.split(","))
        clean = [f for f in clean if f["class"] in keep]
    clean = clean[::args.stride][:args.count]
    if not clean:
        print("no clean media files to bench", file=sys.stderr)
        return 2

    runners = {"ffmpeg": f"{FFMPEG} -v error -nostdin -i {{file}} -f null -"}
    for r in args.runner:
        if "=" not in r:
            print(f"bad --runner (need NAME='CMD'): {r}", file=sys.stderr)
            return 2
        name, cmd = r.split("=", 1)
        runners[name] = cmd

    per_file = []
    t0 = time.time()
    for i, entry in enumerate(clean):
        path = corpus / entry["rel"]
        frames, dur = decode_frames_and_duration(path)
        rec = {"rel": entry["rel"], "class": entry["class"],
               "bytes": entry["bytes"], "frames": frames, "dur_s": dur}
        for name, tpl in runners.items():
            r = bench_one(tpl, path, args.repeat, args.timeout)
            if r.get("ok"):
                r["fps"] = frames / r["wall_best"] if frames else None
                r["realtime_x"] = dur / r["wall_best"] if dur else None
            rec[name] = r
        per_file.append(rec)
        if (i + 1) % 10 == 0:
            print(f"[bench] {i + 1}/{len(clean)} files, "
                  f"{time.time() - t0:.0f}s", flush=True)

    # ---- aggregate per class per runner ----
    agg = defaultdict(lambda: defaultdict(list))
    for rec in per_file:
        for name, r in rec.items():
            if isinstance(r, dict) and r.get("ok"):
                agg[rec["class"]][name].append(r)
    classes = sorted({r["class"] for r in per_file})
    lines = ["# Speed-parity benchmark (decode-to-null)",
             "",
             f"corpus: `{corpus}` | files: {len(per_file)} | "
             f"best-of-{args.repeat} | runners: {', '.join(runners)}",
             ""]
    header = ["class", "n", "runner", "med wall", "p95 wall", "med fps",
              "med realtime", "ratio vs ffmpeg"]
    widths = [max(len(h), 18) for h in header]
    lines.append(fmt_row(header, widths))
    lines.append("|" + "|".join("-" * (w + 2) for w in widths) + "|")
    for cls in classes:
        for name in runners:
            rows = agg[cls].get(name, [])
            if not rows:
                lines.append(fmt_row([cls, str(len(per_file and [r for r in per_file if r['class'] == cls])), name,
                                      "DNF", "-", "-", "-", "-", "-"], widths))
                continue
            walls = sorted(r["wall_best"] for r in rows)
            med = statistics.median(walls)
            p95 = walls[max(0, int(len(walls) * 0.95) - 1)]
            fps = [r["fps"] for r in rows if r.get("fps")]
            rt = [r["realtime_x"] for r in rows if r.get("realtime_x")]
            ratio = ""
            if name != "ffmpeg":
                ffwalls = [r["wall_best"] for r in agg[cls].get("ffmpeg", [])]
                if ffwalls:
                    ratio = f"{statistics.median(ffwalls) / med:.2f}x"
            n_note = "" if len(rows) >= 3 else " (low-N)"
            lines.append(fmt_row([
                cls + n_note, str(len(rows)), name,
                f"{med * 1000:.1f} ms", f"{p95 * 1000:.1f} ms",
                f"{statistics.median(fps):.1f}" if fps else "-",
                f"{statistics.median(rt):.2f}x" if rt else "-",
                ratio or "-"], widths))
    report = "\n".join(lines)
    print(report)
    if args.md:
        Path(args.md).write_text(report + "\n")
    if args.json:
        Path(args.json).write_text(json.dumps({
            "corpus": str(corpus), "repeat": args.repeat,
            "runners": list(runners), "per_file": per_file,
        }, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
