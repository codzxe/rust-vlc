#!/usr/bin/env python3
"""Generic worklog.md append-append merge-conflict resolver.

Resolution: keep HEAD content in full, then append the branch (theirs) content.
Usage: resolve_worklog_append.py <branch-name>
"""
import sys

branch = sys.argv[1]
PATH = "/home/z/my-project/worklog.md"

with open(PATH, "r", encoding="utf-8") as f:
    text = f.read()

start_marker = "<<<<<<< HEAD\n"
mid_marker = "=======\n"
end_marker = f">>>>>>> {branch}\n"

start = text.index(start_marker)
mid = text.index(mid_marker, start)
end = text.index(end_marker, mid)

pre = text[:start]
head = text[start + len(start_marker):mid]
branch_content = text[mid + len(mid_marker):end]
post = text[end + len(end_marker):]

resolved = pre + head.rstrip("\n") + "\n\n" + branch_content.strip("\n") + "\n" + post

with open(PATH, "w", encoding="utf-8") as f:
    f.write(resolved)

# sanity checks
assert "<<<<<<<" not in resolved and ">>>>>>>" not in resolved
assert "\n=======" not in resolved
head_ids = [ln.split("Task ID:")[1].strip() for ln in head.splitlines() if ln.startswith("Task ID:")]
branch_ids = [ln.split("Task ID:")[1].strip() for ln in branch_content.splitlines() if ln.startswith("Task ID:")]
print(f"resolved: HEAD entries kept [{', '.join(head_ids)}] then branch entries [{', '.join(branch_ids)}]")
print(f"total lines: {len(resolved.splitlines())}")
