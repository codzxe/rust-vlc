# AUDIT: crates/theme-tokens/src/emit.rs
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 814 / 25 (+8 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — build-time codegen only. Chain: `tokens-gen` bin (crates/theme-tokens/src/bin/tokens_gen.rs:102-107 `emit::emit_css`/`emit::emit_rust`) + tests/diff.rs + tests/pipeline.rs. No runtime caller in lumen-cli/desktop. Its CSS OUTPUT is shipped: emit → themes/generated/tokens.css → (diff-verified identical copy) ui/src/tokens/generated/tokens.css → ui/src/main.tsx:27 `import "./tokens/generated/tokens.css";`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `target/debug/deps/diff-081ea81a32cdde49` (run from crates/theme-tokens) -> 2 passed, exit 0; rg over crates/ + apps/ for `theme_tokens::` -> 0 consumers outside theme-tokens; `diff themes/generated/tokens.css ui/src/tokens/generated/tokens.css` -> IDENTICAL; serde_json out-of-range probe (rustup run 1.99.1 rustc, /tmp) -> `Error("number out of range")` (parse layer rejects inf); baseline audit/runs/baseline/cargo-test-workspace.txt:6250-6330 theme-tokens 38 passed exit 0. |

## 1. One-paragraph verdict
Real, working deterministic emitter pair. `emit_css`/`emit_rust` are exercised end-to-end against the committed artifacts by tests/diff.rs — I ran the prebuilt diff test binary myself (2 passed, exit 0), proving themes/generated/tokens.css and src/generated/tokens.rs byte-match regeneration from themes/tokens.json. Duplicate-name rejection, per-mode coverage error paths, and sort-based determinism are all implemented, not claimed. The two real problems: (1) the Rust half of the output is dead — 247 lines of generated constants that no engine code anywhere consumes, while the header and doc 01 §2.3 both justify them as "engine-side needs"; (2) the `ramp_color` doc comment states a data relationship ("dark-mode bright/icon equal the primary") that the code and data flatly contradict.

## 2. What actually works (verified)
— verified by `target/debug/deps/diff-081ea81a32cdde49` (run in crates/theme-tokens) -> `test committed_outputs_match_regeneration ... ok`, exit 0: emit_css + emit_rust reproduce both committed outputs byte-for-byte from themes/tokens.json.
— verified by rg `theme_tokens::` over crates/ and apps/ -> matches only inside crates/theme-tokens (lib.rs/bin/tests): confirms the consumer map below.
— verified by `diff themes/generated/tokens.css ui/src/tokens/generated/tokens.css` -> IDENTICAL (7852 bytes both).

## 3. Findings
F-001 [S3]
Where: crates/theme-tokens/src/emit.rs:9-11
Code:
```
//! * [`emit_rust`] renders `crates/theme-tokens/src/generated/tokens.rs` —
//!   semantic-layer Rust constants for engine-side needs (doc 01 §2.3),
//!   plus mode/accent metadata and the nine accent ramps.
```
What is wrong: The stated consumer does not exist. rg for `theme_tokens::` across every crate and every app returns zero users outside theme-tokens itself; `theme_tokens::generated`, `generated::ACCENT`, `generated::MODES` match nothing. The 247-line generated module (src/generated/tokens.rs) is compiled into the lib (`lib.rs:41-42 #[path = "generated/tokens.rs"] pub mod generated;`) and read by nobody — not the engine, not the CLI, not desktop, not the UI. Doc 01 (docs/01-architecture.md:505-507) claims the crate "emits ... Rust constants for engine-side needs"; the need never materialized. Why it matters: dead generated surface that must be kept fmt-stable (see the 2026-11-13 fmt-drift war documented at emit.rs:551-560) for zero consumers; the doc claim is false in practice. What correct looks like: either an engine consumer (e.g. renderer-wgpu subtitle/OSD colors) or delete emit_rust + the generated module and amend doc 01. Confidence: HIGH — Verification: ran (rg, exhaustive over crates/ + apps/).
F-002 [S3]
Where: crates/theme-tokens/src/emit.rs:13-16 and docs/08-ui-figma-fidelity.md §3.4
Code:
```
//! Determinism contract (doc 08 §3.4 byte-stability): declarations are
//! sorted by name within every block, numbers use a fixed shortest form,
//! and nothing depends on map iteration order — the same [`TokenSet`
//! always renders byte-identical output.
```
What is wrong: Not this quote — the same cited doc 08 §3.4 section also promises: "Coverage check: every semantic token is consumed ≥ once (dead-token detector) and every component-facing token exists in all modes". rg for `dead.token|consumed` over crates/theme-tokens and ui/src/tokens finds only per-mode coverage (parse.rs:314), no consumption/dead-token detector anywhere in the repo. The byte-stability half is real (I ran the diff test); the dead-token-detector half of the cited contract is PAPER-ONLY. Why it matters: unused semantic tokens can rot silently; a documented QA gate does not exist. Confidence: HIGH — Verification: ran (rg) + read doc 08 §3.4.
F-003 [S4]
Where: crates/theme-tokens/src/emit.rs:214-215
Code:
```
/// The accent-family color for `role` under `mode` (doc 08 §3.3): dark-mode
/// bright/icon equal the primary; light-mode values come from the ramp.
```
What is wrong: The comment is false for `bright`. The code two lines below maps `(ROLE_BRIGHT, "dark") => ramp.bright_dark`, and the data contradicts "equal the primary": ACCENT_ORANGE_BRIGHT_DARK = 0xe8851a vs ACCENT_ORANGE_PRIMARY = 0xd37918 (src/generated/tokens.rs:176,181); doc 08 §3.3 itself lists #E8851A as a distinct "on-light" value. Only `icon` dark equals primary. A maintainer trusting the comment would "fix" the code and silently lose the brighter dark-mode accent. Confidence: HIGH — Verification: read (code + generated data + doc 08:239).
F-004 [S4]
Where: crates/theme-tokens/src/emit.rs:147-152
Code:
```
    for (mode, block) in &mut mode_blocks {
        document.push_str(&render_block(
            &format!(":root[data-theme=\"{mode}\"]"),
            block,
        )?);
    }
```
What is wrong: A non-default mode with zero declarations still emits an empty ruleset (`:root[data-theme="X"] {\n}\n`); there is no `if block.is_empty() { continue; }` guard. Cannot trigger with the committed theme (light carries ~15 per-mode decls), but the emitter accepts any TokenSet satisfying validate_mode_pair and would ship a dead selector. Confidence: MEDIUM — Verification: read (mutation reasoning; not executed). What correct looks like: skip empty blocks in render_block callers.
F-005 [S4]
Where: crates/theme-tokens/src/emit.rs:39-50
Code:
```
const CSS_GENERIC_FAMILIES: &[&str] = &[
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
```
What is wrong: The generic-family allowlist is incomplete versus css-fonts-4: `math`, `emoji`, `fangsong` are missing. Any of those in a fontFamily stack would be quoted by `css_family` (emit.rs:334-340), turning the generic keyword into a literal family name — silently wrong font resolution. No current token uses them (committed stacks are Inter/-apple-system/Segoe UI/Roboto/sans-serif). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| emit_css | 62-66 | REAL | — | thin wrapper, exercised by diff test |
| emit_rust | 70-74 | REAL | — | exercised by diff test; output has 0 consumers (F-001) |
| css_document | 83-193 | REAL | S4 | accent/mode block assembly correct; empty-block nit (F-004) |
| render_block | 196-212 | REAL | — | sorts + rejects duplicate names; verified by pipeline tests |
| ramp_color | 216-238 | REAL | S4 | correct mapping; doc comment lies (F-003) |
| ramp_for | 240-244 | REAL | — | ok_or_else internal error |
| role_is_per_mode | 248-253 | REAL | — | unwrap_or(false) safe: parse guarantees family tokens exist |
| validate_mode_pair | 257-269 | REAL | — | hard-codes dark/light pair per doc 08 §3.3 |
| css_value | 272-298 | REAL | — | non-empty family guard; shadow rgba form correct |
| css_num | 302-308 | REAL | — | inf/NaN unreachable from parse: serde_json rejects out-of-range numbers (I verified: `Error("number out of range")`), parse.rs:427-449 requires finite non-negative dimensions |
| css_num32 | 312-318 | REAL | — | f32 twin of css_num (duplication, deliberate) |
| css_shadow_color | 320-332 | REAL | — | alpha>=1 → hex, else rgba; alpha range enforced at parse.rs:542 |
| css_family | 334-340 | REAL | S4 | incomplete generic list (F-005) |
| list_with_default | 343-354 | REAL | — | header annotation only |
| css_header | 356-405 | REAL | — | {dm}/{da} substitution correct |
| rust_document | 418-521 | REAL | — | separate semantic/ramp vectors, dup-name rejection, both verified |
| const_declaration | 532-549 | REAL | — | rustfmt-stable shapes; fmt-drift fix, CI run 43373340074 cited but not verifiable from repo |
| rust_const | 561-610 | REAL | — | continuation-line budget logic; unit-tested |
| rgb_expr | 612-614 | REAL | — | `0x{:02x}` form |
| rust_f32 / rust_f32_lit | 618-628 | REAL | — | integral keeps `.0`; unit-tested |
| screaming | 631-641 | REAL | — | collisions caught by dup check at 478-485 |
| rust_str_lit | 644-656 | REAL | — | escapes `"` and `\`; does NOT escape control chars/newlines — unreachable via committed data, parse doesn't validate family-name charset |
| rust_array | 661-673 | REAL | — | wrap decision unit-tested |
| rust_header | 675-716 | REAL | — | mirrors css_header |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| css_numbers_use_shortest_stable_form | YES | exact-string asserts on css_num/css_num32 |
| rust_floats_keep_the_point_suffix_for_integers | YES | exact-string asserts |
| css_families_quote_non_generic_names | YES | exact asserts incl. full stack join |
| css_shadow_renders_box_shadow_form | YES | exact full-string assert |
| rust_shadow_literal_is_multiline_when_long | YES | prefix/suffix asserts on real wrap path |
| rust_array_wraps_only_past_the_line_width | YES | both single and wrapped branches asserted |
| rust_string_literals_escape_quotes_and_backslashes | YES | exact asserts |
| screaming_sanitizes_ids | YES | exact asserts |
Unit tests cover helpers only; full-document honesty comes from tests/diff.rs (committed artifacts byte-compared) and tests/pipeline.rs `emission_is_deterministic` — both pass in baseline (cargo-test-workspace.txt:6290, 6286) and diff binary re-run by me.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "same TokenSet always renders byte-identical output" | emit.rs:13-16 | REAL | diff test binary run by me: committed_outputs_match_regeneration ok; pipeline emission_is_deterministic ok (baseline:6290) |
| "Rust constants for engine-side needs (doc 01 §2.3)" | emit.rs:9-11; docs/01-architecture.md:505-507 | FALSE (no consumer exists) | rg `theme_tokens::` over crates/+apps/ → only theme-tokens itself |
| "dark-mode bright/icon equal the primary" | emit.rs:214-215 | FALSE | code maps bright→bright_dark; data 0xe8851a ≠ 0xd37918 (generated/tokens.rs:176,181) |
| "dead-token detector / every semantic token consumed ≥ once" | docs/08 §3.4 | PAPER-ONLY | rg `dead.token|consumed` → nothing implements it |
| "Regeneration is idempotent; committed copy is diff-tested" | emit.rs:364-366 | REAL | my diff-binary run, exit 0 |
| "2026-11-13 fmt drift, CI run 43373340074" | emit.rs:524, 552 | UNVERIFIED | historical CI reference, not checkable from repo |

## 7. Dead / duplicated / unreachable code
- src/generated/tokens.rs (the emit_rust OUTPUT): compiled, zero readers — see F-001.
- css_num/css_num32: intentional f64/f32 duplication (documented), acceptable.
- Not unreachable in the library sense: `pub mod generated` is public API surface, so it compiles into every theme-tokens consumer that never links it (none exist).

## 8. Dependencies & flags
- std-only + crate::color / crate::model / crate::TokenError; no external deps in this file. Crate-level: serde_json only (lib.rs:30-31). No feature gates on emit paths — nothing here is cfg-gated.
- Release lane: theme-tokens is not in the release build graph except as a workspace member (lumen-cli/desktop do not depend on it — verified by rg). Dev-tool crate.

## 9. Open questions for the next agent
- Does anything in ui/ lint the generated CSS for the css-fonts-4 generic families (F-005)? stylelint.config.js mentions tokens.css but I did not audit it.
- Is the "engine-side needs" Rust-token consumer planned in a later wave (roadmap), making F-001 a timing issue rather than dead code? Check docs/ROADMAP.md token proof mentions.
