# AUDIT: crates/product-config/src/diagnostic.rs
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 237 / 7 (Stage::as_str, Stage::Display, Severity::as_str, Diagnostic::location, wrap, Diagnostic::Display, render_all + 5 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (via control-server). Chain: product_config::Diagnostic → `impl Display` (diagnostic.rs:121-142) → apps/control-server/src/config.rs:104-105 `write!(f, "\n  {diagnostic}")` in AppError::ProductLoad → control-server startup error surface. Schema/semantic/merge stages all construct these. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | rg `Severity::Warning` over crates/+apps/+tools/ (never constructed); rg `render_all` callers; read doc 07 §3.5 (docs/07-config-driven-system.md:440-463); baseline cargo-test-workspace.txt:5028 (41 unit tests incl. the 5 here, exit 0). |

## 1. One-paragraph verdict
Clean, correct implementation of the doc 07 §3.5 diagnostic contract: the mandatory `error[config]: file:line:key.path` / `value:` / `rule:` / `hint:` format is byte-pinned by the golden test (which I confirmed passes in the baseline) and is consumed verbatim by the control-server's error surface. All defects are hygiene: the `render_all` doc comment promises blank separator lines between diagnostics that the code does not produce; `wrap` measures bytes, not the "visible columns" its contract claims (multi-byte `§` chars appear in real hints); and `Severity::Warning` is a never-constructed variant duplicated into settings-schema.

## 2. What actually works (verified)
— verified by baseline run (cargo-test-workspace.txt:5028-5187, product-config exit 0, 41 unit tests) covering: location with/without line, all §3.5 mandatory fields present, wrapped-hint label alignment, long-word wrap bounds.
— verified by reading apps/control-server/src/config.rs:98-105: the Display output of these diagnostics is what a real shipped app prints on bad product.toml.
— doc 07 §3.5's example format (docs/07-config-driven-system.md:444-451) cross-checked against Display: prefix, `file:key.path` + line extension, `value:`, `rule:` (two-space, matching the doc), `hint:` wrap indent — all match.

## 3. Findings
F-001 [S4]
Where: crates/product-config/src/diagnostic.rs:144-154
Code:
```
/// Render a list of diagnostics, each followed by a blank separator line.
pub fn render_all(diagnostics: &[Diagnostic]) -> String {
    let mut out = String::new();
    for d in diagnostics {
        out.push_str(&d.to_string());
    }
```
What is wrong: `Diagnostic::Display` ends each rendering with a single `\n` (writeln on the last hint line, 133-139), so concatenated diagnostics have NO blank line between them; the single `out.push('\n')` at 150-152 adds one trailing newline after the LAST diagnostic only. The comment describes the opposite. A fixture producing 3 diagnostics renders as one unseparated wall of `error[config]:` blocks. Cosmetic-only (multi-diagnostic output readability); no test covers separation. Confidence: HIGH — Verification: read (string-concatenation logic; trivially deterministic). 
F-002 [S4]
Where: crates/product-config/src/diagnostic.rs:89-99
Code:
```
/// Wrap `text` at `width` visible columns on word boundaries; continuation
/// lines are indented to `indent` so wrapped prose stays under its label.
```
and
```
        let candidate_len = if current.is_empty() {
            word.len()
        } else {
            current.len() + 1 + word.len()
        };
```
What is wrong: `len()` is BYTES, not visible columns. Real hints contain `§` (U+00A7, 2 bytes UTF-8) — e.g. merge.rs hints "…(doc 07 \u{a7}4.4)." — so each `§` inflates the measured width by 1, wrapping earlier than the contract states. Purely cosmetic (over-wrap, never under-wrap of the label budget). ASCII-only hints are unaffected. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/product-config/src/diagnostic.rs:41-54
Code:
```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}
```
What is wrong: `Severity::Warning` is never constructed anywhere in the workspace (rg over crates/, apps/, tools/: only the `as_str` match arm at diagnostic.rs:51 and a twin definition in crates/settings-schema/src/diagnostic.rs:26). Every producer emits Error. Dead variant kept as planned surface — harmless, but the enum lies about current behavior (nothing can be a warning, and merge.rs:198-200 treats ANY diagnostic as fatal, so a future Warning would abort resolution unless that call site is also updated). Confidence: HIGH — Verification: ran (rg).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Stage::as_str / Display | 24-39 | REAL | — | 4 stages per doc 07 §3.4 |
| Severity::as_str | 47-54 | REAL | S4 | Warning arm dead (F-003) |
| Diagnostic::location | 77-87 | REAL | — | file:line:key / file:key forms, tested |
| wrap | 91-119 | REAL | S4 | byte-width bug (F-002); long-word overflow handled; empty-text edge yields one empty line |
| Diagnostic::Display | 121-142 | REAL | — | matches doc 07 §3.5 byte-for-byte on machine-generated parts (golden test) |
| render_all | 145-154 | REAL | S4 | separator comment false (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| location_includes_file_line_and_key_path | YES | exact string |
| location_without_line_is_doc_07_s35_form | YES | exact string |
| display_has_all_mandatory_s35_fields | YES | exact first line + contains value/rule/hint/expected |
| wrapped_hint_keeps_label_alignment | YES | asserts continuation indent + prose (no ':' leak) |
| wrap_never_exceeds_width_plus_indent_for_long_words | WEAK | only asserts multi-word lines fit; single over-long words allowed by design |
All pass in baseline (41 unit tests, exit 0). Golden S3.5 test (tests/golden_s35.rs) additionally pins the format end-to-end from the range-probe-max-bytes fixture.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| Mandatory format `error[config]: file:key.path` + value/rule/hint | doc 07 §3.5:444-451 | REAL | Display impl + golden test (baseline pass) |
| "line (1-based, when locatable)" | diagnostic.rs:3 | REAL | Option<u32> line; t10/t12 assert Some(2) through the load path |
| "did you mean" Levenshtein hints | diagnostic.rs:6-7, doc 07 §3.5 rule 4 | REAL | suggest.rs (not this file); t10 asserts "did you mean `width`" |
| "each followed by a blank separator line" | diagnostic.rs:144 | FALSE | no separator between items (F-001) |
| "wrap at width visible columns" | diagnostic.rs:89-90 | FALSE for non-ASCII | byte count (F-002) |

## 7. Dead / duplicated / unreachable code
- Severity::Warning: dead in product-config AND duplicated dead in crates/settings-schema/src/diagnostic.rs (F-003).
- Stage::Structural/Semantic/Merge all actively produced (schema.rs/semantic.rs/merge.rs) — no dead stages.

## 8. Dependencies & flags
- std-only (fmt). No features, no unsafe. Consumed by every validating module in product-config; Display consumed by apps/control-server.

## 9. Open questions for the next agent
- settings-schema/src/diagnostic.rs (twin file): is it a copy-paste duplicate of this file? The settings-schema auditor should diff them — I saw the same Severity::Warning dead arm at line 26.
- If doc 07 §4.5 T12 wants an "offer reset" flow, will Warning-severity diagnostics ever exist, or should the variant be removed?
