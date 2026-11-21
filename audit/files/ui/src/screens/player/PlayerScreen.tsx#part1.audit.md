# AUDIT: ui/src/screens/player/PlayerScreen.tsx#part1 (lines 1-517)
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 517 (chunk 1 of 3; part2=518-1034 A02, part3=1035-1553 A04) / 12 units |
| File verdict | REAL-UNPROVEN (part1: menu/seam layer is real, data-driven, honestly-disabled, test-covered; whole-screen reachability blocked by unbuildable host) |
| Reachable from shipped entry point? | NO at HEAD — PlayerScreen is mounted by App.tsx router under /player; the only shipped host (`lumen-tauri`) cannot compile (W14-A01 E0425, baseline #2 exit 101). Real rendering proven only in vitest/jsdom (baseline #23: PlayerScreen.test.tsx in the 1705-pass set). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "MediaSurface" ui/src` → mounts at PlayerScreen.tsx:1062,1098 only; `rg -n '"(navigation\.…|video\.record|window\.hide-ui|media\.quit|playlist\.advanced-controls)"' generated/bindings.ts` → all 8 accelerator ids exist (bindings.ts:76,89,91,108,111,112,143,164); `rg -n "uiAction" keymap.ts PlayerScreen.tsx` → SHELL_UI_ACTIONS maps only "show-controls-fullscreen" (keymap.ts:245-247); `rg -n "data-lumen-chrome"` → set at PlayerScreen.tsx:773-775 (part2), consumed appshell.css:519-550; `rg -n "describe\(|it\(" PlayerScreen.test.tsx` → 60+ tests incl. canvas-menu describe at 902; sed verification of every quote. |

## 1. One-paragraph verdict
Part1 is the canvas context-menu and seam layer of the player screen, and it is built the way the rest of the honest UI is: accelerators resolved from generated keymap DATA (never hardcoded), unimplemented actions rendered disabled with disclosed reasons, commands routed to real playlist/transport seams, and a 230-line test suite covering pinned order, accelerators, and honest-disabled rows. The defects are peripheral but real: three advertised hotkey bindings (hide-ui Ctrl+\, quit Ctrl+Q, record Shift+R — plus the F1–F10 play-bookmark family) exist as enabled rows in the generated bindings data and the Settings hotkeys screen, but no handler implements their uiActions (`SHELL_UI_ACTIONS` maps only `show-controls-fullscreen`), so the settings screen advertises keys that do nothing in the player; the `NavigationSeam` fallback would navigate the tauri webview away from the SPA if it ever fired in production; and the module header still describes the media surface as "url sources render real bytes … file sources mount with no src" — i.e. the screen itself concedes it cannot show the user's local video files, the Wave-15 core defect, now inherited by every menu built here.

## 2. What actually works (verified)
— verified by `rg` over generated bindings → all 8 accelerator ids referenced by canvasMenuEntries exist in HOTKEY_BINDINGS (bindings.ts:76,89,91,108,111,112,143,164), so acceleratorOf never silently drops a row that has a frame accelerator.
— verified by PlayerScreen.test.tsx:902-1133 → canvas-menu tests assert pinned 14-row order + customize row, accelerators from DATA ("frame spelling"), honest disabled rows, real shuffle/repeat commands, Escape/outside-dismiss, ContextMenu-key and Shift+F10 keyboard open. Mutation-sensitive: changing a row's disabled logic or dropping a submenu breaks them.
— verified by reading variant.ts → loadPersistedVariant/persistVariant are real localStorage reads/writes (lumen.player.variant), so the variant state genuinely persists — unlike the settings seam (W11).
— verified by baseline #23 (audit/runs/baseline/ui-vitest.log.txt, 1705 passed/1 skipped) → this suite executed green.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/PlayerScreen.tsx:342-362 and 480-488 (+ ui/src/screens/settings/areas/hotkeys/generated/bindings.ts:143,164,76,30-33; ui/src/screens/player/keymap.ts:245-247)
Code:
```
      // No hide-UI surface exists yet (Player - hide UI.svg is the target
      // state; chrome auto-hide is idle-driven only) — disabled.
      id: "hide-ui",
      labelKey: tm("player.canvas.menu.hide-ui"),
      icon: "hide-ui",
      iconAccent: true,
      accelerator: acceleratorOf("window.hide-ui"),
      disabled: true,
```
What is wrong: The generated bindings register `window.hide-ui` (Ctrl+\), `media.quit` (Ctrl+Q), `video.record` (Shift+R) and the F1–F10 `advanced.play-bookmark-*` family with `disabled: false` and uiActions that NOTHING implements: `SHELL_UI_ACTIONS` in keymap.ts:245-247 maps only `"show-controls-fullscreen"`, and PlayerScreen's keymap handler (793-816) handles only goto-time/bookmarks/av-effects/advanced-controls. The canvas menu honestly disables three of them (quoted above), but the Settings → Hotkeys screen renders straight from the same data with `data-disabled={binding.disabled || undefined}` (HotkeysEditor.tsx:114) — i.e. it presents all of these as live bindings. A user pressing Ctrl+\ or F1 in the player gets nothing, with no disabled affordance or reason anywhere on that screen.
Why it matters: This is the exact dead-button class the wave hunts: an advertised input surface (Settings hotkeys list) whose rows have no executor. The menu discloses; the settings screen does not — inconsistent honesty for the same data.
What correct looks like: either the bindings data carries `disabled: true` (or a "no handler" flag the editor renders like the menu's disabled-with-reason rows), or SHELL_UI_ACTIONS grows the missing actions.
Confidence: HIGH — Verification: read bindings.ts, keymap.ts, HotkeysEditor.tsx; grep found zero other uiAction handlers.

### F-002 [S4]
Where: ui/src/screens/player/PlayerScreen.tsx:499-505
Code:
```
function NavigationSeam({ children }: { children: (navigate: (to: string) => void) => ReactNode }) {
  return useInRouterContext() ? (
    <RoutedNavigation>{children}</RoutedNavigation>
  ) : (
    <>{children((to) => window.location.assign(to))}</>
  );
}
```
What is wrong: The no-Router fallback does a full document navigation. In the only shipped context (tauri webview serving a bundled SPA on a custom protocol) `window.location.assign("/settings")` would either 404 or blank the app. The comment scopes it to "jsdom trees without a Router", and the shipped app always mounts the Router (App.tsx), so the branch is test-only — but nothing structurally prevents a future bare mount from shipping it.
Why it matters: A fallback that corrupts the app if ever reached is worse than throwing (the pattern the file itself preaches for layout parsing at 521-522).
What correct looks like: fallback should throw or no-op with a console error, matching the LayoutParseError precedent one screen over.
Confidence: HIGH — Verification: read; App.tsx Router wrap verified separately (my App.tsx audit).

### F-003 [S4]
Where: ui/src/screens/player/PlayerScreen.tsx:210-214
Code:
```
function fromEditableField(event: KeyboardEvent): boolean {
  const target = event.target as Element | null;
  if (target === null || typeof target.closest !== "function") return false;
  return target.closest("input, textarea, select, [contenteditable='true']") !== null;
}
```
What is wrong: The editable-field guard misses `[contenteditable=""]`, `[contenteditable="plaintext-only"]` and the bare `contenteditable` attribute (all editable states in HTML). A contenteditable region without the exact `='true'` spelling would have its Ctrl+A stolen by the customize dialog.
Why it matters: The guard exists precisely to never steal native shortcuts; it under-matches the spec.
What correct looks like: `input, textarea, select, [contenteditable]:not([contenteditable='false'])`.
Confidence: HIGH — Verification: read; no other editable spellings in repo contradict.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module header comment (composition + honest placeholders) | 1-75 | REAL | — | accurate: media surface = url bytes only, file sources src-less (verified in part2 renders 1062-1098) |
| imports | 77-140 | REAL | — | all resolve (tsc noEmit green in baseline #22) |
| PLAYER_TOGGLE_SIDEBAR_EVENT / PLAYER_OPEN_CUSTOMIZE_EVENT | 142-158 | REAL | — | CustomEvent seam constants; listeners mounted in part2 (610-627) — verified |
| CanvasLaneKey / canvasCatalog | 164-180 | REAL | — | merged lane catalog; duplicate strings with messages.ts verified identical for hide-ui/quit rows |
| CUSTOMIZE_DIALOG_KEYMAP | 191-206 | REAL | — | single-entry Map from bindings id `playlist.advanced-controls` (exists, bindings.ts:112); silently empty if id ever renamed — test at :895 pins Ctrl+A so drift is caught |
| fromEditableField | 210-214 | REAL | S4 | F-003 under-matching editable states |
| acceleratorOf | 220-228 | REAL | — | DATA-driven; " + " frame spelling; returns undefined harmlessly for unknown ids |
| canvasTrackSubmenu | 234-271 | REAL | — | honest no-media single disabled row; "Automatic" never fake-checked; auto/none/track radio rows call real actions.selectTrack |
| canvasMenuEntries | 280-491 | REAL | S3 | F-001 disabled trio; 15 rows + 3 seps matches frame+lead-approved 15th row; prev/next honest queueEmpty disable; shuffle/repeat call real playlist commands |
| NavigationSeam | 499-505 | REAL | S4 | F-002 test-only fallback corrupts SPA if reached in prod |
| RoutedNavigation | 507-510 | REAL | — | thin useNavigate wrapper |
| PlayerMediaContext / PlayerScreenProps (docs) | 512-553 (boundary at 517) | REAL | — | "present-but-invalid layoutSource throws" claim verified: variantLayout.ts → layout.ts LayoutParseError (layout.ts:26) + test :529 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| PlayerScreen.test.tsx:902-941 (14 rows + pinned order) | YES | exact row/label/order assertions on the rendered menu |
| :942-962 (accelerators from DATA) | YES | asserts the exact "Ctrl + Q"-style strings; hardcoding a string would pass until bindings data changes, then fail — correct pin |
| :963-999 (honest disabled rows; real commands) | YES | clicks enabled rows and asserts playlist/transport calls; disabled rows assert inertness |
| :1082-1133 (Escape/outside/keyboard open) | YES | behavioral, jsdom events |
| :59-70 (canvas is NOT a fake <video>/<canvas>) | YES | pins the placeholder honesty itself — the test suite asserts the no-canvas state is intentional |
| baseline run #23 | — | executed green (1705 passed/1 skipped) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "14 rows + 3 separators, REAL frame labels … Record/Hide UI/Quit render disabled — never theater" | PlayerScreen.tsx:51-63 | REAL | rows 342-362/480-489 disabled:true with disclosed reasons |
| "Accelerators are display strings produced from keymap DATA" | PlayerScreen.tsx:58-59 | REAL | acceleratorOf → HOTKEY_BINDINGS (220-228); all ids verified present |
| "url sources render real bytes …; file sources mount with no src until the engine lands" | PlayerScreen.tsx:25-29 | REAL (and damning) | part2 renders 1062-1098 confirm; the shipped desktop case (file open) shows nothing — the engine's LUF1 frame channel has no consumer in these branches |
| "A present-but-invalid source throws (LayoutParseError) — never a silent fallback" | PlayerScreen.tsx:520-523 | REAL | layout.ts:26 class + PlayerScreen.test.tsx:529 |
| keymaps/default.toml hide-ui/quit/record bindings usable | bindings.ts:76,143,164 (`disabled: false`) | FALSE in practice | F-001: no executor exists |

## 7. Dead / duplicated / unreachable code
No dead exports in part1 (both event constants consumed by part2 listeners + AppShell; NavigationSeam consumed by part2 canvas renders). Duplication: lane catalogs (w26.c6/f09/i02 key modules) duplicate strings also merged into messages.ts — by design ("identical before/after the Lead's merge"), verified identical for the rows checked; a sync test would be cheaper than the design doc.

## 8. Dependencies & flags
Depends on: generated bindings (hotkeys lane), generated commands (SourceSpec type), playlist context, controls context, i18n lane keys, react-router-dom. No feature flags. All imports typecheck (baseline #22 `tsc --noEmit` OK).

## 9. Open questions for the next agent
- A02 (part2): does the keymap handler forward unhandled uiActions to `onBinding` (App.tsx) — and does App.tsx do anything with them (F-001's blast radius)?
- A04 (part3): confirm the `lumen:open-customize` listener teardown doesn't leak when PlayerScreen unmounts mid-edit (listener is in part2 623-627; modal mount in part3).
