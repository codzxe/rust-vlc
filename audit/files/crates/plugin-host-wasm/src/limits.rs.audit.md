# AUDIT: crates/plugin-host-wasm/src/limits.rs
| Field | Value |
|---|---|
| Auditor | W12-A05 |
| Date | 2026-02-14 |
| Lines / functions | 237 / 7 (ResourceLimits + for_kind + test_tiny, TokenBucket + 4 methods, 3 tests) |
| File verdict | REAL-UNPROVEN (enforcement real and wasmi-config-backed; the §5.4 DEFAULTS are never applied by any non-test caller) |
| Reachable from shipped entry point? | NO — plugin-host-wasm has zero reverse deps; only its own bins/tests. Enforcement is wired crate-internally (instance.rs/state.rs/imports.rs). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | rg (all const/for_kind/test_tiny callers tree-wide); sed quote checks; baseline citation |

## 1. One-paragraph verdict
This is the hostile-focus file, and the answer to "what limits exist and are they enforced by wasmi config" is: memory (32/64 MiB by kind), startup+call fuel, single-transfer I/O cap, token-bucket rate/burst, per-minute HTTP window, handle-table cap, descriptor cap, escalation threshold — and YES, the two wasmi primitives are genuinely configured: `StoreLimitsBuilder::memory_size(limits.memory_bytes).table_elements(10_000).instances(1).tables(1).memories(1).trap_on_grow_failure(true)` + `store.limiter(...)` (instance.rs:81-106) for memory/table, and `Config::consume_fuel(true)` + `set_fuel(startup_fuel)`/`set_fuel(call_fuel)` (instance.rs:108, 186, 322) for CPU — proven live by the §9.4 suite (fuel kill, 1 MiB cap trap). Everything else is honest host-side accounting (TokenBucket, rolling window, handle table), all exercised by the passing sandbox tests. The defects: the §5.4 default table (`for_kind`) has ZERO non-test callers — nothing ever spawns an instance with the production defaults, only `test_tiny` is exercised; and the two host-level ceilings (4 active instances, 256 MiB total heap) are declared constants with no enforcement anywhere.

## 2. What actually works (verified)
— baseline cargo-test-workspace.txt (plugin_host_wasm unittests, :4905-4907): `limits::tests::bucket_burst_then_throttle`, `memory_class_follows_doc06_row1`, `bucket_refills_over_time` all `ok`.
— Memory + table enforcement: instance.rs:81-88 store limiter built from `limits.memory_bytes`; instance.rs:106 `store.limiter(...)`. Live-proven by `plugin_memory_exhaustion` (MemoryLimit cap 1 MiB, baseline :5010).
— Fuel enforcement: instance.rs:322 `config.consume_fuel(true)`; :108 `set_fuel(limits.startup_fuel)`; :186 `set_fuel(limits.call_fuel)` per invoke. Live-proven by `plugin_trap_loop` (CpuLimit, baseline :5011).
— Escalation threshold: ESCALATION_DISABLE_THRESHOLD used at imports.rs:161, 209, 347 — live-proven by two §9.4 tests.
— TokenBucket arithmetic: `refill` computes in u128 and only advances `last` when `gained > 0`, so sub-second fractional time is not lost between calls (no drift); `take` never mutates on failure. Read-verified limits.rs:180-190.

## 3. Findings
F-001 [S3]
Where: crates/plugin-host-wasm/src/limits.rs:102-120 (callers: lib.rs:117, limits.rs:198-215 only)
Code:
```
    pub fn for_kind(kind: PluginKind) -> ResourceLimits {
        let memory_bytes = match kind {
            PluginKind::UiPanel | PluginKind::AudioFilter | PluginKind::VideoFilter => {
                MEM_PANEL_BYTES
            }
            _ => MEM_DEFAULT_BYTES,
        };
```
What is wrong: `for_kind` — the entire doc 06 §5.4 production defaults table (32/64 MiB classes, 300M call fuel, 1.5B startup fuel, 16 MiB transfers, 32 req/min, 8 handles) — is applied by NO non-test code. rg across the crate: callers are the unit test `memory_class_follows_doc06_row1` and the compile-check `headline_surface_is_wired` (lib.rs:117, `let _limits = ...` — discarded). The supervisor does not spawn instances (rg supervisor.rs: no ResourceLimits, no spawn). Every live spawn in the tree uses `test_tiny()`. The production numbers are declarative.
Why it matters: the crate's stated contract ("The doc 06 §5.4 defaults for an extension of `kind`") is never exercised; the calibration numbers (`[PROPOSAL]` fuel band) can never be validated if nothing uses them; an embedder copying the supervisor path gets no limits application for free.
What correct looks like: supervisor/registry code that spawns with `ResourceLimits::for_kind(manifest.kind)`, or delete for_kind until the embedder exists.
Confidence: HIGH — Verification: read (rg for_kind → 2 hits, both tests).

F-002 [S3]
Where: crates/plugin-host-wasm/src/limits.rs:85-89
Code:
```
/// 4 active instances per host (doc 06 §5.4 row 8).
pub const HOST_MAX_ACTIVE_INSTANCES: usize = 4;

/// 256 MiB total extension heap across instances (doc 06 §5.4 row 9).
pub const HOST_TOTAL_HEAP_BYTES: u64 = 256 * 1024 * 1024;
```
What is wrong: dead constants — zero references outside their declarations (rg tree-wide: 2 hits). The §5.4 rows 8-9 host-level ceilings (active-instance count, aggregate heap) are not enforced by any code, and no test covers them. The module header claims "the host-level accounting ceilings (§5.4 last two rows)" — accounting exists only as comments.
Why it matters: the "host survives" posture is only proven for sequential single instances; a real embedder could spawn unbounded instances with no aggregate memory ceiling — the exact failure mode §5.4 row 9 exists to prevent.
What correct looks like: enforce in the supervisor (instance counter + summed memory_bytes against the caps) or remove with a doc-debt note.
Confidence: HIGH — Verification: read (rg HOST_MAX_ACTIVE_INSTANCES|HOST_TOTAL_HEAP_BYTES → declarations only).

F-003 [S4]
Where: crates/plugin-host-wasm/src/limits.rs:53-59 vs instance.rs:83
Code:
```
pub const CALL_FUEL: u64 = 300_000_000;
```
What is wrong: the wall-clock↔fuel calibration ("roughly 2–4 fuel per simple instruction", "1e8–1e9 fuel/second on 2026 desktop hardware") is unmeasured guesswork flagged `[PROPOSAL]` — and the table-size limit (`table_elements(10_000)`, instance.rs:83) lives outside ResourceLimits entirely, so it is not tunable with the rest and not documented in the §5.4 mapping. Honest about being a proposal; still unvalidated numbers and a hidden limit.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ResourceLimits (struct) | 20-44 | REAL | — | 9 fields, all consumed by spawn/state/imports paths. |
| for_kind | 102-120 | REAL-UNPROVEN | S3 | Correct logic; zero production callers (F-001). |
| test_tiny | 125-137 | REAL | — | The only limits ever exercised (all §9.4 suites via common::spawn). |
| TokenBucket::new | 153-159 | REAL | — | Full bucket at spawn. |
| TokenBucket::take | 164-172 | REAL | — | Refill-then-take; no mutation on refusal. |
| TokenBucket::fits_capacity | 176-178 | REAL | — | One-shot transfer bound. |
| TokenBucket::refill | 180-190 | REAL | — | u128 math, cap at capacity, no fractional loss (verified by reasoning + bucket_refills_over_time). |
| memory_class_follows_doc06_row1 (test) | 198-215 | REAL | — | Pins 64 MiB classes + 32 MiB default. |
| bucket_burst_then_throttle (test) | 218-226 | REAL | — | Burst drain + refusal + capacity bound. |
| bucket_refills_over_time (test) | 229-236 | REAL | — | 6 ms sleep → ≥5 tokens at 1000/s; real time dependence, mildly flaky-proof (margin 1000×). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| memory_class_follows_doc06_row1 | YES | Equality asserts on for_kind's memory class per kind. |
| bucket_burst_then_throttle | YES | take(10) ok then take(1) refused; fits_capacity 10/11 boundary. |
| bucket_refills_over_time | YES | Would fail if refill math were wrong by ≥1 token over 6 ms; sleeps are real. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "wasmi gives us two enforcement primitives ... the store-level StoreLimits memory cap and fuel metering" | L6-8 | REAL | instance.rs:81-88/106/108/186/322; live-proven by baseline §9.4 tests. |
| "a tight loop is killed at exactly `call_fuel` burned instructions" | L12-14 | REAL-UNPROVEN | Fuel kill proven (CpuLimit); the EXACT instruction count is not asserted anywhere (no fuel-remaining probe in tests). |
| "ResourceLimits::for_kind — The doc 06 §5.4 defaults for an extension of `kind`" | L96-101 | SHALLOW | Correct computation, zero production application (F-001). |
| "host-level accounting ceilings (§5.4 last two rows)" (module header) | L1-2 | FALSE-as-implied | HOST_MAX_ACTIVE_INSTANCES/HOST_TOTAL_HEAP_BYTES are unenforced dead constants (F-002). |

## 7. Dead / duplicated / unreachable code
- HOST_MAX_ACTIVE_INSTANCES, HOST_TOTAL_HEAP_BYTES: dead (F-002).
- MEM_DEFAULT_BYTES/MEM_PANEL_BYTES/CALL_FUEL/STARTUP_FUEL/IO_* /HTTP_REQUESTS_PER_MIN/MAX_OPEN_HANDLES/MAX_DESCRIPTOR_BYTES: reachable only through for_kind → which itself has no production caller (F-001 chain).
- Table-size/table-count/memory-count/instance-count limits: enforced but hard-coded in instance.rs:83-86, outside this file's tunable set.

## 8. Dependencies & flags
plugin-api (PluginKind) only. No unsafe, no features, std-only. The wasmi feature posture ("validate", "auto-dispatch" — debug stack-overflow fix) is documented in Cargo.toml:9-24 and is load-bearing for these tests.

## 9. Open questions for the next agent
- Issue #46 (supervisor/restart policy) — does it plan the for_kind wiring and host ceilings? If the supervisor is the intended enforcement point, F-001/F-002 are "not yet wired" rather than dead; the supervisor as committed has neither.
- Should `table_elements(10_000)` move into ResourceLimits before any embedder copies the spawn path?
