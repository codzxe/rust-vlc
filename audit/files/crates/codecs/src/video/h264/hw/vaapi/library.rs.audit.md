# AUDIT: crates/codecs/src/video/h264/hw/vaapi/library.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 293 / 4 (VaLibrary::{load,sonames,decode_calls}, transmute_symbol, pin_fn_ptr_size! macro) + 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — inherits decode.rs F-001: the hw lane (H264Decoder::from_setting -> installed_bridges) is never constructed by the shipped registry (software H264SwDecoder is); this module is library-real but shipped-dead. cfg(target_os = "linux") (hw/mod.rs:94-95) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | nm -D over installed libva.so.2 / libva-drm.so.2 (all 19 core symbols + vaGetDisplayDRM confirmed exported, incl. versioned vaCreateSurfaces@@VA_API_0.33.0); baseline test results cited; dl.rs loader inspected (real dlopen/dlsym FFI) |

## 1. One-paragraph verdict
A real, careful dlsym-symbol-resolution module: it dlopen's libva.so.2 + libva-drm.so.2 through the genuine dl.rs loader (extern "C" dlopen/dlsym, RTLD_NOW_LOCAL), resolves ALL 21 lane symbols up front (a partial table is a typed load failure, never a mid-decode surprise), and guards the address→fn-pointer conversion with compile-time pointer-size pins for every prototype plus documented SAFETY reasoning per site. I independently verified with nm -D that every symbol name in this file is exported by the installed libva 2.22.0 in this sandbox, and the baseline host test (`loads_the_real_installed_libva_or_reports_it_typed`, ok at baseline L675) exercised the real load. Defects: signature (not just name/size) correctness is spec-transcription only — unverifiable until the first real call, and the lane is shipped-dead (nothing constructs it).

## 2. What actually works (verified)
— verified by `nm -D /usr/lib/x86_64-linux-gnu/libva.so.2` + per-symbol loop: all 19 core names present (vaCreateSurfaces as versioned `T vaCreateSurfaces@@VA_API_0.33.0`), and `nm -D libva-drm.so.2` -> `T vaGetDisplayDRM`. The module's claim "the installed libva.so.2.2200.0 exports every symbol below" holds on this host.
— verified baseline audit/runs/baseline/cargo-test-workspace.txt:674-675 -> both unit tests ok (the Ok-arm ran: SONAME assertions passed against the real library).
— verified dl.rs is a real loader (extern "C" { dlopen, dlsym, ... }, L34-36; non-NULL symbol results) — library.rs is not resolving fake pointers.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/library.rs:205-222
Code: (verbatim, L210-214)
```
    debug_assert_eq!(
        std::mem::size_of::<F>(),
        std::mem::size_of::<*mut c_void>(),
        "ffi.rs prototype is not pointer-sized on this target"
    );
```
What is wrong (limitation, honestly half-admitted at L20): the pins prove pointer SIZE, and the host test proves symbol NAME existence — neither proves the SIGNATURE (argument/return ABI) matches the installed library. The doc itself says "The *signatures* are the 2.22.0 spec prototypes of ffi.rs" — i.e., transcribed from spec, not verified against the binary. A signature drift would be UB at first call. Currently moot because the lane is unreachable in the shipped build and driver-mediated decode is compile-verified only, but this is the module's standing risk. Confidence: HIGH — Verification: read + nm.
F-002 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/library.rs:266-286
Code: (verbatim, L274-279)
```
        match VaLibrary::load() {
            Ok(lib) => {
                let (core, drm) = lib.sonames();
                assert_eq!(core, "libva.so.2", "versioned SONAME wins");
                assert_eq!(drm, "libva-drm.so.2", "versioned DRM SONAME wins");
```
What is wrong: on a host without libva the test passes via the Err arm (asserts the error names candidates) — a green CI run on a libva-less host proves only the failure path; the whole-table resolution proof is host-dependent. The comment says "skipped with a typed note on hosts without libva" but the test does not skip — it takes the Err arm and still reports ok. Cosmetic doc/test mismatch. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/library.rs (whole file) + hw/mod.rs:94-95
What is wrong: the entire VA-API lane is unreachable from the shipped entry points (see decode.rs audit F-001: the registry constructs H264SwDecoder; from_setting has no shipped caller). Library-real, shipped-dead. Confidence: HIGH — Verification: rg construction sites (none outside codecs).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| VaLibrary::load | 109-146 | REAL | - | full up-front table; typed LibraryLoadError; executed on host (baseline Ok-arm) |
| VaLibrary::sonames | 150-152 | REAL | - | diagnostics |
| VaLibrary::decode_calls | 161-170 | REAL | - | Copy snapshot to end borrows before &mut updates — sound borrow-splitting rationale |
| transmute_symbol | 205-222 | REAL | S4 | documented transmute_copy escape hatch; size pins; signature risk (F-001) |
| pin_fn_ptr_size! | 228-259 | REAL | - | compile-time assert per prototype — genuine guard |
| tests (2) | 265-293 | REAL | S4 | host-dependent value (F-002) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| loads_the_real_installed_libva_or_reports_it_typed | YES on libva hosts (wrong SONAME preference or a missing-symbol panic fails it); WEAK on libva-less hosts (Err arm only) | baseline ran the Ok-arm on this host |
| candidate_lists_start_with_the_versioned_sonames | YES (narrowly) | pins the const arrays |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The installed libva.so.2.2200.0 exports every symbol below — verified against the loader's table (nm -D, recorded in the worklog)" | L14-19 | REAL | my own nm -D run: all 20 names exported (incl. versioned vaCreateSurfaces) |
| "a missing symbol is an immediate typed LibraryUnavailable load failure, never a later surprise mid-decode" | L8-12 | REAL | transmute_symbol propagates symbol() Err; load() resolves all before returning Ok |
| matrix "VA-API (Linux) compile-verified + failure-path runtime-tested on the no-GPU host ... driver-mediated decode compile-verified only" | docs/feature-matrix.toml:416 | REAL | consistent with this module + no runtime decode surface here |

## 7. Dead / duplicated / unreachable code
Shipped-unreachable (F-003) — same root cause as decode.rs. No internal dead code.

## 8. Dependencies & flags
cfg(target_os = "linux") at the parent (hw/mod.rs:94). No external crates — hand-rolled ffi.rs prototypes + dl.rs dlopen. #![allow(unsafe_code)] scoped to this module with documented justification.

## 9. Open questions for the next agent
- ffi.rs prototype transcription vs libva 2.22.0 headers — a diff against /usr/include/va/va.h (if present) would upgrade F-001 from spec-trust to verified; not done here (header set not checked).
- vaapi session/decode modules (siblings) not in my assignment — the "failure-path runtime-tested" claim lives there.
