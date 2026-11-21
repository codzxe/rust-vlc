# AUDIT: ui/src/tokens/TokenSandbox.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 76 / 1 (TokenSandbox) + 2 module constants (MODE_SETTINGS, SWATCH_ROLES) |
| File verdict | REAL (dev fixture doing exactly what its header claims; the claim "NOT a product screen" is contradicted by the router — see F-001) |
| Reachable from shipped entry point? | YES — embarrassingly: App.tsx:297 `<Route path="/dev/tokens" element={<TokenSandbox />} />`, an ungated production route (lazy chunk :68-69); no in-app link points at it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "#[0-9a-fA-F]{3,8}|rgb\(" ui/src/tokens/TokenSandbox.css` → ZERO raw colors (semantic-only claim TRUE); `rg -n "data-role" TokenSandbox.css` → all 6 swatch roles wired to var(--color-…); `rg -n "surface-raised|content-primary" themes/generated/tokens.css` → both exist (:39,:62); read ThemeProvider.tsx:85-130 — `useLayoutEffect` sets `document.documentElement.dataset.theme/.accent` (the sandbox's switching genuinely drives the root cascade); read settingsThemeSync.ts (persistence seam is the SETTINGS store, not the provider — sandbox switches are session-only by design); App.test.tsx:96-108 read (mounts /dev/tokens under the real provider stack, awaits first paint); cross-ref my App.tsx audit F-001 (ungated prod route). |

## 1. One-paragraph verdict
A 76-line dev fixture that keeps its promises: mode/accent buttons call the real ThemeProvider setters, whose layout effect paints `data-theme`/`data-accent` onto `<html>` before first paint (ThemeProvider.tsx:99-103, read), six swatches each bind a distinct semantic token with zero raw colors in the css, and App.test.tsx mounts the lazy chunk under the production provider stack. The single defect is inherited from the router: the header swears "NOT a product screen" while App.tsx:297 ships it as an ungated production route — the S3 for that contradiction is already booked in my App.tsx audit; here it rates the stale-comment S4. Session-only switching (no persistence write) is by design and disclosed.

## 2. What actually works (verified)
— verified by reading ThemeProvider.tsx:99-112: `useLayoutEffect(() => { const root = document.documentElement; root.dataset.theme = resolvedMode; root.dataset.accent = accent; }, …)` — the sandbox's buttons drive the exact cascade the header claims; "auto" resolves against prefers-color-scheme (:93 resolvedMode line).
— verified by `rg` raw-color scan of TokenSandbox.css: zero hex/rgb literals; all 6 roles (accent, accent-bright, accent-strong, accent-tinted, surface-raised, content-primary) map to `var(--color-…)` selectors at css:73-93; both non-obvious tokens exist in the generated cascade (tokens.css:39,62).
— verified by App.test.tsx:96-108 (executed in baseline #23): renders App at /dev/tokens inside ThemeProvider+ClientProvider and waits for `.ts-sandbox` — the lazy chunk paints.
— setAccent cannot throw from this surface: the sandbox maps `ACCENTS` (the generated 9) directly, and ThemeProvider.setAccent only throws outside that set (ThemeProvider.tsx:114-118 read).

## 3. Findings
### F-001 [S4]
Where: ui/src/tokens/TokenSandbox.tsx:1-2 (+ App.tsx:297)
Code:
```
// TokenSandbox — card 4.A2 dev fixture (NOT a product screen; real screens
// and components land with wave 4.B+). Mounted at /dev/tokens, it proves the
```
What is wrong: "NOT a product screen" is false as shipped: App.tsx:297 mounts it as an ungated production route and App.tsx:68-69 lazy-imports it, so the fixture's chunk ships in every production build. The primary severity (S3 — dev surface shipping in prod) is booked in my App.tsx audit F-001; what remains here is the stale header that a reader would trust.
Why it matters: The comment is the only thing telling a maintainer this surface is dev-only; it is wrong about the one fact that matters (reachability).
What correct looks like: gate the route behind import.meta.env.DEV (then the comment becomes true), or change the comment to "ships ungated at /dev/tokens — see App.tsx".
Confidence: HIGH — Verification: App.tsx:68-69,297 read; my App.tsx audit F-001; App.test.tsx:95 comment confirms "ungated dev fixture… full-bleed route by design".

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MODE_SETTINGS | 18 | REAL | — | dark/light/auto — matches ThemeModeSetting; provider resolves auto |
| SWATCH_ROLES | 20-27 | REAL | — | 6 roles, all present in generated tokens (verified); 6 of 86 tokens swatched — header claims only what it shows |
| TokenSandbox | 29-76 | REAL | S4 | F-001 stale "NOT a product screen"; buttons use technical identifiers (union values) per the doc 07 §6.5 exemption, headings via useMsg — verified both |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| App.test.tsx /dev/tokens mount (baseline #23) | only for chunk/render breakage | asserts `.ts-sandbox` paints under the real provider stack — does not assert attribute switching (covered by ThemeProvider.test.tsx, which exists per file listing) |
| tokens.test.ts (adjacent) | YES for pipeline drift | pins TOKENS_CSS_SHA256 + structure (read: pin at :54) — see gen-tokens.mjs audit for the CI-gap caveat |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "NOT a product screen" | :1 | FALSE as shipped | App.tsx:297 ungated prod route (F-001) |
| "mode/accent switching drives the root-element attributes" | :3-4 | REAL | ThemeProvider.tsx:99-103 read |
| "every visual property below resolves through SEMANTIC tokens only" | :4-5 | REAL | zero raw colors in TokenSandbox.css (rg) |
| "Button labels are technical identifiers… not user copy" | :7-10 | REAL | buttons render union values via expressions; headings use msg() |

## 7. Dead / duplicated / unreachable code
None in-file. The file itself is the "dead" surface question — it has no in-app entry link (rg: only App.tsx route + tests + a SwatchGroup.tsx comment reference), reachable solely by typing the URL.

## 8. Dependencies & flags
useTheme/useMsg/ACCENTS + TokenSandbox.css. No flags. No transport/engine dependency. Switching is session-only: persistence of theme/accent lives in the settings store (settingsThemeSync.ts read — localStorage seam `lumen.settings.overrides.v1`), which this fixture deliberately does not touch.

## 9. Open questions for the next agent
- None for this file. The /dev/tokens gating decision is App.tsx's (booked there).
