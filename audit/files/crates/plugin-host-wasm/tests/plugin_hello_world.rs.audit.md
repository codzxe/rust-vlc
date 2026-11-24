# AUDIT: crates/plugin-host-wasm/tests/plugin_hello_world.rs
| Field | Value |
|---|---|
| Auditor | W12-A06 |
| Date | 2026-11-24 |
| Lines / functions | 184 / 2 tests + 2 fixture builders (hello_limits, hello_wasm) |
| File verdict | REAL (executed) — an honest end-to-end sandbox test |
| Reachable from shipped entry point? | N/A (test file); runs in the workspace test lane — verified in baseline cargo-test-workspace.txt:4982-4986 and by my own run |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `cargo test -p plugin-host-wasm --test plugin_hello_world` -> 2 passed, 0 failed; read tests/common/mod.rs (real wasm binary emitter; spawn_with_limits -> WasmInstance::spawn with real wasmi engine + fresh SecurityLedger); read imports.rs:300-345 (host-side manifest-lie verification); sed verification of quotes |

## 1. One-paragraph verdict
This is what an acceptance test should be: it emits a real wasm binary in-test
(std-only emitter in tests/common, validated by wasmi's `Module::new`), spawns
it through the REAL host path (`WasmInstance::spawn` + real `SecurityLedger`),
and then verifies every claim in its header with specific values — the
api-version handshake answers the engine major, the `hello` export drives one
`log_write` and one `report_manifest` with exact expected status, the reported
manifest is recorded verbatim AND parsed (id `ext.acme.hello`, api_version 1),
the log record arrives with level and text intact, a second dispatch proves
nothing faulted, the host→guest buffer direction round-trips and an
out-of-bounds host write is the typed `GuestMemoryOob` denial with exact
ptr/len. The second test proves api_version 2 is the typed window error. I
executed both: pass. Zero findings.

## 2. What actually works (verified)
— verified by `~/.cargo/bin/cargo test -p plugin-host-wasm --test
plugin_hello_world` -> "test result: ok. 2 passed; 0 failed".
— verified by reading the chain the test depends on: tests/common/mod.rs:606
spawn_with_limits -> `WasmInstance::spawn(&engine(), ...)` (real wasmi);
imports.rs:311-321 the host compares the reported manifest's id/kind/version/
api_version against the installed identity (so the test's accepted report is
genuinely host-verified, not just recorded).

## 3. Findings
None. What was checked, and how:
- Fakeness: no mocks of the system under test (the only double in common is
  `OkTransport`, not used by this suite); no constants asserted against
  themselves (api asserted against plugin_api::ENGINE_PLUGIN_API_MAJOR, the
  negotiated source of truth).
- Correctness: fixture byte counts checked by hand — greeting is exactly 36
  bytes (matches `i32c(36)`), data segments at 0 and 64 do not overlap the
  128-offset echo payload region; the OOB write (ptr 1_048_572, len 8) really
  crosses the 1 MiB tiny-cap boundary.
- Test honesty (mutation reasoning): swapping any import status, mangling the
  manifest report, dropping the log record, or breaking the bounds check
  fails a specific assert (each has its own assert_eq/matches!).
- Hidden skips: no #[ignore], no cfg gating beyond the platform-neutral kind;
  both tests ran in the orchestrator baseline (cargo-test-workspace.txt:4985-4986).
- Stack-order check: the `hello` body pushes (level, ptr, len) in call order —
  verified against log_write's (I32, I32, I32) signature in api.rs's catalog.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| hello_limits | 22-27 | REAL | — | tiny budget + v1 descriptor cap; comment explains why |
| hello_wasm | 34-66 | REAL | — | real emitter-built module: 3 imports, 2 data segments, 2 exports |
| hello_world_extension_runs_and_reports_manifest | 69-158 | REAL | — | see §5 |
| api_version_mismatch_is_the_typed_error | 160-184 | REAL | — | typed ApiVersion{found:2,min:1,max:1} asserted |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| hello_world_extension_runs_and_reports_manifest | YES | every step has a specific assertion: exact status vectors, exact log tuple, exact reported-manifest bytes, parsed id/api_version, re-dispatch status, exact OOB ptr/len in the typed error; a no-op host fails at the first assert |
| api_version_mismatch_is_the_typed_error | YES | the string replace makes a genuinely different manifest; a host that silently loaded v2 hits the `unreachable!` |
Both executed by me: 2 passed, 0 failed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a real wasm module spawns under wasmi" | plugin_hello_world.rs:1-2 | REAL | spawn goes through WasmInstance::spawn(engine()) — common/mod.rs:606-617 |
| "negotiates the API version through `lumen:v1.host`" | :3-4 | REAL | check_api assertion + negotiate path at instance.rs:76 |
| "the host verifying every step against the installed identity" | :5-6 | REAL | imports.rs:311-321 id/kind/version/api_version comparison; mismatch = escalation event |

## 7. Dead / duplicated / unreachable code
None in this file. (The `ok`/`some` helpers live in tests/common — noted as
duplication in the api.rs/fsjail.rs audits already.)

## 8. Dependencies & flags
Test-only; uses crate internals (limits, state, HostError) + plugin_api.
No feature gates. The suite requires no network, no wat crates.

## 9. Open questions for the next agent
- plugin_sandbox_security.rs auditor: confirm the manifest-LIE branch
  (imports.rs:330-345) has a test that reports a mismatched identity and
  asserts the escalation count — the happy branch is covered here, the denial
  branch is this suite's natural complement.
