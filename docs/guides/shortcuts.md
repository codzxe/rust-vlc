# Shortcuts

Two surfaces, two mechanisms — both data-driven, neither hand-written
here:

1. **The UI keymap** — `keymaps/default.toml` (150 bindings, task
   0.B3): VLC 3.0.23 defaults where clean-room-observable, overridden
   by the design package where it specifies a binding (decision D-003).
   The UI's shortcuts screen (`/keyboard-shortcuts`) and the hotkey
   editor render from data generated out of this file
   (`npm run hotkeys:generate`).
2. **The CLI REPL verbs** — typed commands, not keys (see
   [getting-started](getting-started.md) §2 for the full list).

## Player core (verified against `keymaps/default.toml`)

| Action | Key | Note |
|---|---|---|
| Play / pause | `Space` | |
| Stop | `S` | |
| Next / previous track | `N` / `P` (also `>` / `<`) | design keeps both pairs (D-2) |
| Frame step | `.` next / `,` previous | design-supplied; `,` is a NEW capability (VLC 3.0.23 has no frame-previous) |
| Seek short | `←` / `→` (±10 s) | design override of VLC's disc-menu arrows (C-01) |
| Seek extrashort / medium / long | `Shift+←/→` (±3 s), `Ctrl+←/→` (±60 s), `Ctrl+Alt+←/→` (±300 s) | VLC binary-observed jump sizes |
| Rate | `+` / `-` / `=` normal / `[` `]` fine | |
| Volume | `↑` / `↓` (±5 %), `M` mute | design override of VLC's Ctrl+arrows (C-02) |
| Audio / subtitle track cycle | `B` / `V` | |
| Subtitle toggle / delay | `Shift+V` / `H` `G` (±100 ms) | |
| Fullscreen / exit | `F` / `Esc` | |
| Aspect-ratio / crop cycle | `A` / `C` | |
| Snapshot | `Shift+S` | |
| Show position | `T` | |
| Jump to time | `Ctrl+T` | |
| Playlist panel / queue / file browser | `Ctrl+L` / `Q` / `L` | |
| Quit | `Ctrl+Q` | |

macOS: menu-style `Ctrl` → `Cmd` (the platform convention; additional
differences live in `[platform_overrides.macos]`, e.g. audio-delay
keys — marked [VERIFY] against a pinned macOS run in the keymap).

**Honest limits:** these bindings describe the shipped default keymap
data. Bindings whose feature is not yet implemented (e.g.
`open-disc`, `open-capture-device`, `convert`, `record`,
deinterlace — feature-gated in v1) ship disabled-from-data: the
command layer answers a typed `feature-not-exposed` error, never a
dead key that pretends to work. Media/hardware keys
(`MediaPlayPause`, `VolumeUp`, …) are bound as VLC's alternates. The
remaining sections (`[playlist]`, `[window]`, `[advanced]`, ~150
bindings total) are enumerated in the keymap file itself — the source
of truth this page deliberately does not duplicate.

## CLI equivalents

`open`, `close`, `play`, `pause`, `toggle`, `stop`,
`seek 90s | +10s | -1m | 50% | chapter 3`, `info`, `time`,
`duration`, `tracks`, `engine-info`, `commands`,
`setting get|set`, `decode-bench`, `advance`, `help`, `quit` — from
`lumen --help` (generated from the settings schema + command catalog).

## Where bindings come from (provenance)

`keymaps/default.toml` records per-entry provenance
(`vlc-3.0.23-default` vs `design-package`) and the conflict decisions
(C-01…C-07, D-1, D-2); the full transcription and [VERIFY] ledger is
`docs/inventory/ui-inventory.md` §5–§8. Keymap edits are config-plane
changes — the `hotkeys:check` generator gate fails on drift between
the keymap and the generated UI data.
