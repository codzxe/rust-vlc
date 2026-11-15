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

**Phase 8 — Hardening & release: in progress (2026-11-15, wave 30 — the
remediation wave after the 12-agent read-only audit).**
Phases 0–6 are complete (Phase 5: PASS WITH REGISTERED CAVEATS, Lead
sign-off 2026-10-24, [`docs/gates/phase5-exit.md`](docs/gates/phase5-exit.md);
the Phase-6 exit ledger is drafted with verdict PENDING LEAD DECISION,
[`docs/gates/phase6-exit.md`](docs/gates/phase6-exit.md)). Phase 7 landed
across waves 20–23 (merge evidence: `git log --oneline --grep="merge wave"`):
platform adapter crates (#40), hw-decode adapter lanes compile-verified
(#41), device hotplug + discovery (#42), transcode graph core (#43),
recording + MP4 remux (#44), wasmi plugin host + crash isolation (#45/#46),
apps/web v1 (#47) + its render/audio/io pillars (#48), control server
(#50), the tag-triggered release pipeline (#56), the TLS decision record
(#39), and the security sweeps + CycloneDX SBOM (#52). **Releases v0.1.0,
v0.1.1 and v0.1.2 are published** (2026-11-13 → 2026-11-15; the first
real-run record with its two iterations is
[`docs/RELEASE.md`](docs/RELEASE.md)). Since v0.1.0: H.264 software decode
landed (w27 — bit-exact vs the ffmpeg oracle), the real `lumen-tauri`
Windows app shipped (w28, v0.1.1), and wave 30 — the remediation wave
after the 12-agent read-only audit — lands the Windows fixes: every local
file opens by path or `file:` URL (the v0.1.2 open blocker is gone), the
GUI shows its failures (crash dialog + log) instead of silent-exiting, GUI
playback actually progresses (the ~30 ms pacer), installers are
launch-proven with a Desktop shortcut, plus MP4 hardening (sample-table
caps, PCM-in-MP4 for ffmpeg≥7 files) and streaming auth hardening. What
plays today, honestly:
**38 of the 73 format rows** in [`docs/feature-matrix.toml`](docs/feature-matrix.toml)
are claimed (implemented or partial) on desktop (14 containers, 7 video
codecs, 9 audio codecs, 8 subtitle formats) — **zero on web** — and the
`lumen` CLI drives them headless (decode + sync real). The honest product
gaps: **audio output is the ADR-005 null device — no sound on any OS yet**;
the desktop app opens any local file but **displays no video yet** (the UI
mounts an honest empty media surface; the canvas path is built but
unwired); **URL sources (http/hls) are refused with a clear message** (the
streaming engine exists; enablement is the next wave). See
[`docs/KNOWN-LIMITS.md`](docs/KNOWN-LIMITS.md),
[`docs/ROADMAP.md`](docs/ROADMAP.md) and [`NEXT-STEPS.md`](NEXT-STEPS.md).
The authoritative project brief is
[`docs/MASTER-PROMPT.md`](docs/MASTER-PROMPT.md). Nothing is claimed
implemented or verified until `docs/feature-matrix.toml` records automated
evidence for it (statuses: `not_started`, `in_progress`, `implemented`,
`verified`, `blocked`).
## Install

**v0.1.0–v0.1.2 are published** (billing was unblocked 2026-11-12; the
tag-triggered pipeline's first real run and its honest limitation record
are in [`docs/RELEASE.md`](docs/RELEASE.md)); **v0.1.3 is being cut** with
the wave-30 Windows fixes — the curated notes draft is
[`docs/release-notes/skeleton-v0.1.3.md`](docs/release-notes/skeleton-v0.1.3.md).
Every release ships `SHA256SUMS.txt` — verify first:

```sh
sha256sum --check SHA256SUMS.txt        # GNU coreutils (Linux, git-bash)
shasum -a 256 --check SHA256SUMS.txt    # macOS
```

Two programs ship in every artifact: `lumen` — the CLI, the real engine
surface (demux/decode/playback, headless) — and `lumen-tauri` — the
desktop app (the real engine host in an OS-webview shell, shipped since
v0.1.1). From v0.1.3 the `lumen-desktop` exit-86 stub leaves every user
payload (wave 30; through v0.1.2 it shipped as a third binary). Binaries
are unsigned — expect Windows SmartScreen warnings and the macOS
Gatekeeper right-click → Open workaround (the honest register:
[`docs/RELEASE.md`](docs/RELEASE.md)).

**Build from source** remains fully supported (see
[`docs/guides/getting-started.md`](docs/guides/getting-started.md) for a
walkthrough): clone the repo, then `cargo build --release --locked -p
lumen-cli -p desktop` (toolchain pinned by `rust-toolchain.toml`); the
desktop app builds from `apps/desktop-tauri` (own lockfile; the recipe is
its `INFRA-BLOCKED.md` §C); the packaging scripts under `scripts/release/`
build the per-OS artifacts listed in
[`docs/RELEASE.md`](docs/RELEASE.md) from those binaries.
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
| `docs/guides/` | User documentation (getting started, settings, shortcuts, troubleshooting) |
| `docs/release-checklist.md` | The executable end-to-end release checklist (RC smoke harness) |
| `design/figma/` | Pinned design exports (visual source of truth) |

## Ground rules (short form)

- Clean-room: no VLC source, comments, assets, logos, shaders or branding. Public
  specs, observed behavior, independent tests only. VLC may run externally as a
  test oracle; it must never be vendored.
- 100% Rust engine; UI is React+TS; Tauri shell; typed command layer is the only
  UI↔engine contract; truthful UI (accepted ≠ completed); config-driven; no
  native dynamic plugin loading; unsafe code denied in application/core crates.
- Every defect gets a regression test. Tests are never weakened to pass.
- Merge-blocking CI, honestly stated: the workflow set (`.github/workflows/`
  `ci.yml` + `ci-full.yml` 3-OS matrix, `ui.yml`, `soak-nightly.yml`,
  `release.yml`) is written and actionlint-clean. Billing was unblocked
  2026-11-12 and workflows have executed since: the release pipeline's
  first real tag run shipped v0.1.0 (2026-11-13; the record, including its
  first failed iteration, is in `docs/RELEASE.md`), v0.1.1/v0.1.2 followed,
  and the CI lanes run on `main` (worklog w28.2/w28.3). The honest
  residue: **verified parity is still 0/252** (`docs/PARITY-REPORT.md` §5
  — the evidence-outbox flip pass has not run; all landed evidence is
  local-run, `hardware_class = mock`), and some older docs still carry the
  pre-unblock "billing-blocked" wording. Local gates
  (`scripts/full_gate.sh`) remain a co-evidence of record. The earlier
  rustfmt-drift red (w17, fixed by `621fa20` on `w17/ci-green`, since
  merged) is history — recorded here because older docs still cite it.

## Licensing notes

- This repository's own code: **MIT** (decided 2026-10-19; [`LICENSE`](LICENSE);
  `docs/adr/ADR-006-license-mit.md`; decision D-009; closes risk R-05).
- The design package in `design/figma/` originates from a Figma Community file;
  its license terms were cleared by user attestation 2026-10-19 (risk R-07
  closed, decision D-010; residuals: font identities [VERIFY], trademark
  wording rules on shipped strings per `docs/14-risk-register.md`).
