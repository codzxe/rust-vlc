# AUDIT: crates/settings-schema/src/parse.rs
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-23 |
| Lines / functions | 992 (both chunks L1-800 + L801-992) / 24 fns + 7 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: lumen-cli `apps/cli/src/host.rs:1402 setting_set -> settings_schema::runtime::validate_value -> runtime.rs:55 embedded() -> parse::parse_str(EMBEDDED_SOURCE)`; desktop composes the same host (`apps/desktop/src/dispatch.rs:67 ShellHost{host: lumen_cli::host::CliHost}`). Runs on first config.get/set-setting. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `sed -n` (quote verification, exit 0 x6); `rg` over schemas/settings.toml (exit 0/1); baseline `cargo-test-workspace.txt:5310-5390` cited (43+8 tests pass, exit 0) |

## 1. One-paragraph verdict
This is a real two-stage schema parser: `toml`+serde into optional-field raw structs, then a per-entry conversion that produces the documented E-PARSE/E-FIELD/E-TYPE/E-DOMAIN/E-DEFAULT/E-FORMAT/D1 diagnostics with line attribution from a hand-rolled line scanner. All 43 crate unit tests + 8 integration tests pass in the orchestrator baseline, including the committed-seed self-validation and byte-idempotent regeneration. The defects found are quality-level: a misleading diagnostic message for a missing (not mistyped) range bound, a scanner that mishandles escaped quotes and multiline strings (line-attribution only), an f64 widening of integer range bounds, and one dead fallback. The parser is genuinely reachable from both shipped entry points. The **settings round-trip** the matrix implies does NOT include persistence: see §6 — set→get is in-memory only; there is no serialize-to-disk/deserialize path anywhere in the shipped hosts, and doc 07's layer-4 "persisted settings written by config.set-setting" claim is contradicted by the code.

## 2. What actually works (verified)
— verified by `sed -n '5310,5390p' audit/runs/baseline/cargo-test-workspace.txt` -> settings-schema: 43 passed / 0 failed (incl. `minimal_entry_parses`, `embedded_schema_parses_and_self_validates`, `committed_outputs_have_no_drift`, `regeneration_is_idempotent_byte_for_byte`), package exit_code=0. Committed seed: 121 `[[setting]]` entries, no multiline strings (rg), so the scanner's latent gaps don't fire on the real file.

## 3. Findings
F-001 [S3]
Where: crates/settings-schema/src/parse.rs:563-572
Code:
```
    if let Some(range) = raw.range.as_ref() {
        let min = range.min.as_ref().and_then(value_as_number).or_else(|| {
            ctx.err(
                diags,
                "E-TYPE",
                "range.min",
                "range.min must be a number (an integer for int/duration-ms)".to_owned(),
```
What is wrong: when the `min`/`max` KEY IS ABSENT from `[range]` (not merely mistyped), the diagnostic claims a type problem ("must be a number") instead of "missing mandated key". The error still fires and parse still fails, so nothing invalid passes — but the message sends the author hunting a type error that does not exist. Same for `max` at :573-581. Why it matters: this crate's whole value proposition is "actionable diagnostics" (module doc L1-2). What correct looks like: distinguish `None` (missing key → E-FIELD) from `Some(value)` that fails `value_as_number` (→ E-TYPE).
Confidence: HIGH — Verification: read (control flow traced; `Option::or_else` runs the closure when the key is absent).

F-002 [S3]
Where: crates/settings-schema/src/parse.rs:819-833
Code:
```
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
What is wrong: (a) an escaped quote `\"` inside a basic string toggles `in_basic` off, so a later `#` inside the string truncates the line early; (b) multiline basic/literal strings (`"""`/`'''`) are not recognized at all, so any content line containing `=` is indexed by `LineIndex::new` as an assignment and can collide with a real lookup path (e.g. a content line reading `key = "x"` registers `setting.<i>.key` at the wrong line). Blast radius: diagnostic LINE NUMBERS only — the `toml` crate is the authoritative parser and `parse_str` L34 uses it. Verified the committed `schemas/settings.toml` contains no `"""` and no `\"#` patterns (rg, exit 1), so this is latent. What correct looks like: handle `\"`/`\\` escapes and bracket multiline delimiters, or drop the fallback precision claim in the module doc.
Confidence: HIGH — Verification: read + rg over the committed seed.

F-003 [S4]
Where: crates/settings-schema/src/parse.rs:645-653
Code:
```
/// A numeric TOML value as `f64` (integers widen — representation, not
/// coercion, for float-typed settings).
fn value_as_number(value: &toml::Value) -> Option<f64> {
    match value {
        toml::Value::Integer(n) => Some(*n as f64),
```
What is wrong: integer range bounds are stored as f64 (`model.rs:239 pub min: f64`); any bound with |n| > 2^53 silently rounds, and `check_value`'s int path (model.rs:356 `check_range(domain, n as f64, ...)`) compares in the same lossy space. Unreachable with realistic settings ranges; a correctness smell in a validator that promises exactness. What correct looks like: an i64 range arm for integer-valued types.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/settings-schema/src/parse.rs:375-379
Code:
```
        // w26 Lead: the empty Path/Url default ("unconfigured") — accepted
        // above without the type check.
        (Some(value), _) => DefaultValue::Literal(value.clone()),
        (None, None) => DefaultValue::Literal(toml::Value::Boolean(false)),
    };
```
What is wrong: the `(None, None)` arm fabricates `Literal(false)` for an entry that has neither default nor default_ref. It is unreachable-in-effect today only because the D1 "neither" diagnostic (L246-254) precedes it and makes `parse_str` return `Err`; if D1 ever moved to a separate pass this would silently invent `false` defaults. Dead defensive code masquerading as a value.
Confidence: HIGH — Verification: read (D1 diag pushed at L247-254 before this match).

F-005 [S4]
Where: crates/settings-schema/src/parse.rs:54
Code:
```
            header_line: index.line_of(&format!("setting.{i}")).unwrap_or(1),
```
What is wrong: if the scanner misses an entry header (quoted header segments, header on the same line as other syntax), all of that entry's diagnostics fall back to line 1 — the top of file — instead of a nearby line. Cosmetic, diagnostics-only, but the fallback is the least useful choice available (first entry's header would be closer).
Confidence: MEDIUM — Verification: read; no fixture exercises the fallback path.

## 4. Per-function table (mandatory, >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse_str | 33-66 | REAL | — | full-diagnostic contract honored; Err carries every problem |
| RawFile/RawAreas/RawSetting/RawRange/RawPlacement/RawCli | 70-126 | REAL | — | deny_unknown_fields on all six shapes |
| EntryCtx::field_line/field_path/err | 137-154 | REAL | — | path form `setting[i].field` |
| parse_areas | 159-210 | REAL | — | missing table/key, empty list, kebab, duplicate checks; O(n²) `seen.contains` fine for ~10 areas |
| convert_entry | 216-492 | REAL | — | 8 mandated fields presence → format checks → domain → default → placement → cli → extensions; `?`-returns None after presence pass so all E-FIELDs are reported |
| convert_domain | 496-643 | REAL | — | range/enum exclusivity + requirement matrix correct; inverted range caught (L598) |
| value_as_number | 647-653 | REAL | S4 | F-003 f64 widening |
| render_value | 656-661 | REAL | — | diagnostic rendering |
| is_kebab | 665-673 | REAL | — | ascii lower+digit |
| is_setting_key | 675-686 | REAL | — | exactly two kebab segments |
| is_snake_path | 688-697 | REAL | — | ≥2 snake segments |
| is_long_flag | 699-702 | REAL | — | `--`+kebab |
| is_short_flag | 704-707 | REAL | — | single ascii lowercase |
| is_version | 709-717 | REAL | — | rejects 2-part and 4-part versions; accepts pre-release suffix |
| line_of_offset | 720-726 | REAL | — | byte-count, `offset.min(src.len())` guards |
| LineIndex::new / line_of | 737-772 | REAL | S3 | F-002 scanner gaps (line attribution only) |
| table_header | 780-800 | REAL | — | `[[`-array detection, unquotes segments |
| assignment_key | 802-817 | REAL | — | first `=`, rejects `[`-prefixed keys |
| strip_comment | 819-833 | REAL | S3 | F-002 |
| unquote | 835-845 | REAL | — | byte-index slicing assumes well-formed pairs; scanner-only |
| tests (7) | 847-992 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| minimal_entry_parses | YES | asserts areas, len==1, entry lookup on real parse |
| missing_mandated_field_is_actionable_with_line | YES | pins rule, line 5, and path string |
| unknown_field_is_rejected | YES | typo'd field must yield E-PARSE |
| default_and_default_ref_both_is_d1 | YES | pins D1 "both" message |
| int_without_range_is_e_domain | YES | pins E-DOMAIN "numeric settings" |
| literal_default_out_of_range_is_e_default | YES | pins V-RANGE inside E-DEFAULT |
| line_index_maps_setting_fields | YES | pins exact lines 3/5/6/13 |
Gaps (mutation survivors, UNVERIFIED by execution — reasoning only): enumeration member checks (empty/whitespace/duplicate members, L611-638), areas kebab/duplicate rules (L189-208), scope/array_policy invalid-value arms (L440-461), cli.short format (L424-434), and the Path/Url empty-default bypass (L338-339) have no parse.rs test; `validate.rs`/integration tests cover some downstream effects.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Two-stage: toml + serde parse … then a conversion pass" | parse.rs:4-7 | REAL | L34 toml::from_str + L57 convert_entry; baseline tests pass |
| doc 07 §4.1 layer 4: "User preferences … persisted settings written by `config.set-setting`" | docs/07-config-driven-system.md:466 | FALSE (vs shipped code) | apps/cli/src/host.rs:1404 `self.settings.insert(full, value.clone())` — in-memory `BTreeMap` (host.rs:181 "Session-local settings overrides"); no write-to-disk anywhere; `rg` for set_setting persistence found none; desktop wraps the same CliHost (apps/desktop/src/dispatch.rs:67) |
| doc 07 §4.1: "user-scope keys persist (layer 4)" | docs/07-config-driven-system.md:470 | FALSE (vs shipped code) | same proof; also settings.toml currently has ZERO `scope = "session"` entries (rg exit 1) — every key is nominally "persisted" yet nothing persists |
| matrix ui-screen-settings "implemented" (windows/linux/macos) | docs/feature-matrix.toml:2709 | SHALLOW | the row's own notes (L2711) admit "mock persistence live; real config.set-setting write path is the open handoff" — status cell says implemented while the write path is localStorage mock + in-memory BTreeMap |
| "running twice is byte-identical (idempotence … CI's drift check via `--check`)" | emit.rs:2-4 (parse's sibling) | REAL | baseline `regeneration_is_idempotent_byte_for_byte` + `committed_outputs_have_no_drift` pass (cargo-test-workspace.txt:5385-5387) |
**ROUND-TRIP ANSWER (hostile focus):** get/set/serialize/deserialize — set: `Command::ConfigSetSetting` → host.rs:1390 `setting_set` → `setting_value_to_toml` (host.rs:1593) → `runtime::validate_value` → `parse_str`-parsed embedded schema + `check_value` → insert into in-memory BTreeMap. get: host.rs:1366 reads the same map (or `default_value`). **In-process set→get preserves the exact SettingValue (it stores a clone, not the converted toml)** — verified by baseline-passing `config_writes_round_trip_through_dispatch` (apps/desktop/src/shell.rs:452-487). **No serialize/deserialize of user values exists**: nothing writes the map to disk, and emit.rs only generates TS/help/docs from the schema (one-way; no TOML emitter), so a cross-restart round-trip is impossible in the shipped build. Conversion-seam losses if it ever existed: whole floats become TOML integers (host.rs:1597), ints >2^53 lose precision (f64 `Number`), List is string-only. Additional honesty gap: `default_value` for `DefaultValue::Ref` returns the marker string `"-> product.toml {path}"` (host.rs:1569) — get-setting on an unset default_ref key reports a fake value, admitted `[PROPOSAL]` at host.rs:1562-1564.

## 7. Dead / duplicated / unreachable code
- L378 `(None, None) => Literal(false)` fallback (F-004) — unreachable-in-effect.
- No dead exports; every predicate is called.

## 8. Dependencies & flags
- `toml` (serde), `serde::Deserialize` — used. No feature gates in this file; crate builds in the release lane (baseline cargo-build-release-lumen-cli.txt exit 0).
- LineIndex duplicates product-config's `lines.rs` approach by design (module doc L10-13) — acknowledged duplication, one shared abstraction would be better (S4, cross-crate).

## 9. Open questions for the next agent
- runtime.rs/validate.rs/emit.rs/integration.rs are NOT in my assignment; the D1 join (`validate.rs` + `tests/integration.rs::every_seed_default_ref_resolves_in_product_toml`) passed in baseline but was not audited line-by-line by me.
- Who owns the layer-4 persistence card — if any crate implements settings file writing, it escaped my rg over `apps/`; UNVERIFIED beyond apps/ (I did not audit crates/playback-engine for a settings store).
