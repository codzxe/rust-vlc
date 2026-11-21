# AUDIT: crates/settings-schema/src/parse.rs#part2 (lines 496-992)
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-21 |
| Chunk boundary | part2 = lines 496-992: `convert_domain` + numeric/format predicates + `line_of_offset` + `LineIndex` scanner (`table_header`/`assignment_key`/`strip_comment`/`unquote`) + `#[cfg(test)] mod tests` (847-992). Lines 1-495 (parse_str, raw serde shapes, EntryCtx, parse_areas, convert_entry) = part1, W11-A09 — read for context, not re-audited except where a part2 consumer forced a re-read. |
| Lines / functions | 497 / 29 (16 production fns + 2 structs + 1 const + 2 test helpers + 7 tests + `mod tests`) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI `config.set-setting`/`config.get-setting` → apps/cli/src/host.rs:1402 `settings_schema::runtime::validate_value` / host.rs:1371,1400 `schema_meta` → crates/settings-schema/src/runtime.rs:55 `parse_str(EMBEDDED_SOURCE)` → parse.rs:332 `convert_domain` and parse.rs:46 `LineIndex::new`; also `lumen --help` via apps/cli/src/help.rs:157 `runtime::embedded()`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 5 |
| Commands run | Read parse.rs 1-992 (whole file; chunk 496-992 audited) · `rg -n "parse_str" crates/settings-schema` (callers: lib.rs:62,89, runtime.rs:55, validate.rs:511, emit.rs:450 — all in-crate) · `rg -n "settings_schema::" apps/cli` (host.rs:1371,1400,1402; help.rs:34,157 — shipped CLI seam) · `rg -n "settings-schema" **/Cargo.toml` (dependents: apps/cli, plugin-host-wasm) · Read runtime.rs 1-238 (embedded schema + validate_value) · Read model.rs 106-180, 330-470 (`is_ranged`/`is_integer_valued`/`check_value`/`check_range` — the domain consumers) · Read emit.rs 60-130 (enum members flow into `<a\|b\|c>` help placeholder, line 69, and docs `members.join(", ")`, line 89) · `rg -n '"""' schemas/settings.toml` → NO matches (no multi-line strings in committed schema — LineIndex desync is latent) · `rg -n "Running unittests" audit/runs/baseline/test-workspace.log.txt` → settings-schema absent (fail-fast aborted at apps/desktop) · `rg -n "parse::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → 4879-4895 all 7 tests `ok` (execution evidence) · `sed -n` verification of every quote below |

## 1. One-paragraph verdict
Part2 is genuinely implemented, reachable from the shipped CLI, and its seven tests executed green in the baseline no-fail-fast run — no stubs, no swallowed errors, no panics (all slicing is boundary-safe: `unquote` slices on ASCII quote bytes, `line_of_offset` clamps the offset). The defects are diagnostic-quality and invariant-enforcement gaps inside a crate whose entire product IS diagnostics: a missing `range.min`/`range.max` is reported as a TYPE error ("must be a number") instead of a missing-field error, a non-numeric bound on an int type produces a second nonsensical diagnostic ("must be an integer" about a string), and non-finite TOML floats (`nan`/`inf`) pass as range bounds — which silently voids the very invariant this chunk's own error message asserts ("numeric settings are always bounded", line 522-523), since `min = -inf, max = inf` yields a fully unconstrained range that runtime `check_range` accepts for any value. The line-numbering scanner is heuristic by design (documented at file top) but its `setting_count` desyncs on multi-line strings — latent only because the committed schema contains none.

## 2. What actually works (verified)
— Verified by baseline `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4879-4895` → `settings_schema` lib unittests: 43 tests ran, including all 7 `parse::tests::*` (`minimal_entry_parses`, `missing_mandated_field_is_actionable_with_line`, `unknown_field_is_rejected`, `default_and_default_ref_both_is_d1`, `int_without_range_is_e_domain`, `literal_default_out_of_range_is_e_default`, `line_index_maps_setting_fields`) — all `ok`; plus `runtime::tests::embedded_schema_parses_and_self_validates ... ok` (line 4894), proving the committed 1627-line `schemas/settings.toml` round-trips through this exact parser + D2–D6 self-validation.
— Verified by call-chain grep: every part2 function is live — `convert_domain` (parse.rs:332), all predicates (277, 314, 322, 361, 412, 424), `line_of_offset` (parse.rs:37), `LineIndex`/`table_header`/`assignment_key`/`strip_comment`/`unquote` (parse.rs:46, 54, 139, 174-194). Zero dead code in this chunk.
— Verified by hand-trace of `line_index_maps_setting_fields` against MINIMAL (line 2 `[areas]`, 3 `known`, 5 `[[setting]]`, 6 `key`, 13 `cli`): all four asserted line numbers are correct.
— Verified the multi-line `known = [...]` array in schemas/settings.toml:43-48 maps correctly (array element lines contain no `=`, `assignment_key` returns None, `line_of("areas.known")` hits the `known = [` line).

## 3. Findings

### F-001 [S3]
Where: crates/settings-schema/src/parse.rs:563-581
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
What is wrong: `RawRange { min: Option<Value>, max: Option<Value> }` (parse.rs:107-110) makes both bounds optional at the serde layer, so `range = { min = 1 }` (max absent) reaches this code with `range.max == None`; the `.or_else` closure then fires and reports "range.max must be a number" — a TYPE diagnostic for a field that is MISSING, not mistyped.
Why it matters: this crate's sole product is actionable diagnostics (module doc, lines 1-2). A schema author who forgot `max` is told their (absent) value has the wrong type — the exact class of non-actionable serde error the two-stage design exists to replace (doc comment, lines 4-6). No test covers the half-present range, so the wrong wording is pinned by nothing.
What correct looks like: distinguish `None` (E-FIELD "missing mandated range.max — numeric settings are always bounded") from `Some(v)` where `value_as_number(v).is_none()` (E-TYPE), mirroring the mandated-field treatment in convert_entry (224-245).
Confidence: HIGH — Verification: read (traced both branches; `RawRange` optionality confirmed at 107-110).

### F-002 [S3]
Where: crates/settings-schema/src/parse.rs:563-609
Code:
```
        match (min, max) {
            (Some(min), Some(max)) => {
                if min > max {
                    ctx.err(
                        diags,
                        "E-DOMAIN",
                        "range",
                        format!("range is inverted: min {min} > max {max}"),
                    );
                }
                Domain::Range(ValueRange { min, max })
```
What is wrong: non-finite TOML floats (`inf`, `nan` are valid TOML 1.0 floats) are accepted as bounds. `min = -inf, max = inf` parses clean into `Domain::Range { -inf, inf }` — a fully unbounded range — and `min = nan` passes the `min > max` guard (NaN comparisons are false), then silently disables one side of the runtime bound: model.rs:443-446 checks `if n < range.min || n > range.max` and `n < NaN` is always false.
Why it matters: this chunk itself enforces and preaches the opposite — parse.rs:521-523 rejects a missing range with "numeric settings are always bounded (doc 07 §5.2)". The parser demanding a range while accepting `-inf..inf` hands authors the exact escape hatch that voids the invariant, and runtime `config.set-setting` then accepts ANY numeric value for that setting (G4's "never silent coercion" claim, runtime.rs:1-4, defeated by a schema typo). The literal-default check cannot catch it: every default is inside `[-inf, inf]`.
What correct looks like: reject non-finite bounds in the 563-609 block (`min.is_finite()` / `max.is_finite()` gates with an E-DOMAIN diagnostic), same as the inverted-range check.
Confidence: HIGH — Verification: read (TOML float grammar and NaN comparison semantics are language-level facts; check_range read at model.rs:440-456). Not executed (no cargo runs allowed) — mutation is mechanical.

### F-003 [S4]
Where: crates/settings-schema/src/parse.rs:564-595
Code:
```
                if let Some(v) = value
                    && v.as_integer().is_none()
                {
                    ctx.err(
                        diags,
                        "E-TYPE",
                        "range",
                        format!("range.{side} must be an integer for type `{}`", ty.as_str()),
                    );
```
What is wrong: for an integer-typed entry with a NON-numeric bound (`min = "x"`), two diagnostics fire for one defect: E-TYPE "range.min must be a number" (from the failed `value_as_number`, 564-572) plus E-TYPE "range.min must be an integer for type `int`" — a nonsensical statement about a string value. Same noise class: an entry carrying both `range` and `enumeration` gets the mutual-exclusion diagnostic (504-513) AND "cannot carry an enumeration" (551-561) — two diagnostics, one mistake.
Why it matters: diagnostic noise in a diagnostics product; the module doc (line 31-32) promises "the FULL diagnostic list (every problem, one pass)" — one problem yielding two entries, one of them false, misleads.
What correct looks like: run the integer-ness check (582-595) only when `value_as_number` succeeded; collapse mutual-exclusion into the two "cannot carry" checks.
Confidence: HIGH — Verification: read (both branches traced for `min = "x"` on `type = "int"`).

### F-004 [S4]
Where: crates/settings-schema/src/parse.rs:596-609
Code:
```
                if min > max {
                    ctx.err( ... "range is inverted: min {min} > max {max}" ... );
                }
                Domain::Range(ValueRange { min, max })
            }
            _ => Domain::Unconstrained,
```
What is wrong: an inverted range still returns `Domain::Range` with `min > max` (a domain in which NO value can ever be valid), while the non-numeric-bound error path returns `Domain::Unconstrained` (a domain in which EVERY value is valid) — opposite fallbacks for the same failure class.
Why it matters: no live effect today (any diagnostic makes `parse_str` return Err, 61-65, so the poisoned Domain is discarded), but the inconsistency is a trap: `check_value` is `pub` (model.rs:342) and lenient on mismatched domains (`else { return Ok(()) }`), so any future caller that ignores diags inherits either a reject-all or accept-all domain depending on which typo the author made.
What correct looks like: return `Domain::Unconstrained` on the inverted branch too (or a `Domain::Invalid` variant), matching the sibling error path.
Confidence: HIGH — Verification: read (parse_str 61-65 confirms Err-on-any-diag, so no current consumer sees the poisoned value).

### F-005 [S4]
Where: crates/settings-schema/src/parse.rs:748-756, 819-833
Code:
```
            if let Some(header) = table_header(trimmed) {
                let mut path = header.segments;
                if header.is_array {
                    path.push(setting_count.to_string());
                    setting_count += 1;
                }
```
What is wrong: `LineIndex::new` scans line-by-line with no multi-line-string awareness. A line inside a `"""` value that looks like `[[setting]]` increments `setting_count`, desyncing the `setting.<i>` numbering from the real entries: every later entry's `line_of("setting.{i}")` (parse.rs:54) misses or hits the WRONG line, and diagnostics silently point elsewhere (fallback `unwrap_or(1)` = line 1). `strip_comment` (819-833) is likewise per-line: an escaped quote (`\"`) toggles `in_basic` off mid-string, though I traced that this cannot corrupt KEY attribution (keys precede the first `=` on the same line) — only the multi-line-string header spoof is a real desync.
Why it matters: wrong line numbers in diagnostics — the crate's product. Latent only: `rg -n '"""' schemas/settings.toml` → no matches, so the committed schema is unaffected; the embedded-schema runtime test (baseline `embedded_schema_parses_and_self_validates ... ok`) pins the committed file only.
What correct looks like: track `"""`/`'''` open/close state across lines in the scanner (a 4-line state machine), or have `setting_count` increment only on headers whose segments == ["setting"].
Confidence: MEDIUM — Verification: read + trace; desync path not executed (constructing it would require editing the schema, forbidden by read-only rule). REQUEST-ORCH-RUN not warranted (one-off unit test, low value).

### F-006 [S4]
Where: crates/settings-schema/src/parse.rs:620-628
Code:
```
        for member in members {
            if member.is_empty() || member.contains(char::is_whitespace) {
                ctx.err(
                    diags,
                    "E-DOMAIN",
                    "enumeration",
                    format!("enumeration member `{member}` must be non-empty and single-word"),
```
What is wrong: the error message promises "single-word" but the check only rejects whitespace. Members containing `,`, `|`, `<`, `>` pass validation and flow into consumers that format them positionally: emit.rs:69 `format!("<{}>", enum_members(def).join("|"))` (CLI `--help` placeholder — a member with `|` or `>` corrupts the `<a|b|c>` syntax) and emit.rs:112 / model.rs:391 `members.join(", ")` (a member with `,` makes "one of auto, of,f, off" unparseable to a human).
Why it matters: latent (committed enums are clean kebab tokens — schemas/settings.toml:64,80,101,175,214 all verified clean), but the grammar gate is weaker than its own error text, and the generated help text is the UI contract for CLI users.
What correct looks like: validate members with the same shape used elsewhere in this file — `is_kebab(member)` (665-673) — or at minimum reject `,`/`|`/`<`/`>`.
Confidence: HIGH — Verification: read (emit.rs consumers read at 60-130; committed enum members verified clean by grep).

### F-007 [S4]
Where: crates/settings-schema/src/parse.rs:847-992 (test module) vs 496-846 (production)
Code:
```
    #[test]
    fn line_index_maps_setting_fields() {
        let index = LineIndex::new(MINIMAL);
        assert_eq!(index.line_of("areas.known"), Some(3));
        assert_eq!(index.line_of("setting.0"), Some(5));
        assert_eq!(index.line_of("setting.0.key"), Some(6));
        assert_eq!(index.line_of("setting.0.cli"), Some(13));
```
What is wrong: the test module covers 3 of this chunk's ~16 production functions directly (`convert_domain`'s needs-range and Range-construction paths via tests 5-6, `LineIndex` via test 7, plus indirect predicate coverage through MINIMAL). Zero tests exercise: the inverted-range guard (598-605), non-numeric or half-present bounds (563-595 — the F-001/F-003 defects are unpinned), enum-member validation (610-639: empty list, empty member, whitespace member, duplicate member — 26 lines of checks with no test), range/enum mutual exclusion (504-513), `is_version` pre-release handling (709-717), `strip_comment`/`unquote`/`table_header`/`assignment_key` edges (775-845), `line_of_offset` (720-726).
Why it matters: untested code is unpinned code — F-001's wrong message and F-003's double diagnostic could not have survived a test each.
What correct looks like: table-driven tests per diagnostic rule, mirroring validate.rs's per-D-rule coverage (validate::tests covers D1-D5 individually — the same discipline applied to E-DOMAIN/E-TYPE is absent here).
Confidence: HIGH — Verification: read (test bodies 880-991 enumerated against production line ranges).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `convert_domain` | 496-643 | REAL | S3 (F-001, F-002), S4 (F-003, F-004) | Core domain checks correct for finite inputs: mutual exclusion, needs_range/needs_enum vs is_ranged (model.rs:175-180: Int/Float/DurationMs/Percent), integer-ness (is_integer_valued = Int/DurationMs), inverted range, enum empty/whitespace/duplicate. Defects: F-001 missing-bound wording, F-002 inf/nan acceptance, F-003 double diagnostics, F-004 inconsistent error fallback. `seen.contains` is O(n²) over enum members — n<10, immaterial. No panics; all `format!`-based. |
| `value_as_number` | 647-653 | REAL | S4 (i64→f64 `as` widening) | Widens TOML integers to f64; precision loss beyond 2^53 — no schema uses such bounds (schemas/settings.toml maxes at 2000-class values). Returns None for non-numeric — consumed by F-001/F-003 paths. |
| `render_value` | 656-661 | REAL | — | Diagnostic-only TOML rendering; quoted-string special case; reached from E-DEFAULT (parse.rs:350). |
| `is_kebab` | 665-673 | REAL | — | Matches `^[a-z0-9]+(-[a-z0-9]+)*$`: rejects empty, empty dash-parts, uppercase, non-ASCII. Hand-verified against regex claim in parse_areas error (196). |
| `is_setting_key` | 675-686 | REAL | — | Exactly two dot-separated kebab parts; `unwrap_or_default` on empty split is safe (empty → false via kebab check). Matches the regex in the E-FORMAT message (283-285). |
| `is_snake_path` | 688-697 | REAL | S4 | `>= 2` dot parts, per-part `[a-z0-9_]`; accepts underscore-only parts (`_.__`) — cosmetic. Double `split('.')` pass, trivial n. |
| `is_long_flag` | 699-702 | REAL | — | `--kebab`; rejects `--` alone, `-v`, `---a`, `a--b` (all hand-traced via is_kebab). |
| `is_short_flag` | 704-707 | REAL | — | Exactly one ASCII lowercase letter after `-`; rejects `-V`, `--v`, `v`. |
| `is_version` | 709-717 | REAL | S4 | Accepts `0.1.0`, `0.1.0-rc.1` (pre-release tail after first `-`/`+` is ignored wholesale — `0.1.0-@@@` passes); rejects 2-part and 4-part. Lenient, never rejects a sane version. |
| `line_of_offset` | 720-726 | REAL | — | Clamps offset to len (no panic), counts `\n` bytes (safe in UTF-8), 1-based. Used for TOML syntax-error line (parse.rs:37). |
| `LineIndex` (struct) | 732-734 | REAL | — | Vec<(path, line)>; linear `line_of` find — file is 1627 lines, immaterial. |
| `LineIndex::new` | 737-765 | REAL | S4 (F-005) | Correct for the committed schema (verified: multi-line arrays, comments, inline placement tables all map right — inline-table fields are not indexed and fall back to header_line via EntryCtx:137-141). Desyncs on multi-line strings (F-005). `setting_count` increments only on array headers; in serde-valid files those are only `[[setting]]` (deny_unknown_fields, RawFile 70-76) — sound given F-005's caveat. |
| `LineIndex::line_of` | 767-772 | REAL | — | First-match find; duplicate paths impossible in valid TOML (same key re-assignment is a serde error). |
| `Header` (struct) | 775-778 | REAL | — | Plain data. |
| `table_header` | 780-800 | REAL | — | Correct `[a.b]` / `[[a.b]]` dissection; malformed hybrids (`[[a]`, `[a]]`) either rejected or serde-fail upstream. `unquote`s quoted segments. |
| `assignment_key` | 802-817 | REAL | — | First-`=` split, rejects `[`-prefixed keys, splits dotted keys. Quoted keys containing `=` mis-split — but those are unknown fields → E-PARSE upstream, dead in practice. |
| `strip_comment` | 819-833 | REAL | S4 (F-005) | Correct for basic/literal strings without escapes; `\"` mis-toggles state (cannot corrupt key attribution — traced); no multi-line awareness (F-005). |
| `unquote` | 835-845 | REAL | — | Byte-index slicing on ASCII quote bytes is boundary-safe (no panic); leaves escapes (`"\""` → `\"`) — cosmetic, diagnostic paths only. |
| `const MINIMAL` | 851-866 | REAL | — | Valid 15-line schema fixture; line numbers verified against test 7's asserts. |
| `tests::diags_of` | 868-873 | REAL | — | Helper: Err-diags / Ok-empty. |
| `tests::rendered` | 875-877 | REAL | — | Helper: render_all for failure messages. |
| `tests::minimal_entry_parses` | 880-894 | REAL | — | Discriminates (any parse regression fails it); asserts areas, len, entry lookup. Baseline: ok. |
| `tests::missing_mandated_field_is_actionable_with_line` | 897-917 | REAL | — | Pins E-FIELD count, line Some(5) (header fallback), path `setting[0].description`. Traced correct: src lines 1-7, [[setting]] at 5. Baseline: ok. |
| `tests::unknown_field_is_rejected` | 920-931 | REAL | — | Pins `deny_unknown_fields` (typo'd `restart_requierd` → E-PARSE). Baseline: ok. |
| `tests::default_and_default_ref_both_is_d1` | 934-947 | REAL | — | Pins the D1 "both" arm (parse.rs:255-263, part1). Baseline: ok. |
| `tests::int_without_range_is_e_domain` | 950-963 | REAL | — | Pins THIS chunk's needs_range branch (516-527). Baseline: ok. |
| `tests::literal_default_out_of_range_is_e_default` | 966-982 | REAL | — | Pins this chunk's `Domain::Range` construction feeding check_value's V-RANGE (E-DEFAULT). Baseline: ok. |
| `tests::line_index_maps_setting_fields` | 985-991 | REAL | — | Direct LineIndex pin; all four asserts hand-verified against MINIMAL. Baseline: ok. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| minimal_entry_parses | YES | Any Err from parse_str renders a non-empty failure string → assert_eq!(failure, "") fails. |
| missing_mandated_field_is_actionable_with_line | YES | Removes or misnumbers the E-FIELD (count≠1), the header-line fallback (line≠Some(5)), or the path format → three independent asserts. |
| unknown_field_is_rejected | YES | Deleting `deny_unknown_fields` from RawSetting → parse succeeds → no E-PARSE → assert fails. |
| default_and_default_ref_both_is_d1 | YES | Deleting the (true,true) arm → no D1 "both" diag → fails. |
| int_without_range_is_e_domain | YES | Deleting needs_range check (516-527) → no matching E-DOMAIN → fails. |
| literal_default_out_of_range_is_e_default | YES | If convert_domain returned Unconstrained on the success path, check_value passes → no V-RANGE E-DEFAULT → fails. |
| line_index_maps_setting_fields | YES | Any off-by-one in setting_count/header/assignment indexing breaks one of four exact asserts. |
All seven executed green: baseline orchrun-test-workspace-nofailfast.log.txt:4879-4895. Gap: see F-007 — the WRONG-behavior branches (F-001..F-005) have no test at all.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "numeric settings are always bounded (doc 07 §5.2)" | parse.rs:522-523 (this chunk's own diagnostic), schemas/settings.toml:16-18 header | REAL for the committed schema; NOT ENFORCED for `-inf..inf` (F-002) | schemas/settings.toml every int/float/duration/percent entry carries finite bounds (spot-verified: tick-interval-ms 50..=2000 per runtime test msg); parser accepts inf/nan (trace, F-002). |
| "Line locations come from a small read-only scanner… header/assignment lines are indexed by walking the source once" | parse.rs:10-13 module doc | REAL with a documented blind spot (F-005) | LineIndex::new 737-765 does exactly this; multi-line strings unhandled (F-005); committed schema has none (`rg '"""'` → no matches). |
| "On any violation the FULL diagnostic list is returned (every problem, one pass)" | parse.rs:31-32 | REAL | parse_str 61-65 returns Err(diags) only with the full accumulated vec; convert_entry never early-returns on first error except the unknown-type bail (301) which still follows the mandated-field loop. Duplication noise noted (F-003). |
| "a second run is byte-identical" (schema-gen idempotence) | crates/settings-schema/Cargo.toml:14-15 | REAL-UNPROVEN (not re-run — cargo forbidden) | emit.rs:450 parse_str consumer + baseline emit tests `generation_is_deterministic ... ok` (no-failfast log 4883). |
| enum members are "single-word" | parse.rs:626 (this chunk's error text) | SHALLOW | Check rejects whitespace only (F-006); committed members are clean kebab tokens. |

## 7. Dead / duplicated / unreachable code
None. All 16 production functions in this chunk have live callers (verified: convert_domain←332; predicates←277/314/322/361/412/424; line_of_offset←37; LineIndex family←46/54/139/174-194). The `LineIndex`/`strip_comment` scanner duplicates the approach of product-config's `lines.rs` (module doc, line 10-12, admits this) — intentional parallel implementation, not flagged as a violation; boundaries.toml ownership was not re-checked here (W11 product-config agents' scope).

## 8. Dependencies & flags
`serde` (derive), `toml` v1, `thiserror` v2 — matches the crate's declared no-new-dependency policy (Cargo.toml:27-29). No feature flags in this crate; nothing cfg-gated except `#[cfg(test)]`. Workspace lints inherited (`[lints] workspace = true`); no `#[allow]` in this chunk. Baseline clippy CLEAN (INDEX.md #5/#6).

## 9. Open questions for the next agent
- `check_value` (model.rs:347-431) silently returns `Ok(())` when an Enum/Int type meets a mismatched Domain (`let Domain::Enumeration(..) = domain else { return Ok(()) }`). Unreachable via parse_str (any domain mismatch produces a diagnostic → Err), but it is a `pub` fn — W11-A07 (model.rs): is any external caller passing a hand-built Domain?
- validate.rs D1 (product.toml join) runs only in the generator/validator, not in the runtime embed (runtime.rs:57 passes `None`) — a `default_ref` whose product.toml target vanishes would embed fine and resolve to what at runtime? (W11-A08/A09: validate.rs / lib.rs:89 `load`.)
- F-002's inf/nan bound acceptance interacts with `fmt_bound` in emit.rs — what does `--help` print for `min = -inf`? (Untraced; low stakes.)
