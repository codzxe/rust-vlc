# AUDIT: ui/src/screens/network-stream/network-stream.css
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 295 / 26 selector rules |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web app build): `/open-network-stream` route (App.tsx ROUTE_TITLES) → NetworkStreamScreen.tsx mounts `<Dialog … className="network-stream-dialog">` and applies every class in this sheet (verified: all 21 `network-stream__*` classes appear in NetworkStreamScreen.tsx). Baseline #22 build OK, #23 vitest 1705 passed incl. NetworkStreamScreen.test.tsx. No GUI binary hosts it (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `Read` network-stream.css 1-295 (complete); `rg -o "network-stream__[a-z-]" NetworkStreamScreen.tsx | sort -u` (21 classes — all sheet selectors used, no dead rules); `for t in …` token loop (all 21 referenced tokens exist incl. --font-size-section, --font-weight-semibold, --color-record, --font-size-caption, --color-surface-deep); `rg -n "network-stream__urls|__name|__rule" *.tsx` (class application sites); `rg -n "localStorage|recent" NetworkStreamScreen.tsx` (the recents the sheet styles are a REAL persisted list — localStorage `lumen.streams.recent.v1`, newest-first); `sed -n` quote verification. |

## 1. One-paragraph verdict
A disciplined, frame-faithful dialog stylesheet with zero raw colors (every value is a token or a token `color-mix`), documented light-mode fixes for both near-invisible-surface hazards (the 50px head strip vanishing at Δ4/255 against the panel), a correctly reasoned divider geometry (the h19 shame #1 fix explains exactly why the old negative-margin bleed produced an asymmetric rule), and — notably — the recents it styles are a genuinely persisted localStorage list (newest-first, deduped), not the demo transport's reversed history. Only hygiene debt, both already flagged from the shortcuts side: the redundant `width`+`max-width` pair, and the flat-Cancel `color-mix` recipe duplicated verbatim with shortcuts.css (this file is the second site).

## 2. What actually works (verified)
— Verified by token loop: all 21 custom properties referenced exist in themes/generated/tokens.css (no dangling var()).
— Verified by `rg`: all 26 selector rules target classes used by NetworkStreamScreen.tsx (zero dead rules); the shared-control overrides (`lumen-field__control` at 146-162) apply to real elements (NetworkStreamScreen.tsx:207/278).
— Verified by read: light-well remaps (292-294) use `--color-surface-deep` with the same AA math (title ink 4.8:1 on #bababa) the shortcuts sheet ships — consistent cross-dialog treatment; the recents rows (229-253) carry hover + focus-visible rings and `overflow-wrap: anywhere` (long URLs wrap, never clip).
— Verified by baseline #23: NetworkStreamScreen.test.tsx runs green, including a structural assertion on `.network-stream__rule` (test:83).

## 3. Findings

### F-001 [S4]
Where: ui/src/screens/network-stream/network-stream.css:24-25
Code:
```css
  width: min(92vw, 1048px); /* frame: the 1048px dialog window */
  max-width: min(92vw, 1048px);
```
What is wrong: identical `width`/`max-width` pair — the max-width can never constrain anything. Same pattern as shortcuts.css:33-36 (that file's F-001).
Why it matters: duplication; a future single-sided edit creates a silently dead declaration.
What correct looks like: keep `width` only, or comment why the pair is deliberate.
Confidence: HIGH — Verification: read.

### F-002 [S4]
Where: ui/src/screens/network-stream/network-stream.css:279-282
Code:
```css
.network-stream__cancel.lumen-button {
  border-color: transparent;
  background: color-mix(in srgb, var(--color-surface-raised) 22%, var(--color-surface-panel));
}
```
What is wrong: the flat-Cancel recipe is byte-identical to shortcuts.css:421-424 — the comment itself says "the same flat lift the shortcuts window's Cancel ships". Two sheets maintain the same 22% mix magic number.
Why it matters: the retune history (30%→22% per the shortcuts comment) had to be applied in both; the next retune needs the same discipline with no mechanical help.
What correct looks like: a shared `.lumen-button--flat` modifier or a `--surface-cancel` token consumed by both dialogs.
Confidence: HIGH — Verification: read (both sites compared).

## 4. Per-function (selector) table
| Selector | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-dialog.network-stream-dialog` | 23-38 | REAL | F-001 | Frame window chrome override (95% panel fill, backdrop blur, σ7.5 shadow, no stroke). |
| `.network-stream` | 40-48 | REAL | — | Column anatomy; token-typed type. |
| `__head` / `__title` / `__close` | 52-82 | REAL | — | 50px strip; 15px semibold via --font-size-section (token-lane [PROPOSAL] documented); ellipsis. |
| `__form` / `__body` | 86-103 | REAL | — | Scrollable body; thin scrollbar tokens. |
| `__section` / `__label` / `__optional` / `__hint` / `__error` | 107-139 | REAL | — | Lead WCAG rulings documented for the two #6B6B6B debts; error ink --color-record. |
| `.lumen-field__control` overrides | 146-162 | REAL | — | Frame well skin: hover/focus/invalid border states; accent focus; record invalid. |
| `__urls` | 166-172 | REAL | — | 192px textarea well; resize none. |
| `__name` | 175-177 | REAL | — | Trivial. |
| `__rule` (+descendant bg) | 189-195 | REAL | — | The h19 asymmetric-rule fix — geometry reasoning documented and correct (body gutters place the rule at the frame's 20px insets). |
| `__recents*` / `__recent` (+hover/focus) | 200-253 | REAL | — | Persisted-history rows (localStorage, newest-first — the CORRECT order, unlike demo-transport recentOpenings); wrap-anywhere. |
| `__actions` / `.lumen-button` / `__cancel` | 261-282 | REAL | F-002 | Footer grammar; duplicated flat-Cancel recipe. |
| `:root[data-theme="light"] __head` | 292-294 | REAL | — | Light well remap with AA math documented. |

## 5. Test honesty (NetworkStreamScreen.test.tsx — behavior tests over the screen; ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| Structural assertions incl. `.network-stream__rule` presence (test:83) | YES | Pins the divider the h19 fix positioned. |
| Recents behavior (persisted list, dedupe, cap) per the localStorage seam | YES | The list the sheet styles is real persisted state (NetworkStreamScreen.tsx:53-98, newest-first at :135). |
| CSS visuals themselves | — | No visual regression in this suite (Playwright visual configs elsewhere — another agent's group). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (contract §4); raw px appears solely for frame-measured structural dimensions" (14-16) | this file | REAL | Zero raw hex/rgb in declarations; raw px only on geometry (all `frame:` commented). |
| "Recent streams — persisted live history (no frame counterpart)" (197) | this file | REAL | localStorage `lumen.streams.recent.v1` verified in the screen (read). |
| "the same flat lift the shortcuts window's Cancel ships" (277-278) | this file | REAL (and F-002's evidence) | Byte-identical rule in shortcuts.css:421-424. |

## 7. Dead / duplicated / unreachable code
- No dead selectors (all used).
- Duplicated: flat-Cancel recipe with shortcuts.css (F-002).
- Unreachable as a shipped binary (W14) — global UI condition.

## 8. Dependencies & flags
- Shared Dialog (`.lumen-dialog`), form field primitive (`.lumen-field__control`), Separator, button grammar; 21 semantic tokens (all exist). No flags.

## 9. Open questions for the next agent
- The two [PROPOSAL] token asks recorded here (15px heading size at 71-73; the light surface-vs-panel Δ4 retune at 290-291) belong to the token lane — same open asks as shortcuts.css.
- useNetworkStreamMsg.ts is in the w15-g004 group (my file #26) — its audit will confirm the lane-key seam this sheet's screen consumes.
