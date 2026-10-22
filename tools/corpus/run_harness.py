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
     is a FAILED contract row in the report.
  5. Aggregate everything into measured-number tables and write
     docs/corpus/report-<stamp>.md (+ corpus/REPORT.md copy).

Usage:
  run_harness.py [--corpus DIR] [--skip-gen-check] [--profile release]
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import statistics
import subprocess
import sys
import time
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
    refs_dir.mkdir(parents=True, exist_ok=True)

    t0 = time.time()
    files = manifest["files"]
    print(f"[h[harness] corpus: {len(files)} files, seed {manifest.get('seed')}")

    # ------------------------------------------------------------------
    # 1. ffprobe truth for every file (the external oracle).
    #    Checkpointed: a reaper kill mid-run resumes from truth.json.
    # ------------------------------------------------------------------
    truth_path = harness_dir / "truth.json"
    if truth_path.is_file():
        truth = json.loads(truth_path.read_text())
        print(f"[h[harness] truth checkpoint: {len(truth)} files (resumed)")
    else:
        truth: dict[str, dict] = {}
        for i, entry in enumerate(files):
            path = corpus / entry["rel"]
            if not path.is_file():
                print(f"[h[harness] WARNING missing file {entry['rel']}")
                continue
            j = ffprobe_json(path, args.ffprobe)
            rec: dict = {"format": "-", "duration_ms": "-", "streams": "-", "v_pkts": "-", "a_pkts": "-"}
            if j is not None:
                fmt = j.get("format", {})
                rec["format"] = fmt.get("format_name", "-") or "-"
                d = fmt.get("duration")
                rec["duration_ms"] = f"{float(d) * 1000:.1f}" if d else "-"
                rec["streams"] = str(len(j.get("streams", [])))
                if entry["family"][0] == "avi":
                    rec["v_pkts"] = str(ffprobe_packets(path, args.ffprobe, "v:0") or "-")
                    rec["a_pkts"] = str(ffprobe_packets(path, args.ffprobe, "a:0") or "-")
            truth[entry["rel"]] = rec
            if (i + 1) % 50 == 0:
                print(f"[h[harness] ffprobe {i + 1}/{len(files)}", flush=True)
                truth_path.write_text(json.dumps(truth))  # incremental checkpoint
        truth_path.write_text(json.dumps(truth))

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

    # mjpeg cases: AVI + mjpeg video (clean AND dirty — dirty measures
    # concealment/error behavior against ffmpeg's own recovery).
    mjpeg_entries = [e for e in avi_entries
                     if e["params"].get("codec") == "mjpeg" or e["params"].get("codec", "").startswith("mjpeg")]
    mjpeg_cases = harness_dir / "mjpeg-cases.tsv"
    mjpeg_rows: list[dict] = []
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

    # pcm cases: AVI + pcm audio (clean AND dirty).
    pcm_entries = [e for e in avi_entries if e["params"].get("audio_codec", "").startswith("pcm")]
    pcm_cases = harness_dir / "pcm-cases.tsv"
    pcm_rows: list[dict] = []
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
          f"mjpeg={len([r for r in mjpeg_rows if 'ref' in r])} pcm={len([r for r in pcm_rows if 'ref' in r])}")

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

    rc_probe, res_probe = run_suite("media-probe", "corpus", probe_cases, "probe-results.tsv")
    rc_demux, res_demux = run_suite("demux", "corpus", avi_cases, "demux-results.tsv")
    rc_mjpeg, res_mjpeg = run_suite("codecs", "corpus", mjpeg_cases, "mjpeg-results.tsv",
                                    "corpus_mjpeg_decode_measures_against_oracle")
    rc_pcm, res_pcm = run_suite("codecs", "corpus", pcm_cases, "pcm-results.tsv",
                                "corpus_pcm_decode_is_byte_exact_or_recorded")

    def read_tsv(path: Path) -> list[dict]:
        if not path.is_file():
            return []
        lines = path.read_text().splitlines()
        if len(lines) < 2:
            return []
        hdr = lines[0].split("\t")
        return [dict(zip(hdr, ln.split("\t"))) for ln in lines[1:] if ln.strip()]

    probe_res = read_tsv(res_probe)
    demux_res = read_tsv(res_demux)
    mjpeg_res = read_tsv(res_mjpeg)
    pcm_res = read_tsv(res_pcm)

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
    report.append(f"- suite exit codes: probe={rc_probe} demux={rc_demux} mjpeg={rc_mjpeg} pcm={rc_pcm} "
                  f"(**0 = no panic, no contract violation**)")
    report.append("")

    # -- probe judgement --------------------------------------------------
    report.append("## Probe (bounded format detection) vs ffprobe oracle")
    report.append("")
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
    report.append(f"| probe latency p50/p95/max | {percentile(probe_lat, 0.5):.0f}µs / "
                  f"{percentile(probe_lat, 0.95):.0f}µs / {percentile(probe_lat, 1.0):.0f}µs |")
    report.append("")

    # -- demux judgement (scoped to the fourccs the demuxer can feed) -----
    # The AVI demuxer's supported set is mjpeg video + pcm audio (the
    # codecs that exist); files outside it are expected to be TYPED
    # refusals, judged on the refusal, not on conformance.
    report.append("## AVI demux walk vs ffprobe oracle (supported fourccs: mjpeg + pcm)")
    report.append("")

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
            span_ms = (int(r.get("last_v_pts_ns", 0)) - int(r.get("first_v_pts_ns", 0))) / 1e6
            if tdm not in ("-", "", None):
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
        report.append(f"| PTS-span duration error p50/p95/max | {fmt_ms(percentile(dur_err, 0.5))} / "
                      f"{fmt_ms(percentile(dur_err, 0.95))} / {fmt_ms(percentile(dur_err, 1.0))} ms |")
    dirty_typed = sum(1 for r in dirty if r.get("terminal") in ("eos", "error", "start-error"))
    dirty_recovered = sum(1 for r in dirty if num(r, "recovered") and num(r, "recovered") > 0)
    report.append(f"| dirty AVIs reached a typed terminal (no panic, no hang) | **{dirty_typed}/{len(dirty)}** |")
    report.append(f"| dirty AVIs that recovered damage mid-walk | {dirty_recovered} |")
    refused_ok = sum(1 for r in refused if r.get("terminal") == "start-error")
    report.append(f"| unsupported-fourcc AVIs answered with a typed refusal | {refused_ok}/{len(refused)} "
                  f"(no panic on any) |")
    report.append("")

    # -- mjpeg judgement ---------------------------------------------------
    report.append("## MJPEG decode vs ffmpeg oracle (real throughput + fidelity)")
    report.append("")
    by_res: dict[str, list[dict]] = {}
    res_px: dict[str, int] = {}
    for row in mjpeg_res:
        meta = next((m for m in mjpeg_rows if m["rel"] == row["rel"]), {})
        rn = meta.get("res_name", "?")
        res_px[rn] = max(res_px.get(rn, 0), int(meta.get("w", 0)) * int(meta.get("h", 0)))
        by_res.setdefault(rn, []).append(row)
    report.append("| resolution | files | frames | decode errors | concealed | PSNR min (dB) | PSNR mean (dB) | max abs | exact bytes | throughput |")
    report.append("|---|---|---|---|---|---|---|---|---|---|")
    for rn in sorted(by_res, key=lambda k: res_px.get(k, 0)):
        rows = by_res[rn]
        frames = sum(int(r.get("decoded") or 0) for r in rows)
        errors = sum(int(r.get("errors") or 0) for r in rows)
        concealed = sum(int(r.get("concealed") or 0) for r in rows)
        pmin = min((int(r.get("psnr_min_mdb") or 0) for r in rows if int(r.get("psnr_min_mdb") or 0) > 0), default=0)
        pmeans = [int(r.get("psnr_mean_mdb") or 0) for r in rows if int(r.get("psnr_mean_mdb") or 0) > 0]
        pmean = statistics.mean(pmeans) if pmeans else 0
        mabs = max((int(r.get("max_abs") or 0) for r in rows), default=0)
        exact = sum(int(r.get("exact_bytes") or 0) for r in rows)
        compared = sum(int(r.get("compared_bytes") or 0) for r in rows)
        payload = sum(int(r.get("payload_bytes") or 0) for r in rows)
        dus = sum(int(r.get("decode_us") or 0) for r in rows)
        thr = f"{payload / (1 << 20) / max(dus, 1) * 1e6:.1f} MB/s" if payload else "-"
        fps = f"{frames / max(dus, 1) * 1e6:.0f} fps" if frames else "-"
        exact_pct = f"{100.0 * exact / compared:.2f}%" if compared else "-"
        report.append(f"| {rn} | {len(rows)} | {frames} | {errors} | {concealed} | {pmin / 1000:.1f} | "
                      f"{pmean / 1000:.1f} | {mabs} | {exact_pct} | {thr}, {fps} |")
    report.append("")
    report.append("_PSNR across independent JPEG IDCT implementations is expected well above 45 dB; "
                  "byte-exactness is not expected cross-implementation. `exact bytes` is the bit-equal "
                  "fraction vs ffmpeg's decode._")
    report.append("")

    # -- pcm judgement -----------------------------------------------------
    report.append("## PCM decode vs ffmpeg oracle (byte-exact expected)")
    report.append("")
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

    # -- suite health ------------------------------------------------------
    report.append("## Suite health")
    report.append("")
    report.append(f"- probe suite rc={rc_probe}, demux rc={rc_demux}, mjpeg rc={rc_mjpeg}, pcm rc={rc_pcm} — all must be 0")
    report.append(f"- harness wall: {time.time() - t0:.0f}s")
    report.append("")

    text = "\n".join(report)
    out_md = REPO / "docs" / "corpus" / f"report-{time.strftime('%Y%m%d-%H%M')}.md"
    out_md.parent.mkdir(parents=True, exist_ok=True)
    out_md.write_text(text)
    (corpus / "REPORT.md").write_text(text)
    print(f"[h[harness] report: {out_md}")
    if not args.keep_refs:
        shutil.rmtree(refs_dir, ignore_errors=True)
    return 0 if all(rc == 0 for rc in (rc_probe, rc_demux, rc_mjpeg, rc_pcm)) else 1


if __name__ == "__main__":
    sys.exit(main())
