# ui/src/screens/player/PlayerScreen.tsx — the screen wiring (W13-A02)

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A02 |
| File | `ui/src/screens/player/PlayerScreen.tsx` (1553 LOC) |
| Reachability | lumen-tauri binary → embedded ui/dist → router → AppShell main → PlayerScreen → canvas region (`.lumen-player__canvas`) → `MediaSurface` (lines 1062-1066 / 1097-1102) → `FrameCanvasSurface` → `session.set-frame-channel` bind → LUF1 frames → canvas. Events in: `usePlayerEngineView(client)` (line 677) ← `lumen://event` ← shell ticker. |
| Verdict | **Real wiring with one stale-doc wound: the screen's own comments and the screen-reader note still claim the decode/frames half is "engine-wired nowhere" — the opposite of the w31.4 code it mounts. The wiring itself (state → mount → surface) is coherent.** |

## Per-function table (>100 LOC; composition-level)

| Unit | Lines | What it does (sed-verified) |
|---|---|---|
| module docs | 1-76 | composition + "HONEST PLACEHOLDERS still true here" block — lines 36-42 claim the render engine is unwired (see Finding S3). |
| `canvasCatalog` / `CUSTOMIZE_DIALOG_KEYMAP` / `acceleratorOf` | 176-228 | menu label/keymap DATA resolution (no hardcoded strings). |
| `canvasTrackSubmenu` / `canvasMenuEntries` | 234-530 | the 14-row context-menu tree; every command is real (play/stop/prev/next via playlist+controls); record/hide-UI rows render DISABLED (lines 343-352, 354-363). |
| `PlayerScreen` | 555-568 | mounts `PlayerControlsProvider` + `PlaylistProvider` once around the inner screen. |
| `PlayerScreenInner` | 570-1160 | state: variant, sidebar, customize, canvas menu, dialogs; derives `bridgeView = usePlayerEngineView(client)` (677), `controls` (678), `playlist` (682); `effectiveEngineState = engineState ?? bridgeView` (683); `derivedMedia` = null when state is `"idle"` (684-695); `surfaceSource = queueSource ?? demoSource` (708-721); `hasRealVideoSurface` = url source with video track (726-729); renders `MediaSurface` when `hasMedia` (1061-1066, 1097-1102), `NowPlayingMark` identity stand-in otherwise (1113-1117), `StateRouter` overlays (1125-1131); context menu (1138-1157). |
| canvas region render | 1048-1158 | sr-note (1051), MediaSurface under the browse face and in the default face, overlays, MenuPopup. |

## The GUI chain through this screen (trace question)

Open → engine events → state → mount → bind → frames:
1. Any "Open media" affordance dispatches `window.dispatchEvent(new CustomEvent("lumen:open-media"))` (line 471; ControlBar.tsx:958) — AppShell owns the ONE listener and mounts `OpenMediaDialog` (AppShell.tsx:83, 851).
2. The dialog sends `openPathRequest(source, requestId)` — cmd `session.open` (open-path.ts:58-69) → `lumen_dispatch` → engine (see dispatch.rs audit).
3. Engine events (`session.state-changed` etc.) return on `lumen://event` → `usePlayerEngineView(client)` (line 677) → `effectiveEngineState.state` leaves `"idle"` → `derivedMedia` becomes non-null (lines 684-696) → `hasMedia` true → `MediaSurface` mounts (line 1098) → `FrameCanvasSurfaceLive` sends `session.set-frame-channel` (MediaSurface.tsx:147-152) → shell installs the tap → pacer ticks ship LUF1 records → `presentLuf1Frame` blits.
4. Controls (play/pause/stop/seek) ride `usePlayerControls` → the same single command.

## Findings

- **S3 — STALE DOC: the screen's own header and the live screen-reader note deny the frame path this screen mounts.** Sed-verified lines 36-42:
  ```
  // HONEST PLACEHOLDERS still true here:
  //   * render engine — the DECODE/FRAMES half of the media surface is still
  //     engine-wired nowhere (card 4.H1 handoff: sr-only note + this comment +
  //     ui/NEXT-STEPS.md). The <video> media surface below is the UI half only:
  ```
  and line 1051: `<p className="lumen-player__sr-note">{m("player.canvas.placeholder")}</p>` under the comment "the DECODE/FRAMES half stays engine-wired nowhere" (lines 1048-1050), and the canvas region carries `data-placeholder="engine-wiring-4.H1"` (PlayerScreen.test.tsx:63). All three contradict the w31.4 code in the same screen's own import (lines 94-97) and MediaSurface's canvas mount. A screen-reader user inside the desktop shell is told video will not appear while the canvas is blitting frames. Doc/a11y defect, not a functional one.
- **S3 — mount gating delays the bind until after the session is live** (lines 687-696 `effectiveEngineState.state === "idle" ? null : {...}` feeding `hasMedia`) — see shell.rs audit S3 (startup frame loss). The code's own alternative (`bind first: the pre-open stash` — shell.rs:656) exists at the shell; the UI just never uses it because the surface is conditional.
- **S4 — `surfaceSource` in the desktop shell is `queueSource ?? null`** (lines 708-721: `isDemoTransport(client)` is false in the shell, so `demoSource` is null). For a file source, the `<video>` half mounts src-less (honest) and the frame canvas does the showing — intended. `hasRealVideoSurface` (url-only, line 727-729) correctly keeps the identity card for file sources instead of hiding the frame canvas.
- **S4 — browse face keeps the surface mounted under the opaque grid** (lines 1055-1060: "the media surface stays MOUNTED underneath the opaque grid so url sources keep their real bytes and audio flowing") — for the shell's LUF1 canvas this also keeps the frame channel bound while browsing. Correct behavior; recorded.
- **S4 — 1553 LOC single screen** (menus, dialogs, variants, keymap, customize host) — sizeable but each region is data-driven; no dead component found (`MiniPlayer`/`MusicMode`/`GotoTimeDialog`/`BookmarksDialog`/`ToolsDialogHost`/`CustomizePanels` all referenced).

## Test honesty

`PlayerScreen.test.tsx` (1262 LOC, 12 describes) — honesty check for THIS audit's scope:
- Lines 59-70: `"renders the canvas as a labelled region, not a fake <video>/<canvas> element"` asserts `container.querySelector("canvas")` is NULL — PlayerScreen tests run in browser mode (no `__TAURI_INTERNALS__`), so **the shell-mode canvas is never exercised through the screen wiring**; its coverage lives only in MediaSurface.test.tsx's component-level mocks. The mount-gating + bind flow (state → hasMedia → MediaSurface → bind) has NO committed end-to-end test even at the seam level. S3 coverage gap.
- The open funnel tests (279-354, 1189) drive `openPathRequest` + `session.open` through a recording transport — the command half only.
- The `lumen:open-media` listener contract is pinned (445-464).
- 1705-test total: orchestrator-claimed + worklog-claimed (worklog.md:2280-2281); not re-run here (rg/sed-only mandate; `npm --prefix ui run test` not executed).

## Doc claims

- feature-matrix.toml: NO row for `ui-desktop-tauri`/GUI canvas exists (rg over the 3421-line file → zero hits for desktop-tauri/lumen-tauri). The w31.6 commit-message claim ("GUI video display end to end") is the strongest artifact; the feature matrix does not carry it — the matrix's GUI claims are absent, which is honest given the runtime-unproven status but inconsistent with the commit-message confidence.
- KNOWN-LIMITS.md:50 covers the shell being "real and shipped; deep verdicts open" — consistent with this audit's UNVERIFIED posture.

## Dead code / deps / flags

- `media` prop default null (line 572) + `engineState ?? bridgeView` (683) — the host-injection seam is live for tests/hosts. No dead exports found.
- Deps: React, react-router-dom, i18n, generated commands types, in-repo components — no `@tauri-apps/api` (allowlist posture held).

## Open questions

1. If the engine's playlist surface is not exposed for a single-session open (`playlist.entries` empty until `playlist.load` lands), `queueSource` stays null in the shell — harmless for the canvas (frames flow regardless), but the `<video>` half never gets a src and `hasRealVideoSurface` stays false: the identity card shows for file sessions. User-visible behavior in the shell: NowPlayingMark (not the filename poster) + the blitting canvas. UNVERIFIED at runtime.
2. `usePlayerEngineView` depends on the `[VERIFY at the ADR-004 admission build]` event-listen shape (tauri-transport.ts:93-98) — the entire state machine in this screen inherits that unresolved marker (S2, see app.rs audit).
