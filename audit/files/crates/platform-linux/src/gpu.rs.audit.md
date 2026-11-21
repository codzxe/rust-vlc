# AUDIT: crates/platform-linux/src/gpu.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-21 |
| Lines / functions | 104 / 6 items (struct + method + vendor table + trait + fallback + impl) |
| File verdict | UNREACHABLE (clean vocabulary; orphan crate — cross-ref W03-A02 coreaudio.rs audit / W01-A09 manifest F-001, not re-filed) |
| Reachable from shipped entry point? | NO. `rg 'platform-linux' --glob 'Cargo.toml'` (excluding its own manifest): zero dependents — the crate is an orphan like its two siblings. This vocabulary file is consumed by `linux/drm.rs:24` (the DRM sysfs scanner), `lib.rs` (`gpu_seam` in `LinuxPlatform::discover`) and tests. Executed in-crate: platform-linux 13/13 integration tests pass in baseline, incl. `drm_scanner_reads_the_real_host_sysfs_honestly` (live /sys — orchrun :3646) and `discover_assembles_this_hosts_report` (:3648). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `sed -n` full read · PCI vendor ids cross-checked against the PCI ID registry from auditor knowledge (all 11 mappings correct; 0x1002+0x1022 both → AMD is defensible — ATI + AMD's second id) · `ls crates/platform-linux/src/linux/` + `rg 'GpuAdapters' linux/drm.rs` (:24/:66 — the real consumer) · `rg 'platform-linux' --glob 'Cargo.toml'` (orphan proof) · `sed -n '3638,3684p' orchrun…log` (13/13 linux_integrations, 4 drm_scanner tests) · `rg '0x8086\|0x5143\|0x1234\|0x1af4'` across the three platform crates (tables currently identical — F-001 hazard is prospective) |

## 1. One-paragraph verdict
A minimal, correct GPU-adapter vocabulary: the struct's five fields are exactly what the three OS lanes can honestly produce (the module doc's `[VERIFY]` note admits the wgpu consumer does not exist yet); the 11-entry PCI vendor table is accurate (every id verified against the PCI registry, including both AMD ids and the virtualization trio VMware/virtio/Bochs); the trait's empty-list-is-honest contract and the typed fallback mirror the audio seam exactly; and the lane's real enumeration (DRM sysfs) lives in `linux/drm.rs` and is genuinely executed in the baseline — including against the live host `/sys`. Nothing shipped reaches any of it: like its two sibling crates, platform-linux has zero dependents. The single finding is the triplication of this vocabulary across three crates with no cross-crate equality test — identical today (verified by comparison), a silent-drift hazard tomorrow.

## 2. What actually works (verified)
— PCI vendor table (:51-66): all 11 mappings correct (0x8086 Intel, 0x1002/0x1022 AMD, 0x10de NVIDIA, 0x13b5 Arm, 0x5143 Qualcomm, 0x1010 Imagination, 0x15ad/0x1b36 VMware, 0x1af4 Red Hat virtio, 0x1234 Bochs/QEMU); `0` and unknown → "unknown" (pinned by the macos mirror's test `gpu_adapter_info_answers_the_metal_shape`).
— Honest emptiness contract (:70-72): headless machines have no DRM nodes — the baseline's `drm_scanner_is_honest_on_empty_and_absent_trees` passed (orchrun :3644).
— The consuming implementation is real and executed: `linux/drm.rs:66 impl GpuAdapters for DrmAdapterScanner` — `drm_scanner_reads_a_realistic_sysfs_tree`, `handles_degenerate_nodes`, and `reads_the_real_host_sysfs_honestly` all passed (orchrun :3643-3646), i.e. this vocabulary carries live host data in the committed run.
— Fallback (:100-104): uniform typed refusal — mirror-tested by the platform-linux seam suite (passed).

## 3. Findings

### F-001 [S4]
Where: crates/platform-linux/src/gpu.rs:51-66
Code:
```
pub fn pci_vendor_name(vendor_id: u32) -> &'static str {
    match vendor_id {
        0x8086 => "Intel",
        0x1002 => "AMD",
        0x1022 => "AMD",
```
What is wrong: This table, the `GpuAdapterInfo` struct, and the `GpuAdapters` trait + fallback are maintained as three manual copies — `crates/platform-linux/src/gpu.rs`, `crates/platform-macos/src/gpu.rs`, `crates/platform-windows/src/gpu.rs` — with no test comparing them. Today they are identical (verified: the 11 vendor ids match across all three files, line-offset by the macos/windows file headers), but nothing keeps them that way: a vendor added to the Linux table (the lane that actually reads PCI ids from sysfs) silently leaves the Windows/macOS labels stale, and the per-crate tests pin each copy against itself only.
Why it matters: The lib.rs docs of all three crates record the same `[PROPOSAL]` — the Lead will "unify into a shared L5 crate later" — which is the honest fix, but until then the "stable" vocabulary is three unlinked copies. Blast radius today: zero (orphan crates), hence S4; the hazard is real for the unification card, which will have to guess which copy is authoritative.
What correct looks like: One cross-crate test (in any crate with a dev-dep on the other two, or in a workspace-level test) asserting the three `pci_vendor_name` tables and struct field sets are equal; or a shared `platform-seam-vocabulary` crate now instead of after the [PROPOSAL].
Confidence: HIGH — Verification: read + three-file comparison (identical today).

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-14 | REAL | — | Honest scope: seam answers "what hardware is present", not "create a GPU context"; `[VERIFY]` marks the not-yet-existing wgpu consumer. |
| `GpuAdapterInfo` | 20-37 | REAL | — | 5 fields, per-lane semantics documented (Metal registryID vs PCI); `dedicated_video_memory: Option` honest. |
| `GpuAdapterInfo::vendor_name` | 39-46 | REAL | — | Delegates to the table. |
| `pci_vendor_name` | 51-66 | REAL | S4 | Table correct (11/11 ids verified); F-001 triplication. |
| `trait GpuAdapters` | 68-78 | REAL | — | One method; empty-honest; typed errors. |
| `UnavailableGpuAdapters` + impls | 81-104 | REAL | — | Mirror of the audio fallback; `#[must_use]` on constructors. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `drm_scanner_*` (4 tests, linux_integrations) | YES (for the lane) | Fixture trees + degenerate nodes + live host /sys — passed (orchrun :3643-3646). These test drm.rs through this vocabulary. |
| `gpu_adapter_info_answers_the_metal_shape` (macos seam_behaviour.rs:308, the mirrored copy) | YES | Pins 5 vendor labels + Metal's vendor-0 → "unknown" — passed (orchrun :3699). Covers this table's logic via the mirror, not this file. |
| No test in platform-linux pins `pci_vendor_name` directly | — | The Linux lane's own seam suite exercises discover()/fallbacks; the table is only cross-pinned by the macos copy — part of F-001's hazard. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The Linux lane enumerates DRM/KMS adapter nodes via sysfs… pure std, no unsafe, runtime-testable everywhere" | gpu.rs:3-5 | REAL | `linux/drm.rs` (no unsafe, std fs); 4 drm tests passed incl. live-host (orchrun :3643-3646). |
| "this seam answers 'what hardware is present'… not 'create a GPU context'" | gpu.rs:7-9 | REAL | Trait has exactly one enumerate method. |
| "[VERIFY] The in-tree wgpu adapter… will consume these descriptors when its admission lands" | gpu.rs:11-14 | REAL-UNPROVEN (forward claim, honestly marked) | renderer-wgpu does not import wgpu (verified by W01-A09 and by the crate's own Cargo.toml header). |

## 7. Dead / duplicated / unreachable code
Unreachable in product (orphan crate — cross-ref coreaudio.rs audit §7; not re-filed). Duplicated: F-001 (three manual vocabulary copies, identical today).

## 8. Dependencies & flags
Only `crate::error::PlatformError`. No cfg gates (compiles on every OS — the Linux FFI modules are the cfg-gated ones), no unsafe, no external deps. The heavy lane (DRM sysfs walking) lives in `linux/drm.rs`, correctly outside this vocabulary file.

## 9. Open questions for the next agent
- `linux/drm.rs` owner (wave-3 colleague, if assigned): the `is_software` classification ("virtio nodes classified by PCI vendor") is implemented there — verify the classification list matches this table's virtualization ids (0x1af4/0x15ad/0x1b36/0x1234).
- The Lead's L5-unification [PROPOSAL]: F-001's equality test would be a cheap pre-unification freeze.
