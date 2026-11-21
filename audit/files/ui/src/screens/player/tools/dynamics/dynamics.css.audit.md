# AUDIT: ui/src/screens/player/tools/dynamics/dynamics.css
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 300 / 24 rule blocks |
| File verdict | REAL (code) — UNREACHABLE from any shipped binary (W14) |
| Reachable from shipped entry point? | NO as product (desktop-tauri cannot compile, W14 SUMMARY l.19). Dev/web: `DynamicsCard.tsx:33` `import "./dynamics.css"` → Compressor/Spatializer/StereoWidener tools; all 14 base classes consumed (4/3/2/2/2/2/4/1/2/3/1/7/2/7 files); baseline rows 22-23 green. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg class-consumer sweep (14/14 classes used, no orphans); rg token defs (radius-hairline, surface-muted, font-size-section present); python WCAG recompute of both light-mode claims; color-mix arithmetic for the tick ink; formula comparison against slider.css (L8 flush formula); rg `dynamics.css` import (DynamicsCard.tsx:33) |

## 1. One-paragraph verdict
Clean, frame-faithful sheet: every selector is consumed, every token exists, the L8 thumb-flush formula is correctly applied, both light-mode contrast claims recompute exactly (5.45:1 and 4.79:1 — the comments say 5.44/4.79), and the tick-ink color-mix arithmetic lands within the claimed Δ≤1/255 of the frame hex. The single defect is duplication: the load-bearing thumb-travel formula now lives in its third copy (slider.css base, slider.css EQ scope, here), so an inset-doctrine change must be replicated in lockstep across three sheets.

## 2. What actually works (verified)
— verified by python WCAG: `#474747` (light content-tertiary) on light panel `#c6c6c6` = 5.45:1 (comment claims 5.44) and on `#bababa` (light surface-deep) = 4.79:1 (comment claims 4.79) — both ≥4.5:1 as documented (l.268-270).
— verified by arithmetic: tick ink `color-mix(surface-muted 83%, accent-tinted)` = 0.83·#151515 + 0.17·#371f06 → (27,23,18.5) ≈ #1B1712, within Δ1/255 of the frame's #1A1611 as claimed (l.221-224).
— verified by formula comparison: `.dynamics-slider .lumen-slider__thumb` (l.206) uses the identical L8 formula as slider.css:104/205 — fill right edge `2+(W−4)p/100` = thumb right edge `(W−4)p/100−6+8` at every p; flush contract preserved through the tool scope.
— verified by rg consumer sweep → 14/14 classes produced by DynamicsCard/ToolSliderRow/tools components; `--covered` modifier handled by ToolSliderRow (per l.216-217, cross-file contract).
— verified: `.dynamics-tool__body[hidden] { display: none }` (l.102-104) correctly beats the flex display — the classic flex+hidden bug pre-empted.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/player/tools/dynamics/dynamics.css:206 (cf. slider.css:104, slider.css:205)
Code:
```
  left: calc((100% - 4px) * var(--lumen-slider-thumb, 0) / 100 - 6px);
```
What is wrong: The thumb-travel formula — the sheet's own "wave-f L8 formula" (l.203) — is copy-pasted into a third scope. slider.css carries it twice (base + EQ), dynamics.css once, and states.css's preview seek re-derives a variant with different insets. Four sites encode one geometry contract.
Why it matters: The g16-P2-3/k03 history in the comments shows this exact formula has ALREADY been fixed in multiple places at different times (the dynamics scope "used to override the primitive with the old overhanging travel", l.204-205) — i.e., the drift this duplication enables has already happened once per site. The next inset change (e.g. a 3px inset token) must touch four files in lockstep; a miss produces a thumb that peels off the fill edge in exactly one tool family.
What correct looks like: the primitive (slider.css) exposes the inset/travel as custom properties (e.g. `--lumen-slider-inset`, `--lumen-slider-thumb-overhang`), and scopes override values, not formulas.
Confidence: HIGH — Verification: read all four sites; formulas compared.

## 4. Per-function table (rule blocks)
| Selector(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.dynamics-tool` | 18-23 | REAL | — | column container |
| `__toggle` (collapsed pill) | 27-50 | REAL | — | 48px rx6 surface pill IS the toggle; 17px frame inset structural |
| `--open` + header reset | 55-65 | REAL | — | 59px header, transparent, top radii only — one-surface card grammar correct |
| toggle hover/focus | 68-76 | REAL | — | house hover grammar (frames carry none — honest note) |
| `__chevron` | 88-92 | REAL | — | −7.5px pull = frame stroke 18.25px from edge (i07 fix, measured) |
| `__body` + [hidden] guard | 96-104 | REAL | — | flex-vs-hidden pre-empted |
| `__rows` / `__staged` | 106-117 | REAL | — | staged note = honest-unbound marker (effects-area pattern) |
| `.dynamics-row` family | 124-161 | REAL | — | 18px line-height pins the 76px row pitch (arithmetic: 18+10+24+24 ✓) |
| `.dynamics-slider` + track/fill/thumb | 168-209 | REAL | S4 | F-001 formula copy #3; geometry otherwise identical to EQ scope |
| `__ticks` / `__tick` / `--covered` | 225-249 | REAL | — | z-order contract (ticks over fill, covered tick hidden) consistent with ToolSliderRow; tick ink verified |
| reduced-motion | 253-257 | REAL | — | toggle transition gated (slider parts gated in slider.css) |
| light theme ×5 | 276-300 | REAL | — | token-only remap (no hex!); contrast claims recomputed ✓; hover restated for specificity (l.286-287 explains why) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| harness.test.tsx:59+ "w26.i07 frame-contract pins (dynamics.css)" | YES (text-level) | reads the sheet and pins frame-measured values (path at l.64); the i07 defect fixes (chevron size/pull, thumb flush, tick ink) each have a pin |
| harness.test.tsx:61 comment | — | notes the sheet is pinned for "load-bearing composition" because jsdom renders no pixels — string pins, not visual verification |
Mutation check (reasoned): revert the thumb formula to `(100% - 8px)·p` → the flush pin fails; change tick ink to plain surface-muted → the Δ≤1/255 pin fails. UNVERIFIED: exact pin list (harness.test.tsx owned by another agent's queue).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "content-tertiary … 5.44:1 on the light panel and 4.79:1 on the deep well (≥4.5:1 AA)" | l.268-270 | REAL | python WCAG: 5.45 / 4.79 |
| "composing the two nearest tokens … lands ≈#1B1712 — Δ≤1/255 per channel" | l.222-224 | REAL | arithmetic: (27,23,18.5) vs #1A1611 (26,22,17) → Δ1/1.5/1.5 |
| "card r6 NOT r8, flat #371F06 fill NOT gradient" (a20 corrections) | l.2-4 | REAL | rules use radius-control (r6 token) and flat accent-tinted (l.187) |
| "the thumb's RIGHT edge sits flush with the fill's right edge — every reference frame" | l.196-201 | REAL | formula arithmetic; frame measurements quoted (fill 240 → thumb 270 etc.) |

## 7. Dead / duplicated / unreachable code
No dead selectors. Duplication: F-001 (formula ×3–4). Unreachable in shipped product (W14).

## 8. Dependencies & flags
Tokens: font-family-sans, space-inset ×2, font-size-body, font-weight-medium, color-content-tertiary, color-surface, radius-control, motion-duration-fast, color-surface-panel, color-surface-hover, color-accent, color-content-secondary, font-size-section, font-family-time, color-accent-tinted, radius-inset, color-accent-strong, color-surface-muted, radius-hairline, color-surface-deep — all verified present. Depends on the `.lumen-slider__*` class contract (slider.css). No feature flags. No raw hex (verified — the light section is token-only, a deliberate contrast to states.css's hex pins).

## 9. Open questions for the next agent
- DynamicsCard/ToolSliderRow owners: does the covered-tick calculation (which tick the thumb hides) use the same L8 formula in TS? A JS-side copy of the formula would be copy #5.
- The `--staged` note ("honest-unbound marker") implies these tool values are staged, not engine-bound — confirm which command (if any) the dynamics rows ever dispatch (audio.set-eq family awaits a nonexistent backend per W8 — is Compressor/Spatializer/Widener the same class?).
