# AUDIT: crates/platform-windows/src/windows/com.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-21 |
| Lines / functions | 416 / 15 (1 extern block, 5 consts + Hresult alias, 3 types + 2 vtable structs, 8 fns, 1 Drop) |
| File verdict | UNREACHABLE (compile-verified real code; orphan crate — no consumer; cfg-gated off the baseline OS) |
| Reachable from shipped entry point? | NO, twice over: (1) `#[cfg(target_os = "windows")] pub mod windows` (platform-windows/src/lib.rs:61-62) — not compiled on the Linux baseline; (2) NO workspace member depends on `platform-windows` at all (verified every Cargo.toml: only platform-linux's doc comment and the crate's own manifest mention it) — even a Windows build of `lumen`/`lumen-tauri` links none of this |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 1 |
| Commands run | for-loop `rg -q "platform-windows"` over root+apps+crates Cargo.tomls → zero dependents; `rg -n "ComApartment\|co_create_instance\|property_store_string\|from_wide\|to_wide\|free_wide" crates/platform-windows/src/` → consumers only inside the crate (wasapi.rs:54-55, dxgi.rs:23, dialogs.rs:35-36); `sed -n 695,760p wasapi.rs` → NotificationSession holds the apartment but is `!Send` (raw pointer field); baseline cross-check: `audit/runs/baseline/orchrun-check-platform-windows.log.txt` → `cargo check -p platform-windows --target x86_64-pc-windows-msvc` EXIT=0; `sed -n 3712,3730p orchrun test log` → platform_windows Linux unit tests: 0 (cfg-gated out), 16 seam tests ok (fallback paths only) |

## 1. One-paragraph verdict
A careful, hand-rolled ole32/COM foundation (apartment guard, CoCreateInstance,
vtable discipline, PROPVARIANT/PROPERTYKEY vocabulary) whose constants and
layouts I verified against the Win32 ABI from memory of the SDK — all correct
(S_OK/S_FALSE/RPC_E_CHANGED_MODE/COINIT_APARTMENTTHREADED/CLSCTX_INPROC_SERVER/
STGM_READ/VT_LPWSTR; IUnknown and IPropertyStore vtable orders; 16-byte
PROPVARIANT union) — and which the orchestrator's cross-check compile-verified
for the Windows target. But none of it can run: the crate is an orphan (zero
workspace dependents), the module is cfg-gated off the baseline OS, and the
runtime is honestly marked `[VERIFY]`-unexercised. Inside the code are two
genuine unsafe-discipline defects: the apartment guard's same-thread
contract is documented but not enforced by the type (auto-Send), and the
property-string reader leaks the CoTaskMemAlloc'd payload on the
invalid-UTF-16 error path.

## 2. What actually works (verified)
— Compile-verified for the real target: `cargo check -p platform-windows
--target x86_64-pc-windows-msvc` → `Finished dev profile … EXIT=0`
(audit/runs/baseline/orchrun-check-platform-windows.log.txt, orchestrator
run 2026-11-21T07:07:29Z). The module doc's compile-verified claim (:8-10)
is backed by committed evidence.
— ABI constants verified against the Win32 SDK values: S_OK=0, S_FALSE=1,
RPC_E_CHANGED_MODE=0x80010106 (:41), COINIT_APARTMENTTHREADED=0x2 (:46),
CLSCTX_INPROC_SERVER=0x1 (:50), STGM_READ=0 (:53), VT_LPWSTR=31 (:259) —
all correct.
— `S_FALSE` apartment handling is correct: both S_OK and S_FALSE set
`uninitialize: true` (:126) — the balancing CoUninitialize the ole32
contract requires for both codes.
— `release_interface` casts through a `#[repr(C)]` IUnknown prefix
(:205-209, :213-222) — layout-guaranteed for any COM pointer; correct.
— IPropertyStore vtable order (:400-415): IUnknown, GetCount, GetAt,
GetValue, SetValue, Commit — matches propsys.h.
— PROPVARIANT size: header {u16, [u16;3], *mut u16} = 16 bytes, pinned by
`raw: [u64; 2]` (:239-244) — correct x64 ABI size.
— Current call sites verified thread-safe: wasapi.rs:211 and dialogs.rs:341
are function-locals; wasapi.rs:712's `NotificationSession` holds the
apartment but is `!Send` (raw `*mut c_void` field, wasapi.rs:711).

## 3. Findings
### F-001 [S3]
Where: crates/platform-windows/src/windows/com.rs:1-10 (module) — with crates/platform-windows/src/lib.rs:61-62
Code:
```rust
//! Hand-rolled per the clean-room rule: no `windows-s`/`winapi`
//! crates — the ole32 C-ABI is declared here, every `unsafe` block
//! carrying its invariant. **Compile-verified via `cargo check
//! --target x86_64-pc-windows-msvc`; runtime NOT exercised in this
//! sandbox (no Windows SDK) — `[VERIFY]`.**
```
What is wrong: The crate this file lives in is an orphan: no workspace
member declares a dependency on `platform-windows` (checked root, all
`apps/*/Cargo.toml`, all `crates/*/Cargo.toml` — the only mentions are
platform-linux's template comment and the crate's own manifest). Combined
with `#[cfg(target_os = "windows")]`, com.rs is unreachable from every
shipped entry point on every OS: the Linux `lumen` binary never compiles
it, and a hypothetical Windows build never links it because nothing
depends on the crate.
Why it matters: ~416 lines of hand-rolled unsafe COM FFI (and its siblings
wasapi/dxgi/dialogs) ship as dead weight; the doc 01 §2.7 L5 registry row
presents platform adapters as the wired layer, and the parity report
(baseline #9: windows=66/252) counts capabilities nothing can invoke.
What correct looks like: device-discovery/audio-output declare
`[target.'cfg(windows)'.dependencies] platform-windows` and probe it via
the seam, or the crate is marked not-yet-wired in the registry.
Confidence: HIGH — Verification: Cargo.toml sweep (commands row) + lib.rs
cfg read.

### F-002 [S3]
Where: crates/platform-windows/src/windows/com.rs:107-109 (with 141-151)
Code:
```rust
pub struct ComApartment {
    uninitialize: bool,
}
```
(whose Drop contract at 144-147:)
```rust
            // SAFETY: pairs with the CoInitializeEx success in `new`,
            // on the same thread (the guard never crosses threads:
            // every integration creates and drops it inside one call).
```
What is wrong: `ComApartment` contains only a `bool`, so the compiler
auto-derives `Send + Sync`. The documented invariant — CoUninitialize must
pair with CoInitializeEx **on the same thread** — is therefore not upheld
by the type: any future caller can create the guard on thread A, move it to
thread B (a plain move, no unsafe needed), and drop it there, calling
`CoUninitialize` on the wrong thread — a COM contract violation with
unspecified behavior. The three current use sites are safe (verified:
two function-locals, one inside a `!Send` struct), so the hole is latent,
but this is exactly the class of bug an unsafe-scoped module must make
unrepresentable.
Why it matters: The whole module's pitch is "every unsafe block carrying
its invariant"; this invariant is carried by a comment, not the type.
What correct looks like: `pub struct ComApartment { uninitialize: bool,
_marker: std::marker::PhantomData<*mut ()> }` (or an explicit
`impl !Send`/`!Sync` equivalent).
Confidence: HIGH — Verification: read; `rg "PhantomData|unsafe impl Send"`
in com.rs → none.

### F-003 [S3]
Where: crates/platform-windows/src/windows/com.rs:289-299
Code:
```rust
    let pointer = unsafe { variant.header.pointer };
    // SAFETY: the caller invariant of from_wide holds — the payload
    // is a NUL-terminated wide string owned by this variant.
    let text = unsafe { from_wide(pointer)? };
    // SAFETY: PropVariantClear is the documented deallocator for a
    // PROPVARIANT the property store filled in; the pointer argument
    // is this live variant.
    unsafe {
        PropVariantClear(variant);
    }
```
What is wrong: On the error path the `?` at :292 returns before
`PropVariantClear` runs. When the VT_LPWSTR payload is not valid UTF-16
(a lone surrogate — driver-authored device names can contain them),
`from_wide` fails, and the CoTaskMemAlloc'd wide string owned by the
variant leaks — once per failed property read, forever unclaimed.
Why it matters: A memory leak on a realistic (if rare) input, in the exact
"typed refusal" path the module advertises as its safety story; the
device-enumeration loop reads this per endpoint per discovery.
What correct looks like: Clear the variant before propagating:
`let text = match unsafe { from_wide(pointer) } { Ok(t) => t, Err(e) => {
unsafe { PropVariantClear(variant); } return Err(e); } }`.
Confidence: HIGH — Verification: read of 274-300; PropVariantClear declared
at :100.

### F-004 [S4]
Where: crates/platform-windows/src/windows/com.rs:311-329 (whole file: 0 tests)
Code:
```rust
pub unsafe fn from_wide(wide: *const u16) -> Result<String, PlatformError> {
    if wide.is_null() {
        return Ok(String::new());
    }
    let mut len = 0usize;
```
What is wrong: The file has no tests at all (the crate's 16 passing
seam_behaviour tests exercise the non-Windows fallbacks, not this module;
the Linux unit-test run for platform_windows is `0 tests`, orchrun log
:3712-3716). Every ABI constant, GUID-adjacent layout, and vtable order
rests on review alone. `from_wide`'s NUL-scan (:318-322) is unbounded if
the caller invariant breaks — a cheap unit test over a fixed buffer would
pin the happy path and the terminator contract on any OS (the function is
testable headlessly).
Why it matters: Hand-rolled FFI without executable pins regresses silently;
the compile check proves it type-checks, not that the layouts are right.
What correct looks like: `#[cfg(test)]` tests for from_wide/to_wide/
empty_prop_variant round-trips and the vtable-offset expectations
(`mem::size_of::<PropVariant>() == 16`, offset_of checks) that run on
every OS.
Confidence: HIGH — Verification: rg for `#[test]`/`mod tests` in com.rs →
none; baseline unit-run read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| extern block (ole32) | 88-101 | REAL (compile-verified) | — | Prototypes match Win32; `unsafe extern` 2024 syntax; link name correct |
| consts (S_OK…VT_LPWSTR) | 33-53, 259 | REAL | — | All values verified correct vs SDK |
| `Guid` | 57-68 | REAL | — | Win32 layout correct |
| `UnknownVtbl` | 71-80 | REAL | — | Correct 3-slot order |
| `ComApartment::new` | 119-138 | REAL | — | S_FALSE pairing correct; RPC_E_CHANGED_MODE typed with exact detail |
| `Drop for ComApartment` | 141-152 | REAL | S3 (F-002) | Same-thread invariant comment-only; type is auto-Send |
| `co_create_instance` | 166-189 | REAL | — | hr+null checked; safety contract documented; CLSCTX_INPROC_SERVER only |
| `release_interface` | 198-210 | REAL | — | IUnknown-prefix cast, single release, null no-op |
| `ComObject`/`VtblPrefix` | 213-222 | REAL | — | Layout shims |
| `PropertyKey` | 225-232 | REAL | — | propsys layout |
| `PropVariant` union + header | 237-256 | REAL | — | 16 bytes pinned; VT_LPWSTR arm only (documented) |
| `empty_prop_variant` | 262-264 | REAL | — | Zeroed per API contract |
| `prop_variant_string` | 274-300 | REAL | S3 (F-003) | Error path leaks payload |
| `from_wide` | 311-329 | REAL | S4 (F-004) | Strict UTF-16 (no lossy); null → empty; unbounded scan on broken invariant (documented) |
| `to_wide` | 337-346 | REAL | — | Interior-NUL rejected — correct |
| `free_wide` | 355-361 | REAL | — | CoTaskMemFree; NULL legal |
| `property_store_string` | 369-390 | REAL | — | GetValue vtable call correct; typed hr error |
| `PropertyStoreObject`/`Vtbl` | 393-415 | REAL | — | Order matches propsys.h (verified) |

## 5. Test honesty (if tests exist in/for this file)
No tests exist in or for this file. The crate's 16 `seam_behaviour` tests
(baseline orchrun :3718-3730, all ok) exercise the **non-Windows fallback**
paths only — none of this module's code executes in any committed test run.
The module doc's `[VERIFY]` runtime marker (:9-10) is honest.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "compile-verified via cargo check --target x86_64-pc-windows-msvc" | module doc :8-10 | REAL | orchrun-check-platform-windows.log.txt EXIT=0 |
| "runtime NOT exercised … [VERIFY]" | module doc :9-10 | Honest UNVERIFIED marker | No Windows runtime evidence exists anywhere in baseline |
| platform-windows is the L5 Windows adapter wired to hosts | doc 01 §2.7 registry row | FALSE in product | Zero dependents (F-001); parity windows=66/252 counts unreachable capability |
| vtable layouts "mirror the SDK headers … runtime confirmation is [VERIFY]" | module doc :20-22 | REAL (review) / UNVERIFIED (runtime) | Orderings verified by review here; no runtime run |

## 7. Dead / duplicated / unreachable code
The entire module is unreachable from shipped entry points (F-001) — the
file-level headline. Within the crate, com.rs itself IS used (wasapi/dxgi/
dialogs import it), so it is not intra-crate dead code.

## 8. Dependencies & flags
`#![allow(unsafe_code)]` module-level escape (the audited OS-service
exception per doc 01 §2.7 / MP §9) — scoped and documented. `#[link(name =
"ole32")]` — resolved only on Windows targets (the cfg gate makes the
Linux baseline not link it). No external crates — clean-room rule upheld
(no windows-s/winapi anywhere in the crate's manifest).

## 9. Open questions for the next agent
- Who is supposed to consume platform-windows? If Wave 4+ wires
  device-discovery/audio-output to it, F-001 downgrades to
  "not-yet-wired"; if nothing ever does, the crate and its ~4 sibling
  modules are dead weight and the parity-report windows column (66/252)
  is counting ghosts.
- wasapi.rs auditor: the NotificationClient vtbl and ref-counting
  (`refs: AtomicU32::new(1)`, wasapi.rs:729) deserve the same
  layout/lifetime scrutiny applied here.
