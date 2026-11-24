# AUDIT: ui/src/components/button/Button.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 71 / 1 forwardRef component (+2 types, 1 const) |
| File verdict | REAL — clean primitive; loading state is genuinely used in production (5 mount sites) |
| Reachable from shipped entry point? | YES (ubiquitous: OpenMediaDialog.tsx:288, ControlBar, OnboardingScreen.tsx:692, NetworkStreamScreen.tsx:302, dialogs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg `loading=` production mounts (exit 0 — 5 sites, e.g. ControlBar.tsx:334); read full file |

## 1. One-paragraph verdict
Correct minimal primitive: variants/sizes as documented in states.md (cross-audited), `loading` renders aria-busy + disabled (fully inert — matches the doc contract "never fires click handlers"), forwardRef with displayName, type="button" default (no accidental form submits). The pending/loading state is not dead API: five production mounts wire it to real command-in-flight state (e.g. `loading={opening}` on the open-media submit, `loading={mapped.play.pending}` on the play button) — this is one of the honest async affordances in the app. One micro-nit: `{...rest}` spreads after `aria-busy`, so a caller could pass `aria-busy` through rest and desync the attribute from the inert state.

## 2. What actually works (verified)
— verified by rg: loading wired at OpenMediaDialog.tsx:288, OnboardingScreen.tsx:692, NetworkStreamScreen.tsx:302, ControlBar.tsx:334, MiniPlayer.tsx:228 (pending states tied to command dispatches).
— verified by read: class composition filters falsy; BASE_VARIANTS alias renders identical classes (states.md claim #cross-ref).

## 3. Findings
F-001 [S4]
Where: ui/src/components/button/Button.tsx:63-65
Code:
```
      aria-busy={loading || undefined}
      disabled={disabled || loading}
      {...rest}
```
What is wrong: `{...rest}` is spread last, so a caller supplying `aria-busy` via the HTML-attributes rest overrides the loading-derived value while `disabled` (destructured, not overridable) stays — attribute and behavior can desync for that caller. No current caller does this (rg). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ButtonVariant/ButtonSize (types) | 20-24 | REAL | — | matches states.md |
| ButtonProps | 26-36 | REAL | — | loading contract documented |
| BASE_VARIANTS | 39 | REAL | — | alias mechanism |
| Button | 47-68 | REAL | S4 | F-001 spread order |
| displayName | 70 | REAL | — | |

## 5. Test honesty
Button.test.tsx + snapshot exist (states.md claim; baseline vitest). CSS-source assertions cover states. UNVERIFIED beyond baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A loading button never fires click handlers — it is fully inert" (L32-33) | doc comment | REAL | disabled={disabled || loading} L64 |
| variants grounded in frames (L5-19) | header | REAL | cross-audited in states.md claim ledger |

## 7. Dead / duplicated / unreachable code
None. `secondary` alias is intentional (documented).

## 8. Dependencies & flags
react + button.css only. No feature flags.

## 9. Open questions for the next agent
- Move `{...rest}` before the derived props (or omit aria-busy from rest) — one-line hardening for F-001.
