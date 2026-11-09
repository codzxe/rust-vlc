#!/usr/bin/env python3
"""vlc_f32_deviation.py — issue #19 §6.6 f32 deviation-path measurement.

CONTEXT (docs/corpus/vlc-diff-report-20261023.md §"Oracle run
2026-11-09" + docs/corpus/vlc-oracle-run-20261109.json):
  The VLC 3.0.23 differential run's 6 pcm_f32le rows are not
  bit-exact (VLC's float aout chain), so doc 10 §6.6 sends them to
  the sample-wise RMS/peak deviation path. The Lead decision
  (ratified by the orchestrator 2026-11-09): f32 parity is
  TOLERANCE-BASED, not bit-based — a row PASSES iff

      per-row RMS deviation  <= 1e-4   (−80 dB)
      per-row peak deviation <= 1e-3   (−60 dB)

  thresholds derived from f32 rounding at VLC's float pipeline scale.
  This script computes the measured values from ACTUAL PCM on both
  sides so the thresholds stay auditable — nothing is mocked:

    * Lumen side  = the AVI's first-audio-track `NNwb` payload
      concatenation (an independent Python RIFF walk — the same walk
      the #19 run used to re-verify every Lumen digest; for PCM-in-AVI
      Lumen's decode is the passthrough wire bytes, and this script
      ASSERTS the payload MD5 equals the Lumen harness digest of the
      SAME file before using it).
    * VLC side    = the pinned `afile` float32 dump
      (`vlc -I dummy --play-and-exit --vout none --aout afile
      --no-audiofile-wav --audiofile-format float32 …`, doc 10 §6.6).

  Deviation window: the dump is always the shorter side (afile leaves
  a tail unflushed), so samples are compared head-aligned over the
  VLC-delivered window (min(payload, dump) bytes / 4 f32 samples).

Usage:
  python3 scripts/vlc_f32_deviation.py \
      --rows REL:AVI:DUMP[:REL:AVI:DUMP…] \
      --lumen-json PATH [--out-json PATH] [--rms-max 1e-4] [--peak-max 1e-3]

  Exit code: 0 always (measurements are recorded, PASS/FAIL is data,
  not an error) — unless an ASSERT fails (input/chain breakage), which
  is exit 2.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import sys
from pathlib import Path

RMS_MAX_DEFAULT = 1e-4  # −80 dB — Lead-ratified 2026-11-09
PEAK_MAX_DEFAULT = 1e-3  # −60 dB — Lead-ratified 2026-11-09


def _walk_chunks(buf: bytes, start: int, end: int):
    """Yield (name, payload_start, payload_end) over a RIFF chunk range."""
    pos = start
    while pos + 8 <= end:
        (name, size) = struct.unpack_from("<4sI", buf, pos)
        body = pos + 8
        stop = body + size + (size & 1)  # RIFF chunks are word-aligned
        yield name.decode("latin-1"), body, min(body + size, end)
        pos = stop


def riff_first_audio_payload(buf: bytes) -> tuple[bytes, int]:
    """First `auds` strl's stream ordinal + its `NNwb` payload concat."""
    if buf[0:4] != b"RIFF" or buf[8:12] != b"AVI ":
        raise ValueError("not an AVI (RIFF/AVI magic missing)")
    stream = None
    payload = bytearray()
    for name, start, stop in _walk_chunks(buf, 12, len(buf)):
        if name != "LIST":
            continue
        ltype = buf[start : start + 4].decode("latin-1")
        if ltype == "hdrl" and stream is None:
            ordinal = 0
            for n2, s2, e2 in _walk_chunks(buf, start + 4, stop):
                if n2 != "LIST" or buf[s2 : s2 + 4].decode("latin-1") != "strl":
                    continue
                for n3, s3, e3 in _walk_chunks(buf, s2 + 4, e2):
                    if n3 == "strh" and buf[s3 : s3 + 4] == b"auds":
                        stream = ordinal  # first audio stream (track rule)
                    break
                if stream is not None:
                    break
                ordinal += 1
        elif ltype == "movi":
            tag = "%02dwb" % stream if stream is not None else None
            stack = [(start + 4, stop)]
            while stack:  # chunks may nest one LIST 'rec ' deep
                mstart, mstop = stack.pop()
                for n2, s2, e2 in _walk_chunks(buf, mstart, mstop):
                    if n2 == "LIST" and buf[s2 : s2 + 4] == b"rec ":
                        stack.append((s2 + 4, e2))
                    elif tag is not None and n2 == tag:
                        payload += buf[s2:e2]
    if stream is None or not payload:
        raise ValueError("no first-audio-track 01wb payload found")
    return bytes(payload), stream


def rms_peak(lumen: bytes, dump: bytes) -> dict:
    """Head-aligned f32 sample-wise deviation over the delivered window."""
    n = min(len(lumen), len(dump))
    if n % 4:
        n -= n % 4
    count = n // 4
    a = struct.unpack_from("<%df" % count, lumen, 0)
    b = struct.unpack_from("<%df" % count, dump, 0)
    acc = 0.0
    peak = 0.0
    for x, y in zip(a, b):
        d = x - y
        acc += d * d
        ad = d if d >= 0 else -d
        if ad > peak:
            peak = ad
    return {
        "samples_compared": count,
        "lumen_bytes": len(lumen),
        "vlc_bytes": len(dump),
        "rms_dev": (acc / count) ** 0.5 if count else None,
        "peak_dev": peak if count else None,
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--rows", required=True,
                    help="REL:AVI:DUMP triples, ':'-separated within, ','-separated between")
    ap.add_argument("--lumen-json", required=True,
                    help="the Lumen batch JSON for the SAME files (digest assert)")
    ap.add_argument("--out-json")
    ap.add_argument("--rms-max", type=float, default=RMS_MAX_DEFAULT)
    ap.add_argument("--peak-max", type=float, default=PEAK_MAX_DEFAULT)
    args = ap.parse_args()

    lumen = json.loads(Path(args.lumen_json).read_text())
    digests = {}
    for row in lumen["results"]:
        digests[row["rel"]] = row.get("md5") or row.get("md5_hex")

    rows = []
    broken = False
    for triple in args.rows.split(","):
        rel, avi, dump = triple.split(":")
        buf = Path(avi).read_bytes()
        payload, _stream = riff_first_audio_payload(buf)
        md5 = hashlib.md5(payload).hexdigest()
        assert_ok = md5 == digests.get(rel)
        if not assert_ok:
            broken = True
        meas = rms_peak(payload, Path(dump).read_bytes())
        rms, peak = meas["rms_dev"], meas["peak_dev"]
        verdict = ("PASS" if (rms is not None and rms <= args.rms_max
                              and peak is not None and peak <= args.peak_max)
                   else "FAIL")
        rows.append({
            "rel": rel,
            "payload_md5": md5,
            "lumen_digest_assert": assert_ok,
            **meas,
            "rms_max": args.rms_max,
            "peak_max": args.peak_max,
            "verdict": verdict,
        })
        print("%-28s RMS %-14.7g peak %-14.7g over %8d samples  %s%s" % (
            rel, rms, peak, meas["samples_compared"], verdict,
            "" if assert_ok else "  [LUMEN-DIGEST-ASSERT-FAILED]"))

    out = {
        "schema": "corpus/vlc-f32-tolerance/1",
        "issue": 19,
        "decision": {
            "ratified": "2026-11-09 (Lead decision, ratified by the orchestrator)",
            "policy": "f32 parity is tolerance-based, not bit-based: "
                      "PASS iff per-row RMS deviation <= 1e-4 (-80 dB) "
                      "AND peak deviation <= 1e-3 (-60 dB)",
            "rationale": "thresholds derived from f32 rounding at VLC's "
                         "float pipeline scale (doc 10 §6.6 deviation path)",
        },
        "rows": rows,
        "summary": {
            "pass": sum(1 for r in rows if r["verdict"] == "PASS"),
            "fail": sum(1 for r in rows if r["verdict"] == "FAIL"),
        },
    }
    if args.out_json:
        Path(args.out_json).write_text(json.dumps(out, indent=1) + "\n")
    return 2 if broken else 0


if __name__ == "__main__":
    sys.exit(main())
