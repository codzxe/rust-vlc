# BASELINE RUNS INDEX — Wave 0 pre-flight

All commands executed on 2026-11-21 (UTC) in an isolated sandbox clone of
`codzxe/rust-vlc` @ main `e83e5bd3f56b8cea541761a7c60ed28f3d733eb1`, branch
`audit/devil-20261121`. Rust 1.99.1 (repo pin). `CARGO_TARGET_DIR` shared.

## Verdict table

| # | Command | Exit | Result | Log |
|---|---|---|---|---|
| 1 | `cargo build --release --locked --workspace` | 0 | CLEAN, 3m46s | build-release-workspace.log.txt |
| 2 | `cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml` | 101 | FAILED — missing system libs (gdk-3.0/gtk pkg-config); sandbox has no GUI dev libs → tauri shell UNVERIFIED-BUILD | build-desktop-tauri.log.txt |
| 3 | `cargo test --workspace --locked` (release profile*) | 101 | **1681 passed / 1 failed / 24 ignored** — fail: `apps/desktop/tests/zero_alloc_transport.rs:98` zero-alloc claim (608 bytes allocated) | test-workspace.log.txt |
| 4 | `cargo test --workspace --all-features --locked` (release profile*) | 101 | **1751 passed / 2 failed / 25 ignored** — fails: `tests/integration/first-vertical/tests/evidence.rs:47` committed transcripts DRIFT from fresh runs (`open`, `play` scenarios) | test-workspace-allfeatures.log.txt |
| 5 | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | CLEAN | clippy-workspace.log.txt |
| 6 | `cargo clippy --workspace --all-targets --all-features` (via full_gate) | 0 | CLEAN | full-gate.log.txt |
| 7 | `cargo fmt --all --check` | 1 | **FAILED** — 1 dirty file: `crates/plugin-api/tests/testkit_wiring.rs` | fmt-check.log.txt |
| 8 | `cargo xtask feature-audit` | 1 | **FAILURE — 2 violations**: row 23 evidence cites missing `crates/codecs/src/video/h264/sw`; row 163 evidence cites missing `ui/src/screens/settings/areas/advanced.tsx`. Cells: implemented=193 partial=147 blocked=129 not_started=503 unsupported=36 | xtask-feature-audit.log.txt |
| 9 | `cargo xtask parity-report` | 0 | **TOTAL parity 0/252 (0.0%) on ALL four platform columns**; windows=66/252 linux=61/252 macos=66/252 web=0/252 | xtask-parity-report.log.txt |
| 10 | `cargo xtask config-validator` | 1 | **FAILURE — 4 violations**: brand icon assets (`icon.ico`, `icon.icns`, `icon-512.png`, `logo.svg`) do not exist | xtask-config-validator.log.txt |
| 11 | `cargo xtask check-architecture` | 1 | FAILURE — 1 edge violation: `lumen-cli` depends on `streaming`, not declared in boundaries.toml. cycles=0, unsafe-lints 39/39 inherit | gate-check_architecture.log.txt |
| 12 | `cargo xtask fixtures --check` | 0 | PASS | gate-fixtures___check.log.txt |
| 13 | `cargo xtask lint-literals` | 1 | FAILURE — 1 violation `apps/cli/src/host.rs:1799 identity` | gate-lint_literals.log.txt |
| 14 | `cargo xtask check-identity` | 1 | FAILURE — 1 violation (same site) | gate-check_identity.log.txt |
| 15 | `cargo xtask sbom --check` | 1 | **DRIFT** — committed `docs/security/sbom/cyclonedx.json` differs from regenerated (158 components, lock v4) | gate-sbom___check.log.txt |
| 16 | `cargo xtask web-budget --check` | 0 | PASS | gate-web_budget___check.log.txt |
| 17 | `bash scripts/packaging/gate.sh` | 0 | PASS | gate-packaging.log.txt |
| 18 | `bash scripts/fuzz_registry_check.sh` (via full_gate) | 0 | PASS — 35 targets, diff empty | full-gate.log.txt |
| 19 | `bash scripts/full_gate.sh` | 1 | **GATE RESULT: FAIL** (fmt real-fail; clippy+fuzz-registry real-pass; stages 3–11 poisoned by sandbox disk exhaustion mid-run — each rerun individually, see #3,#4,#11–17) | full-gate.log.txt |
| 20 | `RC_SMOKE_SKIP_WEB=1 bash scripts/rc-smoke.sh` | 0 | **PASS** — 6/6 engine checks (AVI first-vertical only, by design) | rc-smoke-engine-only.log.txt |
| 21 | `npm ci --no-audit --no-fund` (ui/) | 0 | OK | ui-npm-ci.log.txt |
| 22 | `npm run build` (tsc --noEmit && vite build) | 0 | OK — built in 386ms | ui-build.log.txt |
| 23 | `npm test` (vitest run) | 0 | **1705 passed / 1 skipped, 136 files** | ui-vitest.log.txt |
| 24 | `npm run tokens:check / settings:check / hotkeys:check / l10n:check` | 0 | 4/4 PASS | ui-*.log.txt |
| 25 | `cargo deny check` | 1 | advisories FAILED (config drift: unused wrapper `libappindicator-sys` for libloading); **bans ok, licenses ok, sources ok** | cargo-deny-check.log.txt |
| 26 | `cargo install cargo-deny --locked` | 0 | installed | cargo-deny-install.log.txt |

\* Test suites were run with `--release`: the sandbox disk (9.9 GB) cannot hold
both the release and the ~6 GB debug profile; identical test selection, only
optimization level differs. Recorded as an orchestrator infrastructure deviation.

## MP4 SMOKE TEST (the litmus test) — VERBATIM

Binary: `/home/z/my-project/target/release/lumen` (self-reports `lumen 0.1.3`;
`lumen engine-info` → `decoders: mjpeg, pcm_s16le`,
`features: first-vertical:avi-mjpeg-pcm, audio.null-device, video.software-compositor`).

### decode-bench (H.264+AAC MP4, moov-at-end) — mp4-smoke-decode-bench.txt
```
bench: video track#0 h264 — not benched (no decoder in this build (first vertical: mjpeg + pcm)); audio track#1 aac — not benched (no decoder in this build (first vertical: mjpeg + pcm)) (container mp4)
BENCH file=audit/runs/baseline/samples/sample.mp4 bytes=90732 v_frames=0 v_decoded=0 a_frames=0 a_decoded=0 wall_s=0.006 decode_s=0.000
EXIT=0
```
All variants identical (0 frames decoded): faststart (moov-at-front) `mp4-smoke-bench-mp4-faststart.txt`, HEVC `mp4-smoke-bench-mp4-hevc.txt`, AAC-only `mp4-smoke-bench-mp4-aac-only.txt`.

### interactive open (mp4-smoke-open-session.txt)
```
ok: accepted session=1 gen=0
event: state-changed Idle → Opening
event: tracks 2 ready
event: state-changed Opening → Buffering
event: TimingDurationKnown { duration: Timestamp(3000000) }
event: MediaInfoReady { ... tracks: [Video h264, Audio aac] ... }
event: error [decode.unsupported] unsupported in h264 (h264.sw.unsupported)
event: state-changed Buffering → Error
event: state-changed Error → Idle
EXIT=0
```
**An MP4 cannot be opened for playback. Demux works (2 tracks, duration, seekable), the decoder layer refuses H.264 ("h264.sw.unsupported" — the software H.264 decoder directory `crates/codecs/src/video/h264/sw` does not exist, confirmed by feature-audit violation row 23). The CLI exits 0 after a failed open.**

### contrast: AVI first-vertical (avi-firstvertical-open-session.txt)
```
event: state-changed Buffering → Playing
event: state-changed Playing → Stopped
event: state-changed Stopped → Idle
EXIT=0
```
`decode-bench` on `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi`: `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60` — the one real end-to-end lane (MJPEG+PCM in AVI), matching the docs' "first vertical" claim.

## Samples
Generated with ffmpeg 7.1.5 under `samples/` (all <100 KB):
`sample.mp4` (H.264+AAC, moov-at-end), `sample-faststart.mp4` (moov-at-front),
`sample-hevc.mp4` (H.265 4:2:0+AAC), `sample-aac.mp4` (audio-only).

## Hygiene counts (exact literal, tracked `*.rs`)
`.unwrap()` 15 · `.expect(` 33 · `todo!` 8 · `unimplemented!` 6 ·
`unreachable!` 4154 · `unsafe fn` 34 / `unsafe impl` 15 / `unsafe {` 316
(concentrated in platform-FFI crates and `codecs/src/video/h264/hw/{videotoolbox,vaapi}`, `codecs/src/video/av1/decoder.rs`) ·
`#[ignore]` 17 · `#[allow(` 121.
39/40 crates inherit `[lints] workspace = true` (exception: `apps/desktop-tauri`, workspace-excluded, policy mirrored inline per ADR-004).
See hygiene-counts.txt / hygiene-perfile.txt.

## Tooling gaps (UNVERIFIED items)
- `tokei`/`scc` unavailable; LOC counted per-file in manifest.tsv (980 Rust files / 445,395 raw lines; 3,279 tracked files total).
- `cargo llvm-cov` / `cargo mutants` not preinstalled — Wave 16 to decide feasibility.
- GUI (tauri) build blocked by missing system webkit/gtk dev libs — sandbox limitation, not a repo defect.
- full_gate.sh stages 3–11 originally failed on sandbox disk exhaustion; each was rerun individually (see table). full_gate.sh also defaults its root to `/home/z/my-project` — a hard-coded Lead-sandbox path (S3 finding for Wave 17).

## CORRECTION v2 (2026-11-21, after Wave-1 cross-examination) — READ THIS FIRST

Wave-1 auditors (W01-A01, W01-A09) refuted two Wave-0 conclusions. The
orchestrator re-verified both by hand and **confirms the correction**:

1. **The software H.264 decoder EXISTS.** `crates/codecs/src/video/h264/sw.rs`
   (25,964 bytes, rust_h264-based) is compiled by default and wired into the
   pipeline. Wave-0's claim "the software H.264 decoder does not exist in the
   tree" is FALSE and is withdrawn. The feature-audit violation "evidence
   cites missing path crates/codecs/src/video/h264/sw" is an evidence-STRING
   defect (the path points at a directory; the code is in sw.rs) — not absent
   code.
2. **The original MP4 smoke sample was invalid.** `sample.mp4` was generated
   from an RGB test source without a pixel-format conversion and ffprobe
   confirms `h264 High 4:4:4 Predictive, yuv444p`. The engine's sw decoder
   supports 4:2:0 8-bit only and refuses 4:4:4 BY DOCUMENTED DESIGN
   (sw.rs:54, sw.rs:367-371, sw.rs:527). The "MP4 cannot be opened" Wave-0
   headline was an artifact of a malformed sample.

### Corrected MP4 smoke matrix (all runs in this directory)

| Sample | ffprobe | `lumen open` result | exit |
|---|---|---|---|
| sample-420.mp4 (NEW, Constrained Baseline yuv420p + AAC) | h264 CB 4:2:0 | **Buffering → Playing → Stopped → Idle — PLAYS** | 0 |
| sample-420-faststart.mp4 (moov-at-front) | h264 CB 4:2:0 | (same lane; see open-session-420) | 0 |
| sample.mp4 (original, yuv444p) | h264 High 4:4:4 | tracks ready → `error [decode.unsupported] unsupported in h264 (h264.sw.unsupported)` → Buffering → **Error** → Idle | **0** |
| sample-hevc.mp4 (Main 4:2:0 + AAC) | hevc Main | Opening → **Error** → Idle (no HEVC decoder) | 0 |
| sample-aac.mp4 (audio-only AAC) | aac | Buffering → **Playing** → Stopped (audible output: none — null device) | 0 |
| AVI MJPEG+PCM fixture (contrast) | mjpeg/pcm_s16le | full lifecycle, 60/60 frames decoded in bench | 0 |

### Standing defects the correction EXPOSES (not retires)
- `lumen engine-info` reports `decoders: mjpeg, pcm_s16le` from a
  **hard-coded literal** (`apps/cli/src/host.rs` `engine_info()`, both entries
  `active: false`) — it under-reports the real registry (h264 sw decoder
  exists) and is not derived from live state. Wave-0 trusted this output.
- `lumen decode-bench` prints "no decoder in this build (first vertical:
  mjpeg + pcm)" for **every** MP4 track and decodes 0 frames even for the
  4:2:0 MP4 that `open` plays — the bench lane is hard-coded to MJPEG+PCM
  (`apps/cli/src/bench.rs` fall-through). The tool's output is misleading
  about build capability.
- The refused paths exit **0** (High 4:4:4 MP4, HEVC MP4): a failed open is
  reported as success by exit code.
- TrackInfo `language: Some("voe")` on tracks with no language metadata —
  garbage default (demux wave to trace).
- Audio "plays" only into the null device; no sound output path exists.
- Baseline `*.log` files are committed as `*.log.txt` — the repo's own
  `.gitignore:32` (`*.log`) silently excluded them from the first commit
  (found by W01-A03).

The original, now-corrected sections above are retained verbatim for
provenance. Wave-2+ agents: cite THIS section for MP4 reality.
