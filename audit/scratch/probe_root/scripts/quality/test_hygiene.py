#!/usr/bin/env python3
"""test_hygiene.py - structural test-quality census (T13).

Not a heuristic regex over raw text: comments and string literals are stripped
first (they contain the word #[test] in doc comments), then every #[test] body
is brace-matched and inspected for a real failure mechanism.

Outputs docs/audit/reports/test-hygiene.json and a human summary on stdout.
"""
import json
import os
import re
import sys
from collections import Counter, defaultdict

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))

ASSERT_RE = re.compile(
    r"\b(assert|assert_eq|assert_ne|debug_assert|debug_assert_eq|panic|unreachable|"
    r"require|matches)\s*[!(]|"
    r"\binsta::|trybuild|assert_cmd|\.must_|\.must\(|\.expect_err\(|"
    r"assert_matches"
)
TEST_ATTR = re.compile(r"#\[\s*(?:tokio::)?test\s*\]")
IGNORE_ATTR = re.compile(r"#\[\s*ignore(\s*=\s*\"([^\"]*)\")?\s*\]")
SP_ATTR = re.compile(r"#\[\s*should_panic(\s*\(\s*expected\s*=\s*\"([^\"]*)\"\s*\))?\s*\]")
SLEEP_RE = re.compile(r"\b(?:std::)?thread::sleep\s*\(|tokio::time::sleep\s*\(")


def strip_code(src: str) -> str:
    """Remove comments and string/char literal *contents*, preserving offsets."""
    out = []
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            j = src.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif c == "/" and i + 1 < n and src[i + 1] == "*":
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth += 1
                    j += 2
                elif src.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            chunk = src[i:j]
            out.append("".join("\n" if ch == "\n" else " " for ch in chunk))
            i = j
        elif c == '"':
            j = i + 1
            while j < n:
                if src[j] == "\\":
                    j += 2
                    continue
                if src[j] == '"':
                    j += 1
                    break
                j += 1
            chunk = src[i:j]
            out.append('"' + "".join("\n" if ch == "\n" else " " for ch in chunk[1:-1]) + '"')
            i = j
        elif c == "'":
            # char literal or lifetime; only strip a real char literal
            m = re.match(r"'(?:\\.|[^\\'])'", src[i:])
            if m:
                out.append("' '")
                i += m.end()
            else:
                out.append(c)
                i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def body_of(src: str, start: int):
    """Given an offset just after a `fn ...` signature, return (body, end) by brace matching."""
    brace = src.find("{", start)
    if brace < 0:
        return None, start
    depth = 0
    i = brace
    while i < len(src):
        if src[i] == "{":
            depth += 1
        elif src[i] == "}":
            depth -= 1
            if depth == 0:
                return src[brace:i + 1], i + 1
        i += 1
    return src[brace:], len(src)


def main():
    counts = Counter()
    rows = []          # (file, line, name, category)
    walk_skip = {".git", "target", "node_modules", "download", "fuzz"}
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in walk_skip]
        for fn in filenames:
            if not fn.endswith(".rs"):
                continue
            p = os.path.join(dirpath, fn)
            try:
                raw = open(p, encoding="utf-8", errors="replace").read()
            except Exception:
                continue
            src = strip_code(raw)
            rel = os.path.relpath(p, ROOT).replace("\\", "/")

            for m in IGNORE_ATTR.finditer(src):
                counts["ignore"] += 1
                if not m.group(2):
                    counts["ignore_no_reason"] += 1
                    rows.append({"file": rel, "line": src[:m.start()].count("\n") + 1,
                                 "test": "<ignore>", "category": "ignore_without_reason"})
            for m in SP_ATTR.finditer(src):
                counts["should_panic"] += 1
                if not m.group(2):
                    counts["should_panic_no_expected"] += 1
                    rows.append({"file": rel, "line": src[:m.start()].count("\n") + 1,
                                 "test": "<should_panic>", "category": "should_panic_without_expected"})

            for m in TEST_ATTR.finditer(src):
                counts["tests"] += 1
                line = src[:m.start()].count("\n") + 1
                fm = re.search(r"\bfn\s+([A-Za-z0-9_]+)", src[m.end():m.end() + 400])
                name = fm.group(1) if fm else "<anonymous>"
                body, _ = body_of(src, m.end())
                if body is None:
                    rows.append({"file": rel, "line": line, "test": name, "category": "unparsable_test"})
                    counts["unparsable"] += 1
                    continue
                # an immediately-preceding #[should_panic] is a failure mechanism
                prefix = src[max(0, m.start() - 200):m.start()]
                has_failure = bool(ASSERT_RE.search(body)) or "should_panic" in prefix
                if not has_failure:
                    counts["assertion_free"] += 1
                    rows.append({"file": rel, "line": line, "test": name, "category": "assertion_free"})
                if SLEEP_RE.search(body):
                    counts["sleeps"] += 1
                    rows.append({"file": rel, "line": line, "test": name, "category": "sleep_in_test"})
                if len(body) < 20:
                    counts["empty_body"] += 1
                    rows.append({"file": rel, "line": line, "test": name, "category": "empty_test_body"})

    out = {"counts": dict(counts), "findings": rows}
    os.makedirs(os.path.join(ROOT, "..", "..", "..", "..", "audit", "scratch"), exist_ok=True)
    with open(os.path.join(ROOT, "..", "..", "..", "..", "audit", "scratch", "probe-test-hygiene.json"), "w", encoding="utf-8") as fh:
        json.dump(out, fh, indent=1)

    print("tests found:                     %d" % counts["tests"])
    print("  assertion-free:                %d  <-- candidate fake tests" % counts["assertion_free"])
    print("  with thread::sleep:            %d  <-- flaky by construction" % counts["sleeps"])
    print("  trivially short body:          %d" % counts["empty_body"])
    print("ignored tests:                   %d (no reason: %d)" % (counts["ignore"], counts["ignore_no_reason"]))
    print("should_panic:                    %d (no expected=: %d)" % (counts["should_panic"], counts["should_panic_no_expected"]))
    print()
    by_cat = defaultdict(list)
    for r in rows:
        by_cat[r["category"]].append(r)
    for cat in ("assertion_free", "sleep_in_test", "ignore_without_reason",
                "should_panic_without_expected", "empty_test_body", "unparsable_test"):
        items = by_cat.get(cat, [])
        if not items:
            continue
        per_crate = Counter(i["file"].split("/")[1] if i["file"].startswith(("crates/", "apps/", "tools/")) else i["file"].split("/")[0] for i in items)
        print("-- %s: %d  (%s)" % (cat, len(items),
              ", ".join("%s:%d" % (k, v) for k, v in per_crate.most_common(6))))
        for i in items[:15]:
            print("     %s:%s  %s" % (i["file"], i["line"], i["test"]))
        print()


if __name__ == "__main__":
    main()
