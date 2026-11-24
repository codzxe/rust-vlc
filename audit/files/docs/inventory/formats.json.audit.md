# CLAIM AUDIT (DATA): docs/inventory/formats.json
| Field | Value |
|---|---|
| Auditor | W18-A03 |
| Date | 2026-02-14 |
| Lines / families | 2101 / 82 family records (single JSON object) |
| File verdict | REAL-UNPROVEN (arithmetic + consolidated evidence verified; raw-log preservation claim FALSE — see F-001) |
| Reachable from shipped entry point? | N/A (data file). Consumed as scope/evidence source by docs/inventory/formats.md, docs/corpus/compat-matrix-20261110.json, docs/feature-matrix.toml (rg hits). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | python3 json parse (82 families, kind recount); rg/awk over 17-format-probes.txt (wav-demux 12-section count, 62 `##` sections); sed quote verification (12 ranges); LS/Glob for evidence paths; sed over apps/cli/src/containers/mod.rs:47-76, es.rs open_flac, dispatch.rs:188-199, pipeline_host.rs:1236-1239, demux_host.rs:355-360; baseline cites (mp4-smoke.md, W13 SUMMARY, W13-A05/A10 audits). |

## 1. Blunt verdict
This is the Phase-0 VLC format inventory: 82 families, and its internal arithmetic is honest —
every count block (families_total 82, by_kind, probed_by_kind, not_probe_verified_by_kind)
recomputes exactly from the 82 records, every "probed" claim I spot-checked reproduces
byte-for-byte in the in-repo consolidation (17-format-probes.txt), and every unverifiable
external stat is flagged [VERIFY] in the file itself. It is NOT a Lumen capability doc and
never claims to be. Two defects: (1) the header's "preserved" raw-log evidence path
(`docs/baselines/raw/linux/probe-work/logs/`) does not exist in the repo — ~120 per-family
`log_files` entries dangle on a dead directory; only the consolidation file survives; (2) the
container census undercounts what Lumen actually ships: no IVF family anywhere while the CLI
has a real, executed IVF demuxer. Against the shipped CLI, the gap this file defines is large
and quantifiable: 12 of 21 container families wired (+1 refusal arm, 8 families refused
outright), ~9 of 18 audio codecs, 4 of 12 video codecs, 0 of 12 subtitle families, 0 of 7
playlists, 2 of 12 protocols.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "families_total": 82 | fj:38 | REAL | python3 parse → 82 records |
| 2 | by_kind: container 21, audio-codec 18, playlist 7, protocol 12, subtitle 12, video-codec 12 | fj:39-46 | REAL | recount per record matches exactly |
| 3 | probed_by_kind sums (59) + not_probe_verified (23) = 82 | fj:47-62 | REAL | recount: 59 probed / 23 census-only |
| 4 | "logs": "docs/baselines/raw/linux/probe-work/logs/ — 61 .log files + status.txt (57 entries; …)" | fj:9 | FALSE | LS probe-work/ = {gen_py.py, run_probes.sh, dir/, pls/}; Glob `**/mkv_h264_flac.log` → no results |
| 5 | "consolidation": "docs/baselines/raw/linux/17-format-probes.txt (one section per probe, verbatim decisive lines)" | fj:10 | REAL | file exists 74291 B; awk → 62 `##` sections (61 probes + cross-checks) |
| 6 | "oracle_sessions": "61 log files = 60 VLC sessions (58 with content) + 1 python http.server log" | fj:63 | REAL-UNPROVEN | 61 sections exist in consolidation; raw logs unverifiable (missing) |
| 7 | "Every family in this inventory carries an explicit probe_status" | fj:65 | REAL | all 82 records parsed with `probed` \| `not-probe-verified` |
| 8 | "58 of 61 logs contain module-selection evidence (rtp_ffmpeg/udp_ffmpeg empty, http_server is the python daemon log)" | fj:65 | REAL-UNPROVEN | consolidation documents rtp_ffmpeg/udp_ffmpeg as 0-byte sender captures; raw logs missing |
| 9 | "The wav demux appears in 12 of the 58 evidenced logs" | fj:299 | REAL | awk over 17-format-probes.txt → exactly 12 sections with `using demux module "wav"` |
| 10 | matroska-webm: chosen_demux "mkv"; decoders avcodec(h264), flac, dav1d(av1), opus, avcodec(vp9) | fj:86-94 | REAL | 17-format-probes.txt mkv sections show those exact module lines |
| 11 | midi-smf verdict: "'no audio decoder modules matched' -> 'Codec `MIDI` … is not supported'" | fj:554 | REAL | midi_minimal section carries both lines verbatim |
| 12 | cue verdict: "NEGATIVE: pinned VLC 3.0.23 has no CUE-sheet support" | fj:1769 | REAL | pl_cue_sample section: `cannot peek` ×5, `nothing to play`, no cue reader |
| 13 | rtp "exit_code": 124 … "live stream never EOFs under --play-and-exit; timeout(1) kill at 25 s" | fj:1937-1938 | REAL | rtp_ts section: `exit code: 124` + PAT/PMT/SDT walk lines |
| 14 | usage_share rationales (Commons counts, Bitmovin share, P0-P3 priorities) | throughout | UNVERIFIED | external-network claims; file itself flags [VERIFY]/bot-blocked 403 — honest, unverifiable from sandbox |

## 3. Container cross-check vs shipped CLI (the assigned analysis)
Shipped CLI container support = the 15 probe ids with arms at apps/cli/src/containers/mod.rs:47-76
(14 real demuxer arms + 1 typed refusal; Wave 13 dispatch findings + audit/files/apps/cli/src/containers/mod.rs.audit.md).
Executed proof points: mp4-smoke.md (`lumen open` H.264+AAC MP4 exit 0; wav decode-bench a_decoded=5; hevc exit 65).

| # | inventory family (families[]) | CLI probe id | Shipped state | Evidence |
|---|---|---|---|---|
| 00 | matroska-webm | matroska | WIRED — real demuxer | mod.rs:57 `"matroska" => mkv::open(stream, opts)`; W13-A06 |
| 01 | mp4-mov-3gp | mp4 | WIRED — EXECUTED open | mp4-smoke.md:6-20 exit_code=0 |
| 02 | ogg | ogg | WIRED — defective (flac-in-ogg broken, S1) | W13-A06 (W13 SUMMARY:52-53) |
| 03 | mpegts | mpegts | WIRED — defective (TS probe panic exit 101 live, S1) | W13-A08/A09 (W13 SUMMARY:48-49) |
| 04 | mpegps | mpegps | WIRED — real demuxer | mod.rs:71 `"mpegps" => ps::open(stream, opts)` |
| 05 | avi | avi | WIRED — EXECUTED decode | rc-smoke headless-decode AVI/MJPEG/PCM (AGENT-SPEC §13) |
| 06 | asf-wmv | — | NOT WIRED (unwired_refusal) | mod.rs:74 `other => Err(unwired_refusal(other))` |
| 07 | wav | wav | WIRED — EXECUTED decode | mp4-smoke.md:86-92 `a_decoded=5` (container wav) |
| 08 | flac-native | flac | ARM = TYPED REFUSAL | es.rs:258 `open_flac` → `Err(... "has not landed")`; not shipped |
| 09 | aiff | aiff | WIRED — real demuxer | mod.rs:53 `"aiff" => aiff::open(stream, opts)` |
| 10 | au | — | NOT WIRED | no `au` id minted (15 ids enumerated) |
| 11 | adts-aac | adts | WIRED — defective (raw-AAC session dies at 1st decoded frame, S2) | W13-A04 (W13 SUMMARY:55-56) |
| 12 | mp3-raw | mp3 | WIRED — real ES demuxer | mod.rs:66 `"mp3" => es::open_mpeg_audio(stream, opts, format)` |
| 13 | mpeg-es-video | — | NOT WIRED | es arms are flac/h264/hevc/mp2/mp3 only (mod.rs:62-66) |
| 14 | h264-annexb | h264 | WIRED — real ES demuxer | mod.rs:64 `"h264" => es::open_annexb(AnnexbCodec::H264, ...)` |
| 15 | hevc-annexb | hevc | WIRED — demuxer real, decoder refused | mp4-smoke.md:54-63 exit_code=65 |
| 16 | realmedia | — | NOT WIRED | census-only even on VLC side (fj:531) |
| 17 | midi-smf | — | NOT WIRED | no smf demuxer in CLI |
| 18 | archive-zip | — | NOT WIRED | no archive lane |
| 19 | adaptive-hls-dash | — | NOT WIRED | no adaptive demuxer |
| 20 | census-long-tail-demux | — | NOT WIRED (rollup) | 55 VLC demux registrations, none in CLI |

Totals: 21 container families → 12 wired real demuxers (3 with live S1/S2 defects) + 1 refusal
arm (flac-native) + 8 families the CLI refuses outright.
REVERSE GAP (the inventory undercounts Lumen): probe id `ivf` (mod.rs:55 `"ivf" => ivf::open(stream, opts)`, real demuxer,
W13-A05 executed `cargo test -p lumen-cli --lib containers::ivf` → 8 passed) has NO family
record in this 82-family inventory — rg `ivf` over formats.json: zero hits.

Non-container kinds vs shipped (summary):
- audio-codec 18: shipped per W13 true registry (14 descriptors) ≈ pcm-family, adpcm, mp3, mp2, aac-lc, alac, flac, vorbis, opus (9); NOT shipped: ac3-eac3, dts, truehd-mlp, wma-1-2-pro, wavpack, musepack, speex, midi-synth (+census rollup). dnxhd is shipped with no inventory family (same reverse-gap class as ivf).
- video-codec 12: shipped = mjpeg, h264, vp9, av1; NOT shipped: hevc (exit 65), vp8, theora, mpeg1-2, mpeg4-asp, wmv2, vc1-wmv9 (+tail).
- subtitle 12: ZERO wired — pipeline_host.rs:1237-1238 `// Subtitle/data limbs are not wired in the first vertical.` / `Frame::Subtitle(_) => Ok(())`; dispatch.rs:197-198 `"the subtitle path is a Phase-5 card"`.
- playlist 7: ZERO wired — dispatch.rs:191-192 `"the playlist model is a Phase-4 card"`; `lumen open` takes exactly one path (mp4-smoke.md:81-83).
- protocol 12: shipped = file (path/`file:` URL lanes) and http-https (ureq/rustls, demux_host.rs:357-358 "serves every `http(s)` source"); NOT shipped: directory, udp, rtp, rtsp (blocked-scheme refusal, demux_host.rs:840), smb, nfs, ftp-sftp, mmsh-mms, disc-family, tail.

Net: of 82 inventory families, Lumen ships real support for ≈27 (12 containers + 9 audio + 4 video + 2 protocols), while shipping 2 container/codec lanes (ivf, dnxhd) the inventory never lists.

## 4. Findings
F-001 [S2]
Where: docs/inventory/formats.json:9 (and every `log_files` entry, ~120 refs, e.g. fj:80-85)
Code:
```
      "evidence": {
        "logs": "docs/baselines/raw/linux/probe-work/logs/ — 61 .log files + status.txt (57 entries; 4 logs lack status rows: smoke_wav, http_server, rtp_ffmpeg, udp_ffmpeg)",
        "consolidation": "docs/baselines/raw/linux/17-format-probes.txt (one section per probe, verbatim decisive lines)",
```
What is wrong: the claimed preserved raw-log directory does not exist. `LS docs/baselines/raw/linux/probe-work/` = gen_py.py, run_probes.sh, dir/, pls/ — no logs/; Glob `**/mkv_h264_flac.log` (and the 17-probes cross-reference) → no results anywhere in the tree. Every probed family's `probe_evidence.log_files` therefore cites a dead path; the g5_statement's "a preserved -vv log" is only true for the consolidated decisive lines. Why it matters: the doc's evidentiary chain promises 61 inspectable logs and delivers 1 consolidation file; a reader auditing a single module-selection line must trust the consolidation's rg extraction, not the log. What correct looks like: either commit the logs or restate the pointer as "consolidated-only (17-format-probes.txt); raw logs ephemeral under tmp/probe (gitignored)". Confidence: HIGH — Verification: ran (LS/Glob/awk).
F-002 [S3]
Where: docs/inventory/formats.json:598-646 (census-long-tail-demux) and the absence of any ivf family across fj:66-2099
Code:
```
      "family": "census-long-tail-demux",
      "kind": "container",
      "vlc_modules": [
        "caf",
```
What is wrong: the container census omits IVF entirely (rg -i `ivf` formats.json → 0 hits) while the shipped CLI has a real IVF demuxer (containers/mod.rs:50 `"ivf" => ivf::open(stream, opts)`, W13-A05 verdict REAL, 8 tests executed including full AV1/VP9 decodes). The inventory's stated purpose is the coverage target; it undercounts Lumen's shipped container surface by one family and offers no census row (VLC side has no ivf-named module either) to anchor a future parity row. Why it matters: parity/scope math done off this file will say "Lumen covers 12/21 containers" when the true shipped set is 14 arms incl. ivf; roadmap consumers inherit the omission. Confidence: HIGH — Verification: ran (rg, cited W13-A05).
F-003 [S4]
Where: docs/inventory/formats.json:845, 1132
Code:
```
      "usage_share_rationale": "doc04 §3 P0; the audio codec of the free-web stack (Commmmons application/ogg 2.62 M)",
```
and
```
      "usage_share_rationale": "doc04 §4 P1 (legacy WebM;reasoned share via WebM corpus [VERIFY])",
```
What is wrong: copy defects ("Commmmons", "WebM;reasoned" missing space) in rationale strings that feed downstream docs. Cosmetic; blast radius limited to text reuse. Confidence: HIGH — Verification: sed -n '845p;1132p'.

## 5. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "61 .log files + status.txt (57 entries)" preserved at probe-work/logs/ (fj:9) | Repo filesystem: no logs/ dir | Filesystem — consolidation exists, raw logs do not (LS, Glob) |
| mkv family: decoders dav1d/avcodec/flac (fj:87-93) | 17-format-probes.txt mkv sections | Both agree — doc verified against consolidation (sed 110-135) |
| Inventory implies 21-container target scope | CLI ships 15 probe ids incl. ivf (W13) | Both true, different axes — the gap quantified in §3; ivf missing from inventory |

## 6. Fabrication / inflation indicators
- None found in the arithmetic: every count block recomputes exactly (82/21/18/7/12/12; 59/23).
- Negative results are recorded as negative (cue "NEGATIVE", midi decode-unsupported, rtp exit 124, ac3/dts passthrough-only caveats at fj:922/950) — the opposite of inflation.
- External usage stats are the only unverifiable tier, and the doc self-flags them ([VERIFY], Bitmovin 403, "reasoned") rather than laundering them as measured. Honest.
- The one inflated-feeling claim is F-001's "preserved" logs path — evidence-pointer inflation, not data inflation.

## 7. Notes for the next agent
- If you audit docs/inventory/formats.md (the prose sibling) or feature-matrix.toml rows citing probe-work/logs paths, expect the same dangling-path class as F-001 (Wave-0 xtask feature-audit flagged only 2 OTHER missing paths; this class is invisible to that tool).
- The W13 S1/S2 defects (ogg flac header 38→34, TS panic exit 101, adts first-frame death) mean "WIRED" in §3's table is not "WORKS"; do not reuse the table as a health claim without those riders.
