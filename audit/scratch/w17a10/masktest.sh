#!/usr/bin/env bash
set -uo pipefail
say() { printf '%s\n' "$*"; }
python3 - <<'PYEOF'
import sys
sys.exit(1)
PYEOF
say "written: out.txt"
