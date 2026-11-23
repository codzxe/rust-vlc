# AUDIT: crates/codecs/src/video/h264/hw/vaapi/dl.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-23 |
| Lines / functions | 170 / 6 (+ extern block, +Send impl) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES on Linux builds — `hw/mod.rs:94-95` `#[cfg(target_os = "linux")] pub mod vaapi;` → `library.rs:110-111` `SharedLibrary::open(VA_SONAMES)` for `libva.so.2` / `libva-drm.so.2`; the lane serves the opt-in `playback.hw-decode-policy` hw path and capability reporting. Not compiled on non-Linux (cfg-gated). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 --exact video::h264::hw::vaapi::library::tests::loads_the_real_installed_libva_or_reports_it_typed --nocapture` → **1 passed** (this sandbox has the real `/usr/lib/x86_64-linux-gnu/libva.so.2` — the test actually dlopened it and resolved symbols through this module). `./target/debug/deps/codecs-bae0f1756cc0a809 vaapi` → 24 passed. `./target/debug/deps/h264_hw_path-f4e0fd72a3109215` → 20 passed (the hw contract suite). |

## 1. One-paragraph verdict
A small, disciplined dlopen(3) wrapper: no link-time dependency (machines without libva get a typed `LibraryUnavailable`), the POSIX dlerror clear-then-check discipline at both call sites, interior-NUL candidates treated as failed candidates rather than mis-called libc, an owned handle closed exactly once in Drop, and a documented, defensible `Send` impl. I executed the real thing: the in-tree host test dlopened this sandbox's actual libva.so.2 and resolved its symbols through this module, and the 20-test hw contract suite passes. The one blemish is a comment/code mismatch: `symbol()`'s SAFETY note says the NULL result is "checked, with dlerror consulted", but the failure detail never includes dlerror's message — diagnostics only, no safety impact.

## 2. What actually works (verified)
- Real dlopen+dlsym execution against the installed libva.so.2 (host test passed on this machine).
- 24/24 vaapi unit tests, 20/20 hw-path contract tests pass.
- The dlerror-clear-before-call pattern (the classic dlsym-NULL disambiguation) is present at both `open` (L107) and `symbol` (L139).

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/dl.rs:136-146
Code:
```rust
        // SAFETY: `self.handle` is a live dlopen handle (owned, not
        // yet closed) and `symbol` is NUL-terminated. dlsym returns
        // NULL for "not found" — checked, with dlerror consulted.
        let _ = last_error(); // clear stale error (POSIX)
        let address = unsafe { dlsym(self.handle, symbol.as_ptr()) };
        if address.is_null() {
            return Err(LibraryLoadError {
                names: vec![self.name.clone()],
                detail: format!("symbol {name} not found"),
```
What is wrong: the comment claims dlerror is consulted, but the returned `detail` is a static string — the loader's actual diagnosis (undefined symbol vs. any other error) is discarded. Diagnostics gap only; the safety pattern itself is correct. Confidence: HIGH — Verification: read.
(Observation, not a finding: `RTLD_NOW_LOCAL = 0x2` hard-codes glibc's mask where RTLD_LOCAL == 0; correct on Linux — the module is cfg-gated to `target_os = "linux"`, so no portability exposure.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| extern "C" dl* block | 34-39 | REAL | — | declared ABI, invariants documented per call site |
| last_error | 42-54 | REAL | — | reads thread-local static as CStr, never freed — correct |
| SharedLibrary (struct) + unsafe Send | 56-67 | REAL | — | single-owner discipline; rationale documented |
| Debug impl | 69-75 | REAL | — | prints name only, not the raw handle |
| SharedLibrary::open | 94-122 | REAL | — | candidate loop, interior-NUL guard, clear-then-open, executed on real libva |
| SharedLibrary::symbol | 129-148 | REAL | S4 | F-001 (dlerror message discarded) |
| SharedLibrary::name | 151-154 | REAL | — | accessor |
| Drop | 157-169 | REAL | — | dlclose exactly once; null-guard; idempotent shape |

## 5. Test honesty
This module's behavior is pinned indirectly: `loads_the_real_installed_libva_or_reports_it_typed` (library.rs:266) executes the real dlopen path on hosts with libva and asserts the typed failure on hosts without — both branches named in the test. It passed here (real branch). The failure branch was not exercised on this machine (libva present) — UNVERIFIED here, read-verified only.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no link-time dependency, so machines without libva still run Lumen and get a typed LibraryUnavailable" | header L2-6 | REAL | dlopen path + typed LibraryLoadError verified by the passing host test (open branch) |
| "kept to the four dlfcn symbols with the same invariants" | header L10-11 | REAL | exactly dlopen/dlsym/dlclose/dlerror declared |
| "dlerror results are read before any other dl* call" | L32-33 | REAL | clear-before-call at L107/L139 |
| "with dlerror consulted" (symbol failure) | L138 | SHALLOW | message discarded (F-001) |

## 7. Dead / duplicated / unreachable code
Deliberate duplication of platform-linux's dl module, justified by the layering wall (header L8-11, doc 01 §1.4) — documented, acceptable.

## 8. Dependencies & flags
libc via extern "C" (no crate dependency); `#![allow(unsafe_code)]` is the module-scoped sanctioned exception (per header, MP §9 / ADR-001 §5); cfg-gated to Linux at the parent module. The `LUMEN_*` env knobs do not appear here.

## 9. Open questions for the next agent
- library.rs/ffi.rs (symbol transmutes to the libva 2.22 prototypes) are not in my assignment — the actual unsafe surface that consumes `symbol()`'s raw pointers; audit the transmute types against the installed libva headers.
- Whether the hw-decode-policy lane (settings → H264Decoder::from_setting) is exercised by any UI-level test: the 20 hw-path tests cover the contract layer; the settings wiring is a different file's problem.
