# CLAIM AUDIT: docs/inventory/formats.md
| Field | Value |
|---|---|
| Auditor | W18-A06 |
| Date | 2026-02-11 |
| Claims extracted | 21 |
| REAL | 15 |
| REAL-UNPROVEN | 2 |
| SHALLOW | 0 |
| STUB | 0 |
| PAPER-ONLY | 2 |
| FALSE | 2 |
| Internal contradictions found | 1 (evidence location: line 5 vs lines 11-12) |

## 1. Blunt verdict
The numbers in this doc are honest to a degree unusual for this repo: the
headline counts, per-kind splits, P0 list, and P1×23/P2×19/P3×13 distribution
reproduce EXACTLY against `formats.json`, the per-family probe multiplicities
(mkv ×3, mp4 ×3+8, avi ×2, ts ×3, ogg ×4) reproduce exactly against the
in-repo consolidated `17-format-probes.txt`, and the doc volunteers its own
method's blind spots ([V1]–[V10]). The one real defect: the "Evidence base"
paragraph claims **61 log files "in `probe-work/logs/`"** — that directory
does not exist; zero log files are committed, and `formats.json`'s
probe_evidence cites **159 log-file paths, all dead** (verified
programmatically: dead 159 / live 0). The actual per-probe evidence in-tree
is the consolidated decisive-line extraction in 17-format-probes.txt (62
blocks: 61 probes + reconciliation), which the doc's own line 5 correctly
names — so the doc contradicts itself about where the evidence lives. Every
claim that cites only the missing raw logs (not the consolidated txt) is
UNVERIFIED: e.g. the udp quote "opening server=:0 local=:5004",
"loading spu-es slave", http_server.log's "Serving HTTP on 127.0.0.1 port
8123", the adpcm fourcc `  ms`.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| F-01 | "82 families" (machine-readable twin formats.json) | formats.md:4-5 | REAL | python3: json families_total=82; len(families)=82. |
| F-02 | Headline table: container 21/18/3, audio 18/13/5, video 12/9/3, subtitle 12/8/4, playlist 7/6/1, protocol 12/5/7, total 82/59/23 | formats.md:15-25 | REAL | formats.json counts.by_kind + probed_by_kind + not_probe_verified_by_kind + probe_status Counter (probed=59, not=23) — every cell matches. |
| F-03 | "(27 of 82 families" + the P0 bullet list | formats.md:31-41 | REAL | usage_share=='P0' in formats.json → exactly 27 families; name sets identical (containers 9 + audio 6 + video 3 + subs 3 + playlists 3 + protocols 3). |
| F-04 | "everything else is P1×23 / P2×19 / P3×13 in formats.json" | formats.md:41-42 | REAL | usage_share distribution: P0=27, P1=23, P2=19, P3=13 (python Counter). |
| F-05 | "Evidence base: 61 log files … in `probe-work/logs/`" | formats.md:11-12 | FALSE | `find docs/baselines/raw/linux/probe-work -type f` → 9 files, no logs/ dir; `ls probe-work/logs` → "No such file or directory". formats.json probe_evidence log paths: 159 dead / 0 live (os.path.exists sweep). |
| F-06 | "of which 58 contain module-selection lines; `status.txt` records 57 exit codes (all 0 except `rtp_ts` = 124…)" | formats.md:12-13 | REAL-UNPROVEN | Consistent with the in-repo consolidation: 17-format-probes.txt:20-21 "The logs directory holds 61 .log files + status.txt (57 status entries).", :1663 "58 of 61 logs", rtp_ts block "exit code: 124". status.txt itself absent — numbers not independently checkable. |
| F-07 | "Per-probe verbatim evidence: `../baselines/raw/linux/17-format-probes.txt`" | formats.md:5-6 | REAL | File exists (1,664+ lines); 62 "## " blocks; verified decisive lines below. |
| F-08 | "Pure-Python struct-writers (`probe-work/gen_py.py`, preserved, 16 numbered fixtures): canonical WAV, 6 playlists, 8 text-subtitle files, one directory fixture" | formats.md:47-49 | REAL | gen_py.py (108 lines) writes exactly: media/wav_pcm_py.wav (:13), 6 pls files (m3u/pls/xspf/asx/b4s/cue), 8 subs (srt/ssa/ass/vtt/microdvd/subviewer/sami/ttml :49-96), dir/tone.wav copy (:105-106) = 16. |
| F-09 | "30 media fixtures covering containers and codec payloads" (ffmpeg synthesis) | formats.md:50-51 | REAL | run_probes.sh containers loop lists exactly 30 filenames (wav_pcm_py.wav … tone_zipped.zip). |
| F-10 | "**The ffmpeg command lines were not preserved**" [V1] | formats.md:52-53 | REAL (honest confession) | No ffmpeg cmd file in probe-work (9 files only); corroborated by txt:1565 "sender command not preserved [VERIFY]". |
| F-11 | Census long-tail demux list "caf/voc/xa/tta/mpc/mod/sid/nsv/pva/ty/nuv/rawdv/rawvid/rawaud/image/diracsys/vc1/nsc/cdg/stl/vobsub/sap/stats/demuxdump/linsys_sdi/libbluray/dvdnav" | formats.md:102 | REAL | All 27 names present in modules-linux.json by_capability demux/demux_filter module lists (cross-checked against demux census list; 57 total demux registrations). |
| F-12 | Verbatim decisive lines preserved | formats.md:63-64, §3 tables | REAL | rg counts in 17-format-probes.txt: "detected SubRIP format"=1, "found valid M3U playlist"=2, "found valid PLS playlist"=1, "pls file version"=1, "detected MicroDVD/SubViewer/SAMI format"=1 each, "Movie fps: 10.000000"=8, "refusing to decode non validated subtitle codec"=4, "detected MPEG2 TS"=1, "Codec `MIDI' is not supported" (:743), spdif=4, "forced subtitle"=16, "using access module \"udp\"" (udp_ts block :1554+), "using access_demux module \"rtp\"" (rtp_ts block), `using audio decoder module "avcodec"` (wav_adpcm_ima :696). |
| F-13 | Family multiplicities: matroska-webm "probed ×3 (MKV×2, WebM)", mp4 "probed ×3 (incl. M4A)", avi "probed ×2", mpegts "probed ×3 (file/udp/rtp)", ogg "probed ×4" | formats.md:82-87,84,87-89 | REAL | python3 block scan of 17-format-probes.txt: wav-demux blocks: mkv 3 (mkv_h264_flac, webm_vp9_opus, mkv_av1_opus); mp4 3 direct + 8 subsl carriers; avi 2 (avi_mjpeg, avi_mpeg4); ts 3 (mpegts_h264, udp_ts, rtp_ts); ogg 4 (vorbis/opus/theora/speex). |
| F-14 | wav "probed in 6 distinct sessions" | formats.md:89 | FALSE (undercount) | Actual: 12 blocks carry `using demux module "wav"` (wav_pcm_py, wav_adpcm_ima, tone_zipped, pl_m3u/pls/xspf/asx/b4s, dir_access, http_wav, http_m3u, smoke_wav). Understates its own evidence; harmless direction. S4. |
| F-15 | udp chose `udp` ("opening server=:0 local=:5004") | formats.md:184 | PAPER-ONLY | The module selection IS preserved ("using access module \"udp\"", txt udp_ts block); the quoted log line "opening server=:0 local=:5004" appears NOWHERE in-tree (rg: 0 matches; the only ":5004" hit is the reconstructed command line txt:1554). Quote cites the absent log. |
| F-16 | sidecar loads log "…`loading spu-es slave`" | formats.md:161-162, 277-279 | PAPER-ONLY | "forced subtitle" preserved (16 hits); "loading spu-es slave" / any "spu-es" string: 0 hits in the consolidated txt. Quote cites the absent logs. |
| F-17 | "`http_server.log` (4 lines): … ("Serving HTTP on 127.0.0.1 port 8123" + 3 GETs)" | formats.md:285-287 | UNVERIFIED | http_server.log absent; the consolidated txt has a block (:1634-1635, reconstructed command "python3 -m http.server 8123") but not the quoted "Serving HTTP" line or the 3 GETs. |
| F-18 | adpcm chose `avcodec` (fourcc `  ms`) | formats.md:109 | REAL-UNPROVEN | avcodec selection preserved (wav_adpcm_ima block :696 `using audio decoder module "avcodec"`); the fourcc detail `  ms` is not in the txt (log absent). |
| F-19 | "subsl_* (8 logs)" second pass proves subtitle parsing/decoding | formats.md:275-278 | REAL | Exactly 8 subsl_* blocks (txt:1211-1460): srt, ssa, ass, vtt, microdvd, subviewer, sami, ttml; each with `using spu decoder module "subsdec"/"libass"/"webvtt"/"ttml"` + `forced subtitle` lines. |
| F-20 | §2.4 blind spots (spdif passthrough, no bit-exact decode proof, 10 excluded debs incl. fluidsynth, dummy-vout size) | formats.md:65-74 | REAL (honest) | spdif×4 + cross-check note txt:1664; MIDI negative :739-743; excluded_debs_headless list in modules-linux.json (10 packages incl. vlc-plugin-fluidsynth) — cross-verified. |
| F-21 | Oracle pin "VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)" | formats.md:8-10 | REAL | modules-linux.json vlc_version: "3.0.23-2-0-g79128878dd (VLC 3.0.23 Vetinari, Debian 3.0.23-0+deb13u1)"; run_probes.sh pins EXTRACT=/home/z/my-project/tmp/pinning/extract. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| formats.md:5-6 "Per-probe verbatim evidence: ../baselines/raw/linux/17-format-probes.txt" | formats.md:11-12 "Evidence base: 61 log files … in `probe-work/logs/`" | Both describe the same evidence set; only the consolidated txt exists in-tree. The 61 logs + status.txt are absent (find: 9 files). The JSON's 159 log paths all point into the missing directory. |

## 4. Fabrication / inflation indicators
- None of the quantitative claims inflate; the two numeric defects found
  (wav sessions 6 vs 12 actual) UNDERSTATE. The [VERIFY] ledger is used
  aggressively (10 items) and the §2.4 "Known blind spots of the method"
  section volunteers limitations (spdif passthrough ≠ decode proof; no
  bit-exact oracle; excluded debs). This is the best-behaved claim doc
  audited in this chunk.
- The structural weakness: every probe_evidence path in formats.json
  (159 of them) resolves to a nonexistent file; the CI/validation story
  ("per-probe verbatim evidence") survives only because 17-format-probes.txt
  was manually consolidated. Any tooling that resolves those paths (like the
  xtask feature-audit that already fails on 2 evidence paths per baseline)
  will find dead links here at scale.

## 5. Matrix rows
Not a matrix chunk — n/a.

## 6. Commands run
- python3: formats.json counts/Counter (families 82, probed 59, priorities
  27/23/19/13, dead-log sweep 159/0); block-scan of 17-format-probes.txt
  (62 blocks; wav 12 / mkv 3 / mp4 11 / avi 2 / ts 3 demux sessions)
- rg counts for 20+ verbatim quoted log lines (results in F-12)
- find probe-work (9 files); ls probe-work/logs (absent, exit 2)
- sed -n block reads (udp_ts :1554+, rtp_ts, wav_adpcm_ima :680+, sub_srt
  :784+, subsl blocks :1211-1460, http_server :1634-1635, reconciliation
  :1663-1664)
- cat run_probes.sh; rg gen_py.py structure (16 fixtures verified)
