# AUDIT: crates/settings-schema/src/diagnostic.rs
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-21 |
| Lines / functions | 138 / 7 (2 types, 4 fns/impls, 3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (chain exists: lumen CLI `config.get-setting`/`config.set-setting` → runtime::embedded → parse_str/validate → render_all on invalid schema; fires only on a schema the committed file never produces) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |

## 1. One-paragraph verdict

The diagnostic vocabulary for `schemas/settings.toml` is small, real, and
format-locked by exact-string tests that ran and passed in the baseline
no-fail-fast rerun. Two defects in the renderer: `render_all` sorts the
rendered STRINGS lexicographically, which scrambles numeric line order for
any file with multi-digit line numbers (`:100:` sorts before `:10:` before
`:2:`) while its doc comment promises "highest line number first" — a claim
its own test contradicts; and `render()` hard-codes the file name
`schemas/settings.toml` even though `schema_gen --source <path>` and
`lib::load(schema_path, …)` feed it arbitrary paths, so diagnostics lie
about which file is at fault. `Severity::Warning` is a dead variant with no
constructor.

## 2. What actually works (verified)

- All 3 unit tests ran and passed in baseline:
  `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4875-4877`
  (`diagnostic::tests::missing_line_renders_without_colon`,
  `error_renders_rule_line_and_path`, `render_all_sorts_and_ends_with_newline`).
- Consumers wired and compiling in the shipped graph: `SchemaDiagnostic::error`
  is the constructor used by parse.rs (part1, this wave) and validate.rs;
  `render_all` is used by runtime.rs:61,64 (embedded self-check failure path
  behind the `lumen` CLI's `config.get-setting`/`config.set-setting`), lib.rs
  load paths, and emit.rs — rg list in Commands.
- Format is deterministic and exact: `schemas/settings.toml:<line>: <path>:
  error[<rule>]: <message>` (test-asserted byte-for-byte).

## 3. Findings

### F-001 [S3]
Where: crates/settings-schema/src/diagnostic.rs:80-91
Code:
```rust
/// Render every diagnostic, one per line, highest line number first (the
/// conventional compiler order: the first reported failure is the one
/// nearest the top of the file after stable ordering).
pub fn render_all(diags: &[SchemaDiagnostic]) -> String {
    let mut rendered: Vec<String> = diags.iter().map(SchemaDiagnostic::render).collect();
    rendered.sort();
```
What is wrong: three-way contradiction. (1) The sort is lexicographic on the
rendered string, not numeric on the line: for lines {2,3,10,100} the output
order is 100, 10, 2, 3 (verified: `"…:100:" < "…:10:" < "…:2:" < "…:3:"` as
byte strings) — scrambled relative to any numeric order. (2) The comment's
first clause promises "highest line number first" — 2-before-3 (its own
test's assertion for lines 2 and 9) is lowest-first. (3) The parenthetical
promises "the first reported failure is the one nearest the top of the file"
— true for the test's same-digit-width pair, false for {2,10} (10 sorts
first). No reading of the comment matches the code for mixed-width lines.
Why it matters: `schemas/settings.toml` is 1627 lines; any multi-error run
of schema-gen or the embedded self-check interleaves diagnostics in a
non-monotonic order, and the doc comment actively misleads the next
maintainer about which guarantee to preserve. The test only samples
single-digit lines (2, 9) and so locks in none of the claimed orders.
What correct looks like: `sort_by_key` on `(line.unwrap_or(0), path, rule)`
(numeric), with a comment that states the actual order; a test with lines
9 and 10 (and 100) to pin it.
Confidence: HIGH — Verification: read + sort behavior reproduced (string
comparison: `['…:100:', '…:10:', '…:2:', '…:3:']`); baseline test asserts
only the 2-before-9 case.

### F-002 [S3]
Where: crates/settings-schema/src/diagnostic.rs:58-71
Code:
```rust
    /// `settings.toml:<line>: <path>: error[<rule>]: <message>`
    fn render(&self) -> String {
        let line = match self.line {
            Some(n) => format!(":{n}"),
            None => String::new(),
        };
        format!(
            "schemas/settings.toml{line}: {}: {}[{}]: {}",
```
What is wrong: the file name is a hard-coded literal. `SchemaDiagnostic`
carries no source-file field, yet the crate's own loaders accept arbitrary
paths: `settings_schema::load(schema_path, …)` (lib.rs:99-110) and
`schema_gen --source <path>` (bin/schema_gen.rs:92-95, `--source` parsed at
188). Diagnostics for a user-supplied schema file render as coming from
`schemas/settings.toml`; D1 diagnostics that join `product.toml` also blame
`schemas/settings.toml`.
Why it matters: doc 07 §3.5's diagnostic contract — "the key path is always
fully qualified from the file root (`file:key.path`)" and names "the file"
the user can fix. Pointing at the wrong file breaks actionability exactly
when someone is iterating on a scratch copy of the schema.
What correct looks like: a `file: String` (or `&'static str`) field on
`SchemaDiagnostic` set by the loader; default to the embedded name at the
runtime call sites.
Confidence: HIGH — Verification: read of render + lib.rs load + schema_gen
arg parsing (rg citations above).
### F-003 [S4]
Where: crates/settings-schema/src/diagnostic.rs:11-29
Code:
```rust
/// Severity of a schema finding. All current rules are hard errors (a
/// violation is build-breaking, doc 07 §5); the variant exists so later
/// advisory checks (e.g. deprecation notices) reuse the same channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
```
What is wrong: `Severity::Warning` is never constructed anywhere in the
crate (rg: the only hit is its own `as_str` arm), and the only constructor,
`SchemaDiagnostic::error`, hard-codes `Severity::Error` — there is no
`warning()` builder, so the promised "advisory checks reuse the same
channel" requires hand-assembling the struct through its pub fields.
Why it matters: dead variant + missing constructor is a small debt, but it
is the natural home for the `deprecated_in` advisory validation missing in
parse.rs part1 (this wave, F-004 there) — the channel exists on paper only.
What correct looks like: a `SchemaDiagnostic::warning(rule, line, path, msg)`
constructor (and eventually a rule that uses it).
Confidence: HIGH — Verification: read + rg (`Severity::Warning` single hit
at diagnostic.rs:26).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Severity` (enum) | 14-20 | REAL | S4 (F-003) | Error/Warning; Warning dead (never constructed). |
| `Severity::as_str` | 22-29 | REAL | — | Trivial mapping; Warning arm unreachable in practice. |
| `SchemaDiagnostic` (struct) | 32-44 | REAL | — | Pub fields (rule/severity/line/path/message); Eq-derived; no source-file field (F-002). |
| `SchemaDiagnostic::error` | 46-56 | REAL | — | Only constructor; hard-codes Error severity. Used by parse.rs/validate.rs; shipped-reachable via runtime::embedded. |
| `SchemaDiagnostic::render` | 58-71 | REAL | S3 (F-002) | Hard-coded `schemas/settings.toml` prefix; line rendered as `:n` or omitted. |
| `Display for SchemaDiagnostic` | 74-78 | REAL | — | Delegates to render; trivial. |
| `render_all` | 83-91 | REAL | S3 (F-001) | Lexicographic sort of rendered strings; trailing newline only when non-empty (empty input → `""`, correct). |
| tests ×3 | 97-137 | REAL | — | See §5. |

## 5. Test honesty (all ran in baseline: log lines 4875-4877, ok)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `error_renders_rule_line_and_path` | YES | Exact full-string equality on the rendered form, including line `:42`, path, `error[D2]`. |
| `missing_line_renders_without_colon` | YES | Exact string with `None` line — locks the no-line form. |
| `render_all_sorts_and_ends_with_newline` | PARTLY | Locks 2-before-9 (same digit width) and the trailing newline; would NOT catch F-001 (multi-digit scramble) nor the "highest first" comment claim — a numeric sort or a reversed sort both pass or fail differently than the comment implies. |

## 6. Doc claims touching this file

| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "highest line number first (…the first reported failure is the one nearest the top of the file…)" | diagnostic.rs:80-82 (own doc) | FALSE | Lexicographic sort: {2,3,10,100} → 100,10,2,3; own test asserts 2-before-9 (lowest first) (F-001). |
| "the key path is always fully qualified from the file root (`file:key.path`)" | docs/07-config-driven-system.md §3.5 rule 1 | SHALLOW (settings-schema analog) | `file` is a hard-coded literal, not the actual source path (F-002). |
| "Every failure carries: the rule …, the 1-based source line when locatable, the offending entry path …, and a message that states the constraint, what was found and how to fix it" | diagnostic.rs:4-7 (module doc) | REAL | Fields exist and are populated by parse.rs/validate.rs (parse.rs part1 audit, this wave; baseline tests render them). |
| "All current rules are hard errors" | diagnostic.rs:11-12 | REAL | Confirmed: no Warning construction anywhere (F-003). |

## 7. Dead / duplicated / unreachable code

`Severity::Warning` (F-003) is dead. No duplication. The runtime
(shipped-CLI) path through `render_all` is reachable-but-dormant: it fires
only if the committed embedded schema is invalid, which the committed file
is not (baseline `embedded_schema_parses_and_self_validates ... ok`,
no-failfast log line 4894) — the render path is exercised by tests, not by
any shipped run to date.

## 8. Dependencies & flags

`std::fmt` only; no external deps, no unsafe, no unwrap/expect/panic
(workspace lints). No feature gates. Exported via lib.rs:56.

## 9. Open questions for the next agent

- A10 (parse.rs part2 / schema_gen.rs): when `--source` is used, does any
  output path echo the real source path so a human can correlate? If not,
  F-002 makes schema-gen's errors actively confusing on scratch files.
- A01 (validate.rs): if a Warning-level rule is ever added (e.g.
  `deprecated_in` advisories), add the `warning()` constructor first — the
  current struct is Error-only by construction.
- Whoever owns doc 07 §3.5: the settings-schema renderer predates the
  §3.5 `[PROPOSAL]` format (value/rule/hint block) — reconcile which format
  is canonical for settings.toml diagnostics.
