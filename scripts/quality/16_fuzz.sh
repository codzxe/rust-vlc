#!/usr/bin/env bash
# T16 - fuzz the parsers with the real-media seeds already committed in fuzz/.
# Usage: 16_fuzz.sh {list|run <target> [seconds]|repro <artifact>}
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

mkdir -p "$OUT/fuzz"

head1 "16a. fuzz targets present in the repo"
if [ -f fuzz/Cargo.toml ]; then
  say "fuzz/Cargo.toml found:"
  grep -E '^\[\[bin\]\]|^name\s*=|^path\s*=' fuzz/Cargo.toml | head -40 | tee "$OUT/fuzz/targets.txt"
  say ""
  say "seed corpus:"
  ls fuzz | grep -E '^seed_' | head -40 | tee -a "$OUT/fuzz/targets.txt"
  say "seed count: $(ls fuzz | grep -c '^seed_')"
else
  not_run "fuzz" "fuzz/Cargo.toml missing"
fi

cmd="${1:-list}"
case "$cmd" in
  list)
    if need cargo-fuzz; then cargo fuzz list 2>&1 | tee "$OUT/fuzz/list.txt"; fi
    ;;
  run)
    target="${2:?target name}"
    secs="${3:-3600}"
    if need cargo-fuzz; then
      head1 "fuzz $target for ${secs}s"
      cargo fuzz run "$target" -- -max_total_time="$secs" -rss_limit_mb=4096 \
        2>&1 | tee "$OUT/fuzz/$target.log" | tail -30
      cp -r fuzz/artifacts/"$target" "$OUT/fuzz/artifacts-$target" 2>/dev/null || true
    else
      not_run "cargo-fuzz" "cargo install --locked cargo-fuzz (needs nightly for sanitizers)"
    fi
    ;;
  repro)
    art="${2:?artifact path}"
    if need cargo-fuzz; then cargo fuzz run "${3:-parser}" "$art"; fi
    ;;
esac

head1 "note"
cat <<'TXT'
cargo-fuzz runs under ASan by default -> a crash is a real memory-safety finding.
Seeds are the repo's own real-media corpus (D-11), not synthetic input.
Every artifact in fuzz/artifacts/<target>/ becomes a regression case for the
fix program - never delete them.
TXT
