#!/usr/bin/env bash
# T09 - feature powerset. CI only ever builds --all-features; this finds the rest.
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

need cargo-hack || exit 0
# NOTE: --no-dev-deps is deliberately NOT used: cargo-hack rewrites the real
# Cargo.toml while running, and manifests are Lead-owned in this repo. We accept
# the slower run instead of mutating manifests during an audit.
head1 "9a. --each-feature (linear, all crates)"
cargo hack check --each-feature --workspace \
  --exclude platform-linux --exclude platform-macos \
  2>&1 | tee "$OUT/features-each.log" | tail -30
say "each-feature exit=$?"

head1 "9b. --feature-powerset --depth 2 (crates with <= 6 features only)"
$PY - "$ROOT" <<'PYEOF' > "$OUT/features-crates.txt"
import json, subprocess, sys, os, tomllib
root = sys.argv[1]
meta = json.loads(subprocess.check_output(["cargo","metadata","--format-version","1","--no-deps","--locked"], cwd=root, text=True))
for p in meta["packages"]:
    feats = list((p.get("features") or {}).keys())
    if 0 < len(feats) <= 6 and not p["name"].startswith("platform-"):
        print("%s %d %s" % (p["name"], len(feats), ",".join(feats)))
PYEOF
cat "$OUT/features-crates.txt"
while read -r name ncombo feats; do
  [ -z "${name:-}" ] && continue
  say "--- $name ($ncombo features)"
  cargo hack check --feature-powerset --depth 2 --no-dev-deps -p "$name" \
    2>&1 | tee -a "$OUT/features-powerset.log" | tail -5
done < "$OUT/features-crates.txt"

head1 "9c. --no-default-features"
cargo hack check --no-dev-deps --workspace --exclude platform-linux --exclude platform-macos --no-default-features \
  2>&1 | tee "$OUT/features-nodefault.log" | tail -20

head1 "verdict"
say "logs: $OUT/features-*.log"
