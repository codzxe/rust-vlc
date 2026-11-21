# AUDIT: apps/control-server/src/config.rs
| Field | Value |
|---|---|
| Auditor | W13-A05 |
| Date | 2026-11-21 |
| Lines / functions | 333 / 9 items (1 enum + 3 impls + 4 fns + tests: 4 tests + 3 helpers) |
| File verdict | REAL (executed in test lanes; UNREACHABLE from shipped entry points — the app ships nowhere and its product gate is off) |
| Reachable from shipped entry point? | NO. Proof: product.toml:167 `control-server = false` (rule F2) → `resolve_server_config` → `require_feature` refuses → the binary exits 2 before binding (main.rs:174-210, 178); and the `lumen-control-server` binary is absent from every packaging payload (W01-A01's Cargo.toml audit: packaging.toml ships BIN_CLI + BIN_TAURI only; 0 grep hits for the binary name outside the app's own files). The code IS compiled in every workspace build and its 4 tests executed in the baseline no-fail-fast run (orchrun-test-workspace-nofailfast.log.txt:1457-1459, 1467). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `Read` over the full file + apps/control-server/src/main.rs (the caller — verifies the NotFound→defaults composition and the exit-code contract 0/2/3/4); `rg -n "control-server" product.toml` (line 167: `false # Phase 5+ [PROPOSAL]`); `rg "control_server::config|config::tests::" orchrun log` (4/4 tests ok at 1457-1467); prior-wave context: audit/files/apps/control-server/Cargo.toml.audit.md (W01-A01 — the unreachability proof and the exit-2 honesty); `rg "control-server" scripts/packaging/` (no hits — not shipped); `sed -n` quote verification for each finding. |

## 1. One-paragraph verdict
A small, disciplined config-resolution layer that does exactly what its two-layer story claims: the product-model gate (rule F2) fails closed (`require_feature` refuses on both `false` and absent flags — the repo's own product.toml makes the whole app exit 2 at startup, and its test pins that current state as a change-control tripwire), the operator file is strictly parsed with typed errors, unknown grants are typed startup errors, and the advertised `engineInfo` is honest (empty decoder/session lists — no engine is embedded). All four tests execute against the REAL repo product.toml and schema and are mutation-resistant. The defects are documentation-level: `load_config_file`'s doc contract states the opposite of its behavior on a missing file (the function errors; the defaults posture lives in main.rs), the pinning test's name asserts the opposite of its body, and the composition double-reads the config file. The elephant is inherited, not local: none of this ships (W01-A01's S3) — but this file is precisely where that unshipped surface fails closed.

## 2. What actually works (verified)
— The F2 gate EXECUTED against the real repo state: `config::tests::repo_product_today_refuses_the_feature_gate ... ok` (orchrun log 1458) — loads the real product.toml + product.schema.json and asserts the FeatureDisabled refusal naming "false" (the repo's `control-server = false`, product.toml:167). Flipping the product flag without touching this test turns the suite red — a working change-control pin.
— Full resolution path EXECUTED: `config::tests::resolve_maps_the_operator_file_onto_the_wire_config ... ok` (log 1467) — synthetic flag flip + a complete PolicyFile (bind `::1`, port 4355, read_only, fixed token, a `sensitive_allow` grant) mapped onto ServerConfig with all five fields asserted, including `policy.check(ConfigSetSetting, true).is_ok()`.
— Identity propagation EXECUTED: `engine_info_carries_product_identity_and_catalog_checksum ... ok` (log 1459) — display_version, live catalog checksum, `lumen-control/1`, host `control-server`, and the enabled-features mirror all pinned.
— Missing-file behavior pinned: `a_missing_config_file_is_the_strict_default_not_an_error ... ok` (log 1457) — `load_config_file` on a nonexistent path returns `AppError::ConfigRead` (the defaults fallback is main.rs:190-192's `NotFound` arm, verified by read).
— Exit-code contract coherent: AppError::exit_code (2 config/product, 3 bind) matches main.rs's header contract (0/2/3/4) — the serve-loop's 4 is main.rs's literal (main.rs:259), correctly outside AppError.

## 3. Findings

### F-001 [S3] `load_config_file`'s documented error contract is false: "a MISSING file is not [an error]" — the function errors on a missing file, and its own test pins that
Where: apps/control-server/src/config.rs:121-127 (doc) vs 128-142 (behavior), pinned by the test at 321-331
Code:
```
/// Load and parse `control-server.toml`. Unknown keys are errors (the
/// [`PolicyFile`] serde contract); a MISSING file is not — the strict
/// defaults (loopback, ephemeral port, generated token, every sensitive
/// op denied) are the documented no-config posture.
```
What is wrong: read literally — and this is a `pub` library function of the `control_server` crate — the doc says `load_config_file` does not error on a missing file. It does: `std::fs::read_to_string` on a missing path fails and maps to `AppError::ConfigRead` (129-137), which is exactly what the test asserts (327-329: `Err(AppError::ConfigRead { .. })` for `does-not-exist-control-server.toml`). The strict-defaults posture is the CALLER's composition (main.rs:182-192: pre-read the file, `NotFound` → `PolicyFile::default()`); the doc describes main's behavior on a function that does not have it.
Why it matters: a contract statement that lies about the error surface of a public function — an integrator who trusts it will not handle `ConfigRead` for the no-config case and will crash or misreport on a clean no-config deployment. Blast radius today is small (the app ships nowhere), but the doc/behavior split is exactly the kind tests cannot catch when the test's own comment has to explain the doc away (322-325).
What correct looks like: doc states "errors on any read failure including NotFound — the no-config defaults are the caller's composition (see main)"; better: move the NotFound→default decision into `load_config_file` as an explicit `Ok(PolicyFile::default())` arm or a distinct `ConfigMissing` variant.
Confidence: HIGH — Verification: read of the doc, the function, the test, and main.rs:182-192.

### F-002 [S4] Test name asserts the opposite of its body
Where: apps/control-server/src/config.rs:321-331
Code:
```
    #[test]
    fn a_missing_config_file_is_the_strict_default_not_an_error() {
        // load_config_file's missing-file path: the caller (main)
        // treats NotFound as "use defaults"; other read errors are
        // typed. Here we pin the typed error for an unreadable path.
```
What is wrong: the name promises "missing file is not an error"; the body asserts that it IS an error (`Err(AppError::ConfigRead { .. })`). The comment concedes the mismatch. A reader scanning test names (or a failure report) gets the inverted fact.
Why it matters: test names are documentation; this one documents the composition (main's behavior) under a test of the function's opposite behavior.
What correct looks like: rename to `a_missing_config_file_is_a_typed_config_read_error` (and, if desired, a separate main-level test for the defaults fallback).
Confidence: HIGH — Verification: read.

### F-003 [S4] The config file is read twice (main pre-reads, then `load_config_file` re-reads)
Where: apps/control-server/src/config.rs:129 (the re-read) composed with apps/control-server/src/main.rs:182-192 (the pre-read)
Code:
```
pub fn load_config_file(path: &Path) -> Result<PolicyFile, AppError> {
    let text = match std::fs::read_to_string(path) {
```
What is wrong: to distinguish NotFound from other read errors, main.rs reads the whole file first (182) and then `load_config_file` reads it again (129) — two reads and a benign TOCTOU window (file deleted between reads → main's default path; file created between → parse path; both outcomes typed, so no correctness hole, just double I/O and a race that changes which typed path runs).
Why it matters: hygiene; the shape also forces the misplaced doc of F-001.
What correct looks like: one read inside `load_config_file`, with `ErrorKind::NotFound` mapped to the default (or a `ConfigMissing` variant the caller maps).
Confidence: HIGH — Verification: read of both sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum AppError` (+`exit_code`, `Display`, `Error`) | 31-119 | REAL | — | 6 variants, redaction-safe details (ConfigParse carries `detail: String` — no token echo paths here; the token never flows through this file); exit codes 2/3 match main.rs's contract; Display aggregates ProductLoad diagnostics one-per-line. |
| `load_config_file` | 128-142 | REAL | S3 (F-001), S4 (F-003) | Strict TOML → `PolicyFile` (unknown keys are serde errors per the policy crate's contract); typed ConfigRead/ConfigParse. Doc contract false on missing files (F-001). Executed by test (log 1457). |
| `require_feature` | 149-159 | REAL | — | Rule F2: `Some(true)` passes; `Some(false)` and `None` both refuse with distinct details. Executed (log 1458). Fails closed — the gate that makes the unshipped app refuse startup. |
| `engine_info_from_product` | 166-183 | REAL | — | Identity + live catalog checksum + protocol string; empty plugins/decoders/sessions are TRUE (no engine embedded — the honest opposite of the CLI's hard-coded engine-info literal, INDEX standing defect). `lumen-allow-literal` on `host: "control-server"` is a structural role name — justified. Executed (log 1459). |
| `resolve_server_config` | 191-207 | REAL | — | Gate order verified: feature flag FIRST (195), then `ServerPolicy::from_file` (unknown grant ids → typed PolicyResolve), then `ServerConfig::strict` (deny-by-default) with the operator's bind/port/policy/token overrides. Token: Fixed vs Generated, no third mode. Executed (log 1467). |
| `tests::repo_root` / `root_schema` / `load_repo_product` | 214-242 | REAL | — | Test scaffolding over the REAL repo product.toml + product.schema.json (no fixtures, no mocks of the subject). |
| `tests::repo_product_today_refuses_the_feature_gate` | 244-256 | REAL | — | Mutation-resistant (a gate that stops refusing hits `unreachable!`); doubles as a change-control tripwire on product.toml:167. |
| `tests::engine_info_carries_product_identity_and_catalog_checksum` | 259-281 | REAL | — | Pins all five advertised fields incl. the exact protocol string. |
| `tests::resolve_maps_the_operator_file_onto_the_wire_config` | 283-319 | REAL | — | The full mapping incl. the sensitive-grant policy check. |
| `tests::a_missing_config_file_is_the_strict_default_not_an_error` | 321-331 | REAL | S4 (F-002) | Pins the (mis-documented) behavior; name inverted (F-002). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| repo_product_today_refuses_the_feature_gate | YES | A `require_feature` that passes `false`/absent returns Ok → `unreachable!("feature gate did not refuse")`. |
| engine_info_carries_product_identity_and_catalog_checksum | YES | Exact-value asserts on all five fields; a checksum/identity/protocol regression fails. |
| resolve_maps_the_operator_file_onto_the_wire_config | YES | Five field asserts incl. the policy grant check; a mapping that drops read_only or misroutes the token fails. |
| a_missing_config_file_is_the_strict_default_not_an_error | YES | Pins the error variant — but its name inverts the fact (F-002). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`features.control-server` must be `true` (rule F2 … the flag flips together with the feature-matrix row when the Lead ratifies)" | module docs 5-9 | REAL | require_feature (149-159) + the repo-flag-false test (log 1458) + product.toml:167. |
| "Unknown keys are errors; unknown grant ids are typed errors at STARTUP" | module docs 10-14 | REAL | PolicyFile serde contract + `ServerPolicy::from_file` → PolicyResolve (196-197); the resolve test exercises the happy path, the refusal shape is the policy crate's (audited W-wave: crates/control-protocol/src/policy.rs.audit.md). |
| "a MISSING file is not [an error] — the strict defaults … are the documented no-config posture" (on `load_config_file`) | this file 121-124 | FALSE (for this function) | The function errors (129-137); its test pins the error (327-329); the defaults are main.rs:190-192 (F-001). |
| "empty plugin/decoder/session lists (no engine is embedded — honest, not aspirational)" | this file 163-165 | REAL | Read (179-181); the truthful inverse of the CLI's hard-coded engine-info (INDEX standing defect). |
| Exit contract "0 ok · 2 config/product · 3 bind · 4 serve loop" | this file 70-71 + main.rs:3-9 | REAL | exit_code() (72-81) covers 2/3; 4 is main.rs:259's literal; `--help`/cap → 0 (main.rs:163-166, 249-256). |

## 7. Dead / duplicated / unreachable code
- No dead code locally. The file's unreachability is app-level (inherited W01-A01 S3: no packaging payload, flag off) — this file is the enforcement point that makes the unshipped app fail closed, which is the correct posture.

## 8. Dependencies & flags
- Deps: commands (EngineInfo), control_protocol (PolicyFile/ServerPolicy/ServerConfig/TokenSource/protocol consts — all audited in an earlier wave), product_config (ProductConfig), toml (parse), serde_json (not used in THIS file — it is used by the app's dispatch/logging files; manifest-level, already covered by W01-A01). No feature gates, no unsafe, no panics in production paths (tests use `unreachable!` idiomatically).

## 9. Open questions for the next agent
- A07 (control-server/src/main.rs is in YOUR list): the double-read shape (F-003) and the NotFound composition live in your file — please cross-reference; also `parse_args`/`value_after`/`usage` are yours.
- Whoever owns the Lead's feature-ratification decision: this file's gate test (`repo_product_today_refuses_the_feature_gate`) will FAIL the moment product.toml:167 flips — that is by design (change control), but the flip commit must update the test in the same change or CI blocks it.
