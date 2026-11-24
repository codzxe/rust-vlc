# AUDIT: crates/plugin-host-wasm/src/imports.rs
| Field | Value |
|---|---|
| Auditor | W12-A04 |
| Date | 2026-11-30 |
| Lines / functions | 917 / 15 (host fns) + 10 registered imports |
| File verdict | REAL (executed via tests) but UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO — no app depends on plugin-host-wasm (proof below) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 3 |
| Commands run | `cargo test -p plugin-host-wasm --test plugin_sandbox_security` → 9 passed, exit 0; `... --test plugin_examples_conformance --test plugin_hello_world` → 7 passed, exit 0; rg reachability sweeps (below) |

## 1. One-paragraph verdict
This is the strongest code audited in my domain: a genuine wasmi import table with typed status codes, per-permission module gating, bounds-checked guest-memory I/O, a manifest-lie check, an escalation ledger that faults the instance at 3 denials, and real wasm-guest integration tests that execute every import. It is also dead product code: **no shipped entry point reaches it** — no app's manifest or source references plugin-host-wasm. The remaining defects are an unenforced host-heap accounting claim (§5.4 rows 8–9), a write-only UI panel sink, and silent truncation on two out-parameter paths.

## 2. What actually works (verified)
— verified by `cargo test -p plugin-host-wasm --test plugin_sandbox_security` → `test result: ok. 9 passed` (fs_open_slurp_is_bounded, oob_probes_trip_the_disable_threshold, plugin_permission_escalation, plugin_host_pointer_smuggling, plugin_io_abuse, plugin_sandbox_escape, plugin_memory_exhaustion, plugin_trap_loop, devices_list_is_bucket_metered) — real wasm guests built by the hand-rolled emitter in tests/common/mod.rs drive these imports.
— verified by `cargo test -p plugin-host-wasm --test plugin_examples_conformance --test plugin_hello_world` → 7 passed (manifest reporting, api-version negotiation, ui panel conformance, lumenext CLI round-trip).
— Baseline corroborates: audit/runs/baseline/cargo-test-workspace.txt:4884-5022 (`### PACKAGE plugin-host-wasm exit_code=0`, 70 tests passed).

## 3. Findings
F-001 [S2]
Where: crates/plugin-host-wasm/src/imports.rs:228-245 (and whole file)
Code:
```
pub fn build_linker(engine: &Engine, grants: &Grants) -> Result<Linker<HostState>, HostError> {
    let mut linker = <Linker<HostState>>::new(engine);
    define_host(&mut linker)?;
```
What is wrong: Not this function — its reachability. The only caller chain is `WasmInstance::spawn` (instance.rs:80,110) → `build_linker`; `WasmInstance::spawn` has zero non-test callers. `rg 'plugin_host_wasm|WasmInstance|ExtensionStore' apps/**/*.rs` → No matches; `rg 'plugin[-_]host[-_]wasm' **/Cargo.toml` → only plugin-host-wasm's own manifest and a comment in plugin-api's. Release lane (`cargo build --release -p lumen-cli -p desktop`, baseline cargo-build-release-*.txt) never compiles this crate in. Why it matters: the entire "sandboxed WASM extension host" surface — the security machinery this file implements — protects nothing in any shipped binary; it is elaborate dead code (the crate's own lib.rs:8 admits "nothing depends on this crate but the apps that embed it", and none do). What correct looks like: either an app embeds ExtensionStore/WasmInstance or docs state the host is unbuilt-in. Confidence: HIGH — Verification: ran (rg sweeps, both quoted).

F-002 [S2]
Where: crates/plugin-host-wasm/src/imports.rs:509-529; crates/plugin-host-wasm/src/limits.rs:86-89
Code:
```
let cap = caller.data().limits.io_transfer_bytes;
let mut content = Vec::with_capacity(meta.len().min(cap) as usize);
let mut bounded = file.take(cap + 1);
```
What is wrong: fs_open slurps the whole file into a host-side Vec in the handle table — *outside* wasmi's store limiter (memory_bytes caps only guest linear memory). Per instance: 8 handles × 16 MiB (IO_TRANSFER_BYTES) = 128 MiB host heap. Doc 06 §5.4 rows 8–9 (4 active instances, 256 MiB total extension heap) are claimed via `HOST_MAX_ACTIVE_INSTANCES`/`HOST_TOTAL_HEAP_BYTES` — both constants are referenced NOWHERE else in the tree (rg: only limits.rs:86,89 definitions). Aggregate extension host-heap is unbounded up to instances×128 MiB. Why it matters: the §5.4 accounting claim is unenforced paper; a multi-instance embedder gets unaccounted host memory. What correct looks like: a shared counter enforced in fs_open/insert. Blast radius today: zero (F-001 — nothing embeds it). Confidence: HIGH — Verification: read + rg.

F-003 [S3]
Where: crates/plugin-host-wasm/src/imports.rs:848-852
Code:
```
let descriptor = String::from_utf8_lossy(&bytes).to_string();
caller
    .data_mut()
    .panels
    .push(crate::state::PanelRecord { descriptor });
```
What is wrong: `ui_register_panel` validates nothing — any ≤64 KiB byte soup is lossily stringified and pushed. The descriptor is never parsed, never schema-checked, and `panels` is consumed only by `WasmInstance::panels()` (instance.rs:151) whose only caller is tests/plugin_examples_conformance.rs:337. The `ui-surface` permission buys a plugin zero user-visible UI. Why it matters: a "UI panel" surface that renders nothing is SHALLOW; the cap check (`panels.len() >= 8`) guards a write-only Vec. What correct looks like: parse against the panel descriptor schema at the seam, or document the surface as record-only. Confidence: HIGH — Verification: read + rg callers.

F-004 [S3]
Where: crates/plugin-host-wasm/src/imports.rs:897-899 (devices_list); 780-782 (http_get)
Code:
```
let inventory = caller.data().services.device_inventory.clone();
let n = (inventory.len() as u64).min(out_cap as u64) as usize;
match write_guest(&mut caller, out_ptr, &inventory.as_bytes()[..n])
```
What is wrong: when the payload exceeds the guest's out_cap, both `devices_list` and `http_get` (line 780: `let n = (resp.body.len() as u64).min(out_cap as u64)`) silently return Ok(0) with a truncated payload — mid-JSON cuts for devices_list. The typed `ETooLarge` status exists (api.rs:337 "Over the single-transfer cap") and fs_open uses it for the same condition (imports.rs:518-522); these two paths do not. Why it matters: a guest cannot distinguish truncation from completion without an out_len==out_cap heuristic; silent data corruption of the descriptor/response. What correct looks like: `if inventory.len() as u64 > out_cap as u64 { return ETooLarge }`. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/plugin-host-wasm/src/imports.rs:746-764
Code:
```
if !caller.data_mut().http_admit() {
...
if !caller.data_mut().io_bucket.take(out_cap as u64) {
```
What is wrong: http_get admits into the per-minute window and charges the I/O bucket *before* the transport seam; with the v1 default NoTransport (ETransport at line 770-775) a guest burns 1/32 of its request budget and out_cap tokens per refused call while zero bytes move. Also fs_read (579) charges buf_cap before handle validation — bogus-handle probes burn quota. Defensible as anti-hammering, but the asymmetry (fs_open's ETooLarge path charges nothing) is undocumented. Confidence: MEDIUM — Verification: read.

F-006 [S4]
Where: crates/plugin-host-wasm/src/imports.rs:802-807
Code:
```
Err(crate::http::TransportError::Failed(_)) => {
    caller
        .data_mut()
        .trace_import(import, GuestStatus::EIo.code());
    Ok(GuestStatus::EIo.code())
```
What is wrong: the transport failure detail is discarded (`Failed(_)`); neither the guest nor the import trace records why a fetch failed — no error context, contra the crate's own "every failure is a typed HostError with an actionable message" claim (lib.rs). Confidence: HIGH — Verification: read.

F-007 [S4]
Where: crates/plugin-host-wasm/src/imports.rs:251-268
Code:
```
let Some(spec) = crate::api::lookup_module(&module_name) else {
    return Err(HostError::UnknownImportModule {
```
What is wrong: import-policy checking is per-module only; a guest importing `lumen:v1.fs::fs_write` (unknown function in a granted module) passes check_import_policy and dies later at linker instantiation with a generic Instantiation error. Function-level catalog lookup exists (api.rs `lookup`) and is unused here. Naming/diagnostic quality only — fail-closed either way. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory, >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| guest_memory | 87-89 | REAL | — | wasmi export lookup |
| read_guest | 99-143 | REAL | — | bounds-checked; OOB/missing-memory count escalations |
| ReadErr/escalation_or_status | 146-168 | REAL | — | threshold-3 fault; test-verified (oob_probes_trip_the_disable_threshold) |
| write_guest / _u32 / _u64 | 171-194 | REAL | — | bounds-checked writes; negative-ptr E_ARG |
| deny | 199-215 | REAL | — | E_PERM + ledger + threshold fault |
| build_linker | 228-245 | REAL-UNPROVEN | S2 | correct gating by grants; unreachable in shipped builds (F-001) |
| check_import_policy | 251-268 | REAL | S4 | module-level only (F-007) |
| define_host/api_version | 272-278 | REAL | — | constant major; exercised by hello_world test |
| define_host/report_manifest | 283-353 | REAL | — | parses + identity-compares; lie → escalation (F-001 caps impact) |
| define_log/log_write | 362-406 | REAL | — | level 0..=4, 4096 B cap, bucket-charged |
| define_fs/fs_open | 410-552 | REAL | S2 | bounded jail slurp; host-heap accounting gap (F-002); TOCTOU narrowed honestly |
| define_fs/fs_read | 555-614 | REAL | S4 | quota-before-handle-validation (F-005) |
| define_fs/fs_size | 617-649 | REAL | — | u64 out-param; handle-validated |
| define_fs/fs_close | 652-678 | REAL | — | double-close → E_HANDLE; unit-tested |
| define_http/http_get | 686-813 | REAL | S3/S4 | allowlist→rate→caps→seam chain; silent truncation (F-004), quota order (F-005), swallowed error (F-006) |
| define_ui/ui_register_panel | 817-861 | SHALLOW | S3 | write-only sink (F-003) |
| define_devices/devices_list | 865-917 | REAL | S3 | bucket-metered; silent truncation (F-004) |
| FaultKind/HostFault impls | 48-80 | REAL | — | Display + wasmi HostError trait |

## 5. Test honesty (no tests in file; crate integration suites cover it)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| plugin_sandbox_security::fs_open_slurp_is_bounded | YES | asserts ETooLarge + no host over-read past cap (ran: ok) |
| plugin_sandbox_security::oob_probes_trip_the_disable_threshold | YES | asserts instance faults after 3 OOB probes (ran: ok) |
| plugin_sandbox_security::plugin_permission_escalation | YES | undeclared import must fail instantiation (ran: ok) |
| plugin_examples_conformance::ui_panel_example_passes_conformance | NO (weakly) | verifies register returns Ok + panel recorded — would pass even if panels were never rendered (F-003) |
| plugin_hello_world::hello_world_extension_runs_and_reports_manifest | YES | guest runs, manifest compared against install identity (ran: ok) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "host implementations of every `lumen:v1.*` import (doc 06 §5.2–§5.3)" | imports.rs:1-2 | REAL | 10 catalog imports (api.rs V1_IMPORTS) ↔ 10 func_wrap registrations; tests execute them |
| "reaching the session threshold faults the instance and disables the extension" | imports.rs:15-17 | REAL (fault) / UNVERIFIED (disable) | fault: oob_probes test; the *disable* (extension-level) lives in supervisor.rs — not exercised from this file, not run by me |
| ui-surface grants panel registration | doc 06 §2.2 via api.rs:141 | SHALLOW | descriptor stored, never rendered (F-003) |

## 7. Dead / duplicated / unreachable code
The whole file is unreachable from shipped binaries (F-001) while being real, tested code — the purest UNREACHABLE verdict case. HOST_TOTAL_HEAP_BYTES / HOST_MAX_ACTIVE_INSTANCES (limits.rs:86-89) are dead constants feeding nothing (F-002).

## 8. Dependencies & flags
wasmi 2.0.0 (registry), plugin-api (path). No features gate this file. No unsafe. Lints: workspace deny set inherited (crates/* member).

## 9. Open questions for the next agent
- Does any roadmap doc promise "extensions load in the desktop app"? If a user-facing doc says plugins work end-to-end, F-001 escalates to S0 (doc-vs-product fraud). Check docs/ + feature-matrix rows for plugin claims (orchestrator parity-report shows web 0/252 verified).
- supervisor.rs disable path: does anything call ExtensionSupervisor outside tests?
