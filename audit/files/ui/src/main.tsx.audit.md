# AUDIT: ui/src/main.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 68 / 0 (module side effects + root render) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — it IS the entry point (index.html → main.tsx) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read; cross-check selectAppTransport (app-transport.ts:40-51) + ClientProvider signature |

## 1. One-paragraph verdict
The composition root, and the answer to "is the UI wired to a real backend": ONE host probe (`selectAppTransport()`) decides — inside the Lumen desktop shell the UI runs on the REAL TauriTransport ("every command through the shell's single `lumen_dispatch` command, engine events on `lumen://event`"); in a plain browser it runs on the DemoTransport simulation, badged "Demo", with the choice announced via console.debug. Provider order: Theme → Client → Router → App. Locale override `?locale=` is the single supported hook (pseudo-locale for clipping proofs). Font CSS imported first with font-display:block for baseline parity.

## 2. What actually works (verified)
— verified by read + cross-check: `selectAppTransport()` (app-transport.ts:40+) probes `detectTauriBackend()`; ClientProvider takes the transport (ClientProvider.tsx props). The transport choice is real code, not a comment-only claim.
— UNVERIFIED at runtime: an actual Tauri shell run (desktop build unavailable in sandbox — environment limit per baseline).

## 3. Findings
F-001 [S4]
Where: ui/src/main.tsx:52-54
Code:
```
const requestedLocale = new URLSearchParams(window.location.search).get("locale");
if (requestedLocale !== null) {
  setActiveLocale(requestedLocale);
```
What is wrong: any tag is passed to setActiveLocale (the comment says unsupported tags "fall back to en-US" — that behavior lives in setActiveLocale, not here); the entry point itself does no allow-listing. Harmless if messages.ts enforces it (UNVERIFIED here). Confidence: LOW — Verification: read.

## 4. Per-function table
Module-scope: font-first import (L1-7), locale hook (L52-56), transport probe (L59), root render tree (L61-68). All REAL.

## 5. Test honesty
N/A (entry point; exercised by every e2e/visual harness run — those suites are other lanes').

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Inside the Lumen desktop shell ... the app runs on the REAL TauriTransport — every command through the shell's single `lumen_dispatch` command" | comment L12-16 | REAL-UNPROVEN | wiring verified in code (selectAppTransport → TauriTransport); engine side not executed by me |
| "The choice is announced once via console.debug — never silent" | comment L19-20 | REAL-UNPROVEN | app-transport.ts options.log default is console.debug (read in app-transport.ts:46-51 area) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
No env vars read (URL param only). No feature flags at this layer.

## 9. Open questions for the next agent
- See ClientProvider.tsx audit (this wave) for the bridge-depth analysis: which commands the UI actually sends vs the 65-command catalog.
