# CLAIM AUDIT: docs/formats/container-avi.md
| Field | Value |
|---|---|
| Auditor | W19-A02 (recovered by W19-R5) |
| Claims extracted | 8 |
| REAL 7 / REAL-UNPROVEN 1 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Everything checkable verifies: matrix row implemented ×3 (web not_started) with the avi
module as evidence; all seven self-built module files; the cargo-fuzz target + in-crate
harness; the fixture writer + the full fixture set incl. oracle sha256/structure sidecars
and the noidx0 class-B shape; the design-note quotes are byte-verbatim in mod.rs; the
session-script and first-vertical integration evidence exist. The first-vertical
headless-decode claim is corroborated by the ORCHESTRATOR baseline (rc-smoke check 4 PASS,
v_decoded==v_frames). Gate numbers themselves are at-time.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Matrix row container-avi: implemented win/linux/macos, web not_started, evidence crates/demux/src/avi/** | :9-11 | REAL | feature-matrix.toml:194-203 status implemented ×3 + web not_started; evidence string names the module + the 2.E1 ledger (rg). |
| 2 | self_built = demux::avi (mod/header/index/riff/tests/test_support/fuzz) | :20-21 | REAL | ls crates/demux/src/avi/ -> exactly those 7 files. |
| 3 | fuzz_target = fuzz_demux_avi (cargo-fuzz target + in-crate survival harness) | :16-18 | REAL | fuzz/fuzz_targets/fuzz_demux_avi.rs present; crates/demux/src/avi/fuzz.rs present. |
| 4 | Fixture corpus via tools/xtask/src/cmd/fixtures.rs with committed expectation ledgers (avi_mjpeg_pcm.avi + .oracle.sha256 + .structure.json; 2pcm; noidx0) | :44-48 | REAL | ls tests/fixtures/generated/valid -> avi_mjpeg_pcm.avi, .oracle.sha256, .structure.json, avi_mjpeg_2pcm.avi(+.structure.json); malformed/ -> avi_mjpeg_pcm__noidx0.avi (12 avi variants total); tools/xtask/src/cmd/fixtures.rs exists. |
| 5 | mod.rs design notes: "the movi walk is the packet-extraction truth", "declared sizes are hints, recognition is authority" | :41-43 | REAL | mod.rs:17 `**The `movi` walk is the packet-extraction truth.**` — byte-verbatim quote (rg). |
| 6 | First-vertical integration evidence: tests/integration/first-vertical/ (mock-labeled); CLI session pinned by apps/cli/tests/session_script.rs | :49-55 | REAL | ls -> evidence/ (ledger.json + 2.E1-first-vertical.txt + transcripts), src, tests; session_script.rs exists. |
| 7 | Release binary opens the parent fixture headless — headless-decode green, 60/60 video + 60/60 audio | :66-71 | REAL-UNPROVEN (corroborated) | run recorded at commit `ff001e5` (unverifiable, no-git); ORCHESTRATOR BASELINE audit/runs/baseline/rc-smoke.txt:129-131 "headless-decode (decode-bench; every frame must decode) ... PASS (v_decoded==v_frames, a_decoded==a_frames)" on the same AVI/MJPEG/PCM fixture class — the claim is baseline-executed. |
| 8 | Web column not_started although wasm host wires AVI open (apps/web/src/open.rs) | :78-80 | REAL | consistent with matrix web=not_started; apps/web exists (sibling lane evidence); honesty rule (column follows evidence, not code presence) applied as stated. |

## 3. Contradictions
None. The record's demux-only scope matches the module (no AVI writer anywhere in-tree —
rg for avi muxer symbols -> none in demux/src/avi).

## 4. Fabrication / inflation indicators
None — the record's own honest-limits section carries the Linux-only/mock-labeled caveats
the program-wide KNOWN-LIMITS records.

Commands run: rg container-avi + status/evidence matrix; ls avi module + fuzz_targets (fuzz_demux_avi.rs) + xtask fixtures.rs + session_script.rs; ls fixtures valid/malformed (avi_mjpeg_pcm*.avi + sidecars + noidx0); rg design-note quote mod.rs:17; ls tests/integration/first-vertical/evidence; baseline rc-smoke.txt:129-131 cited.
