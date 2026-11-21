# AUDIT: crates/platform-macos/src/macos/metal.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-21 |
| Lines / functions | 384 / 14 units (extern block, 2 impls on MetalDevices incl. trait method, 3 free fns, MetalDevice/MetalLayer/MetalDrawable with 5 methods + 2 Drops + 3 Debugs) |
| File verdict | UNREACHABLE (no crate or app depends on `platform-macos`; additionally the module is `#[cfg(target_os = "macos")]`-gated so the Linux baseline never compiled it — the only wiring is the crate-internal `MacPlatform::discover` gpu_seam, itself caller-free) |
| Reachable from shipped entry point? | NO — manifest scan: no `Cargo.toml` outside the crate lists `platform-macos`; `MacPlatform` has no external callers (mirror of the platform-linux finding); on the Linux baseline build (build log :243 `Compiling platform-macos`) the `macos` module is cfg'd out — this file's bytes were never compiled by any recorded run. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | Read of full file; `sed -n` quote verification (29-33, 149-152, 186-197, 205-212, 247-250); `rg "impl Drop for MetalDevice"` → 0 hits codebase-wide; `rg "MetalDevice\|MetalLayer\|MetalDrawable"` → only gpu.rs/lib.rs comments + this file (no tests, no consumers); manifest scan for `platform-macos` dependents → none; baseline `build-release-workspace.log.txt:243` (crate compiled on Linux — module cfg'd out); INDEX.md consulted: NO cross-target (`--target *-apple-darwin`) check run recorded. |

## 1. One-paragraph verdict
Hand-declared Metal/libobjc FFI with a genuinely careful ownership story on paper and a hole in it big enough to drop a GPU through: `MetalDevice` is documented three separate times as "retained; released on `Drop`" — and has no `Drop` impl, so every `retain_device_at` call leaks an `objc_retain`'d `MTLDevice` reference, and the SAFETY comment inside the retain call ("honored by Drop") is false. `retain_device_at`'s contract claims to index "the last enumeration round" while the implementation re-runs `MTLCopyAllDevices` fresh (a unit struct cannot remember a last round), so the documented index-correspondence with `enumerate` rests on an ordering guarantee Metal does not give. The whole layer-glue half of the file (MetalDevice/MetalLayer/MetalDrawable — the "ADR-003 handoff point") has zero consumers, zero tests, and was never compiled by any recorded run: the module doc's "compile-verified via cargo check --target aarch64/x86_64-apple-darwin" is not reproducible from `audit/runs/baseline/` evidence, and the Linux baseline compiles the crate with this module cfg'd out. The module honestly tags runtime as `[VERIFY]` — but the compile claim is asserted as done and unverifiable, and the missing Drop contradicts the file's own safety narrative.

## 2. What actually works (verified)
Nothing executed — no recorded run compiles or exercises this file (Linux baseline cfg's the module out; no darwin-target run in `audit/runs/baseline/INDEX.md`). Verified by reading only:
— The CF Create/Copy handling in `enumerate` (87-103) is the correct shape: null-check, copy out, single `release(array)`.
— `MetalLayer::new` (295-318) uses the correct scoped-`AutoreleasePool` + retain-before-pool-drain pattern; `MetalLayer`/`MetalDrawable` Drops (278-286, 377-383) pair with their retains.
— `unsafe impl Send for MetalDevice` / deliberately-not-Send `MetalLayer` matches the real thread-affinity facts (MTLDevice thread-safe; CALayer main-thread-affine) — the one Send justification in this domain set that is actually documented correctly.
— `set_drawable_size`'s two-f64 `CGSize` ABI shape is correct for both 64-bit Apple targets (arm64: FP registers; x86_64: 16-byte SSE struct in xmm0/xmm1).
— RegistryID truncation to u32 with the generation-counter rationale documented (169-173).

## 3. Findings

### F-001 [S2]
Where: crates/platform-macos/src/macos/metal.rs:186-192 (contradicted by the absent Drop; promises at 29-31 and 247-249)
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
(the retain it documents:)
```
            // SAFETY: device is a live object (array-owned);
            // objc_retain adds our reference, honored by Drop.
            let retained = retain(device)?;
            Ok(MetalDevice { device: retained })
```
What is wrong: `MetalDevice` has **no `Drop` implementation** — `rg "impl Drop for MetalDevice"` across the codebase returns nothing; the file implements Drop for `MetalLayer` (278) and `MetalDrawable` (377) but not for the device. Three separate texts promise release-on-Drop: the module ownership docs (29-31: "retains the one it wraps and releases it on `Drop`"), the struct doc (186), and the SAFETY comment on the retain itself (248: "honored by Drop").
Why it matters: every `retain_device_at` leaks one `objc_retain`'d MTLDevice reference for the process lifetime — a reference-count leak in unsafe FFI code whose entire safety narrative is the Create/Copy/retain/release discipline. Worse for the audit's purpose: the false SAFETY comment means the next maintainer trusts a release that never comes; in FFI code a false lifetime claim is how use-after-free bugs are born (the inverse mistake — releasing without the Drop — would be a double-free at drop). Downgraded from S1 only because nothing constructs a `MetalDevice` today (crate unreachable, layer glue consumer-free).
What correct looks like: `impl Drop for MetalDevice { fn drop(&mut self) { unsafe { release(self.device); } } }` — exactly the pattern the file already uses for `MetalLayer`/`MetalDrawable` — plus a test that one retain/release pair is balanced.
Confidence: HIGH — Verification: read + codebase-wide rg (0 Drop impls).

### F-002 [S3]
Where: crates/platform-macos/src/macos/metal.rs:205-213 (implementation at 217)
Code:
```
impl MetalDevices {
    /// Retain the device at `index` of the last enumeration round as
    /// an owned [`MetalDevice`] handle — the renderer-facing
    /// handoff. The index order matches [`GpuAdapters::enumerate`].
```
What is wrong: the doc promises the device at `index` of "the **last** enumeration round", but `MetalDevices` is a stateless unit struct (`#[derive(Default)] pub struct MetalDevices;`, 71-72) — it cannot remember any round — and the body calls `MTLCopyAllDevices()` fresh (217). The claimed index correspondence with a previous `enumerate()` call therefore rests on `MTLCopyAllDevices` returning devices in the same order across invocations, which Metal does not guarantee (a GPU hotplug between the two calls shifts every index; the docs make no ordering promise at all).
Why it matters: a caller that enumerated `[Integrated, Discrete]` and asks `retain_device_at(1)` can receive the integrated GPU (order flipped), a different generation's device (hotplug), or a spurious out-of-bounds error (device vanished) — while believing it pinned the enumerated device. When the ADR-003 renderer eventually consumes this handoff, that is "present on the wrong GPU" territory.
What correct looks like: either store the last round's array (retain it for the handle's lifetime) so "last enumeration round" is true, or fix the doc to "a fresh enumeration round" and return the device's identity (registryID) with the handle so callers can detect drift.
Confidence: HIGH — Verification: read (struct is a unit type; call site 217).

### F-003 [S4]
Where: crates/platform-macos/src/macos/metal.rs:149-152
Code:
```
    let vram_sel = sel("maxWorkingSetSize")?;
    // SAFETY: device is live; maxWorkingSetSize returns uint64_t
    // (a macOS Metal method — both cross-check targets are macOS).
    let working_set = unsafe { msg_send_u64(device, vram_sel) };
```
What is wrong: `maxWorkingSetSize` exists on `MTLDevice` only from macOS 10.15; on an older system the raw `msgSend` to an unrecognized selector raises `NSInvalidArgumentException` — a hard crash, not the typed `PlatformError` the crate's whole doctrine promises ("a call that cannot work returns a typed error, never a guessed success" — lib.rs). `sel(...)` resolving the selector name does not mean the object implements it; there is no `respondsToSelector:` guard.
Why it matters: violates the crate's own typed-failure doctrine on the one call here most likely to be absent on old systems; practically negligible in 2026 (10.15 is 7 years old), hence S4.
What correct looks like: a `respondsToSelector:` probe with a typed "unsupported by this macOS" error (the pattern the dialogs seam already uses for class lookup), or a documented deployment-target floor of 10.15 in the crate.
Confidence: MEDIUM — Verification: read; the 10.15 availability of `maxWorkingSetSize` is from the Metal API docs (not re-verifiable in-sandbox); the crash mechanism (unrecognized selector) is Objective-C runtime semantics.

### F-004 [S3]
Where: crates/platform-macos/src/macos/metal.rs:182-384 (the layer glue) with the evidence claim at 38-40
Code:
```
//! **Evidence status:** compile-verified via `cargo check --target
//! aarch64-apple-darwin` and `x86_64-apple-darwin`; runtime NOT
//! exercised in this sandbox (no macOS SDK) — `[VERIFY]`.
```
What is wrong: the layer glue (`MetalDevice`, `MetalLayer`, `MetalDrawable` — 200 of the file's 384 lines) has zero consumers (only doc-comment references in gpu.rs/lib.rs), zero tests (`tests/seam_behaviour.rs` tests the vocabulary and error strings, never these types), and is compiled by no recorded run: the Linux baseline builds the crate with the module cfg'd out, and `audit/runs/baseline/INDEX.md` records no darwin-target check. The "compile-verified" claim above is therefore not reproducible from the audit's evidence set — it may have happened pre-baseline in a dev sandbox, but nothing in the repo's committed runs proves these 384 lines ever passed a compiler.
Why it matters: never-compiled unsafe FFI is the highest-risk code class there is — typos in selectors, wrong ABI shapes, and the F-001-style lifetime bugs all hide exactly here; the honest `[VERIFY]` on runtime is undermined by an unverifiable positive claim on compilation.
What correct looks like: a committed cross-target check lane (or at least the check's log under `audit/runs/`), and a `#[cfg(test)]` smoke that exercises `retain_device_at`'s error paths on any macOS CI runner.
Confidence: HIGH — Verification: read + rg consumer/test scans (0 hits) + INDEX.md review (no darwin run).
Confidence note: whether the historical `cargo check` actually ran is UNVERIFIED — I state only that no committed evidence exists.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-40 | — | S3 | Ownership narrative is false for MetalDevice (F-001); compile claim unverifiable (F-004). |
| `extern "C" MTLCopyAllDevices` | 59-64 | REAL-UNPROVEN | — | Declaration shape correct (CF Create rule); never linked by any recorded build. |
| `MetalDevices`/`new` | 71-80 | REAL-UNPROVEN | — | Stateless unit — the root of F-002's contract lie. |
| `GpuAdapters::enumerate` | 83-103 | REAL-UNPROVEN | — | Correct CF discipline (null check, single release); wired into the caller-free `MacPlatform::discover` (lib.rs:171). |
| `devices_from_array` | 107-127 | REAL-UNPROVEN | — | count/objectAtIndex: correct; null device skipped non-fatally. |
| `describe_device` | 131-180 | REAL-UNPROVEN | S4 | **maxWorkingSetSize crash risk on <10.15 (F-003)**; registryID u32 truncation documented; is_software=false rationale documented. |
| `MetalDevice` + `Send` + Debug | 190-203 | REAL-UNPROVEN | S2 | **No Drop — the F-001 leak;** Send justification itself is correct. |
| `retain_device_at` | 213-258 | REAL-UNPROVEN | S3 | **"last enumeration round" contract false (F-002)**; bounds-checked; never returns a NULL handle. |
| `MetalLayer` Drop | 278-286 | REAL-UNPROVEN | — | Pairs with retain in `new`. |
| `MetalLayer::new` | 295-318 | REAL-UNPROVEN | — | Correct pool/retain/setDevice: sequence. |
| `set_drawable_size` | 326-332 | REAL-UNPROVEN | — | Two-f64 CGSize ABI correct on both 64-bit Apple targets. |
| `next_drawable` | 341-358 | REAL-UNPROVEN | — | Null → typed error; retain for the handle. |
| `MetalDrawable` Debug/Drop | 371-384 | REAL-UNPROVEN | — | Drop pairs with retain. |
| `Debug` impls ×3 | 199-203, 272-276, 371-375 | REAL-UNPROVEN | — | finish_non_exhaustive; no raw pointers printed. |

## 5. Test honesty
No tests exist for this file. `tests/seam_behaviour.rs:53` asserts the *string* "the Metal framework returned no device array" (a copy of the error literal) and `:303-309` tests `GpuAdapterInfo`'s shape — neither compiles or executes a single line of metal.rs (cfg'd out on Linux). There is nothing to be honest or dishonest at runtime; the honesty problem is the doc-level evidence claims (F-004).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "compile-verified via `cargo check --target aarch64-apple-darwin` and `x86_64-apple-darwin`" | module docs 38-40 | UNVERIFIED | No darwin-target run exists in `audit/runs/baseline/` (INDEX.md reviewed); the Linux baseline compiles the crate with this module cfg'd out (build log :243). May have happened; not reproducible from committed evidence. |
| "runtime NOT exercised … `[VERIFY]`" | module docs 39-40 | REAL | Honest: no macOS runtime exists in any recorded run. |
| "`MetalDevice` retains the one it wraps and releases it on `Drop`" | module docs 29-31 | FALSE | No `impl Drop for MetalDevice` exists (codebase-wide rg: 0 hits) — F-001. |
| "the ADR-003-gated wgpu renderer will consume [the layer glue]" | module docs 14-22 | PAPER-ONLY (by design) | No consumer exists; the wgpu import is Lead-gated (records/wgpu-30.0.1.md); matrix row `platform-vout-wgpu` = not_started ×4. |
| "Metal has no software adapter … `is_software` is `false` by construction" | module docs 10-13 | REAL-UNPROVEN | Matches the Metal programming guide (no WARP equivalent); consistent with the code (178). |

## 7. Dead / duplicated / unreachable code
Everything: the crate has no dependents (manifest scan), `MacPlatform::discover` has no callers, and on Linux the module is not compiled. Within the crate: the layer glue (182-384) is additionally dead *within* the crate (no internal consumer beyond doc references).

## 8. Dependencies & flags
- `#[cfg(target_os = "macos")]` gates the whole `macos` module (lib.rs:61) — the seam falls back to `UnavailableGpuAdapters` elsewhere; `#![allow(unsafe_code)]` module exception (44) per MP §9.
- `#[link(name = "Metal", kind = "framework")]` — framework link on Apple targets only; std-only crate otherwise (deps: audio-output, media-types — mirror of platform-linux's manifest shape).
- No feature gates.

## 9. Open questions for the next agent
- `crates/platform-macos/src/macos/objc.rs` (the msgSend shapers, AutoreleasePool, retain/release) is unaudited — every SAFETY claim in this file leans on it; its ABI shapes (`msg_send_void_f64_f64`, `msg_send_id_u64`) deserve their own audit, especially since no compiler ever checked them against a real objc runtime in recorded evidence.
- If the orchestrator can run `cargo check -p platform-macos --target aarch64-apple-darwin` on a mac-equipped runner (REQUEST-ORCH-RUN, optional), the F-004 compile claim becomes checkable and any selector-typo class of bug surfaces.
- Platform-windows (`error.rs`, assigned to me later) and platform-macos share the seam-trait mirror design — whoever audits platform-macos/src/gpu.rs should confirm the `GpuAdapterInfo` vocabulary claims match seam_behaviour.rs:303-309.
