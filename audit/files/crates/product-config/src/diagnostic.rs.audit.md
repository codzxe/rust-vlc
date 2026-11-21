# AUDIT: crates/product-config/src/diagnostic.rs
| Field | Value |
|---|---|
| Auditor | W11-A01 |
| Date | 2026-11-21 |
| Lines / functions | 237 / 10 (2 enums + 2 as_str + 1 struct + location + wrap + 2 Display impls + render_all) + 5 tests + 1 test helper |
| File verdict | REAL |
| Reachable from shipped entry point? | NO from `lumen` CLI / `lumen-tauri` (no dependency edge — rg over all Cargo.tomls: `product-config` appears only in apps/control-server/Cargo.toml:43). YES from workspace-built `control-server` binary (member via `members = ["crates/*", "apps/*", …]`, compiled in baseline #1 build-release-workspace.log.txt:234): main.rs:131-154 `load_file` → load.rs builds `Diagnostic` → AppError::ProductLoad Display (config.rs:99-106) formats each via this file's `Display` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "product-config" apps/*/Cargo.toml crates/*/Cargo.toml` (only control-server depends on it) · `rg -n "product_config::" -g '*.rs'` (consumers: control-server src/tests, crate tests, tests/rebrand) · `rg -n "\.stage|Stage::" crates/product-config apps/control-server` (11 construction sites, reads in tests only) · `rg -n "Severity::Warning|Severity::Error"` repo-wide (Warning never constructed) · `rg -n "\{stage\}|stage\.to_string"` repo-wide (Display-for-Stage never invoked; hits are the engine's unrelated ErrorStage) · `sed -n` on all quoted ranges (verbatim verification) · `rg -n "members|control-server" Cargo.toml` + baseline build log (membership + compilation proof) |

## 1. One-paragraph verdict
A real, doc-conformant diagnostic renderer: the `Display` impl reproduces doc 07 §3.5's mandated `error[config]: file:key.path / value / rule / hint` format (plus the disclosed 1.B1 `line` and `expected:` extensions), and a crate-level golden test (tests/golden_s35.rs) drives the real fixture→load→diagnose→render pipeline and asserts the doc's own example byte-for-byte — it passed in baseline run #3. It is consumed at runtime only by the `control-server` binary; the two named shipped entry points (`lumen`, `lumen-tauri`) have no edge to this crate at all. The hostile finding: `Diagnostic.stage` is write-only in every production path — constructed at 11 sites, read by zero non-test consumers, deliberately unrendered, and its `Display`/`as_str` impls are never invoked anywhere in the repo (S3, the exact "declared but never read" class this wave hunts). Three S4s: `Severity::Warning` never constructed anywhere, `wrap()` promises "visible columns" but measures bytes, and `render_all` has no production caller.

## 2. What actually works (verified)
— `Display for Diagnostic` renders the doc 07 §3.5 mandatory format: golden test `probe_max_bytes_renders_the_doc_07_s35_example` (tests/golden_s35.rs:12-60) loads fixture `range-probe-max-bytes.toml` through `load_fixture_as("product.toml", …)`, asserts the exact first line `error[config]: product.toml:{line}:limits.probe_max_bytes`, the exact `value:`/`rule:`/`expected:` lines and the doc's hint prose whitespace-normalized. Passed in baseline #3 (`cargo test --workspace --locked` → 1681 passed / 1 failed / 24 ignored; failure = apps/desktop zero_alloc, unrelated).
— Runtime consumption exists: `apps/control-server/src/config.rs:99-106` renders every `Diagnostic` of a rejected product.toml through this Display impl, and `AppError::exit_code()` maps it to process exit 2 — implementing doc 07 §3.5 rule (5)'s "CLI: non-zero exit" for the one host that actually loads product.toml. control-server compiled clean in baseline #1.
— CI runs the crate's committed suite (`ci.yml:275-276` `cargo test -p product-config`), which includes this file's 5 unit tests and the golden test.

## 3. Findings

### F-001 [S3]
Where: crates/product-config/src/diagnostic.rs:59-61 (field) + 124-127 (deliberate non-rendering)
Code:
```rust
pub struct Diagnostic {
    pub severity: Severity,
    pub stage: Stage,
```
```rust
        // `error[config]:` is the doc 07 §3.5 mandatory prefix, kept verbatim
        // (severity swaps it to `warning[config]:`; the pipeline stage rides
        // along as a structured field, not rendered prose).
```
What is wrong: `Diagnostic.stage` is the wave's textbook "declared but never read". It is written at 11 production sites (load.rs:35,55,86,123,140; schema.rs:460; semantic.rs:112; merge.rs:125,143,178,222) and read by zero production consumers — the only reads are test assertions (load.rs:195, tests/fixtures_matrix.rs:44, tests/semantic.rs:51, tests/merge.rs:270). The Display impl explicitly refuses to render it, no caller branches on it, and `Stage`'s own `as_str` + `Display` impls (lines 24-39) are never invoked anywhere in the repo (repo-wide rg for `{stage}`/`stage.to_string` hits only the engine's unrelated `ErrorStage`).
Why it matters: The stage is the one field that tells an operator WHICH pipeline pass (parse/structural/semantic/merge) rejected their file — the information exists in every diagnostic and is thrown away at every consumption point. Dead structured state also rots: nothing guarantees a future renderer formats it correctly, because nothing exercises it.
What correct looks like: Either render it (e.g. `error[config/semantic]:` or a `  stage: semantic` line — one line in Display + one golden-test update), or delete the field and the enum until a consumer exists. The doc 07 §3.4 pipeline stages are worth surfacing; the current state is a typed field with no audience.
Confidence: HIGH — Verification: read + repo-wide rg (construction sites, zero production reads, Display-for-Stage never invoked).

### F-002 [S4]
Where: crates/product-config/src/diagnostic.rs:41-45
Code:
```rust
pub enum Severity {
    Error,
    Warning,
}
```
What is wrong: `Severity::Warning` is never constructed anywhere in the repo (repo-wide rg: every construction site — load.rs, schema.rs, semantic.rs, merge.rs, this file's tests — uses `Severity::Error`). The `warning[config]:` rendering path is unreachable. Unlike the sibling settings-schema `Severity` (whose doc comment discloses "the variant exists so later advisory checks reuse the same channel"), this enum carries no rationale comment.
Why it matters: Dead variant on the crate's central error type; a reader auditing the error surface must trace all constructors to learn warnings cannot occur — the type implies a capability nothing provides.
What correct looks like: Either a doc comment matching settings-schema's disclosure ("reserved for advisory checks"), or remove the variant until an advisory rule exists (deprecation notices are the natural first user).
Confidence: HIGH — Verification: read + repo-wide rg.

### F-003 [S4]
Where: crates/product-config/src/diagnostic.rs:89-99
Code:
```rust
/// Wrap `text` at `width` visible columns on word boundaries; continuation
/// lines are indented to `indent` so wrapped prose stays under its label.
fn wrap(text: &str, width: usize, indent: usize) -> Vec<String> {
```
```rust
        let candidate_len = if current.is_empty() {
            word.len()
        } else {
            current.len() + 1 + word.len()
        };
```
What is wrong: The contract says "visible columns" but the implementation measures `str::len()` — bytes. For non-ASCII hint prose (the golden hint itself contains `§`, 2 bytes) lines wrap earlier than 68 columns, never later — conservative, no overflow — but the stated contract and the code disagree, and for CJK-heavy text (3 bytes/char) wrapping would visibly undershoot (≈23 columns).
Why it matters: Comment/contract drift in the one formatting utility the §3.5 golden output depends on; anyone "fixing" to true column counting later must know the tests were written against byte behavior.
What correct looks like: Either `text.chars().count()` (true columns for the ASCII+Latin-1 hints actually produced) or the comment says "bytes (upper bound on columns; wraps early for non-ASCII)".
Confidence: HIGH — Verification: read; golden hint content verified to contain `§` (tests/golden_s35.rs:60).

### F-004 [S4]
Where: crates/product-config/src/diagnostic.rs:144-154
Code:
```rust
/// Render a list of diagnostics, each followed by a blank separator line.
pub fn render_all(diagnostics: &[Diagnostic]) -> String {
```
What is wrong: `render_all` has no production caller. The only non-test use in the repo is `runtime.rs:149` — inside `#[cfg(test)]`, feeding an `unreachable!` message. The real operator-facing multi-diagnostic renderer is control-server's own loop (config.rs:99-106 `write!(f, "\n  {diagnostic}")`), which reimplements the joining/spacing rather than calling this helper.
Why it matters: Duplicated rendering responsibility: the crate offers the canonical list renderer, its only binary consumer ignores it and hand-rolls a different one (whose `"\n  "` prefix + Display's trailing newline yields different spacing than `render_all`). Library API with zero production consumers is structure debt.
What correct looks like: control-server's `ProductLoad` arm uses `render_all(&diagnostics)` (adjusting the two-space indent), or `render_all` is demoted to test-support.
Confidence: HIGH — Verification: read + rg (callers: lib.rs re-export, runtime.rs:149 test-only, crate tests).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Stage` (enum) | 12-22 | REAL | S3 | Four variants (Parse/Structural/Semantic/Merge) matching doc 07 §3.4 stages; constructed 11×; write-only in production → F-001. |
| `Stage::as_str` | 24-33 | DEAD | S3 | Only caller is `Display for Stage`, which is itself never invoked → F-001. |
| `impl Display for Stage` | 35-39 | DEAD | S3 | Repo-wide rg: no format site for this type (`{stage}` hits are the engine's ErrorStage). Folded into F-001. |
| `Severity` (enum) | 41-45 | REAL | S4 | `Error` used everywhere; `Warning` never constructed → F-002. |
| `Severity::as_str` | 47-54 | REAL | — | Called by Display for Diagnostic every render. |
| `Diagnostic` (struct) | 56-75 | REAL | — | Carries doc 07 §3.5 fields + 1.B1 extensions; derives Debug/Clone/PartialEq/Eq — no serde (control-server formats via Display; fine). |
| `Diagnostic::location` | 81-86 | REAL | — | `file:line:key_path` / `file:key_path`; unit-tested both arms; used by Display. |
| `wrap` | 89-119 | REAL | S4 | Correct word-boundary wrapping, safe on empty input (`lines.is_empty()` guard pushes one empty line), over-long single words pass through (disclosed + tested). Byte-vs-column contract drift → F-003. O(n) per call. |
| `impl Display for Diagnostic` | 121-142 | REAL | — | The §3.5 renderer; golden-tested through the real pipeline; hint wrapped at 68 cols with 9-col continuation indent; consumed by control-server AppError. |
| `render_all` | 144-154 | REAL | S4 | Correct (per-diag block + blank separator; no trailing blank when empty) but production-dead → F-004. |
| test `sample` | 160-172 | REAL | — | Fixture mirroring the doc 07 §3.5 example verbatim. |
| test `location_includes_file_line_and_key_path` | 174-178 | REAL | — | Exact-string assert. |
| test `location_without_line_is_doc_07_s35_form` | 180-185 | REAL | — | Both location arms covered. |
| test `display_has_all_mandatory_s35_fields` | 187-203 | REAL | — | First line exact; value/rule/hint/expected contains-asserts. |
| test `wrapped_hint_keeps_label_alignment` | 205-220 | REAL | — | Requires actual wrapping to occur; asserts continuation indent + prose-ness. |
| test `wrap_never_exceeds_width_plus_indent_for_long_words` | 222-236 | REAL | — | Multi-word lines must fit ≤ width+indent; single over-long words exempt (disclosed). Would fail if wrap never split (the 63-char input line contains spaces and exceeds 19). Weak against the opposite mutation (splitting every word) but `wrapped_hint_keeps_label_alignment` covers that direction. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `display_has_all_mandatory_s35_fields` | YES | Exact first-line equality + four contains-asserts on distinct labeled lines; removing any writeln or corrupting location() fails it. |
| `wrapped_hint_keeps_label_alignment` | YES | Requires ≥2 hint lines (asserts on `hint_idx + 1` existence via unreachable!) and 9-space continuation indent; a non-wrapping or wrong-indent wrap fails. |
| `wrap_never_exceeds_width_plus_indent_for_long_words` | PARTIAL | Catches never-wrap; cannot catch always-wrap (vacuous when no multi-word line exists) — compensated by the alignment test. |
| crate golden `tests/golden_s35.rs` | YES | Drives fixture → real load pipeline → this Display; asserts doc 07 §3.5's own example byte-for-byte on machine-generated lines. This is the strongest honesty evidence in the crate: it pins the file's one externally-visible contract. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Mandatory format `[PROPOSAL]`: `error[config]: product.toml:limits.probe_max_bytes` / value / rule / hint" | docs/07 §3.5 (:440-455) | REAL | Display:121-142 + golden test; extensions (line number, `expected:`) disclosed in code comments and asserted by the golden test. |
| "(4) near-miss keys get a did-you-mean hint (Levenshtein)" | docs/07 §3.5 rule 4 | REAL | Implemented in sibling `suggest.rs` (this wave, file 3 of this agent); wired into unknown-key hints by schema.rs/load.rs. |
| "(5) runtime behavior on critical violation is abort into the config-error surface… CLI: non-zero exit" | docs/07 §3.5 rule 5 | SHALLOW | Only one host loads product.toml (control-server, exit 2 — real). The shipped `lumen` CLI has NO product-config edge (rg over Cargo.tomls) and never loads product.toml; the tauri app re-parses `[window]` with its own `parse_window_table` (app.rs:243). "CLI: non-zero exit" is true only of the control-server binary, not the product's CLI. |
| "Every failure carries file, line (1-based…), key path, rule, expected vs found, fix hint" | module doc lines 2-7 | REAL | Struct fields + Display render all of them; line numbers supplied by lines.rs LineIndex (A03's file). |
| "the pipeline stage rides along as a structured field, not rendered prose" | code comment 125-127 | REAL | Honest — and indicts itself: the "structured field" has no reader (F-001). |

## 7. Dead / duplicated / unreachable code
- `Stage::as_str` + `impl Display for Stage`: dead (never invoked repo-wide) — F-001.
- `Severity::Warning`: never constructed — F-002.
- `render_all`: production-dead; control-server duplicates its job — F-004.
- Nothing else dead: `location`, `wrap`, `Display for Diagnostic` all exercised by tests and (Display) by control-server.

## 8. Dependencies & flags
- std-only (`std::fmt`); zero external crates, zero internal crate deps — consistent with the L1 "no internal deps" claim in lib.rs.
- No feature flags, no cfg-gating, no `unsafe`, no `unwrap`/`expect`/`panic` in non-test code (tests use `unreachable!` with messages — acceptable). Clippy-clean per baseline #5/#6.

## 9. Open questions for the next agent
- A02 (product-config/runtime.rs) / A05 (lib.rs): product-config is absent from BOTH shipped entry points' dep trees. Who is supposed to consume `ProductConfig` at runtime — is control-server the only intended host, or is the `lumen` CLI's product-identity wiring (apps/cli/src/help.rs:29 "not wired in v0", host.rs:1569 renders `default_ref` as the literal string `"-> product.toml {path}"`) a Phase-2 gap? host.rs:1569 in particular means the shipped CLI never RESOLVES schema default_refs — it prints a pointer; the D1 machinery audited in my file 1 guards refs the CLI cannot even evaluate.
- Whoever audits apps/control-server: its AppError spacing differs from `render_all` (F-004) — pick one renderer.
- Docs agent: doc 07 §3.5 rule (5)'s "CLI: non-zero exit" should name control-server (or the claim stays unmet for the actual CLI).
