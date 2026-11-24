# AUDIT: ui/src/screens/player/tools/equalizer/useEqMsg.ts
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 45 / 1 (useEqMsg + laneCatalog const + EqMessageKey type) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the equalizer's i18n seam; consumed by equalizer tool components (e.g. GraphicEQ.tsx, which references screen.eq.* keys) -> ToolsDialog -> player screen |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -ln "useEqMsg"`; dead-keys cross-check (screen.eq.* NOT grandfathered — used); screen.eq.preamp referenced in GraphicEQ.tsx |

## 1. One-paragraph verdict
A correct lane i18n seam: central catalog first (Lead-merged keys win), lane
pre-registration keys as fallback, raw key as the final honest fallback (never
`undefined`). The lane-merge contract is coherent — once keys land in
messages.ts both paths return the same string. Related cross-finding: the
equalizer's own keys (screen.eq.*) are live (used by GraphicEQ.tsx); only the
settings-effects eq copy (settings.effects.eq.low/mid/high) and
player.tools.open sit in the grandfathered dead list.

## 2. What actually works (verified)
— Code read: fallback chain `m(key) ?? laneCatalog[key] ?? key` (L42) is
null-safe (useMsg returns undefined only for non-MessageKey inputs, which the
lane catalog then serves); useCallback deps [m] correct. — Consumers verified
(equalizer components; the seam pattern is shared with 5 sibling modules).
— Key liveness cross-checked against ui/i18n-dead-keys.json: screen.eq.* not
grandfathered (live); 4 eq-adjacent keys are dead (see §7).

## 3. Findings
None. Checked and how: undefined rendering (impossible — final `?? key`);
catalog drift (lane keys are fallback-only by contract, and l10n-check's
definition-site exclusion covers the keys/ files); dead code (none — used
seam); crash paths (none — pure lookups).

## 4. Per-function table
N/A (<100 LOC — compact audit). useEqMsg (L35-45) REAL.

## 5. Test honesty
No dedicated test file; exercised via equalizer component suites (other
agents' scope). The `?? key` contract is asserted in sibling tests per the
w26.k10 comment (UNVERIFIED here).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every user-visible equalizer string flows through here — no literals" | useEqMsg.ts:6-7 | REAL-UNPROVEN | GraphicEQ references keys, not literals (spot-check); full census is the eq components' audits |
| "once the Lead merges them ... both paths return the same string" | useEqMsg.ts:4-6 | REAL | central-first ordering (L42) |

## 7. Dead / duplicated / unreachable code
None in-file. Adjacent dead keys in the grandfather list (not this file's
fault): settings.effects.eq.low/mid/high, player.tools.open.

## 8. Dependencies & flags
i18n/messages useMsg + lane key files w26.c1/w26.a18/w26.f05. No feature
gates; pure presentational seam.

## 9. Open questions for the next agent
- EqualizerTool/GraphicEQ (other agents): does the equalizer dispatch
  anything, or is it the same staged local-state pattern as the video/dynamics
  tools? (No equalizer command exists in the catalog — expected staged.)
