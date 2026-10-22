"""corpus_builders.py — ffmpeg command assembly for every corpus class.

Each builder returns (argvs, params, aux) where:
  argvs : list of ffmpeg argv LISTS (multi-pass classes run them in order;
          the LAST one produces the entry's target file).  Path markers:
          __OUT__ (target file), __OUTDIR__ (target dir for HLS),
          __TMP__ (per-entry temp dir), __AUX__ (per-entry aux dir),
          __AUXIDX__ (resolved to the aux input index at build time).
  params: manifest `params` dict (harness-compatible keys: width/height/
          fps/pix_fmt/duration/audio_rate/audio_channels/audio_codec/res_name)
  aux   : list of (relname, content-bytes) written before argvs run
          (subtitles, FFMETADATA chapters, concat lists, m3u8 masters).
          Content is a pure function of the entry's rng — replay-safe.

A builder returns None when a draw is unsupported (pix fmt without the
capability probe, etc.) — the generator records an honest rejected draw.

All randomness flows through `rng` (random.Random seeded per-entry) and
`caps` (probed ffmpeg capabilities).  Nothing is hardcoded that ffmpeg
itself can adjudicate.
"""

from __future__ import annotations

import random

from corpus_vocab import (
    ANAMORPHIC_SAR, AUDIO_CODECS, AUDIO_CONTAINERS, AUDIO_SOURCES,
    ANY_CONTAINERS, CONTAINERS, FPS_POOL, FPS_WEIGHTS, MAX_AUDIO_TRACKS,
    MAX_CHAPTERS, MAX_SUB_TRACKS, NO_DURATION_SRCS, RAW_VIDEO_TARGETS,
    RES_BY_NAME, RES_WEIGHTS, SUB_TEXT_POOL, SUBTITLE_CODEC_FOR,
    VIDEO_CODECS, VIDEO_SOURCES, VIDEO_SOURCE_WEIGHTS, draw_gop, vfr_expr,
)

STD_RATES_LIST = [8000, 11025, 16000, 22050, 32000, 44100, 48000, 88200, 96000]
ODD_RATES_LIST = [5512, 37800, 44056, 47250, 176400, 384000]

# pix_fmt -> capability probe name
PIX_CAP = {"yuv420p10le": "x264_10bit", "yuv422p10le": "prores",
           "yuv444p10le": "prores444", "yuv422p": "x264_422",
           "yuv444p": "x264_444"}


def _fmt_fps(f: float) -> str:
    return f"{f:g}"


def draw_resolution(rng, force_even, max_pixels=None, max_res=None, res_order=None):
    names = list(RES_WEIGHTS)
    if max_pixels is not None:
        names = [n for n in names if RES_BY_NAME[n][0] * RES_BY_NAME[n][1] <= max_pixels]
    if max_res is not None and res_order is not None:
        names = [n for n in names if res_order[n] <= res_order[max_res]]
    if not names:
        names = ["tiny16"]
    name = rng.choices(names, weights=[RES_WEIGHTS[n] for n in names], k=1)[0]
    w, h = RES_BY_NAME[name]
    if not force_even and rng.random() < 0.18:
        w2, h2 = w, h
        if rng.random() < 0.5 and w2 >= 40:
            w2 -= 1
        elif h2 >= 40:
            h2 -= 1
        return f"{name}odd", w2, h2
    if force_even:
        w += w % 2
        h += h % 2
    return name, w, h


def draw_fps(rng, lo=0.5, hi=240.0):
    pool = [f for f in FPS_POOL if lo <= f <= hi]
    w = [FPS_WEIGHTS[FPS_POOL.index(f)] for f in pool]
    return rng.choices(pool, weights=w, k=1)[0]


def draw_duration(rng, w, h, max_dur=14.0):
    pixels = w * h
    base = (1.0, 2.5) if pixels > 3_000_000 else \
           (1.0, 4.0) if pixels > 1_000_000 else \
           (1.5, 6.0) if pixels > 400_000 else (1.0, 12.0)
    return rng.uniform(max(0.3, base[0]), min(max_dur, base[1]))


def draw_audio_params(rng, codec, mp4_family=False):
    spec = AUDIO_CODECS[codec]
    if spec["rates"] == "std":
        pool = list(STD_RATES_LIST)
        if rng.random() < 0.10:
            pool = pool + ODD_RATES_LIST
        rate = rng.choice(pool)
    elif spec["rates"] == "mp3":
        pool = MP3_MP4_RATES if mp4_family else \
            [8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000]
        rate = rng.choice(pool)
    elif spec["rates"] == "aac":
        rate = rng.choice([8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000, 88200, 96000])
    else:  # opus
        rate = rng.choice([48000, 24000, 16000, 12000, 8000])
    # Encoder-measured rate floors (ffmpeg 7.1.5 native encoders):
    #   ac3: only 32000/44100/48000 ("Specified sample rate ... is not
    #        supported by the ac3 encoder");
    #   mp2 (libtwolame): MPEG-1/2 rates only.
    if codec == "ac3":
        rate = rng.choice([32000, 44100, 48000])
    elif codec == "mp2":
        rate = rng.choice([16000, 22050, 24000, 32000, 44100, 48000])
    if spec["ch"] == "all":
        ch = rng.choice([1, 1, 2, 2, 2, 3, 4, 6, 8])
    elif spec["ch"] == "six":
        ch = rng.choice([1, 2, 2, 6])
    else:
        ch = rng.choice([1, 2, 2])
    return {"codec": codec, "rate": rate, "channels": ch}


def audio_source_args(rng, rate, ch, dur, with_ac=True):
    """lavfi audio input argv fragment.  with_ac=False for multi-track
    builds: a bare `-ac` between inputs leaks into the next input's option
    group (measured: 'Multiple -ac options' warnings / broken headers) —
    channel shaping belongs on the per-output-stream -ac:a:i options."""
    src = rng.choice(AUDIO_SOURCES)
    d = f"{dur:.3f}"
    ac = ["-ac", str(ch)] if with_ac else []
    if src == "sine":
        if rng.random() < 0.3:
            return ["-f", "lavfi", "-i",
                    f"anoisesrc=color=pink:duration={d}:sample_rate={rate}:amplitude=0.5"] + ac
        freq = rng.choice([55, 110, 220, 440, 880, 1000, 3000, 4400])
        return ["-f", "lavfi", "-i",
                f"sine=frequency={freq}:duration={d}:sample_rate={rate}"] + ac
    if src == "anoisesrc":
        color = rng.choice(["white", "pink", "brown", "blue", "violet"])
        return ["-f", "lavfi", "-i",
                f"anoisesrc=color={color}:duration={d}:sample_rate={rate}:amplitude=0.6"] + ac
    a = rng.uniform(80, 400)
    b = rng.uniform(400, 4000)
    c = rng.uniform(0.2, 2.0)
    expr = (f"0.4*sin(2*PI*{a:.1f}*t)+0.3*sin(2*PI*{b:.1f}*t)"
            f"+0.2*sin(2*PI*{a * b / 100:.1f}*t)*sin(2*PI*{c:.1f}*t)")
    return ["-f", "lavfi", "-i",
            f"aevalsrc={expr}:duration={d}:sample_rate={rate}"] + ac


# mp3 in the mp4 family must use MPEG-1/2 sample rates — the mp4 muxer
# refuses MPEG-2.5 rates (8000/11025/12000) with "muxing mp3 at 8000hz is
# not standard" (measured).
MP3_MP4_RATES = [16000, 22050, 24000, 32000, 44100, 48000]


def video_source_args(rng, w, h, fps, dur):
    """Returns (input_argv, needs_output_t).  For infinite sources
    (mandelbrot/cellauto/life/testsrc) the -t MUST be an OUTPUT-side option:
    placed here it would land in the NEXT input's option group and leave the
    infinite source unbounded (measured: eternal encodes when -shortest is
    absent)."""
    src = rng.choices(VIDEO_SOURCES, weights=VIDEO_SOURCE_WEIGHTS, k=1)[0]
    if src in NO_DURATION_SRCS:
        return ["-f", "lavfi", "-i",
                f"{src}=size={w}x{h}:rate={_fmt_fps(fps)}"], True
    return ["-f", "lavfi", "-i",
            f"{src}=duration={dur:.3f}:size={w}x{h}:rate={_fmt_fps(fps)}"], False


def out_t(needs_t: bool, dur: float) -> list[str]:
    return ["-t", f"{dur:.3f}"] if needs_t else []


def pick_pix(rng, codec, caps, heavy_ok):
    """Draw a pix_fmt supported by BOTH the codec table and probed ffmpeg.
    Returns None when nothing in the pool is currently supported."""
    spec = VIDEO_CODECS[codec]
    pool = [p for p in spec["pix"] if heavy_ok or "10le" not in p]
    ok = [p for p in pool if PIX_CAP.get(p) is None or caps.get(PIX_CAP[p])]
    if not ok:
        return None
    return rng.choice(ok)


def venc_args(rng, codec, pix, fps, caps):
    spec = VIDEO_CODECS[codec]
    args = ["-c:v", spec["enc"], "-pix_fmt", pix]
    if "preset" in spec:
        args += ["-preset", spec["preset"]]
    if "deadline" in spec:
        args += ["-deadline", spec["deadline"]]
    feats = {}
    rc = rng.random()
    if codec == "mjpeg":
        args += ["-q:v", str(rng.choice([2, 5, 8, 12, 20]))]
    elif rc < 0.55 and codec in ("h264", "h265", "vp8", "vp9", "av1", "theora"):
        args += ["-crf", str(rng.choice([14, 18, 23, 28, 32, 35, 40]))]
    elif codec in ("mpeg4", "mpeg2video", "wmv1", "wmv2", "flv1"):
        args += ["-b:v", str(rng.choice([30, 120, 250, 500, 1000, 2000, 4000])) + "k"]
    elif codec in ("h264", "h265"):
        args += ["-b:v", str(rng.choice([80, 250, 800, 2500, 6000])) + "k"]
    if codec in ("h264", "h265", "mpeg4", "mpeg2video", "vp8", "vp9"):
        gop = draw_gop(rng, codec, fps)
        feats["gop"] = gop
        args += ["-g", str(gop["g"])]
        if "bf" in gop:
            args += ["-bf", str(gop["bf"])]
        xparams = []
        if gop.get("bpyramid"):
            xparams.append("b-pyramid=1")
        if gop.get("opengop"):
            xparams.append("open-gop=1" if codec == "h265" else "open_gop=1")
        if xparams and codec in ("h264", "h265"):
            key = "x264-params" if codec == "h264" else "x265-params"
            args += [f"-{key}", ":".join(xparams)]
    if codec == "prores":
        args += ["-profile:v", str(rng.choice([0, 1, 2, 3]))]
    return args, feats


def aenc_args(rng, codec, stream=None):
    """Audio encoder argv fragment; stream=None -> plain -c:a, else -c:a:i."""
    sfx = "" if stream is None else f":{stream}"
    args = [f"-c:a{sfx}", AUDIO_CODECS[codec]["enc"]]
    if codec == "mp3":
        args += [f"-b:a{sfx}", str(rng.choice([64, 96, 128, 192, 320])) + "k"]
    elif codec == "aac":
        args += [f"-b:a{sfx}", str(rng.choice([48, 96, 128, 256])) + "k"]
    return args


def sar_filter_for(wanted):
    for name, n, d in ANAMORPHIC_SAR:
        if name == wanted:
            return name, f"setsar={n}/{d}"
    name, n, d = ANAMORPHIC_SAR[0]
    return name, f"setsar={n}/{d}"


# ---------------------------------------------------------------------------
# Aux writers — deterministic per rng, adversarial by construction.
# ---------------------------------------------------------------------------

def _ts_hmsms(sec: float) -> str:
    ms = int(round(sec * 1000))
    h, ms = divmod(ms, 3600000)
    m, ms = divmod(ms, 60000)
    s, ms = divmod(ms, 1000)
    return f"{h:02d}:{m:02d}:{s:02d},{ms:03d}"


def write_srt(rng, dur: float) -> bytes:
    """Adversarial SRT: BOM, CRLF, out-of-order indices, zero-duration cues,
    beyond-duration cues, '-->' inside text, unicode chaos."""
    lines = ["\ufeff"]
    n = rng.randint(2, 8)
    t = 0.0
    for i in range(1, n + 1):
        start = t + rng.uniform(-0.3, 0.6)
        kind = rng.random()
        if kind < 0.12:
            end = start
        elif kind < 0.24:
            end = start + rng.uniform(0.2, 3.0) + dur
        else:
            end = start + rng.uniform(0.4, 3.5)
        idx = i if rng.random() > 0.2 else rng.randint(1, 99)
        lines.append(str(idx))
        lines.append(f"{_ts_hmsms(max(0, start))} --> {_ts_hmsms(max(0, end))}")
        lines.append(rng.choice(SUB_TEXT_POOL))
        lines.append("")
        t += rng.uniform(0.2, 1.6)
    return "\r\n".join(lines).encode("utf-8")


def write_vtt(rng, dur: float) -> bytes:
    lines = ["WEBVTT", ""]
    t = 0.0
    for _ in range(rng.randint(2, 6)):
        start, end = t, t + rng.uniform(0.5, 3.0)
        lines.append(f"{_ts_hmsms(start).replace(',', '.')} --> "
                     f"{_ts_hmsms(end).replace(',', '.')}")
        lines.append(rng.choice(SUB_TEXT_POOL))
        lines.append("")
        t += rng.uniform(0.3, 1.5)
    return "\n".join(lines).encode("utf-8")


def write_ass(rng, w: int, h: int, dur: float) -> bytes:
    font = rng.choice(["Arial", "DejaVu Sans", "Noto Sans SC",
                       "Liberation Serif", "Comic Sans"])
    head = (
        "[Script Info]\nScriptType: v4.00+\n"
        f"PlayResX: {max(w, 16)}\nPlayResY: {max(h, 16)}\nWrapStyle: 2\n\n"
        "[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, "
        "SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, "
        "StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, "
        "Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n"
        f"Style: Default,{font},{rng.randint(12, 42)},&H00FFFFFF,&H000000FF,"
        "&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,2,1,2,10,10,10,1\n"
        f"Style: Big,{font},{rng.randint(36, 72)},&H0000FFFF,&H000000FF,"
        "&H00000000,&H80000000,-1,-1,0,0,{rng.randint(110, 160)},100,0,0,1,"
        f"{rng.randint(1, 4)},1,{rng.randint(1, 9)},10,10,10,1\n\n"
        "[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, "
        "MarginV, Effect, Text\n")
    events = []
    t = 0.0
    for _ in range(rng.randint(3, 10)):
        start = t
        end = start + rng.uniform(0.4, 4.0)
        style = rng.choice(["Default", "Big"])
        text = rng.choice(SUB_TEXT_POOL)
        if rng.random() < 0.25:
            text = (f"{{\\pos({rng.randint(10, max(11, w - 10))},"
                    f"{rng.randint(10, max(11, h - 10))})}}{text}")
        if rng.random() < 0.15:
            text = "{{\\k" + str(rng.randint(10, 90)) + "}}" + text
        events.append(f"Dialogue: 0,{_ts_hmsms(start).replace(',', '.')},"
                      f"{_ts_hmsms(end).replace(',', '.')},{style},Lumen,0,0,0,,{text}")
        t += rng.uniform(0.2, 1.8)
    return (head + "\n".join(events) + "\n").encode("utf-8")


def write_ffmetadata(rng, dur: float) -> bytes:
    n = rng.randint(3, MAX_CHAPTERS)
    cuts = sorted(rng.uniform(0, dur) for _ in range(n - 1))
    bounds = [0.0] + cuts + [dur]
    out = [";FFMETADATA1", f"title=lumen-chapters-{rng.randint(1000, 9999)}"]
    for i in range(n):
        s, e = bounds[i], bounds[i + 1]
        out += ["[CHAPTER]", "TIMEBASE=1/1000",
                f"START={int(s * 1000)}", f"END={int(max(e, s + 0.001) * 1000)}",
                f"title=Chapter {i + 1}"]
    return ("\n".join(out) + "\n").encode("utf-8")


def subtitle_aux(rng, form, dur, w, h):
    if form == "webvtt":
        return "sub.vtt", write_vtt(rng, dur), "webvtt"
    if form in ("ass", "ssa"):
        return f"sub.{form}", write_ass(rng, w, h, dur), form
    return "sub.srt", write_srt(rng, dur), "srt"


# ---------------------------------------------------------------------------
# Builders
# ---------------------------------------------------------------------------

class Ctx:
    def __init__(self, limits, caps, res_order):
        self.limits = limits
        self.caps = caps
        self.res_order = res_order


def _res_cap_for(codec):
    return 400_000 if codec in ("huffyuv", "ffv1", "prores") else None


def _resolve_res(rng, vdef, forced_res, ctx, cap=None):
    if forced_res:
        w, h = RES_BY_NAME[forced_res]
        if vdef["even"]:
            w += w % 2
            h += h % 2
        return forced_res, w, h
    return draw_resolution(rng, vdef["even"], max_pixels=cap,
                           max_res=ctx.limits.get("max_res"),
                           res_order=ctx.res_order)


def _ensure_frames(fps: float, dur: float, max_dur: float) -> tuple[float, float]:
    """Guarantee >= ~3 encoded frames: at 0.5 fps a 0.4 s draw yields a
    zero-frame file (ffmpeg 'Nothing was written').  Bump dur first, then
    fps, staying within the preset's max_dur."""
    if dur * fps < 3.0:
        dur = min(max_dur, 3.0 / fps)
    if dur * fps < 2.0:
        fps = max(fps, 2.0 / dur)
    return fps, dur


def _theora_min(w: int, h: int) -> tuple[int, int]:
    # libtheora writes broken streams below 32x32 (measured: rc=0 but the
    # decode side rejects the picture region) — bump to the safe floor.
    if w < 32 or h < 32:
        w, h = max(w, 32), max(h, 32)
        w += w % 2
        h += h % 2
    return w, h


def build_video(rng, ctx, family, forced_res=None, forced_feats=None):
    container, codec = family
    cdef = CONTAINERS[container]
    vdef = VIDEO_CODECS[codec]
    rname, w, h = _resolve_res(rng, vdef, forced_res, ctx, _res_cap_for(codec))
    if codec == "theora":
        w, h = _theora_min(w, h)
    fps = draw_fps(rng)
    dur = draw_duration(rng, w, h, ctx.limits.get("max_dur", 14.0))
    fps, dur = _ensure_frames(fps, dur, ctx.limits.get("max_dur", 14.0))
    pix = pick_pix(rng, codec, ctx.caps, ctx.limits.get("heavy", True))
    if pix is None:
        return None
    feats = dict(forced_feats or {})
    vin, needs_t = video_source_args(rng, w, h, fps, dur)
    argv = vin
    vf = []
    if feats.get("sar"):
        sar_name, filt = sar_filter_for(feats["sar"])
        vf.append(filt)
        feats["sar"] = sar_name
    if vf:
        argv += ["-vf", ",".join(vf)]
    enc, gopf = venc_args(rng, codec, pix, fps, ctx.caps)
    feats.update(gopf)
    argv += enc
    if feats.get("rotate") is not None:
        if container in ("mp4", "mov") and ctx.caps.get("rotate_meta"):
            argv += ["-metadata:s:v:0", f"rotate={feats['rotate']}"]
        else:
            feats["rotate_unsupported"] = feats.pop("rotate")  # honest ledger
    argv += out_t(needs_t, dur) + ["-f", cdef["fmt"], "-y", "__OUT__"]
    params = {"class": "video", "container": container, "codec": codec,
              "res_name": rname, "width": w, "height": h, "fps": fps,
              "pix_fmt": pix, "duration": round(dur, 3), "features": feats,
              "source": argv[3] if len(argv) > 3 else "?"}
    return [argv], params, []


def _av_common(rng, ctx, container, vcodec, acodec, forced_res, forced_feats,
               dur_override=None, pixel_cap=None):
    vdef = VIDEO_CODECS[vcodec]
    cap = pixel_cap if pixel_cap is not None else _res_cap_for(vcodec)
    if (forced_feats or {}).get("small"):
        cap = min(cap or 10 ** 9, 100_000)
    rname, w, h = _resolve_res(rng, vdef, forced_res, ctx, cap)
    fps = draw_fps(rng)
    dur = dur_override or draw_duration(rng, w, h, ctx.limits.get("max_dur", 14.0))
    fps, dur = _ensure_frames(fps, dur, ctx.limits.get("max_dur", 14.0))
    pix = pick_pix(rng, vcodec, ctx.caps, ctx.limits.get("heavy", True))
    if pix is None:
        return None
    feats = dict(forced_feats or {})

    vin, needs_t = video_source_args(rng, w, h, fps, dur)
    argv = vin
    ap = draw_audio_params(rng, acodec,
                           mp4_family=(container in ("mp4", "m4a")))
    if acodec == "opus":
        ap["rate"] = rng.choice([48000, 24000, 16000, 12000, 8000])
    a_dur = dur
    if feats.get("av_mismatch"):
        a_dur = dur * rng.uniform(1.02, 1.15) if rng.random() < 0.5 \
            else dur * rng.uniform(0.85, 0.98)
    argv += audio_source_args(rng, ap["rate"], ap["channels"], a_dur)

    vf = []
    if feats.get("sar"):
        sar_name, filt = sar_filter_for(feats["sar"])
        vf.append(filt)
        feats["sar"] = sar_name
    if feats.get("vfr"):
        # settb FIRST: the lavfi source timebase (1/fps) quantizes setpts
        # output to whole frame ticks — sub-frame jitter would be truncated
        # away (measured: sinus-jitter came out perfectly CFR without this).
        vf.append("settb=1/90000")
        name, expr = vfr_expr(rng, fps)
        feats["vfr_mode"] = name
        if name.endswith("|select"):
            vf.append(f"select='{expr}'")
        else:
            vf.append(f"setpts='{expr}'")
    if vf:
        argv += ["-vf", ",".join(vf)]
    enc, gopf = venc_args(rng, vcodec, pix, fps, ctx.caps)
    feats.update(gopf)
    argv += enc
    if feats.get("vfr"):
        argv += ["-fps_mode:v", "vfr"]
    argv += aenc_args(rng, acodec)
    argv += ["-ar", str(ap["rate"]), "-ac", str(ap["channels"])]
    if not feats.get("av_mismatch"):
        argv += ["-shortest"]
    if feats.get("rotate") is not None:
        if container in ("mp4", "mov") and ctx.caps.get("rotate_meta"):
            argv += ["-metadata:s:v:0", f"rotate={feats['rotate']}"]
        else:
            feats["rotate_unsupported"] = feats.pop("rotate")  # honest ledger
    params = {"res_name": rname, "width": w, "height": h, "fps": fps,
              "pix_fmt": pix, "duration": round(dur, 3),
              "audio_rate": ap["rate"], "audio_channels": ap["channels"],
              "audio_codec": acodec,
              "source": argv[3] if len(argv) > 3 else "?"}
    return argv, params, feats, needs_t


def build_av(rng, ctx, family, forced_res=None, forced_feats=None,
             dur_override=None):
    container, combo = family
    vcodec, acodec = combo.split("+")
    built = _av_common(rng, ctx, container, vcodec, acodec, forced_res,
                       forced_feats, dur_override)
    if built is None:
        return None
    argv, params, feats, needs_t = built
    argv += out_t(needs_t, params["duration"]) + \
        ["-f", CONTAINERS[container]["fmt"], "-y", "__OUT__"]
    params.update({"class": "av", "container": container, "codec": vcodec,
                   "features": feats})
    return [argv], params, []


def build_audio(rng, ctx, family, forced_res=None, forced_feats=None):
    container, codec = family
    cdef = ANY_CONTAINERS[container]
    ap = draw_audio_params(rng, codec)
    if codec == "opus":
        ap["rate"] = rng.choice([48000, 24000, 16000, 12000, 8000])
    dur = min(rng.uniform(1.0, 14.0), ctx.limits.get("max_dur", 14.0))
    argv = audio_source_args(rng, ap["rate"], ap["channels"], dur)
    argv += aenc_args(rng, codec)
    argv += ["-f", cdef["fmt"], "-y", "__OUT__"]
    params = {"class": "audio", "container": container, "codec": codec,
              "duration": round(dur, 3), "audio_rate": ap["rate"],
              "audio_channels": ap["channels"],
              "features": {"odd_rate": ap["rate"] not in STD_RATES_LIST}}
    return [argv], params, []


def build_multi_av(rng, ctx, family, forced_res=None, forced_feats=None):
    """Multi-audio (+ optional subs/chapters).  Input layout:
    0=video, 1..N=audio tracks, then optional sub inputs, then FFMETADATA."""
    container, combo = family
    cdef = CONTAINERS[container]
    tokens = combo.split("+")
    vcodec = tokens[0]
    acodecs = [t for t in tokens[1:] if t in AUDIO_CODECS]
    want_subs = "subs" in tokens or (forced_feats or {}).get("subs")
    want_chap = "chapters" in tokens or (forced_feats or {}).get("chapters")
    if not acodecs:
        acodecs = [rng.choice(["aac", "mp3", "ac3"])]
    n_audio = rng.randint(2, MAX_AUDIO_TRACKS)
    pool = [c for c in ("aac", "mp3", "ac3", "opus", "flac", "pcm_s16le",
                        "vorbis") if c in AUDIO_CODECS]
    while len(acodecs) < n_audio:
        extra = rng.choice([c for c in pool if c not in acodecs] or ["aac"])
        acodecs.append(extra)

    vdef = VIDEO_CODECS[vcodec]
    _cap = _res_cap_for(vcodec)
    cap = min(_cap, 400_000) if _cap is not None else 400_000
    rname, w, h = _resolve_res(rng, vdef, None, ctx, cap)
    fps = draw_fps(rng)
    dur = draw_duration(rng, w, h, ctx.limits.get("max_dur", 14.0))
    fps, dur = _ensure_frames(fps, dur, ctx.limits.get("max_dur", 14.0))
    pix = pick_pix(rng, vcodec, ctx.caps, ctx.limits.get("heavy", True))
    if pix is None:
        return None

    # ---- INPUT SECTION (all -i before any output option) ----
    vin, needs_t = video_source_args(rng, w, h, fps, dur)
    argv = vin
    aparams = []
    for ac in acodecs:
        ap = draw_audio_params(rng, ac,
                               mp4_family=(container in ("mp4", "m4a")))
        if ac == "opus":
            ap["rate"] = rng.choice([48000, 24000, 16000, 12000, 8000])
        argv += audio_source_args(rng, ap["rate"], ap["channels"], dur,
                                  with_ac=False)
        aparams.append(ap)
    aux = []
    feats = {"multi_audio": acodecs, **(forced_feats or {})}
    next_input = 1 + len(acodecs)
    sub_inputs = []
    if want_subs:
        forms = [f for f in SUBTITLE_CODEC_FOR.get(container, [])
                 if f not in ("dvbsub",)]
        if forms:
            n_subs = rng.randint(1, MAX_SUB_TRACKS)
            for s in range(n_subs):
                form = rng.choice(forms)
                fname, blob, csub = subtitle_aux(rng, form, dur, w, h)
                aux.append((fname, blob))
                argv += ["-i", f"__AUX__/{fname}"]
                sub_inputs.append((next_input, csub))
                next_input += 1
            feats["subs"] = [c for _, c in sub_inputs]
    meta_input = None
    if want_chap:
        aux.append(("chapters.txt", write_ffmetadata(rng, dur)))
        argv += ["-i", "__AUX__/chapters.txt"]
        meta_input = next_input
        next_input += 1
        feats["chapters"] = True

    # ---- OUTPUT SECTION ----
    enc, gopf = venc_args(rng, vcodec, pix, fps, ctx.caps)
    feats.update(gopf)
    argv += enc
    argv += ["-map", "0:v"]
    for i in range(len(acodecs)):
        argv += ["-map", f"{1 + i}:a"]
    for i, (ac, ap) in enumerate(zip(acodecs, aparams)):
        argv += aenc_args(rng, ac, stream=i)
        argv += [f"-ar:a:{i}", str(ap["rate"]), f"-ac:a:{i}", str(ap["channels"])]
        argv += [f"-metadata:s:a:{i}",
                 f"language={rng.choice(['eng', 'spa', 'ger', 'jpn', 'und', 'zxx'])}"]
    argv += ["-disposition:a:0", "default"]
    for i in range(1, len(acodecs)):
        argv += [f"-disposition:a:{i}", "none"]
    for idx, (inp, _) in enumerate(sub_inputs):
        argv += ["-map", f"{inp}:s"]
    for idx, (inp, csub) in enumerate(sub_inputs):
        argv += [f"-c:s:{idx}", csub]
        argv += [f"-metadata:s:s:{idx}",
                 f"language={rng.choice(['eng', 'spa', 'ger', 'jpn', 'und'])}"]
    if meta_input is not None:
        argv += ["-map_metadata", str(meta_input),
                 "-map_chapters", str(meta_input)]
    argv += ["-shortest"] + out_t(needs_t, dur) + ["-f", cdef["fmt"], "-y", "__OUT__"]
    params = {"class": "multi-av", "container": container, "codec": vcodec,
              "res_name": rname, "width": w, "height": h, "fps": fps,
              "pix_fmt": pix, "duration": round(dur, 3),
              "audio_codec": acodecs[0], "audio_rate": aparams[0]["rate"],
              "audio_channels": aparams[0]["channels"], "features": feats}
    return [argv], params, aux


def build_dynres(rng, ctx, family, forced_res=None, forced_feats=None):
    container, codec = family
    ok_conts = [c for c in ("mkv", "ts", "mp4")
                if ctx.caps.get(f"concat_copy:{c}")]
    if ok_conts and container not in ok_conts:
        container = ok_conts[0]
    if not ok_conts:
        return None  # capability-probed: no container accepts copy-concat
    cdef = CONTAINERS[container]
    vdef = VIDEO_CODECS[codec]
    fps = draw_fps(rng, lo=10, hi=60)
    n_seg = rng.randint(2, 3)
    ladder = []
    used = set()
    for _ in range(n_seg):
        rname, w, h = draw_resolution(rng, vdef["even"], max_pixels=400_000,
                                      max_res=ctx.limits.get("max_res"),
                                      res_order=ctx.res_order)
        tries = 0
        while (w, h) in used and tries < 6:
            rname, w, h = draw_resolution(rng, vdef["even"], max_pixels=400_000)
            tries += 1
        used.add((w, h))
        ladder.append((rname, w, h, rng.uniform(1.5, 4.0)))
    pix = pick_pix(rng, codec, ctx.caps, ctx.limits.get("heavy", True))
    if pix is None:
        return None
    argvs = []
    seg_names = []
    total = 0.0
    for i, (rname, w, h, sd) in enumerate(ladder):
        sname = f"seg{i}.{cdef['ext']}"
        seg_names.append(sname)
        a, seg_needs_t = video_source_args(rng, w, h, fps, sd)
        a += ["-c:v", vdef["enc"], "-pix_fmt", pix]
        if "preset" in vdef:
            a += ["-preset", vdef["preset"]]
        if "deadline" in vdef:
            a += ["-deadline", vdef["deadline"]]
        a += ["-g", str(max(1, int(fps)))] + out_t(seg_needs_t, sd) + \
            ["-f", cdef["fmt"], "-y", f"__TMP__/{sname}"]
        argvs.append(a)
        total += sd
    concat_txt = "\n".join(f"file '__TMP__/{s}'" for s in seg_names)
    argvs.append(["-f", "concat", "-safe", "0", "-i", "__AUX__/concat.txt",
                  "-c", "copy", "-y", "__OUT__"])
    aux = [("concat.txt", concat_txt.encode())]
    params = {"class": "dynres-video", "container": container, "codec": codec,
              "res_name": "+".join(r[0] for r in ladder),
              "width": ladder[0][1], "height": ladder[0][2], "fps": fps,
              "pix_fmt": pix, "duration": round(total, 3),
              "features": {"res_ladder": [f"{r[1]}x{r[2]}" for r in ladder]}}
    return argvs, params, aux


def build_raw(rng, ctx, family, forced_res=None, forced_feats=None):
    _, target = family
    spec = RAW_VIDEO_TARGETS[target]
    vcodec = spec["codec"]
    vdef = VIDEO_CODECS[vcodec]
    rname, w, h = _resolve_res(rng, vdef, None, ctx, 400_000)
    fps = draw_fps(rng)
    dur = draw_duration(rng, w, h, ctx.limits.get("max_dur", 14.0))
    fps, dur = _ensure_frames(fps, dur, ctx.limits.get("max_dur", 14.0))
    pix = pick_pix(rng, vcodec, ctx.caps, ctx.limits.get("heavy", True))
    if pix is None:
        return None
    vin, needs_t = video_source_args(rng, w, h, fps, dur)
    argv = vin
    enc, gopf = venc_args(rng, vcodec, pix, fps, ctx.caps)
    argv += enc
    argv += out_t(needs_t, dur) + ["-f", spec["fmt"], "-y", "__OUT__"]
    params = {"class": "raw-video", "container": "raw", "codec": vcodec,
              "res_name": rname, "width": w, "height": h, "fps": fps,
              "pix_fmt": pix, "duration": round(dur, 3),
              "features": {"no_timestamps": True, **gopf}}
    return [argv], params, []


def build_fmp4(rng, ctx, family, forced_res=None, forced_feats=None):
    container, codec = family
    cdef = CONTAINERS[container]
    vdef = VIDEO_CODECS[codec]
    rname, w, h = _resolve_res(rng, vdef, None, ctx, 800_000)
    fps = draw_fps(rng)
    dur = draw_duration(rng, w, h, ctx.limits.get("max_dur", 14.0))
    fps, dur = _ensure_frames(fps, dur, ctx.limits.get("max_dur", 14.0))
    pix = pick_pix(rng, codec, ctx.caps, ctx.limits.get("heavy", True))
    if pix is None:
        return None
    vin, needs_t = video_source_args(rng, w, h, fps, dur)
    argv = vin
    enc, gopf = venc_args(rng, codec, pix, fps, ctx.caps)
    argv += enc
    frag_us = rng.choice([250000, 500000, 1000000, rng.randint(200000, 2000000)])
    argv += ["-movflags", "frag_keyframe+empty_moov+default_base_moof",
             "-frag_duration", str(frag_us)] + out_t(needs_t, dur) + \
            ["-f", cdef["fmt"], "-y", "__OUT__"]
    params = {"class": "fmp4-video", "container": container, "codec": codec,
              "res_name": rname, "width": w, "height": h, "fps": fps,
              "pix_fmt": pix, "duration": round(dur, 3),
              "features": {"frag_us": frag_us, "fragmented": True, **gopf}}
    return [argv], params, []


def build_hls(rng, ctx, family, forced_res=None, forced_feats=None):
    _, combo = family
    vcodec, acodec = combo.split("+")
    seg_type = "fmp4" if family[0] == "hlsf" else "ts"
    if seg_type == "fmp4" and not ctx.caps.get("hls_fmp4"):
        seg_type = "ts"
    fps = draw_fps(rng, lo=15, hi=30)
    dur = min(rng.uniform(4.0, 12.0), ctx.limits.get("max_dur", 14.0))
    hls_time = round(rng.uniform(1.0, 4.0), 2)
    n_tiers = rng.randint(1, 3)
    tiers = []
    for _ in range(n_tiers):
        rname, w, h = draw_resolution(rng, True, max_pixels=400_000,
                                      max_res=ctx.limits.get("max_res"),
                                      res_order=ctx.res_order)
        bw = rng.choice([400, 800, 1500, 2500, 4000, 6000])
        tiers.append((rname, w, h, bw))
    argvs = []
    aux = []
    master_lines = ["#EXTM3U",
                    "#EXT-X-VERSION:" + ("7" if seg_type == "fmp4" else "3"),
                    "#EXT-X-INDEPENDENT-SEGMENTS"]
    for i, (rname, w, h, bw) in enumerate(tiers):
        a, tier_needs_t = video_source_args(rng, w, h, fps, dur)
        a += audio_source_args(rng, 48000, 2, dur)
        a += ["-c:v", "libx264", "-preset", "veryfast", "-pix_fmt", "yuv420p",
              "-b:v", f"{bw}k", "-maxrate", f"{int(bw * 1.2)}k",
              "-bufsize", f"{bw * 2}k", "-g", str(max(1, int(fps * 2))),
              "-c:a", AUDIO_CODECS[acodec]["enc"], "-ar", "48000", "-ac", "2",
              "-shortest", "-f", "hls", "-hls_time", str(hls_time),
              "-hls_list_size", "0",
              "-hls_flags", "independent_segments+program_date_time"]
        if seg_type == "fmp4":
            a += ["-hls_segment_type", "fmp4",
                  # init filename must be RELATIVE to the segment dir — an
                  # absolute path makes the muxer look for it INSIDE the
                  # segment dir and fail (measured: 'Failed to open segment')
                  "-hls_fmp4_init_filename", f"init{i}.mp4",
                  "-hls_segment_filename", f"__OUTDIR__/v{i}_%03d.m4s"]
        else:
            a += ["-hls_segment_type", "mpegts",
                  "-hls_segment_filename", f"__OUTDIR__/v{i}_%03d.ts"]
        a += out_t(tier_needs_t, dur) + ["-y", "__OUTDIR__/v%d.m3u8" % i]
        argvs.append(a)
        master_lines += [f"#EXT-X-STREAM-INF:BANDWIDTH={bw * 1000 + 128000},"
                         f"RESOLUTION={w}x{h},NAME=\"{rname}\"", f"v{i}.m3u8"]
    aux.append(("master.m3u8", ("\n".join(master_lines) + "\n").encode()))
    params = {"class": "hls-av", "container": "hls", "codec": vcodec,
              "audio_codec": acodec, "res_name": "+".join(t[0] for t in tiers),
              "width": tiers[0][1], "height": tiers[0][2], "fps": fps,
              "duration": round(dur, 3), "audio_rate": 48000,
              "audio_channels": 2,
              "features": {"hls_time": hls_time, "seg_type": seg_type,
                           "tiers": [f"{t[1]}x{t[2]}@{t[3]}k" for t in tiers]}}
    return argvs, params, aux


COVER_CONT_FOR = {"mp3": "mp3", "flac": "flac", "m4a": "ipod",
                  "ogg": "ogg", "opus": "opus"}


def build_cover(rng, ctx, family, forced_res=None, forced_feats=None):
    _, combo = family
    acodec = combo.split("+")[0]
    container = family[0]
    if container not in COVER_CONT_FOR or container not in AUDIO_CODECS.get(acodec, {}).get("cont", []):
        options = [c for c in AUDIO_CODECS[acodec]["cont"] if c in COVER_CONT_FOR]
        if not options:
            return None
        container = options[0]
    cdef = ANY_CONTAINERS[container]
    ap = draw_audio_params(rng, acodec)
    if acodec == "opus":
        ap["rate"] = 48000
    dur = min(rng.uniform(3.0, 12.0), ctx.limits.get("max_dur", 14.0))
    s = rng.choice([300, 400, 500, 600, 800, 1000, 1200])
    argv = audio_source_args(rng, ap["rate"], ap["channels"], dur)
    argv += ["-f", "lavfi", "-i", f"testsrc2=duration=0.05:size={s}x{s}:rate=10"]
    argv += ["-map", "0:a", "-map", "1:v", "-frames:v", "1",
             "-disposition:v", "attached_pic",
             "-c:v", "mjpeg", "-q:v", str(rng.choice([2, 5, 10]))]
    argv += aenc_args(rng, acodec)
    argv += ["-ar", str(ap["rate"]), "-ac", str(ap["channels"])]
    argv += ["-metadata:s:v", "title=cover", "-f", cdef["fmt"], "-y", "__OUT__"]
    params = {"class": "cover-audio", "container": container, "codec": acodec,
              "audio_codec": acodec, "duration": round(dur, 3),
              "audio_rate": ap["rate"], "audio_channels": ap["channels"],
              "features": {"cover_size": s}}
    return [argv], params, []


def build_multivideo(rng, ctx, family, forced_res=None, forced_feats=None):
    _, combo = family
    parts = combo.split("+")
    container = family[0]
    cdef = CONTAINERS[container]
    vcodecs = [p for p in parts if p in VIDEO_CODECS][:2]
    acodec = next((p for p in parts if p in AUDIO_CODECS), "pcm_s16le")
    if len(vcodecs) < 2:
        pool = [c for c in CONTAINERS[container]["codecs"] if c in VIDEO_CODECS]
        vcodecs = rng.sample(pool, 2) if len(pool) >= 2 else ["mpeg4", "mjpeg"]
    ladder = []
    argv = []
    needs_t = False
    for vc in vcodecs:
        vdef = VIDEO_CODECS[vc]
        rname, w, h = _resolve_res(rng, vdef, None, ctx, 300_000)
        fps = draw_fps(rng, lo=10, hi=30)
        dur = min(rng.uniform(2.0, 8.0), ctx.limits.get("max_dur", 14.0))
        ladder.append((rname, w, h, fps, dur))
        vin, nt = video_source_args(rng, w, h, fps, dur)
        argv += vin
        needs_t = needs_t or nt
    ap = draw_audio_params(rng, acodec)
    argv += audio_source_args(rng, ap["rate"], ap["channels"], ladder[0][4])
    argv += ["-map", "0:v", "-map", "1:v", "-map", "2:a"]
    for vi, vc in enumerate(vcodecs):
        pix = pick_pix(rng, vc, ctx.caps, ctx.limits.get("heavy", True))
        if pix is None:
            return None
        argv += [f"-c:v:{vi}", VIDEO_CODECS[vc]["enc"], "-pix_fmt", pix]
        if "preset" in VIDEO_CODECS[vc]:
            argv += ["-preset", VIDEO_CODECS[vc]["preset"]]
        if "deadline" in VIDEO_CODECS[vc]:
            argv += ["-deadline", VIDEO_CODECS[vc]["deadline"]]
    argv += aenc_args(rng, acodec)
    argv += ["-ar", str(ap["rate"]), "-ac", str(ap["channels"]), "-shortest",
             "-metadata:s:v:1", "title=angle2"] + out_t(needs_t, ladder[0][4]) + \
            ["-f", cdef["fmt"], "-y", "__OUT__"]
    params = {"class": "multivideo-av", "container": container,
              "codec": vcodecs[0], "audio_codec": acodec,
              "res_name": ladder[0][0], "width": ladder[0][1],
              "height": ladder[0][2], "fps": ladder[0][3],
              "duration": round(ladder[0][4], 3),
              "audio_rate": ap["rate"], "audio_channels": ap["channels"],
              "features": {"video_codecs": vcodecs,
                           "res": [f"{r[1]}x{r[2]}" for r in ladder]}}
    return [argv], params, []


def build_drift(rng, ctx, family, forced_res=None, forced_feats=None):
    container, combo = family
    vcodec, acodec = combo.split("+")
    rname, w, h = draw_resolution(rng, True, max_pixels=100_000,
                                  max_res="w360p", res_order=ctx.res_order)
    fps = draw_fps(rng, lo=10, hi=30)
    dur = rng.uniform(45.0, 110.0)
    rate = rng.choice([44100, 48000])
    vin, needs_t = video_source_args(rng, w, h, fps, dur)
    argv = vin
    argv += audio_source_args(rng, rate, 2, dur)
    vdef = VIDEO_CODECS[vcodec]
    argv += ["-c:v", vdef["enc"], "-pix_fmt", "yuv420p"]
    if "preset" in vdef:
        argv += ["-preset", vdef["preset"]]
    if "deadline" in vdef:
        argv += ["-deadline", vdef["deadline"]]
    argv += ["-g", str(max(1, int(fps * 4)))]
    argv += aenc_args(rng, acodec)
    argv += ["-ar", str(rate), "-ac", "2", "-shortest"] + \
        out_t(needs_t, dur) + ["-f", CONTAINERS[container]["fmt"], "-y", "__OUT__"]
    params = {"class": "drift-av", "container": container, "codec": vcodec,
              "audio_codec": acodec, "res_name": rname, "width": w,
              "height": h, "fps": fps, "duration": round(dur, 3),
              "audio_rate": rate, "audio_channels": 2,
              "features": {"drift_probe": True}}
    return [argv], params, []


BOMB_KEYS = ["title", "artist", "album", "comment", "description",
             "synopsis", "lyrics", "composer", "genre", "encoder", "copyright"]


def build_bomb(rng, ctx, family, forced_res=None, forced_feats=None):
    container, combo = family
    built = _av_common(rng, ctx, container, combo.split("+")[0],
                       combo.split("+")[1], None, {"small": True})
    if built is None:
        return None
    argv, params, feats, needs_t = built
    n_tags = rng.randint(2, 6)
    bomb = {}
    cjk = "的一是了我不人在他有这上们来到时大地为子中你说生国年着就那和要她出也得里后自以会家可下而过天去能对小多然于心学么之都好看起发当没成只如事把还用第样道想作种开美总从无情己面最女但前真所让已"
    ascii_pool = "abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ0123456789"
    for _ in range(n_tags):
        key = rng.choice(BOMB_KEYS)
        size = rng.choice([512, 2048, 8192, 32768, 65536])
        if rng.random() < 0.5:
            val = "".join(rng.choice(cjk) for _ in range(min(size, 21845)))
        else:
            val = "".join(rng.choice(ascii_pool) for _ in range(size))
        bomb[key] = len(val.encode("utf-8"))
        argv += ["-metadata", f"{key}={val}"]
    argv += out_t(needs_t, params["duration"]) + ["-f", CONTAINERS[container]["fmt"], "-y", "__OUT__"]
    params.update({"class": "meta-bomb-av", "container": container,
                   "features": {**feats, "metadata_bomb": bomb}})
    return [argv], params, []


BUILDERS = {
    "video": build_video, "av": build_av, "audio": build_audio,
    "multi-av": build_multi_av, "dynres-video": build_dynres,
    "raw-video": build_raw, "fmp4-video": build_fmp4, "hls-av": build_hls,
    "cover-audio": build_cover, "multivideo-av": build_multivideo,
    "drift-av": build_drift, "meta-bomb-av": build_bomb,
}
