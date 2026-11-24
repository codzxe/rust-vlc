# AUDIT: ui/src/screens/playlist/subtitles/harness.css
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 64 / 8 class selectors |
| File verdict | REAL (dev-only tooling, isolated) |
| Reachable from shipped entry point? | NO by design: imported only by harness.tsx (DEV-ONLY vision harness) + harness.test.tsx; rg confirms no app route imports it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg import isolation check (exit 0) |

## 1. One-paragraph verdict
Dev-only comparison-stage styling for the subtitles vision harness: near-black stage at frame tones, fragment cells at measured frame dimensions, and Chromium `zoom` factors (1.73x / 2.22x) that match the figma render upscale ratios — the non-standard `zoom` property is fine for a Chromium-only internal tool and is documented as such. Isolation verified: the sheet is imported by harness.tsx and its test only, never by app routes, so it cannot leak into the shipped bundle through any route import.

## 2. What actually works (verified)
— rg isolation: zero imports outside the subtitles harness pair.
— All dimensions annotated with their frame source; tokens used except the intentional raw zoom/px geometry.

## 3. Findings
None. (If a bundler ever picks the harness up via a barrel export, this changes — currently no barrel exports the harness.)

## 4. Per-function table
N/A (small CSS): stage/cell/caption/frame selectors reviewed — 8/8 classes consumed by harness.tsx.

## 5. Test honesty
harness.test.tsx mounts the harness (baseline pass) — pins that the harness renders, not shipped styles.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY vision harness styling (never imported by app routes)" | L1 | TRUE | rg isolation |

## 7. Dead / duplicated / unreachable code
None (all classes live within the harness).

## 8. Dependencies & flags
Tokens; depends on lumen-dropdown classes inside the fragment (shared component, present).

## 9. Open questions for the next agent
- None.
