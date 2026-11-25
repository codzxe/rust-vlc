# User documentation — index

| Guide | What it covers |
|---|---|
| [`getting-started.md`](getting-started.md) | Build from source, first play, what works today |
| [`settings.md`](settings.md) | Every setting key, its default and where the default comes from |
| [`shortcuts.md`](shortcuts.md) | Keyboard bindings (UI) and REPL verbs (CLI) |
| [`troubleshooting.md`](troubleshooting.md) | Errors you will actually hit, and their causes |

**Read this first — the honest scope (2026-11-10; audio lane
refreshed 2026-11-25, Devil-Fix wave FX3).** Lumen is
pre-release software (`product.toml` `display_version = "0.1.0-dev"`).
What a user can actually do today:

- **The `lumen` CLI is the real player surface**: open, play, pause,
  seek, stop, track info, settings get/set — driven interactively or
  one-shot (`docs/05-command-layer.md` §5.3). Playback decodes for
  real; by default the audio path renders into the **null backend —
  instrumented with sample/frame counters** (real progress evidence,
  not silence) — and video renders to **no window**. A real OS audio
  adapter now exists behind the optional `backend-cpal` build feature
  of `crates/audio-output` (ADR-005, amended 2026-11-25 — Devil-Fix
  wave FX3, FIX-PLAN §3 F1.1 Route A; the windows-latest CI lane
  compiles it), but the shipped hosts do not select it yet — backend
  selection and the volume/mute/balance/EQ verbs are wired into the
  hosts in wave FX4 — and audible output on real hardware is
  **[VERIFY] pending a Windows run** (`docs/KNOWN-LIMITS.md` §4). The
  desktop shell is a std-only skeleton (ADR-004).
- **The React UI** runs as a dev-server app (`npm run dev` in `ui/`)
  with the settings screens, shortcuts screen, onboarding wizard and
  routed player/playlist surfaces; it does not yet play media through
  the engine — the wasm host (`apps/web/`) and the desktop webview
  shell are separate, earlier-stage surfaces.
- **37 of 73 format rows** in `docs/feature-matrix.toml` are claimed
  (implemented or partial) on desktop, zero on web. No "all formats"
  claims are made anywhere in this documentation.

Sources these guides are derived from (config-driven, no invented
features): `product.toml`, `schemas/settings.toml`,
`keymaps/default.toml`, `locales/en-US.toml`, the `lumen --help`
output generated from the settings schema, and the format records in
`docs/formats/`.
