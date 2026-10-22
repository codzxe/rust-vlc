#!/usr/bin/env python3
"""Resolve the W1-07/W1-08 session_flow.rs merge conflict by union:
W1-07's self-contained bounds tests are inserted BEFORE the W1-08
concealment block (region 1), and both sides' tail fns are kept with an
explicit closing brace inserted between them (region 2)."""

from pathlib import Path

p = Path("crates/playback-engine/tests/session_flow.rs")
lines = p.read_text().splitlines(keepends=True)

# 0-based half-open indices (converted from the 1-based rg output):
# region 1: <<< @1298, == @1541, >>> @1600
# region 2: <<< @1605, == @1843, >>> @1850
a1 = (1298, 1540)      # HEAD side of region 1 (0-based slice 1298:1540)
b1 = (1541, 1599)      # W1-07 side of region 1
mid = (1600, 1604)     # text between >>> of r1 and <<< of r2 (the '<<<<<<<' of r2 is at 1604 0-based)
a2 = (1605, 1842)      # HEAD side of region 2
b2 = (1843, 1849)      # W1-07 side of region 2
tail_start = 1850      # after >>> of r2

def blk(rng):
    return lines[rng[0]:rng[1]]

out = []
out += lines[:1298]                     # pre-region-1 (0-based up to marker line)
# W1-07's complete tests first (they were pure additions)
out += blk(b1)
if out and not out[-1].endswith("\n\n"):
    out.append("\n")
# then W1-08's HEAD block, which flows into the post-marker continuation
out += blk(a1)
out += lines[1599:1604]                 # >>> line itself skipped; text up to r2 marker
# region 2: HEAD fn then explicit close, then W1-07 fn, then shared tail
out += blk(a2)
out.append("}\n\n")
out += blk(b2)
out += lines[tail_start:]

p.write_text("".join(out))
print("resolved; lines:", len("".join(out).splitlines()))
