#!/usr/bin/env python3
"""bench-vs-vlc run.py — differential-vs-VLC harness v1 (card 3.E3, doc 10 §6.6).

Runs the pinned external VLC 3.0.23 oracle over corpus fixtures and
compares its decoded-PCM MD5 against Lumen's decoded-PCM MD5 for
LOSSLESS-comparable paths (PCM-class audio in AVI, the landed first
vertical). Non-comparable paths (lossy codecs where conforming decoders
legitimately differ, containers/decoders Lumen has not landed, output
families VLC's `afile` module cannot dump) get an explicit
non-comparable row with the reason — never a silent skip.

MOCK vs REAL — the binding separation rule
==========================================
The oracle backend is one of:

  --oracle vlc      the REAL oracle: a `vlc`/`cvlc` binary on PATH whose
                    `--version` is asserted to be exactly 3.0.23
                    (drift aborts the run — doc 10 §6.6). If no binary
                    is present the run REFUSES (exit 2); it never falls
                    back to the mock.
  --oracle mock     the MOCK oracle (tools/bench-vs-vlc/mock_vlc.py): a
                    clearly-labeled stand-in that exercises the harness
                    plumbing end to end. Every row, label and artifact
                    it produces is prefixed [MOCK]; the report title and
                    the JSON envelope say `oracle.kind = "mock"`. A mock
                    run is NEVER a measurement and NEVER a conformance
                    claim.
  --oracle pending  no oracle at all: comparable rows carry the exact
                    to-run VLC command and a `PENDING-VLC` cell for the
                    oracle digest (the mode used when the sandbox has
                    no VLC — see docs/corpus/vlc-diff-report-20261023.md).

There is no code path where a mock digest reaches a real label: the two
oracle classes share only their call signature, and the report writer
asserts the labeling invariants for whichever kind it was handed.

VLC invocation (documented, from the repo's own pinned-binary baselines)
========================================================================
  vlc -I dummy --play-and-exit --vout none --aout afile \
      --no-audiofile-wav --audiofile-format {u8|s16|float32} \
      --audiofile-file OUT INPUT

  * `afile` = "File audio output" — docs/baselines/raw/linux/
    14-options-full-help.txt lines 2335-2348 (the pinned 3.0.23 Linux
    build's own help): `--audiofile-file` ("-" for stdout),
    `--audiofile-format {u8,s16,float32,spdif}`, `--audiofile-channels`
    (0 = all incoming channels saved — the passthrough default this
    harness pins), `--audiofile-wav` (default ON; disabled here so the
    dump is headerless raw PCM).
  * `s16`/`float32` are NATIVE-endian, so the comparable set is
    little-endian PCM on a little-endian host (recorded per-run; a
    big-endian host makes every row non-comparable with that reason).
  * `-I dummy --play-and-exit --vout none`: headless, no video output,
    exit at end of the (single-item) playlist.
  * [VERIFY — untested without the binary] the flags are transcribed
    from the pinned build's own help text; the first real-oracle run
    must confirm the dump's byte count equals Lumen's pcm_bytes on a
    known-good fixture before any MATCH is trusted.

Lumen side
==========
`tools/bench-vs-vlc/` (this directory) is a standalone cargo workspace
whose integration test decodes each fixture's FIRST audio track through
the real demux+codec crates and digests the decoded PCM (MD5, RFC 1321
implemented in-repo). This script drives it via
`cargo test --test lumen_pcm -- --ignored` with LUMEN_DIFF_FIXTURES /
LUMEN_DIFF_OUT, or consumes a pre-generated digest file (--lumen-json).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

TOOL_ROOT = Path(__file__).resolve().parent
PINNED_VLC_VERSION = "3.0.23"
LUMEN_SCHEMA = "bench-vs-vlc/lumen-pcm-digests/1"

# The afile output format per PCM wire family (little-endian host: VLC's
# native-endian s16/float32 == the LE wire; u8 is byte-order-free).
AFILE_FORMAT_FOR_PCM = {
    "pcm_u8": "u8",
    "pcm_s16le": "s16",
    "pcm_f32le": "float32",
}
# PCM families VLC's afile cannot dump at all (help text: u8, s16,
# float32, spdif only).
PCM_UNDUMPABLE = {
    "pcm_s24le": "no afile output format for 24-bit int (u8/s16/float32/spdif only)",
    "pcm_s32le": "no afile output format for 32-bit int",
    "pcm_f64le": "no afile output format for float64",
}
# Lossy audio codecs: conforming decoders legitimately differ; decoded
# bytes are never comparable (doc 10 §6.6 "what cannot be compared").
LOSSY_AUDIO = {
    "aac", "mp3", "opus", "vorbis", "ac3", "eac3", "dts", "mp2", "wma",
    "speex", "amr", "adpcm_ima", "adpcm_ms", "mlp",
}
# Lossless-class audio codecs (byte-exact comparison is meaningful
# once BOTH sides decode them).
LOSSLESS_AUDIO = {"flac", "alac", "wavpack", "truehd"}


class OracleUnavailable(RuntimeError):
    """No VLC binary on PATH (the real oracle cannot run)."""


class VersionDrift(RuntimeError):
    """The oracle binary's version is not the pinned 3.0.23."""


# ---------------------------------------------------------------------------
# Oracles — the mock/real separation lives here
# ---------------------------------------------------------------------------


def vlc_dump_command(binary: str, input_path: Path, out_path: Path,
                     afile_format: str) -> list[str]:
    """The documented PCM-dump invocation (see module docstring)."""
    return [
        binary, "-I", "dummy", "--play-and-exit", "--vout", "none",
        "--aout", "afile", "--no-audiofile-wav",
        "--audiofile-format", afile_format,
        "--audiofile-file", str(out_path), str(input_path),
    ]


class VlcOracle:
    """The REAL oracle: the pinned external VLC 3.0.23 binary."""

    kind = "vlc"
    label = f"VLC {PINNED_VLC_VERSION} (real oracle, version-asserted)"

    def __init__(self, binary: str):
        self.binary = binary

    @classmethod
    def resolve(cls) -> "VlcOracle":
        for name in ("vlc", "cvlc"):
            found = shutil.which(name)
            if found:
                return cls(found)
        raise OracleUnavailable(
            "no vlc/cvlc on PATH — install the pinned VLC 3.0.23 or run "
            "with --oracle pending (PENDING-VLC rows, no oracle digests)"
        )

    def assert_version_pin(self) -> str:
        """Doc 10 §6.6: every invocation asserts the version pin; drift
        aborts the run."""
        try:
            proc = subprocess.run(
                [self.binary, "--version"], capture_output=True,
                text=True, timeout=60, check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as err:
            raise OracleUnavailable(f"cannot run {self.binary} --version: {err}") from err
        text = (proc.stdout + proc.stderr).strip()
        first = text.splitlines()[0] if text else ""
        # The pinned build prints: "VLC version 3.0.23 Vetinari (...)"
        # (docs/baselines/raw/linux/04-vlc-version.txt).
        if not first.startswith("VLC version "):
            raise VersionDrift(f"unrecognized version line: {first!r}")
        version = first.split()[2]
        if version != PINNED_VLC_VERSION:
            raise VersionDrift(
                f"oracle version {version} != pinned {PINNED_VLC_VERSION} — aborting"
            )
        return first

    def dump_pcm(self, input_path: Path, out_path: Path, afile_format: str,
                 timeout_s: float) -> tuple[bool, str]:
        command = vlc_dump_command(self.binary, input_path, out_path, afile_format)
        try:
            proc = subprocess.run(
                command, capture_output=True, text=True,
                timeout=timeout_s, check=False,
            )
        except subprocess.TimeoutExpired:
            return False, f"timeout after {timeout_s}s"
        except OSError as err:
            return False, str(err)
        if proc.returncode != 0:
            tail = (proc.stderr or "").strip().splitlines()[-3:]
            return False, f"exit {proc.returncode}: " + " | ".join(tail)
        if not out_path.exists():
            return False, "no dump file produced"
        return True, ""


class MockOracle:
    """The MOCK oracle: tools/bench-vs-vlc/mock_vlc.py.

    Accepts the same documented flags, prints a version line the pin
    check accepts, and writes a DETERMINISTIC synthetic PCM dump (never
    a real decode). Test hook: LUMEN_MOCK_VLC_ECHO=<file> copies that
    file's bytes as the dump (agreement-path testing). LUMEN_MOCK_VLC_VERSION
    overrides the version line (drift-path testing). Everything it
    produces is labeled MOCK by the report writer — by construction.
    """

    kind = "mock"
    label = "MOCK ORACLE (mock_vlc.py — plumbing validation only, NOT a measurement)"

    def __init__(self) -> None:
        self.script = TOOL_ROOT / "mock_vlc.py"
        if not self.script.is_file():
            raise OracleUnavailable(f"mock oracle missing: {self.script}")

    def assert_version_pin(self) -> str:
        try:
            proc = subprocess.run(
                [sys.executable, str(self.script), "--version"],
                capture_output=True, text=True, timeout=60, check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as err:
            raise OracleUnavailable(f"cannot run mock oracle: {err}") from err
        first = proc.stdout.strip().splitlines()[0] if proc.stdout.strip() else ""
        if not first.startswith("VLC version "):
            raise VersionDrift(f"unrecognized mock version line: {first!r}")
        version = first.split()[2]
        if version != PINNED_VLC_VERSION:
            raise VersionDrift(
                f"mock oracle version {version} != pinned {PINNED_VLC_VERSION} — aborting"
            )
        return first

    def dump_pcm(self, input_path: Path, out_path: Path, afile_format: str,
                 timeout_s: float) -> tuple[bool, str]:
        command = vlc_dump_command(
            sys.executable, input_path, out_path, afile_format
        )
        # vlc_dump_command builds [binary, ...flags..., input]; the mock
        # is driven as `python mock_vlc.py <flags> <input>` — splice the
        # script in after the interpreter.
        command = [command[0], str(self.script), *command[1:]]
        try:
            proc = subprocess.run(
                command, capture_output=True, text=True,
                timeout=timeout_s, check=False,
            )
        except subprocess.TimeoutExpired:
            return False, f"timeout after {timeout_s}s"
        except OSError as err:
            return False, str(err)
        if proc.returncode != 0:
            return False, f"exit {proc.returncode}: {proc.stderr.strip()}"
        if not out_path.exists():
            return False, "no dump file produced"
        return True, ""


class PendingOracle:
    """No oracle: comparable rows get PENDING-VLC cells + the exact
    to-run command. Produces no digest of any kind."""

    kind = "pending"
    label = "PENDING-VLC (no oracle binary in this environment — nothing mocked)"

    def assert_version_pin(self) -> str:
        return "PENDING-VLC (no version to assert)"

    def dump_pcm(self, input_path: Path, out_path: Path, afile_format: str,
                 timeout_s: float) -> tuple[bool, str]:
        return False, "PENDING-VLC — no oracle available in this environment"


# ---------------------------------------------------------------------------
# Classification (manifest truth → comparable / non-comparable / excluded)
# ---------------------------------------------------------------------------


def classify_entry(entry: dict) -> dict:
    """Classify one corpus manifest entry.

    Returns {status, reason, afile_format} with status one of
    'comparable', 'non_comparable', 'excluded'.
    """
    params = entry.get("params", {})
    klass = entry.get("class", "")
    container = params.get("container")
    video_codec = params.get("codec")
    audio_codec = params.get("audio_codec")
    # Audio-only classes carry the audio codec in params.codec
    # (manifest shape: params.class == "audio").
    if audio_codec is None and params.get("class") == "audio":
        audio_codec = video_codec

    # v1 scopes to CLEAN files: corrupt-input differential compares
    # error classes, not decoded bytes (doc 10 §6.6 "error message
    # text: compare error class only") — a separate card.
    if klass in ("dirty", "dirty-lie-ext", "garbage"):
        return {
            "status": "excluded",
            "reason": "corrupt/garbage input: v1 compares clean files; "
                      "corrupt-input differential is the error-class rule "
                      "(doc 10 §6.6), deferred to its own card",
            "afile_format": None,
        }

    if container != "avi":
        reason = (
            f"container {container}: no Lumen demuxer in this build "
            "(first vertical: AVI, D-018)"
        )
        if audio_codec is not None and (
            audio_codec in AFILE_FORMAT_FOR_PCM
            or audio_codec in PCM_UNDUMPABLE
            or audio_codec in LOSSLESS_AUDIO
        ):
            reason += (
                f"; the {audio_codec} audio itself is lossless-class and "
                "becomes comparable when its container card lands"
            )
        return {"status": "non_comparable", "reason": reason, "afile_format": None}

    # AVI from here on. The comparison is over the FIRST AUDIO
    # track's decoded PCM, so the video codec does NOT gate it: the v0
    # demuxer filters unsupported video streams (one `Recovered`
    # decision record each, `skipped == 0`) and still walks the audio —
    # verified on the corpus (mpeg4-video AVIs with PCM audio digest
    # fine, e.g. media/0040-avi-av.avi). When nothing at all is
    # supported the demuxer refuses at start and the row shows that
    # typed refusal as its Lumen result.
    if audio_codec is None:
        return {
            "status": "non_comparable",
            "reason": "no audio track (nothing to digest)",
            "afile_format": None,
        }
    if audio_codec in LOSSY_AUDIO:
        return {
            "status": "non_comparable",
            "reason": f"lossy audio codec {audio_codec}: conforming decoders "
                      "legitimately differ — byte-equality is never claimed "
                      "(doc 10 §6.6)",
            "afile_format": None,
        }
    if audio_codec in PCM_UNDUMPABLE:
        return {
            "status": "non_comparable",
            "reason": f"lossless both sides, but VLC afile cannot dump "
                      f"{audio_codec} ({PCM_UNDUMPABLE[audio_codec]} — "
                      "baselines 14-options-full-help.txt L2338); the "
                      "transcode-chain comparison is doc 10 §6.6's open "
                      "[VERIFY], deferred",
            "afile_format": None,
        }
    if audio_codec not in AFILE_FORMAT_FOR_PCM:
        # e.g. big-endian wire formats or planar variants
        return {
            "status": "non_comparable",
            "reason": f"PCM family {audio_codec} has no pinned afile format "
                      "mapping in this harness (endianness/layout pin "
                      "unresolved) — deferred",
            "afile_format": None,
        }
    if sys.byteorder != "little":
        return {
            "status": "non_comparable",
            "reason": f"host is {sys.byteorder}-endian: VLC afile s16/float32 "
                      "are native-endian; the LE-wire pin only holds on LE "
                      "hosts",
            "afile_format": None,
        }
    return {
        "status": "comparable",
        "reason": f"lossless PCM passthrough ({audio_codec}) on both sides",
        "afile_format": AFILE_FORMAT_FOR_PCM[audio_codec],
    }


def load_manifest_entries(manifest_path: Path, corpus_root: Path,
                          limit: int | None) -> list[dict]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    entries = []
    for file_entry in manifest.get("files", []):
        rel = file_entry["rel"]
        path = corpus_root / rel
        entry = dict(file_entry)
        entry["abs_path"] = path
        entries.append(entry)
    if limit is not None:
        entries = entries[:limit]
    return entries


def probe_file_facts(path: Path) -> dict:
    """--files mode: classify an explicit file via ffprobe (the honest
    external prober this repo already pins for corpus truth)."""
    ffprobe = shutil.which("ffprobe")
    if ffprobe is None:
        raise SystemExit("--files mode needs ffprobe on PATH for classification")
    proc = subprocess.run(
        [ffprobe, "-v", "error", "-show_entries",
         "format=format_name:stream=index,codec_type,codec_name",
         "-of", "json=c=1", str(path)],
        capture_output=True, text=True, timeout=120, check=False,
    )
    if proc.returncode != 0:
        return {"container": None, "codec": None, "audio_codec": None}
    data = json.loads(proc.stdout)
    container = (data.get("format", {}).get("format_name") or "").split(",")[0]
    video_codec = None
    audio_codec = None
    for stream in data.get("streams", []):
        if stream.get("codec_type") == "video" and video_codec is None:
            video_codec = stream.get("codec_name")
        if stream.get("codec_type") == "audio" and audio_codec is None:
            audio_codec = stream.get("codec_name")
    return {"container": container, "codec": video_codec, "audio_codec": audio_codec}


# ---------------------------------------------------------------------------
# Digests and comparison
# ---------------------------------------------------------------------------


def md5_file(path: Path) -> str:
    digest = hashlib.md5()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def compare_pcm_streams(left: bytes, right: bytes) -> dict:
    """Byte-level comparison of two PCM streams (the unit-tested core of
    the digest comparison; the report path uses digests + byte counts,
    this is the diagnostic used when both byte streams are at hand)."""
    verdict = "MATCH" if left == right else "MISMATCH"
    result = {"verdict": verdict, "left_bytes": len(left), "right_bytes": len(right)}
    if left != right:
        divergence = first_divergence(left, right)
        result["first_divergence"] = divergence
    return result


def first_divergence(left: bytes, right: bytes) -> dict | None:
    """The first differing offset (None when one is a prefix of the
    other or they are equal)."""
    common = min(len(left), len(right))
    for offset in range(common):
        if left[offset] != right[offset]:
            return {
                "offset": offset,
                "left_byte": left[offset],
                "right_byte": right[offset],
            }
    if len(left) != len(right):
        return {"offset": common, "left_byte": None, "right_byte": None}
    return None


def ffprobe_payload_bytes(path: Path) -> int | None:
    """The container's audio payload bytes (a:0 packet sizes summed) —
    the granularity-independent truth the W4-01 harness pins. Used here
    as a SECONDARY cross-check of Lumen's decoded byte count (not an
    oracle; clearly labeled as ffprobe's answer)."""
    ffprobe = shutil.which("ffprobe")
    if ffprobe is None:
        return None
    proc = subprocess.run(
        [ffprobe, "-v", "error", "-select_streams", "a:0",
         "-show_entries", "packet=size", "-of", "csv=p=0", str(path)],
        capture_output=True, text=True, timeout=300, check=False,
    )
    if proc.returncode != 0:
        return None
    total = 0
    for line in proc.stdout.splitlines():
        line = line.strip()
        if line:
            total += int(line)
    return total


# ---------------------------------------------------------------------------
# The Lumen side (drives the cargo test)
# ---------------------------------------------------------------------------


def run_lumen_digests(fixtures: list[tuple[str, Path]], out_path: Path) -> None:
    """Drive the standalone crate's ignored batch test over `fixtures`.

    Paths in the TSV and both env vars are resolved to absolute form:
    the cargo test binary runs with its own CWD (the package root), not
    this process's — relative paths would silently miss there.
    """
    cargo = shutil.which("cargo") or str(Path.home() / ".cargo" / "bin" / "cargo")
    if not Path(cargo).exists() and shutil.which("cargo") is None:
        raise SystemExit("cargo not found — the Lumen side needs the Rust toolchain")
    out_path = out_path.resolve()
    tsv = out_path.with_suffix(".fixtures.tsv")
    with tsv.open("w", encoding="utf-8") as handle:
        for rel, path in fixtures:
            handle.write(f"{rel}\t{path.resolve()}\n")
    env = dict(os.environ)
    env["LUMEN_DIFF_FIXTURES"] = str(tsv)
    env["LUMEN_DIFF_OUT"] = str(out_path)
    command = [
        cargo, "test", "--manifest-path", str(TOOL_ROOT / "Cargo.toml"),
        "--test", "lumen_pcm", "--", "--ignored", "--nocapture",
    ]
    proc = subprocess.run(command, env=env, check=False)
    if proc.returncode != 0:
        raise SystemExit(f"Lumen digest batch failed (exit {proc.returncode})")
    if not out_path.exists():
        raise SystemExit("Lumen digest batch produced no results file")


def load_lumen_results(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8"))
    if data.get("schema") != LUMEN_SCHEMA:
        raise SystemExit(
            f"unexpected Lumen results schema {data.get('schema')!r} (want {LUMEN_SCHEMA!r})"
        )
    return data


# ---------------------------------------------------------------------------
# Report writing (mock/real labeling enforced here)
# ---------------------------------------------------------------------------


def report_title(oracle) -> str:
    if oracle.kind == "mock":
        return "MOCK RUN — differential-vs-VLC harness plumbing validation"
    if oracle.kind == "pending":
        return "Differential-vs-VLC 3.0.23 — PENDING-VLC (no oracle in this environment)"
    return "Differential-vs-VLC 3.0.23 — real oracle run"


def build_report(oracle, entries: list[dict], lumen_results: dict,
                 run_meta: dict, ffprobe_check: bool) -> tuple[str, dict]:
    """Build (markdown, json-evidence). The mock/real labeling
    invariants are asserted, not assumed."""
    mock = oracle.kind == "mock"
    pending = oracle.kind == "pending"
    by_rel = {row["rel"]: row for row in lumen_results.get("results", [])}

    rows = []
    for entry in entries:
        rel = entry["rel"]
        verdict = classify_entry(entry)
        row = {
            "rel": rel,
            "class": entry.get("class"),
            "bytes": entry.get("bytes"),
            "abs_path": entry.get("abs_path"),
            "classification": verdict,
            # The manifest's declared audio codec (the table shows it when
            # the Lumen side refused — the declared vs decoded distinction
            # stays visible).
            "declared_audio": entry.get("params", {}).get("audio_codec"),
            "lumen": by_rel.get(rel),
        }
        rows.append(row)

    # --- run the oracle over comparable rows (mock or real). Rows whose
    # Lumen digest refused are STILL dumped: "Lumen refuses, oracle
    # decodes" is exactly the kind of difference this harness exists to
    # surface (the verdict cell says LUMEN-REFUSED, never MATCH). ---
    dumps_dir = Path(tempfile.mkdtemp(prefix="bench-vs-vlc-dump-"))
    try:
        for row in rows:
            if row["classification"]["status"] != "comparable" or pending:
                continue
            out_path = dumps_dir / (row["rel"].replace("/", "__") + ".raw")
            ok, note = oracle.dump_pcm(
                entry_path(row), out_path,
                row["classification"]["afile_format"], timeout_s=300.0,
            )
            if ok:
                row["oracle_md5"] = md5_file(out_path)
                row["oracle_bytes"] = out_path.stat().st_size
            else:
                row["oracle_error"] = note
    finally:
        shutil.rmtree(dumps_dir, ignore_errors=True)

    # --- markdown ---
    lines: list[str] = []
    lines.append(f"# {report_title(oracle)}")
    lines.append("")
    if mock:
        lines.append(
            "> **MOCK ORACLE** — every number below was produced by "
            "`mock_vlc.py`, a deterministic stand-in, NOT by VLC. This "
            "section validates harness plumbing only. It is never a "
            "measurement and never a conformance claim."
        )
        lines.append("")
    if pending:
        lines.append(
            "> **PENDING-VLC** — no VLC binary exists in this environment "
            "(install attempt recorded in the run metadata). Oracle cells "
            "below carry the exact to-run command instead of a digest. "
            "Nothing is mocked: no oracle digest of any kind appears in "
            "this report."
        )
        lines.append("")
    lines.append(f"- Oracle: {oracle.label}")
    lines.append(f"- Version assertion: {oracle.assert_version_pin()}")
    lines.append(f"- Lumen side: {LUMEN_SCHEMA} (real decode via the workspace crates)")
    lines.append(f"- Host endianness: {sys.byteorder} "
                 f"(comparability requires little)")
    lines.append(f"- Generated: {run_meta['generated_at']}")
    lines.append(f"- Fixtures: {len(rows)}")
    source = run_meta.get("source") or {}
    if source.get("mode") == "manifest":
        lines.append(
            f"- Corpus source: manifest `{source['manifest']}` "
            f"(seed {source.get('manifest_seed')}, generated "
            f"{source.get('manifest_generated_at')}, "
            f"{source.get('manifest_count')} files)"
        )
    else:
        lines.append(
            f"- Corpus source: explicit file list ({source.get('n_files')} files, "
            "classified via ffprobe)"
        )
    lines.append("")

    comparable = [r for r in rows if r["classification"]["status"] == "comparable"]
    non_comparable = [r for r in rows if r["classification"]["status"] == "non_comparable"]
    excluded = [r for r in rows if r["classification"]["status"] == "excluded"]

    # Comparable table.
    lines.append("## Comparable fixtures (lossless paths)")
    lines.append("")
    lines.append(
        "| fixture | audio | ch | rate | Lumen MD5 | Lumen B | oracle MD5 | verdict |"
    )
    lines.append("|---|---|---|---|---|---|---|---|")
    for row in comparable:
        lumen = row.get("lumen") or {}
        audio = (lumen.get("codec") if lumen.get("status") == "ok"
                 else None) or row.get("declared_audio") or "-"
        prefix = "[MOCK] " if mock else ""
        if pending:
            oracle_cell = "PENDING-VLC"
            verdict = "PENDING-VLC"
        elif "oracle_md5" in row:
            oracle_cell = row["oracle_md5"]
            if lumen.get("status") != "ok":
                # The oracle answered; the Lumen side refused — a real
                # difference, never a MATCH.
                verdict = "LUMEN-REFUSED" + (" [MOCK]" if mock else "")
            else:
                same = (row["oracle_md5"] == lumen.get("md5")
                        and row.get("oracle_bytes") == lumen.get("pcm_bytes"))
                verdict = ("MATCH" if same else "MISMATCH") + (" [MOCK]" if mock else "")
        else:
            oracle_cell = f"error: {row.get('oracle_error', '?')}"
            verdict = "ORACLE-ERROR" + (" [MOCK]" if mock else "")
        lines.append(
            f"| {prefix}{row['rel']} | {audio} | {lumen.get('channels', '-')} | "
            f"{lumen.get('rate', '-')} | {lumen.get('md5', 'ERROR')} | "
            f"{lumen.get('pcm_bytes', '-')} | {oracle_cell} | {verdict} |"
        )
    lines.append("")

    # PENDING-VLC exact commands.
    if pending and comparable:
        lines.append("### Exact to-run commands (PENDING-VLC)")
        lines.append("")
        for row in comparable:
            lumen = row.get("lumen") or {}
            fmt = row["classification"]["afile_format"]
            dump = f"/tmp/bench-vs-vlc/{row['rel'].replace('/', '__')}.raw"
            command = vlc_dump_command("vlc", entry_path(row), Path(dump), fmt)
            lines.append(f"```")
            lines.append(f"# {row['rel']} (Lumen: {lumen.get('md5', '?')} / "
                         f"{lumen.get('pcm_bytes', '?')} B)")
            lines.append("mkdir -p /tmp/bench-vs-vlc")
            lines.append(shlex.join(command))
            lines.append("```")
        lines.append("")

    # ffprobe cross-check.
    if ffprobe_check:
        checked = 0
        agreed = 0
        for row in comparable:
            lumen = row.get("lumen") or {}
            if lumen.get("status") != "ok":
                continue
            payload = ffprobe_payload_bytes(entry_path(row))
            row["ffprobe_payload_bytes"] = payload
            checked += 1
            if payload == lumen.get("pcm_bytes"):
                agreed += 1
        lines.append(
            f"## Byte-count cross-check vs ffprobe (secondary, NOT an oracle)\n"
        )
        lines.append(
            f"- comparable rows checked: {checked}; Lumen decoded PCM bytes == "
            f"ffprobe a:0 packet-payload bytes on {agreed}/{checked}"
        )
        lines.append("")

    # Lumen-side refusals on comparable-classified rows (honest data:
    # the typed refusal is the difference; the oracle cell above shows
    # what the other side answered, when it ran).
    refused = [r for r in comparable
               if r.get("lumen") and r["lumen"].get("status") != "ok"]
    if refused:
        lines.append("## Lumen-side refusals on comparable-classified rows")
        lines.append("")
        lines.append("| fixture | Lumen refusal (stage-tagged) |")
        lines.append("|---|---|")
        for row in refused:
            error = (row["lumen"].get("error") or "?").replace("|", "\\|")
            lines.append(f"| {row['rel']} | {error} |")
        lines.append("")

    # Digest-integrity flags: rows whose digest is over a damaged or
    # recovered decode (concealed frames, demux bytes actually
    # skipped, mid-stream format changes). `demux_recovered > 0` alone
    # is NOT damage on AVIs — the v0 codec filter's stream decisions
    # ride the same event with `skipped == 0` (see the comparable
    # set's filter note); `demux_skip_bytes` separates them.
    flagged = []
    for row in comparable:
        lumen = row.get("lumen") or {}
        if lumen.get("status") != "ok":
            continue
        flags = []
        if lumen.get("concealed"):
            flags.append(f"concealed={lumen['concealed']}")
        if lumen.get("demux_skip_bytes"):
            flags.append(f"demux_skip_bytes={lumen['demux_skip_bytes']}")
        if lumen.get("format_changes"):
            flags.append(f"format_changes={lumen['format_changes']}")
        if flags:
            flagged.append((row["rel"], ", ".join(flags)))
    if flagged:
        lines.append("## Digest-integrity flags (treat these digests with care)")
        lines.append("")
        lines.append(
            "Rows appear here when the Lumen walk concealed decoder damage, "
            "skipped container bytes while recovering, or saw a mid-stream "
            "format change. Note the two benign sources on this build: the "
            "v0 codec filter's stream decisions (`unsupported-video-handler`, "
            "`skipped == 0`), and the filtered streams' dropped `idx1` entries "
            "(`idx1-entry-unknown-stream`, 16 B per entry — seek-table only, "
            "not walked data; the W5-14 branch silences them for "
            "declared-but-filtered streams). What was independently verified "
            "for THIS run is recorded in the run notes."
        )
        lines.append("")
        lines.append("| fixture | flags |")
        lines.append("|---|---|")
        for rel, flags in flagged:
            lines.append(f"| {rel} | {flags} |")
        lines.append("")

    # Non-comparable list.
    lines.append("## Non-comparable fixtures (explicit reasons)")
    lines.append("")
    grouped: dict[tuple, list[dict]] = {}
    for row in non_comparable:
        key = (row["classification"]["reason"],)
        grouped.setdefault(key, []).append(row)
    lines.append("| reason | fixtures |")
    lines.append("|---|---|")
    for (reason,), group in sorted(grouped.items(), key=lambda item: -len(item[1])):
        names = ", ".join(sorted(r["rel"] for r in group))
        lines.append(f"| {reason} | {len(group)}: {names} |")
    lines.append("")
    if excluded:
        lines.append("## Excluded from v1 (corrupt/garbage inputs)")
        lines.append("")
        by_class: dict[str, list[str]] = {}
        for row in excluded:
            by_class.setdefault(str(row["class"]), []).append(row["rel"])
        for klass in sorted(by_class):
            names = ", ".join(sorted(by_class[klass]))
            lines.append(f"- {klass} ({len(by_class[klass])}): {names}")
        lines.append(
            "- clean-file differential only in v1; corrupt-input comparison "
            "is the error-class rule (doc 10 §6.6), a separate card."
        )
        lines.append("")

    # Run notes (the --note channel: provenance and environment facts
    # the operator recorded for this specific run).
    notes = run_meta.get("notes") or []
    if notes:
        lines.append("## Run notes")
        lines.append("")
        for note in notes:
            lines.append(f"- {note}")
        lines.append("")

    markdown = "\n".join(lines) + "\n"

    # --- JSON evidence ---
    evidence = {
        "schema": "bench-vs-vlc/oracle-diff-report/1",
        "generated_at": run_meta["generated_at"],
        "oracle": {
            "kind": oracle.kind,
            "label": oracle.label,
            "version_pin": PINNED_VLC_VERSION,
            "version_line": oracle.assert_version_pin(),
        },
        "host": {"byteorder": sys.byteorder},
        "lumen_schema": LUMEN_SCHEMA,
        "counts": {
            "total": len(rows),
            "comparable": len(comparable),
            "non_comparable": len(non_comparable),
            "excluded": len(excluded),
        },
        "rows": [
            {
                "rel": r["rel"],
                "class": r["class"],
                "classification": r["classification"],
                "lumen": r.get("lumen"),
                "oracle_md5": r.get("oracle_md5"),
                "oracle_bytes": r.get("oracle_bytes"),
                "oracle_error": r.get("oracle_error"),
                "ffprobe_payload_bytes": r.get("ffprobe_payload_bytes"),
            }
            for r in rows
        ],
    }

    # --- labeling invariants (the separation, mechanically enforced) ---
    if mock:
        assert evidence["oracle"]["kind"] == "mock"
        assert "[MOCK]" in markdown, "mock reports carry the [MOCK] label"
        for row in rows:
            if row.get("oracle_md5") is not None or row.get("oracle_error"):
                assert any(
                    line.startswith("| [MOCK] ") and row["rel"] in line
                    for line in markdown.splitlines()
                ), f"mock artifact for {row['rel']} must sit on a [MOCK]-labeled row"
    if pending:
        assert "PENDING-VLC" in markdown
        assert all(r.get("oracle_md5") is None for r in evidence["rows"]), (
            "pending runs must carry no oracle digest"
        )
    return markdown, evidence


def entry_path(row: dict) -> Path:
    return row.get("abs_path") or Path(row["rel"])


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="differential-vs-VLC harness v1 (card 3.E3, doc 10 §6.6)"
    )
    input_group = parser.add_mutually_exclusive_group(required=True)
    input_group.add_argument(
        "--manifest", type=Path,
        help="corpus manifest JSON (docs/corpus/corpus-big-manifest-*.json)",
    )
    input_group.add_argument(
        "--files", nargs="+", type=Path,
        help="explicit media files (classified via ffprobe)",
    )
    parser.add_argument(
        "--corpus-root", type=Path, default=Path("/home/z/my-project/corpus-big"),
        help="root the manifest's rel paths resolve against",
    )
    parser.add_argument(
        "--oracle", choices=("vlc", "mock", "pending"), default="pending",
        help="oracle backend — vlc (real, version-asserted), mock (clearly "
             "labeled plumbing validation), or pending (PENDING-VLC rows; "
             "no oracle digests)",
    )
    parser.add_argument("--out", type=Path, required=True, help="report markdown path")
    parser.add_argument("--json-out", type=Path, default=None,
                        help="JSON evidence path (default: <out>.json)")
    parser.add_argument("--limit", type=int, default=None,
                        help="first N manifest entries only (debug)")
    parser.add_argument("--lumen-json", type=Path, default=None,
                        help="pre-generated Lumen digest JSON (skip the cargo run)")
    parser.add_argument("--note", action="append", default=None,
                        metavar="TEXT",
                        help="record a run note in the report's Run notes "
                             "section (repeatable) — provenance, environment "
                             "facts, install attempts")
    parser.add_argument("--no-ffprobe-check", action="store_true", default=False,
                        help="skip the ffprobe byte-count cross-check")
    args = parser.parse_args(argv)

    # 1. Fixture list.
    if args.manifest is not None:
        entries = load_manifest_entries(args.manifest, args.corpus_root, args.limit)
        manifest_meta = {}
        try:
            manifest_data = json.loads(args.manifest.read_text(encoding="utf-8"))
            manifest_meta = {
                "manifest_seed": manifest_data.get("seed"),
                "manifest_generated_at": manifest_data.get("generated_at"),
                "manifest_count": manifest_data.get("count"),
            }
        except (OSError, ValueError):
            manifest_meta = {}
        source = {"mode": "manifest", "manifest": str(args.manifest), **manifest_meta}
    else:
        entries = []
        for path in args.files or []:
            facts = probe_file_facts(path)
            # rel: corpus-relative when the file lives under --corpus-root
            # (stable, machine-independent); the absolute path otherwise.
            # class: the corpus tree's top-level directory decides the
            # corrupt/garbage exclusion (dirty/, garbage/); anything else
            # is an explicit file classified by its ffprobe facts.
            try:
                rel_path = path.resolve().relative_to(args.corpus_root.resolve())
                rel = str(rel_path)
                top = rel_path.parts[0] if len(rel_path.parts) > 1 else ""
                klass = top if top in ("dirty", "garbage") else "explicit"
            except ValueError:
                rel = str(path)
                klass = "explicit"
            entries.append({
                "rel": rel,
                "class": klass,
                "abs_path": path.resolve(),
                "params": facts,
            })
        source = {"mode": "files", "n_files": len(entries)}

    # 2. Oracle (mock/real/pending — chosen explicitly, never inferred).
    if args.oracle == "vlc":
        try:
            oracle = VlcOracle.resolve()
        except OracleUnavailable as err:
            print(f"error: {err}", file=sys.stderr)
            return 2
        try:
            oracle.assert_version_pin()
        except VersionDrift as err:
            print(f"error: {err}", file=sys.stderr)
            return 3
    elif args.oracle == "mock":
        oracle = MockOracle()
        try:
            oracle.assert_version_pin()
        except VersionDrift as err:
            print(f"error: {err}", file=sys.stderr)
            return 3
    else:
        oracle = PendingOracle()

    # 3. Lumen digests: every comparable fixture (plus the
    #    non-comparable AVIs, whose honest refusals are also data).
    lumen_json = args.lumen_json
    if lumen_json is None:
        lumen_json = args.out.with_suffix(".lumen.json")
        fixtures = [(e["rel"], entry_path(e)) for e in entries]
        run_lumen_digests(fixtures, lumen_json)
    lumen_results = load_lumen_results(lumen_json)

    # 4. Report.
    run_meta = {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "source": source,
        "notes": args.note,
    }
    markdown, evidence = build_report(
        oracle, entries, lumen_results, run_meta,
        ffprobe_check=not args.no_ffprobe_check,
    )
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(markdown, encoding="utf-8")
    json_out = args.json_out or args.out.with_suffix(".json")
    json_out.write_text(json.dumps(evidence, indent=1) + "\n", encoding="utf-8")
    counts = evidence["counts"]
    print(
        f"wrote {args.out} (+ {json_out}): "
        f"{counts['comparable']} comparable / {counts['non_comparable']} "
        f"non-comparable / {counts['excluded']} excluded, oracle={oracle.kind}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
