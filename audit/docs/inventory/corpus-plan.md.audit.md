# CLAIM AUDIT: docs/inventory/corpus-plan.md
| Field | Value |
|---|---|
| Auditor | W18-A06 |
| Date | 2026-02-11 |
| Claims extracted | 30 |
| REAL | 18 |
| REAL-UNPROVEN | 4 |
| SHALLOW | 0 |
| STUB | 0 |
| PAPER-ONLY | 3 |
| FALSE | 5 |
| UNVERIFIED | 1 (G6 corpus file-check extension) |
| Internal contradictions found | 2 |

## 1. Blunt verdict
This plan doc is unusually honest for this repo — its §7 hash ledger is
byte-exact (all 9 sha256s re-verified), and the v3-status table's numeric
claims about `tools/corpus/` reproduce exactly against the code. But the doc
has not been re-synced after the v4 escalation: the generator is now v4
(29 capability probes, 15 escalation classes), rotation is no longer
"blocked", fidelity.py's per-frame parser exists, and the §3 coverage sums
contradict the doc's own table (claims 60/22, table says 69/13). The
"evidence base" row cites **61 probe logs that do not exist anywhere in the
tree** — the only preserved form is the consolidated decisive-line extraction
in 17-format-probes.txt. The Status row still says "Nothing here has been
built", contradicted three paragraphs later and by `tests/fixtures/` +
`tools/xtask/src/cmd/fixtures.rs` on disk.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| C-01 | "Nothing here has been built — this is the plan that card 2.A3 (gated on 1.A3) and Phase-5 cards execute." | corpus-plan.md:8 | FALSE | `ls tools/corpus/` -> 8 files incl. gen_corpus.py; `ls tests/fixtures/` -> CORPUS.md, generated/, expectations/; audit/runs/baseline/full_gate.txt:54 "result: no drift — 55 file(s) verified". Card 2.A3 executed. |
| C-02 | "probe-work `gen_py.py` + `run_probes.sh` + 61 logs (pure-Python fixture generation proven)" | corpus-plan.md:6 | FALSE (logs component) | `find docs/baselines/raw/linux/probe-work -type f` -> 9 files, **0 logs**. No probe-work/logs/ dir. Log-derived evidence survives only as decisive-line blocks in 17-format-probes.txt:20 "The logs directory holds 61 .log files + status.txt (57 status entries)." — that directory is not committed. |
| C-03 | "formats.json (82 families, 59 probed — the matrix rows)" | corpus-plan.md:6 | REAL | python3 json load: families_total=82, probe_status probed=59 (format 82; counter output pasted in log). |
| C-04 | "modules-linux.json / modules-summary.md (57 demuxers, 32 subtitle, 21 playlist = fuzz-target population)" | corpus-plan.md:6 | REAL | modules-linux.json seven_category_rollup: demuxers 57, subtitle 32, playlist 21. Re-derived from raw dump 06-vlc-list-verbose.txt: demux 55 + demux_filter 2 = 57; spu decoder 22 + sub filter 1 + sub source 7 + text renderer 2 = 32 (the `spu decoder (-1)` line needs the negative-score regex). |
| C-05 | v3 delivered: "tools/corpus/ … gen_corpus.py + corpus_vocab.py + corpus_builders.py + minimize.py + fidelity.py + speed_bench.py" | corpus-plan.md:14-21 | REAL | `ls tools/corpus/` -> README.md, corpus_builders.py, corpus_vocab.py, fidelity.py, gen_corpus.py, minimize.py, run_harness.py, speed_bench.py (run_harness.py named at corpus-plan.md:39, also present). |
| C-06 | "(~2.6 kloc)" for those six files | corpus-plan.md:20-21 | FALSE (undercount) | `wc -l`: 1338+454+1663+187+243+209 = 5,094 lines. With run_harness.py: 5,507. Off by ~2x. |
| C-07 | "15 video × 8 containers, 17 audio × 12 containers, 29-res ladder … 0.5–240 fps, GOP anarchy, anamorphic SAR pool" | corpus-plan.md:32 | REAL | python3 import corpus_vocab: VIDEO_CODECS=15, CONTAINERS=8, AUDIO_CODECS=17, AUDIO_CONTAINERS=12, RES_ORDER len=29, FPS_POOL 0.5..240, GOP_KNOBS/anarchy at corpus_vocab.py:207, ANAMORPHIC_SAR=7. |
| C-08 | "62 stratification pins guarantee per-surface coverage" | corpus-plan.md:32 | REAL | STRATIFICATION=84 total; excluding V4_CLASSES pins = 62 (matches the v3 lane; gen_corpus.py:815 filters v4). |
| C-09 | "12 corruption ops (incl. cross-file splice-other, header size lies) + 22 garbage archetypes" | corpus-plan.md:33 | REAL | corpus_vocab: CORRUPTION_OPS=12 (list includes splice-other, head-size-lie), GARBAGE_ARCHETYPES=22. |
| C-10 | "multi-av (≤3 audio + ≤2 subs + ≤12 chapters), vfr-av (6 timestamp-chaos modes) … drift-av (45–110 s)" | corpus-plan.md:34 | REAL | corpus_vocab.py:363-365 MAX_AUDIO_TRACKS=3, MAX_SUB_TRACKS=2, MAX_CHAPTERS=12; corpus_vocab.py:180 six mode choices; corpus_builders.py:900 `dur = rng.uniform(45.0, 110.0)`; builders registered at corpus_builders.py:1655 incl. cover-audio, dynres-video, meta-bomb-av, hls, fmp4, raw. |
| C-11 | "class D … **Not started** (still none — nothing acquired)" | corpus-plan.md:35 | REAL | `ls tests/fixtures/` -> no curated/; tests/fixtures/CORPUS.md:46 "Curated third-party files (class D): none in v0". |
| C-12 | "speed_bench.py (ffmpeg decode-to-null baseline, pluggable runners, per-class median/p95, ratio-vs-ffmpeg)" | corpus-plan.md:36 | REAL | speed_bench.py:13 docstring + :93 `statistics.median`, :167/:181 p95, :188 ffmpeg-ratio; runner hooks at :16. |
| C-13 | "full plan frozen to `plan.json` before encoding; `--seed` / `--replay manifest.json`; resume after kill" | corpus-plan.md:37 | REAL | gen_corpus.py:13-14 (plan.json before encoding), :1281 `--seed`, :1282 `--replay`, :1313-1322 resume mode, :995 plan_path. |
| C-14 | "17 ffmpeg capability probes recorded in the manifest" | corpus-plan.md:37 | FALSE (stale) | gen_corpus.py:1268 "29 capability probes; byte-level post-ops; manifest v3 " — the shipped tool is v4 with 29 probes (probe_commands() + rotate_meta/vfr_mp4/hls_fmp4/concat_copy/rot_copy/rot_tkhd/odd1x1/still_frac). |
| C-15 | "manifest records params/cmd/sha256 + `ffprobe_ok`" | corpus-plan.md:39 | REAL | gen_corpus.py:973 `"ffprobe_ok": ffok,`. |
| C-16 | "expectation/1 TOMLs remain the fixtures-tree format" | corpus-plan.md:39 | REAL | tests/fixtures/expectations/valid/avi_mjpeg_pcm.avi.toml:5 `schema = "expectation/1"`; 3 valid + 22 malformed TOMLs present. |
| C-17 | "the harness (run_harness.py) computes oracle-side truth at report time" | corpus-plan.md:39 | REAL-UNPROVEN | run_harness.py exists (1,413 lines); not executed (would require ffmpeg corpus generation — cargo/ffmpeg budget not spent). |
| C-18 | "preset budgets 400 MiB (smoke) → 6 GiB (insane), gated at plan time" | corpus-plan.md:41 | REAL | corpus_vocab PRESETS: smoke budget_mb=400, insane budget_mb=6000 (python import output). |
| C-19 | "gitignored, regenerated by seed" for /corpus/, /corpus-smoke/, /corpus-big/ | corpus-plan.md:22-23 | REAL | .gitignore:98-105 contains `/corpus/`, `/corpus-smoke/`, `/corpus-big/` (plus `corpus-v4/` at :107 — undocumented in this doc). |
| C-20 | "per-frame PSNR series in `fidelity.py` summary mode (parses empty today)" | corpus-plan.md:43-44 | FALSE (stale) | fidelity.py:63-68 builds `per_frame` list via PSNR_RE; :82-88 result dict carries `frames`, `min`, `max`, `mean`, `inf_frames`. It parses and is not empty. |
| C-21 | "the Lumen decode-CLI runner hook for `speed_bench.py` (needs the decode-null card)" | corpus-plan.md:44-45 | REAL-UNPROVEN | speed_bench.py:15-16 "Challenger(s): any number of `--runner …` hooks (the Lumen decode CLI when it lands…)" — hook exists, Lumen runner not wired (consistent with claim). |
| C-22 | "rotation coverage blocked on the ffmpeg CLI (movenc drops `rotate=` metadata — needs an MP4 box editor)" | corpus-plan.md:45-47 | FALSE (stale) | gen_corpus.py:538 `def post_tkhd_rotate(path: Path, spec: dict, erng: random.Random) -> dict:` — the MP4 box editor exists; rot-display class (corpus_builders.py:1021, registered :1655), rot_tkhd probe (gen_corpus.py:313). |
| C-23 | "DASH packaging (MPD) as a class alongside HLS/fMP4" remains undone | corpus-plan.md:47-48 | REAL | KIND_FOR_CLASS (gen_corpus.py:103-128) has hls-av, fmp4-video; no dash class. |
| C-24 | §7 seed ledger: 9 rows (7 fixtures + gen_py.py + run_probes.sh) with sha256 + sizes | corpus-plan.md:353-364 | REAL | `sha256sum` re-run: all 9 hashes match byte-exact (m3u 61eca4…, pls 127eaa…, xspf d281ad…, asx c7830e…, b4s 8c29c9…, cue af17e9…, tone.wav 1ac726…, gen_py 377d52…, run_probes 089cae…); sizes match (59/103/303/176/216/89/8044 B). |
| C-25 | "Coverage sums: 82/82 families assigned … (60 with fixtures …, 22 REG until their phase). P0 families: 27" | corpus-plan.md:255-256 | FALSE (sums) | Parsed §3 table: 82 rows ✓, P0=27 ✓, but REG rows = **13** and fixture rows = **69** — the 60/22 split contradicts the doc's own table (13 REG families listed: realmedia, census tails x4, wavpack, musepack, smb, nfs, ftp-sftp, mmsh-mms, disc-family). |
| C-26 | §2 layout: "tests/fixtures/scripts/ … gen_valid.py gen_malformed.py gen_structural.py structmaps.py" | corpus-plan.md:136-139 | PAPER-ONLY | `ls tests/fixtures/scripts/` -> "No such file or directory". Superseded by the Rust port (tools/xtask/src/cmd/fixtures.rs exists; tests/fixtures/CORPUS.md:3 "Generated by `cargo xtask fixtures`"). |
| C-27 | §4 layout: generated/valid+malformed, MANIFEST.toml, expectations/, CORPUS.md | corpus-plan.md:300-309 | REAL | `ls tests/fixtures/` -> CORPUS.md, expectations/{valid,malformed}, generated/{valid,malformed,MANIFEST.toml,ivf}. (scripts/, curated/, bench/ from the same layout do NOT exist — consistent with C-26 and C-11.) |
| C-28 | "doc 11 §8.1.4 `tools/license-audit` cross-checks rows against non-VLC sources" | corpus-plan.md:327-328 | PAPER-ONLY | `ls tools/license-audit` -> "No such file or directory". No such tool in tree. |
| C-29 | "G6 feature-audit extends its file check to the corpus (fixture without ledger row = CI fail)" | corpus-plan.md:325-327 | UNVERIFIED | Could not find a fixtures-ledger check in the G6/feature-audit tool; baseline xtask-feature-audit.txt is FAILURE (2 missing evidence paths — matrix evidence, not fixtures). fixtures --check itself PASSES (full_gate.txt:54-55). |
| C-30 | "v1 committed-corpus budget: < 50 MB total `[PROPOSAL]`" | corpus-plan.md:332 | REAL-UNPROVEN | Marked [PROPOSAL] in-doc; not adjudicated (§8 Q1 open). du of tests/fixtures not run (cheap but claim is a proposal, not a fact — no verification needed). |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| corpus-plan.md:8 "Nothing here has been built" | corpus-plan.md:10-41 "the randomized side has been delivered by the v3 suite" | The delivery is real: tools/corpus/ + tests/fixtures/ exist; full_gate.txt:54-55 "no drift — 55 file(s) verified". The Status row is stale. |
| corpus-plan.md:255-256 "60 with fixtures … 22 REG" | corpus-plan.md:172-253 table | The table wins: 69 fixture rows, 13 REG rows (python row-parse, lists pasted above). |
| corpus-plan.md:37 "17 ffmpeg capability probes" | gen_corpus.py:1268 "29 capability probes" | The code wins (29). The plan was written against v3. |

## 4. Fabrication / inflation indicators
- Evidence inflation: "61 logs" cited as evidence base; zero log files exist in
  the tree (only the consolidated txt + 9 probe-work files). The probe-work
  artifacts are real; the log count as an in-repo evidence claim is not.
- Numeric drift without re-sync: 2.6 kloc (actual 5,094), 17 probes (actual
  29), 60/22 coverage split (actual 69/13). All errors understate or mis-split
  rather than inflate capability — this doc errs toward the conservative side,
  unlike the feature matrix.
- Positive honesty signals: [PROPOSAL]/[VERIFY] markers used consistently;
  §7 hashes byte-exact; class-D "not started" admitted; fidelity docstring
  itself is slightly generous (fidelity.py:13 promises "min/median/mean +
  worst-frame index"; the result dict at fidelity.py:82-88 has min/max/mean/
  frames/inf_frames — no median, no worst-frame index — S4 hygiene).

## 5. Matrix rows
Not a matrix chunk — n/a (rows 105-117 audited separately in
audit/docs/feature-matrix.toml.rows-105-117.audit.md).

## 6. Commands run (all exit 0 unless noted)
- sha256sum on 9 probe-work files (all match §7)
- python3: formats.json counts (82/59 probed); modules-linux.json rollup
  (57/32/21); raw 06-vlc-list-verbose.txt capability re-count (502 modules)
- python3 import corpus_vocab (STRATIFICATION 84/62, CORRUPTION_OPS 12,
  GARBAGE 22, VIDEO 15, AUDIO 17, containers 8/12, PRESETS 400/6000)
- wc -l tools/corpus/*.py; ls tests/fixtures, tools/xtask/src/cmd
- find probe-work (9 files); ls probe-work/logs (No such file — exit 2)
- rg/verbal verification of gen_corpus.py:1262-1268, 538-539; fidelity.py:63-68
