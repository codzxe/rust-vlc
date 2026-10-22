#!/usr/bin/env python3
"""Lumen randomized real-media corpus generator — v3, INDUSTRIAL GRADE.

ED-FIX-5 charter; user directives 2026-10-22 (random, real, many, measured)
and 2026-10-23 (industrial: parallel, VFR chaos, multi-track/subs/chapters,
dynamic resolution, anamorphic, crash minimization companion, HLS/fMP4,
fidelity scoring companion, FFmpeg speed parity companion, adversarial by
default, no hardcoded shortcuts).

Design contract (binding, extends v2):
  * RANDOM by default — seed from OS entropy; the full randomness is frozen
    into plan.json before any encoding starts, so a reaper kill mid-run
    loses nothing and `--seed S` (or replaying the recorded seed)
    reproduces the exact corpus.
  * PARALLEL — ffmpeg encodes run on a ThreadPool (subprocesses give true
    CPU parallelism); jobs default to the core count.  Manifest writes are
    serialized in the main thread; completion order cannot change content.
  * COVERAGE-GUIDED — the stratification matrix pins every key surface
    (codec x container, VFR, multi-track, subs, chapters, dyn-res, raw,
    fMP4, HLS, cover art, drift, bombs); random composition fills the rest.
  * CAPABILITY-DISCOVERED — 10-bit profiles, prores, rotation metadata,
    HLS-fMP4, concat-copy dynres are PROBED against the real ffmpeg at run
    start; unsupported draws land in the honest rejected-draws ledger.
  * DIRTY WORLD (D-3) — ~18% of clean files get a corrupted sibling (12
    byte-level ops incl. cross-file splices and header size lies) plus 23
    garbage archetypes.  Originals stay clean for oracle comparison.
  * HONEST LEDGER — encoder rejections, over-budget skips, capability
    gaps, and ffprobe failures of CLEAN files are all recorded as data.

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
    VIDEO_CODECS,
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
}


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
    """Deterministic plan-time size estimate (budget gate)."""
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
                 do_dirty: bool, do_garbage: bool, replay_seed: int | None):
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
        for pin in STRATIFICATION:
            pin = tuple(pin) + (None,) * (4 - len(pin))
            cls, family, forced_res, forced_feats = pin[:4]
            plan.append((cls, tuple(family), forced_res, forced_feats))
        for res in MANDATORY_RESOLUTIONS:
            plan.append(("avi-video", ("avi", "mjpeg"), res, None))
        classes = [c for c, _ in CLASS_WEIGHTS]
        weights = [w for _, w in CLASS_WEIGHTS]
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
                if cls in ("audio-only", "cover-audio"):
                    ext = AUDIO_CONTAINERS.get(fam[0], CONTAINERS.get(fam[0], {"ext": "bin"}))["ext"]
                elif cls == "raw-video":
                    ext = RAW_VIDEO_TARGETS[fam[1]]["ext"]
                else:
                    ext = CONTAINERS[fam[0]]["ext"]
                stem = f"{idx:04d}-{cls}"
                rel = f"media/{stem}.{ext}"
            built = builder(erng, self.ctx, fam, forced_res, feats)
            if built is None:
                self.rejected.append({"idx": idx, "family": list(fam),
                                      "reason": "unsupported-draw"})
                continue
            argvs, params, aux = built
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
                 ("dirty", "dirty-lie-ext", "garbage")]
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
            "tool_version": 3,
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
        description="Lumen randomized real-media corpus generator v3")
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
    if args.replay:
        man = json.loads(Path(args.replay).read_text())
        seed = man.get("seed")
        replay_seed = seed
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
                    not args.no_garbage, replay_seed)
    gen.run()
    return 0


if __name__ == "__main__":
    sys.exit(main())
