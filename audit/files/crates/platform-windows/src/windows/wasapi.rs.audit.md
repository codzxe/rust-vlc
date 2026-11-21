# AUDIT: crates/platform-windows/src/windows/wasapi.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-21 |
| Lines / functions | 787 / 27 (incl. 13 consts, 4 vtbl/struct types) |
| File verdict | REAL-UNPROVEN (compiles for the Windows target per orchestrator run; runtime never executed anywhere) |
| Reachable from shipped entry point? | NO in the shipped Linux build (module is `#[cfg(target_os = "windows")]`, compiled out; no workspace member depends on `platform-windows` at all). On a Windows-target build: YES — `WindowsPlatform::discover` → `audio_seam()` (lib.rs:143-158) → `WasapiAudioDevices::new()` + `enumerate()`. |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 2 |
| Commands run | `sed -n`/`Read` (full file, 1-787); `od -c` line 587 (proved Bash tool display mangles `#[m`→`` — file really contains `#[must_use = …]`); `rg -n "WasapiAudioDevices|wasapi::" crates/` (only caller: platform-windows/src/lib.rs:146); Grep `platform_windows` across `**/*.rs` (only its own tests/seam_behaviour.rs); Grep `platform-windows` across `**/Cargo.toml` (no dependents); `git blame -L 585,590` (line dates to origin commit cf71390a); `git diff --stat e83e5bd3..HEAD -- crates/` (empty — tree unchanged since baseline). Baseline: audit/runs/baseline/orchrun-check-platform-windows.log.txt (`cargo check -p platform-windows --target x86_64-pc-windows-msvc`, EXIT=0). |

NOTE on quote verification: the Bash tool display eats the byte pair `[m` after
`#` (e.g. `#[must_use]` displays as `#ust_use]`). Every quote below was verified
with the Read tool (true bytes) and, where `#[` appears, `od -c`. Prior agents
(W02-A07 log) documented the same artifact.

## 1. One-paragraph verdict
This is genuine, carefully-structured hand-rolled COM/WASAPI code — correct GUID
constants, correct vtable layouts in SDK order, single-release discipline on
every COM reference, a bounded push queue, and an honestly-documented "Release
never frees" lifetime model. It is compile-verified for the Windows target
(orchestrator run, EXIT=0) but has **zero runtime evidence and zero test
coverage**, it is **compiled out of the shipped Linux binary**, and the crate it
lives in is an **orphan** — no workspace member depends on `platform-windows`,
so no shipped entry point on any platform currently reaches it. Two code-level
defects stand out: `enumerate()` joins its worker thread with no deadline
(violating the crate's own "no unbounded waits" directive D-2) on the app-startup
path, and `QueryInterface` refuses `IID_IMMNotificationClient` — the very
interface the object is registered as — a COM-rules violation whose runtime
consequence (registration failure or dead notifications) is unproven.

## 2. What actually works (verified)
— GUID/HRESULT/PKEY constants match mmdeviceapi.h values (CLSID_MMDeviceEnumerator
{BCDE0395-…}, IID {A95664D2-…}, E_NOINTERFACE 0x80004002, PKEY_Device_FriendlyName
fmtid {A45C254E-…} pid 14 / DeviceDesc pid 2) — verified by reading lines 63-122
against the documented ABI values.
— Compile for the Windows target: verified by
`audit/runs/baseline/orchrun-check-platform-windows.log.txt` →
`cargo check -p platform-windows --target x86_64-pc-windows-msvc` EXIT=0
(2026-11-21T07:07:29Z). Runtime behavior: NOT executed anywhere (no Windows
host in the audit sandbox).

## 3. Findings

### F-001 [S2]
Where: crates/platform-windows/src/windows/wasapi.rs:614-621 (with 603-613)
Code:
```
        match outcome.join() {
            Ok(result) => result,
            Err(_) => Err(PlatformError::CallFailed {
                api: "WASAPI enumeration thread",
                code: None,
                detail: "the enumeration thread panicked".to_string(),
            }),
        }
```
What is wrong: `enumerate()` spawns the STA enumeration thread and then
`join()`s it with **no deadline**. `list_endpoints` performs
`CoCreateInstance`/`EnumAudioEndpoints`/`GetCount`/per-device `GetId`/
`OpenPropertyStore` — all of which are exactly the calls known to block
indefinitely on misbehaving audio drivers on real Windows. The module enforces
its own directive D-2 ("no unbounded waits", line 124 `WATCH_DEADLINE`) only for
`subscribe`; the enumeration lane has no bound at all.
Why it matters: `audio_seam()` (platform-windows/src/lib.rs:146-148) calls
`devices.enumerate()` inside `WindowsPlatform::discover` — the startup probe. A
hung driver blocks `join()` forever and hangs application startup with no typed
error, no timeout, and no diagnostics (the crate has no logging).
What correct looks like: a deadline on the join (e.g. park the thread with a
timeout channel like the subscribe lane, or `recv_timeout` on a result channel
with a documented bound), returning `PlatformError::TimedOut` — mirroring the
`subscribe` design 50 lines below.
Confidence: HIGH — Verification: read (code fact: `join()` has no timeout by
API contract); impact reasoning UNVERIFIED at runtime (no Windows host).

### F-002 [S2]
Where: crates/platform-windows/src/windows/wasapi.rs:442-448
Code:
```
        *ppv = std::ptr::null_mut();
        if (*riid) == IID_IUNKNOWN {
            client_add_ref(this);
            *ppv = this;
            S_OK
        } else {
            E_NOINTERFACE
        }
```
What is wrong: the `NotificationClient` is registered with
`RegisterEndpointNotificationCallback` **as an `IMMNotificationClient`** (the
vtable implements all five methods), but `QueryInterface` answers only
`IID_IUNKNOWN` and returns `E_NOINTERFACE` for everything else — including
`IID_IMMNotificationClient`. COM's cardinal rule is that QI for any interface
the object exposes must succeed; this object hands out an `IMMNotificationClient`
vtable it simultaneously refuses to acknowledge.
Why it matters: if the WASAPI enumerator (or any marshaling path) QIs the client
for `IID_IMMNotificationClient`, registration can fail or the reference counting
etiquette breaks — and the failure mode is silent (device-change notifications
just never arrive). Microsoft does not document that the MMDevice API skips QI
on the passed pointer.
What correct looks like: define `IID_IMMNOTIFICATION_CLIENT`
({7991EEC9-7E89-4D85-8390-6C703CEC60C0}), answer it in `client_query_interface`
(AddRef + return `this`), alongside IUnknown.
Confidence: MEDIUM — Verification: read (the refusal is in the code); runtime
consequence UNVERIFIED (no Windows host; behavior of the OS side undocumented).

### F-003 [S3]
Where: crates/platform-windows/src/windows/wasapi.rs:280-282
Code:
```
        if let Ok(Some(desc)) = describe_device(device) {
            devices.push(desc);
        }
```
What is wrong: `describe_device` returns a typed `Err` when `IMMDevice::GetId`
fails (lines 293-307); here the `Err` arm is silently dropped — the device
vanishes from the list with no counter, no log, no event. The crate defines no
logging, so underreporting is undetectable.
Why it matters: "empty list is honest" is the seam contract (audio.rs:31-32);
a list silently missing the device whose GetId hiccuped is not honest, and the
caller cannot distinguish "2 devices" from "3 devices, one dropped".
What correct looks like: propagate a typed partial-failure (count dropped
endpoints, or surface `PlatformError` with the per-device detail the code
already builds).
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/platform-windows/src/windows/wasapi.rs:462-467
Code:
```
        client_of(this)
            .refs
            .fetch_sub(1, Ordering::Relaxed)
            .saturating_sub(1)
```
What is wrong: `fetch_sub` wraps on underflow (atomics wrap), so after an
over-release the stored counter wraps toward `u32::MAX` while the returned
value saturates at 0 — the reported refcount and the internal state diverge.
No memory-safety impact (Release never frees, by design), but the returned
COM etiquette value becomes a lie after any accounting bug — hiding the very
bug it would reveal.
What correct looks like: `fetch_update` with a checked decrement, or document
the wrap; ideally assert the invariant in debug builds.
Confidence: HIGH — Verification: read (wrap semantics of `AtomicU32::fetch_sub`
are documented std behavior).
Severity note: rated S4 only because "Release never frees" removes the safety
consequence; the divergence itself is real.

### F-005 [S4]
Where: whole file (787 lines); evidence: `rg -n "wasapi|Wasapi" crates/platform-windows/tests/` → no matches.
What is wrong: zero test coverage and zero runtime evidence for 787 lines of
hand-rolled `unsafe` COM ABI. The module doc admits it ("runtime NOT exercised
in this sandbox — `[VERIFY]`"), which is honest — but as of Wave 3 no `[VERIFY]`
has ever been discharged, and the workspace test suite (4534 passed / 7 failed
per audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt) exercises
none of this code because it is cfg-compiled-out on the Linux test host.
Why it matters: the audited unsafe surface with the least evidence is exactly
the surface where a single wrong vtable offset or GUID byte silently corrupts
memory or deadlocks.
What correct looks like: at minimum a Windows CI lane running a smoke of
enumerate/subscribe, or an ABI-layout unit test asserting `size_of`/offset
equivalences against the SDK headers.
Confidence: HIGH — Verification: read + rg (no test references).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts CLSID/IID/E_*/DEVICE_STATE_*/PKEY_*/WATCH_DEADLINE | 63-124 | REAL | — | GUID/HRESULT values match mmdeviceapi.h; WATCH_DEADLINE 2 s exists only for subscribe (see F-001) |
| `EnumeratorVtbl` / `CollectionVtbl` / `DeviceVtbl` / `ObjectHeader` | 132-188 | REAL-UNPROVEN | — | repr(C), SDK slot order, unused slots underscore-named; layout unverified at runtime (no Windows host) |
| `preferred_format` | 199-205 | REAL | — | hardcoded 48 k stereo F32, documented as per-endpoint suggestion only |
| `list_endpoints` | 210-223 | REAL-UNPROVEN | — | apartment + CoCreateInstance + single release on all paths |
| `enumerate_with` | 227-253 | REAL-UNPROVEN | — | EnumAudioEndpoints; typed Err on hr != S_OK; releases collection |
| `devices_from_collection` | 256-289 | REAL-UNPROVEN | S3 | GetCount+Item loop; Err/None swallow (F-003); mid-enum device loss = skip (documented) |
| `describe_device` | 293-357 | REAL-UNPROVEN | — | GetId/free_wide/from_wide; property-store name with 2-level fallback; releases store; honest None for empty id |
| `NotificationClient`/`NotificationClientVtbl`/`NOTIFICATION_VTBL` | 362-408 | REAL-UNPROVEN | — | static vtable, repr(C) object, Arc queue + AtomicU32 refs |
| `client_of` | 411-416 | REAL-UNPROVEN | — | unsafe borrow; invariant documented |
| `id_of` | 419-428 | REAL-UNPROVEN | — | null-safe; None → AudioSetChanged advice (honest degradation) |
| `client_query_interface` | 433-450 | REAL-UNPROVEN | S2 | IUnknown-only refusal (F-002) |
| `client_add_ref` | 453-456 | REAL-UNPROVEN | — | atomic; etiquette only |
| `client_release` | 460-468 | REAL-UNPROVEN | S4 | never frees (documented); wrap divergence (F-004) |
| `client_on_device_state_changed` | 471-487 | REAL-UNPROVEN | — | push, no panic path, degrades to AudioSetChanged |
| `client_on_device_added` | 492-501 | REAL-UNPROVEN | — | same shape |
| `client_on_device_removed` | 508-517 | REAL-UNPROVEN | — | same shape |
| `client_on_default_device_changed` | 524-538 | REAL-UNPROVEN | — | filters flow==eRender && role==eConsole; others silently ignored (acceptable: capture flows irrelevant) |
| `client_on_property_value_changed` | 544-554 | REAL-UNPROVEN | — | any property change → AudioSetChanged (coarse but honest) |
| `WasapiAudioDevices` / `Watch` structs | 561-568 | REAL | — | plain data |
| `Drop for Watch` | 571-578 | REAL-UNPROVEN | — | drop sender then join — COM teardown completes before owner proceeds |
| `WasapiAudioDevices::new` | 584-591 | REAL | — | trivial ctor; `#[must_use]` (verified via od: Bash display mangles it) |
| `Default` | 593-597 | REAL | — | delegates to new |
| `AudioDevices::enumerate` | 599-622 | REAL-UNPROVEN | S2 | unbounded join (F-001); thread-per-call (documented, rare op) |
| `AudioDevices::subscribe` | 624-671 | REAL-UNPROVEN | — | replace-on-resubscribe; recv_timeout deadline; all three outcome paths reap the thread — correct |
| `AudioDevices::poll` | 673-678 | REAL | — | `Ok(0)` push-based — matches documented seam contract (audio.rs:56-58) |
| `watch_thread_body` | 681-707 | REAL-UNPROVEN | — | register→report→park→teardown; timeout race handled via dropped sender |
| `NotificationSession` + `register_notification_client` | 709-763 | REAL-UNPROVEN | — | apartment moved into session (lives for parked lifetime); error path releases enumerator; client ptr registered via addr_of_mut — Box not moved |
| `teardown_notification_client` | 765-787 | REAL-UNPROVEN | — | unregister→release→drop Box→apartment last (field order) — documented pairing upheld |

## 5. Test honesty
No tests exist in or for this file (`rg` over `crates/platform-windows/tests/`
finds no wasapi reference). tests/seam_behaviour.rs exercises the typed
fallbacks and queue only. N/A — nothing to grade; recorded as F-005.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "compile-verified via `cargo check --target x86_64-pc-windows-msvc`" | wasapi.rs:20-22 module doc | REAL | audit/runs/baseline/orchrun-check-platform-windows.log.txt, EXIT=0, 2026-11-21T07:07:29Z |
| "runtime NOT exercised … `[VERIFY]` on real-device behavior" | wasapi.rs:22 | REAL (honest admission) | no run anywhere in audit/runs/baseline touches a Windows target at runtime |
| "On any non-Windows host this entire module is compiled out" | wasapi.rs:17-18 | REAL | windows/mod.rs:19-20 `#[cfg(target_os = "windows")] pub mod wasapi;` |
| "Release never frees — the Box on the owning thread governs the lifetime" | wasapi.rs:27-31 | REAL (as code) | client_release:460-468 never deallocates; Box dropped in teardown_notification_client:784-787 |
| WASAPI enumeration/notification wired into the platform seam | platform-windows/src/lib.rs:141-158 | REAL-UNPROVEN | code path exists; no shipped binary on any OS reaches the crate (orphan, see §7) |

## 7. Dead / duplicated / unreachable code
- The whole `platform-windows` crate is an **orphan**: Grep for
  `platform_windows` across `**/*.rs` matches only
  `crates/platform-windows/tests/seam_behaviour.rs`; Grep for `platform-windows`
  across all `Cargo.toml` finds no dependents. No app (`apps/cli`, `apps/web`,
  `apps/desktop*`) or engine crate imports it. The Windows WASAPI lane — and
  this file — is unreachable from every shipped entry point today, on every OS.
  (Cross-wave note: `cargo xtask parity-report` baseline shows windows=66/252 —
  this orphan status is consistent with that gap. Downgraded to file-level note
  because the wiring absence is a crate/workspace-level defect, not this file's.)
- No dead functions inside the file itself; `_get_device`/`_activate`/`_get_state`
  vtable slots are deliberately declared-but-unused (ABI padding) — correct.

## 8. Dependencies & flags
- `#![allow(unsafe_code)]` module-scoped — matches the crate's SAFETY-SCOPED
  discipline (MP §9 exception), documented at wasapi.rs:36-37.
- No external Windows crate (clean-room hand-rolled ole32 ABI — consistent with
  the no-`windows-s` policy stated in com.rs:5-8).
- Feature flags: none; gating is purely `target_os = "windows"`.

## 9. Open questions for the next agent
- Does any Wave-3+ plan wire `platform-windows` (and platform-linux/macos) into
  an actual entry point, or are all three platform crates dead weight for the
  shipped product? (Orphan status confirmed here for platform-windows.)
- F-002 needs a real Windows host: does `RegisterEndpointNotificationCallback`
  QI the client for IID_IMMNotificationClient? If yes, notifications are dead
  on arrival and the bug is S1-grade in a Windows build.
