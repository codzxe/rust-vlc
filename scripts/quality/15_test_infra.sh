#!/usr/bin/env bash
# T15 - test infrastructure audit: what testing primitives exist, what is missing.
# Discovery only: never edits Cargo.toml (Lead-owned). Emits a ready-to-apply plan.
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

WANT=(
  "nextest:cargo-nextest"
  "insta:insta"
  "insta-cmd:insta-cmd"
  "rstest:rstest"
  "proptest:proptest"
  "test-strategy:test-strategy"
  "trybuild:trybuild"
  "assert_cmd:assert_cmd"
  "assert_fs:assert_fs"
  "predicates:predicates"
  "escargot:escargot"
  "fail:fail"
  "serial_test:serial_test"
  "arbitrary:arbitrary"
  "dhat:dhat"
  "mockall:mockall"
)

head1 "15a. dev-dependency census"
: > "$OUT/test-infra.txt"
for entry in "${WANT[@]}"; do
  label="${entry%%:*}"; crate="${entry##*:}"
  hits=$(grep -rl --include=Cargo.toml -E "^\s*${crate//-/[-]}\s*=" crates apps tools tests 2>/dev/null | wc -l)
  printf '%-16s %s\n' "$label" "$([ "$hits" -gt 0 ] && echo "present in $hits manifest(s)" || echo MISSING)" | tee -a "$OUT/test-infra.txt"
done

head1 "15b. test primitive usage in source"
for m in proptest insta rstest trybuild assert_cmd arbitrary loom shuttle quickcheck; do
  n=$(grep -rn --include=*.rs -E "\b${m}::|#\[${m}\]|use ${m}" crates apps tools tests 2>/dev/null | wc -l)
  printf '%-12s %d usages\n' "$m" "$n"
done | tee -a "$OUT/test-infra.txt"

head1 "15c. ready-to-apply plan (Lead must approve each line)"
cat <<'TXT'
Per-crate dev-dependencies to add where the primitive is used (workspace
[workspace.dependencies] first, then per crate):

  insta            = "1.49.0"
  insta-cmd        = "0.7.0"
  rstest           = "0.28.0"
  proptest         = "1.12.0"
  test-strategy    = "0.4.5"
  trybuild         = "1.0.125"
  assert_cmd       = "2.2.2"
  assert_fs        = "1.1.4"
  predicates       = "3.1.4"
  escargot         = "1.0.1"
  fail             = "0.5.1"
  serial_test      = "4.0.1"
  arbitrary        = { version = "1.5.0", features = ["derive"] }
  dhat             = "0.3.3"

Constraint (AGENTS.md 3.2): none of these may enter the shipped graph.
Verify with: cargo tree -e normal --workspace | grep -E 'insta|proptest|rstest'
TXT

head1 "15d. example primitives"
cat "$ROOT/docs/audit/tasks/T15-examples.md" 2>/dev/null || say "(examples file not present)"
