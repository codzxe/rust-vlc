#!/usr/bin/env bash
# T01 - strict clippy sweep: L0 baseline, L1 pedantic, L2 nursery, L3 targeted restriction.
# Usage: 01_clippy.sh [crate ...]
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

TARGETS=("$@")
if [ ${#TARGETS[@]} -eq 0 ]; then
  SCOPE=(--workspace --all-targets --all-features --exclude platform-linux --exclude platform-macos)
  # platform-linux tests do not compile off Linux (finding F-001); excluded, recorded.
  not_run "clippy/workspace-all-targets" "platform-linux+platform-macos excluded: their test targets do not compile on this host (F-001). Re-run on ubuntu for full coverage."
else
  SCOPE=()
  for c in "${TARGETS[@]}"; do SCOPE+=(-p "$c" --all-targets --all-features); done
fi

CLIPPY_LINTS_WARN=(
  -W clippy::pedantic
)
CLIPPY_LINTS_RESTRICT=(
  -W clippy::arithmetic_side_effects
  -W clippy::as_conversions
  -W clippy::indexing_slicing
  -W clippy::unwrap_used
  -W clippy::expect_used
  -W clippy::panic
  -W clippy::exit
  -W clippy::mem_forget
  -W clippy::lossy_float_literal
  -W clippy::cast_possible_truncation
  -W clippy::cast_sign_loss
)

run_level() {
  local level="$1"; shift
  local logf="$OUT/clippy-$level.log"
  head1 "clippy $level"
  cargo clippy "${SCOPE[@]}" "$@" 2>&1 | tee "$logf" >/dev/null
  local rc=${PIPESTATUS[0]}
  local warns errors
  warns=$(grep -c '^warning: ' "$logf" 2>/dev/null || echo 0)
  errors=$(grep -c '^error' "$logf" 2>/dev/null || echo 0)
  say "level=$level exit=$rc warnings=$warns errors=$errors log=$logf"
  # per-lint histogram (clippy prints '= note: #[warn(lint)]' / '= note: `#[deny(lint)]`')
  grep -o '#\[warn([a-z_:0-9]*)\]' "$logf" 2>/dev/null | sed 's/#\[warn(//;s/)\]//' | sort | uniq -c | sort -rn | head -40
}

run_level L0 -D warnings
run_level L1 "${CLIPPY_LINTS_WARN[@]}"
run_level L2 -W clippy::nursery

# L3 is per-crate only: workspace-wide it drowns in noise by design.
if [ ${#TARGETS[@]} -eq 0 ]; then
  HOT=(codecs demux media-clock packet-queue playback-engine media-types transcode)
  for c in "${HOT[@]}"; do
    if ! ( cd "$ROOT" && cargo metadata --no-deps --format-version 1 >/dev/null 2>&1 ); then break; fi
    head1 "clippy L3 $c"
    cargo clippy -p "$c" --all-targets --all-features "${CLIPPY_LINTS_RESTRICT[@]}" 2>&1 \
      | tee "$OUT/clippy-L3-$c.log" >/dev/null
    grep -o '#\[warn([a-z_:0-9]*)\]' "$OUT/clippy-L3-$c.log" 2>/dev/null \
      | sed 's/#\[warn(//;s/)\]//' | sort | uniq -c | sort -rn | head -15
  done
fi

say ""
say "logs: $OUT/clippy-*.log"
