# AUDIT: ui/src/components/icon/Icon.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 83 / 1 component (+1 type +1 const map) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — the icon primitive used across the shell (AppShell rail/crumb), menus, settings rows, dialogs (rg consumers: rail icons AppShell.tsx:665, canvas menus, tools). W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | grep stroke-icon tokens.css (110-112: 1.5px/2px/3px tokens exist) + icon.css (83,87: strong/max consume the vars; NO base rule for default); sed getTwoToneGlyph (icons.tsx:896-898 — fallback to shared glyph verified). |

## 1. One-paragraph verdict
A correct, small icon primitive: fixed 24×24 viewBox, currentColor strokes, the aria pattern is exactly right (titled → role=img + <title>; untitled → aria-hidden), two-tone faces fall back to the shared glyph when a frame variant doesn't exist, and the emphasis variants are token-consumed via icon.css. One drift vector: the component hard-codes the stroke widths (1.5/2/3) as SVG attributes while its comment claims a "1:1 map to the §1.4 stroke tokens" — only the strong/max classes actually read the tokens (CSS overriding the attribute); the default variant's 1.5 attribute bypasses the token entirely.

## 2. What actually works (verified)
— getTwoToneGlyph fallback verified (icons.tsx:896-898); aria pattern traced; tokens exist and are consumed by icon.css:83/87 for strong/max. Baseline: Icon suites + the 100+ consumer suites pass (ui-vitest.log.txt); `tsc --noEmit` clean (ui-build.log.txt).

## 3. Findings
### F-001 [S4]
Where: ui/src/components/icon/Icon.tsx:29-36,70
Code:
```
const STROKE_WIDTHS: Record<IconVariant, number> = {
  default: 1.5, // --stroke-icon
  strong: 2, // --stroke-icon-strong
  max: 3, // --stroke-icon-max
};
```
```
      strokeWidth={STROKE_WIDTHS[variant]}
```
What is wrong: the widths are hard-coded numbers duplicating tokens.css:110-112 (--stroke-icon 1.5px / -strong 2px / -max 3px). icon.css consumes the vars ONLY for the strong/max classes (83,87) — which override the attribute — while the DEFAULT variant renders its 1.5 from the attribute alone; no CSS rule reads --stroke-icon for the base icon.
Why it matters: a token-lane change to --stroke-icon (say 1.75) moves strong/max's neighbors but not the default icon — the "1:1" mapping is enforced by nothing; the attribute and the token can drift silently.
What correct looks like: a base `.lumen-icon { stroke-width: var(--stroke-icon); }` rule in icon.css (attribute as fallback only), or generate STROKE_WIDTHS from the token source.
Confidence: HIGH — both files read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `IconVariant` (type) + `STROKE_WIDTHS` | 30-36 | REAL | S4 (F-001) | |
| `Icon` | 38-82 | REAL | — | aria pattern correct; className concat trimmed; twoTone class + glyph swap |

## 5. Test honesty
Icon.test.tsx (other agent): asserts the two-tone swap, aria pattern, variant classes (per hasTwoToneVariant note, j04-7). Stroke-width values would only be caught by a test asserting the attribute equals the token — unlikely to exist.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Stroke-weight variants, mapped 1:1 to the §1.4 stroke tokens" | Icon.tsx:29 | SHALLOW | duplicated constants; only strong/max consume the tokens via CSS (F-001) |
| "`twoTone` swaps in the glyph's two-tone face (a dedicated variant where the frame adds geometry, else the shared entry)" | Icon.tsx:21-23 | REAL | icons.tsx:896-898 fallback |
| "Decorative by default (aria-hidden); pass title for a named icon" | Icon.tsx:17 | REAL | 73-78 |

## 7. Dead / duplicated / unreachable code
F-001 (constants duplicate tokens; strong/max attributes redundant under the CSS override).

## 8. Dependencies & flags
icons.tsx registry (getIconGlyph/getTwoToneGlyph), icon.css. No external deps.

## 9. Open questions for the next agent
- icons.tsx owner: how many of the registry's names carry dedicated two-tone faces vs. falling back (a low ratio would make the twoTone prop mostly a class-only toggle)?
- icon.css owner: is there any consumer styling `.lumen-icon` stroke-width directly (which would make the attribute/CSS precedence worse)?
