# AUDIT: ui/src/main.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 68 / 1 (bootstrap flow) |
| File verdict | REAL (code) — entry point verifies; the `global.language` setting is unwired to the locale seam (W11 pin confirmed at the root) |
| Reachable from shipped entry point? | This IS the ui entry point (index.html → main.tsx; vite build OK, baseline row 22). As product: unreachable — its only host (desktop-tauri) cannot compile at HEAD (W14 SUMMARY l.19); in a plain browser it runs on DemoTransport (honest, badged). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg `setActiveLocale` callers (main.tsx:51 + tests ONLY); rg `language|locale` in generated/schema.ts (global.language at :67-76, enumeration = ONE option "en-US"); rg `ErrorBoundary|componentDidCatch|getDerivedStateFromError` across ui/src → 0 hits; read app-transport.ts:1-60 (probe rule + honesty line); read the bootstrap order |

## 1. One-paragraph verdict
A clean, correctly-ordered bootstrap: bundled font CSS first, tokens → base sheet → focus floor before render, one host probe (selectAppTransport) deciding real-vs-demo transport with a single honest debug line, StrictMode, ThemeProvider → ClientProvider → Router → App. The two defects: the locale seam is query-param-only — the settings schema's `global.language` row (a language dropdown whose enumeration contains exactly ONE value, en-US) is never wired to `setActiveLocale`, so the shipped "Language" setting cannot change the language even if it had another to offer (W11's pin, verified at the entry point); and the app has no error boundary anywhere (repo-wide grep) — any component render throw white-screens the entire UI.

## 2. What actually works (verified)
— verified by reading l.1-37 → import order is load-bearing and correct: fonts.css (font-display:block holds first paint), tokens.css before lumen.css, focus floor before render; the boundary rule (imports stay inside ui/src) is stated and holds.
— verified by reading l.39-42 → the #root null check throws a named, actionable error (not a bare TypeError on null).
— verified by reading app-transport.ts:1-60 + l.56 → ONE probe decides the transport; browser fallback is DemoTransport with a visible badge and a one-line console.debug — the honesty rule is implemented as documented.
— verified by baseline row 22 (vite build OK, 386ms) and row 23 (vitest 1705/1) → this entry point compiles and its tree tests green.

## 3. Findings
### F-001 [S3]
Where: ui/src/main.tsx:49-52 (the only locale wiring) + screens/settings/generated/schema.ts:67-76 (the dead setting)
Code:
```
const requestedLocale = new URLSearchParams(window.location.search).get("locale");
if (requestedLocale !== null) {
  setActiveLocale(requestedLocale);
}
```
What is wrong: `setActiveLocale` has exactly ONE production caller — this query-param check (repo-wide rg; all other callers are tests). The settings schema ships a `global.language` row (schema.ts:67-76, "Interface language", cliFlag --global-language) whose enumeration contains a single option, "en-US" — and nothing connects that setting (or any user preference) to the locale seam. The UI language is therefore: en-US, overridable only by a URL query parameter. The comment's own honesty ("Any other tag is ignored (falls back to en-US, doc 07 §6.3)") describes the ceiling.
Why it matters: This is W11's "localization crate orphaned, UI hardcodes en-US" pin, confirmed at the entry point with the exact shape: a settings ROW that pretends to offer language choice (one option, unwired) above a locale system that only a developer's query string can move. A user opening Settings → General → Language sees a dropdown with one item that would do nothing if there were more.
What correct looks like: settings-driven locale (read global.language at bootstrap + on change → setActiveLocale), the enumeration grown to the catalogs that exist (the messages module supports the seam — tests switch locales through it), or the row removed until localization lands.
Confidence: HIGH — Verification: caller sweep; schema read; W11 cross-pin.
### F-002 [S4]
Where: ui/src/main.tsx:58-68
Code:
```
createRoot(rootElement).render(
  <StrictMode>
    <ThemeProvider>
```
What is wrong: No error boundary exists anywhere in ui/src (rg `ErrorBoundary|componentDidCatch|getDerivedStateFromError` → 0 hits). The root render is bare: any render-time throw in any screen unmounts the tree to a blank window (plus a console error invisible to users). For a media player GUI whose screens consume engine events of arbitrary shape, a defensive boundary with an honest error card (the app already has the Dialog/Card grammar for it) is standard practice.
Why it matters: One malformed event payload in a single component takes the whole shell down with no user-facing message — the "honest error surface" doctrine the UI applies everywhere else stops at the root.
What correct looks like: a top-level ErrorBoundary rendering the surface/error card grammar + a reload affordance.
Confidence: HIGH — Verification: repo-wide grep; read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| import order (fonts → tokens → lumen → focus/page) | 1-37 | REAL | — | load-bearing order documented and correct |
| #root guard | 39-42 | REAL | — | named error |
| locale query-param wiring | 44-52 | REAL | S3 | F-001 — the ONLY production setActiveLocale caller; `?locale=pseudo` debug hook documented |
| transport selection | 54-56 | REAL | — | one probe, one log line, honest origin/label |
| root render tree | 58-68 | REAL | S4 | F-002 — StrictMode/ThemeProvider/ClientProvider/BrowserRouter order correct (client available to router consumers, theme outside client) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| App.test.tsx / main-adjacent suites (other agents' files) | UNVERIFIED | main.tsx itself has no direct test — bootstrap is exercised transitively by every RTL suite that renders App (baseline 1705 tests). A broken import order or missing #root would fail ALL of them loudly — coarse but real coverage. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "ONE host probe decides the transport … DemoTransport simulation drives the UI … badged 'Demo' … announced once via console.debug — never silent" | l.11-19 | REAL | app-transport.ts read; badge + one-line log implemented |
| "This is the ONLY supported override point [?locale=]" | l.44-48 | REAL (and damning) | caller sweep: no other production locale path exists (F-001) |
| "Import stays inside ui/src (boundary-imports rule)" | l.26-27 | REAL | all imports are ./-relative |

## 7. Dead / duplicated / unreachable code
No dead code in-file. The `?locale` hook is a supported debug surface (visual harness uses it), not dead. Unreachable as product via the shell (W14).

## 8. Dependencies & flags
fonts.css, react-dom/client, react-router-dom, App, ClientProvider, app-transport, i18n messages (setActiveLocale), ThemeProvider, tokens.css, lumen.css, focus.css, page.css. No feature flags. No direct transport construction here (delegated — correct layering).

## 9. Open questions for the next agent
- ThemeProvider.tsx owner: does it read global.language or interface.theme from settings (settingsThemeSync exists)? If theme syncs from settings but locale does not, the asymmetry is F-001's smoking gun.
- i18n/messages.ts owner (W11 follow-up): how many catalogs exist behind the seam? The test at messages.test.ts:73 switches to "de-DE" expecting fallback — confirming no non-en catalog ships.
