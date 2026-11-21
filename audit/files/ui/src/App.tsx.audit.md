# AUDIT: ui/src/App.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 328 / 10 (DEMO_SHELL, playerVariantPath, useOnboardingBypass, GatedShell, PlayerRoute, ROUTE_TITLES, SETTINGS_AREA_TITLES, routeTitleKey, DocumentTitle, App) |
| File verdict | REAL-UNPROVEN (root router is real, code-split, gated, titled, and test-covered; rendered only in dev/vitest because the shipped host cannot compile) |
| Reachable from shipped entry point? | NO at HEAD — App is mounted by main.tsx:63; the only shipped host `lumen-tauri` cannot compile (W14-A01 E0425; baseline #2 exit 101). Executed: App.test.tsx in baseline #23 (1705 passed), built in #22. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "app.document-title.template" src/i18n/messages.ts src/i18n/keys/w26.k19.ts` → registered at messages.ts:2934 (lane merged; template resolves); `rg -n "dev/tokens" src` → App.tsx:297 route + AppShell.tsx:2 comment + TokenSandbox.tsx:1 ("NOT a product screen") — no in-app link; `rg -n "loadPersistedVariant\|persistVariant" PlayerScreen.tsx` → :590 read (uncontrolled init only), :596 write (always); `rg -n "describe\|it\(" App.test.tsx` → 14 tests incl. gate/theme/accent hydration; `rg -n 'interface.theme' -A6 schemas/settings.toml` → enum now ["dark","light","auto"] (stale comment in settingsThemeSync.ts); sed quote verification. |

## 1. One-paragraph verdict
The root router is carefully built: route-level code splitting with a documented eager-player policy, a first-run onboarding gate wrapped around the WHOLE shell (tested route-by-route), one-main-landmark discipline, per-route document titles composed from existing msgids, a DEV-only onboarding bypass, and 14 tests covering gate, redirects, titles, theme/accent hydration. Two defects: the `/dev/tokens` dev fixture is mounted as an ungated production route (App.tsx:297) — its lazy chunk ships in every production build, contradicting the file's own principle that test/screenshot affordances "must not ship as product surface" (line 118-120); and the player-variant persistence is write-only in this app — PlayerRoute always controls `variant` from the URL, so PlayerScreen's `persistVariant` write (PlayerScreen.tsx:596) maintains a localStorage key no app route ever reads. PlayerRoute also passes no `onBinding`, confirming (cross-ref my PlayerScreen#part1 F-001) that forwarded uiActions (hide-ui, quit, record, play-bookmark) terminate here: nothing in App handles them.

## 2. What actually works (verified)
— verified by App.test.tsx (baseline #23, green): first-run gate redirects every shell route (:65), music route renders the real variant (:51), unknown paths 404 (:128), ?onboarding=1 per-navigation bypass (:147), persisted theme/accent hydrate on every route incl. full-bleed onboarding (:201-230).
— verified by `rg` i18n: the document-title template key is registered in the central catalog (messages.ts:2934), so DocumentTitle's lane-key fallback never fires — the seam graduated as designed.
— verified by reading main.tsx:63: `<App />` is the UI root; baseline #22 built the split chunks (386 ms build).

## 3. Findings
### F-001 [S3]
Where: ui/src/App.tsx:294-297 (+ TokenSandbox.tsx:1)
Code:
```
        {/* Full-bleed routes (no app chrome): the onboarding hero and the
            token dev fixture. */}
        <Route path="/onboarding" element={<OnboardingEntry />} />
        <Route path="/dev/tokens" element={<TokenSandbox />} />
```
What is wrong: The token dev fixture — whose own header says "card 4.A2 dev fixture (NOT a product screen)" — is mounted as an unconditional route with no `import.meta.env.DEV` gate (contrast the onboarding bypass at :158, which the file explicitly says "no longer ships in production bundles" because "a test/screenshot affordance must not ship as product surface"). Vite includes every lazy() import in the production build, so the TokenSandbox chunk ships in the product. No in-app link targets it (rg: only App.test.tsx and comments reference /dev/tokens), so in the desktop webview it is unreachable dead payload — but on any web deployment of this bundle the route is live.
Why it matters: The file's own stated principle is violated by its own route table; a dev fixture is product-reachable surface in web builds and dead weight in the desktop bundle.
What correct looks like: `{import.meta.env.DEV ? <Route path="/dev/tokens" …/> : null}` — the same gate the bypass uses.
Confidence: HIGH — Verification: read App.tsx + TokenSandbox.tsx:1-5; baseline #22 (vite build) includes lazy chunks by construction; no DEV gate present.

### F-002 [S4]
Where: ui/src/App.tsx:202-207 vs ui/src/screens/player/PlayerScreen.tsx:589-596
Code (PlayerScreen.tsx:593-597):
```
  const setVariant = useCallback(
    (next: PlayerVariant) => {
      setVariantState(next);
      persistVariant(next);
      onVariantChange?.(next);
```
What is wrong: In this app the variant is always route-controlled (`PlayerRoute` passes `variant` and routes on change), so the localStorage write `persistVariant(next)` fires on every toggle while the read (`loadPersistedVariant()`, :590) only seeds state for UNCONTROLLED mounts — which the app never produces. The persisted key is maintained but never consulted by any shipped route.
Why it matters: Write-only persistence is dead weight and a false signal — a reader of variant.ts believes the variant survives restarts; in the app it survives only as long as the URL does.
What correct looks like: either the route is the single source of truth (drop the persist call when controlled), or the app hydrates the initial route from the persisted variant.
Confidence: HIGH — Verification: read both files; loadPersistedVariant callers = PlayerScreen.tsx:590 only.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| lazy route imports | 33-70 | REAL | — | named→default mapping documented; player + onboarding eager by policy |
| flow-map comment | 72-127 | REAL | — | accurate: verified gate wraps all shell routes (:302,:318); provideMain split verified |
| DEMO_SHELL | 135 | REAL | — | module-level probe via app-transport (W14 lane); badge only in demo |
| playerVariantPath | 138-142 | REAL | — | URL = variant truth |
| useOnboardingBypass | 156-159 | REAL | — | DEV-gated (correct pattern F-001 should copy); per-navigation only |
| GatedShell | 173-186 | REAL | — | gate at layout level; Suspense inside shell chrome |
| PlayerRoute | 195-208 | REAL | S4 | F-002; no onBinding — forwarded uiActions terminate (cross-ref PlayerScreen#part1 F-001) |
| ROUTE_TITLES / SETTINGS_AREA_TITLES / routeTitleKey | 219-263 | REAL | — | all 13 settings areas present (matches areas/index registry); unknown → 404 title |
| DocumentTitle | 269-282 | REAL | — | template key registered (messages.ts:2934); onboarding bare-title case; locale switch re-titles (msg dep) |
| App (route tree) | 284-328 | REAL | S3 | F-001 /dev/tokens ungated; `*` inside gate; /settings redirect |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| App.test.tsx:46-64 gate/redirect tests | YES | assert wizard redirect per route |
| :65-110 whole-app gate + /dev/tokens lazy paint | YES | route-by-route gate assertions; awaits the lazy chunk |
| :112-121 settings framework + unknown-area redirect | YES | |
| :138-168 completed-profile + bypass + bypass-permanence | YES | asserts the bypass does NOT write the store |
| :171-240 theme/accent hydration | YES | persisted localStorage seam boot-straps provider state — real behavior, mutation-sensitive |
| baseline #23 | — | executed green |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the bypass … no longer ships in production bundles" | App.tsx:118-120 | REAL | import.meta.env.DEV guard at :158 |
| "/dev/tokens (dev fixture)" route list entry | App.tsx:110 | REAL but ships | F-001: no DEV gate on the route itself |
| "every open-media CTA dispatches the shared CustomEvent exactly ONCE" | App.tsx:95-97 | REAL | PlayerRoute passes no onOpenMedia; PlayerScreen affordances dispatch (PlayerScreen part1/2 verified) |
| "the player stays EAGER … 853 kB chunk split" | App.tsx:19-32 | REAL-UNPROVEN | chunk sizes not in baseline (build log #22 exists but sizes unverified here) |
| settingsThemeSync "schema enumeration is still dark/light only" (touching App hydration) | settingsThemeSync.ts comment | STALE | schemas/settings.toml:80 now ["dark","light","auto"] — engine-side embedded copy is the W11 lane's question |

## 7. Dead / duplicated / unreachable code
F-001 (dev fixture route in prod bundle), F-002 (write-only variant persistence). No other dead routes — every path maps to a real screen (verified against screens/ dirs). Whole file unreachable from shipped binaries (W14-A01).

## 8. Dependencies & flags
react-router-dom (Navigate/Outlet/Route/Routes/useLocation/useNavigate/useSearchParams), AppShell, PlayerScreen, lazy screens. No feature flags; the only build-mode flag is import.meta.env.DEV (bypass only — F-001).

## 9. Open questions for the next agent
- Whoever audits main.tsx (W15-A01 has it): confirm the Router wrapper (NavigationSeam's production branch safety, my PlayerScreen#part1 F-002).
- Web-deployment lane (if any): is the ui/ bundle ever actually served as a web app (playwright.web.config.ts suggests yes for tests) — F-001's exposure is real there.
