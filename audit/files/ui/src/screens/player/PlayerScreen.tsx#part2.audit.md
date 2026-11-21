# AUDIT: ui/src/screens/player/PlayerScreen.tsx#part2 (lines 555-1218)
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Chunk boundary | part2 = 555-1218 (`PlayerScreen` provider wrapper + `PlayerScreenInner` through its closing brace at 1218). Part1 = 1-554 (module doc, canvasCatalog, menu builders, seams, interfaces — W15-A03); part3 = 1219-1554 (`PlayerDialogs`, `PlaylistsDock`, `NowPlayingMark`, `PanelRegion` — W15-A04). Seam at function boundaries; no overlap. Full file read for context; only 555-1218 audited in depth. |
| Lines / functions | 664 / 21 (PlayerScreenInner decomposed by concern in the table) |
| File verdict | SHALLOW (as a media player: the composition is real, tested UI chrome — but the video-display core it composes presents nothing for engine-decoded media in every buildable configuration) |
| Reachable from shipped entry point? | YES from the web app: `ui/src/main.tsx` → `App.tsx` Router → `PlayerRoute` (App.tsx:203 `<PlayerScreen variant=… onVariantChange=…>`) on routes /player, /player/music, /miniplayer; `npm run build` passes (baseline #22, ui-build.log.txt, built in 386ms). NO shipped GUI binary exists to host it: W14-A01 proved `apps/desktop-tauri/src/app.rs` cannot compile at HEAD (manifest verdict FALSE, S0:1). |
| Findings | S0: 1 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `Read` PlayerScreen.tsx 1-1554 (full context, part audited: 555-1218); `rg -n "PlayerScreen" ui/src` (App.tsx:203 mount + routes); `rg -n "canvas|LUF1|frame|channel" + Read MediaSurface.tsx` (frame-canvas half exists, desktop-shell-gated); `rg -n "set-frame-channel" ui/src apps/desktop-tauri/src` (command catalog + backend frame_channel.rs exist); `rg "desktop-tauri" audit/runs/W14-A0*.manifest.tsv` (W14-A01 app.rs FALSE S0:1 — host cannot compile); `rg -n "recentOpenings" -B4 -A12 ui/src/client/demo-transport.ts` + `rg -n "rememberOpening|private recent"` (getter comment "most recent first" vs actual oldest-first — confirmed); `rg -n "usePlayerEngineView" states/context-bridge.ts` (real event bridge + snapshot restore); `rg -n "describe|it(" PlayerScreen.test.tsx` (44 behavioral tests); `rg -n "player.canvas.placeholder" i18n/messages.ts` (sr-note text); `sed -n` verification of every quoted range. |

## 1. One-paragraph verdict
This is the most honestly-engineered screen in the repo — real keymap-data-driven shortcuts, real engine-event bridge, real context menus with disabled-not-dead rows, 44 behavioral tests — and it is also where the product's core function is missing: the media surface it composes can display engine-decoded video in NO buildable configuration. For file sources the `<video>` mounts with no src (a dark rectangle plus an identity stand-in); the engine-frame canvas half (`FrameCanvasSurface`) exists in the tree but mounts only inside the desktop shell, and W14-A01 proved the tauri host crate cannot compile at HEAD — so the one environment where frames could flow does not build. The screen says so itself (`data-placeholder="engine-wiring-4.H1"`, sr-only note), which converts this from fraud to a documented gap, but the user-visible core of a video player — showing the video — is still absent, and this composition owns that surface. Secondary: the module's "engine-wired nowhere" honesty comments are now stale against the landed w31.4 frame channel (misinforming screen-reader users in the shell), and the demo-source seam works around a real contract bug in the demo transport's `recentOpenings` ordering (oldest-first against a "most recent first" doc comment — a defect that also corrupts the Open Media dialog's Recent list, see F-003).

## 2. What actually works (verified)
— Verified by baseline #22/#23 (`audit/runs/baseline/ui-build.log.txt`, `ui-vitest.log.txt` — 1705 passed/1 skipped incl. PlayerScreen.test.tsx's 44 cases): the composition renders, the keymap/dialog/sidebar seams behave, url sources mount a real `<video src>`, idle sessions show the empty hero.
— Verified by read: the keydown handler (786-862) is data-driven end to end (`DIALOG_KEYMAP`/`CUSTOMIZE_DIALOG_KEYMAP`/`PLAYER_SHELL_KEYMAP` from generated keymap data; `fromEditableField` guards Ctrl+A select-all); effect deps list every closed-over value (`variant`, `hasMedia`, `actions`…); listeners all have cleanup (613, 626, 661, 775, 861); `setSidebarOpen((open) => !open)` functional update avoids stale toggles.
— Verified by read: `derivedMedia` (684-695) derives media presence from engine track truth, fabricates nothing (`hasChapters: false` conservative); `hasRealVideoSurface` (726-729) requires a url source AND engine-reported video before suppressing the identity stand-in.
— Verified by read + `rg recentOpenings demo-transport.ts`: the `.at(-1)` seam really does take the last element, and the demo transport really does return oldest-first — the workaround is correct against today's getter.

## 3. Findings

### F-001 [S0]
Where: ui/src/screens/player/PlayerScreen.tsx:1088-1102 (mount), 726-729 (url-only gate); cross-file: ui/src/components/player/MediaSurface.tsx:62 (`src = source.kind === "url" ? source.url : null`), MediaSurface.tsx:100-108 (canvas desktop-shell-only)
Code:
```tsx
                      {/* w26.i02 (h03 #2) — the real MEDIA SURFACE: a <video>
                          element mounted while a session is up (the UI half of
                          the 4.H1 render seam). Real bytes only: url sources
                          address the browser's media stack; file sources mount
                          with no src (honest empty dark surface — never a
                          fabricated poster or synthetic frames). */}
                      {hasMedia ? (
                        <MediaSurface
```
What is wrong: the player screen — the product's core surface — has no path that presents engine-decoded video in any buildable configuration. (1) File sources: `MediaSurface` gives the `<video>` a src only for `kind === "url"`; a file source mounts src-less — the engine may be decoding AVI/MJPEG frames at full rate, the user sees a dark canvas and a title card. (2) Engine frames: the `FrameCanvasSurface` half of `MediaSurface` mounts a `<canvas>` ONLY when `__TAURI_INTERNALS__` is detected (the desktop shell), binding `session.set-frame-channel` — and W14-A01 proved the desktop-tauri host crate cannot compile at HEAD (manifest: app.rs FALSE, S0:1), so that environment does not exist. (3) In the browser build (the only one that compiles, baseline #22), the frame canvas "NEVER renders" (MediaSurface.tsx:15-16).
Why it matters: this is the audit's litmus defect at the UI layer: a video player whose player screen cannot display video for engine-decoded media. The owner's complaint (an MP4 cannot be opened) terminates here visually even where the engine succeeds: the browser shell displays nothing for file sources; there is no desktop shell to display frames. The code labels this honestly (`data-placeholder="engine-wiring-4.H1"`, sr-note 1051), so this is a documented gap, not disguised theater — but S0's definition is the user-visible core being broken, and it is.
What correct looks like: a browser-build frame path (decode LUF1 records over the app/web transport without `__TAURI_INTERNALS__`), or a compiling desktop host shipping `session.set-frame-channel`; until either exists, every doc/matrix row claiming GUI video playback must say NOT STARTED.
Confidence: HIGH — Verification: read (both halves of MediaSurface traced) + baseline #22 (ui build OK) + W14-A01 manifest (host cannot compile; cross-cited, not re-run).

### F-002 [S3]
Where: ui/src/screens/player/PlayerScreen.tsx:1048-1051; cross-file: ui/src/components/player/MediaSurface.tsx:9-16 (frame-canvas half), ui/src/i18n/messages.ts:1517
Code:
```tsx
                  {/* Honest placeholder note: the DECODE/FRAMES half stays
                      engine-wired nowhere (card 4.H1 handoff — recorded in
                      ui/NEXT-STEPS.md). Screen-reader note says so. */}
                  <p className="lumen-player__sr-note">{m("player.canvas.placeholder")}</p>
```
What is wrong: the comment (and the w26-era module header it echoes) claims the decode/frames half is "engine-wired nowhere", but the file's own import (line 97) mounts `MediaSurface`, whose w31.4 half IS wired: a `<canvas>` bound to the shell's frame shipper via `session.set-frame-channel` (MediaSurface.tsx:147-152). The stale claim also ships to users: `player.canvas.placeholder` = "Video appears here during playback — the render surface arrives with the desktop build." (messages.ts:1517) renders as an sr-only note on every session — inside the desktop shell it would announce "the render surface arrives with the desktop build" while the adjacent canvas is presenting frames.
Why it matters: comment drift between lanes (w26 wrote the placeholder; w31.4 landed the wiring) misinforms both future maintainers and screen-reader users; "honest placeholder" text that outlives the placeholder is itself dishonesty.
What correct looks like: update the comment and the msgid to the w31.4 reality ("engine frames render in the desktop shell; the browser shell shows url sources only"), or gate the sr-note on the same desktop-shell probe the canvas uses.
Confidence: HIGH — Verification: read (both files; msgid text verified in messages.ts:1517).

### F-003 [S3]
Where: ui/src/screens/player/PlayerScreen.tsx:710-720 (workaround); cross-file defect: ui/src/client/demo-transport.ts:353-356 (getter), 982-984 (prepend)
Code:
```tsx
  // ORDERING NOTE (client-lane handoff): the demo transport's
  // `recentOpenings` getter DOCUMENTS "most recent first" but SHIPS
  // oldest-first (rememberOpening prepends newest; the getter reverses an
  // already-newest-first list). The most recent opening is therefore the
  // LAST element today; this seam takes `.at(-1)` and the player test
```
What is wrong: the demo transport's public getter contradicts its own contract — `get recentOpenings()` is documented "The open-media dialog's 'Recent' section (most recent first)" (demo-transport.ts:353) but returns oldest-first (`rememberOpening` prepends at 982-984; the getter `.reverse()`s at 355, flipping it). This seam compensates with `.at(-1)` and documents the handoff, which is the right local move — but the underlying bug is live for every OTHER consumer: the Open Media dialog's Recent list (the getter's own namesake consumer) renders the list oldest-first while claiming most-recent-first.
Why it matters: a public API whose doc comment lies is exactly the class of defect this audit exists to catch; the workaround here proves the defect is real and known, and the comment-only "handoff" leaves the user-visible dialog wrong (verified when auditing OpenMediaDialog.tsx — see that file's audit).
What correct looks like: fix the getter (delete the `.reverse()`), flip this seam to `[0]`, update the player test — the comment itself prescribes the exact change.
Confidence: HIGH — Verification: read (demo-transport.ts:353-356 and 982-984 both read; PlayerScreen's compensation read; player test name confirmed in PlayerScreen.test.tsx:277).

### F-004 [S4]
Where: ui/src/screens/player/PlayerScreen.tsx:880 (center first-only), 1177-1199 (bottom maps ALL regions)
Code:
```tsx
  const canvasRegion = regionsOf("center")[0];
```
What is wrong: anchor handling is inconsistent: top/right/left/bottom regions are all mapped and rendered, but only `regionsOf("center")[0]` renders — a layout config declaring two center-anchor regions silently drops the second (no warning, no error). Conversely, `regionsOf("bottom").map(...)` at 1177 renders a COMPLETE `<ControlBar />` inside every bottom region — a custom layout with two bottom regions renders two full control bars (duplicate transport rows, duplicate aria labels).
Why it matters: layout.v1 is a documented config surface (doc 07 §7.2; `layoutSource` prop); config-driven screens that silently ignore or duplicate regions turn author mistakes into invisible breakage.
What correct looks like: render all center regions (or reject a multi-center layout at parse time in `resolveVariantLayout`), and mount ControlBar once regardless of bottom-region count.
Confidence: HIGH — Verification: read (default layout single-center assumption traced to usage; no validation call between 629-632 and render).

### F-005 [S4]
Where: ui/src/screens/player/PlayerScreen.tsx:1078-1079
Code:
```tsx
                      {resolveOverlay(effectiveEngineState) === "empty" ||
                      resolveOverlay(effectiveEngineState) === "none" ? null : (
```
What is wrong: `resolveOverlay(effectiveEngineState)` is computed twice per render of the browse face (pure function over the same view) — trivial waste, and the double call invites drift if the two calls are ever edited differently.
Why it matters: hygiene only; no behavioral defect.
What correct looks like: `const overlay = resolveOverlay(effectiveEngineState);` before the JSX.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PlayerScreen` | 555-568 | REAL | — | Mounts PlayerControlsProvider + PlaylistProvider once around the composition; straight pass-through of props. |
| `PlayerScreenInner` — variant state/`setVariant` | 586-600 | REAL | — | Controlled prop wins; persists via `persistVariant`; callback memoized on `onVariantChange`. |
| — sidebar toggle listener | 609-614 | REAL | — | Window event seam; add/removeEventListener cleanup correct. |
| — customize-open listener | 622-627 | REAL | — | Per-open modal state; cleanup correct. |
| — `layout` memo | 629-632 | REAL | — | `resolveVariantLayout(layoutSource, variant)`; invalid source throws (tested, PlayerScreen.test.tsx:529). |
| — fullscreen wiring | 634-636 | REAL | — | `useFullscreen(rootRef)`; ref attached in both variant branches. |
| — canvas menu state + `closeCanvasMenu` | 643-648 | REAL | — | Focus restore on close; callback stable. |
| — pointerdown dismiss effect | 651-662 | REAL | — | Scrim dismiss without focus steal; re-subscribes only when menu open; cleanup correct. |
| — dialog open state | 667-668 | REAL | — | Per-open mounts (fresh form state). |
| — client/bridge/controls/playlist + `derivedMedia` | 676-696 | REAL | — | Engine-truth derivation; nothing fabricated; `hasChapters` conservative false. |
| — `surfaceSource` resolution | 698-729 | REAL | F-003, F-001 | queue-first, demo fallback with `.at(-1)` compensation; url+video gate for real surface. |
| — browse state (`browseId`/`previewOpen`) | 738-744 | REAL | — | Saved-playlist browse face; missing id → null. |
| — strip host | 748 | REAL | — | Portal target for the shell strip slot. |
| — chrome auto-hide | 755-765 | REAL | — | `enabled: isFullscreen`, locked by panels/customize; `showChrome` primitive dep (churn fix verified by comment + test 637). |
| — chrome mirror effect | 771-776 | REAL | — | data-lumen-chrome on `<html>`; idempotent remove in cleanup. |
| — keymap keydown effect | 786-862 | REAL | — | Data-driven combos; editable-field guard; deps complete; cleanup correct. goto gated on `hasMedia`; bookmarks/tools open on idle (VLC parity, documented). |
| — condition ctx / `visible` / `regionsOf` / `canvasRegion` | 864-880 | REAL | F-004 | Single truth source (J06-5); center first-only. |
| — panel gating (`sidePanel`…`sidebarVisible`) | 882-896 | REAL | — | Music-mode right regions ungated (documented); docked sidebar default-open; openPanels override. |
| — miniplayer branch render | 900-939 | REAL | — | Honest: missing layout regions → null, not a fake card; window-close host-owned; in-app close = real stop command. |
| — full render (body row, dock, canvas, context menu) | 941-1162 | REAL | F-001, F-002 | Canvas region: tab stop, ContextMenu/F10 keyboard path with anchor geometry; sr-note (stale, F-002); media surface (F-001); now-playing mark honest (overlay-free + no-real-surface); StateRouter engine-truth only; MenuPopup fixed at pointer. |
| — bottom band + strip portal | 1177-1199 | REAL | F-004 | Portal when inside shell; in-place otherwise. |
| — `ToolsDialogHost` / `PlayerDialogs` mounts | 1203-1215 | REAL | — | Full branch only for tools (miniplayer has no entry point — documented); shared dialog trio (J06-11 dedup). |

## 5. Test honesty (PlayerScreen.test.tsx, 1262 lines, ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "renders the canvas as a labelled region, not a fake <video>/<canvas> element" (59) | YES | Asserts the honest structure itself. |
| "a session mounts the <video> surface; url sources carry REAL bytes, file sources mount without a src" (277) | YES | Drives the demo transport through a real session open; asserts `src` present for url, absent for file. Pins the F-003 seam contract. |
| "the surface follows the session state and never drives it" (341) | YES | Emits state-changed events; asserts play/pause follows engine truth. |
| dock/sidebar toggle suite (173-201), affordance row (203-274) | YES | Real store, real events, aria-pressed assertions. |
| empty-state + layout-config suites (442-606) | YES | Custom layoutSource regions asserted; invalid layout asserts a THROW (no silent fallback). |
| chrome lock/auto-hide (608-663) | YES | Includes a pointermove-storm no-rerender assertion (J06-1). |
| keymap/fullscreen/ESC/dialog suites (664-869+) | YES | Exact-modifier negative tests (Ctrl+Shift+T NOT the combo), editable-field guard test, seam-event mount test. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The real MEDIA SURFACE is a <video> element … url sources render real bytes; file sources mount with no src" | module header 24-30, 36-44 | REAL | MediaSurface.tsx:62; PlayerScreen.test.tsx:277 (baseline #23 passed). |
| "the DECODE/FRAMES half of the media surface is still engine-wired nowhere" | header 36-39 + inline 1048-1050 | FALSE (stale) | F-002: MediaSurface's FrameCanvasSurface is wired to `session.set-frame-channel` (w31.4); only the desktop shell is missing (cannot compile, W14-A01). |
| "Record … Hide UI … Quit VLC render disabled — never theater" | header 59-63 | REAL | canvasMenuEntries rows `disabled: true` (351, 362, 488 — part1 range, verified in full-file read). |
| Keymap combos "from keymap DATA, never hardcoded" | header 58-60 | REAL | `CUSTOMIZE_DIALOG_KEYMAP`/`DIALOG_KEYMAP`/`PLAYER_SHELL_KEYMAP` + `acceleratorOf` all read HOTKEY_BINDINGS (191-228). |

## 7. Dead / duplicated / unreachable code
- No dead code in 555-1218. `onOpenMedia` prop deliberately unused by the default route host (App.tsx:198-201 documents why: double-fire fix) — API surface, not dead.
- The whole composition is unreachable as a SHIPPED BINARY (W14: host crate cannot compile); reachable in the browser demo build only.

## 8. Dependencies & flags
- Imports 40 modules — all in-tree (react, react-router-dom, react-dom, plus ui/src lanes). No feature flags, no env gates in this range. The desktop-shell gate lives in MediaSurface's `__TAURI_INTERNALS__` probe (cross-file).

## 9. Open questions for the next agent
- part3 (1219-1554): `PlaylistsDock` consumes the same `usePlaylists` store — check the create-row → NewPlaylistDialog wiring against the real store (my file #10 audits NewPlaylistDialog).
- Whoever owns `ui/src/client/demo-transport.ts` owns F-003's root cause: the `recentOpenings` getter. The fix instructions are written in this file's comment (710-717).
- Whoever owns `MediaSurface.tsx`/`frame-channel.ts`: F-001's browser-build half — is a non-Tauri frame path planned (ui/NEXT-STEPS.md)?
