#!/usr/bin/env bash
# T13 - test hygiene: fake-test heuristics, ignores, sleeps, flaky detection, timings.
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

head1 "13a. nextest test inventory"
if need cargo-nextest; then
  cargo nextest list --workspace --all-features --format json \
    > "$OUT/tests.json" 2> "$OUT/tests-list.err" \
    && say "tests: $(grep -c '"name"' "$OUT/tests.json" 2>/dev/null || echo '?')" \
    || not_run "nextest list" "$(head -3 "$OUT/tests-list.err" | tr '\n' ' ')"
else
  not_run "nextest list" "cargo-nextest not installed"
fi

head1 "13b. AST heuristics (no-op tests, tautologies, sleeps)"
for id in no-op-test assert-tautology sleep-in-test; do
  [ -f "$OUT/astgrep/$id.json" ] || bash "$ROOT/scripts/quality/03_astgrep.sh" "$id" >/dev/null 2>&1
  f="$OUT/astgrep/$id.json"
  if [ -s "$f" ]; then
    n=$($PY -c "import json,sys;print(len(json.load(open(sys.argv[1],encoding='utf-8'))))" "$f" 2>/dev/null || echo '?')
    say "$id: $n hits"
  else
    say "$id: 0 hits (or scan not run)"
  fi
done

head1 "13c. ignore / should_panic / assertion census (structural, comment-aware)"
$PY "$ROOT/scripts/quality/test_hygiene.py" | tee "$OUT/test-hygiene.txt"

head1 "13d. flaky-test detection (retries)"
if need cargo-nextest; then
  cargo nextest run --workspace --all-features --retries 3 --no-fail-fast \
    --exclude platform-linux --exclude platform-macos --exclude input \
    2>&1 | tee "$OUT/flaky.log" | tail -20
else
  not_run "nextest run" "cargo-nextest not installed"
fi
