#!/usr/bin/env python3
"""Union-resolve the W2-04 x W2-02 scheduler.rs merge: both sides'
doc lines and test blocks are kept."""

from pathlib import Path

p = Path("crates/video-engine/src/scheduler.rs")
lines = p.read_text().splitlines(keepends=True)

# 0-based marker indices from rg (1-based 489, 492, 498 / 536, 540, 547 /
# 1061, 1205, 1343)
def idx(one_based):
    return one_based - 1

out = []
# pre-region-1
out += lines[:idx(489)]
# region 1: union of both doc comments (HEAD then theirs)
out += lines[idx(489) + 1:idx(492)]
out += lines[idx(492) + 1:idx(498)]
# between 1 and 2
out += lines[idx(498) + 1:idx(536)]
# region 2: union both comments
out += lines[idx(536) + 1:idx(540)]
out += lines[idx(540) + 1:idx(547)]
# between 2 and 3
out += lines[idx(547) + 1:idx(1061)]
# region 3: HEAD test block then theirs (both complete)
out += lines[idx(1061) + 1:idx(1205)]
out += lines[idx(1205) + 1:idx(1343)]
# tail
out += lines[idx(1343) + 1:]

p.write_text("".join(out))
joined = "".join(out)
assert "<<<<<<<" not in joined and ">>>>>>>" not in joined
print("resolved:", len(joined.splitlines()), "lines")
