# AUDIT: ui/src/components/button/IconButton.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 58 / 1 (IconButton, forwardRef) + props interface |
| File verdict | REAL (primitive in wide shipped use; tested on every contract it documents; spinner promise verified in css) |
| Reachable from shipped entry point? | YES (UI tree): consumers include AppShell.tsx, ControlBar.tsx, PlaylistPanel.tsx, PlaylistOptionsMenu.tsx, NewPlaylistDialog.tsx, AddSubtitlesDialog.tsx, GotoTimeDialog.tsx, IconWrapper.tsx (rg). W14 ceiling (no GUI binary) applies. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "aria-busy\|spinner" ui/src/components/button/button.css` → :308-320 real pending rules (opacity token, pointer-events:none, ::before ring); `rg -ln "IconButton" ui/src` → 12+ consumer files; read IconButton.test.tsx assertions (7 tests: a11y name, classes, clicks, disabled, loading, snapshot, css contract); read of the whole file. |

## 1. One-paragraph verdict
A clean, well-tested primitive: mandatory `label` → aria-label (icon-only a11y), forwardRef + displayName, size modifiers, a loading contract that disables AND aria-busies the button, and a css spinner the comments promised (button.css:308-320 delivers the 14px currentcolor ring with layout-box preservation). The test suite pins every documented behavior including the css contract (transparent rest, 1px transparent border, control radius, 32px hit area) and executes in baseline #23. One latent footgun: `{...rest}` spreads AFTER the explicit attributes, so a caller passing `aria-label` or `aria-busy` in rest silently overrides the component's own a11y output — no current caller does, but nothing prevents it.

## 2. What actually works (verified)
— verified by reading button.css:308-320: `.lumen-icon-button[aria-busy="true"]` (opacity token + pointer-events:none + relative) and `::before` ring — the w26.k01 "promised hook is real now" comment is TRUE.
— verified by IconButton.test.tsx (baseline #23 green): accessible name from label (:28), size/class composition (:38), click fires (:49), disabled blocks handlers (:61-63), loading → aria-busy + disabled + no click (:74-77), DOM snapshot (:86), css contract assertions (:93-95). Each FAILS on the breakage it names.
— verified by rg: 12+ shipped consumers across appshell, player controls, playlist, dialogs — the primitive is not orphaned.

## 3. Findings
### F-001 [S4]
Where: ui/src/components/button/IconButton.tsx:40-51
Code:
```
      aria-label={label}
      aria-busy={loading || undefined}
```
(…:51)
```
      {...rest}
```
What is wrong: `{...rest}` is spread last, so any caller-supplied `aria-label` or `aria-busy` (both part of ButtonHTMLAttributes, neither destructured out) wins over the component's own values — an accidental `<IconButton label={t("play")} aria-label="Play" />` silently replaces the i18n label, and an `aria-busy` in rest defeats the loading contract's ARIA half.
Why it matters: The component's core promise is the a11y contract ("Required — callers pass the localized string"); the spread order makes that promise overridable by typo. `type`, `disabled`, `className` are destructured and safe; only the two aria attributes are exposed.
What correct looks like: destructure `aria-label: _ignored` / spread `{...rest}` BEFORE the explicit attributes (standard library practice), so explicit props always win.
Confidence: HIGH — Verification: read; JSX spread-last-wins semantics; rg of consumers shows none currently pass aria-label/aria-busy (latent, not live).
Severity: S4 — latent, zero current callers affected.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| IconButtonProps | 9-23 | REAL | — | label required; loading documented with the css hook reference (verified real) |
| IconButton | 32-56 | REAL | S4 | F-001 spread order; type defaults "button" (good); disabled||loading; size class only for non-default (sm/lg) — correct |
| displayName | 58 | REAL | — | forwardRef naming for devtools |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| exposes its label as the accessible name | YES | role query by name "Play" |
| applies size modifiers + icon-host + className | YES | toHaveClass with all four |
| renders the glyph and fires clicks | YES | onClick spy |
| disabled means disabled | YES | toBeDisabled + spy not called |
| loading means inert | YES | aria-busy attr + disabled + spy |
| DOM snapshot | YES (drift) | toMatchSnapshot — see snapshot-honesty audits for the pattern's limits |
| rest css contract | YES | asserts literal css declarations in button.css — a styling regression fails |
| gap | — | no test pins F-001's spread-order (an override test would) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "all are ≥ 28px" hit areas | :5-6 | REAL (css-asserted) | test asserts 32px default; sm/lg classes in button.css |
| "the promised hook is real now, see button.css" (spinner) | :19-21 | REAL | button.css:308-320 read |
| "Icon-only, so `label` is mandatory for ARIA" | :28 | PARTIAL | mandatory at the type level, overridable at runtime via rest (F-001) |
| "Carries `lumen-icon-host` so nested glyphs pick up state tints" | :29-30 | REAL | class emitted :44; icon.css protocol exists (icon.css consumer hit) |

## 7. Dead / duplicated / unreachable code
None. All props consumed; widely used.

## 8. Dependencies & flags
React only + button.css. No engine dependency. No feature flags.

## 9. Open questions for the next agent
- None.
