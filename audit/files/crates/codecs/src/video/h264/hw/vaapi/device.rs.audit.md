# AUDIT: crates/codecs/src/video/h264/hw/vaapi/device.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | 293 / 7 fns + 6 tests (+2 test helpers) |
| File verdict | REAL (pure std, host-tested 6/6 in baseline; sandbox claims verified live) |
| Reachable from shipped entry point? | COMPILED-YES / RUNTIME-DEFAULT-OFF. Consumed by vaapi/session.rs:753-754,1431,1438 (`probe_with`/`probe`) via the lane negotiation — which executes only when hw-decode is enabled (default `software-only` + `features.hw-decode = false`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | ls /dev/dri (ABSENT — doc claim verified); rg consumer trace; baseline log lines 668-673; sed quote checks |

## 1. One-paragraph verdict
A small, clean, honest DRM-node discovery module: pure std, zero unsafe (as claimed at :2), injectable root for fixture tests, typed four-way discovery result, render-before-card ordering, strict name validation (only `renderD<digits>` / `card<digits>` count), raw io::Error passthrough on open. The module doc's sandbox claim — "on this sandbox the answer is 'none' (/dev/dri absent)" — is true in this sandbox (`/dev/dri` absent, verified). All six tests passed in baseline, including the EACCES-verbatim test (so the baseline ran unprivileged, as the test's root-skip guard expects). The only defects are cosmetic: the wrong card citation (7.B1; VA-API is 7.B3 per hw/mod.rs's truth table — same error as ffi.rs) and the root-skip in the permission test silently counting green under root CI.

## 2. What actually works (verified)
- Baseline: cargo-test-workspace.txt:668-673 — all 6 `vaapi::device::tests::*` ok.
- Sandbox reality: `ls /dev/dri` → absent → the typed `NoDriDirectory` path is this host's real answer (doc :7-8 verified).
- Consumer wiring: session.rs:1431 `probe()` → `probe_with(&DriScanner::new())`; :753-754 fixture-injectable seam — the discovery code is genuinely the lane's front door.
- Read the whole file: no unsafe (the module claims none and has none), no panics on parse paths (read_dir errors typed; entry.flatten()), no unbounded work (one directory read).

## 3. Findings
F-001 [S4]
Where: device.rs:1
Code:
```
//! DRM device-node discovery for the VA-API lane (card 7.B1) —
```
What is wrong: same cross-reference error as ffi.rs:1 — hw/mod.rs:37-38 assigns 7.B1 to VideoToolbox and 7.B3 to VA-API. Confidence: HIGH — Verification: read both files.

F-002 [S4]
Where: device.rs:223-228
Code:
```
        // Skip under root (CI service accounts): chmod-000 denial is
        // only enforceable for unprivileged users.
        if nix_euid_is_root() {
            return;
        }
```
What is wrong: under a root CI runner the EACCES-verbatim assertion silently never runs while the test reports green — the same pass-counting skip pattern as the ffmpeg-absent skips (this baseline ran unprivileged, so it did execute: line 671 ok). Confidence: HIGH — Verification: read; baseline proves non-root execution.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DriScanner::new / with_root / dri_dir | 53-73 | REAL | — | injectable-root test seam; baseline-exercised |
| discover | 77-112 | REAL | — | NotFound→NoDriDirectory, other err→Unreadable(msg); render-first; strict name filter (`renderD`+digits len>7, `card`+digits len>4); by-path/version ignored. NOTE: sort is lexicographic (`renderD1000` < `renderD999`) — numeric order not preserved past 3 digits; harmless (any render node serves VA-API equally), no finding |
| open_node | 120-125 | REAL | — | O_RDWR; raw io::Error to caller (documented) |
| Default | 128-132 | REAL | — | |
| tests: discovers_render_nodes_first_then_cards | 151-184 | REAL | — | ordering + scratch-entry rejection asserted |
| tests: missing_dri/empty/scratch | 186-220 | REAL | — | typed-path assertions |
| tests: open_node_maps_permission_denied_verbatim | 222-249 | REAL | S4 | F-002 root-skip; otherwise strong (mode-000 → EACCES) |
| tests: open_node_succeeds_on_an_openable_fixture | 251-266 | REAL | — | |
| helpers: temp_root / discover_debug_shape / nix_euid_is_root | 141-149, 270-292 | REAL | — | pid-keyed temp dirs; /proc Uid parse std-only |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| discovers_render_nodes_first_then_cards | YES | exact ordered-name assertion incl. ignored scratch entries |
| missing_dri / empty / scratch | YES | variant pinned by name |
| open_node_maps_permission_denied | YES (when unprivileged) | EACCES kind asserted; would fail if open swallowed errors — root-skip is the hole (F-002) |
| open_node_succeeds… | YES | a broken open_node (e.g. wrong flags/path) fails |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "pure std, no `unsafe`, no ioctls" | device.rs:2 | REAL | full read: zero unsafe blocks, only fs/OpenOptions |
| "on this sandbox the answer is 'none' (/dev/dri absent)" | :7-8 | REAL | `ls /dev/dri` → absent (this audit's sandbox) |
| "The scanner mirrors platform-linux's sysfs DRM scanner design (injectable roots …)" | :12-14 | REAL-UNPROVEN | injectable-root pattern present; the platform-linux mirror claim not diffed (other crate) |

## 7. Dead / duplicated / unreachable code
None. Every item is consumed by session.rs or the tests.

## 8. Dependencies & flags
Pure std. cfg(target_os = "linux") via parent `pub mod vaapi` (hw/mod.rs:94). Compiled in the shipped Linux build; runtime path behind the OFF-by-default hw-decode settings.

## 9. Open questions for the next agent
- session.rs auditor (not my file): confirm `probe_with`'s NoDriDirectory mapping to the typed `NoDeviceNode` reason the module doc at :8-10 references — the mapping lives there, not here.
