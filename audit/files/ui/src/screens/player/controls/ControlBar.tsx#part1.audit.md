# AUDIT: ui/src/screens/player/controls/ControlBar.tsx#part1 (lines 1-785)
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 785 (of 1113; chunk boundary: part1 = 1-785, ends before `function PanelPopover` at :786 — part2 = 786-1113 belongs to A09) / 12 |
| File verdict | REAL-UNPROVEN (genuine engine-command wiring through the transport seam; executed only under vitest/demo simulation — no shipped host binary at HEAD) |
| Reachable from shipped entry point? | NO. SPA-internal chain: PlayerScreen mounts ControlBar (controls lane); /player route App.tsx:303. The only real backend path is TauriTransport (app-transport.ts: `window.__TAURI_INTERNALS__` probe), whose host crate `apps/desktop-tauri` does not compile at HEAD (app.rs:301 `spawn_session_pump` vs only `spawn_event_pump` defined at app.rs:190 — E0425, W14-A01/A02/A03). Browser/dev context runs DemoTransport (simulation). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 2 |
| Commands run | `rg -n "isDemoTransport\|openPathRequest…"` (imports used only in part2 :903-1020 — not my chunk); read `state-mapping.ts` head (pure doc-08 mapping); `sed -n '361,400p' ui/src/screens/player/context.tsx` (actions dispatch `playback.toggle-play-pause`/`playback.seek`/`audio.set-volume`/`audio.set-mute`/`tracks.set-*-track`); `sed -n '225,265p' PlaylistProvider.tsx` (`playlist.play-item/next/prev/set-shuffle/set-repeat` dispatches); read `ClientProvider.tsx` + `app-transport.ts` head (Tauri vs Demo selection); `rg -n "player.controls.queue-meta" ui/src/i18n/messages.ts` (k07 keys REGISTERED :1456-1458, :2938-2940 → resolver fallback dead); `rg -n "audio.list-devices"` repo-wide (exists: crates/commands/src/catalog/command.rs:195-201, docs/05 §2.6/§523, engine accepts it per docs/perf/raw/stats-probe.jsonl:1 variant list); `rg -ln bookmark` (BookmarksDialog + `media.get-bookmarks` command exist); ControlBar.test.tsx inventory: 74 tests, 12 describes. |

## 1. One-paragraph verdict
Part 1 of the control bar is the strongest "wired to a real backend" evidence in the UI so far: every interactive control dispatches a genuine engine command through the transport (`playback.toggle-play-pause`, `playback.seek` with µs conversion, `audio.set-volume/set-mute`, `tracks.set-{video,audio,subtitle}-track`, `playlist.{play-item,next,prev,set-shuffle,set-repeat}`), with disabled-with-reason faces everywhere state is unknown — never optimistic. The rot is at the edges: the Output Device section claims "no output-device seam exists in any transport" while `audio.list-devices`/`audio.set-device` are typed in the same transport catalog and accepted by the real engine; the queue popover ships Bookmarks/Chapters tabs that can never render anything; and the whole bar can today only run against the DemoTransport simulation because the desktop host crate does not compile.

## 2. What actually works (verified)
— Wiring verified by read: context.tsx:361-400 + PlaylistProvider.tsx:225-265 actions dispatch the exact commands the bar's onClicks name; generated/commands.ts:24 types both sides.
— Engine reality: docs/perf/raw/stats-probe.jsonl:1 (live session stderr) enumerates the real accepted command variants including `audio.set-volume`, `playback.seek`, `playlist.next` — the dispatch targets exist engine-side.
— Tests: baseline run #23 (ui-vitest.log.txt, 1705 passed) includes ControlBar.test.tsx — 74 tests, including "transport cluster — every control invokes a real command" (:181) and "volume/mute — engine-reported, never optimistic" (:384).
— Dead-resolver cleanup regression test exists (:1028) — the file polices its own lane-seam debt.

## 3. Findings

### F-001 [S2]
Where: ui/src/screens/player/controls/ControlBar.tsx:606-616
Code:
```
          /* No output-device seam exists in any transport — the frame's
           * section renders its honest unavailable face, never a fabricated
           * device list (doc 08 §4.4). The frame's right-aligned rescan
           * glyph (audio variant, Vector 19/20 x1870–1890) rides the header
           * inert — no seam to wire. */
          <PanelSection
            title={m("player.controls.section.output-device")}
            trailingIcon="refresh"
          >
```
What is wrong: The comment's premise is FALSE. The output-device seam exists in this very transport: `audio.list-devices` and `audio.set-device` are typed in `ui/src/generated/commands.ts:24`, defined in the Rust catalog (`crates/commands/src/catalog/command.rs:195-201`), documented (docs/05-command-layer.md:261, :523, feature-matrix.toml:1794 "audio.list-devices/set-device commands … adopted per crosswalk"), and accepted by the real engine (docs/perf/raw/stats-probe.jsonl:1 variant list). macOS has a real `list_devices` impl (crates/platform-macos/src/macos/coreaudio.rs). The UI simply never wired it, then wrote a comment claiming the backend doesn't exist.
Why it matters: A settings section shipped permanently reading "unavailable" while the feature-matrix claims the command lane is "adopted" — doc/comment vs code vs matrix three-way drift. Users cannot select an audio device from the player UI even though the backend command exists and the frame (Controls.svg audio variant) demands it.
What correct looks like: Dispatch `audio.list-devices` on panel open, render real devices, `audio.set-device` on select; keep the honest "unavailable" face only for hosts answering the typed `unsupported-for-host` refusal.
Confidence: HIGH — Verification: read + rg across catalog/matrix/docs; engine acceptance cited from committed probe artifact (not re-run).

### F-002 [S3]
Where: ui/src/screens/player/controls/ControlBar.tsx:683-687
Code:
```
  /** Bookmarks/Chapters have no bar-context seam (the bookmarks engine data
   * lives behind the tools/dialogs surfaces, w26.c5) — honest faces. */
  const renderUnavailableTab = (): ReactNode => (
    <PanelReasonRow reason={m("player.controls.reason.unknown")} />
  );
```
What is wrong: The queue popover ships Bookmarks and Chapters tabs (queueTabs :518-522) whose render function is unconditionally "unavailable" — dead tabs. `media.get-bookmarks` and `media.get-chapters` exist in the transport catalog (generated/commands.ts:24) and BookmarksDialog.tsx already consumes bookmarks; only "no bar-context seam" (context.tsx doesn't expose them) — a scope cut, not a backend gap.
Why it matters: Two of the three tabs in the queue popover can never show content — UI theater in the mildest form: tabs that look like features but permanently render "unavailable". The user paid the click to learn nothing.
What correct looks like: Either wire the tabs (fetch `media.get-bookmarks`/`get-chapters` on open) or don't render tabs whose content is a constant.
Confidence: HIGH — Verification: read; commands typed in catalog verified; BookmarksDialog existence verified.

### F-003 [S3]
Where: ui/src/screens/player/controls/ControlBar.tsx:97-100 (keys at i18n/keys/w26.k07.ts; registration at messages.ts:2938-2940)
Code:
```
const k07Resolver =
  (m: (key: MessageKey) => string) =>
  (key: K07LaneKey): string =>
    m(key as MessageKey) ?? k07Catalog[key];
```
What is wrong: The three k07 keys are already registered in the main catalog (`messages.ts:1456-1458` union, `:2938-2940` values), so `m(key)` always resolves and the `?? k07Catalog[key]` fallback is unreachable dead code — exactly the "dead post-merge resolver" pattern this file's own comment (:86-91) says j07 P1 deleted elsewhere. The comment's "LIVE pre-registration seam" claim is stale.
Why it matters: Duplicated i18n truth (lane copy vs catalog copy) that can silently drift; the file already has a regression test (:1028) for deleting exactly this pattern.
What correct looks like: Delete k07Resolver/k07Catalog/K07LaneKey, call `m("player.controls.queue-meta")` directly — as the comment itself prescribes for "the next cleanup pass".
Confidence: HIGH — Verification: read + rg of both key sets (identical strings today, so no user-visible bug yet).

### F-004 [S3]
Where: ui/src/screens/player/controls/ControlBar.tsx:611-615 (with 605-606)
Code:
```
          <PanelSection
            title={m("player.controls.section.output-device")}
            trailingIcon="refresh"
          >
            <PanelReasonRow reason={m("player.controls.reason.unknown")} />
          </PanelSection>
```
What is wrong: The `trailingIcon="refresh"` glyph renders in the section header with no interactive role and no handler — an inert icon that reads as a "rescan" affordance. The comment admits it "rides the header inert".
Why it matters: An affordance-shaped element that does nothing on click/Enter — the exact "dead button" pattern this wave hunts, in icon form. Screen-reader and sighted users both read "refresh" and get silence.
What correct looks like: Either omit the glyph or make it a real rescan action (which F-001's wiring would provide).
Confidence: HIGH — Verification: read (SettingsPanel's PanelSection rendering not audited here — trailingIcon rendering assumed from name; if it renders as <Icon> only, finding stands as visual-only affordance).

### F-005 [S4]
Where: ui/src/screens/player/controls/ControlBar.tsx:655-665
Code:
```
    const meta =
      knownSeconds.length === playlist.entries.length && total > 0
        ? km("player.controls.queue-meta-total")
            .replace("{count}", count)
```
What is wrong: `String.replace` with a string pattern replaces only the FIRST occurrence of each placeholder — safe here only because each placeholder appears once; fragile template plumbing done by hand. Also `total > 0` hides a "0h 0m" total for sub-minute queues (renders count-only branch) — defensible but undocumented.
Why it matters: Minor i18n robustness debt; a catalog copy edit adding a second `{count}` silently renders wrong.
What correct looks like: replaceAll or a tiny format helper asserting placeholder counts.
Confidence: MEDIUM — Verification: read (no runtime issue with current catalog strings).

### F-006 [S4]
Where: ui/src/screens/player/controls/ControlBar.tsx:37-38, 690-691
Code:
```
//           cluster's documented extra — menu.png grammar, MenuPopup.
```
```
   * the 3-tab TRACK panels end in Button.png's quiet
```
What is wrong: The header and footer comments cite `menu.png` and `Button.png`, but the committed frames are `design/figma/menu.svg` and `design/figma/Button.svg` — no `.png` files exist in `design/figma/` (102 files checked: Player.svg, play.svg, volume.svg, Controls.svg, Current Time.svg, Duration.svg all present as `.svg`).
Why it matters: Trivial provenance drift: an editor following the citation to `menu.png` finds nothing and must guess which frame was meant. The design authority itself IS committed (good) — the citations just name stale filenames.
What correct looks like: Cite `menu.svg` / `Button.svg`.
Confidence: HIGH — Verification: `ls design/figma/` (102 entries) + rg for exact names — .png frames absent, .svg equivalents present.

## 4. Per-function table (part1)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `k07Resolver` (+k07Catalog, K07LaneKey) | 93-100 | SHALLOW (dead fallback) | F-003 | Keys registered in messages.ts → fallback unreachable; duplicated truth. |
| `acceleratorOf` | 109-117 | REAL | — | Reads generated HOTKEY_BINDINGS data; no hardcoded accelerators. Used in part2 only (:957-1043) — helper belongs to this chunk by line range. |
| `OPEN_CUSTOMIZE_EVENT` const | 123 | REAL | — | Frozen string contract, duplicated from PlayerScreen (documented reason: module cycle); used at :1037 (part2). |
| `formatTimecode` | 132-136 | REAL | — | Floor/clamp/pad correct; `Math.floor(null)`→0 tolerates null position without crash. |
| `NavigationSeam` / `RoutedNavigation` | 155-178 | REAL | — | Stable-identity fallback navigate; correct useInRouterContext guard. |
| `ControlBar` | 185-213 | REAL | — | Primitive props to memo'd TransportCluster (per-tick re-render containment — verified sound). |
| `trackRows` | 229-263 | REAL | — | Engine-truth radios; Automatic never checked; noMedia → [] (caller renders reason row); real `tracks.set-*` dispatch. |
| `TransportCluster` (memo) | 270-350 | REAL | — | prev/next disabled with reason when queue empty/notExposed; play mapping from state-mapping (doc 08 §5.1); real dispatches. |
| `SeekCluster` | 360-407 | REAL | — | Seekbar honest (data-empty when no duration); GotoTimeDialog on timecode click; seek dispatch µs-correct (context.tsx:382-392). |
| `RightCluster` | 410-418 | REAL | — | Composition only. |
| `DurationTimecode` | 423-447 | REAL | — | Remaining toggle; disabled when unknown; clamps at 0. |
| `VolumeCluster` | 450-487 | REAL | — | Engine-reported volume/mute; icon tiers; disabled when null; real `audio.set-volume`/`set-mute`. |
| `OutputCluster` (+renderPlaybackTab/renderTrackTab/renderQueueTab/renderUnavailableTab/renderTrackFooter/renderSettingsFooter) | 496-776 | REAL | F-001, F-002, F-004, F-005 | Shuffle/Repeat real dispatches; track tabs engine-truth; queue rows → playlist.play-item; output-device section false-comment dead wiring (F-001); bookmarks/chapters dead tabs (F-002); inert refresh glyph (F-004). |

## 5. Test honesty (ControlBar.test.tsx, 74 tests — file-level, covers both parts)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "transport cluster — every control invokes a real command" (:181) | YES | Asserts the dispatched command per click through a mock transport — mutation of the dispatch breaks it. |
| "volume/mute — engine-reported, never optimistic" (:384) | YES | Asserts faces come from engine responses, not local state. |
| "track menus … real ids, real labels, honest unknowns" (:440) | YES | Asserts radio rows from track data + disabled faces. |
| "w26.k07 P1 regression — the dead lane-seam resolvers are gone" (:1028) | YES (for presence) | Ironically does NOT catch the surviving k07Resolver fallback (F-003) — it tests the deleted ones, not this one. |
Suite passed in baseline run #23. MockTransport doubles the DEPENDENCY (transport), not the subject — legitimate.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No output-device seam exists in any transport" | ControlBar.tsx:606 | FALSE | audio.list-devices/set-device: generated/commands.ts:24; crates/commands/src/catalog/command.rs:195-201; docs/05:261; engine variant list in docs/perf/raw/stats-probe.jsonl:1. |
| "Bookmarks/Chapters have no bar-context seam" | ControlBar.tsx:683 | REAL (bar-context) / misleading (transport) | context.tsx has no bookmark state (true), but `media.get-bookmarks`/`get-chapters` exist in the transport and BookmarksDialog consumes them. |
| "menu.png grammar" accelerators from keymap data | ControlBar.tsx:103-107 | REAL | acceleratorOf reads HOTKEY_BINDINGS (generated); part2 usage. |
| Feature-matrix: "audio.list-devices/set-device commands … adopted per crosswalk" | docs/feature-matrix.toml:1794 | FALSE for this surface | The player UI's Output Device section renders constant "unavailable" (F-001). (Matrix row range owned by another agent.) |

## 7. Dead / duplicated / unreachable code
- k07Resolver fallback (F-003) — dead post-registration.
- `trailingIcon="refresh"` inert glyph (F-004).
- Bookmarks/Chapters tab descriptors — constants rendering constants (F-002).
- Whole bar: unreachable from any shipped binary (tauri E0425); reachable only in dev/browser (DemoTransport) and vitest.

## 8. Dependencies & flags
Imports `isDemoTransport`, `openPathRequest/openPathRoute`, profile/menu builders — all consumed in part2 (A09's range). No feature flags. state-mapping.ts is a pure function module (not audited here — unassigned to me). settings panels via SettingsPanel.tsx (my file 6).

## 9. Open questions for the next agent
- A09 (part2): does the app menu's "Open File…" flow work end-to-end through openPathRequest on the demo transport (recentOpenings seam :924)?
- Does `design/figma/` exist in the repo? (F-006 provenance.)
- Who wires `audio.list-devices` anywhere in the UI? (rg found zero UI call sites besides the generated type — likely nowhere; matrix row owner should record the contradiction.)
