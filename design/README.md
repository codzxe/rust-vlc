# Design Package — VLC Redesign (Figma Community export)

Pinned visual source of truth for the UI (MASTER-PROMPT §7). Exported SVG frames
from the Figma Community file "VLC Redesign (Community)". Committed as received;
no modifications. **License terms of the community file must be verified before
commercial use** (risk R-07, docs/14-risk-register.md).

## Inventory (102 files, ~77 MB)

| Group | Files | Notes |
|-------|-------|-------|
| Player (dark) | `Player.svg`, `Player-1..6.svg` | Main window states: default, variants (fullscreen, menus open, etc.) |
| Player — Light | `Player - Light.svg` | Light theme |
| Player — Music | `Player - Music.svg`, `-1` | Audio/music mode |
| Player — hide UI | `Player - hide UI.svg` | Unobtrusive/hidden controls state |
| Player — customize | `Player - customize.svg`, `-1..8` | Customize panel states |
| Miniplayer | `Miniplayer.svg` | Compact player |
| Settings | global(-1), interface(-1), privacy, playback(-1), video(-1), audio(-1), subtitles(-1), controls, library, network, performance, recording, extensions | 13 settings areas (the `-1` variants are states of the same area) — direct input for settings-schema UI placement; count confirmed by decision D-002 |
| Audio tools | `Audio - Equalizer.svg`, `EQ.svg`, `EQ presets.svg`, `Audio - Compressor Settings.svg`, `Audio - Spatializer Settings.svg`, `Audio - Stereo Widener.svg`, `Audio Visualizer.svg`, `audio value.svg` | |
| Video tools | `Video - Colors.svg`, `Video - Crop.svg`, `Video - Essential.svg`, `HDR Options.svg` | |
| Playlists | `playlist.svg`, `new playlist.svg`, `Add Subtitles to Playlist/Folder*.svg` (4), `subtitle one/multiple.svg`, `sub filename.svg`, `thumbnail.svg`, `media.svg`, `Sorting Options.svg`, `View Options.svg` | |
| Menus | `menu.svg`, `context menu.svg`, `file/folder/playlist context menu.svg`, `right-click menu.svg`, `option.svg`, `setting category.svg`, `dropdown.svg`, `Profiles.svg`, `Setting Preset.svg`, `origin.svg`, `target.svg`, `play.svg`, `sidebar.svg`, `volume.svg`, `Controls.svg`, `Button.svg`, `Toggle.svg`, `Values.svg(-1)`, `Current Time.svg`, `Duration.svg`, `Icon Wrapper.svg`, `Icon.svg` | Component-level exports |
| Dialogs / flows | `Open network stream.svg`, `Keyboard Shortcuts.svg`, `Onboarding.svg`, `Onboarding-1..10.svg` | |
| System | `Components.svg`, `Designs.svg` | Overview sheets |

## Intake status

- [x] Unpacked, committed as-is
- [ ] Screen-by-screen analysis → `docs/08-ui-figma-fidelity.md` (Task 1.B8)
- [ ] Token extraction (colors, type scale, spacing, radii, shadows) — note: SVG
      exports carry per-layer values; obtaining the original Figma variables file
      remains the preferred path ([VERIFY] with file owner)
- [ ] Screenshot-diff baseline generation (Phase 2)
