#!/usr/bin/env bash
# T32 - format/codec coverage gate: you claim it, you prove it.
#
# The tree claims support for 12 codecs and 14 containers through its own fuzz
# targets - that is the machine-readable claim list (`fuzz/fuzz_targets/*.rs`).
# A speed gate can only measure what actually decodes, so coverage has to be
# gated BEFORE speed, and separately:
#
#   claim (fuzz target)  ->  fixture in the tree?  ->  a test names it?
#
# This is static, deterministic and takes seconds, so it runs on every PR.
# It fails when a claimed format has NO fixture: that claim is unbacked, and
# any "we support X" statement about it is unmeasured.
#
# Usage: 28_format_coverage.sh [--strict]
#   default : report gaps, exit 0  (first cycles: gaps are expected)
#   --strict: exit 1 when a claimed format/codec has no fixture
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

STRICT=0
[ "${1:-}" = "--strict" ] && STRICT=1
mkdir -p "$OUT/coverage-matrix"

head1 "28. format/codec coverage matrix (strict=$STRICT)"

$PY - "$ROOT" "$STRICT" "$OUT/coverage-matrix/coverage.txt" <<'PYEOF'
import json, os, re, sys

root, strict, out_path = sys.argv[1], sys.argv[2] == "1", sys.argv[3]

# ---- 1. the claim list: fuzz target names are the repo's own statement ----
claim_codecs, claim_formats = [], []
fdir = os.path.join(root, "fuzz", "fuzz_targets")
for fn in sorted(os.listdir(fdir)) if os.path.isdir(fdir) else []:
    if not fn.endswith(".rs"):
        continue
    n = fn[:-3]
    if n.startswith("fuzz_codec_"):
        claim_codecs.append(n[len("fuzz_codec_"):])
    elif n.startswith("fuzz_demux_"):
        claim_formats.append(n[len("fuzz_demux_"):])

ALIAS = {
    "mp3stream": ["mp3", "mp2"],
    "es":        ["mp3", "mp2", "aac"],
    "adts":      ["aac"],
    "flac":      ["flac"],
    "av1":       ["av1", "ivf"],
}

# ---- 2. the evidence: every media file in the tree ----
EXT = {".mp4", ".mkv", ".ts", ".ps", ".avi", ".ogg", ".ogv", ".wav", ".aiff",
       ".aif", ".aac", ".ivf", ".flac", ".mp3", ".mp2", ".m4a", ".mov", ".webm"}
fixtures = []
for dp, dns, fns in os.walk(root):
    dns[:] = [d for d in dns if d not in ("target", "node_modules", ".git")]
    for fn in fns:
        if os.path.splitext(fn)[1].lower() in EXT:
            try:
                sz = os.path.getsize(os.path.join(dp, fn))
            except OSError:
                continue
            if sz > 0:
                fixtures.append((os.path.relpath(os.path.join(dp, fn), root).replace("\\", "/"), sz))

# ---- 3. tests that name the format (a second kind of evidence) ----
tests_text = []
for top in ("crates", "apps"):
    base = os.path.join(root, top)
    if not os.path.isdir(base):
        continue
    for dp, dns, fns in os.walk(base):
        dns[:] = [d for d in dns if d not in ("target", "node_modules")]
        for fn in fns:
            if fn.endswith(".rs"):
                try:
                    tests_text.append(open(os.path.join(dp, fn), encoding="utf-8", errors="replace").read())
                except OSError:
                    pass
blob = "\n".join(tests_text).lower()


def find_fixtures(token):
    keys = [token] + ALIAS.get(token, [])
    hits = []
    for path, sz in fixtures:
        low = path.lower()
        if any(("/" + k + "/") in low or low.endswith("." + k) or ("_" + k + ".") in low
               or ("-" + k + ".") in low or ("/" + k + "_") in low for k in keys):
            hits.append((path, sz))
    return hits


L = ["# Format / codec coverage matrix\n"]
L.append("Claims come from the repo's own fuzz targets (`fuzz/fuzz_targets/*.rs`).")
L.append("Evidence is a committed media fixture. A claim with no fixture is")
L.append("**unmeasured** - no speed number, no correctness number, nothing.\n")
L.append("| claimed codec | fixtures | bytes | named in tests | status |")
L.append("|---|---|---|---|---|")
gaps = []
for c in claim_codecs:
    fx = find_fixtures(c)
    named = ("fuzz_codec_" + c) in blob or (c in blob)
    status = "ok" if fx else ("**NO FIXTURE**" if not named else "**NO FIXTURE** (test-only)")
    if not fx:
        gaps.append("codec:" + c)
    L.append("| %s | %d | %s | %s | %s |" % (
        c, len(fx), "{:,}".format(sum(s for _, s in fx)), "yes" if named else "no", status))

L.append("")
L.append("| claimed container | fixtures | bytes | status |")
L.append("|---|---|---|---|")
for f in claim_formats:
    fx = find_fixtures(f)
    if not fx:
        gaps.append("container:" + f)
    L.append("| %s | %d | %s | %s |" % (
        f, len(fx), "{:,}".format(sum(s for _, s in fx)), "ok" if fx else "**NO FIXTURE**"))

L.append("")
L.append("**claims: %d codecs + %d containers | fixtures in tree: %d | gaps: %d**" % (
    len(claim_codecs), len(claim_formats), len(fixtures), len(gaps)))
if gaps:
    L.append("")
    L.append("Unbacked claims:")
    for g in gaps:
        L.append("- " + g)

open(out_path, "w", encoding="utf-8").write("\n".join(L) + "\n")
print("\n".join(L[-12 - len(gaps):]))
sys.exit(1 if (strict and gaps) else 0)
PYEOF

say ""
say "written: $OUT/coverage-matrix/coverage.txt"
