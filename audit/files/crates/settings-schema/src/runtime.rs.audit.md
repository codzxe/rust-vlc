# AUDIT: crates/settings-schema/src/runtime.rs
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-23 |
| Lines / functions | 238 / 8 (embedded, validate_value, schema_meta, unknown_key_hint, edit_distance + 6 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — lumen-cli (release lane): apps/cli/src/host.rs:1371 `runtime::schema_meta` (config.get-setting), :1400-1402 `runtime::schema_meta` + `runtime::validate_value` (config.set-setting, mapped to ErrorCode::InvalidArgument); apps/cli/src/help.rs:157 `runtime::embedded()` (CLI help); also crates/plugin-host-wasm/src/supervisor.rs:123,171. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg for callers (exit 0, sites listed above); sed verifications (exit 0); baseline citation: cargo-test-workspace.txt settings-schema — 43 lib + 8 integration tests, `test result: ok`, package exit_code=0. |

## 1. One-paragraph verdict
This is the one config file in my assignment that is genuinely wired into the shipped binary and behaves exactly as documented: the schema is embedded at compile time via `include_str!`, parsed and self-validated (D2–D6; D1 deliberately deferred to the generator/validator, honestly documented at lines 6-8), cached in a `OnceLock`, and every `config.set-setting` write in the lumen-cli host goes through `validate_value` (type/range/enum/format → `invalid-argument`, never silent coercion — verified at the call site). The Levenshtein "did you mean" hint is a real byte-DP implementation, not a stub. Only hygiene defects: a misleading error mapping when the embedded schema itself is invalid, and a stale near-tautological count assertion in the self-test.

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt (settings-schema block): `running 43 tests ... test result: ok. 43 passed` + `tests/integration.rs ... 8 passed`; package exit_code=0. In-file tests include `embedded_schema_parses_and_self_validates` (the committed schemas/settings.toml must parse + self-validate) and domain rejection with exact V-ENUM/V-RANGE/V-TYPE texts.
- Call-site verification (read): host.rs:1399-1402 — `schema_meta` then `setting_value_to_toml(&value)` then `validate_value(&full, &toml_value)` → `CommandError::new(Stage::Config, ErrorCode::InvalidArgument, …)`. The doc 05 §2.11 "invalid-argument on violation, never silent coercion" claim holds at the seam.
- `setting_value_to_toml` (host.rs:1593-1605) is faithful (bool/int/float/string/list); the float-setting int-coercion case is safe because `check_value` for Float/Percent uses `value_as_f64` (model.rs:358-361), accepting both TOML Integer and Float.

## 3. Findings
F-001 [S4]
Where: crates/settings-schema/src/runtime.rs:74-77
Code:
```
    let schema = embedded().map_err(|err| ValueError::UnknownKey {
        key: key.to_owned(),
        hint: err.to_string(),
    })?;
```
What is wrong: when the EMBEDDED SCHEMA is invalid (a build-breaking state per the SchemaError doc), the user-visible error is `invalid-argument: unknown setting key \`X\`` with the schema failure stuffed into the hint. Wrong category: the key may be perfectly known; the schema is broken. Same mapping in `schema_meta` (91-94). Impact is minimal because the state is build-breaking and the hint carries the real message, but the error type lies.
Confidence: HIGH — Verification: read.
What correct looks like: a third ValueError variant (e.g. `SchemaBroken`) or propagating SchemaError.

F-002 [S4]
Where: crates/settings-schema/src/runtime.rs:154
Code:
```
        assert!(schema.len() >= 20, "Phase-1 seed has ~27 entries");
```
What is wrong: the comment says "~27 entries"; the committed schemas/settings.toml now defines 121 entries (ui/src/screens/settings/generated.test.ts:251 `expect(SETTINGS_MSGIDS).toHaveLength(121)`; my earlier count of `description = "settings.` in settings.toml = 121). The `>= 20` bound is satisfied by an order of magnitude — the count assertion is near-tautological and the comment is stale. The test's real value (parse + self-validate + engine area present) is unaffected.
Confidence: HIGH — Verification: read + rg count (121) + cross-ref generated.test.ts.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| EMBEDDED_SOURCE / EMBEDDED | 17-20 | REAL | — | include_str! of schemas/settings.toml + OnceLock cache. |
| SchemaError / ValueError | 23-51 | REAL | — | Typed; Display embeds the doc-05 `invalid-argument` surface. |
| embedded | 54-68 | REAL | — | parse_str + validate(&schema, None); D1 join deliberately excluded (doc lines 6-8). Err string cloned per call — negligible. |
| validate_value | 73-86 | REAL | S4 | F-001 error mapping; otherwise: entry lookup → check_value (type/range/enum/format). |
| schema_meta | 90-99 | REAL | S4 | F-001 (same mapping). |
| unknown_key_hint | 102-118 | REAL | — | Levenshtein ≤ 3, best candidate; fallback message names the regen command. O(entries × key-len²) per unknown key — fine at 121 entries. |
| edit_distance | 121-137 | REAL | — | Correct byte DP; verified logic incl. empty-string edges by trace; unit test asserts kitten/sitting=3. |
| tests (6) | 143-237 | REAL | S4 | F-002; others genuine (domain accept/reject with V-* texts, did-you-mean, schema_meta feature_gate/placement/ty). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| embedded_schema_parses_and_self_validates | YES | Any commit that breaks schemas/settings.toml self-validation fails the build-lane test (baseline passes on current file). Count assertion weak (F-002). |
| validate_value_accepts_in_domain_values / rejects_out_of_domain | YES | Exact V-ENUM/V-RANGE/V-TYPE texts + legal-value list — a validator that silently accepted "bogus" or mislabeled the violation fails. |
| unknown_key_gets_a_did_you_mean_hint | YES | Requires the Levenshtein machinery to actually find subtitle-mode from subtitle-mod. |
| schema_meta_backs_get_setting | YES | Pins feature_gate=hw-decode, screen, ty — a schema regression on that entry fails it. |
| edit_distance_is_symmetric_and_small_for_near_misses | YES (weak) | Only 3 literals; DP is exercised more meaningfully by the did-you-mean test. |
| Executed? | Baseline yes (43 lib tests incl. these; exit 0). Own run not attempted (sandbox linker fault — see log). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every config.set-setting-style write is validated against it (type/range/enum) before applying — invalid-argument on violation, never silent coercion (doc 05 §2.11)" | runtime.rs:1-4 | REAL | host.rs:1399-1402 wiring verified; tests assert invalid-argument text; baseline passes. |
| "The embedded copy is self-checked (parse + D2–D6) on first use" | runtime.rs:6-7 | REAL | embedded() 55-65 calls parse_str + validate; test embedded_schema_parses_and_self_validates. |
| "the D1 product.toml join … runs in the generator binary / the 1.E2 config-validator wiring, not at process start" | runtime.rs:6-8 | REAL-UNPROVEN | Honest deferral; generator = src/bin/schema_gen.rs (exists); the "1.E2 config-validator wiring" is the same unwired card as the l10n one (see my audit.rs F-001 — the wired xtask config-validator is a separate v0). |

## 7. Dead / duplicated / unreachable code
None. All five non-test functions have production callers (host.rs, help.rs, supervisor.rs).

## 8. Dependencies & flags
Crate: no internal deps (L1), toml + thiserror. No feature gates. settings-schema is depended on by plugin-host-wasm (Cargo.toml:53) and used path-wise by apps/cli (host/help) — check apps/cli Cargo.toml ownership if needed (not my file).

## 9. Open questions for the next agent
- commands/catalog/command.rs:614 marks `schema_meta` as a free-form `BTreeMap<String, String>` `[VERIFY]` — the strongly-typed SettingDef degrades to strings at the command boundary; who closes that?
