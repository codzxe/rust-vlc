# AUDIT: ui/src/screens/player/layout.ts
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 441 / 18 |
| File verdict | REAL (parser + condition evaluator work and are tested) with an honestly-labeled PAPER-ONLY supply side: the config system it consumes does not exist, so production always runs baked-in TOML constants |
| Reachable from shipped entry point? | YES — PlayerScreen.tsx:630 `resolveVariantLayout(layoutSource, variant)` → variantLayout.ts:114 → `resolveLayout` (this file); PlayerScreen.tsx:877 `evaluateCondition(region.visibleWhen, …)` gates region rendering. App.tsx:203-207 mounts PlayerScreen WITHOUT `layoutSource`, so the only sources ever parsed in production are the module constants. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | ls config / ui/config (NO-CONFIG-DIR both); rg layoutSource (only tests supply it); rg resolveLayout/evaluateCondition callers; sed App.tsx:200-212; baseline ui-test.txt:135 `✓ src/screens/player/layout.test.ts (12 tests) 8ms` |

## 1. One-paragraph verdict
The TOML-subset parser, schema/anchor/token validation, and condition evaluator are real and honestly labeled: the header states the config system "has NOT landed yet" and the default ships as a parsed string constant — verified TRUE (no config/ dir at repo root or ui/). The parser itself is solid (typed refusals, loud errors, duplicate-region detection) and 12 tests pass in the orchestrator baseline. But the master-question answer for layouts: NOTHING supplies a layout from outside. `PlayerScreen.layoutSource` (PlayerScreen.tsx:524) is a dead production seam — the only callers passing it are tests; App.tsx passes just `variant`/`onVariantChange`. Which regions/menus exist is therefore hard-coded constants in every shipped build; "config owns existence and order" (doc 07 §7) is a proposal, not a behavior.

## 2. What actually works (verified)
— verified by baseline audit/runs/baseline/ui-test.txt:135 — `layout.test.ts (12 tests)` PASS, including parse-error cases (test file read: asserts default regions in order, custom layout sort, fallback-only-for-empty, error envelopes).
— verified by rg: `evaluateCondition` consumed at PlayerScreen.tsx:877 for real region visibility; `parseTomlSubset` reused by keymap.ts:17,32 (the parser has a second real consumer).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/layout.ts:415-418 (with PlayerScreen.tsx:524, App.tsx:203-207)
Code:
```
export function resolveLayout(source: string | null | undefined): LayoutConfig {
  if (source === null || source === undefined || source.trim() === "") return DEFAULT_LAYOUT;
  return parseLayoutConfig(source);
}
```
What is wrong: The custom-source branch is unreachable in production. rg over ui/src shows `layoutSource` supplied ONLY by PlayerScreen.test.tsx (lines 507, 532, 578); the sole production mount (App.tsx:203-207) passes no such prop, and no engine command, file read, or host callback delivers a layout string (no config/ dir exists — verified). The config-driven-layout claim of doc 07 §7 is consumption-side only; existence/order is effectively compile-time constant. The file's own header admits this ("[PROPOSAL]"), so this is a dead seam, not a fabricated feature. What correct looks like: wire layoutSource to a real config surface (engine config command or bundled file) or delete the prop from the public API until it exists. Confidence: HIGH — Verification: ran rg + ls.
F-002 [S4]
Where: ui/src/screens/player/layout.ts:126-128
Code:
```
    return s
      .slice(1, -1)
      .replace(/\\(.)/g, (_, c: string) => (c === "n" ? "\n" : c === "t" ? "\t" : c));
```
What is wrong: Unknown escapes pass through silently (`"\q"` → `q`, `\uXXXX` → `uXXXX`), contradicting the header's "Anything else is a parse error — no guessing" (L107). Real TOML rejects both. A future config file with a typo'd escape yields silently wrong labels instead of a diagnostic. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: ui/src/screens/player/layout.ts:112-120 (stripComment)
Code:
```
    if (ch === '"') inString = !inString;
    else if (ch === "#" && !inString) return line.slice(0, i);
```
What is wrong: Escaped quotes (`\"`) toggle `inString` because the backslash is not tracked; an odd number of inner escaped quotes makes a trailing `# comment` look in-string, the comment is kept, and parseScalar then fails with a spurious "unterminated string" instead of a useful diagnostic. Edge case (no shipped layout uses escapes). Confidence: MEDIUM — reasoning only, not executed (UNVERIFIED: no node_modules).
F-004 [S4]
Where: ui/src/screens/player/layout.ts:333-334 — menu ids are not deduplicated (regions are, L328-332). Inconsistent validation; a duplicate `[[menu]] id` silently produces two menus. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| LayoutParseError | 26-31 | REAL | — | prefixed message |
| stripComment | 112-120 | REAL | S4 | F-003 escaped-quote edge |
| parseScalar | 122-134 | REAL | S4 | F-002 lenient escapes |
| parseInlineTable | 136-169 | REAL | — | depth/string-aware comma split correct |
| parseValue | 171-174 | REAL | — | dispatch |
| parseTomlSubset | 177-205 | REAL | — | [[a.b]] appends to last [[a]] (TOML semantics); path depth ≤2 enforced |
| isTable / ensureArray / lastOf / pushTable | 207-233 | REAL | — | typed refusals on conflicts |
| requireString / requireLabelKey | 249-261 | REAL | — | label keys validated against i18n (isMessageKey) — real cross-check |
| parseRegion | 263-284 | REAL | — | anchor/token whitelists |
| parseMenuItem | 286-300 | REAL | — | |
| parseMenu | 302-311 | REAL | — | items sorted by order |
| byOrder | 313-315 | REAL | — | |
| parseLayoutConfig | 318-336 | REAL | S4 | F-004 no menu-id dedupe |
| DEFAULT_LAYOUT_TOML / DEFAULT_LAYOUT | 348-408 | REAL | — | the actual production layout (constant) |
| resolveLayout | 415-418 | REAL | S3 | F-001 dead custom branch in production |
| evaluateCondition | 421-441 | REAL | — | exhaustive switch; consumed by PlayerScreen |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| layout.test.ts (12, baseline PASS) | YES | asserts exact region order/ids, fallback identity, and (per test names/bodies read) error paths — mutation of sort/validate/fallback would flip assertions |
| variants.test.tsx:109-126 | YES | pins full-variant layout === DEFAULT_LAYOUT and condition tokens |
| PlayerScreen.test.tsx:507-532 | YES (for the seam) | proves layoutSource works when a host supplies it — but no production host does (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "That config system has NOT landed yet (no config/ directory at repo root)" | layout.ts:5-7 | TRUE | ls: NO-CONFIG-DIR, NO-UI-CONFIG-DIR |
| "consumption side is fully built here" | layout.ts:8-9 | TRUE | parser+evaluator real, tested, consumed (PlayerScreen:877, keymap.ts:32) |
| "Anything else is a parse error — no guessing" | layout.ts:106-107 | FALSE for escapes | F-002: unknown escapes silently pass |

## 7. Dead / duplicated / unreachable code
`resolveLayout`'s non-empty branch and `PlayerScreen.layoutSource` — reachable only from tests. Everything else live.

## 8. Dependencies & flags
i18n isMessageKey (real validation dependency). No feature flags.

## 9. Open questions for the next agent
- keymap.ts (same TOML subset): does its source come from anywhere real, or is it also a baked constant? Same dead-seam question.
