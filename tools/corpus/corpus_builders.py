"""corpus_builders.py — ffmpeg command assembly for every corpus class.

Each builder returns (argvs, params, aux) — or, for v4 classes that need
byte-level post-processing, (argvs, params, aux, post) — where:
  argvs : list of ffmpeg argv LISTS (multi-pass classes run them in order;
          the LAST one produces the entry's target file).  Path markers:
          __OUT__ (target file), __OUTDIR__ (target dir for HLS),
          __TMP__ (per-entry temp dir), __AUX__ (per-entry aux dir),
          __AUXIDX__ (resolved to the aux input index at build time).
  params: manifest `params` dict (harness-compatible keys: width/height/
          fps/pix_fmt/duration/audio_rate/audio_channels/audio_codec/res_name;
          v4 builders may add `est_hint` — a builder-computed byte estimate
          the budget gate uses instead of the generic formula — and record
          their post-op spec under `post`)
  aux   : list of (relname, content-bytes) written before argvs run
          (subtitles, FFMETADATA chapters, concat lists, m3u8 masters,
          maxtracks data-stream bins, still-24h source frames).
          Content is a pure function of the entry's rng — replay-safe.
  post  : optional list of post-op specs executed by the generator AFTER the
          argvs succeed and BEFORE harvest/validation (v4): {"op":
          "avi-rechunk"|"riff-size-lie"|"tkhd-rotate", ...drawn params}.

A builder returns None when a draw is unsupported (pix fmt without the
capability probe, etc.) — the generator records an honest rejected draw.

All randomness flows through `rng` (random.Random seeded per-entry) and
`caps` (probed ffmpeg capabilities).  Nothing is hardcoded that ffmpeg
itself can adjudicate.
"""

from __future__ import annotations

import random

from corpus_vocab import (
    ANAMORPHIC_SAR, AUDIO_CODECS, AUDIO_SOURCES,
    ANY_CONTAINERS, AVCC_PROFILES, BFRAME_STORM_H264, BFRAME_STORM_H265,
    CHAPTER_BOMB_RANGE, CONTAINERS, FPS_EXTREME_HI,
    FPS_EXTREME_LO, FPS_POOL, FPS_WEIGHTS,
    HEVC_AVI_TAGS, MAXTRACKS_AUDIO_RATES, MAXTRACKS_DATA_STREAMS,
    MAXTRACKS_RANGE, MAX_AUDIO_TRACKS, MAX_CHAPTERS, MAX_SUB_TRACKS,
    MIDCHANGE_HI_KBPS, MIDCHANGE_LO_KBPS, NO_DURATION_SRCS, ODD_DIM_SMALL,
    ODD_DIM_SUB16, ODD_LAYOUTS, ODD_SOURCES, RAW_VIDEO_TARGETS,
    RECHUNK_SPLIT_POOL, RES_BY_NAME, RES_WEIGHTS, RIFF_LIE_MODES,
    ROT_TRANSFORMS, SR_CHAOS_RATES, STILL_HOURS_RANGE, STILL_SLIDE_INTERVAL,
    SUB_TEXT_POOL, SUBTITLE_CODEC_FOR, VBV_BUF_KB, VBV_FLOOR_KBPS,
    VBV_SPIKE_KBPS, VIDEO_CODECS, VIDEO_SOURCES, VIDEO_SOURCE_WEIGHTS,
    draw_gop, vfr_expr,
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
    floor = vdef.get("min", 0)  # encoder-measured minimum (svt-av1/prores
    #                            # reject tiny frames: measured 16x16/32x32)
    if forced_res:
        w, h = RES_BY_NAME[forced_res]
        if vdef["even"]:
            w += w % 2
            h += h % 2
        return forced_res, max(w, floor), max(h, floor)
    name, w, h = draw_resolution(rng, vdef["even"], max_pixels=cap,
                                 max_res=ctx.limits.get("max_res"),
                                 res_order=ctx.res_order)
    if floor and (w < floor or h < floor):
        w, h = max(w, floor), max(h, floor)
        name = f"{name}+floor{floor}"
    return name, w, h


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


# ---------------------------------------------------------------------------
# v4 escalation builders (W5-16).  Every mechanism below was probed against
# the real ffmpeg 7.1.5 before being wired in (see README §9 v4 realities);
# remaining uncertainty flows through the per-draw honest-rejects ledger.
# ---------------------------------------------------------------------------

def _odd_source(rng, w, h, fps, dur):
    """Odd-preserving lavfi video input (color-family sources only: testsrc2
    rounds odd sizes down to even internally — measured)."""
    src = rng.choice(ODD_SOURCES)
    extra = ""
    if src == "color":
        extra = f"color={rng.choice(['red', 'green', 'blue', 'gray', 'black'])}:"
    elif src == "gradients" and rng.random() < 0.5:
        extra = f"n={rng.randint(2, 8)}:"
    return ["-f", "lavfi", "-i",
            f"{src}={extra}size={w}x{h}:rate={_fmt_fps(fps)}:duration={dur:.3f}"]


def _tiny_res(rng, cap=100_000):
    names = [n for n in RES_WEIGHTS
             if RES_BY_NAME[n][0] * RES_BY_NAME[n][1] <= cap]
    name = rng.choices(names, weights=[RES_WEIGHTS[n] for n in names], k=1)[0]
    w, h = RES_BY_NAME[name]
    return name, w, h


def build_odd_dims(rng, ctx, family, forced_res=None, forced_feats=None):
    """1x1 .. 101x101 odd/sub-16/non-mod-2 geometry, yuv422/444 where the
    encoder permits.  Odd dims are preserved by the color-family sources;
    ffmpeg silently crops odd sizes handed to even-only encoders, so the
    codec pool is the measured odd-preserving set."""
    container, codec = family
    feats = dict(forced_feats or {})
    if feats.get("dims"):
        w, h = feats["dims"]
        rname = f"odd{w}x{h}"
    else:
        lo, hi = rng.choice([ODD_DIM_SUB16, ODD_DIM_SMALL, ODD_DIM_SUB16])
        w = rng.randrange(lo, hi + 1, 2)
        h = rng.randrange(lo, hi + 1, 2)
        rname = f"odd{w}x{h}"
    fps = rng.randint(5, 25)
    dur = rng.uniform(0.4, 2.0)
    if w == 1 and h == 1 and not ctx.caps.get("odd1x1"):
        return None  # capability-probed: encoder refuses 1x1 on this build
    if codec == "h264":
        pix = "yuv444p"  # 4:2:0 h264 silently crops odd dims (measured)
        if not ctx.caps.get("x264_444"):
            return None
    elif codec == "mjpeg":
        pix = rng.choice(["yuv420p", "yuv422p", "yuv444p", "gray"])
    elif codec == "ffv1":
        pix = rng.choice(["yuv420p", "yuv444p"])
    else:  # mpeg4
        pix = "yuv420p"
    argv = _odd_source(rng, w, h, fps, dur)
    argv += ["-c:v", VIDEO_CODECS[codec]["enc"], "-pix_fmt", pix]
    if "preset" in VIDEO_CODECS[codec]:
        argv += ["-preset", VIDEO_CODECS[codec]["preset"]]
    argv += ["-f", CONTAINERS[container]["fmt"], "-y", "__OUT__"]
    params = {"class": "odd-dims", "container": container, "codec": codec,
              "res_name": rname, "width": w, "height": h, "fps": fps,
              "pix_fmt": pix, "duration": round(dur, 3),
              "features": {"odd_dims": True}}
    return [argv], params, []


def build_rot_display(rng, ctx, family, forced_res=None, forced_feats=None):
    """Display-matrix rotation metadata (90/180/270 + flips) on mp4/mkv/mov.
    Pure rotations ride the encoder-assisted route: `-display_rotation` on
    the input + `-c copy` — movenc AND matroskaenc write real display
    matrices this way (measured; the v3 `rotate=` metadata route is dead).
    Flip combos need the tkhd byte-patch route (ffmpeg 7.1.5 refuses
    -display_hflip/-display_vflip as CLI options)."""
    container, vcodec = family
    feats = dict(forced_feats or {})
    name = feats.get("rotation")
    if name is None:
        name = rng.choice([t[0] for t in ROT_TRANSFORMS])
    tf = next((t for t in ROT_TRANSFORMS if t[0] == name), ROT_TRANSFORMS[0])
    _, angle, hflip, vflip = tf
    rname, w, h = _tiny_res(rng)
    fps = rng.randint(10, 30)
    dur = rng.uniform(1.0, 3.0)
    pix = "yuv420p"
    if vcodec not in ("h264", "mpeg4", "mjpeg"):
        vcodec = "h264"
    vin = ["-f", "lavfi", "-i",
           f"testsrc2=duration={dur:.3f}:size={w}x{h}:rate={fps}"]
    pass1 = vin + ["-c:v", VIDEO_CODECS[vcodec]["enc"], "-pix_fmt", pix,
                   "-preset", "ultrafast", "-f", "mp4", "-y", "__TMP__/r0.mp4"]
    fmt = CONTAINERS[container]["fmt"]
    if (hflip or vflip):
        if container not in ("mp4", "mov") or not ctx.caps.get("rot_tkhd"):
            return None  # patch route is ISO-BMFF only, probe-gated
        pass2 = ["-i", "__TMP__/r0.mp4", "-c", "copy",
                 "-f", fmt, "-y", "__OUT__"]
        post = [{"op": "tkhd-rotate", "rot": angle, "hflip": hflip,
                 "vflip": vflip, "name": name}]
        route = "tkhd-patch"
    else:
        if not ctx.caps.get(f"rot_copy:{container}"):
            return None  # capability-probed: no display-matrix write route
        pass2 = ["-display_rotation:v:0", str(angle), "-i", "__TMP__/r0.mp4",
                 "-c", "copy", "-f", fmt, "-y", "__OUT__"]
        post = []
        route = "display-rotation-copy"
    params = {"class": "rot-display", "container": container, "codec": vcodec,
              "res_name": rname, "width": w, "height": h, "fps": fps,
              "pix_fmt": pix, "duration": round(dur, 3),
              "features": {"rotation": name, "route": route},
              "est_hint": int(w * h * dur * 0.10) + 12_288}
    return [pass1, pass2], params, [], post


def build_maxtracks_avi(rng, ctx, family, forced_res=None, forced_feats=None):
    """AVI with 16-32 streams (mixed video/audio/data).  Subtitle streams
    are REFUSED by the AVI muxer ('Not yet implemented') — data streams via
    `-f data` aux bins are the third brand (muxed with an empty fourcc)."""
    n_streams = rng.randint(*MAXTRACKS_RANGE)
    n_data = rng.randint(*MAXTRACKS_DATA_STREAMS)
    n_video = rng.randint(max(2, n_streams // 6), max(3, n_streams // 2))
    n_audio = max(1, n_streams - n_video - n_data)
    w, h = rng.choice([(64, 48), (96, 64), (128, 96), (160, 120)])
    fps = rng.randint(4, 8)
    dur = rng.uniform(0.8, 1.5)
    vcodec = rng.choice(["mjpeg", "mjpeg", "mpeg4"])
    argv = []
    maps = []
    aux = []
    est = 0
    for i in range(n_video):
        argv += ["-f", "lavfi", "-i",
                 f"testsrc2=duration={dur:.3f}:size={w}x{h}:rate={fps}"]
        maps.append((len([a for a in argv if a == "-i"]) - 1, "v"))
    for i in range(n_audio):
        arate = rng.choice(MAXTRACKS_AUDIO_RATES)
        argv += ["-f", "lavfi", "-i",
                 f"anoisesrc=color=pink:duration={dur:.3f}:"
                 f"sample_rate={arate}:amplitude=0.5"]
        maps.append((len([a for a in argv if a == "-i"]) - 1, "a"))
        est += int(arate * dur * 1.2)
    for i in range(n_data):
        blob = bytes(rng.randrange(256) for _ in range(rng.randint(64, 512)))
        aux.append((f"data{i}.bin", blob))
        argv += ["-f", "data", "-i", f"__AUX__/data{i}.bin"]
        maps.append((len([a for a in argv if a == "-i"]) - 1, "d"))
    for inp, kind in maps:
        argv += ["-map", f"{inp}:{kind}"]
    for i in range(n_video):
        argv += [f"-c:v:{i}", VIDEO_CODECS[vcodec]["enc"],
                 f"-pix_fmt:v:{i}", "yuv420p"]
    for i in range(n_audio):
        argv += [f"-c:a:{i}", "pcm_u8"]
    for i in range(n_data):
        argv += [f"-c:d:{i}", "copy"]
    est += int(w * h * dur * 0.25 * 0.5 * n_video) + 24_576
    argv += ["-f", "avi", "-y", "__OUT__"]
    params = {"class": "maxtracks-avi", "container": "avi", "codec": vcodec,
              "res_name": f"{w}x{h}", "width": w, "height": h, "fps": fps,
              "pix_fmt": "yuv420p", "duration": round(dur, 3),
              "audio_codec": "pcm_u8", "audio_rate": MAXTRACKS_AUDIO_RATES[0],
              "audio_channels": 1,
              "features": {"n_streams": n_streams, "n_video": n_video,
                           "n_audio": n_audio, "n_data": n_data},
              "est_hint": est}
    return [argv], params, aux


def build_bframe_storm(rng, ctx, family, forced_res=None, forced_feats=None):
    """h264/h265 with bf >= 3 + b-pyramid: maximum decode-reorder depth.
    The demuxer must hold a deep reorder window; ts carries it raw."""
    container, codec = family
    if codec == "h264":
        bf = rng.randint(*BFRAME_STORM_H264)
        cont_pool = ["mp4", "ts", "mkv", "mov"]
    else:
        bf = rng.randint(*BFRAME_STORM_H265)  # x265 lookahead cap (v3 #2)
        cont_pool = ["mp4", "ts", "mkv"]
    if container not in cont_pool:
        container = rng.choice(cont_pool)
    rname, w, h = _tiny_res(rng, cap=200_000)
    fps = rng.randint(12, 30)
    dur = rng.uniform(1.0, 2.5)
    argv = ["-f", "lavfi", "-i",
            f"testsrc2=duration={dur:.3f}:size={w}x{h}:rate={fps}"]
    if rng.random() < 0.3:
        ar = rng.choice([22050, 44100, 48000])
        argv += ["-f", "lavfi", "-i",
                 f"sine=duration={dur:.3f}:sample_rate={ar}"]
        have_audio = True
    else:
        ar = None
        have_audio = False
    argv += ["-c:v", VIDEO_CODECS[codec]["enc"], "-pix_fmt", "yuv420p",
             "-preset", "ultrafast", "-bf", str(bf), "-b-pyramid", "1",
             "-g", str(rng.randint(12, 90))]
    all_b = rng.random() < 0.35 and codec == "h264"
    if all_b:
        argv += ["-x264-params", "b-adapt=0"]
    if have_audio:
        argv += ["-map", "0:v", "-map", "1:a", "-c:a", "aac",
                 "-ar", str(ar)]
    argv += ["-f", CONTAINERS[container]["fmt"], "-y", "__OUT__"]
    params = {"class": "bframe-storm", "container": container, "codec": codec,
              "res_name": rname, "width": w, "height": h, "fps": fps,
              "pix_fmt": "yuv420p", "duration": round(dur, 3),
              "features": {"bf": bf, "bpyramid": True, "all_b": all_b}}
    if have_audio:
        params["audio_codec"] = "aac"
        params["audio_rate"] = ar
        params["audio_channels"] = 2
    return [argv], params, []


def build_packed_bs(rng, ctx, family, forced_res=None, forced_feats=None):
    """Packed-bitstream / AVCDecoderConfiguration oddities where ffmpeg
    permits: libxvid packed b-frames in AVI, native mpeg4 b-frame reordering,
    and h264 avcC profile/level/inline-header lies in the mp4 family."""
    container, mode = family
    rname, w, h = _tiny_res(rng)
    fps = rng.randint(10, 25)
    dur = rng.uniform(0.8, 2.0)
    argv = ["-f", "lavfi", "-i",
            f"testsrc2=duration={dur:.3f}:size={w}x{h}:rate={fps}"]
    feats = {}
    if mode == "xvid":
        if not ctx.caps.get("xvid"):
            return None  # capability-probed: no libxvid on this build
        bf = rng.randint(2, 3)
        argv += ["-c:v", "libxvid", "-bf", str(bf),
                 "-g", str(rng.randint(12, 60)),
                 "-b:v", str(rng.randint(80, 400)) + "k"]
        feats = {"mode": "xvid-packed", "bf": bf}
        fmt = "avi"
    elif mode == "mpeg4bf":
        bf = rng.randint(2, 3)
        argv += ["-c:v", "mpeg4", "-bf", str(bf),
                 "-g", str(rng.randint(12, 60)),
                 "-b:v", str(rng.randint(80, 500)) + "k"]
        feats = {"mode": "mpeg4-bf", "bf": bf}
        fmt = container if container in ("avi", "mp4") else "avi"
    else:  # avcc oddities
        profile = rng.choice(list(AVCC_PROFILES))
        level = rng.choice(AVCC_PROFILES[profile])
        xp = []
        if rng.random() < 0.35:
            xp.append("repeat-headers=1")
        if rng.random() < 0.35:
            xp.append("aud=1")
        argv += ["-c:v", "libx264", "-preset", "ultrafast",
                 "-pix_fmt", "yuv420p", "-profile:v", profile,
                 "-level", str(level)]
        if xp:
            argv += ["-x264-params", ":".join(xp)]
        feats = {"mode": "avcc-oddity", "profile": profile, "level": level,
                 "x264_params": xp}
        fmt = container if container in ("mp4", "mov") else "mp4"
    argv += ["-f", fmt, "-y", "__OUT__"]
    params = {"class": "packed-bs", "container": fmt,
              "codec": "mpeg4" if mode != "avcc" else "h264",
              "res_name": rname, "width": w, "height": h, "fps": fps,
              "pix_fmt": "yuv420p", "duration": round(dur, 3),
              "features": feats}
    return [argv], params, []


def _concat_segs_pass(segs):
    return ["-f", "concat", "-safe", "0", "-i", "__AUX__/concat.txt",
            "-c", "copy", "-y", "__OUT__"]


def build_param_midchange(rng, ctx, family, forced_res=None, forced_feats=None):
    """Mid-stream SPS/parameter change (same resolution, different encode
    parameters) via per-segment encode + concat copy — h264 in ts, mpeg2 in
    mp4, per the capability probes."""
    container, codec = family
    if not ctx.caps.get(f"concat_copy:{container}"):
        return None
    if codec == "mpeg2video" and container == "mp4" \
            and not ctx.caps.get("mpeg2_mp4"):
        return None  # capability-probed: mp4 muxer refuses mpeg2 here
    rname, w, h = _tiny_res(rng)
    fps = rng.randint(10, 24)
    n_seg = rng.randint(2, 3)
    argvs = []
    seg_names = []
    ladder = []
    total = 0.0
    for i in range(n_seg):
        sd = rng.uniform(0.8, 2.5)
        total += sd
        seg = f"seg{i}"
        seg_names.append(seg)
        a = ["-f", "lavfi", "-i",
             f"testsrc2=duration={sd:.3f}:size={w}x{h}:rate={fps}"]
        a += ["-c:v", VIDEO_CODECS[codec]["enc"], "-pix_fmt", "yuv420p"]
        if "preset" in VIDEO_CODECS[codec]:
            a += ["-preset", VIDEO_CODECS[codec]["preset"]]
        seg_feats = {}
        if i % 2 == 0:
            seg_feats["b"] = rng.randint(*MIDCHANGE_LO_KBPS)
        else:
            seg_feats["b"] = rng.randint(*MIDCHANGE_HI_KBPS)
        a += ["-b:v", f"{seg_feats['b']}k"]
        if codec == "h264" and i > 0:
            prof = rng.choice(["baseline", "main", "high"])
            a += ["-profile:v", prof]
            seg_feats["profile"] = prof
        a += ["-g", str(rng.randint(6, 40)), "-f",
              CONTAINERS[container]["fmt"], "-y", f"__TMP__/{seg}"]
        argvs.append(a)
        ladder.append(seg_feats)
    concat_txt = "\n".join(f"file '__TMP__/{s}'" for s in seg_names)
    argvs.append(_concat_segs_pass(seg_names))
    aux = [("concat.txt", concat_txt.encode())]
    params = {"class": "param-midchange", "container": container,
              "codec": codec, "res_name": rname, "width": w, "height": h,
              "fps": fps, "pix_fmt": "yuv420p",
              "duration": round(total, 3),
              "features": {"seg_params": ladder},
              "est_hint": int(w * h * total * 0.35) + 24_576}
    return argvs, params, aux


def build_vbv_burst(rng, ctx, family, forced_res=None, forced_feats=None):
    """Extreme bitrate spikes: a low floor with a short high-bitrate spike,
    either via per-segment encode + concat copy (true step function) or
    single-pass VBV burst settings (maxrate >> b:v with a tiny bufsize)."""
    container, codec = family
    concat_ok = ctx.caps.get(f"concat_copy:{container}")
    use_concat = concat_ok and rng.random() < 0.6
    rname, w, h = _tiny_res(rng)
    fps = rng.randint(10, 24)
    floor_k = rng.randint(*VBV_FLOOR_KBPS)
    spike_k = rng.randint(*VBV_SPIKE_KBPS)
    buf_k = rng.randint(*VBV_BUF_KB)
    if use_concat:
        base_d = rng.uniform(1.5, 4.0)
        spike_d = rng.uniform(0.8, 2.0)
        argvs = []
        seg_names = []
        for name, kbps, d in (("low", floor_k, base_d),
                              ("spike", spike_k, spike_d)):
            seg = f"{name}0"
            seg_names.append(seg)
            src = "mandelbrot" if name == "spike" else "testsrc2"
            a = ["-f", "lavfi", "-i",
                 f"{src}=size={w}x{h}:rate={fps}"]
            a += ["-c:v", VIDEO_CODECS[codec]["enc"], "-pix_fmt", "yuv420p",
                  "-preset", "ultrafast", "-b:v", f"{kbps}k",
                  "-maxrate", f"{int(kbps * 1.2)}k", "-bufsize", f"{buf_k}k",
                  "-g", str(rng.randint(15, 60)), "-t", f"{d:.3f}",
                  "-f", CONTAINERS[container]["fmt"], "-y", f"__TMP__/{seg}"]
            argvs.append(a)
        concat_txt = "\n".join(f"file '__TMP__/{s}'" for s in seg_names)
        argvs.append(_concat_segs_pass(seg_names))
        aux = [("concat.txt", concat_txt.encode())]
        feats = {"mode": "concat-step", "floor_kbps": floor_k,
                 "spike_kbps": spike_k, "spike_s": round(spike_d, 3)}
        est = int(spike_k * 125 * spike_d + floor_k * 125 * base_d) + 32_768
        dur = base_d + spike_d
        source = "concat:mandelbrot-spike"
    else:
        dur = rng.uniform(2.0, 5.0)
        argv = ["-f", "lavfi", "-i", f"testsrc2=duration={dur:.3f}:"
                f"size={w}x{h}:rate={fps}"]
        argv += ["-c:v", VIDEO_CODECS[codec]["enc"], "-pix_fmt", "yuv420p",
                 "-preset", "ultrafast", "-b:v", f"{floor_k}k",
                 "-maxrate", f"{spike_k}k", "-bufsize", f"{buf_k}k",
                 "-f", CONTAINERS[container]["fmt"], "-y", "__OUT__"]
        argvs = [argv]
        aux = []
        feats = {"mode": "single-pass-vbv", "floor_kbps": floor_k,
                 "spike_kbps": spike_k, "bufsize_kb": buf_k}
        est = int((floor_k + spike_k) / 2 * 125 * dur * 0.6) + 32_768
        source = "testsrc2"
    params = {"class": "vbv-burst", "container": container, "codec": codec,
              "res_name": rname, "width": w, "height": h, "fps": fps,
              "pix_fmt": "yuv420p", "duration": round(dur, 3),
              "features": feats, "est_hint": est, "source": source}
    return argvs, params, aux


def build_fps_extremes(rng, ctx, family, forced_res=None, forced_feats=None):
    """1 fps slideshow <-> 240 fps short burst.  High rates only at tiny
    resolutions (bounded frames); low rates stretch a handful of frames
    over seconds — clocks and frame-timers get no rest either way."""
    container, codec = family
    feats = dict(forced_feats or {})
    mode = feats.get("fps_mode") or rng.choice(["lo", "hi"])
    if mode == "hi":
        fps = rng.choice(FPS_EXTREME_HI)
        rname, w, h = _tiny_res(rng, cap=40_000)
        dur = rng.uniform(0.4, 2.0)
    else:
        fps = rng.choice(FPS_EXTREME_LO)
        rname, w, h = _tiny_res(rng, cap=150_000)
        dur = rng.uniform(3.0, min(12.0, ctx.limits.get("max_dur", 14.0)))
        fps, dur = _ensure_frames(fps, dur, ctx.limits.get("max_dur", 14.0))
    pix = "yuv420p"
    if codec not in ("h264", "mpeg4", "mjpeg"):
        codec = rng.choice(["h264", "mpeg4"])
    argv = ["-f", "lavfi", "-i",
            f"testsrc2=duration={dur:.3f}:size={w}x{h}:rate={_fmt_fps(fps)}"]
    argv += ["-c:v", VIDEO_CODECS[codec]["enc"], "-pix_fmt", pix]
    if "preset" in VIDEO_CODECS[codec]:
        argv += ["-preset", VIDEO_CODECS[codec]["preset"]]
    argv += ["-f", CONTAINERS[container]["fmt"], "-y", "__OUT__"]
    params = {"class": "fps-extremes", "container": container, "codec": codec,
              "res_name": rname, "width": w, "height": h, "fps": fps,
              "pix_fmt": pix, "duration": round(dur, 3),
              "features": {"fps_mode": mode},
              "est_hint": int(w * h * dur * 0.10) + 16_384}
    return [argv], params, []


def build_sr_chaos(rng, ctx, family, forced_res=None, forced_feats=None):
    """Audio sample-rate chaos (8000/11025/192000/384000 Hz and friends) +
    odd channel layouts the encoder accepts (pan-built hexagonal/octagonal/
    22.2/...).  High rates draw narrow layouts, exotic layouts draw sane
    rates, so est_bytes stays bounded either way."""
    container, codec = family
    exotic = rng.random() < 0.65
    if exotic:
        layout, ch = rng.choice(ODD_LAYOUTS)
        rate = rng.choice([8000, 11025, 16000, 22050, 32000, 44100, 48000])
    else:
        layout, ch = None, rng.choice([1, 2])
        rate = rng.choice(SR_CHAOS_RATES)
    dur = rng.uniform(0.5, 2.5)
    argv = ["-f", "lavfi", "-i",
            f"anoisesrc=color={rng.choice(['pink', 'white', 'brown'])}:"
            f"duration={dur:.3f}:sample_rate={rate}:amplitude=0.5"]
    if layout:
        specs = "|".join(f"c{i}=c0" for i in range(ch))
        argv += ["-af", f"pan={layout}|{specs}"]
    else:
        argv += ["-ac", str(ch)]
    argv += ["-c:a", AUDIO_CODECS[codec]["enc"], "-ar", str(rate)]
    cdef = ANY_CONTAINERS[container]
    argv += ["-f", cdef["fmt"], "-y", "__OUT__"]
    params = {"class": "sr-chaos", "container": container, "codec": codec,
              "audio_codec": codec, "duration": round(dur, 3),
              "audio_rate": rate, "audio_channels": ch,
              "features": {"layout": layout or f"{ch}ch",
                           "odd_rate": rate not in STD_RATES_LIST},
              "est_hint": int(rate * ch * 2 * dur * 0.9) + 16_384}
    return [argv], params, []


def write_chapter_bomb(rng, dur, n):
    """FFMETADATA chapter bomb: n >= 1000 densely-cut chapters with random
    titles.  Chapter slices are >= ~1 ms; overlapping is allowed (hostile)."""
    per = dur / n
    chars = "abcdefghijkmnopqrstuvwxyz0123456789"
    out = [";FFMETADATA1", f"title=lumen-chapter-bomb-{rng.randint(1000, 9999)}"]
    for i in range(n):
        s = i * per
        e = max((i + 1) * per, s + 0.0012)
        t = "".join(rng.choice(chars) for _ in range(rng.randint(3, 14)))
        out += ["[CHAPTER]", "TIMEBASE=1/1000",
                f"START={int(s * 1000)}", f"END={int(e * 1000)}",
                f"title=b{i:05d}{t}"]
    return ("\n".join(out) + "\n").encode("utf-8")


def build_chapter_bomb(rng, ctx, family, forced_res=None, forced_feats=None):
    """1000-3000 chapters on a seconds-long clip — chapter-driven UIs,
    seek tables and progress bars all get their stress test."""
    container, combo = family
    vcodec, acodec = combo.split("+")
    n = rng.randint(*CHAPTER_BOMB_RANGE)
    max_dur = ctx.limits.get("max_dur", 14.0)
    dur = rng.uniform(max(2.0, n * 0.0011), max(max(2.5, n * 0.0011), max_dur))
    rname, w, h = _tiny_res(rng)
    fps = rng.randint(8, 20)
    argv = ["-f", "lavfi", "-i",
            f"testsrc2=duration={dur:.3f}:size={w}x{h}:rate={fps}",
            "-f", "lavfi", "-i",
            f"sine=duration={dur:.3f}:sample_rate=44100",
            "-i", "__AUX__/bomb.txt",
            "-map", "0:v", "-map", "1:a",
            "-c:v", VIDEO_CODECS[vcodec]["enc"], "-pix_fmt", "yuv420p"]
    if "preset" in VIDEO_CODECS[vcodec]:
        argv += ["-preset", VIDEO_CODECS[vcodec]["preset"]]
    argv += ["-c:a", AUDIO_CODECS[acodec]["enc"], "-ar", "44100", "-ac", "1",
             "-map_chapters", "2", "-f", CONTAINERS[container]["fmt"], "-y",
             "__OUT__"]
    aux = [("bomb.txt", write_chapter_bomb(rng, dur, n))]
    params = {"class": "chapter-bomb", "container": container,
              "codec": vcodec, "audio_codec": acodec, "res_name": rname,
              "width": w, "height": h, "fps": fps, "pix_fmt": "yuv420p",
              "duration": round(dur, 3), "audio_rate": 44100,
              "audio_channels": 1, "features": {"n_chapters": n},
              "est_hint": int(w * h * dur * 0.10) + n * 48 + 32_768}
    return [argv], params, aux


def build_zeroframe_stream(rng, ctx, family, forced_res=None,
                           forced_feats=None):
    """A video track that exists in the container but emits ZERO frames
    (select=0 drops every frame pre-encode).  mkv keeps the empty track and
    ts still declares it in the PMT; the mp4 muxer silently drops the track
    (probe truth) — only probed containers are drawn."""
    container, combo = family
    if not ctx.caps.get(f"zeroframe:{container}"):
        return None  # capability-probed: muxer drops or refuses empty tracks
    vcodec, acodec = combo.split("+")
    rname, w, h = _tiny_res(rng)
    fps = rng.randint(10, 24)
    dur = rng.uniform(1.0, 3.0)
    ar = rng.choice([22050, 44100, 48000])
    argv = ["-f", "lavfi", "-i",
            f"testsrc2=duration={dur:.3f}:size={w}x{h}:rate={fps}",
            "-f", "lavfi", "-i", f"sine=duration={dur:.3f}:sample_rate={ar}",
            "-map", "0:v", "-map", "1:a", "-vf", "select=0",
            "-c:v", VIDEO_CODECS[vcodec]["enc"], "-pix_fmt", "yuv420p",
            "-preset", "ultrafast", "-c:a", AUDIO_CODECS[acodec]["enc"],
            "-ar", str(ar), "-f", CONTAINERS[container]["fmt"], "-y",
            "__OUT__"]
    params = {"class": "zeroframe-stream", "container": container,
              "codec": vcodec, "audio_codec": acodec, "res_name": rname,
              "width": w, "height": h, "fps": fps, "pix_fmt": "yuv420p",
              "duration": round(dur, 3), "audio_rate": ar, "audio_channels": 1,
              "features": {"zero_frames": True},
              "est_hint": int(ar * 2 * dur * 0.2) + 12_288}
    return [argv], params, []


def build_still_24h(rng, ctx, family, forced_res=None, forced_feats=None):
    """Single-frame (or one-frame-per-N-seconds) still with a huge duration:
    image2 -loop 1 at a fractional framerate claims ~6-48 h from a handful
    of frames at a tiny bitrate — bounded bytes, pathological durations."""
    container, codec = family
    if not ctx.caps.get("still_frac"):
        return None  # capability-probed: no fractional-framerate image2 loop
    hours = rng.randint(*STILL_HOURS_RANGE)
    total = hours * 3600
    w = rng.choice([160, 240, 320, 480])
    h = int(w * rng.choice([0.75, 1.0, 1.25]))
    slideshow = rng.random() < 0.4
    # movenc writes 32-bit track durations in the track timescale (16384
    # for these weird timebases): a single-frame 1/total cadence past
    # 2^31/16384 = 131 072 s (~36.4 h) overflows — "Application provided
    # duration: ... is invalid", rc 234 (measured: 36 h OK, 40 h fails;
    # mkv has no such limit).  Long mp4/mov draws switch to the slideshow
    # route, whose coarse cadence keeps duration x timescale in range.
    if container in ("mp4", "mov") and not slideshow and total > 130_000:
        slideshow = True
    if slideshow:
        interval = rng.randint(*STILL_SLIDE_INTERVAL)
        cadence = f"1/{interval}"
        n_frames = total // interval + 1
    else:
        cadence = f"1/{total}"
        n_frames = 1
    src = rng.choice(["testsrc2", "smptebars", "gradients", "rgbtestsrc"])
    pass1 = ["-f", "lavfi", "-i",
             f"{src}=duration=0.04:size={w}x{h}:rate=25",
             "-frames:v", "1", "-y", "__TMP__/still.png"]
    argv = ["-loop", "1", "-framerate", cadence, "-i", "__TMP__/still.png",
            "-t", str(total)]
    if codec == "h264":
        argv += ["-c:v", "libx264", "-preset", "ultrafast",
                 "-tune", "stillimage", "-crf", str(rng.choice([30, 35, 40]))]
    elif codec == "mjpeg":
        argv += ["-c:v", "mjpeg", "-q:v", str(rng.choice([20, 30, 40]))]
    else:  # mpeg4
        argv += ["-c:v", "mpeg4", "-b:v", str(rng.choice([8, 16, 32])) + "k"]
    argv += ["-pix_fmt", "yuv420p", "-f", CONTAINERS[container]["fmt"],
             "-y", "__OUT__"]
    params = {"class": "still-24h", "container": container, "codec": codec,
              "res_name": f"{w}x{h}", "width": w, "height": h,
              "fps": round(1.0 / (total if not slideshow else interval), 9),
              "pix_fmt": "yuv420p", "duration": total,
              "features": {"hours": hours, "cadence": cadence,
                           "n_frames": n_frames},
              "est_hint": 131_072}
    return [pass1, argv], params, []


def build_riff_size_lie(rng, ctx, family, forced_res=None,
                        forced_feats=None):
    """Dirty-class addition: an honest little AVI whose RIFF size field lies
    (oversized / 0xFFFFFFFF / tiny / zero) via byte post-processing —
    v3's head-size-lie corruption op promoted to a dedicated factory with
    randomized lies.  Lands in dirty/ (excluded from the clean gate)."""
    _, combo = family
    vcodec, acodec = combo.split("+")
    w = rng.choice([64, 96, 128, 160])
    h = int(w * 0.75)
    fps = rng.randint(8, 15)
    dur = rng.uniform(0.6, 1.5)
    ar = rng.choice([8000, 11025, 16000])
    argv = ["-f", "lavfi", "-i",
            f"testsrc2=duration={dur:.3f}:size={w}x{h}:rate={fps}",
            "-f", "lavfi", "-i", f"sine=duration={dur:.3f}:sample_rate={ar}",
            "-map", "0:v", "-map", "1:a",
            "-c:v", VIDEO_CODECS[vcodec]["enc"], "-pix_fmt", "yuv420p",
            "-c:a", AUDIO_CODECS[acodec]["enc"], "-ar", str(ar), "-ac", "1",
            "-f", "avi", "-y", "__OUT__"]
    mode = rng.choice(RIFF_LIE_MODES)
    post = [{"op": "riff-size-lie", "mode": mode,
             "oversize_bytes": rng.randint(1 << 10, 1 << 28),
             "tiny_bytes": rng.randint(0, 4096)}]
    params = {"class": "riff-size-lie", "container": "avi", "codec": vcodec,
              "audio_codec": acodec, "res_name": f"{w}x{h}", "width": w,
              "height": h, "fps": fps, "pix_fmt": "yuv420p",
              "duration": round(dur, 3), "audio_rate": ar,
              "audio_channels": 1,
              "features": {"lie_mode": mode}}
    return [argv], params, [], post


def build_odd_chunk_avi(rng, ctx, family, forced_res=None,
                        forced_feats=None):
    """AVI re-packed with 1-byte-payload movi chunks + JUNK pad storms
    (byte post-processing).  Structurally valid RIFF that ffprobe still
    parses — but a 1.5 s file becomes ~10^5 chunks, and each 1-byte chunk
    is a packet on the demux side (measured).  idx1 is rebuilt to match or
    dropped as JUNK (drawn)."""
    _, combo = family
    vcodec, acodec = combo.split("+")
    w = rng.choice([64, 96, 128, 160])
    h = int(w * 0.75)
    fps = rng.randint(8, 15)
    dur = rng.uniform(0.5, 1.2)
    ar = rng.choice([4000, 5512, 8000])
    argv = ["-f", "lavfi", "-i",
            f"testsrc2=duration={dur:.3f}:size={w}x{h}:rate={fps}",
            "-f", "lavfi", "-i", f"sine=duration={dur:.3f}:sample_rate={ar}",
            "-map", "0:v", "-map", "1:a",
            "-c:v", VIDEO_CODECS[vcodec]["enc"], "-pix_fmt", "yuv420p",
            "-c:a", AUDIO_CODECS[acodec]["enc"], "-ar", str(ar), "-ac", "1",
            "-f", "avi", "-y", "__OUT__"]
    split = rng.choice(RECHUNK_SPLIT_POOL)
    junk_every = rng.randint(2, 8)
    junk_lo, junk_hi = 5, rng.choice([32, 64, 128])
    drop_idx1 = rng.random() < 0.5
    post = [{"op": "avi-rechunk", "split": split, "junk_every": junk_every,
             "junk_lo": junk_lo, "junk_hi": junk_hi,
             "drop_idx1": drop_idx1}]
    # movi payload estimate: mjpeg factor 0.25 (est_bytes convention) +
    # pcm_u8 at 1 byte/sample; each split-byte piece costs 8 header bytes
    # + 1 pad byte on disk (measured: 49 584 chunks from a 1.2 s file).
    payload_est = int(w * h * dur * 0.25) + int(ar * dur)
    params = {"class": "odd-chunk-avi", "container": "avi", "codec": vcodec,
              "audio_codec": acodec, "res_name": f"{w}x{h}", "width": w,
              "height": h, "fps": fps, "pix_fmt": "yuv420p",
              "duration": round(dur, 3), "audio_rate": ar,
              "audio_channels": 1,
              "features": {"rechunk_split": split, "junk_every": junk_every,
                           "drop_idx1": drop_idx1},
              "est_hint": int(payload_est * (24 / split)) + 65_536}
    return [argv], params, [], post


def build_hevc_in_avi(rng, ctx, family, forced_res=None, forced_feats=None):
    """The lie-class: an HEVC bitstream renamed to an AVI video tag
    (bait-and-switch family/codec metadata).  Valid RIFF, valid HEVC — the
    container family lies about what the stream is.  -tag:v is required;
    without it the muxer writes an empty tag (rawvideo guess — also probed)."""
    _, _codec = family
    if not ctx.caps.get("hevc_avi"):
        return None  # capability-probed: AVI muxer refuses HEVC tags here
    tag = rng.choice(HEVC_AVI_TAGS)
    w = rng.choice([96, 128, 160, 192])
    h = int(w * 0.75)
    fps = rng.randint(10, 24)
    dur = rng.uniform(0.8, 2.0)
    argv = ["-f", "lavfi", "-i",
            f"testsrc2=duration={dur:.3f}:size={w}x{h}:rate={fps}"]
    have_audio = rng.random() < 0.4
    if have_audio:
        argv += ["-f", "lavfi", "-i",
                 f"sine=duration={dur:.3f}:sample_rate=8000",
                 "-map", "0:v", "-map", "1:a"]
    argv += ["-c:v", "libx265", "-preset", "ultrafast", "-pix_fmt",
             "yuv420p", "-tag:v", tag]
    if have_audio:
        argv += ["-c:a", "pcm_u8", "-ar", "8000"]
    argv += ["-f", "avi", "-y", "__OUT__"]
    params = {"class": "hevc-in-avi", "container": "avi", "codec": "h265",
              "res_name": f"{w}x{h}", "width": w, "height": h, "fps": fps,
              "pix_fmt": "yuv420p", "duration": round(dur, 3),
              "features": {"fourcc_lie": tag}}
    if have_audio:
        params["audio_codec"] = "pcm_u8"
        params["audio_rate"] = 8000
        params["audio_channels"] = 1
    return [argv], params, []


BUILDERS = {
    "video": build_video, "av": build_av, "audio": build_audio,
    "multi-av": build_multi_av, "dynres-video": build_dynres,
    "raw-video": build_raw, "fmp4-video": build_fmp4, "hls-av": build_hls,
    "cover-audio": build_cover, "multivideo-av": build_multivideo,
    "drift-av": build_drift, "meta-bomb-av": build_bomb,
    # ---- v4 escalation builders (W5-16) ----
    "odd-dims": build_odd_dims, "rot-display": build_rot_display,
    "maxtracks-avi": build_maxtracks_avi, "bframe-storm": build_bframe_storm,
    "packed-bs": build_packed_bs, "param-midchange": build_param_midchange,
    "vbv-burst": build_vbv_burst, "fps-extremes": build_fps_extremes,
    "sr-chaos": build_sr_chaos, "chapter-bomb": build_chapter_bomb,
    "zeroframe-stream": build_zeroframe_stream, "still-24h": build_still_24h,
    "riff-size-lie": build_riff_size_lie, "odd-chunk-avi": build_odd_chunk_avi,
    "hevc-in-avi": build_hevc_in_avi,
}
