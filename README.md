# Lumen Media Player (working name)

A clean-room, from-scratch reimplementation of VLC media player:
- **Media engine**: 100% Rust — playback state, probing, demuxing, decoding, filters, A/V pipelines, synchronization, streaming, devices, playlists, subtitles.
- **UI**: React + TypeScript (presentation only).
- **Desktop shell**: Tauri (OS webview is a documented exception to the Rust-only rule — this project is never described as "all-Rust").
- **Web build**: same engine compiled to wasm32 behind a typed binding; UI code shared with desktop.
- **Config-driven**: `product.toml` + schemas are the source of truth for product decisions.
- **Plugin-based**: built-in compile-time plugins + sandboxed WASM extension plugins.

Working name "Lumen" is a placeholder only — the product is fully rebrandable via
`product.toml` (see `docs/07-config-driven-system.md`, rebrand acceptance test).

## Status

**Phase 6 — Network & streaming (in progress).** Phases 0–5 are complete
(Phase 5: PASS WITH REGISTERED CAVEATS, Lead sign-off 2026-10-24,
[`docs/gates/phase5-exit.md`](docs/gates/phase5-exit.md)); the streaming
core, HLS, and DASH-parser work is landing now — see the Status block in
[`docs/ROADMAP.md`](docs/ROADMAP.md). The authoritative project brief is
[`docs/MASTER-PROMPT.md`](docs/MASTER-PROMPT.md). Nothing is claimed implemented or verified
until `docs/feature-matrix.toml` records automated evidence for it (statuses:
`not_started`, `in_progress`, `implemented`, `verified`, `blocked`).

Parity is defined against a pinned baseline, **VLC 3.0.23** (desktop), and is the
share of inventory features verified by automated evidence. No "exactly like VLC"
or "all formats supported" claims beyond that evidence.

## Repository layout (target — see docs/01-architecture.md)

```
AGENTS.md  Cargo.toml  rust-toolchain.toml  product.toml  product.schema.json
config/    themes/ keymaps/ layouts/ profiles/
locales/   en-US.toml ...
schemas/
apps/      desktop (Tauri)  web (wasm host)  cli  control-server
ui/        React + TypeScript workspace
crates/    media-types ... testkit (one responsibility per crate)
plugins/   builtin/  extensions/
tools/     xtask config-validator feature-audit dependency-audit license-audit
           parity-report bench-vs-vlc
tests/     fixtures integration playback compatibility security regression visual
fuzz/  benches/  docs/  design/  .github/workflows/
```

## Documents

| Doc | Subject |
|-----|---------|
| `docs/MASTER-PROMPT.md` | Authoritative project brief (wins over any earlier prompt) |
| `docs/ROADMAP.md` | Phase gates 0–8, wave program, decision register |
| `docs/01–15 *.md` | Domain plans (architecture, engine, codecs, UI, plugins, testing, …) |
| `design/figma/` | Pinned design exports (visual source of truth) |

## Ground rules (short form)

- Clean-room: no VLC source, comments, assets, logos, shaders or branding. Public
  specs, observed behavior, independent tests only. VLC may run externally as a
  test oracle; it must never be vendored.
- 100% Rust engine; UI is React+TS; Tauri shell; typed command layer is the only
  UI↔engine contract; truthful UI (accepted ≠ completed); config-driven; no
  native dynamic plugin loading; unsafe code denied in application/core crates.
- Every defect gets a regression test. Tests are never weakened to pass.
- Merge-blocking CI, honestly stated: **live today as one lean Linux job per
  workflow** (`.github/workflows/ci.yml` replicates the local gate;
  `ui.yml` covers the UI lane); the full Windows/Linux/macOS matrix is
  designed but parked under `.github/workflows-disabled/` pending Lead
  reconciliation.

## Licensing notes

- This repository's own code: **MIT** (decided 2026-10-19; [`LICENSE`](LICENSE);
  `docs/adr/ADR-006-license-mit.md`; decision D-009; closes risk R-05).
- The design package in `design/figma/` originates from a Figma Community file;
  its license terms were cleared by user attestation 2026-10-19 (risk R-07
  closed, decision D-010; residuals: font identities [VERIFY], trademark
  wording rules on shipped strings per `docs/14-risk-register.md`).
