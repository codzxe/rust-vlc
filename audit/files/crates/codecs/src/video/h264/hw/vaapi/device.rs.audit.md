# AUDIT: crates/codecs/src/video/h264/hw/vaapi/device.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-21 |
| Lines / functions | 293 / 15 (9 production items + 6 tests/helpers) |
| File verdict | UNREACHABLE (real, host-tested discovery code; no shipped entry point reaches the VA-API lane) |
| Reachable from shipped entry point? | NO — the only consumption chain is `H264Decoder::from_setting` → `hw::negotiate_lane` → `hw::discover_lane` (hw/mod.rs:528) → `vaapi::discover` (vaapi/mod.rs:93) → `session::probe` → `DriScanner::new()` (session.rs:754); `H264Decoder::from_setting` has zero production callers (my decode.rs audit F-001, rg evidence), and product.toml:168 keeps `hw-decode = false`. The shipped h264 lane is `H264SwDecoder` (pipeline_host.rs:346), which never touches DRM discovery. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "DriScanner\|VaLibrary::load" crates/codecs/src` → callers all inside vaapi/{session,library}.rs; `sed -n` quote verification (86-101, 120-125); baseline: test-workspace.log.txt:572-576,578 (all 6 device tests `ok`) |

## 1. One-paragraph verdict
Clean, small, honest code: a typed `/dev/dri` scanner (Found/NoDriDirectory/Empty/Unreadable) with injectable roots for fixture tests, deterministic render-before-card ordering, strict `renderD<digits>`/`card<digits>` name filtering (byte-boundary-safe — the ASCII `starts_with` prefix guarantees the `[7..]`/`[4..]` slice boundary), no unsafe, no ioctls — exactly what its module doc claims. Six fixture tests exercise every discovery shape plus the permission-denied open path, and all ran green in the baseline workspace run on this Linux host. The problem is the same one that runs through this whole lane: nothing shipped can ever call it. The hardware H.264 decoder that owns this chain has no production caller and the product config keeps hardware decode off — this file is compiled-in dead weight on every shipped Linux build, kept alive only by its tests.

## 2. What actually works (verified)
— All 6 in-file tests `ok` in baseline run #3 (test-workspace.log.txt:572-576, 578): discovery ordering, typed NoDriDirectory/Empty, scratch-name rejection, permission-denied verbatim mapping, openable fixture. The tests use real fixture trees under the system temp root — genuine fs-driven evidence, not mocks.
— Name filtering safety hand-verified: `name[7..]`/`name[4..]` can only slice at a char boundary because `starts_with("renderD")`/`starts_with("card")` pins an ASCII prefix; lossy-converted non-UTF-8 names fail the `is_ascii_digit` check and are ignored.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/h264/hw/vaapi/device.rs:45-126 (whole scanner), evidence hw/mod.rs:528, session.rs:754
Code:
```rust
pub fn discover(&self) -> DriDiscovery {
    let dir = self.dri_dir();
    let entries = match fs::read_dir(&dir) {
```
What is wrong: the entire scanner is unreachable from shipped entry points — its only consumers are the VA-API probe inside the hardware lane (session.rs:754, 1431), which only `H264Decoder::from_setting` triggers, which nothing in apps/ constructs (see my decode.rs audit F-001; `rg "H264Decoder" apps/` → comment only). product.toml:168 `hw-decode = false` seals it.
Why it matters: dead weight compiled into every shipped Linux build; the matrix row's "failure-path runtime-tested" evidence is test-only execution.
What correct looks like: land the Phase-7 wiring (ADR-013) or feature-gate the vaapi lane so shipped builds do not carry it.
Confidence: HIGH — Verification: read + rg call graph + product.toml.

### F-002 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/device.rs:120-125 (with struct 21-28)
Code:
```rust
    pub fn open_node(&self, node: &DriNode) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.dri_dir().join(&node.name))
    }
```
What is wrong: `DriNode` is a fully public struct with a public `name` field, and `open_node` joins it into a path without re-validating the `renderD<digits>`/`card<digits>` shape that `discover` enforces. A hand-constructed `DriNode { name: "../../etc/shadow".into(), .. }` opens an arbitrary path read-write (subject to OS permissions). The types are re-exported publicly (`pub use device::{DriDiscovery, DriNode, DriScanner};`, vaapi/mod.rs:75).
Why it matters: no untrusted input reaches this today (nodes come from `discover`'s directory listing), and the crate's own apps never construct nodes — but this is a public API of the codecs crate whose safety rests on caller discipline; one line of validation removes the class.
What correct looks like: `open_node` validates `node.name` against the same digit-suffix pattern before opening (or `DriNode` becomes non-exhaustive with a validated constructor).
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/device.rs:86-87
Code:
```rust
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
```
What is wrong: `flatten()` silently drops per-entry read errors (a single unreadable entry — e.g. a dangling symlink under odd permissions — vanishes without any `Unreadable` report), and the `Unreadable` variant itself (82) is never exercised by any test (no test creates an unreadable `/dev/dri` directory).
Why it matters: the typed-discovery story ("never a bare io error") has a silent-degradation hole and an untested arm; blast radius is a diagnostic-quality gap on an unreachable lane.
What correct looks like: count per-entry errors and surface them in `Found`/`Unreadable`; add the unreadable-directory fixture test (chmod 0o500 dir + read attempt, skipped under root like the existing permission test).
Confidence: HIGH — Verification: read + test inventory.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DriNode` struct | 21-28 | REAL | S4 | pub name — F-002 traversal surface |
| `DriDiscovery` enum | 31-43 | REAL | S4 | Unreadable arm untested (F-003) |
| `DriScanner` struct | 46-49 | REAL | S3 | F-001 unreachable lane |
| `DriScanner::new` | 53-58 | REAL | — | root "/" |
| `DriScanner::with_root` | 62-67 | REAL | — | fixture seam — used by tests |
| `dri_dir` | 70-73 | REAL | — | |
| `discover` | 77-112 | REAL | S4 | typed NotFound split; flatten() drops entry errors; name sort lexicographic (renderD128 < renderD9 — documented "name-sorted", deterministic, acceptable) |
| `open_node` | 120-125 | REAL | S4 | F-002 unvalidated join; RW open (what VAAPI needs) |
| `impl Default` | 128-132 | REAL | — | |
| tests: `temp_root` | 141-149 | REAL | — | pid-tagged scratch; `lumen-allow-literal` comment present |
| tests: `discovers_render_nodes_first_then_cards` | 151-184 | REAL | — | would fail on ordering/filter mutation |
| tests: `missing_dri_directory…` / `empty_dri…` / `scratch_names…` | 186-220 | REAL | — | pin the typed shapes |
| tests: `open_node_maps_permission_denied_verbatim` | 222-249 | REAL | — | root-skip guard is honest |
| tests: `open_node_succeeds_on_an_openable_fixture` | 251-266 | REAL | — | |
| tests: `discover_debug_shape` / `nix_euid_is_root` | 270-292 | REAL | — | std-only root check |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| discovers_render_nodes_first_then_cards | YES | exact ordered list incl. scratch rejection |
| missing_dri_directory_is_the_typed_no_hardware_path | YES | variant-name assertion |
| empty_dri_directory_is_typed_empty | YES | |
| scratch_names_are_not_nodes | YES | "renderD", "card", "renderD12x", "card0x", "render" all rejected |
| open_node_maps_permission_denied_verbatim | YES (when not root) | EACCES kind asserted; skip-under-root is a documented environment guard, not a silent skip |
| open_node_succeeds_on_an_openable_fixture | YES | |
Gap: `Unreadable` never tested (F-003).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "pure std, no `unsafe`, no ioctls" | device.rs :1-2 | REAL | read — fs only, zero unsafe |
| "on this sandbox the answer is 'none' (`/dev/dri` absent), which is exactly the typed NoDeviceNode path the tests pin" | device.rs :7-10 | REAL | test 187-196 ran ok in baseline |
| "mirrors platform-linux's sysfs DRM scanner design (injectable roots → deterministic fixture tests)" | device.rs :12-14 | REAL | with_root seam + fixture tests |
| matrix row codec-video-h264: "VA-API (Linux) compile-verified + failure-path runtime-tested on the no-GPU host" | docs/feature-matrix.toml:416 notes | REAL (as stated) | tests ran ok in baseline — but note the execution is test-scope only, lane unreachable in shipped builds (F-001) |

## 7. Dead / duplicated / unreachable code
The whole file is unreachable-from-shipped-lane code (F-001) — no intra-file dead code. Duplicated concept (not code) with platform-linux's DRM scanner, justified by the documented layering wall (comment :12-14).

## 8. Dependencies & flags
std-only. Compiled only on `target_os = "linux"` (hw/mod.rs:94-95 cfg). No cargo features. No unsafe.

## 9. Open questions for the next agent
- session.rs auditor: confirm `session::probe` actually surfaces `DriDiscovery::Unreadable` as the typed capability reason (device.rs only strings it; the mapping is the probe's job).
- If ADR-013 lands: F-001 flips to S0-relevant overnight — the open_node traversal (F-002) and the flatten() hole (F-003) should be fixed before the lane ships.
