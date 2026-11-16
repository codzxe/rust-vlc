#!/usr/bin/env bash
# T14 - CRAP score: complexity x coverage. The "dangerous change" ranking.
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

need cargo-crap || exit 0

if [ ! -f "$OUT/lcov.info" ]; then
  not_run "cargo-crap" "no coverage data: run scripts/quality/11_coverage.sh first"
  exit 0
fi

head1 "14a. CRAP ranking (min score 30)"
cargo crap --lcov "$OUT/lcov.info" --min 30 --format json \
  > "$OUT/crap.json" 2> "$OUT/crap.err"
rc=$?
say "exit=$rc"
if [ $rc -ne 0 ] || [ ! -s "$OUT/crap.json" ]; then
  not_run "cargo-crap" "run failed: $(head -3 "$OUT/crap.err" | tr '\n' ' ')"
  exit 0
fi

head1 "14b. top 40 by CRAP score"
$PY - "$OUT/crap.json" <<'PYEOF'
import json, sys
d = json.load(open(sys.argv[1], encoding="utf-8"))
items = d if isinstance(d, list) else d.get("functions", d.get("data", []))
rows = []
for it in items:
    if not isinstance(it, dict):
        continue
    rows.append((it.get("crap") or it.get("crap_score") or 0,
                 it.get("complexity", 0),
                 it.get("coverage", it.get("coverage_percent", 0)),
                 it.get("name", "?"),
                 (it.get("file") or it.get("path") or "?")))
rows.sort(reverse=True)
print("%8s %6s %7s  %s" % ("CRAP", "CX", "COV%", "function @ file"))
for crap, cx, cov, name, f in rows[:40]:
    print("%8.1f %6s %7s  %s  (%s)" % (crap, cx, cov, name, f.replace("\\", "/")))
PYEOF

head1 "note"
cat <<'TXT'
CRAP = complexity^2 * (1 - coverage)^3 + complexity

The cubic coverage term is the point: a complex function with no tests scores
in the thousands. Those are the functions where a change is most likely to ship
a bug, and they are the priority list for property tests (T17) and Kani (T20).
TXT
