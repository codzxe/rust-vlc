# AUDIT: crates/platform-linux/src/linux/pulse_ffi.rs
| Field | Value |
|---|---|
| Auditor | W08-A10 |
| Date | 2026-11-24 |
| Lines / functions | 402 / 1 (`load`) + PulseApi table (20 fn ptrs) + 12 struct types + 4 callback typedefs + `sym!` macro |
| File verdict | REAL-UNPROVEN (load + connect-phase executed on this host; sink-callback marshalling never ran) |
| Reachable from shipped entry point? | NO — no workspace crate depends on `platform-linux` (rg over all Cargo.tomls: only its own) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `nm -D` loop over 20 symbols (all OK, exit 0); ctypes runtime probe (exit 0, output below); `rg pulse .github/workflows/*.yml` (exit 1 — no matches); sed -n quote checks (exit 0); baseline cargo-test-workspace.txt:3964-3965 |

## 1. One-paragraph verdict
libpulse.so.0 (PulseAudio 0.24.3) IS installed in this sandbox, so the bindings were
verified against the real library: all 20 dlsym symbols exist, 8 signatures were
executed directly via ctypes (pa_strerror text, mainloop new/get_api/free,
context_new/get_state/unref), and the crate's own load()+connect path ran against the
real library in the baseline tests (both pulse tests passed, producing the typed
ServiceUnavailable). Statically, every prototype matches the documented libpulse ABI
(const-qualification and enum-signedness differences are ABI-identical). The two real
defects are documentation lies: the transmute's soundness argument cites CI cross-checks
that do not exist, and the header miscounts its own function table (21 claimed, 20 present).

## 2. What actually works (verified)
— `nm -D /lib/x86_64-linux-gnu/libpulse.so.0` -> all 20 declared symbols exported (PA 0.24.3).
— ctypes against the real library: `pa_strerror(0) -> b'OK'`, `pa_strerror(-1) -> b'Access denied'`,
  `pa_mainloop_new -> 0x2deec70`, `pa_mainloop_get_api -> 0x2deecc8`, `pa_context_new(api,name) -> 0x2df1d80`,
  `pa_context_get_state -> 0 (PA_CONTEXT_UNCONNECTED)`, `pa_context_errno -> 0`, unref+free clean, no crash.
— Baseline cargo-test-workspace.txt:3964-3965: `pulse_probe_reports_this_hosts_truth ... ok`,
  `pulse_subscribe_without_a_server_is_the_typed_error ... ok` — the crate's load() (all 20
  transmutes) and the connect phase executed against the real libpulse.so.0 on this host.
— Static prototype diff against pulse/mainloop.h, context.h, introspect.h, operation.h,
  error.h (headers NOT installed — /usr/include/pulse absent; verified against the documented
  API from knowledge): all 20 match. PaSinkInfo field-for-field v1.0 shape, alignment trace
  = 416 bytes, identical to the C layout; pulse.rs:209-227 reads only
  name/index/description/sample_spec as the doc claims.

## 3. Findings
F-001 [S3]
Where: crates/platform-linux/src/linux/pulse_ffi.rs:268-273
Code:
```
    // the exact documented signature. SAFETY (shared invariant):
    // the transmute only re-types a dlsym address — sound iff the
    // signature matches the library's prototype, which is checked
    // against pulse/mainloop.h / pulse/context.h / pulse/introspect.h
    // here and pinned by the compile-only cross-checks in CI.
```
What is wrong: No CI workflow pins these signatures: `rg pulse .github/workflows/*.yml`
-> zero matches (exit 1). "checked against pulse/mainloop.h … here" is also hollow — no
pulse headers exist in the repo or sandbox; nothing mechanical checks anything. The
soundness argument for 20 `transmute`s cites evidence that does not exist. (The
signatures are in fact correct — I verified — so this is a false evidence claim, not a
live soundness bug.) What correct looks like: delete the CI claim, or actually add a
compile-only C harness declaring the prototypes and calling through the table.
Confidence: HIGH. Verification: ran (rg over .github/workflows).

F-002 [S4]
Where: crates/platform-linux/src/linux/pulse_ffi.rs:4-6
Code:
```
//! Scope: exactly the 21 functions + 3 value structs the
//! enumeration/notification seam needs (the synchronous `pa_mainloop`
//! family — no threads, bounded waits via `pa_mainloop_prepare`).
```
What is wrong: PulseApi has 20 function-pointer fields (mainloop 6, context 10,
operation 3, strerror 1) and load() resolves 20 symbols. "21 functions" is wrong.
Why it matters: a reader auditing "the 21" against the table would hunt a missing
entry; the count is the doc's own inventory. Confidence: HIGH (counted).
Verification: read.

## 4. Per-function table
| Function/type | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| opaque types (PaMainloop/Api/Context/Operation/SpawnApi/Proplist/FormatInfo) | 36-65,108-112 | REAL | — | `[u8;0]` never deref'd; correct token pattern |
| PaSampleSpec | 71-81 | REAL | — | format c_int (enum), rate u32, channels u8 — matches pulse/sample.h |
| PaChannelMap | 83-90 | REAL | — | u8 + [c_int;32] — matches channelmap.h (PA_CHANNELS_MAX=32) |
| PaCvolume | 92-99 | REAL | — | u8 + [u32;32] (pa_volume_t=u32) — matches volume.h |
| PaSinkPortInfo | 101-106 | REAL | — | name-only prefix; never deref'd beyond existence per doc |
| PaSinkInfo | 114-168 | REAL-UNPROVEN | — | Layout statically correct (416B trace); marshalling needs a live server — never executed anywhere |
| 4 callback typedefs | 174-192 | REAL-UNPROVEN | — | Match documented cb prototypes; sink-info/subscribe cbs never fire in any test (no server) |
| PulseApi (20 fn ptrs) | 199-259 | REAL | — | All 20 prototypes match; 8 executed via my ctypes probe; connect-phase executed by baseline tests; `mainloop_prepare` timeout in ms + negative=forever comment is correct |
| `sym!` macro | 274-280 | REAL | — | Standard dlsym transmute; soundness rests on F-001's phantom CI check |
| load | 267-402 | REAL | — | Executed on this host (baseline :3964-3965); first missing symbol -> SymbolMissing |

## 5. Test honesty
No tests in this file. Crate-level: `pulse_probe_reports_this_hosts_truth` executes
load()+connect against the real host library and asserts the typed no-server error —
it WOULD fail if load()'s transmutes or the connect-phase prototypes were wrong (it
reached ServiceUnavailable, i.e. pa_context_connect/errno/strerror all ran). It would
NOT catch a wrong PaSinkInfo layout — the sink callback never fires without a server;
that riskiest surface has zero runtime coverage (UNVERIFIED: no PA daemon in sandbox).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exactly the 21 functions" | pulse_ffi.rs:4 | FALSE | PulseApi has 20 fields (counted) |
| "pinned by the compile-only cross-checks in CI" | pulse_ffi.rs:273 | FALSE | rg over .github/workflows: no pulse anywhere |
| "later PulseAudio versions only *appended* fields" (PaSinkInfo prefix safety) | pulse_ffi.rs:12-14 | REAL-UNPROVEN | True through PA 17 per release history; headers absent, no mechanical check |
| "Everything else of PulseAudio is deliberately NOT declared" | pulse_ffi.rs:7 | REAL | Only the 20 symbols appear; rg pa_ in crate confirms |

## 7. Dead / duplicated / unreachable code
Not dead at the crate level (used by pulse.rs), but the whole crate is unwired from
the product: no Cargo.toml outside platform-linux's own names it. lumen-cli/desktop
never link it.

## 8. Dependencies & flags
No -sys crate (AGENTS.md zero-dep rule honored); dlopen at runtime via
`crate::linux::dl::DynamicLibrary`; `#![allow(unsafe_code)]` needed against the
workspace `unsafe_code = "deny"` lint. `#[allow(clippy::struct_field_names)]` on
PulseApi is cosmetic.

## 9. Open questions for the next agent
- pulse.rs holds the actual call discipline (bounded prepare/poll/dispatch waits,
  operation_cancel on timeout) — audit whether `mainloop_prepare`'s ms timeout is
  ever passed a µs value by mistake.
- If a PA daemon is ever available in CI, one live-server enumeration test would
  close the PaSinkInfo-marshalling gap (the only UNPROVEN surface here).
