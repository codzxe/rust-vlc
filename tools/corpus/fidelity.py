#!/usr/bin/env python3
"""fidelity.py — differential fidelity scoring: Lumen's decode vs ffmpeg's.

"Did it exit without crashing?" is not a fidelity answer.  This tool puts a
NUMBER on decode correctness by comparing two raw streams (or two media
files, auto-decoded) with ffmpeg's own PSNR / SSIM filters — the external
oracle computes the score, never Lumen.

Verdict bands (PSNR, 8-bit luma scale):
  >= 48 dB  imperceptible   42-48 excellent   35-42 good
  28-35     noticeable      < 28  FAIL (flagged)

Video:  ffmpeg psnr + ssim filters over two rawvideo inputs; per-frame
        series parsed and reported (min/median/mean + worst-frame index).
Audio:  pure-Python RMS/peak error over interleaved PCM (s16le/f32le) —
        no numpy dependency, chunked so 100 MB files stay cheap; reports
        RMS dBFS error, peak delta, and exact-equal sample count.

Usage:
  fidelity.py --a lumen.raw --b ref.raw --video --pix-fmt yuv420p \\
              --size 1280x720 [--fps 30] [--json out.json]
  fidelity.py --a lumen.raw --b ref.raw --audio --fmt s16le --rate 48000 \\
              --channels 2
"""

from __future__ import annotations

import argparse
import json
import math
import re
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path

FFMPEG = "ffmpeg"
FFPROBE = "ffprobe"

PSNR_RE = re.compile(r"n:(\d+)\s+mse_avg:([\d._infy]+).*?psnr_avg:([\d._infy]+)")
# ssim prints "SSIM Y:v (dB) U:v (dB) V:v (dB) All:v (dB)" — per-frame
# lines AND a final summary share this shape; the last match is the summary.
SSIM_RE = re.compile(r"SSIM Y:([\d.]+) \([^)]*\) U:([\d.]+) \([^)]*\) "
                     r"V:([\d.]+) \([^)]*\) All:([\d.]+)(?: \(([\d.]+) dB\))?")
PSNR_SUM_RE = re.compile(r"average:([\d._infy]+)")


def sh(cmd: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, capture_output=True, text=True, timeout=1800)


def video_scores(a: Path, b: Path, pix_fmt: str, size: str, fps: float,
                 ) -> dict:
    """Run ffmpeg's psnr and ssim filters over two rawvideo streams."""
    common = ["-f", "rawvideo", "-pix_fmt", pix_fmt, "-s", size,
              "-r", f"{fps:g}"]
    results = {}
    for metric in ("psnr", "ssim"):
        p = sh([FFMPEG, "-hide_banner", "-nostdin"] + common + ["-i", str(a)]
               + common + ["-i", str(b)]
               + ["-lavfi", f"[0:v][1:v]{metric}", "-f", "null", "-"])
        per_frame, summary = [], None
        for line in (p.stderr or "").splitlines():
            if metric == "psnr":
                m = PSNR_RE.search(line)
                if m:
                    per_frame.append(float(m.group(3)))
                s = PSNR_SUM_RE.search(line)
                if s:
                    summary = float(s.group(1))
            else:
                m = SSIM_RE.search(line)
                if m:
                    per_frame.append(float(m.group(4)))  # All:
                    # the last "SSIM ..." line is the summary
                    summary = float(m.group(4))
        if not per_frame and summary is None:
            return {"error": f"{metric} filter produced no output",
                    "stderr_tail": (p.stderr or "")[-400:]}
        finite = [v for v in per_frame if math.isfinite(v)]
        results[metric] = {
            "average": summary,
            "frames": len(per_frame),
            "min": min(finite) if finite else None,
            "max": max(finite) if finite else None,
            "mean": statistics.fmean(finite) if finite else None,
            "inf_frames": len(per_frame) - len(finite),  # identical frames
        }
    return results


def audio_scores(a: Path, b: Path, fmt: str, rate: int, channels: int) -> dict:
    """RMS/peak error over interleaved PCM — pure Python, chunked."""
    bytes_per_sample = {"s16le": 2, "s32le": 4, "f32le": 4, "u8": 1}[fmt]
    frame = bytes_per_sample * channels
    n = min(a.stat().st_size, b.stat().st_size)
    aligned = (n // frame) * frame
    import array
    typecode = {"s16le": "h", "s32le": "i", "f32le": "f", "u8": "B"}[fmt]
    if fmt == "u8":
        # unsigned 8-bit: center at 128
        scale, center = 128.0, 128.0
    elif fmt == "f32le":
        scale, center = 1.0, 0.0
    else:
        scale, center = float(2 ** (8 * (bytes_per_sample - 1))), 0.0
    sq_err = peak = 0.0
    samples = exact = total = 0
    CH = 1 << 20
    with a.open("rb") as fa, b.open("rb") as fb:
        while True:
            da = fa.read(CH)
            db = fb.read(CH)
            if not da or not db:
                break
            k = min(len(da), len(db))
            k = (k // frame) * frame
            if k == 0:
                break
            aa = array.array(typecode)
            aa.frombytes(da[:k])
            bb = array.array(typecode)
            bb.frombytes(db[:k])
            for x, y in zip(aa, bb):
                if fmt == "f32le":
                    dx, dy = x, y
                else:
                    dx, dy = (x - center) / scale, (y - center) / scale
                e = dx - dy
                sq_err += e * e
                ae = abs(e)
                if ae > peak:
                    peak = ae
                if ae == 0.0:
                    exact += 1
                total += 1
    if total == 0:
        return {"error": "no comparable samples"}
    rms = math.sqrt(sq_err / total)
    db = 20 * math.log10(rms) if rms > 0 else float("inf")
    return {"samples": total, "rms": rms, "rms_db": db,
            "peak_error": peak, "exact_frac": exact / total}


def probe_media(path: Path) -> dict:
    p = sh([FFPROBE, "-v", "error", "-select_streams", "v:0",
            "-show_entries", "stream=width,height,pix_fmt,avg_frame_rate",
            "-of", "json", str(path)])
    import json as j
    try:
        st = j.loads(p.stdout)["streams"][0]
        w, h = st["width"], st["height"]
        num, den = (st.get("avg_frame_rate") or "25/1").split("/")
        fps = float(num) / float(den) if float(den) else 25.0
        return {"w": w, "h": h, "pix_fmt": st.get("pix_fmt", "yuv420p"),
                "fps": fps or 25.0}
    except Exception:
        return {}


def decode_raw(media: Path, out: Path, what: str) -> bool:
    if what == "video":
        p = sh([FFMPEG, "-hide_banner", "-nostdin", "-v", "error",
                "-i", str(media), "-map", "0:v:0", "-c:v", "rawvideo",
                "-pix_fmt", "yuv420p", "-f", "rawvideo", "-y", str(out)])
    else:
        p = sh([FFMPEG, "-hide_banner", "-nostdin", "-v", "error",
                "-i", str(media), "-map", "0:a:0", "-c:a", "pcm_s16le",
                "-f", "s16le", "-y", str(out)])
    return p.returncode == 0 and out.stat().st_size > 0


def main() -> int:
    ap = argparse.ArgumentParser(description="Lumen fidelity scorer")
    ap.add_argument("--a", required=True, help="first raw stream (Lumen)")
    ap.add_argument("--b", required=True, help="second raw stream (oracle)")
    ap.add_argument("--video", action="store_true")
    ap.add_argument("--audio", action="store_true")
    ap.add_argument("--pix-fmt", default="yuv420p")
    ap.add_argument("--size", default=None, help="WxH (required for --video)")
    ap.add_argument("--fps", type=float, default=25.0)
    ap.add_argument("--fmt", default="s16le",
                    choices=["s16le", "s32le", "f32le", "u8"])
    ap.add_argument("--rate", type=int, default=48000)
    ap.add_argument("--channels", type=int, default=2)
    ap.add_argument("--json", default=None)
    args = ap.parse_args()

    a, b = Path(args.a), Path(args.b)
    if not args.video and not args.audio:
        print("specify --video or --audio", file=sys.stderr)
        return 2

    if args.video:
        if args.size is None:
            print("--video requires --size WxH", file=sys.stderr)
            return 2
        res = video_scores(a, b, args.pix_fmt, args.size, args.fps)
        if "error" in res:
            print(f"ERROR: {res['error']}\n{res.get('stderr_tail','')}",
                  file=sys.stderr)
            return 3
        psnr = res["psnr"]
        avg = psnr["average"]
        if avg is not None and math.isfinite(avg):
            band = ("imperceptible" if avg >= 48 else
                    "excellent" if avg >= 42 else
                    "good" if avg >= 35 else
                    "noticeable" if avg >= 28 else "FAIL")
        else:
            band = "identical" if psnr.get("inf_frames") else "?"
        print(f"Video Fidelity: {avg if avg is not None else float('inf')}"
              f" dB ({band})")
        psnr_mean = psnr['mean'] if psnr['mean'] is not None else float('inf')
        s = res["ssim"]
        ssim_mean = s['mean'] if s.get('mean') is not None else s['average']
        print(f"  PSNR  avg={avg} min={psnr['min']} mean={psnr_mean:.3f} "
              f"frames={psnr['frames']} inf(identical)={psnr['inf_frames']}")
        print(f"  SSIM  all={s['average']} min={s['min']} "
              f"mean={ssim_mean}")
    else:
        res = audio_scores(a, b, args.fmt, args.rate, args.channels)
        if "error" in res:
            print(f"ERROR: {res['error']}", file=sys.stderr)
            return 3
        print(f"Audio Fidelity: RMS error {res['rms_db']:.1f} dBFS, "
              f"peak {res['peak_error']:.6f}, "
              f"exact-equal {res['exact_frac'] * 100:.4f}% "
              f"({res['samples']} samples)")

    if args.json:
        Path(args.json).write_text(json.dumps(res, indent=1))
    # exit code carries the verdict for CI wiring
    if args.video and res["psnr"]["average"] is not None \
            and math.isfinite(res["psnr"]["average"]) \
            and res["psnr"]["average"] < 28:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
