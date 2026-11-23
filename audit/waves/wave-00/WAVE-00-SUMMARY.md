# WAVE 00 — Orchestrator Pre-Flight Summary

| Field | Value |
|---|---|
| Date | 2026-11-24 (branch audit/devil-20261124; sandbox clock UTC 2026-11-23) |
| Repo | https://github.com/codzxe/rust-vlc @ main e83e5bd3f56b8cea541761a7c60ed28f3d733eb1 |
| Files tracked | 3,279 (manifest.tsv; audit/ excluded) |
| Total LOC | 1,070,164 (Rust 445,395 / JSON 117,242 / markdown 94,768 / TS 79,659 / TOML 21,335 / shell 6,323 / Python 12,237) |
| Coverage proof | 3,279 assigned / 3,279 manifest / 0 missing / 0 extra (assignments.tsv) |
| Head of branch | pushed: audit/devil-20261124 (never main; gpush credential-helper only) |

## Orchestrator adaptations (recorded per §6.2/§7.4, driven by sandbox limits)
1. Sub-agents do NOT run git or touch credentials. The orchestrator commits
   and pushes centrally (parallel agents sharing one working tree would
   collide on index.lock; credentials stay single-point). Commit cadence is
   per-file via the orchestrator with §6.3 message format.
2. Machine is 2-core / 3 GB RAM / 8 GB disk. Workspace-wide cargo/npm
   commands run ONCE by the orchestrator into audit/runs/baseline/; agents
   cite those logs (AGENT-SPEC §2 load discipline).
3. `cargo test --workspace --locked` cannot link in one invocation here
   (disk exhaustion: linker SIGBUS — see
   cargo-test-workspace.attempt1-diskpoisoned.txt). Tests ran per-package
   with CARGO_INCREMENTAL=0, debuginfo=0 (semantics unchanged).
4. cargo-deny not installed (install exceeds load budget) -> cargo deny
   check is UNVERIFIED (tool missing). deny.toml exists and is assigned to
   Wave 1/17 for static audit.

## Baseline reality runs (verbatim logs in audit/runs/baseline/)
| Check | Command | Result |
|---|---|---|
| Release build (lane: release.yml) | `cargo build --release --locked -p lumen-cli` | PASS (2m28s) |
| Release build (lane: release.yml) | `cargo build --release --locked -p desktop` | PASS |
| desktop-tauri | `cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml` | FAIL @ gdk-sys pkg-config — ENVIRONMENT (no GTK/webkit in sandbox; ADR-004 expects webview hosts). UNVERIFIED, not a repo defect |
| Tests | per-package `cargo test -p X --locked` (34 pkgs) | 4,744 passed / **2 failed** / 51 ignored / 315 suites. Failures (first-vertical): `committed_transcripts_match_fresh_runs`, `final_frame_is_current` |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | CLEAN (exit 0) |
| fmt | `cargo fmt --check` | **FAILED** (exit 1) |
| xtask feature-audit | `cargo run -p xtask -- feature-audit` | **FAILURE — 2 violations** (evidence cites missing paths `crates/codecs/src/video/h264/sw`, `ui/src/screens/settings/areas/advanced.tsx`). Even path-existence checking fails |
| xtask parity-report | `cargo run -p xtask -- parity-report` | exit 0. **Parity 0/252 (0.0%) on all platforms** (windows 66/252, linux 61/252, macos 66/252, web 0/252). per-status cells: implemented=193, partial=147, blocked=129, not_started=503, unsupported=36 |
| xtask config-validator | `cargo run -p xtask -- config-validator` | **FAILURE — 4 violations** (brand icon/logo assets missing) — matches docs' "RED with 4" |
| UI | `npm ci` / `typecheck` / `test` / `build` | ALL PASS (vitest 136 files, 1,705 passed, 1 skipped) |
| full gate | `bash scripts/full_gate.sh <repo>` | **GATE RESULT: FAIL** — fmt FAILED; test FAILED (the 2 first-vertical failures); check-architecture FAILED (1/5); lint-literals FAILED (1); check-identity FAILED (1); sbom FAILED; web-budget FAILED (wasm32 rust-std missing at first run — environment); clippy/fuzz-registry/fixtures/packaging PASS |
| RC smoke | `bash scripts/rc-smoke.sh` | **FAIL overall**: wasm-web-build FAILED (env: rust-std wasm32 absent; passes after `rustup target add`, see addendum), all other checks PASS incl. headless-decode — **but its fixture is tests/fixtures/generated/valid/avi_mjpeg_pcm.avi (the one working vertical), not an MP4** |
| cargo deny | not run | UNVERIFIED (tool missing; load budget) |

## MP4 smoke test (the litmus) — verbatim in mp4-smoke.md
- `lumen open sample.mp4` (H.264+AAC, 320x240, 3s, moov-at-end): exit 0;
  state machine Idle→Opening→Buffering→Playing→Stopped→Idle in ~19 ms wall;
  tracks detected h264+aac; `language: Some("voe")` on both tracks (suspect
  misparse); `container: None`.
- `lumen decode-bench sample.mp4`: **v_frames=0 v_decoded=0 a_frames=0
  a_decoded=0** — "no decoder in this build (first vertical: mjpeg + pcm)".
- `lumen engine-info`: `decoders: mjpeg, pcm_s16le`; `features:
  first-vertical:avi-mjpeg-pcm, audio.null-device,
  video.software-compositor`.
- Contradiction inside one binary: open-path error text for HEVC claims
  "landed: mjpeg, pcm, flac, mp3, mp2, vorbis, opus, aac-lc, alac, adpcm,
  av1, h264" — engine-info and decode-bench say mjpeg+pcm only. One of the
  two registries lies. (Waves 5/6/13 must resolve.)
- faststart variant: exit 0 (same shell behavior). AAC-only: exit 0.
  HEVC: exit 65 codec_unavailable.
- Calibration: decode-bench on PCM WAV: a_frames=5 a_decoded=5 (instrument
  works); on AVI/MJPEG/PCM fixture (rc-smoke): v_decoded==v_frames==60.

## Anti-pattern census (loc-patterns.txt)
unreachable! = 4,126 · let _ = = 931 · unsafe = 660 · .ok() = 370 ·
unwrap_or_default = 210 · #[allow] = 121 · #[ignore] tests = 66 · expect( =
33 · unwrap( = 19 · panic! = 13 · todo! = 2 · eslint-disable = 26 · ts any = 6.
Top unwrap/unsafe concentrations listed in the census file.

## Sharding (assignments.tsv; domain map recorded per §8 remap authority)
W01 root+xtask+member-tools(47) · W02 core-types+commands(48) · W03
playback-engine+clock+queues(109) · W04 demux-I+probe(120) · W05
demux-II-mp4-priority(27) · W06 video-codecs+codec-api+renderer(223) · W07
audio-codecs+audio-engine(70) · W08 audio-output+filters+platform(62) · W09
subtitles(119) · W10 streaming+input+playlists+discovery+transcode(128) ·
W11 config+locale+themes(131) · W12 plugins+wasm+web(69) · W13
apps-cli+desktop(55) · W14 desktop-tauri(13) · W15 ui-src(400) · W16
tests+fuzz+e2e+infra(755) · W17 scripts+ci+packaging+release-docs(216) · W18
matrix rows 1-126+inventory(32) · W19 matrix rows 127-252+docs(674) · W20
special (re-verification, traces, scoring, report).

Remaps vs §8 guesses (real tree): media-probe→W04; video-engine→W03;
audio-engine→W07; renderer-wgpu→W06; platform-*→W08; control-server→W02;
doc 05→W02; tools/mem-audit+sweep+corpus→W16; tools/bench-vs-vlc+perf→W19;
docs subdirs dependencies/packaging/gates/release-*/security→W17;
inventory→W18. Matrix sharded 13 rows/agent (§7.1 chunking).

## Wave-0 verdict
The release binaries build and the project's own narrow vertical (AVI +
MJPEG + PCM to null output) demonstrably decodes. Everything beyond that
vertical is paperwork: parity 0/252, the MP4 litmus decodes zero frames,
the project's own gate and smoke harness are red on their own tree, and the
two halves of the CLI contradict each other about which decoders exist.
