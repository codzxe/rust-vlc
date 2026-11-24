# AUDIT: crates/settings-schema/src/validate.rs
| Field | Value |
|---|---|
| Auditor | W11-A07 |
| Date | 2025-06-14 |
| Lines / functions | 772 / 11 prod fns + 4 const tables + 16 tests |
| File verdict | REAL (with S2 dead-namespace context) |
| Reachable from shipped entry point? | YES — lumen-cli `host.rs:1371/1400` -> `settings_schema::runtime::schema_meta`/`validate_value` -> `runtime.rs:57` calls `validate(&schema, None)` on first use. D1 product join + E-GATE run only in the `schema_gen` bin / xtask (dev lane, by design). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 2 |
| Commands run | `cargo test -p settings-schema --lib` -> 43 passed / 0 failed (0.03s); sed quote verification (7 blocks, all byte-verified) |

## 1. One-paragraph verdict
The D1–D6 + E-GATE rule engine is genuinely implemented and genuinely tested: 16 single-mutation tests each prove one rule fires (and only that rule), and the run passes. This is not a stub. The two S2s are about what the validation *guards*: (a) D3 polices a CLI flag namespace (`cli.flag` per setting) that the shipped lumen-cli argv parser never parses — flags are decorative help text; (b) the writes the runtime seam validates land in a BTreeMap that nothing but `setting get` ever reads — the settings round-trip is a validate→store→echo loop with zero behavioral effect.

## 2. What actually works (verified)
— verified by `cargo test -p settings-schema --lib` -> `test result: ok. 43 passed; 0 failed` including `validate::tests::{d2,d3×4,d4,d5×3,d6,d1×3,e_gate,literal_cross_area,reserved_tables,clean_fixture}` and `tests::committed_seed_validates_against_committed_product`.

## 3. Findings
F-001 [S2]
Where: crates/settings-schema/src/validate.rs:150-233 (D3) + apps/cli/src/parse.rs:154-169 + apps/cli/src/help.rs:123-127
Code (parse.rs — the consumer that does not exist):
```
    for (index, arg) in argv.iter().enumerate() {
        match arg.as_str() {
            "--json" => invocation.json = true,
            "--control-stdio" => invocation.control_stdio = true,
            "--version" | "-v" => invocation.version = true,
            "-h" | "--help" => {
```
and help.rs:123 advertises:
```
    out.push_str("\nSettings-derived flags (--<area>-<key> <value>):\n");
```
What is wrong: D3 enforces global uniqueness + reserved/subcommand disjointness for `cli.flag` values, but the shipped parser recognizes exactly six tokens (`--json`, `--control-stdio`, `--version`, `-v`, `-h`, `--help`); every settings-derived flag (e.g. `--engine-log-level debug`) falls into `command_words` and dies as `unknown command`. Why it matters: `lumen --help engine` prints flags the same binary refuses to accept — the D3 namespace is dead code dressed as a CLI contract; the only live consumers of `def.cli.flag` are help-text generation and the `setting get` meta map. What correct looks like: either parse_argv applies schema flags via `runtime::validate_value`, or help.rs/emit G2 stops advertising them as invocation flags.
Confidence: HIGH — Verification: read + ran (test run proves the schema side; parse path read end-to-end parse.rs:151-227, run.rs:73-146).

F-002 [S2] (settings round-trip — hostile-focus answer)
Where: apps/cli/src/host.rs:1390-1409 (the only shipped consumer of `validate_value`)
Code:
```
        settings_schema::runtime::validate_value(&full, &toml_value)
            .map_err(|err| invalid(err.to_string()))?;
        self.settings.insert(full, value.clone());
```
What is wrong: the validated write goes into `self.settings: BTreeMap` (host.rs:181) whose only reader in the entire CLI is `setting_get` (host.rs:1372). No engine, playback, decode, or UI path consults it — `engine.tick-interval-ms`, `playback.hw-decode-policy`, `privacy.telemetry` etc. change nothing after "apply". Why it matters: `config.set-setting` answers `SettingApplied` while the process behavior is unchanged — the round-trip validate.rs serves ends at an echo. What correct looks like: the host feeding validated values into the engine knobs they name, or the schema documenting that Phase-1 settings are inert. (Cross-file; flagged here because it defines validate.rs's real blast radius.)
Confidence: HIGH — Verification: read (rg `self.settings` in apps/cli/src -> exactly 4 hits: decl, init, get, insert).

F-003 [S3]
Where: crates/settings-schema/src/validate.rs:393-415
Code:
```
            DefaultValue::Literal(value) => {
                // [PROPOSAL heuristic: same file+area] — a literal default
                // that duplicates a product.toml value under the same area
                // should be a default_ref instead.
                let area = def.key.split('.').next().unwrap_or_default();
                for (path, leaf) in &leaves {
                    let leaf_area = path.split('.').next().unwrap_or_default();
                    if leaf_area == area && leaf == value {
```
What is wrong: value-equality + same-first-segment is the *entire* relatedness test. Two unrelated settings in one area that happen to share a common literal (`false`, `0`, `250`, `"auto"`) with any product.toml leaf collide, and the schema is rejected until a `default_ref` points at the unrelated leaf — which then type-checks against the wrong entry or D6-blocks. Why it matters: blocks legitimate schemas with a false error; the hint instructs a wrong fix. What correct looks like: comparing the setting's key tail against the leaf's key (e.g. `subtitle-mode` vs `subtitle_mode`), not raw value equality. Mitigation: it is labeled `[PROPOSAL heuristic]` and the committed seed passes today.
Confidence: HIGH (logic read; no false positive in the committed tree — verified by the passing seed test) — Verification: read + ran.

F-004 [S4]
Where: crates/settings-schema/src/validate.rs:131-141
Code:
```
                format!(
                    "duplicate setting key `{}` (first defined at line {first_line} as part \
                     of the `{first_key}` group) — setting keys are globally unique; rename \
                     one of the entries",
                    def.key
                ),
```
What is wrong: `first_key == def.key` by construction (that is how the duplicate matched), so the message reads "duplicate setting key `playback.one` (… as part of the `playback.one` group)" — there is no group concept anywhere in D2; the clause is copy-paste noise that misleads the schema author. What correct looks like: drop the "group" clause.
Confidence: HIGH — Verification: read (test d2_duplicate_key only asserts `contains("duplicate setting key")`, so the noise is untested).

F-005 [S4]
Where: crates/settings-schema/src/validate.rs:53-89 (RESERVED_CLI_SUBCOMMANDS) vs apps/cli/src/parse.rs:196-214
Code: the table lists `open … repeat` (doc 05 §5.3) but omits every host verb the shipped CLI actually dispatches: `commands`/`list-commands`, `advance`, `decode-bench`, `help`, `quit`/`q`.
What is wrong: D3's subcommand-collision guard is keyed to a doc table, not the real verb set; a future flag `--decode-bench` or `--advance` would pass D3 while naming a live verb. Today this is inert because of F-001 (no flag is parsed at all). Also `--generate-manifest` sits in RESERVED_CLI_FLAGS though no shipped binary has that flag (doc 07 G2 future work) — harmless over-reserving. Confidence: HIGH — Verification: read (table diffed against parse.rs match arms by hand).

## 4. Per-function table (mandatory, 772 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| RESERVED_CLI_FLAGS/SHORTS/SUBCOMMANDS | 36-89 | REAL | S4 | F-005: table drift vs real verbs; `--generate-manifest` unshipped |
| validate | 92-104 | REAL | — | Entry; product=None at runtime (runtime.rs:57) |
| entry_err | 106-119 | REAL | — | Uniform diagnostic shape |
| d2_keys_unique | 123-146 | REAL | S4 | F-004 message noise; O(n²) scan fine at n≈32 |
| d3_cli_unique_and_reserved | 150-233 | REAL | S2 | F-001: guards a namespace no parser consumes |
| d4_descriptions_unique | 237-259 | REAL | — | Correct linear-dup detection |
| d5_placements | 263-311 | REAL | — | Screen known + ui/advanced + order-unique all tested |
| d6_default_ref_slots | 315-339 | REAL | — | Correct; Ref-only by design |
| d1_product_join | 343-418 | REAL | S3 | F-003 value-equality false positives; dev-lane only |
| e_gate_features | 423-460 | REAL | — | Missing-[features] case handled |
| collect_scalar_leaves | 464-478 | REAL | — | Arrays treated as leaves (documented) |
| resolve_product_path | 481-487 | REAL | — | `?` chain, no panic |
| tests (16) | 489-772 | REAL | — | See §5 |

## 5. Test honesty (16 tests)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| clean_fixture_has_no_findings | YES | Asserts empty diag vec both with and without product |
| d2_duplicate_key_fails… | YES | Mutates one field; asserts len==1 + rule + message substrings |
| d3_duplicate_flag_fails | YES | len==1 + message |
| d3_reserved_flag_fails | YES | rule set must equal ["D3"] exactly |
| d3_subcommand_collision_fails | YES | asserts "subcommand" substring |
| d3_duplicate_short_fails | YES | rule set equality |
| d4_duplicate_description_fails | YES | len==1 + rule + "1:1 with entries" |
| d5_duplicate_order…fails | YES | len==1 + rule |
| d5_unknown_screen_fails | YES | len==2, all D5 |
| d5_ui_false_without_advanced_fails | YES | len==1 + message |
| d6_two_entries_same_default_ref_fails | YES | len==1 + rule (proves only D6 fires alongside D1 join) |
| d1_default_ref_must_exist_in_product | YES | len==1 + rule |
| d1_default_ref_type_mismatch_fails | YES | len==1 + "does not satisfy" |
| d1_same_area_literal_duplication…fires | YES | Filters D1, asserts exactly 1 |
| e_gate_unknown_feature_fails | YES | len==2 all E-GATE + "rule F1" |
| literal_cross_area_default_is_not_flagged | YES (inverse) | Guards the heuristic's area boundary |
| reserved_tables_are_the_doc_05_set | WEAK | Tautology-ish (contains-checks on own consts) — acceptable as a pin |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "D3 — CLI flags globally unique and disjoint from the fixed reserved-flag table and the doc 05 §5.3 subcommand words" | validate.rs:11-13 doc comment | REAL (as stated) but the namespace is consumerless | F-001; parse.rs:154-169 |
| "`--help` text for settings-derived flags is generated from the settings schema" | help.rs:4 (doc 05 §5.3) | REAL for the text, FALSE for the flags' existence | F-001: parser rejects every settings-derived flag |
| "every `config.set-setting`-style write is validated … before applying" | runtime.rs:2-4 | REAL for validation; "applying" = write-only store | F-002; host.rs:1404/1372 |
| D5 "ui = false entries must set advanced" | validate.rs:16-18 | REAL | d5_ui_false test + run |

## 7. Dead / duplicated / unreachable code
- d1_product_join + e_gate_features: unreachable from shipped binaries (runtime passes `product: None`, runtime.rs:6-8 says so); reachable via `cargo xtask`/schema_gen dev lane — by design, not a defect.
- The D3 flag namespace itself is functionally dead in the shipped CLI (F-001).

## 8. Dependencies & flags
No feature gates; crate deps: toml, thiserror (lib.rs re-exports only). No unsafe/unwrap/expect in this file (verified by read; workspace lint bans them).

## 9. Open questions for the next agent
- apps/cli/src/help.rs + parse.rs owner: F-001's fix location (advertising vs parsing) is a product decision.
- plugin-host-wasm/supervisor.rs also calls `runtime::validate_value` — does any *plugin* consumer make settings behavioral (would soften F-002)?
