# AUDIT: ui/src/screens/network-stream/network-stream.css
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 294 / n.a. (pure stylesheet, 4 class families + element/scoped rules) |
| File verdict | REAL-UNPROVEN (presentational; wired; no dead selectors) |
| Reachable from shipped entry point? | YES — imported by NetworkStreamScreen.tsx:51 `import "./network-stream.css"`; /open-network-stream is a lazy App route (App.tsx:307) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 dead-selector sweep (4/4 class families used); grep import chain; header read |

## 1. One-paragraph verdict
Pure presentation for the open-network-stream dialog: frame-measured geometry (1048px column, 50px header strip), semantic tokens, honest `frame:` citations for raw px, documented override of the shared dialog chrome to match the SVG (95% fill + 25px backdrop blur, no stroke). No dead selectors, no raw hex outside comments, no hacks.

## 2. What actually works (verified)
— verified by python3 sweep (dead: []) and import grep (NetworkStreamScreen.tsx:51). Screen behavior (the URL open path) is the screen lane's scope, not this sheet's.

## 3. Findings
None. Checked: dead selectors, import, hex escapes, `!important`, media/hardcoded breakpoints (none found in grep).

## 4. Per-function table
n.a. — stylesheet. Families: `.network-stream-dialog` chrome override, `.network-stream` column, `__head/__body/__footer` anatomy.

## 5. Test honesty
n.a. — CSS; NetworkStreamScreen.test.tsx owns behavior.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only ... raw px appears solely for frame-measured structural dimensions" | header L12-14 | REAL | full read; px values carry `frame:` comments |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Semantic tokens, shared Dialog grammar. No feature flags.

## 9. Open questions for the next agent
None.
