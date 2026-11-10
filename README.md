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

**Phase 8 — Hardening & release: in progress (2026-11-10, wave 24).**
Phases 0–6 are complete (Phase 5: PASS WITH REGISTERED CAVEATS, Lead
sign-off 2026-10-24, [`docs/gates/phase5-exit.md`](docs/gates/phase5-exit.md));
the Phase-6 exit ledger is drafted with verdict PENDING LEAD DECISION,
[`docs/gates/phase6-exit.md`](docs/gates/phase6-exit.md)). Phase 7 landed
across waves 20–23 (merge evidence: `git log --oneline --grep="merge wave"`):
platform adapter crates (#40), hw-decode adapter lanes compile-verified
(#41), device hotplug + discovery (#42), transcode graph core (#43),
recording + MP4 remux (#44), wasmi plugin host + crash isolation (#45/#46),
apps/web v1 (#47) + its render/audio/io pillars (#48), control server
(#50), the tag-triggered release pipeline (#56), the TLS decision record
(#39), and the security sweeps + CycloneDX SBOM (#52); the perf/parity/
known-limits docs were finalized in wave 22 (#55/#57:
[`docs/PERF-VS-VLC.md`](docs/PERF-VS-VLC.md),
[`docs/PARITY-REPORT.md`](docs/PARITY-REPORT.md)). Wave 24 (Phase 8) is
in flight: packaging per OS (lane C), this docs-completeness + release-
checklist lane (E), and siblings. What plays today, honestly: **37 of the
73 format rows** in [`docs/feature-matrix.toml`](docs/feature-matrix.toml)
are claimed (implemented or partial) on desktop — **zero on web** — and
the `lumen` CLI drives them headless (decode + sync real; audio output is
the ADR-005 null device, the desktop shell is the ADR-004 skeleton).
Two defects were found on `main` by the 2026-11-10 docs sweep
([`docs/audit/docs-sweep-20261110.md`](docs/audit/docs-sweep-20261110.md)):
committed merge-conflict markers in `ui/src/App.tsx` (breaks
`npm run build` in `ui/`) and a stale generated settings screen — both
reported to the Lead, fixes owned outside this lane. See
[`docs/ROADMAP.md`](docs/ROADMAP.md) and [`NEXT-STEPS.md`](NEXT-STEPS.md).
The authoritative project brief is
[`docs/MASTER-PROMPT.md`](docs/MASTER-PROMPT.md). Nothing is claimed
implemented or verified until `docs/feature-matrix.toml` records automated
evidence for it (statuses: `not_started`, `in_progress`, `implemented`,
`verified`, `blocked`).
## Install

**No release has been published yet.** The release pipeline exists and is
validated by construction + local dry-runs (tag-triggered:
`.github/workflows/release.yml`; process and honest limitations:
[`docs/RELEASE.md`](docs/RELEASE.md)), but CI billing is blocked on this
private repository, so no tag has ever been pushed and no artifact has
ever been published. When the first release is cut, this section is where
per-OS artifacts will be listed. Two programs would ship in every
artifact: `lumen` — the CLI, the real engine surface (demux/decode/
playback) — and `lumen-desktop` — the ADR-004 std-only composition-root
skeleton (no window yet; it exists so packaging paths are exercised
honestly). When releases exist, every one will ship `SHA256SUMS.txt` —
verify first:

```sh
sha256sum --check SHA256SUMS.txt        # GNU coreutils (Linux, git-bash)
shasum -a 256 --check SHA256SUMS.txt    # macOS
```

Until then, **build from source** (the only supported path today; see
[`docs/guides/getting-started.md`](docs/guides/getting-started.md) for a
walkthrough): clone the repo, then `cargo build --release --locked -p
lumen-cli -p desktop` (toolchain pinned by `rust-toolchain.toml`); the
packaging scripts under `scripts/release/` build the per-OS artifacts
listed in [`docs/RELEASE.md`](docs/RELEASE.md) from those two binaries.
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
  `ci.yml` + `ci-full.yml` 3-OS matrix, `ui.yml`, `soak-nightly.yml`) is
  written and actionlint-clean, **but CI billing is blocked on this private
  repository — no workflow has ever executed** (worklog 7.CLOSEOUT; the
  standing caveat in `docs/RELEASE.md` known-limits). Local gates
  (`scripts/full_gate.sh`) remain the evidence of record. The earlier
  rustfmt-drift red (w17, fixed by `621fa20` on `w17/ci-green`, since
  merged) is history — recorded here because older docs still cite it.

## Licensing notes

- This repository's own code: **MIT** (decided 2026-10-19; [`LICENSE`](LICENSE);
  `docs/adr/ADR-006-license-mit.md`; decision D-009; closes risk R-05).
- The design package in `design/figma/` originates from a Figma Community file;
  its license terms were cleared by user attestation 2026-10-19 (risk R-07
  closed, decision D-010; residuals: font identities [VERIFY], trademark
  wording rules on shipped strings per `docs/14-risk-register.md`).
