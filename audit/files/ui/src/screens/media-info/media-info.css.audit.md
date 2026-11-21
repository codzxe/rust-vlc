# AUDIT: ui/src/screens/media-info/media-info.css
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 191 / 17 rule blocks |
| File verdict | REAL (code) — two stale "temporary fix" comments describing already-landed fixes — UNREACHABLE from any shipped binary (W14) |
| Reachable from shipped entry point? | NO as product (desktop-tauri cannot compile, W14 SUMMARY l.19). Dev/web: imported by MediaInfoPanel.tsx:42 and MediaInfoScreen.tsx:14 (side region + /media-info route); all 16 base classes consumed; baseline rows 22-23 green. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg class-consumer sweep (16/16 used); rg token defs (all present; `--color-surface-panel-raised` defined at tokens.css:61 dark + :137 light); rg `--font-size-label|--font-size-section` (BOTH 15px at tokens.css:69/72); sed surface.css:308-312 (SectionHeader still consumes --font-size-label) |

## 1. One-paragraph verdict
Clean, token-only sheet: every selector consumed, honest-unknown styling for absent values, tabular numerals, one responsive grid without media queries. The two defects are stale documentation of "temporary" fixes: the w26.i18 16px-drift override is now a NO-OP (the label token was retuned to 15px, equal to section, so the override and its 16-vs-15 comments describe a fixed problem), and the light-mode card step's fallback token has already landed (defined in both modes), leaving a dead fallback behind a comment that still calls the token "pending". Neither mis-renders; both mislead the next maintainer about which fixes are still open.

## 2. What actually works (verified)
— verified by rg → 16/16 class families produced by MediaInfoPanel/MediaInfoScreen; both files import this sheet.
— verified by rg token sweep → all referenced custom properties defined; `overflow-wrap: anywhere` on values (l.89) correctly handles long stream names.
— verified: `[data-unknown]` row styling (l.94-97) demotes absent values to tertiary/regular — the "honest unknown" contract the panel tests assert.
— verified: `grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr))` (l.63) genuinely reflows side-panel vs page without a media query, as the comment claims.
— baseline rows 22-23: compiles, suite green (MediaInfoPanel.test.tsx contracts listed at l.14-15).

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/media-info/media-info.css:19-25 (and 35-37, 171)
Code:
```
/* w26.i18 (h19 §3/§4/§5 — the 16px drift): the SectionHeader primitive
 * ships --font-size-label (16px) where its own doc + §2's band pin the
 * 15px class. Usage-side pin until the surface lane retunes the primitive:
 * the section token (--font-size-section, 15px) is the §2-legal step. */
.lumen-mi .lumen-section-header__title {
  font-size: var(--font-size-section);
}
```
What is wrong: The token sheet has since been retuned: `--font-size-label: 15px` (tokens.css:69) and `--font-size-section: 15px` (tokens.css:72) are EQUAL. The override is a no-op, and both comments (l.19-22, l.36-37 "not the 16px label token", l.171) still describe a 16px drift that no longer exists.
Why it matters: The next maintainer reading this file believes the label token is still 16px and the primitive still mistuned — stale "temporary" fixes are how real drift gets reintroduced (someone "cleaning up" the override has no accurate record). It also hides that the surface lane already did its job.
What correct looks like: delete the override + comments (or rewrite them as history: "retuned at the token level in <change>; override kept as belt-and-braces").
Confidence: HIGH — Verification: token values read directly; surface.css:308-312 confirms the primitive still consumes --font-size-label (now 15px).
Note: no rendering bug results — the computed size is 15px either way.

### F-002 [S4]
Where: ui/src/screens/media-info/media-info.css:182-191
Code:
```
 * survived only via their #acacac border. Usage-side step to the raised
 * surface (light elevation runs lighter, Player - Light.svg ramp); the
 * pending panel-step token (theme lane i19) wins the day it lands. */
:root[data-theme="light"] .lumen-mi__section,
:root[data-theme="light"] .lumen-mi__state-card {
  background-color: var(--color-surface-panel-raised, var(--color-surface-raised));
}
```
What is wrong: The comment calls `--color-surface-panel-raised` "pending (theme lane i19)" — but the token IS defined, in both modes (tokens.css:61 `#191919`, :137 `#dedede`). The fallback `var(--color-surface-raised)` can never fire; the rule already resolves through the "day it lands" token.
Why it matters: Same stale-comment class as F-001: the tracked work item is done, the file says it isn't. (The Δ4 analysis itself is accurate — light surface-panel #c6c6c6 vs surface #c2c2c2 verified from the token sheet.)
What correct looks like: drop the fallback and the "pending" framing; the rule is simply the light card step.
Confidence: HIGH — Verification: token definitions read at both lines.

## 4. Per-function table (rule blocks)
| Selector(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-mi .lumen-section-header__title` override | 23-25 | REAL (no-op) | S4 | F-001 |
| `.lumen-mi` | 27-33 | REAL | — | column, flex:1, min-height:0 (scroll-safe) |
| `__title` | 38-44 | REAL | S4 | F-001 comment ("not the 16px label token" — both 15px) |
| `__section` | 48-53 | REAL | — | Card primitive carries border/radius |
| `__rows` | 57-65 | REAL | — | auto-fit minmax(15rem,1fr) — no media query needed ✓ |
| `__row` | 69-76 | REAL | — | label-left/value-right + hairline; `minmax(6rem, 42%)` label column |
| `__term`/`__value` | 78-90 | REAL | — | tabular-nums + overflow-wrap:anywhere |
| `[data-unknown]` | 94-97 | REAL | — | honest unknown: tertiary + regular weight |
| `__stream` family | 101-128 | REAL | — | border-block-start separators; icon+label row |
| `__note` / `__state-card` | 134-146 | REAL | — | honest states share the Card grammar |
| `.lumen-mi-screen` family | 156-180 | REAL | — | 680px column, page scroll in shell main |
| light card step | 188-191 | REAL | S4 | F-002 dead fallback + stale comment |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| MediaInfoPanel.test.tsx contracts (l.14-15: data-field, data-mi-section, data-mi-stream, data-track-id, data-state, headings) | YES (behavioral) | the panel suite renders and asserts structure; this sheet's classes are the rendering surface. Owned by W15-A10 — I verified only the import + consumer links, not the suite's internals (UNVERIFIED: pin-level CSS assertions, if any). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the SectionHeader primitive ships --font-size-label (16px)" | l.19-21 | STALE/FALSE at HEAD | tokens.css:69 label = 15px (retuned) |
| "the pending panel-step token (theme lane i19) wins the day it lands" | l.186-187 | STALE at HEAD | token defined both modes (tokens.css:61,137) |
| "light surface-panel #c6c6c6 is Δ4/255 on the #c2c2c2 surface" | l.183-184 | REAL | token values verified |
| "no media query needed" (two-column page / one-column panel) | l.61-62 | REAL | auto-fit minmax behavior |
| "Contracts kept (MediaInfoPanel.test.tsx)" | l.14-15 | REAL-UNPROVEN | suite exists + baseline green; internals owned by W15-A10 |

## 7. Dead / duplicated / unreachable code
Dead-in-effect: the F-001 override (no-op), the F-002 fallback (unreachable). No duplicated rules. Unreachable in shipped product (W14).

## 8. Dependencies & flags
Tokens: font-size-section, space-inset, font-family-sans, font-weight-semibold, color-content-bright, color-stroke-border, font-size-body, color-content-secondary, color-content-primary, font-weight-medium, color-content-tertiary, space-grid, color-surface-panel-raised, color-surface-raised — all defined. Class contract with the Card/SectionHeader/Button/ListEmpty primitives. No feature flags, no raw hex, no stylelint suppressions.

## 9. Open questions for the next agent
- Surface lane owner: is the SectionHeader 16→15 retune recorded anywhere as DONE, or do other sheets carry the same stale no-op override (rg `lumen-section-header__title` in *.css)?
- MediaInfoPanel.tsx (W15-A10): does the panel's "not-exposed verdict" state correspond to a real engine refusal path, or is media-info also fed by the hard-coded engine-info lane (W5/W13)?
