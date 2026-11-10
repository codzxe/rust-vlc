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

echo "=== 7. lint-literals (doc 07 §9 C1 — config-plane literal scan, card 7.w21a) ==="
if cargo xtask lint-literals > /tmp/litlit.log 2>&1; then
  tail -1 /tmp/litlit.log; echo "lint-literals: PASS"
else
  echo "lint-literals: FAILED"; rg 'FAIL' -A 15 /tmp/litlit.log | head -20; fail=1
fi

echo "=== 8. check-identity (doc 07 §9 C2 — no-hardcoded-identity scan, card 7.w21a) ==="
if cargo xtask check-identity > /tmp/chkident.log 2>&1; then
  tail -1 /tmp/chkident.log; echo "check-identity: PASS"
else
  echo "check-identity: FAILED"; rg 'FAIL' -A 15 /tmp/chkident.log | head -20; fail=1
fi

echo "=== 9. sbom --check (issue #52 / doc 11 §3.4 — CycloneDX 1.5 SBOM drift gate) ==="
if cargo xtask sbom --check > /tmp/sbom.log 2>&1; then
  tail -1 /tmp/sbom.log; echo "sbom: PASS"
else
  echo "sbom: FAILED"; tail -5 /tmp/sbom.log; fail=1
fi

# Step 10 is the W24-B soak lane's slot (issue #54, card 8.C1 — sibling
# worktree, lands via its own commit; numbering per the wave-24 plan).
echo "=== 11. packaging gate (issue #56 / cards 8.E1–8.E3 — packaging.toml schema + packaging script syntax + payload stagers + unsigned-fallback contracts) ==="
if bash scripts/packaging/gate.sh > /tmp/packaging-gate.log 2>&1; then
  tail -1 /tmp/packaging-gate.log; echo "packaging: PASS"
else
  echo "packaging: FAILED"; tail -25 /tmp/packaging-gate.log; fail=1
fi

echo "=== GATE RESULT: $([ $fail -eq 0 ] && echo PASS || echo FAIL) ==="
exit $fail
