# AUDIT: ui/src/screens/player/states/states.css
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 780 / 33 selector blocks (5 class families) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (PlayerScreen.tsx:126 `import { StateRouter, ... } from "./states"` -> states/index.ts:7 `import "./states.css"`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg class cross-check (0 dead families); sed quote verification; rg token values |

## 1. One-paragraph verdict
Real, dense, working stylesheet for the player state overlays (loading/buffering/
error/unsupported), first-run hero, home face, playlist browse, and the floating
now-playing preview card. Every class family (.lumen-state-overlay/__hero/__home/
__browse/__preview) has live TSX consumers (StageOverlays.tsx, PlaylistBrowse.tsx)
— zero dead selectors at family level. It styles surfaces whose DATA is partly
demo-only: the home "Continue Watching"/"Listen Again" rows are populated solely
by `isDemoTransport(client) ? client.recentOpenings : []` (StageOverlays.tsx:504)
and "Frequently Visited Folders" is permanently empty by design — the header
comment admits "no folders seam exists in any transport" (line 335-336). The CSS
itself is honest; the surface it dresses overstates the product.

## 2. What actually works (verified)
— `rg -o "lumen-state-[a-z-]+" src -g '*.tsx'` -> all 5 families present in
StageOverlays.tsx / PlaylistBrowse.tsx; `comm -13` (css classes minus used
classes) -> empty, no dead families.
— stylelint-disable count = 18, each paired with a picture-frame justification
comment (verified by reading lines 291-330, 449-475, 602-625, 777-780).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/states/states.css:332-341 (header) vs 380-385
Code: ` * Listen Again (album squares) → Frequently Visited Folders (honest empty —
 * no folders seam exists in any transport). Frame geometry @1920 scaled to`
What is wrong: The sheet implements the full Player-5 home layout for three
sections, but the third section can NEVER hold content (no folders API anywhere),
and the first two only fill in the demo transport (StageOverlays.tsx:504 gates on
isDemoTransport). In a real desktop session the entire .lumen-state-home block
renders only "empty" paragraphs. Why it matters: dead surface styled to look
like a feature-rich home screen; a user sees an empty skeleton that implies
history/folders features that do not exist in the shipped backend. What correct
looks like: ship the sections only when the transport provides the seam, or
document the demo-only nature in user-visible UI, not just code comments.
Confidence: HIGH — Verification: read + rg of isDemoTransport call sites.

F-002 [S4]
Where: ui/src/screens/player/states/states.css:337-338 vs 384-385
Code: ` * the app's 1440 render (×0.75): video cards 316×177.75 r12 → 237×133 r9
 * on a 252px pitch (20px gap); album/folder squares 200×200 r12 → 150×150;`
then `gap: calc(var(--space-grid) * 5); /* frame card pitch 336 → 20px gap */`
What is wrong: 237px card + 20px gap (--space-grid 4px × 5, verified tokens.css:106)
= 257px pitch, contradicting the claimed "252px pitch" in the same comment.
The browse grid (line 543) uses 15px columns gaps to actually hit 252px
(237+15=252). The home row is 5px off its own documented frame pitch.
Confidence: HIGH — Verification: read + token value lookup (arithmetic).

F-003 [S4]
Where: ui/src/screens/player/states/states.css:24-26
Code: ` * Semantic tokens only (§3.2); raw px appears solely for structural
 * dimensions with no token (indicator sizes — same precedent as the 4.C1
 * shell's structural px).`
What is wrong: The file ships 18 raw hex colors (lines 293, 298, 303, 317, 319,
329, 451, 457, 463, 468, 474, 604, 606, 612, 619, 624, 779). Each has a
stylelint-disable + "picture-frame rule" justification (stays-dark canvas in
light theme), so this is a documented exception, but the blanket "semantic tokens
only" headline is contradicted by the file body. Comment overstates purity.
Confidence: HIGH — Verification: read + rg count of disables.

## 4. Per-function (selector block) table
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .lumen-state-overlay (+__cluster/__icon/__title/__label/__body/__action/__stagetag/__facts/__progress/__spinner/__bar/__fill) | 35-227 | REAL | - | consumed by StageOverlays.tsx |
| [data-chrome="hidden"] loading/buffering fade | 65-70 | REAL | - | matches useChromeAutoHide.ts behavior claims |
| @keyframes lumen-state-spin + motion media blocks | 208-228 | REAL | - | reduced-motion honored both directions |
| .lumen-state-hero family | 232-304 | REAL | - | EmptyOverlay consumer |
| light-mode pins (hero/scrim/bar) | 291-330 | REAL | - | hex pins w/ documented picture-frame rationale |
| .lumen-state-home family | 342-475 | REAL | F-001 | demo-only data; folders section permanently empty |
| .lumen-state-browse family | 488-625 | REAL | - | PlaylistBrowse.tsx consumer |
| .lumen-state-preview family (+slider overrides) | 641-780 | REAL | - | uses --lumen-slider-fill/thumb set by slider.css (verified slider.css:80,104) |

## 5. Test honesty
StateRouter.test.tsx:18 reads states.css text and pins skin strings — string
presence tests, not rendering tests; they would not catch visual regressions,
only selector deletion. Noted, not counted (test file owned by another agent).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "determinate bar renders only when the engine reported a real percentage" | states.css:11-12 | REAL-UNPROVEN | StageOverlays.tsx rendering not executed; claim cross-check deferred to StageOverlays owner |
| "error/unsupported are role=alert and never fade on idle" | states.css:19-20 | REAL | StageOverlays.tsx:213,263 `role="alert"`; fade selectors 65-66 scope to loading/buffering only |
| "no folders seam exists in any transport" | states.css:335-336 | REAL | rg for folder sources found none; OpenMediaDialog file/stream subset |

## 7. Dead / duplicated / unreachable code
No dead selectors (all 5 families consumed). Duplication: the 4 light-pin blocks
(hero/scrim/home/browse) repeat the same 6 hex values ~4 times — acknowledged in
comments as awaiting a "stays-dark canvas token" that does not exist yet.

## 8. Dependencies & flags
Consumes tokens.css variables (--space-grid=4px, --space-inset=8px,
--radius-panel=12px, --space-track-inset=2px — verified) and slider.css custom
properties (--lumen-slider-fill, --lumen-slider-thumb — verified slider.css:80,104).

## 9. Open questions for the next agent
StageOverlays.tsx owner: confirm the determinate bar truly renders only with
engine-reported percentages (states.css:11-12 claim is CSS-comment-only here).
