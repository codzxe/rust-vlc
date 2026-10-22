#!/usr/bin/env python3
"""Lumen randomized-corpus harness driver — the judge and reporter
(ED-FIX-5; user directive 2026-10-22: "test with them ... which tell us
real numbers").

Pipeline (all ground truth is EXTERNAL — ffprobe/ffmpeg, never Lumen's own
output):
  1. Read the corpus manifest (gen_corpus.py's ledger).
  2. ffprobe every file  -> per-file truth (format, duration, streams,
     packet counts). Unparseable files (garbage/heavy corruption) are
     truth="-" — that IS the oracle answer for them.
  3. Extract ffmpeg reference decodes for the mjpeg/pcm cases (rawvideo
     yuv420p / raw sample streams). Odd-dimension mjpeg files whose ref
     frame stride disagrees with the I420 ceil-half layout are recorded
     and excluded from PSNR comparison (honest note, not silent drop).
  4. Drive the three env-gated Rust corpus suites (release profile):
     probe / avi-demux / codecs(mjpeg+pcm). Any panic or non-zero exit
     is a FAILED contract row in the report. `--skip-suites` omits this
     step entirely (no cargo, no build lock): when persisted result TSVs
     from a previous full run exist they are re-used for the judgement
     sections — labeled as persisted rows, never as a fresh measurement;
     with no persisted TSVs the sections stay empty.
  5. Record the per-file DECODED rows (W1-04): oracle decoded format and
     dimensions (ffprobe -show_streams: pix_fmt, width/height, sample_fmt,
     channels, sample_rate) side by side with the Lumen suites' decoded
     output format/dims, in corpus/_harness/decoded-rows.tsv. The Rust
     suites do not emit the Lumen cells yet — they are marked
     "not-emitted (suite gap)" (never fabricated) until the registered
     suite-emission follow-up lands; the driver consumes the out_* result
     columns as soon as they exist.
  6. Aggregate everything into measured-number tables and write
     docs/corpus/report-<stamp>.md (+ corpus/REPORT.md copy; a
     --skip-suites run writes *-skip-suites names instead, so an
     oracle-side validation run never masquerades as the canonical
     measurement report).

Metric changes (W2-08 — judge-side only, no engine code touched):
  * Duration (demux section), W1-03's prescription: span_ms =
    (last_v_pts_ns − first_v_pts_ns + v_frame_period_ns) / 1e6 with
    v_frame_period_ns = 1e9·strh.scale/strh.rate, parsed from the
    file's own first `vids` strh. The oracle's duration includes the
    last frame's display period; the previous span-only form
    under-reported by exactly one frame period (−40.0/−16.7/−8.3 ms
    p50/p95/max on the 2026-10-22 corpus; corrected → 0.0/0.0/0.0
    measured on that same corpus). THIS ROW'S NUMBERS CHANGE — that
    is the point.
  * MJPEG per-class table, W1-01's prescription: clean and dirty
    corpus classes aggregate into separate rows (manifest-class join),
    so dirty concealment can never pollute clean-class PSNR minima;
    fidelity columns (PSNR min/mean, max abs, exact bytes) exclude
    files with any concealed frame (file-level — per-frame exclusion
    needs suite-emitted clean-only PSNR columns, registered
    follow-up); demux terminal/recovered are recorded per case
    (mjpeg-verdicts.tsv) so packet-count deltas are attributable.
  * --skip-suites re-uses persisted suite-result TSVs when present
    (see step 4). All other sections keep their shape; new
    columns/sections are additive.

Usage:
  run_harness.py [--corpus DIR] [--ffprobe BIN] [--ffmpeg BIN]
                 [--skip-gen-check] [--skip-suites] [--keep-refs]
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import statistics
import struct
import subprocess
import sys
import time
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent

# ffprobe format_name -> Lumen's FormatId vocabulary (what the registry
# is expected to answer; "mp4" covers the mov-family since Lumen's MP4
# signature reports the mp4 id).
LUMEN_FORMAT_OF = {
    "avi": "avi",
    "mov,mp4,m4a,3gp,3g2,mj2": "mp4",
}

PCM_REF_FMT = {
    "pcm_s16le": "s16le", "pcm_s24le": "s24le", "pcm_u8": "u8",
    "pcm_f32le": "f32le", "pcm_s16be": "s16be",
}

# --- decoded-rows vocabulary (W1-04) ------------------------------------
# The Rust suites measure counts/PSNR/bytes but do not yet emit the decoded
# output format/dims. These markers keep that gap EXPLICIT in the rows and
# the report — a missing measurement is recorded as missing, never guessed.
GAP = "not-emitted (suite gap)"
NO_DECODER = "no-decoder (codec out of scope)"
NO_PATH = "no-path (container unsupported)"

# Lumen DecodedFormat variant (snake_case, as the suite follow-up will emit
# it) -> pix sampling family, comparable with the oracle classifier below.
# i420 and the oracle's yuvj420p are the same family — the j suffix is a
# full-range flag, not a layout change.
LUMEN_PIX_FAMILY = {"i420": "yuv420", "rgba8": "rgb"}


def pix_family(pix_fmt: str) -> str:
    """ffprobe pix_fmt -> sampling family (yuv420/yuv422/yuv444/gray/rgb)."""
    if not pix_fmt or pix_fmt == "-":
        return "-"
    q = pix_fmt.lower()
    if q.startswith("gray"):
        return "gray"
    if "420" in q:
        return "yuv420"
    if "422" in q:
        return "yuv422"
    if "444" in q:
        return "yuv444"
    if q.startswith(("rgb", "bgr", "gbr", "pal", "y8", "ya8")):
        return "rgb"
    return q


def sample_family(fmt: str) -> str:
    """ffprobe sample_fmt OR a pcm_* descriptor -> sample family.

    ffprobe reports 24-bit PCM as the s32-padded layout — same sample
    family as s24le, not a mismatch."""
    if not fmt or fmt == "-":
        return "-"
    q = fmt.lower().removeprefix("pcm_")
    if q in ("s16", "s16le", "s16be"):
        return "s16"
    if q in ("s24", "s24le", "s32", "s32le"):
        return "s24"
    if q in ("u8", "u8p"):
        return "u8"
    if q in ("flt", "f32", "f32le"):
        return "f32"
    return q


def ref_frame_cap(entry: dict) -> int | None:
    """Per-file reference frame cap — enough frames for stable PSNR
    statistics at every resolution, bounded disk (a full 4K rawvideo ref
    is ~12 MB/frame; uncapped refs blew the disk at 6.6 GB). The Rust
    RefReader stops comparing at ref EOF and keeps decoding, so a capped
    ref never misaligns frames."""
    w = entry["params"].get("width") or 0
    h = entry["params"].get("height") or 0
    px = w * h
    dirty = entry["class"] in ("dirty", "dirty-lie-ext")
    if px >= 3_000_000:      # 4K/DCI+
        cap = 16
    elif px >= 1_000_000:    # QHD
        cap = 24
    elif px >= 400_000:      # FHD-ish
        cap = 45
    else:
        return None           # all frames (small anyway)
    return min(cap, 16) if dirty else cap


def sh(cmd: list[str], timeout: int = 300) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)


def ffprobe_json(path: Path, ffprobe: str) -> dict | None:
    p = sh([ffprobe, "-v", "error", "-print_format", "json", "-show_format", "-show_streams", str(path)])
    if p.returncode != 0:
        return None
    try:
        return json.loads(p.stdout)
    except json.JSONDecodeError:
        return None


def ffprobe_packets(path: Path, ffprobe: str, sel: str) -> int | None:
    p = sh([ffprobe, "-v", "error", "-select_streams", sel, "-count_packets",
            "-show_entries", "stream=nb_read_packets", "-of", "csv=p=0", str(path)])
    if p.returncode != 0:
        return None
    out = p.stdout.strip().splitlines()
    if not out or not out[0].strip():
        return None
    try:
        return int(out[0].strip())
    except ValueError:
        return None


def avi_video_strh_rate(path: Path, prefix_bytes: int = 1 << 18) -> tuple[int, int] | None:
    """First `vids` strh's (scale, rate) of an AVI file, or None.

    Bounded RIFF walk over a header prefix (the strl headers sit at the
    file start, before movi): the video frame period 1e9·scale/rate is
    the container's own declared ladder — the same one the AVI demuxer
    stamps PTS with — so the W1-03-corrected duration span uses the
    file's truth rather than a Lumen-reported value. Dirty-world safe:
    reads at most `prefix_bytes`, stops at the declared RIFF end, honors
    word alignment, and a garbage size simply ends the walk (-> None),
    never loops.
    """
    try:
        with path.open("rb") as f:
            buf = f.read(prefix_bytes)
    except OSError:
        return None
    if len(buf) < 12 or buf[0:4] != b"RIFF" or buf[8:12] != b"AVI ":
        return None
    end = min(len(buf), 8 + struct.unpack_from("<I", buf, 4)[0])

    def chunks(pos: int, stop: int):
        """Yield (chunk_id, body_offset, size) over a bounded chunk run."""
        while pos + 8 <= stop:
            cid = buf[pos:pos + 4]
            size = struct.unpack_from("<I", buf, pos + 4)[0]
            yield cid, pos + 8, size
            pos = pos + 8 + size + (size & 1)  # RIFF chunks are word-aligned

    for cid, body, size in chunks(12, end):                          # RIFF level
        if cid != b"LIST" or buf[body:body + 4] != b"hdrl":
            continue
        for c2, b2, s2 in chunks(body + 4, min(end, body + size)):   # hdrl level
            if c2 != b"LIST" or buf[b2:b2 + 4] != b"strl":
                continue
            for c3, b3, s3 in chunks(b2 + 4, min(end, b2 + s2)):     # strl level
                if c3 == b"strh" and s3 >= 28 and b3 + 28 <= end and buf[b3:b3 + 4] == b"vids":
                    scale, rate = struct.unpack_from("<II", buf, b3 + 20)
                    if scale > 0 and rate > 0:
                        return scale, rate
                    return None  # first vids strh is corrupt — no honest period
    return None


def percentile(values: list[float], q: float) -> float:
    if not values:
        return float("nan")
    vs = sorted(values)
    idx = min(len(vs) - 1, max(0, int(round(q * (len(vs) - 1)))))
    return vs[idx]


def fmt_ms(x: float) -> str:
    return f"{x:.1f}" if x == x else "-"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", default=str(REPO / "corpus"))
    ap.add_argument("--ffprobe", default="ffprobe")
    ap.add_argument("--ffmpeg", default="ffmpeg")
    ap.add_argument("--keep-refs", action="store_true", help="keep _harness/ refs after the run")
    ap.add_argument("--skip-gen-check", action="store_true",
                    help="compatibility flag: skip corpus freshness verification (this driver only "
                         "requires manifest.json to exist; nothing else is verified today)")
    ap.add_argument("--skip-suites", action="store_true",
                    help="do NOT drive the Rust suites (no cargo, no build lock): oracle-side "
                         "validation only — persisted result TSVs from a previous full run, when "
                         "present, are re-used for the judgement sections (labeled as persisted, "
                         "never as a fresh measurement); the report is written under a "
                         "*-skip-suites name so it cannot pose as a measurement")
    args = ap.parse_args()

    corpus = Path(args.corpus).resolve()
    manifest_path = corpus / "manifest.json"
    if not manifest_path.is_file():
        print(f"no corpus manifest at {manifest_path} — run gen_corpus.py first", file=sys.stderr)
        return 2
    manifest = json.loads(manifest_path.read_text())
    harness_dir = corpus / "_harness"
    refs_dir = harness_dir / "refs"
    harness_dir.mkdir(parents=True, exist_ok=True)
    if not args.skip_suites:  # refs exist only to feed the suites
        refs_dir.mkdir(parents=True, exist_ok=True)

    t0 = time.time()
    files = manifest["files"]
    print(f"[h[harness] corpus: {len(files)} files, seed {manifest.get('seed')}")

    # ------------------------------------------------------------------
    # 1. ffprobe truth for every file (the external oracle).
    #    Checkpointed: a reaper kill mid-run resumes from truth.json.
    #    Since W1-04 each record also carries the DECODED stream facts —
    #    video pix_fmt/width/height, audio sample_fmt/channels/sample_rate
    #    (ffprobe -show_streams) — the expected side of the decoded-fidelity
    #    comparison. Checkpoints from older runs lack those keys and are
    #    upgraded file-by-file; entries missing from a partial checkpoint
    #    are gap-filled here instead of being silently dropped.
    # ------------------------------------------------------------------
    def probe_truth(entry: dict, path: Path) -> dict:
        j = ffprobe_json(path, args.ffprobe)
        rec: dict = {"format": "-", "duration_ms": "-", "streams": "-", "v_pkts": "-", "a_pkts": "-",
                     "v_codec": "-", "v_pix_fmt": "-", "v_w": "-", "v_h": "-",
                     "a_codec": "-", "a_sample_fmt": "-", "a_ch": "-", "a_rate": "-"}
        if j is not None:
            fmt = j.get("format", {})
            rec["format"] = fmt.get("format_name", "-") or "-"
            d = fmt.get("duration")
            rec["duration_ms"] = f"{float(d) * 1000:.1f}" if d else "-"
            rec["streams"] = str(len(j.get("streams", [])))
            for s in j.get("streams", []):
                if s.get("codec_type") == "video" and rec["v_codec"] == "-":
                    rec["v_codec"] = s.get("codec_name") or "-"
                    rec["v_pix_fmt"] = s.get("pix_fmt") or "-"
                    rec["v_w"] = str(s.get("width") or "-")
                    rec["v_h"] = str(s.get("height") or "-")
                elif s.get("codec_type") == "audio" and rec["a_codec"] == "-":
                    rec["a_codec"] = s.get("codec_name") or "-"
                    rec["a_sample_fmt"] = s.get("sample_fmt") or "-"
                    rec["a_ch"] = str(s.get("channels") or "-")
                    rec["a_rate"] = str(s.get("sample_rate") or "-")
            if entry["family"][0] == "avi":
                rec["v_pkts"] = str(ffprobe_packets(path, args.ffprobe, "v:0") or "-")
                rec["a_pkts"] = str(ffprobe_packets(path, args.ffprobe, "a:0") or "-")
        return rec

    truth_path = harness_dir / "truth.json"
    truth: dict[str, dict] = {}
    if truth_path.is_file():
        try:
            truth = json.loads(truth_path.read_text())
        except json.JSONDecodeError:
            truth = {}  # damaged checkpoint — rebuild honestly, never trust it
    n_probed = 0
    for i, entry in enumerate(files):
        path = corpus / entry["rel"]
        if not path.is_file():
            print(f"[h[harness] WARNING missing file {entry['rel']}")
            truth.pop(entry["rel"], None)
            continue
        rec = truth.get(entry["rel"])
        if rec is not None and "v_pix_fmt" in rec and "a_sample_fmt" in rec:
            continue  # current-schema checkpoint hit
        truth[entry["rel"]] = probe_truth(entry, path)
        n_probed += 1
        if n_probed % 50 == 0:
            print(f"[h[harness] ffprobe (schema upgrade / gap fill) {n_probed} probed, "
                  f"{i + 1}/{len(files)} scanned", flush=True)
            truth_path.write_text(json.dumps(truth))  # incremental checkpoint
    if n_probed:
        truth_path.write_text(json.dumps(truth))
    print(f"[h[harness] truth: {len(truth)} files on record "
          f"({n_probed} probed this run, {len(truth) - n_probed} from checkpoint)")

    # ------------------------------------------------------------------
    # 2. Case lists.
    # ------------------------------------------------------------------
    probe_cases = harness_dir / "probe-cases.tsv"
    with probe_cases.open("w") as f:
        f.write("rel\ttruth_format\tclass\n")
        for entry in files:
            if entry["rel"] in truth:
                f.write(f"{entry['rel']}\t{truth[entry['rel']]['format']}\t{entry['class']}\n")

    avi_entries = [e for e in files
                   if e["family"][0] == "avi" and e["rel"] in truth
                   and e["class"] in ("video", "av", "dirty", "dirty-lie-ext", "avi-video", "avi-av")]
    avi_cases = harness_dir / "avi-cases.tsv"
    with avi_cases.open("w") as f:
        f.write("rel\tclass\ttruth_duration_ms\ttruth_streams\ttruth_v_packets\ttruth_a_packets\n")
        for e in avi_entries:
            t = truth[e["rel"]]
            f.write(f"{e['rel']}\t{e['class']}\t{t['duration_ms']}\t{t['streams']}\t{t['v_pkts']}\t{t['a_pkts']}\n")

    # mjpeg + pcm cases: AVI + mjpeg video / pcm audio (clean AND dirty —
    # dirty measures concealment/error behavior against ffmpeg's own
    # recovery). Both need ffmpeg reference decodes, which exist only to
    # feed the Rust suites — skipped entirely under --skip-suites.
    mjpeg_rows: list[dict] = []
    pcm_rows: list[dict] = []
    if args.skip_suites:
        print("[h[harness] --skip-suites: ffmpeg reference decodes + mjpeg/pcm case lists "
              "not built (no suites to feed)")
    else:
        mjpeg_entries = [e for e in avi_entries
                         if e["params"].get("codec") == "mjpeg" or e["params"].get("codec", "").startswith("mjpeg")]
        mjpeg_cases = harness_dir / "mjpeg-cases.tsv"
        with mjpeg_cases.open("w") as f:
            f.write("rel\tref\twidth\theight\tframes\n")
            for i, e in enumerate(mjpeg_entries):
                w, h = e["params"].get("width"), e["params"].get("height")
                if not w or not h:
                    continue
                src = corpus / e["rel"]
                ref = refs_dir / f"mjpeg-{i:04d}.raw"
                cap = ref_frame_cap(e)
                if ref.is_file() and ref.stat().st_size > 0:
                    fb = w * h + 2 * ((w + 1) // 2) * ((h + 1) // 2)
                    size = ref.stat().st_size
                    frames, rem = divmod(size, fb)
                    if rem == 0 and frames > 0:
                        f.write(f"{e['rel']}\t_harness/refs/{ref.name}\t{w}\t{h}\t{frames}\n")
                        mjpeg_rows.append({"rel": e["rel"], "ref": f"_harness/refs/{ref.name}", "w": w, "h": h,
                                           "frames": frames, "res_name": e["params"].get("res_name", "?")})
                        continue
                extract = [args.ffmpeg, "-v", "error", "-i", str(src), "-map", "0:v:0",
                           "-c:v", "rawvideo", "-pix_fmt", "yuv420p", "-f", "rawvideo", "-y"]
                if cap is not None:
                    extract += ["-frames:v", str(cap)]
                extract.append(str(ref))
                p = sh(extract)
                if p.returncode != 0 or not ref.is_file() or ref.stat().st_size == 0:
                    mjpeg_rows.append({"rel": e["rel"], "note": "ref-extraction-failed"})
                    continue
                fb = w * h + 2 * ((w + 1) // 2) * ((h + 1) // 2)
                size = ref.stat().st_size
                frames, rem = divmod(size, fb)
                if rem != 0 or frames == 0:
                    mjpeg_rows.append({"rel": e["rel"], "note": f"ref-layout-mismatch:{size}not%{fb}"})
                    ref.unlink(missing_ok=True)
                    continue
                f.write(f"{e['rel']}\t_harness/refs/{ref.name}\t{w}\t{h}\t{frames}\n")
                mjpeg_rows.append({"rel": e["rel"], "ref": f"_harness/refs/{ref.name}", "w": w, "h": h,
                                   "frames": frames, "res_name": e["params"].get("res_name", "?")})

        pcm_entries = [e for e in avi_entries if e["params"].get("audio_codec", "").startswith("pcm")]
        pcm_cases = harness_dir / "pcm-cases.tsv"
        with pcm_cases.open("w") as f:
            f.write("rel\tref\tdescriptor\trate\tchannels\n")
            for i, e in enumerate(pcm_entries):
                desc = e["params"]["audio_codec"]
                if desc not in PCM_REF_FMT:
                    continue
                src = corpus / e["rel"]
                ref = refs_dir / f"pcm-{i:04d}.raw"
                if not (ref.is_file() and ref.stat().st_size > 0):
                    p = sh([args.ffmpeg, "-v", "error", "-i", str(src), "-map", "0:a:0",
                            "-f", PCM_REF_FMT[desc], "-y", str(ref)])
                    if p.returncode != 0 or not ref.is_file():
                        pcm_rows.append({"rel": e["rel"], "note": "ref-extraction-failed"})
                        continue
                f.write(f"{e['rel']}\t_harness/refs/{ref.name}\t{desc}\t{e['params']['audio_rate']}\t{e['params']['audio_channels']}\n")
                pcm_rows.append({"rel": e["rel"], "ref": f"_harness/refs/{ref.name}", "desc": desc,
                                 "rate": e["params"]["audio_rate"], "ch": e["params"]["audio_channels"]})

    print(f"[h[harness] cases: probe={len(files)} avi={len(avi_entries)} "
          f"mjpeg={len([r for r in mjpeg_rows if 'ref' in r])} pcm={len([r for r in pcm_rows if 'ref' in r])}"
          + (" (suites skipped)" if args.skip_suites else ""))

    # ------------------------------------------------------------------
    # 3. Drive the Rust suites (release).
    # ------------------------------------------------------------------
    def run_suite(crate: str, test: str, cases: Path, results_name: str, test_filter: str | None = None) -> tuple[int, Path]:
        results = harness_dir / results_name
        results.unlink(missing_ok=True)
        cargo = shutil.which("cargo") or str(Path.home() / ".cargo" / "bin" / "cargo")
        env = dict(os.environ)
        env.update({
            "LUMEN_CORPUS": str(corpus),
            "LUMEN_CORPUS_CASES": str(cases),
            "LUMEN_CORPUS_RESULTS": str(results),
            "CARGO_TARGET_DIR": os.environ.get("CARGO_TARGET_DIR", str(REPO / "target")),
            "PATH": env["PATH"] + os.pathsep + str(Path(cargo).parent),
        })
        cmd = [cargo, "test", "--release", "-p", crate, "--test", test, "--", "--ignored", "--nocapture"]
        if test_filter is not None:
            # One test per invocation: the codecs corpus binary carries two
            # tests that would otherwise run in parallel and interleave their
            # appends into the shared results TSV.
            cmd.append(test_filter)
        p = subprocess.run(cmd, cwd=REPO, env=env, capture_output=True, text=True, timeout=7200)
        (harness_dir / f"{results_name}.log").write_text(p.stdout[-20000:] + "\n=== STDERR ===\n" + p.stderr[-20000:])
        return p.returncode, results

    def read_tsv(path: Path) -> list[dict]:
        if not path.is_file():
            return []
        lines = path.read_text().splitlines()
        if len(lines) < 2:
            return []
        hdr = lines[0].split("\t")
        return [dict(zip(hdr, ln.split("\t"))) for ln in lines[1:] if ln.strip()]

    reused: list[str] = []
    man_rels = {e["rel"] for e in files}

    def reuse_results(name: str) -> list[dict]:
        """Persisted suite-result rows for --skip-suites validation runs.

        Only rows whose rel is in the CURRENT manifest are kept — a corpus
        regenerated since the last full run must not leak stale rows into
        the judgement. Every reuse is printed with the TSV's mtime so
        staleness is visible; the report labels these rows as persisted.
        """
        p = harness_dir / name
        if not p.is_file():
            return []
        rows = read_tsv(p)
        kept = [r for r in rows if r.get("rel") in man_rels]
        stale = len(rows) - len(kept)
        print(f"[h[harness] --skip-suites reuse: {name} -> {len(kept)} persisted rows "
              f"(mtime {time.strftime('%Y-%m-%d %H:%M', time.localtime(p.stat().st_mtime))})"
              + (f", {stale} stale rows dropped (absent from current manifest)" if stale else ""))
        if kept:
            reused.append(name)
        return kept

    if args.skip_suites:
        probe_res = reuse_results("probe-results.tsv")
        demux_res = reuse_results("demux-results.tsv")
        mjpeg_res = reuse_results("mjpeg-results.tsv")
        pcm_res = reuse_results("pcm-results.tsv")
        rc_probe = "reused" if probe_res else "skipped"
        rc_demux = "reused" if demux_res else "skipped"
        rc_mjpeg = "reused" if mjpeg_res else "skipped"
        rc_pcm = "reused" if pcm_res else "skipped"
        print("[h[harness] --skip-suites: Rust suites NOT driven (no cargo, no build lock)"
              + (f" — judgement re-aggregated from persisted: {', '.join(reused)}" if reused else ""))
    else:
        rc_probe, res_probe = run_suite("media-probe", "corpus", probe_cases, "probe-results.tsv")
        rc_demux, res_demux = run_suite("demux", "corpus", avi_cases, "demux-results.tsv")
        rc_mjpeg, res_mjpeg = run_suite("codecs", "corpus", mjpeg_cases, "mjpeg-results.tsv",
                                        "corpus_mjpeg_decode_measures_against_oracle")
        rc_pcm, res_pcm = run_suite("codecs", "corpus", pcm_cases, "pcm-results.tsv",
                                    "corpus_pcm_decode_is_byte_exact_or_recorded")

        probe_res = read_tsv(res_probe)
        demux_res = read_tsv(res_demux)
        mjpeg_res = read_tsv(res_mjpeg)
        pcm_res = read_tsv(res_pcm)

    # ------------------------------------------------------------------
    # 3.5 Decoded-rows ledger (W1-04): per-file oracle decoded format/dims
    #     side by side with the Lumen suites' decoded output. The suites
    #     do not emit the Lumen cells yet (registered follow-up) — the gap
    #     is recorded explicitly, never fabricated. When the suites grow
    #     the out_* result columns, this code picks them up unchanged.
    # ------------------------------------------------------------------
    mjpeg_by_rel = {r.get("rel"): r for r in mjpeg_res if r.get("rel")}
    pcm_by_rel = {r.get("rel"): r for r in pcm_res if r.get("rel")}

    # Generator-intent accessors — the manifest records the video codec
    # under `codec` only for video/av draws, and the audio codec under
    # `audio_codec` (av draws) or `codec` (audio-only draws). Reading the
    # wrong slot would misclassify rows, so the slots are resolved once.
    def video_codec_intent(params: dict) -> str:
        return (params.get("codec") or "") if params.get("class") in ("video", "av") else ""

    def audio_codec_intent(params: dict) -> str:
        if params.get("audio_codec"):
            return str(params["audio_codec"])
        return (params.get("codec") or "") if params.get("class") == "audio" else ""

    def lumen_video_cells(entry: dict, t: dict) -> tuple[str, str, str]:
        """(format, width, height) as measured by Lumen, or an explicit
        gap/scope marker. Video decoder scope today: AVI + mjpeg."""
        has_video = t.get("v_codec", "-") != "-" or bool(video_codec_intent(entry.get("params", {})))
        if not has_video:
            return ("", "", "")
        fam = entry.get("family") or []
        if not fam or fam[0] != "avi":
            return (NO_PATH, NO_PATH, NO_PATH)
        if not video_codec_intent(entry.get("params", {})).startswith("mjpeg"):
            return (NO_DECODER, NO_DECODER, NO_DECODER)
        row = mjpeg_by_rel.get(entry["rel"])
        if row and row.get("out_format"):
            return (row.get("out_format", "-"), row.get("out_w", "-"), row.get("out_h", "-"))
        return (GAP, GAP, GAP)

    def lumen_audio_cells(entry: dict, t: dict) -> tuple[str, str, str]:
        """(descriptor, rate, channels) as measured by Lumen, or an explicit
        gap/scope marker. Audio decoder scope today: AVI + pcm."""
        acodec = audio_codec_intent(entry.get("params", {}))
        has_audio = t.get("a_codec", "-") != "-" or bool(acodec)
        if not has_audio:
            return ("", "", "")
        fam = entry.get("family") or []
        if not fam or fam[0] != "avi":
            return (NO_PATH, NO_PATH, NO_PATH)
        if not acodec.startswith("pcm"):
            return (NO_DECODER, NO_DECODER, NO_DECODER)
        row = pcm_by_rel.get(entry["rel"])
        if row and row.get("out_desc"):
            return (row.get("out_desc", "-"), row.get("out_rate", "-"), row.get("out_ch", "-"))
        return (GAP, GAP, GAP)

    DEC_HDR = ["rel", "class", "v_codec", "v_pix_fmt", "v_w", "v_h",
               "a_codec", "a_sample_fmt", "a_ch", "a_rate",
               "lumen_v_format", "lumen_v_w", "lumen_v_h",
               "lumen_a_desc", "lumen_a_rate", "lumen_a_ch"]
    decoded_rows_path = harness_dir / "decoded-rows.tsv"
    n_gap_cells = 0
    with decoded_rows_path.open("w") as f:
        f.write("\t".join(DEC_HDR) + "\n")
        for entry in files:
            t = truth.get(entry["rel"])
            if t is None:
                t = {"v_codec": "-", "v_pix_fmt": "-", "v_w": "-", "v_h": "-",
                     "a_codec": "-", "a_sample_fmt": "-", "a_ch": "-", "a_rate": "-"}
            lv = lumen_video_cells(entry, t)
            la = lumen_audio_cells(entry, t)
            n_gap_cells += sum(1 for c in lv + la if c == GAP)
            f.write("\t".join([
                entry["rel"], entry["class"],
                t["v_codec"], t["v_pix_fmt"], t["v_w"], t["v_h"],
                t["a_codec"], t["a_sample_fmt"], t["a_ch"], t["a_rate"],
                *lv, la[0], la[1], la[2],
            ]) + "\n")
    n_vid_dim = sum(1 for e in files if truth.get(e["rel"], {}).get("v_w", "-") != "-")
    n_aud_fmt = sum(1 for e in files if truth.get(e["rel"], {}).get("a_sample_fmt", "-") != "-")
    print(f"[h[harness] decoded-rows: {len(files)} rows -> {decoded_rows_path} "
          f"({n_vid_dim} with oracle video dims, {n_aud_fmt} with oracle audio sample_fmt, "
          f"{n_gap_cells} suite-gap cells)")

    # ------------------------------------------------------------------
    # 4. Judge + measure.
    # ------------------------------------------------------------------
    report: list[str] = []
    stamp = time.strftime("%Y-%m-%d %H:%M")
    report.append(f"# Lumen randomized-corpus report — {stamp}")
    report.append("")
    report.append(f"- corpus seed: **{manifest.get('seed')}** (non-deterministic run; replay with "
                  f"`gen_corpus.py --replay corpus/manifest.json`)")
    report.append(f"- files: **{len(files)}** ({manifest.get('bytes_total', 0) / (1 << 20):.0f} MiB clean), "
                  f"rejected draws: {len(manifest.get('rejected_draws', []))}")
    classes: dict[str, int] = {}
    for e in files:
        classes[e["class"]] = classes.get(e["class"], 0) + 1
    report.append("- composition: " + ", ".join(f"{k}={v}" for k, v in sorted(classes.items())))
    if args.skip_suites:
        if reused:
            report.append("- mode: **--skip-suites (persisted-results reuse)** — the Rust suites "
                          "were NOT driven (no cargo, no build lock): the suite rows below are the "
                          "persisted results of the last full run, re-aggregated here to validate "
                          "this judge's logic — NOT a fresh measurement")
        else:
            report.append("- mode: **--skip-suites** — the Rust suites were NOT driven (oracle-side "
                          "validation run): the suite sections below carry no rows by construction, "
                          "and every Lumen-side decoded-format cell is a suite-gap row")
    report.append(f"- suite exit codes: probe={rc_probe} demux={rc_demux} mjpeg={rc_mjpeg} pcm={rc_pcm} "
                  + (("(**0 = no panic, no contract violation**)") if not args.skip_suites
                     else ("(reused from the persisted last-run TSVs — not driven this run)" if reused
                           else "(skipped — not driven)")))
    report.append("")

    def suite_note(rows: list[dict]) -> None:
        """Suite-section banner: reuse notice when persisted rows are being
        re-aggregated under --skip-suites, empty-marker when there are none,
        nothing when the suites were actually driven."""
        if not args.skip_suites:
            return
        if rows:
            report.append("_suites not driven (--skip-suites): the rows below are the persisted "
                          "results of the last full run — aggregation validation, not a fresh "
                          "measurement_")
        else:
            report.append("_suite skipped (--skip-suites): no rows; section intentionally empty_")
        report.append("")

    # -- probe judgement --------------------------------------------------
    report.append("## Probe (bounded format detection) vs ffprobe oracle")
    report.append("")
    suite_note(probe_res)
    false_pos: list[str] = []
    probe_lat: list[float] = []
    for row in probe_res:
        probe_lat.append(float(row.get("elapsed_us") or 0))
        verdict = row["verdict"]
        truth_fmt = row["truth_format"]
        cls = row["class"]
        if verdict == "probed":
            if truth_fmt not in LUMEN_FORMAT_OF:
                false_pos.append(f"{row['rel']} probed={row['format']} truth={truth_fmt or '-'} class={cls}")
    n_avi = sum(1 for r in probe_res if r["truth_format"] == "avi")
    n_mp4 = sum(1 for r in probe_res if r["truth_format"].startswith("mov,mp4"))
    ok_avi = sum(1 for r in probe_res if r["truth_format"] == "avi" and r["verdict"] == "probed" and r["format"] == "avi")
    ok_mp4 = sum(1 for r in probe_res if r["truth_format"].startswith("mov,mp4") and r["verdict"] == "probed" and r["format"] == "mp4")
    garbage_total = sum(1 for r in probe_res if r["class"].startswith("garbage"))
    garbage_rejected = sum(1 for r in probe_res if r["class"].startswith("garbage") and r["verdict"] == "rejected")
    report.append(f"| metric | value |")
    report.append(f"|---|---|")
    report.append(f"| AVI identification (ffprobe says avi) | **{ok_avi}/{n_avi}** |")
    report.append(f"| MP4 identification (ffprobe says mov/mp4) | **{ok_mp4}/{n_mp4}** |")
    report.append(f"| garbage rejection | **{garbage_rejected}/{garbage_total}** (must be 100%) |")
    report.append(f"| false positives (probed a non-AVI/MP4/garbage) | {len(false_pos)} |")
    if false_pos:
        for fp in false_pos[:10]:
            report.append(f"  - {fp}")
    if probe_lat:
        report.append(f"| probe latency p50/p95/max | {percentile(probe_lat, 0.5):.0f}µs / "
                      f"{percentile(probe_lat, 0.95):.0f}µs / {percentile(probe_lat, 1.0):.0f}µs |")
    report.append("")

    # -- demux judgement (scoped to the fourccs the demuxer can feed) -----
    # The AVI demuxer's supported set is mjpeg video + pcm audio (the
    # codecs that exist); files outside it are expected to be TYPED
    # refusals, judged on the refusal, not on conformance.
    report.append("## AVI demux walk vs ffprobe oracle (supported fourccs: mjpeg + pcm)")
    report.append("")
    suite_note(demux_res)

    def avi_supported(entry: dict) -> bool:
        fam = entry.get("family", [])
        if len(fam) < 2:
            return False
        parts = fam[1].split("+")
        v = parts[0]
        a = parts[1] if len(parts) > 1 else None
        return v == "mjpeg" and (a is None or a.startswith("pcm"))

    man_index = {e["rel"]: e for e in files}
    sup = [r for r in demux_res if avi_supported(man_index.get(r.get("rel", ""), {}))]
    refused = [r for r in demux_res if not avi_supported(man_index.get(r.get("rel", ""), {}))]
    clean = [r for r in sup if r["class"] in ("video", "av", "avi-video", "avi-av")]
    dirty = [r for r in demux_res if r["class"] in ("dirty", "dirty-lie-ext")]

    def num(r: dict, k: str) -> int | None:
        v = r.get(k, "")
        try:
            return int(v)
        except (ValueError, TypeError):
            return None

    tracks_match = streams_match = 0
    vp_match = ap_match = 0
    vp_diffs: list[float] = []
    dur_err: list[float] = []
    dur_no_strh = 0
    walked_eos = 0
    for r in clean:
        if num(r, "tracks") is not None and r.get("truth_streams") not in ("-", ""):
            if num(r, "tracks") == int(r["truth_streams"]):
                tracks_match += 1
            streams_match += 1
        if num(r, "v_pkts") is not None and r.get("truth_v_packets") not in ("-", ""):
            tv = int(r["truth_v_packets"])
            if num(r, "v_pkts") == tv:
                vp_match += 1
            vp_diffs.append((num(r, "v_pkts") or 0) - tv)
        if num(r, "a_pkts") is not None and r.get("truth_a_packets") not in ("-", ""):
            if num(r, "a_pkts") == int(r["truth_a_packets"]):
                ap_match += 1
        if r.get("terminal") == "eos":
            walked_eos += 1
        tdm = r.get("truth_duration_ms")
        try:
            first_ns = int(r.get("first_v_pts_ns", ""))
            last_ns = int(r.get("last_v_pts_ns", ""))
            if tdm not in ("-", "", None) and (num(r, "v_pkts") or 0) > 0:
                # W1-03's corrected span: the oracle's duration includes the
                # last frame's display period, so the measured span must add
                # one frame period. The period is the container's own ladder
                # (1e9·strh.scale/strh.rate, parsed from the file's first
                # `vids` strh) — the same ladder the demuxer stamps PTS
                # with. Rows without a parsable strh are counted, never
                # silently dropped; a walk with zero video packets has no
                # span to measure (typed refusal, judged elsewhere).
                sr = avi_video_strh_rate(corpus / r["rel"])
                if sr is None:
                    dur_no_strh += 1
                else:
                    span_ms = (last_ns - first_ns + 1e9 * sr[0] / sr[1]) / 1e6
                    dur_err.append(span_ms - float(tdm))
        except ValueError:
            pass
    report.append("| metric | value |")
    report.append("|---|---|")
    report.append(f"| supported clean AVIs walked to EOS | **{walked_eos}/{len(clean)}** |")
    report.append(f"| stream count matches oracle | {tracks_match}/{streams_match} |")
    report.append(f"| video packet count matches oracle | **{vp_match}/{len([r for r in clean if r.get('truth_v_packets') not in ('-', '')])}** |")
    report.append(f"| audio packet count matches oracle | {ap_match}/{len([r for r in clean if r.get('truth_a_packets') not in ('-', '')])} |")
    if vp_diffs:
        report.append(f"| v-packet delta vs oracle p50/p95/max | {percentile([float(x) for x in vp_diffs], 0.5):+.0f} / "
                      f"{percentile([float(x) for x in vp_diffs], 0.95):+.0f} / {percentile([float(x) for x in vp_diffs], 1.0):+.0f} packets |")
    if dur_err:
        report.append(f"| duration error (PTS span + frame period) p50/p95/max | {fmt_ms(percentile(dur_err, 0.5))} / "
                      f"{fmt_ms(percentile(dur_err, 0.95))} / {fmt_ms(percentile(dur_err, 1.0))} ms |")
    if dur_no_strh:
        report.append(f"| duration rows excluded (no parsable vids strh) | {dur_no_strh} |")
    dirty_typed = sum(1 for r in dirty if r.get("terminal") in ("eos", "error", "start-error"))
    dirty_recovered = sum(1 for r in dirty if num(r, "recovered") and num(r, "recovered") > 0)
    report.append(f"| dirty AVIs reached a typed terminal (no panic, no hang) | **{dirty_typed}/{len(dirty)}** |")
    report.append(f"| dirty AVIs that recovered damage mid-walk | {dirty_recovered} |")
    refused_ok = sum(1 for r in refused if r.get("terminal") == "start-error")
    report.append(f"| unsupported-fourcc AVIs answered with a typed refusal | {refused_ok}/{len(refused)} "
                  f"(no panic on any) |")
    report.append("")
    report.append("_Duration metric (W2-08, W1-03's prescription): span = last video PTS − first video PTS "
                  "+ one frame period, period = 1e9·strh.scale/strh.rate parsed from the file's own first "
                  "`vids` strh — the oracle's duration includes the last frame's display period, so the "
                  "previous span-only form under-reported by exactly one frame period (−40.0/−16.7/−8.3 ms "
                  "p50/p95/max on this corpus's 2026-10-22 13:33 run)._")
    report.append("")

    # -- mjpeg judgement ---------------------------------------------------
    report.append("## MJPEG decode vs ffmpeg oracle (real throughput + fidelity)")
    report.append("")
    suite_note(mjpeg_res)

    # W2-08 (W1-01's prescription): clean and dirty corpus classes are
    # aggregated separately — a dirty sibling's concealment (the documented
    # damage response on corrupt tails) must never pollute a clean class's
    # PSNR minima again. The class comes from the manifest join; the suite
    # result rows carry only rel.
    demux_by_rel = {r.get("rel"): r for r in demux_res if r.get("rel")}
    mjpeg_meta_by_rel = {m["rel"]: m for m in mjpeg_rows if "rel" in m}

    def mjpeg_case_meta(rel: str) -> dict:
        """Case metadata (res_name/w/h) from the case build, falling back to
        the manifest params — identical values, so a --skip-suites reuse run
        aggregates the same groups a full run would."""
        m = mjpeg_meta_by_rel.get(rel)
        if m is not None:
            return m
        p = man_index.get(rel, {}).get("params", {})
        return {"res_name": p.get("res_name", "?"), "w": p.get("width") or 0, "h": p.get("height") or 0}

    def case_kind(rel: str) -> str:
        cls = man_index.get(rel, {}).get("class", "?")
        if cls in ("video", "av", "avi-video", "avi-av"):
            return "clean"
        if cls in ("dirty", "dirty-lie-ext"):
            return "dirty"
        return cls  # unknown class labels its own rows — never silently merged

    groups: dict[tuple[str, str], list[dict]] = {}
    res_px: dict[str, int] = {}
    for row in mjpeg_res:
        rel = row.get("rel", "")
        meta = mjpeg_case_meta(rel)
        rn = str(meta.get("res_name", "?"))
        res_px[rn] = max(res_px.get(rn, 0), int(meta.get("w", 0) or 0) * int(meta.get("h", 0) or 0))
        groups.setdefault((rn, case_kind(rel)), []).append(row)
    report.append("| resolution | kind | files | frames | decode errors | concealed | PSNR min (dB) | PSNR mean (dB) | max abs | exact bytes | throughput | demux terminal | demux rec |")
    report.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for rn, kind in sorted(groups, key=lambda k: (res_px.get(k[0], 0), k[0], k[1] != "clean")):
        rows = groups[(rn, kind)]
        frames = sum(int(r.get("decoded") or 0) for r in rows)
        errors = sum(int(r.get("errors") or 0) for r in rows)
        concealed = sum(int(r.get("concealed") or 0) for r in rows)
        # Fidelity aggregates exclude files with ANY concealed frame: a
        # concealed frame is the documented damage response (mid-gray tail
        # vs ffmpeg's partial pixels), not decoder infidelity. File-level
        # exclusion — per-frame exclusion needs the suites to emit
        # clean-only PSNR columns (registered follow-up). Counts and
        # throughput stay whole-group.
        fid = [r for r in rows if int(r.get("concealed") or 0) == 0]
        pmin = min((int(r.get("psnr_min_mdb") or 0) for r in fid if int(r.get("psnr_min_mdb") or 0) > 0), default=0)
        pmeans = [int(r.get("psnr_mean_mdb") or 0) for r in fid if int(r.get("psnr_mean_mdb") or 0) > 0]
        pmean = statistics.mean(pmeans) if pmeans else 0
        mabs = max((int(r.get("max_abs") or 0) for r in fid), default=0)
        exact = sum(int(r.get("exact_bytes") or 0) for r in fid)
        compared = sum(int(r.get("compared_bytes") or 0) for r in fid)
        payload = sum(int(r.get("payload_bytes") or 0) for r in rows)
        dus = sum(int(r.get("decode_us") or 0) for r in rows)
        thr = f"{payload / (1 << 20) / max(dus, 1) * 1e6:.1f} MB/s" if payload else "-"
        fps = f"{frames / max(dus, 1) * 1e6:.0f} fps" if frames else "-"
        exact_pct = f"{100.0 * exact / compared:.2f}%" if compared else "-"
        # demux attribution (W1-01 prescription 3): terminal + recovered per
        # class row so packet-count deltas are attributable at a glance; the
        # per-case ledger below carries the full join.
        terms: Counter = Counter()
        rec = 0
        for r in rows:
            d = demux_by_rel.get(r.get("rel"))
            if d is None:
                terms["no-demux-row"] += 1
            else:
                terms[str(d.get("terminal") or "?")] += 1
                rec += int(d.get("recovered") or 0)
        term_s = " ".join(f"{k}:{v}" for k, v in sorted(terms.items())) or "-"
        report.append(f"| {rn} | {kind} | {len(rows)} | {frames} | {errors} | {concealed} | {pmin / 1000:.1f} | "
                      f"{pmean / 1000:.1f} | {mabs} | {exact_pct} | {thr}, {fps} | {term_s} | {rec} |")
    report.append("")
    report.append("_PSNR across independent JPEG IDCT implementations is expected well above 45 dB; "
                  "byte-exactness is not expected cross-implementation. `exact bytes` is the bit-equal "
                  "fraction vs ffmpeg's decode._")
    report.append("_W2-08 (W1-01's prescription): `kind` splits clean corpus classes from dirty siblings "
                  "(manifest-class join) — dirty concealment can no longer pollute clean-class PSNR "
                  "minima. Fidelity columns (PSNR min/mean, max abs, exact bytes) exclude files with any "
                  "concealed frame; per-frame exclusion needs suite-emitted clean-only PSNR columns "
                  "(registered follow-up). `demux terminal`/`demux rec` come from the demux walk of the "
                  "same file, so packet-count deltas are attributable._")

    # Per-case verdict ledger: every mjpeg decode row joined with its demux
    # walk (terminal/recovered) and the oracle packet truth — the
    # attribution record W1-01 asked for (packet-count deltas traceable per
    # file, not just per class).
    mjpeg_verdicts_path = harness_dir / "mjpeg-verdicts.tsv"
    if mjpeg_res:
        with mjpeg_verdicts_path.open("w") as f:
            f.write("\t".join(["rel", "class", "kind", "res_name", "decoded", "errors", "concealed",
                               "psnr_min_mdb", "psnr_mean_mdb", "demux_terminal", "demux_recovered",
                               "demux_v_pkts", "truth_v_packets", "v_delta"]) + "\n")
            for row in sorted(mjpeg_res, key=lambda r: r.get("rel", "")):
                rel = row.get("rel", "")
                d = demux_by_rel.get(rel) or {}
                try:
                    vdelta = str(int(d.get("v_pkts", "")) - int(d.get("truth_v_packets", "")))
                except (ValueError, TypeError):
                    vdelta = "-"
                f.write("\t".join([
                    rel, man_index.get(rel, {}).get("class", "?"), case_kind(rel),
                    str(mjpeg_case_meta(rel).get("res_name", "?")),
                    row.get("decoded", ""), row.get("errors", ""), row.get("concealed", ""),
                    row.get("psnr_min_mdb", ""), row.get("psnr_mean_mdb", ""),
                    d.get("terminal", ""), d.get("recovered", ""),
                    d.get("v_pkts", ""), d.get("truth_v_packets", ""), vdelta,
                ]) + "\n")
        print(f"[h[harness] mjpeg-verdicts: {len(mjpeg_res)} rows -> {mjpeg_verdicts_path} "
              "(demux terminal/recovered + packet deltas per case)")
        report.append(f"Per-case attribution ledger (decode ⋈ demux ⋈ oracle packets): "
                      f"`{mjpeg_verdicts_path}` ({len(mjpeg_res)} rows).")
    report.append("")

    # -- pcm judgement -----------------------------------------------------
    report.append("## PCM decode vs ffmpeg oracle (byte-exact expected)")
    report.append("")
    suite_note(pcm_res)
    # A case is COMPARABLE only when the AVI demuxed the audio stream at
    # all; refused/undemuxable cases (unsupported fourcc combos, damaged
    # headers) are counted separately — they are demuxer-scope findings,
    # not PCM decoder verdicts.
    comparable = [r for r in pcm_res if not r.get("note", "").startswith(("demux.", "pcm.unknown"))]
    not_demuxable = len(pcm_res) - len(comparable)
    pcm_exact = 0
    pcm_mismatch_files = 0
    pcm_short = 0
    for row in comparable:
        mis = int(row.get("mismatch") or 0)
        if mis == 0 and not row.get("note"):
            pcm_exact += 1
        elif mis > 0:
            pcm_mismatch_files += 1
        elif row.get("note", "").startswith("len:"):
            pcm_short += 1
    report.append("| metric | value |")
    report.append("|---|---|")
    report.append(f"| comparable PCM cases (audio stream demuxed) | {len(comparable)}/{len(pcm_res)} "
                  f"({not_demuxable} not demuxable — see demux section) |")
    report.append(f"| comparable cases byte-exact vs ffmpeg | **{pcm_exact}/{len(comparable)}** |")
    report.append(f"| cases with byte mismatches | {pcm_mismatch_files} (any mismatch is a defect signal) |")
    report.append(f"| cases with shorter demuxed payload than oracle (len notes) | {pcm_short} |")
    for row in comparable:
        if int(row.get("mismatch") or 0) > 0:
            report.append(f"  - {row['rel']}: {row['mismatch']} mismatched bytes, max_abs={row['max_abs']}, note={row.get('note', '')}")
    report.append("")

    # -- decoded format fidelity (W1-04) -----------------------------------
    report.append("## Decoded format fidelity vs oracle (decoded output format + dimensions)")
    report.append("")
    report.append("Per-file ledger: `corpus/_harness/decoded-rows.tsv` — oracle columns from ffprobe "
                  "`-show_streams` (video pix_fmt/width/height; audio sample_fmt/channels/sample_rate), "
                  "Lumen columns from the corpus suites' decoded-output rows. Match-rate scope: the "
                  "supported-format subset, AVI files whose fourccs Lumen decodes (mjpeg video + pcm "
                  "audio), clean and dirty — the same population as the demux section's supported set.")
    report.append("")

    def t_of(e: dict) -> dict:
        return truth.get(e.get("rel", ""), {})

    vid_cases = [e for e in avi_entries if video_codec_intent(e.get("params", {})).startswith("mjpeg")]
    aud_cases = [e for e in avi_entries if audio_codec_intent(e.get("params", {})).startswith("pcm")]
    vid_oracle = [(e, t_of(e)) for e in vid_cases if t_of(e).get("v_w", "-") != "-"]
    vid_pix = [(e, t) for e, t in vid_oracle if t.get("v_pix_fmt", "-") != "-"]
    aud_oracle = [(e, t_of(e)) for e in aud_cases if t_of(e).get("a_sample_fmt", "-") != "-"]
    mjpeg_elsewhere = sum(1 for e in files
                          if video_codec_intent(e.get("params", {})).startswith("mjpeg")
                          and (e.get("family") or ["?"])[0] != "avi")
    pcm_elsewhere = sum(1 for e in files
                        if audio_codec_intent(e.get("params", {})).startswith("pcm")
                        and (e.get("family") or ["?"])[0] != "avi")

    fam_counts = Counter(pix_family(t["v_pix_fmt"]) for _, t in vid_pix)
    sfam_counts = Counter(sample_family(t["a_sample_fmt"]) for _, t in aud_oracle)
    ch_counts = Counter(t["a_ch"] for _, t in aud_oracle)
    rate_counts = Counter(t["a_rate"] for _, t in aud_oracle)
    # oracle vs generator intent — measurable today; validates the oracle
    # row itself and catches encoder-side drift (e.g. a gray-intent mjpeg
    # draw coming back from the AVI as yuvj444p). Intent is only compared
    # where the manifest actually recorded it (the av draw path persists
    # no pix_fmt; audio-only draws record their codec under `codec`).
    vid_dims_intent = [(e, t) for e, t in vid_oracle if e.get("params", {}).get("width")]
    vid_pix_intent = [(e, t) for e, t in vid_pix if e.get("params", {}).get("pix_fmt")]
    aud_chrate_intent = [(e, t) for e, t in aud_oracle if e.get("params", {}).get("audio_rate")]
    dims_drift = [(e, t) for e, t in vid_dims_intent
                  if str(e.get("params", {}).get("width")) != t["v_w"]
                  or str(e.get("params", {}).get("height")) != t["v_h"]]
    pix_drift = [(e, t) for e, t in vid_pix_intent
                 if pix_family(str(e.get("params", {}).get("pix_fmt") or "-")) != pix_family(t["v_pix_fmt"])]
    chrate_drift = [(e, t) for e, t in aud_chrate_intent
                    if str(e.get("params", {}).get("audio_channels")) != t["a_ch"]
                    or str(e.get("params", {}).get("audio_rate")) != t["a_rate"]]

    def dist(counter: Counter) -> str:
        return ", ".join(f"{k}={v}" for k, v in sorted(counter.items(), key=lambda kv: -kv[1])) or "-"

    report.append("### Oracle-side decoded formats (measured)")
    report.append("")
    report.append("| metric | value |")
    report.append("|---|---|")
    report.append(f"| mjpeg AVIs with oracle dims (w×h) | {len(vid_oracle)}/{len(vid_cases)} |")
    report.append(f"| oracle dims equal generator intent | {len(vid_dims_intent) - len(dims_drift)}/{len(vid_dims_intent)} |")
    report.append(f"| oracle pix_fmt families (mjpeg AVIs) | {dist(fam_counts)} |")
    report.append(f"| oracle pix_fmt family equal generator intent | {len(vid_pix_intent) - len(pix_drift)}/{len(vid_pix_intent)} "
                  f"(intent recorded for {len(vid_pix_intent)}/{len(vid_pix)} — the av draw path persists no pix_fmt) |")
    report.append(f"| pcm AVIs with oracle sample_fmt | {len(aud_oracle)}/{len(aud_cases)} |")
    report.append(f"| oracle sample families (pcm AVIs) | {dist(sfam_counts)} |")
    report.append(f"| oracle channel layouts (pcm AVIs) | {dist(ch_counts)} |")
    report.append(f"| oracle sample rates (pcm AVIs) | {dist(rate_counts)} |")
    report.append(f"| oracle channels+rate equal generator intent | {len(aud_chrate_intent) - len(chrate_drift)}/{len(aud_chrate_intent)} |")
    report.append(f"| mjpeg/pcm streams outside AVI (mov/mkv/wav…) | {mjpeg_elsewhere} video / {pcm_elsewhere} audio "
                  "(out of scope until their demuxers land — those rows carry no-path markers) |")
    report.append("")

    # Lumen vs oracle match rates. Forward-compatible: computes for real
    # the moment the suites emit out_format/out_w/out_h (mjpeg) and
    # out_desc/out_rate/out_ch (pcm); until then every Lumen cell is an
    # explicit suite-gap marker, never a fabricated match.
    v_dims_ok = v_fam_ok = v_compared = 0
    for e, t in vid_oracle:
        row = mjpeg_by_rel.get(e["rel"]) or {}
        if not row.get("out_format"):
            continue
        v_compared += 1
        if row.get("out_w") == t["v_w"] and row.get("out_h") == t["v_h"]:
            v_dims_ok += 1
        if LUMEN_PIX_FAMILY.get(str(row.get("out_format", "")).lower()) == pix_family(t["v_pix_fmt"]):
            v_fam_ok += 1
    a_fmt_ok = a_chrate_ok = a_compared = 0
    for e, t in aud_oracle:
        row = pcm_by_rel.get(e["rel"]) or {}
        if not row.get("out_desc"):
            continue
        a_compared += 1
        if sample_family(row.get("out_desc", "-")) == sample_family(t["a_sample_fmt"]):
            a_fmt_ok += 1
        if row.get("out_ch") == t["a_ch"] and row.get("out_rate") == t["a_rate"]:
            a_chrate_ok += 1

    def rate(ok: int, n: int, population: int) -> str:
        if n:
            return f"**{100.0 * ok / n:.1f}%** ({ok}/{n} comparable)"
        return f"N/A — {GAP} (0/{population} comparable)"

    report.append("### Lumen vs oracle match rates")
    report.append("")
    report.append("| metric | match | population |")
    report.append("|---|---|---|")
    report.append(f"| video dims match | {rate(v_dims_ok, v_compared, len(vid_oracle))} | {len(vid_oracle)} mjpeg AVIs with oracle dims |")
    report.append(f"| pix_fmt family match | {rate(v_fam_ok, v_compared, len(vid_pix))} | {len(vid_pix)} with oracle pix_fmt |")
    report.append(f"| audio sample_fmt match | {rate(a_fmt_ok, a_compared, len(aud_oracle))} | {len(aud_oracle)} pcm AVIs with oracle sample_fmt |")
    report.append(f"| channels+rate match | {rate(a_chrate_ok, a_compared, len(aud_oracle))} | {len(aud_oracle)} |")
    report.append("")
    if v_compared == 0 and a_compared == 0:
        report.append(f"Lumen-side decoded format/dims: **{GAP}** — the mjpeg/pcm corpus suites measure "
                      "counts/PSNR/bytes but do not write the decoded `DecodedFrame`'s format "
                      "(`DecodedFormat::I420`/`Rgba8`) or width/height, nor the PCM output format, into "
                      "their result rows. Registered follow-up (suite emission): extend "
                      "`mjpeg-results.tsv` with `out_format`/`out_w`/`out_h` (first decoded frame's "
                      "DecodedFormat + dims) and `pcm-results.tsv` with `out_desc`/`out_rate`/`out_ch`; "
                      "this harness already consumes those columns when present — no further driver "
                      "change needed.")
        report.append("")

    # mismatch listing (file, expected, got)
    report.append("### Mismatch listing (file, expected, got)")
    report.append("")
    vid_listing: list[str] = []
    for e, t in vid_oracle:
        row = mjpeg_by_rel.get(e["rel"]) or {}
        exp = f"{t['v_w']}x{t['v_h']} {t['v_pix_fmt']} (family {pix_family(t['v_pix_fmt'])})"
        if row.get("out_format"):
            got = f"{row.get('out_w', '-')}x{row.get('out_h', '-')} {row.get('out_format', '-')}"
            if (row.get("out_w") == t["v_w"] and row.get("out_h") == t["v_h"]
                    and LUMEN_PIX_FAMILY.get(str(row.get("out_format", "")).lower()) == pix_family(t["v_pix_fmt"])):
                continue  # confirmed match — not a mismatch row
        else:
            got = GAP
        vid_listing.append(f"- {e['rel']} [{e['class']}]: expected {exp}, got {got}")
    aud_listing: list[str] = []
    for e, t in aud_oracle:
        row = pcm_by_rel.get(e["rel"]) or {}
        exp = f"{t['a_sample_fmt']} (family {sample_family(t['a_sample_fmt'])}) {t['a_rate']}Hz x{t['a_ch']}"
        if row.get("out_desc"):
            got = f"{row.get('out_desc', '-')} {row.get('out_rate', '-')}Hz x{row.get('out_ch', '-')}"
            if (sample_family(row.get("out_desc", "-")) == sample_family(t["a_sample_fmt"])
                    and row.get("out_ch") == t["a_ch"] and row.get("out_rate") == t["a_rate"]):
                continue
        else:
            got = GAP
        aud_listing.append(f"- {e['rel']} [{e['class']}]: expected {exp}, got {got}")
    for line in vid_listing[:10]:
        report.append(line)
    if len(vid_listing) > 10:
        report.append(f"- … and {len(vid_listing) - 10} more video rows (full ledger: decoded-rows.tsv)")
    for line in aud_listing[:10]:
        report.append(line)
    if len(aud_listing) > 10:
        report.append(f"- … and {len(aud_listing) - 10} more audio rows (full ledger: decoded-rows.tsv)")
    if not vid_listing and not aud_listing:
        report.append("- none")
    report.append("")

    # generator-intent vs oracle drift — the mismatches that ARE measurable
    # today, on the oracle side of the ledger.
    drift_rows: dict[str, tuple[dict, list[str]]] = {}
    for e, t in dims_drift:
        drift_rows.setdefault(e["rel"], (e, []))[1].append(
            f"dims intent {e['params'].get('width')}x{e['params'].get('height')} vs oracle {t['v_w']}x{t['v_h']}")
    for e, t in pix_drift:
        drift_rows.setdefault(e["rel"], (e, []))[1].append(
            f"pix intent {e['params'].get('pix_fmt')} (family {pix_family(str(e['params'].get('pix_fmt')))}) "
            f"vs oracle {t['v_pix_fmt']} (family {pix_family(t['v_pix_fmt'])})")
    for e, t in chrate_drift:
        drift_rows.setdefault(e["rel"], (e, []))[1].append(
            f"audio intent {e['params'].get('audio_rate')}Hz x{e['params'].get('audio_channels')} "
            f"vs oracle {t['a_rate']}Hz x{t['a_ch']}")
    report.append("#### Generator-intent vs oracle drift (real mismatches, measurable today)")
    report.append("")
    report.append(f"{len(drift_rows)} file(s) where the oracle's decoded facts disagree with the "
                  "generator's declared parameters — dirty-class corruption is one legitimate source; "
                  "encoder-side format drift (a gray-intent draw encoded as yuvj444p) is another:")
    for rel, (e, notes) in list(drift_rows.items())[:8]:
        report.append(f"- {rel} [{e['class']}]: " + "; ".join(notes))
    if len(drift_rows) > 8:
        report.append(f"- … and {len(drift_rows) - 8} more")
    if not drift_rows:
        report.append("- none")
    report.append("")

    # -- suite health ------------------------------------------------------
    report.append("## Suite health")
    report.append("")
    if args.skip_suites:
        report.append(f"- probe suite rc={rc_probe}, demux rc={rc_demux}, mjpeg rc={rc_mjpeg}, pcm rc={rc_pcm} — "
                      + ("SUITES NOT DRIVEN (--skip-suites): judgement re-aggregated from the persisted "
                         f"last-run TSVs ({', '.join(reused)}) — not a fresh measurement" if reused
                         else "SUITES SKIPPED (--skip-suites): oracle-side validation run, not a measurement run"))
    else:
        report.append(f"- probe suite rc={rc_probe}, demux rc={rc_demux}, mjpeg rc={rc_mjpeg}, pcm rc={rc_pcm} — all must be 0")
    report.append(f"- harness wall: {time.time() - t0:.0f}s")
    report.append(f"- decoded-rows ledger: `{decoded_rows_path}` ({len(files)} rows, {n_gap_cells} suite-gap cells) — "
                  "registered follow-up: suites emit `out_format`/`out_w`/`out_h` (mjpeg) and "
                  "`out_desc`/`out_rate`/`out_ch` (pcm)")
    if mjpeg_res:
        report.append(f"- mjpeg verdicts ledger (per case: decode ⋈ demux terminal/recovered ⋈ oracle "
                      f"packets): `{mjpeg_verdicts_path}` ({len(mjpeg_res)} rows)")
    report.append("")

    text = "\n".join(report)
    suffix = "-skip-suites" if args.skip_suites else ""
    out_md = REPO / "docs" / "corpus" / f"report-{time.strftime('%Y%m%d-%H%M')}{suffix}.md"
    out_md.parent.mkdir(parents=True, exist_ok=True)
    out_md.write_text(text)
    corpus_report = corpus / ("REPORT-skip-suites.md" if args.skip_suites else "REPORT.md")
    corpus_report.write_text(text)
    print(f"[h[harness] report: {out_md}")
    if not args.keep_refs and not args.skip_suites:
        shutil.rmtree(refs_dir, ignore_errors=True)
    if args.skip_suites:
        print("[h[harness] --skip-suites: oracle-side validation complete (rc=0 by construction)")
        return 0
    return 0 if all(rc == 0 for rc in (rc_probe, rc_demux, rc_mjpeg, rc_pcm)) else 1


if __name__ == "__main__":
    sys.exit(main())
