# AUDIT: crates/product-config/src/lines.rs
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-21 |
| Lines / functions | 240 / 6 fns + 6 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NOT from `lumen` CLI / `lumen-tauri` (neither depends on product-config — rg over apps/*/Cargo.toml: only apps/control-server does). YES from the workspace release binary `lumen-control-server`: main.rs:174 `load_product` → main.rs:154 `product_config::load_file` → load.rs:159 `load_str` → load.rs:77 `LineIndex::new(src)` → schema.rs:465 `lines.line_of(path)` for every diagnostic. Note: this binary's feature is switched OFF in the shipped product.toml (`control-server = false`, product.toml:167) — but the load (and thus this scanner) runs BEFORE the feature gate, so the code executes at startup regardless. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | Read file complete; Grep `LineIndex\|line_of` repo-wide (production callers: load.rs:77,114 + schema.rs:465; tests elsewhere); Read settings-schema/src/parse.rs:700-815 (the duplicated scanner) and compare; `rg -n '\"\"\"' product.toml` (no multi-line strings in the committed seed — blind spot latent, not live); Read apps/control-server/src/main.rs:95-174 + config.rs:210-269 (startup chain + feature gate); Grep `load_partial_str\|resolve_layers` (test-only callers — see load.rs audit); baseline logs: all 6 lines.rs tests green (orchrun-test-workspace-nofailfast.log.txt:4621-4626) |

## 1. One-paragraph verdict
A small, careful, correct-for-the-seed TOML line indexer whose only job is putting file:line locations on product-config diagnostics; it is genuinely exercised (six passing baseline tests, executed on the control-server startup path, consumed by schema.rs:465). Its defects are structural, not functional in the committed inputs: a near-duplicate of this exact scanner lives in settings-schema/src/parse.rs with already-diverged semantics (the copy acknowledges the copy), and the hand-rolled parsing has blind spots (multi-line strings, escaped quotes) that the committed product.toml does not currently trip. One test is tautological for its named property.

## 2. What actually works (verified)
- Indexing semantics match the committed product.toml's shape: table headers, dotted headers (`[identity.ids.windows]`), assignments, quoted segments (unquote), and the future `[[migrations.entries]]` array-of-tables contract (per-path `array_counts`, element pushed as `"0"`, `"1"` — lines.rs:38-48) — all pinned by `array_of_tables_elements_are_indexed` and `finds_table_keys` asserting exact line numbers (14, 19, 4, 8, 11), green in baseline nofailfast 4621-4626.
- Ancestor fallback (`line_of`, 64-76): deepest-prefix lookup, returning the `[identity]` header line for a missing member and `None` for a wholly unknown root — asserted by two tests including the None case.
- Byte-safety: `table_header`/`unquote` slice only at ASCII quote brackets (`[`/`]`/`"`/`'` are 1-byte), so no UTF-8 boundary panic; `strip_comment` is char-based. No unwrap/expect/panic in the file (crate-wide no-panic policy holds here).
- Production wiring: schema.rs:465 `lines.line_of(path)` — the scanner's output is what makes every structural diagnostic carry a real line. Chain verified from main.rs:174 backward.

## 3. Findings

### F-001 [S3] The scanner is duplicated across two L1 crates with already-divergent semantics
Where: crates/product-config/src/lines.rs:21-77 vs crates/settings-schema/src/parse.rs:728-773
Code (the twin acknowledging the copy, settings-schema/src/parse.rs:728-752):
```rust
// --- line index (same read-only scanner approach as product-config lines.rs)
```
```rust
impl LineIndex {
    fn new(src: &str) -> Self {
        let mut lines: Vec<(String, u32)> = Vec::new();
        let mut table: Vec<String> = Vec::new();
        let mut setting_count = 0usize;
        for (idx, raw) in src.lines().enumerate() {
```
What is wrong: `table_header`, `assignment_key`, `strip_comment`, `unquote` and the `LineIndex` constructor are re-implemented nearly verbatim in settings-schema's parse.rs. The copies have already diverged in behavior: this file keys a `HashMap` and tracks a per-path counter per array-of-tables kind (`array_counts.entry(path.clone())`, lines.rs:39-44), so two different `[[a]]`/`[[b]]` arrays number independently; the settings-schema twin uses ONE global `setting_count` for every array header (parse.rs:750-752), which would misnumber elements if a second array-of-tables kind ever appeared in settings.toml. This file's `line_of` falls back to ancestors (64-76); the twin does exact-match only (parse.rs:767-772).
Why it matters: Two hand-rolled TOML scanners that must agree on TOML's grammar will not stay in agreement — a fix to one (e.g. the multi-line-string blind spot below, or escape handling) silently misses the other. The duplication is forced by the layering rule (both crates L1, no internal deps), but the current split leaves the grammar logic owned by nobody in particular.
What correct looks like: Either an L0 shared micro-crate for spanless TOML line indexing (both crates already depend on `toml` — the boundary precedent exists), or an explicit cross-reference contract test in each crate running the SAME corpus through both scanners and asserting identical output.
Confidence: HIGH — Verification: read both files side by side (product-config/src/lines.rs:21-168, settings-schema/src/parse.rs:728-815).

### F-002 [S4] Scanner blind spots: multi-line string content is misindexed; escaped quotes flip the quote-state
Where: crates/product-config/src/lines.rs:135-149
Code:
```rust
fn strip_comment(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_basic = false;
    let mut in_literal = false;
    for c in line.chars() {
        match c {
            '"' if !in_literal => in_basic = !in_basic,
            '\'' if !in_basic => in_literal = !in_literal,
            '#' if !in_basic && !in_literal => break,
```
What is wrong: (a) No multi-line string state: a line inside a `"""…"""` block that looks like an assignment (`key = value` as prose) or a header (`[section]` as prose) is indexed as real structure, and — worse for headers — resets the current-table context for subsequent lines. (b) No escape handling: in `key = "va\"lue"` the `\"` toggles `in_basic` off early, so a trailing `#` after the closing quote is no longer stripped (state ends `in_basic = true`). (c) A `[[a]]`-shaped prose line inside a literal-string block similarly misindexes.
Why it matters: Blast radius is diagnostics-only (wrong line numbers / spurious entries in the index; the typed model is produced by the real `toml` parser, so no config VALUE is ever wrong). The committed product.toml contains no multi-line strings (`rg '"""' product.toml` → no matches) and no escaped quotes in values, so today this is latent, not live. But the file's own contract comment advertises the scanner as general ("It tracks table headers … and key assignments", lines 6-8) and product.toml is user-editable input to a future layer system.
What correct looks like: Track `"""`/`'''` open/close per line and skip lines inside multi-line blocks; handle `\"`/`\\` escapes in basic strings. Or, simpler: accept the limitation and document it in the module doc so editors of product.toml know multiline prose is unsupported.
Confidence: HIGH for the mechanism (pure code trace) — live exposure today: none (seed verified clean).

### F-003 [S3] `hash_inside_string_is_not_a_comment` cannot fail for the property it names
Where: crates/product-config/src/lines.rs:213-216
Code:
```rust
    #[test]
    fn hash_inside_string_is_not_a_comment() {
        let idx = LineIndex::new("accent = \"#4f8cff\" # trailing\n");
        assert_eq!(idx.line_of("accent"), Some(1));
    }
```
What is wrong: The named property is "a `#` inside a quoted string is not a comment start". But the assertion only checks that the path `accent` was indexed at line 1 — and that stays true even if `strip_comment` is broken and truncates the line at the in-string `#`: the truncated text `accent = "` still contains the `=` (first-`=` semantics), so `assignment_key` still extracts `accent` and indexes it at line 1. Mutation: make `strip_comment` break at EVERY `#` regardless of quotes — this test still passes.
Why it matters: The place where comment stripping actually matters is table headers: `[window] # comment` — without stripping, the line no longer ends with `]`, `table_header` returns None, and every subsequent key is indexed under the PREVIOUS table (wrong lines in real diagnostics). That case has no test. The suite green-checks a property it does not test — exactly the "test names promising more than the body" pattern.
What correct looks like: Assert the header case (`LineIndex::new("[window] # c\nwidth = 1\n")` → `line_of("window.width") == Some(2)`), and/or assert the in-string-# case via a quoted KEY (`"a#b" = 1`) where truncation observably changes the indexed path.
Confidence: HIGH — Verification: read + mutation reasoned (break-at-every-# variant keeps `line_of("accent") == Some(1)`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `LineIndex::new` | 23-59 | REAL | — | Core scanner; per-path array counters (39-44) correct for multiple array kinds; `or_insert` first-wins (TOML duplicate keys rejected by real parser before diagnostics render — benign). Consumed load.rs:77,114. |
| `LineIndex::line_of` | 64-76 | REAL | — | Deepest-ancestor fallback; O(len) joins per miss — fine at product.toml scale (244 lines). Consumed schema.rs:465. |
| `join_path` | 79-81 | REAL | — | Trivial, correct. |
| `table_header` | 89-109 | REAL | — | `[a]`/`[[a]]`/dotted/quoted; ASCII-bracket slicing is boundary-safe; misses nothing in the committed seed; blind only inside multiline strings (F-002). |
| `assignment_key` | 112-131 | REAL | — | First-`=` split; rejects `[`-prefixed keys; robust to comment stripping failures (which is what makes F-003's test tautological). |
| `strip_comment` | 135-149 | REAL | S4 | Quote-state comment stripping; no multiline/escape awareness (F-002). |
| `unquote` | 152-168 | REAL | — | Matching-quote strip, byte-safe; asymmetric quotes untouched. |
| tests ×6 | 195-239 | REAL | S3 | Five mutation-sensitive; `hash_inside_string_is_not_a_comment` tautological for its named property (F-003). All green in baseline nofailfast 4621-4626. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `finds_top_level_keys` (196-199) | YES | Exact line assert on a real construct. |
| `finds_table_keys` (202-210) | YES | Asserts exact lines 4/8/11 through three nesting levels + dotted headers. |
| `hash_inside_string_is_not_a_comment` (213-216) | NO (for its named property) | F-003: truncation at the in-string `#` still leaves `accent = "` → key extracted → assert holds. Catches only total scanner failure. |
| `array_of_tables_elements_are_indexed` (219-223) | YES | Exact lines 14/19 for elements 0/1 of two `[[migrations.entries]]` blocks. |
| `missing_key_falls_back_to_nearest_ancestor` (226-232) | YES | Asserts both the Some(3) fallback AND the None case (root unknown). |
| `table_header_line_is_the_fallback_for_missing_members` (235-239) | YES | Header line vs member line distinguished exactly. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "toml crate exposes spans only on parse errors, not on parsed values, and no spanned-TOML crate is pre-audited … so this is a small read-only scanner" | lines.rs:4-6 | REAL | Verified against the crate's deps (toml 1.x per W01-A10 audit); scanner exists and is consumed (schema.rs:465). |
| "Lookups fall back to the deepest known ancestor (a missing identity.name points at the [identity] header; a missing table points at line 1)" | lines.rs:9-10 | PARTIAL | First half REAL (test 226-232). "A missing table points at line 1" holds only when the deepest ancestor is a line-1 top-level key (e.g. `schema_version.x` → line 1); a missing ROOT table returns `None` (asserted by test line 231: `nonexistent.table.key` → None) — the doc comment overstates the fallback. |
| "v1 implements product defaults + user: load the base with load_str, the user layer with load_partial_str" (context for this file's consumers) | lib.rs:23-24 | SHALLOW | load_partial_str/load_partial_file have ZERO production callers (tests only: tests/web_budgets.rs:61, tests/merge.rs:23,77,96) — detailed in the load.rs audit. |
| Doc 07 §3.5 "Every failure carries file + line + fully-qualified key path" | docs/07-config-driven-system.md (diagnostics contract) | REAL (for structural stage) | schema.rs:465 `lines.line_of(path)` feeds the Diagnostic.line field. |

## 7. Dead / duplicated / unreachable code
- DUPLICATED: the entire scanner family in settings-schema/src/parse.rs:728-815 (F-001) — acknowledged by comment, already diverged (global vs per-array counters, exact-match vs ancestor fallback).
- NOT dead: every function here has a production consumer via load.rs → schema.rs (control-server startup).
- `LineIndex::new`'s array-of-tables machinery is exercised today only by tests — the committed product.toml has zero `[[...]]` headers (migrations.entries is commented out, product.toml:236-244) — latent-but-contracted, not dead.

## 8. Dependencies & flags
std-only (`std::collections::HashMap`); no external deps in this file; no unsafe/unwrap/expect/panic; workspace lints inherited (baseline #5 clippy clean); no feature gates.

## 9. Open questions for the next agent
- If the multi-line-string blind spot (F-002) is ever fixed here, remember the settings-schema twin needs the same fix (F-001) — and the two crates cannot share code without a Lead decision on a new L0 crate.
- The doc-comment inaccuracy ("a missing table points at line 1", lines.rs:10) — whoever edits this file should either implement the line-1 fallback or fix the comment; today `parse_diagnostic` in load.rs has its OWN `.or(Some(1))` fallback for parse errors, which may be what the comment meant.
