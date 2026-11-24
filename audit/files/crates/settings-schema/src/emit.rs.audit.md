# AUDIT: crates/settings-schema/src/emit.rs
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines / functions | 543 / 11 (+5 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — lumen-cli release lane: apps/cli/src/run.rs:96,116,243 `help::help_text` -> apps/cli/src/help.rs:156-160 `generated_help()` -> `settings_schema::generate_str(schema, GenTarget::CliHelp)` (help.rs:34 import) -> this file `emit_help` (280). Every `lumen --help` runs it. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | rg reserved-flag list in apps/cli (exit 0); rg `generate-manifest` repo-side (only the generated artifact itself); ls ui/src/generated + ui/src/screens/settings/generated (exit 0); baseline citation cargo-test-workspace.txt:5310-5390 (43 unit + 8 integration pass); sed quote verification |

## 1. One-paragraph verdict
Real, deterministic string generators over the parsed settings schema; the acceptance criteria (idempotence, drift check, D2 failure) are proven by binary-level integration tests that all pass in the baseline, and `--help` in the shipped lumen-cli executes `emit_help` live. The worst defect is user-visible: the G2 text that `lumen --help <area>` prints advertises `--<area>-<key>` settings flags the shipped parser does not implement (settings are `setting get/set` subcommands), plus a reserved flag `--generate-manifest` no binary parses, contradicting the CLI's own four-flag list. Also: the G1 header points at a UI artifact path that does not exist while the real artifact lives elsewhere and has already shipped, and the TS type-tag unions are hand-maintained duplicates of `model::SettingType::as_str` with no mechanical coverage tie.

## 2. What actually works (verified)
— verified by baseline audit/runs/baseline/cargo-test-workspace.txt:5311-5387: `emit::tests::*` 5/5 pass and integration.rs 8/8 pass, including `committed_outputs_have_no_drift` (the committed G1/G2/G3 artifacts are fresh vs the seed) and `regeneration_is_idempotent_byte_for_byte` (two separate process runs, byte-identical).
— verified by `ls ui/src/screens/settings/generated/` -> `Screens.tsx schema.ts` exist (the UI value table is real).
— verified by reading model.rs:31-52: `screens_in_use()` sorts+dedups, `screen_entries()` sorts by (section, order, key) — cross-run determinism of the emitters holds.

## 3. Findings
F-001 [S2]
Where: crates/settings-schema/src/emit.rs:284-290 (and 317, the per-area flag lines)
Code:
```
    out.push_str("Usage:\n");
    out.push_str("  lumen [OPTIONS] [COMMAND]\n\n");
    out.push_str("  Settings surface as generated long flags (--<area>-<key> <value>).\n");
    out.push_str("  `lumen --help` lists areas with counts; `lumen --help <area>` lists that\n");
    out.push_str("  area's flags (the VLC --longhelp pattern, generated).\n");
    out.push_str("  Reserved (non-settings) flags: -h/--help, -v/--version, --json,\n");
    out.push_str("  --control-stdio, --generate-manifest.\n\n");
```
What is wrong: two defects, both reaching users. (a) The header claims "Settings surface as generated long flags (--<area>-<key> <value>)" and the per-area sections list flags like `--playback-subtitle-mode <auto|off>` — but the shipped CLI parses NO settings long flags: apps/cli/src/parse.rs implements settings as the SUBCOMMANDS `setting get <area.key>` / `setting set <area.key> <value>` (parse.rs:64-66, 343-372). `lumen --help <area>` prints these very flag sections to users (help.rs:137-154 `help_for_area` -> `area_section`), and typing an advertised flag is rejected — W11-A04 EXECUTED `./target/debug/lumen --audio-volume-step 5` -> `error: unknown command `--audio-volume-step``, exit 64 (audit/files/schemas/generated/settings-docs.md.audit.md, row 5). The shipped binary therefore advertises, in its own --help output, a flag surface its parser refuses. (b) The reserved-flag list names five flags including `--generate-manifest`, which no binary in the repo parses (`rg "generate-manifest|generate_manifest"` repo-wide: only the generated artifacts + audit files), and it contradicts the CLI's own four-flag list at apps/cli/src/help.rs:126-127; the usage line also disagrees (`[COMMAND]` optional here vs `<COMMAND>` required at help.rs:113). CI's `--check` pins the artifact to this emitter, never to the real CLI, so none of this self-corrects. What correct looks like: emit the actual verb-based surface (`lumen setting set <area.key> <value>`) or land the flag parser; derive the reserved list from the parser. Confidence: HIGH — Verification: read + rg + W11-A04's executed run (cross-audit corroboration).

F-002 [S3]
Where: crates/settings-schema/src/emit.rs:158-159 (committed into schemas/generated/settings.d.ts:7-8)
Code:
```
    out.push_str("// (ui/src/generated/settings-schema.ts) lands with the settings framework\n");
    out.push_str("// (card 4.E1); this ambient file fixes the type surface only.\n\n");
```
What is wrong: the pointer is wrong twice. The value-bearing table exists today at `ui/src/screens/settings/generated/schema.ts` (generated by `ui/scripts/gen-settings-screens.mjs`, `npm run settings:generate`), not at `ui/src/generated/settings-schema.ts` — that directory contains only `commands.ts` + `README.md` (ls verified). And it no longer "lands with" anything: card 4.E1 shipped. The stale sentence is mechanically regenerated into the committed d.ts header on every `cargo xtask schema-gen`, so it can never self-heal. Confidence: HIGH — Verification: read + ls.

F-003 [S3]
Where: crates/settings-schema/src/emit.rs:161-179 and 185-196
Code:
```
    out.push_str("export type SettingType =\n");
    let tags = [
        "bool",
        "int",
        "float",
```
What is wrong: the 13-entry `tags` array and the 7-entry control list are hand-maintained duplicates of `model::SettingType::as_str` (model.rs:151-167) and `Control::as_str` (model.rs:212-222). They match today (verified variant-by-variant), but there is no mechanical tie: adding a `SettingType` variant compiles, G3 prints the new tag via `as_str` (emit.rs:390), while G1's `SettingType` TS union silently omits it — TS type drift invisible to `--check` because the check compares generator output to generator output, not to the model's variant set. The unit tests exercise only `enum` and `duration-ms`. What correct looks like: iterate `SettingType::ALL` (or a const array consumed by both `as_str` and `emit_ts`). Confidence: HIGH — Verification: read + variant comparison.

F-004 [S4]
Where: crates/settings-schema/src/emit.rs:466-476
Code:
```
    #[test]
    fn generation_is_deterministic() {
        let schema = fixture_schema();
        for target in [GenTarget::TsTypes, GenTarget::CliHelp, GenTarget::Docs] {
            assert_eq!(
                generate_str(&schema, target),
                generate_str(&schema, target),
                "{target:?} must be byte-identical across runs"
            );
        }
    }
```
What is wrong: near-tautology — two calls in the same process on the same input prove nothing about cross-run byte-identity (the acceptance criterion). The real proof is integration.rs:237-280 (two separate binary runs compared byte-for-byte), which passes. This unit test adds a false sense of coverage. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| generate | 43-51 | REAL | - | writer wrapper; tested via integration binary |
| generate_str | 55-61 | REAL | - | dispatch; executed by shipped `lumen --help` (help.rs:159) |
| value_placeholder | 68-85 | REAL | - | exercised by help tests incl. `<auto\|off>` and `<50..2000 ms>` |
| enum_members | 87-92 | REAL | - | empty slice for non-enum — placeholder falls to `<number>`/`<ms>` |
| default_display | 95-100 | REAL | - | TOML Display: strings quoted (`default: "off"` in committed artifact, line 39) |
| domain_display | 103-117 | REAL | - | `—` for unconstrained; unit `ms` only for DurationMs |
| marker_list | 119-134 | REAL | - | order pinned by test (503-505) |
| screen_is_internal | 137-140 | REAL | - | all-ui=false => annotation; tested (497) |
| emit_ts | 144-276 | REAL | S3 | hard-coded tag unions (F-003), stale header pointer (F-002) |
| emit_help | 280-331 | REAL | S2 | advertises unparsed settings flags + phantom `--generate-manifest`; contradicts CLI (F-001) |
| emit_docs | 335-413 | REAL | - | markdown tables; row shape pinned byte-exact by test (522-527) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| generation_is_deterministic (467) | NO | same-process double call; tautology (F-004) |
| ts_types_list_every_key_and_screen (479) | YES | pins key/screen/interface emission |
| help_lists_areas_flags_and_defaults (493) | YES | pins placeholder, markers, default-ref arrow, literal default |
| docs_tables_carry_every_column (515) | YES | byte-exact full row match incl. `(advanced)` marker |
| generate_into_writer_matches_string_form (533) | YES | writer path vs string path |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "All three are pure functions... running twice is byte-identical" | emit.rs:2-4 | REAL | integration.rs two-process idempotence test passes (baseline 5375) |
| "G1 writes the TYPE surface only... value-bearing table lands with the settings framework (card 4.E1)" | emit.rs:6-9 | SHALLOW | the table already shipped at a different path (F-002) |
| "G2 — settings-help.txt (CLI --help golden)" | emit.rs:27-28 | FALSE in part | its flag surface is not parsed by the CLI (W11-A04 executed rejection, exit 64); reserved list disagrees with help.rs (F-001) |
| "the emitters themselves are infallible once the schema parses" | emit.rs:33-34 | REAL | pure string building, no fallible ops in emit_* |

## 7. Dead / duplicated / unreachable code
- Reserved-flag sentence duplicated between emit.rs:289-290 and apps/cli/src/help.rs:126-127 with divergent content (F-001).
- Tag lists duplicated from model.rs as_str impls (F-003).
- No dead functions in this file.

## 8. Dependencies & flags
crate::model only (SettingDef/Domain/DefaultValue/fmt_bound); std::io::Write; thiserror for GenError (Io only). No feature gates.

## 9. Open questions for the next agent
- Does `cargo xtask schema-gen` (tools/xtask) invoke this crate's bin or duplicate the emitters? If it duplicates, the committed artifacts have two generators.
- help.rs `area_section` header matching (`line == area`) — a screen whose name equals another screen's prefix plus annotation parsing; verify no screen name is a prefix of another with parens in play (schema screens: 11 committed; seemed safe).
