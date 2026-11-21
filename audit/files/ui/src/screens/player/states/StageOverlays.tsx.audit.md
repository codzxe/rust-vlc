# AUDIT: ui/src/screens/player/states/StageOverlays.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 506 / 14 |
| File verdict | REAL-UNPROVEN (overlays genuine and vitest-proven; home face is demo-simulation-only; no shipped host binary at HEAD) |
| Reachable from shipped entry point? | NO (same as all ui/src — tauri host crate fails to compile at HEAD, app.rs:301 E0425 per W14). SPA-internal chain: PlayerScreen:1080/1126 `<StateRouter …>` → StateRouter.tsx:60 → these overlays; /player route App.tsx:303. Executed under vitest (baseline run #23). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 2 |
| Commands run | `rg -n "IdleSurface\|…Overlay" player/` (sole consumer StateRouter.tsx, mounted PlayerScreen.tsx:1080/1126); `rg -n "onRetry\|onOpenMedia" App.tsx PlayerScreen.tsx` (PlayerRoute App.tsx:203-207 passes NEITHER → retry always stub, hero CTA rides the window event); `rg -n "lumen:open-media"` (listener exists: AppShell.tsx:314 → OpenMediaDialog AppShell:851 — CTA is real in-SPA); `rg -n "recentOpenings"` (demo-transport.ts:354 ONLY — no real-transport seam); `sed -n '765,790p' crates/control-protocol/src/server.rs` (anti-replay: strictly increasing requestId per connection, protocol close on breach) + docs/05-command-layer.md:82 (`RequestId … monotonic per host connection, §4.1`); `rg -c "useRef\(0\)"` ui/src → ≥13 independent request-id counters (AppShell.tsx:299 uses base 0x9000 with a collision comment); `rg -n "SourceSpec = "` commands.ts:90 (file|url only — folders claim TRUE); matrix row plfeat-recent-items (feature-matrix.toml:1997) → not_started everywhere; StateRouter.test.tsx inventory: 53 tests. |

## 1. One-paragraph verdict
The five state overlays are the most honest components audited so far: progress is engine-truth or honestly indeterminate, unsupported/error surfaces render only the typed payload's data, and the first-run hero's CTA is genuinely wired (window event → AppShell:314 → OpenMediaDialog). The rot: the returning-user HOME face ("Continue Watching / Listen Again", Player-5.svg) is powered exclusively by the DemoTransport's in-memory history — no real transport exposes `recentOpenings`, so the feature silently cannot exist in any real desktop shell; the error card's Retry button is a permanently-disabled stub because the only production mount (App.tsx PlayerRoute) never passes `onRetry`; and HomeOverlay joins ≥13 UI components minting independent per-component request-id counters that violate the engine's per-connection monotonic requestId contract (doc 05 §4.1, enforced by control-protocol server.rs:771-783 with connection close).

## 2. What actually works (verified)
— Hero/Unsupported CTA wiring: `lumen:open-media` dispatch (StageOverlays:63-66) has a real listener — AppShell.tsx:314 `window.addEventListener(OPEN_MEDIA_EVENT, onOpenMedia)` opening OpenMediaDialog (AppShell:851). Verified by rg chain.
— Engine-truth progress: ProgressIndicator renders aria-valuenow ONLY when engine percent exists; tests assert "no fake number" (StateRouter.test.tsx:106).
— Honest error faces: typed stage tag + whitelisted detail keys only; test :165 "unknown detail keys are not surfaced".
— Baseline run #23 (ui-vitest, 1705 passed) includes the 53 StateRouter tests.

## 3. Findings

### F-001 [S2]
Where: ui/src/screens/player/states/StageOverlays.tsx:502-505 (with :416-419, 427-428)
Code:
```
export function IdleSurface({ onOpenMedia }: { onOpenMedia?: () => void }) {
  const client = useClient();
  const recents = isDemoTransport(client) ? client.recentOpenings : [];
  return recents.length > 0 ? <HomeOverlay recents={recents} /> : <EmptyOverlay onOpenMedia={onOpenMedia} />;
}
```
What is wrong: The HOME face (Continue Watching / Listen Again / Frequently Visited Folders — Player-5.svg, the returning-user idle canvas) renders only when the transport IS the demo simulation. `recentOpenings` exists on DemoTransport only (demo-transport.ts:354; rg across tauri-transport.ts/transport.ts: zero hits). On the real desktop transport, `recents` is always `[]` → the home face is unreachable; real users see the first-run hero forever.
Why it matters: The exact question of this wave — "is any UI element wired to a real backend?" — answers NO for the home face: it is wired to the simulation's fake history. All its tests pass against DemoTransport, creating coverage theater for a feature no real host can populate. (Mitigating: matrix row plfeat-recent-items honestly says not_started; the code comment :500-501 discloses "the demo transport's recents getter".)
What correct looks like: A `session.get-recent-openings`-style engine seam (or shell-side persisted history) feeding IdleSurface on every transport, with the demo implementing the same seam.
Confidence: HIGH — Verification: read + rg of all three transport files.

### F-002 [S3]
Where: ui/src/screens/player/states/StageOverlays.tsx:284-293 (mount chain App.tsx:203-207)
Code:
```
      <Button
        // Honest stub: retry only when a real action is wired — the UI never
        // offers a control that would fake recovery (doc 05 §1 C1/C3).
        variant="primary"
        disabled={onRetry === undefined}
        data-wiring-stub={onRetry === undefined ? "retry command dispatch — card 4.C2" : undefined}
```
What is wrong: In the only production mount, `onRetry` is never provided: App.tsx PlayerRoute (:203-207) mounts `<PlayerScreen variant onVariantChange>` only. Every error overlay in the real app therefore ships a disabled Retry button tagged `data-wiring-stub="retry command dispatch — card 4.C2"` — the "C2 wires real commands" promise (PlayerScreen.tsx:540) was never completed for the app shell.
Why it matters: Dead button in the primary failure surface: a user whose open fails sees "Retry" permanently greyed with no explanation (the data attribute is developer-facing only). The stub is disclosed, but disclosure lives in code, not in the UI.
What correct looks like: App route wiring a retry (re-dispatch last `session.open`), or rendering the button only when an action exists.
Confidence: HIGH — Verification: read + rg "onRetry" across App.tsx/PlayerScreen.tsx/StateRouter.tsx (only tests pass it).

### F-003 [S2→S3 noted] [S3]
Where: ui/src/screens/player/states/StageOverlays.tsx:422, 434-443
Code:
```
  const nextRequestId = useRef(0);
```
```
  const open = (source: SourceSpec, navigate: (to: string) => void): void => {
    nextRequestId.current += 1;
    void client
      .send(openPathRequest(source, nextRequestId.current))
```
What is wrong: HomeOverlay mints request ids from its own `useRef(0)` counter. The UI has ≥13 such independent counters (rg: AppShell, MediaSurface, config-settings, PlaylistProvider, OpenMediaDialog, BookmarksDialog, OnboardingScreen, context.tsx, useFullscreen, CustomizePanels, StageOverlays, ControlBar, MediaInfoProvider, NetworkStreamScreen); ids from different components interleave non-monotonically on the same transport. The engine's protocol contract is "RequestId … monotonic per host connection, §4.1" (docs/05-command-layer.md:82) and the control server ENFORCES it — a non-increasing id is `NonMonotonicRequestId` and the connection is CLOSED (crates/control-protocol/src/server.rs:771-783). Only AppShell.tsx:299 even attempts mitigation (base 0x9000, with a comment admitting the collision concern).
Why it matters: On any transport that enforces the documented contract, two components sending near-simultaneously (e.g. a home-card click racing a playlist provider refresh) can kill the host connection — every control dies until reconnect. Today it is latent (DemoTransport doesn't enforce; the tauri lane cannot compile at HEAD so the enforcing path is unreachable), but it is a protocol-contract violation baked across the UI.
What correct looks like: ONE transport-scoped monotonic id source (e.g. the transport assigning ids, or a shared module-level counter).
Confidence: MEDIUM — Severity reduced from S2 because the enforcing backend is unreachable at HEAD and the tauri in-process dispatch path's enforcement is UNVERIFIED; the contract text and TCP-server enforcement are verified.
(Recorded as S3 in the counts.)

### F-004 [S3]
Where: ui/src/screens/player/states/StageOverlays.tsx:437-441
Code:
```
      .send(openPathRequest(source, nextRequestId.current))
      .catch(() => {
        /* Transport unreachable — the player surfaces its own typed
         * errors; the card still routes (the dialog's openSource path). */
      });
```
What is wrong: Swallowed rejection. The comment's justification is only partially true: the PLAYER surfaces typed errors that arrive as command RESULTS; a transport-level rejection (IPC dead, promise thrown — tauri-transport.ts:156-159 throws a wrapped Error) surfaces NOWHERE. The user clicks a home card, navigation happens, and the open silently fails.
Why it matters: The checklist's swallowed-error pattern; failure invisibility in exactly the case (transport dead) where the user needs an error card.
What correct looks like: Route the rejection into the same error-overlay state (the engine-truth error view PlayerScreen renders).
Confidence: HIGH — Verification: read; rejection path traced through tauri-transport.ts send().

### F-005 [S4]
Where: ui/src/screens/player/states/StageOverlays.tsx:341-353
Code:
```
  const text = source.kind === "file" ? source.path : source.url;
  const lower = text.toLowerCase();
  if (source.kind === "url" && (lower.includes("ice") || lower.includes("radio"))) return true;
```
What is wrong: Presentation-only classification heuristic; substring "ice" matches any URL containing "ice" ("police.gov/stream"), "radio" likewise. The code's own [VERIFY] comment discloses this.
Why it matters: Misfiled cards (an Icecast VIDEO stream lands under "Listen Again"); disclosed, low-impact, presentation-only.
What correct looks like: Once any track-kind truth exists, use it (the comment already promises this).
Confidence: HIGH — Verification: read.

### F-006 [S4]
Where: ui/src/screens/player/states/StageOverlays.tsx:485-491
Code:
```
          <section className="lumen-state-home__section" aria-label={tm("player.home.folders")}>
            <h3 className="lumen-state-home__header">{tm("player.home.folders")}</h3>
```
What is wrong: The "Frequently Visited Folders" section renders a constant empty face — SourceSpec has only file|url kinds (generated/commands.ts:90), so no transport can ever populate it. A permanent-empty section is shipped.
Why it matters: Same dead-UI class as ControlBar's bookmarks tabs: a frame-mandated section with no possible data source. Honest (no fabricated folders) but still a section that can never show content.
What correct looks like: Render the section only when a folder source kind exists.
Confidence: HIGH — Verification: read + SourceSpec union verified.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `dispatchOpenMedia` | 63-66 | REAL | — | Event + optional prop; listener verified AppShell:314. |
| `ConeMark` | 72-96 | REAL | — | Decorative SVG, aria-hidden, currentColor; frame-cited (Onboarding.svg committed in design/figma/). |
| `OverlayShell` | 104-130 | REAL | — | role status/alert/presentation; aria-busy; card variant. |
| `ProgressIndicator` | 137-165 | REAL | — | Determinate only with engine value; correct aria min/max/now. |
| `LoadingOverlay` | 169-177 | REAL | — | Polite status. |
| `BufferingOverlay` | 181-189 | REAL | — | Same grammar; never blocks controls (CSS layering — states.css, not audited here). |
| `UnsupportedOverlay` | 195-242 | REAL | — | Whitelist keys format/codec from typed error; real CTA. |
| `ErrorOverlay` | 248-296 | REAL | F-002 | Retry stub always disabled in production mount. |
| `EmptyOverlay` | 305-319 | REAL | — | Hero; CTA real via event. |
| `isAudioSource` | 341-353 | REAL | F-005 | Disclosed heuristic. |
| `HomeNavigationSeam`/`HomeRoutedNavigation` | 358-377 | REAL | — | Correct router-presence guard (pattern duplicated 3× across codebase — hygiene). |
| `HomeCard` | 383-409 | REAL | — | Real button, aria-label, honest quiet glyph art. |
| `HomeOverlay` | 416-496 | REAL | F-001, F-003, F-004, F-006 | Demo-only data; per-component request ids; swallowed rejection; folders section constant-empty. |
| `IdleSurface` | 502-506 | REAL | F-001 | Demo-transport gate for the whole home face. |

## 5. Test honesty (StateRouter.test.tsx, 53 tests — sibling file covering these overlays)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "loading without engine progress is indeterminate — no aria-valuenow, no fake number" (:106) | YES | Asserts absence of fabricated progress — the honesty invariant. |
| "retry fires when a real action is wired; disabled honest stub otherwise" (:202) | YES | But it tests the PROP path; no test asserts the production mount provides one (it doesn't — F-002). |
| "shows title, body, and the Open-media affordance wired to the shell dialog event" (:225) | YES | Dispatches and asserts the event. |
| Home-face tests (HomeOverlay block, via StateRouter.test.tsx:14 import) | YES for demo data | All pass against DemoTransport recents — cannot catch F-001 (no real-transport case exists to test). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the cards are the REAL remembered openings" | StageOverlays.tsx:27-28 | FALSE in real hosts | recentOpenings exists only on DemoTransport (:504); TauriTransport has no history seam. |
| "No folder source exists in any transport" | StageOverlays.tsx:487-488 | REAL | SourceSpec = file|url (commands.ts:90). |
| "RequestId … monotonic per host connection" (engine contract the UI must honor) | docs/05-command-layer.md:82 | VIOLATED by UI pattern | ≥13 independent counters; server.rs:771-783 closes connection on breach. |
| "Recent media" feature status | feature-matrix.toml:1997 plfeat-recent-items | REAL (honest) | not_started all platforms — matches the no-real-seam reality. |

## 7. Dead / duplicated / unreachable code
- Folders section: constant-empty (F-006).
- `onOpenMedia` prop: never passed by the production mount (App.tsx:200-204 comment explains why — double-fire fix); remains for hosts/tests — documented API, acceptable.
- NavigationSeam pattern duplicated: ControlBar.tsx:155, PlaylistScreen.tsx:270, StageOverlays.tsx:358 — 3 copies of the same 20-line seam (S4 hygiene, counted here once).

## 8. Dependencies & flags
error-display.ts (stage/message key tables — not my file), laneMsg (shared resolver), demo-transport (recents), open-path (one funnel). No feature flags. states.css styles not audited here (A01 owns states.css? — wave list A01 has states.css).

## 9. Open questions for the next agent
- Does the tauri in-process dispatch (apps/desktop dispatch.rs) enforce monotonic requestIds like the TCP server? Decides F-003's true severity.
- MediaSurface.tsx:115 also starts ids at 0 — same F-003 pattern at the frame-channel bind; its owner should record it.
- Who owns the demo-transport recents lifecycle (do entries ever persist)? demo-transport.ts is another agent's file.
