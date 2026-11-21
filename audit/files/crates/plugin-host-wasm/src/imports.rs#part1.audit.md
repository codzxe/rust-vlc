# AUDIT: crates/plugin-host-wasm/src/imports.rs#part1 (lines 1–407)
| Field | Value |
|---|---|
| Auditor | W12-A02 |
| Date | 2026-11-21 |
| Lines / functions | 407 of 918 (chunk) / 19 |
| Chunk boundary | part1 = lines 1–407 (module docs, HostFault plumbing, guest-memory helpers, `build_linker`, `check_import_policy`, `define_host`, `define_log`); part2 = lines 408–918 (`define_fs`, `define_http`, `define_ui`, `define_devices`) — boundary taken at the `-- lumen:v1.fs --` section header (line 408), the only function-boundary split near the 918/2 midpoint. A03 owns part2. |
| File verdict | UNREACHABLE (real, executed end-to-end by 5 test suites in baseline; no shipped entry point links the crate) |
| Reachable from shipped entry point? | NO at crate level; YES in-crate — `instance.rs:80` calls `check_import_policy`, `instance.rs:110` calls `build_linker`; `WasmInstance::spawn/invoke` exercised by tests/plugin_hello_world, plugin_sandbox_security, plugin_crash_isolation, plugin_examples_conformance, plugin_install_lifecycle (all green: orchrun-test-workspace-nofailfast.log.txt:4557-4609). No non-test caller exists anywhere: `plugin-host-wasm` has zero dependents (rg over **/Cargo.toml → only its own manifest); `install.rs:33` mentions `WasmInstance` in a doc comment only; `bin/lumenext.rs` never spawns an instance. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | sed -n quotes (all verified); grep `ESCALATION_DISABLE_THRESHOLD` → limits.rs:93 = 3; grep `build_linker\|check_import_policy\|HostFault` in instance.rs → :18,:80,:110,:328,:344,:361; grep `Grants::has`/`lookup_module` in api.rs → :307,:424-433 (Base always granted); grep `4096\|2048\|MAX_` in api.rs → no cap constants; rg `4096\|2048` docs/06-plugin-architecture.md → no matches; grep part1 range for `unwrap\|expect(\|panic!\|todo!` → none; baseline log checks above. |

## 1. One-paragraph verdict
This is the real host-side half of the `lumen:v1.*` ABI: bounds-checked guest-memory access with per-import caps, a per-manifest-grant linker table, an import-policy pre-check that refuses undeclared modules before any guest code runs, and an escalation ledger that turns repeated denials into a typed host fault. The code is disciplined — zero unwrap/expect/panic, every refusal is a typed `GuestStatus`, allocations bounded by `min(per-import cap, io_transfer_bytes)` — and the baseline proves it executing (hello-world extension reports its manifest; OOB probes trip the disable threshold; permission escalation is refused at instantiation). But it is a host for guests nobody runs: no shipped binary links `plugin-host-wasm`, so every import here is reachable only from the crate's own test suites. Two real defects in-range: OOB *write* probes are silently unaccounted (reads count toward the disable threshold, writes never do), and per-import buffer caps are magic numbers that the limits-layer documentation points to a nonexistent constant for.

## 2. What actually works (verified)
— `hello_world_extension_runs_and_reports_manifest ... ok` (orchrun-test-workspace-nofailfast.log.txt:4584) — `report_manifest` (lines 272-358) executes against a real wasmi guest end to end.
— `plugin_sandbox_escape ... ok` (:4609) and `oob_probes_trip_the_disable_threshold ... ok` (:4602) — `read_guest` OOB/negative paths, security events, and `escalation_or_status` threshold faulting execute.
— `plugin_permission_escalation ... ok` (:4607) — `check_import_policy` refuses undeclared `lumen:v1.fs` import with `HostError::UndeclaredImport` before instantiation; `build_linker` gating verified.
— `metadata_provider_example_passes_conformance` + `ui_panel_example_passes_conformance` (:4574-4575) — the committed example extensions run through this import table.

## 3. Findings

### F-001 [S3] OOB write probes never reach the escalation ledger — asymmetric with the read path, so a write-hammering guest is never disabled
Where: crates/plugin-host-wasm/src/imports.rs:171-184
Code:
```rust
fn write_guest(caller: &mut Caller<HostState>, ptr: i32, data: &[u8]) -> Result<(), i32> {
    if ptr < 0 {
        return Err(GuestStatus::EArg.code());
    }
    if data.is_empty() {
        return Ok(());
    }
    let Some(memory) = guest_memory(caller) else {
        return Err(GuestStatus::EOob.code());
    };
    memory
        .write(&mut *caller, ptr as usize, data)
        .map_err(|_| GuestStatus::EOob.code())
}
```
What is wrong: `write_guest` returns `Err(EOob)` for a missing memory export and for an out-of-bounds write WITHOUT calling `ledger.record_escalation` — compare `read_guest` (imports.rs:130-141), which records a security event ("out-of-bounds guest buffer (sandbox escape attempt)") and routes through `escalation_or_status` so the third probe faults the instance (`ESCALATION_DISABLE_THRESHOLD = 3`, limits.rs:93). Callers of `write_guest` (fs_read:599, http_get:781, devices_list:899 — part2 range) only `trace_import` the status, never escalate. The read path's own doc (imports.rs:94-98) calls the OOB probe an escalation that "faults the call"; the write path silently drops the same probe class.
Why it matters: The escalation threshold exists to stop a malicious guest from hammering (doc 06 §5.3 `[PROPOSAL: threshold 3 per session]`). A guest that loops OOB *writes* (bad `out_ptr` into `fs_read`/`http_get`/`devices_list`) gets `E_OOB` forever, burns host cycles per call, and is never counted or disabled — exactly the "answered with E_OOB forever" gap the module doc says sweep 2/4 closed, but closed for reads only. Containment itself holds (wasmi bounds-checks the write), so this is a policy/accounting hole, not an escape.
What correct looks like: `write_guest` takes the import name and mirrors `read_guest`: record `SecurityEvent` on the missing-memory and OOB branches, and let callers route through `escalation_or_status`/`deny` so the threshold applies; add the write-probe case to `oob_probes_trip_the_disable_threshold` (plugin_sandbox_security.rs:641 — currently read-path only).
Confidence: HIGH — Verification: read; all three call sites in part2 traced; test suite grep shows no write-probe escalation test.

### F-002 [S3] The whole import surface is unreachable from any shipped entry point (crate-level dead code)
Where: crates/plugin-host-wasm/src/imports.rs:228-245 (and the whole chunk)
Code:
```rust
pub fn build_linker(engine: &Engine, grants: &Grants) -> Result<Linker<HostState>, HostError> {
    let mut linker = <Linker<HostState>>::new(engine);
    define_host(&mut linker)?;
    define_log(&mut linker)?;
```
What is wrong: `build_linker`/`check_import_policy` are called only by `WasmInstance::spawn` (instance.rs:80,110), and nothing outside the crate's own tests ever spawns an instance: `plugin-host-wasm` has zero dependents in every Cargo.toml in the repo (only its own manifest names it); `install.rs:33` references `WasmInstance` in prose only; `bin/lumenext.rs` and `bin/lumenext-examples.rs` never import it. The `lumen` CLI and `lumen-tauri` cannot reach a single line of this file.
Why it matters: The wave question — "does the wasmi host actually execute a plugin end to end?" — answers YES in tests and NO in anything shipped. Doc 06 §5.2–§5.3's entire host ABI, including its security accounting, exists only under `cargo test`.
What correct looks like: an embedding app linking `plugin-host-wasm` (the crate's lib.rs:10-11 assumes such apps exist — none do), or the matrix/doc story states plainly that the host is test-only until Phase-7 wiring.
Confidence: HIGH — Verification: rg/grep caller sweeps + baseline test-log citations (same evidence as supervisor.rs F-001).
### F-003 [S4] Per-import buffer caps are undocumented magic numbers; the limits layer points at constants that do not exist
Where: crates/plugin-host-wasm/src/imports.rs:379
Code:
```rust
                let caps = (4096, caller.data().limits.io_transfer_bytes);
                let bytes = match read_guest(&mut caller, import, ptr, len, caps) {
```
What is wrong: The log-record cap is the literal `4096` inline in `log_write`; the same pattern repeats in part2 (`4096` for the fs path at :421, `2048` for the http URL at :705 — A03's range). `limits.rs:41-43` documents `max_descriptor_bytes` as "manifest-report / panel-descriptor / log-record byte caps (v1 ABI bounds, see `api.rs`)" — but `api.rs` contains no such constants (grep `4096|2048|MAX_` in api.rs → nothing), and doc 06 does not state the numbers either (rg over docs/06-plugin-architecture.md → no matches). The only place these ABI bounds exist is three unlinked literals.
Why it matters: Three independently-editable magic numbers implementing what the code itself calls "v1 ABI bounds"; a change to one silently diverges the others, and the pointer comment in limits.rs misdirects maintainers to api.rs.
What correct looks like: Named constants (`pub const MAX_LOG_RECORD_BYTES: u64 = 4096;` etc.) in api.rs next to the import specs, referenced from imports.rs and limits.rs.
Confidence: HIGH — Verification: read + grep api.rs/limits.rs/docs.
### F-004 [S4] Zero-length read succeeds without a memory export — inconsistent with the missing-memory denial contract
Where: crates/plugin-host-wasm/src/imports.rs:113-115
Code:
```rust
    if len == 0 {
        return Ok(Vec::new());
    }
```
What is wrong: `read_guest` checks `len == 0` (113-115) BEFORE the `memory` export check (116-126): a module that exports no memory gets `Ok(empty)` for a zero-length read and `E_OOB` + escalation for a non-zero one. The zero-length write path is consistent about it (`write_guest` 175-177 also short-circuits), but the read path's missing-memory branch treats the module shape itself as an escalation-worthy anomaly — except when len is 0.
Why it matters: Behaviorally benign (nothing is read), but the same call shape gets two different verdicts depending only on `len`, which is guest-chosen — a conformance-testing wart and a small spec inconsistency.
What correct looks like: Check the memory export first (module-shape anomaly), then the length; or document that zero-length transfers are unconditionally `Ok`.
Confidence: HIGH — Verification: read (both branches traced).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `HostState` (alias) | 42-43 | REAL | — | alias to `InstanceState`. |
| `FaultKind` (enum) | 48-53 | REAL | — | one variant, `EscalationDisabled { count }`. |
| `HostFault` + `Display`/`Error`/`wasmi HostError` impls | 55-73 | REAL | — | downcast consumers at instance.rs:328,344,361 (verified). |
| `HostFault::kind` | 75-80 | REAL | — | trivial getter; in-crate caller in instance.rs fault classification. |
| `guest_memory` | 87-89 | REAL | — | `get_export("memory")`; no panic paths. |
| `read_guest` | 99-143 | REAL | F-004 | negative ptr/len → E_ARG; cap = min(caps.0, caps.1) — bounded alloc `vec![0u8; len]`; OOB read → escalation + E_OOB; missing memory → escalation + E_OOB. |
| `ReadErr` (enum) | 146-153 | REAL | — | Status vs Fault distinction, clean. |
| `escalation_or_status` | 160-168 | REAL | — | threshold 3 (limits.rs:93); fault via `wasmi::Error::host`. |
| `write_guest` | 171-184 | REAL | F-001 | no escalation accounting on any refusal branch. |
| `write_guest_u32` / `write_guest_u64` | 187-194 | REAL | — | LE out-params; delegate to write_guest. |
| `deny` | 199-215 | REAL | — | records escalation; threshold → host fault; else E_PERM. |
| `build_linker` | 228-245 | REAL | F-002 | host+log always; fs/http/ui/devices grant-gated (matches api.rs Permission::Base). |
| `check_import_policy` | 251-268 | REAL | — | unknown module → `UnknownImportModule`; ungranted → `UndeclaredImport`; Base always granted (api.rs:429). Unknown import *names* inside a granted module fail at wasmi instantiation instead — acceptable, typed differently. |
| `define_host`/`api_version` | 272-278 | REAL | — | returns `ENGINE_PLUGIN_API_MAJOR` (1); window [1,1] asserted in lib.rs test. |
| `define_host`/`report_manifest` | 280-353 | REAL | — | caps from `max_descriptor_bytes`; re-parses via `PluginManifest::from_toml` (same rules as install); compares id/kind/version/api_version; lie → security event + threshold fault; non-UTF8 → plain E_ARG (no escalation — defensible as malformed arg, noted). |
| `define_log`/`log_write` | 362-406 | REAL | F-003 | level 0..=4 validated; 4096 magic cap; token bucket charged BEFORE storing; lossy UTF-8 (log text, fine); `log_record` bounded ring in state.rs:305-307. |

## 5. Test honesty
No tests in the chunk; the suite that executes it (all green in baseline):
| Test (external) | Would it fail if the code were broken? | Reason |
|---|---|---|
| hello_world_extension_runs_and_reports_manifest (tests/plugin_hello_world.rs) | YES | exercises report_manifest happy path + manifest_report storage through a real guest. |
| plugin_sandbox_escape (tests/plugin_sandbox_security.rs:319) | YES | pins E_OOB + exactly one security event + E_ARG for negative ptr + survival. Read path only. |
| oob_probes_trip_the_disable_threshold (:641) | YES | escalation counts 2 then fault at 3 on OOB reads. Would NOT catch F-001 (write path untested). |
| plugin_permission_escalation (:389) | YES | UndeclaredImport refusal + third-escalation fault + ledger counts (450-461). |
| metadata/ui_panel example conformance | YES | committed `plugins/extensions/*/plugin.wasm` run through this table — proves the committed artifacts actually execute. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "a module whose permission was not declared is simply absent … fails instantiation with `UndeclaredImport` before any guest code runs" | module doc 6-9 | REAL | plugin_permission_escalation (:390-403) asserts exactly this; baseline :4607 ok. |
| "a call that exceeds a sub-policy … returns the typed `E_PERM` … reaching the session threshold faults the instance and disables the extension" | module doc 13-17 | REAL for reads/denials; FALSE for OOB writes | F-001: write path never escalates. |
| No-host-pointers model: "Every buffer argument is `(ptr, len)` … bounds-checked `Memory` accessors; handles are indices … validated on every use" | module doc 20-25 | REAL | read_guest/write_guest traced; handle validation in part2 fs_read/fs_size/fs_close + state.rs handle table. |
| §5.4 limits drive the caps | limits.rs:41-43 comment ("see `api.rs`") | FALSE (pointer) | api.rs has no cap constants; F-003. |
| matrix `plugin-wasm-extensions` = not_started (feature-matrix.toml:2329) | shipping view REAL / code view under-claim | same as supervisor audit: host implemented + executed in tests, unlinked from every shipped binary. |

## 7. Dead / duplicated / unreachable code
- Crate-level: the entire chunk is production-dead (F-002) — same zero-dependents evidence as supervisor.rs.
- No duplicated code within the chunk; the read/write helper split is clean.
- `HostFault::kind()` has in-crate callers (instance.rs:361-364 area) — not dead.

## 8. Dependencies & flags
- Uses `wasmi::{Caller, Engine, Extern, Linker, Memory}` (2.0.0, validated module path per Cargo.toml:9-24), `plugin_api::ENGINE_PLUGIN_API_MAJOR`, `crate::api` constants (verified to exist), `crate::error::HostError`, `crate::limits::ESCALATION_DISABLE_THRESHOLD`, `crate::state::{InstanceState, SecurityEvent}`. No unused imports; no feature gates (crate has no `[features]`).
- Lint compliance verified: no unwrap/expect/panic/todo in range (workspace denies them).

## 9. Open questions for the next agent
- A03 (part2, 408-918): (a) fs_read/http_get/devices_list write_guest call sites — F-001 fix must route write refusals through the ledger; (b) the `4096` at :421 and `2048` at :705 — same F-003 pattern; (c) fs_open's no-jail path (:431-447) records a GUEST escalation for a HOST misconfiguration (fs granted, no roots) — three legit calls fault the extension; decide if that deserves a finding.
- A03 (state.rs): `http_admit` (state.rs:316+) uses `std::time::Instant::now()` for the rolling-minute window while the rest of the host is deterministic/injected-time — check what the tests pin.
- A09 (instance.rs): confirm the escalation-disable path (`fault_to_host_error` → `HostError::ExtensionDisabled`) and whether the "disable the extension" half (beyond faulting one instance) is wired anywhere — my read: nothing persists disable state across instances.
