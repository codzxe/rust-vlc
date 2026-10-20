#!/usr/bin/env python3
"""Resolve worklog.md append-append merge conflict in chronological order.

HEAD side entries:  2.WAVE2-INCIDENT, 2.C1 (close-out), INFRA-BACKUP-1
Branch side entry:  2.A1 (continuation)
Chronological resolution: 2.WAVE2-INCIDENT, 2.C1, 2.A1, INFRA-BACKUP-1
"""
import sys

PATH = "/home/z/my-project/worklog.md"
with open(PATH, "r", encoding="utf-8") as f:
    text = f.read()

start = text.index("<<<<<<< HEAD\n")
mid = text.index("=======\n", start)
end = text.index(">>>>>>> wip/2a1-input\n", mid)

pre = text[:start]
head = text[start + len("<<<<<<< HEAD\n"):mid]
branch = text[mid + len("=======\n"):end]
post = text[end + len(">>>>>>> wip/2a1-input\n"):]

# Split HEAD at the INFRA-BACKUP-1 header (insert 2.A1 before it)
anchor = "Task ID: INFRA-BACKUP-1 (Lead)"
idx = head.index(anchor)
# walk back to the '---' separator line that precedes the entry
sep = head.rindex("---", 0, idx)
head_part1 = head[:sep]
head_part2 = head[sep:]

resolved = pre + head_part1 + branch.rstrip("\n") + "\n\n" + head_part2 + post

with open(PATH, "w", encoding="utf-8") as f:
    f.write(resolved)

# sanity: no markers remain, all four entries present in order
assert "<<<<<<<" not in resolved and ">>>>>>>" not in resolved and "\n=======" not in resolved
order = [resolved.index("Task ID: 2.WAVE2-INCIDENT"), resolved.index("Task ID: 2.C1 (Lead close-out)"),
         resolved.index("Task ID: 2.A1 (continuation)"), resolved.index(anchor)]
assert order == sorted(order), f"entry order wrong: {order}"
print("resolved OK — entries in order: 2.WAVE2-INCIDENT, 2.C1, 2.A1, INFRA-BACKUP-1")
print(f"total lines: {len(resolved.splitlines())}")
