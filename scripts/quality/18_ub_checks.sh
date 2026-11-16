#!/usr/bin/env bash
# T18 - UB, overflow, memory and race detection lanes.
# Usage: 18_ub_checks.sh {overflow|careful|miri|asan|ubsan|tsan} [crate]
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

mkdir -p "$OUT/ub"
cmd="${1:-overflow}"
crate="${2:-}"
PKG_FLAG=(); [ -n "$crate" ] && PKG_FLAG=(-p "$crate")
EXCL=(--workspace --exclude platform-linux --exclude platform-macos --exclude input)

case "$cmd" in
  overflow)
    head1 "overflow lane (stable): arithmetic overflow + debug assertions"
    RUSTFLAGS="-C overflow-checks=on -C debug-assertions=on" \
      cargo test "${EXCL[@]}" --all-features --no-fail-fast \
      2>&1 | tee "$OUT/ub/overflow.log" | tail -30
    say "exit=$?"
    say "any 'attempt to add with overflow' / 'panic' line above is a finding"
    ;;
  careful)
    if need cargo-careful; then
      head1 "cargo-careful lane"
      cargo careful test "${EXCL[@]}" --all-features 2>&1 | tee "$OUT/ub/careful.log" | tail -30
    else
      not_run "cargo-careful" "cargo install --locked cargo-careful@0.4.10"
    fi
    ;;
  miri)
    head1 "miri lane"
    if rustup toolchain list 2>/dev/null | grep -q nightly; then
      cargo +nightly miri test "${PKG_FLAG[@]}" 2>&1 | tee "$OUT/ub/miri.log" | tail -30
    else
      not_run "miri" "nightly toolchain required (blocker B3); cargo miri available but needs +nightly"
    fi
    ;;
  asan|ubsan|tsan)
    head1 "$cmd lane (nightly, Linux)"
    case "$(uname -s)" in
      Linux) : ;;
      *) not_run "$cmd" "sanitizers are Linux-only; host is $(uname -s)"; exit 0 ;;
    esac
    if ! rustup toolchain list 2>/dev/null | grep -q nightly; then
      not_run "$cmd" "nightly toolchain required (blocker B3)"; exit 0
    fi
    san="$cmd"
    RUSTFLAGS="-Zsanitizer=$san" cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu \
      "${EXCL[@]}" --all-features 2>&1 | tee "$OUT/ub/$cmd.log" | tail -30
    ;;
  *)
    say "usage: 18_ub_checks.sh {overflow|careful|miri|asan|ubsan|tsan} [crate]"
    ;;
esac

head1 "why this matters here"
cat <<'TXT'
Release builds wrap on integer overflow silently. Codec math like
  sample_rate * channels * bytes_per_sample
produces a wrong number instead of a panic, and no test fails unless the suite
runs with overflow-checks=on. The overflow lane above is the cheap stable-only
version of that check.
TXT
