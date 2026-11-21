# AUDIT: crates/settings-schema/src/parse.rs#part1 (lines 1–493)
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-21 |
| Lines / functions | 493 (of 992; chunk part1) / 12 rows (6 fns, 6 raw structs) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 4 S4: 2 |

Chunk boundary: part1 = lines 1–493 (module docs, `parse_str`, Raw* serde
shapes, `EntryCtx`, `parse_areas`, `convert_entry`). Part2 = lines 494–992
(`convert_domain`, format predicates, `LineIndex`, `mod tests`) — A10's chunk.
Tests (lines 848–992) live in part2 but exercise part1 functions; they are
assessed here as evidence only.

## 1. One-paragraph verdict

This is the two-stage schema parser for `schemas/settings.toml` and it is
genuine, careful, shipped code: `parse_str` feeds the typed model used by the
`lumen` CLI's live `config.get-setting`/`config.set-setting` seam, and a
baseline run proves it parses the committed 121-entry schema cleanly
(`orchrun-test-workspace-nofailfast.log.txt:4894
runtime::tests::embedded_schema_parses_and_self_validates ... ok`). The
diagnostic machinery (mandated-field presence, D1 default xor, format
predicates, per-field line addressing) is real and tested. The defects are
contract drift, not fakeness: the "FULL diagnostic list (every problem, one
pass)" promise is broken by an early-return `?` chain that suppresses all
semantic checks for an entry missing any mandated field; doc 07 §5.2's own
canonical entry example (`unit = "seconds"`) does not parse under
`deny_unknown_fields`; the "empty string / false where not applicable"
guidance is false for `cli.flag`/`key`/`description`/`since`; and
`deprecated_in` is a validation hole (no version check, unlike `since`).

## 2. What actually works (verified)

- Reachability chain (rg): `lumen` CLI grammar `config.get-setting`/
  `config.set-setting` (apps/cli/src/parse.rs:64-66) → dispatch.rs:149,152 →
  `Host::setting_get`/`setting_set` (apps/cli/src/host.rs:1366,1390) →
  `settings_schema::runtime::schema_meta`/`validate_value` (runtime.rs:47,59)
  → `runtime::embedded()` (runtime.rs:46) → `parse::parse_str` (this file,
  line 33). Secondary shipped path: apps/cli/src/help.rs:157
  (`runtime::embedded()` for `--help` areas). Also plugin-host-wasm
  supervisor.rs:123.
- Executed in baseline: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4894`
  `runtime::tests::embedded_schema_parses_and_self_validates ... ok` —
  `parse_str(EMBEDDED_SOURCE)` ran on the committed `schemas/settings.toml`
  (121 `[[setting]]` entries, 121 non-empty `flag =` lines, 24 `default_ref` +
  97 `default`) and returned Ok.
- parse::tests ×7 all ok in the same log (lines 4886-4893). NOTE: the primary
  `test-workspace.log.txt` aborted at package `desktop` and never ran this
  crate's tests — the no-fail-fast rerun is the only baseline evidence.
- Committed schema satisfies part1's own checks: `rg -c '^\[\[setting\]\]' schemas/settings.toml` → 121; `rg -n 'flag = ""'` → no hits (no empty
  flags, so F-003's contradiction is latent for the committed file, live for
  any author following the doc).

## 3. Findings

### F-001 [S3]
Where: crates/settings-schema/src/parse.rs:31-32 (contract), 265-273 (violation)
Code:
```rust
/// Parse and structurally validate schema source text. On any violation the
/// FULL diagnostic list is returned (every problem, one pass).
```
```rust
    let key = raw.key.clone()?;
    let type_tag = raw.r#type.clone()?;
    let description = raw.description.clone()?;
    let restart_required = raw.restart_required?;
    let raw_placement = raw.placement.as_ref()?;
    let raw_cli = raw.cli.as_ref()?;
    let feature_gate = raw.feature_gate.clone()?;
    let since = raw.since.clone()?;
```
What is wrong: `convert_entry` returns `None` at the first missing mandated
field, so for such an entry every downstream check never runs: format
predicates (key/description/feature_gate/since), `convert_domain`, the
default type check, placement.screen/section, cli.flag/short, scope,
array_policy. Only the 8 presence errors from the preceding loop are
reported. An entry missing `description` with `since = "soon"` reports the
missing field but not the bad `since`.
Why it matters: `parse_str`'s documented contract — and the stated purpose of
the two-stage design ("every problem, one pass", so schema authors fix
everything in one run) — is false. Realistic case: new entry missing one
mandated field plus a typo elsewhere → masked diagnostics, iterative
fix-re-run cycles. This is exactly the failure mode the module was built to
avoid.
What correct looks like: replace the `?` chain with `unwrap_or_default()`-style
extraction (or `if let` guards) that always runs every semantic check and
returns `None` only at the end when diags are non-empty.
Confidence: HIGH — Verification: read (control flow traced); tests in
baseline execute only the happy path for this (missing-field test has
otherwise-valid remaining fields).

### F-002 [S3]
Where: crates/settings-schema/src/parse.rs:84-103 vs docs/07-config-driven-system.md:546
Code:
```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSetting {
    key: Option<String>,
    r#type: Option<String>,
    range: Option<RawRange>,
```
What is wrong: doc 07 §5.2's canonical entry-format example includes
`unit = "seconds"  # display + CLI parsing hint [PROPOSAL]`, but `RawSetting`
has no `unit` field and `deny_unknown_fields` makes any `unit` key a hard
serde error surfaced as `E-PARSE` ("unknown field `unit`"). The documented
entry format does not parse.
Why it matters: §5.2 is the spec authors write `schemas/settings.toml`
against; copying the spec's own example verbatim produces a parse failure
with a misleading "TOML syntax error" label (see F-005). The committed schema
avoids `unit` only via a `[VERIFY] unit/range` comment (schemas/settings.toml:1079).
What correct looks like: either implement `unit` (even as an accepted-ignored
or stored field), or remove it from the doc 07 §5.2 example until it ships.
Confidence: HIGH — Verification: read (deny_unknown_fields semantics of serde
+ toml); `rg -n 'unit' schemas/settings.toml` shows no active use.

### F-003 [S3]
Where: crates/settings-schema/src/parse.rs:241-243 (guidance), 405-419 (contradiction), docs/07-config-driven-system.md:562
Code:
```rust
                    "missing mandated field `{field}` — every entry carries all ten doc 07 \
                     §5.2 fields; use an empty string / false where not applicable"
```
```rust
    if !is_long_flag(&cli.flag) {
        ctx.err(
            diags,
            "E-FORMAT",
            "cli.flag",
```
What is wrong: the E-FIELD guidance (and doc 07 §5.2's own "every entry
carries all of them (empty string / false where not applicable)") tells
authors to write empty strings for not-applicable fields, but an empty
`cli.flag` is REJECTED by `is_long_flag("") == false` → E-FORMAT "must be a
long flag `--kebab-case`". Same for `key` (is_setting_key rejects ""),
`description` (E-FORMAT "never empty", line 305), `since` (is_version("")
false). Only `feature_gate` may be empty. Additionally, a present `[cli]`
table with a MISSING `flag` member is reported as E-FORMAT (bad flag "") rather
than E-FIELD (missing mandated member), even though doc 07 §5.2 lists
`cli.flag` among the mandated fields.
Why it matters: the parser's own remediation advice is unactionable for 4 of
the 9 mandated slots; an author who follows it gets a second, contradictory
error. The committed schema dodges this only because all 121 entries carry
real flags.
What correct looks like: `is_long_flag` accepts empty (absent CLI exposure),
or the guidance names the fields where empty is legal; missing `flag` member
should be E-FIELD like screen/section.
Confidence: HIGH — Verification: read (`is_long_flag` =
`strip_prefix("--").unwrap_or("")` then `is_kebab`, which rejects "");
committed schema has zero empty flags.

### F-004 [S3]
Where: crates/settings-schema/src/parse.rs:327-334 (since), 488 (deprecated_in)
Code:
```rust
    if !is_version(&since) {
        ctx.err(
            diags,
            "E-FORMAT",
            "since",
            format!("since `{since}` must be a product version like `0.1.0`"),
        );
    }
```
```rust
        deprecated_in: raw.deprecated_in.clone().unwrap_or_default(),
```
What is wrong: `since` must be a version, but `deprecated_in` — the same kind
of field, rendered by emit.rs:130-131 as `[deprecated in {value}]` and by
emit.rs:378-379 in docs tables — accepts any string. `deprecated_in = "soon"`
or `"vNext"` passes parse and validate (rg confirms zero checks anywhere in
the crate) and flows into generated artifacts.
Why it matters: this file's entire job is validating the schema; leaving a
version-typed field unvalidated is an asymmetric hole that emits garbage into
G2/G3 generated output. Doc 07 §5.2:563 shows `deprecated_in = ""` "set when
retiring" — a version is implied by the marker semantics.
What correct looks like: `if !deprecated_in.is_empty() && !is_version(&deprecated_in)` → E-FORMAT, mirroring `since`.
Confidence: HIGH — Verification: read + rg (`deprecated_in` has no other
check in crates/settings-schema).

### F-005 [S4]
Where: crates/settings-schema/src/parse.rs:36-43
Code:
```rust
        Err(err) => {
            let line = err.span().map(|span| line_of_offset(src, span.start));
            return Err(vec![SchemaDiagnostic::error(
                "E-PARSE",
                line,
                "<file>",
                format!("TOML syntax error: {err}"),
            )]);
        }
```
What is wrong: every `toml::from_str::<RawFile>` failure — including serde
DATA errors (wrong field type, unknown field via `deny_unknown_fields`) — is
labeled "TOML syntax error". A wrong-typed field is not a syntax error; the
label misleads the schema author.
Why it matters: diagnostic honesty is this crate's product (doc 07 §5.5's
actionable-diagnostics promise); the `unknown_field_is_rejected` test even
asserts this mislabeled rule.
What correct looks like: branch on error kind (toml distinguishes syntax vs
data errors) or label it "TOML/serde error".
Confidence: HIGH — Verification: read; baseline test 4893 confirms the
E-PARSE path for a data error (unknown field), i.e. the mislabel is exercised.

### F-006 [S4]
Where: crates/settings-schema/src/parse.rs:52-56
Code:
```rust
    for (i, entry) in raw.setting.iter().enumerate() {
        let ctx = EntryCtx {
            index: &index,
            header_line: index.line_of(&format!("setting.{i}")).unwrap_or(1),
            i,
        };
```
What is wrong: if the `LineIndex` scanner fails to key `setting.{i}` (its
mechanics are part2/A10), the header line silently fabricates line 1, and
`EntryCtx::field_line` then inherits that wrong anchor for every field of the
entry. A diagnostic could point at line 1 of the file.
Why it matters: line accuracy is the documented feature (module doc: "the
entry's line, not a bare serde error"); a silent wrong line is worse than no
line for a reader trusting it.
What correct looks like: `Option<u32>` line, or a fallback that scans
forward for the nth `[[setting]]` header, or a visible marker.
Confidence: MEDIUM — Verification: read; the fallback branch was not
exercised by any baseline test (scanner happy path only).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_str` | 33-66 | REAL | S4 (F-005, F-006) | Two-stage entry; toml error path mislabels data errors as syntax (F-005); `unwrap_or(1)` header fallback (F-006). Returns Ok only when diags empty — correct. No unwrap/expect/panic (workspace lints deny). Baseline-executed on real schema. |
| `RawFile` (struct) | 70-76 | REAL | — | `areas: Option`, `setting` `#[serde(default)]` — an empty (zero-setting) schema parses Ok; not clearly a defect (areas must be non-empty). `deny_unknown_fields` active. |
| `RawAreas` (struct) | 78-82 | REAL | — | Only `known`; extra keys → E-PARSE. |
| `RawSetting` (struct) | 84-103 | REAL | S3 (F-002) | 16 optional fields, `deny_unknown_fields`; no `unit` field although doc 07 §5.2 example has one (F-002). |
| `RawRange` (struct) | 105-110 | REAL | — | min/max as raw `toml::Value`; number/integer checks live in part2 `convert_domain`. |
| `RawPlacement` (struct) | 112-119 | REAL | — | screen/section/order/ui; `ui` defaults true at consumption (line 388). |
| `RawCli` (struct) | 121-126 | REAL | — | flag/short optional strings. |
| `EntryCtx::field_line` | 137-141 | REAL | — | `unwrap_or(header_line)` fallback; sound. |
| `EntryCtx::field_path` | 143-145 | REAL | — | `setting[i].field` format; trivial. |
| `EntryCtx::err` | 147-154 | REAL | — | Pushes error diag with field line+path; trivial, correct. |
| `parse_areas` | 159-210 | REAL | S4 (perf note only) | Missing `[areas]`/`known`/empty list/kebab/duplicate all diagnosed; returns `known.clone()` even when diags fired (harmless — parse_str discards on Err). `seen.contains` is O(n²) — n=14 areas, irrelevant. Verified: committed file's 14 areas pass (baseline embedded test). |
| `convert_entry` | 216-492 | REAL | S3 (F-001, F-003, F-004) | The core. Presence loop (8 fields) good; `?` chain breaks the full-diagnostics contract (F-001); `empty_ok` Path/Url carve-out is documented (w26 Lead comment) and consistent with model.rs `check_value` strictness split; scope/array_policy unknown-value errors fall back to defaults while still diagnosing (correct: entry built, parse fails anyway); `deprecated_in` unvalidated (F-004). Baseline tests cover D1 both-arms, unknown type, default-out-of-range. |

## 5. Test honesty (tests physically in part2, lines 848–992 — assessed for part1 coverage)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `minimal_entry_parses` | YES | Mutation: `convert_entry` always returns None → `parse_str` Err → `assert_eq!(failure, "")` fails. Ran ok in baseline (4889). |
| `missing_mandated_field_is_actionable_with_line` | YES | Asserts exactly one E-FIELD mentioning `description`, line `Some(5)` (header fallback), path `setting[0].description`. Removing the presence loop or breaking field_line fails it. Baseline 4891. |
| `unknown_field_is_rejected` | YES (weakly) | Asserts any E-PARSE exists for `restart_requierd`. Removing `deny_unknown_fields` fails it; but it cannot distinguish mislabeled rules (F-005). Baseline 4893. |
| `default_and_default_ref_both_is_d1` | YES | Asserts D1 rule + "both" message; removing the `(true, true)` arm fails it. Baseline 4886. |
| `int_without_range_is_e_domain` | PARTLY (part2) | Exercises `convert_domain` (A10) reached via part1's call at line 337. |
| `literal_default_out_of_range_is_e_default` | YES | Exercises part1's default arm (lines 348-368) calling `check_value`; deleting that arm fails it. Baseline 4890. |
| `line_index_maps_setting_fields` | NO (part1) | Pure `LineIndex` (part2) — out of my chunk. |
| Gap | — | No test covers F-001's suppression (missing field + second semantic error), F-003's empty `cli.flag`, or F-004's `deprecated_in`. |

## 6. Doc claims touching this file

| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "On any violation the FULL diagnostic list is returned (every problem, one pass)" | parse.rs:31-32 (code doc) | FALSE | F-001: `?` chain at 265-273 suppresses all semantic checks after first missing mandated field. |
| "every entry carries all ten mandated fields … (empty string / `false` where not applicable)" | docs/07-config-driven-system.md §5.2:561-562 | FALSE (for cli.flag/key/description/since) | is_long_flag("")→E-FORMAT (405-419); description "never empty" (305-310); since is_version (327-334). Committed schema: 121/121 non-empty flags. |
| `unit = "seconds"` is part of the entry format | docs/07-config-driven-system.md §5.2:546 | PAPER-ONLY / FALSE | RawSetting (84-103) has no `unit`; `deny_unknown_fields` → E-PARSE. (F-002) |
| "`feature_gate` … absent = ungated" | docs/07-config-driven-system.md §5.2:557 | FALSE | Absent → E-FIELD missing mandated field (226-246); only `""` is ungated (316-323). Doc contradicts its own "every entry carries all" line. |
| D1: default xor default_ref, "exactly one is required" | docs/07 §5.2:565 + parse.rs:248-262 | REAL | Code arms `(false,false)`/`(true,true)` both diagnose; baseline test 4886 ok. |
| "missing mandated field becomes *our* diagnostic with the entry's line" | parse.rs module doc:5-6 | REAL | Baseline 4891 (line 5 header fallback asserted). Field-level lines only when the field key exists (LineIndex, part2). |
| "[areas].known … at least one screen id" (D5 seed) | parse.rs:186-191 | REAL | Committed file has 14 areas; embedded test 4894 ok. |

## 7. Dead / duplicated / unreachable code

None in part1. All 16 `RawSetting` fields are consumed by `convert_entry`;
all functions are called (`parse_areas`/`convert_entry` only from `parse_str`,
which is called from runtime.rs:55, lib.rs:89, plus tests). Duplication note:
`is_kebab`/line-scanning approach mirrors product-config `lines.rs` by design
(module doc, cross-crate duplication accepted and documented).

## 8. Dependencies & flags

Crate `settings-schema` (Cargo.toml): deps serde+toml+thiserror only — no
feature flags, no cfg-gating in part1. No unsafe. Workspace lints inherited
(`unwrap_used`/`expect_used`/`panic` deny — part1 complies; zero
unwrap/expect/panic in lines 1-493).

## 9. Open questions for the next agent

- A10 (part2): does `LineIndex` ever miss `setting.{i}` keys (multi-line
  arrays, inline tables spanning lines)? F-006's `unwrap_or(1)` fallback
  depends on it. `placement = { … }` on one line works (test asserts
  `setting.0.cli` line), but multi-line `placement` would break field lines.
- A01 (validate.rs): `deprecated_in` is unvalidated there too — F-004 stands
  crate-wide; consider whether D-rules should own it.
- A03 (model.rs): `check_value` rejects empty Path strings at runtime while
  the schema default carve-out (`empty_ok`, lines 339-368) allows them —
  verify `runtime::validate_value` callers never feed the literal default ""
  back through `check_value` (host.rs `default_value()` path).
