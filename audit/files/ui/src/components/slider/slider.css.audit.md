# AUDIT: ui/src/components/slider/slider.css
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 413 / 28 rule blocks |
| File verdict | REAL (code) — UNREACHABLE from any shipped binary (W14) |
| Reachable from shipped entry point? | NO as product (sole host desktop-tauri cannot compile, W14 SUMMARY l.19). Dev/test: `Slider.tsx:3` and `Seekbar.tsx:5` import it; all four variant classes produced by Slider/Seekbar/Volume/EQSlider; baseline rows 22-23 green. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg token definitions (`--color-eq-fill-start/end`, `--gradient-volume-level` → 0 definitions, fallbacks live as documented); rg producers for `lumen-eq-slider|lumen-seekbar|lumen-volume` (4 components + tests); rg `data-dragging|data-pending|data-empty|aria-disabled` setters/ assertions; arithmetic verification of all four flush formulas (below) |

## 1. One-paragraph verdict
The best-audited sheet so far: every flush contract is arithmetically correct — base horizontal fill right edge `2+(W−4)p/100` equals thumb right edge `(W−4)p/100−6+8` (verified); vertical mirror fill top `H−2−(H−4)p/100` equals thumb top (verified); EQ scopes restate the same formulas consistently. The fallback tokens it references are honestly absent from the token sheet and documented as standing grammar, not temporary. Only two quality defects: the seekbar/volume circular thumb's center drifts up to ±4px from the fill's edge across the range (the exact misalignment class this sheet's own g16/j03 fixes eliminated for the pill sliders), and the geometry contract is duplicated by states.css's preview-seek restatement (cross-file drift risk, filed from the other side as states.css F-002).

## 2. What actually works (verified)
— verified by arithmetic: base fill right edge = 2+(W−4)p/100; thumb right edge = ((W−4)p/100−6)+8 = (W−4)p/100+2 → flush at every p ✓ (l.75-108).
— verified by arithmetic: vertical fill top = H−(2+(H−4)p/100); thumb top = H−((H−4)p/100−6)−8 = H−(H−4)p/100−2 → flush ✓ (l.132-156, restated at l.389-413 for EQ vertical).
— verified by rg → `--color-eq-fill-start`, `--color-eq-fill-end`, `--gradient-volume-level` have ZERO definitions in tokens.css — the `var(--x, fallback)` pattern renders the documented fallbacks; no silent dead reference.
— verified by rg → all state hooks (`data-dragging`, `data-pending`, `data-empty`, `aria-disabled`) are set by the components and asserted in Slider/Seekbar/Volume/EQSlider tests (e.g. Seekbar.test.tsx:83,110,141).
— baseline rows 22-23: compiles, tests green; Slider.test.tsx:230-233 pins the `[data-pending]` and `[data-empty]` rules as strings.

## 3. Findings
### F-001 [S4]
Where: ui/src/components/slider/slider.css:228-231, 246-255 (and 274-288 volume)
Code:
```
.lumen-seekbar .lumen-slider__fill {
  background: var(--color-accent);
  border-radius: 999px;
}
```
What is wrong: The seekbar fill inherits the base inset-2 anatomy (`left: 2px; width: (100%−4px)·p`, l.79-80) so its right edge sits at `2+(W−4)p/100`, while the seekbar thumb (l.250) travels the full width: `left: (W−12)p/100`, center at `6+(W−12)p/100`. The two agree only at p=50; at p=0 the thumb center is 4px right of the fill edge, at p=100 4px left. The volume slider repeats the same pair (l.274-288). The sheet's own history (l.66-74: "the g16-P2-3 Lead fix" for the pill sliders, "err = 2 − 4p/100") identifies exactly this misalignment class as a defect worth fixing — the circular thumbs never received the equivalent treatment.
Why it matters: The position indicator (thumb center) and the progress edge disagree by up to 4px — invisible at 12px thumb widths (offset stays under the thumb body), but it is a live geometry inconsistency in the one control whose whole job is position truth, and it contradicts the sheet's own flush doctrine.
What correct looks like: either thumb center = fill right edge (`left: calc((100% - 4px) * var(--lumen-slider-thumb, 0) / 100 - 4px)` for a 12px thumb) or a documented "circular thumb full-travel" contract with the fill using the same travel.
Confidence: HIGH — Verification: arithmetic on the quoted formulas.

### F-002 [S4]
Where: ui/src/components/slider/slider.css:45-108 (base geometry) vs ui/src/screens/player/states/states.css:694-718
Code:
```
.lumen-state-preview__seek .lumen-slider__fill {
  top: var(--space-track-inset);
  bottom: var(--space-track-inset);
  left: var(--space-track-inset);
  width: calc((100% - var(--space-track-inset) * 2) * var(--lumen-slider-fill, 0) / 100);
```
What is wrong: The slider's fill/thumb positioning contract exists in TWO independent implementations: this sheet (hard-coded `2px`/`4px`/`6px` insets and travels) and the preview card's restatement (token-driven `--space-track-inset` math). Both are live; a change to one's inset doctrine cannot propagate to the other.
Why it matters: Same drift class the file's own changelog celebrates killing (the g16/j03 fixes); the second copy will silently diverge on the next geometry change.
What correct looks like: one parameterized scope (`.lumen-slider--mini` here, or custom-property-driven insets) that states.css re-uses; filed from the states.css side too (its F-002).
Confidence: HIGH — Verification: read both; formulas compared.

## 4. Per-function table (rule blocks)
| Selector(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-slider` base | 28-38 | REAL | — | outline:none compensated by :focus-visible (l.293); touch-action none correct for pointer scrub |
| `__track` | 45-54 | REAL | — | 14px r7 well, surface |
| `__fill` (horizontal) | 75-88 | REAL | — | inset-2 + gradient fallbacks (tokens honestly absent); flush math verified |
| `__thumb` (horizontal) | 91-108 | REAL | — | −6px offset makes right edge flush; p=0 overhang documented |
| `--vertical` ×4 | 121-156 | REAL | — | vertical mirror math verified (bottom-anchored) |
| `.lumen-eq-slider` ×4 | 168-209 | REAL | — | 16px rx6/12px rx4 inset 2/8×24 rx4 per frames; flush math verified |
| `.lumen-seekbar` base+track | 216-226 | REAL | — | 4px pill on surface-deep |
| seekbar fill / buffer | 228-243 | REAL | S4 | F-001 fill; buffer honest engine overlay, no ARIA (spec-assigned) |
| seekbar thumb | 246-255 | REAL | S4 | F-001 travel |
| `.lumen-volume` ×4 | 263-288 | REAL | S4 | F-001 pair; gradient fallback documented |
| focus-visible | 293-296 | REAL | — | 2px accent ring |
| hover lifts ×3 | 302-313 | REAL | — | per-family well lift; specificity (0,3,0) beats base (0,2,0) ✓ |
| data-dragging ×2 | 316-322 | REAL | — | grabbing cursor + lit handle |
| aria-disabled | 325-328 | REAL | — | token opacity + inert |
| seekbar thumb reveal ×2 rules | 331-342 | REAL | — | hover/focus/drag/pending reveal + pending accent-strong |
| volume thumb reveal | 345-349 | REAL | — | same grammar |
| data-empty guard | 355-358 | REAL | — | ordered-last guard, consistent with aria-disabled inertness |
| reduced-motion | 360-367 | REAL | — | all four animated parts gated |
| EQ vertical ×4 | 373-413 | REAL | — | 24×8 handle; flush math verified (comment cites live-measured pre-fix drift) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| Slider.test.tsx:170-175 disabled inert | YES | behavioral (aria-disabled asserted) |
| Slider.test.tsx:230-233 pending/empty string pins | YES (text-level) | pins the exact rules incl. `.lumen-seekbar[data-empty]` |
| Seekbar.test.tsx:83/110/120/141 state attributes | YES | behavioral, includes pending→cleared transition |
| EQSlider.test.tsx:66-69 dragging truth | YES | asserts attribute REMOVAL on release (engine-truth discipline) |
| EQSlider pixel-math test (cited l.204, l.408) | YES | computes flush positions — the strongest guard in the set; would catch an inset regression in THIS sheet but not the states.css copy (F-002) |
Mutation check (reasoned): change `- 6px` to `- 8px` in the thumb formula → EQSlider pixel-math test fails. Change the seekbar thumb travel to the inset formula → F-001 becomes observable; no test distinguishes the two contracts today.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the thumb's RIGHT edge must sit flush with the fill's right edge" (g16 Lead fix) | l.97-99 | REAL | arithmetic verified for base/EQ horizontal and both verticals |
| "--gradient-volume-level … still absent as of w26.k03" | l.10-13 | REAL | rg: 0 definitions in tokens.css |
| "--color-eq-fill-* tokens win once the token lane lands them" | l.59-64 | REAL (absence) | rg: 0 definitions; fallbacks render |
| "hit area: 14px pill track + the 20px handle's overhang" (l.33) / "the hit area is sized to cover it" (l.102-103) | header/inline | PARTIAL | the p=0 thumb overhangs the `.lumen-slider` BOX by 6px (left edge −6px); the child remains pointer-hit-testable outside the parent box, so function holds, but the box is not literally "sized to cover it" |
| "honest buffer tint" (engine-reported) | l.233-236 | REAL-UNPROVEN | CSS renders `.lumen-seekbar__buffer` sub-regions; whether the engine ever reports ranges is the Seekbar/ControlBar owner's question (not in my list) |

## 7. Dead / duplicated / unreachable code
No dead selectors (all four families produced). Duplication: F-002 (geometry contract re-implemented in states.css). Unreachable in shipped product (W14).

## 8. Dependencies & flags
Tokens: color-surface, color-surface-deep, color-accent, color-accent-tinted, color-accent-strong, color-content-secondary, color-content-bright, color-surface-hover, radius-control, radius-inset, opacity-disabled — all verified present. Three documented absent-token fallbacks (verified absent). No feature flags.

## 9. Open questions for the next agent
- Seekbar.tsx owner: does the buffer tint ever receive engine ranges (`.lumen-seekbar__buffer` producers), or is it an always-empty overlay?
- Token lane: three fallback tokens re-logged since w26.k03 — if they land, re-run the EQSlider pixel-math tests (gradient stops change visual, not geometry).
