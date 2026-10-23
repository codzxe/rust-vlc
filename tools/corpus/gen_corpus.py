#!/usr/bin/env python3
"""Lumen randomized real-media corpus generator — v4, ESCALATION.

ED-FIX-5 charter; user directives 2026-10-22 (random, real, many, measured),
2026-10-23 (industrial: parallel, VFR chaos, multi-track/subs/chapters,
dynamic resolution, anamorphic, crash minimization companion, HLS/fMP4,
fidelity scoring companion, FFmpeg speed parity companion, adversarial by
default, no hardcoded shortcuts), and the W5-16 escalation order (corpus's
only job: break the player — more brittle, more unusual, more hostile).

Design contract (binding, extends v3):
  * RANDOM by default — seed from OS entropy; the full randomness is frozen
    into plan.json before any encoding starts, so a reaper kill mid-run
    loses nothing and `--seed S` (or replaying the recorded seed)
    reproduces the exact corpus.
  * PARALLEL — ffmpeg encodes run on a ThreadPool (subprocesses give true
    CPU parallelism); jobs default to the core count.  Manifest writes are
    serialized in the main thread; completion order cannot change content.
  * COVERAGE-GUIDED — the stratification matrix pins every key surface
    (codec x container, VFR, multi-track, subs, chapters, dyn-res, raw,
    fMP4, HLS, cover art, drift, bombs + the 15 v4 escalation classes);
    random composition fills the rest.
  * CAPABILITY-DISCOVERED — 10-bit profiles, prores, rotation metadata
    routes, HLS-fMP4, concat-copy, zeroframe tracks, libxvid, HEVC-in-AVI,
    mpeg2-in-mp4, odd 1x1, fractional-framerate stills are PROBED against
    the real ffmpeg at run start; unsupported draws land in the honest
    rejected-draws ledger.
  * DIRTY WORLD (D-3) — ~18% of clean files get a corrupted sibling (12
    byte-level ops incl. cross-file splices and header size lies) plus 22
    garbage archetypes, plus v4's dedicated dirty-class factories
    (riff-size-lie, odd-chunk-avi byte post-processing).  Originals stay
    clean for oracle comparison.
  * HONEST LEDGER — encoder rejections, over-budget skips, capability
    gaps, and ffprobe failures of CLEAN files are all recorded as data.

v4 escalation classes (W5-16; every param randomized within
measured-encoder-feasible bounds, no lazy hardcoded constants):
  rot-display      display-matrix rotation (90/180/270 + flips) mp4/mkv/mov
  odd-dims         1x1..101x101 odd/sub-16 geometry, yuv422/444 where allowed
  maxtracks-avi    AVI with 16-32 mixed video/audio/data streams
  bframe-storm     h264/h265 bf>=3 + b-pyramid reorder storms (mp4/ts/mkv/mov)
  packed-bs        libxvid packed b-frames / mpeg4 bf / avcC oddities
  param-midchange  mid-stream SPS/bitrate/profile change via concat copy
  vbv-burst        extreme bitrate spikes (concat step or single-pass VBV)
  fps-extremes     1 fps slideshow <-> 240 fps tiny-res burst
  sr-chaos         8000/11025/192000/384000 Hz + odd channel layouts
  chapter-bomb     1000-3000 FFMETADATA chapters on a seconds-long clip
  zeroframe-stream video track declared, zero samples (mkv/ts)
  still-24h        single-frame ~6-48 h still, bounded bytes
  riff-size-lie    dirty-class: AVI RIFF size field lies (byte post-op)
  odd-chunk-avi    1-byte-payload movi chunks + JUNK pad storms (post-op)
  hevc-in-avi      the lie-class: HEVC stream renamed to an AVI video tag

Usage:
  gen_corpus.py [--out DIR] [--preset smoke|standard|big|insane]
                [--count N] [--budget-mb MB] [--jobs J] [--seed S]
                [--no-dirty] [--no-garbage] [--no-validate]
  gen_corpus.py --replay manifest.json [--out DIR]   # re-run recorded seed

Companions (same directory):
  minimize.py   — ddmin crash reproducer minimization
  fidelity.py   — PSNR/SSIM differential scoring vs the ffmpeg oracle
  speed_bench.py— decode-speed benchmark vs ffmpeg (parity target)
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import random
import secrets
import shutil
import struct
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from corpus_vocab import (  # noqa: E402
    ANAMORPHIC_SAR, AUDIO_CODECS, AUDIO_CONTAINERS, CLASS_WEIGHTS,
    CONTAINERS, CORRUPTION_OPS, GARBAGE_ARCHETYPES, MANDATORY_RESOLUTIONS,
    PRESETS, RAW_VIDEO_TARGETS, RES_ORDER, ROTATIONS, STRATIFICATION,
    V4_CLASSES, VIDEO_CODECS,
)
from corpus_builders import BUILDERS, Ctx, draw_audio_params  # noqa: E402

FFMPEG = os.environ.get("FFMPEG", "ffmpeg")
FFPROBE = os.environ.get("FFPROBE", "ffprobe")

AVI_COMBOS = ["mjpeg+pcm_s16le", "mjpeg+pcm_u8", "mpeg4+pcm_s16le",
              "mpeg4+pcm_u8", "mjpeg+pcm_s24le", "mjpeg+pcm_f32le",
              "mpeg4+pcm_s24le", "mjpeg+pcm_s32le"]
VFR_CONTAINERS = ["mp4", "mkv", "mov"]
MULTI_CONTAINERS = ["mkv", "mp4"]
BOMB_CONTAINERS = ["mkv", "mp4"]
DRIFT_CONTAINERS = ["mkv", "mp4"]

# manifest class -> builder kind (BUILDERS is keyed by kind)
KIND_FOR_CLASS = {
    "avi-video": "video", "mp4-video": "video", "mkv-video": "video",
    "mov-video": "video", "ts-video": "video", "flv-video": "video",
    "webm-video": "video", "ogv-video": "video",
    "avi-av": "av", "muxed-av": "av", "vfr-av": "av",
    "audio-only": "audio", "multi-av": "multi-av",
    "dynres-video": "dynres-video", "raw-video": "raw-video",
    "fmp4-video": "fmp4-video", "hls-av": "hls-av",
    "cover-audio": "cover-audio", "multivideo-av": "multivideo-av",
    "drift-av": "drift-av", "meta-bomb-av": "meta-bomb-av",
    # ---- v4 escalation classes (W5-16) ----
    "rot-display": "rot-display", "odd-dims": "odd-dims",
    "maxtracks-avi": "maxtracks-avi", "bframe-storm": "bframe-storm",
    "packed-bs": "packed-bs", "param-midchange": "param-midchange",
    "vbv-burst": "vbv-burst", "fps-extremes": "fps-extremes",
    "sr-chaos": "sr-chaos", "chapter-bomb": "chapter-bomb",
    "zeroframe-stream": "zeroframe-stream", "still-24h": "still-24h",
    "riff-size-lie": "riff-size-lie", "odd-chunk-avi": "odd-chunk-avi",
    "hevc-in-avi": "hevc-in-avi",
}

# Classes whose generated files are lies by construction — they land in
# dirty/ and are excluded from the clean-file ffprobe gate (like v3's
# derived dirty classes).
DIRTY_CLASSES = {"riff-size-lie"}


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def run(cmd: list[str], timeout: int) -> tuple[int, str]:
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return 124, "timeout"
    except OSError as e:
        return 127, str(e)
    if p.returncode != 0:
        tail = (p.stderr or "").strip().splitlines()
        return p.returncode, (tail[-1][:200] if tail else f"rc={p.returncode}")
    return 0, ""


# ---------------------------------------------------------------------------
# Capability probes — the anti-hardcode layer.  Each probe is a tiny real
# encode; the result gates the corresponding draws and is recorded in the
# manifest.  Probes run in parallel.
# ---------------------------------------------------------------------------

def probe_commands() -> dict[str, list[str]]:
    tiny = ["-f", "lavfi", "-i", "testsrc2=duration=0.3:size=128x96:rate=10"]
    sine = ["-f", "lavfi", "-i", "sine=duration=0.3:sample_rate=48000"]
    null = ["-f", "null", "-"]
    return {
        "x264_10bit": tiny + ["-c:v", "libx264", "-preset", "ultrafast",
                              "-pix_fmt", "yuv420p10le"] + null,
        "x264_422": tiny + ["-c:v", "libx264", "-preset", "ultrafast",
                            "-pix_fmt", "yuv422p"] + null,
        "x264_444": tiny + ["-c:v", "libx264", "-preset", "ultrafast",
                            "-pix_fmt", "yuv444p"] + null,
        "x265_10bit": tiny + ["-c:v", "libx265", "-preset", "ultrafast",
                              "-pix_fmt", "yuv420p10le"] + null,
        "vp9_10bit": tiny + ["-c:v", "libvpx-vp9", "-deadline", "realtime",
                             "-pix_fmt", "yuv420p10le"] + null,
        "svtav1_10bit": tiny + ["-c:v", "libsvtav1", "-preset", "8",
                                "-pix_fmt", "yuv420p10le"] + null,
        "alac": sine + ["-c:a", "alac", "-f", "matroska", "-y", "__TMP__"],
        "prores": tiny + ["-c:v", "prores_ks", "-pix_fmt", "yuv422p10le",
                          "-f", "mov", "-y", "__TMP__"],
        "prores444": tiny + ["-c:v", "prores_ks", "-pix_fmt", "yuv444p10le",
                             "-f", "mov", "-y", "__TMP__"],
        "adpcm_ms": sine + ["-c:a", "adpcm_ms", "-f", "wav", "-y", "__TMP__"],
        "adpcm_ima": sine + ["-c:a", "adpcm_ima_wav", "-f", "wav", "-y", "__TMP__"],
        # ---- v4 escalation probes (W5-16) ----
        "xvid": tiny + ["-c:v", "libxvid", "-bf", "2", "-g", "24"] + null,
        "hevc_avi": tiny + ["-c:v", "libx265", "-preset", "ultrafast",
                            "-pix_fmt", "yuv420p", "-tag:v", "h265",
                            "-f", "avi", "-y", "__TMP__"],
        "mpeg2_mp4": tiny + ["-c:v", "mpeg2video", "-b:v", "300k",
                             "-f", "mp4", "-y", "__TMP__"],
    }


def probe_caps(ffmpeg: str, tmp_root: Path) -> dict:
    caps: dict = {}
    probes = probe_commands()
    with ThreadPoolExecutor(max_workers=4) as ex:
        futs = {}
        for name, cmd in probes.items():
            if "__TMP__" in cmd:
                cmd = [c for c in cmd]
                tmp = tmp_root / f"probe-{name}.bin"
                cmd[cmd.index("__TMP__")] = str(tmp)
                futs[ex.submit(run, [ffmpeg, "-hide_banner", "-loglevel",
                                     "error", "-nostdin"] + cmd, 60)] = (name, tmp)
            else:
                futs[ex.submit(run, [ffmpeg, "-hide_banner", "-loglevel",
                                     "error", "-nostdin"] + cmd, 60)] = (name, None)
        for fut in as_completed(futs):
            name, tmp = futs[fut]
            rc, _ = fut.result()
            caps[name] = (rc == 0)
            if tmp is not None:
                Path(tmp).unlink(missing_ok=True)
    # rotation metadata probe: encode + verify ffprobe sees the rotation
    rot_tmp = tmp_root / "probe-rot.mp4"
    rc, _ = run([ffmpeg, "-hide_banner", "-loglevel", "error", "-nostdin",
                 "-f", "lavfi", "-i", "testsrc2=duration=0.3:size=64x48:rate=10",
                 "-metadata:s:v:0", "rotate=90", "-c:v", "libx264",
                 "-preset", "ultrafast", "-pix_fmt", "yuv420p",
                 "-f", "mp4", "-y", str(rot_tmp)], 60)
    if rc == 0:
        p = subprocess.run([FFPROBE, "-v", "error", "-select_streams", "v:0",
                            "-show_entries", "stream_tags=rotate",
                            "-of", "csv=p=0", str(rot_tmp)],
                           capture_output=True, text=True, timeout=30)
        caps["rotate_meta"] = "90" in (p.stdout or "")
        rot_tmp.unlink(missing_ok=True)
    else:
        caps["rotate_meta"] = False
    # vfr probe: jittered setpts must mux into mp4
    vfr_tmp = tmp_root / "probe-vfr.mp4"
    rc, _ = run([ffmpeg, "-hide_banner", "-loglevel", "error", "-nostdin",
                 "-f", "lavfi", "-i", "testsrc2=duration=0.5:size=64x48:rate=30",
                 "-vf", "setpts='(N*0.033+0.008*abs(sin(N/7)))/TB'",
                 "-fps_mode:v", "vfr", "-c:v", "libx264", "-preset",
                 "ultrafast", "-pix_fmt", "yuv420p", "-f", "mp4", "-y",
                 str(vfr_tmp)], 60)
    caps["vfr_mp4"] = (rc == 0)
    vfr_tmp.unlink(missing_ok=True)
    # hls fmp4 probe
    hls_dir = tmp_root / "probe-hls"
    hls_dir.mkdir(parents=True, exist_ok=True)
    rc, _ = run([ffmpeg, "-hide_banner", "-loglevel", "error", "-nostdin",
                 "-f", "lavfi", "-i", "testsrc2=duration=1:size=64x48:rate=10",
                 "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt",
                 "yuv420p", "-f", "hls", "-hls_time", "0.5",
                 "-hls_list_size", "0", "-hls_segment_type", "fmp4",
                 "-hls_segment_filename", str(hls_dir / "s%03d.m4s"),
                 "-y", str(hls_dir / "p.m3u8")], 60)
    caps["hls_fmp4"] = (rc == 0 and (hls_dir / "p.m3u8").is_file())
    shutil.rmtree(hls_dir, ignore_errors=True)
    # concat -c copy with mid-stream resolution change, per container
    for cont, fmt in (("mkv", "matroska"), ("ts", "mpegts"), ("mp4", "mp4")):
        d = tmp_root / f"probe-conc-{cont}"
        d.mkdir(parents=True, exist_ok=True)
        s1, s2 = d / f"s1.{cont}", d / f"s2.{cont}"
        rc1, _ = run([ffmpeg, "-hide_banner", "-loglevel", "error", "-nostdin",
                      "-f", "lavfi", "-i", "testsrc2=duration=0.4:size=160x120:rate=10",
                      "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt",
                      "yuv420p", "-f", fmt, "-y", str(s1)], 60)
        rc2, _ = run([ffmpeg, "-hide_banner", "-loglevel", "error", "-nostdin",
                      "-f", "lavfi", "-i", "testsrc2=duration=0.4:size=320x240:rate=10",
                      "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt",
                      "yuv420p", "-f", fmt, "-y", str(s2)], 60)
        lst = d / "list.txt"
        lst.write_text(f"file '{s1}'\nfile '{s2}'\n")
        out = d / f"out.{cont}"
        rc3, _ = run([ffmpeg, "-hide_banner", "-loglevel", "error", "-nostdin",
                      "-f", "concat", "-safe", "0", "-i", str(lst),
                      "-c", "copy", "-y", str(out)], 60)
        caps[f"concat_copy:{cont}"] = (rc1 == 0 and rc2 == 0 and rc3 == 0
                                       and out.stat().st_size > 0)
        shutil.rmtree(d, ignore_errors=True)
    # ---- v4 escalation probes (W5-16) ----
    # display-matrix rotation via -display_rotation on the input + copy:
    # movenc AND matroskaenc write real display matrices this way (the v3
    # `rotate=` metadata route is dead on ffmpeg 7.x).
    rot_src = tmp_root / "probe-v4-rot-src.mp4"
    rc, _ = run([ffmpeg, "-hide_banner", "-loglevel", "error", "-nostdin",
                 "-f", "lavfi", "-i", "testsrc2=duration=0.3:size=64x48:rate=10",
                 "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt",
                 "yuv420p", "-f", "mp4", "-y", str(rot_src)], 60)
    for cont, fmt in (("mp4", "mp4"), ("mov", "mov"), ("mkv", "matroska")):
        dst = tmp_root / f"probe-v4-rot.{cont}"
        if rc == 0:
            rc2, _ = run([ffmpeg, "-hide_banner", "-loglevel", "error",
                          "-nostdin", "-display_rotation:v:0", "90",
                          "-i", str(rot_src), "-c", "copy", "-f", fmt,
                          "-y", str(dst)], 60)
            ok = rc2 == 0
            if ok:
                p = subprocess.run([FFPROBE, "-v", "error", "-show_streams",
                                    str(dst)], capture_output=True, text=True,
                                   timeout=30)
                ok = "displaymatrix" in (p.stdout or "").lower()
        else:
            ok = False
        caps[f"rot_copy:{cont}"] = ok
        dst.unlink(missing_ok=True)
    rot_src.unlink(missing_ok=True)
    # tkhd patch route (flips): encode, patch, ffprobe must see the matrix
    tk_src = tmp_root / "probe-v4-tkhd.mp4"
    rc, _ = run([ffmpeg, "-hide_banner", "-loglevel", "error", "-nostdin",
                 "-f", "lavfi", "-i", "testsrc2=duration=0.3:size=64x48:rate=10",
                 "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt",
                 "yuv420p", "-f", "mp4", "-y", str(tk_src)], 60)
    if rc == 0:
        note = post_tkhd_rotate(tk_src, {"rot": 90, "hflip": 1, "vflip": 0},
                                random.Random(1))
        p = subprocess.run([FFPROBE, "-v", "error", "-show_streams",
                            str(tk_src)], capture_output=True, text=True,
                           timeout=30)
        caps["rot_tkhd"] = bool(note.get("applied")) and \
            "displaymatrix" in (p.stdout or "").lower()
    else:
        caps["rot_tkhd"] = False
    tk_src.unlink(missing_ok=True)
    # 1x1 odd-dimension probe (color source preserves 1x1; mpeg4 accepts)
    odd_tmp = tmp_root / "probe-v4-odd.avi"
    rc, _ = run([ffmpeg, "-hide_banner", "-loglevel", "error", "-nostdin",
                 "-f", "lavfi", "-i", "color=size=1x1:rate=10:duration=0.3",
                 "-c:v", "mpeg4", "-pix_fmt", "yuv420p", "-f", "avi", "-y",
                 str(odd_tmp)], 60)
    if rc == 0:
        p = subprocess.run([FFPROBE, "-v", "error", "-select_streams", "v:0",
                            "-show_entries", "stream=width", "-of", "csv=p=0",
                            str(odd_tmp)], capture_output=True, text=True,
                           timeout=30)
        caps["odd1x1"] = (p.stdout or "").strip() == "1"
    else:
        caps["odd1x1"] = False
    odd_tmp.unlink(missing_ok=True)
    # zeroframe tracks: select=0 + audio; the video TRACK must survive the
    # muxer (mp4 rc=0 but silently drops the track — that is the probe truth)
    for cont, fmt in (("mkv", "matroska"), ("ts", "mpegts"), ("mp4", "mp4")):
        zf = tmp_root / f"probe-v4-zf.{cont}"
        rc, _ = run([ffmpeg, "-hide_banner", "-loglevel", "error", "-nostdin",
                     "-f", "lavfi", "-i",
                     "testsrc2=duration=0.5:size=64x48:rate=10",
                     "-f", "lavfi", "-i", "sine=duration=0.5:sample_rate=8000",
                     "-map", "0:v", "-map", "1:a", "-vf", "select=0",
                     "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt",
                     "yuv420p", "-c:a", "pcm_u8", "-f", fmt, "-y",
                     str(zf)], 60)
        ok = rc == 0
        if ok:
            p = subprocess.run([FFPROBE, "-v", "error", "-select_streams",
                                "v:0", "-show_entries", "stream=codec_type",
                                "-of", "csv=p=0", str(zf)],
                               capture_output=True, text=True, timeout=30)
            ok = "video" in (p.stdout or "")
        caps[f"zeroframe:{cont}"] = ok
        zf.unlink(missing_ok=True)
    # fractional-framerate image2 loop (single-frame still, huge duration)
    still_dir = tmp_root / "probe-v4-still"
    still_dir.mkdir(parents=True, exist_ok=True)
    png = still_dir / "s.png"
    out24 = still_dir / "s.mp4"
    rc1, _ = run([ffmpeg, "-hide_banner", "-loglevel", "error", "-nostdin",
                  "-f", "lavfi", "-i",
                  "testsrc2=duration=0.04:size=160x120:rate=25",
                  "-frames:v", "1", "-y", str(png)], 60)
    rc2, _ = run([ffmpeg, "-hide_banner", "-loglevel", "error", "-nostdin",
                  "-loop", "1", "-framerate", "1/21600", "-i", str(png),
                  "-t", "21600", "-c:v", "libx264", "-preset", "ultrafast",
                  "-pix_fmt", "yuv420p", "-y", str(out24)], 60)
    ok = rc1 == 0 and rc2 == 0
    if ok:
        p = subprocess.run([FFPROBE, "-v", "error", "-show_entries",
                            "format=duration", "-of", "csv=p=0", str(out24)],
                           capture_output=True, text=True, timeout=30)
        try:
            ok = float((p.stdout or "0").strip()) > 10_000.0
        except ValueError:
            ok = False
    caps["still_frac"] = ok
    shutil.rmtree(still_dir, ignore_errors=True)
    return caps


def available_encoders(ffmpeg: str) -> set[str]:
    out = subprocess.run([ffmpeg, "-hide_banner", "-encoders"],
                         capture_output=True, text=True, timeout=60)
    encs: set[str] = set()
    for line in out.stdout.splitlines():
        fields = line.split()
        if len(fields) >= 2 and len(fields[0]) == 6 and fields[0][0] in "VAS":
            encs.add(fields[1])
    return encs


# ---------------------------------------------------------------------------
# v4 byte-level post-ops (W5-16).  Builders freeze a spec into the plan;
# these run AFTER the argvs succeed and BEFORE harvest/validation.  All
# structure walks are bounded and refuse (recorded, not raised) on shapes
# they do not recognize — dirty-world discipline (D-3).
# ---------------------------------------------------------------------------

def _avi_boxes(buf: bytes, start: int, end: int):
    """Bounded top-level RIFF chunk walk (size<8 or overrun stops it).
    RIFF chunk sizes cover the DATA only — advance past header+data+pad."""
    off = start
    while off + 8 <= end:
        size = struct.unpack("<I", buf[off + 4:off + 8])[0]
        if size < 8 or off + 8 + size > end:
            return
        yield buf[off:off + 4], off, size
        off += 8 + size + (size & 1)


def _avi_find_movi_idx1(buf: bytes):
    movi = idx1 = None
    for cid, off, size in _avi_boxes(buf, 12, len(buf)):
        if cid == b"LIST" and buf[off + 8:off + 12] == b"movi" and movi is None:
            movi = (off, size)
        elif cid == b"idx1" and idx1 is None:
            idx1 = (off, size)
    return movi, idx1


def _avi_movi_chunks(buf: bytes, movi):
    """Chunks inside the movi LIST (children start past the 'movi' fourcc),
    descending into 'rec ' groupings; JUNK skipped."""
    start, end = movi[0] + 12, movi[0] + 8 + movi[1]
    stack = [(start, end)]
    while stack:
        lo, hi = stack.pop()
        off = lo
        while off + 8 <= hi:
            cid = buf[off:off + 4]
            size = struct.unpack("<I", buf[off + 4:off + 8])[0]
            if off + 8 + size > hi:
                break
            if cid == b"LIST" and buf[off + 8:off + 12] == b"rec ":
                stack.append((off + 12, off + size))
            elif cid not in (b"JUNK", b"LIST"):
                yield cid, off, size, buf[off + 8:off + 8 + size]
            off += 8 + size + (size & 1)


def post_avi_rechunk(path: Path, spec: dict, erng: random.Random) -> dict:
    """Re-pack the movi payload as `split`-byte chunks + JUNK pad storms;
    rebuild idx1 to match (or drop it as JUNK).  Stale header fields (e.g.
    dwSuggestedBufferSize) are left lying — that is the point."""
    buf = path.read_bytes()
    if buf[:4] != b"RIFF":
        return {"op": "avi-rechunk", "applied": False,
                "reason": "not-riff"}
    movi, idx1 = _avi_find_movi_idx1(buf)
    if movi is None:
        return {"op": "avi-rechunk", "applied": False,
                "reason": "no-movi"}
    split = max(1, int(spec.get("split", 1)))
    junk_every = max(0, int(spec.get("junk_every", 3)))
    junk_lo = max(1, int(spec.get("junk_lo", 5)))
    junk_hi = max(junk_lo, int(spec.get("junk_hi", 64)))
    drop_idx1 = bool(spec.get("drop_idx1", False))
    out = bytearray(buf[:movi[0] + 12])
    idx_entries = []
    n_in = 0
    for k, (cid, _off, _sz, payload) in enumerate(_avi_movi_chunks(buf, movi)):
        n_in += 1
        for j in range(0, len(payload), split):
            piece = payload[j:j + split]
            rel = len(out) - (movi[0] + 8)
            out += cid + struct.pack("<I", len(piece)) + piece
            if len(piece) & 1:
                out += b"\x00"
            idx_entries.append((cid, rel, len(piece), 0x10 if k == 0 else 0))
        if junk_every and k % junk_every == 0:
            jsz = erng.randint(junk_lo, junk_hi)
            out += b"JUNK" + struct.pack("<I", jsz) + \
                bytes(erng.randrange(256) for _ in range(jsz))
            if jsz & 1:
                out += b"\x00"
    # movi payload ends HERE (before idx1 / its JUNK replacement is appended)
    movi_end = len(out)
    if idx1 is not None and drop_idx1:
        out += b"JUNK" + struct.pack("<I", idx1[1]) + \
            b"\x00" * idx1[1]
    else:
        idx = bytearray(b"idx1" + struct.pack("<I", 16 * len(idx_entries)))
        for cid, rel, sz, fl in idx_entries:
            idx += cid + struct.pack("<III", fl, rel, sz)
        out += idx
    # The movi LIST size MUST be repatched to the re-packed payload — a
    # stale (smaller) size makes every idx1 entry land "outside movi" and
    # demuxers truncate the walk at the old boundary (measured: ffmpeg
    # decode of the rechunked file failed with "No JPEG data found" until
    # this patch).  LIST size covers the 'movi' fourcc + chunk bytes.
    struct.pack_into("<I", out, movi[0] + 4, movi_end - (movi[0] + 8))
    struct.pack_into("<I", out, 4, len(out) - 8)
    path.write_bytes(bytes(out))
    return {"op": "avi-rechunk", "applied": True, "split": split,
            "chunks_in": n_in, "chunks_out": len(idx_entries),
            "junk_every": junk_every, "drop_idx1": drop_idx1,
            "bytes": len(out)}


def post_riff_size_lie(path: Path, spec: dict, erng: random.Random) -> dict:
    """Patch the RIFF size field with a drawn lie (oversize/max/tiny/zero)."""
    buf = bytearray(path.read_bytes())
    if len(buf) < 8 or buf[:4] != b"RIFF":
        return {"op": "riff-size-lie", "applied": False,
                "reason": "not-riff"}
    real = len(buf) - 8
    mode = spec.get("mode", "oversize")
    if mode == "oversize":
        lie = real + int(spec.get("oversize_bytes", 1 << 20))
    elif mode == "max":
        lie = 0xFFFFFFFF
    elif mode == "tiny":
        lie = int(spec.get("tiny_bytes", 7))
    else:  # zero
        lie = 0
    lie &= 0xFFFFFFFF
    buf[4:8] = struct.pack("<I", lie)
    path.write_bytes(bytes(buf))
    return {"op": "riff-size-lie", "applied": True, "mode": mode,
            "real": real, "lie": lie}


def _mp4_boxes(buf: bytes, start: int, end: int):
    off = start
    while off + 8 <= end:
        size = struct.unpack(">I", buf[off:off + 4])[0]
        typ = buf[off + 4:off + 8]
        if size == 1:
            if off + 16 > end:
                return
            size = struct.unpack(">Q", buf[off + 8:off + 16])[0]
        if size < 8 or off + size > end:
            return
        yield typ, off, size
        off += size + (size & 1)


def post_tkhd_rotate(path: Path, spec: dict, erng: random.Random) -> dict:
    """Patch the video trak's tkhd transformation matrix (16.16 fixed).
    v0 matrix sits at body+40, v1 at body+52 (creation/mod/duration grow).
    Composes flip scale into the rotation, ISO 14496-12 style."""
    buf = bytearray(path.read_bytes())
    mat_off = None
    for typ, off, size in _mp4_boxes(buf, 0, len(buf)):
        if typ != b"moov":
            continue
        for t2, o2, s2 in _mp4_boxes(buf, off + 8, off + size):
            if t2 != b"trak":
                continue
            is_video = False
            tkhd_mat = None
            for t3, o3, s3 in _mp4_boxes(buf, o2 + 8, o2 + s2):
                if t3 == b"tkhd":
                    body = o3 + 8
                    ver = buf[body]
                    tkhd_mat = body + (40 if ver == 0 else 52)
                elif t3 == b"mdia":
                    for t4, o4, s4 in _mp4_boxes(buf, o3 + 8, o3 + s3):
                        if t4 == b"hdlr" and buf[o4 + 16:o4 + 20] == b"vide":
                            is_video = True
            if is_video and tkhd_mat is not None:
                mat_off = tkhd_mat
                break
        if mat_off is not None:
            break
    if mat_off is None or mat_off + 36 > len(buf):
        return {"op": "tkhd-rotate", "applied": False,
                "reason": "no-video-tkhd"}
    rot = int(spec.get("rot", 0)) % 360
    hflip = bool(spec.get("hflip", 0))
    vflip = bool(spec.get("vflip", 0))
    # rotation quadrant -> 2x2 integer matrix (row-major)
    quads = {0: (1, 0, 0, 1), 90: (0, 1, -1, 0), 180: (-1, 0, 0, -1),
             270: (0, -1, 1, 0)}
    a, b, c, d = quads[rot]
    if hflip:
        a, b = -a, -b
    if vflip:
        c, d = -c, -d
    one, w16 = 0x10000, 0x40000000
    matrix = [a * one, b * one, 0, c * one, d * one, 0, 0, 0, w16]
    buf[mat_off:mat_off + 36] = b"".join(
        struct.pack(">i", v) for v in matrix)
    path.write_bytes(bytes(buf))
    return {"op": "tkhd-rotate", "applied": True, "rot": rot,
            "hflip": hflip, "vflip": vflip,
            "name": spec.get("name", str(rot))}


POST_OPS = {
    "avi-rechunk": post_avi_rechunk,
    "riff-size-lie": post_riff_size_lie,
    "tkhd-rotate": post_tkhd_rotate,
}


# ---------------------------------------------------------------------------
# Plan drawing
# ---------------------------------------------------------------------------

def draw_family(rng: random.Random, cls: str):
    if cls == "avi-video":
        codec = rng.choices(CONTAINERS["avi"]["codecs"],
                            weights=[30, 18, 8, 6, 6, 4, 4], k=1)[0]
        return ("avi", codec)
    if cls == "avi-av":
        return ("avi", rng.choice(AVI_COMBOS))
    if cls in ("mp4-video", "mkv-video", "mov-video", "ts-video", "flv-video",
               "webm-video", "ogv-video"):
        container = cls.split("-")[0]
        return (container, rng.choice(CONTAINERS[container]["codecs"]))
    if cls == "audio-only":
        codec = rng.choices(list(AUDIO_CODECS.keys()),
                            weights=[16, 6, 4, 5, 4, 3, 12, 10, 10, 8, 6, 4,
                                     3, 3, 3, 2, 2], k=1)[0]
        return (rng.choice(AUDIO_CODECS[codec]["cont"]), codec)
    if cls == "muxed-av":
        return rng.choice([
            ("mp4", "h264+aac"), ("mp4", "h265+aac"), ("mkv", "h264+mp3"),
            ("mkv", "vp9+opus"), ("mkv", "h264+aac"), ("mp4", "mpeg4+aac"),
            ("mov", "h264+aac"), ("ts", "h264+aac"), ("webm", "vp9+opus"),
        ])
    if cls == "vfr-av":
        return (rng.choice(VFR_CONTAINERS),
                rng.choice(["h264+aac", "h264+mp3", "vp9+opus",
                            "mjpeg+pcm_s16le", "mpeg4+aac", "h265+aac"]))
    if cls == "multi-av":
        base = rng.choice(["h264", "h264", "mpeg4"])
        a1 = rng.choice(["aac", "mp3", "ac3"])
        a2 = rng.choice([c for c in ("aac", "mp3", "ac3", "opus", "flac",
                                     "pcm_s16le", "vorbis") if c != a1])
        toks = f"{base}+{a1}+{a2}"
        if rng.random() < 0.55:
            toks += "+subs"
        if rng.random() < 0.45:
            toks += "+chapters"
        return (rng.choice(MULTI_CONTAINERS), toks)
    if cls == "dynres-video":
        return (rng.choice(["mkv", "ts", "mp4"]),
                rng.choice(["h264", "h264", "h264", "mpeg2video", "mpeg4"]))
    if cls == "raw-video":
        return ("raw", rng.choice(["h264", "hevc"]))
    if cls == "fmp4-video":
        return (rng.choice(["mp4", "mp4", "mov"]),
                rng.choice(["h264", "h265"]))
    if cls == "hls-av":
        return (rng.choice(["hls", "hlsf"]), "h264+aac")
    if cls == "cover-audio":
        return (rng.choice(["mp3", "flac", "m4a"]),
                rng.choice(["mp3+mjpeg", "flac+mjpeg", "aac+mjpeg"]))
    if cls == "multivideo-av":
        return rng.choice([
            ("avi", "mpeg4+mjpeg+pcm_s16le"),
            ("avi", "mjpeg+mpeg4+pcm_s16le"),
            ("mkv", "h264+vp9+aac"), ("mkv", "h264+h264+aac"),
        ])
    if cls == "drift-av":
        return (rng.choice(DRIFT_CONTAINERS),
                rng.choice(["h264+aac", "h264+mp3", "mpeg4+aac", "h264+opus"]))
    if cls == "meta-bomb-av":
        return (rng.choice(BOMB_CONTAINERS), rng.choice(["h264+aac", "mpeg4+mp3"]))
    # ---- v4 escalation classes (W5-16) ----
    if cls == "rot-display":
        return (rng.choice(["mp4", "mov", "mkv"]),
                rng.choice(["h264", "h264", "mpeg4", "mjpeg"]))
    if cls == "odd-dims":
        codec = rng.choices(["mpeg4", "mjpeg", "ffv1", "h264"],
                            weights=[30, 30, 20, 20], k=1)[0]
        cont = {"mpeg4": ["avi", "mp4", "mkv", "mov"],
                "mjpeg": ["avi", "mov", "mp4", "mkv"],
                "ffv1": ["avi", "mkv"],
                "h264": ["mp4", "mkv", "mov"]}[codec]
        return (rng.choice(cont), codec)
    if cls == "maxtracks-avi":
        return ("avi", "max")
    if cls == "bframe-storm":
        codec = rng.choice(["h264", "h264", "h264", "h265"])
        cont = rng.choice(["mp4", "ts", "mkv", "mov"]) \
            if codec == "h264" else rng.choice(["mp4", "ts", "mkv"])
        return (cont, codec)
    if cls == "packed-bs":
        mode = rng.choices(["xvid", "mpeg4bf", "avcc"], weights=[3, 3, 4],
                           k=1)[0]
        if mode == "xvid":
            return ("avi", "xvid")
        if mode == "mpeg4bf":
            return (rng.choice(["avi", "mp4"]), "mpeg4bf")
        return (rng.choice(["mp4", "mov"]), "avcc")
    if cls == "param-midchange":
        codec = rng.choice(["h264", "h264", "mpeg2video", "mpeg4"])
        return (rng.choice(["ts", "mp4", "mkv"]), codec)
    if cls == "vbv-burst":
        return (rng.choice(["mp4", "mkv", "ts"]),
                rng.choice(["h264", "mpeg4", "mpeg2video"]))
    if cls == "fps-extremes":
        return (rng.choice(["mp4", "mkv", "ts", "avi", "mov"]),
                rng.choice(["h264", "mpeg4", "mjpeg"]))
    if cls == "sr-chaos":
        codec = rng.choices(["pcm_s16le", "pcm_f32le", "pcm_s24le",
                             "flac", "alac"], weights=[4, 3, 2, 3, 2], k=1)[0]
        return (rng.choice(AUDIO_CODECS[codec]["cont"]), codec)
    if cls == "chapter-bomb":
        return (rng.choice(["mkv", "mp4"]),
                rng.choice(["h264+aac", "mpeg4+aac", "h264+mp3"]))
    if cls == "zeroframe-stream":
        return (rng.choice(["mkv", "ts"]), rng.choice(["h264+aac",
                                                          "mpeg4+aac"]))
    if cls == "still-24h":
        return (rng.choice(["mp4", "mkv", "mov"]),
                rng.choice(["h264", "mjpeg", "mpeg4"]))
    if cls == "riff-size-lie":
        return ("avi", rng.choice(["mjpeg+pcm_s16le", "mpeg4+pcm_u8",
                                    "mjpeg+pcm_u8"]))
    if cls == "odd-chunk-avi":
        return ("avi", rng.choice(["mjpeg+pcm_s16le", "mpeg4+pcm_u8",
                                    "mjpeg+pcm_u8"]))
    if cls == "hevc-in-avi":
        return ("avi", "h265")
    raise ValueError(cls)


def random_features(rng: random.Random, cls: str, family: tuple) -> dict:
    """Roll the composition dice.  Stratification may force overrides."""
    feats: dict = {}
    if cls in ("video", "avi-video", "mp4-video", "mkv-video", "mov-video",
               "ts-video", "flv-video", "webm-video", "ogv-video"):
        if rng.random() < 0.12:
            feats["sar"] = rng.choice(ANAMORPHIC_SAR)[0]
    if cls in ("av", "avi-av", "muxed-av"):
        if rng.random() < 0.10:
            feats["sar"] = rng.choice(ANAMORPHIC_SAR)[0]
        if rng.random() < 0.15:
            feats["av_mismatch"] = True
        if family[0] in ("mp4", "mov") and rng.random() < 0.07:
            feats["rotate"] = rng.choice(ROTATIONS)
    if cls == "vfr-av":
        feats["vfr"] = True
        if rng.random() < 0.4:
            feats["av_mismatch"] = True
    if cls == "multi-av":
        if rng.random() < 0.3:
            feats["subs"] = True
        if rng.random() < 0.25:
            feats["chapters"] = True
    return feats


def est_bytes(params: dict) -> int:
    """Deterministic plan-time size estimate (budget gate).  v4 builders
    with pathological geometry (maxtracks, rechunk blow-up, chapter bombs,
    ~24 h stills) carry a builder-computed `est_hint` that wins here."""
    if params.get("est_hint"):
        return int(params["est_hint"])
    w = params.get("width") or 64
    h = params.get("height") or 64
    dur = params.get("duration") or 2.0
    codec = params.get("codec") or ""
    if params.get("class") == "audio":
        rate = params.get("audio_rate") or 44100
        ch = params.get("audio_channels") or 2
        return int(rate * ch * 2 * dur * 0.8) + 4096
    factor = 1.0 if codec in ("ffv1", "huffyuv") else \
        0.7 if codec == "prores" else \
        0.25 if codec == "mjpeg" else 0.10
    v = int(w * h * dur * factor)
    if params.get("audio_codec"):
        rate = params.get("audio_rate") or 44100
        ch = params.get("audio_channels") or 2
        v += int(rate * ch * 2 * dur * 0.2)
    if params.get("class") == "hls-av":
        v *= max(1, len((params.get("features") or {}).get("tiers") or [1]))
    if params.get("class") == "drift-av":
        v = int(v * 1.0)
    return v + 8192


class Generator:
    def __init__(self, out: Path, count: int, budget_mb: int, jobs: int,
                 seed: int, limits: dict, validate: bool,
                 do_dirty: bool, do_garbage: bool, replay_seed: int | None,
                 replay_tool_version: int | None = None):
        self.out = out
        self.media = out / "media"
        self.count = count
        self.budget = budget_mb * 1024 * 1024
        self.jobs = max(1, jobs)
        self.rng = random.Random(seed)
        self.seed = seed
        self.limits = limits
        self.validate = validate
        self.do_dirty = do_dirty
        self.do_garbage = do_garbage
        self.replay_seed = replay_seed
        # v3 replay-compat: replaying a pre-v4 manifest restricts the
        # drawable universe to the v3 classes so the recorded seed
        # reproduces the exact corpus the v3 tool drew.
        self.replay_v3 = bool(replay_seed is not None
                              and (replay_tool_version or 4) < 4)
        self.encoders: set[str] = set()
        self.caps: dict = {}
        self.files: list[dict] = []
        self.rejected: list[dict] = []
        self.bytes_total = 0
        self.bytes_planned = 0
        self.t0 = time.time()
        self.per_class: dict[str, dict] = {}
        self.ctx = Ctx(limits, {}, RES_ORDER)

    # -- planning ----------------------------------------------------------
    def draw_plan(self) -> list[dict]:
        plan: list[tuple] = []
        pins = STRATIFICATION
        class_weights = CLASS_WEIGHTS
        if self.replay_v3:
            pins = [p for p in STRATIFICATION if p[0] not in V4_CLASSES]
            class_weights = [(c, w) for c, w in CLASS_WEIGHTS
                             if c not in V4_CLASSES]
        for pin in pins:
            pin = tuple(pin) + (None,) * (4 - len(pin))
            cls, family, forced_res, forced_feats = pin[:4]
            plan.append((cls, tuple(family), forced_res, forced_feats))
        for res in MANDATORY_RESOLUTIONS:
            plan.append(("avi-video", ("avi", "mjpeg"), res, None))
        classes = [c for c, _ in class_weights]
        weights = [w for _, w in class_weights]
        guard = 0
        while len(plan) < self.count and guard < self.count * 4:
            plan.append((self.rng.choices(classes, weights=weights, k=1)[0],
                         None, None, None))
            guard += 1
        self.rng.shuffle(plan)
        entries: list[dict] = []
        for idx, (cls, family, forced_res, forced_feats) in enumerate(plan):
            fam = family if family is not None else draw_family(self.rng, cls)
            builder = BUILDERS.get(KIND_FOR_CLASS.get(cls, cls))
            if builder is None:
                self.rejected.append({"idx": idx, "family": list(fam),
                                      "reason": f"no-builder:{cls}"})
                continue
            aux_seed = self.rng.getrandbits(64)
            erng = random.Random(aux_seed)
            feats = dict(forced_feats or {})
            if forced_feats is None:
                feats = random_features(self.rng, cls, fam)
            else:
                feats.update(random_features(self.rng, cls, fam))
            if cls == "hls-av":
                rel = f"hls/{idx:04d}/master.m3u8"
                ext = "m3u8"
            else:
                if cls in ("audio-only", "cover-audio", "sr-chaos"):
                    ext = AUDIO_CONTAINERS.get(fam[0], CONTAINERS.get(fam[0], {"ext": "bin"}))["ext"]
                elif cls == "raw-video":
                    ext = RAW_VIDEO_TARGETS[fam[1]]["ext"]
                else:
                    ext = CONTAINERS[fam[0]]["ext"]
                stem = f"{idx:04d}-{cls}"
                if cls in DIRTY_CLASSES:
                    # lies by construction — live with the derived dirty
                    # files, excluded from the clean ffprobe gate
                    rel = f"dirty/{stem}.{ext}"
                else:
                    rel = f"media/{stem}.{ext}"
            built = builder(erng, self.ctx, fam, forced_res, feats)
            if built is None:
                self.rejected.append({"idx": idx, "family": list(fam),
                                      "reason": "unsupported-draw"})
                continue
            if len(built) == 4:
                argvs, params, aux, post = built
            else:
                argvs, params, aux, post = *built, []
            est = est_bytes(params)
            if self.bytes_planned + est > self.budget:
                self.rejected.append({"idx": idx, "family": list(fam),
                                      "reason": "over-budget",
                                      "est_bytes": est})
                continue
            self.bytes_planned += est
            entries.append({"rel": rel, "cls": cls, "family": list(fam),
                            "argvs": argvs, "params": params,
                            "aux": [{"name": n, "b64": base64.b64encode(b).decode()}
                                    for n, b in (aux or [])],
                            "post": post or [],
                            "aux_seed": aux_seed, "est": est})
        return entries

    # -- execution ---------------------------------------------------------
    def execute_entry(self, entry: dict) -> dict | None:
        idx_dir = Path(entry["rel"]).parent
        tmp = self.out / "_tmp" / Path(entry["rel"]).stem
        tmp.mkdir(parents=True, exist_ok=True)
        target_dir = self.out / idx_dir
        target_dir.mkdir(parents=True, exist_ok=True)
        target = self.out / entry["rel"]
        # HLS aux files (master playlist) live WITH the segments; all other
        # aux (subs/chapters/concat lists) live in the per-entry tmp dir.
        aux_dir = target_dir if entry["cls"] == "hls-av" else tmp
        est = entry["est"]
        codec = entry["params"].get("codec") or ""
        # slow-codec realism: vp9/av1/theora/wavpack encodes need headroom,
        # especially with jobs>1 on a small core count (measured timeouts)
        slow = 3.0 if codec in ("vp9", "av1", "theora", "wavpack") else 1.0
        # expensive-to-RENDER lavfi sources (fractals, cellular automata)
        # spend most of their wall time generating pixels, not encoding
        src = str(entry["params"].get("source", ""))
        src_cost = 6.0 if "mandelbrot" in src else \
                   2.0 if ("cellauto" in src or "life" in src) else 1.0
        timeout = int(min(1500, (120 + est / 200_000 * 8 + 60) * slow * src_cost))
        # write aux files (deterministic content; __TMP__ markers resolved)
        for item in entry.get("aux") or []:
            if isinstance(item, dict):
                name, blob = item["name"], base64.b64decode(item["b64"])
            else:
                name, blob = item
            if b"__TMP__" in blob:
                blob = blob.replace(b"__TMP__", str(tmp).encode())
            (aux_dir / name).write_bytes(blob)
        rc_all, err, real_last = 0, "", None
        for argv in entry["argvs"]:
            real = []
            for tok in argv:
                tok = tok.replace("__OUT__", str(target))
                tok = tok.replace("__OUTDIR__", str(target_dir))
                tok = tok.replace("__TMP__", str(tmp))
                tok = tok.replace("__AUX__", str(aux_dir))
                real.append(tok)
            real_last = real
            rc_all, err = run([FFMPEG, "-hide_banner", "-loglevel", "error",
                               "-nostdin"] + real, timeout)
            if rc_all != 0:
                break
        if rc_all != 0:
            shutil.rmtree(tmp, ignore_errors=True)
            self.rejected.append({"idx": entry.get("idx", -1),
                                  "family": entry["family"], "reason": err,
                                  "params": entry["params"]})
            target.unlink(missing_ok=True)
            return None
        # v4 byte-level post-ops (rechunk / riff lies / tkhd rotation):
        # applied to the finished file BEFORE harvest + validation.  The
        # per-entry erng makes pad bytes deterministic (replay-safe).
        post_notes = []
        for spec in entry.get("post") or []:
            fn = POST_OPS.get(spec.get("op"))
            if fn is None:
                post_notes.append({"op": spec.get("op"), "applied": False,
                                   "reason": "unknown-op"})
                continue
            erng = random.Random(entry.get("aux_seed", 0))
            try:
                post_notes.append(fn(target, spec, erng))
            except (OSError, ValueError, struct.error) as e:
                post_notes.append({"op": spec.get("op"), "applied": False,
                                   "reason": f"{type(e).__name__}: {e}"})
        # harvest
        if entry["cls"] == "hls-av":
            segs = sorted(p for p in target_dir.iterdir() if p.is_file())
            size = sum(p.stat().st_size for p in segs)
            seg_list = [p.name for p in segs]
            sha = sha256_file(target)
            ffok = self._validate(target)
        else:
            size = target.stat().st_size
            seg_list = None
            sha = sha256_file(target)
            ffok = self._validate(target)
        shutil.rmtree(tmp, ignore_errors=True)
        rec = {
            "path": Path(entry["rel"]).name, "rel": entry["rel"],
            "class": entry["cls"], "family": entry["family"], "bytes": size,
            "sha256": sha, "params": entry["params"],
            "ffprobe_ok": ffok,
            "cmd": ["ffmpeg", "-hide_banner", "-loglevel", "error",
                     "-nostdin"] + real_last,
            "argvs": entry["argvs"],
        }
        if seg_list is not None:
            rec["segments"] = seg_list
        if post_notes:
            rec["post"] = post_notes
        return rec

    def _validate(self, path: Path):
        if not self.validate:
            return None
        rc, _ = run([FFPROBE, "-v", "error", "-show_entries",
                     "format=format_name", "-of", "csv=p=0", str(path)], 60)
        return rc == 0

    def run(self) -> None:
        self.media.mkdir(parents=True, exist_ok=True)
        tmp_root = self.out / "_tmp"
        tmp_root.mkdir(parents=True, exist_ok=True)
        plan_path = self.out / "plan.json"
        if plan_path.is_file():
            manifest_path = self.out / "manifest.json"
            if manifest_path.is_file():
                prior = json.loads(manifest_path.read_text())
                self.files = prior.get("files", [])
                self.bytes_total = prior.get("bytes_total", 0)
            plan = json.loads(plan_path.read_text())
            print(f"[gen] resuming plan: {len(plan)} entries, "
                  f"{len(self.files)} already done", flush=True)
        else:
            self.encoders = available_encoders(FFMPEG)
            t_caps = time.time()
            self.caps = probe_caps(FFMPEG, tmp_root)
            self.ctx.caps = self.caps
            print(f"[gen] capability probes ({time.time() - t_caps:.1f}s): "
                  + " ".join(f"{k}={'Y' if v else 'N'}"
                             for k, v in sorted(self.caps.items())), flush=True)
            plan = self.draw_plan()
            plan_path.write_text(json.dumps(plan))
        done = {f["rel"] for f in self.files}
        todo = [e for e in plan if e["rel"] not in done]
        n_done = 0
        t_exec = time.time()
        with ThreadPoolExecutor(max_workers=self.jobs) as ex:
            futs = {ex.submit(self.execute_entry, e): e for e in todo}
            for fut in as_completed(futs):
                rec = fut.result()
                n_done += 1
                if rec is not None:
                    self.files.append(rec)
                    self.bytes_total += rec["bytes"]
                    cls = rec["class"]
                    st = self.per_class.setdefault(cls, {"count": 0, "bytes": 0})
                    st["count"] += 1
                    st["bytes"] += rec["bytes"]
                if n_done % 12 == 0 or n_done == len(todo):
                    self.write_manifest()
                    print(f"[gen] {n_done}/{len(todo)} done, "
                          f"{self.bytes_total >> 20} MiB, "
                          f"{time.time() - t_exec:.0f}s", flush=True)
        print(f"[gen] clean phase done: {len(self.files)} files, "
              f"{len(self.rejected)} rejected draws", flush=True)
        if self.do_dirty:
            self.dirty_pass()
        if self.do_garbage:
            self.garbage_pass()
        shutil.rmtree(self.out / "_tmp", ignore_errors=True)
        self.write_manifest(final=True)
        bad_clean = [f["rel"] for f in self.files
                     if f.get("ffprobe_ok") is False
                     and not f["rel"].startswith(("dirty/", "garbage/", "hls/"))]
        print(f"[gen] TOTAL {len(self.files)} files, "
              f"{self.bytes_total >> 20} MiB, "
              f"wall {time.time() - self.t0:.0f}s, seed {self.seed}", flush=True)
        if bad_clean:
            print(f"[gen] WARNING: {len(bad_clean)} clean files failed "
                  f"ffprobe validation (recorded in manifest): "
                  f"{bad_clean[:5]}", flush=True)

    # -- dirty pass ----------------------------------------------------------
    def dirty_pass(self) -> None:
        dirty_dir = self.out / "dirty"
        dirty_dir.mkdir(parents=True, exist_ok=True)
        clean = [f for f in self.files if f["class"] not in
                 {"dirty", "dirty-lie-ext", "garbage"} | DIRTY_CLASSES]
        n_dirty = max(6, int(len(clean) * self.rng.uniform(0.14, 0.22)))
        picks = self.rng.sample(clean, min(n_dirty, len(clean)))
        for i, entry in enumerate(picks):
            src = self.out / entry["rel"]
            op = self.rng.choice(CORRUPTION_OPS)
            if op == "lie-ext":
                true_ext = Path(entry["path"]).suffix
                pool = [e for e in (".avi", ".mp4", ".mkv", ".wav", ".mp3",
                                    ".ts", ".flv", ".bin", ".m3u8", ".265")
                        if e != true_ext]
                fake = self.rng.choice(pool)
                dst = dirty_dir / f"lie-{i:03d}-{Path(entry['path']).stem}{fake}"
                shutil.copyfile(src, dst)
                params = dict(entry["params"])
                params["lies_about_extension"] = true_ext
                self.files.append({
                    "path": dst.name, "rel": f"dirty/{dst.name}",
                    "class": "dirty-lie-ext", "family": entry["family"],
                    "bytes": dst.stat().st_size, "sha256": sha256_file(dst),
                    "params": params, "corruption": {"op": op},
                    "derived_from": entry["path"],
                })
                continue
            data = bytearray(src.read_bytes())
            note: dict = {"op": op}
            if op == "truncate" and len(data) > 64:
                keep = int(len(data) * self.rng.uniform(0.15, 0.85))
                note["kept_frac"] = round(keep / len(data), 3)
                data = data[:keep]
            elif op == "zero-mid" and len(data) > 2048:
                n = self.rng.randint(512, 4096)
                off = self.rng.randint(64, max(65, len(data) - n - 1))
                data[off:off + n] = b"\x00" * n
                note.update(offset=off, zeroed=n)
            elif op == "flip-bytes" and len(data) > 16:
                for _ in range(self.rng.randint(8, 64)):
                    data[self.rng.randrange(len(data))] ^= 1 << self.rng.randrange(8)
            elif op == "smash-head":
                for pos in range(min(64, len(data))):
                    data[pos] = self.rng.randrange(256)
            elif op == "tail-garbage":
                garbage = bytes(self.rng.randrange(256)
                                for _ in range(self.rng.randint(4096, 65536)))
                note["appended"] = len(garbage)
                data += garbage
            elif op == "cut-mid-chunk" and len(data) > 256:
                keep = int(len(data) * self.rng.uniform(0.3, 0.7))
                data = data[:keep] + b"\x00" * 16
            elif op == "swap-halves" and len(data) > 128:
                mid = len(data) // 2
                data = data[mid:] + data[:mid]
            elif op == "dup-range" and len(data) > 512:
                a = self.rng.randint(0, len(data) - 256)
                n = self.rng.randint(256, min(16384, len(data) - a))
                data = data[:a + n] + data[a:a + n] + data[a + n:]
                note.update(offset=a, duped=n)
            elif op == "splice-other" and len(clean) > 1:
                other = self.rng.choice([f for f in clean if f is not entry])
                od = (self.out / other["rel"]).read_bytes()
                if len(od) > 64:
                    oa = self.rng.randint(0, len(od) - 32)
                    on = self.rng.randint(32, min(65536, len(od) - oa))
                    at = self.rng.randint(0, len(data))
                    data = data[:at] + od[oa:oa + on] + data[at:]
                    note.update(spliced_from=other["path"], at=at, n=on)
            elif op == "head-size-lie" and len(data) > 32:
                if data[:4] == b"RIFF":
                    lie = self.rng.choice([
                        len(data) + self.rng.randint(1, 1 << 20),
                        self.rng.randint(0, 4096),
                        0xFFFFFFFF])
                    data[4:8] = struct.pack("<I", lie & 0xFFFFFFFF)
                    note["target"] = "riff-size"
                elif data[4:8] == b"ftyp":
                    lie = self.rng.choice([8, 0x7FFFFFFF,
                                           self.rng.randint(0, 4096)])
                    data[0:4] = struct.pack(">I", lie & 0xFFFFFFFF)
                    note["target"] = "ftyp-size"
                else:
                    for _ in range(16):
                        data[self.rng.randrange(len(data))] ^= 1 << self.rng.randrange(8)
                    note["target"] = "fallback-flip"
            elif op == "bit-drift-region" and len(data) > 256:
                a = self.rng.randint(0, len(data) - 64)
                n = self.rng.randint(64, min(4096, len(data) - a))
                for k in range(a, a + n):
                    data[k] ^= 0x01
                note.update(offset=a, region=n)
            dst = dirty_dir / f"{op}-{i:03d}-{entry['path']}"
            dst.write_bytes(bytes(data))
            self.files.append({
                "path": dst.name, "rel": f"dirty/{dst.name}", "class": "dirty",
                "family": entry["family"], "bytes": dst.stat().st_size,
                "sha256": sha256_file(dst), "params": entry["params"],
                "corruption": note, "derived_from": entry["path"],
            })

    # -- garbage pass --------------------------------------------------------
    def garbage_pass(self) -> None:
        gdir = self.out / "garbage"
        gdir.mkdir(parents=True, exist_ok=True)
        for i, kind in enumerate(GARBAGE_ARCHETYPES):
            dst = gdir / f"{kind}-{i:02d}.bin"
            n = self.rng.randint(1, 65536)
            r = lambda k: bytes(self.rng.randrange(256) for _ in range(k))  # noqa: E731
            if kind == "empty":
                dst.write_bytes(b"")
            elif kind == "one-byte":
                dst.write_bytes(bytes([self.rng.randrange(256)]))
            elif kind == "riff-magic":
                dst.write_bytes(b"RIFF" + r(self.rng.randint(8, 200)))
            elif kind == "avi-magic-head":
                dst.write_bytes(b"RIFF\x24\x7e\x06\x00AVI " + r(self.rng.randint(16, 512)))
            elif kind == "ftyp-magic":
                dst.write_bytes(b"\x00\x00\x00\x18ftyp" + r(self.rng.randint(12, 400)))
            elif kind == "ebml-magic":
                dst.write_bytes(b"\x1a\x45\xdf\xa3" + r(self.rng.randint(12, 4096)))
            elif kind == "random-bytes":
                dst.write_bytes(r(n))
            elif kind == "text":
                words = ["lumen", "not", "a", "media", "file", "RIFF", "AVI ",
                         "garbage", "test"]
                dst.write_bytes(" ".join(self.rng.choice(words)
                                         for _ in range(400)).encode())
            elif kind == "all-ff":
                dst.write_bytes(b"\xff" * self.rng.randint(256, 32768))
            elif kind == "all-zero":
                dst.write_bytes(b"\x00" * self.rng.randint(256, 32768))
            elif kind == "id3-junk":
                dst.write_bytes(b"ID3" + r(self.rng.randint(64, 8192)))
            elif kind == "moov-only":
                m = r(self.rng.randint(64, 4096))
                dst.write_bytes(struct.pack(">I", 8 + len(m)) + b"moov" + m)
            elif kind == "mdat-only":
                m = r(self.rng.randint(64, 8192))
                dst.write_bytes(struct.pack(">I", 8 + len(m)) + b"mdat" + m)
            elif kind == "ftyp-only":
                brand = self.rng.choice([b"isom", b"mp42", b"avc1", b"dash"])
                body = brand + b"\x00\x00\x02\x00" + brand + brand
                dst.write_bytes(struct.pack(">I", 8 + len(body)) + b"ftyp" + body)
            elif kind == "id3-size-lie":
                # synchsafe-int size claiming huge payload, tiny actual file
                lie = 0x7F7F7F7F
                dst.write_bytes(b"ID3\x04\x00\x00" +
                                bytes([(lie >> 21) & 0x7F, (lie >> 14) & 0x7F,
                                       (lie >> 7) & 0x7F, lie & 0x7F]) +
                                r(self.rng.randint(16, 512)))
            elif kind == "m3u8-garbage":
                lines = ["#EXTM3U", "#EXT-X-VERSION:3",
                         "#EXTINF:2.0,", "nonexistent-segment-xyz.ts",
                         "#EXT-GARBAGE-HEADER", "\x00\x01\x02binary"]
                dst.write_bytes("\n".join(lines).encode())
            elif kind == "ebml-size-lie":
                dst.write_bytes(b"\x1a\x45\xdf\xa3" +
                                struct.pack(">B", 0xFF) + b"\x01" +
                                b"\xff" * 4 + r(self.rng.randint(32, 2048)))
            elif kind == "gif-magic":
                dst.write_bytes(b"GIF89a" + r(self.rng.randint(32, 2048)))
            elif kind == "png-magic":
                dst.write_bytes(b"\x89PNG\r\n\x1a\n" + r(self.rng.randint(32, 2048)))
            elif kind == "jpeg-magic":
                dst.write_bytes(b"\xff\xd8\xff\xe0" + r(self.rng.randint(32, 2048)))
            elif kind == "zip-magic":
                dst.write_bytes(b"PK\x03\x04" + r(self.rng.randint(32, 2048)))
            elif kind == "html-text":
                dst.write_bytes(("<html><body>not a media file</body></html>"
                                 * self.rng.randint(2, 20)).encode())
            self.files.append({
                "path": dst.name, "rel": f"garbage/{dst.name}",
                "class": "garbage", "family": [kind],
                "bytes": dst.stat().st_size, "sha256": sha256_file(dst),
                "params": {"kind": kind},
            })

    # -- manifest ------------------------------------------------------------
    def write_manifest(self, final: bool = False) -> None:
        manifest = {
            "generator": "tools/corpus/gen_corpus.py",
            "tool_version": 4,
            "schema_note": "v4 = v3 keys + per-file 'post' notes + v4 "
                           "classes/probes; v3 manifests replay unchanged",
            "seed": self.seed,
            "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
            "count": len(self.files),
            "bytes_total": self.bytes_total,
            "bytes_planned_est": self.bytes_planned,
            "wall_seconds": round(time.time() - self.t0, 1),
            "jobs": self.jobs,
            "capability_probes": self.caps,
            "rejected_draws": self.rejected,
            "files": self.files,
        }
        if final:
            manifest["per_class"] = self.per_class
        tmp = self.out / "manifest.json.tmp"
        tmp.write_text(json.dumps(manifest, indent=1))
        tmp.replace(self.out / "manifest.json")


def main() -> int:
    global FFMPEG, FFPROBE
    ap = argparse.ArgumentParser(
        description="Lumen randomized real-media corpus generator v4 "
                    "(W5-16 escalation: rot-display, odd-dims, maxtracks-avi, "
                    "bframe-storm, packed-bs, param-midchange, vbv-burst, "
                    "fps-extremes, sr-chaos, chapter-bomb, zeroframe-stream, "
                    "still-24h, riff-size-lie, odd-chunk-avi, hevc-in-avi; "
                    "29 capability probes; byte-level post-ops; manifest v3 "
                    "join keys unchanged — v3 manifests replay unchanged)",
        epilog="v3 classes (video/av/audio/multi/dynres/raw/fmp4/hls/cover/"
               "multivideo/drift/meta-bomb) are unchanged; v4 adds the 15 "
               "escalation classes above with randomized params inside "
               "measured-encoder-feasible bounds (D-11).")
    ap.add_argument("--out", default="corpus")
    ap.add_argument("--preset", choices=list(PRESETS), default=None,
                    help="count/budget/duration bundle (default: standard)")
    ap.add_argument("--count", type=int, default=None)
    ap.add_argument("--budget-mb", type=int, default=None)
    ap.add_argument("--jobs", type=int,
                    default=min(4, os.cpu_count() or 1))
    ap.add_argument("--seed", type=int, default=None)
    ap.add_argument("--replay", default=None,
                    help="manifest.json whose seed is re-run verbatim")
    ap.add_argument("--ffmpeg", default=FFMPEG)
    ap.add_argument("--ffprobe", default=FFPROBE)
    ap.add_argument("--no-dirty", action="store_true")
    ap.add_argument("--no-garbage", action="store_true")
    ap.add_argument("--no-validate", action="store_true")
    args = ap.parse_args()

    FFMPEG, FFPROBE = args.ffmpeg, args.ffprobe

    preset = PRESETS[args.preset or "standard"]
    count = args.count if args.count is not None else preset["count"]
    budget = args.budget_mb if args.budget_mb is not None else preset["budget_mb"]
    limits = {"max_res": preset["max_res"], "max_dur": preset["max_dur"],
              "heavy": preset["heavy"]}

    out = Path(args.out).resolve()
    seed = args.seed
    replay_seed = None
    replay_tool_version = None
    if args.replay:
        man = json.loads(Path(args.replay).read_text())
        seed = man.get("seed")
        replay_seed = seed
        replay_tool_version = man.get("tool_version") or 1
        if replay_tool_version < 4:
            print(f"[replay] v{replay_tool_version} manifest: drawable "
                  f"universe restricted to v3 classes (bit-exact replay)",
                  flush=True)
        print(f"[replay] re-running corpus seed {seed} verbatim", flush=True)
    if (out / "plan.json").is_file() and seed is None \
            and (out / "manifest.json").is_file():
        # Resume of a reaper-killed run: the seed comes from the partial
        # manifest so the identical random plan continues.
        seed = json.loads((out / "manifest.json").read_text()).get("seed")
        print(f"[gen] plan.json present in {out} — resume mode (seed {seed})",
              flush=True)
    if seed is None:
        seed = secrets.randbits(96)
    if not (out / "plan.json").is_file() \
            and out.exists() and any(out.iterdir()) \
            and not args.replay:
        print(f"refusing to write into non-empty {out} (pass a fresh --out)",
              file=sys.stderr)
        return 2
    out.mkdir(parents=True, exist_ok=True)

    gen = Generator(out, count, budget, args.jobs, seed, limits,
                    not args.no_validate, not args.no_dirty,
                    not args.no_garbage, replay_seed, replay_tool_version)
    gen.run()
    return 0


if __name__ == "__main__":
    sys.exit(main())
