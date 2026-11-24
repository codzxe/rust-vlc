# AUDIT: ui/src/screens/player/controls/ControlBar.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 1113 / 15 (k07Resolver, acceleratorOf, formatTimecode, NavigationSeam, RoutedNavigation, ControlBar, trackRows, TransportCluster, SeekCluster, RightCluster, DurationTimecode, VolumeCluster, OutputCluster(+5 inner render fns), PanelPopover, AppMenuButton, ErrorSurface) |
| File verdict | REAL-UNPROVEN (UI wiring verified by tests; end-to-end engine run not executable in sandbox) |
| Reachable from shipped entry point? | YES: ui/src/App.tsx:4 imports PlayerScreen -> routes /player (App.tsx:140-141) -> PlayerScreen.tsx:1195 `<ControlBar />` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `rg -n "<ControlBar" src/` (mount at PlayerScreen.tsx:1195); `rg -n "togglePlayPause|setVolume|selectTrack" src/screens/player/context.tsx` (actions dispatch real cmds); `sed -n` quote verification (5 blocks); `rg -n "lumen_dispatch" apps/desktop/src/` |

## 1. One-paragraph verdict
This file answers the master question affirmatively for the core transport surface: every primary button (play/pause, prev/next, seek, volume, mute, track selection, shuffle/repeat, queue rows, error dismiss) dispatches a real protocol command (`playback.toggle-play-pause`, `playlist.prev`, `playback.seek`, `audio.set-volume`, `tracks.set-audio-track`, ...) through `usePlayerControls()` (context.tsx:361-412) and `usePlaylist()` (PlaylistProvider.tsx:231-262), both of which `client.send` over the selected transport (TauriTransport in the desktop shell, app-transport.ts:63-71). Disabled states are honest (engine-reported-only, never optimistic). The dead weight is in secondary surfaces: the queue popover's Bookmarks/Chapters tabs are permanently "unknown" (no seam), the audio output-device section is permanently "unknown" with an inert rescan glyph, and Open Recent is demo-transport-only — dead on the real desktop backend. This is disclosed in comments but is still user-visible dead surface.

## 2. What actually works (verified)
— verified by reading context.tsx:361-412 (actions dispatch `cmd: "playback.toggle-play-pause"` etc. via `wire`) and PlaylistProvider.tsx:231-262 (`playlist.play-item`/`prev`/`next`/`set-shuffle`/`set-repeat`), plus ControlBar.test.tsx:181-440 ("transport cluster — every control invokes a real command", seek/volume/track dispatch assertions against a fake wire transport). Baseline: vitest 1705 passed (audit/runs/baseline). Not executed end-to-end here — desktop build blocked (gdk-sys, baseline §13).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/controls/ControlBar.tsx:924
Code:
```
  const recents = isDemoTransport(client) ? client.recentOpenings : [];
```
What is wrong: Open Recent works ONLY on DemoTransport. In the shipped desktop shell (TauriTransport, app-transport.ts:63-71) `recents` is always `[]`, so the submenu renders only the disabled "empty" row — a permanently dead feature on the real backend, alive only in the browser simulation. Why it matters: the one surface where the UI looks most "real" (a remembered history) is powered exclusively by the mock transport. What correct looks like: a `recentOpenings` getter on the real transport (host-provided or persisted), or the row hidden on desktop. Confidence: HIGH — Verification: read + `rg` of `recentOpenings` (only demo-transport implements it).

F-002 [S3]
Where: ui/src/screens/player/controls/ControlBar.tsx:683-687
Code:
```
  /** Bookmarks/Chapters have no bar-context seam (the bookmarks engine data
   * lives behind the tools/dialogs surfaces, w26.c5) — honest faces. */
  const renderUnavailableTab = (): ReactNode => (
    <PanelReasonRow reason={m("player.controls.reason.unknown")} />
  );
```
What is wrong: the queue trigger (Controls.svg frame) advertises a 3-tab panel; the Bookmarks and Chapters tabs can NEVER render data from this bar — they are permanent "Unknown" placeholders (rendered at L767). Dead tabs shipped as visible UI. Why it matters: 2/3 of the queue popover's surface is unreachable functionality; bookmarks data exists in the app (BookmarksDialog) but is not plumbed here. What correct looks like: reuse the bookmarks engine seam the dialogs use, or drop the tabs. Confidence: HIGH — Verification: read (L761-770 mounts queueTabs with `tab === "queue" ? renderQueueTab() : renderUnavailableTab()`).

F-003 [S3]
Where: ui/src/screens/player/controls/ControlBar.tsx:610-616
Code:
```
          <PanelSection
            title={m("player.controls.section.output-device")}
            trailingIcon="refresh"
          >
            <PanelReasonRow reason={m("player.controls.reason.unknown")} />
          </PanelSection>
```
What is wrong: the audio panel's Output Device section permanently renders "Unknown" and the `refresh` rescan glyph is inert (comment L609-610: "rides the header inert — no seam to wire"). Dead section + dead glyph in shipped UI. Why it matters: users see an output-device picker that can never pick; a rescan button that does nothing on click. AGGRAVATING EVIDENCE (post-audit cross-check): the wire protocol DOES define the seam — src/generated/commands.ts CommandRequest includes `{ cmd: "audio.list-devices" }` and `{ args: { id: string }; cmd: "audio.set-device" }`, and the settings screen wires it for real (src/screens/settings/areas/audio.tsx:120-131 `AudioDeviceControl` sends `cmd: "audio.list-devices"`). The comment "no output-device seam exists in any transport" is FALSE as of the current catalog — the seam exists and a sibling screen uses it; this bar section was simply never wired. What correct looks like: reuse the AudioDeviceControl seam the settings area already implements. Confidence: HIGH — Verification: read + rg on generated/commands.ts and audio.tsx.

F-004 [S4]
Where: ui/src/screens/player/controls/ControlBar.tsx:93-100
Code:
```
const k07Catalog: Readonly<Record<string, string>> = k07Keys;
type K07LaneKey = keyof typeof k07Keys;
const k07Resolver =
  (m: (key: MessageKey) => string) =>
  (key: K07LaneKey): string =>
    m(key as MessageKey) ?? k07Catalog[key];
```
What is wrong: dead fallback. The k07 keys ARE registered in messages.ts (union at L1456-1458, catalog at L2938-2940: `"player.controls.queue-meta-total": "{count} items - {duration}"`), and `useMsg` returns `catalog[key] ?? messages[key]` (messages.ts:3016) — never `undefined` for these keys, so `?? k07Catalog[key]` is unreachable. The header comment (L83-91) admits the identical pattern was deleted for f13/gfixC/i03/i16 resolvers in j07 P1 and this one awaits the same cleanup. Confidence: HIGH — Verification: read + sed of messages.ts:2938-2940 and messages.ts:3011-3016.

F-005 [S4]
Where: ui/src/screens/player/controls/ControlBar.tsx:960-991
Code:
```
    {
      // No folder source exists in any transport (OpenMediaDialog's honest
      // file/stream subset) — inert with its frame accelerator.
      id: "open-folder",
      ...
      disabled: true,
    },
```
What is wrong: three app-menu rows (open-folder, open-disc, open-capture-device) are permanently `disabled: true` — frame-fidelity theater. Honest (labeled, inert, no fake action) but they are dead menu rows shipped to match a Figma frame. Confidence: HIGH — Verification: read (L960-991).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| k07Catalog/K07LaneKey/k07Resolver | 93-100 | STUB (dead fallback) | S4 | F-004; both paths return same string now |
| acceleratorOf | 109-117 | REAL | — | reads generated HOTKEY_BINDINGS data (settings/areas/hotkeys/generated/bindings) |
| formatTimecode | 132-136 | REAL | — | HH:MM:SS, clamps negatives |
| NavigationSeam/RoutedNavigation | 155-178 | REAL | — | router-presence fallback to window.location.assign; hooks unconditional |
| ControlBar | 185-213 | REAL | — | composes clusters; data-playback-state/data-engine-reported attrs |
| trackRows | 229-263 | REAL | — | engine-truth radios; "Automatic" never checked (comment L246-247) |
| TransportCluster | 270-350 | REAL | — | memo'd on primitives; prev/next via playlist context, honest disabled |
| SeekCluster | 360-407 | REAL | — | seek dispatch + pending; GotoTimeDialog on timecode click |
| RightCluster | 410-418 | REAL | — | composition only |
| DurationTimecode | 423-447 | REAL | — | remaining toggle, disabled when no duration |
| VolumeCluster | 450-487 | REAL | — | engine-reported mute/volume; honest disabled when null |
| OutputCluster | 496-776 | REAL w/ dead surfaces | S3 | F-002/F-003; playback tab real (shuffle/repeat cmds) |
| PanelPopover | 786-866 | REAL | — | outside-dismiss, Escape focus restore, tab reset per open |
| AppMenuButton | 890-1091 | REAL w/ dead rows | S3/S4 | F-001/F-005; real actions: open-media CustomEvent, routes, demo recents, profiles local store |
| ErrorSurface | 1096-1113 | REAL | — | typed error chip, dismiss action |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ControlBar.test.tsx:182 play/pause dispatches playback.toggle-play-pause | YES | asserts dispatched cmd on fake transport |
| :191 previous dispatches playlist.prev | YES | seeds playlist.changed event, asserts cmd |
| :203 previous inert while queue empty | YES | asserts no command dispatched |
| :285-337 seek pending/confirm/failed trio | YES | asserts pending set + cleared + typed error surfaced |
| :385-432 volume/mute engine-reported | YES | asserts cmd args, no optimism |
| :440-579 track menus real ids/labels | YES | asserts tracks.set-audio-track with real id, honest unknowns |
Tests run against a fake wire transport — they prove UI→command wiring, NOT engine execution (correct scope for unit tests; engine proof lives in the rc-smoke baseline).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Behavior contract UNCHANGED ... play/pause/seek/volume/mute wiring below is a restyle of the same dispatch" (L40-43) | file header | REAL | context.tsx:361-412 dispatches; tests L181-440 |
| "no-seam rows ... render inert ... never theater (doc 08 §4.4)" (L874-879) | file header | REAL | L967/982/990 `disabled: true` |
| "the frame's right-aligned rescan glyph ... rides the header inert — no seam to wire" (L609-610) | comment | REAL (honest disclosure of dead glyph) | L613 `trailingIcon="refresh"` with no onClick |

## 7. Dead / duplicated / unreachable code
- k07Resolver fallback (F-004) — unreachable branch.
- OPEN_CUSTOMIZE_EVENT string duplicated from PlayerScreen (L119-123, disclosed module-cycle rationale — accepted duplication).
- Bookmarks/Chapters tabs, output-device section, rescan glyph, 3 disabled menu rows, demo-only recents: reachable UI, permanently inert functionality (F-001..F-003, F-005).

## 8. Dependencies & flags
Imports generated data `HOTKEY_BINDINGS` from `../../settings/areas/hotkeys/generated/bindings` (generated-vs-handwritten drift audited in the settings areas files of this assignment). No feature flags gate this file. `isDemoTransport` import ties menu behavior to the mock transport (F-001).

## 9. Open questions for the next agent
- Does the desktop shell actually register the `lumen_dispatch` Tauri command (apps/desktop/src/dispatch.rs:22-27 shows registration as a COMMENT; INFRA-BLOCKED.md referenced)? If unregistered, EVERY ControlBar command is dead in the shipped desktop build — S0 for the desktop agent to adjudicate.
- Profile flyout (`buildProfileFlyout`) is local-device data by ruling — verify no doc claims it syncs with an engine profile system.
