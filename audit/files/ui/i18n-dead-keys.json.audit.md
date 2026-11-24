# AUDIT: ui/i18n-dead-keys.json
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 630 / 125 entries + 1 $comment block |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (build-gate data file) — consumed by scripts/l10n-check.mjs:234-262 (`l10n:check`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 json parse (125 entries, all owner w26.k18); rg spot-checks of 3 keys |

## 1. One-paragraph verdict
A grandfather list of 125 dead i18n catalog keys, each with a reason and owner, enforced by scripts/l10n-check.mjs so new dead keys fail the gate and stale entries (key gone or re-referenced) also fail. Spot-checks confirm the entries are genuinely dead. The honest shame here is the size: 125 dead keys ≈ catalog rot acknowledged and parked rather than pruned ("messages.ts is Lead-owned — prune at the next catalog pass" on every entry).

## 2. What actually works (verified)
— verified by `python3` parse: 125 entries, every one has key+reason+owner.
— verified by `rg`: `screen.player.music`, `player.tools.open`, `customize.tile.remove-hint` all exist in src/i18n/messages.ts (L19/1500, L941/2421, L1210) and have ZERO references outside src/i18n/ — dead as claimed.

## 3. Findings
F-001 [S3]
Where: ui/i18n-dead-keys.json (all 125 entries)
Code:
```
    "reason": "scaffold route-title keys, superseded by the per-screen title wiring of the W26 screens; messages.ts is Lead-owned — prune at the next catalog pass",
```
What is wrong: 125 dead catalog keys (~9 route-title scaffold keys + control/tool strings) are grandfathered indefinitely; the "next catalog pass" is not scheduled anywhere in this file. Catalog bloat shipped to every locale load. Why it matters: dead weight in the message catalog and a standing incentive to keep grandfathering. What correct looks like: a pruned catalog + empty list. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
N/A (data file; <100 LOC rule would apply but entries enumerated programmatically).

## 5. Test honesty
N/A — the gate itself (l10n-check.mjs) fails on both new-dead and stale entries (mjs:242-262 read); the orchestrator baseline does not list l10n:check as failing (not in the wave-0 environment facts) — UNVERIFIED whether the gate currently passes; the file's self-consistency is verified.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A catalog key unreferenced by any ui/src|e2e|scripts|tools file is rot and FAILS l10n:check unless listed here" | $comment | REAL-UNPROVEN | consumer code read (mjs:234-262); gate not executed by me |

## 7. Dead / duplicated / unreachable code
The file IS the list of dead code — 125 keys, all confirmed-shape.

## 8. Dependencies & flags
None; plain JSON, no env vars.

## 9. Open questions for the next agent
- Run `npm run l10n:check` if cheap (I did not — npm budget discipline) to confirm 0 failures today.
