# AUDIT: ui/src/screens/settings/areas/audio.css
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 25 / 1 selector block |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — `.settings-area-audio-reason` rendered at areas/audio.tsx:214 (the Output-device row's capability reason, role="note"), route /settings/audio. W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Grep 'settings-area-audio-reason' src (consumer audio.tsx:214 + test audio.test.tsx:140); token check (--font-size-caption, --color-content-tertiary exist — verified in the form.css pass). |

## 1. One-paragraph verdict
One honest rule, fully consumed, token-pure: the capability-reason caption line inside the audio row's lead (flex-basis 100% for its own line, −2px margin canceling the 4px line gap so the two-line lead stays inside the 36px row, overflow-wrap for long reasons). The header documents the defect it fixed (+22px badge drift) and what it deliberately does NOT carry (the staged trigger and info-dot live in shared sheets). Zero findings — checked: consumer exists, tokens exist, arithmetic (−2 cancels +4 gap; ~35px ≤ 36px) consistent, no specificity hazards (single class, no competing rule found).

## 2. What actually works (verified)
— Grep: class rendered exactly once in production (audio.tsx:214) and asserted in audio.test.tsx:140. Tokens exist. Baseline: audio area suites pass (ui-vitest.log.txt); `npm run build` OK (ui-build.log.txt).

## 3. Findings
None. (A file with zero findings must state what was checked: consumer, tokens, geometry arithmetic, specificity, duplication — all verified as above.)

## 4. Per-function table
| Selector block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.settings-area-audio-reason` | 18-25 | REAL | — | flex 1 0 100% own-line; -2px gap cancel; caption ink tertiary |

## 5. Test honesty
audio.test.tsx:140 asserts visibility of the reason caption — would fail if the class or its display were removed. (Other agent's file.)

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ZERO layout footprint … the two-line lead (~35px) stays inside the row's 36px min-height" | audio.css:12-16 | REAL-UNPROVEN | arithmetic consistent (13px line-height + 22px label line ≈ 35); not rendered/measured here |
| "the full-strength inert trigger comes from the shared [data-staged] rule in settings.css" | audio.css:5-7 | REAL-UNPROVEN | settings.css is another agent's file; delegation pattern matches the effects-area precedent cited |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
settings.css row grammar (parent layout), tokens only. No raw px except the 13px line-height (frame-marked inline via the caption scale — caption token carries the size; 13px is the line box).

## 9. Open questions for the next agent
- audio.tsx owner: what is `query.status` when the output-device capability probe fails in the browser shell — does the reason line render there (reachable-in-demo) or only in the desktop shell?
