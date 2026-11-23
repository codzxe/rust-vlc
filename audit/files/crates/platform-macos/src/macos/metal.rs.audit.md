# AUDIT: crates/platform-macos/src/macos/metal.rs
| Field | Value |
|---|---|
| Auditor | W08-A09 |
| Date | 2026-11-23 |
| Lines / functions | 384 / 9 (MetalDevices::{new,enumerate,retain_device_at}, devices_from_array, describe_device, MetalDevice(+Send/Debug), MetalLayer::{new,set_drawable_size,next_drawable}, MetalDrawable) |
| File verdict | REAL-UNPROVEN (real hand-rolled FFI, cfg-gated off on this host; runtime never executed anywhere; contains one false ownership claim) |
| Reachable from shipped entry point? | NO — crate has zero dependents (see seam_behaviour audit F-002); module additionally `#[cfg(target_os = "macos")]` (macos/mod.rs:21-22), so Linux host never compiles it |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | rg (Drop impls, cfg gates, wiring), sed quote verification (with `[m`-escape), python3 FNV cross-check n/a here |

## 1. One-paragraph verdict
This is genuine hand-rolled FFI, not a refusal and not a stub: a real
`extern "C" MTLCopyAllDevices` over the Metal framework (metal.rs:59-64) plus
objc_msgSend-per-signature calls for `name`, `registryID`, `maxWorkingSetSize`,
`lowPower`, `isHeadless`, `CAMetalLayer layer/setDevice:/setDrawableSize:/nextDrawable`,
each with a per-block SAFETY comment. But it has one false safety claim that
matters: `MetalDevice` is documented as "retained; released on `Drop`" and no
`impl Drop for MetalDevice` exists anywhere in the crate (rg: only MetalLayer:278,
MetalDrawable:377, AutoreleasePool, coreaudio Watch have Drop) — every
`retain_device_at` call leaks a `objc_retain` reference to an MTLDevice. Runtime
behavior is `[VERIFY]`-labeled and has never executed anywhere in-tree.

## 2. What actually works (verified)
- The FFI declarations are the real ABIs, not invented symbols:
  `#[link(name = "Metal", kind = "framework")] unsafe extern "C" { fn
  MTLCopyAllDevices() -> ObjcId; }` (metal.rs:59-64) is the documented Metal C
  entry point; the msgSend alias set (objc.rs:74-132) types receiver/sel/args
  per signature, and the classic arm64 BOOL-return pitfall is handled correctly
  by declaring `msg_send_bool -> u8` (objc.rs:60,116) — only the low byte of x0
  is read.
- Wiring exists: `MacOsPlatform::discover` on macOS constructs
  `macos::metal::MetalDevices::new()` (lib.rs:171), so on a real mac build the
  scanner is the live `GpuAdapters` impl.
- Compile-verification provenance: worklog.md:556 (task 7.w22b) records
  `cargo check -p platform-macos --target aarch64-apple-darwin and
  x86_64-apple-darwin [--all-targets] clean and warning-free`.
- Nothing verified at runtime: no test, no CI job, no smoke run executes any
  line of this file (macOS runners build only `-p lumen-cli -p desktop`, which
  do not depend on this crate).

## 3. Findings
F-001 [S2]
Where: crates/platform-macos/src/macos/metal.rs:186-250
Code:
```
/// An owned `id<MTLDevice>` (retained; released on `Drop`).
///
/// `MTLDevice` is documented thread-safe, so the handle is `Send`.
/// Obtain one from an enumeration round via [`MetalDevices`].
pub struct MetalDevice {
    device: ObjcId,
}
```
and
```
            // SAFETY: device is a live object (array-owned);
            // objc_retain adds our reference, honored by Drop.
            let retained = retain(device)?;
            Ok(MetalDevice { device: retained })
```
What is wrong: `impl Drop for MetalDevice` does not exist — rg over the whole
crate returns Drop only for MetalLayer (metal.rs:278), MetalDrawable
(metal.rs:377), AutoreleasePool (objc.rs:335) and coreaudio Watch
(coreaudio.rs:623). The doc comment and the SAFETY comment both promise release
on Drop; the code never releases. Every `retain_device_at` call leaks one
`objc_retain` reference to an MTLDevice (a heavyweight system object). Why it
matters: this is the renderer-facing handoff ("the ADR-003 handoff point"); a
renderer that creates and drops surfaces leaks GPU device references
indefinitely, and the false SAFETY comment corrupts the module's documented
ownership discipline that the rest of the file actually upholds. What correct
looks like: `impl Drop for MetalDevice { fn drop(&mut self) { unsafe {
release(self.device); } } }` mirroring MetalLayer/MetalDrawable. Blast radius
today is zero only because nothing shipped reaches this crate. Confidence:
HIGH — Verification: read + rg.

F-002 [S3]
Where: crates/platform-macos/src/macos/metal.rs:206-217
Code:
```
    /// Retain the device at `index` of the last enumeration round as
    /// an owned [`MetalDevice`] handle — the renderer-facing
    /// handoff. The index order matches [`GpuAdapters::enumerate`].
```
against
```
    pub fn retain_device_at(&self, index: u64) -> Result<MetalDevice, PlatformError> {
        // SAFETY: no arguments; the returned array is released
        // exactly once at the end of this function, after the
        // selected device has been retained for the handle.
        let array = unsafe { MTLCopyAllDevices() };
```
What is wrong: the doc says "the last enumeration round" but the code performs
a FRESH `MTLCopyAllDevices()` round. If the device set changes between the
caller's `enumerate()` and `retain_device_at(index)` (GPU hotplug, eGPU
unplug), the fresh round's index `i` can denote a different device and the
caller silently retains the wrong GPU. Why it matters: index-addressed
acquisition after a re-enumeration is a classic wrong-device bug; the doc
promises stability the code does not provide. What correct looks like: either
retain during the original round (store the array), or return the device's
registryID alongside `GpuAdapterInfo` and look up by identity.
Confidence: HIGH (doc/code mismatch is textual) — Verification: read.

F-003 [S3]
Where: crates/platform-macos/src/macos/metal.rs:131-142
Code:
```
    let name_sel = sel("name")?;
    // SAFETY: device is live (array-owned, array still alive); name
    // takes no arguments and returns an NSString valid while the
    // array lives — copied out below, before the array's release.
    let name_string = unsafe { msg_send_id(device, name_sel) };
    if name_string.is_null() {
        return Ok(None);
    }
    // SAFETY: the NSString is live for this call (its owner, the
    // device/array, is alive); the copy happens before any release.
    let name = unsafe { nsstring_to_string(name_string)? };
```
What is wrong: `[device name]` returns an AUTORELEASED NSString, and neither
`enumerate` nor `devices_from_array`/`describe_device` creates an
`AutoreleasePool`. The module's own ownership discipline (objc.rs:38-42: "Every
round that creates such values wraps itself in a pool") is violated on this
path. On the main thread the run loop's pool hides it; on any secondary thread
(enumerate is callable anywhere, `&mut self`) each call leaks one NSString per
device, and Objective-C logs "autoreleased with no pool in place".
`MetalLayer::new` does wrap itself in a pool (metal.rs:296), proving the
authors know the rule. What correct looks like: `let _pool =
AutoreleasePool::new()?;` at the top of `enumerate`. Confidence: HIGH —
Verification: read.

F-004 [S4]
Where: crates/platform-macos/src/macos/metal.rs:361
Code:
```
/// An owned `id<CAMDurableDrawable>` (`nextDrawable`'s result).
```
What is wrong: `CAMetalLayer.nextDrawable` is documented to return
`id<CAMetalDrawable>` (legacy protocol) / `id<MTLDrawable>`; I can find no
Apple protocol named `CAMDurableDrawable`. Cannot be checked against an SDK in
this sandbox (no macOS SDK). Doc-only defect. Confidence: MEDIUM —
Verification: read; SDK comparison UNVERIFIED (no macOS SDK in sandbox).

F-005 [S4]
Where: crates/platform-macos/src/macos/metal.rs:38-40
Code:
```
//! **Evidence status:** compile-verified via `cargo check --target
//! aarch64-apple-darwin` and `x86_64-apple-darwin`; runtime NOT
//! exercised in this sandbox (no macOS SDK) — `[VERIFY]`.
```
What is wrong: the standing "compile-verified" property has no captured
artifact (no log in docs/gates/logs/, no CI step; macOS runners build only
lumen-cli/desktop which do not compile this crate) and cannot drift-fail: a
future edit that breaks the darwin build breaks nothing in any gate. Sole
provenance worklog.md:556. Honest label, unfalsifiable claim.
Confidence: HIGH — Verification: rg over docs/gates + release.yml.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MetalDevices::new | 74-80 | REAL | — | trivial ctor |
| GpuAdapters for MetalDevices::enumerate | 82-104 | REAL-UNPROVEN | S3 (F-003) | real MTLCopyAllDevices + Create-rule release; missing autorelease pool |
| devices_from_array | 107-127 | REAL-UNPROVEN | — | count/objectAtIndex: loop; null device skipped (documented) |
| describe_device | 131-180 | REAL-UNPROVEN | — | name/registryID/maxWorkingSetSize/lowPower/isHeadless; registry_id `as u32` truncation documented (167-173) |
| MetalDevice + Send + Debug | 190-203 | FALSE DOC / LEAK | S2 (F-001) | no Drop despite "released on Drop" |
| MetalDevices::retain_device_at | 213-258 | REAL-UNPROVEN | S3 (F-002) | fresh round, not "last round"; bounds-checked; leaks retain (F-001) |
| MetalLayer::new | 295-318 | REAL-UNPROVEN | — | pool-scoped, retain-before-drain correct |
| MetalLayer::set_drawable_size | 326-332 | REAL-UNPROVEN | — | two-f64 CGSize ABI ok on both targets |
| MetalLayer::next_drawable | 341-358 | REAL-UNPROVEN | — | typed refusal on NULL |
| MetalDrawable + Drop | 367-384 | REAL-UNPROVEN | S4 (F-004) | Drop correct; protocol name in doc suspect |

## 5. Test honesty
No tests in this file; the crate's seam_behaviour tests never compile this
module on the Linux host (cfg-gated out). Zero runtime coverage anywhere —
admitted by the header.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "MTLCopyAllDevices follows the CF Create/Copy rule … released exactly once" | metal.rs:26-28, 96-101 | REAL (code does release once) | read: release(array) on both paths |
| "released on `Drop`" (MetalDevice) | metal.rs:186 | FALSE | rg: no `impl Drop for MetalDevice` in crate |
| "objc_retain adds our reference, honored by Drop" | metal.rs:247-248 | FALSE | same rg |
| "lowPower marks Apple-silicon efficiency GPUs … still hardware" | metal.rs:10-13 | REAL-UNPROVEN | code sets is_software:false unconditionally (metal.rs:178); matches Metal reality |
| "compile-verified via cargo check --target aarch64/x86_64-apple-darwin" | metal.rs:38-40 | REAL-UNPROVEN | worklog.md:556 only; no captured log/CI |
| "registryID … truncated to u32 (high bits generation counter)" | metal.rs:169-173 | REAL-UNPROVEN | plausible; cannot verify against SDK here |

## 7. Dead / duplicated / unreachable code
Whole crate unreachable from shipped entry points (no dependents). Within the
crate, reachable only under `target_os = "macos"` via `MacOsPlatform::discover`
(lib.rs:171). MetalLayer/MetalDrawable have no in-crate consumer at all (the
wgpu renderer is Lead-gated, not yet imported) — dead until ADR-003 lands.

## 8. Dependencies & flags
No external deps (std-only crate, Cargo.toml:39-41). `#![allow(unsafe_code)]`
inner attribute at metal.rs:44 scopes the crate's unsafe exception; acceptable
but it is one of the tree's 121 lint allows.

## 9. Open questions for the next agent
- objc.rs:227-235 doc/code mismatch (not my file): doc promises
  `InvalidEncoding` on NULL NSString, code returns `Ok(String::new())` —
  whoever owns objc.rs should file it.
- Does `MTLDevice.maxWorkingSetSize` exist on both cross-check SDK versions
  (macOS 10.15+)? Compile check would catch it; runtime never will.
- When renderer-wgpu lands (ADR-003), F-001 becomes a live leak on the
  renderer path — fix before wiring.
