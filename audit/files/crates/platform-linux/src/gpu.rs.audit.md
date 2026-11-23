# AUDIT: crates/platform-linux/src/gpu.rs
| Field | Value |
|---|---|
| Auditor | W08-A10 |
| Date | 2026-11-24 |
| Lines / functions | 104 / 5 (vendor_name, pci_vendor_name, trait enumerate, because, reason + fallback enumerate) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — platform-linux has no dependents; within the crate it IS the live seam (drm.rs:24,66-67 consumes it, lib.rs discover wires it) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg impl GpuAdapters in drm.rs (exit 0); rg gpu.rs mirrors in platform-windows/macos (exit 0); od -c byte check of line 50 (exit 0); baseline cargo-test-workspace.txt:3959-3991 |

## 1. One-paragraph verdict
This is the OS-neutral GPU vocabulary file — the `GpuAdapterInfo` struct, a correct
11-entry PCI vendor label table, the `GpuAdapters` seam trait, and the typed
`UnavailableGpuAdapters` fallback. It does what it says and its consumers (the real
DRM sysfs scanner in linux/drm.rs, which reuses `pci_vendor_name` at drm.rs:134) are
tested against both synthetic and the real host sysfs in the baseline. Both defects
are structural: the module header narrates the sysfs enumeration that this file does
not contain (it lives in drm.rs), and the whole vocabulary is hand-mirrored across the
three platform crates. No unsafe, no logic errors found — every vendor id checked
against the PCI id ledger is correct.

## 2. What actually works (verified)
— Baseline cargo-test-workspace.txt:3983,3991: `gpu_adapter_info_labels_well_known_pci_vendors ... ok`,
  `unavailable_gpu_adapters_returns_the_typed_reason ... ok`; the consumer scanner's
  tests (:3959-3962, incl. `drm_scanner_reads_the_real_host_sysfs_honestly`) also passed.
— Static check of the vendor table: 0x8086 Intel, 0x1002/0x1022 AMD (ATI graphics +
AMD platform), 0x10de NVIDIA, 0x13b5 Arm, 0x5143 Qualcomm, 0x1010 Imagination,
0x15ad/0x1b36 VMware, 0x1af4 Red Hat virtio, 0x1234 Bochs/QEMU — all correct PCI ids.
— rg: drm.rs:66 `impl GpuAdapters for DrmAdapterScanner` — the seam is implemented and
  wired; is_software classification (drm.rs:106: `0x1234 | 0x15ad | 0x1b36 | 0x1af4`)
  matches this file's doc claim "virtio nodes are classified by PCI vendor".
— od -c on gpu.rs:50 confirms `#[must_use = "pure classification"]` (an rg/sed display
  artifact had rendered it as `#ust_use`; the file bytes are correct).

## 3. Findings
F-001 [S4]
Where: crates/platform-linux/src/gpu.rs:3-5
Code:
```
//! The Linux lane enumerates **DRM/KMS adapter nodes** via sysfs
//! (`/sys/class/drm/card*/device/{vendor,device}`) — pure std, no
//! unsafe, runtime-testable everywhere. The wgpu device/adapter
```
What is wrong: This file contains no enumeration — it is vocabulary + trait + fallback.
The sysfs scanner the header describes lives in `linux/drm.rs` (impl at drm.rs:66). A
reader landing on gpu.rs to audit "the DRM enumeration" finds none of it here; the
header narrates another file's implementation. Why it matters: doc placement that
misdirects review effort on exactly the unsafe-adjacent surface an auditor must trace.
What correct looks like: "The Linux scanner lives in [`crate::linux::drm`]; this module
holds the shared vocabulary and seam." Confidence: HIGH. Verification: read + rg.

F-002 [S4]
Where: crates/platform-linux/src/gpu.rs:20-66,69-104
Code:
```
pub struct GpuAdapterInfo {
```
What is wrong: `GpuAdapterInfo`, `pci_vendor_name`, the `GpuAdapters` trait and the
`UnavailableGpuAdapters` fallback are hand-duplicated in platform-windows/src/gpu.rs
(struct at :23, table at :53) and platform-macos/src/gpu.rs (struct at :41 area) —
three copies of the same contract to keep in sync by hand, same [PROPOSAL]-deferred
unification as error.rs. Confidence: HIGH (verified both mirrors exist).
Verification: rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| GpuAdapterInfo::vendor_name | 42-45 | REAL | — | Passthrough to the table |
| pci_vendor_name | 51-66 | REAL | — | All 11 ids verified correct; unknown → "unknown" (honest) |
| trait GpuAdapters::enumerate | 69-78 | REAL | — | Contract doc: empty list is honest headless truth; errors typed |
| UnavailableGpuAdapters::because | 89-91 | REAL | — | Constructor |
| UnavailableGpuAdapters::reason | 95-97 | REAL | — | Accessor |
| UnavailableGpuAdapters::enumerate (impl) | 101-103 | REAL | — | Always Err(reason.clone()) — the typed refusal, tested |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| gpu_adapter_info_labels_well_known_pci_vendors (seam_behaviour.rs:372, baseline :3983) | YES | Maps concrete ids through vendor_name() and asserts labels; a wrong table entry fails |
| unavailable_gpu_adapters_returns_the_typed_reason (seam_behaviour.rs:36, baseline :3991) | YES | Feeds a reason, asserts enumerate returns exactly it |
No tests in this file itself; the real scanner (drm.rs) carries the substantive sysfs tests.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The Linux lane enumerates DRM/KMS adapter nodes via sysfs" | gpu.rs:3-4 | REAL (but misplaced — drm.rs does it) | drm.rs:66 impl; tests :3959-3962 passed |
| "this seam answers what hardware is present, not create a GPU context" | gpu.rs:7-9 | REAL | No context creation anywhere in the crate |
| "virtio nodes are classified by PCI vendor — documented per-lane" | gpu.rs:33-35 | REAL | drm.rs:106 matches 0x1af4 et al. |
| "[VERIFY] The in-tree wgpu adapter … will consume these descriptors" | gpu.rs:11-14 | REAL-UNPROVEN | renderer-wgpu does not import wgpu yet (per doc); future consumer |

## 7. Dead / duplicated / unreachable code
Nothing dead in-crate (used by drm.rs/lib.rs/capability.rs). Triple duplication of the
vocabulary (F-002). Crate unreachable from lumen-cli/desktop.

## 8. Dependencies & flags
std-only; no unsafe (doc claim verified — zero unsafe blocks in this file).

## 9. Open questions for the next agent
- drm.rs owns the actual sysfs parsing (read_hex on vendor/device files) — audit it for
  unbounded reads and for the "card*" glob matching render nodes vs non-GPU cards.
- When platform-* unification lands, GpuAdapterInfo + error vocabulary should move
  together; the windows copy is the one whose consumers are broken (see my dxgi.rs audit).
