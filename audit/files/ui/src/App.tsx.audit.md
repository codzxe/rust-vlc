# AUDIT: ui/src/App.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 329 / 7 (App, GatedShell, PlayerRoute, DocumentTitle, playerVariantPath, routeTitleKey, useOnboardingBypass) + 2 const maps + 9 lazy chunks |
| File verdict | REAL — routing/gate/title plumbing is genuine; one shipped dev route (S3) |
| Reachable from shipped entry point? | YES — this IS the shipped root (main.tsx -> App) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg `import.meta.env.DEV` src/tokens/TokenSandbox.tsx (exit 0 — NO guard); Read App.tsx:1-329, TokenSandbox.tsx:1-45; rg `RequireOnboarding` OnboardingScreen.tsx (exit 0) |

## 1. One-paragraph verdict
The real route tree: onboarding-first-run gate (persisted `lumen.onboarding.v1`, whole-shell gate via GatedShell/RequireOnboarding), route-level code splitting with eager player, honest Demo/real badge split via `isDesktopShell()`, document-title sync, variant routing for the player. All of it runs against the real client seam and the real transport selection. One violation of the file's own stated rule: the `/dev/tokens` token-sandbox fixture is registered unconditionally and ships as a product route in production bundles (the neighboring onboarding bypass IS dev-gated; the tokens route is not, and TokenSandbox.tsx contains no `import.meta.env.DEV` guard — rg proves zero hits). PlayerRoute is also where the shipped app forgets to pass `onRetry` (charged to StageOverlays.tsx F-002, cross-ref).

## 2. What actually works (verified)
— verified by Read of the full file + rg of OnboardingScreen: the first-run gate is reactive to a persisted store; the player variants route for real (`playerVariantPath` L138-142, routes L303-305); `DEMO_SHELL = !isDesktopShell()` (L135) drives the honest Demo badge; DocumentTitle composes real msgids (L269-282).
— routeTitleKey covers every registered route (read L219-263 vs L291-324: all match; unknown -> 404 title).

## 3. Findings
F-001 [S3]
Where: ui/src/App.tsx:297 (and Read-verified TokenSandbox.tsx:29 — no DEV guard)
Code:
```
        <Route path="/onboarding" element={<OnboardingEntry />} />
        <Route path="/dev/tokens" element={<TokenSandbox />} />
```
What is wrong: the token sandbox is a card-4.A2 DEV fixture ("NOT a product screen", TokenSandbox.tsx:1-2) registered unconditionally — no `import.meta.env.DEV` gate on the route and no guard inside the component (rg `DEV` in TokenSandbox.tsx -> 0 hits). It ships in production bundles and is user-reachable at /dev/tokens, where it mutates the LIVE theme (mode/accent buttons write through useTheme). App.tsx itself states the rule at L150-154: "a test/screenshot affordance must not ship as product surface" — the onboarding bypass obeys it (L156-159), the tokens route does not. Why it matters: dev surface in the product + a stated design rule violated in the same file that states it. What correct looks like: gate the route on `import.meta.env.DEV` exactly like useOnboardingBypass. Confidence: HIGH — Verification: read + rg (exit 0).
F-002 [S4]
Where: ui/src/App.tsx:180-183
Code:
```
    <AppShell demo={DEMO_SHELL} provideMain={provideMain}>
      <Suspense fallback={null}>
        <Outlet />
```
What is wrong: `fallback={null}` on chunk load means a slow/failing lazy import paints nothing under the persistent chrome — no error state, no retry; a broken chunk in the desktop webview is a silent blank region. Desktop-disk loads are fast (comment L31-32), so blast radius is small. Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| lazy chunk consts | 33-70 | REAL | — | named->default mapping, player/onboarding eager (documented) |
| DEMO_SHELL | 135 | REAL | — | honest badge split via real transport probe |
| playerVariantPath | 138-142 | REAL | — | URL is variant truth |
| useOnboardingBypass | 156-159 | REAL | — | correctly DEV-gated (contrast F-001) |
| GatedShell | 173-186 | REAL | — | whole-shell first-run gate |
| PlayerRoute | 195-208 | REAL | — | deliberately no onOpenMedia (double-fire fix documented); missing onRetry -> StageOverlays F-002 |
| ROUTE_TITLES / SETTINGS_AREA_TITLES / routeTitleKey | 219-263 | REAL | — | complete coverage verified against the route list |
| DocumentTitle | 269-282 | REAL | — | locale-reactive |
| App | 284-328 | REAL | S3 | route tree; /dev/tokens ungated (F-001) |

## 5. Test honesty
App.test.tsx exists (root listing shows it); not in my assignment. Baseline vitest 1705 passed covers routing tests. Would they fail if /dev/tokens were dev-gated? Likely yes (tests run in DEV where the gate is transparent) — so the gap is invisible to the suite. UNVERIFIED beyond baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every route is a real screen; the placeholder routes are gone" (L72-74) | module header | REAL | route list L291-324 all resolve to real modules |
| "a test/screenshot affordance must not ship as product surface. ... production builds ignore the param" (L150-154) | comment | REAL for onboarding bypass / FALSE for /dev/tokens | TokenSandbox route ships unguarded (F-001) |
| "the placeholder routes are gone since the 2026-11-08 app-shell workstream" | L73-74 | REAL | no placeholder elements in the tree |

## 7. Dead / duplicated / unreachable code
None in-file. (The `/settings` -> `/settings/global` redirect is intentional.)

## 8. Dependencies & flags
react-router-dom, AppShell, player screen (eager), onboarding (eager, gate), 9 lazy screens, app-transport isDesktopShell, i18n. No feature flags; `import.meta.env.DEV` used exactly once (onboarding bypass).

## 9. Open questions for the next agent
- TokenSandbox owner: is there any plan to dev-gate /dev/tokens? (F-001 — one-line fix mirroring useOnboardingBypass.)
- AppShell.tsx owner: verify the Demo badge renders in the browser shell and NOT in the tauri shell as claimed (not my file).
