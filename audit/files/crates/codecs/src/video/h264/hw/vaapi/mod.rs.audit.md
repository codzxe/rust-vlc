# AUDIT: crates/codecs/src/video/h264/hw/vaapi/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | 237 / 6 fns + 3 impls + 2 tests |
| File verdict | REAL (bridge/accelerator/discover wired and host-tested 2/2 in baseline; the lane's runtime-tested claims verified against baseline evidence) |
| Reachable from shipped entry point? | COMPILED-YES / RUNTIME-DEFAULT-OFF. Chain: `decode::H264Decoder::from_setting` (decode.rs:178,296-300) installs `hw::installed_bridges()` (hw/mod.rs:450, `b.push(Arc::new(vaapi::VaApiBridge))` under cfg linux) → `VaApiBridge::open` → `VaApiSession::new`. `discover()` ← hw/mod.rs:526-530 `discover_lane`. Executes only when hw-decode is enabled (default `software-only` + `features.hw-decode = false`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg baseline vaapi test results (lines 674-691); sed hw/mod.rs:445-460 bridge wiring; rg decode.rs from_setting; ls /dev/dri (absent); sed quote checks |

## 1. One-paragraph verdict
The lane's plug surface is genuinely wired: `discover()` mints an opaque `VaApiAccelerator` only on probe success, the bridge downcasts exactly its own accelerator type and mints an unopened session, and the shipped Linux binary carries this bridge in `installed_bridges()`. The module's runtime-evidence claims check out against the baseline log: the no-device and permission-denied probe paths, the real-libva dlopen symbol resolution, and the pure reconstruction functions all ran green (baseline :674-691), and `/dev/dri` really is absent in this sandbox as claimed. The one defect is documentation consistency: this file says the lane is card 7.B3 (correct, matching hw/mod.rs's truth table) while its own child modules ffi.rs:1 and device.rs:1 say 7.B1 — the same lane carrying two contradictory card citations inside itself.

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:690-691 — `vaapi::tests::bridge_claims_only_its_own_accelerator ... ok`, `accelerator_is_opaque_but_truthful ... ok`.
- The module doc's "Runtime-tested here" list is real: :682-683 (probe no-device / permission-denied), :675 (`loads_the_real_installed_libva_or_reports_it_typed` — the dlopen claim), :676-689 (session pure functions). `/dev/dri` absent verified live (my device.rs audit).
- Bridge wiring into the shipped build: hw/mod.rs:453-456 (sed-verified: `b.push(Arc::new(vaapi::VaApiBridge));` under `#[cfg(target_os = "linux")]`).
- Read the whole file: zero unsafe, no panics on reachable paths, downcast-failure → None (typed upstream `h264.hw.no_bridge`).

## 3. Findings
F-001 [S4]
Where: mod.rs:1-2 vs ffi.rs:1 vs device.rs:1
Code:
```
//! The Linux hardware-decode lane: **VA-API** (card 7.B3, issue
//! #41; ADR-001 §5's E2 OS-service binding; ADR-013 **[PROPOSAL]**
```
What is wrong: this file correctly cites card 7.B3 (matching hw/mod.rs:37-38's numbering), but the two child modules it maps in its own module-map section cite 7.B1 (ffi.rs:1 "VA-API C-ABI declarations (card 7.B1)", device.rs:1 "for the VA-API lane (card 7.B1)"). One lane, two card numbers, three files. Cosmetic cross-reference rot — but exactly the kind of citation drift this audit exists to catch. Confidence: HIGH — Verification: read all three files (quotes in my ffi.rs and device.rs audits).
(Note: session.rs:1424 also says "card 7.B1" per my rg in the ffi audit — the error is repeated across the lane.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| discover | 93-105 | REAL | — | probe → mint-iff-success; consumed by hw::discover_lane (hw/mod.rs:526-530) |
| VaApiAccelerator::new / node / vendor | 123-139 | REAL | — | opaque, no device state (probe's display terminated — documented) |
| Debug impl | 142-149 | REAL | — | test pins the exact Debug shape, no pointers |
| HwAccelerator impl | 151-159 | REAL | — | backend_id "hw.vaapi" (test-pinned) |
| VaApiBridge::open | 166-175 | REAL | — | downcast-or-None; mints `VaApiSession::new(node.clone())` |
| tests: bridge_claims_only_its_own_accelerator | 181-210 | REAL | — | own-vs-other accelerator; session label asserted |
| tests: accelerator_is_opaque_but_truthful | 212-236 | REAL | — | label, vendor accessor, exact Debug string |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| bridge_claims_only_its_own_accelerator | YES | wrong downcast breadth or wrong mint would fail the assertions; baseline green |
| accelerator_is_opaque_but_truthful | YES | exact-string Debug + accessor assertions |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Runtime-tested here: the probe's no-device / permission-denied paths … the real-libva symbol-table resolution (library.rs host test)" | mod.rs:45-48 | REAL | baseline :675, :682-683 all ok; /dev/dri absent in sandbox |
| "Compile-verified only; runtime hardware behavior NOT exercised — [VERIFY]: the driver-mediated decode pipeline … No frame was ever decoded by this lane" | mod.rs:51-55 | REAL (honest) | no GPU here; no vaapi decode path ran in baseline (decode tests use a fake bridge per hw/mod.rs:447-449) |
| "the machines-without-libva path stays a typed LibraryUnavailable, not a load failure" | mod.rs:37-38 | REAL-UNPROVEN | library.rs (another agent's file) test name covers it; branch itself not re-run by me — libva IS installed here, so the typed-failure arm needs library.rs's fixture test to be confirmed there |
| "ADR-013 [PROPOSAL] is the pending Lead decision record for this lane's activation" | mod.rs:2-3 | REAL-UNPROVEN | consistent with hw/mod.rs:52-55 and feature-matrix notes; ADR-013 document itself not audited here |

## 7. Dead / duplicated / unreachable code
None. All pub items are consumed (device/ffi/library/session re-exports, discover by hw::discover_lane, VaApiBridge by installed_bridges, accelerator by session/probe and tests).

## 8. Dependencies & flags
Zero new workspace deps (hand-rolled FFI + dlopen — the ADR-013 option-(a/b)-not-admitted posture, documented at :27-41). cfg(target_os = "linux") parent gate. No unsafe in THIS file.

## 9. Open questions for the next agent
- session.rs / library.rs / dl.rs auditors: the "without libva" typed path (LibraryUnavailable) and the dlopen/dlsym error handling are the remaining hostile-focus surfaces of this lane not in my assignment.
- Whoever fixes the card citations: 7.B1 → 7.B3 in ffi.rs:1, device.rs:1, session.rs:1424 (mod.rs is already correct).
