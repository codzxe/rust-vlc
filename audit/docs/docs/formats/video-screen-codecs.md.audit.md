# CLAIM AUDIT: docs/formats/video-screen-codecs.md
| Field | Value |
|---|---|
| Auditor | W19-A07 (recovered by W19-R5) |
| Claims extracted | 9 |
| REAL 7 / UNVERIFIED 1 / PAPER-ONLY 1 (doc-level) | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Honest deferred decision record — the doc claims NOTHING is implemented and the tree agrees
(no screen codec anywhere in crates/codecs/src/video). Every in-tree factual claim checks:
no dedicated matrix row, legacy-blocked umbrella row exists with exactly the named codecs, no
FLV demuxer, AVI demuxer present with typed decode refusal, ASF record present, corpus
contains no screen-codec lane. The only unverifiable content is the crates.io census (offline,
dated 2026-10-24). Zero inflation: the doc proposes a row instead of claiming one.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Status: deferred — decision record (card 5.P2b-I part B) | :3 | PAPER-ONLY | Self-describing record; no code anywhere claims screen codecs (see #3). |
| 2 | No dedicated feature-matrix row exists today (only codec-video-legacy-blocked for Cinepak/Hap/Dirac/MS Video 1) | :15-17 | REAL | `rg -c codec-video-screen-capture docs/feature-matrix.toml` -> 0; row :607 `id = "codec-video-legacy-blocked"`, :610 description "Indeo 3/4/5, RealVideo RV10–RV40, Cinepak, Hap, Dirac, MS Video 1 and similar legacy codecs". |
| 3 | "Nothing implemented" (family) | :30 | REAL | `ls crates/codecs/src/video` -> av1 dnx h264 hevc mjpeg mpeg12 vp9 only; `ls crates/codecs/src/audio` has no screen codec either. |
| 4 | crates.io direct names all NOT_FOUND (2026-10-24) | :40-47 | UNVERIFIED | No network re-run possible (offline sandbox); claim is point-in-time and consistent with repo isolation. |
| 5 | VLC has no dedicated decoder modules; modules-linux.json's 14 video-decoder registrations contain no screen-codec entries | :48-54 | REAL | audit/runs/W18-A04.modules-linux.json by_capability["video decoder"].count = 14; module names exactly png, theora, oggspots, cdg, avcodec×2, rawvideo, dav1d, ddummy×2, jpeg, xwd, svgdec, stats (python3 parse, json.dumps excerpt). |
| 6 | FSV1/2 live in FLV; no FLV demuxer in tree | :56-58 | REAL | `ls crates/demux/src` -> adts aiff asf avi es ivf mkv mp4 ogg ps ts wav — no flv. |
| 7 | AVI demuxer exists; fourcc pass-through; decode refused (typed-unsupported) | :58-60 | REAL | crates/demux/src/avi/mod.rs:494 `match classify_movi_chunk(&header.fourcc)`; apps/cli/src/pipeline_host.rs:376-378 `"no decoder for `{other}` in this build (landed: mjpeg, pcm, flac, ...)"` (sed-verified). |
| 8 | MSS1/2 live in ASF (Phase-6 proposal, container-asf.md) | :60-61 | REAL | `ls docs/formats | grep asf` -> container-asf.md. |
| 9 | Corpus: no screen-codec lane in corpus-v4 | :88-89 | REAL | grep of corpus manifests: only 2 "fps1" hits in corpus-big-manifest-20261022.json, both random base64 encoder-name substrings (python3 walk: /files[23]/cmd[43]) — not FRAPS material; corpus-v4 has zero. |

## 3. Contradictions
None. Doc's "not-probe-verified census tail" caveat (formats.json video-census-tail) matches
the census-tail row's own [VERIFY] posture.

## 4. Fabrication / inflation indicators
None. The record explicitly refuses to add the matrix row inside an investigation ("feature-
matrix.toml deliberately untouched") — the opposite of inflation.

Commands run: ls crates/codecs/src/{audio,video}, crates/demux/src, docs/formats; rg on feature-matrix.toml (screen/legacy-blocked/cinepak, codec-video-screen-capture); python3 json parse of audit/runs/W18-A04.modules-linux.json (by_capability video decoder count/names); sed -n 374-380p apps/cli/src/pipeline_host.rs; grep -o screen-codec tokens in corpus-big-manifest + python3 fps1-hit walk; rg on demux/src/avi fourcc.
