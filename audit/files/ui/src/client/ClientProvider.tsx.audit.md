# AUDIT: ui/src/client/ClientProvider.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 26 / 2 (ClientProvider, useClient) |
| File verdict | REAL — minimal, correct context seam; the memo is decorative |
| Reachable from shipped entry point? | YES in the app graph: main.tsx:61 mounts it with the app transport (TauriTransport in the desktop shell — which cannot compile at HEAD, W14; DemoTransport in the browser). Every `useClient()` consumer (audio device control, toolbar dispatch, config settings, frame channel, …) flows through this 26-line seam. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read (whole file); rg `useClient(` consumers (the seam's dependents, spot-checked in earlier audits: audio.tsx, toolbarItems.tsx, MediaSurface.tsx, SettingsScreen.tsx); rg imports in this file (react + ./transport only — the "no Tauri/wasm imports" rule holds) |

## 1. One-paragraph verdict
The entire UI↔engine boundary in 26 lines: a typed context, a provider that takes an injected transport, and a hook that throws a named error outside a provider. Correct, layering-clean (no host imports — hosts configure), and the null-guard error is actionable. The only blemish is a `useMemo` that memoizes a value into itself — it stabilizes nothing that the raw prop wouldn't, and mildly misleads readers into thinking context identity is being managed here.

## 2. What actually works (verified)
— verified by reading → the layering rule ("No Tauri/wasm imports are allowed here — hosts configure", l.5-6) holds: imports are react + ./transport only.
— verified by consumer sweep (via earlier file audits) → audio.tsx:122, toolbarItems.tsx:89, MediaSurface.tsx:113, SettingsScreen.tsx:457+ all consume useClient() — the seam carries every command in the app.
— verified by reading main.tsx:56-61 → the transport is created ONCE at module scope and injected; consumers' subscribe effects therefore run once per mount (the pattern the decorative memo pretends to provide).
— baseline rows 22-23: compiles, suite green.

## 3. Findings
### F-001 [S4]
Where: ui/src/client/ClientProvider.tsx:16
Code:
```
  const value = useMemo(() => transport, [transport]);
  return <ClientContext.Provider value={value}>{children}</ClientContext.Provider>;
```
What is wrong: The memo returns its own dependency — `value === transport` always, by construction. It provides no referential stability beyond what passing `transport` directly would (if the prop identity changes, the memoized value changes identically; if it doesn't, both are stable). It is a no-op with the appearance of context-value management.
Why it matters: Readers infer that transport identity is being handled here and skip the real discipline (create the transport once at the host — which main.tsx correctly does, but a future host might rely on this memo to "fix" a re-created transport and be wrong).
What correct looks like: `value={transport}` with a comment that hosts must create the transport once (or a genuine useMemo over a factory).
Confidence: HIGH — Verification: read; memo semantics are definitional.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ClientContext` | 7 | REAL | — | null default enables the guard |
| `ClientProvider` | 9-18 | REAL | S4 | F-001 decorative memo |
| `useClient` | 20-26 | REAL | — | throws a named, actionable error outside the provider — the correct guard pattern |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| Every consumer suite (audio.test, SettingsProvider.test, toolbar/CustomizePanels.test, frame-channel tests) | YES (indirectly) | they mount ClientProvider with mock transports and assert dispatched commands — the seam is the most-exercised 26 lines in the UI. No dedicated unit test file for the provider itself (its two behaviors — injection and the missing-provider throw — are covered incidentally; the throw path unverified by a direct test). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Hosts inject the real transport; tests and the scaffold use MockTransport" | l.4-6 | REAL | main.tsx injects selectAppTransport(); tests mount mocks (verified across suites) |
| "No Tauri/wasm imports are allowed here" | l.5-6 | REAL | imports read: react + ./transport |

## 7. Dead / duplicated / unreachable code
None (the memo is live-but-inert, not dead). Unreachable as product via the desktop shell (W14); live in the browser dev build on DemoTransport.

## 8. Dependencies & flags
react (createContext/useContext/useMemo), ./transport (type only). No feature flags. This file is the boundary-imports rule's cleanest example.

## 9. Open questions for the next agent
- transport.ts / tauri-transport.ts / demo-transport.ts owners: the `subscribe` contract's ordering guarantee ("channel order — the shell's EventSink preserves engine order") is asserted in comments — verify the EventSink actually orders (engine-side, apps/desktop).
- If a host ever creates the transport per-render, every `useEffect([client])` subscription in the app churns — the F-001 comment fix should document that contract explicitly.
