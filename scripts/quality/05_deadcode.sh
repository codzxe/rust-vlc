#!/usr/bin/env bash
# T05 - dead code, orphans, unused dependencies and features.
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

head1 "5a. cargo-machete (declared but never imported)"
if need cargo-machete; then
  cargo machete . 2>&1 | tee "$OUT/machete.txt" | tail -40
fi

head1 "5b. cargo-shear (metadata-accurate unused deps + features)"
if need cargo-shear; then
  cargo shear 2>&1 | tee "$OUT/shear.txt" | tail -60
fi

head1 "5c. cargo-modules orphans (modules never referenced)"
if need cargo-modules; then
  cargo modules orphans --workspace 2>&1 | tee "$OUT/orphans.txt" | tail -60
fi

head1 "5d. cargo-udeps (nightly, resolution-level)"
if need cargo-udeps; then
  if rustup toolchain list 2>/dev/null | grep -q nightly; then
    cargo +nightly udeps --workspace --all-targets 2>&1 | tee "$OUT/udeps.txt" | tail -40
  else
    not_run "cargo-udeps" "nightly toolchain not installed (blocker B3)"
  fi
fi

head1 "5e. lint-suppression census (ast-grep allow-attr)"
if [ -s "$OUT/astgrep/allow-attr.json" ]; then
  $PY -c "import json,sys,collections;h=json.load(open(sys.argv[1],encoding='utf-8'));c=collections.Counter(x.get('text','')[:60] for x in h);print('total #[allow]:',len(h));[print('%5d %s'%(v,k)) for k,v in c.most_common(15)]" "$OUT/astgrep/allow-attr.json"
else
  say "run 03_astgrep.sh first for the #[allow] census"
fi
