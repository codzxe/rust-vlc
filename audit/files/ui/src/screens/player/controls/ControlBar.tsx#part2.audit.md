# AUDIT: ui/src/screens/player/controls/ControlBar.tsx#part2
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 558 (557–1114 of 1114) / 9 (renderTrackTab doc+body from 557, renderQueueTab, renderUnavailableTab, renderTrackFooter, renderSettingsFooter, OutputCluster JSX, PanelPopover, AppMenuButton, ErrorSurface) |
| Chunk boundary | part2 = 557–1114 (assignments.tsv even split; part1 = 1–556 owned by W15-A08). The split cuts the `renderTrackTab` doc comment (553–559) mid-sentence — lines 553–556 read for context only, findings owned there stay with A08. Part1 context (ControlBar root, TransportCluster, SeekCluster, trackRows, VolumeCluster, NavigationSeam, k07Resolver def) was read in full but is A08's audit surface. |
| File verdict | REAL (part2: reachable, transport-wired, behaviorally tested in baseline) |
| Reachable from shipped entry point? | YES (call chain: App.tsx:303-305 `/player` routes → GatedShell → PlayerRoute → PlayerScreen.tsx:1195 `<ControlBar />` → OutputCluster/PanelPopover/AppMenuButton/ErrorSurface). Product caveat: the desktop shell binary does not exist (W14-A01 E0425); in the only runnable lane (browser fallback) these mount over DemoTransport. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `rg -n "it\(" ControlBar.test.tsx` (74 behavioral tests; queue meta at 1135/1149, app menu 669-808, panels 878-1029, radiogroups 1069/1112); `rg -n "recentOpenings" src/client` (getter exists ONLY in demo-transport.ts:354); `rg -n "queue-meta" src/i18n/messages.ts` (2938-2940: keys registered; templates `"{count} items"`, `"{hours}h {minutes}m"`); Read messages.ts:3011-3016 (useMsg is total — never undefined); Read SettingsPanel.tsx:176-205 (trailingIcon renders aria-hidden decorative Icon, not a button); `rg -n "ToolsDialogTab" ToolsDialog.tsx` (51: `"audio" \| "video"` — no subtitle tab); `rg -n "about" areas/index.tsx` (278: `about: "global"` fold — menu's About→/settings/global verified); Read ClientProvider.tsx (context value = stable transport object, memo claim of AppMenuButton holds); icon-name existence loop (all 22 icon names used in my range exist in icons.tsx). Baseline: ui-vitest.log.txt → 136/136 files, 1705 passed (includes ControlBar.test.tsx). |

## 1. One-paragraph verdict
The second half of the ControlBar is the densest honestly-wired surface audited so far: every interactive element dispatches a real wire command through the player context (`tracks.set-{video,audio,subtitle}-track` at context.tsx:395-403, `audio.set-volume`/`set-mute`, `playlist.playItem/prev/next/setShuffle/setRepeat`), every engine-absent surface renders a labeled disabled/reason row instead of fabricating state, and 74 behavioral tests assert the dispatches and DOM (executed in baseline: 136/136 files, 1705 passed). The hostile findings are two demo-only/dead seams and grammar nits: **Open Recent works only on DemoTransport** — the real desktop shell's submenu is permanently the disabled empty row because `recentOpenings` exists nowhere but demo-transport.ts:354; the `km` resolver consumed at 500/657/661/665 is a dead indirection whose fallback `useMsg`'s total chain makes unreachable (the file's own header documents this exact pattern being deleted elsewhere as dead); the subtitles panel's "Show Advanced Settings" footer can only open the AUDIO tools dialog (no subtitle tools tab exists); and the queue-meta templates have no plural/sub-hour grammar ("1 items", "0h 45m").

## 2. What actually works (verified)
— verified by `rg -n "selectTrack" src/screens/player/context.tsx` → 395-403: selectTrack maps kind → `tracks.set-video-track`/`tracks.set-audio-track`/`tracks.set-subtitle-track` and dispatches through the transport — the track radio rows my part renders are wired to the real command surface.
— verified by ControlBar.test.tsx:559-589 ("switching dispatches tracks.set-audio-track with the real id", "'Disable' dispatches the none selector", "'Automatic' dispatches the auto selector") + baseline ui-vitest.log.txt (136/136 files, 1705 passed, exit 0) — these tests execute and pass.
— verified by ControlBar.test.tsx:1135-1163 — queue meta composition asserts the exact string "2 items - 1h 30m" and the partial-durations case asserts count-only; mutation of renderQueueTab's composition fails them.
— verified by `rg -n "about" src/screens/settings/areas/index.tsx` → 278 `about: "global"` — the app menu's About row navigating to /settings/global lands on the real About panel, not a dead route.
— verified by Read ClientProvider.tsx:16-17 — context value is the memoized stable transport, so AppMenuButton's `memo` claim (position ticks don't re-render it) is structurally sound.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/controls/ControlBar.tsx:919-924
Code:
```ts
  // Open Recent — real data through the ONE funnel when the transport
  // remembers openings (the demo shell; the open-media dialog's own Recent
  // section reads the same getter); the honest empty reason row otherwise
  // (v2 §4.3's honest-unknown pattern, restated as a menu: discoverable,
  // labeled, inert — never a fabricated history).
  const recents = isDemoTransport(client) ? client.recentOpenings : [];
```
What is wrong: `recentOpenings` is a DemoTransport-only getter (`rg -n "recentOpenings" src/client` → demo-transport.ts:354 only; tauri-transport.ts has nothing). In the real desktop shell the Open Recent submenu is permanently the disabled "empty" row; the feature is implemented exclusively inside the simulation. The comment frames this as the transport "remembering openings", but no real lane ever will — nothing in the tauri transport or the shell records openings.
Why it matters: Demo theater: every browser/dev demo shows a working Open Recent (and working re-open click-through at 933-943); the product would ship it permanently inert. This is the same family as the wave's standing "demo shows what the product lacks" pattern.
What correct looks like: Persist recents client-side in the real lane (localStorage, the exact `lumen.profiles.v1` precedent two rows below at 1007-1011) or record openings in the shell host; the menu then reads one getter regardless of transport.
Confidence: HIGH — Verification: read + rg (getter grep result above); behavior UNVERIFIED at runtime (no GUI binary exists — W14-A01).

### F-002 [S3]
Where: ui/src/screens/player/controls/ControlBar.tsx:498-500 (definition at 97-100, A08's range)
Code:
```ts
  // w26.k07 (j07 P2-4): the queue meta composition's lane seam — the ONLY
  // live pre-registration resolver left in this file (see k07Resolver).
  const km = k07Resolver(m);
```
What is wrong: `k07Resolver` is `(m) => (key) => m(key) ?? k07Catalog[key]`, but `useMsg` is total — `return (key) => catalog[key] ?? messages[key]` (messages.ts:3016, with the comment "a missing entry can never render `undefined`") — so the `?? k07Catalog[key]` fallback is unreachable, and the k07 keys are already registered in the catalog (messages.ts:2938-2940). `km` is an identity function in practice; my range's call sites (657, 661, 665) could be plain `m(...)`. The file's own header comment (83-91) documents this exact dead pattern being deleted for f13/gfixC/i03/i16 because "their fallbacks were unreachable" — the k07 seam is the same lie, relabeled "LIVE".
Why it matters: Dead indirection whose comment claims liveness misleads the next maintainer into preserving "seam" machinery that can never fire; it is precisely the doc-vs-code drift this audit hunts.
What correct looks like: Delete k07Resolver/k07Catalog/K07LaneKey; call `m(...)` directly (the j07 P1 cleanup applied to itself), or keep the seam only while keys are genuinely absent from messages.ts.
Confidence: HIGH — Verification: read messages.ts:3011-3016 + registration at 2938-2940. (Definition-line findings owned by A08; cross-referenced here because the consumers are in my range.)

### F-003 [S4]
Where: ui/src/screens/player/controls/ControlBar.tsx:697-705 (and duplicate arm 706-716)
Code:
```ts
  const renderTrackFooter = (tab: string, close: () => void): ReactNode => (
    <QuietButton
      label={m("player.controls.advanced-settings")}
      onClick={() => {
        close();
        openToolsDialog(tab === "video" ? "video" : "audio");
      }}
    />
  );
```
What is wrong: `ToolsDialogTab = "audio" | "video"` (ToolsDialog.tsx:51) — the tools dialog has no subtitles surface, so the SUBTITLES panel's "Show Advanced Settings" footer opens the AUDIO tools dialog. The user on the subtitles panel clicks "advanced settings" and lands on audio adjustments.
Why it matters: Misleading affordance on one of the four panels; the label promises general "advanced settings" but the destination is tab-dependent and never subtitle-related.
What correct looks like: Either a subtitles tab in the tools dialog (frame parity) or a subtitle-specific footer label/destination (e.g. route to /settings/subtitles).
Confidence: HIGH — Verification: read ToolsDialog.tsx:51-64; both footer arms verified at 697-716.

### F-004 [S4]
Where: ui/src/screens/player/controls/ControlBar.tsx:655-665 (templates at messages.ts:2938-2940)
Code:
```ts
    const meta =
      knownSeconds.length === playlist.entries.length && total > 0
        ? km("player.controls.queue-meta-total")
            .replace("{count}", count)
            .replace(
              "{duration}",
              km("player.controls.queue-total")
                .replace("{hours}", String(Math.floor(total / 3600)))
                .replace("{minutes}", String(Math.floor((total % 3600) / 60))),
            )
        : km("player.controls.queue-meta").replace("{count}", count);
```
What is wrong: Grammar defects in the composed meta: (a) `"{count} items"` has no plural rule — a single-entry queue reads "1 items"; (b) `"{hours}h {minutes}m"` always prints hours — a 45-minute total reads "0h 45m". The committed test (ControlBar.test.tsx:1135) only covers the 1h 30m case, so neither defect is pinned.
Why it matters: Visible copy defects on a panel users see every time they open the queue; the i18n seam exists (this code was specifically reworked to be localizable, 646-649) but the en-US reference grammar itself is wrong.
What correct looks like: ICU-style plural selection for count and an hours-omitted branch when `total < 3600` (or a duration formatter reused from `formatDuration`, already imported at line 66).
Confidence: HIGH — Verification: read templates at messages.ts:2938-2940; test coverage read at 1135-1163.

### F-005 [S4]
Where: ui/src/screens/player/controls/ControlBar.tsx:683-687 (mounted at 767)
Code:
```ts
  /** Bookmarks/Chapters have no bar-context seam (the bookmarks engine data
   * lives behind the tools/dialogs surfaces, w26.c5) — honest faces. */
  const renderUnavailableTab = (): ReactNode => (
    <PanelReasonRow reason={m("player.controls.reason.unknown")} />
  );
```
What is wrong: The queue panel's Bookmarks and Chapters tabs permanently render only an "Unknown" reason row. A real BookmarksDialog exists (src/screens/player/dialogs/BookmarksDialog.tsx) and the transport carries bookmark data (demo-bookmarks.test.ts) — the data is reachable in the app, just not consumed here. The tabs are frame-required dead surfaces with honest faces.
Why it matters: Two of the queue panel's three tabs can never show content; a user opening the queue popover sees "Unknown" twice. Documented and honest (doc 08 §4.4), but it is a shipped dead surface, and the "reason unknown" copy is inaccurate — the reason is known (no bar-context seam), the copy says unknown.
What correct looks like: Consume the bookmark data the dialog consumes, or fold the tabs until a seam exists; if kept, use a truthful reason string ("not available in this view").
Confidence: HIGH — Verification: read + rg (BookmarksDialog.tsx exists; rg -ln bookmark lists demo-transport + dialogs).

## 4. Per-function table (my range only)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `renderTrackTab` (closure) | 553-627 (mine 557-627) | REAL | — | Engine-truth rows via trackRows (A08) + PanelRadioGroup; defaults section reads selected track + language with honest unknown fallbacks; audio "Output Device" + subtitle "External tracks" sections render reason rows (documented no-seam); rescan glyph is decorative (SettingsPanel.tsx:194-201 — aria-hidden, no handler: NOT a dead button). |
| `renderQueueTab` (closure) | 635-681 | REAL | S4 | Real entries, current-checked, click/arrow plays via `playlist.playItem`; meta composition F-004; partial-duration guard is genuine (tested at 1149). |
| `renderUnavailableTab` (closure) | 685-687 | REAL | S4 | F-005 honest dead face. |
| `renderTrackFooter` / `renderSettingsFooter` | 697-716 | REAL | S4 | Both open the real tools dialog via window CustomEvent (ToolsDialog.tsx:63-64 listens); subtitle→audio mapping F-003; playback-tab null footer matches frame per comment. |
| `OutputCluster` JSX (4× PanelPopover + NavigationSeam) | 718-776 | REAL | — | audio/subtitles share trackTabs with distinct initialTab; gear adds playback tab with REAL shuffle/repeat dispatches (527-551, context.tsx); queue panel F-005; AppMenuButton behind router-presence seam (bare-mount fallback `window.location.assign` — 163, no crash in tests). |
| `PanelPopover` | 786-866 | REAL | — | Open resets tab to initialTab (841-847); outside-pointer dismiss contains via rootRef — verified SettingsPanel renders NO portal (rg createPortal → none), so containment is correct; Escape restores focus (858 → close(true)); footer close is focus-neutral for the tools dialog handoff. |
| `AppMenuButton` (memo) | 890-1091 | REAL | S3 | 13 entries: 8 live actions (open-media event 958, network-stream/convert/keyboard-shortcuts/settings/about routes, customize event 1037, recents submenu), 4 honestly-inert rows (open-folder 963-968, open-disc 977-983, capture-device 985-991 + frame accelerators); accelerators from HOTKEY_BINDINGS data (957 etc.); recents F-001; profile flyout real localStorage store (ProfilesFlyout.tsx:43,137-163) — works in BOTH transports, unlike recents; memo premise verified (stable context, ClientProvider.tsx:16). |
| `ErrorSurface` | 1096-1113 | REAL | — | Renders typed `lastError` (stage/message/code) with dismiss wired to `actions.dismissError` (context.tsx:407-409); role="status" polite live region; tested at 604-667. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ControlBar.test.tsx:1135 "known total composes '2 items - 1h 30m'" | YES | Asserts the exact composed DOM string through the real PanelPopover → renderQueueTab path with a FakeTransport queue. |
| :1149 "partial durations stay count-only" | YES | Asserts `queryByText(/1h/)` is null — mutation that sums partials fails. |
| :559/569/579 track-selector dispatches | YES | Asserts the dispatched request (cmd + args) of the row buttons my range renders. |
| :738 "Open Recent opens onto the honest empty reason" | PARTIALLY | Pins the empty face but never exercises the demo-recents arm (no test feeds `recentOpenings`) — F-001's demo-only behavior is unpinned; a tauri-lane regression to "always empty" cannot be caught. |
| :669/:691/:711 app menu rows/accelerators/inert gates | YES | Row count, order, accelerator strings from keymap data, and inert/active gating all asserted. |
| :978 "gear opens the 4-tab settings panel; Playback carries the real shuffle/repeat seams" | YES | Asserts tab set + that toggling dispatches through playlist context. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "real data through the ONE funnel when the transport remembers openings" | ControlBar.tsx:919-920 comment | MISLEADING | Only the demo transport ever remembers (demo-transport.ts:354); real lane has no recents — F-001. |
| "the ONLY live pre-registration resolver left in this file" | ControlBar.tsx:498-499 comment | FALSE (not live in the fallback sense) | useMsg is total (messages.ts:3016); keys registered (2938-2940) — fallback unreachable — F-002. |
| "no-seam rows ... render inert with their frame accelerators — never theater" | ControlBar.tsx:874-879 comment | REAL | Rows 963-991 `disabled: true` + accelerators; test 711 pins the gating. |
| "Bookmarks/Chapters ... honest faces" | ControlBar.tsx:683-684 comment | REAL (honest) but the surface is dead | F-005: real bookmark data exists elsewhere (BookmarksDialog). |

## 7. Dead / duplicated / unreachable code
Dead: `km` indirection (F-002 — the `??` fallback can never fire); Bookmarks/Chapters tabs (F-005 — permanently-reason-row surfaces). Duplicated: `renderTrackFooter`/`renderSettingsFooter` are the same function ±`bare` prop (697-716); the outside-pointer-dismiss effect is written twice (813-822 and 908-917, PanelPopover vs AppMenuButton — verbatim pattern copy, documented as such). Unreachable: nothing in my range; product lane unreachable as ever (no GUI binary — W14-A01).

## 8. Dependencies & flags
Imports consumed in my range: SettingsPanel family (PanelSection/PanelRadioGroup/PanelValueRow/PanelReasonRow/QuietButton), MenuPopup + identityResolveLabel, buildProfileFlyout/createProfile/selectProfile/useMediaProfiles (localStorage `lumen.profiles.v1`), openPathRequest/openPathRoute, HOTKEY_BINDINGS (generated from keymaps/default.toml), GotoTimeDialog/openToolsDialog (window CustomEvent seam), i18n k07 keys. No feature flags. All 22 Icon names used exist in icons.tsx (verified by loop).

## 9. Open questions for the next agent
- A08 owns k07Resolver's definition (97-100) — F-002's deletion fix belongs to their range; confirm they flagged it.
- Does any real-lane spec require Open Recent (VLC parity)? If yes, F-001 upgrades from S3 to a product gap.
- The queue panel's Bookmarks tab vs BookmarksDialog: two bookmarks surfaces, one dead — is folding the tab tracked anywhere?
