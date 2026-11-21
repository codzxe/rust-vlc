# AUDIT: crates/codecs/src/video/h264/hw/videotoolbox/ffi.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-11-21 |
| Lines / functions | 366 / 51 items (20 type aliases, 9 constants, 2 `#[repr(C)]` structs + callback fn-type, 5 const ABI-pin asserts, 19 extern fns + 1 extern static across 4 framework blocks) |
| File verdict | UNREACHABLE (not compiled on the shipped Linux build at all; on macOS it feeds only the unwired hw lane) |
| Reachable from shipped entry point? | NO — the module is `#[cfg(target_os = "macos")]`-gated at its parent (hw/mod.rs:96-97 `#[cfg(target_os = "macos")] pub mod videotoolbox;`), so the baseline Linux `lumen` binary does not compile a single line of it; on macOS builds its only consumer is `session.rs`, which serves `H264Decoder` (hw lane) — constructed by no shipped host (see my slice.rs#part3 audit). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | full `Read` of the 366-line file; per-symbol usage count in session.rs via `rg -c` (all 20 extern items used ≥1×, zero dead declarations); `rg "cfg\(target_os"` in hw/mod.rs + videotoolbox/*.rs (gating map); `rg "unsafe_code"` root Cargo.toml:54 (`deny`) + crates/codecs/Cargo.toml:39-40 (inherits); `cat audit/runs/baseline/orchrun-platform-macos-crosscheck.log.txt` (the only committed darwin cross-check — covers platform-macos, NOT codecs) |

## 1. One-paragraph verdict
This is a hand-rolled VideoToolbox/CoreMedia/CoreVideo/CoreFoundation ABI binding whose every declaration I checked against Apple's public header shapes from auditor knowledge — all 19 functions, the callback record, the `CMTime` by-value struct (layout pinned by compile-time asserts that would fail loudly rather than corrupt timestamps), and all 9 constants (`0x0800_0100` UTF-8, `'avc1'`, `'420v'`, `'yuvs'`, CFNumber types 3/4, async flag `1<<0`) are correct as far as static review can establish. The file's provenance doc is unusually honest: it self-labels `[VERIFY]`, admits no SDK and no macOS hardware exist for verification. The two defects: that same doc claims "compile-verified only, via `cargo check --target aarch64-apple-darwin`" for which no committed artifact exists — the only committed darwin cross-check covers a different crate — and the module opts out of the workspace's `unsafe_code = "deny"` posture via a bare inner `#![allow]`. None of it is reachable from any shipped build today: Linux doesn't compile it, macOS compiles it into a lane nothing constructs.

## 2. What actually works (verified)
— All 19 extern declarations match Apple's documented signatures (parameter order/types checked one by one: `VTDecompressionSessionCreate`'s 6 params incl. `*const VTDecompressionOutputCallbackRecord`; `CMVideoFormatDescriptionCreateFromH264ParameterSets`' `size_t` count / `int32_t` header length; `CMSampleBufferCreateReady`'s 9 params; `CMSampleBufferSetOutputPresentationTimestamp` taking `CMTime` **by value**). — verified by read + auditor knowledge of the public headers; **no SDK in this sandbox to diff against (same limitation the file documents) — runtime behavior UNVERIFIED**.
— `CMTime` ABI pins (138-142): size 24 / align 8 / offsets 8,12,16 asserted at compile time — on any target where the layout would drift (e.g. ILP32), compilation fails instead of silently corrupting timestamps. This is the correct defensive pattern.
— `kCFBooleanTrue` correctly linked as a **data symbol** (`#[link_name]` static) rather than a function — the one constant that cannot be runtime-created.
— Zero dead declarations: every extern item has ≥1 use in session.rs (rg count, all 20 ≥ 1).

## 3. Findings
### F-001 [S3] "Compile-verified via `cargo check --target aarch64-apple-darwin`" — no committed artifact covers this crate
Where: crates/codecs/src/video/h264/hw/videotoolbox/ffi.rs:13-18
Code:
```rust
//! **No macOS SDK exists in this sandbox to diff them against**
//! (unlike the VA-API lane, whose `ffi.rs` was verified against the
//! installed libva 2.22.0 headers), and **no macOS hardware exists
//! to exercise them** — every call site below is compile-verified
//! only, via `cargo check --target aarch64-apple-darwin`:
```
What is wrong: The only committed darwin cross-check in the evidence tree is `audit/runs/baseline/orchrun-platform-macos-crosscheck.log.txt`, whose command is `cargo check -p platform-macos --target aarch64-apple-darwin --locked; then --target x86_64-apple-darwin` — it checks `platform-macos` (+ its `audio-engine`/`audio-output` deps), never `codecs`. So the ffi.rs claim of a codecs cross-check has no artifact anywhere. Two aggravations: (a) `cargo check` does not link, so even if run it would verify signature compilation against Rust types only — not that `#[link(name = "VideoToolbox", kind = "framework")]` resolves; (b) the VA-API lane's contrast claim ("verified against the installed libva 2.22.0 headers") raises the bar this file then fails to meet.
Why it matters: This file is the macOS hw lane's entire ABI surface; when Phase 7 wires the lane, the first link attempt on real macOS is the first real test. An uncommitted verification claim is exactly the "evidence string that proves nothing" pattern.
What correct looks like: An `REQUEST-ORCH-RUN`-style committed artifact: `rustup target add aarch64-apple-darwin; cargo check -p codecs --target aarch64-apple-darwin --locked` (or better, `cargo build` with an SDK-bearing runner), appended to the baseline tree; until then the doc should say "NOT compile-verified".
Confidence: HIGH — Verification: read + `cat` of the only committed crosscheck log (platform-macos only). Whether the Lead ran the codecs check ad hoc: UNVERIFIED.
### F-002 [S4] Module silently opts out of the workspace `unsafe_code = "deny"` posture with a bare inner allow
Where: crates/codecs/src/video/h264/hw/videotoolbox/ffi.rs:40-42 vs Cargo.toml:53-54
Code:
```rust
// SAFETY-SCOPED MODULE (doc 01 §2.7 / MP §9): the Apple framework
// C ABIs are the audited OS-service exception this file declares.
#![allow(unsafe_code)]
```
What is wrong: The workspace advertises `unsafe_code = "deny"` (root Cargo.toml:53-54, `[workspace.lints.rust]`), and 39/40 crates inherit it (baseline hygiene note). This module — and per the baseline's own counts, the whole h264-hw/vaapi/videotoolbox cluster plus av1 — escape with a one-line inner `#![allow]` carrying a comment but no mechanical review gate (no unsafe-inventory list, no clippy restriction re-armed inside, no #[forbid] boundary around the extern blocks). The allow is *legitimate* (an FFI declarations file is the textbook exception, and ADR-001 E2 admits it), but nothing forces the exception to stay this small: a future edit can add safe-code `unsafe {}` blocks under the same allow with zero friction.
Why it matters: Lint-posture escape hatches without a ratchet tend to grow; the workspace's "unsafe denied" claim is only as strong as its least-audited allow.
What correct looks like: Keep the allow but add a module-level `unsafe_op_in_unsafe_fn`-style ratchet or a committed unsafe-inventory (the file has exactly 5 `unsafe extern` blocks + 0 `unsafe {}` — pin that count in a test or xtask gate, the way fuzz_registry_check pins its 35 targets).
Confidence: HIGH — Verification: read of both files; baseline hygiene counts (INDEX.md: `unsafe {` 316, concentrated in platform-FFI + h264 hw + av1).

## 4. Per-function table
| Item group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| 20 type aliases (CF*/CM*/CV*/VT* handles, `CFIndex=isize`, `Boolean=u8`, `OSStatus=i32`, `FourCharCode=u32`) | 49-91 | REAL (unreachable) | — | all opaque `c_void` pointers, never dereferenced; types match Apple's |
| 9 constants | 95-117 | REAL (unreachable) | — | UTF-8 `0x0800_0100`, CFNumber 3/4, `'avc1'` `0x61766331`, `'420v'` `0x34323076`, `'yuvs'` `0x79757673`, flags — all correct values |
| `CMTime` struct + 5 ABI-pin asserts | 124-142 | REAL (unreachable) | — | `#[repr(C)]`, value/timescale/flags/epoch order matches; compile-time size/align/offset pins fail loudly on drift |
| `VTDecompressionOutputCallback` fn type | 152-163 | REAL (unreachable) | — | 7 params in C order; `Option<fn>` null semantics correct |
| `VTDecompressionOutputCallbackRecord` + size pin | 166-176 | REAL (unreachable) | — | 16 bytes pinned; callback+refcon order matches |
| CoreFoundation block: `CFRelease`, `CFStringCreateWithCString`, `CFNumberCreate`, `CFDictionaryCreateMutable`, `CFDictionarySetValue`, static `kCFBooleanTrue` | 184-225 | REAL (unreachable) | — | signatures correct; the boolean as data-symbol link is the right call |
| CoreVideo block: `CVPixelBufferRetain/Release/GetWidth/GetHeight` | 233-243 | REAL (unreachable) | — | correct; GetWidth/Height return `usize` (`size_t`) |
| CoreMedia block: `CMVideoFormatDescriptionCreateFromH264ParameterSets`, `CMBlockBufferCreateWithMemoryBlock`, `CMBlockBufferReplaceDataBytes`, `CMSampleBufferCreateReady`, `CMSampleBufferSetOutputPresentationTimestamp` | 252-316 | REAL (unreachable) | — | all parameter lists match the documented C ABI (checked individually); CMTime by value on the timestamp setter |
| VideoToolbox block: `VTIsHardwareDecodeSupported`, `VTDecompressionSessionCreate/DecodeFrame/WaitForAsynchronousFrames/Invalidate` | 325-366 | REAL (unreachable) | — | 6/5/1/1 params correct; `Boolean` return on the probe; `Invalidate` returns void |

## 5. Test honesty
No tests exist in or for this file (it is declarations only; the baseline suites never execute it — it is not compiled on Linux). The file's own `[VERIFY]` labeling is the honest substitute. Nothing here proves the ABI; nothing claims to (except the compile-check claim of F-001, which has no artifact).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every call site below is compile-verified only, via `cargo check --target aarch64-apple-darwin`" | ffi.rs:16-17 | REAL-UNPROVEN (no committed artifact) | F-001: the only committed darwin crosscheck covers platform-macos, not codecs |
| "No macOS SDK exists in this sandbox to diff them against … runtime behavior NOT exercised — `[VERIFY]`" | ffi.rs:13-18 | REAL (honest) | this audit faced the same limitation; static review found no ABI defect |
| (hw lane claims — see h264/hw/mod.rs, other agents) hardware-first H.264 with typed refusals | ADR-001 | REAL for the refusal posture; UNREACHABLE for the decode | hw lane constructed by no shipped host (my slice.rs#part3 audit) |

## 7. Dead / duplicated / unreachable code
- The entire file is unreachable from shipped builds (macOS-gated + unwired lane) — but on macOS it is *live-compiled*, so ABI correctness still matters the day the lane wires up.
- No dead declarations (all 20 extern items used by session.rs — verified by count).
- Duplication note: this is the repo's second hand-rolled OS-FFI surface (vaapi being the first); the pattern (opaque handles, SAFETY comments, Create-rule docs) is consistent across both.

## 8. Dependencies & flags
- Platform gate: `#[cfg(target_os = "macos")]` at the parent module (hw/mod.rs:96-97) — the legitimate platform-gate form (contrast fake feature gates; this one is real and correctly scoped).
- Lint posture: workspace `unsafe_code = "deny"` (root Cargo.toml:53-54) vs this module's inner allow (F-002).
- Wave-6 hostile question: this file has no bearing on H.264 sw reachability (it is the opposite lane — hardware, macOS-only, unwired). Full trace in mjpeg/decoder.rs#part2.audit.md §8: sw decode IS reachable (default features, pipeline_host.rs:346); this hw FFI is not compiled on the shipped Linux build at all.

## 9. Open questions for the next agent
- session.rs owner (W06 other agent): the refcon lifetime invariant (`Arc<Shared>` as raw pointer) and the Create-rule ownership (which handles `CFRelease` covers — 18 call sites) — audit the drop order against `VTDecompressionSessionInvalidate`'s "handle is dead afterwards" contract.
- Whoever wires Phase 7: run the codecs darwin cross-check and commit it (F-001), and consider an on-device smoke (even `VTIsHardwareDecodeSupported('avc1')` returning 0/1) before trusting the lane.
- vaapi/ffi.rs owner: the "verified against installed libva 2.22.0 headers" claim — confirm a committed diff artifact exists, or it has the same F-001 shape.
