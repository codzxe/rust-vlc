# AUDIT: crates/codecs/src/video/h264/hw/vaapi/ffi.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | 608 / 5 fns + 2 pack modules + 2 impls + 21 type aliases + 30 consts |
| File verdict | REAL (constants executed against the installed libva.so.2.2200.0 via its own string mappers — all exact) |
| Reachable from shipped entry point? | COMPILED-YES / RUNTIME-DEFAULT-OFF. cfg(target_os = "linux") (hw/mod.rs:94), no cargo feature gate; compiled into the shipped Linux release build. The call path (library.rs dlsym → session.rs) executes only when hw-decode is enabled; hw/mod.rs:520 documents `discover_lane` is "never called under the OFF default". |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | python3 ctypes dlopen of /usr/lib/x86_64-linux-gnu/libva.so.2 + libva-drm.so.2 (vaErrorStr/vaEntrypointStr/vaProfileStr/vaConfigAttribTypeStr/vaBufferTypeStr sweeps, symbol existence); rg constant-consumer trace; sed quote checks |

## 1. One-paragraph verdict
This is a hand-rolled but demonstrably faithful VA-API ABI mirror, and I could verify it against ground truth that is stronger than usual: the host has exactly the libva the module claims (libva.so.2.2200.0, `dpkg -l` → libva2 2.22.0-3), and libva exports string mappers for its enums, so I executed every checkable constant against the real library — VA_STATUS_* (6/6), VAProfile (4/4), VAEntrypointVLD, VAConfigAttribType (3/3), VABufferType (4/4) are all EXACT, including the three r3-corrected values the module brags about. All 20 dlsym target symbols (19 va* + vaGetDisplayDRM in libva-drm) exist. The struct size pins (36/8/672/240/3128) are compile-verified in the shipped Linux build, and my independent C-layout recomputation reproduces all three H.264 buffer sizes including the documented 3-pad-byte detail. The defects are documentation-level: a doc comment claims DECODING_ERROR is "mapped to the typed Decode channel" when the session discards the status entirely, and the module cites the wrong decision card (7.B1 = VideoToolbox per hw/mod.rs; VA-API is 7.B3).

## 2. What actually works (verified)
- python3 ctypes on libva.so.2: `vaErrorStr(3)` → 'invalid VADisplay', `(12)` → 'the requested VAProfile is not supported', `(20)` → 'the requested function is not implemented', `(23)` → 'internal decoding error' — every claimed VA_STATUS constant exact (0, 1, 3, 12, 20, 23).
- `vaEntrypointStr(1)` → 'VAEntrypointVLD' (the claimed `VA_ENTRYPOINT_VLD = 1` is exact — NOT 0).
- `vaProfileStr(-1/6/7/13)` → None/H264Main/H264High/H264ConstrainedBaseline — exact.
- `vaConfigAttribTypeStr(0/18/19)` → RTFormat/MaxPictureWidth/MaxPictureHeight — exact.
- `vaBufferTypeStr(0/1/4/5)` → PictureParameter/IQMatrix/SliceParameter/SliceData — exact.
- Symbol existence: all 19 va* targets exported by libva.so.2; `vaGetDisplayDRM` exported by libva-drm.so.2 — "missing: NONE".
- Size pins: baseline release build of codecs (audit/runs/baseline/cargo-build-release-lumen-cli.txt, exit 0) compiles the `const _: () = assert!(size_of ...)` pins at ffi.rs:164-165, 606-608 — they hold.
- Manual C-layout recomputation: VAPictureH264=36, VAPictureParameterBufferH264=672 (36+576+2+2+1+1+1+pad+4+1+1+2+1+1+1+1+4+2+pad+32), VAIQMatrixBufferH264=240 (96+128+16), VASliceParameterBufferH264=3128 (12+2+2+9 bytes→offset 25, +3 pad→28, +2×1152 ref lists, weights block → 3110, +2 pad, +16 reserved) — all match the pins; the r3 note's "offset 25 rounds to 28" is arithmetically exact.

## 3. Findings
F-001 [S3]
Where: ffi.rs:78-81 (doc) vs session.rs:1409-1416
Code:
```
/// `VA_STATUS_ERROR_DECODING_ERROR` (0x17) — the driver's bitstream
/// damage verdict (mapped to the typed Decode channel by the
/// session's `va_op_error`).
pub const VA_STATUS_ERROR_DECODING_ERROR: VAStatus = 23;
```
What is wrong: no such mapping exists. `va_op_error` is:
```
fn va_op_error(code: &'static str, status: ffi::VAStatus) -> H264HwError {
    // The spec spelling is recoverable in a debug session; the
    // stable code names the op. [VERIFY: structured status payload
    // when the reporting seam grows one.]
    let _ = va_status_text(status);
    H264HwError::Decode { code }
}
```
The status (including DECODING_ERROR) is computed into a String and discarded; every non-success status — allocation failure, invalid parameter, decoding error — produces the same op-code error with no status payload. The value 23 itself is correct (verified), but the doc promises semantics the session does not implement. Why it matters: a caller cannot distinguish "driver says the bitstream is damaged" (recoverable, drop frame) from "invalid parameter" (bug, fatal) — exactly the distinction the doc claims exists. What correct looks like: branch on VA_STATUS_ERROR_DECODING_ERROR in va_op_error to select the Decode vs Configure channel or attach `va_status_text` to the error detail. Confidence: HIGH — Verification: read (session.rs:1409-1421) + rg (zero non-ffi consumers of any VA_STATUS_ERROR_* constant).
Related dead code: all five VA_STATUS_ERROR_* constants and VA_PROFILE_NONE are never branched on anywhere — they exist only as `va_status_text` match arms, and that function's result is discarded on both error paths (session.rs:1414, 1420).

F-002 [S4]
Where: ffi.rs:1
Code:
```
//! VA-API C-ABI declarations (card 7.B1) — hand-rolled, no `-sys`
//! crate (AGENTS.md rule 2: zero new dependencies).
```
What is wrong: hw/mod.rs's truth table assigns card 7.B1 to VideoToolbox and 7.B3 to VA-API ("card numbering per doc 13 §10: 7.B1 VideoToolbox, 7.B2 Media Foundation, 7.B3 VA-API" — hw/mod.rs:37-38). ffi.rs (and session.rs:1424 "card 7.B1's typed negotiation entry") cite 7.B1 for the VA-API lane. Cross-reference error only. Confidence: HIGH — Verification: read both files.

F-003 [S4]
Where: ffi.rs:150-165 vs the real header
What is wrong: nothing demonstrated — but the struct FIELD ORDER/bitfield pack orders (seq_fields, pic_fields LSB-first bit orders; `slice_data_bit_offset: u16`) are unverifiable on this host: libva-dev headers are not installed (no /usr/include/va), and libva exports no layout-reflecting symbol. Sizes are pinned and my recomputation matches, which constrains but does not prove field order. UNVERIFIED: field-order fidelity (blocked: libva-dev absent; cargo-build-budget irrelevant — no headers exist to compile against). Confidence in the layouts: MEDIUM-HIGH (sizes + pad arithmetic + pack-order doc comments are all internally consistent with the known va_dec_vp.h shape).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| va_status_text | 84-95 | REAL | — | pure; 6 arms + hex fallback; result discarded by both error mappers (F-001) |
| VAPictureH264::empty | 315-328 | REAL-UNPROVEN | — | consumed by session.rs (rg:1 hit); invalid-id + zero counts + zero reserved |
| VAIQMatrixBufferH264::flat_default | 478-488 | REAL-UNPROVEN | — | all-16 flat lists; §7.3.2.1.1 default claim consistent; scaling-list streams refused typed by session per doc |
| VASliceParameterBufferH264::empty | 561-600 | REAL-UNPROVEN | — | neutral weights, invalid ref lists |
| seq_fields::pack | 388-417 | REAL-UNPROVEN | — | 11-field LSB-first packing; masks prevent bit bleed; order matches documented spec order (header diff UNVERIFIED) |
| pic_fields::pack | 425-452 | REAL-UNPROVEN | — | 10-field LSB-first; same note |
| (type aliases, 21) | 172-281 | REAL | — | all 20 dlsym target symbols verified to exist; `VaCreateSurfacesFn` attrib_list declared `*mut c_void` for the null pass — ABI-equivalent, documented |
| (consts, 30) | 73-148 | REAL (24 verified) / REAL-UNPROVEN (6) | S3* | VA_RT_FORMAT_YUV420=1, VA_INVALID_ID, VA_PICTURE_H264_INVALID/SHORT_TERM/LONG_TERM, VA_SLICE_DATA_FLAG_ALL lack host-verifiable mappers — stable-ABI values, consistent, UNVERIFIED |

## 5. Test honesty
No tests in this file (declarations + pure helpers). The pin asserts (164-165, 606-608) are compile-time and DID run in the shipped build — that is real, automatic verification. The pack helpers and `empty()`/`flat_default()` constructors are exercised only via session.rs (another agent's file). Mutation reasoning: flipping a wrong constant here (e.g. VA_ENTRYPOINT_VLD=0) would not fail any test in this repo — no test loads libva and round-trips values. The lane's failure-path integration tests (per hw/mod.rs: "failure-path runtime-tested on this host") exercise the dlopen/probe chain, which would catch a bad SYMBOL name but not a bad ENUM value.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "mirrored field-for-field from the public VA-API specification headers of libva 2.22.0 — the exact libva.so.2 installed on this build host (libva.so.2.2200.0)" | ffi.rs:4-8 | REAL (constants) / REAL-UNPROVEN (field order) | dpkg: libva2 2.22.0-3; ctypes vaErrorStr/vaEntrypointStr/vaProfileStr/vaConfigAttribTypeStr/vaBufferTypeStr sweeps — all exact |
| "three VA_STATUS_* values were wrong in the r2 draft (INVALID_DISPLAY 2→3, UNSUPPORTED_PROFILE 14→12, UNIMPLEMENTED 3→20)" | ffi.rs:38-41 | REAL | vaErrorStr(3)='invalid VADisplay', (12)='requested VAProfile is not supported', (20)='function is not implemented' |
| "VASliceParameterBufferH264 is 3128 bytes, not 3124 (the 3 alignment pad bytes … offset 25 rounds to 28)" | ffi.rs:41-44 | REAL (arithmetic) / REAL-UNPROVEN (vs header) | my recomputation: 25→28 pad, total 3128; header absent on host |
| "VA_STATUS_ERROR_DECODING_ERROR … mapped to the typed Decode channel by the session's va_op_error" | ffi.rs:79-80 | FALSE (overstated) | session.rs:1409-1416 discards the status (F-001) |
| "the va_reserved padding fields exist exactly for that promise" (ABI freeze) | ffi.rs:10-13 | REAL-UNPROVEN | consistent with all struct layouts verified; upstream policy not independently checkable |

## 7. Dead / duplicated / unreachable code
- VA_STATUS_ERROR_OPERATION_FAILED / _INVALID_DISPLAY / _UNIMPLEMENTED / _UNSUPPORTED_PROFILE / _DECODING_ERROR, VA_PROFILE_NONE: zero decision-site consumers (rg across crates/codecs/src excluding ffi.rs); stringified only (F-001).
- Everything else is consumed by library.rs/session.rs (VA_SLICE_DATA_FLAG_ALL, empty()/flat_default(), pack helpers, va_status_text: rg-verified).

## 8. Dependencies & flags
- `#![allow(unsafe_code)]` (ffi.rs:47) — declarations only; no unsafe blocks in this file at all (the only "unsafe" is the `unsafe extern "C" fn` type aliases, which are types, not calls).
- Zero new dependencies (no -sys crate) — matches AGENTS.md rule 2 claim.
- Gate: `#[cfg(target_os = "linux")] pub mod vaapi;` (hw/mod.rs:94). No cargo feature gate — always compiled on Linux.

## 9. Open questions for the next agent
- library.rs auditor: the dlsym symbol-name strings must match these 21 names exactly — I verified all exist in libva.so.2.2200.0 / libva-drm.so.2; confirm the strings in library.rs do too.
- session.rs auditor: does any driver-visible path depend on the UNVERIFIED field orders (F-003)? If a real GPU ever runs this, a field-order mismatch corrupts decode silently.
- Whoever owns docs: the "card 7.B1" citations in the vaapi lane (ffi.rs:1, session.rs:1424) should read 7.B3 per hw/mod.rs:37-38.
