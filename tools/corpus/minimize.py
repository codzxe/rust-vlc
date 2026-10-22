#!/usr/bin/env python3
"""minimize.py — automated crash reproducer minimization (delta debugging).

Turns "file #248 (a 45 MB random video) crashes Lumen" into "this 1 KB
prefix still crashes Lumen" — the difference between hours of binary
staring and a five-minute triage.

Strategy (both classic ddmin stages, container-aware):
  1. TRUNCATE  — binary-search the minimal prefix that still reproduces
     (preserves container headers; catches header-path crashes fast).
  2. DDMIN     — chunk removal on the truncated prefix (Zeller-style:
     remove chunks, on failure-to-reproduce backtrack to finer chunks),
     then byte-granularity tail refinement on the surviving region.

Predicate: a shell command template containing {file} (and optionally
{size}).  The file "reproduces" when the command exits with the expected
return code (default 0 = your test command asserts the crash; use
--expect-rc N or --expect-rc any-nonzero to invert).

Flake control: --verify N re-runs the predicate on the final file N times;
a final file that does not reproduce stably is reported as FLAKY with the
observed rate (never silently accepted).

Usage:
  minimize.py --in crashing.file --predicate 'lumen-decode-check {file}' \
              [--out minimized.file] [--expect-rc 0|any-nonzero|N]
              [--max-tests 500] [--stage both|truncate|ddmin]
              [--chunk-align 16] [--verify 3] [--json report.json]
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path


class Pred:
    def __init__(self, template: str, expect: str, timeout: int):
        self.template = template
        self.expect = expect
        self.timeout = timeout
        self.calls = 0

    def __call__(self, data: bytes, tmpdir: Path, tag: str) -> bool:
        path = tmpdir / f"cand-{tag}.bin"
        path.write_bytes(data)
        cmd_str = self.template.replace("{file}", str(path)) \
                            .replace("{size}", str(len(data)))
        self.calls += 1
        try:
            p = subprocess.run(["bash", "-c", cmd_str], capture_output=True,
                               timeout=self.timeout)
            rc = p.returncode
        except subprocess.TimeoutExpired:
            return False  # a hang is not a reproduce (counted in calls)
        if self.expect == "any-nonzero":
            return rc != 0
        return rc == int(self.expect)


def truncation_min(pred: Pred, data: bytes, tmpdir: Path, max_tests: int,
                   state: dict) -> bytes:
    """Minimal prefix via binary search on the keep-length."""
    lo, hi = 1, len(data)  # invariant: pred(data[:hi]) is True
    while lo < hi and pred.calls < max_tests:
        mid = (lo + hi) // 2
        if pred.calls >= max_tests:
            break
        if pred(data[:mid], tmpdir, "tr"):
            hi = mid
        else:
            lo = mid + 1
        state["best"] = data[:hi]
    return data[:hi]


def ddmin(pred: Pred, data: bytes, tmpdir: Path, max_tests: int,
          align: int, state: dict) -> bytes:
    """Classic ddmin chunk removal + tail byte refinement."""
    n = 8
    cur = data
    while len(cur) > align and pred.calls < max_tests:
        chunk = max(align, (len(cur) + n - 1) // n)
        chunk = (chunk + align - 1) // align * align
        reduced = False
        i = 0
        while i < len(cur) and pred.calls < max_tests:
            cand = cur[:i] + cur[i + chunk:]
            if cand and pred(cand, tmpdir, "dd"):
                cur = cand
                reduced = True
                state["best"] = cur
                n = max(n - 1, 2)  # stay eager after a success
            else:
                i += chunk
        if not reduced:
            if n >= len(cur) // align:
                break
            n = min(n * 2, max(2, len(cur) // align))
    # byte-granularity tail trim (the last bytes often carry the trigger)
    trimmed = True
    while trimmed and pred.calls < max_tests and len(cur) > 1:
        trimmed = False
        for cut in (1, 2, 4, 8, 16, 64, 256, 1024):
            if len(cur) <= cut:
                continue
            if pred(cur[:-cut], tmpdir, "tt"):
                cur = cur[:-cut]
                state["best"] = cur
                trimmed = True
                break
    return cur


def main() -> int:
    ap = argparse.ArgumentParser(description="Lumen crash minimizer (ddmin)")
    ap.add_argument("--in", dest="inp", required=True)
    ap.add_argument("--predicate", required=True,
                    help="shell command; {file} substituted, e.g. "
                         "'check-crash {file}'")
    ap.add_argument("--out", default=None)
    ap.add_argument("--expect-rc", default="0",
                    help="0 (default), any-nonzero, or a specific code")
    ap.add_argument("--max-tests", type=int, default=400)
    ap.add_argument("--stage", choices=["both", "truncate", "ddmin"],
                    default="both")
    ap.add_argument("--chunk-align", type=int, default=16)
    ap.add_argument("--verify", type=int, default=3)
    ap.add_argument("--timeout", type=int, default=60)
    ap.add_argument("--json", default=None)
    args = ap.parse_args()

    src = Path(args.inp)
    data = src.read_bytes()
    out = Path(args.out) if args.out else src.with_suffix(
        src.suffix + ".min" + src.suffix)
    t0 = time.time()
    state = {"best": data}

    pred = Pred(args.predicate, args.expect_rc, args.timeout)
    with tempfile.TemporaryDirectory(prefix="lumen-min-") as td:
        tmpdir = Path(td)
        if not pred(data, tmpdir, "orig"):
            print("ERROR: the ORIGINAL file does not reproduce under the "
                  "predicate — nothing to minimize", file=sys.stderr)
            return 2
        cur = data
        if args.stage in ("both", "truncate"):
            cur = truncation_min(pred, cur, tmpdir, args.max_tests, state)
            print(f"[min] truncate stage: {len(cur)} bytes "
                  f"({100 * len(cur) / len(data):.1f}% of original)", flush=True)
        if args.stage in ("both", "ddmin"):
            cur = ddmin(pred, cur, tmpdir, args.max_tests,
                        args.chunk_align, state)
            print(f"[min] ddmin stage:    {len(cur)} bytes "
                  f"({100 * len(cur) / len(data):.1f}% of original)", flush=True)
        out.write_bytes(cur)
        # flake control: verify the final reproducer
        ok = sum(1 for _ in range(args.verify)
                 if pred(cur, tmpdir, "verify"))
        wall = time.time() - t0
        stable = ok == args.verify
        print(f"[min] RESULT {src.name}: {len(data)} -> {len(cur)} bytes "
              f"({100 * (1 - len(cur) / len(data)):.2f}% reduction), "
              f"{pred.calls} predicate calls, {wall:.1f}s, "
              f"verify {ok}/{args.verify} {'STABLE' if stable else 'FLAKY'}")
        print(f"[min] output: {out}")
        if args.json:
            Path(args.json).write_text(json.dumps({
                "input": str(src), "output": str(out),
                "orig_bytes": len(data), "min_bytes": len(cur),
                "reduction_pct": round(100 * (1 - len(cur) / len(data)), 3),
                "predicate": args.predicate, "expect_rc": args.expect_rc,
                "predicate_calls": pred.calls, "wall_s": round(wall, 1),
                "verify_ok": ok, "verify_n": args.verify,
                "stable": stable,
            }, indent=1))
        return 0 if stable else 1


if __name__ == "__main__":
    sys.exit(main())
