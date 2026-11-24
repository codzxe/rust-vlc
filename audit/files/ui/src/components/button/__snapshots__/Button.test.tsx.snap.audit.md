# AUDIT: ui/src/components/button/__snapshots__/Button.test.tsx.snap
| Field | Value |
|---|---|
| Auditor | W15-R2 (credited: A02) |
| Date | 2026-11-23 |
| Lines / functions | 10 / 1 snapshot export |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (test artifact exercising shipped code) — Button.tsx is the app-wide button primitive; the golden pins its class assembly |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg snapshot test in Button.test.tsx (exit 0); sed Button.test.tsx:100-112; grep baseline ui-test.txt:86 (22 tests passed); rg class logic Button.tsx:49-58 + button.css:92/193 |

## 1. One-paragraph verdict
Committed vitest golden for the single "DOM snapshot" test (Button.test.tsx:
105-112). The recorded DOM matches what the real component produces for
`variant="primary" size="sm"`: Button.tsx:57-58 assembles
`lumen-button--primary` (variant, not in BASE_VARIANTS) and `lumen-button--sm`
(size !== "md"), both classes have real CSS (button.css:92, :193), type
defaults to "button" (Button.tsx:49). The baseline run executed it and passed
(ui-test.txt:86 — Button.test.tsx 22 tests ✓, meaning the golden matched a
fresh render; a DOM change would have failed or marked it obsolete). One
hygiene finding: exactly one golden for one of the variant×size×loading×
disabled combinations — the pin is real but narrow.

## 2. What actually works (verified)
— verified by `sed -n '100,112p' src/components/button/Button.test.tsx` -> the
  test renders exactly the props the golden records.
— verified by `grep -n "Button.test.tsx" audit/runs/baseline/ui-test.txt` ->
  line 86: `✓ src/components/button/Button.test.tsx (22 tests) 138ms`.

## 3. Findings
F-001 [S4]
Where: ui/src/components/button/__snapshots__/Button.test.tsx.snap:3-10
Code: (verbatim quote)
```
exports[`Button > DOM snapshot 1`] = `
<button
  class="lumen-button lumen-button--primary lumen-button--sm"
```
What is wrong: One snapshot covers one combination (primary/sm). Variant
default, danger, ghost, link, loading, disabled, and size lg/md-vs-default
class assembly have no golden — though sibling non-snapshot tests may cover
them (Button.test.tsx has 22 tests; not all read this run). Why it matters:
snapshot coverage is narrower than the prop surface; a class-assembly
regression in an untested combo would slip past the golden. What correct looks
like: one snapshot per variant or a parameterized it.each. Confidence: HIGH —
Verification: read + rg.

## 4. Per-function table
Not applicable — single snapshot export, < 100 LOC.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| Button > DOM snapshot 1 | YES | Golden pins exact class list + type + children; changing class assembly for primary/sm fails it (proven executed in baseline, not obsolete) |

## 6. Doc claims touching this file
None.

## 7. Dead / duplicated / unreachable code
None. Golden is current (baseline passed; no obsolete-snapshot failure).

## 8. Dependencies & flags
Vitest snapshot v1 format. No feature gates.

## 9. Open questions for the next agent
Button.test.tsx's other 21 tests (computed-style-proxy assertions per the
header comment at :12) belong to Button.test.tsx's own audit.
