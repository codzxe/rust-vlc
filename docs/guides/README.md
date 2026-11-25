# User documentation — index

| Guide | What it covers |
|---|---|
| [`getting-started.md`](getting-started.md) | Build from source, first play, what works today |
| [`settings.md`](settings.md) | Every setting key, its default and where the default comes from |
| [`shortcuts.md`](shortcuts.md) | Keyboard bindings (UI) and REPL verbs (CLI) |
| [`troubleshooting.md`](troubleshooting.md) | Errors you will actually hit, and their causes |

**Read this first — the honest scope (2026-11-10; audio lane
refreshed 2026-11-25 Devil-Fix wave FX3, then 2026-11-26 Devil-Fix
wave FX4; video/frame lane refreshed 2026-11-26 Devil-Fix wave FX6).** Lumen is
pre-release software (`product.toml` `display_version = "0.1.0-dev"`).
What a user can actually do today:

- **The `lumen` CLI is the real player surface**: open, play, pause,
  seek, stop, track info, settings get/set — driven interactively or
  one-shot (`docs/05-command-layer.md` §5.3). Playback decodes for
  real; by default the audio path renders into the **null backend —
  instrumented with sample/frame counters** (real progress evidence,
  not silence) — and the CLI renders video to **no window** (headless
  by design; `decode-bench` is the frame-count oracle). A real OS audio
  adapter now exists behind the optional `backend-cpal` build feature
  of `crates/audio-output` (ADR-005, amended 2026-11-25 — Devil-Fix
  wave FX3, FIX-PLAN §3 F1.1 Route A; the windows-latest CI lane
  compiles it) — and, since Devil-Fix wave FX4 (2026-11-26), the
  shipped **CLI host** routes through the backend factory (the
  cpal adapter when the feature is on, the instrumented null backend
  otherwise; `--audio-backend`/`--no-audio` flags — FX4-A01/A06) and
  the volume/mute/balance/EQ verbs are wired to the pipeline controls
  (FX4-A02); the desktop/tauri GUI lanes are **pending verification**
  (FX4-A03/A04), and audible output on real hardware is
  **[VERIFY] pending the Windows CI run of the `audio-cpal-check`
  lane + a Windows release smoke** (`docs/KNOWN-LIMITS.md` §4). The
  desktop shell is the real, shipped tauri app (`lumen-tauri`, w28)
  with its frame lane bounded and instrumented since Devil-Fix wave
  FX6 (2026-11-26): the canvas is real
  (`ui/src/components/player/MediaSurface.tsx:342`
  `putImageData`, wave-15 finding 1), the mailbox carries a documented
  drop policy + timestamp passthrough + produced/dropped counters
  (FX6-A01), the 10 ms ticker ships policy-aware with the counters
  surfaced (FX6-A02), the tauri frame channel is bounded with
  shipped/dropped-by-overflow counters (FX6-A03 — a stalled webview
  cannot grow an unbounded queue), and the UI surfaces the frame-lane
  counters the webview observes (presented/skipped; FX6-A05). What is still **[VERIFY]**: a user-watched
  end-to-end run on Windows hardware — frames actually seen, smoothness,
  4K starvation at the 10 ms cadence (`docs/KNOWN-LIMITS.md` §4).
- **The React UI** runs as a dev-server app (`npm run dev` in `ui/`)
  with the settings screens, shortcuts screen, onboarding wizard and
  routed player/playlist surfaces; its real media-surface lane (the
  canvas above) rides the desktop webview host through
  `lumen_dispatch`'s frame channel. The dev-server/browser session is
  the honestly-badged demo lane (wave-15: `DemoTransport`, browser
  build genuinely canvas-less), and the wasm host (`apps/web/`) is a
  separate, earlier-stage surface. End-to-end video through the UI is
  **[VERIFY] pending the Windows GUI run** (same guard as above —
  `docs/KNOWN-LIMITS.md` §4).
- **37 of 73 format rows** in `docs/feature-matrix.toml` are claimed
  (implemented or partial) on desktop, zero on web. No "all formats"
  claims are made anywhere in this documentation.

Sources these guides are derived from (config-driven, no invented
features): `product.toml`, `schemas/settings.toml`,
`keymaps/default.toml`, `locales/en-US.toml`, the `lumen --help`
output generated from the settings schema, and the format records in
`docs/formats/`.
