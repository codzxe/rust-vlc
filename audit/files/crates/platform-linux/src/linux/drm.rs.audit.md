# AUDIT: crates/platform-linux/src/linux/drm.rs (whole file, 135 lines)

| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-21 |
| Lines / functions | 135 / 9 units (1 struct, 4 constructors/accessors, 1 trait impl, 3 free fns) |
| File verdict | UNREACHABLE (real, fixture-tested sysfs scanner; the crate has zero consumers) |
| Reachable from shipped entry point? | NO — crate-wide zero-consumer fact (my lib.rs.audit.md header). In-crate it IS wired: `gpu_seam` constructs `DrmAdapterScanner::new()` (lib.rs:178) and drives `enumerate()` through the `GpuAdapters` seam. Executed: 4 drm tests `ok` in `orchrun-test-workspace-nofailfast.log.txt:3647-3650` (degenerate nodes, empty/absent trees, realistic fixture tree, real-host scan). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | Read tool over whole file (verbatim quotes); `rg -n "DrmNode" -S .` (single hit — the doc line itself: the referenced type does not exist); `rg -n "DrmAdapterScanner\|with_roots"` (lib.rs:178 wiring + tests/linux_integrations.rs:124/158/165/181/195); `sed -n '100,215p' tests/linux_integrations.rs` (all 4 test bodies read); `rg -n "pub struct GpuAdapterInfo" -A 20 src/gpu.rs` (field contracts); `rg -n "wgpu" crates/renderer-wgpu/Cargo.toml` + `rg -n "wgpu::" crates/renderer-wgpu/src/` (the `[VERIFY]` wgpu-gate claim: no wgpu dependency; the three `wgpu::` hits are doc comments — claim accurate); `ls /sys/class/drm/ /dev/dri/` on the sandbox host (no DRM here — `version` only; connector-duplication not demonstrable live, see F-001); `rg -n "drm_scanner" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (execution proof) |

## 1. One-paragraph verdict
The most honest file in the platform-linux crate: pure std sysfs walking with genuinely zero `unsafe` and zero ioctls (the header claim is true), injectable roots for deterministic fixtures, absent-sysfs → honest empty list (not an error), unreadable PCI ids → reported with 0 rather than guessed, VRAM `None` because sysfs genuinely does not expose it, and four tests (fixture, empty, degenerate, real-host) that all run green in the no-fail-fast baseline. The defects: the `card`/`renderD` prefix filter admits kernel connector entries (`card0-HDMI-A-1`, `card0-eDP-1`, …), so on a real GPU host the same physical adapter is enumerated once per connector plus its render node as extra ghost rows — a realistic-case wrongness the fixture deliberately omits and this DRM-less sandbox cannot demonstrate live (S3, unreachable crate); `scan_node`'s doc promises a `None` degenerate case its body can never produce (unconditional `Some`); the header references a `[DrmNode]` type and a `device_node: false` field that exist nowhere in the repo — the fact is smuggled into the human-readable `name` string where no consumer can branch on it; and vendor/device `0` carries a third undocumented meaning on top of the two `GpuAdapterInfo`'s docs already give it.

## 2. What actually works (verified)
— "pure std sysfs walking, no `unsafe`, no ioctls" (header claim) — verified: the entire file is `std::fs`/`std::path` + crate imports; zero `unsafe` tokens; no `/dev` opening beyond an `.exists()` probe.
— Absent-sysfs honesty — verified: `enumerate` (69-74) maps `read_dir` failure to `Ok(Vec::new())` with the container-without-sysfs rationale; `drm_scanner_is_honest_on_empty_and_absent_trees` pins both the empty tree and the absent root — `ok` at baseline 3648.
— Realistic-tree correctness — verified: `drm_scanner_reads_a_realistic_sysfs_tree` builds card0 (Intel 0x8086:0x9bc4), renderD128 (same GPU), card1 (virtio 0x1af4:0x1050, no /dev node), plus `version`/`minor0` scratch, and asserts count 3, sort order, exact display strings, `is_software` for virtio only, VRAM `None` — `ok` at baseline 3649.
— Degenerate-node honesty — verified: `drm_scanner_handles_degenerate_nodes` (card2 with no `device/` dir) asserts the node is reported with ids 0, never guessed — `ok` at baseline 3647. (The doc contradicting this behavior is F-002.)
— Real-host discipline — verified: `drm_scanner_reads_the_real_host_sysfs_honestly` runs the live `/sys` (here: `version` only → honest empty) asserting no panics/empty names — `ok` at baseline 3650. Sandbox host checked directly: `/sys/class/drm` → `version` only; `/dev/dri` absent.
— The `[VERIFY]` wgpu-gate claim (14-18) — verified accurate: renderer-wgpu has NO wgpu dependency (Cargo.toml comment records the C5 std-only decision; the `wgpu::` hits in its src are doc comments only). The scanner correctly does not create GPU contexts.

## 3. Findings

### F-001 [S3]
Where: crates/platform-linux/src/linux/drm.rs:78-85
Code:
```rust
            let name = entry.file_name().to_string_lossy().into_owned();
            // Version-scratch entries (`version`, `minorN`) are not
            // adapter nodes.
            if !(name.starts_with("card") || name.starts_with("renderD")) {
                continue;
            }
```
What is wrong: The prefix filter keeps every entry starting with `card` — which on a real DRM host includes the kernel's per-connector symlinks (`card0-HDMI-A-1`, `card0-eDP-1`, `card0-VGA-1`, …) that `/sys/class/drm` publishes alongside the adapter nodes themselves. A connector entry has no `device/vendor` file, so `scan_node` reports it as a ghost adapter: `DRM card0-HDMI-A-1 (unknown vendor 0x0000 device 0x0000, no device node)`. On a typical one-GPU desktop the enumeration returns the real `card0`, its `renderD128`, and one garbage row per connector. The comment shows the author knew the directory carries non-adapter entries but filtered only the two scratch shapes (`version`, `minorN`) the fixture happens to plant.
Why it matters: The seam's entire output contract is "the adapters this host has"; on the only hosts that matter (real GPU machines) the list is inflated with connector ghosts — a UI or the 7.C1 backend-negotiation chain would present phantom adapters. Mitigating to S3: the crate is unreachable from shipped binaries, the fixture suite cannot catch it (its tree has no connector entries), and this sandbox has no DRM to demonstrate live (`/sys/class/drm` = `version` only — checked).
What correct looks like: Match adapter-node names exactly — `card` + digits, `renderD` + digits (e.g. `name.strip_prefix("card").is_some_and(|rest| rest.bytes().all(|b| b.is_ascii_digit()))`, same for `renderD`) — and add a fixture row `card0-HDMI-A-1` asserting it is skipped.
Confidence: HIGH on the filter logic (pure string predicate, read); MEDIUM on the real-world impact (standard sysfs layout, not demonstrable on this DRM-less host — the live claim is UNVERIFIED here, the code path is not).

### F-002 [S4]
Where: crates/platform-linux/src/linux/drm.rs:92-93 (doc) vs 114-122 (body)
Code:
```rust
/// One scanned node → [`GpuAdapterInfo`] (`None` when the node is so
/// degenerate it cannot be described honestly).
```
against the unconditional tail:
```rust
    Some(GpuAdapterInfo {
        vendor_id: vendor,
```
What is wrong: The doc promises a `None` return for "so degenerate" nodes; the body has exactly one exit and it is `Some`. No input — missing `device/` dir, unreadable vendor, missing /dev node — produces `None`. The degenerate test pins the real behavior (report with ids 0), so the doc describes a contract the code and tests jointly contradict.
Why it matters: Doc-code contradiction on the function's own contract; a maintainer adding a `None` case per the doc, or a caller matching on `None`, is working from fiction.
What correct looks like: Delete the parenthetical, or make it true (e.g. return `None` when the entry has neither a `device` dir nor a `/dev/dri` node — the sysfs-ghost case the doc at 98-99 gestures at).
Confidence: HIGH — Verification: read (single `Some` exit; test asserts Some-shaped behavior).

### F-003 [S4]
Where: crates/platform-linux/src/linux/drm.rs:10-12 (doc)
Code:
```rust
//! - `/dev/dri/<node>` — the device node a userspace driver would
//!   actually open (cards without one are reported but flagged
//!   `device_node: false` — see [`DrmNode`]).
```
What is wrong: `[`DrmNode`]` does not exist anywhere in the repository — `rg -n "DrmNode" -S .` returns exactly this one doc line — so the intra-doc link is broken and the `device_node: false` "field" it points at is a fiction. The actual mechanism is a human-language suffix baked into the display string: `if node_exists { "" } else { ", no device node" }` (112). No consumer can branch on "openable" programmatically without string-matching the name.
Why it matters: The one operationally important fact this scanner adds beyond PCI ids (usable vs ghost node) is not in the data model, and the doc points at a type that was never written — doc rot on the file's stated output contract.
What correct looks like: Either a `device_node: bool` (or `Option<PathBuf>`) field on `GpuAdapterInfo` with the doc updated, or drop the `[DrmNode]` reference and document the string convention explicitly.
Confidence: HIGH — Verification: read + repo-wide grep (single hit).

### F-004 [S4]
Where: crates/platform-linux/src/linux/drm.rs:101 and 116 (with gpu.rs:22-25 doc)
Code:
```rust
    let vendor = vendor_id.unwrap_or(0);
```
```rust
        vendor_id: vendor,
        device_id: device_id.unwrap_or(0),
```
What is wrong: `GpuAdapterInfo`'s own field docs give `0` two meanings ("`0` when the API has no PCI notion (Metal reports a registry id instead)" — gpu.rs:22-25). This lane adds a third: `0` = "sysfs entry missing/unreadable". The degenerate test blesses it (`vendor_id == 0` asserted), and `pci_vendor_name(0)` renders "unknown" — but a consumer distinguishing "no PCI notion" from "read failed" from "ghost node" cannot, and `is_software` evaluates `false` for a vendor-0 ghost (a driverless card reads as a hardware adapter).
Why it matters: Overloaded sentinel values on the seam's public data model; each new lane (DXGI, Metal, DRM) invents another meaning for the same zero. Blast radius today: none (crate unreachable).
What correct looks like: `vendor_id: Option<u32>` per-lane, or a `pci_ids_source: enum {Pci, RegistryId, SysfsMissing}` — or at minimum one doc line on `GpuAdapterInfo` enumerating all three zero-meanings.
Confidence: HIGH — Verification: read (both files' docs + the degenerate test's assertion).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `DrmAdapterScanner` (struct) | 27-31 | REAL | — | Two injectable `PathBuf` roots — the whole test seam; `Debug`+`Clone`. |
| `new` | 35-41 | REAL | — | Roots `/` + `/` — with the `sys/class/drm` / `dev/dri` joins this yields the real paths. |
| `with_roots` | 45-51 | REAL | — | Fixture constructor; used by 3 of 4 tests. |
| `sys_root` | 53-57 | REAL | — | Pure accessor; asymmetric — no `dev_root()` twin (hygiene, not worth a finding row). |
| `Default` | 60-64 | REAL | — | Delegates to `new`. |
| `GpuAdapters::enumerate` | 66-90 | REAL | S3 (F-001) | `read_dir` err → honest empty; `entries.flatten()` drops mid-iteration errors (acceptable for sysfs); sort by display name (87) — deterministic but lexicographic ("card10" < "card2"); no panics, no allocation beyond the Vec. |
| `scan_node` | 94-123 | REAL | S4 (F-002, F-003, F-004) | Always `Some` (doc claims otherwise); `node_exists` only feeds a string suffix (F-003); `unwrap_or(0)` sentinel overload (F-004); `format!` per node — fine at adapter counts. |
| `read_hex` | 126-130 | REAL | — | `ok()?` swallows read AND parse errors into one `None` (deliberate); `trim_start_matches("0x")` strips repeated prefixes (`"0x0x86"` parses as 0x86 — accepts garbage silently) and would not strip uppercase `"0X"` (real sysfs is lowercase `0x`); both nits cosmetic at S4- level. |
| `vendor_label` | 133-135 | REAL | — | Thin delegate to the shared `pci_vendor_name` table (gpu.rs:51+) — no duplication, good. |

## 5. Test honesty (drm tests live in tests/linux_integrations.rs:100-207)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `drm_scanner_reads_a_realistic_sysfs_tree` (100) | YES | Exact count, exact display strings, ids, `is_software`, VRAM `None` — a wrong filter that kept `version` would fail the count assert. Mutation check: dropping the scratch filter → len 5 ≠ 3. Baseline ok (3649). |
| `drm_scanner_is_honest_on_empty_and_absent_trees` (150) | YES | Empty tree → `is_empty`; absent root → `is_empty` (an error return fails both). Baseline ok (3648). |
| `drm_scanner_handles_degenerate_nodes` (169) | YES | No-`device/`-dir node must still be reported with ids 0 — pins the anti-guess contract (and pins F-002's doc as fiction). Baseline ok (3647). |
| `drm_scanner_reads_the_real_host_sysfs_honestly` (192) | WEAK | Only asserts non-empty names and no NUL bytes — passes on any host, proves non-panic only. Honest as a smoke, proves nothing about correctness. Baseline ok (3650). |
| Coverage holes | — | No connector-entry fixture (F-001 invisible — the suite's "realistic" tree is less realistic than the kernel's); no `renderD`-without-`card` case; no unreadable-permission case (`.ok()` swallows EACCES into ids 0 — same shape as degenerate, untested distinction). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "pure std sysfs walking, no `unsafe`, no ioctls" | drm.rs:1-2 | REAL | Whole file read: std::fs/path only, zero unsafe, no /dev opens beyond `.exists()`. |
| "card 7.A2" | drm.rs:1 | REAL | docs/13-agent-wave-program.md:705 (platform-linux scope: "GPU via wgpu adapter" — the enumeration half). |
| "cards without one are reported but flagged `device_node: false` — see [`DrmNode`]" | drm.rs:10-12 | FALSE (referent) / SHALLOW (mechanism) | `DrmNode` exists nowhere (grep: 1 hit = this line); the "flag" is a string suffix in `name` (F-003). |
| "`[VERIFY]` … when renderer-wgpu's wgpu import is admitted, this enumeration is the hardware-presence input to the 7.C1 fallback chain" | drm.rs:14-18 | REAL (gate state) / REAL-UNPROVEN (the chain) | renderer-wgpu Cargo.toml has no wgpu dep (C5 std-only, recorded decision; src `wgpu::` hits are comments); the 7.C1 chain itself is unshipped future work. |
| "A kernel without DRM (or a container without sysfs) honestly has no adapters: empty, not an error" | drm.rs:71-73 | REAL | Absent-trees test, baseline ok. |
| "`None` when the node is so degenerate it cannot be described honestly" (scan_node doc) | drm.rs:92-93 | FALSE | Unconditional `Some` (F-002); degenerate test asserts Some. |
| "sysfs does not expose VRAM size (debugfs-only, root); `None` is the honest answer" | drm.rs:118-120 | REAL | Debugfs requires root; fixture asserts `None`; consistent with the field doc. |

## 7. Dead / duplicated / unreachable code
- Crate-wide unreachable (header proof; inherited).
- In-file: nothing dead; every fn is exercised by the four tests. The `sys_root()` accessor has no in-repo caller (tests use it once — linux_integrations.rs:196 — so not dead).
- `vendor_label` is a one-line delegate — borderline dead indirection, but it documents the "shared table" intent; not a finding.

## 8. Dependencies & flags
std only (`fs`, `path`) + crate-internal `error`/`gpu`. No unsafe, no feature flags, no cfg gates, no external crates. `[lints] workspace = true` inherited. The module is behind `pub mod linux`'s `target_os = "linux"` gate (lib.rs:58-59) — non-Linux hosts never compile it.

## 9. Open questions for the next agent
1. Whoever composes this crate: F-001's connector filter must be fixed before any UI shows the adapter list on real hardware; a one-fixture-line test addition pins it.
2. gpu.rs auditor (this wave or next): `GpuAdapterInfo.vendor_id: u32` with the `0`-sentinel is a three-meaning overload across lanes (F-004) — decide `Option<u32>` before a fourth lane (Metal) ships.
3. The Windows/macOS siblings of this scanner (their `dxgi.rs`/`metal` equivalents) should be checked for the same "prefix admits junk entries" shape — mirroring copies defects as reliably as it copies vocabulary.
4. `sort_by(name)` is lexicographic — "card10" sorts before "card2"; if any consumer ever maps index→preference, switch to natural sort. Noted here, filed nowhere.
