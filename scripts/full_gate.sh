#!/usr/bin/env bash
# Full workspace gate with HONEST exit-code propagation (no pipe-masking).
# Usage: bash /home/z/my-project/scripts/full_gate.sh [worktree-dir]
set -uo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cd "${1:-/home/z/my-project}"

fail=0

echo "=== 1. fmt ==="
if cargo fmt --all --check; then echo "fmt: CLEAN"; else echo "fmt: FAILED"; fail=1; fi

echo "=== 2. clippy ==="
if cargo clippy --workspace --all-targets --all-features -- -D warnings > /tmp/clippy.log 2>&1; then
  echo "clippy: CLEAN"
else
  echo "clippy: FAILED"; tail -20 /tmp/clippy.log; fail=1
fi

echo "=== 3. test ==="
if cargo test --workspace --all-features > /tmp/test.log 2>&1; then
  rg "test result:" /tmp/test.log | awk '{p+=$4; f+=$6; i+=$8} END {print "tests: "p" passed / "f" failed / "i" ignored"}'
  echo "test: CLEAN"
else
  echo "test: FAILED"; rg "FAILED|panicked|error\[" /tmp/test.log | head -20; fail=1
fi

echo "=== 4. check-architecture ==="
if cargo xtask check-architecture > /tmp/arch.log 2>&1; then
  tail -2 /tmp/arch.log; echo "arch: PASS"
else
  echo "arch: FAILED"; tail -10 /tmp/arch.log; fail=1
fi

echo "=== 5. fixtures --check ==="
if cargo xtask fixtures --check > /tmp/fixtures.log 2>&1; then
  tail -1 /tmp/fixtures.log; echo "fixtures: PASS"
else
  echo "fixtures: FAILED"; tail -5 /tmp/fixtures.log; fail=1
fi

echo "=== 6. fuzz-registry (doc 04 §8.5 — registry-vs-implementation diff) ==="
if bash scripts/fuzz_registry_check.sh > /tmp/fuzzreg.log 2>&1; then
  tail -1 /tmp/fuzzreg.log; echo "fuzz-registry: PASS"
else
  echo "fuzz-registry: FAILED"; cat /tmp/fuzzreg.log; fail=1
fi

echo "=== GATE RESULT: $([ $fail -eq 0 ] && echo PASS || echo FAIL) ==="
exit $fail
