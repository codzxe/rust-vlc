# AUDIT: crates/platform-windows/src/windows/wasapi.rs
| Field | Value |
|---|---|
| Auditor | W08-A01 |
| Date | 2026-11-23 |
| Lines / functions | 787 / 24 |
| File verdict | REAL-UNPROVEN (code) + UNREACHABLE (shipped builds) |
| Reachable from shipped entry point? | NO — `platform-windows` is an orphan crate; no Cargo.toml in the repo depends on it |
| Findings | S0: 1 S1: 0 S2: 1 S3: 3 S4: 3 |
| Commands run | rg over all Cargo.tomls (exit 0); rg `WasapiAudioDevices` (exit 0); rg `WindowsPlatform` (exit 0); sed quote verification (exit 0); `cargo check -p platform-windows --target x86_64-pc-windows-msvc` -> FAILED (target rust-std not installed) |

## 1. One-paragraph verdict
This is real hand-rolled COM FFI, not a typed refusal: it declares the true `CLSID_MMDeviceEnumerator`/`IID_IMMDeviceEnumerator` GUIDs and walks `EnumAudioEndpoints`/`Item`/`GetId`/`OpenPropertyStore` vtables over an `ole32` link (`com.rs:88-101`). But it is **enumeration + change notifications only** — there is no `IAudioClient`, no `IAudioRenderClient`, no buffer delivery; a user cannot HEAR anything through this file (the module itself admits it at wasapi.rs:196-199). Worse: none of it can ever run, because no crate in the workspace depends on `platform-windows` — the shipped CLI plays into `NullBackend` (`apps/cli/src/pipeline_host.rs:1599`). Runtime is untested everywhere (Linux test lane cfg-compiles this module out; the Windows target's rust-std is absent here, so even the doc's "compile-verified" claim is unreproducible). Verdict: real, well-disciplined code that ships nowhere and outputs no sound.

## 2. What actually works (verified)
- Verified by `rg -n "platform-" --glob **/Cargo.toml` -> only the three `platform-*` crates' own manifests mention them; no app/core crate links them (reachability: NO).
- Verified by `sed -n '63,76p'` -> GUIDs `BCDE0395-E52F-467C-8E3D-C4579291692E` (CLSID_MMDeviceEnumerator) and `A95664D2-9614-4F35-A746-DE8DB63617E6` match the public Windows SDK constants — the FFI targets the real MMDevice API.
- Baseline `audit/runs/baseline/cargo-test-workspace.txt:4044-4083`: platform-windows package tests = 16 passed, all exercising the non-Windows typed fallbacks; **zero tests execute wasapi.rs**.

## 3. Findings
F-001 [S0]
Where: crates/platform-windows/src/windows/wasapi.rs:599-676 (whole `impl AudioDevices for WasapiAudioDevices`)
Code: (wasapi.rs:146, the only constructor call site in the repo)
```
        let mut devices = windows::wasapi::WasapiAudioDevices::new();
```
What is wrong: The only caller of `WasapiAudioDevices::new` is `platform-windows/src/lib.rs:146` (`audio_seam`), whose only caller is `WindowsPlatform::discover`, whose only callers are `crates/platform-windows/tests/seam_behaviour.rs:15,393` (verified by `rg -n "WindowsPlatform"` — 2 files, both inside the crate). No member of the workspace (apps/cli, apps/web, apps/desktop, any crates/*) depends on `platform-windows` (verified by `rg -n "platform-" --glob **/Cargo.toml`). This real WASAPI code is unreachable from every shipped entry point; on a Windows host running `lumen-cli`, device enumeration falls back to nothing — the CLI never calls into it.
Why it matters: The repo's only real-device audio integration is dead code; the product's audio story is null-device-only end to end.
What correct looks like: apps/cli/desktop declare `[target.'cfg(windows)'.dependencies] platform-windows` and the settings/discovery path calls `WindowsPlatform::discover`.
Confidence: HIGH — Verification: read (rg call-chain + manifests).

F-002 [S2]
Where: crates/platform-windows/src/windows/wasapi.rs:195-206
Code:
```
/// This is the honest per-endpoint *suggestion*; the exact mix format
/// is negotiated at open time behind the ADR-005 admission gate
/// (doc 01 §2.6 — the streaming half this card does not claim).
fn preferred_format() -> AudioFormat {
```
What is wrong: The file contains no streaming half and the crate contains no `IAudioClient`/`IAudioRenderClient` anywhere (`rg -n "IAudioClient|AudioRenderClient"` over the repo: no hits). The shipped CLI opens `NullBackend` (apps/cli/src/pipeline_host.rs:1599). "How does a user hear sound?" — they don't: WASAPI here can only list devices and watch them come and go; sample delivery exists nowhere on Windows.
Why it matters: Any doc/matrix row claiming Windows audio output is PAPER-ONLY; enumeration+notification is the ceiling of this integration.
What correct looks like: A `platform-windows` `AudioBackend` impl activating `IAudioClient` in shared mode, pulling from `audio_output::SpscRing` via `render_into`.
Confidence: HIGH — Verification: read.

F-003 [S3]
Where: crates/platform-windows/src/windows/wasapi.rs:280-282
Code:
```
        if let Ok(Some(desc)) = describe_device(device) {
            devices.push(desc);
        }
```
What is wrong: `describe_device` returns `Err` on a failed `IMMDevice::GetId` (wasapi.rs:299-305), but that error is silently dropped here — not logged, not counted, not surfaced. A partial enumeration (e.g., a COM security failure) is returned as `Ok(devices)` indistinguishable from success. The adjacent comment claims skipping is for devices "disappearing mid-enumeration", but the `Err` arm covers hard failures too.
Why it matters: Swallowed errors (checklist 6.2): the caller cannot tell a 2-device truth from a 5-device truth truncated by an error.
What correct looks like: Distinguish `Ok(None)` (skip) from `Err` (at minimum count it, or fail the round).
Confidence: HIGH — Verification: read.

F-004 [S3]
Where: crates/platform-windows/src/windows/wasapi.rs:199-206
Code:
```
    AudioFormat {
        sample: SampleFormat::F32,
        channels: 2,
        layout: AudioLayout::Interleaved,
        rate: 48_000,
    }
```
What is wrong: Every endpoint on every machine is described as F32/2ch/48kHz. Real Windows mix formats are commonly 44.1kHz (and the device's actual mix format is queryable via `IAudioClient::GetMixFormat`). The comment calls it a "suggestion", but `DeviceDesc.preferred` is a typed value consumers may trust; on a 44.1kHz endpoint it is simply wrong data.
Why it matters: Hard-coded output masquerading as per-device info (checklist 6.1).
What correct looks like: Read the endpoint's real format from its property store or `GetMixFormat`, or type `preferred` as Option.
Confidence: HIGH — Verification: read.

F-005 [S3]
Where: crates/platform-windows/src/windows/wasapi.rs:672-676
Code:
```
    fn poll(&mut self) -> Result<u32, PlatformError> {
        // Push-based: WASAPI delivers via the callbacks; the events
        // already sit in the queue (see the seam contract).
        Ok(0)
    }
```
What is wrong: `poll` is a constant. If the watch thread died (panicked in a COM callback, killed by COM teardown failure), events silently stop arriving and `poll` still reports Ok(0) — "healthy, just quiet". The seam (audio.rs:48-56) documents 0 as legitimate for push-based, but there is no liveness signal at all, so a dead subscription is undetectable by the API's own contract.
Why it matters: A stuck/dead notification lane looks identical to a quiet device set; the UI never re-enumerates.
What correct looks like: Track watch-thread liveness (e.g., an AtomicBool the thread clears on exit) and report it via poll/subscribe error.
Confidence: MEDIUM — Verification: read.

F-006 [S4]
Where: crates/platform-windows/src/windows/wasapi.rs:78-80, 442-448
Code:
```
        if (*riid) == IID_IUNKNOWN {
            client_add_ref(this);
            *ppv = this;
            S_OK
```
What is wrong: `QueryInterface` answers only `IID_IUnknown`. The object is registered as an `IMMNotificationClient`; a COM caller that QIs for `IID_IMMNotificationClient` receives E_NOINTERFACE — a COM etiquette violation that only works because the MMDevice enumerator happens not to QI. The IID for IMMNotificationClient (7991EEC9-7BA9-4E44-B8B6-1F612C62EBAD) is not even declared.
Why it matters: Relies on undocumented enumerator behavior; any Windows update QI-ing the real interface breaks registration silently.
Confidence: MEDIUM — Verification: read; runtime behavior UNVERIFIED (no Windows host).

F-007 [S4]
Where: crates/platform-windows/src/windows/wasapi.rs:533-537
Code:
```
        if flow == E_RENDER && role == E_CONSOLE {
```
What is wrong: Default-device tracking watches only the eConsole role. Media playback on Windows is assigned the eMultimedia role; a default-device change for multimedia would be invisible to this integration. The doc comment says "console role is what we track" — true but a product-behavior gap.
Confidence: MEDIUM — Verification: read.

F-008 [S4]
Where: crates/platform-windows/src/windows/wasapi.rs:571-582
Code:
```
        if let Some(handle) = self.handle.take() {
            // Join so the COM teardown is complete before the owner
            // proceeds (the queue may outlive us — it is Arc'd).
            let _ = handle.join();
        }
```
What is wrong: The 2s deadline (WATCH_DEADLINE) bounds only the registration handshake; `Watch::drop` joins the watch thread with **no timeout**. A `UnregisterEndpointNotificationCallback` that blocks (documented to wait on the callback dispatch in pathological cases) blocks the dropping thread indefinitely — the D-2 "no unbounded waits" discipline is violated by the teardown path.
Confidence: LOW — Verification: read; hang behavior UNVERIFIED.

## 4. Per-function table (mandatory — file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| preferred_format | 199-206 | STUB (constant) | S3 | Same F32/2/48k for every device |
| list_endpoints | 210-224 | REAL-UNPROVEN | — | Real COM walk; never executed anywhere |
| enumerate_with | 227-253 | REAL-UNPROVEN | — | EnumAudioEndpoints + release on all paths |
| devices_from_collection | 256-289 | REAL-UNPROVEN | S3 | Swallows describe_device Err (F-003) |
| describe_device | 293-352 | REAL-UNPROVEN | — | Two-level naming fallback; GetId Err propagated (but swallowed upstream) |
| client_of | 407-409 | REAL-UNPROVEN | — | Unchecked raw deref, invariant documented |
| id_of | 417-427 | REAL-UNPROVEN | — | NULL/empty -> None, strict UTF-16 |
| client_query_interface | 433-450 | REAL-UNPROVEN | S4 | IUnknown only (F-006) |
| client_add_ref | 453-456 | REAL-UNPROVEN | — | Atomic |
| client_release | 460-468 | REAL-UNPROVEN | — | Never frees (Box owns) — matches doc |
| client_on_device_state_changed | 471-489 | REAL-UNPROVEN | — | Pushes typed event; no panic |
| client_on_device_added | 492-505 | REAL-UNPROVEN | — | Same |
| client_on_device_removed | 508-521 | REAL-UNPROVEN | — | Same |
| client_on_default_device_changed | 524-540 | REAL-UNPROVEN | S4 | eConsole only (F-007); NULL id passed as None |
| client_on_property_value_changed | 544-554 | REAL-UNPROVEN | — | Coarse AudioSetChanged; args ignored by design |
| WasapiAudioDevices::new | 588-590 | REAL-UNPROVEN | — | No-op constructor |
| Default::default | 593-597 | REAL-UNPROVEN | — | Delegates |
| AudioDevices::enumerate | 600-622 | REAL-UNPROVEN | — | Thread-per-round, panic converted to typed Err |
| AudioDevices::subscribe | 624-670 | REAL-UNPROVEN | — | 2s handshake deadline; replaces prior watch |
| AudioDevices::poll | 672-676 | STUB (constant) | S3 | Ok(0); no liveness signal (F-005) |
| Watch::drop | 571-582 | REAL-UNPROVEN | S4 | Unbounded join (F-008) |
| watch_thread_body | 681-700 | REAL-UNPROVEN | — | register -> park -> teardown |
| register_notification_client | 719-760 | REAL-UNPROVEN | — | Apartment held for thread life; error path releases |
| teardown_notification_client | 765-787 | REAL-UNPROVEN | — | Unregister -> release -> Box drop order documented |

## 5. Test honesty
No tests exist for this file. Baseline platform-windows lane (cargo-test-workspace.txt:4044-4083) runs 16 tests, all against the non-Windows typed fallbacks; `stable_device_id_is_deterministic_and_pinned` tests audio.rs's hash, not wasapi.rs. On Linux the module is compiled out (`#[cfg(target_os = "windows")]`, lib.rs:61-62), so no CI lane anywhere has ever executed a line of wasapi.rs. Mutation reasoning: deleting the entire watch-thread implementation would change no test outcome in the repo.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compile-verified via `cargo check --target x86_64-pc-windows-msvc`" | wasapi.rs:20-22 | UNVERIFIED | My re-run failed: target rust-std not installed in this sandbox (`cargo check -p platform-windows --target x86_64-pc-windows-msvc` -> "the target may not be installed", E0463 in media-types). Cannot reproduce; not contradicted either. |
| "On any non-Windows host this entire module is compiled out" | wasapi.rs:17-18 | REAL | lib.rs:61-62 `#[cfg(target_os = "windows")] pub mod windows;` |
| "Release never frees — the Box on the owning thread governs the lifetime" | wasapi.rs:28-31 | REAL-UNPROVEN | client_release (460-468) decrements only; Box dropped in teardown_notification_client:785-786 after unregister |
| "the streaming half this card does not claim" | wasapi.rs:196-199 | REAL (honest admission) | No IAudioClient/AudioRenderClient anywhere in repo (rg: 0 hits) |

## 7. Dead / duplicated / unreachable code
The entire file is unreachable from shipped builds (F-001). Within the file: nothing dead; all 24 functions are on the enumerate/subscribe/poll paths.

## 8. Dependencies & flags
- `crate::windows::com` (real `#[link(name = "ole32")]` externs, com.rs:88-101) — the FFI foundation is genuine.
- `audio_output::{DeviceDesc, DeviceId}` via the `AudioDevices` seam (platform-windows/src/audio.rs:29-57).
- No feature gates; gating is `cfg(target_os = "windows")` only. The release lane (`cargo build --release -p lumen-cli -p desktop`) never builds this crate on any OS.

## 9. Open questions for the next agent
- Who was supposed to wire `platform-windows` into apps? Check docs/registry claims (doc 01 §2.7) for whether any matrix row cites Windows audio enumeration as working — if so, that row is FALSE-by-unreachability.
- Does apps/desktop-tauri (excluded from workspace) depend on platform-windows? (UNVERIFIED — outside my file list; quick check recommended.)
