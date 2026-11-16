#!/usr/bin/env bash
# T11 - coverage per crate (llvm-cov). Produces LCOV for T14 (CRAP).
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

need cargo-llvm-cov || exit 0
rustup component add llvm-tools-preview >/dev/null 2>&1

# Crates whose test targets cannot currently compile are excluded and RECORDED.
# F-001: platform-linux/platform-macos test targets are not cfg-gated.
# F-002: crates/input/tests/file_stream.rs has a denied unused import.
EXCLUDES=(--exclude platform-linux --exclude platform-macos --exclude input)
not_run "coverage:input" "crates/input test target does not compile (F-002) - excluded from coverage, not scored as 0%"
not_run "coverage:platform-linux" "test target does not compile off Linux (F-001)"
not_run "coverage:platform-macos" "test target does not compile off macOS (F-001)"

head1 "11a. suite summary (JSON + LCOV)"
cargo llvm-cov nextest --workspace --all-features "${EXCLUDES[@]}" \
  --json --output-path "$OUT/cov.json" \
  --lcov --output-path "$OUT/lcov.info" \
  2>&1 | tee "$OUT/coverage.log" | tail -30
say "exit=$?"

head1 "11b. per-crate summary"
: > "$OUT/coverage-per-crate.txt"
crates_list | while IFS=$'\t' read -r name path; do
  case "$name" in
    platform-linux|platform-macos|input|_template) continue ;;
  esac
  out=$(cargo llvm-cov report --package "$name" --summary-only 2>/dev/null | tail -5)
  printf '%-24s %s\n' "$name" "$(echo "$out" | tr '\n' ' ' | cut -c1-120)" | tee -a "$OUT/coverage-per-crate.txt"
done

head1 "11c. zero-coverage files (first mutation + fuzz targets)"
$PY - "$OUT/cov.json" <<'PYEOF'
import json, sys
try:
    d = json.load(open(sys.argv[1], encoding="utf-8"))
except Exception as e:
    print("cannot parse cov.json:", e); raise SystemExit
rows = []
for f in d.get("data", [{}])[0].get("files", []):
    s = f.get("summary", {})
    lines = s.get("lines", {})
    pct = lines.get("percent", 100.0)
    if pct < 1.0:
        rows.append((pct, f.get("filename", "?").replace("\\", "/")))
rows.sort()
print("files with ~0%% line coverage: %d" % len(rows))
for pct, name in rows[:40]:
    print("  %5.1f%%  %s" % (pct, name))
PYEOF
