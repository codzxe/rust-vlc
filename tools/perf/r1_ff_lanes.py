#!/usr/bin/env python3
"""r1_ff_lanes.py — ffmpeg decode-to-null reference times, median-of-5
(card 5.R1 breadth snapshot; the `speed_bench.py` protocol narrowed to
fixed lanes: fresh process per run, warm-up run discarded, wall clock
via time.monotonic around subprocess).

Usage: r1_ff_lanes.py FILE [FILE...] — prints CSV: lane,n,med_s,min_s,max_s
"""
import statistics
import subprocess
import sys
import time

REPS = 5  # +1 warm-up, discarded (doc 12 §3.5)


def bench(path: str) -> list[float]:
    walls = []
    for i in range(REPS + 1):
        t0 = time.monotonic()
        p = subprocess.run(
            ["ffmpeg", "-v", "error", "-nostdin", "-i", path, "-f", "null", "-"],
            capture_output=True, timeout=300)
        dt = time.monotonic() - t0
        if p.returncode != 0:
            print(f"# {path}: rc={p.returncode} {p.stderr.decode()[:120]}", file=sys.stderr)
            return []
        if i > 0:  # warm-up discarded
            walls.append(dt)
    return walls


def main() -> int:
    print("lane,n,med_s,min_s,max_s")
    for path in sys.argv[1:]:
        walls = bench(path)
        if walls:
            med = statistics.median(walls)
            print(f"{path},{len(walls)},{med:.4f},{min(walls):.4f},{max(walls):.4f}")
        else:
            print(f"{path},0,DNF,DNF,DNF")
    return 0


if __name__ == "__main__":
    sys.exit(main())
