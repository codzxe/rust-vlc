#!/usr/bin/env python3
"""Resolve worklog.md append-append merge conflicts by keeping BOTH sides in order (HEAD first, then branch)."""
import sys

path = '/home/z/my-project/worklog.md'
lines = open(path).readlines()
start = next(i for i, l in enumerate(lines) if l.startswith('<<<<<<<'))
mid = next(i for i, l in enumerate(lines) if l.startswith('=======') and start < i)
end = next(i for i, l in enumerate(lines) if l.startswith('>>>>>>>') and i > mid)
# HEAD side, then a blank separator if needed, then branch side
head_side = lines[start + 1:mid]
theirs = lines[mid + 1:end]
sep = ['\n'] if (head_side and not head_side[-1].strip()) or not head_side else []
resolved = lines[:start] + head_side + sep + theirs + lines[end + 1:]
open(path, 'w').writelines(resolved)
n = sum(1 for l in open(path) if l.startswith('<<<<<<<') or l.startswith('>>>>>>>'))
print(f"resolved; remaining markers: {n}")
sys.exit(0 if n == 0 else 1)
