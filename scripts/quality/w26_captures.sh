#!/usr/bin/env bash
# w26_captures.sh — wave-26 verification-capture regen harness (FX8-A06,
# FIX-PLAN §4 F2.4 evidence culture). Audit finding
# audit/final/DEVIL-AUDIT-REPORT.md:112 (§4 #7): "Evidence lives in /tmp and
# .gitignore. w26 verification captures never committed" — every capture
# artifact the docs/waves/w26/** docs cite lived outside the repo (in /tmp or
# an agent scratch dir) and no bytes were ever committed.
#
# This harness makes that evidence class reproducible:
#   probe                 check-only: is the UI stack the capture regen needs
#                         present (node, ui/node_modules/playwright, dev
#                         server on :5173)? exit 0 present / 1 missing.
#   capture [--out DIR] [--port N]
#                         regenerate the w26 capture lane IN-TREE with the
#                         committed tool ui/scripts/dev-shots.mjs, over the
#                         25-route truth of ui/e2e/visual/manifest.ts.
#                         Probes first; fails loud (exit 1) if the stack is
#                         missing. Default out: docs/waves/w26/captures/regen.
#   compare DIR_A [DIR_B]
#                         capture diff: the two dirs must hold the same files
#                         byte-for-byte (md5); exit 1 on any difference.
#                         DIR_B defaults to docs/waves/w26/captures/regen.
#   index-gate [--write]  the drift gate: regenerate
#                         docs/waves/w26/captures/INDEX.md (the committed
#                         inventory of the capture artifacts the w26 docs
#                         cite) from a fresh scan of docs/waves/w26/** and
#                         diff it against the committed copy; exit 1 on
#                         drift. --write regenerates the committed copy in
#                         place (the regeneration command).
#
# Exit codes:
#   0  ok (gate green / probe green / capture clean / captures identical)
#   1  DRIFT, missing UI stack, capture failure, or capture-set difference
#   2  usage error or generator failure (scan-shape drift — the harness
#      refuses to guess rather than understate)
#
# Python 3 stdlib only (the embedded INDEX generator); no cargo anywhere.
# Runs from any working directory.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "${script_dir}/../.." && pwd)"
w26="${root}/docs/waves/w26"
index_file="${w26}/captures/INDEX.md"
dev_shots="${root}/ui/scripts/dev-shots.mjs"
manifest="${root}/ui/e2e/visual/manifest.ts"
port=5173
out_rel="docs/waves/w26/captures/regen"

usage() {
    cat >&2 <<'USAGE'
usage: w26_captures.sh <mode> [options]
modes:
  probe                 check the UI stack (node, ui/node_modules/playwright,
                        dev server on :5173); exit 1 with remediation if missing
  capture [--out DIR] [--port N]
                        regenerate the w26 capture lane in-tree via
                        ui/scripts/dev-shots.mjs (probes first, fails loud)
  compare DIR_A [DIR_B] byte-level capture diff (md5 over both file sets)
  index-gate [--write]  drift-gate docs/waves/w26/captures/INDEX.md against
                        a fresh scan of docs/waves/w26/** (--write regenerates)
USAGE
}

# ---------------------------------------------------------------------------
# index-gate: the generator + drift gate for the committed inventory
# ---------------------------------------------------------------------------
index_generate() {
    python3 - "${root}" <<'PY'
import collections
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1])
w26 = root / "docs" / "waves" / "w26"
index_path = w26 / "captures" / "INDEX.md"

# --- the scan rule (documented verbatim in the generated header) ------------
# A citation token is a path rooted at /tmp/ or /home/z/ (agent scratch
# space). The token charset is conservative: braces, commas, '*' and dots
# survive (balanced {...} brace expansions and globs are cited artifact
# SETS); the token stops at whitespace, backtick, quotes, parens, square
# brackets, angle brackets, pipe and backslash (prose and placeholders).
TOKEN = re.compile("/(?:tmp|home/z)/[^\\s`()\\[\\]<>\"'|\\\\]+")
STRIP = ".,;:!?"
# kind by known extension of the LAST segment; anything else is a session
# directory citation (never a fabricated extension: ".e01" in
# shots-w26.e01 is the w26 session id, not an extension).
KIND_BY_EXT = {
    "png": "shot", "log": "log", "txt": "log", "json": "report",
    "mjs": "tool", "js": "tool", "cjs": "tool", "py": "tool",
    "html": "tool", "ts": "tool",
}


def artifact_of(raw):
    """Normalize one cited token to an artifact path, or None (not one)."""
    tok = raw
    # an unbalanced '{' means prose cut the token mid-brace: keep the part
    # before the brace (usually collapses to the session dir citation)
    if "{" in tok and "}" not in tok:
        tok = tok[: tok.index("{")]
    tok = tok.rstrip(STRIP).rstrip("/")
    if "{" in tok and "}" not in tok:
        tok = tok[: tok.index("{")]
        tok = tok.rstrip(STRIP).rstrip("/")
    # placeholder fragments (/tmp/shots-<task-id> truncates to /tmp/shots-)
    if tok.endswith("-") or tok in ("/tmp", "/home/z"):
        return None
    if tok.startswith("/tmp/"):
        segs = tok[len("/tmp/"):].split("/")
    elif tok.startswith("/home/z/"):
        segs = tok[len("/home/z/"):].split("/")
    else:
        return None
    segs = [s for s in segs if s]
    if not segs:
        return None
    first = segs[0]
    # the repo itself (/home/z/my-project/...) is not a scratch artifact
    if first == "my-project":
        return None
    # the agent node_modules install is environment, not an artifact
    if first == "node_modules":
        return None
    # a bare file straight under /home/z is only an artifact when it is
    # clearly a w26 session file; scratch DIRECTORIES count regardless
    if len(segs) == 1 and "." in first and "w26" not in first:
        return None
    if not any(c.isalnum() for c in segs[-1]):
        return None
    return tok


def kind_of(artifact):
    if "*" in artifact or "{" in artifact:
        return "set"
    last = artifact.rsplit("/", 1)[-1]
    if "." in last:
        ext = last.rsplit(".", 1)[-1].lower()
        if ext in KIND_BY_EXT:
            return KIND_BY_EXT[ext]
    return "dir"


citing = collections.defaultdict(set)
scan_files = sorted(p for p in w26.rglob("*") if p.is_file() and p != index_path)
for p in scan_files:
    rel = p.relative_to(w26).as_posix()
    text = p.read_text(errors="replace")
    for m in TOKEN.findall(text):
        a = artifact_of(m)
        if a is not None:
            citing[a].add(rel)

artifacts = sorted(citing)
citing_docs = sorted(set().union(*citing.values())) if citing else []
kind_counts = collections.Counter(kind_of(a) for a in artifacts)

W = 76


def wrap(text):
    out, line = [], ""
    for word in text.split(" "):
        if line and len(line) + 1 + len(word) > W:
            out.append(line)
            line = word
        else:
            line = word if not line else line + " " + word
    if line:
        out.append(line)
    return "\n".join(out)


print("# w26 verification captures — the committed inventory (GENERATED)")
print()
print("> GENERATED FILE — do not edit by hand. Regenerate:")
print("> `scripts/quality/w26_captures.sh index-gate --write`")
print("> Drift gate (exit 1 when this file disagrees with a fresh scan of")
print("> `docs/waves/w26/**`): `scripts/quality/w26_captures.sh index-gate`")
print()
print(wrap(
    "Why this inventory exists (FX8-A06, FIX-PLAN §4 F2.4): the Devil "
    "Audit's process-failure finding #7 (audit/final/DEVIL-AUDIT-REPORT.md:112) "
    "— \"Evidence lives in /tmp and .gitignore. w26 verification captures "
    "never committed\" — every capture artifact the wave-26 docs cite below "
    "lived OUTSIDE the repo (in /tmp or an agent scratch dir under /home/z) "
    "and no bytes were ever committed."))
print()
print(wrap(
    "NO original capture bytes exist to recover. /tmp is recycled and the "
    "agent scratch dirs are gone; nothing in this directory fabricates "
    "bytes. This INDEX is the committed record of WHAT the docs cite and "
    "HOW the class is regenerated — the bytes come back only by running the "
    "regen, never by trusting this list."))
print()
print("## Regen basis — the committed tool, in-tree-only output")
print()
print(wrap(
    "- Capture tool: `ui/scripts/dev-shots.mjs` (committed — the wave-26 "
    "agent screenshot tool, w26 contract §4.7/§5). The regen harness "
    "`scripts/quality/w26_captures.sh capture` drives it over the 25-route "
    "truth of `ui/e2e/visual/manifest.ts` (ROUTES) with `--out "
    "docs/waves/w26/captures/regen/` — regenerated captures land IN-TREE, "
    "never /tmp-only."))
print(wrap(
    "- The regen needs the UI stack (node + ui/node_modules/playwright + "
    "the dev server on :5173). `scripts/quality/w26_captures.sh probe` is "
    "check-only; `capture` probes first and fails loud (exit 1) when the "
    "stack is missing — it never pretends a regen ran."))
print(wrap(
    "- Capture diff: `scripts/quality/w26_captures.sh compare DIR_A "
    "[DIR_B]` — byte-level (md5) comparison of two capture sets."))
print(wrap(
    "- The `.gitignore` exceptions `!docs/waves/w26/captures/**` and "
    "`!docs/waves/w26/captures/*.log` keep everything under this directory "
    "committable — including the `console-*.log` artifacts dev-shots.mjs "
    "writes when a state logs console errors, which the global `*.log` rule "
    "would otherwise ignore."))
print(wrap(
    "- What the regen produces: the capture CLASS — screenshots of the "
    "routed surface plus console-error logs — not the historical filenames "
    "one-for-one (many rows below are one-off session scripts and reports "
    "whose bytes never existed in the repo; rows of kind shot/log/report "
    "under a shots-*/cap-* session dir are the rows the regen class "
    "reproduces)."))
print()
print("## The scan rule (what the drift gate re-runs)")
print()
print(wrap(
    "Corpus: every file under docs/waves/w26/** except this INDEX (a "
    "generated file never scans itself). A citation is a token rooted at "
    "/tmp/ or /home/z/ (agent scratch space) in a conservative charset: "
    "balanced {...} brace expansions, commas and * globs survive; the token "
    "stops at whitespace, backtick, quotes, parens, brackets, angle "
    "brackets, pipe and backslash. Normalization: an unbalanced '{' (prose "
    "fragment) cuts the token at the brace; trailing prose punctuation and "
    "slashes are stripped; placeholder fragments (tokens ending in '-') "
    "are dropped; /home/z/my-project (the repo itself) and /home/z/"
    "node_modules (environment, not an artifact) are dropped; a bare file "
    "straight under /home/z counts only when its name says w26. Kind: known "
    "extension of the last segment (png→shot, log/txt→log, json→report, "
    "mjs/js/cjs/py/html/ts→tool), any * or {...} → set, anything else → dir "
    "(a session directory citation; '.e01' in shots-w26.e01 is the session "
    "id, not an extension)."))
print()
print("## Summary")
print()
print(f"- corpus scanned: {len(scan_files)} files under docs/waves/w26/")
print(f"- citing docs: {len(citing_docs)}")
print(f"- distinct cited artifacts: {len(artifacts)}")
kinds = ", ".join(f"{k} {kind_counts[k]}" for k in
                  ("shot", "log", "report", "tool", "dir", "set")
                  if kind_counts.get(k))
print(f"- by kind: {kinds}")
print()
print(f"## Inventory — {len(artifacts)} cited artifacts, {len(citing_docs)} citing docs")
print()
print("Citing docs are paths relative to docs/waves/w26/. No row implies")
print("the bytes exist anywhere — see the honesty note above.")
print()
print("| artifact | kind | cited by |")
print("|---|---|---|")
for a in artifacts:
    docs = ", ".join(sorted(citing[a]))
    print(f"| `{a}` | {kind_of(a)} | {docs} |")
PY
}

index_gate() {
    local write=0
    if [ "${1:-}" = "--write" ]; then
        write=1
        if [ -n "${2:-}" ]; then
            usage
            exit 2
        fi
    elif [ -n "${1:-}" ]; then
        usage
        exit 2
    fi

    fresh="$(mktemp "${TMPDIR:-/tmp}/w26-index.XXXXXX")"
    # embed the path (a `local` would be unbound when the trap fires)
    trap "rm -f '${fresh}'" EXIT
    if ! index_generate > "${fresh}"; then
        echo "w26-index: generator FAILED — scan-shape failure, never understated" >&2
        exit 2
    fi

    if [ "$write" -eq 1 ]; then
        mkdir -p "${w26}/captures"
        cp "${fresh}" "${index_file}"
        echo "w26-index: REGENERATED ${index_file} ($(grep -c '^| `' "${index_file}") artifacts)"
        return 0
    fi

    if [ ! -f "${index_file}" ]; then
        echo "w26-index: DRIFT — ${index_file} is missing; generate and commit it:" >&2
        echo "  scripts/quality/w26_captures.sh index-gate --write" >&2
        exit 1
    fi

    if diff -u "${index_file}" "${fresh}" > /dev/null; then
        echo "w26-index: OK — committed inventory matches a fresh scan of docs/waves/w26/**"
        return 0
    fi
    echo "w26-index: DRIFT — ${index_file} does not match a fresh scan of docs/waves/w26/**" >&2
    echo "  regenerate: scripts/quality/w26_captures.sh index-gate --write" >&2
    echo "  diff (committed vs fresh):" >&2
    diff -u "${index_file}" "${fresh}" >&2 || true
    exit 1
}

# ---------------------------------------------------------------------------
# probe: check-only UI-stack availability (the capture regen's prerequisites)
# ---------------------------------------------------------------------------
probe() {
    local fail=0
    if command -v node > /dev/null 2>&1; then
        echo "probe: PASS — node on PATH ($(node --version))"
    else
        echo "probe: FAIL — node not on PATH (ui/scripts/dev-shots.mjs runs on node)" >&2
        fail=1
    fi
    if [ -f "${root}/ui/node_modules/playwright/package.json" ]; then
        echo "probe: PASS — ui/node_modules/playwright present (dev-shots resolves playwright from the ui workspace)"
    else
        echo "probe: FAIL — ui/node_modules/playwright missing; install it: (cd ui && npm ci)" >&2
        fail=1
    fi
    local http="not-checked"
    if command -v curl > /dev/null 2>&1; then
        http="$(curl -s -o /dev/null -w '%{http_code}' --max-time 3 "http://localhost:${port}/" || true)"
    elif command -v python3 > /dev/null 2>&1; then
        http="$(python3 -c "import urllib.request
try:
    print(urllib.request.urlopen('http://localhost:${port}/', timeout=3).status)
except Exception as e:
    print('unreachable:', e)" 2>&1 | head -1)"
    else
        echo "probe: FAIL — cannot check the dev server: need curl or python3 on PATH" >&2
        fail=1
    fi
    if [ "${http}" = "200" ]; then
        echo "probe: PASS — dev server answers 200 on http://localhost:${port}/"
    else
        echo "probe: FAIL — dev server not answering on http://localhost:${port}/ (got: ${http}); start it: (cd ui && npm run dev)" >&2
        fail=1
    fi
    if [ "${fail}" -ne 0 ]; then
        echo "probe: UI stack MISSING — the capture regen cannot run honestly; fix the FAILs above (all checks are required)" >&2
        return 1
    fi
    echo "probe: UI stack present — scripts/quality/w26_captures.sh capture can run"
    return 0
}

# ---------------------------------------------------------------------------
# capture: regenerate the w26 capture lane in-tree with the committed tool
# ---------------------------------------------------------------------------
capture() {
    local out="${out_rel}"
    while [ $# -gt 0 ]; do
        case "$1" in
            --out) out="$2"; shift 2 ;;
            --port) port="$2"; shift 2 ;;
            *) usage; exit 2 ;;
        esac
    done
    case "$out" in
        /*) ;;
        *) out="${root}/${out}" ;;
    esac

    probe

    if [ ! -f "${dev_shots}" ]; then
        echo "capture: FAIL — committed capture tool missing: ${dev_shots}" >&2
        exit 1
    fi
    if [ ! -f "${manifest}" ]; then
        echo "capture: FAIL — route manifest missing: ${manifest}" >&2
        exit 1
    fi

    # the 25-route truth of ui/e2e/visual/manifest.ts (the ROUTES block
    # only — the accent-sample routes stay out of the lane: the grid anchor
    # already covers them at every viewport × DPI × mode)
    mapfile -t routes < <(sed -n '/export const ROUTES/,/\] as const;/ s/^  "\(\/[a-z0-9/-]*\)",$/\1/p' "${manifest}")
    if [ "${#routes[@]}" -eq 0 ]; then
        echo "capture: FAIL — parsed 0 routes from ${manifest} (ROUTES shape drifted; the harness refuses to guess)" >&2
        exit 2
    fi
    local pairs=()
    local r slug
    for r in "${routes[@]}"; do
        slug="${r#/}"
        slug="${slug//\//-}"
        pairs+=("${slug}:${r}")
    done
    echo "capture: regenerating the w26 lane — ${#routes[@]} routes (ui/e2e/visual/manifest.ts) -> ${out}"

    mkdir -p "${out}"
    if ! node "${dev_shots}" "${pairs[@]}" --out "${out}" --port "${port}"; then
        echo "capture: FAIL — dev-shots reported failures above (console errors or unreachable routes); nothing swept under the rug" >&2
        exit 1
    fi

    local n
    n="$(find "${out}" -type f | wc -l)"
    echo "capture: OK — ${n} artifacts under ${out}"
    find "${out}" -type f | sort | sed 's/^/  /'
    echo "capture: next: scripts/quality/w26_captures.sh compare <other-set> ${out}"
    echo "capture: the .gitignore exceptions keep everything under docs/waves/w26/captures/ committable — commit the regen with the tree"
}

# ---------------------------------------------------------------------------
# compare: byte-level capture diff between two capture sets
# ---------------------------------------------------------------------------
compare() {
    if [ $# -lt 1 ] || [ $# -gt 2 ]; then
        usage
        exit 2
    fi
    local a="$1" b="${2:-${out_rel}}"
    case "$a" in /*) ;; *) a="${root}/${a}" ;; esac
    case "$b" in /*) ;; *) b="${root}/${b}" ;; esac
    if [ ! -d "$a" ]; then
        echo "compare: FAIL — not a directory: $a" >&2
        exit 1
    fi
    if [ ! -d "$b" ]; then
        echo "compare: FAIL — not a directory: $b" >&2
        exit 1
    fi
    local tmp
    tmp="$(mktemp -d "${TMPDIR:-/tmp}/w26-compare.XXXXXX")"
    # embed the path (a `local` would be unbound when the trap fires)
    trap "rm -rf '${tmp}'" EXIT
    ( cd "$a" && find . -type f -exec md5sum {} + ) | sort > "${tmp}/a.md5"
    ( cd "$b" && find . -type f -exec md5sum {} + ) | sort > "${tmp}/b.md5"
    if diff -u "${tmp}/a.md5" "${tmp}/b.md5" > "${tmp}/diff"; then
        echo "compare: IDENTICAL — $a and $b hold the same files byte-for-byte ($(wc -l < "${tmp}/a.md5") files)"
        return 0
    fi
    echo "compare: DIFF — capture sets differ ($a vs $b); diff (name/md5, A vs B):" >&2
    cat "${tmp}/diff" >&2
    return 1
}

# ---------------------------------------------------------------------------
mode="${1:-}"
if [ $# -gt 0 ]; then shift; fi
case "$mode" in
    probe) probe ;;
    capture) capture "$@" ;;
    compare) compare "$@" ;;
    index-gate) index_gate "$@" ;;
    *) usage; exit 2 ;;
esac
