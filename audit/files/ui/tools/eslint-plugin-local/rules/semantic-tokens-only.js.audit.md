# AUDIT: ui/tools/eslint-plugin-local/rules/semantic-tokens-only.js
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 128 / 1 rule object (7 helpers/handlers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as tooling (eslint.config.js registers `local/semantic-tokens-only`; orchestrator baseline: `npm run lint` passes with it active) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | node regex probe (below); npx eslint attempt BLOCKED (no node_modules in sandbox — UNVERIFIED runtime) |

## 1. One-paragraph verdict
A real, thoughtfully-constructed lint rule: string/template literals scanned
for hex/rgb; px rejected on font-size/radius keys; test/generated/i18n files
exempt with documented reasons; a `frame:`-citation escape hatch with a
nearest-ancestor walk that deliberately stops at Program (a file header can't
blanket-exempt a file — 64-68). I could not execute the rule itself (no
node_modules; npm install forbidden), so end-to-end behavior is UNVERIFIED
beyond the baseline's lint-clean run. The regex itself I DID execute: it has
a false-positive shape — any string containing a 3-8-digit hex-like token
("see issue #404") is flagged as a raw color.

## 2. What actually works (verified)
— node -e regex probe (exact rule regex, line 31):
  "#FFFFFF" -> true; "color: rgb(1,2,3)" -> true; "no hex here" -> false;
  "#12" -> false (too short); "run #43378741271" -> false (>8 digits).
— Baseline (audit/runs/baseline): `npm run lint` passes with this rule active
— the rule executes without crashing on the whole tree.

## 3. Findings
F-001 [S4]
Where: ui/tools/eslint-plugin-local/rules/semantic-tokens-only.js:31
Code: `const HEX_OR_RGB_RE = /#[0-9a-fA-F]{3,8}\b|rgba?\(/;`
What is wrong: Flags any string containing "#404"-style issue references,
hex-like serials, or CSS-id text as a raw color — verified by execution
("see issue #404" -> true). A user-facing string citing an issue would need a
frame: citation or eslint-disable to pass. False positive, low blast radius
(lint-time only).
Confidence: HIGH — Verification: ran node regex probe.
F-002 [S4]
Where: ui/tools/eslint-plugin-local/rules/semantic-tokens-only.js:105
Code: `      checkColorLiteral(node, node.quasis.map((q) => q.value.raw).join(""));`
What is wrong: Template literals are joined across expression holes —
`` `#${x}` `` joins to "#" (no match, fine), but `` `#ff${rest}` `` joins to
"#ff..." which can match hex while the real runtime value differs. Approximate
by design (documented "interpolated templates need flow awareness" gap,
README:107-108); noted, tracked.
Confidence: MEDIUM — Verification: read (join semantics).

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| exemption gate | 52-62 | REAL | - | tests/generated/i18n/css split documented |
| isFrameMarked | 69-81 | REAL | - | nearest-ancestor, stops at Program |
| checkColorLiteral / checkDimensionLiteral | 82-95 | REAL | - | F-001 regex shape |
| handlers (Literal/TemplateLiteral/Property/JSXAttribute) | 98-124 | REAL | - | F-002 join approximation |

## 5. Test honesty
Rule has no unit tests in this directory (promotion to eslint-plugin-lumen
with tests is NEXT-STEPS #2 per README:108-109). The rule's correctness rests
on the README's claimed "deliberate violations" probe (README:106-107) —
UNVERIFIED (cannot reproduce in sandbox).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v3 ... covers `.ts` files too — v2 skipped everything but .tsx" | rule header 13-15 | REAL | line 58 admits .ts and .tsx |
| "a stray hex with no citation still fails" | header 22-23 | REAL-UNPROVEN | regex verified; full rule path not executed |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
ESLint rule API only (context.report/sourceCode). Registered via
tools/eslint-plugin-local/index.js. No flags.

## 9. Open questions for the next agent
None.
