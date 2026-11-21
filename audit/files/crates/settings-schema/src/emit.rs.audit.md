# AUDIT: crates/settings-schema/src/emit.rs
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-21 |
| Lines / functions | 543 / 19 (13 production items + 6 test fns) |
| File verdict | REAL (generators real + G2 shipped in `lumen --help`; all 5 in-file tests + the CLI golden-tie test executed OK in the orchestrator's no-fail-fast run — but the emitted claim text documents a flag surface the parser rejects) |
| Reachable from shipped entry point? | PARTIAL — YES for `emit_help`/`generate_str` via `lumen --help [AREA]`: main.rs:14 → run::run → parse_argv (`--help`) → run.rs:86/96/116/243 → help.rs:137-152/109-129 → generated_help (help.rs:156-160) → `generate_str(GenTarget::CliHelp)` → `emit_help`. NO for `emit_ts`/`emit_docs`: only via dev binary `schema-gen` (bin/schema_gen.rs:105-107) / `cargo xtask schema-gen`. `generate` (Write API): zero production callers (rg over repo: only emit.rs:536, its own test). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `rg -n "generate_str|schema_gen|schema-gen"` → callers: apps/cli/src/help.rs:34,159 + bin/schema_gen.rs (dev). `rg -n "generate-manifest"` → only emitted text, validate.rs:43 (name reservation), doc 07 §583; NOT in apps/cli. `sed -n '155,167p' apps/cli/src/parse.rs` → parser accepts only 4 flags. `rg -n "settings\.d\.ts"` → no UI consumer. `cat ui/tsconfig.json` → include = src/e2e only. `rg -c "^\[\[setting\]\]" schemas/settings.toml` → 121. Execution evidence: INDEX runs #3/#4 abort before this crate, but the orchestrator repair run `cargo test --workspace --release --locked --no-fail-fast` (audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt, 2026-11-21T08:12) executed the whole workspace: settings-schema unittests 43 passed incl. all 5 `emit::tests::*` (log:4876-4880), and lumen_cli unittests 129 passed incl. `help::tests::generated_help_matches_the_committed_artifact ... ok` (log:2865) and `areas_block_lists_every_screen_the_seed_declares ... ok` (log:2864). |

## 1. One-paragraph verdict
The three generators are honest, deterministic string builders over the parsed schema — no fakeness in the code itself (no todo!, no swallowed errors; dead branches in `enum_members`/`value_placeholder` are unreachable for validated schemas because parse.rs enforces E-DOMAIN: enum⇒`enumeration`, ranged types⇒`range`). The rot is in what the generated text CLAIMS and in who consumes it: G2 ships inside `lumen --help <area>` and documents 121 settings flags plus a "reserved" `--generate-manifest` flag, while the shipped argv parser (`apps/cli/src/parse.rs:155-167`) accepts exactly `--json/--control-stdio/--version|-v/-h|--help` and rejects every documented settings flag as `unknown command`. G1's `settings.d.ts` — billed as "the typed settings surface for the UI" — is consumed by nothing: the UI's tsconfig never includes it and the UI generates its own parallel, structurally different type file from a Node TOML-subset parser. This is the config-system wave's headline pattern: declared, generated, drift-checked — and wired to nothing.

## 2. What actually works (verified)
- Committed artifacts exist and are the generator's shape: `sed -n '1,40p' schemas/generated/settings-help.txt` → Areas block (11 areas: audio 11, controls 9, engine 6 (engine-internal), extensions 6, global 13, interface 18, performance 15, playback 17, privacy 11, recording 2, video 13 = 121 options) with flags/defaults, matching G2's format logic read in emit.rs:280-331.
- G2 reachability from shipped binary proven by call chain: apps/cli/src/run.rs:86 (`help_for_area`), :96/:116/:243 (`help_text`) → help.rs:156-160 `generate_str(schema, GenTarget::CliHelp)` (static trace; binary output not captured in baseline — rc-smoke #20 covers engine checks only).
- Determinism is structural: no HashMap iteration anywhere in the emit path; all lists sorted (`keys.sort_unstable()` emit.rs:209; `screens_in_use()` sorts+dedups, model.rs:33-42). Verified by reading, not execution.
- `rg -c "^\[\[setting\]\]" schemas/settings.toml` = 121; artifact self-report "121 entries across 11 screen(s) of 11 known areas" is consistent with the seed count.

## 3. Findings

### F-001 [S2]
Where: crates/settings-schema/src/emit.rs:286-290 (emitted into schemas/generated/settings-help.txt:7-11 and sliced into `lumen --help <area>` by help.rs:186-223)
Code:
```
    out.push_str("  Settings surface as generated long flags (--<area>-<key> <value>).\n");
    out.push_str("  `lumen --help` lists areas with counts; `lumen --help <area>` lists that\n");
    out.push_str("  area's flags (the VLC --longhelp pattern, generated).\n");
    out.push_str("  Reserved (non-settings) flags: -h/--help, -v/--version, --json,\n");
    out.push_str("  --control-stdio, --generate-manifest.\n\n");
```
What is wrong: The generated help declares the settings surface as CLI long flags (121 of them, e.g. `--audio-normalize-loudness <on|off>` at settings-help.txt:22) — but the shipped argv parser implements none of them. apps/cli/src/parse.rs:155-167:
```
        match arg.as_str() {
            "--json" => invocation.json = true,
            "--control-stdio" => invocation.control_stdio = true,
            "--version" | "-v" => invocation.version = true,
            "-h" | "--help" => {
```
Everything else falls into `command_words` and `parse_command` (parse.rs:215-225) errors with `unknown command `--audio-normalize-loudness``. Settings are actually settable only via `setting set <key> <value>` (parse.rs:211). The per-area sections that `lumen --help <area>` prints (help.rs:186-223 `area_section`) list every flag with value placeholders and defaults — a user following the shipped help gets a hard error for every flag shown.
Why it matters: This is the exact "config declared vs config consumed" fraud pattern — a declared surface (docs + generated help + D3-uniqueness validation of flag names in validate.rs) with zero consumers. 121 flags documented, 0 parsed. Same genus as Wave 8's `audio.set-eq` awaiting a nonexistent backend. Doc 07 §5.3 row G2 asserts "CLI flags + `--help` … long flags `--<area>-<key>`" as shipped behavior — FALSE.
What correct looks like: Either parse_argv maps `--<area>-<key> <value>` onto `config.set-setting` (one generic loop over the schema's flag table, which `runtime::embedded` already provides), or G2/`--help <area>` text states plainly that settings are set via `lumen setting set <area.key> <value>` and the flags shown are the canonical key spelling only.
Confidence: HIGH — Verification: read (parse.rs:151-227, help.rs:109-160, settings-help.txt:1-40); committed artifact matches emitted claim text byte-for-byte per generator logic; not executed (no baseline run covers `lumen --help`).

### F-002 [S3]
Where: crates/settings-schema/src/emit.rs:290 vs apps/cli/src/help.rs:126-127
Code:
```
    out.push_str("  --control-stdio, --generate-manifest.\n\n");
```
vs help.rs:126-127:
```
    out.push_str("  Reserved (non-settings) flags: -h/--help, -v/--version, --json,\n");
    out.push_str("  --control-stdio.\n\n");
```
What is wrong: The golden artifact lists `--generate-manifest` as a reserved CLI flag; no code implements it (`rg -n "generate-manifest"` → only emit.rs:290, validate.rs:43 (name reservation so settings can't collide with it), doc 07 §583 promising "dumping the full option table as TOML/JSON for scripts"). The CLI's own reserved-flag sentence omits it — two disagreeing reserved-flag lists for the same binary.
Why it matters: A "reserved" flag is a contract that the name is meaningful; users/scripts trying `lumen --generate-manifest` get `unknown command`. The reserved table in validate.rs:43 enforces disjointness against a flag that will never exist.
What correct looks like: Remove `--generate-manifest` from the emitted reserved list (and validate.rs) until implemented, or implement the manifest dump in parse_argv.
Confidence: HIGH — Verification: read + rg (no other definition site).

### F-003 [S3]
Where: crates/settings-schema/src/emit.rs:146-159 (G1 header claims) + the artifact's consumption status
Code:
```
    out.push_str("// Generated by `cargo xtask schema-gen` from schemas/settings.toml — DO NOT EDIT.\n");
    out.push_str("// Generator G1 (doc 07 §5.3): the typed settings surface for the UI.\n");
```
What is wrong: `schemas/generated/settings.d.ts` (7,295 bytes per wave-00 manifest.tsv:2110) is consumed by NOTHING. ui/tsconfig.json `include` is `["src","e2e","vite.config.ts",...]` — the file is outside ui/ and outside any include set, and `rg -n "settings\.d\.ts"` over ui/ finds zero imports. The UI's real settings surface is generated by a SEPARATE Node pipeline (ui/scripts/gen-settings-screens.mjs, which re-parses schemas/settings.toml and product.toml with its own TOML-subset parser) into ui/src/screens/settings/generated/schema.ts — whose type union already diverges: schema.ts:9-20 `SettingValueType` has 10 tags (no `list<path>`, `list<url>`, `list<enum>`), while emit.rs:162-176 emits 13 tags. Latent today (settings.toml uses only two `list<string>` entries, lines 603/618) but two "single sources of truth" generate two different TS surfaces.
Why it matters: Declared-but-never-read artifact (the wave's core question); duplicated generator pipelines guarantee silent divergence the moment someone adds a `list<path>` setting — the UI generator will type it differently than the golden d.ts and no test cross-checks the two unions against each other.
What correct looks like: Either delete G1 (the UI pipeline owns the TS surface) or make gen-settings-screens.mjs import the type names from settings.d.ts so one union exists.
Confidence: HIGH — Verification: read tsconfig.json include; rg for imports (none); read schema.ts:8-20.
(Scope note: the value-bearing UI table itself DID land — ui/src/screens/settings/generated/schema.ts, 2,918 lines, Screens.tsx 346 lines — so the settings framework is not vaporware; the finding is the orphaned duplicate type surface, not a missing UI.)

### F-004 [S4]
Where: crates/settings-schema/src/emit.rs:158-159
Code:
```
    out.push_str("// (ui/src/generated/settings-schema.ts) lands with the settings framework\n");
    out.push_str("// (card 4.E1); this ambient file fixes the type surface only.\n\n");
```
What is wrong: The comment (baked into the shipped artifact) names a path that never existed — the value table actually landed at ui/src/screens/settings/generated/schema.ts (verified: file exists, 2,918 lines, header "GENERATED FILE … card 4.E1").
Why it matters: The artifact's own regeneration instructions point readers at a phantom path.
What correct looks like: Update the emitted comment to the real path (regenerate the golden).
Confidence: HIGH — Verification: read + ls ui/src/screens/settings/generated/.

### F-005 [S4]
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
```
What is wrong: Near-tautology — a pure function called twice in the same process will agree unless it reads a clock/RNG/global; it passes even if the output were garbage. Determinism across RUNS (what the card's acceptance criterion and CI `--check` need) is established structurally (no HashMap, sorted iteration) and by the golden-tie tests, not by this one. Execution note (corrected): the INDEX runs #3/#4 abort before this crate, but the orchestrator's no-fail-fast repair run DID execute this test (orchrun-test-workspace-nofailfast.log.txt:4878 `test emit::tests::generation_is_deterministic ... ok`) — so it runs and passes; it just proves little.
Why it matters: The test gives the determinism claim a green checkmark without testing the property that matters (byte-identity across processes of the committed artifact), which the golden-tie tests do cover; a reader auditing only test names would over-credit this one.
What correct looks like: A test that regenerates into two buffers across two `parse_str` calls from freshly read bytes (still weak), or rely on the golden-tie tests and delete this one; the real cross-run guarantee is the committed-artifact drift check.
Confidence: HIGH — Verification: read + orchrun log line cited (executed, passed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum GenTarget` | 23-31 | REAL | — | 3 variants, matches schema_gen.rs + help.rs usage |
| `enum GenError` | 35-40 | REAL | — | Io-only; honest (emitters infallible post-validation) |
| `generate` | 43-51 | REAL | S4 | Write-API; zero production callers (only its own test at 532-542); bin/schema_gen.rs uses `generate_str` |
| `generate_str` | 55-61 | REAL | — | The real API: help.rs:159 (shipped CLI) + schema_gen.rs:105-107 (dev) |
| `value_placeholder` | 68-85 | REAL | — | Enum-without-enumeration branch would emit `<|>` but unreachable: parse.rs E-DOMAIN (parse.rs:519-560) enforces enum⇒enumeration |
| `enum_members` | 87-92 | REAL | — | `&[]` fallbacks unreachable for validated schemas (same E-DOMAIN) |
| `default_display` | 95-100 | REAL | — | Literal via toml Display; Ref as `-> product.toml <path>` |
| `domain_display` | 103-117 | REAL | — | `(_, Range)` first-match means a hypothetical String+Range would print a numeric domain — unreachable, parse.rs gates range to ranged types |
| `marker_list` | 119-134 | REAL | — | gate/advanced/restart/deprecated markers; order fixed |
| `screen_is_internal` | 137-140 | REAL | — | all(!ui) && non-empty; matches help.rs test derivation |
| `emit_ts` | 144-276 | REAL | S3/S4 | Claims "typed settings surface for the UI" (F-003); stale path comment (F-004); 13-tag union; keys sorted → deterministic |
| `emit_help` | 280-331 | REAL | S2 | Emits the false flag-surface sentence + per-flag lines (F-001), phantom `--generate-manifest` (F-002); `{screen:<14}` column cap breaks alignment if a screen id ever exceeds 14 chars (today's ids ≤10 — latent) |
| `emit_docs` | 335-413 | REAL | — | Markdown tables; no escaping of `|` in keys/defaults — keys are kebab-case by validation, defaults are toml-rendered (a string default containing `|` would break the table; none today) |
| `fixture_schema` (test) | 420-464 | REAL | — | Honest guard: parse failure asserts rendered diagnostics empty |
| `generation_is_deterministic` (test) | 466-476 | WEAK | S4 | F-005 |
| `ts_types_list_every_key_and_screen` (test) | 478-490 | REAL | — | Content assertions on a 2-entry fixture; would fail if emit_ts dropped keys/screens |
| `help_lists_areas_flags_and_defaults` (test) | 492-512 | REAL | — | Pins exact flag lines incl. `<50..2000 ms>`, markers, default refs — real mutation coverage for emit_help |
| `docs_tables_carry_every_column` (test) | 514-530 | REAL | — | Pins full table row strings for both fixture entries |
| `generate_into_writer_matches_string_form` (test) | 532-542 | REAL | — | Writer/string equivalence; the only caller of `generate` |

## 5. Test honesty (in-file tests)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| generation_is_deterministic | Only for nondeterminism bugs | Same-process double call of a pure fn; passes on garbage output (F-005) |
| ts_types_list_every_key_and_screen | YES | Asserts exact key/screen/union substrings against a parsed fixture |
| help_lists_areas_flags_and_defaults | YES | Asserts exact emitted lines (`--engine-tick-interval-ms <50..2000 ms>  [advanced] [restart required]`) |
| docs_tables_carry_every_column | YES | Asserts two complete table rows byte-exactly |
| generate_into_writer_matches_string_form | YES for writer/string divergence | Would catch a `generate` that writes a prefix/truncates |
| (execution) | — | All executed OK in the orchestrator no-fail-fast run: 5/5 `emit::tests::*` ok (orchrun log:4876-4880), CLI golden-tie `generated_help_matches_the_committed_artifact` ok (log:2865). INDEX runs #3/#4 alone would have left this UNPROVEN (both abort before the crate) — the no-fail-fast repair run is the executed evidence. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "G2: CLI flags + `--help` … long flags `--<area>-<key>` … plus `--generate-manifest` dumping the full option table" | docs/07-config-driven-system.md §5.3 (line 583) | FALSE | parse.rs:155-225 parses no settings flags, no manifest dump; rg "generate-manifest" → no implementation |
| "`--help` text for settings-derived flags is generated from the settings schema" | doc 05 §5.3 (via help.rs:3-6) | REAL | help.rs:156-160 calls G2 over the embedded schema; the byte-tie to the committed golden is an EXECUTED test (`generated_help_matches_the_committed_artifact ... ok`, orchrun log:2865). The live `lumen --help` binary output itself has no captured run (rc-smoke #20 covers engine checks only). |
| "G1: the typed settings surface for the UI" | emit.rs:149 / lib.rs docs | SHALLOW | Artifact exists + drift-checked, but zero UI consumers (tsconfig include excludes it; UI builds its own schema.ts) |
| "running twice is byte-identical (idempotence … CI's drift check via `--check`)" | emit.rs:2-4 | REAL | Structural (sorted, no HashMap) AND executed: emit tests + the CLI golden-tie over the embedded schema all passed in orchrun-test-workspace-nofailfast.log.txt (4876-4880, 2865) |

## 7. Dead / duplicated / unreachable code
- `generate` (Write API): dead in production; only its own test calls it.
- `enum_members`/`value_placeholder`/`domain_display` non-Enumeration fallbacks: unreachable for validated schemas (parse.rs E-DOMAIN) — defensive, fine.
- Duplication: TWO schema→TS pipelines (this crate's G1 vs ui/scripts/gen-settings-screens.mjs) with divergent type unions (13 vs 10 tags) — F-003.
- `emit_ts`/`emit_docs` unreachable from shipped binaries (dev tooling only) — expected for generators, noted for reachability completeness.

## 8. Dependencies & flags
- Uses crate::model types only + std::io::Write + thiserror (via GenError). No feature flags gate any of this code (no cfg anywhere in file — `rg -n "cfg\(" emit.rs` → only `#[cfg(test)]`). Committed artifacts drift-guarded by bin/schema_gen.rs --check + xtask bridge (not baseline-executed).

## 9. Open questions for the next agent
- A08 (bin/schema_gen.rs): does `--check` mode verify the artifact's own header comments (F-004 path) or only byte-compare (byte-compare would keep the stale path forever)?
- A02 (settings-schema/src/runtime.rs): does `config.set-setting` reach the engine runtime, or does it also dead-end (the `setting set` path is the only live settings lane — if it too is unwired, F-001 escalates)?
- Whoever audits ui/scripts/gen-settings-screens.mjs: why does the UI need a second TOML parser instead of consuming G1 output (F-003)?
