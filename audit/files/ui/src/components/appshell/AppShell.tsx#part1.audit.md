# AUDIT: ui/src/components/appshell/AppShell.tsx#part1 (lines 1–428 of 855)
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 428 (chunk part1) / 13 functions+hooks, 5 type/const modules |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — AppShell is mounted by App.tsx GatedShell (App.tsx:179) around every route except /onboarding and /dev/tokens. W14 caveat: the tauri desktop binary that would host it fails to compile at HEAD (build-desktop-tauri.log.txt exit 101) — browser/demo + vitest are the only live executions (ui-vitest.log.txt, AppShell tests pass). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg 'playback.stop' src/generated/commands.ts (real catalog command) + src/client/demo-transport.ts:433 (simulated in browser); rg 'shell.window.minimize' src/i18n → messages.ts:2340 "Minimize to miniplayer" (honest label check); sed -n '429,470p' (chunk boundary confirm: part2 = OS-controls code); messages.ts useMsg read (3011-3019 — total-catalog contract). |

## 1. One-paragraph verdict
Chunk boundary: part1 = lines 1–428 (imports, lane i18n seam, rail/crumb data, sidebar persistence helpers, fullscreen hook, and the AppShell component's state/effects/callbacks up to the window-controls section); part2 (429–855, render tree) belongs to A04; the OS-window-controls comment block 422–428 is the seam. The logic audited here is real, defensively written (localStorage try/catch, fullscreen-API optional-chaining no-ops, focus-strand prevention, StrictMode-pure updaters), and the one command it dispatches (`playback.stop`, close button) is a real catalog command — but the dispatch helper drops every response and swallows every transport error with an empty catch, so the shell's one "real backend" action can fail silently. The lane-i18n `??` fallback is dead code by the catalog's own totality contract (kept, documented).

## 2. What actually works (verified)
— `playback.stop` exists in the generated command catalog (generated/commands.ts:24 union) and the browser simulation implements it (demo-transport.ts:433) — the close button is wired to a real command shape on both transports. Sidebar persistence round-trips `lumen.shell.sidebar.v1` with try/catch honesty (210-239). Baseline: AppShell suites pass in ui-vitest.log.txt (1705/1705); typecheck clean (ui-build.log.txt `tsc --noEmit` OK).

## 3. Findings
### F-001 [S3]
Where: ui/src/components/appshell/AppShell.tsx:301-309 (used at 415-420)
Code:
```
  const dispatch = useCallback(
    (request: CommandDraft): void => {
      nextRequestId.current += 1;
      void client.send({ ...request, requestId: nextRequestId.current } as CommandRequest).catch(() => {
        // Transport unreachable — screens surface their own honest errors.
      });
    },
    [client],
  );
```
What is wrong: the shell's only command dispatcher discards the response (`void`) and swallows transport failure with an empty catch. The justification comment — "screens surface their own honest errors" — is false for the shell's own use: `onWindowClose` (415-420) dispatches `playback.stop` from the title bar, where no screen is involved; if the transport is down or the engine errors, the user gets navigation home with playback silently un-stopped and zero feedback. Contrast part2's OS-window callbacks (469-477), which log every failure with console.error.
Why it matters: swallowed errors on the one genuinely-backend-wired button in the shell chrome; the difference between this and the OS-controls' honest logging is in the same file.
What correct looks like: inspect the response envelope (`if ("error" in response.result) console.warn(...)`) and at minimum log transport failures like the OS controls do.
Confidence: HIGH — read; demo-transport behavior verified by rg.
### F-002 [S4]
Where: ui/src/components/appshell/AppShell.tsx:128-132
Code:
```
  return useCallback(
    (key: MessageKey | LaneKey | W28WindowKey): string =>
      m(key as MessageKey) ?? laneCatalog[key],
    [m],
  );
```
What is wrong: `useMsg`'s contract (messages.ts:3011-3019) is a total catalog — `catalog[key] ?? messages[key]` — typed `(key: MessageKey) => string`; the `?? laneCatalog[key]` fallback can only fire for keys missing from BOTH catalogs, which the i18n chain forbids by design. The file's own note (134-138) admits the seam "is a no-op passthrough today" kept only as a wave-wide convention.
Why it matters: dead-by-contract fallback replicated across 40+ lane files (per the note) — divergence risk if one lane's copy drifts from the merged catalog and silently wins.
What correct looks like: retire the seam post-wave as the note plans; until then this is documented, accepted debt.
Confidence: HIGH — read messages.ts:3011-3019.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `useShellMsg` | 126-133 | REAL | S4 (F-002) | pre-registration seam, no-op today |
| `RAIL_ITEMS` (const) | 145-158 | REAL | — | 6 real routes, no invented stops |
| `CRUMB_ITEMS` (const) | 161-176 | REAL | — | prefix order correct (music/mini before /player) |
| `PLAYLIST_ROW_HEIGHT` / `SIDEBAR_ID` | 187-190 | REAL | — | |
| `DistributiveOmit` / `CommandDraft` (types) | 197-198 | REAL | — | distributive omit over the request union — correct TS |
| `SIDEBAR_STORAGE_KEY` / `NARROW_QUERY` | 204-208 | REAL | — | |
| `loadSidebarOpen` | 210-231 | REAL | — | miniplayer hard-false; player-family default closed; try/catch → open |
| `persistSidebarOpen` | 233-239 | REAL | — | swallow is honest here (session-only degradation, commented) |
| `useShellFullscreen` | 246-268 | REAL | — | optional-chained no-op hosts; pressed state never lies |
| `AppShell` (component, state/effects half) | 274-428 | REAL | S3 (F-001) | render half in part2 (A04) |
| — `dispatch` (inner) | 301-309 | REAL | S3 (F-001) | response dropped, errors swallowed |
| — open-media listener (inner effect) | 312-316 | REAL | — | ONE shell listener, cleanup correct |
| — `sidebarOpen` state + `stripHost` state | 319, 328 | REAL | — | stripHost consumed by part2 render |
| — `toggleSidebar` (inner) | 331-354 | REAL | — | miniplayer no-op gate; pure updater; DOM-query coupling to player dock documented |
| — focus-strand guard (inner effect) | 361-369 | REAL | — | hands focus to toggle on collapse |
| — narrow matchMedia (inner effect) | 371-383 | REAL | — | collapse-once, no persist clobber |
| — route-change one-sidebar (inner effect) | 397-408 | REAL | — | eslint-disable justified in comment; reads live state via guard |
| — `onWindowMinimize` | 412-414 | REAL | — | navigate("/miniplayer"); label "Minimize to miniplayer" is honest (messages.ts:2340) |
| — `onWindowClose` | 415-420 | REAL | S3 (via F-001) | real playback.stop + navigate |

## 5. Test honesty
AppShell.test.tsx belongs to another agent; part1's logic (sidebar persistence, toggle gating, one-sidebar rule) is asserted there per its header. Not executed by me; baseline vitest passes them (ui-vitest.log.txt).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "screens surface their own honest errors" | AppShell.tsx:305 | FALSE for shell-initiated commands | no screen involved in onWindowClose's dispatch (415-420) |
| "every user-visible shell string flows through here — no literals" | AppShell.tsx:123-125 | REAL | rg: no string literals in rendered labels; keys resolved via messages.ts |
| "no engine seam exists for saved playlists" | AppShell.tsx:86-87 | REAL | playlistsStore is localStorage (w26.f08 note); cross-check in PlaylistOptionsProvider audit |
| "playback.stop — a real command" | AppShell.tsx:416 | REAL | generated/commands.ts:24 catalog union |

## 7. Dead / duplicated / unreachable code
F-002 (dead fallback). `stripHost` state is written here, consumed in part2 — not dead, chunk boundary only.

## 8. Dependencies & flags
react-router-dom, IconButton/Icon/IconWrapper, list primitives (ScrollArea/VirtualList/naturalSort), stripContext, Panel, i18n seam files (w26.a10, w28.window — both exist), ClientProvider/useClient, app-transport probe, window-controls seam (part2), OpenMediaDialog + PLAYER_TOGGLE_SIDEBAR_EVENT, NewPlaylistDialog + playlistsStore, shellGlyphs. All imports exist (verified by ls/rg during architecture pass; typecheck clean in baseline).

## 9. Open questions for the next agent
- A04 (part2): does the demo/badged shell render the in-app window trio (minimize/fullscreen/close) in a plain browser, where "Close" cannot close anything? Labels are honest, but the trio is browser-decorative — check the render conditions.
- Does anything ever re-request after a swallowed dispatch failure (retry), or is the failure terminal until the next user action?
