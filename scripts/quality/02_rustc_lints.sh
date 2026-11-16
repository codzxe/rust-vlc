#!/usr/bin/env bash
# T02 - rustc lint escalation: dead code, unreachable pub, unused deps, lifetime elision.
# WARNING: changing RUSTFLAGS invalidates the whole build cache. Run once per session.
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

LINTS=(
  unreachable_pub
  unused_crate_dependencies
  unused_qualifications
  unused_lifetimes
  elided_lifetimes_in_paths
  let_underscore_drop
  let_underscore_untyped
  trivial_numeric_casts
  unused_results
  missing_debug_implementations
  variant_size_differences
)
FLAGS=""
for l in "${LINTS[@]}"; do FLAGS="$FLAGS -W $l"; done

head1 "rustc lint escalation"
say "flags:$FLAGS"

RUSTFLAGS="$FLAGS" cargo check \
  --workspace --all-targets --all-features \
  --exclude platform-linux --exclude platform-macos \
  --message-format=json 2>/dev/null \
  > "$OUT/rustc-lints.jsonl"
rc=$?
say "exit=$rc  raw=$OUT/rustc-lints.jsonl"

head1 "histogram"
$PY - "$OUT/rustc-lints.jsonl" <<'PYEOF'
import json, sys, collections
counts = collections.Counter()
locs = collections.defaultdict(list)
for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
    line = line.strip()
    if not line.startswith("{"):
        continue
    try:
        m = json.loads(line)
    except Exception:
        continue
    if m.get("reason") != "compiler-message":
        continue
    d = m["message"]
    code = (d.get("code") or {}).get("code")
    if not code or d.get("level") not in ("warning", "error"):
        continue
    counts[code] += 1
    for s in d.get("spans", []):
        if s.get("is_primary"):
            locs[code].append("%s:%s" % (s["file_name"], s["line_start"]))
            break
for code, n in counts.most_common():
    print("%6d  %s" % (n, code))
print("--- sample locations ---")
for code, _ in counts.most_common(8):
    for loc in locs[code][:3]:
        print("  %-40s %s" % (code, loc))
PYEOF
