# WAVE 0 — PRE-FLIGHT SUMMARY

Date: 2026-11-21 · Orchestrator: single agent · Branch: audit/devil-20261121
Base: main @ e83e5bd3f56b8cea541761a7c60ed28f3d733eb1

## Inventory (manifest.tsv)
3,279 tracked files · 980 Rust files (445,395 raw lines) · 464 TS · 448 MD ·
155 TOML · 60 shell · 36 py · 885 binary/fixture assets · 252 feature-matrix rows.
31 crates, 5 apps (cli, desktop, desktop-tauri [workspace-EXCLUDED], control-server, web), ui/, tools/, fuzz/, tests/, docs/ (542 files), design/ (213).

## Baseline reality (full detail + verbatim outputs: audit/runs/baseline/INDEX.md)
- Release build (workspace, locked): **CLEAN** 3m46s. Tauri shell: cannot build in sandbox (missing gtk/webkit system libs).
- Tests (default features): **1681 passed / 1 failed / 24 ignored** — the zero-alloc transport claim test FAILS (608 bytes allocated).
- Tests (--all-features): **1751 / 2 / 25** — committed "evidence" transcripts DRIFT from fresh runs.
- Clippy (-D warnings, all targets, all features): CLEAN. fmt: **FAILED** (1 file).
- feature-audit: **FAILURE** — 2 evidence paths point at non-existent code (`h264/sw`, `advanced.tsx`).
- parity-report: **0/252 (0.0%) parity on every platform column** (windows 66/252, linux 61/252, macos 66/252, web 0/252 claimed partial).
- config-validator: FAILURE (4 brand assets missing — matches KNOWN-LIMITS).
- check-architecture: FAILURE (1 undeclared edge lumen-cli→streaming). sbom --check: DRIFT. lint-literals/check-identity: FAILURE (apps/cli/src/host.rs:1799). web-budget/fixtures/packaging/fuzz-registry: PASS.
- cargo deny: bans/licenses/sources OK; advisories config drift.
- UI: npm ci OK, tsc+vite build OK, vitest **1705 passed / 1 skipped**, 4/4 codegen checks PASS.
- rc-smoke (engine-only): PASS 6/6 — but only exercises the AVI MJPEG+PCM first vertical, by design.

## MP4 SMOKE (litmus) — VERBATIM
`lumen open samples/sample.mp4` (H.264+AAC, 320x240, 3s):
tracks 2 ready (h264/aac), duration 3.0s, seekable →
`event: error [decode.unsupported] unsupported in h264 (h264.sw.unsupported)`
→ `Buffering → Error → Idle`, **EXIT=0**. decode-bench: v_decoded=0 a_decoded=0
on all MP4 variants (moov-at-end, faststart, HEVC, AAC-only).
`lumen engine-info` admits: decoders = mjpeg, pcm_s16le; audio.null-device.
**An MP4 cannot be played. The software H.264 decoder does not exist in the
tree (feature-audit row 23 confirms the missing path). Contrast: AVI
MJPEG+PCM plays the full lifecycle (60/60 frames).**

## Sharding (assignments.tsv + coverage-proof.txt)
19 file waves × 10 agents, effort-balanced; 2394 file/chunk assignments;
885 assets in 23 groups; 252 matrix rows split W18 (1–126) / W19 (127–252);
files >800 LOC chunked at ~700 lines (all parts → same agent).
**Coverage proof: PASS — 0 missing, 0 extra, 252/252 rows.**

## Deviations
See ORCHESTRATOR-DEVIATIONS.md (token handling, orchestrator-only pushes,
release-profile tests, asset grouping, data-dump audits, tool gaps).

## Go/No-Go
GO for Wave 1.
