# AUDIT: crates/theme-tokens/src/model.rs
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines / functions | 231 / 18 methods (data model, no tests in-file) |
| File verdict | REAL (dev-pipeline load-bearing; not linked into any shipped binary) |
| Reachable from shipped entry point? | Indirectly YES via committed artifact: `cargo xtask tokens` -> `cargo run -p theme-tokens --bin tokens-gen` (tools/xtask/src/cmd/tokens.rs:54-61 subprocess bridge) -> parse.rs -> model.rs -> emit.rs -> `themes/generated/tokens.css` -> copied to `ui/src/tokens/generated/tokens.css` -> imported by shipped UI `ui/src/main.tsx:27`. NOT linked at runtime: no crate/app depends on `theme-tokens` (manifest scan; the mentions in crates/commands + settings-schema Cargo.tomls are comments). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | per-manifest rg for `theme-tokens` deps (only comment hits); rg "theme_tokens::" repo-wide (crate-internal only: bin + tests); rg tokens.css consumption in ui/; baseline citation cargo-test-workspace.txt:6250-6331 (22 unit + 2 diff + 12 pipeline + 2 semantic_lint pass); sed quote verification |

## 1. One-paragraph verdict
A clean, small data model: `ResolvedValue` (color/dimension/number/shadow/family) with honest typed accessors, `ModeValues` (single vs per-mode with a documented `value_for` asymmetry), and `TokenSet` accessors over the parsed/validated token set. No panics, no `unwrap`, no stringly-typed parse logic in the hot path; `value_for` is only consumed behind `match`/`Result` contexts (parse.rs:695-701 turns `None` into a typed `TokenError`, never a panic). The model is exercised end-to-end by the pipeline/diff/semantic-lint test suites (36 tests, all green in baseline), and its output artifact (tokens.css) is what the shipped UI actually renders with. The two defects: the layer tag is a stringly-typed `&'static str` compared with `==` in `counts()` (a typo silently undercounts), and the OTHER artifact this model feeds — the generated Rust constants file — has zero consumers, making half the pipeline's output dead.

## 2. What actually works (verified)
— verified by baseline audit/runs/baseline/cargo-test-workspace.txt:6250-6331: theme-tokens package exit_code=0; 22 lib tests + 2 diff tests (committed artifacts match regeneration) + 12 pipeline tests + 2 semantic_lint tests all pass.
— verified by ui/src/main.tsx:27 `import "./tokens/generated/tokens.css"` + ui/src/tokens/README.md:8-11: the committed tokens.css (produced through this model) is the bridge the shipped UI consumes.
— verified by rg: every `value_for` call site (parse.rs:695) handles `None` via typed error; no unwrap on it anywhere.

## 3. Findings
F-001 [S3]
Where: crates/theme-tokens/src/model.rs:124-135 and 211-225; artifact: crates/theme-tokens/src/generated/tokens.rs
Code:
```
    /// `primitive` or `alias`.
    pub layer: &'static str,
```
```
    pub fn counts(&self) -> TokenCounts {
        TokenCounts {
            primitives: self
                .non_semantic
                .iter()
                .filter(|t| t.layer == "primitive")
                .count(),
```
What is wrong: (a) stringly-typed layer tag — `counts()` compares `t.layer == "primitive"` / `== "alias"` by literal; a parse-side typo produces a token counted in neither bucket, silently. An enum with `#[derive]` costs nothing. (b) Related dead-output fact established from this model: `crates/theme-tokens/src/generated/tokens.rs` — header claims "Semantic-layer constants for engine-side consumption" — has ZERO consumers: `theme_tokens::` resolves only inside the crate's own bin/tests; no crate or app depends on theme-tokens. Half the pipeline's output (the Rust constants, 86 semantic tokens × modes × 9 accent ramps) is generated, committed, diff-tested — and read by no engine code. The "engine-side consumption" claim in the generated header is false today. Confidence: HIGH — Verification: read + rg (exhaustive).
F-002 [S4]
Where: crates/theme-tokens/src/model.rs:89-99
Code:
```
    /// The value for `mode` (`None` = any/modeless context). `None` if the
    /// token has no entry for that mode (prevented by validation).
    pub fn value_for(&self, mode: Option<&str>) -> Option<&ResolvedValue> {
        match self {
            ModeValues::Single(v) => Some(v),
            ModeValues::PerMode(pairs) => {
                let mode = mode?;
```
What is wrong: `None` means "any mode" for `Single` but "no answer" for `PerMode` — one parameter, two semantics, divergent by variant. It is documented and the only caller handles it with a typed error whose message ("is not a color") misdescribes the actual failure (modeless context on a per-mode token). Latent confusion, no live bug. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DimensionUnit::as_css | 20-26 | REAL | - | trivial |
| ResolvedValue::as_color | 45-50 | REAL | - | used by lint (raw-hex recognition) |
| ResolvedValue::as_f64 | 53-58 | REAL | - | drops unit — callers must context-know |
| ResolvedValue::as_shadow | 61-66 | REAL | - | |
| ResolvedValue::as_family | 69-74 | REAL | - | |
| ModeValues::value_for | 91-99 | REAL | S4 | None-asymmetry (F-002) |
| ModeValues::is_per_mode | 102-104 | REAL | - | used by emit.rs:251 |
| TokenSet::modes | 164-166 | REAL | - | |
| TokenSet::default_mode | 169-171 | REAL | - | |
| TokenSet::accents | 174-176 | REAL | - | |
| TokenSet::default_accent | 179-181 | REAL | - | |
| TokenSet::semantic | 186-188 | REAL | - | BTreeMap-sorted determinism documented (j18 F12 note) |
| TokenSet::semantic_by_css_name | 191-193 | REAL | - | linear scan, 86 tokens — fine |
| TokenSet::non_semantic | 196-198 | REAL | - | lint universe |
| TokenSet::accent_ramps | 201-203 | REAL | - | |
| TokenSet::accent_family_css | 206-208 | REAL | - | |
| TokenSet::counts | 211-225 | REAL | S4 | stringly-typed filters (F-001a) |
| TokenSet::ramp_for | 228-230 | REAL | - | linear scan, 9 ramps |

## 5. Test honesty
No tests in this file; the model is covered downstream by tests/pipeline.rs (12), tests/diff.rs (2, committed-artifact drift), tests/semantic_lint.rs (2) — all pass (baseline:6250-6331). Mutation reasoning: delete `value_for`'s PerMode arm -> parse.rs ramp mining fails -> pipeline tests fail; corrupt counts() -> pipeline.rs count assertions fail (verified count assertions exist in tests/pipeline.rs — 123/86/86 header numbers are asserted there).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Semantic-layer constants for engine-side consumption" | crates/theme-tokens/src/generated/tokens.rs:6 | FALSE | no `theme_tokens::` consumer outside the crate; no manifest depends on theme-tokens (F-001b) |
| "this is NOT token-file order, but it is stable across runs/platforms" | model.rs:183-185 | REAL | BTreeMap-sorted; diff.rs drift tests pass on regeneration (baseline) |
| "`None` if the token has no entry for that mode (prevented by validation)" | model.rs:90-91 | REAL | parse-time per-mode coverage validation (doc 08 §3.4) — pipeline tests exercise |

## 7. Dead / duplicated / unreachable code
- Generated `crates/theme-tokens/src/generated/tokens.rs` — dead artifact, regenerated + diff-tested forever, consumed by nothing (F-001b).
- No dead functions in model.rs itself.

## 8. Dependencies & flags
crate::color only (Rgb8, Shadow, AccentRamp). No external crates, no features, no #[allow].

## 9. Open questions for the next agent
- If engine-side token consumption is a planned card, note the pipeline cost: every `cargo xtask tokens` run regenerates a dead file — harmless but the header claim should be corrected until a consumer lands.
- ui/scripts/gen-tokens.mjs derives tokens.ts from the CSS — a second parser of this model's output; drift between Rust and TS derivations is guarded by tokens.test.ts (pinned sha256) — verify that pin in the UI audit.
