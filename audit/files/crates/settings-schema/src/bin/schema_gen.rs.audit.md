# AUDIT: crates/settings-schema/src/bin/schema_gen.rs
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-21 |
| Lines / functions | 331 / 11 items (main, run, parse_args, value_of, output_paths, find_workspace_root, failure, drift, write_file, 2 structs + USAGE const) |
| File verdict | REAL |
| Reachable from shipped entry point? | Binary: NO (dev/CI tool). Chain that reaches it: `.github/workflows/ci.yml:278` → `cargo run -p xtask -- schema-gen --check` → `tools/xtask/src/cmd/schema_gen.rs:56-70` (shells out to this binary, cwd = workspace root). Its LIB engine IS shipped: `apps/cli/src/help.rs:34` `use settings_schema::{GenTarget, generate_str};` — the `lumen` CLI regenerates G2 help at runtime from the embedded schema |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | Read binary + lib.rs + tests/integration.rs (425 lines) + tools/xtask/src/cmd/schema_gen.rs; rg consumers of the three artifacts (apps/, ui/, tools/, scripts/, docs/): G2 → apps/cli/src/help.rs:289-301 (drift-tie test) + rebrand.rs:672; G3 → rebrand.rs:671; G1 → NOTHING; read ui/tsconfig.json (include list); rg GenTarget/generate_str/screens_in_use in emit.rs/model.rs; rg config-validator in .github/workflows + scripts/ (absent); read scripts/full_gate.sh fully; orchrun-test-workspace-nofailfast.log.txt:4921-4943 (binary's 8/8 integration suite) + :2864-2865 (CLI help drift-tie); sed -n verification of every quote |

## 1. One-paragraph verdict
This is the rare file in the config system that fully earns its docs: a thin, panic-free CLI over `settings_schema::load` + `generate_str` with an honest exit-code contract (0/1/2), a subprocess bridge from the frozen xtask, a CI drift gate (ci.yml:278), and a binary-surface integration suite that has ACTUALLY RUN AND PASSED (orchrun log: 8/8 — committed-tree `--check` no-drift, byte-idempotence, drift→exit 1, D2 duplicate→exit 2, IO error→exit 2, `--help`→exit 0). The defects are around it, not in it: the G1 output it generates (`settings.d.ts`) is consumed by nothing (doc 07 §10.3's "TS output typechecks in the UI job" is false), and doc 07 §9's claimed gate chain diverges from both the local gate script and CI (`config-validator` runs in neither). Minor CLI-hygiene warts in arg parsing and root discovery.

## 2. What actually works (verified)
- Executed evidence (baseline): `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4927-4943` — `Running tests/integration.rs` → 8/8 passed, including `committed_outputs_have_no_drift ... ok` (the binary's `--check` on the REAL committed tree exits 0), `regeneration_is_idempotent_byte_for_byte ... ok`, `stale_output_drift_fails_check_with_exit_one ... ok`, `duplicate_option_fixture_fails_with_rule_d2 ... ok` (exit 2, `error[D2]`, "first defined at line"), `missing_schema_file_is_an_io_error_not_a_panic ... ok`, `help_flag_prints_usage_and_exits_zero ... ok`. The binary's `unittests src/bin/schema_gen.rs` target ran with 0 tests (thin binary, tests live in integration.rs — orchrun log:4921-4923).
- The G2 golden has a live consumer + drift tie in the shipped CLI: orchrun log:2864-2865 — `help::tests::generated_help_matches_the_committed_artifact ... ok` and `help::tests::areas_block_lists_every_screen_the_seed_declares ... ok` (apps/cli).
- Real drift teeth, historically proven: `docs/waves/w26/findings/w26.l05.md:43-44` records this exact gate FAILING (`committed_outputs_have_no_drift`: settings-help.txt differs at line 143, docs at line 99) when the Lead edited the seed without regenerating; since fixed (current run green).
- The xtask bridge is real and forwards exit codes verbatim (tools/xtask/src/cmd/schema_gen.rs:56-85, `--quiet`, cwd = true root).
- The exit-code contract in USAGE (schema_gen.rs:57-58) matches every code path (usage_error→2, LoadError→2 via `failure`, write/IO→2, drift→1, success→0) — traced all paths.

## 3. Findings

### F-001 [S3]
Where: crates/settings-schema/src/bin/schema_gen.rs:8 (G1 emission) + ui/tsconfig.json:22-29; doc claim docs/07-config-driven-system.md:864
Code (schema_gen.rs:6-10):
```rust
//! emits the three committed outputs deterministically:
//!
//! * `schemas/generated/settings.d.ts`     (G1 — TS type surface)
//! * `schemas/generated/settings-help.txt` (G2 — CLI `--help` golden)
//! * `schemas/generated/settings-docs.md`  (G3 — settings reference)
```
and (ui/tsconfig.json:22-29):
```json
  "include": [
    "src",
    "e2e",
    "vite.config.ts",
    "playwright.config.ts",
    "playwright.web.config.ts",
    "playwright.visual.config.ts"
  ]
```
What is wrong: G1 (`schemas/generated/settings.d.ts`) has ZERO consumers. No `ui/src` file imports it (rg "settings\.d" over ui/src → only unrelated `settings.dirtyCount` hits); ui/tsconfig.json's `include` does not cover it (and `skipLibCheck: true` at line 17 would skip it anyway); the UI's own generator re-parses `schemas/settings.toml` from scratch with a hand-written JS TOML-subset parser instead of consuming the d.ts (ui/scripts/gen-settings-screens.mjs:2-11, which styles itself "doc 07 §5.3 generator G1 (UI side)" — the name G1 now denotes TWO different generators).
Why it matters: Doc 07 §10.3: "TS output typechecks (`tsc --noEmit`) in the UI job" — FALSE: tsc never sees this file. The repo generates, commits, and drift-gates a TS type surface that no compiler, runtime, or test reads beyond existence/shape assertions (integration.rs:413-424 checks non-empty + self-reported counts). Classic Wave-11 declared-but-never-consumed artifact.
What correct looks like: Either import the d.ts from the UI's generated schema module (and exempt it from skipLibCheck) so `tsc --noEmit` actually validates it, or delete the TsTypes target until the card 4.E1 value table consumes it.
Confidence: HIGH — Verification: read (rg over ui/, apps/, tools/, scripts/, docs/; tsconfig read).

### F-002 [S3]
Where: docs/07-config-driven-system.md:843-848,854 (§9 job sequence) vs scripts/full_gate.sh:10-84 and .github/workflows/ci.yml
Code (doc 07 §9:843-848):
```
3  cargo xtask check-architecture          (doc 01 §5)
4  cargo xtask config-validator            (C4 — schema + semantics + D-rules)
5  cargo xtask schema-gen --check          (generated files match source)
```
plus docs/07-config-driven-system.md:854: `Steps 4–8 are the config-driven system's own teeth; they run before any compile-heavy work so config violations fail fast.`
What is wrong: (a) The LOCAL gate `scripts/full_gate.sh` (11 steps: fmt, clippy, test, check-architecture, fixtures, fuzz-registry, lint-literals, check-identity, sbom, web-budget, packaging) contains NEITHER `schema-gen --check` NOR `config-validator`; (b) CI runs `schema-gen --check` only in the separate `schema-validation` job (ci.yml:278), while `cargo xtask config-validator` — the doc's C4 gate — is executed by NO workflow and NO script (rg over .github/workflows/ and scripts/ → 0 invocations; the only recorded run is the orchestrator's manual baseline invocation, INDEX row 10, which FAILED with 4 violations); (c) the fail-fast ordering claim is false in both places that do run config gates (full_gate.sh runs lint-literals/check-identity as steps 7–8, AFTER the compile-heavy clippy/test steps 2–3; CI's gate job likewise runs fmt→clippy→build→test before its xtask steps).
Why it matters: Doc 07 §9 presents this chain as merge-blocking ("the whole chain is merge-blocking … per MP §13"). An engineer running the documented local gate gets a PASS while schema drift (caught only in CI's schema-validation job) and product.toml violations (caught NOWHERE automated) slip through. The C4 gap is the config system's largest enforcement hole — recorded here from this file's perspective because §9 step 5 is this binary; config-validator ownership belongs to its auditing agent.
What correct looks like: Add `schema-gen --check` and `config-validator` steps to full_gate.sh (cheap, non-compile steps — exactly the fail-fast ordering §9 promises), or fix §9 to describe the real gate set and mark C4 as unenforced.
Confidence: HIGH — Verification: read (full_gate.sh read in full; rg over workflows+scripts).

### F-003 [S4]
Where: crates/settings-schema/src/bin/schema_gen.rs:289-298 (drift) and 184-196 (parse_args)
Code:
```rust
    let committed = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => {
            eprintln!(
                "schema-gen: {} is missing — run `cargo xtask schema-gen` and commit it",
```
and:
```rust
            "--source" | "--product" | "--ts-out" | "--help-out" | "--docs-out" | "--out-dir" => {
                let value = value_of(args, i, arg)?;
                let path = PathBuf::from(value);
```
What is wrong: (a) `drift()` reports EVERY read error as "is missing" — a permission-denied or EISDIR output path produces a false "missing" message (outcome drift=true is right, the diagnostic is wrong); (b) `parse_args` accepts the next flag token as a value (`schema-gen --source --check` sets source="--check" and check stays false) and duplicate flags silently last-win.
Why it matters: Wrong diagnostics waste maintainer time; flag-as-value silently inverts an intended `--check` run — a CI wrapper passing a malformed arg list would WRITE outputs instead of checking them.
What correct looks like: Distinguish `ErrorKind::NotFound` from other IO errors in the message; reject values starting with `--` and duplicate flags with usage errors (exit 2).
Confidence: HIGH — Verification: read (both code paths traced; no test covers the flag-as-value case).

### F-004 [S4]
Where: crates/settings-schema/src/bin/schema_gen.rs:258-264
Code:
```rust
    loop {
        let manifest = dir.join("Cargo.toml");
        if let Ok(text) = fs::read_to_string(&manifest)
            && text.contains("[workspace]")
        {
            return Ok(dir);
```
What is wrong: Root discovery is a substring test for "[workspace]" on the first Cargo.toml walking up. Any nested manifest containing a `[workspace]` table stops the walk early: the repo's own `apps/desktop-tauri` (workspace-excluded, own manifest) would be mis-identified as the root when the binary runs with a cwd under it, producing a confusing load error naming `apps/desktop-tauri/schemas/settings.toml`.
Why it matters: Low blast radius — the supported invocation paths (xtask bridge with explicit cwd, repo-root runs) never hit it — but the failure mode is a misleading error instead of "not the Lumen workspace".
What correct looks like: Parse the manifest (the toml crate is already a lib dependency) and require an actual `[workspace]` table with members, or keep walking unless the candidate root also contains `schemas/settings.toml`.
Confidence: HIGH — Verification: read; behavior under apps/desktop-tauri UNVERIFIED (would need a cargo run — forbidden by audit rules).

### F-005 [S4]
Where: crates/settings-schema/tests/integration.rs:344-346 (test of this binary)
Code:
```rust
    let stale = read(&help_path).replace("27 entries", "999 entries");
    // The fixture only has 2 entries; any hand edit is drift:
    let stale = stale.replace("playback", "playbackx");
```
What is wrong: The first `.replace("27 entries", "999 entries")` is a no-op — the CLEAN_FIXTURE generates a 2-entry help text that contains no "27 entries" string (27 was the seed size of an earlier era). The corruption is done entirely by the second replace; the first line is vestigial and implies a count-corruption step that never happens.
Why it matters: Misleading test scaffolding; a future reader assumes the count line is mutated and "fixes" the wrong thing.
What correct looks like: Delete the no-op replace (the comment on the next line already tells the truth).
Confidence: HIGH — Verification: read (CLEAN_FIXTURE at integration.rs:57-83 has exactly two [[setting]] rows; G2 counts derive from them).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `main` | 73-81 | REAL | — | Maps Failure→ExitCode; single eprintln; no panics |
| `run` | 83-154 | REAL | — | Correct orchestration; `--check` evaluates ALL THREE drifts before deciding (bitwise-or, no short-circuit — every drifted file gets its message); writes only in non-check mode; validation happens BEFORE any write (D2 test proves no-output-on-failure, integration.rs:320-324) |
| `Options` (struct) | 156-165 | REAL | — | Plain data |
| `parse_args` | 167-211 | REAL | S4 | Duplicate flags last-win; flag-as-value accepted (F-003); `--out-dir` conflict check correct (203-209) |
| `value_of` | 214-218 | REAL | — | Missing-value → usage error, exit 2 |
| `Outputs` (struct) + `output_paths` | 220-249 | REAL | — | Precedence correct: `--out-dir` > per-target flags > `<root>/schemas/generated/` defaults |
| `find_workspace_root` | 253-276 | REAL | S4 | Substring `[workspace]` match hazard (F-004); honest "not found" error otherwise |
| `failure` | 280-285 | REAL | — | LoadError → exit 2, diagnostics pre-rendered by the lib |
| `drift` | 289-316 | REAL | S4 | Missing ⇒ drift (correct); read-error conflation (F-003); first-diff line via common-prefix count (approximate, informational only) |
| `write_file` | 318-331 | REAL | — | create_dir_all for parents; typed IO error mapping |
| `USAGE` (const) | 34-58 | REAL | — | Exit-code contract documented and matches implementation (all paths traced) |

## 5. Test honesty (tests/integration.rs — 425 lines, 8 tests)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `committed_outputs_have_no_drift` | YES | Runs the real binary `--check` on the committed tree; asserts exit 0 AND stdout contains the DERIVED entry count (never pinned — count re-read from the seed by independent raw-TOML parse, integration.rs:132-141) |
| `seed_shape_is_what_every_committed_artifact_self_reports` | YES | Rebuilds the expected G1 header + every G2 areas row from the seed by independent parse; catches shape drift the byte-compare can't attribute |
| `regeneration_is_idempotent_byte_for_byte` | YES | Two out-dirs, byte-compare all three outputs + a self `--check` (a nondeterministic emitter fails the compare) |
| `duplicate_option_fixture_fails_with_rule_d2` | YES | Exit 2 + `error[D2]` + "duplicate setting key" + "first defined at line" + asserts NO outputs written on failure |
| `stale_output_drift_fails_check_with_exit_one` | YES | Corrupts a generated file; `--check` must exit 1 with "differs from the regenerated output at line" (mutation of `drift` to return false → code 0 → fail) |
| `missing_schema_file_is_an_io_error_not_a_panic` | YES | Exit 2 + "reading" in stderr (a panic would produce a different code/stderr) |
| `help_flag_prints_usage_and_exits_zero` | YES | Exit 0 + "Usage: schema-gen [OPTIONS]" + "--check" in stdout |
| `committed_generated_files_exist` | YES (thin) | Existence + non-empty; the real content check is the drift test |
| (in-binary `mod tests`) | — | None exist; the binary is intentionally thin — its logic surface is covered by the integration suite (unittests target ran 0 tests, orchrun log:4921-4923) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| G2 emitted as CLI `--help` golden, "generated from the settings schema" | doc 07 §5.3; help.rs:3-6 | REAL | Runtime: apps/cli/src/help.rs:12-16 calls G2 over the EMBEDDED schema; golden tied by help.rs tests — VERIFIED PASSING: orchrun log:2864-2865 |
| CI step 5 `cargo xtask schema-gen --check` (merge-blocking) | doc 07 §9:845 | REAL (in CI only) | ci.yml:278; NOT in local full_gate.sh (F-002) |
| "Steps 4–8 … run before any compile-heavy work" | doc 07 §9:854 | FALSE | full_gate.sh steps 7–8 run after clippy/test (steps 2–3); CI gate job runs build/test before its xtask steps |
| "Idempotence: generate twice → byte-identical (all four targets G1–G4)" | doc 07 §10.3:864 | REAL for G1–G3 / wording drift on G4 | Integration test passed (orchrun:4939); G4 is the embedded runtime (lib.rs:16-20), not a generation target |
| "TS output typechecks (tsc --noEmit) in the UI job" | doc 07 §10.3:864 | FALSE | ui/tsconfig.json include excludes schemas/; no ui/src import of settings.d.ts (F-001) |
| "Drift detection: `--check` on a mutated generated file fails" | doc 07 §10.3:864 | REAL | `stale_output_drift_fails_check_with_exit_one` passed (orchrun:4942) |
| "D-rule violation fixtures … all rejected" | doc 07 §10.3:864 | REAL (lib-level, on test names) | Binary-level: D2 only (integration.rs:283); D1/D3/D4/D5/D6 fixtures exist as unit tests in validate.rs:563-724 (that file's owner should verify them under mutation) |
| ci.yml: "COVERED C4: schema-validation job owns product.toml … validation" | ci.yml:469 | FALSE (for the C4 gate as documented) | The C4 gate is `cargo xtask config-validator` (doc 07 §9:835); it is invoked by no workflow/script (F-002) |

## 7. Dead / duplicated / unreachable code
- In this file: none — every fn is on the execution path (run→parse/find/load/generate/check/write).
- Around it: G1 artifact dead-consumed (F-001); `schemas/settings.toml` is parsed by THREE independent implementations (Rust `toml` in settings-schema, xtask's toml_lite in config-validator, hand-written JS parser in ui/scripts/gen-settings-screens.mjs:39) — each drift risk is separately gated, but triple-parser duplication of one seed is real architectural debt.

## 8. Dependencies & flags
- Binary uses only the `settings_schema` lib (schema_gen.rs:30-32) + std. No feature flags. No unsafe/unwrap/expect/panic in this file (verified by reading; workspace lints enforce).
- The xtask bridge deliberately avoids a path dependency (frozen-manifest constraint, tools/xtask/src/cmd/schema_gen.rs:6-9) — subprocess bridge, exit codes forwarded verbatim.

## 9. Open questions for the next agent
- F-002's config-validator half: `cargo xtask config-validator` runs nowhere automated while baseline row 10 shows it failing with 4 violations (brand icon assets `icon.ico`/`icon.icns`/`icon-512.png`/`logo.svg` missing). Which wave owns wiring C4 into CI/full_gate, and were the 4 violations ever fixed?
- F-001: does card 4.E1 (settings framework) plan to consume `settings.d.ts`, or should the TsTypes target be deleted? The UI already ships its own generated schema.ts.
- ui.yml:113-118 runs a `l10n:check` over a UI-side catalog ("en-US 100%") — a THIRD string system besides `locales/en-US.toml` and ESLint literal bans; where is the relationship documented (potential fourth catalog system)?
- validate.rs unit tests (D1–D6 fixtures) belong to another agent — my §6 row trusts their names; verify they fail under mutation.
