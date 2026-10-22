"""corpus_vocab.py — the randomized feature universe for Lumen's adversarial
corpus generator (v3, industrial grade).

Nothing in this file is a "decision" — it is the BOUNDS of what is drawable.
Every concrete value (resolution, fps, gop structure, bitrate, SAR, subtitle
text, chapter boundaries, VFR expression, ...) is drawn per-file by the
generator's RNG.  Capability-dependent features (10-bit encoders, HLS-fMP4,
rotation metadata, ...) are probed against the real ffmpeg at run start and
unsupported draws are recorded in the honest-rejects ledger instead of being
silently hardcoded away.

v3 additions over v2 (user directive 2026-10-23, "industrial grade"):
  * VFR / timestamp-chaos expression factory
  * anamorphic SAR pool + display rotation pool
  * multi-track audio / subtitles / chapters vocabulary
  * dynamic-resolution segment ladders
  * HLS + fMP4 packaging vocabulary
  * attached-picture (cover art) vocabulary
  * long-duration drift probes (DAC/clock drift measurement)
  * metadata bombs (bounded, 1-64 KiB tag values)
  * extended corruption + garbage catalogs
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# Resolution ladder.  Portraits included — the mobile world is portrait-first.
# ---------------------------------------------------------------------------
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
    ("portrait360", 360, 640), ("portrait720", 720, 1280),
    ("portrait1080", 1080, 1920), ("square512", 512, 512),
]
RES_WEIGHTS = {
    "tiny16": 5, "tiny32": 5, "tiny64": 4, "qqvga": 3, "qvga": 4, "qcif": 4,
    "wqvga": 3, "vga": 5, "cif": 4, "w360p": 3, "ntsc": 3, "pal": 3,
    "vga4x3": 3, "hvga": 4, "svga": 2, "w480p": 3, "qhdp": 2, "xga": 2,
    "hd720": 4, "sxga": 1, "hdp": 1, "fhd": 2, "dci2k": 1, "qhd": 1,
    "uhd4k": 1,
    "portrait360": 4, "portrait720": 2, "portrait1080": 1, "square512": 2,
}
RES_BY_NAME = {r[0]: (r[1], r[2]) for r in RESOLUTION_LADDER}

# Frame-rate universe: from slideshow (0.5 fps) through cinema/TV to 240 fps.
FPS_POOL = [0.5, 1, 2, 5, 8, 10, 12, 15, 23.976, 24, 25, 29.97, 30,
            48, 50, 59.94, 60, 90, 100, 120, 240]
FPS_WEIGHTS = [1, 1, 1, 1, 1, 1, 1, 2, 3, 4, 4, 3, 5, 1, 2, 2, 4, 1, 1, 1, 1]

VIDEO_SOURCES = ["testsrc2", "mandelbrot", "smptebars", "cellauto",
                 "rgbtestsrc", "gradients", "life", "testsrc"]
VIDEO_SOURCE_WEIGHTS = [24, 6, 10, 6, 8, 8, 4, 14]
NO_DURATION_SRCS = {"mandelbrot", "cellauto", "life", "testsrc"}
# allyuv/allrgb deliberately EXCLUDED: fixed-size sources (no size= option).
# Sources that are expensive to RENDER (fractals/CA) inflate the timeout:
SOURCE_COST = {"mandelbrot": 6.0, "cellauto": 2.0, "life": 2.0}
AUDIO_SOURCES = ["sine", "anoisesrc", "aevalsrc"]

# codec -> encoder, drawable pix_fmts, even-dims requirement, extra knobs.
# "hi10"/"hi422" pix entries are capability-probed at run start.
VIDEO_CODECS = {
    "mjpeg":      {"enc": "mjpeg",       "pix": ["yuv420p", "yuvj420p", "yuv422p", "yuvj422p", "yuv444p", "gray"], "even": False},
    "mpeg4":      {"enc": "mpeg4",       "pix": ["yuv420p"],                    "even": True},
    "mpeg2video": {"enc": "mpeg2video",  "pix": ["yuv420p", "yuv422p"],         "even": True},
    "huffyuv":    {"enc": "huffyuv",     "pix": ["yuv422p", "rgb24"],           "even": True},
    "ffv1":       {"enc": "ffv1",        "pix": ["yuv420p", "yuv444p", "yuv422p10le", "bgr0"], "even": True},
    "wmv1":       {"enc": "wmv1",        "pix": ["yuv420p"],                    "even": True},
    "wmv2":       {"enc": "wmv2",        "pix": ["yuv420p"],                    "even": True},
    "h264":       {"enc": "libx264",     "pix": ["yuv420p", "yuv444p", "yuv422p", "gray", "yuv420p10le"], "even": True, "preset": "ultrafast"},
    "h265":       {"enc": "libx265",     "pix": ["yuv420p", "yuv420p10le"],     "even": True, "preset": "ultrafast"},
    "vp8":        {"enc": "libvpx",      "pix": ["yuv420p"],                    "even": True},
    "vp9":        {"enc": "libvpx-vp9",  "pix": ["yuv420p", "yuv444p", "yuv420p10le"], "even": True, "deadline": "realtime"},
    "av1":        {"enc": "libsvtav1",   "pix": ["yuv420p", "yuv420p10le"],     "even": True, "preset": "8", "min": 128},
    "theora":     {"enc": "libtheora",   "pix": ["yuv420p"],                    "even": True},
    "flv1":       {"enc": "flv",         "pix": ["yuv420p"],                    "even": True},
    "prores":     {"enc": "prores_ks",   "pix": ["yuv422p10le", "yuv444p10le"], "even": True, "min": 128},
}
# pix_fmts that require a capability probe before they may be drawn.
CAP_PROBED_PIX = {"yuv420p10le": "x264_10bit", "yuv422p10le": "prores",
                  "yuv444p10le": "prores444", "yuv422p": "x264_422",
                  "yuv444p": "x264_444"}
# codecs whose encoders accept -bf / gop anarchy knobs
GOP_KNOBS = {"h264", "h265", "mpeg4", "mpeg2video", "vp8", "vp9"}

CONTAINERS = {
    "avi":  {"ext": "avi",  "fmt": "avi",       "codecs": ["mjpeg", "mpeg4", "mpeg2video", "huffyuv", "ffv1", "wmv1", "wmv2"]},
    "mp4":  {"ext": "mp4",  "fmt": "mp4",       "codecs": ["h264", "h265", "mpeg4", "prores"]},
    "mov":  {"ext": "mov",  "fmt": "mov",       "codecs": ["h264", "mpeg4", "mjpeg", "prores"]},
    "mkv":  {"ext": "mkv",  "fmt": "matroska",  "codecs": ["h264", "h265", "vp8", "vp9", "av1", "ffv1", "mjpeg", "prores"]},
    "webm": {"ext": "webm", "fmt": "webm",      "codecs": ["vp8", "vp9", "av1"]},
    "ts":   {"ext": "ts",   "fmt": "mpegts",    "codecs": ["h264", "mpeg2video"]},
    "flv":  {"ext": "flv",  "fmt": "flv",       "codecs": ["flv1", "h264"]},
    "ogv":  {"ext": "ogv",  "fmt": "ogg",       "codecs": ["theora"]},
}
# raw (containerless) elementary-stream targets: NO timestamps at all —
# the demuxer must synthesize a timeline from the bitstream alone.
RAW_VIDEO_TARGETS = {
    "h264": {"ext": "h264", "fmt": "h264", "codec": "h264"},
    "hevc": {"ext": "265",  "fmt": "hevc", "codec": "h265"},
}

AUDIO_CODECS = {
    "pcm_s16le": {"enc": "pcm_s16le",  "cont": ["wav"],                "rates": "std",  "ch": "all"},
    "pcm_s24le": {"enc": "pcm_s24le",  "cont": ["wav"],                "rates": "std",  "ch": "all"},
    "pcm_s32le": {"enc": "pcm_s32le",  "cont": ["wav"],                "rates": "std",  "ch": "all"},
    "pcm_u8":    {"enc": "pcm_u8",     "cont": ["wav"],                "rates": "std",  "ch": "all"},
    "pcm_f32le": {"enc": "pcm_f32le",  "cont": ["wav"],                "rates": "std",  "ch": "all"},
    "pcm_s16be": {"enc": "pcm_s16be",  "cont": ["wav"],                "rates": "std",  "ch": "all"},
    "flac":      {"enc": "flac",       "cont": ["flac", "wav", "mkv"], "rates": "std",  "ch": "all"},
    "alac":      {"enc": "alac",       "cont": ["mka", "mov"],         "rates": "std",  "ch": "all"},
    "mp3":       {"enc": "libmp3lame", "cont": ["mp3", "wav", "mkv"],  "rates": "mp3",  "ch": "two"},
    "aac":       {"enc": "aac",        "cont": ["m4a", "adts", "mp4"], "rates": "aac",  "ch": "six"},
    "opus":      {"enc": "libopus",    "cont": ["ogg", "opus", "webm"],"rates": "opus", "ch": "six"},
    "vorbis":    {"enc": "libvorbis",  "cont": ["ogg", "oga", "mkv"],  "rates": "std",  "ch": "two"},
    "ac3":       {"enc": "ac3",        "cont": ["ac3", "mkv"],         "rates": "std",  "ch": "six"},
    "wavpack":   {"enc": "wavpack",     "cont": ["wv"],                 "rates": "std",  "ch": "two"},
    "mp2":       {"enc": "libtwolame", "cont": ["mp2"],                "rates": "std",  "ch": "two"},
    "adpcm_ms":  {"enc": "adpcm_ms",   "cont": ["wav"],                "rates": "std",  "ch": "two"},
    "adpcm_ima": {"enc": "adpcm_ima_wav", "cont": ["wav"],             "rates": "std",  "ch": "two"},
}

AUDIO_CONTAINERS = {
    "wav": {"ext": "wav", "fmt": "wav"},     "flac": {"ext": "flac", "fmt": "flac"},
    "m4a": {"ext": "m4a", "fmt": "ipod"},    "adts": {"ext": "aac", "fmt": "adts"},
    "ogg": {"ext": "ogg", "fmt": "ogg"},     "oga": {"ext": "oga", "fmt": "ogg"},
    "opus": {"ext": "opus", "fmt": "opus"},  "mp3": {"ext": "mp3", "fmt": "mp3"},
    "ac3": {"ext": "ac3", "fmt": "ac3"},     "wv": {"ext": "wv", "fmt": "wv"},
    "mp2": {"ext": "mp2", "fmt": "mp2"},     "mka": {"ext": "mka", "fmt": "matroska"},
}
ANY_CONTAINERS = {**AUDIO_CONTAINERS, **CONTAINERS}

STD_RATES = [8000, 11025, 16000, 22050, 32000, 44100, 48000, 88200, 96000]
ODD_RATES = [5512, 37800, 44056, 47250, 176400, 384000]
MP3_RATES = [8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000]
AAC_RATES = [8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000, 88200, 96000]
OPUS_RATES = [48000, 24000, 16000, 12000, 8000]

# Non-square pixel geometries (num/den).  Real DVD/BD delivery values.
ANAMORPHIC_SAR = [
    ("16:15", 16, 15), ("32:27", 32, 27), ("12:11", 12, 11), ("10:11", 10, 11),
    ("40:33", 40, 33), ("59:54", 59, 54), ("64:45", 64, 45),
]
ROTATIONS = [90, 180, 270, -90, 45]

# ---------------------------------------------------------------------------
# VFR / timestamp chaos.  Expressions are factories: (rng, fps) -> filter arg.
# All forms keep PTS strictly monotonic (bounded jitter) except the two
# explicitly-marked chaos forms which muxers may legitimately reject —
# rejections are ledger data, not generator failures.
# ---------------------------------------------------------------------------
def vfr_expr(rng, fps: float) -> tuple[str, str]:
    """Return (name, setpts-expr).  Source cadence is fps; output is VFR."""
    period = 1.0 / fps
    mode = rng.choices(
        ["sinus-jitter", "staircase", "duty-gaps", "start-offset", "telecine-ish", "chaos-nonmono"],
        weights=[24, 20, 20, 14, 12, 10])[0]
    if mode == "sinus-jitter":
        amp = period * rng.uniform(0.05, 0.30)
        k = rng.randint(3, 29)
        return mode, f"(N*{period:.6f} + {amp:.6f}*abs(sin(N/{k})))/TB"
    if mode == "staircase":
        # cumulative +shifts after frame k1, k2: monotone, real stall bursts
        n1, n2 = sorted(rng.sample(range(5, 90), 2))
        s1, s2 = rng.uniform(0.2, 0.9), rng.uniform(0.2, 1.2)
        return mode, f"(PTS + {s1:.3f}*TB*gte(N,{n1}) + {s2:.3f}*TB*gte(N,{n2}))"
    if mode == "duty-gaps":
        # time-based select gate: bursty on/off with source PTS preserved
        cyc = rng.uniform(1.2, 4.0)
        keep = cyc * rng.uniform(0.35, 0.8)
        return mode + "|select", f"lt(mod(t,{cyc:.3f}),{keep:.3f})"
    if mode == "start-offset":
        off = rng.uniform(0.4, 4.0) if rng.random() < 0.8 else -rng.uniform(0.05, 0.25)
        return mode, f"(PTS+{off:.3f}/TB)"
    if mode == "telecine-ish":
        # alternating long/short frames (2:3-ish cadence wobble)
        a = period * rng.uniform(0.55, 0.8)
        b = period * rng.uniform(1.2, 1.6)
        return mode, f"(N/2*({a:.6f}+{b:.6f}) + mod(N,2)*{b:.6f})/TB"
    # chaos-nonmono: jitter larger than the frame period — muxers may reject
    amp = period * rng.uniform(0.6, 1.4)
    k = rng.randint(2, 9)
    return mode, f"(N*{period:.6f} + {amp:.6f}*sin(N/{k}))/TB"


# ---------------------------------------------------------------------------
# GOP anarchy: intra-only .. 999-frame GOPs, b-frame pyramids.
# ---------------------------------------------------------------------------
def draw_gop(rng, codec: str, fps: float) -> dict:
    g = rng.choices([1, rng.randint(2, 5), int(fps), int(fps * 2), rng.randint(250, 999)],
                    weights=[14, 20, 30, 24, 12])[0]
    out = {"g": max(1, min(g, 999))}
    if codec in GOP_KNOBS:
        # x265: "Lookahead depth must be greater than the max consecutive
        # bframe count" — bf > 4 with default lookahead fails to open the
        # encoder (measured); keep h265 conservative.
        bf_pool = [0, 1, 2, 3, 4] if codec == "h265" else [0, 1, 2, 3, 8]
        bf = rng.choices(bf_pool, weights=[30, 20, 30, 12, 8][:len(bf_pool)])[0]
        out["bf"] = bf
        if codec in ("h264", "h265") and bf >= 2 and rng.random() < 0.4:
            out["bpyramid"] = True
        if codec in ("h264", "h265") and rng.random() < 0.15:
            out["opengop"] = True
    return out


# ---------------------------------------------------------------------------
# Subtitles: adversarial content pool + per-container codec map.
# ---------------------------------------------------------------------------
SUBTITLE_CODEC_FOR = {
    "mkv": ["srt", "ass", "ssa"], "mp4": ["mov_text"], "mov": ["mov_text"],
    "webm": ["webvtt"], "ts": ["dvbsub"], "avi": [],
}
SUB_TEXT_POOL = [
    "plain ascii line", "unicode: 字幕測試 검은색", "rtl: مرحبا بالعالم",
    "emoji storm: \U0001F3AC\U0001F525\U0001F4F8\U0001F3A7", "quotes \"both\" kinds 'here'",
    "tags <b>bold</b> <i>it</i>", "long-line " + "x" * 400,
    "overlapping cue A", "overlapping cue B", "--> arrow inside text",
    "line\nbreak\ninside cue", "tab\there", "  leading and trailing spaces  ",
    "null\x00byte in text", "control\x01chars\x02embedded",
    "mixed 漢字 with english WORDS and цифры",
]
ASS_FONTS = ["Arial", "DejaVu Sans", "Noto Sans SC", "Liberation Serif", "Comic Sans"]

# ---------------------------------------------------------------------------
# Corruption operators (dirty pass, applied to a COPY — never in place) and
# garbage archetypes (standalone non-files).
# ---------------------------------------------------------------------------
CORRUPTION_OPS = [
    "truncate", "zero-mid", "flip-bytes", "smash-head", "tail-garbage",
    "cut-mid-chunk", "lie-ext", "swap-halves", "dup-range", "splice-other",
    "head-size-lie", "bit-drift-region",
]
GARBAGE_ARCHETYPES = [
    "empty", "one-byte", "riff-magic", "avi-magic-head", "ftyp-magic",
    "ebml-magic", "random-bytes", "text", "all-ff", "all-zero", "id3-junk",
    "moov-only", "mdat-only", "ftyp-only", "id3-size-lie", "m3u8-garbage",
    "ebml-size-lie", "gif-magic", "png-magic", "jpeg-magic", "zip-magic",
    "html-text",
]

# ---------------------------------------------------------------------------
# Class weights (v3) — avi remains the today-testable surface; new industrial
# classes carry the adversarial load.  "kind" maps 1:1 to a builder.
# ---------------------------------------------------------------------------
CLASS_WEIGHTS = [
    ("avi-video", 18), ("avi-av", 12), ("audio-only", 12), ("mp4-video", 7),
    ("mkv-video", 7), ("vfr-av", 6), ("multi-av", 5), ("muxed-av", 5),
    ("ts-video", 3), ("flv-video", 2), ("webm-video", 3), ("mov-video", 2),
    ("ogv-video", 1), ("dynres-video", 3), ("raw-video", 3), ("fmp4-video", 3),
    ("hls-av", 2), ("cover-audio", 2), ("multivideo-av", 2), ("drift-av", 2),
    ("meta-bomb-av", 1),
]

# Every (class, family[, forced_res, forced_features]) pin below MUST appear
# at least once per corpus — the coverage floor under the randomness.
STRATIFICATION = [
    ("avi-video", ("avi", "mjpeg")), ("avi-video", ("avi", "mpeg4")),
    ("avi-video", ("avi", "mpeg2video")), ("avi-video", ("avi", "huffyuv")),
    ("avi-video", ("avi", "ffv1")), ("avi-video", ("avi", "wmv1")),
    ("avi-video", ("avi", "wmv2")),
    ("avi-video", ("avi", "mjpeg"), "ntsc", {"sar": "10:11"}),
    ("avi-av", ("avi", "mjpeg+pcm_s16le")), ("avi-av", ("avi", "mpeg4+pcm_u8")),
    ("avi-av", ("avi", "mjpeg+pcm_s24le")), ("avi-av", ("avi", "mjpeg+pcm_f32le")),
    ("mp4-video", ("mp4", "h264")), ("mp4-video", ("mp4", "h265")),
    ("mp4-video", ("mp4", "prores")),
    ("mkv-video", ("mkv", "vp9")), ("mkv-video", ("mkv", "av1")),
    ("mkv-video", ("mkv", "ffv1")),
    ("webm-video", ("webm", "vp8")),
    ("ts-video", ("ts", "h264")),
    ("flv-video", ("flv", "flv1")),
    ("ogv-video", ("ogv", "theora")),
    ("mov-video", ("mov", "mjpeg"), None, {"rotate": 90}),
    ("audio-only", ("wav", "pcm_s16le")), ("audio-only", ("wav", "pcm_s24le")),
    ("audio-only", ("wav", "pcm_u8")), ("audio-only", ("wav", "pcm_s32le")),
    ("audio-only", ("flac", "flac")), ("audio-only", ("ogg", "opus")),
    ("audio-only", ("ogg", "vorbis")), ("audio-only", ("mp3", "mp3")),
    ("audio-only", ("m4a", "aac")), ("audio-only", ("adts", "aac")),
    ("audio-only", ("ac3", "ac3")), ("audio-only", ("mka", "alac")),
    ("audio-only", ("wav", "adpcm_ms")), ("audio-only", ("wav", "adpcm_ima")),
    ("muxed-av", ("mp4", "h264+aac")), ("muxed-av", ("mkv", "h264+mp3")),
    ("vfr-av", ("mp4", "h264+aac")), ("vfr-av", ("mkv", "vp9+opus")),
    ("vfr-av", ("mov", "mjpeg+pcm_s16le")),
    ("multi-av", ("mkv", "h264+aac+ac3")), ("multi-av", ("mp4", "h264+aac+mp3")),
    ("multi-av", ("mkv", "h264+aac+subs+chapters")),
    ("dynres-video", ("mkv", "h264")), ("dynres-video", ("ts", "h264")),
    ("raw-video", ("raw", "h264")), ("raw-video", ("raw", "hevc")),
    ("fmp4-video", ("mp4", "h264")), ("fmp4-video", ("mp4", "h265")),
    ("hls-av", ("hls", "h264+aac")), ("hls-av", ("hlsf", "h264+aac")),
    ("cover-audio", ("mp3", "mp3+mjpeg")), ("cover-audio", ("flac", "flac+mjpeg")),
    ("cover-audio", ("m4a", "aac+mjpeg")),
    ("multivideo-av", ("avi", "mpeg4+mjpeg+pcm_s16le")),
    ("multivideo-av", ("mkv", "h264+vp9+aac")),
    ("drift-av", ("mkv", "h264+aac")), ("drift-av", ("mp4", "h264+aac")),
    ("meta-bomb-av", ("mkv", "h264+aac")), ("meta-bomb-av", ("mp4", "h264+aac")),
]
MANDATORY_RESOLUTIONS = ["tiny16", "qcif", "vga", "ntsc", "hd720", "fhd",
                         "uhd4k", "portrait720", "square512"]

# ---------------------------------------------------------------------------
# Presets — bundles of (count, budget_mb, jobs, max_res, max_dur, heavy_ok)
# ---------------------------------------------------------------------------
PRESETS = {
    "smoke":    {"count": 80,  "budget_mb": 400,  "max_res": "hd720",  "max_dur": 4.0,  "heavy": False},
    "standard": {"count": 420, "budget_mb": 1500, "max_res": "uhd4k",  "max_dur": 14.0, "heavy": True},
    "big":      {"count": 650, "budget_mb": 2800, "max_res": "uhd4k",  "max_dur": 16.0, "heavy": True},
    "insane":   {"count": 1000, "budget_mb": 6000, "max_res": "uhd4k",  "max_dur": 20.0, "heavy": True},
}
RES_ORDER = {name: i for i, (name, _, _) in enumerate(
    sorted(RESOLUTION_LADDER, key=lambda r: r[1] * r[2]))}

# Multi-track / drift / bomb bounds (all randomizable within these caps).
MAX_AUDIO_TRACKS = 3
MAX_SUB_TRACKS = 2
MAX_CHAPTERS = 12
DRIFT_DUR_RANGE = (45.0, 110.0)
BOMB_TAG_MAX = 65536
