#!/usr/bin/env bash
# T03 - structural anti-pattern scan (ast-grep rules in scripts/quality/rules/).
# Usage: 03_astgrep.sh [rule-id ...]
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

RULES="$ROOT/scripts/quality/rules"
OUTD="$OUT/astgrep"
mkdir -p "$OUTD"

if [ "$#" -gt 0 ]; then
  FILES=()
  for id in "$@"; do FILES+=("$RULES/$id.yml"); done
else
  FILES=("$RULES"/*.yml)
fi

PATHS=(crates apps tools tests)
[ -d plugins ] && PATHS+=(plugins)

for f in "${FILES[@]}"; do
  id="$(basename "$f" .yml)"
  [ -f "$f" ] || { not_run "ast-grep:$id" "rule file missing: $f"; continue; }
  sg_run scan --rule "$f" --json "${PATHS[@]}" > "$OUTD/$id.json" 2> "$OUTD/$id.err"
  rc=$?
  if [ $rc -ne 0 ] && [ ! -s "$OUTD/$id.json" ]; then
    not_run "ast-grep:$id" "scan failed rc=$rc: $(head -2 "$OUTD/$id.err" | tr '\n' ' ')"
    continue
  fi
  n=$($PY -c "import json,sys;print(len(json.load(open(sys.argv[1],encoding='utf-8'))))" "$OUTD/$id.json" 2>/dev/null || echo "?")
  say "rule=$id hits=$n"
done

head1 "summary"
$PY - "$OUTD" <<'PYEOF'
import json, glob, os, sys, collections
outd = sys.argv[1]
total = 0
for f in sorted(glob.glob(os.path.join(outd, "*.json"))):
    rid = os.path.basename(f)[:-5]
    try:
        hits = json.load(open(f, encoding="utf-8"))
    except Exception as e:
        print("%-22s PARSE ERROR %s" % (rid, e)); continue
    if not isinstance(hits, list):
        continue
    total += len(hits)
    files = collections.Counter(h.get("file", "?").replace("\\", "/") for h in hits)
    print("%-22s %5d hits  top: %s" % (rid, len(hits),
          ", ".join("%s(%d)" % (k, v) for k, v in files.most_common(3))))
print("TOTAL ast-grep hits: %d" % total)
PYEOF

say ""
say "raw: $OUTD/*.json"
