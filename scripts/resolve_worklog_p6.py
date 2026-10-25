#!/usr/bin/env python3
"""Resolve worklog.md merge conflicts: append-only protocol, keep BOTH sides.

Phase-5 proven recipe. Usage: python3 scripts/resolve_worklog_p6.py
Reads the conflicted worklog.md, extracts each side's entries between the
conflict markers, interleaves them (ours first, then theirs), dedups
exact-duplicate Task IDs, writes back, stages.
"""
import re
import subprocess
import sys

PATH = "worklog.md"
text = open(PATH, encoding="utf-8").read()

# Split into pre-conflict, conflict hunks, post-conflict segments.
pattern = re.compile(r"<<<<<<< HEAD\n(.*?)=======\n(.*?)>>>>>>> [^\n]*\n", re.S)
matches = list(pattern.finditer(text))
if not matches:
    print("no conflict markers found — nothing to do")
    sys.exit(0)

out = []
last = 0
for m in matches:
    out.append(text[last : m.start()])
    ours, theirs = m.group(1), m.group(2)

    def entries(side: str) -> list[str]:
        parts = side.split("\n---\n")
        return [p.strip("\n") for p in parts if p.strip()]

    def task_id(entry: str) -> str | None:
        mm = re.search(r"^Task ID:\s*(\S+)", entry, re.M)
        return mm.group(1) if mm else None

    seen: set[str] = set()
    kept: list[str] = []
    for entry in entries(ours) + entries(theirs):
        tid = task_id(entry)
        if tid:
            if tid in seen:
                continue
            seen.add(tid)
        kept.append(entry)
    out.append("\n---\n".join(kept) + "\n")
    last = m.end()
out.append(text[last:])

resolved = "".join(out)
# Collapse >2 consecutive newlines introduced by the join.
resolved = re.sub(r"\n{3,}", "\n\n", resolved)
open(PATH, "w", encoding="utf-8").write(resolved)
subprocess.run(["git", "add", PATH], check=True)
print(f"resolved {len(matches)} hunk(s); marker-free: {'<<<' not in resolved}")
