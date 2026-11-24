# AUDIT: ui/src/screens/player/states/StageOverlays.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 507 / 15 (12 components, 2 helpers, 1 classifier) |
| File verdict | REAL — honest presentational overlays; one face (Home) is demo-shell-only and unreachable in the shipped desktop shell |
| Reachable from shipped entry point? | YES (App.tsx:303-305 -> PlayerRoute -> PlayerScreen -> StateRouter.tsx:9-13 -> all overlays) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | rg `lumen:open-media` src (exit 0 — listener exists at OpenMediaDialog.tsx:56/App.tsx:199); rg `onRetry=|<PlayerScreen` src (exit 0 — only mount site App.tsx:203); sed PlayerScreen.tsx:535-600/1060-1135; Read App.tsx:265-329 |

## 1. One-paragraph verdict
Genuine, carefully-honest UI: five playback state overlays driven by the engine view (`usePlayerEngineView` default in PlayerScreen.tsx:677/683), i18n-only copy, no fabricated progress (`progress: number | null`, indeterminate when engine gave no number). The failure surfaces render typed error payloads only. Two real defects of reachability: the W26 "returning-user home" face (Continue Watching / Listen Again / Frequently Visited Folders) renders ONLY when the transport is the demo transport — in the shipped tauri desktop shell `recents` is always `[]` so the face is unreachable production-dead UI; and the ErrorOverlay's primary Retry button is permanently disabled in every shipped mount because no production caller passes `onRetry` (App.tsx PlayerRoute passes only variant/onVariantChange). Both are honestly labeled in-code but are dead surfaces in the product build.

## 2. What actually works (verified)
— verified by rg `lumen:open-media` -> the CTA event has a real listener (OpenMediaDialog.tsx:56 `export const OPEN_MEDIA_EVENT = "lumen:open-media"`; dispatched from ControlBar.tsx:958 too). The Empty/Unsupported "Open media" button is wired to the real dialog.
— verified by Read App.tsx:303-305 + PlayerScreen.tsx:677-683: overlays are driven by the default engine view from the client transport (context-bridge), not by props — so Loading/Buffering/Error/Unsupported appear in the shipped shell on real engine transitions.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/states/StageOverlays.tsx:502-506
Code:
```
export function IdleSurface({ onOpenMedia }: { onOpenMedia?: () => void }) {
  const client = useClient();
  const recents = isDemoTransport(client) ? client.recentOpenings : [];
  return recents.length > 0 ? <HomeOverlay recents={recents} /> : <EmptyOverlay onOpenMedia={onOpenMedia} />;
}
```
What is wrong: `recentOpenings` exists ONLY on the demo transport (rg: single definition demo-transport.ts:354). The tauri transport has no recents seam, so in the shipped desktop shell `recents` is always `[]` and `HomeOverlay` (Continue Watching / Listen Again / Frequently Visited Folders — L416-496) is UNREACHABLE dead UI in production; it renders only in the browser dev shell. Why it matters: a designed, tested, screenshot-documented product face ships dark. What correct looks like: a recents seam on the real transport (shell event or persisted history), or hiding the home face from product claims. Confidence: HIGH — Verification: read + rg (single `recentOpenings` definition).
F-002 [S3]
Where: ui/src/screens/player/states/StageOverlays.tsx:284-293, and App.tsx:202-207 (the only production mount)
Code:
```
      <Button
        // Honest stub: retry only when a real action is wired — the UI never
        // offers a control that would fake recovery (doc 05 §1 C1/C3).
        variant="primary"
        disabled={onRetry === undefined}
        data-wiring-stub={onRetry === undefined ? "retry command dispatch — card 4.C2" : undefined}
        onClick={onRetry}
```
What is wrong: the shipped app never passes `onRetry` — `PlayerRoute` (App.tsx:202-207) renders `<PlayerScreen variant={variant} onVariantChange={...} />` only; rg confirms no other production mount. The error card's ONLY action is therefore permanently disabled in the product build (a dead primary button), self-labeled `data-wiring-stub`. Why it matters: the error UX's primary affordance is theater-by-omission; "honest stub" is still a dead surface. Confidence: HIGH — Verification: read + rg `onRetry=|<PlayerScreen` (exit 0, one mount site).
F-003 [S4]
Where: ui/src/screens/player/states/StageOverlays.tsx:485-491
Code:
```
          <section className="lumen-state-home__section" aria-label={tm("player.home.folders")}>
            <h3 className="lumen-state-home__header">{tm("player.home.folders")}</h3>
            {/* No folder source exists in any transport (OpenMediaDialog's
             * honest file/stream subset) — the honest empty face, never
             * fabricated folder cards. */}
            <p className="lumen-state-home__empty">{tm("player.home.section-empty")}</p>
```
What is wrong: the "Frequently Visited Folders" section can NEVER show content in ANY shell (no transport models folders) — a permanently-empty section is dead UI by construction, even in the demo where the rest of the home face works. In-code honesty noted, but shipping a section whose only possible state is "empty" is decoration. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: ui/src/screens/player/states/StageOverlays.tsx:341-353
Code:
```
  const text = source.kind === "file" ? source.path : source.url;
  const lower = text.toLowerCase();
  if (source.kind === "url" && (lower.includes("ice") || lower.includes("radio"))) return true;
```
What is wrong: the "ice"/"radio" substring heuristic misfiles video URLs (Icecast video streams land under "Listen Again") — self-labeled [VERIFY] presentation-only limitation. Blast radius: only the demo-only home face (F-001), so the shipped build is unaffected. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: ui/src/screens/player/states/StageOverlays.tsx:436-442
Code:
```
    void client
      .send(openPathRequest(source, nextRequestId.current))
      .catch(() => {
        /* Transport unreachable — the player surfaces its own typed
         * errors; the card still routes (the dialog's openSource path). */
      });
    navigate(openPathRoute(source));
```
What is wrong: open failure is swallowed and navigation happens regardless; the user lands on the player even when the send rejected. Comment claims "the player surfaces its own typed errors", but a rejected transport send produces no engine event — nothing is surfaced for THAT click. Deliberate and commented, limited to the demo-only face. Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| OPEN_MEDIA_EVENT / dispatchOpenMedia | 61-66 | REAL | — | real listener verified (OpenMediaDialog.tsx:56) |
| ConeMark | 72-96 | REAL | — | static SVG, aria-hidden, currentColor |
| OverlayShell | 104-130 | REAL | — | role/aria-busy correct |
| ProgressIndicator | 137-165 | REAL | — | null=indeterminate, never fake %; `Math.round(progress*100)` unclamped if engine ever sends >1 (S4-grade, not numbered) |
| LoadingOverlay | 169-177 | REAL | — | progress from engine opening-progress |
| BufferingOverlay | 181-189 | REAL | — | non-blocking overlay |
| UnsupportedOverlay | 195-242 | REAL | — | renders only error-payload facts; CTA wired (F-001 sibling OK) |
| ErrorOverlay | 248-296 | REAL with dead button | S3 | F-002 retry permanently disabled in shipped mounts |
| EmptyOverlay | 305-319 | REAL | — | CTA wired via window event |
| AUDIO_FILE_EXTENSIONS / isAudioSource | 328-353 | REAL | S4 | F-004 heuristic |
| HomeNavigationSeam / HomeRoutedNavigation | 358-377 | REAL | — | router-context fallback |
| HomeCard | 383-409 | REAL | — | button + honest glyph, no fake thumbnails |
| HomeOverlay | 416-496 | REAL in demo shell / UNREACHABLE in product | S3 | F-001; folders section F-003 |
| IdleSurface | 502-506 | REAL | S3 | the gate that makes Home demo-only |

## 5. Test honesty
No tests in this file. Companion StageOverlays tests exist in states/ (not in my assignment); baseline vitest 1705 passed (orchestrator baseline §13) covers them — would they fail if HomeOverlay broke? Likely yes (demo transport recents used), but the shipped-shell unreachability (F-001) is precisely what tests cannot catch: all tests mount the demo transport. UNVERIFIED beyond baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the cards are the REAL remembered openings (one click re-opens...)" (L27-29) | module header | SHALLOW | real only in demo shell; demo transport's history is the "real" source — in product no history exists (F-001) |
| "the folders section renders its honest empty face — never theater" (L31-32) | module header | REAL as written | but see F-003: permanently-empty section |
| "Honest stub: retry only when a real action is wired" (L285-286) | code comment | REAL | and it never is, in shipped mounts (F-002) |

## 7. Dead / duplicated / unreachable code
HomeOverlay + HomeCard + isAudioSource + AUDIO_FILE_EXTENSIONS + HomeNavigationSeam (~170 LOC) are unreachable in the shipped desktop shell (F-001). Folders section dead in all shells (F-003).

## 8. Dependencies & flags
react-router-dom (useInRouterContext/useNavigate), components/button, components/icon, i18n messages + w26.gfixC keys, client ClientProvider/demo-transport/open-path, generated/commands types, laneMsg, ./error-display. No feature flags.

## 9. Open questions for the next agent
- error-display.ts owner: verify STAGE_MESSAGE_KEYS/ERROR_ACTION_MESSAGE_KEYS cover all engine stages/codes actually emitted by the desktop host.
- Who owns "card 4.C2 retry wiring"? It is still unshipped — cross-check docs/roadmap for a promise that retry works.
