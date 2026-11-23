# AUDIT: crates/platform-windows/src/windows/dxgi.rs
| Field | Value |
|---|---|
| Auditor | W08-A10 |
| Date | 2026-11-24 |
| Lines / functions | 264 / 4 (new, enumerate, enum_adapters, describe_adapter) + extern CreateDXGIFactory1 + 5 type defs |
| File verdict | FALSE |
| Reachable from shipped entry point? | NO — platform-windows has no dependents (rg over all Cargo.tomls) |
| Findings | S0: 1 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | sed -n quote verification (exit 0); static vtable-slot arithmetic vs dxgi.h interface definitions; rg dependents (exit 0) |

## 1. One-paragraph verdict
Both COM vtables in this file omit the four `IDXGIObject` slots (SetPrivateData,
SetPrivateDataInterface, GetPrivateData, GetParent) that every dxgi interface inherits,
shifting every declared slot by four positions. Consequence: `enum_adapters1` actually
invokes `IDXGIFactory::EnumAdapters` (same signature by luck), and `get_desc1` actually
invokes `IDXGIAdapter::EnumOutputs` — a different function with a different arity. On a
real Windows machine the GPU seam therefore returns `Ok(vec![])` on every call (each
describe fails and the error is swallowed), and in a ~2^-29-per-call fluke passes a
garbage out-pointer to EnumOutputs. The module's documented purpose — PCI ids, names,
VRAM for the 7.C1 fallback chain — is unachievable by this code. The docs claim
"dxgi.h order" and crate-level "vtable discipline"; both claims are false here.

## 2. What actually works (verified)
— Static check: `IID_IDXGIFactory1` {770AAE78-F26F-4DBA-A829-253C83D1B387} matches the
real IID; `CreateDXGIFactory1(riid, ppfactory)` prototype is correct; `WARP_VENDOR_ID
= 0x1414` is Microsoft's real PCI vendor id; `DXGI_ADAPTER_DESC1` field order/sizes
(description[128], flags, 3×SIZE_T, LUID, revision, device, vendor, subsys) are correct.
Nothing executed — no Windows target exists in this sandbox (no rustc at all).

## 3. Findings
F-001 [S0]
Where: crates/platform-windows/src/windows/dxgi.rs:48-77 (Factory1Vtbl) and 90-106 (Adapter1Vtbl)
Code:
```
/// `IDXGIFactory1` vtable (dxgi.h order through `EnumAdapters1`;
/// `IsCurrent` trails unused).
#[repr(C)]
struct Factory1Vtbl {
    /// IUnknown.
    pub unknown: crate::windows::com::UnknownVtbl,
    /// `CreateSwapChain(this, …)`.
```
```
#[repr(C)]
struct Adapter1Vtbl {
    /// IUnknown.
    pub unknown: crate::windows::com::UnknownVtbl,
    /// `GetParent(this, …)`.
    pub _get_parent:
```
What is wrong: Real dxgi.h inheritance: IDXGIFactory1 = IUnknown(3) + IDXGIObject(4:
SetPrivateData, SetPrivateDataInterface, GetPrivateData, GetParent) + IDXGIFactory(6)
+ IDXGIFactory1(2) = 15 slots; IDXGIAdapter1 = 3+4+3+1 = 11 slots. Both Rust vtables
declare IUnknown immediately followed by the adapter/factory methods — 11 and 8 slots —
omitting IDXGIObject entirely. Slot arithmetic (each field = 8 bytes; `unknown` = 24):
Rust `enum_adapters1` sits at vtable slot 10 = real `EnumAdapters`; Rust `get_desc1`
sits at slot 8 = real `EnumOutputs`. So dxgi.rs:198 calls EnumAdapters (harmless —
identical signature, but returns IDXGIAdapter, not IDXGIAdapter1, so GetDesc1 does not
even exist on the object), and dxgi.rs:238 calls `EnumOutputs(this, low32(&desc),
<uninitialized 3rd register>)`: with a garbage-huge output index DXGI returns
DXGI_ERROR_NOT_FOUND → describe_adapter errs → line 208 drops it → empty list, always.
In the astronomically rare case the index is valid, EnumOutputs writes through the
garbage register → access violation. Why it matters: the Windows GPU seam cannot ever
return adapter data; a caller sees `Ok(vec![])` and honestly reports "no adapters" —
a silent total failure that no compile check can catch (the declarations are
self-consistent). What correct looks like: insert the four IDXGIObject slots
(`_set_private_data`, `_set_private_data_interface`, `_get_private_data`, `_get_parent`)
into BOTH vtables before the first factory/adapter method, and enumerate via
EnumAdapters1 returning IDXGIAdapter1 (whose real slot 11 is GetDesc1).
Confidence: HIGH — the IDXGIObject interface and slot order are ABI-stable documented
history; the arithmetic is mechanical. Verification: read (static); runtime
UNVERIFIABLE (no Windows host, no rustc).

F-002 [S3]
Where: crates/platform-windows/src/windows/dxgi.rs:199-210
Code:
```
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
```
What is wrong: The comment is false twice. (a) A non-NOT_FOUND failure (device removed,
out of memory) takes the same `break` and is returned as `Ok(partial list)` — never
reported. (b) `describe_adapter`'s `Err` (GetDesc1 failure, invalid UTF-16 name) is
discarded by `if let Ok(Some(info))` — never reported. Failures silently masquerade as
"fewer adapters". Why it matters: combined with F-001 this is what converts a hard ABI
bug into an invisible empty success; even with F-001 fixed, GetDesc1 errors shrink the
list silently. What correct looks like: match DXGI_ERROR_NOT_FOUND (-2005270525)
explicitly and propagate everything else as Err.
Confidence: HIGH. Verification: read.

F-003 [S3]
Where: crates/platform-windows/src/windows/dxgi.rs:127-128,253-256
Code:
```
    /// `DXGI_ADAPTER_FLAG_*`.
    flags: u32,
```
```
    let is_software = desc.vendor_id == WARP_VENDOR_ID || name.contains("Basic Render Driver");
```
What is wrong: The struct declares `flags` (DXGI_ADAPTER_FLAG_SOFTWARE = 0x2 is the
API's own software-adapter marker) but the WARP detection never reads it, relying on
vendor-id equality plus an English display-name substring instead. Vendor 0x1414 is
also used by non-WARP Microsoft virtual adapters; localizing Windows can change the
description string. `flags` is dead weight that misleads readers into thinking it is used.
What correct looks like: `let is_software = desc.flags & 0x2 != 0 || …` plus the
existing heuristics as fallback.
Confidence: HIGH. Verification: read.

F-004 [S4]
Where: crates/platform-windows/src/windows/dxgi.rs:8-9
Code:
```
//! WARP (Microsoft's software rasterizer) is flagged by its vendor id
//! 0x8C / device 0x8C `Microsoft Basic Render Driver` naming.
```
What is wrong: WARP's vendor id is 0x1414 (as the code's own `WARP_VENDOR_ID` and the
comment at :253-254 correctly state); 0x008C is the device id. "vendor id 0x8C" is wrong.
Confidence: HIGH. Verification: read.

F-005 [S4]
Where: crates/platform-windows/src/windows/dxgi.rs:11-13
Code:
```
//! **Evidence status:** compile-verified via `cargo check --target
//! x86_64-pc-windows-msvc`; runtime NOT exercised in this sandbox
//! (no Windows SDK) — `[VERIFY]` on real-device behavior.
```
What is wrong: The [VERIFY] disclosure is honest, but "compile-verified" is worthless
for exactly this class of bug: F-001 type-checks perfectly because the wrong vtable is
internally consistent. The evidence framing implies compile-checking adds safety for
this file; it demonstrably cannot. (Claim also UNVERIFIABLE here — no rustc in sandbox.)
Confidence: HIGH. Verification: read.

F-006 [S3]
Where: crates/platform-windows/src/windows/dxgi.rs:246-249
Code:
```
    // SAFETY: the description field is a NUL-terminated wide string
    // per the API contract; from_wide stops at the terminator inside
    // the fixed array.
    let name = unsafe { from_wide(desc.description.as_ptr()) }?;
```
What is wrong: DXGI does not document NUL-termination of `Description[128]`; from_wide's
scan is unbounded, so a malformed driver returning 128 non-NUL wide chars walks off the
`desc` stack object into arbitrary stack memory (unbounded read → possible crash).
The SAFETY comment asserts the exact invariant that is not guaranteed. What correct
looks like: a bounded scan `desc.description.iter().position(|&c| c == 0)` then
`String::from_utf16(&desc.description[..n])`.
Confidence: MEDIUM (real-world drivers terminate; the defect is relying on an
undocumented contract in an `unsafe` block). Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CreateDXGIFactory1 extern decl | 41-44 | REAL-UNPROVEN | — | Prototype correct |
| Factory1Vtbl | 48-77 | FALSE | S0 | Missing 4 IDXGIObject slots; enum_adapters1 = real EnumAdapters (F-001) |
| Adapter1Vtbl | 90-106 | FALSE | S0 | Missing 4 IDXGIObject slots; get_desc1 = real EnumOutputs (F-001) |
| SwapChainDesc/AdapterDesc | 81-86,109-120 | REAL-UNPROVEN | — | Param blocks of never-called slots; sizes approximate but ABI-irrelevant (pointer-only) |
| AdapterDesc1 | 123-145 | REAL-UNPROVEN | — | Field order/types verified correct vs dxgi.h (SIZE_T→usize, LUID→[u32;2]) |
| DxgiAdapters::new | 160-163 | REAL | — | Trivial |
| GpuAdapters::enumerate | 167-186 | FALSE | S0 | Factory released once correctly; delegates into the broken vtables → always-empty result |
| enum_adapters | 190-218 | SHALLOW | S3 | All failures = end-of-list (F-002); adapter refs released correctly per iteration |
| describe_adapter | 222-264 | FALSE | S0/S3 | Calls EnumOutputs-as-GetDesc1 (F-001); swallows nothing itself but its Err is dropped by caller; flags unused (F-003); unbounded from_wide (F-006) |

## 5. Test honesty
No tests in this file; the crate's 16 Linux tests never compile it (cfg windows). No
test anywhere would catch F-001 — a live Windows machine is the only detector, which is
exactly the [VERIFY] gap the crate acknowledges while still shipping the broken slots.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "IDXGIFactory1 vtable (dxgi.h order through EnumAdapters1)" | dxgi.rs:46 | FALSE | Slot arithmetic: IDXGIObject's 4 slots omitted (F-001) |
| "IDXGIAdapter1 vtable (dxgi.h order through GetDesc1)" | dxgi.rs:88 | FALSE | Same |
| "every other failure is an error we report" | dxgi.rs:201-202 | FALSE | break-on-any-error + `if let Ok` swallow (F-002) |
| "compile-verified via cargo check --target x86_64-pc-windows-msvc" | dxgi.rs:11-12 | UNVERIFIED | No rustc in sandbox; and compile-checking cannot catch vtable slot errors |
| "GPU: DXGI IDXGIFactory1::EnumAdapters1 — the PCI ids, names and VRAM sizes the 7.C1 fallback chain consumes" | platform-windows lib.rs:27-29 | FALSE | F-001: the code path can never return that data |
| "Every vtable struct in this crate declares all slots up to the last one used, in interface order" | com.rs:16-19 | FALSE | dxgi.rs omits 4 inherited slots (F-001) |

## 7. Dead / duplicated / unreachable code
`flags` field of AdapterDesc1 is written but never read (F-003). The file (and crate)
is unreachable from lumen-cli/desktop: no dependents. SwapChainDesc/AdapterDesc exist
only to type never-called vtable slots.

## 8. Dependencies & flags
None beyond com.rs helpers + crate::gpu vocabulary. `#![allow(unsafe_code)]` scoped.
`#[link(name = "dxgi")]` correct for dxgi.dll.

## 9. Open questions for the next agent
- dialogs.rs and wasapi.rs declare their own vtables — re-run the same inherited-slot
arithmetic (IUnknown prefix presence, IDXGIObject/other bases) on every one of them;
com.rs's IPropertyStore was correct, dxgi.rs was not, so each file must be checked
individually.
- If any Lead card plans to wire platform-windows into the product, F-001 must be
fixed first or the GPU capability report will claim "no adapters" on every Windows host.
