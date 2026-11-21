# AUDIT: crates/codecs/src/video/h264/hw/videotoolbox/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-21 |
| Lines / functions | 138 / 11 items (2 module decls, 1 re-export, 1 free fn, 1 struct + 4 impl fns, 1 bridge struct + 2 impl fns) |
| File verdict | UNREACHABLE (macOS-cfg'd lane root; no shipped consumer; not compiled on the baseline Linux host) |
| Reachable from shipped entry point? | NO — chain would be `decode.rs::H264Decoder::from_setting → hw::installed_bridges (macOS arm pushes VideoToolboxBridge, hw/mod.rs:459-464) → VideoToolboxBridge::open → VideoToolboxSession`; `from_setting` has no non-test caller and `hw-decode = false` (product.toml:168). `discover()` is called only by hw/mod.rs:536 (macos cfg arm of `discover_lane`, itself gated behind an enabling policy that shipped defaults never set). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | rg "VideoToolboxBridge\|VideoToolboxAccelerator\|videotoolbox::discover" repo-wide (consumers: hw/mod.rs only + own file); rg "installed_bridges\|from_setting" (hw/mod.rs audit F-001 trace); Read of session.rs (both parts) and hw/mod.rs for the wiring claims; baseline logs searched for darwin/videotoolbox evidence (none) |

## 1. One-paragraph verdict
The lane root: `discover` (probe + accelerator mint), the opaque `VideoToolboxAccelerator` (a detail string and nothing else — by design), and the `VideoToolboxBridge` plug that downcasts the negotiated accelerator and mints an unopened session. The design is coherent and honest about its own shape — the accelerator carries no device state, so a vanished device surfaces as the typed configure refusal, and the module docs label the runtime evidence gap (`[VERIFY]`, compile-verified only). But it is orchestration for an unreachable subsystem: macOS-only compilation (never built by any baseline run), reached only through a constructor no shipped binary calls, with its sole verification claim (a darwin cross-check) having no committed evidence. The bridge's `open` discards the downcast binding by name (`_concrete`), which is correct-but-cryptic for the lane's one type-safety act.

## 2. What actually works (verified)
— Nothing executed (not compiled on Linux; no tests exist for this file — the integration suite's `h264_hw_path.rs` uses its own fake bridge, not `VideoToolboxBridge`).
— Structural verification (read): `discover()` mints an accelerator **iff** the probe succeeded (`detail.map(...)`, :79) — never an accelerator off a failed probe, matching hw/mod.rs's `discover_lane` contract (:522-525); the `Arc<VideoToolboxAccelerator> → Arc<dyn HwAccelerator>` coercion at hw/mod.rs:537 type-checks against `discover`'s signature; `VideoToolboxBridge::open` returns `None` for every non-VideoToolbox accelerator via the `?` on the downcast (:133), which is exactly `H264HwPath::open`'s walk contract (hw/mod.rs:393-397).
— The host-testable halves this lane rides (feed.rs wire forms, CMTime round trip) did run: baseline :557-562.

## 3. Findings

### F-001 [S2] The lane root is unreachable twice over — macOS-only compilation and a constructor no shipped binary calls
Where: crates/codecs/src/video/h264/hw/videotoolbox/mod.rs:58-138; gates at hw/mod.rs:94-97
Code:
```
pub mod ffi;
pub mod session;
```
Cross-file (hw/mod.rs:459-464):
```
    #[cfg(target_os = "macos")]
    let bridges = {
        let mut b = bridges;
        b.push(Arc::new(videotoolbox::VideoToolboxBridge));
        b
    };
```
What is wrong: (a) The module compiles only on macOS — no baseline run (build/test/clippy, all Linux) ever type-checked it; (b) even on macOS, `VideoToolboxBridge` enters the picture only via `installed_bridges` inside `H264Decoder::from_setting`, and `H264Decoder` (the hw decoder) is constructed nowhere outside codecs-crate tests; the shipped CLI hard-codes `H264SwDecoder::new` (apps/cli/src/pipeline_host.rs:346) and the `playback.hw-decode-policy` setting is feature-gated off (`feature_gate = "hw-decode"`, schemas/settings.toml:648; `hw-decode = false`, product.toml:168).
Why it matters: 138 lines here (plus ~1900 in session.rs/ffi.rs) are maintained as a live lane while being doubly dead in every shipped artifact. Same finding class as my hw/mod.rs F-001 — repeated per file because each audit file must stand alone.
What correct looks like: A default-off `codec-h264-hw` Cargo feature gating vaapi+videotoolbox (mirroring `codec-av1`/`codec-vp9`, crates/codecs/Cargo.toml:60-81), or a shipped consumer behind the product gate.
Confidence: HIGH — Verification: read + rg caller trace at every hop (bridge, accelerator, discover).
### F-002 [S3] The "compile-verified via cargo check --target aarch64-apple-darwin" claim has no committed evidence
Where: crates/codecs/src/video/h264/hw/videotoolbox/mod.rs:44-49
Code:
```
//! - **Compile-verified only; runtime hardware behavior NOT
//!   exercised — `[VERIFY]`**: everything in [`ffi`] and
//!   [`session`] (`cargo check --target aarch64-apple-darwin`; no
//!   macOS SDK, no Apple hardware in this sandbox — no frame was
//!   ever decoded by this lane). The feature-matrix row says
//!   exactly that.
```
What is wrong: No log of the darwin check exists anywhere in the repo's evidence set (audit/runs/baseline/INDEX.md enumerates every run; none is a cross-compile). The claim is repeated in session.rs:8-10 and ffi.rs — three files asserting a verification nobody can inspect. If the check ran on a developer machine against a stale revision, or never ran, this lane's FFI decls are unchecked syntax.
Why it matters: This file's entire quality posture rests on that one uncited check. (Same finding as session.rs#part2 F-003 — cited here because the claim also lives here.)
What correct looks like: A committed `cargo check -p codecs --target aarch64-apple-darwin` log (CI lane or orchestrator run). REQUEST-ORCH-RUN: `cargo check -p codecs --target aarch64-apple-darwin`.
Confidence: HIGH (absence of evidence verified); MEDIUM (whether it ever ran — UNVERIFIED).
### F-003 [S4] Bridge `open` discards the downcast binding the lane's type-safety depends on
Where: crates/codecs/src/video/h264/hw/videotoolbox/mod.rs:132-137
Code:
```
    fn open(&self, accel: &Arc<dyn HwAccelerator>) -> Option<Box<dyn H264HwDecoder>> {
        let _concrete = accel.as_any().downcast_ref::<VideoToolboxAccelerator>()?;
        // The session brings itself up at configure (the
        // accelerator carries no device state by design).
        Some(Box::new(VideoToolboxSession::new()))
    }
```
What is wrong: The downcast's *result* is the one place this lane proves the accelerator is its own; binding it to `_concrete` and dropping it makes the check read like an ignored parameter. It is functionally correct (the `?` propagates `None` for foreign accelerators), but the naming hides the load-bearing line.
Why it matters: Readability of the single type-safety act in the lane; a future editor could "clean up" the seemingly-unused binding without realizing the `?` is the whole point.
What correct looks like: `let Some(_concrete) = accel.as_any().downcast_ref::<VideoToolboxAccelerator>() else { return None };` — explicit about the refusal.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pub mod ffi` / `pub mod session` | 58-59 | REAL-UNPROVEN | S2 | Module decls; macOS-cfg'd parent (hw/mod.rs:96-97); unreachable (F-001). |
| `pub use session::{VideoToolboxSession, probe}` | 61 | REAL-UNPROVEN | — | Re-export; both names consumed in-file / by hw/mod.rs macos arm. |
| `discover` | 77-81 | REAL-UNPROVEN | — | Probe + conditional mint; verified: accelerator minted iff probe detail present; matches hw/mod.rs:535-538 consumption. |
| `VideoToolboxAccelerator` (struct) | 86-89 | REAL-UNPROVEN | — | One String field — genuinely stateless per the design claim. |
| `VideoToolboxAccelerator::new` | 91-95 | REAL-UNPROVEN | — | pub(crate); sole caller discover():79. |
| `VideoToolboxAccelerator::detail` | 97-101 | REAL-UNPROVEN | — | Accessor; no production reader outside the hw-active trail (capability.rs, A10). |
| `impl Debug` | 104-110 | REAL-UNPROVEN | — | Prints detail only — no pointer, matches ADR-001 truthful-report style. |
| `impl HwAccelerator::backend_id` | 112-115 | REAL-UNPROVEN | — | Delegates to HwBackend::VideoToolbox label ("hw.videotoolbox" per hw/mod.rs test :569). |
| `impl HwAccelerator::as_any` | 117-119 | REAL-UNPROVEN | — | `self` — the downcast seam. |
| `VideoToolboxBridge` (struct) | 125 | REAL-UNPROVEN | — | Unit struct. |
| `impl H264HwBridge::backend_id` / `open` | 127-138 | REAL-UNPROVEN | S4 | open: downcast-then-mint; `_concrete` naming (F-003); mint is an UNOPENED session — configure owns bring-up (consistent with session.rs:21-29). |

## 5. Test honesty
No tests in this file. The lane's integration coverage (`tests/h264_hw_path.rs`, 20/20 ok in baseline :1029-1050) exercises the SEAM with a fake bridge — it never touches `VideoToolboxBridge`, `discover`, or `VideoToolboxAccelerator`. Nothing in this file has ever executed. (The module docs say so honestly at :44-49; see F-002 for the compile-claim gap.)

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Host-tested (OS-neutral …): the wire forms and timestamp packaging this lane rides" | mod.rs:40-43 | REAL | feed.rs tests 6/6 ok in baseline (:557-562). |
| "Compile-verified only; runtime hardware behavior NOT exercised — [VERIFY] (cargo check --target aarch64-apple-darwin)" | mod.rs:44-49 | REAL-UNPROVEN | No committed evidence of the check (F-002); the honesty label itself is real. |
| "A no is the typed DriverIncompatible truth (VideoToolbox ships with every macOS …)" | mod.rs:9-13 | REAL-UNPROVEN | Reasoning is sound (probe semantics read-verified in session.rs:938-962); runtime behavior unverifiable. |
| "no `-sys`/bindings crate … if the Lead ever ratifies a bindings substrate, ffi is the file it replaces" | mod.rs:29-36 | REAL | Consistent with the tree (hand-rolled ffi.rs; zero new deps — crates/codecs/Cargo.toml has no Apple crates). |

## 7. Dead / duplicated / unreachable code
- The entire module: unreachable per F-001 (macOS cfg + no shipped constructor chain).
- Structural twin of vaapi/mod.rs (bridge + accelerator + discover pattern) — deliberate lane symmetry, not copy-paste drift (types and probe logic differ).

## 8. Dependencies & flags
In-workspace only: `super::capability`, `super::{H264HwBridge, H264HwDecoder}`, `codec_api::HwAccelerator`, `std::{fmt, sync::Arc}`. No unsafe in this file (the unsafe lives in ffi.rs/session.rs under the module-level allowance). No Cargo feature; OS cfg only. Wave-6 hostile question: this lane contributes zero shipped H.264 capability on every platform.

## 9. Open questions for the next agent
- ffi.rs (A05): `KCM_VIDEO_CODEC_TYPE_H264`'s value — the probe's entire truth rides on that constant being right; verify against the public header value (`'avc1'` = 0x61766331).
- capability.rs hw (A10): does the `hw-active` event trail actually read `VideoToolboxAccelerator::detail()`, or is the accessor also dead?
