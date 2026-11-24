# AUDIT: ui/src/screens/player/variants/MusicMode.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 155 / 3 (isPlayerDockOpen, useStageMsg, MusicMode) |
| File verdict | REAL (layout stage, honestly placeholder-marked; its headline feature — artwork — is permanently unwired) |
| Reachable from shipped entry point? | YES — PlayerScreen.tsx:129 imports, :1002 `<MusicMode artRegion={canvasRegion} />` (music variant of the player screen); variants.test.tsx:402-412 exercises the dock toggle round-trip |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg MusicMode ui/src` -> PlayerScreen:129,1002; `rg "artwork\|album" crates/commands/src/catalog/ids.rs ui/src/generated/commands.ts` -> NO artwork command exists; `rg PLAYER_TOGGLE_SIDEBAR_EVENT PlayerScreen.tsx` -> :149 (def), :612-613 (listener) |

## 1. One-paragraph verdict
The music-mode stage is honest about what it is: a large artwork slot that
carries `data-placeholder="artwork-engine-data-unwired"` (:114) because NO
artwork command exists anywhere in the 64-command catalog (verified), a
title line that reads engine truth first and the playlist entry's title
second (real wiring through usePlayerControls/usePlaylist), and a playlists
toggle that dispatches a window event PlayerScreen actually listens to
(:612-613 — real UI-internal wiring, both directions). The one theatrical
element is the ambient tint, and it is labeled decorative. Nothing here
fakes backend data. The defect is scope, not honesty: the mode's defining
feature (the 738×737 artwork treatment) can never show art in any shipped
build — a permanently dead surface behind an honest marker.

## 2. What actually works (verified)
— Toggle chain: MusicMode:128 dispatches -> PlayerScreen:612 listener ->
data-sidebar flip -> MusicMode's MutationObserver (:91-92) mirrors it back
to aria-pressed. Verified by reading both ends; variants.test.tsx:402-412
pins the round-trip (vitest baseline).
— Title fallback chain :98-100: `state.mediaTitle ?? entryTitle` — engine
truth preferred, playlist second, placeholder caption third (:146-150).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/variants/MusicMode.tsx:114,134-141
Code:
      data-placeholder="artwork-engine-data-unwired"
      ...
        <figure className="lumen-music__artwork" aria-hidden="true">
          <span className="lumen-music__artwork-glyph">
            <Icon name="music" size={64} variant="strong" />
          </span>
        </figure>
What is wrong: the artwork slot is a permanent placeholder — no
artwork/album-art command exists in the catalog (rg over ids.rs and
generated/commands.ts -> nothing), so "real artwork renders here once the
engine reports it" (:134-135) can never happen in any shipped build. The
marker is honest; the feature is dead surface. Why it matters: the screen
is billed as "the ONE theatrical screen of the redesign" (:3) — its
centerpiece is a 64px music glyph inside a 738×737 frame. What correct
looks like: either an artwork extraction path in the catalog or the frame
scaled to what is real. Confidence: HIGH — Verification: read + rg.
F-002 [S4]
Where: ui/src/screens/player/variants/MusicMode.tsx:52,55-59
Code:
const PLAYER_ROOT_SELECTOR = ".lumen-player";
function isPlayerDockOpen(): boolean {
  return (
    document.querySelector(PLAYER_ROOT_SELECTOR)?.getAttribute("data-sidebar") === "open"
  );
}
What is wrong: global selector + first-match — if two player roots ever
mount (mini-player + main, route transitions), the toggle mirrors the wrong
root's dock state. Fragile but single-root today. Confidence: MEDIUM —
Verification: read.
F-003 [S4]
Where: ui/src/screens/player/variants/MusicMode.tsx:86-88
Code:
  const [dockOpen, setDockOpen] = useState(false);
  useLayoutEffect(() => {
    setDockOpen(isPlayerDockOpen());
What is wrong: initial render always claims dockOpen=false even when the
dock is open (aria-pressed lies for one paint until the layout effect
corrects it). Cosmetic. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| isPlayerDockOpen | 55-59 | REAL | S4 | F-002 selector fragility |
| useStageMsg | 68-73 | REAL | — | i18n lane seam, same pattern as DiscoveryScreen |
| MusicMode | 75-154 | REAL | S3 | F-001 artwork dead; toggle real |

## 5. Test honesty
variants.test.tsx:402-412 (dock toggle both directions, async act) — would
fail if the event seam or the observer broke. I did not run it separately;
covered by vitest baseline (1705 passed).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "real artwork renders here once the engine reports it" | :134-135 | UNREACHABLE | no artwork command in catalog (rg) — nothing can ever report it |
| "no fabricated art" | :6 | REAL | placeholder glyph only, honest data-placeholder marker |

## 7. Dead / duplicated / unreachable code
The artwork slot is runtime-dead by missing backend (F-001). No dead code
in-file; every element renders.

## 8. Dependencies & flags
usePlayerControls (context — engine state seam), usePlaylist, i18n w26.i02,
Icon/IconButton, PLAYER_TOGGLE_SIDEBAR_EVENT. No transport calls at all —
backend proximity is one context hop away.

## 9. Open questions for the next agent
- PlayerScreen/context auditor: is `state.mediaTitle` populated from engine
  events in BOTH transports, or only when media.get-info succeeds? If the
  demo transport seeds it, browser screenshots show titles the desktop
  build may not have.
