#!/usr/bin/env python3
"""Lumen randomized real-media corpus generator (ED-FIX-5 charter; user
directive 2026-10-22: "all resolutions and types, video and audio, generated
from script, RANDOM not deterministic, too many samples, real numbers").

Design contract (binding):
  * RANDOM by default — the seed is drawn from OS entropy on every run, so
    two invocations never produce the same corpus. Deterministic replay is
    still possible: the manifest records the seed + the exact ffmpeg command
    per file, and `--replay <manifest.json>` re-executes them verbatim
    (reproducibility for defect triage without giving up run-to-run
    randomness).
  * COVERAGE-GUIDED: class draws are random-weighted, with a stratification
    pass that guarantees every codec-family x container-family in the core
    matrix appears at least once before random draws fill the rest.
  * DIRTY WORLD (D-3): ~18% of clean files get a corrupted sibling (never
    in place), plus standalone garbage archetypes. The originals stay clean
    so oracle comparisons remain meaningful.
  * HONEST LEDGER: encoder rejections (a random draw hitting a real encoder
    limit) are recorded in the manifest as `rejected` draws — they are data
    about the world, not failures of the generator.

Usage:
  gen_corpus.py [--out DIR] [--count N] [--budget-mb MB] [--seed S]
  gen_corpus.py --replay manifest.json [--out DIR]

Outputs: media files + manifest.json (params, commands, sha256, corruption
ops) under DIR (default: corpus/ next to the repo root).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import random
import secrets
import shutil
import subprocess
import sys
import time
from pathlib import Path

# ---------------------------------------------------------------------------
# Vocabulary tables — the "all types and resolutions" universe. Draws are
# random; these tables only bound what is drawable.
# ---------------------------------------------------------------------------

# Standard resolution ladder + PAL/NTSC anamorphic + DCI. Every entry has a
# name so the report can group by class.
RESOLUTION_LADDER = [
    ("tiny16", 16, 16), ("tiny32", 32, 32), ("tiny64", 64, 48),
    ("qqvga", 120, 90), ("qvga", 160, 120), ("qcif", 176, 144),
    ("wqvga", 240, 180), ("vga", 320, 240), ("cif", 352, 288),
    ("w360p", 480, 360), ("ntsc", 720, 480), ("pal", 720, 576),
    ("vga4x3", 640, 480), ("hvga", 640, 360), ("svga", 800, 600),
    ("w480p", 854, 480), ("qhdp", 960, 540), ("xga", 1024, 768),
    ("hd720", 1280, 720), ("sxga", 1280, 1024), ("hdp", 1600, 900),
    ("fhd", 1920, 1080), ("dci2k", 2048, 1080), ("qhd", 2560, 1440),
    ("uhd4k", 3840, 2160),
]
# Weight per resolution class: small screens dominate real-world corpora,
# big frames are expensive on 2 cores; 4K is rare but present.
RES_WEIGHTS = {
    "tiny16": 5, "tiny32": 5, "tiny64": 4, "qqvga": 3, "qvga": 4, "qcif": 4,
    "wqvga": 3, "vga": 5, "cif": 4, "w360p": 3, "ntsc": 3, "pal": 3,
    "vga4x3": 3, "hvga": 4, "svga": 2, "w480p": 3, "qhdp": 2, "xga": 2,
    "hd720": 4, "sxga": 1, "hdp": 1, "fhd": 2, "dci2k": 1, "qhd": 1,
    "uhd4k": 1,
}

FPS_POOL = [5, 8, 10, 12, 15, 23.976, 24, 25, 29.97, 30, 48, 50, 59.94, 60, 90, 120]
FPS_WEIGHTS = [1, 1, 1, 1, 2, 3, 4, 4, 3, 5, 1, 2, 2, 4, 1, 1]

VIDEO_SOURCES = [
    "testsrc2", "mandelbrot", "smptebars", "cellauto", "rgbtestsrc",
    "gradients", "life",
]
AUDIO_SOURCES = ["sine", "anoisesrc", "aevalsrc"]

# codec -> encoder name, allowed pix_fmts, even-dims requirement, speed args
VIDEO_CODECS = {
    "mjpeg":      {"enc": "mjpeg",       "pix": ["yuv420p", "yuvj420p", "gray"], "even": False},
    "mpeg4":      {"enc": "mpeg4",       "pix": ["yuv420p"],                    "even": True},
    "mpeg2video": {"enc": "mpeg2video",  "pix": ["yuv420p"],                    "even": True},
    "huffyuv":    {"enc": "huffyuv",     "pix": ["yuv422p", "rgb24"],           "even": True},
    "ffv1":       {"enc": "ffv1",        "pix": ["yuv420p", "yuv444p", "bgr0"], "even": True},
    "wmv1":       {"enc": "wmv1",        "pix": ["yuv420p"],                    "even": True},
    "wmv2":       {"enc": "wmv2",        "pix": ["yuv420p"],                    "even": True},
    "h264":       {"enc": "libx264",     "pix": ["yuv420p", "yuv444p", "gray"], "even": True, "preset": "ultrafast"},
    "h265":       {"enc": "libx265",     "pix": ["yuv420p"],                    "even": True, "preset": "ultrafast"},
    "vp8":        {"enc": "libvpx",      "pix": ["yuv420p"],                    "even": True},
    "vp9":        {"enc": "libvpx-vp9",  "pix": ["yuv420p"],                    "even": True, "deadline": "realtime"},
    "av1":        {"enc": "libsvtav1",   "pix": ["yuv420p"],                    "even": True, "preset": "8"},
    "theora":     {"enc": "libtheora",   "pix": ["yuv420p"],                    "even": True},
    "flv1":       {"enc": "flv",         "pix": ["yuv420p"],                    "even": True},
}

# container family -> (extension, ffmpeg -f muxer, codecs allowed)
CONTAINERS = {
    "avi":  {"ext": "avi",  "fmt": "avi",       "codecs": ["mjpeg", "mpeg4", "mpeg2video", "huffyuv", "ffv1", "wmv1", "wmv2"]},
    "mp4":  {"ext": "mp4",  "fmt": "mp4",       "codecs": ["h264", "h265", "mpeg4"]},
    "mov":  {"ext": "mov",  "fmt": "mov",       "codecs": ["h264", "mpeg4", "mjpeg"]},
    "mkv":  {"ext": "mkv",  "fmt": "matroska",  "codecs": ["h264", "h265", "vp8", "vp9", "av1", "ffv1", "mjpeg"]},
    "webm": {"ext": "webm", "fmt": "webm",      "codecs": ["vp8", "vp9", "av1"]},
    "ts":   {"ext": "ts",   "fmt": "mpegts",    "codecs": ["h264", "mpeg2video"]},
    "flv":  {"ext": "flv",  "fmt": "flv",       "codecs": ["flv1", "h264"]},
    "ogv":  {"ext": "ogv",  "fmt": "ogg",       "codecs": ["theora"]},
}

AUDIO_CODECS = {
    "pcm_s16le": {"enc": "pcm_s16le",  "cont": ["wav"],                "rates": "std",  "ch": "all"},
    "pcm_s24le": {"enc": "pcm_s24le",  "cont": ["wav"],                "rates": "std",  "ch": "all"},
    "pcm_u8":    {"enc": "pcm_u8",     "cont": ["wav"],                "rates": "std",  "ch": "all"},
    "pcm_f32le": {"enc": "pcm_f32le",  "cont": ["wav"],                "rates": "std",  "ch": "all"},
    "pcm_s16be": {"enc": "pcm_s16be",  "cont": ["wav"],                "rates": "std",  "ch": "all"},
    "flac":      {"enc": "flac",       "cont": ["flac", "wav", "mkv"], "rates": "std",  "ch": "all"},
    "mp3":       {"enc": "libmp3lame", "cont": ["mp3", "wav", "mkv"],  "rates": "mp3",  "ch": "two"},
    "aac":       {"enc": "aac",        "cont": ["m4a", "adts", "mp4"], "rates": "aac",  "ch": "six"},
    "opus":      {"enc": "libopus",    "cont": ["ogg", "opus", "webm"],"rates": "opus", "ch": "six"},
    "vorbis":    {"enc": "libvorbis",  "cont": ["ogg", "oga", "mkv"],  "rates": "std",  "ch": "two"},
    "ac3":       {"enc": "ac3",        "cont": ["ac3", "mkv"],         "rates": "std",  "ch": "six"},
    "wavpack":   {"enc": "libwavpack", "cont": ["wv"],                 "rates": "std",  "ch": "two"},
    "mp2":       {"enc": "libtwolame", "cont": ["mp2"],                "rates": "std",  "ch": "two"},
}

AUDIO_CONTAINERS = {
    "wav": {"ext": "wav", "fmt": "wav"},     "flac": {"ext": "flac", "fmt": "flac"},
    "m4a": {"ext": "m4a", "fmt": "ipod"},    "adts": {"ext": "aac", "fmt": "adts"},
    "ogg": {"ext": "ogg", "fmt": "ogg"},     "oga": {"ext": "oga", "fmt": "ogg"},
    "opus": {"ext": "opus", "fmt": "opus"},  "mp3": {"ext": "mp3", "fmt": "mp3"},
    "ac3": {"ext": "ac3", "fmt": "ac3"},     "wv": {"ext": "wv", "fmt": "wv"},
    "mp2": {"ext": "mp2", "fmt": "mp2"},
}

# Any container an audio-only draw may land in (AUDIO_CODECS cont lists
# may include mkv/webm/mp4 — the shared A/V containers).
ANY_CONTAINERS = {**AUDIO_CONTAINERS, **CONTAINERS}

STD_RATES = [8000, 11025, 16000, 22050, 32000, 44100, 48000, 88200, 96000]
ODD_RATES = [5512, 37800, 44056, 47250, 176400, 384000]  # dirty rates: some survive, some get rejected
MP3_RATES = [8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000]
AAC_RATES = [8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000, 88200, 96000]
OPUS_RATES = [48000, 24000, 16000, 12000, 8000]

# Corruption operators for the dirty pass (applied to a COPY; never in place).
CORRUPTION_OPS = ["truncate", "zero-mid", "flip-bytes", "smash-head", "tail-garbage", "cut-mid-chunk", "lie-ext"]

# Class weights for the random draw. avi-* is the today-testable surface
# (AVI demuxer + mjpeg/pcm decoders) so it dominates; the rest exercises
# probe honesty and builds the durable cross-phase asset.
CLASS_WEIGHTS = [
    ("avi-video", 30), ("avi-av", 14), ("mp4-video", 9), ("mkv-video", 8),
    ("mov-video", 3), ("ts-video", 4), ("flv-video", 3), ("webm-video", 4),
    ("ogv-video", 2), ("audio-only", 16), ("muxed-av", 7),
]

# Stratification matrix: (class, family key) pairs that MUST appear at
# least once per corpus. Random draws fill the rest of the count.
STRATIFICATION = [
    ("avi-video", ("avi", "mjpeg")), ("avi-video", ("avi", "mpeg4")),
    ("avi-video", ("avi", "mpeg2video")), ("avi-video", ("avi", "huffyuv")),
    ("avi-video", ("avi", "ffv1")), ("avi-video", ("avi", "wmv1")),
    ("avi-video", ("avi", "wmv2")),
    ("avi-av", ("avi", "mjpeg+pcm_s16le")), ("avi-av", ("avi", "mpeg4+pcm_u8")),
    ("avi-av", ("avi", "mjpeg+pcm_s24le")),
    ("mp4-video", ("mp4", "h264")), ("mp4-video", ("mp4", "h265")),
    ("mkv-video", ("mkv", "vp9")), ("mkv-video", ("mkv", "av1")),
    ("webm-video", ("webm", "vp8")),
    ("ts-video", ("ts", "h264")),
    ("flv-video", ("flv", "flv1")),
    ("ogv-video", ("ogv", "theora")),
    ("audio-only", ("wav", "pcm_s16le")), ("audio-only", ("wav", "pcm_s24le")),
    ("audio-only", ("wav", "pcm_u8")), ("audio-only", ("flac", "flac")),
    ("audio-only", ("ogg", "opus")), ("audio-only", ("ogg", "vorbis")),
    ("audio-only", ("mp3", "mp3")), ("audio-only", ("m4a", "aac")),
    ("audio-only", ("adts", "aac")), ("audio-only", ("ac3", "ac3")),
    ("muxed-av", ("mp4", "h264+aac")), ("muxed-av", ("mkv", "h264+mp3")),
]

# Resolutions that MUST appear at least once (for decode-throughput rows):
# the full ladder gets one guaranteed slot each, randomized placement.
MANDATORY_RESOLUTIONS = ["tiny16", "qcif", "vga", "ntsc", "hd720", "fhd", "uhd4k"]


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def available_encoders(ffmpeg: str) -> set[str]:
    """Query the real encoder set — draws never assume an encoder exists."""
    out = subprocess.run([ffmpeg, "-hide_banner", "-encoders"], capture_output=True, text=True, timeout=60)
    encs: set[str] = set()
    for line in out.stdout.splitlines():
        fields = line.split()
        # lines look like " V....D libx264 ..." — 6-char flags column whose
        # FIRST char is the media type (V/A/S)
        if len(fields) >= 2 and len(fields[0]) == 6 and fields[0][0] in "VAS":
            encs.add(fields[1])
    return encs


class Generator:
    def __init__(self, out: Path, count: int, budget_mb: int, ffmpeg: str, seed: int, replay: dict | None):
        self.out = out
        self.media = out / "media"
        self.count = count
        self.budget = budget_mb * 1024 * 1024
        self.ffmpeg = ffmpeg
        self.rng = random.Random(seed)
        self.seed = seed
        self.replay = replay
        self.encoders = available_encoders(ffmpeg) if replay is None else set()
        self.files: list[dict] = []
        self.rejected: list[dict] = []
        self.bytes_total = 0
        self.t0 = time.time()
        self.force_res: dict[int, str] = {}  # slot idx -> forced resolution name

    # -- draw helpers ------------------------------------------------------
    def draw_resolution(self, force_even: bool, max_pixels: int | None = None) -> tuple[str, int, int]:
        names = [n for n in RES_WEIGHTS
                 if max_pixels is None
                 or next(r for r in RESOLUTION_LADDER if r[0] == n)[1] * next(r for r in RESOLUTION_LADDER if r[0] == n)[2] <= max_pixels]
        if not names:  # cap below the smallest entry: use the smallest
            names = [RESOLUTION_LADDER[0][0]]
        name = self.rng.choices(names, weights=[RES_WEIGHTS[n] for n in names], k=1)[0]
        _, w, h = next(r for r in RESOLUTION_LADDER if r[0] == name)
        if not force_even and self.rng.random() < 0.18:
            # Dirty odd dimension: real-world files with odd edges (D-3).
            w2, h2 = w, h
            if self.rng.random() < 0.5 and w2 >= 40:
                w2 -= 1
            elif h2 >= 40:
                h2 -= 1
            return f"{name}odd", w2, h2
        if force_even:
            w += w % 2
            h += h % 2
        return name, w, h

    def draw_fps(self) -> float:
        return self.rng.choices(FPS_POOL, weights=FPS_WEIGHTS, k=1)[0]

    def draw_duration(self, w: int, h: int) -> float:
        pixels = w * h
        if pixels > 3_000_000:      # 4K: short
            return self.rng.uniform(1.0, 2.5)
        if pixels > 1_000_000:      # QHD/DCI
            return self.rng.uniform(1.0, 4.0)
        if pixels > 400_000:        # FHD-ish
            return self.rng.uniform(1.5, 6.0)
        return self.rng.uniform(1.0, 12.0)

    def draw_audio_params(self, codec: str) -> dict:
        spec = AUDIO_CODECS[codec]
        pools = {
            "std": STD_RATES + (ODD_RATES if self.rng.random() < 0.12 else []),
            "mp3": MP3_RATES, "aac": AAC_RATES, "opus": OPUS_RATES,
        }
        rate = self.rng.choice(pools[spec["rates"]])
        if spec["ch"] == "all":
            ch = self.rng.choice([1, 1, 2, 2, 2, 3, 4, 6, 8])
        elif spec["ch"] == "six":
            ch = self.rng.choice([1, 2, 2, 6])
        else:
            ch = self.rng.choice([1, 2, 2])
        return {"codec": codec, "rate": rate, "channels": ch}

    def audio_source_args(self, rate: int, ch: int, dur: float) -> list[str]:
        src = self.rng.choice(AUDIO_SOURCES)
        if src == "sine":
            if self.rng.random() < 0.3:  # pink noise variant
                return ["-f", "lavfi", "-i",
                        f"anoisesrc=color=pink:duration={dur:.2f}:sample_rate={rate}:amplitude=0.5",
                        "-ac", str(ch)]
            freq = self.rng.choice([55, 110, 220, 440, 880, 1000, 3000, 4400])
            return ["-f", "lavfi", "-i",
                    f"sine=frequency={freq}:duration={dur:.2f}:sample_rate={rate}",
                    "-ac", str(ch)]
        if src == "anoisesrc":
            color = self.rng.choice(["white", "pink", "brown", "blue", "violet"])
            return ["-f", "lavfi", "-i",
                    f"anoisesrc=color={color}:duration={dur:.2f}:sample_rate={rate}:amplitude=0.6",
                    "-ac", str(ch)]
        # aevalsrc — the "music-like" multi-tone class
        a = self.rng.uniform(80, 400)
        b = self.rng.uniform(400, 4000)
        c = self.rng.uniform(0.2, 2.0)
        expr = (f"0.4*sin(2*PI*{a:.1f}*t)+0.3*sin(2*PI*{b:.1f}*t)"
                f"+0.2*sin(2*PI*{a * b / 100:.1f}*t)*sin(2*PI*{c:.1f}*t)")
        return ["-f", "lavfi", "-i",
                f"aevalsrc={expr}:duration={dur:.2f}:sample_rate={rate}", "-ac", str(ch)]

    # -- command assembly ---------------------------------------------------
    def video_cmd(self, family: tuple, path: Path, forced_res: str | None) -> tuple[list[str], dict] | None:
        container, codec = family
        cdef = CONTAINERS[container]
        vdef = VIDEO_CODECS[codec]
        if forced_res is not None:
            _, w, h = next(r for r in RESOLUTION_LADDER if r[0] == forced_res)
            if vdef["even"]:
                w += w % 2
                h += h % 2
            rname = forced_res
        else:
            rname, w, h = self.draw_resolution(vdef["even"], max_pixels=400_000 if codec in ("huffyuv", "ffv1") else None)
        fps = self.draw_fps()
        dur = self.draw_duration(w, h)
        vsrc = self.rng.choice(VIDEO_SOURCES)
        pix = self.rng.choice(vdef["pix"])
        if vsrc in ("mandelbrot", "cellauto", "life"):
            args = ["-f", "lavfi", "-i", f"{vsrc}=size={w}x{h}:rate={fps}", "-t", f"{dur:.2f}"]
        else:
            args = ["-f", "lavfi", "-i", f"{vsrc}=duration={dur:.2f}:size={w}x{h}:rate={fps}"]
        args += ["-c:v", vdef["enc"], "-pix_fmt", pix]
        if "preset" in vdef:
            args += ["-preset", vdef["preset"]]
        if "deadline" in vdef:
            args += ["-deadline", vdef["deadline"]]
        if codec == "mjpeg":
            args += ["-q:v", str(self.rng.choice([2, 5, 8, 12, 20]))]
        elif codec in ("mpeg4", "mpeg2video", "wmv1", "wmv2", "flv1"):
            args += ["-b:v", str(self.rng.choice([200, 500, 1000, 2000, 4000])) + "k"]
        elif codec in ("h264", "h265", "vp8", "vp9", "av1", "theora"):
            args += ["-crf", str(self.rng.choice([18, 23, 28, 35]))]
        args += ["-f", cdef["fmt"], "-y", str(path)]
        params = {"class": "video", "container": container, "codec": codec, "res_name": rname,
                  "width": w, "height": h, "fps": fps, "pix_fmt": pix, "duration": round(dur, 2),
                  "source": vsrc}
        return args, params

    def av_cmd(self, family: tuple, path: Path, forced_res: str | None) -> tuple[list[str], dict] | None:
        container, combo = family
        vcodec, acodec = combo.split("+")
        cdef = CONTAINERS[container]
        vdef = VIDEO_CODECS[vcodec]
        if forced_res is not None:
            _, w, h = next(r for r in RESOLUTION_LADDER if r[0] == forced_res)
            if vdef["even"]:
                w += w % 2
                h += h % 2
            rname = forced_res
        else:
            rname, w, h = self.draw_resolution(vdef["even"])
        fps = self.draw_fps()
        dur = self.draw_duration(w, h)
        vsrc = self.rng.choice(VIDEO_SOURCES)
        ap = self.draw_audio_params(acodec)
        if acodec == "opus":
            ap["rate"] = self.rng.choice(OPUS_RATES)
        aargs = self.audio_source_args(ap["rate"], ap["channels"], dur)
        if vsrc in ("mandelbrot", "cellauto", "life"):
            args = ["-f", "lavfi", "-i", f"{vsrc}=size={w}x{h}:rate={fps}", "-t", f"{dur:.2f}"]
        else:
            args = ["-f", "lavfi", "-i", f"{vsrc}=duration={dur:.2f}:size={w}x{h}:rate={fps}"]
        args += aargs
        args += ["-c:v", vdef["enc"], "-pix_fmt", self.rng.choice(vdef["pix"])]
        if "preset" in vdef:
            args += ["-preset", vdef["preset"]]
        if vcodec == "mjpeg":
            args += ["-q:v", str(self.rng.choice([3, 6, 10]))]
        args += ["-c:a", AUDIO_CODECS[acodec]["enc"], "-ar", str(ap["rate"]), "-ac", str(ap["channels"])]
        args += ["-shortest", "-f", cdef["fmt"], "-y", str(path)]
        params = {"class": "av", "container": container, "codec": vcodec, "audio_codec": acodec,
                  "res_name": rname, "width": w, "height": h, "fps": fps, "duration": round(dur, 2),
                  "audio_rate": ap["rate"], "audio_channels": ap["channels"]}
        return args, params

    def audio_cmd(self, family: tuple, path: Path) -> tuple[list[str], dict] | None:
        container, codec = family
        cdef = ANY_CONTAINERS[container]
        ap = self.draw_audio_params(codec)
        if codec == "opus":
            ap["rate"] = self.rng.choice(OPUS_RATES)
        dur = self.rng.uniform(1.0, 14.0)
        args = self.audio_source_args(ap["rate"], ap["channels"], dur)
        args += ["-c:a", AUDIO_CODECS[codec]["enc"]]
        if codec == "mp3":
            args += ["-b:a", str(self.rng.choice([64, 96, 128, 192, 320])) + "k"]
        if codec == "aac":
            args += ["-b:a", str(self.rng.choice([48, 96, 128, 256])) + "k"]
        args += ["-f", cdef["fmt"], "-y", str(path)]
        params = {"class": "audio", "container": container, "codec": codec, "duration": round(dur, 2),
                  "audio_rate": ap["rate"], "audio_channels": ap["channels"]}
        return args, params

    # -- family draw per class ----------------------------------------------
    def draw_family(self, cls: str) -> tuple:
        if cls == "avi-video":
            codec = self.rng.choices(CONTAINERS["avi"]["codecs"],
                                     weights=[30, 18, 8, 6, 6, 4, 4], k=1)[0]
            return ("avi", codec)
        if cls == "avi-av":
            combo = self.rng.choice(["mjpeg+pcm_s16le", "mjpeg+pcm_u8", "mpeg4+pcm_s16le",
                                     "mpeg4+pcm_u8", "mjpeg+pcm_s24le", "mjpeg+pcm_f32le",
                                     "mpeg4+pcm_s24le"])
            return ("avi", combo)
        if cls in ("mp4-video", "mkv-video", "mov-video", "ts-video", "flv-video", "webm-video", "ogv-video"):
            container = cls.split("-")[0]
            codec = self.rng.choice(CONTAINERS[container]["codecs"])
            return (container, codec)
        if cls == "audio-only":
            codec = self.rng.choices(list(AUDIO_CODECS.keys()),
                                     weights=[18, 6, 5, 4, 3, 12, 10, 10, 8, 6, 4, 3, 3], k=1)[0]
            return (self.rng.choice(AUDIO_CODECS[codec]["cont"]), codec)
        # muxed-av
        return self.rng.choice([
            ("mp4", "h264+aac"), ("mp4", "h265+aac"), ("mkv", "h264+mp3"),
            ("mkv", "vp9+opus"), ("mkv", "h264+aac"), ("mp4", "mpeg4+aac"),
        ])

    # -- encode --------------------------------------------------------------
    def run_ffmpeg(self, args: list[str], timeout: int) -> tuple[bool, str]:
        try:
            p = subprocess.run([self.ffmpeg, "-hide_banner", "-loglevel", "error", "-nostdin"] + args,
                               capture_output=True, text=True, timeout=timeout)
        except subprocess.TimeoutExpired:
            return False, "timeout"
        if p.returncode != 0:
            tail = (p.stderr or "").strip().splitlines()
            return False, tail[-1][:200] if tail else f"rc={p.returncode}"
        return True, ""

    # -- dirty pass ------------------------------------------------------------
    def dirty_pass(self) -> None:
        dirty_dir = self.out / "dirty"
        dirty_dir.mkdir(parents=True, exist_ok=True)
        n_dirty = max(6, int(len(self.files) * self.rng.uniform(0.14, 0.22)))
        picks = self.rng.sample(list(self.files), min(n_dirty, len(self.files)))
        for i, entry in enumerate(picks):
            src = self.out / entry["rel"]
            op = self.rng.choice(CORRUPTION_OPS)
            if op == "lie-ext":
                true_ext = Path(entry["path"]).suffix
                pool = [e for e in (".avi", ".mp4", ".mkv", ".wav", ".mp3", ".ts", ".flv", ".bin")
                        if e != true_ext]
                fake_ext = self.rng.choice(pool)
                dst = dirty_dir / f"lie-{i:03d}-{Path(entry['path']).stem}{fake_ext}"
                shutil.copyfile(src, dst)
                params = dict(entry["params"])
                params["lies_about_extension"] = true_ext
                self.files.append({
                    "path": dst.name, "rel": f"dirty/{dst.name}", "class": "dirty-lie-ext",
                    "family": entry["family"], "bytes": dst.stat().st_size,
                    "sha256": sha256_file(dst), "params": params, "corruption": {"op": op},
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
                garbage = bytes(self.rng.randrange(256) for _ in range(self.rng.randint(4096, 65536)))
                note["appended"] = len(garbage)
                data += garbage
            elif op == "cut-mid-chunk" and len(data) > 256:
                keep = int(len(data) * self.rng.uniform(0.3, 0.7))
                data = data[:keep] + b"\x00" * 16
            dst = dirty_dir / f"{op}-{i:03d}-{entry['path']}"
            dst.write_bytes(bytes(data))
            self.files.append({
                "path": dst.name, "rel": f"dirty/{dst.name}", "class": "dirty",
                "family": entry["family"], "bytes": dst.stat().st_size,
                "sha256": sha256_file(dst), "params": entry["params"], "corruption": note,
                "derived_from": entry["path"],
            })

    # -- garbage pass ------------------------------------------------------------
    def garbage_pass(self) -> None:
        gdir = self.out / "garbage"
        gdir.mkdir(parents=True, exist_ok=True)
        archetypes = ["empty", "one-byte", "riff-magic", "avi-magic-head", "ftyp-magic",
                      "ebml-magic", "random-bytes", "text", "all-ff", "all-zero", "id3-junk"]
        for i, kind in enumerate(archetypes):
            dst = gdir / f"{kind}-{i:02d}.bin"
            n = self.rng.randint(1, 65536)
            if kind == "empty":
                dst.write_bytes(b"")
            elif kind == "one-byte":
                dst.write_bytes(bytes([self.rng.randrange(256)]))
            elif kind == "riff-magic":
                dst.write_bytes(b"RIFF" + bytes(self.rng.randrange(256) for _ in range(self.rng.randint(8, 200))))
            elif kind == "avi-magic-head":
                head = bytearray(b"RIFF\x24\x7e\x06\x00AVI ")
                head += bytes(self.rng.randrange(256) for _ in range(self.rng.randint(16, 512)))
                dst.write_bytes(bytes(head))
            elif kind == "ftyp-magic":
                head = bytearray(b"\x00\x00\x00\x18ftyp")
                head += bytes(self.rng.randrange(256) for _ in range(self.rng.randint(12, 400)))
                dst.write_bytes(bytes(head))
            elif kind == "ebml-magic":
                dst.write_bytes(b"\x1a\x45\xdf\xa3" + bytes(self.rng.randrange(256) for _ in range(self.rng.randint(12, 4096))))
            elif kind == "random-bytes":
                dst.write_bytes(bytes(self.rng.randrange(256) for _ in range(n)))
            elif kind == "text":
                words = ["lumen", "not", "a", "media", "file", "RIFF", "AVI ", "garbage", "test"]
                dst.write_bytes(" ".join(self.rng.choice(words) for _ in range(400)).encode())
            elif kind == "all-ff":
                dst.write_bytes(b"\xff" * self.rng.randint(256, 32768))
            elif kind == "all-zero":
                dst.write_bytes(b"\x00" * self.rng.randint(256, 32768))
            elif kind == "id3-junk":
                dst.write_bytes(b"ID3" + bytes(self.rng.randrange(256) for _ in range(self.rng.randint(64, 8192))))
            self.files.append({
                "path": dst.name, "rel": f"garbage/{dst.name}", "class": "garbage",
                "family": [kind], "bytes": dst.stat().st_size, "sha256": sha256_file(dst),
                "params": {"kind": kind},
            })

    # -- main -----------------------------------------------------------------
    def run(self) -> None:
        self.media.mkdir(parents=True, exist_ok=True)
        if self.replay is not None:
            self.replay_corpus()
            return
        plan_path = self.out / "plan.json"
        if plan_path.is_file():
            # Resume: the plan (drawn with this corpus's seed) survives
            # reaper kills; execution continues where it stopped. Re-load
            # the partial manifest so the done-set and byte budget are
            # honest.
            manifest_path = self.out / "manifest.json"
            if manifest_path.is_file():
                prior = json.loads(manifest_path.read_text())
                self.files = prior.get("files", [])
                self.bytes_total = prior.get("bytes_total", 0)
            plan = json.loads(plan_path.read_text())
            done = {f["rel"] for f in self.files}
            print(f"[gen] resuming plan: {len(plan)} entries, {len(done)} already done", flush=True)
        else:
            plan = self.draw_plan()
            plan_path.write_text(json.dumps(plan))
        produced = 0
        for entry in plan:
            rel = entry["rel"]
            if rel in {f["rel"] for f in self.files}:
                produced += 1
                continue
            if self.bytes_total > self.budget:
                print(f"[budget] stopping at {self.bytes_total >> 20} MiB", flush=True)
                break
            self.execute_entry(produced, entry)
            produced += 1
            self.write_manifest()
            if produced % 20 == 0:
                print(f"[gen] {produced}/{len(plan)} files, {self.bytes_total >> 20} MiB, "
                      f"{time.time() - self.t0:.0f}s", flush=True)
        print(f"[gen] clean phase done: {len(self.files)} files, {len(self.rejected)} rejected draws", flush=True)
        self.dirty_pass()
        self.garbage_pass()
        self.write_manifest()
        print(f"[gen] TOTAL {len(self.files)} files, {self.bytes_total >> 20} MiB, "
              f"wall {time.time() - self.t0:.0f}s, seed {self.seed}", flush=True)

    def draw_plan(self) -> list[dict]:
        """Draw the FULL random plan up front (no encoding here): the
        randomness of the run is frozen into plan.json, so a reaper kill
        mid-execution loses nothing and a resume continues the exact
        same random corpus."""
        classes = [c for c, _ in CLASS_WEIGHTS]
        weights = [w for _, w in CLASS_WEIGHTS]
        plan: list[tuple[str, tuple | None, str | None]] = [(cls, fam, None) for cls, fam in STRATIFICATION]
        for res in MANDATORY_RESOLUTIONS:
            plan.append(("avi-video", ("avi", "mjpeg"), res))
        guard = 0
        while len(plan) < self.count and guard < self.count * 3:
            plan.append((self.rng.choices(classes, weights=weights, k=1)[0], None, None))
            guard += 1
        self.rng.shuffle(plan)  # interleave: no class runs in a block
        entries: list[dict] = []
        for idx, (cls, family, forced_res) in enumerate(plan):
            fam = family if family is not None else self.draw_family(cls)
            codec = fam[1].split("+")[0]
            enc = AUDIO_CODECS[codec]["enc"] if codec in AUDIO_CODECS else VIDEO_CODECS.get(codec, {}).get("enc")
            if enc is not None and enc not in self.encoders:
                self.rejected.append({"idx": idx, "family": list(fam), "reason": "encoder-missing"})
                continue
            kind = "audio" if cls == "audio-only" else ("av" if cls in ("muxed-av", "avi-av") else "video")
            if kind == "audio":
                ext = ANY_CONTAINERS[fam[0]]["ext"]
            else:
                ext = CONTAINERS[fam[0]]["ext"]
            stem = f"{idx:04d}-{cls}"
            rel = f"media/{stem}.{ext}"
            path = self.media / f"{stem}.{ext}"
            if kind == "audio":
                built = self.audio_cmd(fam, path)
            elif kind == "av":
                built = self.av_cmd(fam, path, forced_res)
            else:
                built = self.video_cmd(fam, path, forced_res)
            if built is None:
                self.rejected.append({"idx": idx, "family": list(fam), "reason": "build-failed"})
                continue
            args, params = built
            entries.append({"rel": rel, "cls": cls, "family": list(fam), "args": args,
                            "params": params})
        return entries

    def execute_entry(self, idx: int, entry: dict) -> None:
        path = self.out / entry["rel"]
        ok, err = self.run_ffmpeg(entry["args"], timeout=420)
        if not ok:
            self.rejected.append({"idx": idx, "family": entry["family"], "reason": err,
                                  "params": entry["params"]})
            path.unlink(missing_ok=True)
            return
        size = path.stat().st_size
        self.bytes_total += size
        self.files.append({
            "path": path.name, "rel": entry["rel"], "class": entry["cls"],
            "family": entry["family"], "bytes": size, "sha256": sha256_file(path),
            "params": entry["params"],
            "cmd": ["ffmpeg", "-hide_banner", "-loglevel", "error", "-nostdin"] + entry["args"],
        })

    def replay_corpus(self) -> None:
        assert self.replay is not None
        self.media.mkdir(parents=True, exist_ok=True)
        for entry in self.replay["files"]:
            dst = self.out / entry["rel"]
            dst.parent.mkdir(parents=True, exist_ok=True)
            cmd = list(entry["cmd"])
            if "-y" not in cmd:
                cmd.append("-y")
            ok, err = self.run_ffmpeg(cmd[1:], timeout=420)
            if not ok:
                print(f"[replay] FAILED {entry['rel']}: {err}", flush=True)
                continue
            self.files.append(entry)
        (self.out / "manifest.json").write_text(json.dumps({
            "replayed_from": True, "seed": self.replay.get("seed"), "files": self.files,
        }, indent=1))
        print(f"[replay] {len(self.files)} entries processed", flush=True)

    def write_manifest(self) -> None:
        manifest = {
            "generator": "tools/corpus/gen_corpus.py",
            "seed": self.seed,
            "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
            "count": len(self.files),
            "bytes_total": self.bytes_total,
            "wall_seconds": round(time.time() - self.t0, 1),
            "rejected_draws": self.rejected,
            "files": self.files,
        }
        # Atomic-ish: write partial then rename, so a reaper kill between
        # files never leaves a torn manifest.
        tmp = self.out / "manifest.json.tmp"
        tmp.write_text(json.dumps(manifest, indent=1))
        tmp.replace(self.out / "manifest.json")


def main() -> int:
    ap = argparse.ArgumentParser(description="Lumen randomized real-media corpus generator")
    ap.add_argument("--out", default="corpus", help="output directory (default: corpus/)")
    ap.add_argument("--count", type=int, default=400, help="clean-file target count (default 400)")
    ap.add_argument("--budget-mb", type=int, default=1500, help="byte budget in MiB (default 1500)")
    ap.add_argument("--seed", type=int, default=None,
                    help="RNG seed (default: fresh OS entropy — non-deterministic runs)")
    ap.add_argument("--replay", default=None, help="manifest.json to replay verbatim")
    ap.add_argument("--ffmpeg", default=os.environ.get("FFMPEG", "ffmpeg"))
    args = ap.parse_args()

    out = Path(args.out).resolve()
    seed = args.seed if args.seed is not None else secrets.randbits(96)
    replay = None
    if args.replay:
        replay = json.loads(Path(args.replay).read_text())
        seed = replay.get("seed", seed)
        out.mkdir(parents=True, exist_ok=True)
    elif (out / "plan.json").is_file():
        # Resume of a reaper-killed run — allowed into the non-empty dir.
        print(f"[gen] plan.json present in {out} — resume mode", flush=True)
    else:
        if out.exists() and any(out.iterdir()):
            print(f"refusing to write into non-empty {out} (pass a fresh --out)", file=sys.stderr)
            return 2
        out.mkdir(parents=True, exist_ok=True)

    gen = Generator(out, args.count, args.budget_mb, args.ffmpeg, seed, replay)
    gen.run()
    return 0


if __name__ == "__main__":
    sys.exit(main())
