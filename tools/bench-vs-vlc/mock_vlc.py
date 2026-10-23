#!/usr/bin/env python3
"""mock_vlc.py — the MOCK oracle of the bench-vs-vlc harness (card 3.E3).

THIS IS NOT VLC AND NEVER PRODUCES A MEASUREMENT.

It exists so the harness's real-oracle code path (spawn an external
decoder binary with the documented flags, capture its raw-PCM dump,
digest it, compare, label) can be exercised end to end in environments
without the pinned VLC 3.0.23 binary — CI, this sandbox, unit tests.
Every artifact it feeds is labeled [MOCK] by run.py's report writer (a
mechanically-enforced invariant there), and the JSON envelope records
`oracle.kind = "mock"`.

Clean-room note (AGENTS.md §3.1): no VLC source consulted; this script
emulates only the *CLI shape* documented in this repo's own pinned
baselines (docs/baselines/raw/linux/14-options-full-help.txt — the
observed help text of the pinned binary) plus the version-line shape
from 04-vlc-version.txt.

Behavior
========
* `--version` prints `VLC version 3.0.23 ...` on the first line (the
  harness's pin check accepts exactly this shape). The line carries a
  MOCK marker so a human reading raw output is never misled.
* A normal invocation accepts the harness's documented flags
  (ignoring most), reads the input file, and writes a DETERMINISTIC
  synthetic PCM dump to --audiofile-file: an LCG stream seeded from the
  input file's SHA-256. Two runs over the same input produce identical
  bytes (determinism is what the harness's plumbing needs to test); the
  bytes have no relationship to any real decode.
* Test hooks (env vars, clearly mock-only):
    LUMEN_MOCK_VLC_ECHO=<path>   write <path>'s bytes verbatim as the
                                 dump (agreement-path testing)
    LUMEN_MOCK_VLC_VERSION=<str> override the version line
                                 (drift-path testing)
* stderr always says this is a mock.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import sys
from pathlib import Path

PINNED = "3.0.23"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="vlc (MOCK)", description="MOCK oracle stand-in — never a real measurement"
    )
    parser.add_argument("--version", action="store_true", default=False,
                        help="print the (mock) version line")
    # The documented dump flags (accepted, mostly ignored — the mock's
    # output does not depend on them except for the output path).
    parser.add_argument("-I", dest="interface", default=None)
    parser.add_argument("--play-and-exit", action="store_true", default=False)
    parser.add_argument("--vout", dest="vout", default=None)
    parser.add_argument("--aout", dest="aout", default=None)
    parser.add_argument("--no-audiofile-wav", action="store_true", default=False)
    parser.add_argument("--audiofile-format", dest="audiofile_format", default="s16")
    parser.add_argument("--audiofile-file", dest="audiofile_file", default=None)
    parser.add_argument("input", nargs="?", default=None)
    args = parser.parse_args(argv)

    version = os.environ.get("LUMEN_MOCK_VLC_VERSION", PINNED)
    if args.version:
        print(
            f"VLC version {version} MOCK ORACLE "
            f"(tools/bench-vs-vlc/mock_vlc.py — NOT the real binary, "
            f"NOT a measurement)"
        )
        return 0

    if args.audiofile_file is None or args.input is None:
        print(
            "mock_vlc: --audiofile-file and an input are required "
            "(the harness always passes both)",
            file=sys.stderr,
        )
        return 1
    source = Path(args.input)
    if not source.is_file():
        print(f"mock_vlc: input missing: {source}", file=sys.stderr)
        return 1

    out_path = Path(args.audiofile_file)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    echo = os.environ.get("LUMEN_MOCK_VLC_ECHO")
    if echo:
        # Test hook: copy the given bytes verbatim (agreement-path
        # testing). Still a mock — the report labels it so.
        out_path.write_bytes(Path(echo).read_bytes())
    else:
        # Deterministic synthetic PCM: LCG seeded from the input's
        # SHA-256. ~4 KiB per 64 KiB of input, minimum 4 KiB — enough
        # bytes for digests to be meaningful, cheap for big files.
        seed = int.from_bytes(
            hashlib.sha256(source.read_bytes()).digest()[:8], "big"
        )
        size = max(4096, source.stat().st_size // 16)
        state = seed & 0xFFFFFFFF or 1
        chunk = bytearray()
        while len(chunk) < size:
            state = (1103515245 * state + 12345) & 0xFFFFFFFF
            chunk += state.to_bytes(4, "little")
        out_path.write_bytes(bytes(chunk[:size]))

    print(
        "mock_vlc: MOCK ORACLE output written — synthetic bytes, "
        "never cite as a real measurement",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
