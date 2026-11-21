# AUDIT: crates/platform-windows/src/windows/dxgi.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-21 |
| Lines / functions | 264 / 13 (2 consts, 1 extern fn, 5 repr(C) structs, 2 impls, 3 fns, module docs; 0 tests in file) |
| File verdict | UNREACHABLE (real hand-written FFI, cfg-gated to Windows AND orphaned: no shipped binary depends on platform-windows at all) |
| Reachable from shipped entry point? | NO. Two independent barriers: (1) `pub mod windows` is `#[cfg(target_os = "windows")]` (lib.rs:61-62) — not compiled on the Linux baseline; (2) no Cargo.toml in apps/ or crates/ (outside the crate itself) lists platform-windows as a dependency — the `lumen` CLI, desktop, and tauri binaries cannot link it on ANY target. The internal wiring exists (lib.rs:170-181 `gpu_seam()` constructs `DxgiAdapters` under cfg(windows)) but nothing calls into the crate. |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:1 |
| Commands run | `rg -l "platform-windows" --glob Cargo.toml apps/ crates/` (only its own + a comment in platform-linux) · same for platform-linux/platform-macos (all three orphaned) · `rg -n "cfg(target_os" lib.rs` (:61) · `sed -n '165,185p' lib.rs` (gpu_seam wiring) · vtable/struct layouts checked against the dxgi.h C-ABI from memory of the Windows SDK · baseline log: platform_windows lib unittests = 0 tests (:3712-3714); seam_behaviour 16 passed are the cfg(not(windows)) fallback arms (:3718-3722) — the DXGI path executed nowhere |

## 1. One-paragraph verdict
Genuine hand-declared COM/DXGI FFI, not generated boilerplate: the `IDXGIFactory1` and `IDXGIAdapter1` vtable orders match the dxgi.h ABI, `DXGI_ADAPTER_DESC1`'s field order is correct, references are released exactly once on every path, and the WARP-software detection is a sane vendor-id/name heuristic. But none of it can ever run in a shipped build: the module is cfg-gated to Windows, the crate is a dependency of no application (the whole three-crate platform layer is orphaned), and the file's own header admits runtime was never exercised ("compile-verified ... runtime NOT exercised in this sandbox — [VERIFY]"). Within the dead code there are two real defects: the enumeration loop's comment claims "every other failure is an error we report" while the code silently `break`s on any HRESULT that is not S_OK (and `describe_adapter` errors are dropped the same way), and the never-called `_get_desc`/`_create_swap_chain` parameter blocks are declared with field orders/layouts that do not match the real ABI structs — harmless today precisely because nothing calls them.

## 2. What actually works (verified)
— Vtable order verified against the dxgi.h ABI: `Factory1Vtbl` = IUnknown + CreateSwapChain, GetAdapter, EnumAdapters, MakeWindowAssociation, GetWindowAssociation, CreateSoftwareAdapter, EnumAdapters1, IsCurrent (:48-77) — correct; `Adapter1Vtbl` = IUnknown + GetParent, EnumOutputs, GetDesc, CheckInterfaceSupport, GetDesc1 (:90-106) — correct.
— `AdapterDesc1` (:123-145) field order matches DXGI_ADAPTER_DESC1 (Description[128], Flags, DedicatedVideoMemory, DedicatedSystemMemory, SharedSystemMemory, LUID, Revision, DeviceId, VendorId, SubSysId) — correct.
— Reference discipline: factory released once on both paths (:182-184), each adapter released after describe (:212-214), zero-initialized out-slots, ascending indices per the API contract — the SAFETY comments match the code.
— IID_IDXGIFactory1 {770AAE78-F26F-4DBA-A829-253C83D1B387} (:26-31) — the correct GUID for IDXGIFactory1.
— NOT EXECUTED anywhere: baseline has no Windows-target build (`audit/runs/baseline/INDEX.md` — no cross-check entry); the crate's 16 passing seam tests exercise the cfg(not(windows)) fallbacks. The header's "compile-verified via cargo check --target x86_64-pc-windows-msvc" claim is UNVERIFIED — no such run exists in the committed baseline evidence.

## 3. Findings
### F-001 [S3]
Where: crates/platform-windows/src/windows/dxgi.rs:153-187 (the scanner), crates/platform-windows/src/lib.rs:61-62, repo-wide Cargo.toml graph
Code (dxgi.rs:166-172):
```
impl GpuAdapters for DxgiAdapters {
    fn enumerate(&mut self) -> Result<Vec<GpuAdapterInfo>, PlatformError> {
        // SAFETY: the riid points at the complete IDXGIFactory1 Guid
        // and the out-slot is zero-initialized; the returned factory
        // is released exactly once at the end of this function.
        let mut factory: *mut c_void = std::ptr::null_mut();
```
What is wrong: the entire DXGI integration is unreachable from every shipped entry point. `pub mod windows` requires `target_os = "windows"` (lib.rs:61-62), and no application or engine crate depends on platform-windows: `rg -l "platform-windows" --glob Cargo.toml apps/ crates/` returns only the crate's own manifest (plus a comment in platform-linux's). The same check shows platform-linux and platform-macos are equally orphaned — the whole L5 platform layer is built by `cargo build --workspace` and tested (fallback arms only) but linked into nothing.
Why it matters: the docs position these crates as the product's platform layer (doc 01 §2.7), the capability reporting (my next files) claims to probe DXGI/WASAPI/dialogs, and the feature matrix's Windows column presumably cites this code as implementation evidence — but no `lumen` binary on any OS can reach it. Capability claims riding on this crate are claims about code no shipped build contains.
What correct looks like: a target-conditional dependency from the CLI/desktop app on the matching platform crate (e.g. `[target.'cfg(windows)'.dependencies] platform-windows`), with the seam wired into device discovery — or the crates demoted to a clearly-labeled "library, unwired" status and the matrix rows marked accordingly.
Confidence: HIGH — Verification: exhaustive Cargo.toml grep; lib.rs cfg read; baseline log read.
### F-002 [S3]
Where: crates/platform-windows/src/windows/dxgi.rs:199-210
Code:
```
        let hr = unsafe { ((*factory_vtbl).enum_adapters1)(factory, index, &mut adapter) };
        if hr != S_OK {
            // DXGI_ERROR_NOT_FOUND is the documented end of the
            // enumeration; every other failure is an error we report
            // (the list so far is kept — honest partial data).
            break;
        }
        if adapter.is_null() {
            break;
        }
        if let Ok(Some(info)) = describe_adapter(adapter) {
            adapters.push(info);
        }
```
What is wrong: the comment says "every other failure is an error we report" — the code reports nothing: any HRESULT other than S_OK (including genuine failures like DXGI_ERROR_INVALID_CALL or E_OUTOFMEMORY, distinct from the documented DXGI_ERROR_NOT_FOUND end) breaks the loop and returns `Ok(adapters)` as if enumeration completed. `describe_adapter` failures are swallowed identically — `if let Ok(Some(info))` discards the `Err` (a GetDesc1 CallFailed) without recording it, so a failing adapter silently vanishes from the list.
Why it matters: a partial adapter list is reported as complete success — the exact "swallowed errors" class the audit checks, in the one place (enumeration) where the module's own comments promise honesty. On Windows this would mis-drive the 7.C1 software/hardware fallback decision.
What correct looks like: match `hr` against `DXGI_ERROR_NOT_FOUND` (0x887A0002) for the clean end; everything else becomes a typed `PlatformError::CallFailed` carrying the HRESULT; `describe_adapter` errors at minimum counted or surfaced as a partial-failure detail.
Confidence: HIGH — Verification: read; HRESULT constant values from the DXGI ABI.
### F-003 [S4]
Where: crates/platform-windows/src/windows/dxgi.rs:109-120 and :81-86
Code (dxgi.rs:109-115):
```
/// `DXGI_ADAPTER_DESC` (the `_get_desc` slot's parameter block).
#[repr(C)]
struct AdapterDesc {
    _description: [u16; 128],
    _dedicated_video_memory: usize,
    _dedicated_system_memory: usize,
    _shared_system_memory: usize,
```
What is wrong: the real `DXGI_ADAPTER_DESC` ABI order is Description[128], **VendorId, DeviceId, SubSysId, Revision**, DedicatedVideoMemory, DedicatedSystemMemory, SharedSystemMemory, AdapterLuid — this declaration places the memory fields before the id fields, so the struct does not match the ABI it names. Likewise `SwapChainDesc` (:81-86) declares `_buffer_desc: [u64; 5]` (40 bytes) for what is really the 28-byte `DXGI_MODE_DESC`, and omits `Windowed`. Both are parameter types of vtable slots that are never called (`_get_desc`, `_create_swap_chain`), so no runtime effect today — but they are traps: the day anyone invokes those slots with these blocks, every field after `description` is garbage.
Why it matters: dead-but-wrong ABI declarations in an FFI file are exactly where future bugs hatch; the file's discipline elsewhere (AdapterDesc1 correct) makes the inconsistency worse.
What correct looks like: fix the field order to the dxgi.h layout (or delete the unused blocks and type the slots as `*mut c_void`).
Confidence: HIGH — Verification: read; dxgi.h layouts from the Windows SDK ABI.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-13 | REAL-UNPROVEN | — | honest evidence-status banner; the claimed cargo-check run is UNVERIFIED (not in baseline) |
| `#![allow(unsafe_code)]` | 17 | REAL | — | module-scoped, matches the crate's FFI-scoping policy |
| `IID_IDXGI_FACTORY1` | 26-31 | REAL | — | correct GUID |
| `WARP_VENDOR_ID = 0x1414` | 36 | REAL | — | vendor-only check; the doc's device pair (0x008C) is not checked — more lenient than documented (table note, not a finding) |
| `CreateDXGIFactory1` extern | 41-44 | REAL-UNPROVEN | — | ABI declaration correct; never linked on this OS |
| `Factory1Vtbl` | 48-77 | REAL | — | vtable order verified correct (§2) |
| `SwapChainDesc` | 81-86 | REAL | S4 | F-003: wrong layout, never used |
| `Adapter1Vtbl` | 90-106 | REAL | — | order verified correct |
| `AdapterDesc` | 109-120 | REAL | S4 | F-003: wrong field order, never used |
| `AdapterDesc1` | 123-145 | REAL | — | correct ABI layout; the one actually consumed |
| `ObjectHeader` | 148-151 | REAL | — | standard COM header trick |
| `DxgiAdapters` + `new` | 154-164 | REAL | — | Default-derived, stateless |
| `GpuAdapters::enumerate` | 166-187 | UNREACHABLE | S3 | F-001; error path typed; factory released on both paths |
| `enum_adapters` | 190-218 | UNREACHABLE | S3 | F-002: swallowed non-NOT_FOUND failures and describe errors |
| `describe_adapter` | 222-264 | UNREACHABLE | — | typed GetDesc1 failure (returned, then swallowed by caller); `from_wide` on the NUL-terminated fixed array; `usize as u64` lossless |

## 5. Test honesty
No tests in this file; no test in the repo executes DXGI code (the crate's 16 passing seam tests cover the non-Windows fallback arms — baseline :3718-3722). The module header honestly labels runtime `[VERIFY]`. Nothing to grade — and that is itself the gap: 264 lines of unsafe FFI with zero executable coverage and an unverifiable compile-check claim.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compile-verified via cargo check --target x86_64-pc-windows-msvc" | dxgi.rs:11-12 | UNVERIFIED | no such run in audit/runs/baseline/ (INDEX.md has no cross-target entry) |
| "runtime NOT exercised in this sandbox ... [VERIFY] on real-device behavior" | dxgi.rs:12-13 | REAL (honest) | consistent with the evidence vacuum |
| "the PCI vendor/device ids, the adapter description string and the dedicated-video-memory size, exactly the fields GpuAdapterInfo carries" | dxgi.rs:4-7 | REAL (code read) | describe_adapter :257-263 maps exactly those |
| "WARP ... flagged by its vendor id 0x8C / device 0x8C" | dxgi.rs:8-9 | REAL (approximation) | code checks vendor 0x1414 only + name-contains (:256); doc's "0x8C/0x8C" phrasing garbles the real 0x1414/0x008C pair the comment at :33-35 states correctly |
| gpu seam probes DXGI and feeds the 7.C1 fallback chain | lib.rs:27-31 | UNREACHABLE | F-001: no binary links the crate |

## 7. Dead / duplicated / unreachable code
- The entire file (with the crate): unreachable from every shipped entry point — F-001.
- `AdapterDesc` + `SwapChainDesc`: dead within the dead (never-called vtable slots) — F-003.

## 8. Dependencies & flags
- Links `dxgi` via `#[link(name = "dxgi")]` (Windows-only linker directive — inert on other targets where the module is cfg'd out). No external crates — hand-declared ABI per the crate's no-windows-s policy. Crate built on Linux only in its cfg(not(windows)) form (baseline :3712).

## 9. Open questions for the next agent
- capability.rs / events.rs / dialog.rs (my remaining platform-windows files): same F-001 orphan status applies — do not re-derive it, cite this finding; check whether the capability report text claims to reflect DXGI probing in shipped builds.
- Whoever owns docs/feature-matrix.toml's Windows column: which rows cite platform-windows as evidence? Those rows are UNREACHABLE-as-shipped and need re-verdicting (parity-report already shows windows=66/252 — on what basis?).
- If the Lead ever wires the platform crates: F-002's silent truncation becomes a real Windows behavior bug — fix before wiring.
