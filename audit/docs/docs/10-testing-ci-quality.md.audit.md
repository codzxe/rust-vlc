# CLAIM AUDIT: docs/10-testing-ci-quality.md (L1-1025, both parts)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 16 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 5 REAL (lint block, CI-unblock history, eslint rule, fixtures generator/CORPUS, bench-vs-vlc + gates tools) / 0 / 0 / 0 / 6 PAPER-ONLY (suppressions registry, deps-check, contracts/dashboard/evidence-index/CI-matrix [PROPOSAL]s) / 2 STALE/FALSE-in-part (§4.1 ci.yml description; §6.8-vs-§1.5 fuzz gap) / 0 |
| Internal contradictions found | 2 |

## 1. Blunt verdict
This doc honestly declares itself a specification, not a report (:8 "No command listed here
has been executed"; :1022 "This document specifies; it does not report"), and that framing
holds: every "X passes" is future-tense. The verifiable spine is real: the workspace lint
block exists verbatim in Cargo.toml:53-66, the CI-unblock history is corroborated by
phase1/phase2-exit.md at the exact cited lines, ui/eslint.config.js enforces
local/i18n-no-literals, the fixtures corpus + CORPUS.md + deterministic generator exist, and
full_gate/ci.yml/scripts exist and ran (baseline full_gate.txt: GATE RESULT FAIL — the
honest-failure path works). Two defects. First, §4.1's status update misdescribes the
current reality: `.github/workflows/ci.yml` is no longer "a deliberately lean single-job
Linux pipeline ... commands 1–5 verbatim" — the file now has multiple jobs (lint/gate/
wasm-build, reconciled by issue #5, 2026-11-09), `.github/workflows-disabled/` no longer
exists (deleted; 11 live workflows), and "commands 1–5 verbatim" is wrong against BOTH
readings: §2.1's commands 1–5 are fmt/check/clippy/test/tree, while full_gate.sh runs
fmt/clippy/test + arch/fixtures + 5 more xtask sections and NO standalone `cargo check` or
`cargo tree`. Second, the doc's own quality rule is violated by the repo: §1 principle 5
says a parser may not reach `implemented` until its fuzz target exists, but
codec-audio-opus is `implemented` on all 3 desktop platforms (feature-matrix.toml:738-741)
and `fuzz/fuzz_targets/` has NO fuzz_codec_opus.rs (35 targets enumerated; opus absent) —
14 of the 25 codec fuzz targets listed in §6.8 do not exist.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Planning document. No command listed here has been executed." | :8 | ACCURATE framing (held) | Baseline runs now exist (audit/runs/baseline/*); doc never claims otherwise. |
| 2 | Root Cargo.toml lint block verbatim (§3.1 toml excerpt) | :191-205 | REAL — VERIFIED EXACT | `sed -n '53,66p' Cargo.toml` reproduces all 11 entries incl. `await_holding_lock = "deny"` (:66). |
| 3 | PAT lacked workflow scope; documented at phase1-exit.md:21 and phase2-exit.md:5,74-77 | :313-317 | REAL — VERIFIED | rg: docs/gates/phase1-exit.md:21 "...`.github/workflows-disabled/` per INFRA-BACKUP-1..."; phase2-exit.md:5 "...blocked on a workflow-scoped" + :74-77 "...the repo PAT lacks `workflow`". |
| 4 | "first live workflow is `.github/workflows/ci.yml` (W3-10): a deliberately lean single-job Linux pipeline ... commands 1–5 verbatim, keep-going aggregation preserved" | :319-323 | STALE + INACCURATE | Current ci.yml: multi-job (lint/gate/wasm-build; header :14-19 "THIS card (issue #5, 2026-11-09) reconciles..."), and ci.yml:32-38 itself states its parity target is full_gate.sh's SIX steps (fmt in lint job; clippy/test/arch/fixtures/fuzz-registry). No standalone `cargo check`/`cargo tree` anywhere in full_gate.sh (sed 1-80: sections 1-11). "keep-going" ✓ (fail=1 accumulation). |
| 5 | "parked under `.github/workflows-disabled/` since INFRA-BACKUP-1" | :316-317 | STALE | `ls .github/workflows/` -> 11 live workflows; no -disabled directory (deleted per ci.yml:14-19). |
| 6 | Local gate command list §2.1 (fmt/check/clippy/test/tree + ui/tauri/wasm) | :75-89 | SPEC — commands exist; gate status per baseline: fmt FAILED (cargo-fmt.txt exit 1), clippy CLEAN, test 4744/2/51 | audit/runs/baseline/cargo-fmt.txt + cargo-clippy.txt + cargo-test-workspace.txt; full_gate.txt "GATE RESULT FAIL". |
| 7 | Every crate carries `[lints] workspace = true` (§3.2) | :219-231 | Enforcement tool REAL; currently failing | xtask check_architecture.rs exists; baseline full_gate.txt lists arch as a RED section (check currently fails). |
| 8 | Suppression registry `tools/xtask/src/lints/suppressions.toml` (§3.3) | :241-243 | PAPER-ONLY | No `tools/xtask/src/lints/` directory (ls). Reality: 121 `#[allow]` in tree, unregistered (baseline loc-patterns.txt per AGENT-SPEC §6.4). |
| 9 | UI lint rules incl. `@lumen/i18n-no-literals` (§3.4) | :259-285 | REAL (name drift disclosed by [PROPOSAL] tag) | ui/eslint.config.js exists; :108 `"local/i18n-no-literals": "error"`. Baseline ui-test :146 i18n-no-literals.test.js 2 tests pass. |
| 10 | Fixtures layout `acquired/ generated/ expectations/ scripts/` + CORPUS.md (§5.1) | :519-526 | PARTIAL REAL | tests/fixtures/: CORPUS.md ✓, generated/ ✓, expectations/ ✓; NO acquired/, NO scripts/ (generators live in tools/xtask/src/cmd/fixtures.rs per CORPUS.md:4-5); extra dirs hls/, mjpeg_h3/ not in doc. |
| 11 | Fuzz registry: 35 planned targets incl. `_opus`, `_truehd`, `_rtsp_message`, `fuzz_probe`, `fuzz_commands_json` (§6.8) | :768-781 | PARTIAL — 35 of ~66 exist; opus/truehd/wavpack/speex/rtsp/probe/commands_json absent | `ls fuzz/fuzz_targets/` -> 35 files (aac, adpcm, alac, av1, flac, h264, hevc, mjpeg, mp2, mp3, pcm, vorbis + 12 demux + sub srt/ass/vtt + m3u/pls/xspf/cue + hls/http). Missing vs doc: codec opus/truehd/ac3/dts/wma/wavpack/speex/mpc/midi/rawvideo/mpeg2/theora/vp8/vp9; demux asf/m4v/caf/rm/tx3g; net rtsp; cross-cutting probe+commands_json. Layout fuzz/fuzz_targets/ not fuzz/. |
| 12 | "a parser crate may not reach `implemented` status until its fuzz target exists" (§1 principle 5) | :57-59 | VIOLATED by repo | feature-matrix.toml:738-741 codec-audio-opus status windows/linux/macos "implemented"; fuzz_codec_opus.rs does not exist. Internal contradiction (see §3). |
| 13 | Oracle harness `tools/bench-vs-vlc` + `tools/parity-report` (§6.6) | :720-721 | bench-vs-vlc REAL; parity-report relocated | tools/bench-vs-vlc/ exists (run.py, mock_vlc.py, src, test_run.py). parity-report = xtask subcommand (tools/xtask/src/cmd/parity_report.rs), not a tools/ dir. [PROPOSAL]-marked in doc. |
| 14 | deps snapshot `tools/deps-snapshot.txt` + `cargo xtask deps-check` (§2.3) | :135-137 | PAPER-ONLY | Neither exists (ls tools/; cmd/ has no deps-check). |
| 15 | Evidence index `docs/evidence/INDEX.md` generated (§4.5); dashboard `docs/reports/dashboard.md` (§9.3) | :389-390, 936 | PAPER-ONLY | docs/evidence/ and docs/reports/ do not exist (ls). |
| 16 | Runtime estimates ~20-60 min cold (§2.1) | :88-89 | UNVERIFIED (estimates, flagged [PROPOSAL]) | No measured medians exist; gate-report.json not found repo-wide. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| §1.5 (:57-59): parser crate may not reach `implemented` until its fuzz target exists | feature-matrix.toml:738-741: codec-audio-opus implemented w/l/m | The matrix is true about status, the doc's rule is violated: `ls fuzz/fuzz_targets/` -> no opus target (35 files enumerated). One shipped "implemented" codec has zero fuzz coverage. |
| §4.1 (:319-323): ci.yml = single-job, §2.1 commands 1-5 verbatim, .github/workflows-disabled/ parked | ci.yml:14-19 (multi-job reconciliation, parked dir deleted); full_gate.sh:1-80 (11 sections, no cargo check/tree) | The file and script are true; the doc's description is stale/imprecise on both counts. |

## 4. Fabrication / inflation indicators
- The doc's self-labeling prevents the classic doc-inflation pattern: it explicitly forbids
  reading its tables as results. No "passes" claim found that asserts a completed run.
- The §4.1 status update's "commands 1–5 verbatim" is the one place the doc asserts
  current-state facts about shipped files, and it is wrong in both readings — sloppy
  maintenance, not deliberate inflation (the real ci.yml documents its actual parity
  target honestly in its own header).
- The §6.8 registry is presented as a growth plan; the landed subset (35 targets) is
  smaller, which the doc does not summarize — combined with the §1.5 violation this is
  the closest thing to quiet overstatement in the file.

## 5. Not verified / limits
- GitHub Actions runs: cannot execute in sandbox; "first observed green run: owed by the
  Lead's push" (:325-327) remains UNVERIFIED (no CI log in repo).
- The runner-budget math (§10.4, ~670 h/month) is arithmetic on assumed PR counts —
  not falsifiable here; marked UNVERIFIED (no runner telemetry).
- Byte-level curiosity verified while auditing ci.yml: its header claims `branches: [main]`
  is intact and "ain]" is a display artifact — confirmed TRUE via `od -c` (bytes
  `[ m a i n ]` present at lines 102/104).
