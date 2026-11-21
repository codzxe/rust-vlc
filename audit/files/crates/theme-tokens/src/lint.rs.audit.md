# AUDIT: crates/theme-tokens/src/lint.rs
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 390 / 9 (lint_source, line_start_offsets, position, Boundary::{may_precede,may_follow}, occurrences, for_each_color + 5 tests + 2 fixture tests in tests/semantic_lint.rs) |
| File verdict | UNREACHABLE (real, correct, executed-by-tests-only lint with zero production/gate callers) |
| Reachable from shipped entry point? | NO — no crate depends on theme-tokens (Cargo.toml refs in crates/commands + settings-schema are comments only); the crate ships a `tokens-gen` binary used by `cargo xtask tokens` for CSS emission, but `lint_source` is called ONLY by its own unit tests (lint.rs:319,338,361,374,387) and tests/semantic_lint.rs:37,97. CI's CSS gate is stylelint (`npm run lint:css`, .github/workflows/ui.yml:106-111), which does not call this lint. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "lint_source" -g '!**/node_modules/**'` repo-wide → only lib.rs doc mention + lint.rs tests + tests/semantic_lint.rs (no production caller); `rg -l "theme-tokens" -g 'Cargo.toml' crates apps xtask` → only self + 2 comment mentions; `rg -o "stylelint-disable…" src -g '*.css' \| sort \| uniq -c` → 47 inline disables (44 color-no-hex) vs stylelint.config.js's "ZERO inline disables" claim; `rg -n -i "#2b2b2b\|#eaeaea\|#8f8f8f\|#1a1611" themes/tokens.json` → first three ARE token values, #1a1611 is not; baseline #3 test run for execution evidence; hand-traced `position`/`occurrences` algorithms. |

## 1. One-paragraph verdict
The algorithm is good: byte-safe 1-based line/column mapping (partition_point logic hand-traced correct), ASCII-lowercase hex matching that preserves offsets, boundary rules that reject needle extensions (pinned by test), dedup of raw values under first owner, and deterministic sorted output — all executed green in the baseline workspace test run (5 unit + 2 fixture tests, mutation-sensitive). But this is another built-and-abandoned enforcement seam: `lint_source` has NO caller in any shipped binary, xtask, or CI workflow — the CSS discipline that actually gates (stylelint via .github/workflows/ui.yml:111) is a different implementation, and the two disagree in scope: this lint only flags raw hex that duplicates a primitive/alias value (off-palette hex passes silently), while the stylelint gate's color-no-hex is bypassed by 44 inline disables across 15 UI files — despite stylelint.config.js asserting "Hand-written CSS carries ZERO inline disables". The crate-side lint is the Wave-8 `audio.set-eq` pattern at the tooling layer: correct code, tested, wired to nothing.

## 2. What actually works (verified)
— verified by hand-trace: `position()` (181-186) maps offsets to 1-based (line, column) correctly including offset-at-line-start and char-count (not byte-count) columns; `occurrences()` (220-250) honors boundaries and rejects `xprimitive.color.gray.800` / `--color-gray-8001` (test at 371-380).
— verified by baseline run #3 (test-workspace.log.txt, 1681 passed): the 5 unit tests + tests/semantic_lint.rs's 2 fixture tests (violating.css must fail, clean.css must pass, against the REAL committed themes/tokens.json via parse_committed_source) executed green.
— verified by `rg -n -i` against themes/tokens.json: the raw hexes used in the UI (e.g. #EAEAEA tokens.json:143, #8F8F8F :107, #2B2B2B :87) are token values, so this lint WOULD flag them if anyone ever ran it on ui/src CSS.

## 3. Findings
### F-001 [S3]
Where: crates/theme-tokens/src/lint.rs:4-6 (doc header) and 61 (pub fn lint_source)
Code:
```
//! `ui/` enforcement is the ESLint/Stylelint rule set (doc 08 §3.4, Phase
//! 4); [`lint_source`] is the crate-side checker used by this crate's
//! tests and available to CI.
```
What is wrong: "available to CI" — nothing in CI or any xtask calls `lint_source`. Repo-wide census: callers are lint.rs's own tests and tests/semantic_lint.rs only. The CSS gate CI actually runs is `npm run lint:css` (stylelint, .github/workflows/ui.yml:106-111). Two parallel token-discipline enforcements exist and neither is authoritative: this one runs in no gate; the stylelint one is escaped by 44 inline `color-no-hex` disables in 12 hand-written CSS files (variants.css ×5, states.css ×18, playlist.css ×6, form.css ×4, onboarding.css ×3+1, …) — while ui/stylelint.config.js's own header claims "Hand-written CSS carries ZERO inline disables — every raw color in src/**/*.css is a lint error".
Why it matters: A checker that runs only in its own tests cannot enforce anything (the exact lesson the CI comment at ui.yml:106-109 documents for the earlier local-only stylelint gate: "a CSS gate that CI never runs cannot gate"). The Rust lint would catch real violations (e.g. variants.css's #eaeaea duplicates primitive gray.100) that the stylelint lane is disabled against.
What correct looks like: either wire `lint_source` into an xtask gate (e.g. `cargo xtask tokens --lint-ui src/**/*.css`) or delete it and make the stylelint config honest about its disable policy.
Confidence: HIGH — Verification: full repo grep; workflow file read; disable census command (exact counts above).

### F-002 [S3]
Where: crates/theme-tokens/src/lint.rs:112-160 (raw-value pass) vs doc 08 §3.2
Code:
```
    // Raw color literals: map each distinct non-semantic color to the
    // semantic CSS names carrying the same value (for the suggestion).
```
What is wrong: The RawValue pass only flags a hex literal when its value EQUALS a primitive/alias token value. An off-palette raw hex — one that matches no token at all — produces zero violations, even though it violates "components reference semantic tokens only" (doc 08 §3.2) more severely than a duplicate does. Live example: `#1a1611` in ui/src/screens/settings/setting-field.css:260 exists nowhere in themes/tokens.json (verified: 0 matches) — this lint, run on that file, stays silent; stylelint's color-no-hex is inline-disabled at that exact line.
Why it matters: The lint's most important case (a color invented outside the system) is its blind spot by design; consumers relying on it get false confidence.
What correct looks like: flag ALL raw hex/color literals in component sources (optionally with a severity distinction for duplicates-vs-off-palette).
Confidence: HIGH — Verification: read; #1a1611 absence verified against themes/tokens.json.

### F-003 [S4]
Where: crates/theme-tokens/src/lint.rs:200-207 and 244-248
Code:
```
    fn may_precede(self, c: char) -> bool {
        match self {
            Boundary::Dotted => c.is_ascii_alphanumeric() || c == '_' || c == '.',
```
What is wrong: For dotted paths, '.' counts as an extending neighbor, so `tokens.primitive.color.gray.800` (any accessor-prefixed reference — the natural TSX shape) is NOT flagged, though it is a direct primitive reference. Additionally `occurrences` advances past a rejected match (`from = offset + needle.len()`, line 247), which can skip a valid overlapping occurrence for self-overlapping needles (theoretical for name-shaped needles; no live case found).
Why it matters: The rooted-path rule makes the main TSX consumption shape invisible to the lint.
What correct looks like: dotted-boundary may_precede should treat '.' as extending only for the FIRST path segment context, or the lint should also scan for unrooted `primitive.*`/`alias.*` property accesses.
Confidence: HIGH on the boundary semantics (read + test); the overlap skip is UNVERIFIED-in-practice (no failing input constructed).
Severity: S4.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ViolationKind / Violation | 26-56 | REAL | — | complete, documented data model |
| lint_source | 61-166 | REAL | S3 | F-001 (no caller), F-002 (off-palette hole); sort deterministic; css_form shadowing check correct (91) |
| line_start_offsets | 169-177 | REAL | — | byte offsets; O(n) |
| position | 181-186 | REAL | — | hand-traced correct incl. edge offsets; char-based columns |
| Boundary + may_precede/may_follow | 189-215 | REAL | S4 | F-003 '.'-precedes rule |
| occurrences | 220-250 | REAL | S4 | F-003 advance-past-rejected; ASCII-lowercase trick preserves offsets; empty-needle guarded |
| for_each_color | 253-268 | REAL | — | covers Single + PerMode values |
| tests (5 unit) | 271-389 | REAL | — | all mutation-sensitive; positions asserted exactly (327-328) |
| tests/semantic_lint.rs (2 fixture) | 1-… | REAL | — | runs against the REAL committed tokens.json; both fixtures pinned |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| dotted_paths_are_flagged_with_positions | YES | asserts exact line/column/token |
| primitive_css_names_are_flagged_but_shadowed_alias_forms_are_not | YES | the shadowing rule (semantic --color-surface not flagged) is the subtle case and is pinned |
| raw_hex_is_flagged_case_insensitively_with_a_semantic_suggestion | YES | asserts suggestion text contains var(--color-accent) |
| boundaries_reject_extensions_of_the_needle | YES | F-003's prefix/suffix cases |
| clean_semantic_usage_produces_no_violations | YES | false-positive guard |
| fixture tests (violating/clean) | YES | parse the committed themes/tokens.json, so token renames break them — real drift tripwire |
| baseline #3 | — | executed green |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "lint_source is the crate-side checker used by this crate's tests and available to CI" | lint.rs:4-6 | HALF-FALSE | used by tests: REAL; available to CI: nothing in CI calls it (F-001) |
| "raw color literals … whose value duplicates a primitive/alias color" | lint.rs:15-17 | REAL (disclosed) but the scope is the hole | F-002 |
| "an alias CSS form that coincides with a semantic name … is not flagged; primitive CSS forms never coincide" | lint.rs:11-14 | REAL | shadowing check at :91; primitive names (color.gray.*) never collide with semantic set — verified in generated tokens |
| doc 08 §3.2 "Components reference semantic tokens only" (the rule this lint serves) | doc 08 §3.2 via lint messages | FALSE in practice | 44 inline color-no-hex disables in ui CSS + off-palette hex (F-001/F-002 evidence) |

## 7. Dead / duplicated / unreachable code
`lint_source` itself is the dead export (F-001). Duplication: three token-discipline enforcers exist — this Rust lint, stylelint (color-no-hex/function-disallowed-list/color-named), and the TS-side "local/semantic-tokens-only ESLint rule" (tokens.ts header, stylelint.config.js header) — with three different scopes and none complete. The crate is not a dependency of any shipped binary (tokens-gen is an xtask-side generator).

## 8. Dependencies & flags
Depends only on crate::model (ModeValues, ResolvedValue, TokenSet) + std. No feature flags. Baseline #3 executed this file's tests in the workspace run (crates/theme-tokens is a workspace member).

## 9. Open questions for the next agent
- xtask lane: does `cargo xtask tokens` have any lint mode (grep tokens-gen main)? If it grows one, F-001 retires.
- ui/scripts/lint-css.mjs auditor (mine, file 19): confirm whether the custom runner strips/honors inline disables (stylelint honors them by default) — the config-header contradiction lands there.
