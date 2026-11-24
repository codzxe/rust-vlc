#!/usr/bin/env bash
# scripts/quality/common.sh - shared helpers for the quality audit runner.
# POSIX-ish bash; runs on Git-Bash (Windows) and Linux CI identically.
# Rule: never abort the whole sweep because one tool failed - record and continue.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="$ROOT/docs/audit/reports"
mkdir -p "$OUT"
cd "$ROOT" || exit 1

# python launcher: prefer a real interpreter. The Windows Store `python3`
# App Execution Alias resolves but fails at exec time (REGDB_E_CLASSNOTREG),
# so validate that the interpreter actually runs before using it.
PY=""
if command -v python3 >/dev/null 2>&1 && python3 -c "pass" >/dev/null 2>&1; then
  PY=python3
elif command -v python >/dev/null 2>&1 && python -c "pass" >/dev/null 2>&1; then
  PY=python
else
  PY=python3
fi

say()  { printf '%s\n' "$*"; }
head1() { printf '\n=== %s ===\n' "$*"; }
stamp() { date -u +%Y-%m-%dT%H:%M:%SZ; }

# record a tool that could not run, with the reason. NEVER invent a result.
not_run() {
  local tool="$1" reason="$2"
  printf '%s\t%s\t%s\n' "$(stamp)" "$tool" "$reason" >> "$OUT/not-run.txt"
  say "NOT RUN [$tool]: $reason"
}

need() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    not_run "$tool" "not installed (run scripts/quality/00_install_tools.sh)"
    return 1
  fi
  return 0
}

# ast-grep launcher: cargo binary if present, else pinned npx package
# (npx.cmd is required under Git-Bash on Windows; plain `npx` hits a .ps1 shim)
sg_run() {
  if command -v ast-grep >/dev/null 2>&1; then
    ast-grep "$@"
  elif command -v sg >/dev/null 2>&1; then
    sg "$@"
  elif command -v npx.cmd >/dev/null 2>&1; then
    npx.cmd --yes -p @ast-grep/cli@0.51.3 ast-grep "$@"
  else
    npx --yes -p @ast-grep/cli@0.51.3 ast-grep "$@"
  fi
}

# list workspace crates from cargo metadata (name<TAB>manifest_path)
crates_list() {
  $PY - "$ROOT" <<'PYEOF'
import json, subprocess, sys, os
root = sys.argv[1]
meta = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
    cwd=root, text=True))
for p in meta["packages"]:
    rel = os.path.relpath(os.path.dirname(p["manifest_path"]), root).replace("\\", "/")
    print("%s\t%s" % (p["name"], rel))
PYEOF
}

log() { # log <name> ; reads stdin -> reports/<name>
  local name="$1"
  tee "$OUT/$name"
}
