# AUDIT: ui/src/components/appshell/stripContext.ts (compact, 20 LOC — spec §13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 20 / 1 (useAppShellStrip) + 1 context |
| File verdict | REAL (minimal state-backed portal-host context; both sides wired) |
| Reachable from shipped entry point? | YES — producer: AppShell.tsx (provides the strip host element); consumer: PlayerScreen.tsx (portals its footer into the strip when inside the shell); tests on both sides (AppShell.test.tsx, variants.test.tsx) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -ln "useAppShellStrip\|AppShellStripContext" ui/src -g '!stripContext.ts'` -> AppShell.tsx, AppShell.test.tsx, PlayerScreen.tsx, variants.test.tsx; read of full file |

## 1. One-paragraph verdict
Twenty lines that do one thing correctly: a typed context carrying the
strip host element (HTMLDivElement | null), state-backed so consumers
re-render when the host lands, with a null default that makes the footer
render in place outside the shell — the documented zero-behavior-change
property for isolated hosts. Both ends of the seam exist (producer in
AppShell, consumer in PlayerScreen), and both have tests. This is the
control-band-spans-full-width wiring (w26.g03) — UI layout plumbing, no
backend pretensions, none needed.

## 2. What actually works (verified)
— Producer/consumer census (rg above): both production sides present.
— The context default (null) means no provider = in-place render — the
  graceful-degradation claim in the header is structural, not aspirational.

## 3. Findings
None. Checked: dead exports (none — both context and hook consumed),
naming accuracy (state-backed element per AppShell's provider
implementation — the provider side is AppShell.tsx's file to audit),
type safety (no casts).

## 4. Per-function table (compact)
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| AppShellStripContext + type | 14-16 | REAL | null default |
| useAppShellStrip | 18-20 | REAL | plain useContext |

## 5. Test honesty
AppShell.test.tsx and variants.test.tsx exercise both the in-shell portal
and the out-of-shell fallback (in the vitest baseline).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Outside the shell ... the context is null and the footer renders in place — zero behavior change" | :5-7 | REAL | null default + consumer census |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
react only. No flags.

## 9. Open questions for the next agent
- AppShell.tsx auditor: confirm the provider re-sets the context value when
  the strip host re-mounts (stale-element bug class for portals).
