# AUDIT: crates/platform-windows/src/gpu.rs
| Field | Value |
|---|---|
| Auditor | W08-A09 |
| Date | 2026-11-23 |
| Lines / functions | 106 / 5 (GpuAdapterInfo::vendor_name, pci_vendor_name, trait GpuAdapters::enumerate, UnavailableGpuAdapters::{because, reason, enumerate}) |
| File verdict | REAL (seam module, host-tested) — but its central doc claim rests on a dxgi.rs FFI whose vtable layout is WRONG (cross-file S1, routed to W08-A10) |
| Reachable from shipped entry point? | NO — platform-windows has zero dependents (no Cargo.toml in the repo, crates/ or apps/, lists it) |
| Findings | S0: 0 S1: 1 (cross-file) S2: 0 S3: 1 S4: 1 |
| Commands run | rg (wiring, dependents, assignments), slot-by-slot vtable mapping vs dxgi.h order, sed quote verification, baseline citation |

## 1. One-paragraph verdict
Answering the hostile question ("dxgi — real FFI or refusals?"): the
declarations are REAL FFI, not refusals — `CreateDXGIFactory1` is a genuine
`extern "system"` import from dxgi.dll with the correct IID — but the COM
vtable structs in `crate::windows::dxgi` (which this file's header claims as
the Windows lane) are misaligned with the actual `dxgi.h` interface order,
so the first `EnumAdapters1` call would actually invoke
`GetWindowAssociation` and write through a null pointer. The seam file
itself is clean: a trait with a proper `Result` error channel, an honest
typed fallback, and a PCI vendor table whose 11 ids I verified correct.
The lane has never executed anywhere (compile-only for
x86_64-pc-windows-msvc; [VERIFY]-labeled), which is the only reason the
vtable bug is latent rather than a shipped crash.

## 2. What actually works (verified)
- The vendor label table (gpu.rs:53-67) is correct against public PCI ids:
  0x8086 Intel, 0x1002/0x1022 AMD (ATI + AMD), 0x10de NVIDIA, 0x13b5 Arm,
  0x5143 Qualcomm, 0x1010 Imagination, 0x15ad/0x1b36 VMware, 0x1af4
  Red Hat virtio, 0x1234 Bochs/QEMU — verified by reading against the
  PCI vendor id registry values.
- Host-tested via the crate's suite: baseline cargo-test-workspace.txt
  (platform-windows, 16 seam tests, 0 failed) includes
  `unavailable_gpu_adapters_returns_the_typed_reason ... ok` and
  `gpu_adapter_info_labels_well_known_pci_vendors ... ok`.
- The fallback refuses with the stored reason (gpu.rs:102-106), the exact
  opposite of a fake-success stub.
- The trait carries an error channel — `fn enumerate(&mut self) ->
  Result<Vec<GpuAdapterInfo>, PlatformError>` (gpu.rs:79) — the design the
  audio-output L4 seam lacks (see my device.rs audit F-001).

## 3. Findings
F-001 [S1] (CROSS-FILE — owner: W08-A10, crates/platform-windows/src/windows/dxgi.rs; flagged here because it falsifies this file's header claim)
Where: crates/platform-windows/src/gpu.rs:3-5 (claim) vs
crates/platform-windows/src/windows/dxgi.rs:48-77, 88-106 and
crates/platform-windows/src/windows/com.rs:70-80
Code (this file's claim):
```
//! The Windows lane enumerates **DXGI adapters**
//! (`IDXGIFactory1::EnumAdapters1` + `IDXGIAdapter1::GetDesc1` in
//! [`crate::windows::dxgi`]) — the PCI ids, description strings and
```
Supporting evidence (dxgi.rs:49-77, abridged):
```
struct Factory1Vtbl {
    pub unknown: crate::windows::com::UnknownVtbl,
    pub _create_swap_chain: ...
    pub _get_adapter: ...
    pub _enum_adapters: ...
    pub _make_window_association: ...
    pub _get_window_association: ...
    pub _create_software_adapter: ...
    pub enum_adapters1: ...
    pub _is_current: ...
}
```
with com.rs:72-80 `UnknownVtbl` = exactly 3 slots (QI/AddRef/Release).
What is wrong: the real IDXGIFactory1 vtable is IUnknown(3) +
IDXGIObject(4: SetPrivateData, SetPrivateDataInterface, GetPrivateData,
GetParent) + IDXGIFactory(5: EnumAdapters, MakeWindowAssociation,
GetWindowAssociation, CreateSwapChain, CreateSoftwareAdapter) +
EnumAdapters1 + IsCurrent = EnumAdapters1 at slot 12. The declared struct
omits the four IDXGIObject slots entirely, inserts a "GetAdapter" method
that does not exist on any IDXGIFactory interface, and misorders the
IDXGIFactory five — placing `enum_adapters1` at slot 9, which is really
**GetWindowAssociation**. The call at dxgi.rs:198
`((*factory_vtbl).enum_adapters1)(factory, index, &mut adapter)` with
index=0 therefore asks GetWindowAssociation to write the HWND through
pointer 0 — an access violation on the first enumeration call on any real
Windows machine. Likewise `Adapter1Vtbl.get_desc1` (dxgi.rs:105, called at
dxgi.rs:238) lands on slot 7 = **EnumOutputs**, not GetDesc1 (real slot
10). `cargo check` cannot catch slot-order errors, so the worklog's
"compile-verified … clean and warning-free" (worklog.md:556) is true and
useless here. Not S0 only because nothing shipped reaches it and every doc
labels runtime `[VERIFY]`. What correct looks like: insert the IDXGIObject
4-slot prefix (or extend UnknownVtbl), restore the documented IDXGIFactory
order, drop the phantom GetAdapter, and pin both vtables with a
compile-time slot-count assertion. Confidence: HIGH — the mismatch is
arithmetic against the public ABI; runtime confirmation UNVERIFIED (no
Windows in sandbox, by design). — Verification: read + slot mapping.

F-002 [S3]
Where: crates/platform-windows/src/gpu.rs:1-16 (whole-crate context)
What is wrong: same as the other two platform crates — zero dependents
anywhere in the workspace (rg over every Cargo.toml incl. apps/), so no
shipped binary contains this seam or its (broken) DXGI lane. Disclosed
scaffolding pending issues #41/#42. Confidence: HIGH — Verification: rg.

F-003 [S4]
Where: crates/platform-windows/src/gpu.rs:6-8 (doc chain) vs
crates/platform-windows/src/windows/dxgi.rs:8-9 and 33-36
Code (dxgi.rs — for the A10 owner):
```
//! WARP (Microsoft's software rasterizer) is flagged by its vendor id
//! 0x8C / device 0x8C `Microsoft Basic Render Driver` naming.
```
vs
```
const WARP_VENDOR_ID: u32 = 0x1414;
```
What is wrong: the WARP vendor id is 0x1414 (Microsoft), device 0x008C —
the constant is right, the dxgi.rs header comment is wrong (vendor "0x8C"
is actually the device id), and this file's header defers WARP flagging to
that self-contradicting comment. gpu.rs itself contains no incorrect id.
Confidence: HIGH — Verification: read both sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| GpuAdapterInfo::vendor_name | 41-48 | REAL | — | delegates; host-tested via seam suite |
| pci_vendor_name | 50-68 | REAL | — | 11/11 vendor ids verified correct |
| GpuAdapters::enumerate (trait) | 70-80 | REAL | — | has the Result error channel; impls: DxgiAdapters (latently broken, F-001) + UnavailableGpuAdapters (tested) |
| UnavailableGpuAdapters::because / reason | 88-100 | REAL | — | typed fallback storage |
| UnavailableGpuAdapters::enumerate | 102-106 | REAL | — | refuses with the reason; baseline-tested |

## 5. Test honesty
No tests in this file. The crate's seam suite exercises the fallback and
the vendor table for real (baseline cited in §2). No test can execute the
DXGI lane on any CI runner in this repo (no windows job runs
platform-windows code — runners build only lumen-cli/desktop, which do not
depend on this crate).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The Windows lane enumerates DXGI adapters (EnumAdapters1 + GetDesc1)" | gpu.rs:3-5 | FALSE-AS-IMPLEMENTED | dxgi vtable slots land on GetWindowAssociation/EnumOutputs (F-001); would crash, not enumerate |
| "this seam answers 'what hardware is present' … not 'create a GPU context'" | gpu.rs:9-11 | REAL | read: no context/device creation anywhere in the crate |
| "the field set is pinned to what the three OS lanes can honestly produce" | gpu.rs:15-16 | REAL | vendor 0 semantics documented per-lane (gpu.rs:24-29) |
| "[VERIFY] … runtime NOT exercised" (implied via dxgi.rs:11-13) | dxgi.rs:11-13 | HONEST LABEL | no execution exists; the label is what keeps F-001 at S1 not S0 |

## 7. Dead / duplicated / unreachable code
Crate unreachable from shipped entry points (F-002). The GpuAdapterInfo /
GpuAdapters vocabulary is triplicated across the three platform crates
(pinned cross-lane by the mirrored seam suites; unification [PROPOSAL]).

## 8. Dependencies & flags
std-only, no deps (crate Cargo.toml:39-41 pattern), no unsafe in THIS file
(all unsafe lives in windows/{com,dxgi,wasapi,dialogs}.rs behind module
`allow(unsafe_code)`).

## 9. Open questions for the next agent
- W08-A10 (owns dxgi.rs + com.rs): F-001 vtable misalignment (both
  Factory1Vtbl and Adapter1Vtbl) and the F-003 header/constant WARP-id
  contradiction. Also check whether wasapi.rs's IMMDeviceEnumerator /
  IMMDevice / IAudioClient vtables include their intermediate interfaces
  correctly — com.rs's own IPropertyStore table is correct (direct
  IUnknown child), so the bug pattern is specifically the IDXGIObject
  insertion + phantom method.
- com.rs:202-204 comment says "vtable slot #3 is Release" — it is slot 3
  only in 1-based counting; the code reads the correct field, wording nit.
