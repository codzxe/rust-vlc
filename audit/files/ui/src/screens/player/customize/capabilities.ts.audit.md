# AUDIT: ui/src/screens/player/customize/capabilities.ts
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 61 / 1 hook |
| File verdict | UNREACHABLE (real code, zero production call sites — kept for a frozen barrel API) |
| Reachable from shipped entry point? | NO: rg — only a comment citation (convertQueue.ts:153) and a barrel re-export (customize/index.ts:8); nothing calls it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg consumer search (exit 0); baseline ui-test.txt |

## 1. One-paragraph verdict
A correct capability probe (`system.list-commands` -> id set, honest-empty on failure, "absence of proof is never proof of support") that NOTHING calls anymore: the customize surface retired its gating (the box renders the full 31-tile catalog regardless of listings; engine truth governs dispatch only), and the hook survives solely because the customize barrel API is frozen against removals. The header admits this verbatim. Its own test keeps the dead code green. ConvertScreen's probe re-implements the same pattern inline rather than reusing this — consistent with the freeze but duplicated logic.

## 2. What actually works (verified)
— rg: zero call sites (comment-only + re-export).
— Logic itself is sound (cancellation-guarded, catch -> honest empty, single probe per transport).
— Baseline: capabilities hook test passes (ui-test.txt).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/customize/capabilities.ts:11-16
Code:
```
// The listing query below therefore has NO product consumer inside the
// customize surface anymore; it stays exported (frozen barrel API —
// removals are not additive) as the capability seam other designs cite
```
What is wrong: documented dead code shipped in the bundle (plus a test that pins it). The freeze rationale is stated, so this is hygiene debt, not deception — but it is the rare case where the file's own header is the dead-code evidence. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
N/A (<100 LOC): single hook reviewed (effect, cancellation, honest-empty catch).

## 5. Test honesty
The hook's test would fail if the hook broke — but it tests code no product path executes; the test's value is keeping a frozen API honest, not protecting behavior.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NO product consumer inside the customize surface anymore" | L11-12 | TRUE | rg zero call sites |

## 7. Dead / duplicated / unreachable code
The entire hook is unreachable-in-production (F-001); ConvertScreen's inline probe (ConvertScreen.tsx:122-142) is the live duplicate of this pattern.

## 8. Dependencies & flags
CommandTransport seam. No gates.

## 9. Open questions for the next agent
- When the API freeze lifts: delete, or re-point ConvertScreen's probe at it to deduplicate?
