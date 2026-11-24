# AUDIT: ui/scripts/gen-tokens.mjs
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 153 / 2 (main, renderTs) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (npm run tokens:generate / tokens:check — package.json:22-23; output src/tokens/generated/tokens.ts consumed by ThemeProvider + tests) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `node scripts/gen-tokens.mjs --check` -> exit 0, "OK (86 semantic tokens, 2 modes, 9 accents, sha256 26c55ab2b319…)" |

## 1. One-paragraph verdict
Real, zero-dependency codegen script — and it EXECUTES cleanly in this
sandbox: the drift check passes with 86 semantic tokens, 2 modes, 9 accents.
It parses the Rust-side themes/generated/tokens.css with regexes, extracts
:root semantic names plus data-theme/data-accent selectors, copies the CSS
verbatim, and emits typed TS accessors with a pinned sha256. The --check mode
is a genuine byte-comparison drift guard (not a timestamp or existence check).

## 2. What actually works (verified)
— `node scripts/gen-tokens.mjs --check` -> exit 0 with the summary line
(above). No drift between the committed ui copy and the crate output.
— renderTs emits tokenVar(), SEMANTIC_TOKEN_NAMES, TOKEN_MODES, ACCENTS,
TOKENS_CSS + sha256 — all consumed by src/tokens tests and theme code.

## 3. Findings
F-001 [S4]
Where: ui/scripts/gen-tokens.mjs:41-42
Code:
```
  const declRe = /--[A-Za-z0-9-]+(?=\s*:)/g;
  const blockRe = /(:root(?:\[[^\]]+\])*)\s*\{([^}]*)\}/g;
```
What is wrong: Regex CSS parsing — a `}` inside a declaration value or a
commented-out `--foo:` inside :root would corrupt the extraction. Today's
crate output is machine-generated and flat (check passes, 86 tokens), so the
risk is latent, not active. A comment containing "--x:" in the crate CSS
would silently add a phantom token name to the TS union.
Confidence: HIGH (limitation real) / impact LOW today.
Verification: read + successful --check run.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main | 37-104 | REAL | - | --check compares both outputs byte-wise; exit 1 on drift |
| renderTs | 106-148 | REAL | - | typed accessors + sha256 pin |

## 5. Test honesty
src/tokens/tokens.test.ts pins the sha256 + structure (per generated header,
lines 113-115) — would fail if the crate CSS changed without regeneration.
Not re-run (covered by baseline suite).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "idempotent, sorted output" | gen-tokens.mjs:13 | REAL | semantic.sort() at 62; --check OK proves committed state == regenerated state |
| "The script NEVER touches the Rust side" | gen-tokens.mjs:16-17 | REAL | only reads crateCssPath; writes confined to ui/src/tokens/generated |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
node:crypto/fs/promises/path/url only. --check flag. No npm deps.

## 9. Open questions for the next agent
None.
