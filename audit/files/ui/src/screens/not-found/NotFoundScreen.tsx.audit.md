# AUDIT: ui/src/screens/not-found/NotFoundScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 95 / 3 (useNotFoundMsg, ConeMark, NotFoundScreen) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (App.tsx:310 `<Route path="*" element={<NotFoundScreen />} />` — the catch-all route, lazy-loaded at App.tsx:65-66; any unknown path renders it) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read full file + NotFoundScreen.test.tsx (60 lines) · `rg -n "NotFoundScreen\|not-found\." ui/src` (route wiring, i18n key registration, test coverage) · `rg -n "M573.648 200.666" ui/src` (the cone path data — 3 copies) · `rg -n "<main" ui/src` (AppShell.tsx:837 provides the landmark; App.tsx:299-315 route groups) · `sed -n` on messages.ts:2122-2150 (key registration) |

## 1. One-paragraph verdict
A small, honest screen: routed as the catch-all, all three copy keys registered in the central catalog (the lane-fallback seam resolves central-first exactly as documented), the cone mark is decorative and aria-hidden, the single CTA routes to /player, and the tests assert every one of those contracts including the retired-backdrop regression and the actual navigation. Two hygiene defects: the frame-measured cone art is a three-way verbatim copy (this file, OnboardingScreen.tsx:315, StageOverlays.tsx:83) justified by a lane-rule that forbids cross-screen imports — a shared mark component would retire the drift risk the comment itself acknowledges; and the catalog still carries the superseded Solstice-era keys "not-found.title"/"not-found.body" ("Lost the thread…") with zero consumers, dead weight the l10n gate does not detect.

## 2. What actually works (verified)
— Reachable for real: unknown paths hit the `*` route (App.tsx:310); pseudo.render.test.tsx:47 renders "/definitely-not-a-route" through the real router in the pseudo-locale probe.
— i18n: all three rendered keys (`not-found.heading`, `not-found.body-quiet`, `not-found.action`) are registered centrally (messages.ts:2149-2150, :2124); the lane table (w26.b13.ts:25-26) is the never-hit fallback, matching the "REGISTERED (w26.b19)" claim.
— The "AppShell provides <main>" claim holds: AppShell.tsx:837 renders the landmark for this route group (App.tsx:299), pinned by AppShell.test.tsx:630 ("provides the <main> landmark by default (exactly one)").
— Tests are behavioral: heading level+name, CTA class, aria-hidden mark, no data-backdrop tier on mount/unmount, and the CTA navigation asserted through a location probe (:55-59). All green in baseline ui-vitest.log.txt (1705 passed).

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/not-found/NotFoundScreen.tsx:38-42, 54-65
Code:
```
 * the same 107×126 three-segment mark the onboarding hero and the player
 * empty-state use; geometry is frame-measured, duplicated here because the
 * onboarding module does not export it — cross-screen imports stay one-way
 * per the lane rules). `fill: currentColor` so the accent token drives ink.
```
What is wrong: the identical three-path SVG art (verified byte-identical path data starting `M573.648 200.666`) exists in three screen modules: this file (:55-64), OnboardingScreen.tsx:315+, and StageOverlays.tsx:83+. The duplication is documented and justified by a lane rule (no cross-screen imports), but the rule creates a 3-way lockstep maintenance burden for frame-measured art.
Why it matters: if the mark is ever retuned (frame re-measure, geometry fix), three files must change in unison; a miss leaves one screen rendering stale art with no test to catch it (no test compares the path data across copies).
What correct looks like: extract ConeMark to a shared module (components/mark/ or the assets lane) that all three screens import — the one-way lane rule should govern screen logic, not verbatim art.
Confidence: HIGH — Verification: rg of the exact path data; all three copies read.

### F-002 [S4]
Where: ui/src/i18n/messages.ts:2122-2123 (touching this screen's key namespace)
Code:
```
  "not-found.title": "Lost the thread",
  "not-found.body": "That page slipped off the reel. The player is still rolling.",
```
What is wrong: dead catalog keys. The screen renders `not-found.heading`/`not-found.body-quiet`/`not-found.action`; `not-found.title` and `not-found.body` (the superseded Solstice-flavored copy, per w26.b13.ts:23 "not-found.title/body strings" replaced) have no consumer anywhere in ui/src (grep: only the catalog and the lane-notes mention them).
Why it matters: catalog hygiene — the l10n:check gate (key coverage, duplicates) does not detect unused keys, so dead entries accumulate and mislead translators/pseudo-locale work into covering strings nothing renders.
What correct looks like: delete the two keys (and their w26.b13 lane-note references), or add an unused-key check to l10n-check.mjs.
Confidence: HIGH — Verification: exhaustive rg of both keys.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useNotFoundMsg | 32-35 | REAL | — | central-first resolution; fallback never hit (keys registered) |
| ConeMark | 44-68 | REAL | S3 | frame-exact art, fill=currentColor, aria-hidden + focusable=false; F-001 triplication |
| NotFoundScreen | 70-95 | REAL | — | one section + labelled heading + one CTA → navigate("/player"); no <main> (AppShell provides) |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :32 quiet hero, h1 "Page not found", primary CTA | YES | role/level/name + class assertions |
| :41 cone mark decorative | YES | aria-hidden asserted |
| :48 no data-backdrop tier on mount/unmount | YES | the Solstice-stage retirement regression |
| :55 CTA routes to /player | YES | navigation asserted via location probe |
All green: baseline ui-vitest.log.txt. Gap: no cross-copy art-consistency test (F-001).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "REGISTERED (w26.b19): the catalog now owns these keys … the lane table remains only as a never-hit fallback" | :28-31 | REAL | messages.ts:2149-2150, :2124 |
| "The screen renders NO <main> of its own — the AppShell provides it" | :9 | REAL | AppShell.tsx:837; App.tsx:299 route group; AppShell.test.tsx:630 |
| "exactly ONE way back to the player" | :6 | REAL | single Button; test :55 |
| "duplicated here because the onboarding module does not export it — cross-screen imports stay one-way per the lane rules" | :40-42 | REAL (as stated) / S3 (as design) | F-001 |

## 7. Dead / duplicated / unreachable code
- Duplication: ConeMark ×3 (F-001). Dead: the two superseded catalog keys (F-002). No unreachable code in the screen itself.

## 8. Dependencies & flags
- react-router-dom (useNavigate), Button primitive, i18n seam, not-found.css. No engine dependency (correct for a 404), no feature flags.

## 9. Open questions for the next agent
- StageOverlays.tsx and OnboardingScreen.tsx own the other two cone copies — their auditors should confirm the art matches (byte-identical today) and consider co-owning the extraction.
- l10n-check.mjs's unused-key blind spot (F-002) applies repo-wide — worth a dedicated check.
