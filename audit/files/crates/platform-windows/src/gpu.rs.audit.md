# AUDIT: crates/platform-windows/src/gpu.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-21 |
| Lines / functions | 107 / 5 (struct + vendor_name, pci_vendor_name, trait, fallback ×2) |
| File verdict | UNREACHABLE (real, tested seam — orphan crate, no consumer) |
| Reachable from shipped entry point? | NO — platform-windows has zero workspace dependents (proof in my com.rs audit F-001: Cargo.toml sweep); the DXGI implementation behind this seam (windows/dxgi.rs) compiles only on Windows targets and links to nothing |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | Vendor-ID table verified correct against the PCI registry (11 ids, all match); diff of the three platform gpu.rs files → same 12-arm table byte-identical (windows==macos exact; windows==linux modulo my sed offsets — arms compared line-by-line and equal); `rg -n "Warp\|WARP"` dxgi.rs → WARP_VENDOR_ID 0x1414 + name-sniff at :256; baseline: `gpu_adapter_info_labels_warp_and_well_known_vendors ... ok` (orchrun :3728, platform-windows), `gpu_adapter_info_labels_well_known_pci_vendors ... ok` (:3671, platform-linux copy) |

## 1. One-paragraph verdict
The OS-neutral GPU-adapter vocabulary (one info struct, a correct 11-entry
PCI vendor table, a one-method seam trait, and a typed unavailable
fallback) — small, clean, and correctly labeled, with its Windows half
(WARP flagging, DXGI descriptors) implemented in dxgi.rs and pinned by a
test that ran ok in the baseline. It is also triply unreachable: the crate
is an orphan nothing depends on, the module is cfg-gated off the baseline
OS, and the consumer this seam was built to feed — the 7.C1
"wgpu backend → software present" fallback chain — does not exist in any
shipped binary (renderer-wgpu ships the software compositor,
`video.software-compositor`). The seam is additionally copy-pasted across
the three platform crates with the vendor table byte-identical, and the
one vendor the code demonstrably knows (Microsoft's WARP, 0x1414, named a
constant in dxgi.rs) is deliberately the one the table renders "unknown".

## 2. What actually works (verified)
— Vendor table verified against the PCI registry: 0x8086 Intel, 0x1002 AMD
(ATI), 0x1022 AMD, 0x10de NVIDIA, 0x13b5 Arm, 0x5143 Qualcomm, 0x1010
Imagination, 0x15ad + 0x1b36 VMware, 0x1af4 Red Hat virtio, 0x1234
Bochs/QEMU — all correct.
— Test executed ok: `gpu_adapter_info_labels_warp_and_well_known_vendors`
(baseline orchrun :3728) — constructs a WARP descriptor (vendor 0x1414,
device 0x008C, "Microsoft Basic Render Driver"), asserts `is_software`,
and pins five vendor→label cases including the honest
`0x1414 → "unknown"`.
— Mutation check: deleting any table arm fails the test's case list;
flipping `is_software` in the test's WARP descriptor fails the assert.
— The typed fallback (`UnavailableGpuAdapters`) returns the same typed
reason on every call — no fake success (matches the module's MP §13
discipline).

## 3. Findings
### F-001 [S3]
Where: crates/platform-windows/src/gpu.rs:70-80
Code:
```rust
/// The GPU adapter seam.
pub trait GpuAdapters: Send {
    /// Adapters visible right now. An empty list is honest (headless
    /// machines have no DRM nodes; the 7.C1 chain falls back to
    /// software present).
```
What is wrong: The seam's stated consumer — the 7.C1 fallback chain —
does not exist: no shipped binary queries `GpuAdapters` (the crate is an
orphan, proof in the com.rs audit), and the shipped renderer is the
software compositor by feature string. The doc comment describes a chain
that would consume this ("the auto-negotiation input for the 7.C1
fallback chain", :10-11) as if it were live.
Why it matters: GPU capability negotiation is dead code in every shipped
build; the module doc's "[VERIFY] the in-tree wgpu adapter … will consume
these descriptors when its admission lands" (:13-16) is honest about the
future but the present-tense fallback-chain framing overstates today.
What correct looks like: The doc says "will consume … when admission
lands" consistently (it half does); the real fix is the crate wiring
(same as com.rs F-001) — device-discovery consuming the seam or the
registry marking it not-yet-wired.
Confidence: HIGH — Verification: crate orphan proof (com.rs audit);
baseline features `video.software-compositor` (INDEX.md).
(Counted once for this file; identical defect class as the crate-level
finding — the com.rs filing covers the crate, this row covers this
seam's own consumer claim.)

### F-002 [S3]
Where: crates/platform-windows/src/gpu.rs:53-68 (with platform-linux/src/gpu.rs:53-65, platform-macos/src/gpu.rs)
Code:
```rust
pub fn pci_vendor_name(vendor_id: u32) -> &'static str {
    match vendor_id {
        0x8086 => "Intel",
        0x1002 => "AMD",
        0x1022 => "AMD",
        0x10de => "NVIDIA",
```
What is wrong: The entire seam (struct, 12-arm vendor table, trait,
fallback) is copy-pasted across the three platform crates — the table is
byte-identical (verified by line-by-line diff of all three), the struct
and trait differ only in module-doc prose. Unlike error.rs (which at
least documents a unification PROPOSAL), the gpu triplication carries no
unification note at all in this copy.
Why it matters: Same drift class as the error.rs triplication (my finding
there): a new vendor id or a table fix must land three times with no
equality pin; the seam's own doc (:13-16) already acknowledges the
field set is "pinned to what the three OS lanes can honestly produce" —
a cross-crate invariant held by copy-paste.
What correct looks like: One shared crate (the L5 vocabulary the error
doc proposes) exporting GpuAdapterInfo + pci_vendor_name; or an
interim CI diff-check across the three tables.
Confidence: HIGH — Verification: three-way diff (windows==macos exact;
linux arms compared individually and equal).

### F-003 [S4]
Where: crates/platform-windows/src/gpu.rs:61-66 (with windows/dxgi.rs:33-36)
Code:
```rust
        0x1af4 => "Red Hat (virtio)",
        0x1234 => "Bochs (QEMU)",
        _ => "unknown",
```
What is wrong: The crate knows Microsoft's PCI vendor id — dxgi.rs:33-36
declares `WARP_VENDOR_ID: u32 = 0x1414` ("Microsoft's PCI vendor id — the
WARP software adapter") and uses it to classify — but the label table
deliberately omits 0x1414, so the most common Windows software adapter
renders as vendor "unknown" while exotic entries (Bochs, virtio) get
labels. The omission is test-pinned as intentional
("Microsoft is not in the PCI label table", seam_behaviour.rs:337) —
documented, but the vendor knowledge is split across two files with
opposite answers.
Why it matters: A UI showing "unknown / Microsoft Basic Render Driver"
for WARP relies on the name string alone; one table row would make the
label truthful and consolidate the knowledge.
What correct looks like: `0x1414 => "Microsoft"` in the table (the
is_software classification stays dxgi.rs's job).
Confidence: HIGH — Verification: dxgi.rs:33-36 read; test case read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `GpuAdapterInfo` (struct) | 22-39 | REAL | — | OS-neutral; Option VRAM honest for Metal; ARCH-6 no-OS-types claim holds |
| `GpuAdapterInfo::vendor_name` | 44-47 | REAL | — | Delegates |
| `pci_vendor_name` | 53-68 | REAL | S3 (F-002), S4 (F-003) | 11 ids verified correct; triplicated; omits 0x1414 |
| `GpuAdapters` (trait) | 71-80 | REAL (unreachable) | S3 (F-001) | `Send`, &mut enumerate; empty-list-is-honest contract |
| `UnavailableGpuAdapters::because` / `reason` | 88-100 | REAL | — | Typed fallback carrier |
| `GpuAdapters for UnavailableGpuAdapters` | 102-106 | REAL | — | Always the typed reason; no fake success |

## 5. Test honesty (if tests exist in/for this file)
Tests live in tests/seam_behaviour.rs (crate-level), ran ok in baseline:
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| gpu_adapter_info_labels_warp_and_well_known_vendors | YES | Five exact vendor→label cases + WARP is_software assert; deleting an arm or flipping WARP classification fails it |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "this seam answers 'what hardware is present' … the auto-negotiation input for the 7.C1 fallback chain" | module doc :9-11 | UNREACHABLE — no consumer exists in any shipped binary | F-001; orphan crate; software compositor ships |
| "[VERIFY] the in-tree wgpu adapter … will consume these descriptors when its admission lands" | :13-16 | Honest future marker (REAL-UNPROVEN by design) | renderer-wgpu does not import wgpu (crate manifest check in prior waves; feature string corroborates) |
| "0 and unknown ids → 'unknown'" (best-effort labeling) | :50-51 | REAL | Tested (0x0000 → "unknown") |
| WARP flagged (is_software) | dxgi.rs + test | REAL (library), UNREACHABLE (product) | Test ok (:3728); seam unwired |

## 7. Dead / duplicated / unreachable code
- Unreachable: the whole seam (F-001 — crate-level orphan).
- Duplicated: struct + table + trait + fallback across three crates
  (F-002); this is the second proven triplication in the adapter trio
  (error.rs was the first — same class, there documented with a
  unification proposal, here not even that).

## 8. Dependencies & flags
`crate::error` only. No unsafe (the unsafe DXGI half lives in
windows/dxgi.rs behind the cfg gate), no I/O, no feature gates. `Send`
bound on the trait is the right minimal sharing contract for a &mut
enumerate seam.

## 9. Open questions for the next agent
- dxgi.rs auditor: the `name.contains("Basic Render Driver")` string
  sniff at dxgi.rs:256 is a locale/robustness hazard (localized Windows
  renders a different description) — classify by vendor id alone when
  0x1414, keep the name check as a fallback only.
- If the platform trio is ever unified, both triplications (error.rs,
  gpu.rs) should move together — the seam and its error vocabulary are
  one API surface.
