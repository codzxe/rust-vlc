#!/usr/bin/env python3
"""The status-truth generator (FX8-A01, FIX-PLAN §1 P1–P2 + §4 F2.1).

ONE status layer, parsed from source — never from docs. The audit's
process finding (DEVIL-AUDIT-REPORT §4.4/§5.1: nine hand-edited status
layers drifting in both directions, four decoder registries in one
binary answering 2/16/12/"none") is ended mechanically: every row this
generator emits is derived from the code the shipped binary runs, and
any source shape it can no longer classify is a hard failure (exit 2) —
it never silently understates.

Sources parsed (truth from source, never docs):

* ``apps/cli/src/pipeline_host.rs`` — the decoder dispatch: the codec
  arms of ``HostDecoderProvider::decoder_for``, cross-checked 1:1
  against ``LANDED_DECODER_FAMILIES`` (the registry FX2-A03 exported as
  data beside the match it mirrors). Either side drifting alone is a
  shape failure, not a quieter row.
* ``apps/cli/src/host.rs`` — the host verb handlers: every
  ``pub(crate) fn`` plus the module-level ``pub fn`` handlers
  (``engine_info`` / ``command_listings``). Plain ``pub fn`` *methods*
  (``settle``/``pump``/``track_snapshot``/...) are the host's plumbing
  and test surface, deliberately out of this section — the catalog
  section below carries the command surface they would duplicate.
* ``apps/cli/src/dispatch.rs`` — the wiring truth: ``route()``'s
  non-refusal ``Command`` arms (``wired``) + the ``SERVED`` table,
  cross-checked equal — a served-but-unrouted command is a shape
  failure, never an understated row.
* ``crates/commands/src/catalog/command.rs`` — the ``Command`` payload
  enum: one variant per catalog command (the 69-command catalog).
* ``crates/commands/src/catalog/ids.rs`` — the ``CommandId``
  stable-id twin, cross-checked 1:1 against the ``Command`` variants
  (the ``id`` column and the enum cannot drift apart silently).

Semantics of the emitted columns:

* ``decoder_family.shipped`` — the family's descriptors are accepted
  by live codec arms of the one dispatch (the registry). Always true
  today by construction: a registry row without an arm (or an arm
  without a registry row) fails the build instead of emitting a row.
* ``host_verb.wired`` — the handler is called from the dispatch layer
  (``apps/cli/src/dispatch.rs``, the single command entry ARCH-5).
  ``false`` means the fn lives in ``host.rs`` but no verb reaches it
  through that entry (e.g. an event-shaping helper used host-internally).
* ``catalog_command.wired`` — the variant is routed (not refused) by
  ``route()``; every unwired command answers the typed
  ``unsupported-for-host`` refusal.

Output: TOML on stdout — deterministic (every section sorted, no
timestamps, no environment), idempotent (same tree, same bytes).
Self-validated: the emitted TOML is re-parsed (``tomllib``, python
>= 3.11) and its summary counts re-checked before stdout sees it.

Exit codes:

* ``0`` — success.
* ``2`` — source-shape failure: a file moved, a shape drifted, or a
  cross-check broke. The truth layer refuses to guess rather than
  understate; fix the source or teach this parser the new shape.

Regenerate the committed artifact:

    python3 scripts/quality/status_truth.py > docs/generated/status-truth.toml

Gate (CI): ``bash scripts/quality/status_truth_check.sh`` — exit 1 on
drift vs the committed file, exit 2 propagating this generator's shape
failures. Python 3 stdlib only; no cargo anywhere.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

PIPELINE_HOST = ROOT / "apps" / "cli" / "src" / "pipeline_host.rs"
HOST_RS = ROOT / "apps" / "cli" / "src" / "host.rs"
DISPATCH_RS = ROOT / "apps" / "cli" / "src" / "dispatch.rs"
COMMAND_RS = ROOT / "crates" / "commands" / "src" / "catalog" / "command.rs"
IDS_RS = ROOT / "crates" / "commands" / "src" / "catalog" / "ids.rs"

REL_PIPELINE_HOST = "apps/cli/src/pipeline_host.rs"
REL_HOST = "apps/cli/src/host.rs"
REL_DISPATCH = "apps/cli/src/dispatch.rs"
REL_COMMAND = "crates/commands/src/catalog/command.rs"
REL_IDS = "crates/commands/src/catalog/ids.rs"

KINDS = ("Video", "Audio", "Subtitle", "Data")

TEST_ATTR = re.compile(r"^\s*#\[\s*cfg\s*\(\s*test\s*\)\s*\]")

# A public fn line the verb scan must either capture or reject loudly
# (async/unsafe/const/extern handler shapes are not classified today).
FN_VIS = re.compile(
    r"^\s*pub(\(crate\))?\s+(?:(?:async|unsafe|const|extern(?:\s*\"[^\"]*\")?)\s+)*fn\b"
)
HANDLER_PUB_CRATE = re.compile(r"^\s*pub\(crate\)\s+fn\s+([A-Za-z_]\w*)\s*\(")
HANDLER_FREE_PUB = re.compile(r"^pub\s+fn\s+([A-Za-z_]\w*)\s*\(")
PUB_METHOD_PLUMBING = re.compile(r"^\s+pub\s+fn\b")


class ShapeError(Exception):
    """A source shape this generator can no longer classify (exit 2)."""


def fail(anchor: str, message: str) -> None:
    raise ShapeError(f"{anchor}: {message}")


def read_lines(path: Path, rel: str) -> list[tuple[int, str]]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        fail(rel, f"cannot read ({exc})")
    return list(enumerate(text.splitlines(), start=1))


def find_one(lines: list[tuple[int, str]], pattern: re.Pattern[str], rel: str, what: str) -> int:
    """The unique line matching ``pattern`` (1-based line number)."""
    hits = [lineno for lineno, text in lines if pattern.match(text)]
    if not hits:
        fail(rel, f"{what} not found — the source shape drifted; teach the parser")
    if len(hits) > 1:
        fail(rel, f"{what} found more than once (lines {hits}) — shape not classifiable")
    return hits[0]


# ---------------------------------------------------------------------------
# A. The decoder dispatch (pipeline_host.rs)
# ---------------------------------------------------------------------------

IMPL_PROVIDER = re.compile(r"^impl\s+DecoderProvider\s+for\s+HostDecoderProvider\s*\{")
FN_DECODER_FOR = re.compile(r"^\s*fn\s+decoder_for\s*\(")
MATCH_LINE = re.compile(r"^(\s*)match\b.*\{\s*$")
GUARD_ARM = re.compile(r'^([A-Za-z_]\w*)\s+if\s+\1\.starts_with\("([^"]+)"\)\s*=>')
BARE_ARM = re.compile(r"^([A-Za-z_]\w*|_)\s*=>")
REGISTRY_CONST = re.compile(r"^pub\s+const\s+LANDED_DECODER_FAMILIES\b")
ROW_NAME = re.compile(r'^name:\s*"([^"]+)",?$')
ROW_KIND = re.compile(r"^kind:\s*MediaKind::(\w+),?$")
ROW_DESCRIPTORS = re.compile(r"^descriptors:\s*&\[(.*)\],?$")


def parse_decoder_arms(lines: list[tuple[int, str]]) -> list[dict]:
    """The codec arms of ``decoder_for``'s match (the one registry)."""
    impl = find_one(lines, IMPL_PROVIDER, REL_PIPELINE_HOST,
                    "`impl DecoderProvider for HostDecoderProvider`")
    fn = find_one(lines, FN_DECODER_FOR, REL_PIPELINE_HOST,
                  "`fn decoder_for` (the decoder dispatch)")
    if fn < impl:
        fail(REL_PIPELINE_HOST, f"`fn decoder_for` at :{fn} precedes its `impl` at :{impl}")
    match_idx = None
    match_indent = None
    for lineno, text in lines[fn:]:
        m = MATCH_LINE.match(text)
        if m:
            match_idx, match_indent = lineno, len(m.group(1))
            break
    if match_idx is None:
        fail(REL_PIPELINE_HOST,
             f"`fn decoder_for` at :{fn} contains no `match` — dispatch shape not classifiable")
    arm_indent = match_indent + 4
    close_line = " " * match_indent + "}"
    arms: list[dict] = []
    catchall_line = None
    for lineno, text in lines[match_idx:]:
        if text == close_line:
            break
        stripped = text.strip()
        if not stripped or stripped.startswith("//"):
            continue
        indent = len(text) - len(text.lstrip(" "))
        if indent > arm_indent:
            continue  # an arm body's interior lines
        if indent != arm_indent:
            fail(f"{REL_PIPELINE_HOST}:{lineno}",
                 f"line inside the dispatch match at indent {indent} (arm indent {arm_indent}): {stripped!r}")
        if stripped.startswith("#["):
            fail(f"{REL_PIPELINE_HOST}:{lineno}",
                 f"attribute on a dispatch arm ({stripped!r}) — a conditional arm's shipped-ness is not classifiable")
        if re.fullmatch(r"[})\];,]+", stripped):
            continue  # the previous arm's multi-line expression closer
        if stripped.startswith('"'):
            if "=>" not in stripped:
                fail(f"{REL_PIPELINE_HOST}:{lineno}",
                     f"multi-line literal arm pattern not classifiable: {stripped!r}")
            patterns = re.findall(r'"([^"]+)"', stripped.split("=>", 1)[0])
            if not patterns:
                fail(f"{REL_PIPELINE_HOST}:{lineno}", f"literal arm with no patterns: {stripped!r}")
            arms.append({"kind": "literal", "values": patterns, "line": lineno})
            continue
        guard = GUARD_ARM.match(stripped)
        if guard:
            arms.append({"kind": "prefix", "values": [guard.group(2)], "line": lineno})
            continue
        if BARE_ARM.match(stripped):
            if catchall_line is not None:
                fail(f"{REL_PIPELINE_HOST}:{lineno}",
                     f"a second bare-binding arm (first at :{catchall_line}) — at most one catch-all is classifiable")
            catchall_line = lineno
            continue
        fail(f"{REL_PIPELINE_HOST}:{lineno}", f"unclassifiable dispatch arm: {stripped!r}")
    if catchall_line is None:
        fail(REL_PIPELINE_HOST,
             "the dispatch match has no catch-all arm (`cli.codec_unavailable`) — shape changed")
    if not arms:
        fail(REL_PIPELINE_HOST, "the dispatch match has no codec arms — the decoder truth would be empty")
    if catchall_line < arms[-1]["line"]:
        fail(REL_PIPELINE_HOST, "the catch-all arm is not the last arm — shape not classifiable")
    return arms


def parse_landed_registry(lines: list[tuple[int, str]]) -> list[dict]:
    """The LANDED_DECODER_FAMILIES rows (the registry FX2-A03 exported)."""
    const = find_one(lines, REGISTRY_CONST, REL_PIPELINE_HOST, "`pub const LANDED_DECODER_FAMILIES`")
    start = None
    for _, text in lines[const - 1: const + 3]:
        if text.rstrip().endswith("= &["):
            start = const
            break
        if text.strip() and not text.strip().startswith("//"):
            break
    if start is None:
        fail(REL_PIPELINE_HOST,
             "the LANDED_DECODER_FAMILIES const does not open with `= &[` — shape not classifiable")
    rows: list[dict] = []
    row: dict | None = None
    for lineno, text in lines[start:]:
        stripped = text.strip()
        if stripped == "];":
            if row is not None:
                fail(f"{REL_PIPELINE_HOST}:{lineno}", "a registry row is unterminated at the table close")
            if not rows:
                fail(REL_PIPELINE_HOST, "the LANDED_DECODER_FAMILIES table is empty")
            return rows
        if not stripped or stripped.startswith("//"):
            continue
        if row is None:
            if stripped != "DecoderFamily {":
                fail(f"{REL_PIPELINE_HOST}:{lineno}", f"unclassifiable registry entry: {stripped!r}")
            row = {"line": lineno, "name": None, "kind": None, "descriptors": None}
            continue
        if stripped == "},":
            if row["name"] is None or row["kind"] is None or row["descriptors"] is None:
                fail(f"{REL_PIPELINE_HOST}:{row['line']}", "a registry row is missing name/kind/descriptors")
            if not row["descriptors"]:
                fail(f"{REL_PIPELINE_HOST}:{row['line']}", "a registry row lists no descriptors")
            rows.append(row)
            row = None
            continue
        field = ROW_NAME.match(stripped)
        if field:
            row["name"] = field.group(1)
            continue
        field = ROW_KIND.match(stripped)
        if field:
            if field.group(1) not in KINDS:
                fail(f"{REL_PIPELINE_HOST}:{lineno}", f"unknown MediaKind in a registry row: {field.group(1)!r}")
            row["kind"] = field.group(1)
            continue
        field = ROW_DESCRIPTORS.match(stripped)
        if field:
            row["descriptors"] = re.findall(r'"([^"]+)"', field.group(1))
            continue
        fail(f"{REL_PIPELINE_HOST}:{lineno}", f"unclassifiable registry row field: {stripped!r}")
    fail(REL_PIPELINE_HOST, "the LANDED_DECODER_FAMILIES table is unterminated (no `];`)")


def decoder_rows() -> list[dict]:
    lines = read_lines(PIPELINE_HOST, REL_PIPELINE_HOST)
    arms = parse_decoder_arms(lines)
    rows = parse_landed_registry(lines)
    if len(arms) != len(rows):
        fail(REL_PIPELINE_HOST,
             f"{len(arms)} dispatch codec arms vs {len(rows)} LANDED_DECODER_FAMILIES rows — "
             "the match and its registry have drifted (an arm without a row, or a row without an arm)")
    names = [row["name"] for row in rows]
    if len(set(names)) != len(names):
        fail(REL_PIPELINE_HOST, "duplicate family names in LANDED_DECODER_FAMILIES")
    descriptor_row: dict[str, int] = {}
    for i, row in enumerate(rows):
        for desc in row["descriptors"]:
            if desc in descriptor_row:
                fail(REL_PIPELINE_HOST,
                     f"descriptor {desc!r} appears in two registry rows "
                     f"(:{rows[descriptor_row[desc]]['line']}, :{row['line']})")
            descriptor_row[desc] = i
    literal_arms = [arm for arm in arms if arm["kind"] == "literal"]
    prefix_arms = [arm for arm in arms if arm["kind"] == "prefix"]
    row_arms: dict[int, list[dict]] = {}
    for arm in arms:
        if arm["kind"] == "literal":
            targets = {descriptor_row.get(value) for value in arm["values"]}
        else:
            prefix = arm["values"][0]
            targets = {i for i, row in enumerate(rows)
                       if row["descriptors"] and all(desc.startswith(prefix) for desc in row["descriptors"])}
        targets.discard(None)
        if len(targets) != 1:
            fail(f"{REL_PIPELINE_HOST}:{arm['line']}",
                 "this dispatch arm does not map to exactly one registry row — "
                 "an arm's patterns must be one family's descriptors")
        row_arms.setdefault(targets.pop(), []).append(arm)
    if len(row_arms) != len(rows) or any(len(a) != 1 for a in row_arms.values()):
        fail(REL_PIPELINE_HOST, "the dispatch arms and the registry rows are not 1:1")
    for row in rows:
        for desc in row["descriptors"]:
            landed = any(desc in arm["values"] for arm in literal_arms) or any(
                desc.startswith(arm["values"][0]) for arm in prefix_arms)
            if not landed:
                fail(REL_PIPELINE_HOST,
                     f"registry descriptor {desc!r} has no dispatch arm — the registry overstates the build")
    out = []
    for i, row in enumerate(rows):
        arm = row_arms[i][0]
        out.append({"name": row["name"], "shipped": True,
                    "source": f"{REL_PIPELINE_HOST}:{arm['line']}"})
    return sorted(out, key=lambda r: r["name"])


# ---------------------------------------------------------------------------
# B. The host verb handlers (host.rs, wired via dispatch.rs)
# ---------------------------------------------------------------------------

def host_verb_rows() -> list[dict]:
    host_lines = read_lines(HOST_RS, REL_HOST)
    try:
        dispatch_text = DISPATCH_RS.read_text(encoding="utf-8")
    except OSError as exc:
        fail(REL_DISPATCH, f"cannot read ({exc})")
    if "pub fn lumen_dispatch" not in dispatch_text:
        fail(REL_DISPATCH,
             "the single dispatch entry (`pub fn lumen_dispatch`) is missing — wiring truth not derivable")
    # The wiring truth is the SHIPPED dispatch layer; its #[cfg(test)]
    # module is not a call site that makes a handler wired.
    shipped_dispatch = dispatch_text.split("#[cfg(test)]", 1)[0]
    handlers: dict[str, int] = {}
    test_from = None
    for lineno, text in host_lines:
        if test_from is None:
            if TEST_ATTR.match(text):
                test_from = lineno
                continue
            m = HANDLER_PUB_CRATE.match(text)
            if m:
                handlers[m.group(1)] = lineno
                continue
            m = HANDLER_FREE_PUB.match(text)
            if m:
                handlers[m.group(1)] = lineno
                continue
            if FN_VIS.match(text) and not PUB_METHOD_PLUMBING.match(text):
                fail(f"{REL_HOST}:{lineno}",
                     f"unclassifiable public handler shape: {text.strip()!r} "
                     "(async/unsafe/const are not parsed)")
        elif FN_VIS.match(text):
            fail(f"{REL_HOST}:{lineno}",
                 f"a public fn after the first #[cfg(test)] module (:{test_from}): {text.strip()!r} — "
                 "not classifiable as shipped handler or test code")
    if not handlers:
        fail(REL_HOST,
             "no verb handlers found (pub(crate) fn / module-level pub fn) — host.rs shape drifted")
    out = []
    for name, lineno in handlers.items():
        call = re.compile(rf"\b{re.escape(name)}\s*\(")
        out.append({"name": name, "wired": bool(call.search(shipped_dispatch)),
                    "source": f"{REL_HOST}:{lineno}"})
    return sorted(out, key=lambda r: r["name"])


# ---------------------------------------------------------------------------
# C. The command catalog (command.rs + ids.rs, wired via dispatch.rs)
# ---------------------------------------------------------------------------

CATALOG_ENUM = re.compile(r"^pub\s+enum\s+Command\s*\{$")
VARIANT_LINE = re.compile(r"^([A-Z][A-Za-z0-9]*)(?:\s*\{|\s*\(|,)?\s*$")
IDS_ENUM = re.compile(r"^\s*pub\s+enum\s+CommandId\s*\{\s*$")
ID_ROW = re.compile(r'^([A-Z][A-Za-z0-9]*)\s*=>\s*"([^"]+)"\s*,?$')
ROUTE_FN = re.compile(r"^fn\s+route\(")
SERVED_CONST = re.compile(r"^pub\s+const\s+SERVED\b")
ROUTE_ARM = re.compile(r"Command::([A-Z][A-Za-z0-9]*)")
SERVED_ENTRY = re.compile(r"CommandId::([A-Z][A-Za-z0-9]*)")
CATCHALL_ARM = re.compile(r"^\s*other\s*=>", re.MULTILINE)


def parse_command_variants(lines: list[tuple[int, str]]) -> dict[str, int]:
    enum_at = find_one(lines, CATALOG_ENUM, REL_COMMAND, "the command catalog enum (`pub enum Command`)")
    variants: dict[str, int] = {}
    ended = False
    for lineno, text in lines[enum_at:]:
        if text == "}":
            ended = True
            break
        stripped = text.strip()
        if not stripped or stripped.startswith("//"):
            continue
        indent = len(text) - len(text.lstrip(" "))
        if indent == 0:
            fail(f"{REL_COMMAND}:{lineno}", f"unclassifiable catalog line at column 0: {stripped!r}")
        if indent != 4:
            if indent < 4:
                fail(f"{REL_COMMAND}:{lineno}",
                     f"catalog line at indent {indent} (variants live at 4): {stripped!r}")
            continue  # a braced variant's payload fields live deeper
        if stripped.startswith("#["):
            if stripped.startswith("#[cfg"):
                fail(f"{REL_COMMAND}:{lineno}",
                     f"#[cfg] attribute on a catalog variant ({stripped!r}) — "
                     "a conditional command's truth is not classifiable")
            continue
        if re.fullmatch(r"\}[,;]?", stripped):
            continue  # a braced variant's closing brace
        m = VARIANT_LINE.match(stripped)
        if m:
            if m.group(1) in variants:
                fail(f"{REL_COMMAND}:{lineno}", f"duplicate catalog variant {m.group(1)!r}")
            variants[m.group(1)] = lineno
            continue
        fail(f"{REL_COMMAND}:{lineno}", f"unclassifiable catalog variant line: {stripped!r}")
    if not ended:
        fail(REL_COMMAND, "the Command enum is unterminated (no closing `}` at column 0)")
    if not variants:
        fail(REL_COMMAND, "the Command enum parsed to zero variants — catalog shape drifted")
    return variants


def parse_command_ids(lines: list[tuple[int, str]]) -> dict[str, str]:
    enum_at = find_one(lines, IDS_ENUM, REL_IDS, "the stable-id table (`pub enum CommandId`)")
    ids: dict[str, str] = {}
    ended = False
    for lineno, text in lines[enum_at:]:
        if re.fullmatch(r"\s*\}\s*", text):
            ended = True
            break
        stripped = text.strip()
        if not stripped or stripped.startswith("//"):
            continue
        m = ID_ROW.match(stripped)
        if m:
            if m.group(1) in ids:
                fail(f"{REL_IDS}:{lineno}", f"duplicate CommandId variant {m.group(1)!r}")
            ids[m.group(1)] = m.group(2)
            continue
        fail(f"{REL_IDS}:{lineno}", f"unclassifiable stable-id row: {stripped!r}")
    if not ended:
        fail(REL_IDS, "the CommandId table is unterminated (no closing `}`)")
    if not ids:
        fail(REL_IDS, "the CommandId table parsed to zero entries — shape drifted")
    return ids


def parse_route(lines: list[tuple[int, str]]) -> set[str]:
    route_at = find_one(lines, ROUTE_FN, REL_DISPATCH, "`fn route` (the wiring table)")
    end = None
    for i in range(route_at, len(lines)):
        if lines[i][1] == "}":
            end = i
            break
    if end is None:
        fail(REL_DISPATCH, "fn route() is unterminated (no closing `}` at column 0)")
    region = "\n".join(text for _, text in lines[route_at:end])
    # Comment-stripped: a doc line mentioning Command::X must not mint wiring.
    region = "\n".join(line.split("//", 1)[0] for line in region.splitlines())
    routed = set(ROUTE_ARM.findall(region))
    if not routed:
        fail(REL_DISPATCH, "route() routes no Command variants — wiring truth would be empty")
    if not CATCHALL_ARM.search(region):
        fail(REL_DISPATCH,
             "route() has no catch-all refusal arm (`other =>`) — the wired/refused split is not classifiable")
    return routed


def parse_served(lines: list[tuple[int, str]]) -> set[str]:
    served_at = find_one(lines, SERVED_CONST, REL_DISPATCH, "the SERVED table")
    entries: list[str] = []
    ended = False
    for lineno, text in lines[served_at:]:
        stripped = text.strip()
        if stripped == "];":
            ended = True
            break
        if not stripped or stripped.startswith("//"):
            continue
        if not SERVED_ENTRY.search(stripped):
            fail(f"{REL_DISPATCH}:{lineno}", f"unclassifiable SERVED row: {stripped!r}")
        entries.extend(SERVED_ENTRY.findall(stripped))
    if not ended:
        fail(REL_DISPATCH, "the SERVED table is unterminated (no `];`)")
    if not entries:
        fail(REL_DISPATCH, "the SERVED table is empty")
    return set(entries)


def catalog_rows() -> list[dict]:
    variants = parse_command_variants(read_lines(COMMAND_RS, REL_COMMAND))
    ids = parse_command_ids(read_lines(IDS_RS, REL_IDS))
    dispatch_lines = read_lines(DISPATCH_RS, REL_DISPATCH)
    routed = parse_route(dispatch_lines)
    served = parse_served(dispatch_lines)
    if set(ids) != set(variants):
        only_enum = sorted(set(variants) - set(ids))
        only_ids = sorted(set(ids) - set(variants))
        fail(REL_IDS,
             f"the Command payload enum and the CommandId table have drifted "
             f"(only in Command: {only_enum}; only in CommandId: {only_ids})")
    seen: set[str] = set()
    for wire_id in ids.values():
        if wire_id in seen:
            fail(REL_IDS, f"duplicate stable id {wire_id!r}")
        seen.add(wire_id)
    if routed != served:
        fail(REL_DISPATCH,
             "route()'s wired set and the SERVED table disagree "
             f"(routed-not-served: {sorted(routed - served)}; "
             f"served-not-routed: {sorted(served - routed)})")
    unknown = routed - set(variants)
    if unknown:
        fail(REL_DISPATCH, f"route() routes variants missing from the Command enum: {sorted(unknown)}")
    out = []
    for variant in variants:
        out.append({"name": variant, "id": ids[variant], "wired": variant in routed,
                    "source": f"{REL_COMMAND}:{variants[variant]}"})
    return sorted(out, key=lambda r: r["id"])


# ---------------------------------------------------------------------------
# Emission (deterministic: sorted sections, no timestamps, no environment)
# ---------------------------------------------------------------------------

HEADER = """\
# GENERATED FILE — the status truth, parsed from source by
# scripts/quality/status_truth.py. NEVER hand-edit this file: every
# hand-edited status layer in this repo drifted (the four-registry lie
# answered 2/16/12/"none" for the same question — DEVIL-AUDIT-REPORT §3.3).
#
# Regen: python3 scripts/quality/status_truth.py > docs/generated/status-truth.toml
# Gate:  bash scripts/quality/status_truth_check.sh
#        (exit 1 = drift vs this committed file; exit 2 = source-shape
#         failure — the generator never silently understates)
#
# Sources parsed (truth from source, never docs):
#   apps/cli/src/pipeline_host.rs           — the decoder dispatch (codec arms
#                                             + LANDED_DECODER_FAMILIES, cross-checked 1:1)
#   apps/cli/src/host.rs                    — the host verb handlers
#                                             (pub(crate) fn + module-level pub fn)
#   apps/cli/src/dispatch.rs                — the wiring truth (route() arms +
#                                             the SERVED table, cross-checked equal)
#   crates/commands/src/catalog/command.rs  — the Command payload enum (the catalog)
#   crates/commands/src/catalog/ids.rs      — the CommandId stable-id twin (cross-checked 1:1)
#
# Column semantics:
#   decoder_family.shipped — the family's descriptors are accepted by live
#     codec arms of HostDecoderProvider::decoder_for (the one registry).
#   host_verb.wired — the handler is called from the dispatch layer (the
#     single command entry, ARCH-5); false = the fn lives in host.rs but no
#     verb reaches it through that entry.
#   catalog_command.wired — the variant is routed (not refused) by route();
#     every unwired command answers the typed unsupported-for-host refusal.
"""


def toml_value(value: str) -> str:
    if not re.fullmatch(r"[A-Za-z0-9_.:/-]+", value):
        raise ShapeError(f"refusing to emit a value needing TOML escaping: {value!r}")
    return f'"{value}"'


def render(summary: dict[str, int], sections: dict[str, list[dict]]) -> str:
    out = [HEADER]
    out.append("[summary]")
    for key in sorted(summary):
        out.append(f"{key} = {summary[key]}")
    for family in sections["decoder_family"]:
        out += ["", "[[decoder_family]]",
                f"name = {toml_value(family['name'])}",
                f"shipped = {'true' if family['shipped'] else 'false'}",
                f"source = {toml_value(family['source'])}"]
    for verb in sections["host_verb"]:
        out += ["", "[[host_verb]]",
                f"name = {toml_value(verb['name'])}",
                f"wired = {'true' if verb['wired'] else 'false'}",
                f"source = {toml_value(verb['source'])}"]
    for command in sections["catalog_command"]:
        out += ["", "[[catalog_command]]",
                f"name = {toml_value(command['name'])}",
                f"id = {toml_value(command['id'])}",
                f"wired = {'true' if command['wired'] else 'false'}",
                f"source = {toml_value(command['source'])}"]
    out.append("")
    return "\n".join(out)


def validate(text: str, sections: dict[str, list[dict]]) -> None:
    try:
        import tomllib
    except ImportError as exc:  # pragma: no cover - python >= 3.11 required
        raise ShapeError(f"tomllib unavailable (python >= 3.11 required): {exc}") from exc
    data = tomllib.loads(text)
    summary = data["summary"]
    checks = {
        "decoder_families": len(sections["decoder_family"]),
        "decoder_families_shipped": sum(1 for r in sections["decoder_family"] if r["shipped"]),
        "host_verbs": len(sections["host_verb"]),
        "host_verbs_wired": sum(1 for r in sections["host_verb"] if r["wired"]),
        "catalog_commands": len(sections["catalog_command"]),
        "catalog_commands_wired": sum(1 for r in sections["catalog_command"] if r["wired"]),
    }
    if summary != checks:
        raise ShapeError(f"emitted summary {summary} disagrees with the rows {checks} — emission bug")


def main() -> int:
    try:
        families = decoder_rows()
        verbs = host_verb_rows()
        commands = catalog_rows()
        sections = {
            "decoder_family": families,
            "host_verb": verbs,
            "catalog_command": commands,
        }
        summary = {
            "decoder_families": len(families),
            "decoder_families_shipped": sum(1 for r in families if r["shipped"]),
            "host_verbs": len(verbs),
            "host_verbs_wired": sum(1 for r in verbs if r["wired"]),
            "catalog_commands": len(commands),
            "catalog_commands_wired": sum(1 for r in commands if r["wired"]),
        }
        text = render(summary, sections)
        validate(text, sections)
    except ShapeError as exc:
        print(f"status-truth: SOURCE-SHAPE FAILURE (exit 2, never understated): {exc}", file=sys.stderr)
        return 2
    sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
