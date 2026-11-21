# AUDIT: crates/platform-macos/src/gpu.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2026-11-21 |
| Lines / functions | 106 / 8 production units (module doc, struct + 5 fields, `vendor_name`, `pci_vendor_name`, trait, fallback struct + 2 methods + impl) + 2 executed tests in tests/seam_behaviour.rs |
| File verdict | UNREACHABLE (real vocabulary + honest fallback, executed via tests; the Metal integration that fills the vocabulary is compile-only and unverified) |
| Reachable from shipped entry point? | NO. Same leaf-crate proof as lib.rs (no package depends on platform-macos). Within the crate: `GpuAdapters` is wired by `gpu_seam()` (lib.rs:168-189), the fallback is returned on every non-macOS build, and `GpuAdapterInfo` is produced only by `macos::metal::MetalDevices::enumerate` (metal.rs:83-127) — never executed anywhere. Executed evidence: 2 tests green in the no-fail-fast baseline — `gpu_adapter_info_answers_the_metal_shape` and `unavailable_gpu_adapters_returns_the_typed_reason` (orchrun 3691, 3692). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | full read via Read tool; `diff` of gpu.rs vs platform-linux/src/gpu.rs and targeted `rg` vs platform-windows/src/gpu.rs (struct+table+trait+fallback copy-paste ×3 lanes; trait doc byte-identical); `sed -n '125,185p' macos/metal.rs` (who constructs GpuAdapterInfo; registryID truncation); `sed -n '41,52p;53,67p'` + od (quote verification — sed shows the known `#ust_use` ESC artifact, od confirms `#[must_use`); `rg -n "maxWorkingSetSize" crates/ docs/` (selector has no research record — §9); baseline orchrun 3690-3711 (2/2 gpu tests green) |

## 1. One-paragraph verdict
The GPU seam vocabulary is small, honest, and correctly labeled: the
module doc says exactly what this seam is (enumerate hardware, not
create a context), marks the future wgpu consumer `[VERIFY]`, and
admits the renderer does not consume it yet — true (renderer-wgpu does
not depend on this crate). The fallback refuses with the typed reason
(executed green), the vendor-label table's twelve entries are all
correct PCI vendor ids (checked against the standard registry:
0x8086 Intel, 0x1002/0x1022 AMD, 0x10de NVIDIA, 0x13b5 Arm, 0x5143
Qualcomm, 0x1010 Imagination, 0x15ad/0x1b36 VMware, 0x1af4 virtio,
0x1234 Bochs), and the per-lane field semantics (vendor 0 + registryID
for Metal) are documented at both the field and the construction site.
No S0–S3 findings. Four hygiene defects: `vendor_name`'s doc claims
"unknown ids stay numeric" while the code, the table's own doc, and
the executed test all say `"unknown"` — a doc contradicting itself 15
lines away; the PCI table + trait + struct are copy-pasted across all
three platform lanes with tests pinning only 5 of 12 entries per
copy; the shared trait doc cites the Linux lane's substrate (DRM
nodes, sysfs) verbatim in the macOS crate; and the Metal
registryID→u32 `as` truncation rests on an uncited IOKit claim.
§9 records one UNVERIFIED runtime-crash risk discovered in
metal.rs (wrong-looking MTLDevice selector) that belongs to that
file's audit, not this one.

## 2. What actually works (verified)
— Fallback contract — verified by executed test:
`unavailable_gpu_adapters_returns_the_typed_reason`
(seam_behaviour.rs:49-59) asserts reason() round-trips and enumerate()
fails with the exact typed error; green (orchrun 3692).
— Vendor classification — verified by executed test
`gpu_adapter_info_answers_the_metal_shape` (seam_behaviour.rs:308-344):
vendor 0 → "unknown" (the Metal shape), 0x8086→Intel, 0x1002→AMD,
0x10de→NVIDIA, 0x13b5→Arm; green (orchrun 3691). Table's remaining
seven entries verified against the standard PCI vendor registry
(§1) — all correct.
— Vocabulary honesty — verified: "this seam answers *what hardware is
present* … not *create a GPU context*" matches the trait (one method,
`enumerate`); "renderer-wgpu v0 does not import wgpu … will consume
these descriptors when its admission lands" matches reality
(`rg -l "platform-macos" --glob 'Cargo.toml'` → only its own manifest;
renderer-wgpu has no dependency on it).
— Per-lane field semantics — verified: Metal fills vendor 0 +
registryID-in-device_id + `is_software: false` with a documented
rationale at the construction site (metal.rs:166-177).

## 3. Findings

### F-001 [S4]
Where: crates/platform-macos/src/gpu.rs:42-47
Code:
```rust
    /// The vendor's common name for well-known PCI vendor ids
    /// (best-effort labeling; unknown ids stay numeric).
    #[must_use = "pure classification"]
    pub fn vendor_name(&self) -> &'static str {
        pci_vendor_name(self.vendor_id)
    }
```
What is wrong: "unknown ids stay numeric" is false three ways over:
the code returns the string `"unknown"` (`_ => "unknown"`, line 66),
the table's own doc 15 lines down says the truth ("`0` and unknown ids
→ \"unknown\"", lines 50-51), and the executed test pins
`vendor_name() == "unknown"` for vendor 0. The signature
(`&'static str`) cannot return the numeric id without leaking, so the
doc describes an impossible behavior.
Why it matters: a consumer reading the method doc would parse
`vendor_name()` as "numeric when unknown" and break on the actual
`"unknown"` string; it is also an internal contradiction in the same
file, which is how future readers stop trusting the rest.
What correct looks like: "(best-effort labeling; unknown ids return
\"unknown\")" — or make it true with
`fmt::Display`/`Cow<'static, str>` that formats the id.
Confidence: HIGH — Verification: read + executed test output.

### F-002 [S4]
Where: crates/platform-macos/src/gpu.rs:53-67 (and the byte-identical
table in platform-linux/src/gpu.rs:53-66 and
platform-windows/src/gpu.rs:55-68; the struct and trait are likewise
copy-pasted — `diff` shows the whole region identical)
Code:
```rust
pub fn pci_vendor_name(vendor_id: u32) -> &'static str {
    match vendor_id {
        0x8086 => "Intel",
```
What is wrong: the third instance of the mirror-duplication pattern
already recorded for the fallback arms (lib.rs F-003) and events.rs
(§7): struct + table + trait + fallback copied across the three
lanes. Each lane's test pins the same 5 of 12 table entries (Intel,
AMD-1002, NVIDIA, Arm, unknown); the other 7 (AMD-1022, Qualcomm,
Imagination, VMware ×2, virtio, Bochs) can drift per copy with no
test noticing — the "pinned so the lanes cannot drift" claim the
test comments make covers one third of the table.
Why it matters: twelve-entry tables ×3 with 5-entry pins is where
label regressions live; the [PROPOSAL] shared-L5-crate note
(lib.rs:47-50) is the recorded fix and this is one more thing it
would absorb.
What correct looks like: shared `pci_vendor_name` (and
`GpuAdapterInfo`) in the future L5 crate with one exhaustive table
test (12 entries), or at minimum extend each lane's pin to all 12.
Confidence: HIGH — Verification: read + diff across lanes + test
bodies read.

### F-003 [S4]
Where: crates/platform-macos/src/gpu.rs:72-78
Code:
```rust
    /// Adapters visible right now. An empty list is honest (headless
    /// machines have no DRM nodes; the 7.C1 chain falls back to
    /// software present).
    ///
    /// # Errors
    /// Typed [`PlatformError`] when enumeration cannot run at all
    /// (wrong OS, sysfs unreadable).
```
What is wrong: the trait doc is the Linux lane's doc, verbatim: there
are no DRM nodes and no sysfs on macOS — the macOS empty-list story
would be "no Metal device" (essentially unreachable on real Apple
hardware) and the error story is a HAL/objc failure, not sysfs.
Byte-identical to platform-linux/src/gpu.rs:71-77 (verified by diff);
same drift class as events.rs F-001, smaller blast radius (the module
header doc above it IS per-lane correct).
Why it matters: the trait is the seam contract; its doc teaches the
wrong failure model for this OS.
What correct looks like: per-lane examples ("headless machines have
no DRM nodes" → linux copy; "no Metal device / the runtime lookup
failed" → macos copy).
Confidence: HIGH — Verification: read + diff.

### F-004 [S4]
Where: crates/platform-macos/src/gpu.rs:28-30 (contract) and
crates/platform-macos/src/macos/metal.rs:166-174 (the truncation)
Code:
```rust
    /// PCI device id where the OS reports one; Metal's `registryID`
    /// (an IORegistry handle, not PCI) — documented per-lane.
    pub device_id: u32,
```
(the implementation:)
```rust
        // Metal reports no PCI ids: the vendor field stays 0 (the
        // documented per-lane meaning) and the device field carries
        // the IORegistry id, truncated to the seam's u32 (the high
        // bits are a generation counter; the low 32 distinguish the
        // devices of one boot).
        vendor_id: 0,
        device_id: registry_id as u32,
```
What is wrong: a 64-bit OS handle is `as`-truncated into the 32-bit
seam field, defended only by the parenthetical claim that the high 32
bits of an IORegistry entry ID are a boot generation counter. No
citation backs that claim anywhere in the tree (docs/inventory/
platform-macos.md has no registryID research — `rg` found none), and
it is unverifiable in this sandbox. If it is wrong, two Metal devices
in one boot can collide in `device_id` with no error.
Why it matters: checklist "as truncation" on a file-controlled (well,
device-controlled) identifier; today it is inert — nothing consumes
`device_id`, the crate is unreachable — which is the only reason this
is S4 and not S3.
What correct looks like: `device_id: u64` (the seam is new; the PCI
lanes zero-pad), or a `u64` registry-id field, or a cited source for
the generation-counter claim (Apple IOKit documentation reference in
the comment).
Confidence: MEDIUM — Verification: read; the truncation and its
comment are verified, the truth of the mitigation claim is
UNVERIFIED (no offline source; no research record in-tree).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-16 | REAL | — | Per-lane correct (Metal), honestly `[VERIFY]` on the future consumer; non-consumption verified. |
| `GpuAdapterInfo` struct | 20-39 | REAL, EXECUTED (constructed in tests) | S4 | F-003 field/trait doc drift; F-004 u32 device_id truncation contract. |
| `vendor_name` | 44-48 | REAL, EXECUTED | S4 | F-001 doc contradiction; behavior itself correct and pinned. |
| `pci_vendor_name` | 53-67 | REAL, EXECUTED (5/12 entries) | S4 | F-002 triplication + partial pin; all 12 labels verified correct. |
| `GpuAdapters` trait | 71-80 | REAL, EXECUTED (fallback impl) | S4 | F-003 Linux-substrate doc; one-method seam as documented. |
| `UnavailableGpuAdapters` + `because` + `reason` | 83-100 | REAL, EXECUTED | — | Same honest-fallback shape as audio; reason() round-trip pinned. |
| `impl GpuAdapters for UnavailableGpuAdapters` | 102-106 | REAL, EXECUTED | — | Typed refusal, never a fake success. |
| `MetalDevices::enumerate` (producer, metal.rs) | 83-127 | REAL-UNPROVEN | — | Compile-only; never executed on any host (see §9 for the selector risk). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `gpu_adapter_info_answers_the_metal_shape` (seam_behaviour.rs:308) | YES | Pins vendor-0→"unknown" (the Metal shape), VRAM Some, is_software false, and 4 named vendor labels. Mutation: drop the 0x13b5 arm → "Arm" assert fails; invert the 0 match → "unknown" assert fails. EXECUTED green (orchrun 3691). |
| `unavailable_gpu_adapters_returns_the_typed_reason` (seam_behaviour.rs:49) | YES | enumerate() must fail with exactly the constructed reason. Mutation: return Ok(vec![]) → assert_err fails. EXECUTED green (orchrun 3692). |
| Not covered: 7 of 12 table entries (F-002); the Metal producer path (inherent — no Apple hardware; compile-only per crate doc). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "this seam answers '*what hardware is present*' … not '*create a GPU context*'" | module doc 9-11 | REAL | Trait has exactly one enumeration method; no context/device creation anywhere in the crate. |
| "[VERIFY] The in-tree wgpu adapter (renderer-wgpu v0 does not import wgpu) will consume these descriptors when its admission lands" | module doc 13-16 | REAL | No dependency on platform-macos from renderer-wgpu (Cargo.toml grep); honest future-tense marker. |
| "unknown ids stay numeric" | vendor_name doc 43 | FALSE | F-001: code/test/table-doc all say `"unknown"`. |
| "`0` and unknown ids → \"unknown\"" | table doc 50-51 | REAL | Code line 66; executed test. |
| "Metal reports a registry id instead" (vendor 0) | field doc 25-26 | REAL (code) / UNVERIFIED (runtime) | metal.rs:172 sets 0; the Metal runtime itself never executed. |
| matrix rows platform-vout-wgpu / platform-hwdecode-* all "not_started" | docs/feature-matrix.toml | REAL (consistent) | Nothing in this crate claims GPU decode/present capability — the seam is enumeration only. |

## 7. Dead / duplicated / unreachable code
- Whole module unreachable from shipped entry points (crate leaf —
  lib.rs audit Field row).
- F-002: struct + table + trait + fallback copy-pasted ×3 lanes
  (documented [PROPOSAL] mirror pattern).
- Nothing dead within the file: every item is executed by the seam
  suite except the Metal producer, which is future-lane code honestly
  labeled.

## 8. Dependencies & flags
- Deps: `crate::error::PlatformError` only (+ nothing external) —
  consistent with boundaries.toml row. No features, no unsafe in this
  file (all FFI lives in macos/metal.rs), no cfg — compiles on every
  OS as vocabulary + fallback.

## 9. Open questions for the next agent
- **UNVERIFIED RUNTIME-CRASH RISK in metal.rs (not my file, flagged
  for its auditor):** `describe_device` reads VRAM via
  `sel("maxWorkingSetSize")` (metal.rs:151, verified byte-exact with
  od). The documented MTLDevice property is
  `recommendedMaxWorkingSetSize`; I could not find any in-tree record
  justifying `maxWorkingSetSize` (`rg` over docs/ finds nothing), and
  an unrecognized selector in objc_msgSend raises an ObjC exception
  that aborts the process — a crash on GPU enumeration on real
  hardware that the compile-only cross-check (lib.rs F-002) cannot
  catch, because selectors are runtime-resolved. This needs
  verification against Apple headers before the lane is trusted; it
  feeds this file's `dedicated_video_memory: Some(working_set)`
  (metal.rs:175).
- F-004's generation-counter claim needs an Apple IOKit citation or a
  u64 field; whoever owns the shared-L5 [PROPOSAL] should fold both
  into the seam redesign.
- The sibling-lane auditors (platform-linux/gpu.rs,
  platform-windows/gpu.rs): F-002/F-003 apply to your copies; the
  Linux copy's trait doc is the original (correct there), the Windows
  copy inherits the same DRM/sysfs drift.
