#!/usr/bin/env bash
# Status-truth drift gate (FX8-A01, FIX-PLAN §4 F2.1): the ONE status
# layer is docs/generated/status-truth.toml, generated from source by
# scripts/quality/status_truth.py. This gate regenerates it to a temp
# file and fails the build when the committed copy has drifted.
#
# Exit codes:
#   0  the committed file is byte-identical to the source-derived truth
#   1  DRIFT — regenerate and commit:
#        python3 scripts/quality/status_truth.py > docs/generated/status-truth.toml
#   2  the generator itself failed (source-shape failure: a parsed file
#      moved or a shape drifted) — the status layer refuses to guess
#      rather than understate; fix the source or teach the parser
#
# Python 3 stdlib only; no cargo anywhere. Runs from any working
# directory (CI invokes `bash scripts/quality/status_truth_check.sh`).
set -u

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "${script_dir}/../.." && pwd)"

generator="${root}/scripts/quality/status_truth.py"
committed="${root}/docs/generated/status-truth.toml"

fresh="$(mktemp "${TMPDIR:-/tmp}/status-truth.XXXXXX")"
trap 'rm -f "${fresh}"' EXIT

python3 "${generator}" > "${fresh}"
status=$?
if [ "${status}" -ne 0 ]; then
    echo "status-truth: generator FAILED (exit ${status}) — source-shape failure, never understated" >&2
    exit 2
fi

if [ ! -f "${committed}" ]; then
    echo "status-truth: DRIFT — ${committed} is missing; generate and commit it:" >&2
    echo "  python3 scripts/quality/status_truth.py > docs/generated/status-truth.toml" >&2
    exit 1
fi

if diff -u "${committed}" "${fresh}" > /dev/null; then
    echo "status-truth: OK — committed status truth matches the source-derived truth"
    exit 0
fi

echo "status-truth: DRIFT — ${committed} does not match the source-derived truth" >&2
echo "  regenerate: python3 scripts/quality/status_truth.py > docs/generated/status-truth.toml" >&2
echo "  diff (committed vs fresh):" >&2
diff -u "${committed}" "${fresh}" >&2
exit 1
