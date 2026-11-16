#!/usr/bin/env bash
# T31 - speed-parity gate vs ffmpeg, across EVERY corpus in the tree.
#
# The harness already existed (`tools/corpus/speed_bench.py` + `lumen
# decode-bench`) and already computes `ratio vs ffmpeg`. It never had teeth and
# never had a memory of yesterday. This wrapper adds:
#
#   1. COVERAGE: it runs every fixture directory that contains media, not one
#      hand-picked corpus - "check all formats, not one".
#   2. BASELINE: docs/audit/speed-baseline.json remembers the ratio per class,
#      so a regression is detectable even while we are still slower than ffmpeg.
#   3. GATES: fail on >SPEED_REGRESS (20%) regression, and optionally on a
#      parity floor ("must be within X of ffmpeg").
#
# Ratio convention (speed_bench.py): ratio = ffmpeg_time / lumen_time.
#   1.00x = same speed. 0.34x = we take ~2.9x longer. >1.0x = we are faster.
#
# Usage: 27_speed_parity.sh [corpus-dir ...] [--count N] [--parity X]
# Env:   LUMEN_CMD (default: release build), SPEED_REGRESS (0.20), FFMPEG
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

mkdir -p "$OUT/speed"
BASE="$ROOT/docs/audit/speed-baseline.json"
LOG="$OUT/speed/speed.log"
COUNT=24; PARITY="0.0"; CORPORA=()
while [ "$#" -gt 0 ]; do
  case "$1" in
    --count)  COUNT="$2"; shift 2 ;;
    --parity) PARITY="$2"; shift 2 ;;
    -*)       shift ;;
    *)        CORPORA+=("$1"); shift ;;
  esac
done
REGRESS="${SPEED_REGRESS:-0.20}"
LUMEN_CMD="${LUMEN_CMD:-cargo run -q --release -p lumen-cli -- decode-bench}"

head1 "27. speed parity vs ffmpeg (count=$COUNT, regress=$REGRESS)"
: > "$LOG"

if ! need ffmpeg; then
  not_run "speed-parity" "ffmpeg not installed (CI-only reference decoder; needs Lead approval per AGENTS.md rule 2)"
  exit 0
fi
if ! need "$PY"; then exit 0; fi

# ---- discover every fixture directory that actually holds media -----------
if [ "${#CORPORA[@]}" -eq 0 ]; then
  mapfile -t CORPORA < <($PY - "$ROOT" <<'PYEOF' | tr -d '\r'
import os, sys
root = sys.argv[1]
EXT = {".mp4",".mkv",".ts",".ps",".avi",".ogg",".ogv",".wav",".aiff",".aif",
       ".aac",".ivf",".flac",".mp3",".mp2",".m4a",".mov",".webm"}
dirs = set()
for dp, dns, fns in os.walk(root):
    dns[:] = [d for d in dns if d not in ("target","node_modules",".git")]
    for fn in fns:
        if os.path.splitext(fn)[1].lower() in EXT:
            dirs.add(os.path.relpath(dp, root).replace("\\","/"))
            break
for d in sorted(dirs):
    print(d)
PYEOF
  )
fi
if [ "${#CORPORA[@]}" -eq 0 ]; then
  not_run "speed-parity" "no fixture directories with media found"
  exit 0
fi
say "corpora discovered: ${#CORPORA[@]}"
printf '  %s\n' "${CORPORA[@]}"

JSONS=()
i=0
for c in "${CORPORA[@]}"; do
  i=$((i + 1))
  j="$OUT/speed/speed-$i.json"
  say ""
  say "--- [$i/${#CORPORA[@]}] $c"
  $PY "$ROOT/tools/corpus/speed_bench.py" \
      --corpus "$c" --count "$COUNT" --repeat 3 \
      --runner "lumen=$LUMEN_CMD {file}" \
      --json "$j" --md "$OUT/speed/speed-$i.md" >> "$LOG" 2>&1
  rc=$?
  if [ -s "$j" ]; then
    JSONS+=("$j")
    say "  measured (exit=$rc)"
  else
    say "  no result (exit=$rc) - see $LOG"
  fi
done

if [ "${#JSONS[@]}" -eq 0 ]; then
  not_run "speed-parity" "no corpus produced a JSON result; see $LOG"
  exit 0
fi

$PY - "$BASE" "$PARITY" "$REGRESS" "$OUT/speed/parity.txt" "${JSONS[@]}" <<'PYEOF'
import json, statistics, sys
from datetime import datetime, timezone

base_path, parity, regress, out_path = sys.argv[1], float(sys.argv[2]), float(sys.argv[3]), sys.argv[4]
json_paths = sys.argv[5:]

agg = {}
for jp in json_paths:
    try:
        doc = json.load(open(jp, encoding="utf-8"))
    except (OSError, ValueError):
        continue
    corpus = str(doc.get("corpus", jp)).replace("\\", "/").strip("/")
    tag = "/".join(corpus.split("/")[-2:]) or corpus
    for rec in doc.get("per_file", []):
        cls = "%s :: %s" % (tag, rec.get("class", "?"))
        row = agg.setdefault(cls, {"ffmpeg": [], "lumen": []})
        for name in ("ffmpeg", "lumen"):
            r = rec.get(name)
            if isinstance(r, dict) and r.get("ok") and r.get("wall_best"):
                row[name].append(r["wall_best"])

results, dnf = {}, []
for cls, row in agg.items():
    if not row["ffmpeg"] or not row["lumen"]:
        dnf.append(cls)
        continue
    ff, lu = statistics.median(row["ffmpeg"]), statistics.median(row["lumen"])
    if lu <= 0:
        continue
    results[cls] = {"ratio": ff / lu, "n": len(row["lumen"]),
                    "ff_med_s": ff, "lumen_med_s": lu,
                    "ff_n": len(row["ffmpeg"])}

L = ["# Speed parity vs ffmpeg - all corpora\n"]
L.append("ratio = ffmpeg_time / lumen_time. **1.00x = parity**, <1 = slower, >1 = faster.")
L.append("Gate: regression >%.0f%% below baseline%s.\n" % (
    regress * 100, (", parity floor %.2fx" % parity) if parity > 0 else ""))
L.append("| corpus::class | n | ffmpeg med | lumen med | ratio | distance | status |")
L.append("|---|---|---|---|---|---|---|")

baseline = None
try:
    baseline = json.load(open(base_path, encoding="utf-8"))
except (OSError, ValueError):
    baseline = None
prior = (baseline or {}).get("classes", {})

fails = []
for cls in sorted(results, key=lambda c: results[c]["ratio"]):
    r = results[cls]
    ratio = r["ratio"]
    dist = ("%.2fx slower" % (1 / ratio)) if ratio < 1 else ("%.2fx faster" % ratio)
    status = "ok"
    b = prior.get(cls, {}).get("ratio")
    if b and ratio < b * (1 - regress):
        status = "REGRESSED %.2f->%.2f" % (b, ratio)
        fails.append("regressed in " + cls)
    if parity > 0 and ratio < parity:
        status = "BELOW PARITY" if status == "ok" else status + " + BELOW PARITY"
        fails.append("below parity in " + cls)
    L.append("| %s | %d | %.1f ms | %.1f ms | **%.2fx** | %s | %s |" % (
        cls, r["n"], r["ff_med_s"] * 1000, r["lumen_med_s"] * 1000, ratio, dist, status))

L.append("")
if dnf:
    L.append("**Decoded by ffmpeg, not by us (correctness gap, not speed):** %d class(es)" % len(dnf))
    for c in sorted(dnf)[:20]:
        L.append("- " + c)
    L.append("")

if not results:
    L.append("**Nothing comparable.** Every lumen run failed or decoded nothing.")
elif baseline is None:
    json.dump({"generated": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
               "convention": "ratio = ffmpeg_time / lumen_time; 1.0 = parity",
               "classes": results},
              open(base_path, "w", encoding="utf-8"), indent=1)
    L.append("Baseline created from this run. A >%.0f%% regression now fails." % (regress * 100))
elif fails:
    L.append("**GATE FAILED**")
    for f in fails:
        L.append("- " + f)
else:
    merged = dict(prior)
    for cls, r in results.items():
        if cls not in merged or r["ratio"] > merged[cls].get("ratio", 0):
            merged[cls] = r
    json.dump({"generated": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
               "convention": baseline.get("convention", ""), "classes": merged},
              open(base_path, "w", encoding="utf-8"), indent=1)
    L.append("**GATE OK** - no class regressed; floor raised where we got faster.")

open(out_path, "w", encoding="utf-8").write("\n".join(L) + "\n")
print("\n".join(L))
sys.exit(1 if fails else 0)
PYEOF

rc=$?
say ""
say "written: $OUT/speed/parity.txt"
exit "$rc"
