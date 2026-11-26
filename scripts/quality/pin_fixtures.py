#!/usr/bin/env python3
"""The fixture pin generator (FX8-A04, FIX-PLAN §4 F2.2).

Hash-pin the fixtures the cargo-xtask ledger does not cover. The audit's
census (audit/files/tests/fixtures/generated/MANIFEST.toml.audit.md,
F-001 [S3]): the drift gate ``cargo xtask fixtures --check`` advertised
by CORPUS.md covers 28 of the 116 files under
``tests/fixtures/generated/{valid,malformed,ivf}`` — the other 88 (the
wav/aifc/aiff, ogg, mkv/webm, mp4/mov/3gp and ivf families, plus the two
external-oracle digest sidecars) are ffmpeg 7.1.5 (Lavf61.7.103)
products authored outside the xtask generator and invisible to that
gate by design ("extra files are invisible to it by design" — the suite
provenance headers in crates/demux/tests/). This script pins them:
every row it emits is sha256+size measured from the committed bytes,
and any file it cannot classify into the census below is a hard failure
— it never silently understates coverage.

Determinism contract: same tree, same bytes on disk. No timestamps, no
environment, no git. The census lives in this file (the xtask model:
the generator's code is the closed set), so a census change means
editing this file first, then regenerating — exactly how
``cargo xtask fixtures`` owns its 28.

Scope (the exact census, machine-checked against the tree):

* roots walked — ``tests/fixtures/generated/{valid,malformed,ivf}``;
* excluded — the 28 rows of ``tests/fixtures/generated/MANIFEST.toml``
  (read at run time; the xtask gate remains their authority);
* pinned — the remaining 88 files: 27 valid parents + 59 malformed
  derivatives + 2 oracle sidecars;
* NOT covered (recorded follow-ups, deliberately unpinned):
  ``tests/fixtures/hls/`` (17 files — provenance UNVERIFIED per the
  2026-11 audit, possibly captured broadcast) and
  ``tests/fixtures/mjpeg_h3/`` (4 files — no recorded provenance).

Exit codes:

* ``0`` — success (write emitted the fresh bytes, check found no
  drift).
* ``1`` — drift: a census member missing on disk (deleted fixture), or
  the committed pins.toml disagreeing with the fresh emission (deleted
  row, edited row, a fixture's bytes drifting under its pin).
* ``2`` — census shape failure: a file under the roots this census does
  not know (unknown family or unknown derivative op), an oracle
  sidecar without a registered base, an unreadable input. The pin layer
  refuses to guess rather than understate; fix the census in this file
  or register the fixture in the xtask manifest.

Regenerate the committed artifact:

    python3 scripts/quality/pin_fixtures.py

Gate (CI): ``python3 scripts/quality/pin_fixtures.py --check``.

Python 3 stdlib only (>= 3.11 for ``tomllib``); no cargo anywhere.
"""

from __future__ import annotations

import argparse
import hashlib
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests" / "fixtures"
MANIFEST = FIXTURES / "generated" / "MANIFEST.toml"
PINS = FIXTURES / "pins.toml"

# The three walked roots (the audit's census tree).
ROOTS = (
    FIXTURES / "generated" / "valid",
    FIXTURES / "generated" / "malformed",
    FIXTURES / "generated" / "ivf",
)

# The derivative op vocabulary (the suite headers: exact-prefix
# truncations, seeded single-bit flips, 4096-byte garbage appends).
_NINE_OPS = frozenset(
    {"trunc0", "trunc1", "trunc2", "trunc3", "trunc4",
     "flip1", "flip4", "flip16", "garbage0"}
)

# Every derivative op token this census can ever see; the last four are
# xtask-manifest vocabulary (a manifest row dropped for one of them
# surfaces here as an unknown-family shape failure, never silently).
OP_TOKENS = frozenset(
    _NINE_OPS | {"noidx0", "negheight0", "absidx1", "stszlie0"}
)


@dataclass(frozen=True)
class Family:
    """One unregistered fixture family this census pins.

    ``root``    — the parent's directory under ``generated/``
                  (``valid`` or ``ivf``; derivatives always live in
                  ``malformed``).
    ``ext``     — the parent's file extension (``.wav`` ...).
    ``dext``    — the derivatives' extension (``""`` where the family's
                  derivatives are extensionless: aiff/ogg/wav).
    ``ops``     — the closed derivative-op set (empty for parent-only
                  families; a derivative outside it is exit 2).
    ``parent_provenance``     — the parent's authorship record.
    ``derivative_provenance`` — the derivatives' authorship record
                                 (required whenever ``ops`` is set).
    """

    root: str
    ext: str
    dext: str
    ops: frozenset
    parent_provenance: str
    derivative_provenance: str = ""


_WAV_AIFF = (
    "ffmpeg 7.1 (Lavf61.7.103) lavfi tones — authored with the ffmpeg "
    "CLI outside the xtask generator (crates/demux/tests/"
    "wav_aiff_fixtures.rs 'Fixture provenance' header)"
)
_WAV_AIFF_DERIV = (
    "deterministic byte surgery on the ffmpeg-authored parent "
    "(exact-prefix truncations, seeded single-bit flips, 4096-byte "
    "garbage append) per the wav_aiff_fixtures.rs header"
)
_OGG = (
    "ffmpeg 7.1.5 (Lavf61.7.103) lavfi sine (docs/worklog/"
    "archive-2026-10.md:8587) — authored with the ffmpeg CLI outside "
    "the xtask generator (crates/demux/tests/ogg_fixtures.rs "
    "'Fixture provenance' header)"
)
_OGG_CHAIN = (
    "byte-exact `cat` of the two ffmpeg-authored vorbis parents (RFC "
    "3533 chained stream; docs/worklog/archive-2026-10.md:8587; "
    "crates/demux/tests/ogg_fixtures.rs 'Fixture provenance' header)"
)
_OGG_DERIV = (
    "deterministic byte surgery on the ffmpeg-authored parent "
    "(exact prefixes, single-bit XORs, +4096 B garbage) per docs/"
    "worklog/archive-2026-10.md:8587 and the ogg_fixtures.rs header"
)
_MKV = (
    "ffmpeg 7.1.5 lavfi sources, muxed A/V lanes — authored with the "
    "ffmpeg CLI outside the xtask generator (crates/demux/tests/"
    "mkv_fixtures.rs provenance header)"
)
_MKV_DERIV = (
    "deterministic malformed derivative of the ffmpeg-authored parent "
    "(trunc/flip/garbage) per the mkv_fixtures.rs header"
)
_MP4 = (
    "authored with the sandbox ffmpeg (Lavf61; -movflags +faststart "
    "progressive / +frag_keyframe+empty_moov fragmented) outside the "
    "xtask generator (crates/demux/tests/mp4_fixtures.rs header)"
)
_MP4_DERIV = (
    "deterministic malformed derivative of the ffmpeg-authored parent "
    "(trunc/flip/garbage) per the mp4_fixtures.rs header"
)
_IVF = (
    "ffmpeg 7.1.5 lavfi testsrc2, encoded with libaom-av1 / libvpx-vp9 "
    "(Lavf61.7.103 / Lavc61.19.101) — authored with the ffmpeg CLI "
    "outside the xtask generator (crates/demux/tests/ivf_fixtures.rs "
    "'Fixture provenance' header)"
)
_SIDECAR = (
    "Lumen-authored external-oracle digest sidecar — sha256s of "
    "ffmpeg's own decode of the registered parent (card W1-14; "
    "scripts/oracle_check.sh); text, not media"
)

# The closed census: family stem -> Family. The stem is the parent's
# file stem; a derivative is <stem>__<op>[<dext>] under malformed/.
FAMILIES: dict[str, Family] = {
    # wav_aiff suite (crates/demux/tests/wav_aiff_fixtures.rs).
    "aifc_raw_u8_44k1_mono": Family("valid", ".aifc", "", frozenset(), _WAV_AIFF),
    "aifc_sowt_s16le_44k1_st": Family("valid", ".aifc", "", frozenset(), _WAV_AIFF),
    "aiff_pcm_s16be_44k1_st": Family(
        "valid", ".aiff", "", _NINE_OPS, _WAV_AIFF, _WAV_AIFF_DERIV),
    "aiff_pcm_s24be_48k_st": Family("valid", ".aiff", "", frozenset(), _WAV_AIFF),
    "wav_extensible_6ch_48k": Family("valid", ".wav", "", frozenset(), _WAV_AIFF),
    "wav_pcm_f32le_48k_st": Family("valid", ".wav", "", frozenset(), _WAV_AIFF),
    "wav_pcm_s16le_44k1_st": Family(
        "valid", ".wav", "", _NINE_OPS, _WAV_AIFF, _WAV_AIFF_DERIV),
    "wav_pcm_s24le_48k_st": Family("valid", ".wav", "", frozenset(), _WAV_AIFF),
    "wav_pcm_u8_44k1_mono": Family("valid", ".wav", "", frozenset(), _WAV_AIFF),
    "wav_rf64_44k1_st": Family("valid", ".wav", "", frozenset(), _WAV_AIFF),
    # ogg suite (crates/demux/tests/ogg_fixtures.rs).
    "ogg_chain_two_links": Family("valid", ".ogg", "", frozenset(), _OGG_CHAIN),
    "ogg_opus_48k_mono": Family("valid", ".opus", "", frozenset(), _OGG),
    "ogg_vorbis_16k_mono": Family("valid", ".ogg", "", frozenset(), _OGG),
    "ogg_vorbis_8k_st": Family(
        "valid", ".ogg", "", _NINE_OPS, _OGG, _OGG_DERIV),
    # mkv/webm suite (crates/demux/tests/mkv_fixtures.rs).
    "mkv_h264_aac": Family("valid", ".mkv", ".mkv", _NINE_OPS, _MKV, _MKV_DERIV),
    "mkv_h264_flac": Family("valid", ".mkv", "", frozenset(), _MKV),
    "mkv_multitrack": Family("valid", ".mkv", "", frozenset(), _MKV),
    "webm_vp9_opus": Family("valid", ".webm", ".webm", _NINE_OPS, _MKV, _MKV_DERIV),
    # mp4/mov/3gp suite (crates/demux/tests/mp4_fixtures.rs).
    "mp4_3gp_h263": Family(
        "valid", ".3gp", ".3gp", frozenset({"trunc0", "flip1"}), _MP4, _MP4_DERIV),
    "mp4_chapters": Family(
        "valid", ".mp4", ".mp4", frozenset({"flip1"}), _MP4, _MP4_DERIV),
    "mp4_edts": Family(
        "valid", ".mp4", ".mp4", frozenset({"trunc0"}), _MP4, _MP4_DERIV),
    "mp4_frag": Family(
        "valid", ".mp4", ".mp4", frozenset({"trunc0", "flip1"}), _MP4, _MP4_DERIV),
    "mp4_frag_sidx": Family("valid", ".mp4", "", frozenset(), _MP4),
    "mp4_h264_aac": Family(
        "valid", ".mp4", ".mp4",
        frozenset({"trunc0", "trunc1", "trunc2", "flip1", "flip4",
                   "flip16", "garbage0"}),
        _MP4, _MP4_DERIV),
    "mp4_qt": Family(
        "valid", ".mov", ".mov", frozenset({"trunc0"}), _MP4, _MP4_DERIV),
    # ivf suite (crates/demux/tests/ivf_fixtures.rs).
    "av1_64x48_30fps": Family("ivf", ".ivf", "", frozenset(), _IVF),
    "vp9_64x48_25fps": Family("ivf", ".ivf", "", frozenset(), _IVF),
}

# The two external-oracle digest sidecars: bases are xtask-registered
# parents; the sidecar text itself is unregistered and pinned here.
SIDECARS = frozenset({
    "generated/valid/avi_mjpeg_pcm.avi.oracle.sha256",
    "generated/valid/mp4_mjpeg_pcm.mp4.oracle.sha256",
})


class ShapeError(Exception):
    """A census shape this pin layer refuses to guess about (exit 2)."""


class DriftError(Exception):
    """Coverage drift between the census and the tree (exit 1) — the
    class the --check gate exists to catch."""


def dotted(number: int) -> str:
    """TOML-style thousands grouping (425_516), the MANIFEST convention."""
    return f"{number:,}".replace(",", "_")


def load_registered() -> set[str]:
    """The 28 xtask-registered paths (their gate's authority)."""
    try:
        data = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    except OSError as exc:
        raise ShapeError(f"cannot read {MANIFEST.name}: {exc}") from exc
    except tomllib.TOMLDecodeError as exc:
        raise ShapeError(f"cannot parse {MANIFEST.name}: {exc}") from exc
    registered = {row["path"] for row in data.get("files", [])}
    if not registered:
        raise ShapeError(
            f"{MANIFEST.name} carries no file rows — refusing to pin the "
            "whole tree (the xtask gate would be down)"
        )
    return registered


def walk() -> list[str]:
    """Every file under the three roots, sorted, paths relative to
    tests/fixtures/ (the MANIFEST path convention)."""
    seen: list[str] = []
    for root in ROOTS:
        if not root.is_dir():
            raise ShapeError(f"census root missing on disk: {root}")
        for path in sorted(root.iterdir()):
            if not path.is_file():
                raise ShapeError(
                    f"non-file entry under a census root: {path.name}"
                )
            seen.append(path.relative_to(FIXTURES).as_posix())
    if not seen:
        raise ShapeError("the census roots are empty — nothing to pin")
    return sorted(seen)


def classify(rel: str, registered: set[str]) -> tuple[str, str, str]:
    """One unregistered file -> (family, op, provenance).

    Refuses (exit 2) on any shape this census does not know: unknown
    family, unknown derivative op, wrong extension, wrong directory.
    """
    _, directory, name = rel.split("/", 2)

    if rel in SIDECARS:
        base = rel[: -len(".oracle.sha256")]
        if base not in registered:
            raise ShapeError(
                f"oracle sidecar {rel}: base {base} is not a registered "
                "manifest row — unknown sidecar"
            )
        if directory != "valid":
            raise ShapeError(f"oracle sidecar {rel} outside generated/valid/")
        return "oracle-sidecar", "oracle-sidecar", _SIDECAR

    if "__" in name:
        stem, rest = name.split("__", 1)
        fam = FAMILIES.get(stem)
        if fam is None:
            raise ShapeError(
                f"unknown fixture family: {rel!r} (stem {stem!r} is in "
                "neither this census nor the manifest)"
            )
        if directory != "malformed":
            raise ShapeError(
                f"derivative {rel!r} outside generated/malformed/"
            )
        for op in fam.ops:
            if rest == op + fam.dext:
                if not fam.derivative_provenance:
                    raise ShapeError(
                        f"family {stem!r} carries derivatives but no "
                        "derivative provenance record"
                    )
                return stem, op, fam.derivative_provenance
        for op in OP_TOKENS:
            if rest == op or rest.startswith(op + "."):
                raise ShapeError(
                    f"unknown derivative op: {rel!r} (op {rest!r} is not "
                    f"in the census for family {stem!r}; known: "
                    f"{sorted(fam.ops)})"
                )
        raise ShapeError(f"unclassifiable derivative name: {rel!r}")

    for stem, fam in FAMILIES.items():
        if name == stem + fam.ext:
            if directory != fam.root:
                raise ShapeError(
                    f"parent {rel!r}: family {stem!r} lives under "
                    f"generated/{fam.root}/, found under {directory}/"
                )
            return stem, "parent", fam.parent_provenance
    raise ShapeError(
        f"unknown fixture family: {rel!r} (matches no census family "
        "stem+extension and no manifest row)"
    )


def hash_file(rel: str) -> tuple[str, int]:
    path = FIXTURES / rel
    try:
        data = path.read_bytes()
    except OSError as exc:
        raise DriftError(f"census member unreadable: {rel} ({exc})") from exc
    return hashlib.sha256(data).hexdigest(), len(data)


def expected_census() -> set[str]:
    """The closed census as paths: every family's parent, every
    family's derivatives, both oracle sidecars."""
    expected = {
        f"generated/{fam.root}/{stem}{fam.ext}"
        for stem, fam in FAMILIES.items()
    }
    expected |= {
        f"generated/malformed/{stem}__{op}{fam.dext}"
        for stem, fam in FAMILIES.items()
        for op in fam.ops
    }
    expected |= set(SIDECARS)
    return expected


def render(rows: list[dict]) -> str:
    out = [
        "# Generated by `python3 scripts/quality/pin_fixtures.py` "
        "(FX8-A04, FIX-PLAN §4 F2.2) — DO NOT EDIT.",
        "# Regenerate: python3 scripts/quality/pin_fixtures.py.",
        "# Drift gate: python3 scripts/quality/pin_fixtures.py --check.",
        "#",
        "# Scope: the fixture files under generated/{valid,malformed,ivf}",
        "# that the 28-row cargo-xtask MANIFEST.toml does NOT cover — the",
        "# ffmpeg-authored wav/aifc/aiff, ogg, mkv/webm, mp4/mov/3gp and",
        "# ivf families (valid parents, malformed derivatives and the two",
        "# external-oracle digest sidecars) — hash-pinned here because",
        "# `cargo xtask fixtures --check` only drift-gates its own 28",
        "# outputs (extra files are invisible to it by design; the suite",
        "# provenance headers in crates/demux/tests/). The xtask gate",
        "# remains the authority for its 28 rows.",
        "#",
        "# NOT covered (recorded follow-ups, deliberately unpinned):",
        "# tests/fixtures/hls/ (17 files — provenance UNVERIFIED per the",
        "# 2026-11 audit, possibly captured broadcast) and",
        "# tests/fixtures/mjpeg_h3/ (4 files — no recorded provenance).",
        'schema = "pins/1"',
        'generator = "scripts/quality/pin_fixtures.py"',
        'manifest = "tests/fixtures/generated/MANIFEST.toml"',
        f"file_count = {len(rows)}",
    ]
    for row in rows:
        out += [
            "",
            "[[pins]]",
            f'path = "{row["path"]}"',
            f'family = "{row["family"]}"',
            f'op = "{row["op"]}"',
            f'provenance = "{row["provenance"]}"',
            f'sha256 = "{row["sha256"]}"',
            f"size_bytes = {dotted(row['size_bytes'])}",
        ]
    out.append("")
    return "\n".join(out)


def validate(text: str, rows: list[dict]) -> None:
    data = tomllib.loads(text)
    if data["schema"] != "pins/1":
        raise ShapeError(f"emitted schema {data['schema']!r} is not pins/1")
    if data["file_count"] != len(rows):
        raise ShapeError(
            f"emitted file_count {data['file_count']} disagrees with "
            f"{len(rows)} rows — emission bug"
        )
    pinned = data["pins"]
    if len(pinned) != len(rows):
        raise ShapeError(
            f"emitted TOML round-trips as {len(pinned)} rows, expected "
            f"{len(rows)} — emission bug"
        )
    paths = [row["path"] for row in pinned]
    if paths != sorted(paths):
        raise ShapeError("emitted rows are not path-sorted — emission bug")
    if len(set(paths)) != len(paths):
        raise ShapeError("emitted rows carry a duplicate path — emission bug")
    for parsed, row in zip(pinned, rows):
        if (parsed["path"], parsed["family"], parsed["op"],
                parsed["sha256"], parsed["size_bytes"]) != (
                row["path"], row["family"], row["op"], row["sha256"],
                row["size_bytes"]):
            raise ShapeError(
                f"emitted TOML row for {row['path']!r} disagrees with the "
                "measured row — emission bug"
            )


def build() -> tuple[str, list[dict]]:
    """Walk + classify + hash; returns the rendered document."""
    registered = load_registered()
    census: dict[str, dict] = {}
    for rel in walk():
        if rel in registered:
            continue
        family, op, provenance = classify(rel, registered)
        sha256, size = hash_file(rel)
        census[rel] = {
            "path": rel,
            "family": family,
            "op": op,
            "provenance": provenance,
            "sha256": sha256,
            "size_bytes": size,
        }

    # The census is closed: a known member missing on disk is a deleted
    # fixture (drift); an on-disk file outside the census table was
    # already refused above (unknown family/op).
    expected = expected_census()
    missing = sorted(expected - set(census))
    if missing:
        raise DriftError(
            "census member(s) missing on disk (deleted fixture?): "
            + ", ".join(missing)
        )
    extra = sorted(set(census) - expected)
    if extra:
        raise ShapeError(
            "classified file(s) outside the census table: " + ", ".join(extra)
        )

    rows = [census[rel] for rel in sorted(census)]
    text = render(rows)
    validate(text, rows)
    return text, rows


def drift_details(committed: str, rows: list[dict]) -> list[str]:
    """Human-readable drift classes for the --check failure message."""
    try:
        old = tomllib.loads(committed)["pins"]
    except (tomllib.TOMLDecodeError, KeyError):
        return ["the committed pins.toml does not parse as a pins/1 ledger"]
    old_by_path = {row["path"]: row for row in old}
    new_paths = {row["path"] for row in rows}
    gone = sorted(set(old_by_path) - new_paths)
    added = sorted(new_paths - set(old_by_path))
    changed = sorted(
        f"{row['path']} (sha256 {old_by_path[row['path']]['sha256'][:12]}…"
        f" != {row['sha256'][:12]}…)"
        for row in rows
        if row["path"] in old_by_path
        and old_by_path[row["path"]]["sha256"] != row["sha256"]
    )
    details = []
    if gone:
        details.append("deleted row(s)/fixture(s): " + ", ".join(gone))
    if added:
        details.append("new fixture(s): " + ", ".join(added))
    if changed:
        details.append("fixture byte drift: " + ", ".join(changed))
    if not details:
        details.append("the committed pins.toml differs textually from the "
                        "fresh emission (re-run the writer to resync)")
    return details


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Pin the 88 fixtures the xtask manifest does not cover."
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="compare the committed pins.toml against the tree "
        "(exit 1 on drift, exit 2 on census shape failure)",
    )
    args = parser.parse_args()

    try:
        text, rows = build()
    except ShapeError as exc:
        print(
            "pin_fixtures: CENSUS-SHAPE FAILURE (exit 2, never guessed): "
            f"{exc}",
            file=sys.stderr,
        )
        return 2
    except DriftError as exc:
        print(f"pin_fixtures: DRIFT (exit 1): {exc}", file=sys.stderr)
        return 1

    total = sum(row["size_bytes"] for row in rows)

    if args.check:
        try:
            committed = PINS.read_text(encoding="utf-8")
        except OSError as exc:
            print(
                f"pin_fixtures: DRIFT (exit 1): cannot read committed "
                f"pins.toml ({exc})",
                file=sys.stderr,
            )
            return 1
        if committed == text:
            print(
                f"pin_fixtures: no drift — {len(rows)} pinned file(s), "
                f"{dotted(total)} bytes verified"
            )
            return 0
        details = "; ".join(drift_details(committed, rows))
        print(
            "pin_fixtures: DRIFT (exit 1) — pins.toml disagrees with the "
            "tree; regenerate with `python3 scripts/quality/"
            f"pin_fixtures.py`. {details}",
            file=sys.stderr,
        )
        return 1

    try:
        PINS.write_text(text, encoding="utf-8")
    except OSError as exc:
        print(
            f"pin_fixtures: cannot write pins.toml (exit 2): {exc}",
            file=sys.stderr,
        )
        return 2
    print(
        f"pin_fixtures: wrote pins.toml — {len(rows)} pinned file(s), "
        f"{dotted(total)} bytes"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
