#!/usr/bin/env python3
"""mutants_report.py - turn a cargo-mutants run into a score + survivor list.

Usage:
    python scripts/quality/mutants_report.py <dir or outcomes.json> [...]

Reads cargo-mutants 27.x `outcomes.json` (schema: {outcomes: [...], total_mutants,
caught, missed, timeout, unviable}) and prints:

    mutation score = caught / (caught + missed + timeout)

plus every MISSED mutant - each is a line of production code whose behaviour no
test constrains. Unviable mutants are excluded from the denominator because they
never compiled (neutral, not evidence either way).

Always exits 0: this is a report, not a gate.
"""
import json
import os
import sys
from collections import Counter

GOOD = {"CaughtMutant"}
BAD = {"MissedMutant"}
NEUTRAL = {"Unviable", "Success"}


def find_outcomes(path):
    if os.path.isdir(path):
        for cand in (os.path.join(path, "outcomes.json"),
                     os.path.join(path, "mutants.out", "outcomes.json")):
            if os.path.exists(cand):
                return cand
    elif os.path.exists(path):
        return path
    raise SystemExit("no outcomes.json at %s" % path)


def scenario_fields(o):
    sc = o.get("scenario")
    if isinstance(sc, dict) and "Mutant" in sc:
        sc = sc["Mutant"]
    if not isinstance(sc, dict):
        return ("?", "?", "?", "?")
    fn = sc.get("function") or {}
    name = fn.get("function_name") if isinstance(fn, dict) else "?"
    span = sc.get("span") or {}
    start = (span.get("start") or {})
    return (sc.get("file", "?"), start.get("line", "?"), name or "?", sc.get("replacement", "?"))


def main(paths):
    grand = Counter()
    for arg in paths:
        data = json.load(open(find_outcomes(arg), encoding="utf-8"))
        outcomes = data.get("outcomes", [])
        c = Counter()
        survivors = []
        for o in outcomes:
            s = o.get("summary", "?")
            if s in GOOD:
                c["caught"] += 1
            elif s in BAD:
                c["missed"] += 1
                survivors.append(o)
            elif s == "Timeout":
                c["timeout"] += 1
                survivors.append(o)
            elif s == "Success":
                c["baseline"] += 1
            elif s in NEUTRAL:
                c["unviable"] += 1
            else:
                c["other"] += 1
        denom = c["caught"] + c["missed"] + c["timeout"]
        score = (100.0 * c["caught"] / denom) if denom else 0.0
        print("=" * 78)
        print("package : %s" % os.path.basename(os.path.dirname(find_outcomes(arg))) or arg)
        print("mutants : %d total (%.0fs wall)" % (
            data.get("total_mutants", len(outcomes)),
            _wall(data.get("start_time"), data.get("end_time"))))
        print("  caught    (test noticed)    : %d" % c["caught"])
        print("  MISSED    (nothing noticed) : %d   <-- weak/fake test or dead code" % c["missed"])
        print("  timeout   (mutant hung)     : %d   <-- unbounded loop candidate" % c["timeout"])
        print("  unviable  (does not compile): %d   (excluded from score)" % c["unviable"])
        print("  MUTATION SCORE              : %.1f%%  (%d/%d)" % (score, c["caught"], denom))
        grand.update(c)
        if survivors:
            print("\n  survivors:")
            for o in survivors:
                f, ln, name, repl = scenario_fields(o)
                print("    %s:%s  %s  ->  %s" % (f, ln, name, str(repl)[:70]))
    denom = grand["caught"] + grand["missed"] + grand["timeout"]
    if len(paths) > 1 and denom:
        print("=" * 78)
        print("GRAND: caught=%d missed=%d timeout=%d unviable=%d -> %.1f%%" % (
            grand["caught"], grand["missed"], grand["timeout"], grand["unviable"],
            100.0 * grand["caught"] / denom))
    print("\nbands: >=80%% trustworthy | 60-79%% spotty | <60%% decorative")


def _wall(a, b):
    try:
        from datetime import datetime
        f = lambda s: datetime.fromisoformat(s.replace("Z", "+00:00"))
        return (f(b) - f(a)).total_seconds()
    except Exception:
        return 0


if __name__ == "__main__":
    main(sys.argv[1:] or ["docs/audit/reports/mutants/codec-api"])
