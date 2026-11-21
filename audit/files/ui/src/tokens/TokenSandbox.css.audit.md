# AUDIT: ui/src/tokens/TokenSandbox.css
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 96 / 9 selector blocks |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web UI): imported by TokenSandbox.tsx:16, mounted at App.tsx:297 `<Route path="/dev/tokens" element={<TokenSandbox/>}/>` (ungated, lazy chunk); App.test.tsx:96-109 mounts it through ThemeProvider+App and asserts `.ts-sandbox` renders (baseline vitest row 23). Ceiling: no GUI binary at HEAD (W14); web build bundles it (baseline row 22). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read of TokenSandbox.css + TokenSandbox.tsx + stylelint.config.js; rg census TokenSandbox/ts-sandbox/ts-controls/ts-swatch over ui/ (all hooks consumed); token-name cross-check against src/tokens/generated/tokens.ts TOKENS_CSS :root block (25/25 resolve); App.tsx route read; App.test.tsx:85-115 read. |

## 1. One-paragraph verdict
A 96-line proof-of-consumption stylesheet whose single claim — "every color, radius, font-size, space and shadow below is a var(--…) reference … No raw hex/rgb/px values" — is fully true: all 25 distinct token references resolve in the generated :root block, the only non-var values are unitless zeros in the margin shorthand and the display keywords, and the color discipline is additionally machine-enforced by stylelint's color-no-hex/function-disallowed-list/color-named rules (this file carries zero inline disables; only the generated tokens.css is exempt). Every one of the 9 selector blocks is consumed by TokenSandbox.tsx (hook census: .ts-sandbox, h1, .ts-controls, section+section, h2, button, button[aria-pressed=true], .ts-swatches, .ts-swatch + all six data-role variants map 1:1 to the SWATCH_ROLES array), so there is no dead CSS. The one defect belongs to the page it styles, not the CSS: the host component declares itself "card 4.A2 dev fixture (NOT a product screen)" yet the /dev/tokens route is unconditional in the shipped route table — the fixture, and this stylesheet with it, ships in the production web bundle reachable by any end user; "dev fixture" without a DEV-mode gate is a claim-vs-shipping gap.

## 2. What actually works (verified)
- Claim "No raw hex/rgb/px values": verified by full-file read — every declaration value is `var(--…)` or `0`/keyword; 25 distinct tokens, each present in the generated :root block of src/tokens/generated/tokens.ts (read + name-by-name cross-check: surface, content-primary/bright/secondary, surface-raised/panel, accent/-bright/-strong/-tinted, content-on-accent, stroke-border, stroke-icon (color), font-family-sans, font-size-body/title/caption, font-weight-medium, space-inset/grid, space-item-pitch, radius-card/control, shadow-card, stroke-icon (width)).
- All hooks consumed: TokenSandbox.tsx renders .ts-sandbox (:34), .ts-controls (:36), two sections (section+selector :32 fires), h1/h2, buttons with aria-pressed (:43,:56), .ts-swatches (:64), .ts-swatch + data-role for all six roles (:65-71 — CSS :73-95 covers exactly accent, accent-bright, accent-strong, accent-tinted, surface-raised, content-primary).
- Machine gate coverage: ui/stylelint.config.js:52-63 enforces color token discipline over all src/**/*.css with zero inline-disable policy (:15-23); `npm run lint:css` runs it (CI ui.yml:111). This file cannot regress to raw colors without the gate failing (execution itself: no baseline row — CI-wired only).
- Executed rendering: App.test.tsx mounts the real App at /dev/tokens with the production ThemeProvider stack and awaits `.ts-sandbox` (lazy chunk) — green in baseline row 23; the mode/accent button wiring (aria-pressed ↔ [aria-pressed=true] CSS) is exercised by the theme-hydration tests in the same file.
- BEM class pattern: .ts-sandbox/.ts-controls/.ts-swatches/.ts-swatch all pass the selector-class-pattern regex (stylelint.config.js:75-76).

## 3. Findings
### F-001 [S4]
Where: ui/src/tokens/TokenSandbox.css:1-6 (scope) + ui/src/tokens/TokenSandbox.tsx:1-2 + ui/src/App.tsx:297
Code:
```
/* TokenSandbox styles — card 4.A2 proof-of-consumption (doc 08 §3.2).
 *
 * SEMANTIC TOKENS ONLY: every color, radius, font-size, space and shadow
```
```
// TokenSandbox — card 4.A2 dev fixture (NOT a product screen; real screens
// and components land with wave 4.B+). Mounted at /dev/tokens, it proves the
```
What is wrong: The host component self-describes as a "dev fixture (NOT a product screen)", but App.tsx:297 registers `/dev/tokens` unconditionally — no `import.meta.env.DEV` gate, no feature flag. The lazy chunk (and this stylesheet inside it) ships in the production web bundle (baseline row 22 `npm run build` OK — vite bundles all route-reachable chunks), and any end user can navigate to it.
Why it matters: "Dev fixture" implies dev-only exposure; shipping it ungated contradicts the label and adds a non-product surface (mode/accent toggles that DO mutate the real theme context — harmless but user-reachable state mutation) to the production app. Zero functional damage; the page is honest about being a fixture once opened.
What correct looks like: Gate the route (`import.meta.env.DEV ? <Route …/> : null`) or relabel it a shipped diagnostics page.
Confidence: HIGH — Verification: App.tsx:297 read (unconditional), App.tsx:110 comment itself calls it "dev fixture" while listing it among ungated routes; TokenSandbox.tsx:1 read.
## 4. Per-function table (selector blocks)
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .ts-sandbox | 9-17 | REAL | — | 7 token declarations; min-height uses --space-item-pitch (48px item pitch as min-height — cross-purpose but token-pure) |
| .ts-sandbox h1 | 19-23 | REAL | — | title size + weight + bright ink |
| .ts-controls | 25-30 | REAL | — | raised surface + shadow-card |
| .ts-controls section + section | 32-34 | REAL | — | spacing between the two sections |
| .ts-controls h2 | 36-41 | REAL | — | caption size, secondary ink; margin shorthand zeroes (not "raw px") |
| .ts-controls button | 43-51 | REAL | — | border width uses --stroke-icon (icon-stroke token as border width — semantic stretch, token-pure) |
| .ts-controls button[aria-pressed="true"] | 53-57 | REAL | — | accent states; matches tsx aria-pressed wiring |
| .ts-swatches | 59-64 | REAL | — | flex wrap + token gap |
| .ts-swatch + 6 [data-role] variants | 66-95 | REAL | — | 1:1 with SWATCH_ROLES array; all consumed |
## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| App.test.tsx onboarding-gate suite (:96-109) | PARTIAL | Asserts the lazy chunk mounts (.ts-sandbox non-null) — catches route/import breakage, not visual token resolution (jsdom computes no layout; getComputedStyle cascade not asserted here) |
| tokens.test.ts rootDecl/lightDecl asserts (cross-ref, my file 9) | YES | Independently pins the VALUES of every token this file references — the resolution side is drift-gated by the byte-compare diff test |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every color, radius, font-size, space and shadow below is a var(--…) reference … No raw hex/rgb/px values" | :3-6 | REAL | Full-file read; 25/25 tokens resolve in generated :root; only 0s/keywords besides var() |
| "this file is the CSS-side demonstration of the same rule [local/semantic-tokens-only]" | :5-6 | REAL | Same discipline, different enforcement: stylelint color rules (config :52-63) cover the color half machine-side |
| "dev fixture (NOT a product screen)" (host tsx) | TokenSandbox.tsx:1-2 | FALSE-by-shipping | App.tsx:297 unconditional route — ships in the production bundle (F-001) |
| "Mounted at /dev/tokens, it proves the token pipeline end-to-end: mode/accent switching drives the root-element attributes" | TokenSandbox.tsx:2-5 | REAL | useTheme wiring + aria-pressed buttons; App.test.tsx mounts with real ThemeProvider |
## 7. Dead / duplicated / unreachable code
- None: all 9 blocks + all 6 data-role selectors consumed by TokenSandbox.tsx (census above). No duplication with other css files (ts- prefix is unique to the sandbox).
## 8. Dependencies & flags
Consumer: TokenSandbox.tsx (lazy chunk at /dev/tokens). Upstream: generated tokens.css cascade (25 tokens). Gates: stylelint (colors; lint:css — CI-wired, no baseline row), tokens:check (byte-compare, baseline row 24 PASS — the values this file references cannot drift silently). No feature flags. Note: stylelint enforces COLOR discipline only — raw px font-size/radius in .css files is not machine-restricted (the TSX rule covers that side); this file is token-pure by hand, not by dimension-gate.
## 9. Open questions for the next agent
- If App.tsx ever gates /dev/tokens behind DEV, the App.test.tsx mount (production-shaped render) needs a DEV-mock — the test currently proves the UNGATED behavior (i.e., it pins F-001's status quo).
- themes/generated/tokens.css consumers census (my file 9 addendum): this file consumes --space-item-pitch and --shadow-card — both had production consumers already; no change to the zero-consumer token list (micro/radius-app/stroke-window/motion-slow/motion-step/scrim-subtle).
