# AUDIT: crates/theme-tokens/src/emit.rs#part2
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 408 (407–814 of 814) / 11 functions + 8 tests (`rust_document`, `const_declaration`, `rust_const`, `rgb_expr`, `rust_f32`, `rust_f32_lit`, `screaming`, `rust_str_lit`, `rust_array`, `rust_header`, struct `Const`; tests 718-814) |
| Chunk boundary | part2 = 407–814 = the entire "Rust emission" section (407-716) + the whole `#[cfg(test)] mod tests` (718-814). Equal split lands exactly on the section boundary (814/2 = 407). A10's part1 = 1–406 (API + CSS emission). NOTE for A10: 4 of the 8 tests in MY chunk exercise part1 functions (`css_num`, `css_num32`, `css_family`, `css_value`) — cover them in part1's per-function table by reference, don't re-audit. |
| File verdict | REAL (pipeline executed in baseline tests) / UNREACHABLE from shipped entry points — AND its output artifact has zero consumers (see F-001) |
| Reachable from shipped entry point? | NO for the `lumen` CLI / `lumen-tauri`: no crate in the workspace depends on `theme-tokens` (Cargo.toml grep: only self + two comment-only mentions in settings-schema/commands). Reached only by: `cargo run -p theme-tokens --bin tokens-gen` (dev regeneration, shelled by frozen xtask `tokens`), and tests (diff.rs, pipeline.rs — baseline #3, 1681 pass). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "emit_rust\|emit_css" crates/theme-tokens tests tools/xtask` (callers: tokens_gen bin + 2 test files only); `grep -rln "theme_tokens" crates/*/src apps/*/src` (self-only — no consumer); `grep -rn "theme-tokens" --include=Cargo.toml` (no dependents; 2 comment mentions); `sed -n '70,110p' tests/diff.rs` (3-way committed-copy diff + drift-sensitivity proof); `head -40 src/bin/tokens_gen.rs` (xtask subprocess bridge); `head -20 ui/scripts/gen-tokens.mjs` (UI consumes ONLY the CSS output); `grep -c "pub const" src/generated/tokens.rs` → 186; baseline #3 (tests pass), #24 (`npm run tokens:check` PASS) |

## 1. One-paragraph verdict
The Rust-emission half of the token generator is the most rigorously engineered code in my assignment: deterministic ordering, structural (not arithmetic) section split with the j18 F2 fix documented, explicit duplicate-constant rejection before it can silently shadow in the generated module, rustfmt-stable line-wrapping rules written down from a real CI failure (2026-11-13 fmt drift, run 43373340074), and an end-to-end diff test that regenerates from `themes/tokens.json` and asserts byte-equality against THREE committed copies — plus a mutation test proving the comparison is not vacuous. The hostile finding is on the other side: all of this machinery emits `generated/tokens.rs` — 186 `pub const`s whose header claims "Semantic-layer constants for engine-side consumption (doc 01 §2.3)" — and not one crate in the workspace consumes them. The CSS sibling output drives the entire UI; the Rust output is compiled into a crate nobody depends on.

## 2. What actually works (verified)
— verified by baseline #3 (`cargo test --workspace` 1681 passed) → `emit::tests` (8/8), `diff.rs::committed_outputs_match_regeneration` and `the_comparison_detects_drift` (the sensitivity proof mutates gray.700 #191919 and asserts detection), `pipeline.rs` determinism tests (emit twice → identical bytes) all pass.
— verified by `sed -n '70,110p' tests/diff.rs` → the committed `crates/theme-tokens/src/generated/tokens.rs` IS byte-identical to regeneration at HEAD (else baseline would fail); same for the UI bridge copy `ui/src/tokens/generated/tokens.css` (3-way assert).
— verified by `head -40 src/bin/tokens_gen.rs` → the xtask `tokens` subcommand shells to this binary (Lead-ratified subprocess bridge; exit codes 0/1/2 typed) — the pipeline is wired, not vestigial tooling.
— verified by baseline #24 → `npm run tokens:check` PASS — the UI side of the same pipeline is drift-free.

## 3. Findings

### F-001 [S3]
Where: crates/theme-tokens/src/emit.rs:418-521 (whole `rust_document`) and its output contract; evidence: zero dependents
Code:
```
/// Emit the Rust constant module for `tokens` (semantic layer + accent
/// ramps + mode/accent metadata).
pub fn emit_rust(tokens: &TokenSet, out: &mut impl Write) -> Result<(), TokenError> {
```
What is wrong: the entire Rust-emission section (11 functions, 310 lines) produces `generated/tokens.rs` — 186 `pub const`s — for a consumer that does not exist: `grep -rln "theme_tokens" crates/*/src apps/*/src` matches only the theme-tokens crate itself; no workspace Cargo.toml carries `theme-tokens` as a dependency (settings-schema and commands mention it in comments only); the shipped `lumen` binary therefore contains none of these constants. The generated header's claim "Semantic-layer constants for engine-side consumption (doc 01 §2.3)" is present-tense false. The CSS sibling (`emit_css`, part1) IS consumed (ui/src/tokens/generated/tokens.css drives every sheet in this wave).
Why it matters: 186 constants of dead weight in the build graph of a crate with zero dependents; the "engine-side consumption" doc claim (01 §2.3) cannot be traced to a single `use`; every regeneration cycle maintains an artifact nobody reads, and the diff test enforces its freshness with equal rigor as the live CSS — effort that proves nothing user-visible.
What correct looks like: either an engine consumer (the renderer/compositor referencing `theme_tokens::generated::*`) or delete the Rust emission + its diff assertion until the consumer card lands, with doc 01 §2.3 marked PAPER-ONLY.
Confidence: HIGH — Verification: two greps (source-use, Cargo.toml) + diff.rs read; UNVERIFIED: nothing — the absence is provable and proven.

### F-002 [S4]
Where: crates/theme-tokens/src/emit.rs:561
Code:
```
fn rust_const(_name: &str, value: &ResolvedValue) -> Result<(&'static str, String), TokenError> {
```
What is wrong: `_name` is ignored — a vestige of the pre-fmt-drift design where the wrap decision used the const name (the 551-560 comment says the decisions "now live where they belong"). The parameter survives for signature stability nobody needs (private fn, 3 call sites in the same file).
Why it matters: dead parameter on a private function is exactly the "ignored parameters" fakeness marker the checklist hunts; harmless but sloppy in otherwise-austere code.
What correct looks like: drop the parameter and update 3 call sites.
Confidence: HIGH — Verification: read + all call sites in-file (432, 442, 782).

### F-003 [S4]
Where: crates/theme-tokens/src/emit.rs:618-620
Code:
```
fn rust_f32(v: f64) -> String {
    rust_f32_lit(v as f32)
}
```
What is wrong: silent f64→f32 narrowing (`as`) at the Rust/CSS seam: `css_num` (part1) prints the f64 token value; `rust_f32` narrows to f32 first. A token value needing more than f32 precision (e.g. a dimension 0.123456789) would emit different strings in the two outputs — CSS "0.123456789" vs Rust "0.12345679" — with no comment acknowledging the divergence, while the header sells both outputs as the same resolved set.
Why it matters: latent CSS/Rust drift for sub-f32-precision values; today's tokens.json values (4.76, 13.75, 0.38) all survive f32 exactly, so no live drift — but nothing pins that invariant (no test asserts css_num(v) == rust_f32(v) for the full token set).
What correct looks like: either emit f64 (`f64` consts) or a parse-time validation that every numeric token round-trips f32 exactly (reject sub-precision values at the source).
Confidence: HIGH — Verification: read; current tokens verified round-trip-safe by inspection of the generated file's literals; no covering test found (emit.rs tests + diff.rs don't assert cross-format equality).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct Const` | 412-416 | REAL | — | name/ty/expr triple |
| `rust_document` | 418-521 | REAL | S3 | deterministic; dup-rejection 470-485 (sort + windows(2)) correct; semantic sorted / ramps in declared order; **output unconsumed** (F-001) |
| `const_declaration` | 532-549 | REAL | — | rustfmt-stable 3-case wrap; matches documented CI-drift fix |
| `rust_const` | 561-610 | REAL | S4 | F-002 `_name` ignored; Family empty-stack rejected (567-569); Shadow single/multi-line threshold `single.len() + 5 <= LINE_WIDTH` consistent with the indent rule |
| `rgb_expr` | 612-614 | REAL | — | `Rgb8::new(0x..)` lowercase hex |
| `rust_f32` | 618-620 | REAL | S4 | F-003 `as f32` narrowing |
| `rust_f32_lit` | 622-628 | REAL | — | integral <1e15 → `{:.1}`; finite guard present; non-finite falls to `{v}` which prints `NaN`/`inf` — would be a compile error in generated code, acceptable (parse should reject earlier — part1's territory) |
| `screaming` | 631-641 | REAL | — | non-alnum → `_`; collision risk (card-lg vs card.lg) caught by dup check — defense in depth |
| `rust_str_lit` | 644-656 | REAL | — | escapes `"` and `\`; control chars pass through raw — would break literals (untested path; token ids/modes are ASCII in practice) |
| `rust_array` | 661-673 | REAL | — | prefix-len budget; tested both branches |
| `rust_header` | 675-716 | REAL | — | metadata header; the "engine-side consumption" claim it stamps is F-001's false sentence |
| tests: `css_numbers_use_shortest_stable_form` | 724-733 | REAL | — | exact-output; tests PART1's `css_num`/`css_num32` (boundary note) |
| tests: `rust_floats_keep_the_point_suffix_for_integers` | 736-742 | REAL | — | exact-output, part2 |
| tests: `css_families_quote_non_generic_names` | 745-756 | REAL | — | tests PART1's `css_family`/`css_value` |
| tests: `css_shadow_renders_box_shadow_form` | 759-770 | REAL | — | tests PART1's `css_value` |
| tests: `rust_shadow_literal_is_multiline_when_long` | 773-786 | REAL | — | part2; asserts the multi-line form |
| tests: `rust_array_wraps_only_past_the_line_width` | 789-801 | REAL | — | both branches |
| tests: `rust_string_literals_escape_quotes_and_backslashes` | 804-807 | REAL | — | happy path only (no control-char case — see rust_str_lit note) |
| tests: `screaming_sanitizes_ids` | 810-813 | REAL | — | two cases |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| 8 unit tests (this chunk) | YES | exact-string assertions on real inputs; mutation of any helper breaks them |
| diff.rs::committed_outputs_match_regeneration (baseline-passed) | YES | byte-equality vs 3 committed copies — covers `rust_document` assembly that unit tests skip |
| diff.rs::the_comparison_detects_drift | YES | sensitivity proof: mutates the source, asserts the check FAILS — the anti-tautology discipline most of this repo lacks |
| pipeline.rs determinism (212-227) | YES | emits twice, asserts identical bytes |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantic-layer constants for engine-side consumption (doc 01 §2.3)" — stamped into every generated file by `rust_header` | emit.rs:687-688 (header text) / generated/tokens.rs:8 | FALSE | zero consumers repo-wide (F-001 evidence chain) |
| "Regeneration is idempotent; the committed copy is diff-tested by crates/theme-tokens/tests/diff.rs" | emit.rs:683-685 | REAL | diff.rs read + baseline #3 pass |
| "Determinism contract (doc 08 §3.4 byte-stability): … always renders byte-identical output" | emit.rs:13-16 | REAL | pipeline.rs determinism test (baseline-passed) |
| "Keep this table in sync with the role list in `rust_header` (both list the same eight)" | emit.rs:450-451 | REAL | verified: the 8 roles at 452-461 match rust_header's list at 703-706 |

## 7. Dead / duplicated / unreachable code
The whole Rust-emission section is unreachable-as-consumed (F-001): real, tested, deterministic — and read by no production code. `_name` param (F-002). No duplication.

## 8. Dependencies & flags
Crate is dependency-free except std (+ thiserror via lib). No feature flags. The xtask bridge is subprocess-based (frozen xtask manifest cannot take a path dep — documented Lead decision).

## 9. Open questions for the next agent
- F-001 cross-check: does ANY doc besides 01 §2.3 claim engine-side token consumption (e.g. the software compositor's clear-color)? If the compositor hard-codes its clear color instead of importing `theme_tokens::generated`, that's a second-order finding for the renderer wave (two sources of the same color = drift risk).
- My file 9 (generated/tokens.rs) will carry F-001 forward — the artifact side of the same finding; I'll cross-reference there.
- If Wave 16 wants a mutation-testing dry run, this crate is the cheapest target in the repo (small, deterministic, exact-output tests).
