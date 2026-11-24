# AUDIT: ui/src/client/ClientProvider.tsx
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A08) |
| Date | 2026-11-24 |
| Lines / functions | 26 / 2 (ClientProvider, useClient) + 1 module const (ClientContext) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the spine: main.tsx:61 `<ClientProvider transport={appTransport.transport}>` wraps the entire app (index.html → main.tsx; web UI + desktop webview lanes per baseline rc-smoke web-ui-build/browser-load PASS) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg (mounts, useClient consumers, throw-path tests) -> 21 consumer files, 0 throw-path tests; sed -n '16,26p' -> quote verified |

## 1. One-paragraph verdict
Correct, minimal context provider doing exactly what it claims: one React context carrying the `CommandTransport`, hosts inject the real transport (production: main.tsx:61 via selectAppTransport — Tauri in the desktop shell, DemoTransport in a browser), tests inject fixtures. The "No Tauri/wasm imports" boundary claim is true (imports are react + one type only). Two hygiene nits: a `useMemo` that memoizes nothing, and an error path no test ever exercises.

## 2. What actually works (verified)
— verified by rg: 21 files call `useClient()` — the context is the real command spine (settings areas, dialogs, screens).
— verified by main.tsx:59-61 read: `selectAppTransport()` probed once at module scope → stable transport identity for the whole session, so the provider's context value never churns.
— verified by baseline (rc-smoke browser-load PASS, ui-test 1705 passed): every mounted test harness and the shipped UI run through this provider.

## 3. Findings
F-001 [S4]
Where: ui/src/client/ClientProvider.tsx:16
Code: (verbatim, sed-verified)
```
  const value = useMemo(() => transport, [transport]);
```
What is wrong: the memo is a no-op — `value === transport` for every render; memoizing a value on itself changes nothing (identity, referential stability, and re-render behavior are identical without it). Why it matters: dead pattern suggests stabilization that does not exist; if a host ever passed a re-created transport per render, this memo would NOT protect consumers (dep changes each render). What correct looks like: drop the useMemo, or document that hosts must hold transport identity (main.tsx does). Confidence: HIGH — Verification: read.
F-002 [S4]
Where: ui/src/client/ClientProvider.tsx:22-24
What is wrong: the fail-fast throw ("useClient requires a ClientProvider ancestor") has zero test coverage — rg over `*.test.*` finds no assertion of this error. Why it matters: the guard is the file's only branch, and it is untested; a regression to returning `null as CommandTransport` would surface as far-away TypeError noise instead. What correct looks like: one 3-line test (render useClient outside provider, expect throw). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ClientProvider | 9-18 | REAL | S4 | context provider; value identity = prop identity (F-001) |
| useClient | 20-26 | REAL | S4 | fail-fast on missing provider; throw path untested (F-002) |
| ClientContext (const) | 7 | REAL | — | `CommandTransport | null`, default null — the null enables the honest guard |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ~20 harness tests mounting ClientProvider (e.g. playlist-services.test.tsx:34, ConvertScreen.test.tsx) | YES for wiring | they prove provider+useClient round-trip with injected transports (fixture/mock) — a broken context read fails every one of them |
| throw path (F-002) | N/A — no test exists | untested branch |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Hosts inject the real transport; tests and the scaffold use MockTransport" | :4-6 | REAL | main.tsx:61 injects selectAppTransport() result; tests pass MockTransport/fixtures (rg across *.test.*) |
| "No Tauri/wasm imports are allowed here — hosts configure" | :6 | REAL | file imports: react + `type CommandTransport` only (read) |

## 7. Dead / duplicated / unreachable code
None. The useMemo (F-001) is dead weight, not dead code.

## 8. Dependencies & flags
react (createContext/useContext/useMemo), ./transport type only. No feature gates — the transport choice lives in app-transport.ts (host boundary respected).

## 9. Open questions for the next agent
None for this file. (app-transport.ts / tauri-transport.ts / demo-transport.ts have their own test files — covered elsewhere.)
