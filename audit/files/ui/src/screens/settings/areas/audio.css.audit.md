# AUDIT: ui/src/screens/settings/areas/audio.css
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 25 / n.a. (one class rule + comments) |
| File verdict | REAL-UNPROVEN (presentational; wired; no dead selectors) |
| Reachable from shipped entry point? | YES — audio.tsx:214 uses `.settings-area-audio-reason` for the Output-device not-ready reason caption (audio area, /settings/audio) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 dead-selector sweep (0 dead); grep consumer; full read |

## 1. One-paragraph verdict
One scoped rule: the honest reason caption inside the audio area's staged Output-device row (flex own-line + -2px margin to keep the two-line lead inside the 36px row pitch — the geometry rationale is documented). Tokens only. Clean.

## 2. What actually works (verified)
— verified by sweep + consumer grep (audio.tsx:214 with role="note" data-capability-reason).

## 3. Findings
None.

## 4. Per-function table
n.a. — stylesheet (<100 LOC).

## 5. Test honesty
n.a. (area tests own structure).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the row pitch never grows" (caption fits 36px min-height) | L14-17 | REAL-UNPROVEN | geometry documented; visual harness owns proof |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Semantic tokens. None.

## 9. Open questions for the next agent
None.
