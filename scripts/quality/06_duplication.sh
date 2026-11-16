#!/usr/bin/env bash
# T06 - duplication census (jscpd; Rust tokenizer).
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

head1 "jscpd"
if ! command -v npx >/dev/null 2>&1; then
  not_run "jscpd" "npx/node not installed"
  exit 0
fi

mkdir -p "$OUT/jscpd"
npx --yes jscpd@4 \
  --format rust \
  --min-tokens 50 \
  --reporters json \
  --output "$OUT/jscpd" \
  --ignore "**/target/**" \
  crates apps tools tests 2>&1 | tail -20

if [ -f "$OUT/jscpd/jscpd-report.json" ]; then
  $PY - "$OUT/jscpd/jscpd-report.json" <<'PYEOF'
import json, sys, collections
d = json.load(open(sys.argv[1], encoding="utf-8"))
st = d.get("statistics", {})
tot = st.get("total", {})
print("files:       %s" % tot.get("sources"))
print("lines:       %s" % tot.get("lines"))
print("clones:      %s" % st.get("total", {}).get("clones"))
pct = tot.get("percentage")
print("duplication: %s%%" % pct)
dups = d.get("duplicates", [])
clusters = collections.Counter()
for dup in dups:
    for f in dup.get("firstFile", {}), dup.get("secondFile", {}):
        clusters[f.get("name", "?").replace("\\", "/")] += dup.get("tokens", 0)
print("\ntop files by duplicated tokens:")
for name, toks in clusters.most_common(15):
    print("  %6d  %s" % (toks, name))
PYEOF
else
  not_run "jscpd" "report not produced"
fi
