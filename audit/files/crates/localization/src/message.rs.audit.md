# AUDIT: crates/localization/src/message.rs (parts 1+2, L1-1070)
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-13 |
| Lines / functions | 1070 / 28 (incl. 8 in-file tests) |
| File verdict | UNREACHABLE (real, tested library; zero shipped consumers) |
| Reachable from shipped entry point? | NO — no crate/app depends on `localization` (rg over every Cargo.toml: only crates/localization/Cargo.toml matches); lumen-cli, desktop, desktop-tauri, web manifests all lack the edge; UI renders text via its own hand-written TS catalog `ui/src/i18n/messages.ts` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 5 S4: 2 |
| Commands run | `rg localization --glob **/Cargo.toml` -> only own manifest + a comment in playlists; `cargo test -p localization --lib message` -> 9 passed, 0 failed (exit 0); `ls locales/` -> en-US.toml only |

## 1. One-paragraph verdict
The ICU-subset parser/renderer is genuinely implemented and genuinely tested: parse errors are typed with byte offsets, rendering is panic-free by construction (no unwrap/index/expect in the render path — verified by reading every render function), and the 14-case malformed-input table test would fail under parser regressions. But the entire file is dead in the shipped product: nothing depends on the `localization` crate, the shipped UI keeps a *second*, hand-written TypeScript catalog (`ui/src/i18n/messages.ts`, 3018 lines) with its own `useMsg()` hook, and exactly one locale catalog (`locales/en-US.toml`, 97 lines) ships. The product does not localize through this code; the localization "consumer" question the wave asked has the answer NO. Additionally, the plural engine is documented-approximate for ru/pl/ar, one diagnostic variant is dead with a doc comment describing behavior that does not exist, and the recursive-descent parser has no depth limit.

## 2. What actually works (verified)
— `cargo test -p localization --lib message` -> `test result: ok. 9 passed; 0 failed` (all message tests incl. `rejects_malformed_messages` 14-case table, `rendering_never_panics_on_bad_args`, plural/select render round-trips). Panic-free render claim verified by reading L400-534: every failure path pushes a diagnostic + literal; the only `unwrap_or_default` (L456) is on `split('-').next()` which always yields ≥1 item.

## 3. Findings
F-001 [S3]
Where: crates/localization/src/message.rs:1-1070 (whole file)
Code: (verbatim, crates/localization/Cargo.toml:14,17)
```
name = "localization"
description = "Lumen localization: message keys, catalogs, fallback chain, catalog auditing (doc 07 §6)."
```
What is wrong: The crate is a workspace member but no Cargo.toml in the repo declares it as a dependency (verified: `rg localization --glob **/Cargo.toml` matches only its own manifest and a comment in crates/playlists/Cargo.toml:5). message.rs is therefore unreachable from `lumen`, `desktop`, `desktop-tauri`, and `web`. The UI answers the wave's hostile question: it does NOT localize through this crate — `ui/src/i18n/messages.ts:1-13` says "The real generated locale packs arrive with the i18n pipeline (doc 07); this hand-written table is the scaffold stand-in". Two parallel message catalogs now exist (Rust `locales/en-US.toml` vs TS `messages.ts`), a duplication/drift hazard, and `tools/xtask config_validator` validates `defaults.locale`/msgids against a catalog no shipped binary renders. Why it matters: every test here proves library correctness, not product behavior; "localization" as a shipped capability is a no-op (one locale, zero Rust consumers). What correct looks like: either wire the crate into the UI/host string path or mark the crate dev/CI-only in the registry. Confidence: HIGH — Verification: read + rg (no fabrication: dependency edge absent).
Severity note: would be S0 if any doc claimed shipped localized UI today; doc 07 §6.2/§6.4 carries `[VERIFY]`/`[PROPOSAL]` markers and lib.rs labels the engine choice pending, so this is honest-but-dead, i.e. S3 duplication/dead code, file verdict UNREACHABLE.

F-002 [S3]
Where: crates/localization/src/message.rs:234-236 vs 517-530
Code: (verbatim, 234-236)
```
    /// The selected plural category has no arm in this message; the
    /// `other` arm was used instead (a non-conformant catalog at runtime —
    /// audit rule 5 should have caught it in CI).
```
Code: (verbatim, 517-531)
```
            let chosen = arms
                .iter()
                .find(|(arm, _)| *arm == category)
                .or_else(|| arms.iter().find(|(arm, _)| *arm == PluralCategory::Other));
            match chosen {
                Some((_, sub)) => sub.render_into(out, diagnostics, locale, args, Some(arg)),
                None => {
                    // Unreachable: parse enforces the `other` arm.
                    diagnostics.push(RenderDiagnostic::PluralCategoryMissing {
```
What is wrong: `PluralCategoryMissing` is never emitted on any reachable path: when the category arm is missing the `.or_else` silently picks `other` with NO diagnostic, and the branch that does push the diagnostic requires `other` to be absent, which `parse_arms` (L853-857 `MissingOtherArm`) forbids. The doc comment describes a behavior (fallback + diagnostic) that the code does not implement. Why it matters: the diagnostic exists precisely to surface non-conformant catalogs at runtime; it is dead, and the doc lies about it. Mutation check: removing the `.or_else` fallback breaks no test (no test covers category-missing-at-runtime). What correct looks like: emit `PluralCategoryMissing` when `find(category)` fails and the fallback fires, matching the doc. Confidence: HIGH — Verification: read + mutation reasoning (UNVERIFIED by execution — no such test exists to run).

F-003 [S3]
Where: crates/localization/src/message.rs:476-480
Code: (verbatim)
```
        PlaceholderKind::Select(arms) => {
            let Some(Arg::Str(value)) = find_arg(args, name) else {
                report_missing(out, diagnostics);
                return;
            };
```
What is wrong: a select placeholder given an `Arg::Int`/`Arg::Float` (argument PRESENT, wrong type) reports `ArgMissing` ("missing argument {name:?}") and renders the literal. The crate's own `ArgTypeMismatch` variant exists for exactly this and is used for percent/plural but not select. Why it matters: catalog/QA tooling keyed on diagnostics misdiagnoses a type bug as a missing key; the `rendering_never_panics_on_bad_args` test only exercises Str-into-percent, so the mislabel is untested. What correct looks like: pattern-match `find_arg` into `Some(Arg::Str(v))` / `Some(_)` (mismatch) / `None` (missing). Confidence: HIGH — Verification: read.
F-004 [S3]
Where: crates/localization/src/message.rs:601-636, 833-837 (also render_into L410-432, collect_conflicts L365-395)
Code: (verbatim, 833-837)
```
            // Sub-message: recursion; `#` only inside plural arms.
            let sub = self.parse_message(match grammar {
                ArmGrammar::Option => ParseContext::SelectArm,
                ArmGrammar::Category => ParseContext::PluralArm,
            })?;
```
What is wrong: recursion depth is unbounded — `parse_message` -> `parse_placeholder` -> `parse_arms` -> `parse_message` with no depth counter, and `render`/`collect_*` recurse over the same tree. A catalog string nesting `{a, select, other {` ~100k deep (trivially written in a TOML multi-line value) overflows the stack; stack overflow aborts the process, it is not a catchable `Err`, so the crate's "errors are typed" posture does not cover it. Blast radius today is limited because catalogs are repo files and the crate has no shipped consumer (F-001) — that is what keeps this S3, not S1. What correct looks like: a depth cap (ICU4X uses one) returning `MessageError`. Confidence: HIGH on the mechanics — Verification: read; UNVERIFIED by execution (would need a hostile catalog + debug build; not run — build-budget discipline).
F-005 [S3]
Where: crates/localization/src/message.rs:499-507 (backed by plural.rs:110-116)
Code: (verbatim, plural.rs:110-116)
```
pub fn select_plural_category(language: &str, value: i64) -> PluralCategory {
    match categories_for_language(language) {
        Some(set) if !set.contains(&PluralCategory::One) => PluralCategory::Other,
        _ if value == 1 => PluralCategory::One,
        _ => PluralCategory::Other,
    }
}
```
What is wrong: for ru/pl/ar (categories seeded at plural.rs:77-93) every count except 1 renders the `other` arm — CLDR says Russian 2/3/4 → `few`, 21 → `one`, etc. A ru catalog that correctly provides `few`/`many` arms (which audit rule 5 *requires* for ru) will never render them at runtime. plural.rs:102-109 admits this ("it is *not* the full CLDR algorithm for `ru`/`pl`/`ar` ... `[VERIFY]`"), but message.rs's own module doc (L11 `{name, plural, one {…} other {…}} count choice`) presents plural rendering with no caveat. Latent today (only en-US ships), wrong the moment a ru/pl/ar catalog lands. What correct looks like: implement the CLDR rules for seeded languages or refuse to render pluralized copy in them (the plural.rs doc itself says callers "must not ship pluralized copy in those languages"). Confidence: HIGH — Verification: read + `cargo test -p localization --lib message` passes precisely because no ru-render test exists.
F-006 [S4]
Where: crates/localization/src/message.rs:573-577
Code: (verbatim)
```
    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ') | Some('\t')) {
            self.pos += 1;
        }
    }
```
What is wrong: only space/tab are whitespace. ICU MessageFormat ignores all whitespace; here a newline inside a placeholder header (natural in TOML `"""` multi-line values wrapping long select arms) yields `ExpectedCommaOrClose { found: '\n' }`. Newlines inside arm *bodies* are fine (they land in `Part::Text`), so the failure mode is asymmetric and surprising. What correct looks like: include `\n`/`\r` (and probably the ICU whitespace set) in skip_ws. Confidence: HIGH — Verification: read (trace: L656-673 peek after skip_ws sees '\n' -> error arm).
F-007 [S4]
Where: crates/localization/src/message.rs:88-101, 205-219
Code: (verbatim, 89-90)
```
    let rounded = (value * 100.0 * 100.0).round() / 100.0;
    let mut text = format!("{rounded:.2}");
```
What is wrong: two cosmetic defects. (a) `format_percent(-0.0)` renders "-0%" (`"{:.2}"` of -0.0 is "-0.00", trailing-zero trim leaves "-0"). (b) `MessageError::offset()` returns hardcoded `0` for `ConflictingPlaceholderType` (L217), so the one error catalog tooling cannot byte-locate is the cross-catalog contract violation. Neither breaks the happy path. Confidence: MEDIUM — Verification: read + manual trace; UNVERIFIED by execution.

## 4. Per-function table (file >100 LOC — all 28 units)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Arg::to_display | 44-50 | REAL | — | Str/Int/Float to text; tests cover |
| From<&str>/String/i64/f64 for Arg | 53-75 | REAL | — | trivial conversions |
| format_float | 78-84 | REAL | — | 1e15 guard; NaN/inf → "NaN"/"inf" text, no panic |
| format_percent | 88-101 | REAL | S4 | F-007(a) negative zero |
| PlaceholderKind::tag / Display | 119-133 | REAL | — | tested (placeholder_kind_tags) |
| Message::parse | 276-286 | REAL | S3 | F-004 unbounded recursion entry |
| Message::placeholder_set | 291-295 | REAL | — | feeds audit rule 3 |
| Message::plural_arms_by_name | 299-303 | REAL | — | feeds audit rule 5 |
| collect_plural_arms | 305-327 | REAL | — | recursive; F-004 depth |
| collect_placeholders | 329-348 | REAL | — | recursive; last-write-wins on name dup with same tag |
| validate_placeholder_types | 351-363 | REAL | — | tested (ConflictingPlaceholderType case) |
| collect_conflicts | 365-395 | REAL | — | recursive; F-004 depth |
| Message::render | 400-408 | REAL | — | panic-free verified by reading |
| render_into | 410-432 | REAL | — | `None => out.push('#')` arm (L425) unreachable by construction, commented |
| find_arg | 435-439 | REAL | — | O(args) per placeholder; fine at catalog scale |
| render_placeholder | 441-534 | REAL | S3 | F-002 dead diagnostic, F-003 select mislabel, F-005 plural approximation |
| MessageError::offset | 205-219 | REAL | S4 | F-007(b) hardcoded 0 |
| Display for RenderDiagnostic | 244-263 | REAL | — | unused outside tests? see §7 |
| Parser::peek / starts_with | 565-571 | REAL | — | byte-index + char boundary safe (pos only advances by len_utf8) |
| Parser::skip_ws | 573-577 | REAL | S4 | F-006 newline |
| Parser::parse_ident | 581-595 | REAL | — | matches documented grammar; non-ASCII rejected via InvalidName |
| Parser::parse_message | 601-636 | REAL | S3 | F-004; top-level-only `{{`/`}}` escape documented L16-19 |
| Parser::parse_placeholder | 638-695 | REAL | — | all error arms exercised by tests |
| Parser::parse_number_style | 698-737 | REAL | — | rejects bare `number`, non-percent styles; tested |
| Parser::parse_arms | 739-862 | REAL | — | duplicate-arm + MissingOtherArm enforced; double PluralCategory::parse (L803, L847) redundant but harmless |
| tests (8 fns) | 865-1070 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parses_plain_text_and_escapes | YES | asserts placeholder_set len 0 for escapes |
| parses_all_v1_forms | YES | asserts exact set keys+tags per form |
| rejects_malformed_messages | YES | 14 (input, exact error, offset) triples incl. duplicate arms and conflicting types |
| renders_simple_and_typed_placeholders | YES | exact string "Hello, Lumen! 45%", zero diagnostics |
| renders_select_and_plural | YES | 1→"1 item", 5→"5 items", unmatched select→other |
| rendering_never_panics_on_bad_args | YES | missing arg → literal + 2 diagnostics; Str-into-percent → literal + 1 |
| number_formatting_helpers | YES | exact outputs incl. 0.455→"45.5%" rounding |
| placeholder_kind_tags | YES (weakly) | constant-vs-constant |
Coverage gaps (not dishonesty): no Float plural count test, no ru/pl/ar render test (F-005 would surface), no test that category-missing fallback is silent (F-002 would surface), no multi-line placeholder test (F-006).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Rendering never panics: every problem ... records a RenderDiagnostic and degrades visibly" (L20-22) | message.rs docs | REAL | render path L400-534 has no panic site; test rendering_never_panics_on_bad_args passes |
| "select/plural placeholder is missing the mandatory `other` arm" enforced | L185-186 | REAL | parse_arms L853-857 + test case `{n, plural, one {x} two {y}}` |
| PluralCategoryMissing doc: "the `other` arm was used instead" | L234-236 | FALSE (doc) | F-002: silent fallback L517-520, diagnostic pushed only on parse-impossible branch |
| doc 07 §6.2 subset = these four forms | docs/07 §6.2 | REAL | grammar L601-862 implements exactly them |
| doc 07 §6.3: "a missing translation can never crash rendering" | docs/07:673 area | REAL (library) / UNREACHABLE (product) | F-001: no shipped consumer renders through this crate |

## 7. Dead / duplicated / unreachable code
- Whole file: unreachable from shipped entry points (F-001) — compiled only by workspace-wide lanes and its own tests.
- `RenderDiagnostic::PluralCategoryMissing` construction (L525-528): unreachable branch (parse forbids arm-less `other`).
- `Part::CountSelf` render arm `None => out.push('#')` (L425): unreachable by construction (CountSelf only parses in PluralArm, which always passes count); honestly commented.
- Duplicated catalog system: `ui/src/i18n/messages.ts` (TS) vs `locales/en-US.toml` (this crate) — two sources of user-visible truth.

## 8. Dependencies & flags
Crate deps: toml 1.1, thiserror 2.0 only (Cargo.toml). No feature flags, no cfg gating beyond `#[cfg(test)]`. No unsafe. Workspace lints deny unwrap_used/unsafe_code — consistent with the panic-free claim.

## 9. Open questions for the next agent
- W11-A08 (locales/en-US.toml): does the shipped catalog contain any plural/select keys whose *runtime* rendering nothing consumes? (F-001 makes this moot for product, not for CI gates.)
- Whoever owns ui/src/i18n: confirm the TS catalog and locales/en-US.toml are kept in sync by any gate, or whether `l10n-audit`/config-validator validate a catalog the UI never reads.
- Phase-4 engine decision (doc 07 §6.2 [PROPOSAL]): if Fluent lands, this parser is replaced wholesale — F-002/F-005/F-006 fix-or-delete decision belongs there.
