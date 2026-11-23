# AUDIT: crates/codecs/src/video/h264/hw/videotoolbox/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| Lines / functions | 138 / 5 |
| File verdict | REAL-UNPROVEN (compile-verified only, honestly self-labeled; runtime hardware behavior never exercised anywhere) |
| Reachable from shipped entry point? | NO — the module is `#[cfg(target_os = "macos")]` (hw/mod.rs:96-97); on Linux/Windows builds it is not compiled at all. On macOS it would sit behind the same unwired Phase-7 hw lane as capability.rs (H264Decoder not instantiated by any shipped entry point; default policy `software-only`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read of session.rs `probe()` (mint-iff-success invariant); rg of cfg gates (hw/mod.rs:94-97, 459-462, 534-536); rg of docs/feature-matrix.toml row 416 (honesty labels) |

## 1. One-paragraph verdict
This module is the honest end of the hw-lane spectrum: its own header states in plain words that everything below it is "Compile-verified only; runtime hardware behavior NOT exercised — [VERIFY] ... no frame was ever decoded by this lane", and I verified the feature-matrix row (docs/feature-matrix.toml:416) says exactly that — including the recorded OF-5 honesty fix where an earlier false "Media Foundation landed" claim was retracted. The code in this file is thin and correct by inspection: `discover()` mints an accelerator iff the probe answered yes (verified against session.rs `probe()`: Available→Some(detail), Unavailable→None — never an accelerator off a service that answered no), the bridge downcasts exactly its own accelerator type and mints a stateless session, and the backend label is wired to the shared `HwBackend::VideoToolbox` vocabulary. The lane is macOS-cfg-gated and thus not even present in this sandbox's build.

## 2. What actually works (verified)
— verified by read: the mint-iff-success invariant (probe() at session.rs, quoted in my working notes), the bridge claim/downcast logic, backend_id wiring, and the cfg gating. Runtime behavior UNVERIFIED — impossible in this sandbox (no macOS, no Apple hardware), and the module's own documentation says no one has ever exercised it at runtime. Wire-form halves (feed.rs AVCC serializer, CMTime round trip) are host-tested per the docs (cross-file claim, not re-verified by me).

## 3. Findings
None. What was checked and how: (a) fakeness — no stubs, no typed refusals hiding "not implemented" (the one typed refusal, `h264.hw.videotoolbox.session`, is a real configure-time failure path); (b) the accelerator-minting invariant (read of both call sites); (c) honesty labels — module doc vs feature-matrix row 416 vs hw/mod.rs truth table: all three agree (verified by rg + read, including the retracted-MF-claim history); (d) cfg gating — this file compiles only on macOS (hw/mod.rs:96); (e) unsafe — none in this file (ffi/session own it, outside my assignment).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| discover | 77-81 | REAL-UNPROVEN | — | probe + mint-iff-success; consumed by hw/mod.rs:536 (macOS discover_lane) |
| VideoToolboxAccelerator::new/detail + Debug | 91-110 | REAL | — | stateless by design; detail = the hw-active truth |
| HwAccelerator impl | 112-120 | REAL | — | backend_id → "hw.videotoolbox"; as_any for bridge downcast |
| VideoToolboxBridge::open | 127-138 | REAL-UNPROVEN | — | claims only its own accelerator type; mints unopened session (bring-up at configure → typed refusal on vanished device) |

## 5. Test honesty
No tests in this file. Host-tested claims (wire forms) live in feed.rs-facing suites; the hardware runtime is [VERIFY]-labeled everywhere I looked. The h264_hw_path integration suite (5/5 in baseline; not re-run for this file) exercises the seam with mocks/VA-API-shaped inputs on Linux, not this lane.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Compile-verified only; runtime hardware behavior NOT exercised — [VERIFY] ... no frame was ever decoded by this lane" | mod.rs:44-49 | REAL (self-declared limitation) | no Apple hardware exists in the project's test story; feature-matrix row 416 repeats it verbatim |
| "The feature-matrix row says exactly that" | mod.rs:48-49 | REAL | docs/feature-matrix.toml:416 verified — including the OF-5 retraction note |
| "A no is the typed DriverIncompatible truth ... Never a guessed success" | mod.rs:9-13 | REAL (by code read) | probe(): false → Unavailable{DriverIncompatible}, None detail |

## 7. Dead / duplicated / unreachable code
Unreachable in shipped builds by cfg (macOS-only) AND by policy (hw lane unwired, default software-only). Not "dead" — it is the deliberate Phase-7 lane, truthfully labeled.

## 8. Dependencies & flags
No external deps (hand-rolled ffi, documented substrate decision). `#[cfg(target_os = "macos")]` at the parent. No unsafe in this file.

## 9. Open questions for the next agent
- ffi.rs + session.rs owners: the ABI declarations are `[VERIFY]`-labeled as written-from-header-documentation — a future sandbox with macOS SDK should at minimum `cargo check` them again after any macOS target bump.
