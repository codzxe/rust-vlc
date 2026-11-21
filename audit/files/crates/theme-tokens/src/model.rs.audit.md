# AUDIT: crates/theme-tokens/src/model.rs
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 231 / 14 (2 enums + 4 structs + 8 impl methods + 5 accessor impls) |
| File verdict | REAL (clean resolved-token data model; no logic defects found; every claim cross-checked) |
| Reachable from shipped entry point? | Build-pipeline crate: consumed by parse.rs (constructs TokenSet), emit.rs (A10's chunk), lint.rs, and tokens_gen.rs (my file 8 — `cargo xtask tokens` chain verified there). Exercised by crates/theme-tokens tests (baseline run #3: crate suites green, w26.k18 log 38/38). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read complete; consumers traced (`rg -n "TokenSet\|SemanticToken\|LayerToken\|ModeValues" crates/theme-tokens/src` → parse.rs/emit.rs/lint.rs/bin); sorted-order claim cross-checked against the emitted artifact (ui/src/tokens/generated/tokens.css :root block is alphabetically sorted ✓); counts claim cross-checked (86 semantic = tokens.css :root declaration count; 123/86/86 matches w26.k18 log and tokens_gen output contract). |

## 1. One-paragraph verdict
A genuinely clean data model: five value kinds, single-vs-per-mode values with documented parse-time coverage validation, encapsulated `pub(crate)` fields behind accessors, and an honest comment admitting the semantic ordering is BTreeMap-sorted rather than file order (verified true against the emitted CSS). No fakeness, no panics, no swallowed errors — the only notes are two hygiene items: the `layer` field is a stringly-typed `&'static str` where a two-variant enum would be type-safe, and the by-name lookups are linear scans (harmless at n≤123).

## 2. What actually works (verified)
— Model→artifact consistency: `semantic()`'s "sorted-key order" claim verified against tokens.css (:root block alphabetical); `counts()` output (123/86/86) matches the artifact header and the w26.k18 regeneration log.
— Encapsulation: all TokenSet fields `pub(crate)` (:151-159) with public read accessors — emitters cannot mutate the set.
— `ModeValues::value_for` handles modeless context (`None` mode → None for PerMode) — documented at :89-90 and consistent with parse-time validation (:83-84).
— Crate tests exercising this model passed in baseline run #3 (test-workspace.log.txt: theme-tokens compiled + suite green).

## 3. Findings

### F-001 [S4]
Where: crates/theme-tokens/src/model.rs:130
Code:
```
    /// `primitive` or `alias`.
    pub layer: &'static str,
```
What is wrong: The layer discriminant is a bare string. `counts()` filters by `t.layer == "primitive"` (:216) — a typo in a future layer name compiles fine and silently counts zero.
Why it matters: Type-safety gap in a counts path a gate prints; no current defect (both literals are spelled correctly and only parse.rs writes them).
What correct looks like: `pub enum Layer { Primitive, Alias }` with `layer: Layer`.
Confidence: HIGH — Verification: read + rg of writers (parse.rs only).

### F-002 [S4]
Where: crates/theme-tokens/src/model.rs:191-193, 228-230
Code:
```
    pub fn semantic_by_css_name(&self, css_name: &str) -> Option<&SemanticToken> {
        self.semantic.iter().find(|t| t.css_name == css_name)
    }
```
What is wrong: Linear scans for by-name lookup (`semantic_by_css_name`, `ramp_for`).
Why it matters: Nothing at current scale (86 semantic / 9 accents, called a handful of times per generation). Becomes O(n²) only if a future caller loops lookups.
What correct looks like: A BTreeMap index built at parse time (the set is already sorted).
Confidence: HIGH — Verification: read; callers enumerated (lint.rs, emit.rs, tests).

## 4. Per-function table
| Function/Type | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DimensionUnit` (+as_css) | 10-26 | REAL | — | Px/Ms → "px"/"ms"; exhaustive match. |
| `ResolvedValue` (+as_color/as_f64/as_shadow/as_family) | 28-75 | REAL | — | 5 value kinds; accessors total (Option); as_f64 conflates Number/Dimension (unit dropped — fine for lint). |
| `ModeValues` (+value_for/is_per_mode) | 77-105 | REAL | — | Single/PerMode; modeless → None (documented); coverage validated at parse. |
| `SemanticToken` | 107-121 | REAL | — | path/css_name/rust_name/category/values — all populated by parse (emit consumes; verified in tokens.css artifact). |
| `LayerToken` | 123-135 | REAL | F-001 | Lint-universe input; stringly-typed layer. |
| `TokenCounts` | 137-146 | REAL | — | Report struct. |
| `TokenSet` (+modes/default_mode/accents/default_accent/semantic/semantic_by_css_name/non_semantic/accent_ramps/accent_family_css/counts/ramp_for) | 148-231 | REAL | F-002 | pub(crate) fields + accessors; counts() filters-by-string (F-001's consumer); sorted-order doc claim verified true. |

## 5. Test honesty
No tests in this file. Model exercised by tests/pipeline.rs (parse→emit round trips) and tests/semantic_lint.rs (LayerToken universe) — both green in baseline run #3. Mutation note: breaking `counts()` would fail pipeline tests' count assertions and tokens_gen's printed counts vs the committed artifact header (diff.rs).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "sorted-key order — deterministic (BTreeMap … NOT token-file order, but stable)" | :183-185 | REAL | tokens.css :root block verified alphabetical; j18 F12 note is honest about the limitation. |
| "coverage of all registered modes is validated at parse time" | :83-84 | REAL (trust parse.rs) | parse.rs owns the validation (not my file — UNVERIFIED line-level, but the pipeline tests cover it). |
| "the only layer components may consume" (semantic) | :107-108 | Enforced downstream | lint.rs semantic_lint tests (run #3); UI side verified 76/86 consumed (my tokens.css audit F-001/F-002 notes the bypass case). |

## 7. Dead / duplicated / unreachable code
None. Every field has a consumer (modes/accents/counts → tokens_gen; semantic → emit+lint; non_semantic → lint; ramps/accent_family_css → emit).

## 8. Dependencies & flags
crate::color (Rgb8, Shadow, AccentRamp — color.rs not my assignment). No serde derives here (parse owns deserialization) — clean layering. No feature flags.

## 9. Open questions for the next agent
- parse.rs/color.rs/lint.rs owners (unassigned in my list): F-001's writers live there; the per-mode coverage validation (:83-84) should be quoted line-level in parse.rs's own audit.
