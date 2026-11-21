# AUDIT: ui/src/screens/player/customize/toolbarItems.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 332 / 5 (CustomizeToolbarItems, ToolbarNavigationSeam, RoutedToolbarNavigation, useToolbarMsg, REPEAT_ORDER/TOGGLE_SIDEBAR_EVENT consts) |
| File verdict | REAL (code) — engine-bound tiles dead-end at HEAD (W14); ships a dead-button generator |
| Reachable from shipped entry point? | NO as product (desktop-tauri cannot compile, W14 SUMMARY l.19 — engine dispatches can never leave the webview). Dev/web build: mounted by ControlBar.tsx:207 (`<CustomizeToolbarItems />`); baseline rows 22-23 green. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | Grep `<CustomizeToolbarItems|CustomizeToolbarItems` (ControlBar.tsx:67,207 — the promised wiring EXISTS); read toolbarLayout.ts:1-60 (localStorage seam `lumen.player.toolbar.v1`, no engine command — verified claim); read transport.ts:49-87 + tauri-transport.ts:130-151 (requestId echo-only, no correlation); rg tile catalog (panels.ts:170-185: quit/replay/chapters/favorite/bookmarks are placeable tiles); read CustomizePanels.tsx:240-250 (place() accepts ANY catalog tile into the band) |

## 1. One-paragraph verdict
The consumer half of the toolbar-customization seam is genuinely wired: ControlBar mounts it, Save's `lumen:toolbar-changed` event keeps it live, engine tiles dispatch through the real transport with completion/event-confirmed cycle state (never optimistic), the bookmark unit crossing (s→µs) is correct, and shuffle/repeat read the confirmed playlist context. Two product-level defects: the customization catalog ships tiles (favorite, bookmarks, quit, replay, chapters, text-blocks) that the real bar renders as PERMANENTLY disabled buttons — a feature that manufactures dead controls on user request; and every engine-bound tile dispatches into a transport whose only real backend (desktop shell) cannot be built at HEAD, so in every buildable artifact every "engine-bound" tile is an honest no-change button. Smaller: hardcoded requestIds and silent catch on transport failure.

## 2. What actually works (verified)
— verified by grep → ControlBar.tsx:67 imports and :207 renders `<CustomizeToolbarItems />` — the header's promised 3-line wiring (l.8-11) is real, not aspirational.
— verified by reading toolbarLayout.ts:1-60 → persistence is localStorage (`lumen.player.toolbar.v1`) with strict normalization (unknown/duplicate ids dropped, missing ids appended), and the claim "no engine command exists for it (verified against the 1.D4 catalog)" matches the catalog (rg finds no layout command).
— verified by reading the cycle path (l.100-133, 274-285) → selections advance only on `completed` results and engine events (applyCycleCompletion/applyCycleEvent); errors and transport failures leave state untouched — the never-optimistic contract holds.
— verified by reading l.161-181 → bookmark sends `media.add-bookmark` with `secondsToUs(Math.max(0, Math.floor(state.position)))` — seconds→µs crossing at the send edge, matching the wire contract (comment cites the k17 fix of the last raw-seconds sender).
— verified by reading transport.ts:49-87 + tauri-transport.ts:130-151 → responses ride their own promise; requestId is echoed, never used for correlation — so the hardcoded ids (F-003) cannot misroute responses.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/player/customize/toolbarItems.tsx:287-299
Code:
```
            // UI-owned with no surface in this shell — disabled with a reason
            // (favorite, bookmarks, quit, the text-block family, replay,
            // chapters; the PlayerScreen quit-row precedent — never theater).
            return (
              <IconButton
                key={tile.id}
                data-field-id={tile.id}
                label={`${label} — ${t("customize.toolbar.unavailable")}`}
                disabled
```
What is wrong: The customize modal's catalog (panels.ts:170-185) offers quit/replay/chapters/favorite/bookmarks/text-blocks as placeable tiles; `place()` (CustomizePanels.tsx:240-243) accepts ANY catalog tile into the band; this component then renders those tiles as PERMANENTLY disabled buttons. There is no engine surface for any of them in the command catalog, so they can never become enabled in any build — the shipped feature is a machine that converts user customization into inert, grayed-out controls with an "unavailable" suffix.
Why it matters: This is the wave's "dead buttons" question answered in the affirmative, with the twist that the USER builds the dead bar. The honesty devices (disabled + reason label) are correct per-component, but the product ships a customizer whose catalog advertises 31 tiles of which a subset are guaranteed permanently dead — a customization feature that cannot deliver what its own palette promises.
What correct looks like: filter the palette to tiles that can act in this shell (the modal already knows which tiles are engine-bound vs UI-owned), or implement the surfaces (favorite/bookmarks need engine commands; quit/replay/chapters need shell routes).
Confidence: HIGH — Verification: read panels.ts catalog + CustomizePanels place() + this render branch.

### F-002 [S3]
Where: ui/src/screens/player/customize/toolbarItems.tsx:107-126 (and every engine-bound tile branch)
Code:
```
      const request = { cmd: tile.command, args, requestId: 1 } as unknown as CommandRequest;
      try {
        const response = await wire.send(request);
```
What is wrong: Every engine-bound tile dispatch goes through `wire` — the client transport. The only transport with a real backend is TauriTransport (`lumen_dispatch` IPC into the desktop shell), and that shell cannot be compiled at HEAD (W14). In every buildable artifact the transport is MockTransport/demo-transport, which answers `feature-not-exposed` for everything (transport.ts:64-70) — so playback-speed/zoom/aspect cycles, snapshot, bookmark sends etc. are all guaranteed no-ops in the browser build, and the desktop build does not exist.
Why it matters: The component's core value proposition ("engine-bound tiles dispatch their command", l.16-17) is unreachable in every artifact a user can run today. The code is honest about failures (no-change policy), but the whole tile band is decorative until W14 is fixed.
What correct looks like: the W14 compile fix is the prerequisite; on the UI side nothing to change — the dispatch grammar is correct and will work the day the shell builds.
Confidence: HIGH — Verification: transport chain read + W14 SUMMARY l.19. Filed at S3 for this file (the component's own code is correct; the dead-end is architectural and already banked by W14/W15 orchestration findings).

### F-003 [S4]
Where: ui/src/screens/player/customize/toolbarItems.tsx:113 and 172-176
Code:
```
      const request = { cmd: tile.command, args, requestId: 1 } as unknown as CommandRequest;
```
and
```
                    void wire.send({
                      cmd: "media.add-bookmark",
                      args: { time: secondsToUs(Math.max(0, Math.floor(state.position))) },
                      requestId: 2,
                    });
```
What is wrong: Hardcoded requestIds — every cycle/action dispatch is id 1, every bookmark id 2. Concurrent dispatches (rapid tile clicks) are indistinguishable server-side and in logs. The codebase's own pattern is an incrementing counter (MediaSurface.tsx:146 `nextRequestId.current += 1`).
Why it matters: Wire-hygiene drift; response correlation currently works only because each send owns its promise, but any future id-matching (streaming, cancellation, dedup) breaks silently.
What correct looks like: a per-component counter or a shared useRequestId hook.
Confidence: HIGH — Verification: read both sites + MediaSurface counter pattern.

### F-004 [S4]
Where: ui/src/screens/player/customize/toolbarItems.tsx:121-123
Code:
```
      } catch {
        // Transport failure — same honest no-change policy as the box.
      }
```
What is wrong: Transport failures are swallowed with no user feedback and no log — the tile silently does nothing. "Honest no-change" is honest about state, silent about the failure.
Why it matters: A user clicking a cycle tile on a dead transport gets zero signal; debugging production reports of "the button does nothing" has no trace. The comment even acknowledges the policy is a choice.
What correct looks like: a console.warn or the toast surface on failure (the codebase has one for the frame-channel bind, MediaSurface.tsx:157-166).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `useToolbarMsg` | 63-74 | REAL | — | 4-deep fallback chain ending in the raw key — a missing catalog key renders its own msgid (en-US hardcode per W11); acceptable last resort |
| `TOGGLE_SIDEBAR_EVENT`/`REPEAT_ORDER` | 77-80 | REAL | — | frozen contract strings; repeat order off→all→one matches the box |
| `CustomizeToolbarItems` (hooks/state) | 88-133 | REAL | — | all hooks before the empty-band early return (rules-of-hooks documented); subscription stays live while band empty |
| `dispatch` | 107-126 | REAL | S3/S4 | F-002 dead-end, F-003 requestId, F-004 silent catch; completion-only state advance correct |
| event subscription | 128-133 | REAL | — | applyCycleEvent on all engine events; unsubscribed correctly |
| band mapping + early null | 135-139 | REAL | — | unknown ids filtered (tileById null check); empty band renders nothing (frame -1) |
| spacer/divider | 155-160 | REAL | — | inert, aria-hidden separators ✓ |
| bookmark tile | 161-182 | REAL | — | real command, µs crossing correct; fire-and-forget (no error surface — F-004 family) |
| playlists/right-sidebar | 183-194 | REAL | — | window CustomEvent → shell seam (works in-browser: PlayerScreen listens) |
| vlc-settings/go-home | 195-218 | REAL | — | router navigate; seam fallback for bare mounts |
| shuffle tile | 224-236 | REAL | — | confirmed playlist state, aria-pressed when on; `playlist.setShuffle(!(…))` — undefined treated as false ✓ |
| repeat tile | 237-250 | REAL | — | cycles confirmed order; no aria-state label (mode not conveyed a11y — minor, noted) |
| action/toggle-action tiles | 251-266 | REAL | S3 | F-002; argsFor(playlist?.shuffle ?? null) honest null handling |
| cycle tiles | 267-285 | REAL | S3 | F-002; never-optimistic selection (j08 fix verified in code) |
| dead tiles | 287-299 | REAL | S3 | F-001 |
| `ToolbarNavigationSeam`/`RoutedToolbarNavigation` | 307-332 | REAL | — | useInRouterContext probe; hooks-legal split; window.location fallback for bare mounts |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| CustomizePanels.test.tsx:711 (renders `<CustomizeToolbarItems />`) | YES | the suite drives the box AND the real bar through a mock transport; asserts issued requests via MockTransport.sent |
Mutation check (reasoned): revert the µs crossing (send raw seconds) → the bookmark request assertion (if it checks `time`) fails; break cycleArgs selection advance → the "stuck at 1.25" regression test (j08) fails. Detailed case list owned by the CustomizePanels.test.tsx agent — I verified the mount and mock-transport pattern only. UNVERIFIED: exact per-tile coverage.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The Lead lands the wiring in ControlBar.tsx … `<CustomizeToolbarItems />`" | l.8-11 | REAL | ControlBar.tsx:67,207 |
| "engine-bound tiles dispatch their command" | l.16-17 | REAL code / UNREACHABLE at HEAD | F-002 (W14) |
| "disabled-with-reason … never theater" | l.24-27 | REAL (self-indicting) | F-001 — the disabled tiles ARE the shipped product of the customizer |
| "no engine command exists for [toolbar layout] (verified against the 1.D4 catalog)" | toolbarLayout.ts:8-9 | REAL | rg the catalog: no layout command |
| "the wire `time` is µs … crossed at the send edge" | l.163-165 | REAL | l.174 secondsToUs ✓ |

## 7. Dead / duplicated / unreachable code
No dead code (all branches reachable via the catalog). Duplicated: navigation seam duplicated from PlayerScreen/ControlBar's NavigationSeam (l.308-311 admits copying — third copy of the pattern). Unreachable: engine dispatch half (F-002).

## 8. Dependencies & flags
useClient, wire-time, transport types, Icon, IconButton, i18n keys (3 catalogs chained), usePlayerControls, usePlaylistOptional (direct module import — barrel gap documented), panels (tileById, cycleArgs), cycleState, toolbarLayout. CSS: customize.css. No feature flags. The `as unknown as CommandRequest` cast bridge (l.113) is documented forward-declared-command debt (record/PiP/HDR not in the wire union).

## 9. Open questions for the next agent
- panels.ts / CustomizePanels.tsx owner: does the modal's palette visually distinguish the permanently-dead tiles (F-001), or do they present as fully functional?
- cycleState.ts owner: applyCycleEvent matches "by args equality" — with hardcoded requestId 1 everywhere, can two different tiles' completions cross-match? (args differ per tile, so no — but verify.)
