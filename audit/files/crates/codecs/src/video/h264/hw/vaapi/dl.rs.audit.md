# AUDIT: crates/codecs/src/video/h264/hw/vaapi/dl.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2026-11-21 |
| Lines / functions | 169 / 8 (no tests in-file; no runtime execution evidence anywhere — see §2) |
| File verdict | UNREACHABLE (correct-by-inspection dlfcn wrapper in a lane no shipped entry point constructs) |
| Reachable from shipped entry point? | NO — `SharedLibrary::open`/`symbol` ← `vaapi/library.rs` (VaLibrary::load, W06-A01's file) ← session.rs probe/configure ← `hw::negotiate_lane` ← `decode.rs:312 H264Decoder::from_setting` ← no production caller (chain established in this wave's log). Linux-only via `h264/hw/mod.rs:94 cfg(target_os = "linux")`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read full file; cross-checked RTLD_* values for glibc/musl from knowledge; verified the sibling claim (platform-linux dl) exists via module docs; no baseline test touches this file (grep of test log: vaapi suites cover probe typed-failure paths only, which return before dlopen). |

## 1. One-paragraph verdict
A disciplined 169-line dlfcn wrapper: interior-NUL candidates refused instead of truncated into libc, POSIX dlerror discipline (clear-before-call, read-after-failure), owned handle closed exactly once, and an honest typed error carrying every attempted SONAME. One real deviation from its own stated invariants: the `dlsym` NULL path never consults `dlerror` despite the SAFETY comment claiming it does, conflating POSIX's legal NULL-symbol-value with not-found and discarding the loader's diagnosis. The file has never executed anywhere — the sandbox has no GPU, so the probe dies at the device-node stage before any `dlopen` — and the lane it serves is unreachable in shipped builds.

## 2. What actually works (verified)
— `open`'s POSIX discipline hand-verified: `CString::new` interior-NUL guard (:100-103), stale-error clear before `dlopen` (:107), NULL-handle check, `dlerror` read after failure with a fallback message for the POSIX-legal NULL-without-error case (:115-116), ordered candidates with first-success-wins, empty-candidate typed message (:95). The failure this environment can actually produce (probe → `NoDeviceNode` before any dlopen) is covered by the session probe tests (baseline :585-589).
— `RTLD_NOW_LOCAL = 0x2` — correct for glibc (RTLD_NOW=2, RTLD_LOCAL=0) and musl (same values); module is Linux-only by cfg.
— `Drop`: null-guarded single `dlclose`, handle nulled after (:157-168) — pairs with `open`'s dlopen exactly once.
— `Debug` deliberately elides the raw handle (:69-75) — no pointer leak in reports.
Nothing here executed: UNVERIFIED by execution (no libva/GPU in sandbox; no test constructs this type — `SharedLibrary` appears only in library.rs and this file).

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/video/h264/hw/vaapi/dl.rs:136-145
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
What is wrong: the SAFETY comment says "checked, with dlerror consulted" — but the NULL path fabricates the static detail `"symbol {name} not found"` and never calls `dlerror()` after the failed `dlsym`. Two consequences: (a) the loader's actual diagnosis (glibc: `undefined symbol: vaXYZ`) is swallowed, contradicting the module's own typed-diagnosis doctrine (`open`'s error carries the loader's text verbatim at :115-116); (b) POSIX permits `dlsym` to return NULL for a symbol that EXISTS with value NULL — the standard probe is clear-dlerror/call/check-dlerror, which distinguishes the cases; this code cannot.
Why it matters: a mis-typed symbol resolution failure becomes undiagnosable in the field — exactly the situation the typed `LibraryLoadError` exists for. (Functionally near-zero risk: real libva symbols are never NULL-valued.)
What correct looks like: `let err = last_error(); detail: err.unwrap_or_else(|| format!("symbol {name} not found"))` — and fix the comment.
Confidence: HIGH — Verification: read; POSIX dlsym semantics from knowledge (no libc headers in sandbox to cite — UNVERIFIED against installed man pages).
### F-002 [S4]
Where: crates/codecs/src/video/h264/hw/vaapi/dl.rs:8-11 (doc)
Code:
```rust
//! This is a minimal sibling of `platform-linux`'s audited
//! `linux::dl` (the layering wall forbids importing it from this
//! L3 crate — edges point downward only, doc 01 §1.4), kept to the
//! four `dlfcn` symbols with the same invariants.
```
What is wrong: an acknowledged copy of platform-linux's dl wrapper — the same four-symbol surface with "the same invariants" maintained by hand in two places. Any fix to one (e.g. F-001's dlerror discipline, if platform-linux's copy has it right) must be manually mirrored.
Why it matters: duplication drift between two safety-critical (unsafe) wrappers; the layering rationale is legitimate, but nothing (test, lint, comment cross-reference) pins the two implementations to each other.
What correct looks like: a shared test suite or doc cross-link asserting behavioral parity ("platform-linux/linux/dl.rs must change with this file").
Confidence: HIGH — Verification: read; platform-linux's copy not audited (outside my assignment — its owner should diff the two).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| extern block (`dlopen`/`dlsym`/`dlclose`/`dlerror`) | 34-39 | REAL | — | edition-2024 `unsafe extern` syntax; invariants listed at :27-33 |
| `last_error` | 42-54 | REAL | — | clears per POSIX; lossy-read on libc static; never freed (correct) |
| `SharedLibrary` + `unsafe impl Send` + `Debug` | 59-75 | REAL | — | Send justified (glibc loader thread-safety + single owner); residual call-while-closing hazard is the caller's (library.rs) discipline |
| `LibraryLoadError` | 79-86 | REAL | — | carries all attempted names + detail |
| `SharedLibrary::open` | 94-122 | REAL | — | verified in §2; never executed in any evidence run |
| `SharedLibrary::symbol` | 129-148 | REAL | S3 | F-001 dlerror not consulted on the NULL path; raw-pointer return with transmute responsibility documented to library.rs |
| `SharedLibrary::name` | 152-154 | REAL | — | pure accessor |
| `Drop for SharedLibrary` | 157-168 | REAL | — | single null-guarded dlclose |

## 5. Test honesty
No tests in or for this file. The vaapi test suite (session.rs tests, baseline :579-595) exercises only the probe's typed-failure paths, which return at the device-node stage before `VaLibrary::load()` — so `open`/`symbol`/`Drop` have zero execution evidence. A real test would need a loadable stub `.so` with known symbols (in-repo fixture + `LD_LIBRARY_PATH` or absolute path) — feasible on this Linux sandbox and not done.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no link-time dependency, so machines without libva still run Lumen" | :2-6 | REAL (by construction) | extern-block dlopen, not a cargo/links dependency; the sandbox build itself proves libva-absence compiles |
| "dlsym returns NULL for 'not found' — checked, with dlerror consulted" | :137-138 | FALSE | F-001 — dlerror is cleared but never read on the NULL path |
| "the handle is closed exactly once (ownership in SharedLibrary)" | :31 | REAL | Drop :157-168 |
| "kept to the four dlfcn symbols with the same invariants [as platform-linux]" | :10-11 | REAL-UNPROVEN | parity with the sibling not verified (outside assignment) |

## 7. Dead / duplicated / unreachable code
Unreachable as a whole (lane chain, header). Duplicated surface with `platform-linux`'s dl (F-002, architecturally justified).

## 8. Dependencies & flags
`#![allow(unsafe_code)]` (:19) — the audited E2 exception. std-only (std::ffi). Linux-only by module cfg. No feature gates. Unsafe sites: dlerror (47), dlopen (108), dlsym (140), dlclose (164) — each SAFETY-commented; the dlsym comment is the one that lies (F-001).

## 9. Open questions for the next agent
- W06-A01 (vaapi/library.rs): the transmute of `symbol()`'s raw pointer to fn types — check each signature against libva's headers (same class of risk as session.rs#part3 F-003).
- Whoever owns `platform-linux`'s dl: diff the two implementations for the dlerror-on-dlsym discipline (F-001/F-002 parity).
