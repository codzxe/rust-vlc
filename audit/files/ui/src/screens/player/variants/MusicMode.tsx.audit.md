# AUDIT: ui/src/screens/player/variants/MusicMode.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 154 / 4 |
| File verdict | REAL (honest-placeholder stage over real data: engine/playlist title truth, the real C2 seek/control bar, a test-pinned no-fake-art contract) |
| Reachable from shipped entry point? | YES (web app build): PlayerScreen.tsx:1002 renders `<MusicMode artRegion={canvasRegion} />` when `variant === "music"` (my part2 range); variant set via the G keymap toggle and the `/player/music` route (App.tsx PlayerRoute). Baseline #22 build OK, #23 vitest 1705 passed incl. variants.test.tsx's music suite (5 cases). No GUI binary hosts it (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `Read` MusicMode.tsx 1-154 (complete); `sed -n '355-420p' variants.test.tsx` (music suite: honest-placeholder assertions — `data-placeholder="artwork-engine-data-unwired"`, no `<img>`, no stage waveform, the real seekbar present in the controls band, dock toggle both directions incl. the MutationObserver microtask); `rg -n "MusicMode|music variant|lumen-music" variants.test.tsx` (CSS-rule assertions for the stage geometry); cross-refs: PlayerScreen part2 audit (mount site + G toggle at 833). |

## 1. One-paragraph verdict
The self-described "ONE theatrical screen" is the most honest theatrical screen imaginable: the artwork is a labeled placeholder (`data-placeholder="artwork-engine-data-unwired"`, sr-text "Album art appears once the engine reports it", no `<img>`, no fabricated art — all test-asserted), the title is engine-truth-first with the playlist entry as fallback and a quiet caption when neither exists, the old decorative stage waveform was DELETED in favor of restyling the real interactive seek cluster (test asserts the doubled surface is gone), and the transport is the real ControlBar. The only defects are seams: the stage's dock-toggle aria-pressed truth is mirrored by querying the parent's DOM attribute through a MutationObserver rather than shared state, and the lane-message resolver repeats the type-unsafe `key as MessageKey` cast pattern.

## 2. What actually works (verified)
— Verified by baseline #23 (ui-vitest.log.txt, 1705 passed): the music suite — honest art placeholder (region name, data attributes, no img, no waveform, real seekbar in the footer), dock open by default as the playlists browser (complementary region, left of the stage by DOM order), shell-toggle close + stage-toggle re-open with aria-pressed tracking the flip (the test even documents the microtask-queue reason it must await act).
— Verified by read: title resolution (98-100) — `state.mediaTitle ?? entryTitle ?? placeholder caption`; no fabricated metadata; the toggle dispatches the SAME `PLAYER_TOGGLE_SIDEBAR_EVENT` the shell banner uses (one seam, both directions).
— Verified by read: the MutationObserver lifecycle is correct — initial read in useLayoutEffect (pre-paint), observe with attributeFilter, disconnect on cleanup, jsdom guard.
— Verified cross-file: PlayerScreen (my part2 audit) renders `data-sidebar` on the player root (950) — the attribute this mirror reads — and mounts the real ControlBar in the bottom region for the music variant (1168-1199).

## 3. Findings

### F-001 [S4]
Where: ui/src/screens/player/variants/MusicMode.tsx:52-59, 86-94
Code:
```tsx
/** The player root selector — the stage lives inside the player screen and
 * the dock's open/closed truth lives on its `data-sidebar` attribute
 * (PlayerScreen owns the state; this component only mirrors it). */
const PLAYER_ROOT_SELECTOR = ".lumen-player";

/** Read the dock's open truth from the player root (post-commit DOM). */
function isPlayerDockOpen(): boolean {
```
What is wrong: the stage's `aria-pressed` truth is obtained by `document.querySelector(".lumen-player")` + a MutationObserver on its `data-sidebar` attribute — reading back state the parent already owns, through a duplicated selector string and the DOM, instead of prop or context. The coupling is documented and tested (the test's async-act comment shows the microtask cost), but a rename of the root class or the attribute silently breaks the toggle's honesty (aria-pressed stuck at the last observed value).
Why it matters: layering workaround with a string-contract nobody enforces; the observer also fires on every data-sidebar flip even when the stage is unmounted-free (it disconnects correctly, so no leak — verified).
What correct looks like: PlayerScreen passes `sidebarOpen` (or the playlist context carries it) to MusicMode as a prop — the data-sidebar attribute then remains pure presentation.
Confidence: HIGH — Verification: read (both sides of the mirror traced; test behavior read).

### F-002 [S4]
Where: ui/src/screens/player/variants/MusicMode.tsx:68-73
Code:
```tsx
function useStageMsg() {
  const m = useMsg();
  const laneCatalog: Record<string, string> = { ...i02Keys };
  return (key: MessageKey | keyof typeof i02Keys): string =>
    m(key as MessageKey) ?? laneCatalog[key];
}
```
What is wrong: the same type-unsafe `key as MessageKey` cast as ToolsDialog's `useToolsMsg` (that file's F-002): the cast lies to the compiler; the runtime `??` fallback is the actual mechanism. `laneCatalog` is also re-spread on every call (micro-allocation per render, trivial).
Why it matters: same as ToolsDialog F-002 — a silent runtime break if `useMsg` ever throws on unknown keys; the pattern is replicated across lanes rather than centralized (the settings areas DID centralize it via makeAreaMsg).
What correct looks like: use the shared `makeAreaMsg` factory (settings/areas/shared) or a type-safe `useMsgWith(laneKeys)` helper.
Confidence: HIGH — Verification: read (pattern compared across the three files I audited).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `isPlayerDockOpen` | 55-59 | REAL | F-001 | DOM read of the parent's attribute; guarded null. |
| `useStageMsg` | 68-73 | REAL | F-002 | Lane-widened resolver; sound runtime fallback; unsafe cast. |
| `MusicMode` (component) | 75-154 | REAL | — | Honest placeholder stage; title truth chain; playlists toggle (same seam as the shell); MutationObserver mirror lifecycle correct. |

## 5. Test honesty (variants.test.tsx music suite, ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| honest art placeholder (no img/waveform, real seekbar) | YES | Negative + positive DOM assertions pin the no-fabrication contract. |
| dock-left composition + real control bar + no library | YES | Role/attribute/DOM-order assertions. |
| toggle both directions + aria-pressed truth | YES | Event dispatch, attribute flips, aria-pressed checked after the observer microtask — pins the F-001 mirror's behavior. |
| G keymap toggle + CSS-rule assertions (448, 487-501, 534, 592-593) | YES | Keymap data drive + geometry pins on variants.css. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "an honest placeholder face until the engine reports artwork (no fabricated art, no Solstice sun)" (4-6) | this file | REAL | data-placeholder + test assertions (no img, placeholder caption). |
| "the old decorative stage waveform … is GONE; … there is exactly ONE seek surface and it is the interactive one" (19-23) | this file | REAL | Test asserts `.lumen-music__waveform` is null AND the real `.lumen-seekbar` present in the controls band. |
| "compact controls — the real C2 ControlBar drives transport … this component owns the stage" (12-13) | this file | REAL | PlayerScreen part2: ControlBar in the bottom region; footer data-variant="music" asserted by test. |
| "the toggle's aria-pressed honest across every flip — including ones this stage did not initiate" (83-85) | this file | REAL | MutationObserver path tested (async act comment); mechanism is F-001's seam concern, behavior is correct. |

## 7. Dead / duplicated / unreachable code
- No dead code. The lane-msg cast pattern is duplicated across lanes (F-002).
- Unreachable as a shipped binary (W14) — global UI condition.

## 8. Dependencies & flags
- usePlayerControls (engine truth), usePlaylist (entry title), Icon/IconButton, PLAYER_TOGGLE_SIDEBAR_EVENT from PlayerScreen (cross-module constant import — fine), i02 lane keys. No feature flags; MutationObserver jsdom-guarded.

## 9. Open questions for the next agent
- variants.css (not mine): the stage geometry assertions (min(61.5vmin, 554px) etc.) live in the test — whoever audits variants.css should check the ambient tint stays inside the near-black+orange system the header promises.
- PlayerScreen part3 (W15-A04): the docked sidebar the toggle drives — confirm the music layout's left region id matches "playlist" (the seam's contract).
