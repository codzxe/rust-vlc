# CLAIM AUDIT: docs/corpus/compat-matrix-20261110.json (L1-2986, chunks part1-4)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Claims extracted | 22 (1 retracted) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERACHABLE | 12 / 6 / 0 / 0 / 0 / 1 / 0 (4 UNVERIFIED sub-items inside) |
| Internal contradictions found | 1 (counts block vs raw rows, resolved by bucketing — see ledger #4) |

## 1. Blunt verdict
This is the most honest measurement document audited so far in this ledger: lanes that did not run are stamped blocked with explicit "NO result is inferred" notes, the web lane claims only a build + no decode, the oracle differential reports its own failures (6 f32 tolerance-FAIL rows with rms/peak numbers), lossy codecs are explicitly not byte-compared, and no transition proposes `verified`. The one FALSE claim in the ecosystem this report feeds is external: the shipped binary's `engine-info` hardcodes `decoders: [mjpeg, pcm_s16le]` (apps/cli/src/host.rs:1643-1661) while the registry this report exercised dispatches ~15 codecs — the compat-matrix decode claims (vorbis/opus/aac/av1/flac) are consistent with the code but cannot be re-executed here (no cargo budget) → REAL-UNPROVEN, with a disclosed hand-edit of the machine record (S3) and a stale bench tool message ("no decoder in this build (first vertical: mjpeg + pcm)", apps/cli/src/bench.rs:334) that makes the orchestrator baseline under-report decode capability.

## 2. Claim ledger
| # | Claim (verbatim, ≤25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | `"generated": "2026-11-10T19:27:42Z"`, `"git_commit": "3dfe1c0b…"` | :3,8 | REAL-UNPROVEN | Timestamp plausible; commit UNVERIFIED (git forbidden to agents). Raw rows carry worktree path `/home/z/my-project/wt/w24d/` (branch w24d provenance). |
| 2 | corpus: committed 46 + malformed 54 + regen 66 = 166 files | :10-12 | REAL (arithmetic) | 46+54+66=166 = `files_total`; raw-rows.jsonl has exactly 166 lines (`wc -l`). |
| 3 | lanes: only linux-native ran | :457 | REAL | Lane labels verified: linux-native "the lane that actually ran"; windows/macos `kind: blocked` (:500,:512) citing absent 3-OS runners. |
| 4 | counts: decoded 84, partial 22, refused_probe 2, refused_demux 4, demux_fatal 0, malformed_typed 54 | :2916-2931 | REAL (after reconciliation) | Raw rows: decoded 97 / partial 33 / probe 4 / demux 24 / fatal 8 over all 166; separating the 54 malformed rows (13 decoded, 11 partial, 2 probe, 20 demux, 8 fatal) yields exactly 84/22/2/4/0 for the 112 valid files. Counts block is a valid re-bucketing, not a fabrication. |
| 5 | "54 malformed all-typed / zero crashes" (md :28) | md:28 | REAL-UNPROVEN | All 54 malformed rows carry typed statuses (verified in raw rows); zero crashes asserted — cannot re-run (no cargo). |
| 6 | linux-native decoded: vorbis 4, opus 4, aac 23 (142 concealed), av1 2, flac 3, mp3/mp2 1+1 tracks | formats/codecs cells | REAL-UNPROVEN | Registry arms exist: apps/cli/src/pipeline_host.rs:241-338 ("vorbis"→VorbisDecoder::new etc.); runner drives real open path (tests/compat/src/main.rs:9-11 → `lumen_cli::demux_host::open_source` verified at demux_host.rs:362). Not executed here (cargo-build-budget). |
| 7 | h264 video: "0 tracks decoded across 12 files… no decoder in this build" (2026-11-10) | codec cell | REAL (as of run date) | H.264 decoder admitted 2026-11-15 per pipeline_host.rs:330-334 comment ("Lead-admitted 2026-11-15 — rust_h264 0.6.1"); consistent with pre-11-15 refusal. |
| 8 | h264 annexb format: "1/1 files open+demux (0 fully decoded, 1 NoDecoder track)" | format cell | REAL (honest demux-only) | Demux es covers h264 (containers/es.rs:266 comment). |
| 9 | asf: open path REFUSES corpus files → transition implemented→partial | transitions | REAL | Applied: docs/feature-matrix.toml:215 `linux = "partial"` + notes record the 2026-11-10T20:07:51Z triage verbatim. |
| 10 | flac format refused: raw-FLAC demuxer not landed | format cell | REAL | matches code comment containers/es.rs:266 "raw-FLAC … has not landed". |
| 11 | web-wasm: "the wasm32 module builds (exit 0, 19.85s)"; ships decoder-less | lane steps | REAL-UNPROVEN | Corroborated by baseline rc-smoke (wasm build passes; audit/runs/baseline/rc-smoke.txt) and apps/web capability split; not re-run here. |
| 12 | oracle: 29 bit_exact_prefix, 0 tolerance-pass, 6 tolerance-FAIL | counts + oracle.rows | REAL | Recounted from oracle.rows: 29 bit_exact_prefix + 6 tolerance_fail = 35 rows. Failures honestly reported with deviations (e.g., row 1: rms 0.0125 vs max 1e-4). |
| 13 | "Nothing here self-verifies: no transition proposes `verified`" | notes | REAL | All 5 transitions are implemented→partial (compat-matrix-transitions-20261110.json). |
| 14 | "Transitions are PROPOSALS… only `cargo xtask compat-matrix apply-transitions` writes the matrix" | notes | REAL | applied.jsonl + matrix rows now partial (asf :215, aac-lc :673, alac :720, vp9 :480, pcm :634). |
| 15 | "POST-RUN EDITORIAL NOTE … cell details … surgically updated" | :2984 | REAL (disclosed hand-edit) | A machine measurement record was hand-edited post-run (avi/wav cell details). Disclosed, raw jsonl claimed untouched — raw rows are indeed inconsistent with pre-edit details only in detail strings, and counts reconcile. Integrity concern, not fabrication. |
| 16 | regen provenance: "Rows 1-35 … 19 of 35 are runtime-random realizations, honestly non-pinned" | corpus.regen_provenance | REAL | regen_outcomes: 31 present_verified + 35 present_random + 1 failed = 67 listed; regen_files=66 vs 67 outcomes — one failed regen. Minor off-by-one, not material. |
| 17 | Method (md:10): "real decode through the player's open path (open_source → container registry → HostDecoderProvider → codec_api::Decoder)" | md:10 | REAL | All four symbols exist and are wired (demux_host.rs:362, pipeline_host.rs:235, codec_api). |
| 18 | engine-info would report the decoders this run exercised | (implicit ecosystem claim) | FALSE (external) | apps/cli/src/host.rs:1643-1661 hardcodes `decoders: vec![mjpeg, pcm_s16le]` under a comment claiming "(class S, data-driven)" — static list, not derived from the 15-arm registry; the orchestrator baseline (mp4-smoke) thus under-reports. This is the "which registry lies" trace: engine_info() lies. |
| 19 | bench "no decoder in this build (first vertical: mjpeg + pcm)" | bench.rs:334 | FALSE (stale tool message) | apps/cli/src/bench.rs:326-334 wires only mjpeg/pcm benches and blames the build; the player path has h264/aac/etc. decoders (pipeline_host.rs). The baseline decode-bench v_frames=0 is this tool's limitation, not proof of missing decoders. |
| 20 | (RETRACTED 2026-11-24) claimed `#[must_use]` attribute corruption repo-wide | n/a | RETRACTED | Display-pipeline artifact (auditor output eats open-bracket+lowercase-m pairs); hex-dump proof shows valid `#[must_use]` bytes (host.rs:1630 hex 23 5b 6d 75 73 74 5f 75 73 65 5d). No corruption exists. |
| 21 | windows/macos "blocked" lanes cite absent runner machines + CI billing | :500-512 | REAL (planning record) | reason_ref docs/plans/phase8-cards.md §1 exists (referenced; not re-verified line-wise — UNVERIFIED detail). |
| 22 | lane label: web wasm "DECODE refused for every codec" | web-wasm cell detail | REAL-UNPROVEN | apps/web capability split corroborates; not executed. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| linux-native decoded aac/vorbis/opus/av1/flac (2026-11-10) | Orchestrator baseline (2026-11-23): "engine-info: decoders: mjpeg, pcm_s16le"; decode-bench v_frames=0 | Both artifacts are explained: engine_info() is a hardcoded 2-entry vec (host.rs:1643-1661) and bench.rs only benches mjpeg/pcm (bench.rs:326-334). The registry code supports the compat-matrix claims; the two self-report surfaces lie/stale. Final runtime truth UNVERIFIED (no cargo here). |
| counts demux_fatal=0 | raw-rows.jsonl has 8 `demux_fatal` rows | Both true under different bucketing: the 8 all belong to the 54-file malformed corpus, bucketed into malformed_typed. Verified by recount (ledger #4). |

## 4. Fabrication / inflation indicators
1. "class S, data-driven" engine_info() returns a hardcoded constant list (host.rs:1631-1661) — the doc ecosystem's decoder inventory self-report is fabricated-by-hardcode.
2. The compat-matrix itself carries a disclosed post-run hand-edit (:2984) — machine record integrity is human-edited; mitigated by immutable raw-rows.jsonl (which I reconciled).
3. regen_files=66 vs 67 regen_outcomes (1 failed) — trivial off-by-one, honest (failures listed).
No count inflation found: every headline number reconciles against the 166 raw rows.

## 5. Matrix rows
Not a matrix chunk; format/codec `claims[]` map to feature-matrix rows (container-avi, container-asf-wmv, codec-audio-pcm, …) — spot-checked statuses cited match the live feature-matrix.toml (asf row cites implemented×3 pre-transition; post-transition linux=partial applied and recorded in the row's notes verbatim).
Commands run: python3 json aggregation (above outputs), `wc -l` raw-rows (166), rg over apps/cli/src/{host,pipeline_host,bench}.rs, feature-matrix row greps. Exit 0 unless noted.
File-level findings: S2: 1 (F-018 engine_info hardcode — ecosystem claim), S3: 2 (F-015 hand-edited machine record; F-019 stale bench message), S4: 1 (regen 66 vs 67).
