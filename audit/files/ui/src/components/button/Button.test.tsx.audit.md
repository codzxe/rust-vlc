# AUDIT: ui/src/components/button/Button.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / tests | 204 / 20 |
| File verdict | REAL — behavior tests + a source-level CSS token contract (read via node:fs, the vitest-stubs-css workaround documented in-file) |
| Reachable from shipped entry point? | NO — vitest; baseline ui-test.txt:86 `✓ …Button.test.tsx (22 tests) 138ms` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | rg baseline; ls component dir (button.css, __snapshots__, states.md present) |

## 1. One-paragraph verdict
Honest primitive tests: native-button semantics, variant/size classes,
disabled/loading inertness (no handlers fire), ref forwarding, and a DOM
snapshot; then a second suite that reads the real button.css through node:fs
(because vitest stubs `.css` imports) and pins the semantic-token contract —
token variables in every state rule, 28/32/36px heights, focus ring, spinner
animation with prefers-reduced-motion, icon token remapping per state, and a
no-raw-colors lint over the CSS. The adaptation is documented at L7-16 with
the reason (no screenshot harness yet). No fake assertions.

## 2. What actually works (verified)
— Baseline ui-test.txt:86: 22 vitest-counted tests green in 138 ms.
— `ls ui/src/components/button/` → button.css + __snapshots__/ exist (the
node:fs read and snapshot target are real files).
— Mutation reasoning: disabled/loading tests fail if handlers leaked; the
token tests fail if a raw hex color or wrong token lands in any pinned rule;
the icon remap test (L185-197) fails if primary buttons render gray glyphs
(a real regression class it names).

## 3. Findings
F-001 [S4]
Where: ui/src/components/button/Button.test.tsx:21-24
Code:
const cssRule = (selector: string): string => {
  const match = buttonCss.match(new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}[^{]*\\{([^}]*)\\}`));
  return match?.[1] ?? "";
};
What is wrong: regex CSS parsing — returns the FIRST rule whose selector
string contains the pattern (with `[^{]*` spanning grouped selectors);
silently returns "" on no match, which makes each downstream toContain fail
rather than pinpoint the cause; cannot see @media blocks as separate rules.
Brittle-by-choice, honest in effect (failures are loud).
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: :192-194 `expect(buttonCss).toContain(".lumen-button--primary:hover,\n.lumen-button--primary:active,\n.lumen-button--primary:focus-visible")` — pins exact newline formatting of a grouped selector; a formatter pass breaks the assertion without a behavior change.
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: :18-19 `const { dirname } = import.meta as ImportMeta & { dirname?: string };` — relies on import.meta.dirname (Node ≥20.11); undefined on older node → readFileSync("undefined/button.css") crash with a confusing message. Environment assumption, currently satisfied (baseline green).
Confidence: HIGH — Verification: read.

## 4. Per-function table (describe groups)
| Group | Lines | Verdict | Notes |
|---|---|---|---|
| behavior suite | 26-113 | REAL | native semantics; variants; inert states; ref; DOM snapshot |
| CSS token contract | 115-204 | REAL | 11 rules pinned via cssRule + no-raw-colors lint |

## 5. Test honesty
All 20 fail on their named regression (§2). Snapshot (L111) is one golden DOM
shape — regeneration risk noted in-house (states.md records the adaptation).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "vitest stubs `.css` imports … read via node:fs" | file L9-15 | REAL | readFileSync at L19 + css suite green |
| "prefers-reduced-motion" motion-off rule exists | file L157 | REAL | toContain on real CSS text |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
node:fs static import (dev-only path, documented); no new deps; snapshot file
committed under __snapshots__/.

## 9. Open questions for the next agent
- None.
