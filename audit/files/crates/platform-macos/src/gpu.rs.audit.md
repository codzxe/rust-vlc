# AUDIT: crates/platform-macos/src/gpu.rs
| Field | Value |
|---|---|
| Auditor | W08-A03 |
| Date | 2026-11-23 |
| Lines / functions | 106 / 5 (GpuAdapterInfo::vendor_name, pci_vendor_name, UnavailableGpuAdapters::because, UnavailableGpuAdapters::reason, GpuAdapters::enumerate for the fallback) + trait + struct defs |
| File verdict | REAL for the seam vocabulary + typed fallback (executed on this host) / the macOS lane it describes is REAL-UNPROVEN (Metal impl compile-verified only) / UNREACHABLE from shipped entry points (orphan crate) |
| Reachable from shipped entry point? | NO — no crate outside platform-macos depends on it (rg over every Cargo.toml); lumen-cli/desktop dependency lists exclude it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `cargo test -p platform-macos` → 16 passed (incl. `unavailable_gpu_adapters_returns_the_typed_reason`, `typed_fallbacks_satisfy_the_seam_traits`, `gpu_adapter_info_answers_the_metal_shape`), exit 0; `diff crates/platform-linux/src/gpu.rs crates/platform-macos/src/gpu.rs` → only header lines 1,3-8 differ, body identical; `sed -n` quote verification |

## 1. One-paragraph verdict
This is the macOS GPU seam vocabulary file, and it is a near-byte-identical copy of platform-linux/src/gpu.rs (diff: only the module header differs; struct, PCI vendor table, trait, and typed fallback are the same bytes). The seam machinery itself is real and executed on this host: `UnavailableGpuAdapters::enumerate` returns its stored typed reason (tested), `GpuAdapterInfo::vendor_name` classifies (tested), and the Metal implementation the header advertises is a genuine hand-rolled objc bridge (`macos/metal.rs`: real `MTLCopyAllDevices` extern, name/registryID/maxWorkingSetSize/lowPower/isHeadless queries, retain/release discipline) — but it is compile-verified ONLY (no macOS SDK here; metal.rs:38-40 says so itself). Defects: the trait docs carry Linux vocabulary into the macOS crate ("DRM nodes", "sysfs unreadable" — neither concept exists on macOS); the header presents Metal enumeration as fact while the runtime-unverified caveat lives in a different file; the test comment claims the vendor table is "pinned so they cannot drift" across lanes when the pin covers only 5 of 12 entries with no cross-crate check; and the crate ships to nobody.

## 2. What actually works (verified)
— verified by `cargo test -p platform-macos` → 16 passed, exit 0 (this Linux host; the `macos` module is `#[cfg(target_os = "macos")]` so the Metal impl is compiled out here and the typed-fallback paths are what run — exactly what the tests assert: `gpu.enumerate().is_err()` for the fallback, vendor classification cases, Metal-shape struct usage).
— verified by `diff`: body of this file == platform-linux/src/gpu.rs except the doc header (lines 1, 3-8).

## 3. Findings
F-001 [S4]
Where: crates/platform-macos/src/gpu.rs:22-106 (whole body)
Code:
```
pub fn pci_vendor_name(vendor_id: u32) -> &'static str {
    match vendor_id {
        0x8086 => "Intel",
```
What is wrong: the struct, vendor table, trait, and fallback are copy-pasted verbatim from platform-linux/src/gpu.rs (diff shows only the header differs). Three crates carry three copies of the PCI table (incl. the same `0x1b36 => "VMware"` entry flagged UNVERIFIED in my drm.rs audit). Documented as deliberate ("deliberately mirrored … for the Lead to unify", platform-linux lib.rs:44-47) but the drift risk is real and unmitigated (see F-004). Confidence: HIGH — Verification: read (diff executed).
F-002 [S4]
Where: crates/platform-macos/src/gpu.rs:71-79
Code:
```
    /// Adapters visible right now. An empty list is honest (headless
    /// machines have no DRM nodes; the 7.C1 chain falls back to
    /// software present).
    ///
    /// # Errors
    /// Typed [`PlatformError`] when enumeration cannot run at all
    /// (wrong OS, sysfs unreadable).
```
What is wrong: macOS has neither DRM nodes nor sysfs — this is the Linux trait doc pasted unchanged into the macOS crate. The real macOS error conditions (Metal framework missing, `MTLCopyAllDevices` returning NULL) are not what the doc names. Confidence: HIGH — Verification: read.
F-003 [S3]
Where: crates/platform-macos/src/gpu.rs:1 (whole file) and crate dependency reality
Code:
```
//! The GPU seam (card 7.A3): adapter enumeration.
```
What is wrong: orphan crate — zero dependents outside platform-macos; no shipped binary constructs `MetalDevices` or this seam. Real (compile-verified) Metal code plus executed seam vocabulary, shipping to nobody; any feature-matrix row citing macOS GPU enumeration as a product capability is unreachable-by-wiring. Confidence: HIGH — Verification: read (rg over all Cargo.toml).
F-004 [S4]
Where: crates/platform-macos/src/gpu.rs:3-8 and crates/platform-macos/tests/seam_behaviour.rs:325-331
Code:
```
//! The macOS lane enumerates **Metal devices**
//! (`MTLCopyAllDevices` in [`crate::macos::metal`]) — the device
//! names, IORegistry ids and working-set sizes, plus the retained
//! `CAMetalLayer` glue the renderer will present through (ADR-003
```
```
    // The label table itself still classifies the well-known PCI ids
    // (shared with the other lanes — pinned so they cannot drift).
```
What is wrong: two doc-honesty gaps. (a) The header asserts the macOS lane enumerates Metal devices with no runtime-unverified caveat — that admission lives only in metal.rs:38-40 ("runtime NOT exercised in this sandbox … `[VERIFY]`") and macos/mod.rs; a reader of this file alone would take it as exercised. (b) The test comment "pinned so they cannot drift" is false as a cross-lane claim: the pin covers 5 of 12 table entries, and each crate tests only its own copy — nothing compares the three crates' tables, so they can drift freely (e.g., correcting `0x1b36` in one leaves the other two wrong with all tests green). Confidence: HIGH — Verification: read + ran the pinning test (it passes with 5 cases).
F-005 [S4]
Where: crates/platform-macos/src/gpu.rs:62-63
Code:
```
        0x15ad => "VMware",
        0x1b36 => "VMware",
```
What is wrong: same as the linux-lane finding: two distinct PCI vendor ids labeled "VMware"; 0x15ad is VMware, 0x1b36 is (to my knowledge) a different vendor. Confidence: LOW — Verification: UNVERIFIED (no pci.ids/hwdata in sandbox, no network).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| GpuAdapterInfo::vendor_name | 45-47 | REAL (executed) | — | delegates to table; tested incl. vendor 0 → "unknown" |
| pci_vendor_name | 53-68 | REAL (executed) | S4/S5 | duplicated table (F-001), 0x1b36 label UNVERIFIED (F-005) |
| UnavailableGpuAdapters::because | 91-93 | REAL (executed) | — | tested |
| UnavailableGpuAdapters::reason | 97-99 | REAL (executed) | — | tested |
| <fallback>::enumerate | 103-105 | REAL (executed) | — | returns stored typed reason; tested |
| (documented impl: macos::metal::MetalDevices::enumerate) | metal.rs:83-103 | REAL-UNPROVEN | — | genuine objc FFI; compile-verified only (metal.rs:38-40) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| unavailable_gpu_adapters_returns_the_typed_reason | Yes | asserts the exact stored error comes back from enumerate |
| typed_fallbacks_satisfy_the_seam_traits | Yes | trait-object dispatch over the fallback |
| gpu_adapter_info_answers_the_metal_shape | Partially | pins Metal field semantics (vendor 0, is_software false, VRAM Some) and 5 vendor labels; does NOT pin the full table nor cross-check other crates (F-004) |
| discover_on_a_foreign_host_reports_wrong_os_everywhere | Yes | this host executes it (Linux): all features WrongOs |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The macOS lane enumerates Metal devices (MTLCopyAllDevices …)" | gpu.rs:3-4 | REAL-UNPROVEN | metal.rs:59-64 real extern decl; compile-verified only (metal.rs:38-40); never executed in this sandbox or any CI lane I can see |
| "device names, IORegistry ids and working-set sizes" | gpu.rs:4-5 | REAL-UNPROVEN | metal.rs:132-158 implements the queries (name, registryID, maxWorkingSetSize) — read, not run |
| "the retained CAMetalLayer glue" | gpu.rs:5-6 | REAL-UNPROVEN | metal.rs:261-358 (MetalLayer/MetalDrawable with retain/release) — read, not run |
| "is_software … documented per-lane" (Metal: false by construction) | gpu.rs:35-37 + metal.rs:10-13 | Consistent | metal.rs:178 `is_software: false` with rationale |
| "pinned so they cannot drift" (vendor table, cross-lane) | seam_behaviour.rs:326-327 | FALSE | F-004: 5/12 entries pinned, no cross-crate comparison exists |
| "Typed PlatformError when … sysfs unreadable" | gpu.rs:76-78 | FALSE vocabulary | macOS has no sysfs (F-002) |

## 7. Dead / duplicated / unreachable code
Whole file unreachable from shipped entry points (F-003). Body duplicated from platform-linux/src/gpu.rs (F-001); the same content presumably exists a third time in platform-windows (next file on my list). The `[VERIFY]` wgpu-consumer note (gpu.rs:13-16) is honest.

## 8. Dependencies & flags
Std-only. `macos` module (incl. the real Metal code) is `#[cfg(target_os = "macos")]` (macos/mod.rs:13-20) — on this Linux host the crate compiles with typed fallbacks only, and its 16 tests exercise exactly that. No unsafe in THIS file; the unsafe lives in metal.rs/objc.rs (not my assignment).

## 9. Open questions for the next agent
- Did any CI lane ever actually run `cargo check --target aarch64-apple-darwin` (the claimed compile-verification)? Baseline files only show the Linux host; if the cross-check claim is unrecorded anywhere, metal.rs's "compile-verified" is itself UNVERIFIED.
- Resolve 0x1b36's true owner (F-005) — one fix must land in all three crates' tables, which F-004 shows nothing enforces.
