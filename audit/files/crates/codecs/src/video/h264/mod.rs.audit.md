# AUDIT: crates/codecs/src/video/h264/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 61 / 0 (module manifest: 5 mod decls + 3 re-exports) |
| File verdict | REAL (documentation manifest whose every concrete claim cross-verifies) |
| Reachable from shipped entry point? | YES — `sw::H264SwDecoder` (re-exported here) is the CLI's h264 lane (pipeline_host.rs:346); `decode`/`hw`/`parser` compile unconditionally |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg/sed cross-checks of every doc claim; baseline cites |

## 1. One-paragraph verdict
A 61-line module manifest (docs + `pub mod` + re-exports), no logic. Its value is documentation accuracy, and every concrete claim checks out: the two decode paths exist exactly as described (`decode::H264Decoder` hw path; `sw::H264SwDecoder` behind default-on `codec-h264-sw`, h264/mod.rs:55-56 + Cargo.toml), the "bit-exact vs ffmpeg 7.1.5 on six lanes, both entry forms" differential evidence is real (tests/h264_sw_differential.rs, baseline :1161-1181: 17/17 ok in 24.74s — six lanes a-f × annexb/mp4 against pinned ffmpeg yuv MD5s re-verified live when ffmpeg is present, which it is), and the totality/damage claims have in-crate evidence (video::h264::fuzz::tests 2/2 ok, baseline :645-646; hostile lanes in the differential). This file is also the CORRECT documentation that its own submodule hw/mod.rs contradicts (my h264/hw/mod.rs F-002: hw's header still says "No software decode lives here or anywhere under `h264`" while this file and the code say otherwise).

## 2. What actually works (verified)
- Claim "sw::H264SwDecoder — the software path (codec-h264-sw, default-on)": Cargo.toml default features include codec-h264-sw; `pub mod sw` cfg-gated at :55; re-export at :61.
- Claim "differential evidence: tests/h264_sw_differential.rs (bit-exact vs ffmpeg 7.1.5 on six lanes, both entry forms)": baseline :1161-1181 — lane_a_baseline/…/lane_f_still × {annexb, mp4} all ok + contract/hostile tests, 24.74s (live ffmpeg re-verification ran).
- Claim "Every entry point is total … no panics": h264 fuzz module tests (baseline :645-646) + differential's `hostile_garbage_packets_recover`, `hostile_truncation_sweep_is_panic_free` (baseline ok).
- Claim "With no accelerator negotiated it fails configure with the typed `h264.hw.required`": error code cross-referenced in hw/mod.rs docs and decode.rs (h264_hw_path tests, baseline :1186+ `no_accelerator_is_typed_unsupported_never_silent` for the HEVC mirror; h264 equivalent in the same suite).

## 3. Findings
None. What was checked and how: (a) module tree vs ls (decode/fuzz/hw/parser/sw all exist); (b) feature gating vs Cargo.toml; (c) the software-track paragraph vs the shipped CLI wiring (pipeline_host.rs:346 uses H264SwDecoder); (d) the differential-evidence paragraph vs the baseline run; (e) the damage-model paragraph vs the fuzz/differential test results. Zero defects found in the manifest itself.

## 4. Per-function table
Not required (<100 LOC, no functions). Modules declared: decode (unconditional), fuzz, hw, parser (unconditional), sw (cfg codec-h264-sw).

## 5. Test honesty
No tests in this file. The suites its doc points at are real and green in the baseline (§2).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Two decode paths live under this tree" | mod.rs:8 | REAL | decode.rs + sw.rs exist; CLI uses sw |
| "bit-exact vs ffmpeg 7.1.5 on six lanes, both entry forms" | mod.rs:20-22 | REAL | baseline h264_sw_differential 17/17 (24.74s, live oracle present) |
| "The `blocked` era of the `codec-video-h264-software` matrix row is closed" | mod.rs:22-23 | REAL (code side) — matrix verification is the parity-report's failure domain (0/252 verified on every platform, orchestrator baseline) | sw decoder shipped + CLI-wired |
| "any byte string yields either a parsed structure or a typed `parser::H264Error`" | mod.rs:43-44 | REAL (tested surface) | h264 fuzz tests + truncation sweeps, baseline ok |

## 7. Dead / duplicated / unreachable code
`decode`/`hw` (the hw path) are production-unreachable (my hw/mod.rs F-001) though compiled; `parser` is consumed by both sw and hw paths; `fuzz` is a pub harness.

## 8. Dependencies & flags
`codec-h264-sw` gates `sw` (default-on per codecs Cargo.toml). No other flags.

## 9. Open questions for the next agent
- decode.rs / sw.rs / parser/* auditors: this manifest promises `h264.hw.required` typed refusal and §7.4.1.2.4 AU assembly — verify in those files.
- Fix the stale hw/mod.rs:10 header (my h264/hw/mod.rs F-002) — this file already documents the truth.
