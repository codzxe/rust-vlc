# AUDIT: apps/control-server/tests/e2e.rs
| Field | Value |
|---|---|
| Auditor | W02-A06 |
| Date | 2026-11-27 |
| Lines / functions | 432 / 7 helpers + 6 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test file) — runs in the standard `cargo test --workspace` gate (baseline shows it) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p control-server --test e2e` -> 6 passed, EXIT=0 (27.18s compile incl. product-config); baseline cargo-test-workspace.txt `tests/e2e.rs` -> 6 passed |

## 1. One-paragraph verdict
The e2e is not a synthetic echo chamber: `e2e_full_session_through_the_app_stack` binds a REAL `ControlServer` TcpListener on 127.0.0.1:0, runs the same `serve_loop` the `lumen-control-server` binary runs (verified against apps/control-server/src/main.rs imports and run.rs), and speaks real `lumen-control/1` over loopback TCP through the in-tree WS client (`TestClient::connect` = real HTTP upgrade + masked frames; reads via `read_frame_unmasked`). I executed all 6 tests first-hand — pass. The session test proves the honest-gap behavior a devil audit looks for: a GRANTED sensitive op reaches the dispatch and comes back `NotReady` (not a fake success), an UNGRANTED one is `Denied` with the connection surviving, and the log summary counters (`requests_served=3`, `ops_denied=1`) match the wire exchange exactly. Defects: the token-leak assertion only checks field NAMES (a value carrying the token would pass), the flipped product.toml temp copies are never deleted, and no test at any level exercises a frame above ~6 KB (so the WS 64-bit length form stays untested — cross-ref frame.rs F-002).

## 2. What actually works (verified)
- verified by `cargo test -p control-server --test e2e` -> `test result: ok. 6 passed; 0 failed` EXIT=0, including `e2e_full_session_through_the_app_stack` — real socket, real handshake, welcome/catalog/NotReady/Denied/engine-info/close-echo, real thread + `serve_loop(Some(1))`.
- verified by reading run.rs: `serve_loop` is the same function main.rs imports (apps/control-server/src/main.rs:31 `use control_server::serve_loop`), accept-one-at-a-time, cap-aware.
- Baseline cross-check: audit/runs/baseline/cargo-test-workspace.txt (control-server section) `Running tests/e2e.rs ... 6 passed`.
- Catalog listing size estimate (for the length-form question): `CommandListing` (crates/commands/src/catalog/command.rs:675-689) carries id/class/since/deprecated/requires only ≈ 90 B/row × 65 rows ≈ 6 KB — every frame in this e2e fits the 7/16-bit length forms.

## 3. Findings
F-001 [S3]
Where: e2e.rs:408-410
Code:
```
    for (key, _) in &fields {
        assert_ne!(*key, "token", "the log stream must never carry the token");
    }
```
What is wrong: the assertion proves no field is NAMED "token", not that the log stream never CARRIES the token. A bug that puts `e2e-fixed-token` into a value (an error detail, a redaction failure in a message field) passes this test. The comment above it (L393: "no token, V8") claims more than the check does. Why it matters: this is the only app-level guard for the V8 redaction claim; it is name-shape-only. What correct looks like: also assert every field value does not contain the token string: `assert!(!value.contains("e2e-fixed-token"))`.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: e2e.rs:62-86 (vs 181, 200, 220, 241, 411)
Code:
```
    let path = std::env::temp_dir().join(format!(
        "lumen-control-server-e2e-product-{}-{call}.toml",
        std::process::id()
    ));
    if let Err(err) = std::fs::write(&path, flipped) {
        unreachable!("cannot write {}: {err}", path.display());
    }
    path
```
What is wrong: the flipped product copies are never removed — every other temp artifact gets `std::fs::remove_file` (config paths at L181/200/220/241/411) but `flipped_product()`'s output (3 calls per run across the tests) litters /tmp per test run. Harmless locally, noisy on CI hosts.
Confidence: HIGH — Verification: read (grep for remove_file: only config paths).

F-003 [S4]
Where: e2e.rs:311-324 (whole-suite scope)
Code:
```
    send(&mut client, &request(1, r#""cmd":"system.list-commands""#));
    match next_frame(&mut client) {
        ServerFrame::Response { response } => match response.result {
            Ok(CommandOutcome::Completed(CommandResult::CommandListings { commands })) => {
                assert_eq!(commands.len(), 65);
```
What is wrong: the largest wire payload in the app-level e2e is the ~6 KB listing; nothing at the app level exercises a >65535-byte frame, near-cap (1 MiB) frame, or fragmentation — those exist only in the crate-level suites (conformance: fragmentation yes; 64-bit length form NO — see frame.rs F-002). The app stack's send_json path (no write-side cap) is therefore never stress-tested with a big response.
Confidence: HIGH — Verification: read + size estimate above; `rg` for >65535 payloads across both crates -> none.

F-004 [S4]
Where: e2e.rs:54-61, 98-106
Code:
```
//! pid-anchored path raced write/truncate/read on CI (run 43409155006 —
```
What is wrong: the race-fix comments cite CI run IDs (43409155006, 43451838615) that cannot be verified from this repo. The FIX itself is real and sound (per-call AtomicU32 counter in the filename, L63-65 and L109-111); only the historical evidence is unverifiable. UNVERIFIED (CI logs not accessible).
Confidence: HIGH (fix) / UNVERIFIED (CI narrative) — Verification: read.

## 4. Per-function table (helpers; file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| repo_root | 32-34 | REAL | — | CARGO_MANIFEST_DIR-based |
| root_schema | 37-47 | REAL | — | loads the COMMITTED product.schema.json |
| flipped_product | 62-86 | REAL | S4(F-002) | honest single-flip assert (L73-77); never cleaned up |
| load_product | 89-95 | REAL | — | schema-validated load |
| write_config | 107-119 | REAL | — | per-call unique temp names (race fix) |
| strict_config | 123-128 | REAL | — | fixed token + one granted sensitive op |
| send / next_frame / hello / request | 131-162 | REAL | — | thin, honest helpers over the real client |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| operator_toml_parses_into_the_strict_shape | YES | field-by-field compare of the parsed file |
| dirty_operator_toml_is_a_typed_config_error | YES | broken TOML and unknown key each hit a distinct typed arm (deny_unknown_fields) |
| a_typoed_sensitive_grant_is_a_typed_startup_error | YES | typo'd grant must surface in the error string |
| a_non_loopback_bind_in_the_operator_config_is_refused_at_bind | YES | 0.0.0.0 must yield NonLoopbackBind at bind, not at accept |
| e2e_full_session_through_the_app_stack | YES | real server+client+frames; welcome fields, live 65-command pin, NotReady (not fake success) for granted op, Denied+survives for ungranted, close echo, session counters 3/1, capped — any broken link hits unreachable!/assert |
| the_production_sink_renders_one_parseable_json_line_per_record | YES | parses the rendered line back as JSON and checks keys |
Mutation reasoning (no edits made): if the dispatch faked success for `config.set-setting`, the `Err(NotReady)` match at L336-340 fails; if the policy ignored `sensitive_allow`, L355-360 fails; if the log carried the session twice, L399 fails. Weak spot: F-001 (token value never scanned).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Drives the FULL app stack exactly as `main` assembles it" | e2e.rs:5-9 | REAL | same `resolve_server_config`/`ControlServer::bind`/`serve_loop`/`CatalogDispatch` imports as main.rs:21-31; run passed |
| "the repo's own flag is honestly `false` until the Lead ratifies" | e2e.rs:6-8 | REAL | product.toml:167 `control-server = false`; config.rs test `repo_product_today_refuses_the_feature_gate` (baseline) |
| "The listener is already bound, so the client cannot race it" | e2e.rs:271-273 | REAL | bind at L263 precedes thread spawn; TestClient::connect after rx_ready |
| "no token, V8" (log redaction) | e2e.rs:393 | SHALLOW | name-only check — F-001 |
| "65 commands, from the live table" | e2e.rs:311-318 | REAL | asserted against live dispatch output; matches catalog_shape.rs pin (65) |

## 7. Dead / duplicated / unreachable code
None. All helpers used. No #[ignore], no cfg gates on the tests.

## 8. Dependencies & flags
- Uses control_server lib (serve_loop, resolve_server_config, CatalogDispatch, MemoryLog), control_protocol TestClient/frames, commands envelope types, product_config, serde_json, std mpsc/thread. No feature flags.

## 9. Open questions for the next agent
- Does any REAL host compose `control-server` with an engine-backed dispatch (doc 01 "optionally embeds the control server")? The app dispatch is catalog-only + NotReady — check the desktop composition card's status.
- If a future catalog listing (or engine-info with codec lists) ever exceeds 65535 bytes, `send_json`'s uncapped write + the untested 64-bit length form (frame.rs F-002) meet in production. Add a size regression test at the dispatch boundary.
