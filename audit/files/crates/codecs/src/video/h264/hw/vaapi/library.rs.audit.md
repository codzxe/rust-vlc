# AUDIT: crates/codecs/src/video/h264/hw/vaapi/library.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-21 |
| Lines / functions | 293 / 10 (7 production items + macro + 2 tests) |
| File verdict | UNREACHABLE (real dlopen/symbol-resolution layer, host-tested against the installed libva; no shipped entry point reaches the VA-API lane) |
| Reachable from shipped entry point? | NO — only consumer chain is the VA-API session/probe (session.rs:749, 1498) inside the hardware lane, triggered solely by `H264Decoder::from_setting`, which has zero production callers (my decode.rs audit F-001) with product.toml:168 `hw-decode = false`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `sed -n` + `od -c` quote verification (121-127, 274-279; dl.rs symbol NULL check at :141); `rg -n "VaLibrary::load"` → session.rs + this file's test only; baseline: test-workspace.log.txt:577,580 (both library tests `ok` — the host test resolved the full 20-symbol table against the real installed libva.so.2.2200.0) |

## 1. One-paragraph verdict
The dlopen symbol table for the VA-API lane: loads `libva.so.2`/`libva-drm.so.2` (versioned sonames first), resolves all 20 spec functions up front with per-site `unsafe` conversions, and refuses partially-resolved libraries with a typed error. The unsafe discipline here is the best in this wave's assignment: every invariant needed for the dlsym→fn-pointer conversion is either machine-checked (pointer-size equality pinned at compile time by a `const _: assert!` macro over all 20 prototypes; non-NULL pinned by dl.rs:141's checked `symbol()`) or documented at the exact site, and the host test proved all 20 symbol NAMES resolve against the real installed libva in the baseline run. What no evidence can prove is that the ffi.rs SIGNATURES match the library's real prototypes (nm proves existence, not type) — that residual risk is honestly acknowledged in the module docs and the matrix row's [VERIFY]. As with the whole lane: correct-looking, host-tested, and unreachable from anything shipped.

## 2. What actually works (verified)
— `loads_the_real_installed_libva_or_reports_it_typed` ran `ok` in baseline run #3 (test-workspace.log.txt:580): the Ok arm means `VaLibrary::load()` succeeded, i.e. dlopen of both sonames AND all 20 `transmute_symbol` resolutions succeeded against the installed libva.so.2.2200.0 — every ffi.rs name is a real export.
— Non-NULL invariant verified at the source: dl.rs:129-141 `symbol()` returns `Err` on NULL dlsym result (read), so `transmute_symbol`'s address is never NULL.
— Pointer-size pin verified: the `pin_fn_ptr_size!` macro (228-237) emits `const _: assert!(size_of::<T>() == size_of::<*mut c_void>())` for all 20 fn-pointer types (238-259 — counted, matches the 20 struct fields), so a target where any prototype is not pointer-sized fails the build — making `transmute_copy` (which reads `size_of::<F>()` bytes) sound on the pins' targets.
— Partial-failure hygiene: an error mid-table drops the half-built `VaLibrary`, closing both dlopen handles via `SharedLibrary`'s Drop (dl.rs:159-166 nulls the handle after dlclose — read).

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/h264/hw/vaapi/library.rs:101-146 (whole loader), evidence session.rs:749,1498
Code:
```rust
    pub fn load() -> Result<VaLibrary, LibraryLoadError> {
        let core = SharedLibrary::open(VA_SONAMES)?;
        let drm = SharedLibrary::open(VA_DRM_SONAMES)?;
```
What is wrong: unreachable from shipped entry points — same lane-level dead code as device.rs F-001 (chain: from_setting → negotiate_lane → discover_lane → vaapi::discover → session::probe → VaLibrary::load; from_setting has no production caller; product.toml:168).
Why it matters: an FFI layer — the highest-risk code class in the crate — compiled into every shipped Linux build with zero possibility of execution; its only exercised path is the test's dlopen.
What correct looks like: feature-gate the lane (or land ADR-013's wiring) so shipped builds carry it only when it can run.
Confidence: HIGH — Verification: read + rg call graph.

### F-002 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/library.rs:274-279
Code:
```rust
        match VaLibrary::load() {
            Ok(lib) => {
                let (core, drm) = lib.sonames();
                assert_eq!(core, "libva.so.2", "versioned SONAME wins");
                assert_eq!(drm, "libva-drm.so.2", "versioned DRM SONAME wins");
```
What is wrong: the Ok arm hard-asserts the VERSIONED sonames. On a host where only the unversioned dev form exists (`libva.so` present, `libva.so.2` absent — the second candidate in VA_SONAMES), `load()` legitimately succeeds via the fallback candidate and this test FAILS with "versioned SONAME wins", misreporting a correct fallback as a bug. The candidate list exists precisely for that environment.
Why it matters: test portability/honesty — a legitimate library layout produces a red test; no shipped-code impact.
What correct looks like: assert the loaded name is a member of `VA_SONAMES` (and prefers the versioned form when both exist), not equality with the first candidate.
Confidence: HIGH — Verification: read + VA_SONAMES definition (48).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `VA_SONAMES` / `VA_DRM_SONAMES` | 48, 51 | REAL | — | versioned-first, documented |
| `VaLibrary` struct | 56-99 | REAL | S3 | F-001 unreachable; 20 pub fn fields |
| `VaLibrary::load` | 109-146 | REAL | S3 | all-or-nothing resolution, typed errors; per-site SAFETY comments |
| `VaLibrary::sonames` | 150-152 | REAL | — | diagnostics |
| `VaLibrary::decode_calls` | 161-170 | REAL | — | Copy snapshot ending the borrow before &mut self updates — borrow-scoped FFI discipline |
| `VaDecodeCalls` struct | 176-189 | REAL | — | 6-function picture-pipeline subset |
| `transmute_symbol` | 205-222 | REAL | — | unsafe: necessary (dlsym→fn ptr); invariants documented + machine-pinned (non-NULL at dl.rs:141, pointer-size const asserts); `transmute_copy` choice with documented E0512 rationale; residual signature-vs-spec risk is inherent and acknowledged (see §9) |
| `pin_fn_ptr_size!` macro + invocation | 228-259 | REAL | — | compile-time proof over all 20 types — verified count matches struct fields |
| tests: `loads_the_real_installed_libva…` | 266-286 | REAL | S4 | full-table host evidence (baseline ok); F-002 brittle Ok-arm assertion |
| tests: `candidate_lists_start_with_the_versioned_sonames` | 289-292 | REAL | — | trivially pins the constants |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| loads_the_real_installed_libva_or_reports_it_typed | YES | `load()` succeeds only if ALL 20 symbols resolve — a renamed/absent export fails it; on libva-less hosts it degrades to asserting the typed error carries candidate names (honest environment split, no silent skip) |
| candidate_lists_start_with_the_versioned_sonames | NO (near-tautology) | asserts constants against themselves — pins only against accidental reorder |
Gap: no test of the unversioned-soname fallback path (F-002's flip side) — no host coverage where `libva.so` resolves but `libva.so.2` does not.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "resolves **every** function this lane calls up front … a missing symbol is an immediate typed LibraryUnavailable, never a later surprise mid-decode" | library.rs :7-12 | REAL | load() 120-145 all-`?` construction |
| "The installed `libva.so.2.2200.0` exports every symbol below — verified against the loader's table (`nm -D`) … pinned by the host test" | library.rs :16-19 | REAL | baseline test :580 ok (Ok arm taken — all 20 resolved) |
| "The only `unsafe` here is the raw-address → function-pointer conversion … the pointer is non-NULL (checked by dl::SharedLibrary::symbol)" | library.rs :24-28 | REAL | dl.rs:141 NULL check read; every unsafe site is a transmute_symbol call |
| "The *signatures* are the 2.22.0 spec prototypes of ffi.rs" | library.rs :20 | REAL-UNPROVEN | existence proven by host test; signature correctness rests on ffi.rs review vs spec — no runtime call evidence (sandbox has no VAAPI device); matrix row's "[VERIFY]" agrees |
| matrix row: "runtime hardware decode NOT exercised in sandbox [VERIFY]" | docs/feature-matrix.toml:416 | REAL | consistent with this audit — symbol resolution only, no decode |

## 7. Dead / duplicated / unreachable code
Whole file unreachable from shipped builds (F-001). No intra-file dead code; `decode_calls`' snapshot pattern is consumed by session.rs (unreachable lane).

## 8. Dependencies & flags
`#![allow(unsafe_code)]` at module top — a documented, scoped exception to the crate's no-unsafe convention (comment :30-31); only `transmute_symbol`'s call sites contain unsafe. Linux-only via parent cfg. No cargo features. The 20-type pin macro is the load-bearing compile-time guard.

## 9. Open questions for the next agent
- ffi.rs auditor (not assigned to me): verify each of the 20 prototypes against the libva 2.22.0 headers — this file's unsafe soundness (beyond pointer size) reduces entirely to that review; `vaCreateSurfaces`'s arity changed across libva major versions historically.
- session.rs auditor: confirm the session actually calls through `decode_calls()` (the Copy snapshot) rather than re-resolving, so the borrow-scoped discipline claim (:154-159) holds in practice.
- If the lane ever ships: F-002's fallback-path gap and the signature risk need a real-device differential before ADR-013's activation gate.
