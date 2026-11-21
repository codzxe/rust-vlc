# AUDIT: ui/src/components/appshell/AppShell.tsx#part2
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 429-855 of 855 (chunk part2; part1 = 1-428 is W15-A07's; boundary: part2 opens at the `osWindow` useState) / 12 items |
| File verdict | REAL (chunk) |
| Reachable from shipped entry point? | YES as UI code — AppShell wraps every non-onboarding route (App.tsx GatedShell :186-190); builds + tests green (baseline #22/#23). The desktop host that would exercise the OS-window half cannot compile at HEAD (W14-A01 F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "to=\"/playlist\"" AppShell.tsx` (:810, :822 — every row links bare) · `rg -n "openId|useSearchParams" PlaylistScreen.tsx` (:288 local useState — NO deep-link param exists) · `sed -n '1,120p' ui/src/client/window-controls.ts` (plugin:window\|minimize/toggle_maximize/close/is_maximized via `__TAURI_INTERNALS__`) · `sed -n '6,12p' apps/desktop-tauri/capabilities/default.json` (all four window permissions granted + core:default) · `rg -n "shell.window.(minimize|maximize|close)|shell.sidebar.count-suffix" ui/src/i18n/messages.ts` (:2319 "Items", :2334 "Stop and return to the player", :2339 "Toggle fullscreen", :2340 "Minimize to miniplayer") · `rg -n "forwardRef" IconButton.tsx` (:32 — ref forwarding exists; focus-rescue test AppShell.test.tsx:764-778) · `rg -n "plrow|Cool|navigate" AppShell.test.tsx` (rows asserted by name; no per-row destination test) · baseline: ui-vitest.log.txt (1705 passed), ui-build.log.txt |

## 1. One-paragraph verdict
Part2 is the shell's back half: the OS window-control cluster, the breadcrumb, the playlists sidebar's search/sort/view machinery, and the entire render tree. The wiring discipline holds up under tracing: the desktop-only − / □ / × buttons drive REAL `plugin:window|*` invokes through `__TAURI_INTERNALS__` with the four required permissions actually granted in capabilities/default.json, the maximize glyph tracks `tauri://resize` with honest last-known-state error handling and correct unlisten cleanup, the in-app trio's labels say exactly what the buttons do ("Minimize to miniplayer" / "Toggle fullscreen" / "Stop and return to the player"), and the create flow reuses the shared dialog + store with test-pinned persistence. The real defect is the sidebar's playlist rows: every row — list and grid — links to bare `/playlist`, discarding the clicked playlist's identity, and no deep-link mechanism exists for the library screen to open a specific playlist (`openId` is local state, PlaylistScreen.tsx:288), so the shell's per-playlist rows are functionally indistinguishable from the rail's library stop. Plus the same "1 Items" plural defect as the player dock, and the breadcrumb fallback's type-cast seam.

## 2. What actually works (verified)
— OS window controls are REAL chrome commands: window-controls.ts:69-74 invokes `plugin:window|minimize|toggle_maximize|close|is_maximized`; capabilities/default.json:7-11 grants `core:window:allow-minimize/-toggle-maximize/-close/-start-dragging`; browser mode probes null and renders nothing (:607 `osWindow !== null ? … : null`).
— The in-app trio is honestly labelled: messages.ts:2334/:2339/:2340 — "Stop and return to the player", "Toggle fullscreen", "Minimize to miniplayer" — matching onWindowClose (:415-420 stop+home) and onWindowMinimize (:412-414 navigate /miniplayer, disabled there :582).
— The maximize-state effect (:433-463) subscribes `tauri://resize`, cleans up its unlisten, and degrades to last-known glyph state on failure (logged, not swallowed silently).
— Focus rescue works: IconButton forwards refs (IconButton.tsx:32-38) and AppShell.test.tsx:764-778 asserts focus lands on the toggle when the sidebar auto-collapses — green in baseline ui-vitest.log.txt.
— Create flow: navigates to /playlist (no-op when already there, :528-531) and opens the SAME NewPlaylistDialog; persistence through playlistsStore test-pinned (:463-494).
— The strip host (:846-849) provides the portal target PlayerScreen's control band uses (verified from the player side in my part3 audit).

## 3. Findings

### F-001 [S3]
Where: ui/src/components/appshell/AppShell.tsx:810 and :822 (both views)
Code:
```
                    <Link to="/playlist" className="app-shell__plrow" draggable={false}>
                      <GlyphPlaylist size={14} />
                      <span className="app-shell__plrow-label">{row.label}</span>
```
```
                    <Link key={row.id} to="/playlist" className="app-shell__plcard" draggable={false}>
```
What is wrong: every saved-playlist row — in BOTH the list view and the grid view — links to bare `/playlist`. The clicked playlist's identity is discarded: the destination is the library list, identical for "Cool Songs", "Anime to Watch" and every other row, and identical to the rail's library stop. The library screen cannot even receive the intent — its opened-playlist state is local (`const [openId, setOpenId] = useState<string | null>(null)`, PlaylistScreen.tsx:288; no route param or query exists), and the player's dock (the other consumer of the same store) is the only sidebar that opens a specific playlist (PlayerScreen browse face, my part3 audit).
Why it matters: the sidebar presents per-playlist rows (header "Playlists", "N Items", row labels); a user clicking "Cool Songs" reasonably expects that playlist, and gets the generic library. Functionally every row is the same button wearing different labels — the specific-row affordance is decoration.
What correct looks like: a deep link (`/playlist?open=<id>` handled by PlaylistScreen, or a store-level `openPlaylist(id)` the screen hydrates from) so the row click opens the clicked playlist — matching the player dock's behavior.
Confidence: HIGH — Verification: read of AppShell + PlaylistScreen (openId local) + AppShell.test.tsx (no per-row destination assertion exists).

### F-002 [S4]
Where: ui/src/components/appshell/AppShell.tsx:747-749
Code:
```
            <span className="app-shell__sidebar-count">
              {String(rows.length)} {t("shell.sidebar.count-suffix")}
            </span>
```
What is wrong: `shell.sidebar.count-suffix` = "Items" (messages.ts:2319) — one playlist renders "1 Items". Same grammar family as the player dock count (my PlayerScreen#part3 F-002); the l10n system's message-format plural support is unused.
Why it matters: visible grammar defect on every shell route with the sidebar open; the string is capitalized so it reads as a label fragment, making the mismatch more visible.
What correct looks like: `{count, plural, one {# Item} other {# Items}}` through the formatter.
Confidence: HIGH — Verification: read + catalog value.

### F-003 [S4]
Where: ui/src/components/appshell/AppShell.tsx:483
Code:
```
    return hit ?? { labelKey: "app.name" as LaneKey, icon: "go-home" as IconName };
```
What is wrong: the fallback crumb double-casts (`as LaneKey`, `as IconName`) to squeeze registered values through the seam types — `app.name` is a registered MessageKey and `go-home` a valid IconName, so the casts are lying about types that already fit, and they suppress future compile-time checking of this expression (rename either constant and the cast hides it).
Why it matters: same cast-seam family flagged in the settings generator and onboarding audits; this instance additionally casts values that need no cast, indicating the seam types are mis-shaped rather than the values.
What correct looks like: type the fallback as the union it actually is (`MessageKey | LaneKey` already accepted by `t()`), or register `app.name` in the lane key module.
Confidence: HIGH — Verification: read; `app.name` used uncast at :563 (`t("app.name")` type-checks there).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| osWindow useState (probe) | 429-431 | REAL | — | desktop-only probe; `detectOsWindowBackend()` reads `__TAURI_INTERNALS__` presence — browser → null, cluster unrendered |
| osMaximized effect | 432-463 | REAL | — | resize subscription + refresh; honest last-known-state on error; correct stopped/unlisten cleanup |
| onOsMinimize / onOsToggleMaximize / onOsClose | 464-478 | REAL | — | each invokes the real plugin command, `.catch` logs (never silent) |
| crumb memo | 481-484 | REAL | S4 | F-003 casts; fallback = app name + home icon (honest for unknown routes) |
| sidebar state (search/sort/view) + effects | 486-509 | REAL | — | search-close drops filter via post-commit effect (documented purity fix); focus-on-open |
| rows useMemo | 511-522 | REAL | — | filter + naturalSort/localeCompare — same semantics as the player dock and /playlist |
| onCreatePlaylist | 528-531 | REAL | — | no-op navigation guard against history spam; test-pinned |
| banner render (clusters, demo badge, in-app + OS controls) | 534-645 | REAL | — | aria group labels; drag region attr (inert in browser); in-app minimize disabled on /miniplayer (:582) |
| rail render | 647-688 | REAL | — | NavLink active state = the one orange accent; add-media foot opens the shell dialog |
| sidebar render (header/search/meta/create/rows) | 690-833 | REAL | S3/S4 | F-001 rows; F-002 count; disabled ··· honest (:713-720); VirtualList null-guard documented as the list lane's lag (:805-814) |
| strip context + main | 835-842 | REAL | — | provideMain=false for screens owning <main> (one-landmark rule) |
| strip host + dialogs | 844-853 | REAL | — | empty strip div carries lumen-player classes so portaled controls.css keeps matching (documented workaround) |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| AppShell.test.tsx:92 banner anatomy (toggle aria-controls, drag region) | YES | attribute-level assertions |
| :232-235 sidebar tone/landmark; :381-413 rows by name after store re-hydration | YES (render-level) | catches store-wiring drift; does NOT assert row destinations (F-001 invisible to tests) |
| :463-494 create dialog + persistence + no-history-spam | YES | asserts navigation + store contents |
| :764-778 focus rescue on auto-collapse | YES | real DOM focus assertions |
| :557+ route-aware sidebar defaults (one-sidebar rule) | YES | navigations through the shell asserted |
All green: baseline ui-vitest.log.txt (1705 passed / 1 skipped).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "REAL OS window controls (− / □-restore / ×) — desktop shell only, driving plugin:window|…" | file header :42-46 | REAL | window-controls.ts:69-74 + capabilities/default.json:7-11 (commands + permissions both present) |
| "the rows read the REAL saved-playlists store … no dead chrome, no second data source" | header :20-23 | REAL (store) / SHALLOW (rows) | same playlistsStore verified; but rows navigate generically (F-001) |
| "Labels never changed — they always said exactly what the buttons do" | :571-575 | REAL | msgid copies verified (:2334/:2339/:2340) |
| "the sidebar persists openness in localStorage" | test header :47 | REAL | SIDEBAR_STORAGE_KEY lumen.shell.sidebar.v1 (:204, :233-239) |

## 7. Dead / duplicated / unreachable code
- F-001's rows duplicate the rail's destination functionally (both reach /playlist).
- The sidebar's search/sort/view cluster is a THIRD copy of the same machinery (shell sidebar, player dock PlaylistsDock, /playlist screen) — acknowledged shared grammar, not extracted; duplication risk noted (cross-file: player dock audited by me, /playlist by another agent).
- VirtualList renderItem null-guard (:805-814) papers over a documented list-lane lag — the clamp supposedly lives in the list lane (open question below).

## 8. Dependencies & flags
- react-router (Link/NavLink/useLocation/useNavigate), playlistsStore (localStorage), window-controls seam (desktop-only), i18n lane keys (w26.a10 + w28.window — registered per :134-138 note), stripContext. No feature flags. Desktop half unreachable in practice at HEAD only because the host crate cannot compile (W14-A01) — the UI side is complete.

## 9. Open questions for the next agent
- ui/src/components/list/VirtualList owner: does the windowed-range clamp the shell's guard (:805-814) describes actually exist? If not, the shell's `row ? … : null` is masking a crash-on-filter bug.
- PlaylistScreen owner: F-001's fix needs a deep-link surface on your side.
- w28.window keys registered? (comment at :134-138 says a10 keys are; verify w28 set the same way when auditing messages.ts).
