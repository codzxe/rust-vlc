# AUDIT: crates/plugin-host-wasm/tests/common/mod.rs
| Field | Value |
|---|---|
| Auditor | W12-A09 |
| Date | 2026-11-24 |
| Lines / functions | 767 / 43 (incl. methods, helpers, fixtures) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A — test-fixture module; reached by `cargo test -p plugin-host-wasm` (5 suites `mod common;`), not by lumen-cli/desktop release lane (correct for a tests/common) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg usage counts (exit 0); python3 byte-length check (exit 0); sed quote verification (exit 0); baseline citation |

## 1. One-paragraph verdict
Real, working test-fixture infrastructure: a std-only wasm binary emitter (LEB128, sections, imports, memories, code, data), a stored-entry ZIP writer with deliberate-corruption support, manifest/package builders wired to the crate's real `module_digest`, spawn wrappers, and six crash-injection wasm fixtures. All of it executes in the orchestrator baseline: 26 integration tests across the 5 suites that include this module passed (exit_code=0). The emitter is honest — wasmi's validating `Module::new` (feature `validate` confirmed in Cargo.toml:57) is on the execution path, so an emitter bug would fail loudly. Two hygiene defects: an off-by-one URL length in the escalation fixture (relies on zero-initialized wasm memory) and one dead helper masked by `#![allow(dead_code)]`.

## 2. What actually works (verified)
— baseline `audit/runs/baseline/cargo-test-workspace.txt:4884-5022`: plugin-host-wasm exit_code=0; all 6 crash-isolation tests (which run every fixture in this file), 4 install-lifecycle, 9 sandbox-security, 5 conformance, 2 hello-world tests passed. Fixture emitter output is validated+executed by wasmi.
— python3: `len('https://evil.test/x')==19`, `len('about to trap')==13`, `len('survivor-still-alive')==20` — two of three fixture string lengths match their hardcoded lens; one does not (F-001).
— rg -cF usage counts: all pub helpers used across suites except `i64c` (0 uses).

## 3. Findings
F-001 [S4]
Where: crates/plugin-host-wasm/tests/common/mod.rs:693-699
Code:
```
    w.data(0, b"https://evil.test/x");
    let mut body = Vec::new();
    for _ in 0..3 {
        body.extend_from_slice(&i32c(0));
        body.extend_from_slice(&i32c(20));
```
What is wrong: `https://evil.test/x` is 19 bytes (python3-verified) but the fixture passes `url_len = 20`; the host (imports.rs `http_get` → `String::from_utf8_lossy`) reads one byte past the data segment, a NUL from zero-initialized memory, so the URL the host parses is `https://evil.test/x\0`. Why it matters: the test still gets its allowlist denial (NUL lands in the path, host `evil.test` parses clean), but the fixture's correctness silently depends on wasm zero-init semantics; any fixture copied from this shape for a test where URL bytes matter (e.g. path-equality assertions) would corrupt its own input. What correct looks like: `&i32c(19)` or `b"https://evil.test/x".len() as i32`. Confidence: HIGH — Verification: ran (python3 len + read of imports.rs read_guest path).
F-002 [S4]
Where: crates/plugin-host-wasm/tests/common/mod.rs:1,349-354
Code:
```
#![allow(dead_code)]
```
and
```
/// `i64.const v`.
pub fn i64c(v: i64) -> Vec<u8> {
    let mut out = vec![0x42];
    sleb(v, &mut out);
    out
}
```
What is wrong: `i64c` has zero call sites in all 5 suites (rg -cF "i64c(" → 0); the file-level `#![allow(dead_code)]` masks it and any future rot. Why it matters: dead fixture code presented as part of the "everything §9.4/§9.5 needs" surface; the allow defeats the compiler's unused warning for the whole module. What correct looks like: delete `i64c` or scope the allow to genuinely shared-but-unused items. Confidence: HIGH — Verification: ran (rg counts).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ok / some | 41-54 | REAL | — | panic-on-Err test helpers; 155/3 uses |
| TempDir::new/path/drop | 60-83 | REAL | — | unique via pid+nanos; best-effort cleanup |
| uleb / sleb | 95-119 | REAL | — | correct LEB128 encodings (arithmetic shift for signed); exercised by every fixture |
| section / name | 121-130 | REAL | — | section framing, length-prefixed names |
| Wasm::{new,memory,ty,import,func,export_func,start,data,finish} | 159-328 | REAL | — | full MVP wasm emitter; validated+run by wasmi in baseline |
| code/i32c/call/dropv/lget/ltee/mem_grow/unreachablev/br_if/brv/loopv/endv/nev | 334-419 | REAL | — | instruction byte helpers; opcode values correct (0x41 i32.const, 0x10 call, 0x40 0x00 memory.grow, 0x03 0x40 loop, 0x47 i32.ne) |
| i64c | 350-354 | DEAD | S4 | F-002 |
| zip_stored / zip_raw | 427-498 | REAL | — | correct ZIP layout (local+central+EOCD); corrupt knob flips CRC via `^ 0xFFFF_FFFF` (guaranteed wrong) and/or method; corruption applied to BOTH local and central headers (445/467) so a one-sided reader check is still covered |
| manifest_toml / reported_manifest_toml | 507-534 | REAL | — | real `module_digest`; reported variant deliberately not byte-equal (honest per doc comment) |
| package / signature_block | 538-557 | REAL | — | manifest+[signature]+module zip; 17 uses |
| manifest / spawn / spawn_with_ledger / spawn_with_limits | 565-620 | REAL | — | thin wrappers over real `WasmInstance::spawn` with real engine/limits/ledger; 28+ uses |
| OkTransport | 628-637 | REAL | — | fixed-body transport double; used at plugin_sandbox_security.rs:627 |
| crash_trap_on_entry_wasm | 648-655 | REAL | — | all exports `unreachable`; used by 3 tests |
| crash_trap_mid_call_wasm | 659-678 | REAL | — | log_write then trap; len 13 correct |
| crash_escalate_wasm | 685-708 | REAL | S4 | works, but F-001 off-by-one |
| crash_fuel_burner_wasm | 714-724 | REAL | — | `loop br 0`; fuel-kill proven in baseline |
| crash_oom_wasm | 730-751 | REAL | — | module max 100 pages > 1 MiB host cap → denial provably from limiter |
| crash_survivor_wasm | 755-767 | REAL | — | len 20 correct; survivor asserted alive in 2 tests |

## 5. Test honesty
This file IS the fixture layer; honesty judged by whether consumer tests would fail if fixtures lied: the crash fixtures feed the real wasmi host — a fabricated "trap" (e.g. body that returns instead of trapping) would fail `crash_injection_*` containment assertions; a broken emitter fails wasmi validation at spawn. Baseline run confirms all 26 consumer tests execute and pass. The `manifest_toml` digest is computed from the real module bytes (line 508), so a fake digest would fail install-lifecycle's provenance checks.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "wasmi's validating Module::new (the validate feature) rejects any emitter bug loudly" | mod.rs:12-14 | REAL | Cargo.toml:57 `features = ["std","validate","auto-dispatch"]`; baseline tests compiled+passed |
| "CRCs via the crate's own crc32" | mod.rs:15-17 | REAL | line 445 calls `crc32` imported from `plugin_host_wasm::crc32` (line 32) |
| "[provenance].hash is the real digest of the module" | mod.rs:18-20 | REAL | line 508 `module_digest(wasm)` |

## 7. Dead / duplicated / unreachable code
`i64c` (F-002). Everything else used (rg counts in §2). Not reachable from release lane — by design, it is test-only.

## 8. Dependencies & flags
Imports only `plugin_host_wasm` pub surface (http, instance, limits, package::module_digest, state, HostError, crc32) + `plugin_api::PluginManifest` — all resolve (baseline compile). No dev-deps (no wat/wast) — matches crate discipline claim in Cargo.toml comment.

## 9. Open questions for the next agent
- Whether package.rs's ZIP reader checks method==0 on BOTH local and central headers or one side (fixture corrupts both, so its tests can't distinguish a one-sided check).
