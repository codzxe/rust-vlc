# AUDIT: crates/plugin-host-wasm/tests/plugin_sandbox_security.rs
| Field | Value |
|---|---|
| Auditor | W12-A05 |
| Date | 2026-02-14 |
| Lines / functions | 885 / 19 (10 #[test] + 9 fixture builders) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — `plugin-host-wasm` has zero reverse deps in the workspace (rg over all Cargo.toml: only its own bins `lumenext`/`lumenext-examples`); release lane builds `lumen-cli`+`desktop` only. Tests verify crate-internal behavior. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | rg (deps, consts); sed quote verification; baseline citation |

## 1. One-paragraph verdict
These are real traps, not paperwork. Every fixture is a hand-emitted wasm binary (via tests/common's own emitter) that genuinely attacks the host: OOB guest pointers (`i32c(0x4000_0000)`), negative pointers, a symlink jail escape and `../` traversal, an undeclared import, a WASI import, an allowlist-violating fetch, `loop { br 0 }` fuel spin, a page-by-page memory grow with module max 100 pages ABOVE the 1 MiB host cap, I/O burst spam, cap+1 slurps, and fabricated handles. The host side enforces all of it with real wasmi primitives (fuel metering, store limiter, `trap_on_grow_failure`) — verified in src/instance.rs, src/imports.rs, src/state.rs, src/fsjail.rs. All 9 tests passed in the orchestrator baseline. The gaps: the session ledger's cross-instance semantics are never tested with two instances; the traversal probe exercises the not-found denial branch, not the resolves-outside branch; and the suite is dead code from the shipped product's perspective — nothing depends on this crate.

## 2. What actually works (verified)
— baseline audit/runs/baseline/cargo-test-workspace.txt:5000-5012: "running 9 tests ... test result: ok. 9 passed; 0 failed" (all §9.4 tests).
— Fuel enforcement chain verified: `engine()` sets `consume_fuel(true)` (src/instance.rs:320-324), `invoke` calls `set_fuel(limits.call_fuel)` (src/instance.rs:186); a spin loop would hang the test to timeout if fuel were off — mutation kills it.
— Memory cap chain verified: `StoreLimitsBuilder::new().memory_size(...).trap_on_grow_failure(true)` (src/instance.rs:81-88); `is_memory_denied` maps `GrowthOperationLimited` → `MemoryLimit` (src/instance.rs:379-389).
— fsjail escape denial verified: canonicalize-based, component-aware prefix, reasons contain "sandbox roots" (src/fsjail.rs:104-133).

## 3. Findings
F-001 [S3]
Where: crates/plugin-host-wasm/tests/plugin_sandbox_security.rs:640-685 vs src/state.rs:71-74
Code:
```
    // THE HOST SURVIVES: a fresh instance of the same module still
    // runs (session-ledger disable is per spawn share; the registry
    // wide disable is the host scheduler's concern, issue #46).
```
What is wrong: `SecurityLedger` is documented "session-wide ... shared by every instance of an extension" (src/state.rs:71-72), but no test ever spawns TWO instances sharing one ledger. Every test either uses a fresh `SecurityLedger::new()` per spawn or one instance. If the ledger were silently per-instance, every assertion in this file still passes.
Why it matters: the session-disable property (3 escalations across all instances of an extension) is claimed by doc 06 §5.3 and untested.
What correct looks like: a test spawning two instances over one `ArcLedger`, driving 2 escalations on instance A and 1 on instance B, asserting B faults.
Confidence: HIGH — Verification: read (rg `spawn_with_ledger` — used only with a single instance, L449, L649).

F-002 [S3]
Where: crates/plugin-host-wasm/src/limits.rs:85-89 (cross-ref; affects this suite's claims)
Code:
```
/// 4 active instances per host (doc 06 §5.4 row 8).
pub const HOST_MAX_ACTIVE_INSTANCES: usize = 4;

/// 256 MiB total extension heap across instances (doc 06 §5.4 row 9).
pub const HOST_TOTAL_HEAP_BYTES: u64 = 256 * 1024 * 1024;
```
What is wrong: the host-level accounting ceilings (§5.4 rows 8-9) are declared and never referenced by any code or test (rg across crates/ finds only the declaration). This suite tests per-instance limits only; nothing bounds 4+ concurrent instances or a 256 MiB aggregate heap.
Why it matters: the doc's host-survival posture ("THE HOST SURVIVES") is proven only for sequential fresh spawns, not concurrent instances. An embedder spawning N instances has no enforced aggregate ceiling.
What correct looks like: enforce in the supervisor or delete the constants as unimplemented doc debt.
Confidence: HIGH — Verification: read (rg HOST_MAX_ACTIVE_INSTANCES|HOST_TOTAL_HEAP_BYTES → 2 hits, both declarations).

F-003 [S4]
Where: crates/plugin-host-wasm/tests/plugin_sandbox_security.rs:365-379
Code:
```
    assert_eq!(
        ok(fs.invoke("open_traversal", &[])),
        vec![1],
        "parent traversal is E_PERM"
    );
```
What is wrong: `../outside.txt` is never created (media_jail writes only a.txt, escape/, nope.txt never exists either — L72-75 data segments name files that don't exist). The traversal denial therefore exercises the NotFound fall-through branch ("path ... not found inside the sandbox roots", fsjail.rs:127-132), not the canonicalizes-outside branch (fsjail.rs:111-117). The unix symlink leg covers that branch; the traversal leg would also pass with a jail that merely refuses nonexistent paths.
Confidence: MEDIUM — Verification: read (fsjail.rs:100-133; fixture data segments L72-75).
What correct looks like: create `tmp/outside.txt` so the traversal candidate exists and canonicalizes outside the root.

F-004 [S4]
Where: crates/plugin-host-wasm/tests/plugin_sandbox_security.rs:620-634
Code:
```
    let mut wired = ok(spawn(
        &http_bytes,
        manifest(&manifest_toml(
            &http_bytes,
            "http-fetch = true\nnetwork-host-allowlist = [\"good.test\"]",
        )),
```
What is wrong: the wired-transport happy case never asserts the post-fetch bucket/rate state. `http_get` charges `io_bucket.take(out_cap)` = 960 of 1024 tokens (src/imports.rs:759), so a second `fetch_ok` would be E_QUOTA with 64 tokens left — the charge-by-capacity-not-bytes behavior is untested. Also: charging capacity rather than actual bytes is the conservative choice but means an honest 22-byte body burns 960 tokens; no test pins this policy.
Confidence: HIGH — Verification: read (imports.rs:752-764 arithmetic).
What correct looks like: a second invoke asserting the resulting status, pinning the charge policy.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| oob_wasm | 37-62 | REAL | — | OOB poker: ptr 0x4000_0000, ptr -1, valid call. |
| fs_wasm | 66-149 | REAL | — | 4 open probes incl. `escape/secret.txt` + `../outside.txt`; bogus handle 999; handle 7; 3-open burst. |
| http_wasm | 152-181 | REAL | — | evil.test vs good.test URLs. |
| spin_wasm | 184-189 | REAL | — | `loop { br 0 }` fuel eater. |
| memhog_wasm | 194-215 | REAL | — | Module max 100 pages > 1 MiB host cap — denial provably from limiter. |
| spam_wasm | 219-254 | REAL | — | 3×600 B records vs 1 KiB burst; 2000 B vs cap. |
| open_name_wasm | 258-275 | REAL | — | Cap-boundary slurp poker. |
| devices_wasm | 279-290 | REAL | — | 1024 B out buffer = exactly the cap. |
| media_jail | 294-312 | REAL | — | Unix symlink to sibling secret dir; Windows honestly skips. |
| plugin_sandbox_escape | 318-382 | REAL | — | E_OOB=4, E_ARG=2, event shape-only, E_PERM escapes, happy path still works. |
| plugin_permission_escalation | 388-467 | REAL | — | UndeclaredImport, UnknownImportModule (WASI), 3-strike disable. |
| plugin_trap_loop | 473-510 | REAL | — | CpuLimit + Faulted + InstanceFaulted + survivor instance. |
| plugin_memory_exhaustion | 512-539 | REAL | — | MemoryLimit{cap 1 MiB} + survivor spawn. |
| hello_for_survivor | 542-554 | REAL | — | Minimal healthy module. |
| plugin_io_abuse | 560-634 | REAL | — | Trace [0,7,7], E_TOO_LARGE=6 before bucket charge, E_TRANSPORT=8, E_QUOTA=7, wired body bytes. |
| oob_probes_trip_the_disable_threshold | 640-685 | REAL | — | Sweep-2 regression: 3 OOB probes fault. |
| fs_open_slurp_is_bounded | 687-745 | REAL | — | exact.bin=cap ok, over.bin=cap+1 E_TOO_LARGE, no escalation, Ready. |
| devices_list_is_bucket_metered | 747-778 | REAL | — | Second listing E_QUOTA; refusal not an escalation. |
| plugin_host_pointer_smuggling | 784-885 | REAL | — | E_HANDLE=3 for 999/7/double-close; full lifecycle byte-exact; handle table E_LIMIT=9 trace [0,0,9]. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| plugin_sandbox_escape | YES | Asserts typed codes 4/2/1/0, event count, reason/shape substrings; a jail bypass would return EIo(5) or Ok(0) on open. |
| plugin_permission_escalation | YES | UndeclaredImport/UnknownImportModule are matched structurally; threshold fault is a HostError variant, not a string. |
| plugin_trap_loop | YES | Without fuel the spin hangs → harness timeout; with fuel the CpuLimit match is exact. |
| plugin_memory_exhaustion | YES | Module max 100 pages removes the module-bounds explanation; MemoryLimit.cap_bytes==1 MiB checked. |
| plugin_io_abuse | YES | Import-trace equality pins the throttle-then-refuse order. |
| oob_probes_trip_the_disable_threshold | YES | Third probe must fault; pre-fix behavior (E_OOB forever) would fail L662-669. |
| fs_open_slurp_is_bounded | YES | cap+1 file must yield 6 and stay Ready; an unbounded slurp would pass and return 0. |
| devices_list_is_bucket_metered | YES | Second listing must be 7 with empty ledger; pre-fix (cap-only) would return 0. |
| plugin_host_pointer_smuggling | YES | Byte-exact content, size, double-close, and [0,0,9] trace equality. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every escalation test DENIED, at the real wasm boundary" | file header L1-17 | REAL | Baseline 9/9 passed; fixtures execute through wasmi. |
| "fuel budget ... deterministic: exactly call_fuel instructions" | L481-482, limits.rs L11-14 | REAL-UNPROVEN (determinism) | engine consume_fuel(true) + set_fuel verified; exact-instruction count not measured (no counter assertion). |
| "the host keeps serving" | L10-11 | SHALLOW | Only sequential fresh spawns tested; no concurrent-instances host-survival test (F-002). |

## 7. Dead / duplicated / unreachable code
None in this file. Crate-level: the entire host (incl. this suite) is unreachable from shipped entry points — no reverse deps (see header table).

## 8. Dependencies & flags
Test-only deps via tests/common: none beyond crate + plugin-api. wasmi 2.0.0 `default-features=false, ["std","validate","auto-dispatch"]` (Cargo.toml:57 — the auto-dispatch note documents a real debug-build stack-overflow without it).

## 9. Open questions for the next agent
- Is `supervisor.rs` supposed to enforce HOST_MAX_ACTIVE_INSTANCES/HOST_TOTAL_HEAP_BYTES? If yes it is missing; if no the constants are dead doc debt.
- Does any matrix row cite this suite as evidence for a shipped plugin feature? (would make the UNREACHABLE status a doc-vs-reality finding).
