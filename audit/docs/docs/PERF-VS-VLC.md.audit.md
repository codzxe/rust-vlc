# CLAIM AUDIT: docs/PERF-VS-VLC.md (250 lines)
| Field | Value |
|---|---|
| Auditor | W19-A07 — RECOVERED by W19-R2 |
| Claims extracted | 15 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 11 / 2 / 0 / 0 / 0 / 0 / 2 |
| Internal contradictions found | 0 (one timeline drift vs later baseline, disclosed below) |

## 1. Blunt verdict
The rarest thing in this repo: a headline performance document whose first table is an
honesty ledger that says "NOT MEASURED" for wall-clock Lumen-vs-VLC and never invents a
number. Every numeric claim I cross-checked reproduces EXACTLY against its committed
artifact (29 MATCH-prefix rows recomputed by script; tails min/max 96–16384 B exact; all 6
f32 RMS/peak magnitudes exact; 16/19 input provenance split exact). The doc publishes its
own failures (0/6 f32 tolerance-FAIL, AV1 0.292×, h264/hevc no decoder) in the headline
table. No fabrication found.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Oracle pin "VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd) … version line recorded in docs/corpus/vlc-oracle-run-20261109.json" | :8 | REAL | `rg -n "Vetinari"` -> vlc-oracle-run-20261109.json:7 `"version_line": "VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)"` — byte-verbatim. |
| 2 | "No wall-clock Lumen-vs-VLC timing number exists anywhere in this repo" | :9 | REAL | breadth.md:10-12 header: every number "produced this session … never a silent skip" with VLC-absent oracle row; vlc-oracle-run json `inputs.note`: fixtures regenerated with `ffmpeg 7.1.5-0+deb13u1`; oracle-run `generated` used for differential only. rg sampling of docs/perf/** finds zero VLC timing row. |
| 3 | "35 comparable corpus rows: 29/35 bit-exact, 6/35 f32 tolerance-FAIL" | :17 | REAL | python3 recount of vlc-diff-report-20261023.md -> 29 MATCH-prefix rows, 6 f32-FAIL fixtures; report header :1 states "29/35 parity: 29 bit-exact prefix + 0/6 f32 tolerance-pass". |
| 4 | "tail of 96–16384 B left unflushed" (dump-method artifact) | :70 | REAL | python3 regex over report tails -> min 96, max 16384 (e.g. :350 tail 96 B; max-row 16384 B). EXACT. |
| 5 | f32 table rows (0022 RMS 1.2520e-2 … 0330 peak 1.1998e-3; 0/6 PASS) | :92-97 | REAL | vlc-f32-tolerance-20261109.json rows: rms_dev/peak_dev values match all 6 rows digit-for-digit (0.012520498…, 0.024954646…, …, 6.425152e-06, 0.001199807…); summary `{pass:0, fail:6}`. |
| 6 | Tolerance policy "RMS ≤ 1e-4 AND peak ≤ 1e-3 … ratified 2026-11-09" | :79-81 | REAL | same json `decision` block: policy string verbatim, ratified "2026-11-09 (Lead decision, ratified by the orchestrator)". |
| 7 | "16/35 regenerate sha256-identical … 19/35 … fresh realizations (double-run sha256 proof)" | :58-64 | REAL | oracle-run json inputs: `sha256_identical_to_manifest: 16`, `rebased_fresh_realization: 19`, double-run sha256s 6b7a4f3a… vs aee4767e… present. |
| 8 | §4.1 audio table (FLAC 46 421 260 / 526.3×; MP3 71 675 908 / 3 251×; MP2 105 655 092) | :138-140 | REAL | breadth.md:49-52 — identical values. |
| 9 | §4.2 MJPEG rows (0052 131 Mpix/s 0.477× 2.77×; uhd4k 207/384 0.539× 2.08×; 0124 125/265 0.472× 4.76×; 0087 378/553 0.685× 7.60×; AV1 68.41 Mpix/s, 112.9 fps, 0.292×) | :157-161 | REAL | breadth.md:84-87, :100-102 — identical values incl. "asm OFF" dependency-policy note. |
| 10 | X1b history "0052 3.182 s → 0.902 s … 0.760 s (8.7× end-to-end)" | :168-169 | REAL | breadth.md:90-91 verbatim. |
| 11 | §4.3 pipeline table (open 2.48–2.49 ms; seek 102.10/100.70, 404.35/705.30 ms; mkv 4.77 ms, 53 494 pkt/s, 0.11 calls/pkt) | :178-181 | REAL | breadth.md:121, :136-137, :164-167 — identical. |
| 12 | Harness "--oracle vlc … refuses (exit 2) without it; mock labels [MOCK]; pending emits exact commands" | :29-32 | REAL | tools/bench-vs-vlc/run.py:16-30 (docstring rules), :950-952 `except OracleUnavailable as err: ... return 2`; mock/pending classes at :201/:138. |
| 13 | VLC dump invocation "transcribed from … 14-options-full-help.txt L2335-2348" | :33-40 | REAL | `sed -n '2335,2348p'` -> `--audiofile-file <string>`, `--audiofile-format {u8,s16,float32,spdif}`, `--audiofile-wav` — the claimed lines hold those flags. |
| 14 | §6 "parity-report … windows=64/252, linux=64/252, macos=64/252, web=0/252; verified-only 0/252" | :204-207 | REAL (at its date) | PARITY-REPORT.md:109-112 machine line `rows=252 windows=64/252 linux=64/252 macos=64/252 web=0/252` + :119 "verified parity is 0/252, honestly". Timeline note: Wave-0 baseline (2026-11-23) xtask-parity-report shows windows 66/252 / linux 61/252 — post-2026-11-10 drift (later rows added), not a contradiction of this doc. |
| 15 | §6 "feature-audit — PASS … two stale evidence-path spellings fixed … commit c9a89c6" + §3 commits `76a700b`/`192ffc7` | :208-210, :78 | REAL-UNPROVEN / UNVERIFIED | Baseline xtask-feature-audit.txt:12 is FAILURE (2 violations, rows 23/163) — that run postdates 2026-11-10 (violations cite paths added later, per compat-matrix audit). Commit hashes unverifiable (no-git rule). PASS-at-date accepted on the strength of PARITY-REPORT.md's concurrent machine line, not re-run. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| feature-audit PASS (2026-11-10) | baseline FAILURE (2026-11-23) | Both, at their dates: baseline violations cite `crates/codecs/src/video/h264/sw` + `ui/.../advanced.tsx`, evidence added after this doc. Timeline drift, disclosed in PERF-VS-VLC §7 only implicitly. |
| parity-report 64/64/64 desktop | baseline 66/252 windows, 61/252 linux | Doc true at date (PARITY-REPORT.md:111 machine line); baseline true today. |

## 4. Fabrication / inflation indicators
None. The doc's §1 table writes "NOT MEASURED — no run exists" for its titular comparison
(Lumen vs VLC wall-clock) and points every timing number at the ffmpeg oracle instead.
§3 publishes the 0/6 FAIL outcome with per-row magnitudes; §5's summary uses ✗ for the
repo's own gap rows (h264/hevc decoder absent — consistent with baseline engine-info
"decoders: mjpeg, pcm_s16le").
