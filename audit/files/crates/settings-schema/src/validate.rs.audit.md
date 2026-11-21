# AUDIT: crates/settings-schema/src/validate.rs
| Field | Value |
|---|---|
| Auditor | W11-A01 |
| Date | 2026-11-21 |
| Lines / functions | 772 / 14 (11 fns + 3 pub consts) + 17 tests + 4 test helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI apps/cli → settings-schema dep (apps/cli/Cargo.toml:61) → host.rs:1402 `settings_schema::runtime::validate_value` → runtime.rs:57 `validate(&schema, None)` — D2–D6 subset; full D1 join via CI `cargo run -p xtask --locked -- schema-gen --check`, ci.yml:277-278) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `rg -n "validate\(" crates apps tools` (callers) · `rg -n "RESERVED_CLI" crates` (const usage — validate.rs only) · `rg -n "validate_value|schema_meta" -g '*.rs'` (host.rs:1371/1400/1402, plugin-host-wasm/supervisor.rs:171) · `rg -n "settings|D1|D2|…" tools/xtask/src/cmd/config_validator.rs` (zero hits — no D-rule code there) · `sed -n` on all quoted ranges (verbatim verification) · `rg -n '^\[\[\]\]|^key = ' schemas/settings.toml` + `sed -n '690,820p'` (playback-area bool literals) · `rg -n " = true| = false" product.toml` ([playback] booleans) · `rg -n "owner" schemas/generated/settings.d.ts` (zero hits — D6 owner table absent) · `ls layouts/` (does not exist) · `sed -n '268,285p' .github/workflows/ci.yml` (schema-gen --check step) |

## 1. One-paragraph verdict
This is a real, working validator: D2–D6 uniqueness/reserved-word checks plus the D1 `product.toml` join (ref existence, ref type/domain check, same-area literal-duplication heuristic) and the E-GATE feature-id check, all producing structured diagnostics, all covered by genuine mutation tests that pass in the baseline run. It is reachable both from the shipped `lumen` binary (runtime embedded self-check, product-less) and from CI (full join via `schema-gen --check`). The defects are: the D1 literal-duplication heuristic matches on bare value equality within an area and is one boolean flip away from spraying false errors at four unrelated `playback.*` settings while simultaneously missing real cross-area duplication (S3); D6 is shipped as a much weaker surrogate of what doc 07 §5.2 D6 specifies (no engine runtime-slot owner table exists anywhere) (S3); doc 05 §5.3 misattributes D3 enforcement to `config-validator`, which contains no D-rule code at all (S3); plus two S4 diagnostic/comment quality defects.

## 2. What actually works (verified)
— D2–D6 + E-GATE all execute and fire with rule-tagged, line-numbered diagnostics: verified by the file's own 17 mutation tests (each mutates one thing, asserts exact rule + message fragment), which passed in baseline run #3 `cargo test --workspace --locked` → 1681 passed / 1 failed / 24 ignored, the single failure being `apps/desktop/tests/zero_alloc_transport.rs:98` (unrelated crate; audit/runs/baseline/INDEX.md #3).
— The committed seed + committed product.toml round-trip the full pipeline (parse + D1 join + D2–D6) with zero findings: `committed_seed_validates_against_committed_product` (lib.rs) passed in the same baseline run.
— CI enforces the full D1 join: `.github/workflows/ci.yml:277-278` — step "schemas/settings.toml D1-D6 + generated-output drift (doc 07 §5.2, §9 step 5)" runs `cargo run -p xtask --locked -- schema-gen --check` → `tools/xtask/src/cmd/schema_gen.rs` shells to the crate binary → `settings_schema::load()` (lib.rs:90) → this file's `validate()`.
— Runtime seam is wired: `apps/cli/src/host.rs:1400,1402` call `settings_schema::runtime::{schema_meta,validate_value}` which self-check the embedded schema via this file's `validate(&schema, None)` (runtime.rs:57) — reachable from the shipped `lumen` CLI.

## 3. Findings

### F-001 [S3]
Where: crates/settings-schema/src/validate.rs:393-415
Code:
```rust
            DefaultValue::Literal(value) => {
                // [PROPOSAL heuristic: same file+area] — a literal default
                // that duplicates a product.toml value under the same area
                // should be a default_ref instead.
                let area = def.key.split('.').next().unwrap_or_default();
                for (path, leaf) in &leaves {
                    let leaf_area = path.split('.').next().unwrap_or_default();
                    if leaf_area == area && leaf == value {
```
What is wrong: The D1 literal-duplication heuristic fires on bare TOML value equality (`leaf == value`) between any setting literal and any `product.toml` leaf whose first path segment equals the setting key's first segment. It matches nothing else — not the key name, not semantics. Two failure modes: (a) false positives one edit away: `schemas/settings.toml` carries four `playback.*` bool literals `default = true` (`playback.pause-on-audio-focus` :749, `playback.shuffle` :772, `playback.stop-after-each-item` :783, `playback.seek-precisely` :806) and `product.toml [playback]` currently has only `start_paused = false` (:192); adding ANY `[playback] x = true` boolean (or flipping `start_paused` to `true`) makes all four fail D1 with "use `default_ref = \"playback.x\"`" — an instruction that is semantically wrong for all four. (b) false negatives today: `privacy.telemetry`/`privacy.usage-stats` literal bools duplicate `[telemetry] enabled/usage_stats` in product.toml semantically, but the areas are named "privacy" vs "telemetry", so no fire — the exact duplication D1 exists to catch is unflagged whenever the two files name areas differently.
Why it matters: This is the validator CI trusts to keep "product-level defaults stated once" true. A heuristic that both sprays false errors after innocent edits and misses the real duplication it was written for gives false assurance in the direction that matters (misses) and friction in the other (false hits). The error path is worse than the miss: four build-breaking errors telling the developer to create a wrong reference.
What correct looks like: Compare on (area, key-name) similarity — e.g. flag only when the setting's trailing key segment normalizes (kebab→snake) to the leaf's final path segment, or require the value AND the normalized key to match; that keeps `playback.subtitle-mode` ↔ `playback.subtitle_mode` firing while `playback.shuffle` vs a future `playback.loop = true` does not. Cross-area name-equal duplication (privacy.telemetry vs telemetry.enabled) deserves at least a warning-severity diagnostic (`Severity::Warning` exists but is never produced).
Confidence: HIGH — Verification: read; seed/product values verified by `sed -n '690,820p' schemas/settings.toml`, `rg -n " = true| = false" product.toml`; no current fire confirmed by baseline run #3 (lib test passed).
Severity note: S3, not S2 — latent (no current misfire) and dev-tool blast radius.

### F-002 [S3]
Where: docs/05-command-layer.md:533 (§5.3, "Settings surface as generated long flags…" paragraph) — claim touching this file's D3
Code (doc quote, verified by read):
```text
Short flags are a fixed, documented table; collisions with generated flags are
rejected at build time by `cargo xtask config-validator`.
```
What is wrong: `tools/xtask/src/cmd/config_validator.rs` (1007 lines) contains zero settings-schema code — `rg -n "settings|D1|D2|D3|schema-gen" tools/xtask/src/cmd/config_validator.rs` returns nothing; it validates only `product.toml` vs `product.schema.json` plus its listed cross-file checks. Worse, no CI workflow or gate script runs `cargo xtask config-validator` at all (rg over `.github/workflows/ci.yml` and `scripts/full_gate.sh`: zero hits; CI's product.toml coverage is `cargo test -p product-config` at ci.yml:276, and the tool itself currently exits RED on unrelated brand-icon violations — baseline #10). The D3 flag-collision rejection actually lives in this file and is executed by `cargo xtask schema-gen --check` (ci.yml:277-278 names it correctly: "schemas/settings.toml D1-D6 …") and by the runtime embedded self-check.
Why it matters: A developer who trusts doc 05 and runs `cargo xtask config-validator` to check flag collisions gets zero D3 coverage from it (worse: that tool currently exits RED for unrelated brand-icon reasons — baseline #10). Doc 07 §7-C4's claim that config-validator covers "settings schema D-rules" (docs/07 line 835) repeats the misattribution. The enforcement exists; the map to it is wrong in two docs.
What correct looks like: Both docs name `cargo xtask schema-gen --check` as the D1–D6 gate (doc 07's own CI step already says this); config-validator's documented scope stays product.toml-only.
Confidence: HIGH — Verification: read (rg over config_validator.rs zero hits; ci.yml:277-278 read verbatim).

### F-003 [S3]
Where: crates/settings-schema/src/validate.rs:315-339 (d6_default_ref_slots) vs docs/07-config-driven-system.md:574
Code:
```rust
fn d6_default_ref_slots(schema: &SettingsSchema, diags: &mut Vec<SchemaDiagnostic>) {
    let mut seen: Vec<(&str, u32)> = Vec::new();
    for def in &schema.settings {
        let DefaultValue::Ref(reference) = &def.default else {
            continue;
        };
```
Doc (docs/07:574, verbatim): "**D6**: no two entries may write the same engine runtime slot (validator cross-checks the `owner` mapping in the generated table) `[PROPOSAL]`."
What is wrong: The implemented D6 checks only that two entries do not share a `default_ref` path. The documented mechanism — cross-checking an `owner` mapping in the generated table — does not exist: `rg -n "owner" schemas/generated/settings.d.ts` → zero hits; no generated artifact carries owner mappings, and the module header itself admits "The engine runtime-slot owner table arrives with the settings framework (card 4.E1) `[PROPOSAL]`" (validate.rs:21-22). Consequence: two settings whose *literal* defaults both feed the same engine runtime slot (exactly the class D6 is written to prevent) pass validation today.
Why it matters: D6 is one of the six "single-definition rules" the docs sell as enforced; the shipped check covers only the ref subset of the problem. Combined with F-001's cross-area miss, the "one entry per product decision" invariant is enforced only for `default_ref` users.
What correct looks like: Either the owner table exists (generated or in `[areas]`-style data) and d6 cross-checks it, or doc 07's D6 text states the Phase-1 surrogate explicitly (as the module header already does — the doc should match the header, not overclaim).
Confidence: HIGH — Verification: read; rg for owner mapping in generated artifacts = zero hits; layouts/settings-areas.toml does not exist (`ls layouts/` → no such directory).
Severity note: S3 not S2 — the gap is documented as [PROPOSAL]/pending card 4.E1, and the runtime-slot collision class cannot currently arise (settings writes route through the schema key, which D2 already makes unique).

### F-004 [S4]
Where: crates/settings-schema/src/validate.rs:136-140
Code:
```rust
                    "duplicate setting key `{}` (first defined at line {first_line} as part \
                     of the `{first_key}` group) — setting keys are globally unique; rename \
                     one of the entries",
```
What is wrong: There is no "group" concept anywhere in the model (model.rs keys are flat dotted ids; areas are placement screens, not key groups), and `first_key` is by construction identical to `def.key` (it was found by matching `*key == def.key.as_str()`, line 128). The message tells the reader the first entry was defined "as part of the `playback.one` group" — meaningless text for a duplicate-key error.
Why it matters: The crate's contract (diagnostic.rs header) promises "a message that states the constraint, what was found and how to fix it"; this one injects a fiction. Small blast radius (diagnostic text only).
What correct looks like: "duplicate setting key `{key}` (first defined at line {first_line}) — setting keys are globally unique; rename one of the entries".
Confidence: HIGH — Verification: read (d2_keys_unique matching logic, lines 123-145).

### F-005 [S4]
Where: crates/settings-schema/src/validate.rs:420-422
Code:
```rust
/// E-GATE: a nonempty feature_gate must be a feature id that product.toml
/// [features] actually declares (rule F1 — you cannot even name an
/// unimplemented feature, doc 07 §2.7).
```
What is wrong: The parenthetical overstates the check. The code (lines 428-458) verifies only that the id is *declared* in `[features]`. Naming a declared-but-unimplemented feature passes: `product.toml:168` `hw-decode = false` ("implementation is Phase 7 (7.B)") legally gates `playback.hw-decode-policy` (settings.toml:1175, `feature_gate = "hw-decode"`). "Unimplemented" detection is rule F2, which nothing implements (config_validator.rs header: "Rule F1/F2 … needs the registry surface; v0 checks that every features.* value is boolean").
Why it matters: The comment (and doc 07 §2.7's "you cannot even name an unimplemented feature") reads as an implemented guarantee; the actual guarantee is name-must-be-declared. Doc-vs-comment-vs-code three-way drift on an exposure rule.
What correct looks like: Comment says "must be an id product.toml [features] declares (declared ≠ implemented — that is F2, unimplemented)". Doc 07 §2.7 marks the unimplemented-feature rejection as F2-pending.
Confidence: HIGH — Verification: read; product.toml:168 and settings.toml:1175 read; config_validator.rs header read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `RESERVED_CLI_FLAGS` (const) | 36-44 | REAL | — | 7 entries; matches doc 05 §5.3 machine-mode flags + `--generate-manifest` (doc 07 §5.3 G2). Used only in this file + tests; `pub` API, not dead. |
| `RESERVED_CLI_SHORTS` (const) | 48 | REAL | — | `-h`/`-v` only; matches doc 05's "fixed, documented table" (exactly these two universal shorts). |
| `RESERVED_CLI_SUBCOMMANDS` (const) | 53-89 | REAL | — | 34 words; verified 1:1 against doc 05 §5.3 "Top-30 subcommand mapping" table (open…log-level, incl. shuffle/repeat/setting/engine-info). "Top-30" with 34 rows is the doc's own miscount; code matches the table. |
| `validate` | 92-104 | REAL | — | Entry point; skips D1+E-GATE when `product=None` (runtime path). Callers verified: lib.rs:90, runtime.rs:57, tests. |
| `entry_err` | 106-119 | REAL | — | Trivial helper; correct rule/line/path/message shape per diagnostic.rs contract. |
| `d2_keys_unique` | 123-146 | REAL | S4 | O(n²) scan (fine at n≈130 entries); message wording defect → F-004. |
| `d3_cli_unique_and_reserved` | 150-233 | REAL | — | Duplicate long flag, reserved long flag, subcommand-word collision (via `trim_start_matches('-')`), duplicate short, reserved short. Correct; a duplicated flag can emit 2 errors (dup + reserved) — acceptable noise. Flag-format enforcement ("--" prefix) is parse.rs's job (A09), not re-checked here. |
| `d4_descriptions_unique` | 237-259 | REAL | — | Same O(n²) pattern; correct. |
| `d5_placements` | 263-311 | REAL | — | Screen-known / ui-false-requires-advanced / order-unique-per-(screen,section). Checks `[areas].known`, not layouts/settings-areas.toml (doc 07 §5.2) — that file does not exist; module header + message disclose the interim. |
| `d6_default_ref_slots` | 315-339 | SHALLOW | S3 | Only ref-path duplication; documented owner-table cross-check absent → F-003. |
| `d1_product_join` | 343-418 | REAL | S3 | Ref existence + table-target rejection + type/domain check via shared `check_value` (good: same rule as runtime). Literal-dup heuristic → F-001. `leaves` computed once; O(entries×leaves) fine at seed scale. |
| `e_gate_features` | 423-460 | REAL | S4 | Declared-id check correct; comment overstatement → F-005. Missing `[features]` table emits one error per gated entry (noise, acceptable). |
| `collect_scalar_leaves` | 464-478 | REAL | — | Doc comment says "scalars and scalar arrays"; implementation pushes any non-table (incl. arrays-of-tables). Harmless (no setting has area "migrations"); noted here, not filed. |
| `resolve_product_path` | 481-487 | REAL | — | Table-only walk; array-element refs (`a.0.b`) return None → "does not exist" error. Acceptable (no such refs in seed). |
| test `fixture` | 495-508 | REAL | — | Two-entry clean fixture builder. |
| test `parse_fixture` | 510-525 | REAL | — | Fails loudly on fixture drift (asserts empty render of parse diags). |
| test `parse_product` | 527-531 | REAL | — | `.ok()` swallow is test-only; a broken PRODUCT const would surface through the exact-count asserts of the d1/d6 tests. |
| test `validate_fixture` | 533-536 | REAL | — | Harness. |
| test `clean_fixture_has_no_findings` | 549-560 | REAL | — | Both None and product-joined clean paths. |
| test `d2_duplicate_key_fails_with_rule_and_first_line` | 562-576 | REAL | — | Exact count 1 + rule + message fragments. |
| test `d3_duplicate_flag_fails` | 578-584 | REAL | — | Exact-count assert. |
| test `d3_reserved_flag_fails` | 586-596 | REAL | — | Asserts rule list == ["D3"]. |
| test `d3_subcommand_collision_fails` | 598-608 | REAL | — | `--seek` body collision. |
| test `d3_duplicate_short_fails` | 610-628 | REAL | — | Both shorts `-x`; asserts exactly ["D3"]. |
| test `d4_duplicate_description_fails` | 630-640 | REAL | — | Exact count 1. |
| test `d5_duplicate_order_within_screen_section_fails` | 642-649 | REAL | — | Exact count 1 + message. |
| test `d5_unknown_screen_fails` | 651-658 | REAL | — | Both entries flagged (2 diags) — correctly anticipated by the comment. |
| test `d5_ui_false_without_advanced_fails` | 660-670 | REAL | — | Exact count 1. |
| test `d6_two_entries_same_default_ref_fails` | 672-694 | REAL | — | Enum type swap; only D6 fires (D1 type-check passes) — exact count 1. |
| test `d1_default_ref_must_exist_in_product` | 696-707 | REAL | — | Exact count 1 + "does not exist". |
| test `d1_default_ref_type_mismatch_fails` | 709-721 | REAL | — | bool entry vs string product value → V-TYPE via D1. |
| test `d1_same_area_literal_duplication_heuristic_fires` | 723-742 | REAL | — | The one test of the heuristic's positive side; message names `playback.subtitle_mode` and `default_ref`. |
| test `e_gate_unknown_feature_fails` | 744-752 | REAL | — | Both entries gated → 2 diags; `.replace` replaces all occurrences and the test knows (comment "both entries flagged"). |
| test `literal_cross_area_default_is_not_flagged` | 754-764 | REAL | — | Documents the cross-area blind spot as *intended* (privacy literals) — i.e. F-001's false-negative side is a tested, deliberate behavior. |
| test `reserved_tables_are_the_doc_05_set` | 766-771 | REAL | — | Spot-checks 3 members of the 34-word list (acceptable, the list itself was hand-verified against doc 05). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 17 tests | YES | Each mutates exactly one aspect of a clean fixture and asserts exact diagnostic count + rule id + message fragment, through the real `parse_str → validate` pipeline (no mocks). Mutation reasoning (concrete): delete the `seen.contains` branch in d5 → `d5_duplicate_order_within_screen_section_fails` gets 0 diags → `assert_eq!(diags.len(), 1)` panics. Delete `RESERVED_CLI_SUBCOMMANDS.contains` check → `d3_subcommand_collision_fails` fails its `any()` assert. Remove the type-check arm in d1 → `d1_default_ref_type_mismatch_fails` count assert fails. |
| `clean_fixture_has_no_findings` + `literal_cross_area_default_is_not_flagged` | YES (inverted) | These guard against over-firing; a heuristic widened to cross-area would fail the latter with a stray D1 diag. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| D1: "validator fails on a literal default that duplicates a product.toml value it should reference [PROPOSAL heuristic: same file+area]" | docs/07 §5.2 D1 (:569) | REAL | d1_product_join:393-415 implements exactly this; heuristic limits → F-001. |
| D2 keys globally unique / D3 flags unique + disjoint from short table and top-30 subcommands / D4 msgids 1:1 / D5 orders unique per (screen,section), screen must exist | docs/07 §5.2 (:570-573) | REAL | d2/d3/d4/d5 implementations; D5 checks `[areas].known` not layouts/settings-areas.toml (file absent — interim disclosed in module header). |
| D6: "validator cross-checks the `owner` mapping in the generated table" | docs/07 §5.2 D6 (:574) | FALSE (mechanism) | No owner mapping in any generated artifact (rg zero hits); implemented surrogate = default_ref uniqueness only → F-003. |
| "collisions with generated flags are rejected at build time by `cargo xtask config-validator`" | docs/05-command-layer.md:533 (§5.3) | FALSE | config_validator.rs contains no settings/D-rule code (rg zero hits); it is run by NO CI workflow either (ci.yml + full_gate.sh: zero hits for `config-validator` — CI's product.toml coverage is `cargo test -p product-config`, ci.yml:276). Actual D-rule gate = `cargo xtask schema-gen --check` (ci.yml:277-278) → F-002. |
| "config can never enable a missing codec… enforced jointly with tools/config-validator" (feature-exposure intersection) | docs/02 §"couple to config" / R7 | REAL-UNPROVEN | F2 (feature-vs-registry intersection) unimplemented anywhere per config_validator.rs's own header ("needs the registry surface"); only the declared-id half is enforced here (E-GATE). |
| CI runs "schemas/settings.toml D1-D6 + generated-output drift" via schema-gen --check | .github/workflows/ci.yml:277-278 | REAL | Read verbatim; step present in the CI job. |

## 7. Dead / duplicated / unreachable code
- No dead functions. `RESERVED_CLI_*` consts are `pub` but consumed only within this module (+its tests); they are reachable API surface (lib.rs re-exports only `validate`), not dead — noted, not filed.
- Duplication: five rules (d2/d3/d4/d5/d6) each hand-roll the same "linear scan a `seen` Vec, report first" shape; a shared `check_unique` helper would remove ~80 lines. S4-level hygiene, absorbed into the per-function table (no behavioral defect).
- Nothing unreachable: every function is called from `validate()` or tests.

## 8. Dependencies & flags
- Crate-internal deps only: `crate::diagnostic`, `crate::model` (`DefaultValue`, `SettingDef`, `SettingsSchema`, `check_value`) — matches the layer-L1 "no internal deps" claim; product.toml arrives as parsed data (`toml::Value`), not a crate edge (doc 01 §2.3).
- External: `toml::Value` only. No feature flags, no cfg-gating in this file. No `unsafe`/`unwrap`/`expect`/`panic` in non-test code (only `unwrap_or_default` :397) — consistent with lib.rs's "no unsafe/unwrap/expect/panic" claim; clippy workspace-clean per baseline #5/#6.

## 9. Open questions for the next agent
- A02 (runtime.rs): the runtime self-check runs with `product=None` (D1 skipped) — a schema/product.toml drift (ref target removed from product.toml) is caught ONLY by CI `schema-gen --check` and the lib.rs test; the shipped binary embeds a schema whose `default_ref`s may point nowhere. Who resolves `DefaultValue::Ref` at runtime, and what happens then?
- A09 (parse.rs): does parse enforce the `--`-prefix of `cli.flag` and `-x` shape of `cli.short`? D3's reserved-short check is bypassable if `short = "h"` (no dash) parses clean — `RESERVED_CLI_SHORTS` contains `"-h"`, not `"h"`.
- Docs agents: F-002/F-005 corrections needed in docs/05 §5.3, docs/07 §2.7, §7-C4.
- Wave-8 follow-up (audio.set-eq dead backend): this crate correctly refuses unknown keys, but no validator anywhere detects that a *known* setting's consumer does not exist — the declared-but-unconsumed class; candidate home is the 4.E1 owner table (F-003).
