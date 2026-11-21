# WAVE 6 — SUMMARY (video codecs)

Date: 2026-11-21 (session 2) · Agents W06-A01…A10 · Domain: video codecs —
MJPEG, H.264 integration, HEVC/VP9/AV1/MPEG-1/2 parsers, vaapi hw lane, codec-api.
Gate: **PASS — 92/92 assignments audited (file + chunk), 0 missing, 0 thin,
citation spot-checks clean** (2 initial checker false-positives — regex ate code
fences — manually verified accurate against source; checker fixed).

## Verdicts (file/chunk level)
REAL 31 · REAL-UNPROVEN 4 · SHALLOW 1 · UNREACHABLE 56

## Findings
**S0:0 S1:1 S2:26 S3:106 S4:165 — total 298**

## The wave's hostile question: is H.264 decode reachable in the shipped build?
Convergent verdict of 7+ agents, each traced independently:
- **YES for software H.264 4:2:0**: `codec-h264-sw` is in the codecs crate's
  default features (crates/codecs/Cargo.toml:53) → CLI defaults
  (apps/cli/Cargo.toml:51) → release.yml builds `-p lumen-cli` with no feature
  subset (line 193) → dispatch at crates/codecs/src/pipeline_host.rs:346
  (H264SwDecoder). Executed: baseline sample-420.mp4 reaches Playing, 17/17
  differential decode.
- **NO for everything else video**: the in-house H.264 parser tree, the whole
  vaapi/hw lane (~5,000 lines incl. unsafe FFI, product.toml:168 `hw-decode=false`),
  HEVC (shipped binaries answer HEVC with `cli.codec_unavailable`, exit 65),
  MPEG-1/2, DNxHD (no descriptor producer mints a track that can reach it) —
  all UNREACHABLE in every shipped binary. The sw lane uses the external
  rust_h264 crate; the in-house "bit-accurate" parsers are dead code.

## Worst findings (citations in the per-file audits; sampled at gate)
1. [S1, unreachable-code caveat] vaapi session `fill_weights` ORs `1<<index`
   into a u8 for indices up to 31 — debug panic / silent bit alias on valid
   >8-entry weighted-prediction streams; zero coverage (W06-A10).
2. [S2] Four corpus-gated H.264 slice-parser tests (ffmpeg-trace differentials
   + 240-round mutation property) skip-pass **vacuously** — corpus-v4 is absent
   in this tree, so the baseline's "1681 passed" includes zero-execution passes
   backing docs' "3/3 corpus lanes PASS" claims (W06-A03; cross-proven W05-A09).
3. [S2] av1/fuzz.rs decode pass capped at a 255-byte prefix — the shipped P0
   AV1 decoder's fuzz net is structurally blind despite "full buffer" claims
   (W06-A01).
4. [S2] h264 PPS parser structurally rejects every High-profile PPS with the
   transform_8x8 extension (x264/ffmpeg High default) as a fabricated
   `h264.rbsp.tail` corruption error (W06-A08; consumers unreachable — sw lane
   uses rust_h264 — but the parser is advertised as the in-house lane).
5. [S2] HEVC sps.rs scaling_list_data reads 112 matrix entries where H.265
   §7.3.3 defines 20 — scaling-list streams mis-parse despite the "bit-accurate
   walk" doc claim (W06-A09).
6. [S2] mjpeg `receive_extend` (decoder.rs:967-982) — hostile DHT DC size 31
   overflows i32: debug panic; release emits a garbage frame as non-concealed,
   breaking the concealment policy (W06-A05).

## Session-2 recheck adds (audit/runs/baseline-recheck-2/)
The §7.1-exact ffmpeg sample (testsrc → libx264) is **yuv444p** (High 4:4:4
Predictive): `lumen open` parses, buffers to 100%, then
`error [decode.unsupported] unsupported in h264 (h264.sw.unsupported)` →
Buffering → Error — **and the process never exits** (survives SIGTERM; had to
SIGKILL; exit=124 at the 25s window; it also consumes stdin as commands after
entering Error). So: 4:2:0 plays; 4:4:4 is refused with a hang — a second,
distinct defect in the MP4 open path on top of W05's seek and language bugs.

## Blocked / UNVERIFIED (agents' items)
- corpus-v4 absence at baseline time (circumstantial; REQUEST-ORCH-RUN filed:
  corpus-mounted test rerun)
- doctests never executed in baseline (REQUEST-ORCH-RUN: `cargo test --doc`)
- tauri build sandbox-blocked (standing UNVERIFIED-BUILD)
- darwin compile-check claim (REQUEST-ORCH-RUN: `cargo check -p codecs --target
  aarch64-apple-darwin`)
- rav1d / libva / rusty_vp9 / filmcraft-dnx external-crate sources not in tree
  (record-cited only)
