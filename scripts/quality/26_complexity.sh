#!/usr/bin/env bash
# T30 - quadratic-idiom gate: the "bad algorithm" check that can actually fail.
#
# WHAT THIS IS NOT: a complexity prover. No static tool can look at arbitrary
# Rust and tell you its O(). Anything claiming otherwise is lying.
#
# WHAT IT IS: engineering-directives D-1..D-5 demand "no naive-complexity
# algorithms, zero-allocation hot paths". That policy existed with zero
# enforcement. This enforces the part that is decidable by inspection:
#
#   HARD   idioms that are O(n) by construction and inside a loop become O(n^2):
#            .remove(0)   .insert(0, ...)   .nth(k)
#   REVIEW linear scans embedded in a loop, counted and ranked, not gated:
#            .contains(  .position(  .iter().any/find/filter(  .sort*  clone()
#
# The HARD count is RATCHETED against the COMMITTED
# docs/audit/complexity-baseline.json: a crate may not add an O(n) shift.
# A missing or unparseable baseline FAILS the gate (a ratchet with no floor is
# a pass-through — the audit's W17-A04 finding). Regeneration is deliberate,
# never automatic:
#   scripts/quality/26_complexity.sh --write-baseline
# Run that only after fixing what made the gate fail, never to silence an
# unexplained drift.
#
# Real complexity proof needs measurement (doubling input, fitting the growth
# exponent). That is T21/perf territory and cannot be faked from source.
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

mkdir -p "$OUT/complexity"
BASE="$ROOT/docs/audit/complexity-baseline.json"
OUTF="$OUT/complexity/complexity.txt"

# --write-baseline: the one sanctioned way to (re)generate the ratchet floor
# from the current tree. Without it, a missing/invalid baseline is a FAILURE.
WRITE_BASELINE=0
if [ "${1:-}" = "--write-baseline" ]; then WRITE_BASELINE=1; fi

head1 "26. quadratic-idiom gate"

# Gate semantics (fix F0.5c): the heredoc's exit code (1 = baseline missing/
# invalid, or a crate GREW) must become the script's exit code — 99_run_all.sh's
# JG fails the tier on it. Captured here, re-expressed at the single exit path
# at the bottom; the trailing `say`s used to swallow it (same class as the
# 23_alloc_budget.sh W17-A03 finding).
rc=0
$PY - "$ROOT" "$BASE" "$OUTF" "$WRITE_BASELINE" <<'PYEOF'
import json, os, re, sys
from datetime import datetime, timezone

root, base_path, out_path = sys.argv[1], sys.argv[2], sys.argv[3]
write_baseline = (sys.argv[4] == "1")

HARD = {
    "remove0":  re.compile(r"\.remove\(\s*0\s*\)"),
    "insert0":  re.compile(r"\.insert\(\s*0\s*,"),
    "drain0":   re.compile(r"\.drain\(\s*0?\s*\.\."),
    "nth":      re.compile(r"\.nth\("),
}
REVIEW = {
    "contains": re.compile(r"\.contains\("),
    "position": re.compile(r"\.position\("),
    "any":      re.compile(r"\.iter\(\)\.any\(|\.iter\(\)\.find\(|\.iter\(\)\.filter\("),
    "sort":     re.compile(r"\.sort(_unstable)?(_by|_by_key)?\("),
    "clone":    re.compile(r"\.clone\(\)"),
}
LOOP = re.compile(r"^\s*(for\s|while\s|loop\s*\{)")

SKIP_DIRS = {"tests", "benches", "examples", "target", "node_modules"}
SKIP_FILES = {"tests.rs", "test_util.rs"}


def scrub(text):
    """Strings first, then line comments, then block comments - order matters
    or a `/*` inside a string eats shipped code."""
    text = re.sub(r'r#*"(?:[^"]|"(?!#))*"#*', '""', text)
    text = re.sub(r'"(?:[^"\\]|\\.)*"', '""', text)
    text = re.sub(r"//[^\n]*", "", text)
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return text


def scan_file(path):
    hard, review = [], []
    try:
        txt = scrub(open(path, encoding="utf-8", errors="replace").read())
    except OSError:
        return hard, review
    depth, loops = 0, []
    for idx, line in enumerate(txt.splitlines(), 1):
        if LOOP.match(line) and "{" in line:
            loops.append(depth)
        for name, rx in HARD.items():
            if rx.search(line):
                hard.append((idx, name, line.strip()[:100]))
        if loops:  # only review-class when nested in a loop
            for name, rx in REVIEW.items():
                if rx.search(line):
                    review.append((idx, name, line.strip()[:100]))
        depth += line.count("{") - line.count("}")
        while loops and depth <= loops[-1]:
            loops.pop()
    return hard, review


hard_by_crate, review_by_crate, detail = {}, {}, []
for top in ("crates", "apps", "tools"):
    base = os.path.join(root, top)
    if not os.path.isdir(base):
        continue
    for member in sorted(os.listdir(base)):
        src = os.path.join(base, member, "src")
        if not os.path.isdir(src):
            continue
        key = "%s/%s" % (top, member)
        for dp, dns, fns in os.walk(src):
            dns[:] = [d for d in dns if d not in SKIP_DIRS]
            for fn in fns:
                if not fn.endswith(".rs") or fn in SKIP_FILES:
                    continue
                p = os.path.join(dp, fn)
                h, r = scan_file(p)
                if h or r:
                    hard_by_crate[key] = hard_by_crate.get(key, 0) + len(h)
                    review_by_crate[key] = review_by_crate.get(key, 0) + len(r)
                    rel = os.path.relpath(p, root).replace("\\", "/")
                    for i, n, s in h:
                        detail.append(("HARD", key, rel, i, n, s))

hard_total = sum(hard_by_crate.values())
review_total = sum(review_by_crate.values())

L = ["# Quadratic-idiom gate\n",
     "Policy: engineering-directives D-1..D-5 (\"no naive-complexity algorithms\").",
     "This is a *source-level* approximation. It cannot prove complexity; it",
     "catches idioms that are O(n) by construction and become O(n^2) in a loop.\n",
     "`HARD` is ratcheted (`docs/audit/complexity-baseline.json`). `REVIEW` is",
     "counted and ranked for triage, never gated.\n",
     "| crate | HARD (gated) | REVIEW (triaged) |", "|---|---|---|"]
for k in sorted(set(hard_by_crate) | set(review_by_crate),
                key=lambda k: -(hard_by_crate.get(k, 0) * 100 + review_by_crate.get(k, 0))):
    L.append("| %s | %d | %d |" % (k, hard_by_crate.get(k, 0), review_by_crate.get(k, 0)))
L.append("\n**HARD total: %d** | **REVIEW total: %d**\n" % (hard_total, review_total))
if detail:
    L.append("## HARD sites (an O(n) shift inside a loop is O(n^2))\n")
    for _, crate, rel, i, n, s in detail[:40]:
        L.append("- `%s:%d` **%s** - `%s`" % (rel, i, n, s))

baseline = None
if os.path.exists(base_path):
    try:
        baseline = json.load(open(base_path, encoding="utf-8"))
    except ValueError:
        baseline = None

rc = 0
if baseline is None:
    if write_baseline:
        json.dump({"generated": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
                   "policy": "engineering-directives D-1..D-5",
                   "hard_total": hard_total, "review_total": review_total,
                   "hard": hard_by_crate, "review": review_by_crate},
                  open(base_path, "w", encoding="utf-8"), indent=1)
        L.append("\nBaseline written: HARD=%d (deliberate --write-baseline run; "
                 "the ratchet floor is now this tree)." % hard_total)
    else:
        L.append("\n**GATE FAILED**: ratchet baseline %s is missing or invalid." % base_path)
        L.append("A ratchet without its committed floor is a pass-through — the")
        L.append("gate refuses to run green. Restore the baseline from history, or")
        L.append("regenerate deliberately AFTER fixing what made it fail:")
        L.append("  scripts/quality/26_complexity.sh --write-baseline")
        rc = 1
else:
    prior = baseline.get("hard", {})
    grew = [(k, prior.get(k, 0), hard_by_crate.get(k, 0))
            for k in hard_by_crate if hard_by_crate[k] > prior.get(k, 0)]
    L.append("\nBaseline HARD=%d -> now %d." % (baseline.get("hard_total", 0), hard_total))
    if grew:
        for k, b, n in grew:
            L.append("- **GREW** %s: %d -> %d" % (k, b, n))
        L.append("\n**GATE FAILED**: %d crate(s) added an O(n) shift. Move the "
                 "element, use VecDeque, or iterate without shifting." % len(grew))
        rc = 1
    else:
        json.dump({"generated": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
                   "policy": baseline.get("policy", ""),
                   "hard_total": hard_total, "review_total": review_total,
                   "hard": hard_by_crate, "review": review_by_crate},
                  open(base_path, "w", encoding="utf-8"), indent=1)
        L.append("**GATE OK** - no crate added an O(n) shift. Floor tightened to %d." % hard_total)

open(out_path, "w", encoding="utf-8").write("\n".join(L) + "\n")
print("\n".join(L[-10:]))
sys.exit(rc)
PYEOF
rc=$?

say ""
say "written: $OUTF"
exit "$rc"
