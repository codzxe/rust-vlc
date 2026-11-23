# AUDIT: apps/control-server/src/config.rs
| Field | Value |
|---|---|
| Auditor | W02-A07 |
| Date | 2026-11-23 |
| Lines / functions | 332 / 5 items (AppError enum + 2 impls + 3 fns) + 4 tests + 3 test helpers |
| File verdict | REAL (executed unit + e2e), but the whole surface it configures is OFF in the shipped product model and outside the release lane |
| Reachable from shipped entry point? | NO for the release lane (lumen-cli/desktop do not depend on control-server). Live chain: `lumen-control-server` bin main.rs:29-30 → run() L183 `load_config_file` / L204 `resolve_server_config` → this file. NOTE: with the repo's product.toml (`control-server = false`, product.toml:167) the gate refuses and the process exits 2 before serving — verified by executed test. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `cargo test -p control-server --lib config` → 4 passed/0 failed; `cargo test -p control-server --test e2e` → 6 passed/0 failed; rg control-server.toml / load_config_file / PolicyFile defaults; read toml-1.1.8 crate error formatter |

## 1. One-paragraph verdict
Hostile question answered: the control-server config IS read by exactly one thing — the `lumen-control-server` binary itself (main.rs:182-210 reads `control-server.toml`, maps NotFound to `PolicyFile::default()`, then calls `resolve_server_config`). No other crate, app, script, or test in the repo reads that file. The resolution code is real and honest: two gates (product feature flag, then policy-grant ids), typed errors, strict defaults that match the documented posture (loopback/ephemeral/deny — verified against `PolicyFile`'s `Default` in policy.rs:135-141), and it is executed by 4 unit tests plus a 6-test e2e suite driving the full stack (both run by me). But the configured surface is dead as shipped: `product.toml` says `control-server = false` ("Phase 5+ [PROPOSAL]"), so the binary refuses to start against the shipped product model, and the app is not in the release lane at all. Two doc defects: "Redaction-safe detail" is false (toml parse errors embed the offending source line), and `load_config_file`'s own doc + test name describe the caller's NotFound policy instead of the function's behavior.

## 2. What actually works (verified)
— verified by `cargo test -p control-server --lib config` → `4 passed` (feature gate refuses the REAL repo product.toml; engineInfo carries product identity + live catalog checksum + `lumen-control/1`; operator file maps onto bind/port/read-only/fixed-token/`config.set-setting` grant).
— verified by `cargo test -p control-server --test e2e` → `6 passed` (incl. `e2e_full_session_through_the_app_stack`, `dirty_operator_toml_is_a_typed_config_error`, `operator_toml_parses_into_the_strict_shape`) — temp copy of the real product.toml with the flag flipped, then config → resolve → bind → serve_loop → CatalogDispatch.
— defaults cross-checked: policy.rs:135-141 `ServerSection::default` = `bind "127.0.0.1"`, `port 0` (ephemeral), `read_only false` — matches the doc claim at config.rs:122-124; `deny_unknown_fields` on all sections (policy.rs:112,124,147,156).

## 3. Findings
F-001 [S3]
Where: apps/control-server/src/config.rs:40-46 + 138-141
Code:
```
    /// The operator config file is not valid TOML / not the strict shape.
    ConfigParse {
        /// The path attempted.
        path: PathBuf,
        /// Redaction-safe detail.
        detail: String,
    },
```
What is wrong: `detail` is `toml::from_str::<PolicyFile>`'s error rendered with `to_string()`. The toml crate (1.1.8+spec-1.1.0, Cargo.lock:1150) formats parse errors with the offending SOURCE LINE embedded verbatim (`toml-1.1.8/src/de/error.rs:104-139`: "TOML parse error at line N, column M" + `let content = input.split('\n').nth(line)` printed with a caret). A malformed line that carries the fixed token (`auth.token = …` typo) echoes the secret into `detail`, which main.rs `eprintln!`s to STDERR — the same stream the structured JSON session log rides. Doc 05 §5.4 (line 541) says the token reaches only "the controlling terminal"; this path can leak it into a redirected stderr log. Why it matters: a security-adjacent doc claim ("Redaction-safe") that the code does not uphold. What correct looks like: strip or hash the embedded source line before Display, or document that parse errors may echo the file. Confidence: MEDIUM — Verification: read (toml crate source); live leak run UNVERIFIED (cargo build of the bin is outside this audit's budget).
F-002 [S3]
Where: apps/control-server/src/config.rs (whole file) + product.toml:167
Code:
```
control-server = false          # Phase 5+ [PROPOSAL]
```
What is wrong: nothing in the code — this is the shipped-state record for the hostile question. Every path through `resolve_server_config` starts with `require_feature` (config.rs:195), which with the repo product model always returns `FeatureDisabled` → the binary exits 2 before binding. The config file is therefore read (main.rs:182-201) by a process that then refuses to serve, and neither the binary nor its crate is in the release lane (`-p lumen-cli -p desktop`). The only live consumers of this file's functions are its own tests and apps/control-server/tests/e2e.rs (which flips the flag in a temp copy). Confidence: HIGH — Verification: read + executed tests (`repo_product_today_refuses_the_feature_gate`).
F-003 [S4]
Where: config.rs:121-124 (doc) and config.rs:322-331 (test)
Code:
```
    fn a_missing_config_file_is_the_strict_default_not_an_error() {
```
What is wrong: the doc says "a MISSING file is not [an error]" and the test is named "…is the strict default not an error", but `load_config_file` maps EVERY read error including NotFound to `Err(AppError::ConfigRead)` (config.rs:129-137) — the test's own body pins exactly that (`Err(AppError::ConfigRead { .. }) => {}`). The "missing = defaults" policy belongs to the caller (main.rs:190-192). Both texts describe main.rs, not this function. Confidence: HIGH — Verification: read + executed test.
F-004 [S4]
Where: config.rs:218-242 vs apps/control-server/src/main.rs:131-156 vs tests/e2e.rs:37-93
Code:
```
    fn root_schema() -> Result<Schema, AppError> {
```
What is wrong: the schema-load + product-load pipeline is implemented three times (config.rs tests, main.rs `load_product`, e2e.rs) with minor drift already visible (main.rs returns `ProductConfig` directly; the test helper returns `Result<Schema, AppError>` reusing `AppError::ConfigRead` for a schema file). Copy-paste seam, drift risk on the next product-pipeline change. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AppError (enum) | 31-67 | REAL | — | 6 typed variants; no stringly errors |
| AppError::exit_code | 69-82 | REAL | — | 2/3 mapping matches main.rs contract (4 comes from run()) |
| AppError: Display / Error | 84-119 | REAL | S3 | Display includes `detail` — the F-001 leak path |
| load_config_file | 128-142 | REAL | S3/S4 | Real parse; doc + test describe caller policy (F-003); detail echo (F-001) |
| require_feature | 149-159 | REAL | — | Some(true) required; executed against real product.toml |
| engine_info_from_product | 166-183 | REAL | — | Honest empties (plugins/decoders/sessions) documented as "no engine embedded"; executed |
| resolve_server_config | 191-207 | REAL | — | Gate → policy → strict → bind/port/policy/token; executed |
| tests ×4 + helpers | 209-332 | REAL | S4 | All executed; one name lies (F-003) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| repo_product_today_refuses_the_feature_gate | YES | Loads REAL product.toml+schema; asserts FeatureDisabled("…false…"). Doubles as a tripwire: it FAILS the day the Lead flips the flag — intentional but brittle-by-design |
| engine_info_carries_product_identity_and_catalog_checksum | YES | Exact assertions on identity, checksum, "lumen-control/1", host, feature mirror |
| resolve_maps_the_operator_file_onto_the_wire_config | YES | Field-by-field + `policy.check(ConfigSetSetting, true)` on a granted id |
| a_missing_config_file_is_the_strict_default_not_an_error | YES (body) | Body pins `Err(ConfigRead)` — the NAME is wrong, not the assertion |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`features.control-server` must be `true` (rule F2)" | config.rs:5-9 | REAL | require_feature:149-159 + executed test against real product.toml |
| "Unknown keys are errors; unknown grant ids are typed errors at STARTUP" | config.rs:12-14 | REAL | policy.rs deny_unknown_fields:112,124,147,156 + e2e `dirty_operator_toml_is_a_typed_config_error` (executed) |
| "Redaction-safe detail" | config.rs:44 | FALSE | toml 1.1.8 error Display embeds the offending line (crate source de/error.rs:104-139) — F-001 |
| "a MISSING file is not [an error]" (for load_config_file) | config.rs:122-123 | FALSE | Function errors on NotFound (L129-137); caller main.rs:190-192 owns the default policy — F-003 |
| "empty plugin/decoder/session lists (no engine is embedded — honest, not aspirational)" | config.rs:163-165 | REAL | L179-181 literal empties; contrast CLI host's un-annotated empties (types.rs F-004) |

## 7. Dead / duplicated / unreachable code
Unreachable as shipped: the entire app is double-gated off — (1) not in the release lane, (2) product feature false. Duplicated: product/schema loading in three places (F-004). No dead functions inside the file itself; every public item is called by main.rs or tests.

## 8. Dependencies & flags
`commands` (EngineInfo, catalog checksum), `control_protocol` (PolicyFile/ServerPolicy/ServerConfig/TokenSource/PROTOCOL_*), `product_config` (ProductConfig/load_file/Schema), `toml` 1.1.8, std only otherwise. No feature flags. Workspace lints (deny unwrap/expect/panic) hold — no `unwrap()` outside `#[cfg(test)]` unreachable-guarded matches.

## 9. Open questions for the next agent
- If the Lead flips `features.control-server = true`, `repo_product_today_refuses_the_feature_gate` breaks by design — is a follow-up card tracked for updating it?
- Does anything ever ship `lumen-control-server` (packaging/ has zero references)? If not, this whole app + control-protocol are test-only surface in the product sense.
- dispatch.rs (CatalogDispatch) — verify the "typed not-ready for everything but get-engine-info/list-commands" claim matches the served set; sibling audit exists for e2e.rs.
