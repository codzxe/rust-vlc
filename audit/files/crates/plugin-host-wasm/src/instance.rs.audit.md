# AUDIT: crates/plugin-host-wasm/src/instance.rs
| Field | Value |
|---|---|
| Auditor | W12-A09 |
| Date | 2026-11-21 |
| Lines / functions | 405 / 17 (16 fns/methods + 1 test) |
| File verdict | UNREACHABLE (real wasmi integration; the only callers of `WasmInstance::spawn` are the crate's own integration tests — the shipped `lumenext` bin never executes guest code) |
| Reachable from shipped entry point? | NO. Grep `WasmInstance::spawn\|\.spawn\(` in crate: definition + `tests/common/mod.rs:570-612` helpers + `tests/plugin_examples_conformance.rs:129,405`. No app depends on `plugin-host-wasm`; `lumen`/`desktop`/`lumen-desktop-tauri` never link it. |
| Findings | S0: 0 S1: 1 S2: 0 S3: 1 S4: 3 |
| Commands run | Grep `WasmInstance::spawn\|fn spawn\|\.spawn\(` (callers = tests only); Grep `read_guest_memory\|write_guest_memory` (callers: hello_world/conformance/sandbox tests; guest-reported u32 lengths flow in at conformance.rs:241-244, sandbox_security.rs:766-769); Read lib.rs (bounded-memory claim), limits.rs, error.rs; sed -n verification of all quotes; baseline logs grep (crate compiled at build log :231, never tested — both workspace test runs abort before `plugin-host-wasm`). |

## 1. One-paragraph verdict
This is the only file where wasmi actually runs: a competent spawn pipeline (magic fence → API-version negotiation → compile → import-policy check → store with memory ceiling + startup fuel → granted-only linker → `instantiate_and_start`) and a disciplined `invoke` (per-call fuel, integer-only v1 ABI with typed narrowing refusals, one-way faulting). The wasmi usage itself is correct — trap classification, `trap_on_grow_failure`, the `HostFault` escalation channel and the fuel semantics all check out against the wasmi 2.0 API and the crate's own tests. But the sandbox has one unguarded door: `read_guest_memory` allocates `vec![0u8; len as usize]` from a caller-supplied (in the documented protocol, guest-reported) length BEFORE the bounds check — up to 4 GiB per call from a u32 length, or a capacity-overflow abort from a u64 — directly contradicting lib.rs's "bounded memory everywhere (D-2)". And none of this executes anywhere: spawn's only callers are tests, the bin refuses to run guests, and no baseline run ever reached this crate.

## 2. What actually works (verified)
— Nothing verified by execution in baseline. Compilation proven (`audit/runs/baseline/build-release-workspace.log.txt:231`). The in-tree suites exercise spawn/invoke/memory honestly (see §5) but neither baseline workspace test log contains a `plugin-host-wasm` run (both abort at earlier crates — `test-workspace.log.txt` ends `-p desktop --test zero_alloc_transport`, all-features ends `-p first-vertical --test evidence`).
— Code-traced correct: magic fence before compile (65-75); import-policy check BEFORE store/instantiation (80) so undeclared imports can never execute; store limiter wiring `store.limiter(|state| &mut state.store_limits …)` (106) with `trap_on_grow_failure(true)` (87); fuel reset per call (185-187) so budgets do not accumulate; `invoke` poisons the instance on ANY call error but returns pre-call errors (MissingExport, arity, narrowing) WITHOUT faulting (192-222 before the call, 257-263 after) — correct distinction; result extraction handles the "impossible" non-integer case honestly (244-252).

## 3. Findings

### F-001 [S1]
Where: crates/plugin-host-wasm/src/instance.rs:274-288
Code:
```rust
    pub fn read_guest_memory(&self, ptr: u64, len: u64) -> Result<Vec<u8>, HostError> {
        …
        let mut buf = vec![0u8; len as usize];
        memory
            .read(&self.store, ptr as usize, &mut buf)
            .map_err(|_| HostError::GuestMemoryOob {
```
What is wrong: the host-side result buffer is allocated at the FULL requested length before any bounds check, and the length is not capped. The documented v1 protocols feed guest-reported lengths straight in: `plugin_examples_conformance.rs:241-244` reads a guest-written u32 (`OUT_LEN_AT`) and calls `read_guest_memory(RESULT_AT, len as u64)`; `plugin_sandbox_security.rs:766-769` does the same for the devices module ("out_len at 2048 names the inventory"). A malicious extension writes `u32::MAX` into its out-len cell: the host attempts a 4,294,967,295-byte zeroed allocation (which the subsequent bounds check then discards — the guest's memory is capped at 32–64 MiB, so the read can never legitimately exceed that). The API takes `u64`; a caller passing a huge length gets a capacity-overflow abort, i.e. a host panic.
Why it matters: this is the exact class of hole the crate exists to prevent. lib.rs:46-50 claims "No `unsafe` …; bounded memory everywhere (D-2)" — the guest's own memory is bounded, but the host's read-out buffer is sized by the guest with no ceiling: a sandboxed extension can force multi-GiB host allocations (or an abort) per call — an availability escape from a sandbox whose §9.4 test suite models exactly this adversary. S1 rather than S2 because it is a security-hole/OOM on input the system is designed to accept, per the "pick the higher" rule; mitigating context: no production caller today (crate unwired, bin never executes), and the import-path reads ARE capped (`io_transfer_bytes`, imports.rs).
What correct looks like: clamp `len` to `min(len, memory.size(&self.store) * 64 * 1024 - ptr)` (or `limits.io_transfer_bytes`) BEFORE allocating, and reject the remainder as `GuestMemoryOob`.
Confidence: HIGH — Verification: read + callers traced; allocation-before-check shape confirmed by the code order (280-287).

### F-002 [S3]
Where: crates/plugin-host-wasm/src/instance.rs:56-80
Code:
```rust
    pub fn spawn(
        engine: &Engine,
        wasm: &[u8],
        manifest: PluginManifest,
        services: HostServices,
        limits: ResourceLimits,
```
and
```rust
        api::negotiate_api_version(manifest.api_version)?;
        let module = Module::new(engine, wasm)
            .map_err(|e| HostError::Instantiation(format!("module rejected: {e}")))?;
        let grants = Grants::from_permissions(&manifest.permissions);
        check_import_policy(&module, &grants)?;
```
What is wrong: `spawn` never calls `api::kind_extension_legal(manifest.kind)` — the v1 kind fence ("byte-source, codec and output kinds stay builtin") exists ONLY in `install.rs:479` (install-time). Spawn takes the manifest and the limits as INDEPENDENT parameters; nothing binds `limits` to the manifest's kind, and nothing re-checks kind legality at the last line of defense.
Why it matters: combined with install.rs's F-001 (`load()` also skips the kind re-check its doc promises, and the on-disk manifest is not digest-covered), the kind fence is enforceable only at install time; any path that spawns from a manifest other than a fresh install-validation (the conformance kit's `spawn_installed` does re-load through the store, but `spawn` itself is a public seam) has no kind fence at all. Cross-file drift, counted once here for the spawn side.
What correct looks like: `if !api::kind_extension_legal(manifest.kind) { return Err(HostError::KindRefused { .. }) }` in `spawn`, or a debug_assert + doc that install is the sole fence.
Confidence: HIGH — Verification: read; grep `kind_extension_legal` → sole caller install.rs:479.

### F-003 [S4]
Where: crates/plugin-host-wasm/src/instance.rs:45-48 vs 76-80
Code:
```rust
    /// Spawn: wasm magic → module compile → api-version negotiation →
    /// import-policy check → store with memory ceiling + fuel →
```
What is wrong: the doc comment's stated order is magic → compile → negotiation; the code runs magic → negotiation (76) → compile (77). Harmless behaviorally (both refusals are typed), but the doc misdescribes the pipeline it documents.
Why it matters: doc drift on a security-relevant ordering (a reader auditing which check fires first for a bad-api-version non-wasm blob gets the wrong answer).
What correct looks like: fix the comment order.
Confidence: HIGH — Verification: read of 64-80.

### F-004 [S4]
Where: crates/plugin-host-wasm/src/instance.rs:372-388
Code:
```rust
/// … Both shapes are the same fact: the ceiling held.
fn is_memory_denied(err: &wasmi::Error) -> bool {
    if err.as_trap_code() == Some(TrapCode::GrowthOperationLimited) {
        return true;
    }
    matches!(
        err.kind(),
        wasmi::errors::ErrorKind::Memory(
            MemoryError::ResourceLimiterDeniedAllocation | MemoryError::OutOfSystemMemory
        )
    )
}
```
What is wrong: `MemoryError::OutOfSystemMemory` means the HOST could not satisfy the allocation (real memory pressure), not that the per-instance ceiling refused it. Mapping it to `HostError::MemoryLimit { cap_bytes }` reports "linear-memory cap of N bytes breached — instance faulted" for a condition where the cap was irrelevant and the host machine was out of memory. The comment's claim "Both shapes are the same fact: the ceiling held" is false for that arm.
Why it matters: misdiagnosed errors send the operator tuning the wrong knob (extension limits instead of host memory); the honest mapping is a distinct error variant.
What correct looks like: map `OutOfSystemMemory` to its own `HostError` (e.g. `HostCapacity`), keep `ResourceLimiterDeniedAllocation`/`GrowthOperationLimited` as `MemoryLimit`.
Confidence: HIGH — Verification: read; wasmi error kinds per the `wasmi::errors` path names.

### F-005 [S4]
Where: crates/plugin-host-wasm/src/instance.rs:327-359
Code:
```rust
fn classify_spawn_error(err: &wasmi::Error, memory_cap: u64) -> HostError {
    if let Some(fault) = err.downcast_ref::<HostFault>() {
        return fault_to_host_error(fault);
    }
    if err.as_trap_code() == Some(TrapCode::OutOfFuel) {
        return HostError::CpuLimit;
    }
```
(and the identical first three arms of `classify_call_error`)
What is wrong: `classify_spawn_error` and `classify_call_error` duplicate their first three arms verbatim; they differ only in the fallback (`Instantiation(msg)` vs `Wasm("trap: …")`/`Wasm(msg)`).
Why it matters: copy-paste drift risk — a new fault shape added to one and not the other silently misclassifies; the duplication has no functional justification.
What correct looks like: one classifier taking the fallback as a parameter or closure.
Confidence: HIGH — Verification: read, both bodies compared.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `WasmInstance::spawn` | 56-120 | REAL (defects) | S3/S4 | Magic fence, negotiation, compile, import policy, store limits (mem/table/instances/tables/memories + trap-on-grow), startup fuel, granted-only linker, start under fuel. NO kind check (F-002); doc order drift (F-003); no host-level instance/heap accounting (see my limits.rs audit — `HOST_MAX_ACTIVE_INSTANCES`/`HOST_TOTAL_HEAP_BYTES` enforced nowhere). |
| `WasmInstance::manifest` | 123-125 | REAL | — | Trivial accessor. |
| `WasmInstance::status` | 128-130 | REAL | — | Trivial accessor. |
| `WasmInstance::ledger` | 133-135 | REAL | — | Arc clone of the store's ledger. |
| `WasmInstance::manifest_report` | 139-141 | REAL | — | Reads store data; None until guest reports. |
| `WasmInstance::log_records` | 145-147 | REAL | — | Bounded clone of the 64-record ring (state.rs). |
| `WasmInstance::panels` | 150-152 | REAL | — | Slice of registered PanelRecords. |
| `WasmInstance::import_trace` | 155-157 | REAL | — | Conformance-facing trace. |
| `WasmInstance::invoke` | 178-265 | REAL | — | Faulted gate; per-call fuel; arity + typed i32/i64 narrowing with typed refusals; non-integer params/results honestly refused; faults only on `func.call` errors. Correct wasmi `Val` usage. |
| `WasmInstance::read_guest_memory` | 274-289 | REAL (defect) | S1 | Unbounded pre-check allocation from caller/guest length (F-001); page math `size * 64 * 1024` correct; `ptr as usize` truncation only on 32-bit hosts (not the target). |
| `WasmInstance::write_guest_memory` | 300-313 | REAL | — | Host-controlled data; bounds-checked write; correct OOB typing. |
| `engine` | 320-324 | REAL | — | `consume_fuel(true)`; the `auto-dispatch` backend selection comes from the Cargo feature pin (Cargo.toml:57) — consistent. |
| `classify_spawn_error` | 327-340 | REAL | S4 | Correct HostFault/OutOfFuel/memory arms; fallback Instantiation matches the trapping-start test expectation. Duplication (F-005). |
| `classify_call_error` | 343-359 | REAL | S4 | Adds the trap-code arm; duplication (F-005). |
| `fault_to_host_error` | 361-370 | REAL | — | Single FaultKind (EscalationDisabled) → ExtensionDisabled. |
| `is_memory_denied` | 379-389 | REAL | S4 | OutOfSystemMemory misattribution (F-004). |
| test `engine_config_has_fuel_metering` | 395-404 | REAL | — | Mutation-sensitive: removing `consume_fuel(true)` makes `set_fuel` error → assert fails. |

## 5. Test honesty (integration suites covering this file; none run in baseline)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `plugin_hello_world` (tests/plugin_hello_world.rs) | YES | Real spawn+invoke+memory round-trips (`write_guest_memory(128, …)` → echo → `read_guest_memory`), E_PERM paths. |
| `plugin_sandbox_security` (§9.4 suite) | YES | Pointer-smuggling/OOB probes pinned to typed `GuestMemoryOob`/statuses — kills bounds-check mutations. Does NOT probe a huge out-len into `read_guest_memory` (F-001 blind spot: the adversarial suite itself never lies about the length). |
| `plugin_crash_isolation` | YES | Trapping guests fault only their instance; playback engine unaffected in-process. |
| `plugin_examples_conformance` | YES | Real protocol drives (extract/ui-panel) with byte-exact assertions. |
| In-file `engine_config_has_fuel_metering` | YES | See table above. |
| Baseline evidence | NO RUN | Neither workspace test log reaches this crate. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "No `unsafe` …; bounded memory everywhere (D-2)" | crates/plugin-host-wasm/src/lib.rs:46-50 | FALSE | F-001: host-side read buffer sized by guest-reported length, uncapped. |
| "first execution happens only when something spawns the extension through `WasmInstance::spawn` … (tested with a trapping start section)" | install.rs:30-34 | REAL (code) / REAL-UNPROVEN (run) | spawn is the sole executor (grep); trapping-start test exists (plugin_no_auto_exec (e)) but has no baseline run. |
| "Trap containment comes free with wasmi: a trap destroys only that instance's `Store`" | instance.rs:5-9 + lib.rs:37-44 | REAL (code-traced) | Per-instance Store; fault status refuses further dispatch; crash_isolation suite composes with playback-engine (unrun in baseline). |
| "lumenext deliberately never installs and never executes guest code" | plugins/extensions/README.md:135-136 | TRUE (pending my bin audit) | No `spawn` call in the bin (grep); confirm in W12-A09 bin audit. |

## 7. Dead / duplicated / unreachable code
- Whole file production-unreachable (header row).
- `classify_spawn_error`/`classify_call_error` duplication (F-005).
- No dead symbols found; all accessors consumed by tests/imports.

## 8. Dependencies & flags
- wasmi 2.0.0 pinned `default-features=false, features=["std","validate","auto-dispatch"]` (Cargo.toml:57) — `Module::new` (validating), `Store::set_fuel`, `StoreLimitsBuilder`, `TrapCode::GrowthOperationLimited` all consistent with that pin. No feature gates on this file. No `unsafe`. `[lints] workspace = true`.

## 9. Open questions for the next agent
- imports.rs auditor: the import-path reads cap at `io_transfer_bytes` — confirm no import handler sizes an allocation from a guest-supplied length the way `read_guest_memory` does (state.rs `devices`/`ui` payload reads especially).
- Whoever owns supervisor.rs: does any restart path re-enter `invoke` after `Faulted` (the status is terminal here — `InstanceFaulted` refuses; restart = new spawn, host accounting still absent)?
- Orchestrator: REQUEST-ORCH-RUN `cargo test -p plugin-host-wasm --locked` (covers this file's suites).
