# AUDIT: ui/src/i18n/messages.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 77 / 6 tests across 2 describes |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test-infra for the shipped i18n runtime catalog; runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "messages.test" audit/runs/baseline/ui-test.txt` → `155: ✓ src/i18n/messages.test.ts (6 tests) 36ms`; full file read |

## 1. One-paragraph verdict
Honest runtime-side l10n tests complementing the zero-dep l10n:check script: exact catalog-size pin (1448, with the in-comment rationale that a `> 400` floor "could never fail meaningfully" — j18 F10), per-key non-empty-string loop with the key as failure label, isMessageKey accept/reject, pseudo-locale determinism + placeholder/escape preservation verbatim, a totality loop asserting EVERY key transforms under the pseudo locale with per-key failure labels, and unknown-locale fallback to the reference catalog.

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L155 `✓ ... (6 tests) 36ms` (inside 1705 passed).
- Exact-count pin L29 `expect(keys.length).toBe(1448)` with the wave-growth comment (w28.window +5).
- Per-key loops carry the key as the assertion message (L31-32, L66-68) — failures identify the offending key.

## 3. Findings
None. Zero-findings checklist: tautology scan — catalog iteration asserts real runtime values; the pseudo totality loop asserts transformation (not.toBe reference) per key. Mock-of-subject scan — none; real messages/pseudo modules. Skip scan — no it.skip; baseline 6 passed. Name-vs-body scan — verified. Mutation reasoning: a dead/empty catalog slot fails L30-33; a locale switch that no-ops fails L64-69; fallback breakage fails L72-76; placeholder corruption fails L52-58.

## 4. Per-function table (<100 LOC; still listed)
| Test group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| reference catalog | 23-40 | REAL | — | exact count, per-key non-empty, key guard |
| pseudo locale | 42-77 | REAL | — | determinism, placeholder preservation, per-key totality, fallback |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 6 | YES | exact pins + totality loops; verified green baseline L155 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The zero-dep CI gate is `npm run l10n:check` ... these vitest cases cover the runtime side the script cannot reach" | header L3-5 | REAL-UNPROVEN | scripts/l10n-check.mjs not my file; division of labor consistent with test content |
| "the count moves when the Lead merges lane keys (up) or prunes the dead-key grandfather list (down, ui/i18n-dead-keys.json)" | comment L26-28 | REAL-UNPROVEN | dead-keys list not my file |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
vitest; real messages + pseudo modules; no mocks.

## 9. Open questions for the next agent
- None.
