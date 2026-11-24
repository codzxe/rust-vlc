# AUDIT: crates/plugin-host-wasm/src/instance.rs
| Field | Value |
|---|---|
| Auditor | W12-A03 |
| Date | 2025-06-27 |
| Lines / functions | 405 / 20 (spawn, invoke, 10 accessors, engine, 4 error mappers, 1 test) |
| File verdict | REAL-UNPROVEN (code is real and heavily test-exercised headless; no shipped lane builds the crate) |
| Reachable from shipped entry point? | NO — no crate outside `crates/plugin-host-wasm` depends on it (rg over apps/cli, apps/desktop*, apps/web, Cargo.tomls: only plugin-api is a CLI dep, apps/cli/Cargo.toml:59); release lane builds lumen-cli + desktop only. Reached by the crate's own 5 integration suites + the `lumenext` packaging bin (which never executes guests). Baseline itself records the gap: `test gap_plugins_crashed ... ignored, gap: plugins.crashed not wired — handoff H-10 (sandboxed WASM host, plugin-host-wasm, Phase 7)` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | rg (caller tracing + Cargo.toml dep scan, exit 0); sed -n (quote verification, exit 0); baseline citation (cargo-test-workspace.txt:4884-5015) |

## 1. One-paragraph verdict
This is the best-engineered file so far in this agent's set: a real wasmi sandbox with a wasm-magic fence before compile, api-version negotiation, import-policy check, store-level resource limits (memory cap, table/instance counts, trap-on-grow), per-call fuel reset, integer-only ABI narrowing that refuses rather than reinterprets, and typed fault classification that kills only the faulting instance. The integration suites exercise spawn/invoke/memory for real (44 unit + 6 crash-isolation + 5 conformance + 2 hello-world + install + sandbox suites, all `ok` in baseline). The two real defects are public-API-level: `read_guest_memory` allocates `len` bytes BEFORE bounds-checking (a `len: u64` from a future caller is an unbounded allocation), and `is_memory_denied` maps `OutOfSystemMemory` (a real host OOM) to `MemoryLimit { cap_bytes }` (a policy denial) — mislabeling a host resource exhaustion as a sandbox ceiling hold. Neither is reachable from a shipped surface today because the crate is not in the release lane.

## 2. What actually works (verified)
— Baseline `cargo-test-workspace.txt` lines 4884-5015: PACKAGE plugin-host-wasm, `test result: ok. 44 passed; 0 failed` (unittests), crash_isolation 6/6, examples_conformance 5/5, hello_world 2/2, install + sandbox suites ok; `instance::tests::engine_config_has_fuel_metering ... ok` (line ~4922 region).
— Verified no external crate consumes this crate (rg over apps/*/Cargo.toml + src trees, exit 0, no hits).

## 3. Findings
F-001 [S3]
Where: crates/plugin-host-wasm/src/instance.rs:274-282 (read_guest_memory)
Code:
```
    pub fn read_guest_memory(&self, ptr: u64, len: u64) -> Result<Vec<u8>, HostError> {
        let memory = self
            .instance
            .get_export(&self.store, "memory")
            .and_then(wasmi::Extern::into_memory)
            .ok_or_else(|| HostError::Wasm("module exports no memory".to_string()))?;
        let mut buf = vec![0u8; len as usize];
```
What is wrong: The buffer of `len` bytes is allocated BEFORE the bounds check (`memory.read` fails after the allocation). `len` is an untrusted-shaped `u64`: a caller passing `len = u64::MAX` (or any huge value) attempts an abort-sized allocation before `GuestMemoryOob` can answer. Today the only callers are the crate's own tests (rg: hits exclusively under crates/plugin-host-wasm/tests/), so no shipped path is exposed — but this is a `pub` API on a `pub` struct explicitly documented as "host-side result retrieval", i.e. the seam a future app-side extension runtime will call with guest-declared lengths. Why it matters: the sandbox crate's own contract ("bounded", "typed §9.4 denial") is violated by the allocation-before-check order; on 32-bit hosts `len as usize` also truncates. What correct looks like: compare `ptr.saturating_add(len)` against `memory.size()*64*1024` first, refuse typed, then allocate. Confidence: HIGH — Verification: read (allocation order verified; no production caller to exploit — hence S3 not S1).
F-002 [S3]
Where: crates/plugin-host-wasm/src/instance.rs:379-390 (is_memory_denied)
Code:
```
    matches!(
        err.kind(),
        wasmi::errors::ErrorKind::Memory(
            MemoryError::ResourceLimiterDeniedAllocation | MemoryError::OutOfSystemMemory
        )
    )
```
What is wrong: `OutOfSystemMemory` — the HOST failing an allocation (real machine out of RAM) — is classified as `HostError::MemoryLimit { cap_bytes }`, the sandbox-ceiling denial. A host OOM is not the ceiling holding; it is the host's own resource exhaustion, and the two demand different handling (the doc comment at 371-378 even narrates only the two limiter-denial shapes, then quietly folds system OOM in). Why it matters: an operator seeing `MemoryLimit` will raise the plugin's memory cap — the wrong fix for a host OOM; the fault ledger records the wrong cause. What correct looks like: a distinct typed error for system-memory exhaustion. Confidence: MEDIUM — Verification: read (wasmi 2.0 error kinds; not executed).
F-003 [S4]
Where: crates/plugin-host-wasm/src/instance.rs:320-325 (engine)
Code:
```
pub fn engine() -> Engine {
    let mut config = Config::default();
    config.consume_fuel(true);
    Engine::new(&config)
}
```
What is wrong: Fresh `Engine` per call; every caller that spawns instances shares nothing (doc says "The engine every instance of one host shares" — sharing is the CALLER's duty; tests each build their own). Hygiene-grade naming/doc mismatch only. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/plugin-host-wasm/src/instance.rs:186-191 (invoke, fault refusal)
Code:
```
        if let InstanceStatus::Faulted { reason } = &self.status {
            return Err(HostError::InstanceFaulted {
                reason: reason.clone(),
            });
        }
```
What is wrong: Nothing functionally — the refusal is correct — but `reason` is a `String` round-trip of a typed error (`host_err.to_string()` at 266-268), so the refusal downstream is stringly-typed: the original typed variant is lost on re-report. Doc 06's typed-error discipline leak, limited blast radius. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| InstanceStatus | 25-32 | REAL | — | Ready/Faulted; backoff deferred to #46, honest |
| WasmInstance (struct) | 35-45 | REAL | — | store+instance+status+manifest |
| spawn | 56-118 | REAL | — | magic fence (64-69, byte-exact `\0asm`), api negotiate, import policy, store limits (memory/table/instances/tables/memories + trap_on_grow), startup fuel, instantiate+start, classify; exercised by all 5 integration suites |
| manifest / status / ledger | 123-136 | REAL | — | accessors |
| manifest_report | 139-142 | REAL | — | F1 handshake observability |
| log_records | 145-147 | REAL | — | bounded 64-ring clone |
| panels / import_trace | 150-160 | REAL | — | ui-surface + conformance observability |
| invoke | 178-272 | REAL | — | faulted refusal; per-call fuel; arity check; i64→i32 narrowing typed-refused on overflow; non-integer params/results typed-refused; results zero-initialized then filled; trap → classify + Faulted |
| read_guest_memory | 274-293 | REAL (defect) | S3 | alloc-before-check F-001 |
| write_guest_memory | 300-314 | REAL | — | bounds-checked write; no allocation hazard |
| engine | 320-325 | REAL | S4 | per-call engine; fuel on (test-proven) |
| classify_spawn_error / classify_call_error | 327-359 | REAL | — | HostFault downcast → OutOfFuel → memory-denied → trap fallback; call variant adds trap-code formatting |
| fault_to_host_error | 361-369 | REAL | — | EscalationDisabled mapping with count |
| is_memory_denied | 379-390 | REAL (defect) | S3 | OOM mislabel F-002 |
| test engine_config_has_fuel_metering | 394-405 | REAL | — | set_fuel ok ⇒ fuel consumed; weak but valid |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| engine_config_has_fuel_metering | YES | wasmi `set_fuel` errors when the engine does not consume fuel — a fuel-off regression fails it |
The file's real coverage lives in the integration suites (spawn/invoke/memory paths, fault isolation, escalation): crash_isolation 6/6 proves a faulting guest leaves playback + host intact; sandbox_security exercises the import bounds checks against `read_guest_memory` results. All `ok` in baseline (4884-5015).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Trap containment comes free with wasmi (ADR-002): a trap destroys only this instance's Store" | instance.rs:3-5 | REAL | crash_isolation suite 6/6 ok in baseline (one process, faulting extension + playback engine) |
| "memory-cap breach during start (HostError::MemoryLimit)" | instance.rs:52-55 | REAL | classify_spawn_error maps GrowthOperationLimited + limiter denial; sandbox suites cover |
| spawn error ordering as documented | instance.rs:48-55 | REAL | magic → negotiate → compile → policy → fuel → memory ordering verified by reading the body (56-118) |
| "the same bounds-checked path imports use" (read_guest_memory) | instance.rs:270-271 | REAL for bounds, FALSE for allocation order | F-001: imports check before touching; this accessor allocates first |

## 7. Dead / duplicated / unreachable code
Whole crate is outside the shipped release lane (see header; the repo's own gap test `gap_plugins_crashed` says "not wired — handoff H-10 ... Phase 7"). No intra-file dead code.

## 8. Dependencies & flags
wasmi 2.0.0 (default-features off; std+validate+auto-dispatch), plugin-api, settings-schema, toml. No feature gates on this file. Crate edition 2024.

## 9. Open questions for the next agent
- When the app-side extension runtime lands (Phase 7 / #46), audit the seam that will call `read_guest_memory` with guest-declared lengths — F-001 becomes S1 there. Suggest pre-fixing the bounds-check order now.
- `supervisor.rs` owns the restart/backoff policy referenced here; it was not in my assignment — someone should confirm the escalation ledger's count survives instance restarts as doc 06 §5.3 requires.
