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

**Phase 6 — Network & streaming: substantially complete (2026-11-09).**
Phases 0–5 are complete (Phase 5: PASS WITH REGISTERED CAVEATS, Lead
sign-off 2026-10-24, [`docs/gates/phase5-exit.md`](docs/gates/phase5-exit.md));
the Phase-6 exit-gate ledger is drafted with verdict **PENDING LEAD
DECISION** ([`docs/gates/phase6-exit.md`](docs/gates/phase6-exit.md)).
On 2026-11-09, 27 of 34 tracker issues were resolved across 11 waves
(merge evidence: `git log --oneline --grep="issue #"`): streaming landed
TLS decision record + typed https seam (#8), buffering/reconnect (#6),
auth (#13), HLS master + conformance (#10/#11), DASH + ABR + MPD fuzz
(#12), netfuzz + interrupted-network scenarios (#14), and the RTSP DEFER
verdict + network security review (#15); alongside playback ops (#32),
A/V effects (#33), MKV extras (#34), subtitles (#17), ASF (#9), the
defect ledger (#16), the VLC 3.0.23 oracle corpus differentials — 29/35
inputs bit-exact (#19) — codec-admission records (#7), the cpal
audio-output spike (#1), H.264/HEVC decoder audits + ADR-013 draft (#2),
ADR-010 patent-policy draft (#24), desktop wiring (#3), icon glyphs
(#35), lint hardening (#36), and visual baselines + e2e fully green
(#37). Per-lane test evidence from the merge records: demux 587/0 (#34),
streaming 201/0 (#8), codecs 643 tests (#7), ui vitest 634 tests (#7)
plus the e2e/a11y lane green (#37); the last full-workspace gate count
is 3213/0 at the Phase-5 exit. Still open: #4 (Tauri packaging,
INFRA-BLOCKED), #22/#23 (Phase 7/8 programs), #38 (ogg fix in flight),
#39 (TLS activation follow-up). See the Status block in
[`docs/ROADMAP.md`](docs/ROADMAP.md) and the ledger in
[`NEXT-STEPS.md`](NEXT-STEPS.md). The authoritative project brief is
[`docs/MASTER-PROMPT.md`](docs/MASTER-PROMPT.md). Nothing is claimed implemented or verified
until `docs/feature-matrix.toml` records automated evidence for it (statuses:
`not_started`, `in_progress`, `implemented`, `verified`, `blocked`).

Parity is defined against a pinned baseline, **VLC 3.0.23** (desktop), and is the
share of inventory features verified by automated evidence. No "exactly like VLC"
or "all formats supported" claims beyond that evidence.

## Install

Prebuilt binaries are published on the project's **GitHub Releases** page
(tag-triggered pipeline: `.github/workflows/release.yml`; process and honest
limitations: [`docs/RELEASE.md`](docs/RELEASE.md)). Two programs ship in every
artifact: `lumen` — the CLI, the real engine surface (demux/decode/playback) —
and `lumen-desktop` — the ADR-004 std-only composition-root skeleton (no window
yet; it exists so packaging paths are exercised honestly).

**Verify checksums first** (every release ships `SHA256SUMS.txt`):

```sh
sha256sum --check SHA256SUMS.txt        # GNU coreutils (Linux, git-bash)
shasum -a 256 --check SHA256SUMS.txt    # macOS
```

- **Windows** (`…-x86_64-windows.zip` / `.msi`): the MSI installs both binaries
  into Program Files (no Start-Menu entry yet — the desktop shell is a
  skeleton). Or unzip the archive and add its folder to `PATH`. Binaries are
  **unsigned** (no code-signing certificate yet — issue #56); SmartScreen may
  warn ("More info" → "Run anyway").
- **macOS** (`…-aarch64-macos.dmg` / `…-x86_64-macos.zip`): open the `.dmg`,
  drag the app to `/Applications`. The bundle is **unsigned and not
  notarized** (no Apple Developer account — issue #56): on first run
  right-click the app → *Open* → *Open*, or remove the quarantine attribute:
  `xattr -dr com.apple.quarantine /Applications/Lumen.app`.
- **Linux**:
  - `.deb` (x86_64, arm64): `sudo apt install ./lumen-<version>-<arch>-linux.deb`
    (or `sudo dpkg -i`). Installs both binaries to `/usr/bin` plus a desktop
    entry for the skeleton binary.
  - `…-linux.tar.gz`: unpack, put the directory on `PATH`:
    `tar -xzf …tar.gz && export PATH="$PWD:$PATH"` (add to `~/.profile` to
    persist).
  - `….AppImage`: `chmod +x …AppImage && ./…AppImage`. On systems without
    FUSE, run it extracted: `APPIMAGE_EXTRACT_AND_RUN=1 ./…AppImage`.
  - arm64 artifacts are **best-effort** (see docs/RELEASE.md).

**Build from source**: clone the repo, then
`cargo build --release --locked -p lumen-cli -p desktop` (toolchain pinned by
`rust-toolchain.toml`); the packaging scripts under `scripts/release/` build
every artifact above from those two binaries. Full release engineering notes:
[`docs/RELEASE.md`](docs/RELEASE.md).

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
- Merge-blocking CI, honestly stated: **the full 3-OS matrix is live**
  (issue #5 — `.github/workflows/ci.yml` carries the merge-blocking
  matrix with `ci-full.yml` alongside; `ui.yml` covers the UI lane;
  `soak-nightly.yml` the 4 h nightly tier), **but it is currently red**:
  the lint job fails on rustfmt drift accumulated across the day's wave
  merges — fix in flight on branch `w17/ci-green` (commit `621fa20`,
  "ci: fix rustfmt drift across 47 files from wave merges (lint job #1)").
  Local gates remain the evidence of record until it goes green.

## Licensing notes

- This repository's own code: **MIT** (decided 2026-10-19; [`LICENSE`](LICENSE);
  `docs/adr/ADR-006-license-mit.md`; decision D-009; closes risk R-05).
- The design package in `design/figma/` originates from a Figma Community file;
  its license terms were cleared by user attestation 2026-10-19 (risk R-07
  closed, decision D-010; residuals: font identities [VERIFY], trademark
  wording rules on shipped strings per `docs/14-risk-register.md`).
