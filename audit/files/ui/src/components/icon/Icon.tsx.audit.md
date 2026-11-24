# AUDIT: ui/src/components/icon/Icon.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 82 / 1 (Icon) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the leaf of nearly every control surface (Button/IconButton/menus/ControlBar/settings rows) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; glyph-registry interface check (icons.tsx:826,896 — getIconGlyph/getTwoToneGlyph, 904-LOC mined registry, not in my list) |

## 1. One-paragraph verdict
A correct presentational leaf: module-level glyph lookup (no per-render element creation), currentColor token tinting, aria-hidden by default with title opting into role="img", stroke-width set BOTH as the numeric SVG attribute and via the CSS variant class (CSS wins — documented), two-tone face swap through the registry. Nothing fake.

## 2. What actually works (verified)
— verified by read: all prop branches (disabled, variant, twoTone, title) map to documented CSS classes in icon.css (cross-checked in icon.css audit); className composition is trimmed.

## 3. Findings
Zero findings. Checked: prop-to-class mapping, aria contract, stroke duplication (attribute + CSS, cascade documented in icon.css), glyph lookup totality (IconName is a closed union over the registry).

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| STROKE_WIDTHS | 32-36 | REAL | 1.5/2/3 per §1.4 |
| Icon | 44-82 | REAL | leaf render |

## 5. Test honesty
Icon tests exist in lane (referenced by w26.j04 review: "Icon is an ideal leaf: glyphs are module-level constants"); UNVERIFIED content.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Decorative by default (aria-hidden); pass title for a named icon" | header L15 | REAL | L69-72 |

## 7. Dead / duplicated / unreachable code
None (the "max" variant has no production producer but is documented as catalog-reserved in icon.css).

## 8. Dependencies & flags
icons.tsx registry (frame-mined geometry). No env vars.

## 9. Open questions for the next agent
- icons.tsx (904 LOC, another agent): verify the "verbatim design geometry" mining claim on a sample.
