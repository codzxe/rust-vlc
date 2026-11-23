# AUDIT: crates/platform-linux/src/linux/drm.rs
| Field | Value |
|---|---|
| Auditor | W08-A03 |
| Date | 2026-11-23 |
| Lines / functions | 135 / 8 (new, with_roots, sys_root, Default::default, GpuAdapters::enumerate, scan_node, read_hex, vendor_label) |
| File verdict | REAL (pure-std sysfs scanner, executed on fixtures AND the live host) + UNREACHABLE from shipped entry points (platform-linux has zero dependents) |
| Reachable from shipped entry point? | NO — `DrmAdapterScanner` is constructed only in `platform-linux/src/lib.rs:178` (`gpu_seam` → `LinuxPlatform::discover`); no crate outside platform-linux depends on it (rg over every Cargo.toml); lumen-cli/desktop dependency lists exclude it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `cargo test -p platform-linux drm` → 4 passed (drm_scanner_handles_degenerate_nodes, drm_scanner_is_honest_on_empty_and_absent_trees, drm_scanner_reads_a_realistic_sysfs_tree, drm_scanner_reads_the_real_host_sysfs_honestly), exit 0; `ls /sys/class/drm` → only `version` (host has no DRM nodes — the live-host test honestly returned empty); `sed -n` quote verification |

## 1. One-paragraph verdict
A genuine, small, pure-std sysfs walker: it lists `/sys/class/drm/card*|renderD*`, reads PCI vendor/device ids, checks `/dev/dri/<node>` presence, classifies software vendors, and sorts deterministically — no unsafe, no ioctls, honestly returning `None` VRAM (sysfs cannot report it). I executed all four of its tests including the live-host scan. Two dishonesties: (1) `scan_node`'s doc promises `None` for degenerate nodes but no `None` path exists — the test suite pins the opposite behavior, so the doc and the `Option` signature are both lies about intent; (2) the trait doc promises a typed error "when enumeration cannot run at all (wrong OS, sysfs unreadable)" but this implementation maps EVERY `read_dir` failure to `Ok(vec![])`, so the typed-error path is dead on Linux and an unreadable sysfs reports "Available, zero adapters". And the file ships to nobody: the crate is an orphan.

## 2. What actually works (verified)
— verified by `cargo test -p platform-linux drm` → 4 passed, exit 0. The realistic-tree test asserts exact display strings, vendor/device ids, is_software classification (virtio 0x1af4 → software), `no device node` flagging, scratch-entry filtering (`version`, `minor0`), and `dedicated_video_memory == None`. The live-host test ran against this machine's `/sys/class/drm` (contains only `version` → honest empty list, no panic).

## 3. Findings
F-001 [S3]
Where: crates/platform-linux/src/linux/drm.rs:92-94
Code:
```
/// One scanned node → [`GpuAdapterInfo`] (`None` when the node is so
/// degenerate it cannot be described honestly).
fn scan_node(sys_node: &Path, dri_root: &Path, name: &str) -> Option<GpuAdapterInfo> {
```
What is wrong: `scan_node` has NO path that returns `None` — every `card*`/`renderD*` entry yields `Some`, even a node with no `device/` dir (ids 0, "unknown" label), which the degenerate test explicitly pins (`assert_eq!(adapters.len(), 1)`). The doc comment contradicts the code, and the `Option` return type is dead weight implying filtering that never happens. What correct looks like: either return `None` for nodes without a `device` dir (matching the doc), or fix the doc and drop the `Option`. Confidence: HIGH — Verification: read + ran the pinning test.
F-002 [S3]
Where: crates/platform-linux/src/linux/drm.rs:69-74 and crates/platform-linux/src/gpu.rs:74-77
Code:
```
        let entries = match fs::read_dir(&drm_class) {
            Ok(entries) => entries,
            // A kernel without DRM (or a container without sysfs)
            // honestly has no adapters: empty, not an error.
            Err(_) => return Ok(Vec::new()),
        };
```
```
    /// # Errors
    /// Typed [`PlatformError`] when enumeration cannot run at all
    /// (wrong OS, sysfs unreadable).
```
What is wrong: the trait documents a typed error for "sysfs unreadable", but this impl converts every `read_dir` error — including `EACCES`/`EIO`, not just `ENOENT` — into `Ok(vec![])`. Consequently the `Err` arm of `gpu_seam` (lib.rs:181-184, wiring `UnavailableGpuAdapters`) is unreachable on the Linux lane, and a machine with a broken-but-present sysfs is reported `Capability::Available` with zero adapters instead of the honest Unavailable the crate's capability-negotiation docs promise ("never a guessed success"). What correct looks like: distinguish `NotFound` (→ honest empty) from other io errors (→ typed error). Confidence: HIGH — Verification: read (the Err arm's unreachability is structural: enumerate's only `Err` returns don't exist).
F-003 [S3]
Where: crates/platform-linux/src/linux/drm.rs:1-18 (whole file) and the crate's dependency reality
Code:
```
//! DRM/KMS adapter enumeration (card 7.A2) — pure std sysfs walking,
//! no `unsafe`, no ioctls.
```
What is wrong: like pulse.rs (this agent's audit), the entire crate is wired to nothing — zero dependents outside platform-linux; no shipped binary constructs this scanner. Real, tested code shipping to nobody; any feature-matrix row citing DRM enumeration as a product capability is unreachable-by-wiring. Confidence: HIGH — Verification: read (rg over all Cargo.toml + source).
F-004 [S4]
Where: crates/platform-linux/src/linux/drm.rs:106 and crates/platform-linux/src/gpu.rs:60-61
Code:
```
    let is_software = matches!(vendor, 0x1234 | 0x15ad | 0x1b36 | 0x1af4);
```
```
        0x15ad => "VMware",
        0x1b36 => "VMware",
```
What is wrong: two distinct PCI vendor ids are both labeled "VMware". 0x15ad is VMware; 0x1b36 is, to my knowledge, a different vendor (likely Parallels, Inc.), not a second VMware id — if so the label is wrong and the inclusion of 0x1b36 in the software set deserves its own justification. Confidence: LOW — Verification: UNVERIFIED (no `/usr/share/misc/pci.ids` or hwdata in this sandbox, no network); flagged for the next agent to check against the PCI ID database.
F-005 [S4]
Where: crates/platform-linux/src/linux/drm.rs:87
Code:
```
        adapters.sort_by(|a, b| a.name.cmp(&b.name));
```
What is wrong: lexicographic name sort puts `card10` before `card2` (machines with ≥11 DRM nodes exist: multi-GPU + render nodes). Cosmetic ordering defect. What correct looks like: natural/version sort on the node index. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DrmAdapterScanner::new | 36-41 | REAL (executed) | — | real-host roots /,/ |
| DrmAdapterScanner::with_roots | 46-51 | REAL (executed) | — | fixture seam used by 3 tests |
| DrmAdapterScanner::sys_root | 55-57 | REAL (executed) | — | asserted by live-host test |
| Default::default | 60-64 | REAL | — | delegates to new |
| GpuAdapters::enumerate | 67-89 | REAL (executed) | S3 | filters scratch entries; F-002 error swallowing |
| scan_node | 94-123 | REAL (executed) | S3 | F-001 doc lie; F-004 vendor set |
| read_hex | 126-130 | REAL (executed) | — | 0x-prefix parse, absence → None |
| vendor_label | 133-135 | REAL | — | delegates to shared table |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| drm_scanner_reads_a_realistic_sysfs_tree | Yes | exact-string, id, classification, ordering, scratch-filter assertions on a 3-node fixture |
| drm_scanner_is_honest_on_empty_and_absent_trees | Yes (for the empty cases) | pins absent-sysfs → empty; does NOT cover the EACCES case (F-002) |
| drm_scanner_handles_degenerate_nodes | Yes | pins the always-Some behavior (the doc's `None` claim is what's wrong, not the test) |
| drm_scanner_reads_the_real_host_sysfs_honestly | Weakly | accepts empty OR non-empty; only asserts non-empty names/no NULs — an environment-truth smoke, not a discriminator |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "pure std sysfs walking, no unsafe, no ioctls" | drm.rs:1-2 | TRUE | file read; no unsafe/ioctl anywhere |
| "cards without one are reported but flagged device_node: false" (paraphrase: `no device node` suffix) | drm.rs:10-12 | TRUE | drm.rs:112; test asserts the suffix |
| "`None` when the node is so degenerate it cannot be described honestly" | drm.rs:92-93 | FALSE | F-001: no None path; test pins len 1 for a device-less node |
| "Typed PlatformError when enumeration cannot run at all (…sysfs unreadable)" | gpu.rs:74-76 (this impl) | FALSE | F-002: all read_dir errors → Ok(empty) |
| "sysfs does not expose VRAM size … None is the honest answer" | drm.rs:118-120 | TRUE | test asserts None |
| `[VERIFY]` wgpu integration point is future input | drm.rs:14-18 | TRUE (honest) | renderer-wgpu v0 imports no wgpu (per gpu.rs:11-12); nothing consumes these descriptors |

## 7. Dead / duplicated / unreachable code
Whole file unreachable from shipped entry points (F-003). The `Option` in `scan_node`'s signature is dead (never None, F-001). `vendor_label` is a one-line delegation wrapper around `crate::gpu::pci_vendor_name` (pointless indirection, S4-adjacent, noted here not numbered).

## 8. Dependencies & flags
Std-only (fs, path). No feature gates; `target_os = "linux"` gates the parent `linux` module. No unsafe.

## 9. Open questions for the next agent
- Verify 0x1b36's true owner against the PCI ID database (F-004) — gpu.rs labels it "VMware"; if it is Parallels (my recollection), fix label + re-justify the software-vendor set.
- If a feature-matrix row cites DRM enumeration for Linux GPU capability, cross-check reachability (F-003) — the crate is an orphan.
