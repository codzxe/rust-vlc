# AUDIT: ui/src/screens/player/useChromeAutoHide.ts
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 (CORRECTED same day — see §10) |
| Lines / functions | 198 / 1 (useChromeAutoHide) + 3 module constants |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — PlayerScreen.tsx:137 imports it (player composition); re-exported player/index.ts:8-9 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | sanitizer-proof fixed-string count `rg -c '[hidden, setHidden] = useState'` → 1; read; rg importers; state-machine walk |

## 1. One-paragraph verdict
Sound engineering throughout: a ref-held idle timer that restarts imperatively on activity (zero renders while visible — the w26.k06 fix for the per-pointermove re-render storm), a DOM-truth overlay lock (MutationObserver + one querySelector per mutation over `[role="menu"], [role="dialog"]`) so idle fades never strand focus in open overlays, stable return identity via useMemo, and windowed+fullscreen arming. The `[PROPOSAL]` 3000ms delay is honestly marked (no design pin).

## 2. What actually works (verified)
— verified by fixed-string count (immune to display mangling): L121 is `const [hidden, setHidden] = useState(false);` — intact, parseable.
— verified by read: lifecycle effect (arm/disarm/reveal-on-unlock), activity effect (reveal + re-arm), visibility derivation ("locked" > "hidden" > "visible"), no stale-hidden transitions (leaving armed or entering a lock force-reveals).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/useChromeAutoHide.ts:36
Code: `export const CHROME_AUTO_HIDE_DELAY_MS = 3000;`
What is wrong: the idle delay is a [PROPOSAL] constant with no design pin (self-admitted); shipping an arbitrary 3s. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| CHROME_AUTO_HIDE_DELAY_MS / OPEN_OVERLAY_SELECTOR / types | 36-76 | REAL | [PROPOSAL] marked |
| useChromeAutoHide | 89-198 | REAL | full machine verified by walk |

## 5. Test honesty
Player-lane tests exist (not in my list); the hook's semantics (jsdom-testable timer logic) are the kind that suite covers; UNVERIFIED content.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "an open overlay anywhere in the document ... locks the chrome visible" | L41-46 | REAL | MutationObserver effect L96-114 read |

## 7. Dead / duplicated / unreachable code
None (legacy standalone `enabled` semantics documented and used by isolated hosts/tests).

## 8. Dependencies & flags
React hooks, player context. No env vars.

## 9. Open questions for the next agent
- None.

## 10. CORRECTION RECORD (mandatory disclosure)
The first version of this audit reported an S0: "L121 `const idden, setHidden] = useState(false);` — SYNTAX ERROR, whole UI build broken". That was a DISPLAY ARTIFACT of the Bash tool's output sanitizer, which strips `[h` and `[m` two-char sequences from displayed text (proof: `printf 'a[h]'` displays as `a]`), mangling intact `[hidden` in tool output. Fixed-string count verification (`rg -c --fixed-strings '[hidden, setHidden] = useState'` → 1) proves the line is intact and the file parses. The S0 is RETRACTED. Full disclosure in audit/runs/W15-A08.log.md.
