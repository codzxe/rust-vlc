# AUDIT: crates/platform-linux/src/linux/dl.rs
| Field | Value |
|---|---|
| Auditor | W08-A09 |
| Date | 2026-11-23 |
| Lines / functions | 170 / 6 (last_error, DynamicLibrary::open, symbol, name, Drop, Send) |
| File verdict | REAL — executed on this host: dlopen of libpulse.so.0 succeeds, ~25 symbols resolve, dlclose pairs on drop (my direct prebuilt-binary run, 2/2 tests pass) |
| Reachable from shipped entry point? | NO — platform-linux has zero dependents (no Cargo.toml in the repo lists it; apps/cli + apps/desktop do not); exercised only by its own test suite |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | direct run of prebuilt test binary `linux_integrations-bba4ec7396f3426d pulse` (exit 0, 2 passed), rg (SONAMES, callers, dependents), ldconfig (libpulse present), sed quote verification |

## 1. One-paragraph verdict
Answering the hostile question first: dlopen of WHAT — **the PulseAudio
client library, `libpulse.so.0` then `libpulse.so`** (pulse.rs:51
`PULSE_SONAMES`, opened at pulse.rs:355). Nothing else uses this loader (rg:
only pulse.rs/pulse_ffi.rs import it; no libpulse-simple, no ALSA, no DRM
dlopen). The wrapper is real and I executed it for real: this host has
libpulse.so.0 (ldconfig) but no PA server, and running the prebuilt test
binary directly passes both pulse tests — `probe()` actually dlopens the
library, `loaded_from()` must report the true SONAME, every PA symbol
resolves through `symbol()`, and Drop's dlclose pairs with the open. The
module is small, disciplined (dlerror cleared before each call, POSIX-correct),
and its one defect is a comment that promises a dlerror diagnosis the code
never reads.

## 2. What actually works (verified)
- Direct execution: `./target/debug/deps/linux_integrations-bba4ec7396f3426d
  --test-threads=1 pulse` -> `pulse_probe_reports_this_hosts_truth ... ok`,
  `pulse_subscribe_without_a_server_is_the_typed_error ... ok`,
  `test result: ok. 2 passed` (exit 0). This drives
  `DynamicLibrary::open(["libpulse.so.0","libpulse.so"])` -> dlopen
  SUCCEEDS on this host (asserting `loaded_from() == "libpulse.so.0"|"libpulse.so"`,
  linux_integrations.rs:392), then pulse_ffi::load resolves the whole PA
  table through `symbol()` (pulse_ffi.rs:276 `let addr = $lib.symbol($name)?`),
  so the NULL-vs-handle and symbol paths both executed.
- Host environment check: `ldconfig -p` shows
  `libpulse.so.0 => /lib/x86_64-linux-gnu/libpulse.so.0` — the dlopen target
  is genuinely present, the success path is not simulated.
- Baseline agrees: cargo-test-workspace.txt:3975-3987 (platform-linux 13
  integrations, 0 failed).
- RTLD_NOW|RTLD_LOCAL = 0x2 is the correct glibc/musl value for the
  documented flags (dl.rs:30-33).

## 3. Findings
F-001 [S3]
Where: crates/platform-linux/src/linux/dl.rs:1-7 (whole-crate context)
Code:
```
//! The crate declares no link-time dependency on libpulse: it
//! `dlopen`s the client library at runtime, so a machine without
//! PulseAudio still runs Lumen and gets an honest capability report.
```
What is wrong: the stated purpose — "a machine without PulseAudio still runs
Lumen" — is moot in every shipped build: nothing depends on platform-linux
(rg over every Cargo.toml including apps/: zero dependents), so no Lumen
binary contains this code and no user machine benefits from the graceful
degradation. Disclosed scaffolding (issue #42 wiring pending), and the
runtime verification is real, but the reachability ledger entry is NO.
Confidence: HIGH — Verification: rg over manifests.

F-002 [S4]
Where: crates/platform-linux/src/linux/dl.rs:136-148
Code:
```
        // SAFETY: `self.handle` is a live dlopen handle (owned,
        // not yet closed) and `symbol` is NUL-terminated. dlsym
        // returns NULL for "not found" — checked, with dlerror
        // consulted for the diagnosis.
        let _ = last_error(); // clear stale error (POSIX)
        let address = unsafe { dlsym(self.handle, symbol.as_ptr()) };
        if address.is_null() {
            return Err(PlatformError::SymbolMissing {
                library: self.name.clone(),
                symbol: name,
            });
        }
```
What is wrong: the comment says "with dlerror consulted for the diagnosis",
but after the NULL dlsym the code never calls `last_error()` — the loader's
diagnosis is discarded and `SymbolMissing` carries only the library+symbol
names. Compare `open()`, which does thread `last_error()` into its error
(dl.rs:113-114). Minor diagnostic loss plus a SAFETY comment that asserts
something the code does not do. What correct looks like: read
`last_error()` in the NULL branch and fold it into the error detail.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| last_error | 50-62 | REAL | — | POSIX-correct clear-on-read; lossy CStr -> String ok |
| DynamicLibrary::open | 92-120 | REAL | — | executed on this host (libpulse.so.0); interior-NUL candidate handled; empty-candidates detail |
| DynamicLibrary::symbol | 129-149 | REAL | S4 | executed (~25 PA symbols); F-002 dlerror diagnosis dropped |
| DynamicLibrary::name | 151-155 | REAL | — | pure accessor, asserted by the host-truth test |
| Drop for DynamicLibrary | 158-170 | REAL | — | dlclose exactly once, null-guarded; ran at test end |
| unsafe impl Send | 64-76 | REAL-UNPROVEN | — | documented rationale (glibc loader thread-safe); no concurrent caller exists today |

## 5. Test honesty
No tests inside dl.rs. The exercising tests live in
tests/linux_integrations.rs (not my file): both pulse tests are honest
host-truth probes and I ran them. One gap for that file's owner:
`pulse_probe_reports_this_hosts_truth`'s success branch
(linux_integrations.rs:397-404) iterates `for desc in &list { assert!(!desc.name.is_empty()) }`
— an empty list passes vacuously, so a regression where a serverless host
wrongly got `Ok(vec![])` from enumerate() would not be caught. The
ServiceUnavailable branch is what this host takes (verified by my run +
worklog.md:564 "PulseAudio dlopen probe loads libpulse but no server").

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "dlopen … at runtime, so a machine without PulseAudio still runs Lumen" | dl.rs:5-7 | TRUE mechanism / FALSE reachability | mechanism executed here; but no shipped binary links platform-linux (F-001) |
| "merged into libc since glibc 2.34 and still passed -ldl by the linux-gnu std" | dl.rs:8-10 | REAL-UNPROVEN | linking works (tests compile+run); glibc history not re-verified |
| "closes it exactly once on drop" | dl.rs:17-18 | REAL | Drop impl dl.rs:158-170; ran |
| "dlsym … NULL result" -> SymbolMissing | dl.rs:127-128 | REAL | code path exists; NULL branch not hit on this host (all symbols present) |

## 7. Dead / duplicated / unreachable code
No dead code in-file. Crate-level: unreachable from shipped entry points
(F-001). The identical seam pattern is triplicated across platform-macos /
platform-linux / platform-windows (Lead's L5 unification pending, per
worklog.md:563).

## 8. Dependencies & flags
std-only (no libc crate — raw extern "C" dlopen/dlsym/dlclose/dlerror
declared at dl.rs:42-47, no #[link] needed since std passes -ldl on
linux-gnu). `#![allow(unsafe_code)]` module carve-out (dl.rs:24), consistent
with the crate's deny(unsafe_code) posture.

## 9. Open questions for the next agent
- pulse_ffi.rs (not my file): verify each `sym!(lib, "pa_*", unsafe extern "C"
  fn …)` prototype against pulse headers — the transmute at pulse_ffi.rs:278
  is only sound if signatures match; my run proves all symbols RESOLVE, not
  that the types are right (though the no-server connect path did execute
  mainloop_new/free/get_api and context functions for real).
- Whoever audits linux_integrations.rs: the vacuous-empty-list branch noted
  in §5.
