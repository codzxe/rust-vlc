# CLAIM AUDIT: docs/soak/raw/scenarios-sandbox__seek-storm.md (credit: W19-A07, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Claims extracted | 9 |
| REAL 4 / REAL-UNPROVEN 1 / SHALLOW 1 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNREACHABLE 0 / UNVERIFIED 3 |
| Internal contradictions found | 1 (frames vs payload provenance, resolved in code) |

## 1. Blunt verdict
Internally consistent arithmetic (246309/15395 = exactly 16.0 seeks/cycle; 16.7 B/s × 3600 =
0.060 MB/h; 482−73 samples ≈ 407.0 s window) and a manifest that matches the repo copy. One
material wording defect: the "Decoding" line prints an ENGINE-side frame counter next to
sink-side payload bytes under the shared phrase "at the sinks" — for this scenario the sink
side is 0 bytes, i.e. the report offers zero sink-verified liveness evidence while implying
both numbers are sink-measured.

## 2. Claim ledger
| # | Claim (verbatim) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Manifest `/home/z/my-project/wt/w24b/tests/soak/scenarios-sandbox.toml` (5 scenarios) | :3 | REAL-UNPROVEN | path dead (`ls` -> No such file); repo tests/soak/scenarios-sandbox.toml has 5 kinds (rg 'kind = ' -> play-loop/seek-storm/playlist-cycle/open-close-churn/reopen-stress) |
| 2 | duration 8.00 min; fixtures avi_mjpeg_pcm + avi_mjpeg_2pcm | :20-21 | REAL | manifest:44-47 `kind = "seek-storm"`, `duration_min = 8.0`, fixtures match exactly |
| 3 | ops are WireRequest through soak dispatch seam onto real engine stack | :7 | REAL | crates/playback-engine/tests/soak/dispatch.rs:702 `pub fn lumen_soak_dispatch(host: &mut SoakHost, request: &WireRequest)` |
| 4 | fixtures' seek targets read from recorded expectation TOMLs | :7 | REAL | tests/fixtures/expectations/valid/avi_mjpeg_pcm.avi.toml `targets_us = [0, 500_000, 1_000_000, 1_999_000]`; same in avi_mjpeg_2pcm.avi.toml:22 |
| 5 | ceiling 50 MB/h checked, target 5 MB/h reported not asserted | :23 | REAL | manifest:29-31 `leak_ceiling_mb_per_h = 50.0` / `target_mb_per_h = 5.0` / `assert_target = false` |
| 6 | 15395 cycles/opens/closes, 246309 seeks ok / 0 failed / 0 skipped, 0 plays | :24 | UNVERIFIED (run) — consistent | 16.0 seeks/cycle exact; not re-run (cargo-build-budget) |
| 7 | "Decoding: 6214216 frames, 0 payload bytes at the sinks (the D-6 liveness axis)" | :25 | SHALLOW | generator soak_scenarios/report.rs:253 prints `counts.frames_decoded` (engine-side, runner.rs:615/747/782 accumulate host.stats()) + `counts.decoded_bytes` (sink-side); 0 bytes = no sink-verified decode during the storm — "at the sinks" only true of the second number |
| 8 | live slope 16.7 B/s = 0.060 MB/h; RSS 0.162 MB/h; PASS | :16,26-27 | UNVERIFIED (run) | unit arithmetic reconciles (60 120 B/h → 0.060); both below 50 MB/h ceiling and 8.0 MB rss budget (manifest:32-33) |
| 9 | "every scenario stayed inside its configured liveness and memory budgets" | :31 | SHALLOW | this file reports ONE scenario; suite siblings exist in docs/soak/raw/; wording overreaches the file's own scope |

## 3. Contradictions
| This doc says | Code says | Which is true |
|---|---|---|
| 6214216 frames "at the sinks" | report.rs:253-255 formats engine `frames_decoded` + sink `decoded_bytes`; sink bytes = 0 here | frames are engine-internal decode-to-target discards; sinks verified nothing in this scenario |

## 4. Fabrication / inflation indicators
None detected: dead worktree path is provenance rot; per-seek frame arithmetic (25.2 frames/
seek) and seek-target provenance are plausible and partially code-verified. Quotes verified
byte-verbatim via `sed -n '3p;24p;25p'` (output above).
