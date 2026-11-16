#!/usr/bin/env bash
# T24 - panic budget RATCHET (DoS surface must never grow).
#
# 1,827 shipped `unreachable!` in a media engine are not style debt, they are
# 1,827 "attacker sends bytes -> process aborts" sites. You cannot fix them all
# today, but you can stop the number from growing: this script records the
# count per crate in docs/audit/panic-baseline.json and FAILS any change that
# increases it. A ratchet, not a rewrite.
#
# Method: brace-matched `#[cfg(test)]` regions are excluded (same technique as
# test_hygiene.py), along with tests/, benches/, examples/.
#
# Usage: 24_panic_budget.sh [--update]     (--update re-seeds the floor)
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

mkdir -p "$OUT/panic"
BASE="$ROOT/docs/audit/panic-baseline.json"
MODE="${1:-check}"

head1 "24. panic budget ratchet ($MODE)"

$PY - "$ROOT" "$BASE" "$MODE" "$OUT/panic/panic-budget.txt" <<'PYEOF'
import json, os, re, sys
from datetime import datetime, timezone

root, base_path, mode, out_path = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]

PATTERNS = {
    "unreachable": re.compile(r"\bunreachable!\s*\("),
    "panic":       re.compile(r"\bpanic!\s*\("),
    "unwrap":      re.compile(r"\.unwrap\s*\(\s*\)"),
    "expect":      re.compile(r"\.expect\s*\("),
    "todo":        re.compile(r"\b(?:todo|unimplemented)!\s*\("),
}
SKIP_DIRS = {"tests", "benches", "examples", "target", "node_modules"}
SKIP_FILES = {"tests.rs", "test_util.rs"}


def strip_test_regions(text):
    """Remove `#[cfg(test)] mod ... { ... }` blocks via brace matching.

    Only the module form is stripped. A bare `#[cfg(test)]` on a single item
    (e.g. `#[cfg(test)] use foo::Bar;`) is left alone: its next `{` may belong
    to unrelated code, and deleting from there would silently swallow shipped
    code - which is exactly the bug this guard exists to prevent.
    """
    marker = "#[cfg(test)]"
    mod_rx = re.compile(
        r"#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{")
    out, i = [], 0
    while True:
        m = re.search(r"#\[cfg\(test\)\]", text[i:])
        if not m:
            out.append(text[i:])
            break
        start = i + m.start()
        mm = mod_rx.match(text, start)
        if not mm:
            out.append(text[i:start + len(marker)])
            i = start + len(marker)
            continue
        brace = mm.end() - 1
        depth, j = 0, brace
        while j < len(text):
            if text[j] == "{":
                depth += 1
            elif text[j] == "}":
                depth -= 1
                if depth == 0:
                    j += 1
                    break
            j += 1
        out.append(text[i:start])
        i = j
    return "".join(out)


def strip_comments_and_strings(text):
    text = re.sub(r"//[^\n]*", "", text)
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    text = re.sub(r'"(?:[^"\\]|\\.)*"', '""', text)
    text = re.sub(r"r#*\"(?:[^\"]|\"(?!#))*\"#*", '""', text)
    return text


def scrub(text):
    """Remove string literals, then line comments, then block comments.

    ORDER MATTERS: block comments must be removed only after strings are gone,
    otherwise a `/*` inside a string swallows shipped code up to the next `*/`
    (this bug silently deleted 23 `unreachable!` from input/src/contract.rs).
    """
    text = re.sub(r'r#*"(?:[^"]|"(?!#))*"#*', '""', text)   # raw strings
    text = re.sub(r'"(?:[^"\\]|\\.)*"', '""', text)          # normal strings
    text = re.sub(r"//[^\n]*", "", text)                     # line comments
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)        # block comments
    return text


def scan():
    counts = {}
    for top in ("crates", "apps", "tools"):
        base = os.path.join(root, top)
        if not os.path.isdir(base):
            continue
        for member in sorted(os.listdir(base)):
            src = os.path.join(base, member, "src")
            if not os.path.isdir(src):
                continue
            c = dict.fromkeys(PATTERNS, 0)
            for dp, dns, fns in os.walk(src):
                dns[:] = [d for d in dns if d not in SKIP_DIRS]
                for fn in fns:
                    if not fn.endswith(".rs") or fn in SKIP_FILES:
                        continue
                    try:
                        txt = open(os.path.join(dp, fn), encoding="utf-8", errors="replace").read()
                    except OSError:
                        continue
                    body = strip_test_regions(scrub(txt))
                    for k, rx in PATTERNS.items():
                        c[k] += len(rx.findall(body))
            if any(c.values()):
                counts["%s/%s" % (top, member)] = c
    return counts


def total(c):
    return sum(c.values())


current = scan()
grand = sum(total(c) for c in current.values())

lines = ["# Panic budget ratchet\n"]
lines.append("Shipped panic primitives (test regions excluded). Every one is a")
lines.append("process-abort reachable from input if the guarding invariant breaks.\n")
lines.append("`unreachable!` | `panic!` | `.unwrap()` | `.expect(` | `todo!`\n")
lines.append("| crate | unreachable | panic | unwrap | expect | todo | total |")
lines.append("|---|---|---|---|---|---|---|")
for name, c in sorted(current.items(), key=lambda kv: -total(kv[1])):
    lines.append("| %s | %d | %d | %d | %d | %d | **%d** |" % (
        name, c["unreachable"], c["panic"], c["unwrap"], c["expect"], c["todo"], total(c)))
lines.append("\n**grand total: %d shipped panic sites**\n" % grand)

baseline = None
if os.path.exists(base_path):
    try:
        baseline = json.load(open(base_path, encoding="utf-8"))
    except ValueError:
        baseline = None

if baseline is None or mode == "--update":
    json.dump({"generated": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
               "method": "brace-matched #[cfg(test)] excluded; tests/benches/examples skipped",
               "grand_total": grand, "crates": current},
              open(base_path, "w", encoding="utf-8"), indent=1)
    lines.append("Baseline %s: grand total %d recorded in `docs/audit/panic-baseline.json`."
                 % ("re-seeded" if baseline else "created", grand))
    rc = 0
else:
    prior = baseline.get("crates", {})
    grew, shrank = [], []
    for name, c in current.items():
        before, now = total(prior.get(name, {})), total(c)
        if now > before:
            grew.append((name, before, now))
        elif now < before:
            shrank.append((name, before, now))
    gone = [n for n in prior if n not in current]
    lines.append("Baseline grand total: %d -> now %d.\n" % (baseline.get("grand_total", 0), grand))
    for name, b, n in grew:
        lines.append("- **GREW** %s: %d -> %d (+%d)" % (name, b, n, n - b))
    for name, b, n in shrank:
        lines.append("- shrank %s: %d -> %d" % (name, b, n))
    for name in gone:
        lines.append("- gone %s (0 sites now)" % name)
    if grew:
        lines.append("\n**RATCHET FAILED**: %d crate(s) added panic sites. A panic on hostile"
                     " input is a denial of service - return a typed error instead."
                     % len(grew))
        rc = 1
    else:
        # tighten the floor: never let a win silently rot back
        merged = dict(prior)
        for name, c in current.items():
            merged[name] = c
        json.dump({"generated": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
                   "method": baseline.get("method", ""),
                   "grand_total": grand, "crates": merged},
                  open(base_path, "w", encoding="utf-8"), indent=1)
        lines.append("**RATCHET OK** - no crate grew. Floor tightened to %d." % grand)
        rc = 0

open(out_path, "w", encoding="utf-8").write("\n".join(lines) + "\n")
print("\n".join(lines[-12:]))
sys.exit(rc)
PYEOF

say ""
say "written: $OUT/panic/panic-budget.txt"
