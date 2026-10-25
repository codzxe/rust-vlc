#!/usr/bin/env python3
"""q2v_matrix_backfill.py — 5.Q2-V Phase-5 records-audit matrix backfill.

CONTEXT (docs/gates/phase5-records-audit.md):
  The parity tool (doc 02 §4.6 tools/feature-audit, card 8.F1 debt) is
  ABSENT, so the documented-honest path for the 5.Q2-V audit is this
  committed reproducer script: it encodes every status transition
  (baseline = main db90dbc, wave-9 open) with its evidence citation,
  can APPLY them idempotently, and CHECKS the file against the expected
  post-backfill state (drift detector — exit 1 on any mismatch, so the
  Lead / R2-V gate can re-run it).

  Every transition's evidence is a landed per-format record under
  docs/formats/ (or, where no record exists, the landing card's worklog
  Task ID — stated honestly in the audit report). Statuses follow the
  doc 02 §4.3 vocabulary; the script enforces the R3/R4/R5/R6-shaped
  rules on every row it touches:
    R3  exactly windows/linux/macos/web, values in vocabulary
    R4  implemented/verified => non-empty evidence
    R5  blocked/unsupported => non-empty blocked_reason
    R6  partial => non-empty notes

Usage:
  python3 scripts/q2v_matrix_backfill.py            # --check (default)
  python3 scripts/q2v_matrix_backfill.py --apply    # perform backfill
  python3 scripts/q2v_matrix_backfill.py --explain  # print transitions
"""

from __future__ import annotations

import re
import sys

try:
    import tomllib
except ImportError:  # pragma: no cover
    sys.exit("python3.11+ required (tomllib)")

MATRIX = "docs/feature-matrix.toml"
VOCAB = {"not_started", "in_progress", "implemented", "verified",
         "partial", "unsupported", "blocked"}
PLATFORMS = ("windows", "linux", "macos", "web")
BASELINE = "main db90dbc (wave-9 open, 2026-10-25 audit)"

NOT_STARTED = {p: "not_started" for p in PLATFORMS}
WEB_NS = {**{p: "implemented" for p in ("windows", "linux", "macos")}, "web": "not_started"}
PARTIAL_NS = {**{p: "partial" for p in ("windows", "linux", "macos")}, "web": "not_started"}
BLOCKED4 = {p: "blocked" for p in PLATFORMS}

# ---------------------------------------------------------------------------
# The transition table. old -> new per row, with the evidence that justifies
# the new status (record file and/or landing-card Task ID).
# ---------------------------------------------------------------------------
T: list[dict] = [
    # --- containers (records: 5.REC-1, evidence 5.A2/5.B1/5.C1/5.C2b/5.I*) ---
    dict(id="container-wav", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/demux/src/wav/** (card 5.A2 demux; record docs/formats/container-wav.md, 5.REC-1: differential tables cited to 5.A2); Linux-only evidence, CI 3-OS pending",
         notes="5.Q2-V backfill: record-consolidation status implemented-demux (5.REC-1); RF64 >4 GB untested [VERIFY]."),
    dict(id="container-aiff", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/demux/src/aiff/** (card 5.A2; record docs/formats/container-aiff.md, 5.REC-1); fixture-only evidence — ZERO aiff corpus lanes (census 5.REC-1)",
         notes="5.Q2-V backfill: implemented-demux (5.REC-1); uLaw breadth [VERIFY]."),
    dict(id="container-ogg", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/demux/src/ogg/** (card 5.B1; record docs/formats/container-ogg.md, 5.REC-1: chain header-stash 28v31 divergence recorded); Linux-only evidence",
         notes="5.Q2-V backfill: implemented-demux (5.REC-1); cross-chain seek untested; Kate mapping note stale (5.P2b-I flag)."),
    dict(id="container-mp4-mov-3gp", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/demux/src/mp4/** (card 5.C1; record docs/formats/container-mp4.md, 5.REC-1: 30-lane corpus_mp4 table); Linux-only evidence",
         notes="5.Q2-V backfill: implemented-demux (5.REC-1); 3GP quirk breadth [VERIFY]."),
    dict(id="container-adts-aac", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/demux/src/adts/** (card 5.C2b; record docs/formats/container-adts.md, 5.REC-1: 0059/0110 lanes PASS, dirty-lane 350-vs-351 divergence recorded); Linux-only evidence",
         notes="5.Q2-V backfill: implemented-demux (5.REC-1)."),
    dict(id="container-mpegts", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/demux/src/ts/** (cards 5.I1a+5.I1b; record docs/formats/container-mpegts.md, 5.REC-1: 5 ts corpus lanes); Linux-only evidence",
         notes="5.Q2-V backfill: implemented-demux (5.REC-1); multi-program TS = synthetic-only (corpus single-program)."),
    dict(id="container-mpegps", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/demux/src/ps/** (card 5.I2; record docs/formats/container-mpegps.md, 5.REC-1); fixture-only evidence — ZERO ps/vob corpus lanes",
         notes="5.Q2-V backfill: implemented-demux (5.REC-1); PS nav/span-interpolation ffmpeg divergences recorded not smoothed."),
    dict(id="container-mp3-raw", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/demux/src/es/** (card 5.I3a; record docs/formats/container-es.md, 5.REC-1); Linux-only evidence",
         notes="5.Q2-V backfill: implemented-demux (5.REC-1); keyframe-IDR-only + 0145 attached-pic divergences recorded."),
    dict(id="container-h264-annexb", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/demux/src/es/** (cards 5.F1a parser + 5.I3a wrapper; record docs/formats/container-es.md, 5.REC-1); Linux-only evidence",
         notes="5.Q2-V backfill: implemented-demux (5.REC-1)."),
    dict(id="container-hevc-annexb", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/demux/src/es/** (cards 5.G1 parser + 5.I3a wrapper; record docs/formats/container-es.md, 5.REC-1); Linux-only evidence",
         notes="5.Q2-V backfill: implemented-demux (5.REC-1)."),
    dict(id="container-ivf", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/demux/src/ivf/** (card 5.I3b; record docs/formats/container-ivf.md, 5.REC-1); fixture-only evidence — ZERO ivf corpus lanes; VP8 typed v0 refusal",
         notes="5.Q2-V backfill: implemented-demux (5.REC-1); 5 malformed-shape ffmpeg divergences recorded."),
    # --- matroska: honestly partial (wiring seam open) ---
    dict(id="container-matroska-webm", old=NOT_STARTED, new=PARTIAL_NS,
         evidence="crates/demux/src/mkv/** (cards 5.D1 core + 5.D2a attachments/chapters + 5.D2b editions/tags; record docs/formats/container-matroska.md incl. 5.REC-1 audit addendum: 34-lane differential); Linux-only evidence",
         notes="5.Q2-V backfill: partial per record 'in progress' — open() wiring seam OPEN (mkv/mod.rs skips Tags/Attachments/Chapters; positional-scan getters are the public seam), TrackUID (0x73C5) not retained."),
    # --- audio codecs (records: 5.REC-2) ---
    dict(id="codec-audio-flac", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/codecs/src/audio/flac/** + claxon =0.4.3 (card 5.A1; record docs/formats/audio-flac.md, 5.REC-2: 4 fixtures byte-exact + STREAMINFO MD5, corpus 0046, 539x realtime)",
         notes="5.Q2-V backfill: implemented per record (5.REC-2)."),
    dict(id="codec-audio-mp3", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/codecs/src/audio/mp3/** (card 5.A3 symphonia + in-house glue; record docs/formats/audio-mp3-mp2.md, 5.REC-2); Linux-only evidence",
         notes="5.Q2-V backfill: implemented per record (5.REC-2); MP3 corpus PCM-MD5 table [VERIFY-P2] lost to a deadline — re-print owed (audit gap G-3)."),
    dict(id="codec-audio-mp2", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/codecs/src/audio/mp2/** (card 5.J3b; record docs/formats/audio-mp3-mp2.md, 5.REC-2: honest ±1-LSB verdict on 4 rows — NOT MD5-bit-exact, two independent lossy syntheses)",
         notes="5.Q2-V backfill: implemented per record (5.REC-2); ±1 LSB is the recorded honest ceiling."),
    dict(id="codec-audio-vorbis", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/codecs/src/audio/vorbis/** (card 5.B2; record docs/formats/audio-vorbis.md, 5.REC-2: 5-row differential, silence lane BIT-EXACT, others <=6e-8)",
         notes="5.Q2-V backfill: implemented per record (5.REC-2)."),
    dict(id="codec-audio-aac-lc", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/codecs/src/audio/aac/** (cards 5.C2a+5.C2b; record docs/formats/audio-aac.md, 5.REC-2: 6-row 1-LSB table, 0094 6ch typed partial, dirty-lane 350-vs-351 divergence)",
         notes="5.Q2-V backfill: implemented for AAC-LC (5.REC-2); HE-AAC is the separate blocked row."),
    dict(id="codec-audio-alac", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/codecs/src/audio/alac/** (card 5.J3a; record docs/formats/audio-alac.md, 5.REC-2: 5/5 bit-exact incl. noise stressor)",
         notes="5.Q2-V backfill: implemented per record (5.REC-2); 32-bit row impossible with this ffmpeg."),
    dict(id="codec-audio-adpcm", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/codecs/src/audio/adpcm/** (cards 5.J2a+5.J2b+5.J2-MERGE; record docs/formats/audio-adpcm.md, 5.REC-2: 7 descriptors, corpus 0042/0102 + 8/8 breadth bit-exact)",
         notes="5.Q2-V backfill: implemented for 7 descriptors (5.REC-2); lettered adpcm_oki typed-Unsupported (NO ffmpeg 7.1.5 reference at all)."),
    # --- video codecs ---
    dict(id="codec-video-av1", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/codecs/src/video/av1/** + rav1d =1.1.0 asm-off (cards 5.E1+5.E2; record docs/formats/video-av1.md, 5.REC-2/5.E2: conformance 15 rows pixel-MD5 PASS, 8-bit bit-exact x3, 10-bit typed x5)",
         notes="5.Q2-V backfill: implemented per record — 8-bit 4:2:0; 10/12-bit typed-unsupported [VERIFY-P1]; asm-off 0.309x vs libdav1d honest gap."),
    dict(id="codec-video-h264", old=NOT_STARTED, new=PARTIAL_NS,
         evidence="crates/codecs/src/video/h264/** (cards 5.F1a/5.F1b clean-room parser corpus-verified 19/19 + 5.F2 hw decode-path seam; record docs/formats/video-h264.md: 0019 26/26 + 0014 185/185 slices vs trace_headers)",
         notes="5.Q2-V backfill: partial per record — hw decode-path seam delivered, playback hw-only pending Phase-7 adapters; NO software decoder exists (doc 04 §4.2 stands)."),
    dict(id="codec-video-hevc", old=NOT_STARTED, new=PARTIAL_NS,
         evidence="crates/codecs/src/video/hevc/** (cards 5.G1 parser 3/3 corpus lanes byte-verified + 5.G2 hw decode-path seam 571/0; record docs/formats/video-hevc.md)",
         notes="5.Q2-V backfill: partial per record — hw seam delivered (5.G2), software decode absent; Main-10/4:4:4 corpus absence recorded."),
    dict(id="codec-video-mpeg1-2", old=NOT_STARTED, new=BLOCKED4,
         blocked_reason="No production pure-Rust MPEG-1/2 decoder (card 5.H1 verdict; record docs/formats/video-mpeg12.md): sequence-layer parser + typed refusal LANDED (10/10 differential, 380/43 gates, 8k fuzz rounds) but software decode blocked; Phase-6 gated admission (feasibility video-codecs §4 F-5); no-egress honesty flag preserved.",
         notes="5.Q2-V backfill: blocked per record (5.H1) — sequence-layer facts land, decode typed-refused."),
    dict(id="codec-video-mpeg4-asp", old=NOT_STARTED, new=BLOCKED4,
         blocked_reason="v1 decode typed refusal (card 5.H2-I decision; record docs/formats/video-mpeg4asp.md): no pure-Rust ASP decoder; L-class self-build deferred; Phase-6 admission path recorded in-file.",
         notes="5.Q2-V backfill: blocked for v1 decode per record (5.H2-I)."),
    dict(id="codec-video-vp9", old=NOT_STARTED, new=BLOCKED4,
         blocked_reason="No validated pure-Rust VP9 engine (card 5.P1-I; record docs/formats/video-vp8-vp9.md): candidate named, validation spike pending — Phase-6 admission entry gate.",
         notes="5.Q2-V backfill: blocked (candidate named, validation pending) per record (5.P1-I)."),
    dict(id="codec-video-vp8", old=NOT_STARTED, new=BLOCKED4,
         blocked_reason="No validated pure-Rust VP8 engine (card 5.P1-I; record docs/formats/video-vp8-vp9.md); IVF VP8 already typed v0 refusal at the container seam.",
         notes="5.Q2-V backfill: blocked (no validated engine) per record (5.P1-I)."),
    dict(id="codec-audio-truehd-mlp", old=NOT_STARTED, new=BLOCKED4,
         blocked_reason="Decode typed-unsupported in v1 (card 5.K1; record docs/formats/audio-truehd.md): bitstream-io 4.10.0 ToBits macro OOM root-caused; scratch differential 4/4 bit-exact (4,065,640 samples) proves the decoder; unblock = upstream fix or ~60-line vendor patch — decision at R-10/R-43 gates; no-silent-downmix policy.",
         notes="5.Q2-V backfill: blocked (pending-unblock) per record (5.K1) — blocked WITH a proven unblock path."),
    # --- subtitle formats (5.M wave + 5.N2) ---
    dict(id="subtitle-srt", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/subtitles/src/srt/** (card 5.M1; record docs/formats/sub-srt.md, 5.Q2-V: 6/6 ffmpeg 7.1.5 differential incl. UTF-16 both endians, 105/0 at card, 230/0 current tree)",
         notes="5.Q2-V backfill: implemented per NEW record (5.M1 evidence)."),
    dict(id="subtitle-ass-ssa", old=NOT_STARTED, new=PARTIAL_NS,
         evidence="crates/subtitles/src/ass/** (cards 5.M2a parser + 5.M2b render; record docs/formats/sub-ass-ssa.md, 5.Q2-V: 3/3 real-ffmpeg ASS docs exact, 9/9 alignment-grid pixel-exact, 2000+2048 fuzz-lite runs clean)",
         notes="5.Q2-V backfill: partial per NEW record — plain-text subset ONLY; ALL override-tag styling deferred (the R-09 gap); libass oracle impossible (C, forbidden)."),
    dict(id="subtitle-vtt", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/subtitles/src/vtt/** (card 5.M3; record docs/formats/sub-vtt.md, 5.Q2-V: 3 real ffmpeg -c:s webvtt fixtures + settings_heavy handcrafted, 7003 fuzz inputs zero panics, 92/0 at card)",
         notes="5.Q2-V backfill: implemented per NEW record (5.M3 evidence)."),
    dict(id="subtitle-dvb-subs", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/subtitles/src/dvb/** (card 5.N2, commits ab696b4+93a53ec: EN 300 743 segment/CLUT/RLE/page machine -> CompositedCue; 230/0 subtitles gates incl. dvb_vobsub_fuzz_survival); NO docs/formats record yet — audit gap G-1",
         notes="5.Q2-V backfill: implemented (5.N2) — honest gap: NO reference-encoder differential (ffmpeg 7.1.5 refuses text->bitmap; bit tables cross-checked line-by-line vs ffmpeg RLE semantics), no corpus lane, record owed."),
    dict(id="subtitle-vobsub", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/subtitles/src/vobsub/** (card 5.N2, commits ab696b4+93a53ec: idx/spu parse, 128 KiB cap, grayscale fallback, damage ledger; 230/0 gates); NO docs/formats record yet — audit gap G-1",
         notes="5.Q2-V backfill: implemented (5.N2) — same no-differential honesty as DVB; record owed."),
    dict(id="subtitle-encoding-detection", old=NOT_STARTED, new=PARTIAL_NS,
         evidence="crates/subtitles/src/srt/encoding.rs (card 5.M1: BOM matrix UTF-8/UTF-16LE/UTF-16BE + BOM-less UTF-8 lossy fallback + damage)",
         notes="5.Q2-V backfill: partial — BOM-class detection landed (5.M1); BOM-less UTF-16 deliberately NOT sniffed; full subsdec-class charset heuristics not started."),
    # --- text render ---
    dict(id="textrender-subtitle-rasterizer", old=NOT_STARTED, new=PARTIAL_NS,
         evidence="crates/subtitles/src/compositing/** + model (cards 5.L1+5.L2: CueBitmap/CueLayout/place_cue pixel-exact math, 176-cue x 9-alignment roundtrip matrix, timing edges; 5.M2b placeholder glyph metrics: monospace-widest-char, integer ratios, no floats)",
         notes="5.Q2-V backfill: partial — bitmap compositing + placeholder block glyphs landed; the REAL font stack (freetype replacement) is NOT started — crate choice [VERIFY-P0] stands; every text cue currently renders as placeholder blocks."),
    # --- playlist formats (5.O wave; no docs/formats record owed by doc 13 card specs) ---
    dict(id="plfmt-m3u-m3u8", old=NOT_STARTED, new=PARTIAL_NS,
         evidence="crates/playlists/src/m3u.rs (cards 4.D1 M3U + 5.O1a HLS #EXT-X-* media-playlist v1 subset: 3 ffmpeg-generated fixtures byte-identical, EXTINF sums vs ffprobe 9.046543/9.046539, RFC 3986 join 23+19 pinned, 10k fuzz runs; 38/0)",
         notes="5.Q2-V backfill: partial — M3U + HLS MEDIA playlist landed; master playlists (EXT-X-STREAM-INF) Phase 6; KEY/MAP recorded-not-resolved."),
    dict(id="plfmt-pls", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/playlists/src/pls.rs (card 5.O1b: golden truth-sets handcrafted basic/sparse/dirty, exact warning-sequence asserts, fuzz-lite no-panic; 46/0 playlists gates)",
         notes="5.Q2-V backfill: implemented (5.O1b); PLS<->M3U bridge roundtrip lossless."),
    dict(id="plfmt-xspf", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/playlists/src/xspf.rs (card 5.O1b: hand-rolled XML-subset pull parser, zero new deps, CDATA + entities, depth guard 64; 46/0 playlists gates)",
         notes="5.Q2-V backfill: implemented (5.O1b); creator/annotation M3U-bridge asymmetry documented."),
    dict(id="plfmt-cue", old=NOT_STARTED, new=WEB_NS,
         evidence="crates/playlists/src/cue.rs (card 5.O2: 3 handcrafted AC sheets vs hand-computed frame->ns truth, zero-gap joins pinned exactly, 13 typed warnings, 4600-track stress; playlists gates green at card)",
         notes="5.Q2-V backfill: implemented (5.O2); pregap excluded from duration, exposed separately."),
]

BY_ID = {t["id"]: t for t in T}


def esc(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def status_line(st: dict) -> str:
    inner = ", ".join(f'{p} = "{st[p]}"' for p in PLATFORMS)
    return f"status = {{ {inner} }}"


def load():
    with open(MATRIX, "rb") as f:
        return tomllib.load(f)


def check() -> int:
    m = load()
    rows = {r["id"]: r for r in m["feature"]}
    errs = []
    for t in T:
        r = rows.get(t["id"])
        if r is None:
            errs.append(f"{t['id']}: row MISSING from matrix")
            continue
        for p in PLATFORMS:
            if r["status"][p] != t["new"][p]:
                errs.append(f"{t['id']}.{p}: expected {t['new'][p]}, found {r['status'][p]}")
        if "implemented" in t["new"].values() and not r.get("evidence"):
            errs.append(f"{t['id']}: implemented without evidence (R4)")
        if "blocked" in t["new"].values() and t["new"]["web"] == "blocked" and not r.get("blocked_reason"):
            errs.append(f"{t['id']}: blocked without blocked_reason (R5)")
        if "partial" in t["new"].values() and not r.get("notes"):
            errs.append(f"{t['id']}: partial without notes (R6)")
    # global vocabulary sanity on the whole file (R3)
    for r in m["feature"]:
        if set(r["status"]) != set(PLATFORMS):
            errs.append(f"{r['id']}: platform keys wrong (R3)")
        for p in PLATFORMS:
            if r["status"][p] not in VOCAB:
                errs.append(f"{r['id']}.{p}: status '{r['status'][p]}' not in vocabulary (R3)")
    if errs:
        print(f"DRIFT vs {BASELINE} backfill target ({len(errs)} problem(s)):")
        for e in errs:
            print("  -", e)
        return 1
    print(f"OK: matrix matches the 5.Q2-V post-backfill state ({len(T)} transitions, {len(m['feature'])} rows parse, R3/R4/R5/R6 hold on touched rows).")
    return 0


def apply() -> int:
    src = open(MATRIX, encoding="utf-8").read()
    m = load()
    rows = {r["id"]: r for r in m["feature"]}
    changed = 0
    for t in T:
        r = rows[t["id"]]
        if all(r["status"][p] == t["new"][p] for p in PLATFORMS):
            continue  # already applied (idempotent)
        if not all(r["status"][p] == t["old"][p] for p in PLATFORMS):
            print(f"REFUSE {t['id']}: current status {r['status']} is neither baseline "
                  f"{t['old']} nor target {t['new']} — manual review required", file=sys.stderr)
            return 1
        # textual row-block edit
        pat = re.compile(
            r'(\[\[feature\]\]\nid = "' + re.escape(t["id"]) + r'".*?\n)(status = \{[^\n]*\}\n)(evidence = "[^"]*"\n)',
            re.S)
        m2 = pat.search(src)
        if not m2:
            print(f"REFUSE {t['id']}: row block shape unexpected", file=sys.stderr)
            return 1
        repl = m2.group(1) + status_line(t["new"]) + "\n" + f'evidence = "{esc(t.get("evidence", ""))}"\n'
        src = src[:m2.start()] + repl + src[m2.end():]
        changed += 1
        # notes append (single-line notes per house format)
        if t.get("notes"):
            nm = re.search(r'(^notes = ")([^"]*)(")', src[m2.start():], re.M | re.S)
            if nm:
                s, e = nm.span(2)
                base = m2.start()
                old_notes = src[base + s: base + e]
                if t["notes"][:12] not in old_notes:
                    src = src[:base + s] + old_notes + ("" if old_notes.endswith(" ") else " ") + t["notes"] + src[base + e:]
        # blocked_reason for newly-blocked rows
        if t.get("blocked_reason") and not rows[t["id"]].get("blocked_reason"):
            bm = re.search(r'(^notes = "[^"]*"\n)', src, re.M)
            # insert after the notes line of THIS row block
            blk = re.search(r'\[\[feature\]\]\nid = "' + re.escape(t["id"]) + r'".*?\n(notes = "[^"]*"\n)',
                            src, re.S)
            if blk:
                src = (src[:blk.end(1)] + f'blocked_reason = "{esc(t["blocked_reason"])}"\n' + src[blk.end(1):])
    if changed:
        open(MATRIX, "w", encoding="utf-8").write(src)
        print(f"APPLIED {changed}/{len(T)} transitions to {MATRIX}; re-run with --check.")
    else:
        print("Nothing to apply — matrix already at target state.")
    return 0


def explain() -> int:
    print(f"5.Q2-V matrix backfill — baseline {BASELINE}; {len(T)} transitions:\n")
    for t in T:
        old, new = t["old"], t["new"]
        o = "/".join(old[p][:4] for p in PLATFORMS)
        n = "/".join(new[p][:4] for p in PLATFORMS)
        print(f"  {t['id']:34s} {o:19s} -> {n:19s}  {t.get('evidence', t.get('blocked_reason', ''))[:100]}")
    return 0


if __name__ == "__main__":
    mode = sys.argv[1] if len(sys.argv) > 1 else "--check"
    sys.exit({"--apply": apply, "--check": check, "--explain": explain}.get(mode, check)())
