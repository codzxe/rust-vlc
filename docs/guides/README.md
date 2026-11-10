# User documentation — index

| Guide | What it covers |
|---|---|
| [`getting-started.md`](getting-started.md) | Build from source, first play, what works today |
| [`settings.md`](settings.md) | Every setting key, its default and where the default comes from |
| [`shortcuts.md`](shortcuts.md) | Keyboard bindings (UI) and REPL verbs (CLI) |
| [`troubleshooting.md`](troubleshooting.md) | Errors you will actually hit, and their causes |

**Read this first — the honest scope (2026-11-10).** Lumen is
pre-release software (`product.toml` `display_version = "0.1.0-dev"`).
What a user can actually do today:

- **The `lumen` CLI is the real player surface**: open, play, pause,
  seek, stop, track info, settings get/set — driven interactively or
  one-shot (`docs/05-command-layer.md` §5.3). Playback decodes for
  real, but audio goes to the **null device** and video renders to
  **no window**: real audio backends are not admitted yet (ADR-005,
  `docs/KNOWN-LIMITS.md` §4) and the desktop shell is a std-only
  skeleton (ADR-004).
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
