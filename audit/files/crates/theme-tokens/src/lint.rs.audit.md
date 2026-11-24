# AUDIT: crates/theme-tokens/src/lint.rs
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-23 |
| Lines / functions | 390 / 7 (+5 tests) |
| File verdict | REAL (test-time tooling; UNREACHABLE from shipped entry points) |
| Reachable from shipped entry point? | NO. `lint_source` callers: this file's tests (319,338,361,374,387) and tests/semantic_lint.rs:37,97 only. theme-tokens has zero dependents (the `theme-tokens` hits in crates/commands/Cargo.toml:29 and crates/settings-schema/Cargo.toml:19 are comments; `rg theme_tokens::` outside the crate: no hits). Not in the lumen-cli/desktop release-lane graphs. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | rg lint_source/theme_tokens::/theme-tokens (Cargo.tomls, .github, scripts, tools — no CI wiring); sed quote verification; baseline cargo-test-workspace.txt:6316-6330 (semantic_lint tests ran, exit 0); read ui/stylelint.config.js + ui/tools/eslint-plugin-local/rules/semantic-tokens-only.js |

## 1. One-paragraph verdict
A genuinely working, honestly documented, well-tested semantic-only lint:
dotted primitive/alias references, non-shadowed CSS-form names, and raw hex
duplicating a non-semantic color are all detected with correct 1-based
line/char-column positions, boundary logic that rejects path/name extensions,
and a shadow rule that exempts alias CSS forms coinciding with semantic names.
The module header is unusually honest — it says the REAL ui/ enforcement is
the ESLint/Stylelint set, and that set exists (ui/stylelint.config.js
`color-no-hex` + function-disallowed-list; ui/tools/eslint-plugin-local/
rules/semantic-tokens-only.js). Defects are hygiene-grade: the source is
fully cloned per needle, shorthand/functional color notations escape the
raw-value check (documented hex-only scope), and the whole checker ships
nowhere — it is exercised only by its own tests.

## 2. What actually works (verified)
— baseline audit/runs/baseline/cargo-test-workspace.txt:6316-6330:
`Running tests/semantic_lint.rs ... violating_fixture_fails_the_check ... ok`,
theme-tokens exit_code=0 (unit + fixture tests).
— Verified myself: ui/stylelint.config.js:52 `"color-no-hex": true` + line 54
`"/^rgb/"` disallow list — the "real enforcement" the doc comment points at
exists.
— Boundary rejection verified by reading test
`boundaries_reject_extensions_of_the_needle` (373: `"xprimitive.color.gray.800
primitive.color.gray.8000 --color-gray-8001"` must produce zero hits).

## 3. Findings
F-001 [S4]
Where: crates/theme-tokens/src/lint.rs:61 (whole pass)
What is wrong: unreachable in production. No binary depends on theme-tokens;
`lint_source` runs only under `cargo test`. The header's "available to CI"
(line 5) is technically true only because the crate's own tests run in
`cargo test --workspace`; no workflow or gate invokes the lint on real
component sources — it lints two fixtures, not the tree.
Confidence: HIGH — Verification: rg over .github/, scripts/, tools/: no
wiring; rg Cargo.tomls: comment-only references.
F-002 [S4]
Where: crates/theme-tokens/src/lint.rs:225-229
Code:
```
    let (hay, needle) = if matches!(boundary, Boundary::Hex) {
        (source.to_ascii_lowercase(), needle.to_ascii_lowercase())
    } else {
        (source.to_owned(), needle.to_owned())
    };
```
What is wrong: the non-hex arm clones the ENTIRE source and needle per call
— with ~209 non-semantic tokens × (dotted + css) forms plus raw-hex passes,
that is ~500+ full-source allocations and linear scans per linted file. Only
the Hex arm needs a folded copy. Correct: use `source`/`needle` by reference
in the non-hex path. Harmless at fixture scale; wasteful if ever pointed at
a real tree.
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/theme-tokens/src/lint.rs:15-17, 128
Code:
```
//! * **raw color literals** (`#8f8f8f`, case-insensitive) whose value
//!   duplicates a primitive/alias color — the message names the semantic
```
What is wrong: hex-literal-only scope. `rgb(211 121 24)`, `hsl(...)`, named
colors (`orange`), and 3/4-digit shorthand (`#FFF` for `#FFFFFF`) all escape
the raw-value check; only exact 6-digit-hex text matches. Documented, and the
ui/ side closes the hole via stylelint (`color-no-hex` catches shorthand
too), but as a standalone "given any source text (CSS, TSX, Rust, …)"
checker (line 6) the raw-value rule is bypassable by notation choice.
Confidence: HIGH — Verification: read + stylelint config read.
F-004 [S4]
Where: crates/theme-tokens/src/lint.rs:203-205
Code:
```
            Boundary::Css | Boundary::Hex => {
                c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'
            }
```
What is wrong: `may_precede` for Hex admits `-` and `.`, so `x-#D37918` or
`v.#D37918` in prose/comments is treated as an extension and silently not
flagged (a hex literal is never legitimately preceded by `-`/`.`). Misses
only, no false positives.
Confidence: MEDIUM — Verification: read; no test covers a preceding `-`.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| lint_source | 61-166 | REAL | S4 | shadow rule correct (90-91); raw_seen first-owner dedup deterministic |
| line_start_offsets | 169-177 | REAL | — | byte offsets, \n only |
| position | 181-186 | REAL | — | char columns as documented |
| Boundary::may_precede | 200-207 | REAL | S4 | F-004 |
| Boundary::may_follow | 209-214 | REAL | — | 8-digit hex correctly not matched |
| occurrences | 220-250 | REAL | S4 | F-002 clone-per-needle; ASCII fold offset-safe |
| for_each_color | 253-268 | REAL | — | skips non-color values; covers per-mode |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| dotted_paths_are_flagged_with_positions | YES | exact line/col/token/layer asserts |
| primitive_css_names_are_flagged_but_shadowed_alias_forms_are_not | YES | both directions asserted (343-352) |
| raw_hex_is_flagged_case_insensitively_with_a_semantic_suggestion | YES | kind, snippet case, token, message content |
| boundaries_reject_extensions_of_the_needle | YES | zero-violation assert on 3 extension traps |
| clean_semantic_usage_produces_no_violations | YES | empty assert on semantic-only CSS |
| (integration) violating_fixture_fails_the_check | YES | 5 violation-shape asserts + sort check against real themes/tokens.json |
| (integration) clean_fixture_passes_the_check | YES | empty assert |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The real `ui/` enforcement is the ESLint/Stylelint rule set (doc 08 §3.4, Phase 4)" | lint.rs:4-5 | REAL | ui/stylelint.config.js:52-63, ui/tools/eslint-plugin-local/rules/semantic-tokens-only.js exist |
| "used by this crate's tests and available to CI" | lint.rs:5 | REAL-UNPROVEN | tests run in workspace CI (baseline 6316-6330); no gate lints real sources |
| "An alias CSS form that coincides with a semantic name ... is not flagged" | lint.rs:12-14 | REAL | code 90-91 + test 333-354 |
| "raw color literals ... case-insensitive" | lint.rs:15-17 | SHALLOW | hex-6 only; rgb()/named/shorthand escape (F-003) |

## 7. Dead / duplicated / unreachable code
Nothing dead inside the file; the pass itself is production-unreachable (F-001).

## 8. Dependencies & flags
std only. No feature gates.

## 9. Open questions for the next agent
- Should any gate run `lint_source` over real component trees (e.g. tauri/web
  Rust sources that stylelint cannot see)? Currently nothing does.
- violating.css fixture audit (my next-but-one file) — confirm the fixture
  still matches the committed token values it cites (#9A9A9A gray.300).
