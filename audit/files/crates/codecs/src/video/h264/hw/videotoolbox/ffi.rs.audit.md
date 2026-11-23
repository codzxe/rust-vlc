# AUDIT: crates/codecs/src/video/h264/hw/videotoolbox/ffi.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 366 / 20 extern decls + 2 repr(C) structs + 11 type aliases + 9 constants |
| File verdict | REAL-UNPROVEN (declarations reviewed line-by-line against Apple's public header ABI from knowledge: no defect found; runtime never exercised — honestly labeled `[VERIFY]`; compile-verification claim not reproducible in this sandbox) |
| Reachable from shipped entry point? | NO — `#[cfg(target_os = "macos")] pub mod videotoolbox` (hw/mod.rs:96-97); the Linux release lane never compiles it, and even on macOS the only wiring (`H264Decoder::from_setting`) is test-only (see my h264/hw/mod.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rustup/ls target inspection (no apple target installed); rg worklog for the cross-check claim; ABI review from knowledge (below) |

## 1. One-paragraph verdict
A careful, honestly-labeled hand-rolled FFI surface. I diffed every declaration against Apple's public framework signatures from my own knowledge (no SDK exists in this sandbox — the file says the same): all 20 extern functions (CoreFoundation 5+1 static, CoreVideo 4, CoreMedia 5, VideoToolbox 5), the `CMTime` struct (value:i64@0, timescale:i32@8, flags:u32@12, epoch:i64@16 = 24 bytes/8-align — matches Apple exactly and is compile-time pinned by five `const _: () = assert!` lines 138-142), the callback record (16 bytes, `Option<fn>` niche ✓), and the 9 constants (noErr, UTF8 encoding 0x08000100, SInt32/SInt64 CFNumber types 3/4, 'avc1' 0x61766331, '420v' 0x34323076, 'yuvs' 0x79757673, kCMTimeFlags_Valid 1<<0, kVTDecodeFrame_EnableAsynchronousDecompression 1<<0) — all correct. The provenance header tells the truth: runtime behavior NOT exercised, `[VERIFY]`. The two findings are environmental/cosmetic: the module is unreachable in every shipped build, and one pointer-declaration mutability nit.

## 2. What actually works (verified)
- ABI review (knowledge-diff, cited per item in §1): zero mismatches found in 20 signatures + 2 structs + 9 constants. Notable correctness: `CMVideoFormatDescriptionCreateFromH264ParameterSets` (size_t count / const-uint8_t* array / size_t sizes / int32_t nalUnitHeaderLength — matches Apple), `CMSampleBufferSetOutputPresentationTimestamp` takes CMTime **by value** (matches), `VTDecompressionSessionCreate`'s `const VTDecompressionOutputCallbackRecord *` (matches `*const`).
- The compile-time layout pins (lines 138-142, 176) enforce the CMTime/callback-record ABI at every future compile of the macOS target — real defensive engineering.
- Honest provenance: "**No macOS SDK exists in this sandbox to diff them against** ... **runtime behavior NOT exercised — `[VERIFY]`**" (lines 13-18).

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/video/h264/hw/videotoolbox/ffi.rs:17 (claim), crates/codecs/src/video/h264/hw/mod.rs:96-97 (gate)
Code:
```
//! only, via `cargo check --target aarch64-apple-darwin`:
//! **runtime behavior NOT exercised — `[VERIFY]`**.
```
What is wrong: two-fold. (a) The compile-verification claim is not reproducible from this sandbox's current state: no `aarch64-apple-darwin` rustlib target is installed (only x86_64-linux and wasm32), and my worklog search found no recorded output of that `cargo check` — UNVERIFIED that it ever ran. (b) Even compiled, the lane is unreachable: the module is cfg-gated to macOS while the shipped release lane is Linux, and the only consumer chain (`H264Decoder::from_setting` → `VideoToolboxBridge`) has no shipped call site (h264/hw/mod.rs audit F-001). The `[VERIFY]` label keeps this honest — it is dead-until-7.B code, not a false capability claim. Confidence: HIGH (unreachability, verified by rg/ls) / the cargo-check run itself UNVERIFIED. — Verification: read + ran ls/rg.

F-002 [S4]
Where: crates/codecs/src/video/h264/hw/videotoolbox/ffi.rs:160-161
Code:
```
        presentationTimeStamp: *mut CMTime,
        presentationDuration: *mut CMTime,
```
What is wrong: Apple documents these callback parameters as `CMTime *` (in/out in ancient docs) but the canonical modern signature passes `const CMTime *` — read-only. ABI-identical for a correct caller (the session only reads), but `*mut` grants write permission the ABI does not intend and invites a future UB bug if someone "normalizes" the timestamp in place. Cosmetic-to-hygiene. Confidence: MEDIUM — Verification: knowledge-diff (no SDK in sandbox).

## 4. Per-function table
| Declaration group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CF/CM/VT/CV type aliases (11) | 49-91 | REAL | — | opaque `c_void` handles; CFIndex=isize, OSStatus=i32, Boolean=u8 — all correct |
| constants (9) | 95-117 | REAL | — | all values verified vs Apple (§1) |
| `CMTime` struct + 5 layout pins | 124-142 | REAL | — | exact Apple layout, compile-time pinned |
| `VTDecompressionOutputCallback` type | 152-163 | REAL | S4 | F-002 mutability nit |
| `VTDecompressionOutputCallbackRecord` + pin | 166-176 | REAL | — | 16 bytes ✓ |
| CoreFoundation extern block (6) | 184-225 | REAL | — | incl. `kCFBooleanTrue` static link (correct — singletons have no create fn) |
| CoreVideo extern block (4) | 233-243 | REAL | — | GetWidth/GetHeight return usize ✓ |
| CoreMedia extern block (5) | 252-316 | REAL | — | all five signatures match Apple |
| VideoToolbox extern block (5) | 325-366 | REAL | — | session lifecycle + probe; match Apple |

## 5. Test honesty
No tests in this file (declaration-only). The session module's host-tested "pure halves" (AVCC sample form, CMTime nanosecond round trip — hw/mod.rs docs lines 48-50) live in `feed`/`session` (other agents' files; baseline `video::h264::hw::feed::tests::*` green).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "declarations written from Apple's public framework header documentation" | ffi.rs:8-12 | CONSISTENT (my knowledge-diff found no contradiction) | §2 |
| "No macOS SDK exists in this sandbox ... no macOS hardware ... compile-verified only ... runtime NOT exercised — [VERIFY]" | ffi.rs:13-18 | TRUE for runtime; the compile-check claim itself UNVERIFIED (no apple target installed now, no worklog output found) | F-001 |
| "runtime-created CFStrings compare equal to the constants (CFString equality is by contents)" | ffi.rs:31-35 | TRUE | CFString equality is content-based (CFEqual), a documented CF property |
| "`CFDictionaryCreateMutable` with NULL callbacks selects the documented kCFType…CallBacks" | ffi.rs:36-38 | TRUE | documented CoreFoundation default |

## 7. Dead / duplicated / unreachable code
Whole module unreachable in shipped builds (F-001). Structural sibling: h264/hw/vaapi/ffi.rs (that one was header-diff-verified per its own docs; other agent).

## 8. Dependencies & flags
`#![allow(unsafe_code)]` scoped to this file (second sanctioned exception pattern, like av1). Framework links: CoreFoundation/CoreVideo/CoreMedia/VideoToolbox — macOS-only, cfg-gated upstream. No cargo features.

## 9. Open questions for the next agent
- Find the recorded `cargo check --target aarch64-apple-darwin` output (worklog/CI?) — if none exists, the "compile-verified" provenance line is itself unbacked and should be softened.
- When 7.B1 lands: the `*mut CMTime` callback params (F-002) and the refcon lifetime invariant (session.rs) deserve a dedicated unsafe-audit pass.
