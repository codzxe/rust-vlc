# AUDIT: crates/codecs/src/video/h264/hw/vaapi/ffi.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2026-11-21 |
| Lines / functions | 609 / 6 (va_status_text, VAPictureH264::empty, seq_fields::pack, pic_fields::pack, VAIQMatrixBufferH264::flat_default, VASliceParameterBufferH264::empty) + ~60 type aliases/consts/fn-pointer types/size pins |
| File verdict | UNREACHABLE (declarations consumed only by the dead vaapi lane; layout internally consistent; ABI-value provenance NOT verifiable from the tree) |
| Reachable from shipped entry point? | NO. ffi.rs items ← `library.rs` dlsym table / `session.rs` (both `#[cfg(target_os="linux")]` via hw/mod.rs:94-95) ← `VaApiBridge`/`VaApiSession` ← `H264Decoder::from_setting` (decode.rs) ← zero production callers (Grep over crates/ + apps/: only `crates/codecs/tests/h264_hw_path.rs` and a comment at apps/cli/src/pipeline_host.rs:342). Shipped lane: pipeline_host.rs:346 `"h264" \| "avc1" => self.seeded(track, config, H264SwDecoder::new)`; product.toml:168 `hw-decode = false`, :191 `hw_decode_policy = "software-only"`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read (full file); Grep `VA_STATUS_ERROR_*` in vaapi/ → only ffi.rs:76-92 (no branching consumer); Grep `flat_default|VASliceParameterBufferH264|seq_fields::pack` in session.rs → usage map (520-572, 1256-1319); sed -n on quoted ranges (line numbers match Read; a `#ust_use` rendering artifact in Bash output traced to ANSI-eating in the tool pipeline, file itself verified clean via Read); rg -n "vaapi" audit/runs/baseline/test-workspace.log.txt → lines 572-591 all `ok`; ls -la /lib/x86_64-linux-gnu/libva.so.2* → libva.so.2.2200.0 present (provenance claim about the build host TRUE); find / -name va.h → absent (no libva headers in sandbox). |

## 1. One-paragraph verdict
A hand-rolled libva 2.22.0 C-ABI declaration file: type aliases, ~20 constants, 20 exact fn-pointer prototypes, 4 `#[repr(C)]` H.264 decode structs with compile-time size pins (36/8/672/240/3128), and two bitfield pack helpers. The struct layouts are internally consistent — I re-derived all five sizes by hand from the declared field order and natural alignment and got exactly 36/8/672/240/3128, and the baseline workspace build (build-release-workspace.log.txt, exit 0) proves the const asserts hold on x86_64-unknown-linux-gnu. The pack helpers have executed golden coverage via session tests (baseline test-workspace.log.txt:584,590 `picture_parameter_packs_the_bitfield_words` / `slice_parameter_carries_the_parsed_facts` … ok). But the file's core claim — every constant "re-verified field-for-field against the actual libva 2.22.0 release headers" — is unverifiable from this tree: no header copy, no script, no executable check pins the `VA_STATUS_*` values or the C-side sizes, and my domain knowledge of libva's `va/va.h` contradicts two of the six status values. The whole module is dead code in shipped builds anyway (see reachability).

## 2. What actually works (verified)
— Compile-time size pins held in the baseline release build: `cargo build --release --locked --workspace` exit 0 (audit/runs/baseline/INDEX.md row 1) — the five `const _: () = assert!(size_of…)` (ffi.rs:164-165, 606-608) all evaluated on x86_64.
— seq_fields::pack / pic_fields::pack bit orders executed with hand-computed goldens: test-workspace.log.txt:584 `picture_parameter_packs_the_bitfield_words ... ok`, :590 `slice_parameter_carries_the_parsed_facts ... ok`.
— Symbol names of all 21 prototypes resolved against the REAL installed libva: test-workspace.log.txt:580 `library::tests::loads_the_real_installed_libva_or_reports_it_typed ... ok` (libva.so.2.2200.0 confirmed present on this host — `ls /lib/x86_64-linux-gnu/libva.so.2*`).
— My independent hand-layout of all four structs from declared field order + natural alignment reproduces the pinned sizes exactly (36, 8, 672, 240, 3128 — including the 3 pad bytes at offset 25→28 before RefPicList0 and the 2-byte pad at 3110→3112 before va_reserved).

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/video/h264/hw/vaapi/ffi.rs:36-44 and 71-81
Code:
```
//! the status constants below were re-verified field-for-field
//! against the actual libva 2.22.0 release headers (the spec
//! tarball's `va/va.h`) — three `VA_STATUS_*` values were wrong in
//! the r2 draft (INVALID_DISPLAY 2→3, UNSUPPORTED_PROFILE 14→12,
//! UNIMPLEMENTED 3→20) and `VASliceParameterBufferH264` is 3128
```
```
pub const VA_STATUS_ERROR_UNIMPLEMENTED: VAStatus = 20;
pub const VA_STATUS_ERROR_UNSUPPORTED_PROFILE: VAStatus = 12;
/// `VA_STATUS_ERROR_DECODING_ERROR` (0x17) — the driver's bitstream
/// damage verdict (mapped to the typed Decode channel by the
/// session's `va_op_error`).
pub const VA_STATUS_ERROR_DECODING_ERROR: VAStatus = 23;
```
What is wrong: The "re-verified" provenance is unverifiable from the tree — there is no vendored `va.h`, no checking script, and no test that pins a single `VA_STATUS_*` value or C-side struct size against libva (the size pins assert only the Rust-side layout; dlsym resolves only symbol names, not values). Two values contradict the real libva `va/va.h` per auditor domain knowledge: `VA_STATUS_ERROR_UNIMPLEMENTED` is `0x00000013` (=19), not 20, and `0x00000017` (=23) is `VA_STATUS_ERROR_INVALID_FILTER_CHAIN`; no `VA_STATUS_ERROR_DECODING_ERROR` exists in va.h to my knowledge (the constant's own doc comment even claims it is "mapped to the typed Decode channel by the session's va_op_error" — false: va_op_error (session.rs:1410-1416) discards the status entirely, `let _ = va_status_text(status)`).
Why it matters: The r3 note itself proves the verification process already shipped three wrong values once; a second silent drift is exactly the failure mode this file's credibility rests on. If UNIMPLEMENTED/DECODING_ERROR are ever branched on, every 0x13 (UNIMPLEMENTED) and 0x15-0x17 driver status maps to the wrong diagnosis. Today the blast radius is diagnostic strings in an unreachable lane, so S3 not S2.
What correct looks like: vendor the five referenced header excerpts (or a sha256-pinned copy of libva 2.22.0 va.h/va_dec_h264.h) into docs/dependencies/records, add a test that extracts the constants (e.g. compile-time check against a checked-in C header via cc, or a documented golden table with a spec citation per value), and delete the fictional DECODING_ERROR or point it at a real status.
Confidence: HIGH on "unverifiable from tree / no executable pin / va_op_error ignores status" (verified by Grep: VA_STATUS_ERROR_* referenced only at ffi.rs:76-92); MEDIUM on the two specific value contradictions (domain knowledge; no header in sandbox, no network) — that part UNVERIFIED.
### F-002 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/ffi.rs:83-95
Code:
```
pub fn va_status_text(status: VAStatus) -> String {
    match status {
        VA_STATUS_SUCCESS => "VA_STATUS_SUCCESS".into(),
```
```
        other => format!("VA_STATUS_{other:#x}"),
    }
}
```
What is wrong: Only 6 of the ~30 spec statuses are mapped; every other real driver failure renders as a fabricated spec-looking spelling — e.g. ALLOCATION_FAILED(2) → "VA_STATUS_0x2", INVALID_SURFACE(6) → "VA_STATUS_0x6", INVALID_PARAMETER(0x11) → "VA_STATUS_0x11". These look like enum names but are hex dumps.
Why it matters: Diagnostics-only path, but the function's doc says "Render VA_STATUS_* as its spec spelling" — for 24+ statuses it renders a non-existent spelling; a user grepping the spec for "VA_STATUS_0x11" finds nothing.
What correct looks like: map the full published list, or render `VA_STATUS_UNKNOWN(0x…)`, or keep a spec excerpt table (see F-001) and format from it.
Confidence: HIGH — Verification: read; consumer call sites session.rs:1414/1420/1537 confirmed diagnostics-only.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `va_status_text` | 84-95 | REAL | S4 | Pure; allocates a String per error; 6/30 statuses mapped (F-002); only consumers are dead lane diagnostics; no direct test — its UNIMPLEMENTED/DECODING_ERROR arms are never executed anywhere. |
| `VAPictureH264::empty` | 315-328 | REAL | — | Pure constructor; executed via session tests (initial_ref_list… baseline :582 ok). |
| `seq_fields::pack` | 388-417 | REAL | — | Pure bit packing; masks inputs (&0x3/&0xF); golden-covered (baseline :584). Bit order matches libva's documented LSB-first union layout. |
| `pic_fields::pack` | 425-452 | REAL | — | Pure; golden-covered (baseline :584). `reference_pic_flag` at bit 10 — matches auditor recollection of va_dec_h264.h; same unverifiable-header caveat as F-001. |
| `VAIQMatrixBufferH264::flat_default` | 478-488 | REAL | — | All-16 flat lists = §7.3.2.1.1 defaults; used at session.rs:520; the honesty note (456-466) correctly states streams with scaling lists are refused typed instead. |
| `VASliceParameterBufferH264::empty` | 561-600 | REAL | — | Spec "absent" defaults; used at session.rs:1319; golden-covered (baseline :590). |
| (declarations: types/consts/protos) | 49-281 | REAL-UNPROVEN | — | 6 type aliases, 5 handle aliases, 6 status consts (F-001), 4 profile consts, entrypoint/attrib/rt-format/buffer-type/flag consts, VA_INVALID_ID, 21 fn-pointer prototypes. Signatures match the public libva API surface per auditor knowledge; symbol names proven present in the installed libva.so.2.2200.0 by baseline :580; argument/return TYPES are unverifiable at runtime (dlsym cannot check them). |
| (size pins) | 164-165, 602-608 | REAL | — | 5 const asserts; internally consistent (hand-re-derived); prove Rust layout only, not agreement with libva's C structs (F-001). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| (no tests in this file) | — | Coverage is indirect, via session tests (baseline :572-591, all ok): pack helpers and empty() constructors are golden-covered; the six status constants and all prototypes have ZERO executable checks (a wrong VA_STATUS value or a wrong prototype arity compiles and passes every test in the repo). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the exact libva.so.2 installed on this build host (libva.so.2.2200.0, ldconfig -p)" | ffi.rs:6-8 | REAL | `ls /lib/x86_64-linux-gnu/libva.so.2*` → `libva.so.2.2200.0`; dpkg: libva2 2.22.0-3. |
| "status constants … re-verified field-for-field against the actual libva 2.22.0 release headers" | ffi.rs:36-38 | REAL-UNPROVEN→contradicted | No header in the tree/sandbox to verify against; auditor domain knowledge contradicts UNIMPLEMENTED=20 (real 0x13=19) and DECODING_ERROR=23 (0x17 is INVALID_FILTER_CHAIN) — see F-001. |
| "VA_STATUS_ERROR_DECODING_ERROR … mapped to the typed Decode channel by the session's va_op_error" | ffi.rs:78-80 | FALSE | session.rs:1410-1416: `va_op_error` does `let _ = va_status_text(status);` then returns a code-only `H264HwError::Decode` — the status value is discarded, never mapped to any channel. |
| "compile-verified via cargo check -p codecs" | ffi.rs:30-31 | REAL | Baseline build-release-workspace.log.txt exit 0 covers this and more. |
| "Nothing here *calls* anything: this file only declares the ABI shapes" | ffi.rs:24-26 | REAL | Read the whole file: zero `unsafe` blocks, zero extern calls — declarations only. |

## 7. Dead / duplicated / unreachable code
- The entire file is unreachable from shipped entry points (see header table) — compiled on Linux targets only (hw/mod.rs:94), exercised exclusively by tests.
- `VA_STATUS_ERROR_OPERATION_FAILED/INVALID_DISPLAY/UNIMPLEMENTED/UNSUPPORTED_PROFILE/DECODING_ERROR` (74-81) have no branching consumer anywhere in the tree (Grep: referenced only inside ffi.rs itself) — constants exist solely to feed va_status_text arms that themselves are only called from two `let _ =` discard sites and one error-string format (session.rs:1414/1420/1537).

## 8. Dependencies & flags
- No Cargo feature gates this file (hw modules unconditional per codecs/Cargo.toml:71-72 comment); OS gate only: `#[cfg(target_os = "linux")]` at hw/mod.rs:94. Release Linux artifacts (release.yml:193) carry it compiled-but-dead.
- `#![allow(unsafe_code)]` (ffi.rs:47) overrides the workspace `unsafe_code = "deny"` (root Cargo.toml:54) — legal (deny, not forbid, is overridable by an inner attribute) and consistent with baseline clippy CLEAN (INDEX row 5).
- Zero external deps (no `-sys` crate) — matches AGENTS.md rule 2 claim in the header.

## 9. Open questions for the next agent
- Whoever owns library.rs / device.rs (not in my assignment): does `loads_the_real_installed_libva_or_reports_it_typed` assert anything beyond dlopen success (e.g. all 21 dlsym lookups non-null)? If it resolves the full table, the prototype NAMES are proven; the VA_STATUS values still are not.
- ADR-013 (vaapi activation) — if it ever ratifies activation, F-001's constants become load-bearing for error mapping; fix before, not after.
