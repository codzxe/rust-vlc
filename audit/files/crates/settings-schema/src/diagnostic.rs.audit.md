# AUDIT: crates/settings-schema/src/diagnostic.rs
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-23 |
| Lines / functions | 138 / 5 (Severity::as_str, SchemaDiagnostic::error, SchemaDiagnostic::render, Display::fmt, render_all) + 3 tests |
| File verdict | REAL (with one executed misattribution defect) |
| Reachable from shipped entry point? | YES — via settings-schema runtime self-check (runtime.rs:57-62 renders embedded-schema failures through `render_all`) reachable from lumen-cli `setting get/set`; and via the schema-gen binary (executed). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/schema-gen --source /dev/null --product product.toml` → exit 2, misattributed output pasted below; `cargo test -p settings-schema --offline` (diagnostic tests pass); sed quote verification |

## 1. One-paragraph verdict
A small, clean diagnostics type: rule/severity/line/path/message with a compiler-style renderer and three honest tests that pin exact output strings. Two defects: the renderer hardcodes the filename `schemas/settings.toml` regardless of which source file was actually parsed — executed proof: validating `/dev/null` reports errors "at schemas/settings.toml:1"; and `render_all`'s doc comment claims "highest line number first" while the code sorts rendered strings lexicographically (the test itself asserts the OPPOSITE order — `:2:` before `:9:` — matching the doc's own second clause, not its first).

## 2. What actually works (verified)
— `cargo test -p settings-schema --offline` → `diagnostic::tests::missing_line_renders_without_colon`, `error_renders_rule_line_and_path`, `render_all_sorts_and_ends_with_newline` all ok (pinned exact strings, would fail on any format change).
— Executed `./target/debug/schema-gen --source /dev/null --product product.toml` → exit 2 with a rendered, actionable diagnostic (proving the whole parse→diagnostic→render chain works end to end).

## 3. Findings
F-001 [S3]
Where: crates/settings-schema/src/diagnostic.rs:60-69
Code (verbatim):
```
        let line = match self.line {
            Some(n) => format!(":{n}"),
            None => String::new(),
        };
        format!(
            "schemas/settings.toml{line}: {}: {}[{}]: {}",
            self.path,
            self.severity.as_str(),
            self.rule,
            self.message
        )
```
What is wrong: the filename is hardcoded. `SchemaDiagnostic` has no source-file field, so any diagnostic produced from a non-default source is misattributed. EXECUTED PROOF: `./target/debug/schema-gen --source /dev/null --product product.toml` →
```
schema-gen: invalid settings schema:
schemas/settings.toml:1: areas: error[E-FIELD]: missing mandated `[areas]` table — declare `known = [...]` with the settings screens (doc 07 §5.4); it seeds layouts/settings-areas.toml
```
exit 2. The input was `/dev/null`; the diagnostic claims `schemas/settings.toml:1`. The `--source` flag is a first-class option (schema_gen.rs:40, exercised by integration tests against temp fixtures), so a user validating a custom schema gets line numbers attributed to a file they never passed. Why it matters: an "actionable" diagnostic that names the wrong file sends the user to edit the wrong file — the exact failure mode this type exists to prevent. What correct looks like: a `file: String` field set by the parser from the actual source path. Confidence: HIGH — Verification: ran (output pasted).
F-002 [S4]
Where: crates/settings-schema/src/diagnostic.rs:80-88
Code (verbatim):
```
/// Render every diagnostic, one per line, highest line number first (the
/// conventional compiler order: the first reported failure is the one
/// nearest the top of the file after stable ordering).
pub fn render_all(diags: &[SchemaDiagnostic]) -> String {
    let mut rendered: Vec<String> = diags.iter().map(SchemaDiagnostic::render).collect();
    rendered.sort();
```
What is wrong: the doc contradicts itself and the code. "highest line number first" vs "the one nearest the top of the file" (= lowest first) — the code does neither reliably: `rendered.sort()` is a LEXICOGRAPHIC sort of rendered strings. For lines 2 and 9 it looks numeric (test asserts `:2:` before `:9:` — lowest-first, contradicting the first clause); for lines 2, 9, 42 the order becomes 2, 42, 9 (because "42" < "9" as strings). Cosmetic blast radius (diagnostic ordering in dev tooling), but the stated contract is false twice over. What correct looks like: `diags.sort_by_key(|d| d.line)` (numeric) and a doc that states the actual order. Confidence: HIGH — Verification: read + sort semantics + test assertion cross-read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Severity::as_str | 23-30 | REAL | — | Error/Warning mapping; Warning currently unused (no advisory rules exist — the variant is future-proofing) |
| SchemaDiagnostic::error | 49-56 | REAL | — | plain constructor |
| SchemaDiagnostic::render | 58-70 | REAL | S3 | hardcoded filename (F-001) |
| Display::fmt | 72-75 | REAL | — | delegates to render |
| render_all | 80-92 | REAL | S4 | lexicographic sort + false doc (F-002) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| error_renders_rule_line_and_path | YES | pins the exact full string |
| missing_line_renders_without_colon | YES | pins the no-line form |
| render_all_sorts_and_ends_with_newline | YES for ordering-with-same-digit-width, NO for the doc's "highest first" claim | asserts `:2:` before `:9:` (lowest-first) — the test encodes the OPPOSITE of the doc's first sentence and never exercises mixed-width line numbers |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "highest line number first" | L80 | FALSE | code sorts strings; test asserts lowest-first (F-002) |
| "the first reported failure is the one nearest the top of the file" | L81-82 | REAL (by accident of lexicographic sort for same-width numbers) | test `:2:` before `:9:` |

## 7. Dead / duplicated / unreachable code
- `Severity::Warning` is constructed nowhere (rg: only `Severity::Error` via `SchemaDiagnostic::error`); the doc-comment justifies it as a future channel. Dead-but-documented.

## 8. Dependencies & flags
std-only (fmt). No feature gates, no unsafe.

## 9. Open questions for the next agent
- If schema_gen gains `--source` users (CI validating variant schemas), F-001 becomes a wrong-file-edit hazard; the fix is one field away.
