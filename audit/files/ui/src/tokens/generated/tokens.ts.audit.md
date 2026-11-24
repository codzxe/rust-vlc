# AUDIT: ui/src/tokens/generated/tokens.ts
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 124 / 1 (tokenVar) + generated consts/types |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — tokens.css imported at main.tsx:25 for the cascade; tokenVar/SEMANTIC_TOKEN_NAMES consumed by ThemeProvider + components |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | sha256sum themes/generated/tokens.css; node string-compare of TOKENS_CSS vs tokens.css (EQUAL); rg tokens.test.ts drift cases |

## 1. One-paragraph verdict
Generated token module (gen-tokens.mjs from the Rust theme-tokens pipeline).
I verified the two integrity claims directly: the pinned
TOKENS_CSS_SHA256 (26c55ab2…) equals the actual sha256 of
themes/generated/tokens.css, and the embedded TOKENS_CSS string is
byte-identical to that file after unescaping. Drift is pinned by
tokens.test.ts (sha256 test, mode/accent matrix, typed-accessor mirror).

## 2. What actually works (verified)
- `sha256sum themes/generated/tokens.css` -> 26c55ab2b3193445728537df9cbb878bc77a46b7752714f1ff45752489648a20 == TOKENS_CSS_SHA256 (line 15).
- node comparison: TOKENS_CSS === tokens.css contents -> true.
- tokens.test.ts:72 "pins the generated tokens.css by sha256 — update deliberately", :129 "the typed accessors mirror the :root block exactly" — both integrity fences exist and would catch drift.

## 3. Findings
None. Checked: hash integrity (verified), embedded-string fidelity (verified),
type/list consistency (86 names in both union and array — test-pinned),
generation provenance chain (Rust crate -> CSS -> TS documented).

## 4. Per-function table (file >100 LOC; generated units)
| Export | Lines | Verdict | Notes |
|---|---|---|---|
| TOKENS_CSS_SHA256 | 15 | REAL | verified vs file |
| TOKENS_CSS | 20 | REAL | byte-identical to tokens.css (node check) |
| TokenMode/TOKEN_MODES, AccentId/ACCENTS | 22-28 | REAL | 2 modes, 9 accents — matches CSS blocks |
| SemanticTokenName + SEMANTIC_TOKEN_NAMES | 31-119 | REAL | 86 names; test pins mirror |
| tokenVar | 122-124 | REAL | trivial var() builder |

## 5. Test honesty
tokens.test.ts cases listed in §2 are structural pins (hash, matrix, mirror,
no value-named tokens, measured values). Would fail on any un-regenerated
edit. Honest.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Drift is caught by src/tokens/tokens.test.ts (pinned sha256 + structure)" | tokens.ts:6-8 | REAL | test file + hash verified |
| "same bytes as tokens.css" | tokens.ts:17-19 | REAL | node byte-compare: true |

## 7. Dead / duplicated / unreachable code
None (the TOKENS_CSS duplication of tokens.css is deliberate — test stubs
raw .css imports, documented line 17-19).

## 8. Dependencies & flags
None. Generated; lint rule enforces semantic-token-only consumption.

## 9. Open questions for the next agent
- None.
