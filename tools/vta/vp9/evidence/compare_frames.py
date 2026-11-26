#!/usr/bin/env python3
"""Per-frame MD5 comparator for the rusty_vp9 §D differential (FX8-A06).

Compares the tightly packed I420 rawvideo our vta-vp9 harness writes
(`mine`) against the ffmpeg `libvpx-vp9` rawvideo reference (`ref`), by
the record's method: per-frame MD5 of frame-sized chunks on both sides,
plus a whole-stream MD5 cross-check
(docs/dependencies/records/rusty_vp9-0.1.3.md §D; tools/vta/vp9/README.md).

Frame size = bps * (w*h + 2*ceil(w/2)*ceil(h/2)); bps = 1 for 8-bit
(yuv420p) and 2 for profile-2 10/12-bit (the harness emits u16LE per
sample — byte-layout identical to ffmpeg rawvideo yuv420p10le).

Exit codes:
  0  every frame byte-identical (bit-exact)
  1  any difference (frame count, byte total, per-frame digest,
     whole-stream digest)
  2  usage or input error (fail-loud; never a tolerance — the record's
     classification is BIT-EXACT, not a delta budget)

No third-party imports: python3 stdlib only, no cargo.
"""

from __future__ import annotations

import argparse
import hashlib
import math
import sys
from pathlib import Path


def fail_loud(message: str) -> int:
    print(f"compare_frames: FAIL — {message}", file=sys.stderr)
    return 2


def frame_size(width: int, height: int, bps: int) -> int:
    chroma = math.ceil(width / 2) * math.ceil(height / 2)
    return bps * (width * height + 2 * chroma)


def md5(data: bytes) -> str:
    return hashlib.md5(data).hexdigest()


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Per-frame MD5 compare of the vta-vp9 mine vs ffmpeg "
        "reference rawvideo (record §D method; bit-exact, no tolerance)."
    )
    parser.add_argument("mine", help="rawvideo written by the vta-vp9 harness")
    parser.add_argument("ref", help="ffmpeg libvpx-vp9 rawvideo reference")
    parser.add_argument("--frames", type=int, required=True,
                        help="expected frame count (record §D table)")
    parser.add_argument("--size", required=True,
                        help="coded size WxH (record §D table)")
    parser.add_argument("--bps", type=int, choices=(1, 2), default=1,
                        help="bytes per sample: 1 = 8-bit yuv420p, "
                        "2 = 10/12-bit u16LE (profile 2)")
    args = parser.parse_args(argv)

    if "x" not in args.size:
        return fail_loud(f"bad --size {args.size!r} (want WxH)")
    try:
        width, height = (int(v) for v in args.size.lower().split("x", 1))
    except ValueError:
        return fail_loud(f"bad --size {args.size!r} (want WxH)")
    if width <= 0 or height <= 0 or args.frames <= 0:
        return fail_loud(f"bad geometry: {args.size} @ {args.frames} frames")

    fsz = frame_size(width, height, args.bps)
    expected = fsz * args.frames

    for label, path in (("mine", args.mine), ("ref", args.ref)):
        if not Path(path).is_file():
            return fail_loud(f"{label} input not a file: {path}")
    mine = Path(args.mine).read_bytes()
    ref = Path(args.ref).read_bytes()

    if len(mine) != expected or len(ref) != expected:
        return fail_loud(
            f"byte totals: mine {len(mine)}, ref {len(ref)}, "
            f"expected {expected} ({args.frames} x {fsz} B frames, "
            f"{args.size} @ bps {args.bps})"
        )

    identical = 0
    first_bad = None
    for i in range(args.frames):
        chunk_m = mine[i * fsz:(i + 1) * fsz]
        chunk_r = ref[i * fsz:(i + 1) * fsz]
        if md5(chunk_m) == md5(chunk_r):
            identical += 1
        elif first_bad is None:
            first_bad = (i, md5(chunk_m), md5(chunk_r))

    stream_equal = md5(mine) == md5(ref)
    if identical == args.frames and stream_equal:
        print(
            f"compare_frames: PASS — {identical}/{args.frames} frames "
            f"byte-identical ({args.size}, bps {args.bps}); whole-stream "
            f"MD5 {md5(mine)} equal both sides"
        )
        return 0

    if first_bad is not None:
        i, dm, dr = first_bad
        print(
            f"compare_frames: FAIL — first differing frame {i}: "
            f"mine {dm} vs ref {dr} ({args.size}, bps {args.bps})",
            file=sys.stderr,
        )
    print(
        f"compare_frames: FAIL — {identical}/{args.frames} frames identical; "
        f"whole-stream MD5 mine {md5(mine)} vs ref {md5(ref)}",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
