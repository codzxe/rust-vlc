#!/usr/bin/env python3
"""astgrep_summary.py - classify ast-grep hits into production vs test code.

The raw rule counts overstate the problem: `unreachable!` inside a test helper is
idiomatic, `assert!` inside `tests/` is the point of tests. What matters is the
count in SHIPPED code. This script makes that split explicit.
"""
import json
import os
import sys
from collections import Counter, defaultdict

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
D = os.path.join(ROOT, "docs", "audit", "reports", "astgrep")

TEST_PATH_MARKERS = (
    "/tests/", "/tests.rs", "/test/", "/benches/", "tests.rs", "_test.rs",
    "testkit/", "tests/common/",
)


def is_test_path(path: str) -> bool:
    p = "/" + path.replace("\\", "/").lstrip("/")
    return any(m in p for m in TEST_PATH_MARKERS) or p.endswith("/tests.rs")


def main():
    out = {}
    rows = []
    for fn in sorted(os.listdir(D)):
        if not fn.endswith(".json") or fn == "summary.json":
            continue
        rid = fn[:-5]
        try:
            hits = json.load(open(os.path.join(D, fn), encoding="utf-8-sig"))
        except Exception:
            continue
        if not isinstance(hits, list):
            continue
        prod = [h for h in hits if not is_test_path(h.get("file", ""))]
        test = [h for h in hits if is_test_path(h.get("file", ""))]
        out[rid] = {"total": len(hits), "production": len(prod), "test": len(test)}
        for h in prod:
            rows.append({
                "rule": rid,
                "file": h["file"].replace("\\", "/"),
                "line": h["range"]["start"]["line"],
                "text": h["lines"].strip()[:160],
            })

    print("%-24s %7s %7s %6s" % ("rule", "total", "SHIPPED", "tests"))
    print("-" * 50)
    for rid, v in sorted(out.items(), key=lambda kv: -kv[1]["production"]):
        print("%-24s %7d %7d %6d" % (rid, v["total"], v["production"], v["test"]))
    print()

    print("top shipped-code files per rule (these are the real triage queue):")
    by_rule = defaultdict(Counter)
    for r in rows:
        by_rule[r["rule"]][r["file"]] += 1
    for rid in ("todo-macros", "panic-in-lib", "ignored-result", "unwrap-outside-tests",
                "no-op-test", "process-exit", "allow-attr"):
        if rid not in by_rule:
            continue
        print("\n-- %s" % rid)
        for f, n in by_rule[rid].most_common(8):
            print("   %4d  %s" % (n, f))

    os.makedirs(os.path.join(ROOT, "docs/audit/reports"), exist_ok=True)
    with open(os.path.join(ROOT, "docs/audit/reports/astgrep-summary.json"), "w", encoding="utf-8") as fh:
        json.dump({"counts": out, "production_hits": rows}, fh, indent=1)


if __name__ == "__main__":
    main()
