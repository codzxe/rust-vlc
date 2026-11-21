# AUDIT: ui/src/screens/player/tools/video/harness.css
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 48 / 6 rule blocks |
| File verdict | REAL — dev-only vision-harness styling, exactly as labeled; one global `body` rule documented for the standalone export |
| Reachable from shipped entry point? | NO, by design and verified: imported only by harness.tsx:18, which is imported only by harness.test.tsx (the W26C4_HARNESS_OUT snapshot exporter); unreachable from main.tsx. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg `harness.css` in tools/video (harness.tsx:18 + test path pin); rg importers of the video harness (test only); rg `w26c4` in harness.tsx (8 usages — all selectors consumed); rg `--color-surface-deep` (dark #0e0e0e — the canvas-between-surfaces claim consistent) |

## 1. One-paragraph verdict
Same honest dev-only pattern as the options harness: 48 lines staging the four video tool cards for 1:1 screenshot comparison against the 900px figma renders, imported only by the test-exported harness, tokens throughout. The canvas-choice comment is a genuinely good piece of harness engineering (surface-deep between surface and panel so both card states stay visible — a measured finding from the w26.c2 harness, cited). The one hygiene note: a global `body { margin: 0 }` rule lives in this file; harmless today because the file is unreachable from the app, but it is the kind of rule that bites if the harness ever mounts inside the app shell.

## 2. What actually works (verified)
— verified by import-chain sweep → harness.css ← harness.tsx ← harness.test.tsx only; the app bundle cannot include it.
— verified by rg → all 6 selectors (`w26c4-stage`, `--hdr`, `__caption`, `__frame`, `__hdr-open`, body) consumed/purposeful (8 class references in harness.tsx).
— verified: width arithmetic — 400px canvas − 2×20px insets = 360px cards (the frames' card width); 406 − 2×22 = 362px trigger ✓ matches the header's stated geometry.
— verified: dark surface-deep #0e0e0e sits between surface #090909 and panel #191919 — the "both card states stay evaluable" rationale is token-true.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/player/tools/video/harness.css:10-12
Code:
```
body {
  margin: 0;
}
```
What is wrong: A global element selector in a sheet that is (correctly) never linked from the app — but "never" is enforced only by import discipline, not by structure. If the harness ever mounts inside the app shell (e.g. a future dev route), this rule silently zeroes the app's body margin.
Why it matters: Dev-only sheets should scope their globals (`.w26c4-stage` owns layout; the body reset could be `body:has(.w26c4-stage)` or applied by the exporter).
What correct looks like: scope the reset to the harness root, or set it from the standalone-export generator where the "no app shell" condition actually lives.
Confidence: HIGH — Verification: read; the standalone-export purpose documented at l.8-9.

## 4. Per-function table (rule blocks)
| Selector(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `body` reset | 10-12 | REAL | S4 | F-001 global-in-dev-sheet |
| `.w26c4-stage` | 14-25 | REAL | — | 400px/20px; canvas-token choice documented with its measured rationale |
| `--hdr` | 27-30 | REAL | — | 406px/22px per HDR Options.svg |
| `__caption` | 32-36 | REAL | — | |
| `__frame` | 38-42 | REAL | — | 20px collapsed→expanded gap (frame-cited) |
| `__hdr-open` | 44-48 | REAL | — | 420px min-height for the open list |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| harness.test.tsx (video) | YES | reads this CSS by path (:34) for pins and renders the harness for the snapshot export (W26C4_HARNESS_OUT); the geometry above is what the 1:1 comparisons measure. Internals owned by that test's agent. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY … never imported by app routes" | l.1-2 | REAL | import chain: test only |
| "screenshots compare 1:1 against the 900px figma renders at a 2.25 device scale" | l.4-5 | REAL-UNPROVEN | arithmetic consistent (400/360 card geometry); the 2.25 scale claim belongs to the exporter (dev-shots.mjs — audited in an earlier wave) |
| "on the plain #090909 surface the collapsed pill vanishes — same token, by design" | l.21-23 | REAL | token values: surface #090909 vs card fill surface (same token) — the harness finding is token-true |

## 7. Dead / duplicated / unreachable code
None. (Deliberately unreachable from the app — that is its design.)

## 8. Dependencies & flags
Tokens: color-surface-deep, font-family-sans, font-size-caption, color-content-secondary (all defined). No feature flags.

## 9. Open questions for the next agent
- None for this file. If the harness family ever gains an in-app dev route, revisit F-001 first.
