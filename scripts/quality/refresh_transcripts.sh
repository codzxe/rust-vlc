#!/usr/bin/env bash
# Transcript refresh for tests/integration/first-vertical (FX8-A08,
# FIX-PLAN §4 F2.2 — the audit baseline's currency checks FAILED:
# `committed_transcripts_match_fresh_runs` and `final_frame_is_current`
# are red at audit/runs/baseline/cargo-test-workspace.txt:2764-2767, so
# the committed evidence transcripts + final-frame.png + ledger.json
# are STALE), with the exit-101 cure built in: the baseline
# (cargo-test-workspace.txt:2761-2794) ABORTED the package after
# tests/evidence.rs failed — cargo's default fail-fast stopped the run
# BEFORE tests/journey.rs, tests/recovery_matrix.rs and
# tests/wire_mode.rs ever executed, so one red binary hid three whole
# binaries. EVERY cargo test invocation below carries --no-fail-fast:
# one red binary can never hide the others again.
#
# Modes:
#   (default | --write)  the WRITER — regenerates the committed
#       evidence artifacts via the package's own ignored `regen` test
#       (transcripts/*.json + final-frame.png + ledger.json + the
#       2.E1 record). REFUSES TO WRITE on any non-green run (evidence
#       only commits green runs): a pre-flight gate runs the package's
#       BEHAVIOR binaries first — lib + baselines_consistency +
#       class_b + journey + recovery_matrix + wire_mode. The
#       artifact-currency binary (tests/evidence.rs) is deliberately
#       NOT in the pre-flight: its two currency checks describe exactly
#       the artifacts this writer is about to re-render, so gating on
#       them would deadlock the refresh (stale artifacts blocking
#       their own regeneration). The FULL package — currency checks
#       included — is re-run as the post-flight proof, and only a
#       green post-flight retires the STALE-TRANSCRIPTS.md sidecar
#       (while that sidecar exists, its stale declaration is the truth
#       and `--check` fails).
#   --check  the GATE — read-only (never writes evidence/): exit 1 if
#       the STALE-TRANSCRIPTS.md sidecar is present (the committed
#       transcripts are DECLARED stale) or if the evidence binary's
#       currency checks fail on drift.
#   --inventory  NO CARGO AT ALL (python3 stdlib only) — audit the
#       committed artifacts (12 transcripts + final-frame.png +
#       ledger.json + the 2.E1 record) for shape and ledger
#       consistency.
#
# Exit codes: 0 green / 1 the mode's gate failed / 2 usage error.
#
# FIRST_VERTICAL_ROOT overrides the package root (this exists for the
# tamper negative tests: the inventory is run against a copied,
# deliberately tampered evidence tree under /tmp — never the real
# one). Runs from any working directory; cargo always runs from the
# workspace root.
set -u

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "${script_dir}/../.." && pwd)"
pkg_root="${FIRST_VERTICAL_ROOT:-${root}/tests/integration/first-vertical}"
evidence_dir="${pkg_root}/evidence"
sidecar="${evidence_dir}/STALE-TRANSCRIPTS.md"
package="first-vertical"
cli_package="lumen-cli"

say() { printf 'refresh-transcripts: %s\n' "$*"; }
fail() { printf 'refresh-transcripts: FAIL: %s\n' "$*" >&2; }

# Every cargo TEST invocation goes through here: --no-fail-fast is not
# optional — the audit baseline's exit-101 abort (see header) skipped
# three test binaries because one red binary stopped the package.
cargo_test() {
    cargo test --package "${package}" --no-fail-fast "$@"
}

usage() {
    cat >&2 <<'USAGE'
usage: refresh_transcripts.sh [--write|--check|--inventory]
  (default, --write)  regenerate the committed first-vertical evidence
                      (refuses on any non-green run; needs cargo)
  --check             gate: 1 on the stale sidecar or currency drift
  --inventory         audit the committed artifacts (no cargo)
USAGE
    exit 2
}

# ---------------------------------------------------------------------------
# --check: the gate. Read-only: the currency checks only READ evidence/
# (the package's own design — "a gate run never dirties the tree").
# ---------------------------------------------------------------------------
check_mode() {
    if [ -f "${sidecar}" ]; then
        fail "the committed transcripts are DECLARED STALE by"
        fail "  ${sidecar}"
        say "  cure (the central operator): bash scripts/quality/refresh_transcripts.sh"
        exit 1
    fi
    cd "${root}" || { fail "cannot cd ${root}"; exit 2; }
    say "check: building ${cli_package} (the wire-session scenario spawns target/debug/lumen)"
    if ! cargo build --package "${cli_package}"; then
        fail "the ${cli_package} build failed — a gate run needs the wire binary"
        exit 1
    fi
    say "check: the evidence currency checks (read-only)"
    if cargo_test --test evidence; then
        say "check: OK — no stale sidecar, the committed transcripts are current"
        exit 0
    fi
    fail "currency drift (or a red evidence check) — regenerate with the writer:"
    say "    bash scripts/quality/refresh_transcripts.sh"
    exit 1
}

# ---------------------------------------------------------------------------
# (default, --write): the writer. Evidence only commits green runs, so
# the refusal comes FIRST (pre-flight), the write only on green, and the
# proof LAST (post-flight full run).
# ---------------------------------------------------------------------------
writer_mode() {
    cd "${root}" || { fail "cannot cd ${root}"; exit 2; }
    say "writer: building ${cli_package} (the wire-session scenario spawns target/debug/lumen)"
    if ! cargo build --package "${cli_package}"; then
        fail "the ${cli_package} build failed — the wire scenario needs the binary"
        exit 1
    fi
    say "writer: pre-flight — the package's behavior binaries must be ALL green"
    say "  (lib + baselines_consistency + class_b + journey + recovery_matrix +"
    say "   wire_mode; the artifact-currency binary tests/evidence.rs is excluded"
    say "   BY DESIGN — its checks describe the artifacts this writer re-renders)"
    if ! cargo_test --lib \
        --test baselines_consistency \
        --test class_b \
        --test journey \
        --test recovery_matrix \
        --test wire_mode; then
        fail "pre-flight NOT green — refusing to write (evidence only commits green runs)"
        say "  fix the red scenario(s), then re-run this writer"
        exit 1
    fi
    say "writer: pre-flight green — regenerating via the package's own ignored regen test"
    say "  (the regen test itself re-runs every scenario and refuses on any red check)"
    if ! cargo_test --test evidence -- --ignored regen; then
        fail "the regen test failed — nothing was left half-written (it writes only after every scenario answered green)"
        exit 1
    fi
    say "writer: post-flight — the FULL package run (currency checks included) must be green"
    if ! cargo_test; then
        fail "post-flight NOT green — the artifacts were regenerated but the package is red;"
        fail "  investigate the failures above (do not blindly re-run the writer)"
        exit 1
    fi
    if [ -f "${sidecar}" ]; then
        if ! rm "${sidecar}"; then
            fail "cannot retire the stale sidecar ${sidecar} — remove it by hand"
            exit 1
        fi
        say "writer: retired the stale-transcripts sidecar — the transcripts are fresh now, the stale declaration would be false"
    fi
    say "writer: OK — 12 transcripts + final-frame.png + ledger.json + the 2.E1 record regenerated, full package green"
}

# ---------------------------------------------------------------------------
# --inventory: audit the committed artifacts. No cargo (python3 stdlib
# only): shape checks on the transcripts + the PNG, and the ledger's
# sha256/byte-count claims re-derived from the actual files.
# ---------------------------------------------------------------------------
inventory_mode() {
    python3 - "${evidence_dir}" "${pkg_root}" <<'PY'
import hashlib
import json
import os
import re
import sys

evidence, pkg_root = sys.argv[1], sys.argv[2]
problems = []


def ok(label):
    print(f"  [ok]   {label}")


def bad(label, why):
    print(f"  [FAIL] {label} — {why}")
    problems.append(label)


print(f"inventory: evidence dir {evidence}")

# --- the suite's own scenario id list (source cross-check, no cargo) ---
scenario_ids = []
src_list = os.path.join(pkg_root, "src", "scenarios.rs")
try:
    with open(src_list, encoding="utf-8") as fh:
        text = fh.read()
    block = re.search(r"pub const SCENARIO_IDS: &\[&str\] = &\[(.*?)\];", text, re.S)
    if block is None:
        bad("src/scenarios.rs", "SCENARIO_IDS const not found")
    else:
        scenario_ids = re.findall(r'"([^"]+)"', block.group(1))
        ok(f"src/scenarios.rs SCENARIO_IDS: {len(scenario_ids)} ids")
except OSError as exc:
    bad("src/scenarios.rs", f"unreadable: {exc}")
if scenario_ids and len(scenario_ids) != 12:
    bad("scenario id count", f"{len(scenario_ids)} ids, expected 12")

# --- ledger.json: parse, schema, scenario table ---
ledger = None
ledger_path = os.path.join(evidence, "ledger.json")
try:
    with open(ledger_path, "rb") as fh:
        ledger_bytes = fh.read()
    ledger = json.loads(ledger_bytes.decode("utf-8"))
    if ledger.get("schema") == "evidence/1":
        ok(f"ledger.json: parses, schema evidence/1 ({len(ledger_bytes)} B)")
    else:
        bad("ledger.json schema", f"{ledger.get('schema')!r} != 'evidence/1'")
except (OSError, ValueError) as exc:
    bad("ledger.json", f"unreadable/unparseable: {exc}")

table = ledger.get("scenario_table", []) if isinstance(ledger, dict) else []
if table:
    red = [row.get("id") for row in table if row.get("verdict") != "pass"]
    if red:
        bad("ledger scenario verdicts", f"non-pass rows: {red}")
    else:
        ok(f"ledger scenario_table: {len(table)} rows, every verdict pass")
    ledger_ids = [row.get("id") for row in table]
    if scenario_ids and ledger_ids != scenario_ids:
        bad("ledger scenario ids", f"{ledger_ids} != the suite's {scenario_ids}")
else:
    bad("ledger scenario_table", "missing/empty")

# --- transcripts: exactly the expected files, the right shape ---
troot = os.path.join(evidence, "transcripts")
expected = {f"{i}.json" for i in scenario_ids}
try:
    present = {name for name in os.listdir(troot) if name.endswith(".json")}
except OSError as exc:
    present = set()
    bad("evidence/transcripts/", f"unreadable: {exc}")
if expected and present != expected:
    bad(
        "evidence/transcripts/ file set",
        f"missing: {sorted(expected - present) or '-'}; extra: {sorted(present - expected) or '-'}",
    )

for name in sorted(expected & present):
    path = os.path.join(troot, name)
    try:
        with open(path, "rb") as fh:
            raw = fh.read()
        doc = json.loads(raw.decode("utf-8"))
    except (OSError, ValueError) as exc:
        bad(f"transcripts/{name}", f"unreadable/unparseable: {exc}")
        continue
    why = None
    if doc.get("schema") != "transcript/1":
        why = f"schema {doc.get('schema')!r} != 'transcript/1'"
    elif doc.get("scenario") != name[: -len(".json")]:
        why = f"scenario {doc.get('scenario')!r} != the file name"
    else:
        labels = doc.get("labels", {})
        if labels.get("hardware_class") != "mock":
            why = f"hardware_class {labels.get('hardware_class')!r} != 'mock'"
        elif labels.get("devices") != ["null-audio", "software-render"]:
            why = f"devices {labels.get('devices')!r} != the null-audio/software-render pair"
        elif not isinstance(labels.get("environment_kind"), str) or not labels.get(
            "environment_kind"
        ):
            why = "environment_kind missing/empty (the producing-machine label)"
        elif not isinstance(labels.get("os"), str) or not labels.get("os"):
            why = "os missing/empty (the producing-machine label)"
    if why:
        bad(f"transcripts/{name}", why)
    else:
        ok(f"transcripts/{name}: {len(raw)} B, schema/scenario/labels in shape")

# --- the ledger's artifact claims: sha256 + byte count, re-derived ---
if isinstance(ledger, dict):
    arts = ledger.get("artifacts", [])
    claimed = {}
    for row in arts:
        claimed[row.get("path")] = (row.get("bytes"), row.get("sha256"))
    expected_paths = {f"evidence/transcripts/{i}.json" for i in scenario_ids}
    expected_paths.add("evidence/final-frame.png")
    if expected_paths and set(claimed) != expected_paths:
        bad(
            "ledger artifacts table",
            f"missing: {sorted(expected_paths - set(claimed)) or '-'};"
            f" extra: {sorted(set(claimed) - expected_paths) or '-'}",
        )
    for rel, (size, sha) in sorted(claimed.items()):
        path = os.path.join(pkg_root, rel)
        try:
            with open(path, "rb") as fh:
                raw = fh.read()
        except OSError as exc:
            bad(rel, f"unreadable: {exc}")
            continue
        digest = hashlib.sha256(raw).hexdigest()
        if size == len(raw) and sha == digest:
            ok(f"{rel}: {len(raw)} B, ledger sha256/bytes match")
        else:
            bad(
                rel,
                f"ledger says {size} B/{sha}, file is {len(raw)} B/{digest}",
            )

# --- final-frame.png: the PNG signature + the IHDR extent ---
png = os.path.join(evidence, "final-frame.png")
try:
    with open(png, "rb") as fh:
        head = fh.read(24)
    if head[:8] == b"\x89PNG\r\n\x1a\n":
        w = int.from_bytes(head[16:20], "big")
        h = int.from_bytes(head[20:24], "big")
        if (w, h) == (320, 240):
            ok("final-frame.png: PNG signature + IHDR 320x240 (the media's extent)")
        else:
            bad("final-frame.png IHDR", f"{w}x{h} != 320x240")
    else:
        bad("final-frame.png", "not a PNG (signature mismatch)")
except OSError as exc:
    bad("final-frame.png", f"unreadable: {exc}")

# --- the 2.E1 record: present, non-empty, self-named ---
record = os.path.join(evidence, "2.E1-first-vertical.txt")
try:
    with open(record, "rb") as fh:
        raw = fh.read()
    if raw and b"2.E1-first-vertical" in raw:
        ok(f"2.E1-first-vertical.txt: {len(raw)} B, present and non-empty")
    else:
        bad("2.E1-first-vertical.txt", "empty or missing its own name")
except OSError as exc:
    bad("2.E1-first-vertical.txt", f"unreadable: {exc}")

# --- the stale sidecar: reported either way (it is --check that fails on it) ---
sidecar = os.path.join(evidence, "STALE-TRANSCRIPTS.md")
if os.path.isfile(sidecar):
    print("  [note] STALE-TRANSCRIPTS.md PRESENT — the committed transcripts are")
    print("         declared stale; scripts/quality/refresh_transcripts.sh --check")
    print("         will fail (by design) until the central writer run retires it")
else:
    print("  [ok]   no STALE-TRANSCRIPTS.md sidecar")

if problems:
    print(f"inventory: FAIL — {len(problems)} problem(s) above")
    sys.exit(1)
print("inventory: OK — 12 transcripts + final-frame.png + ledger.json + the 2.E1 record are committed and consistent")
PY
    # The heredoc's python exit status is the mode's verdict.
    return $?
}

case "${1:-}" in
    "") writer_mode ;;
    --write) writer_mode ;;
    --check) check_mode ;;
    --inventory) inventory_mode ;;
    -h|--help) usage ;;
    *) usage ;;
esac
