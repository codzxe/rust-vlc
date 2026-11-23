# AUDIT: crates/platform-windows/src/windows/com.rs
| Field | Value |
|---|---|
| Auditor | W08-A10 |
| Date | 2026-11-24 |
| Lines / functions | 415 / 10 (plus 5 extern FFI decls, 8 repr(C) types) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — no crate in the workspace depends on `platform-windows` (verified: only its own Cargo.toml names it) |
| Findings | S0: 0 S1: 1 S2: 0 S3: 2 S4: 1 |
| Commands run | rg over crates/ + Cargo.tomls (exit 0); sed -n quote verification (exit 0); baseline cargo-test-workspace.txt:3944-4083 read |

## 1. One-paragraph verdict
This is genuine, carefully-commented hand-rolled ole32 COM FFI — vtable layouts,
constants (S_OK/S_FALSE, RPC_E_CHANGED_MODE=0x80010106, COINIT_APARTMENTTHREADED=0x2,
VT_LPWSTR=31) and the IPropertyStore slot order all check out against the real
win32 ABI statically. It is REAL code that has never executed anywhere: the crate is
cfg(windows) so the Linux baseline compiled none of it, no workspace crate depends on
platform-windows, and the header's "compile-verified" claim is unverifiable in this
sandbox. One real ABI defect: `PropVariant` is 16 bytes where x64 `PROPVARIANT` is 24.

## 2. What actually works (verified)
— Static cross-check of every constant and vtable slot order against win32 headers
(propsys.h IPropertyStore order, IUnknown 3-slot prefix, VT_LPWSTR=31): correct.
— Nothing executed: no rustc in this agent sandbox; baseline (cargo-test-workspace.txt:4044-4083)
ran only the 16 cfg-off fallback/seam tests, zero tests of this module.

## 3. Findings
F-001 [S1]
Where: crates/platform-windows/src/windows/com.rs:237-264
Code:
```
pub union PropVariant {
    /// Header: `VT_LPWSTR` = 31 plus three reserved words.
    pub header: PropVariantHeader,
    /// Padding pinning the struct at its ABI size.
    pub raw: [u64; 2],
}
```
What is wrong: On x64 the real `PROPVARIANT` is 24 bytes — its union must hold
counted-array members (e.g. `BLOB {ULONG cbSize; BYTE *pBlobData;}` = 4+pad+8 = 16)
plus the 8-byte vt/reserved header. `PropVariantHeader` (2+6+8) and `[u64; 2]` both
pin this struct at 16 bytes. `property_store_string` (com.rs:373-381) stack-allocates
it and passes `&mut variant` to `IPropertyStore::GetValue` and `PropVariantClear`
(com.rs:296-298); both callees are entitled to write the full 24-byte object
(PropVariantClear zeroes the variant), so up to 8 bytes of adjacent stack can be
clobbered in a release build with no canary. Why it matters: silent stack corruption
during Windows device enumeration, the exact path the crate exists for. What correct
looks like: `raw: [u64; 3]` (24 bytes on 64-bit) or a full union mirroring wtypes.h.
Confidence: HIGH on the size mismatch (layout arithmetic + documented wtypes.h layout);
MEDIUM that a callee writes byte 16..24 in practice. Verification: read — no Windows
runtime exists here.

F-002 [S3]
Where: crates/platform-windows/src/windows/com.rs:107-109,141-152
Code:
```
pub struct ComApartment {
    uninitialize: bool,
}
```
+ Drop SAFETY comment "the guard never crosses threads: every integration creates and
drops it inside one call" (com.rs:144-146).
What is wrong: A one-`bool` struct is automatically `Send + Sync`; nothing enforces the
"same thread" invariant the Drop comment asserts. A caller may legally move the guard
across threads and `CoUninitialize` would then run on the wrong thread, breaking the
ole32 per-thread pairing contract. Current callers (wasapi.rs:211/722, dialogs.rs:341)
keep it on one thread, so the bug is latent. What correct looks like:
`PhantomData<*mut c_void>` field or an explicit `impl !Send`-style guard.
Confidence: HIGH — Send/Sync of a bool-only struct is a language guarantee.
Verification: read.

F-003 [S3]
Where: crates/platform-windows/src/windows/com.rs:279-298
Code:
```
    let text = unsafe { from_wide(pointer)? };
    // SAFETY: PropVariantClear is the documented deallocator for a
```
What is wrong: Two early-return paths skip `PropVariantClear`: (a) `vt != VT_LPWSTR`
(com.rs:279-285) — a variant of some other owning VARTYPE (VT_LPSTR, VT_BLOB…) leaks
its payload; (b) `from_wide(pointer)?` (com.rs:292) — a VT_LPWSTR with invalid UTF-16
(lone surrogate in a device name) leaks the CoTaskMemAlloc'd string. The code itself
anticipates both cases (it formats the VARTYPE number) and leaks on them.
Confidence: HIGH (control flow is direct). Verification: read.

F-004 [S4]
Where: crates/platform-windows/src/windows/com.rs:8-10
Code:
```
//! Hand-rolled per the clean-room rule: no `windows-s`/`winapi`
//! crates — the ole32 C-ABI is declared here, every `unsafe` block
//! carrying its invariant. **Compile-verified via `cargo check
//! --target x86_64-pc-windows-msvc`; runtime NOT exercised in this
```
What is wrong: The "compile-verified" claim cannot be reproduced in this audit
environment (no rustc/target installed; baseline logs contain no windows-target
check — rg over audit/runs/baseline/ found nothing). UNVERIFIED, not necessarily
false. Confidence: HIGH that it is unverifiable here. Verification: UNVERIFIED.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| extern block (5 ole32 fns) | 88-101 | REAL-UNPROVEN | — | Signatures match ole32 (CoInitializeEx/CoUninitialize/CoCreateInstance/CoTaskMemFree/PropVariantClear); `#[link(name="ole32")]` correct |
| ComApartment::new | 119-138 | REAL-UNPROVEN | — | S_FALSE correctly still pairs CoUninitialize; RPC_E_CHANGED_MODE typed |
| ComApartment::drop | 141-152 | REAL-UNPROVEN | S3 | Cross-thread not prevented (F-002) |
| co_create_instance | 166-189 | REAL-UNPROVEN | — | hr+null checked; CLSCTX_INPROC_SERVER only; safety doc present |
| release_interface | 198-210 | REAL-UNPROVEN | — | Release = vtable slot 3 via UnknownVtbl prefix; correct |
| empty_prop_variant | 262-264 | REAL-UNPROVEN | S1 | Zeroes only 16 of 24 ABI bytes (F-001) |
| prop_variant_string | 274-300 | REAL-UNPROVEN | S1/S3 | vt check + UTF-16 decode; leak paths (F-003); undersized struct (F-001) |
| from_wide | 311-329 | REAL-UNPROVEN | — | NUL scan is standard; unbounded read if caller invariant broken (documented) |
| to_wide | 337-346 | REAL-UNPROVEN | — | Rejects interior NULs; correct |
| free_wide | 355-361 | REAL-UNPROVEN | — | CoTaskMemFree(NULL) no-op is documented |
| property_store_string | 369-390 | REAL-UNPROVEN | S1 | Stack slot handed to 24-byte-callee (F-001); vtable order verified correct |
| Guid/UnknownVtbl/PropertyKey/PropVariantHeader/ComObject/VtblPrefix/PropertyStoreObject/PropertyStoreVtbl (types) | 57-80,213-232,239-256,393-415 | REAL-UNPROVEN | S1 | All layouts correct except total PropVariant size (F-001) |

## 5. Test honesty
No tests in this file. The crate's 16 passing tests (baseline
cargo-test-workspace.txt:4044-4083) test only the non-Windows fallbacks — none of
them would fail if every line of com.rs were deleted. Zero test coverage of the FFI.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Compile-verified via cargo check --target x86_64-pc-windows-msvc" | com.rs:8-10 | UNVERIFIED | No rustc in sandbox; no baseline artifact |
| "every unsafe block carrying its invariant" | com.rs:7-8 | FALSE-ish | F-001: the "ABI size" invariant is factually wrong on x64; F-002: the never-crosses-threads invariant is unenforced |

## 7. Dead / duplicated / unreachable code
Whole crate is dead from the shipped product: `rg platform-windows **/Cargo.toml` →
only its own manifest. lumen-cli/desktop/desktop-tauri never link it. The module is
cfg(target_os="windows"), so Linux builds never even compile it.

## 8. Dependencies & flags
audio-output + media-types (path deps) only; no windows-s/winapi (honors the
clean-room rule). `#![allow(unsafe_code)]` at line 26 is needed because workspace
lints deny unsafe_code (root Cargo.toml [workspace.lints.rust]) — scoped to this
module, acceptable.

## 9. Open questions for the next agent
- wasapi.rs hands the 16-byte PropVariant to real propsys — confirm whether any
  other PropVariant construction exists (dialogs.rs IFileDialog paths).
- Does any future card (device-discovery is issue #42) plan to wire platform-* into
  the product? If yes, F-001 becomes a live S1 on the Windows release lane.
