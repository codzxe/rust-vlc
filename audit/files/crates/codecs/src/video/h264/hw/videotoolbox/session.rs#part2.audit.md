# AUDIT: crates/codecs/src/video/h264/hw/videotoolbox/session.rs#part2 (lines 481-962)
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-21 |
| Lines / functions | 482 of 962 (chunk part2; boundary: struct field `emitted` :481 → EOF :962; part1/A08 = :1-480 module docs, pure helpers, CF helpers, callback channel, struct head) / 12 items |
| File verdict | UNREACHABLE (macOS-cfg'd code, not even compiled on the baseline Linux host; and no shipped entry point reaches the hw tree) |
| Reachable from shipped entry point? | NO — (a) `#[cfg(target_os = "macos")]` (hw/mod.rs:96-97) excludes the whole lane from every Linux build incl. all baseline runs; (b) even on macOS the only consumer chain is `installed_bridges → VideoToolboxBridge::open → VideoToolboxSession` behind `H264Decoder::from_setting`, which no shipped host calls (see my hw/mod.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | Read of full file (both parts for context); Read of frame_pool.rs (pool semantics for the rollback/teardown safety claims); rg "release\|emitted\|pool_exhausted" decode.rs + vaapi/session.rs (cross-lane comparison); rg "display_size" session.rs (dead-field proof); rg cfg gates hw/mod.rs:94-97; baseline logs searched for any videotoolbox/darwin evidence (none) |

## 1. One-paragraph verdict
Part2 is the session engine: `new`/`teardown`/`drain`, the `H264HwDecoder` impl (`configure`/`decode`/`flush`), and the `probe`. Read as Rust, the resource accounting is careful and I traced every Create-rule handle through every error path without finding a leak or double free — the drain rollback's exactly-once release logic is correct against `frame_pool.rs`'s actual semantics (acquire-Err retains nothing), and the `Arc::as_ptr` refcon invariant (heap-stable across moves; `Drop::drop` invalidates the VT session before the `Arc` field drops) is sound. But none of this has ever executed, anywhere: the module is compiled only for macOS, the sandbox is Linux, the sole claimed verification is an uncited `cargo check --target aarch64-apple-darwin`, and even a macOS build would reach this code only through the hw tree no shipped binary calls. Layered on top: the lane is by construction a ~10-frame decoder (no present-side surface release exists anywhere in the repo, so the pool exhausts and every later decode is a typed error), and it reports coded dimensions where the trait contract demands display dimensions — a field it stores (`display_size`) and then never reads.

## 2. What actually works (verified)
— Nothing executed. Structural verification only (read):
— Rollback correctness (drain :592-611): on `pool.acquire` Err, acquired handles release buffer-then-slot (`pool.resource` + `release_buffer` + `pool.release`), the never-acquired tail (incl. the failing frame) releases its queue retains — verified against frame_pool.rs:142-162 (Err path stores nothing) and :186-198 (release drops the resource without a destructor, so the manual `release_buffer` before `pool.release` is the exactly-once order). No double release, no leak.
— teardown (:531-570): every retain accounted — session invalidate+release, format description release, queued callback frames released, emitted handles' buffers released + slots freed, pool drained (generation bump makes stale handles unresolvable). Leak-free walk.
— decode (:764-906) error paths: block released at :834 (ReplaceDataBytes failure) and :862 (after CreateReady retains it); sample released at :876 (PTS failure) and :898 (after DecodeFrame); spec/attrs released at :742-745; format_description released at :701/:711/:748. All Create-rule transitions accounted.
— `Arc::as_ptr(&self.shared)` refcon (:721): the Arc target is heap-stable across moves of the session struct; `Drop for VideoToolboxSession` (:639-643) runs `teardown` (Invalidate documented synchronous with the last callback) before any field drops — the :399-405 invariant holds structurally.

## 3. Findings

### F-001 [S3] The lane is a ≤ pool_capacity-frame decoder by construction: no present-side release hook exists, so the pool exhausts and stays exhausted
Where: crates/codecs/src/video/h264/hw/videotoolbox/session.rs:577-630 (drain), 481-483 (emitted field); cross-file frame_pool.rs:26-31, decode.rs:466
Code:
```
    /// Handles handed to the codec layer (the present side's
    /// release hook is later-wave work; teardown walks this list).
    emitted: Vec<u64>,
```
```
            self.pool.emit(handle);
            self.emitted.push(handle);
```
What is wrong: Every decoded frame's handle is pushed to `self.emitted` and never returned. `H264HwDecoder` (hw/mod.rs:278-307) has no release-surface method; decode.rs wraps the handle into `FrameStorage::OpaqueHandle(frame.surface)` (:466) and nothing in the repo ever calls `pool.release` on an emitted handle outside teardown/rollback. `pool_capacity` (session.rs:151-153) = `max_num_ref_frames + 6` clamped to [4, 64] — typically ~10. After the first ~10 pictures, every `drain` hits `PoolExhausted`, rolls back, and returns `h264.hw.videotoolbox.pool_exhausted` for **every subsequent decode call of the stream**.
Why it matters: A hardware lane that can emit ~10 frames total is not a decoder, it is a stub with excellent manners. frame_pool.rs:26-31 documents this honestly ("That side of the contract is the 7.B/R5 renderer follow-up [PROPOSAL]; until it lands, a lane that exhausts its pool reports the typed error … honest back-pressure, not a leak-and-limp") — but the module-level claims ("the decode pipeline (one picture)", session shape) read as a working lane, and any future wiring of the hw path ships a 10-frame player. The VA-API lane has the identical gap (session.rs:648-653).
What correct looks like: A `release_surface(handle: u64)` on the `H264HwDecoder` trait (or a session-external registry) + the codec layer returning handles after frame consumption; until then, the module docs' "decode pipeline" diagram should state the frame-count ceiling in bold.
Confidence: HIGH — Verification: read; pool semantics verified against frame_pool.rs (its own test `acquire_emit_release_lifecycle_is_bounded_and_typed` pins the exhaust behavior); rg confirms no release caller exists.
### F-002 [S3] `H264HwFrame.size` carries coded dimensions, violating the trait's display-size contract; `display_size` is stored and never read
Where: crates/codecs/src/video/h264/hw/videotoolbox/session.rs:614-627 (drain), 757 (configure), 486-488 (field); contract at hw/mod.rs:217-227
Code:
```
            out.frames.push(H264HwFrame {
                surface: handle,
                size: frame.size,
```
```
        self.display_size = stream.display_size;
```
What is wrong: `H264HwFrame.size` is documented as "Display `(width, height)` of the decoded picture" (hw/mod.rs:221-222). `frame.size` comes from `CVPixelBufferGetWidth/Height` in the callback (part1 :428-433) — the pixel buffer is created against `destination_attributes(width, height)` where width/height are the **coded** (macroblock-aligned) dims (session.rs:316-319, 668-670: "display cropping is the present side's business, carried separately"). So a 1920×1080 stream coded as 1920×1088 reports 1920×1088. Meanwhile `self.display_size` — the cropped size configure carefully stores at :757 — is never read anywhere (rg: field decl :486-488, teardown clear :569, configure set :757 — zero read sites). The VA-API lane does the opposite and correct thing: `size: self.display_size.unwrap_or(self.coded)` (vaapi/session.rs:655).
Why it matters: Contract drift between the two lanes of the same trait; downstream consumers scaling on `H264HwFrame.size` get the wrong geometry for every cropped stream (extremely common — 1088-coded 1080-display). Dead field + lying dimension in one.
What correct looks like: `size: self.display_size.unwrap_or(frame.size)` in drain (mirroring vaapi), or delete `display_size` and fix the trait doc to say "coded".
Confidence: HIGH — Verification: read; all three display_size sites listed; vaapi contrast read; contract text read.
### F-003 [S3] The file's only verification claim (`cargo check --target aarch64-apple-darwin`) is uncited and uncorroborated; the file is not even compiled by any baseline run
Where: crates/codecs/src/video/h264/hw/videotoolbox/session.rs:5-17 (module docs, part1 boundary — but the claim covers this part's code); hw/mod.rs:94-97
Code:
```
//! This sandbox is a Linux host with **no macOS SDK and no Apple
//! hardware**: every OS call below is **compile-verified only**, via
//! `cargo check --target aarch64-apple-darwin` — **runtime hardware
//! behavior NOT exercised — `[VERIFY]`**
```
Cross-file (hw/mod.rs:94-97):
```
#[cfg(target_os = "linux")]
pub mod vaapi;
#[cfg(target_os = "macos")]
pub mod videotoolbox;
```
What is wrong: On the Linux baseline host this file is syntax the compiler never touches: `cargo build/test/clippy` (baseline #1-#6) prove nothing about it, and no committed log of the aarch64-apple-darwin check exists in audit/runs/baseline/ (INDEX.md has no cross-compile run). The claim "compile-verified" is therefore UNVERIFIED — 480+ lines of `unsafe` FFI orchestration whose only quality evidence is an assertion in a doc comment. If the check never ran (or ran against a stale revision), a typo'd ffi call would sit undetected until someone builds for macOS.
Why it matters: Every SAFETY comment in this part reasons from ffi declarations that nothing has type-checked in any recorded run. The `[VERIFY]` labels are honest, but honesty about missing evidence does not create evidence.
What correct looks like: A committed CI lane (or a recorded orchestrator run) of `cargo check -p codecs --target aarch64-apple-darwin` with its log in the repo; until then the docs should say "compile-verification UNVERIFIED".
Confidence: HIGH for the absence of evidence (baseline INDEX.md enumerates all runs; none is a darwin check); MEDIUM that the check never happened at all (a developer machine could have run it — unverifiable).
REQUEST-ORCH-RUN: `cargo check -p codecs --target aarch64-apple-darwin` (needs the darwin std component; if unavailable, record the gap).
### F-004 [S4] `destination_attributes` failure reuses the `specification` error code
Where: crates/codecs/src/video/h264/hw/videotoolbox/session.rs:706-716
Code:
```
        let Some(attributes) = destination_attributes(width, height) else {
            // SAFETY: Create-rule handles, sole owner (the spec
            // dictionary was not passed to any framework call yet).
            unsafe {
                ffi::CFRelease(spec.cast());
                ffi::CFRelease(format_description.cast());
            }
            return Err(H264HwError::Configure {
                code: "h264.hw.videotoolbox.specification",
            });
```
What is wrong: Decoder-spec creation failure (:702-704) and destination-attributes failure (:713-715) share the rule id `h264.hw.videotoolbox.specification`. Two distinct bring-up steps are indistinguishable in any error report.
Why it matters: Typed-error discipline is this lane's entire selling point; a maintainer triaging a configure failure cannot tell which dictionary construction failed. Blast radius: diagnostics only.
What correct looks like: `code: "h264.hw.videotoolbox.destination_attributes"`.
Confidence: HIGH — Verification: read both sites.
### F-005 [S4] `unsafe impl Send` cites undocumented threading behavior of VT session handles
Where: crates/codecs/src/video/h264/hw/videotoolbox/session.rs:498-505
Code:
```
// SAFETY: the non-Send members are the opaque framework handles
// (format description, session, retained pixel buffers) — each is
// owned exclusively by this session, created and destroyed by it
// alone, and only ever passed back to framework calls (single
// owner, no aliasing). VideoToolbox session handles are documented
// valid across threads for the owning client; moving the session
// to another thread races with nothing.
unsafe impl Send for VideoToolboxSession {}
```
What is wrong: "VideoToolbox session handles are documented valid across threads for the owning client" — no document is cited, and Apple's actual guidance is that a VTDecompressionSession must not be *used* concurrently from multiple threads (move-safety follows from CF refcounting, which the comment's first half already argues correctly). The claim as written overstates what is documented.
Why it matters: SAFETY comments are the audit surface for every unsafe impl; an overstated citation invites future concurrent-use mistakes (a reader may infer Sync-ness).
What correct looks like: "CF objects are refcounted and safe to move; VT sessions are NOT safe for concurrent use — this impl grants move-only transfer, and &mut self serialization is the trait's contract."
Confidence: MEDIUM — Verification: read; Apple doc text not available in sandbox (UNVERIFIED citation), but the internal reasoning stands without it.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| struct fields (emitted/display_size/sps/pps/sample) | 481-496 | REAL-UNPROVEN | S3 | display_size dead (F-002); sample = reusable scratch (zero steady-state alloc claim rides feed.rs). |
| `unsafe impl Send for VideoToolboxSession` | 498-505 | REAL-UNPROVEN | S4 | Overstated doc citation (F-005); move-safety itself sound (CF refcounting). |
| `VideoToolboxSession::new` | 507-523 | REAL-UNPROVEN | — | Unopened mint; zero-capacity pool safe because decode gates on session null (:765). |
| `teardown` | 531-570 | REAL-UNPROVEN | — | Leak-free walk verified (see §2); poison-tolerant lock matches the callback's panic-free posture. |
| `drain` | 577-630 | REAL-UNPROVEN | S3 | Rollback exactly-once logic verified against frame_pool semantics; but emits into a pool nobody ever releases → permanent pool_exhausted after ~capacity frames (F-001). |
| `impl Default` | 633-637 | REAL-UNPROVEN | — | Forwards to new. |
| `impl Drop` | 639-643 | REAL-UNPROVEN | — | teardown ordering upholds the refcon invariant. |
| `H264HwDecoder::backend_id` | 646-648 | REAL-UNPROVEN | — | Delegates to HwBackend label. |
| `H264HwDecoder::configure` | 650-762 | REAL-UNPROVEN | S4 | 6-step bring-up; parameter-set pick, typed refusals (field picture/profile/geometry), format description off raw NALs with AVCC length size; error-code collision (F-004); display_size stored-never-read (F-002); handles released on every failure path (verified). |
| `H264HwDecoder::decode` | 764-906 | REAL-UNPROVEN | S3 | not_configured gate, MVC scan, field-picture re-check, AVCC sample into scratch (typed u32 refusal), block buffer copy, sample buffer + PTS, async submit, drain. Error-path resource accounting verified; pool-exhaustion ceiling (F-001). |
| `H264HwDecoder::flush` | 908-923 | REAL-UNPROVEN | — | Unopened → Ok(empty) (honest); WaitForAsynchronousFrames then drain. |
| `probe` | 938-962 | REAL-UNPROVEN | — | Single capability query; false → typed DriverIncompatible with honest reasoning (VT present on all macOS). Compile-verified only (F-003). |

## 5. Test honesty
No tests exist in this file — deliberately and honestly: the module docs (:15-17) state a `#[cfg(test)]` module here would never compile in a Linux gate, so the host-testable halves (AVCC wire form, CMTime round trip) live in `feed.rs`, which IS host-tested (baseline :557-559 `video::h264::hw::feed::tests::*` ok). Consequence: every behavior in part2 — including all the resource-accounting I verified by reading — is execution-unverified. Honest shape, but the file's correctness rests entirely on code review.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every OS call below is compile-verified only, via cargo check --target aarch64-apple-darwin" | session.rs:8-10 | REAL-UNPROVEN | No committed log of that check exists (baseline INDEX.md enumerates runs; none is darwin). UNVERIFIED. |
| "surfaced frames beyond the pool cap → pool_exhausted — bounded back-pressure … never an unbounded retain" | session.rs:67 (module docs) | REAL | Read-verified; but the doc understates that the cap is hit after ~10 frames and NEVER recovers (F-001). |
| "A re-configure tears the session down and rebuilds it" | session.rs:28-29 | REAL-UNPROVEN | teardown-then-rebuild verified by reading (:672 + :753-760). |
| "display cropping is the present side's business, carried separately" | session.rs:135-138 | SHALLOW | "Carried separately" = stored at :757 and never read (F-002). |
| feature-matrix hw row "compile-verified … runtime NOT exercised [VERIFY]" (per mod.rs:44-49) | docs/feature-matrix.toml | REAL-UNPROVEN | Consistent with the module docs; same missing evidence (F-003). |

## 7. Dead / duplicated / unreachable code
- Unreachable twice over: cfg(macos) (not compiled on baseline host) + hw-tree consumer chain has no shipped caller (hw/mod.rs audit F-001).
- `display_size` field: dead (F-002).
- Near-duplicate of the VA-API session's structure (pool acquire/emit/release dance, pool_exhausted rollback) — the shared frame_pool.rs is the deliberate dedup; the lane-specific duplication is the FFI orchestration, inherent to two different OS APIs.

## 8. Dependencies & flags
`std` + in-crate ffi/parser/frame_pool/feed + codec-api/media-types. Module-level `#![allow(unsafe_code)]` (session.rs:79, ADR-001 §5 audited exception). Every unsafe block carries a SAFETY comment — read-verified present, semantically UNVERIFIABLE without the macOS SDK. No Cargo feature; OS cfg only. Wave-6 hostile question: this lane contributes nothing to the shipped H.264 capability of `lumen`/`lumen-tauri` on any platform today.

## 9. Open questions for the next agent
- ffi.rs (A05's file): are the `CMBlockBufferCreateWithMemoryBlock` / `CMSampleBufferCreateReady` signatures (9/10 params) correct per the real headers? A wrong arity would compile against wrong decls and corrupt the stack at first macOS run — the compile-verification claim (F-003) is only as good as ffi.rs.
- frame_pool.rs (A03): confirm `release` intentionally drops the resource without running any lane cleanup (the lanes must pre-release — session.rs does, vaapi relies on surface-ID semantics where dropping is fine).
- REQUEST-ORCH-RUN: `cargo check -p codecs --target aarch64-apple-darwin` — closes F-003's evidence gap for both videotoolbox files.
