# AUDIT: crates/platform-linux/src/linux/dl.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-21 |
| Lines / functions | 170 / 8 items (const, extern block, last_error, DynamicLibrary + Send/Debug, open, symbol, name, Drop) |
| File verdict | REAL (executed against the real loader in baseline) — but the crate is UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO from `lumen`/`lumen-tauri` (no manifest depends on platform-linux — rg over **/Cargo.toml: only its own). Executed as a workspace member's test: this sandbox HAS libpulse.so.0 (ldconfig confirmed), and tests/linux_integrations.rs `pulse_probe_reports_this_hosts_truth` performs a REAL successful `dlopen` through this module (asserts `loaded_from() == "libpulse.so.0"`), plus symbol resolution via pulse_ffi::load — 13/13 ok (orchrun log 3638+, `test result: ok. 13 passed`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg `platform-linux` in **/Cargo.toml (dependents: none) · rg `DynamicLibrary\|dl::` in crate (users: pulse.rs:47/92/355, pulse_ffi.rs:30/267) · ldconfig -p \| grep pulse (libpulse.so.0 present) · read tests/linux_integrations.rs:383-420 (real-dlopen test) · sed -n quotes verified · baseline orchrun log (linux_integrations 13/13 ok incl. `pulse_probe_reports_this_hosts_truth`, `pulse_subscribe_without_a_server_is_the_typed_error`) |

## 1. One-paragraph verdict
The unsafe surface is small, disciplined, and — unusually for this repo —
actually executed against the real OS loader in the baseline: the sandbox's
libpulse.so.0 is dlopened through `DynamicLibrary::open` and its symbols
resolved through `symbol()` by a passing integration test. The POSIX hygiene
is mostly right (dlerror cleared before each call, interior-NUL candidates
refused, handle closed exactly once with a defensive null-out, Send justified
per glibc thread-safety). Two defects: the `symbol()` SAFETY comment promises
a dlerror consultation that the code never performs (the loader's diagnosis is
discarded on the NULL path, and the dlsym-NULL-ambiguity is misclassified),
and the entire crate is unreachable from every shipped binary — the same
orphaned-adapter pattern as platform-windows.

## 2. What actually works (verified)
— Real dlopen execution: `pulse_probe_reports_this_hosts_truth` (tests/linux_integrations.rs:383) asserts `loaded_from() == "libpulse.so.0"` — that string can only come from a successful `DynamicLibrary::open` on this host (ldconfig: `/lib/x86_64-linux-gnu/libpulse.so.0`). Passed in baseline (orchrun log, linux_integrations 13/13).
— Symbol resolution: pulse_ffi.rs:267 `load(lib)` resolves the whole PulseApi through `symbol()`; the connect-phase test (`pulse_subscribe_without_a_server_is_the_typed_error`) proves the resolved symbols are callable (typed ServiceUnavailable without a server).
— POSIX error-clearing discipline: `let _ = last_error()` before both `dlopen` (dl.rs:105) and `dlsym` (dl.rs:140) — the reset-before-call requirement is honored.
— Interior-NUL candidates: refused as a failed candidate with a precise diagnosis (98-101) instead of truncating the path into libc.
— Ownership: Drop closes exactly once and nulls the handle (158-169); `open`'s error carries every candidate name plus the last loader diagnosis (116-119) — matching its doc.
— RTLD_NOW_LOCAL = 0x2: correct for glibc (RTLD_NOW=2, RTLD_LOCAL=0) and musl; the eager+private choice is documented (30-33).

## 3. Findings

### F-001 [S3] — the crate is orphaned: no shipped entry point can reach this loader wrapper
Where: crates/platform-linux/src/linux/dl.rs:68-71 (the type nothing outside the crate consumes transitively)
Code:
```rust
pub struct DynamicLibrary {
    handle: *mut c_void,
    name: String,
}
```
What is wrong: rg over every Cargo.toml in the repository: `platform-linux`
has zero dependents (only its own manifest names it). The `lumen` CLI's audio
lane is the null device (`engine-info` feature `audio.null-device`, baseline
INDEX) and its device layer lives in audio-output; no app references
`platform_linux`. dl.rs's only consumers are the crate's own pulse modules,
which themselves are only reached by the crate's own tests. The same
orphaned-adapter pattern I filed against platform-windows/src/lib.rs (F-001
there) — this is the Linux instance.
Why it matters: the one module in this domain with genuinely executed
unsafe-FFI evidence is dead code from the user's perspective; the Linux
parity column (xtask-parity-report: linux=61/252) can count capability that
no shipped Linux binary delivers.
What correct looks like: wire `LinuxPlatform::discover()` into the CLI (or
desktop shell) behind cfg(target_os = "linux"), or record the crate as a
not-yet-wired library in KNOWN-LIMITS.md.
Confidence: HIGH — Verification: read + rg (dependent enumeration exhaustive).

### F-002 [S4] — `symbol()`'s comment claims a dlerror consultation the code never performs; the dlsym NULL ambiguity is misclassified
Where: crates/platform-linux/src/linux/dl.rs:136-147
Code:
```rust
        // SAFETY: `self.handle` is a live dlopen handle (owned,
        // not yet closed) and `symbol` is NUL-terminated. dlsym
        // returns NULL for "not found" — checked, with dlerror
        // consulted for the diagnosis.
        let _ = last_error(); // clear stale error (POSIX)
        let address = unsafe { dlsym(self.handle, symbol.as_ptr()) };
        if address.is_null() {
            return Err(PlatformError::SymbolMissing {
```
What is wrong: The comment says "with dlerror consulted for the diagnosis" —
the NULL branch never calls `last_error()`; the loader's diagnosis (e.g.
"undefined symbol") is discarded, and `SymbolMissing` carries only
library+symbol names. Additionally, the POSIX-correct disambiguation is
absent: a dlsym that returns NULL *with dlerror() == NULL* means the symbol
exists and its value is NULL — this code would report it missing. For
libpulse's function symbols the ambiguity is unreachable (functions are never
NULL), so no behavioral bug on the wired path — but the SAFETY comment
contradicts the code, and the cheap diagnosis is thrown away on the exact
path where a maintainer debugging a version-skewed libpulse needs it.
Why it matters: a false SAFETY comment on an unsafe block is worse than no
comment — auditors and future maintainers trust it.
What correct looks like: `let detail = last_error();` in the NULL branch and
include it in `SymbolMissing` (or at least fix the comment); the
dlerror-is-None case then naturally distinguishes "exists, value NULL".
Confidence: HIGH — Verification: read (both call sites and the error type);
UNVERIFIED: none.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs + `#![allow(unsafe_code)]` | 1-24 | REAL | — | safety-scoped module banner; allow is scoped to this file only |
| const RTLD_NOW_LOCAL | 30-33 | REAL | — | 0x2 correct on glibc/musl; conservative choice documented |
| `unsafe extern "C"` block | 35-47 | REAL | — | ABI declarations with the four invariants listed; no `safe` exports |
| last_error | 49-62 | REAL | — | clears per POSIX; lossy CStr read; never frees libc storage ✓ |
| struct DynamicLibrary + `unsafe impl Send` | 64-76 | REAL | — | Send justified (glibc loader thread-safe + single owner); !Sync retained (conservative, correct) |
| Debug for DynamicLibrary | 78-84 | REAL | — | name only — no raw pointer leak in debug output |
| DynamicLibrary::open | 86-120 | REAL | — | candidate loop, NUL refusal, dlerror reset before each call, honest synthetic detail when dlerror is empty; error names all candidates ✓ executed (real dlopen in baseline) |
| DynamicLibrary::symbol | 122-149 | REAL | S4 | F-002: comment-code contradiction + missing diagnosis on NULL |
| DynamicLibrary::name | 151-155 | REAL | — | pure accessor, #[must_use] |
| Drop for DynamicLibrary | 158-170 | REAL | — | close exactly once + defensive null-out |

## 5. Test honesty
No unit tests in-file. Integration coverage (baseline, all ok):
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| linux_integrations `pulse_probe_reports_this_hosts_truth` (383) | YES | performs the real dlopen + full symbol load; asserts the real SONAME and the typed connect behavior — a broken open/symbol (bad flags, wrong clearing, NULL mishandling on success) fails it |
| linux_integrations `pulse_subscribe_without_a_server_is_the_typed_error` (423) | YES | resolves and CALLS real pulse symbols; connect failure is typed |
| seam_behaviour suite (14 tests) | YES | typed fallbacks/capability vocabulary (crate-level) |
Coverage gap: `open`'s failure arm (candidate loop exhaustion →
LibraryUnavailable with the loop's last diagnosis) is only taken on hosts
without libpulse — this sandbox always takes the success arm, so the
candidate-iteration and synthetic-detail paths are UNEXECUTED in the
committed baseline (read-verified only). Likewise `symbol`'s NULL arm is
unexecuted here (every pulse symbol resolves).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "lets the PulseAudio integration degrade to a typed Unavailable instead of a link-time dependency" | dl.rs:1-7 | REAL | pulse.rs:355 `DynamicLibrary::open(&PULSE_SONAMES)?`; no link-time dep in Cargo.toml (audited W01-A02) |
| "Every invariant is written next to its block" | dl.rs:16-17 | PARTLY FALSE | the `symbol` block's stated dlerror consultation is not performed (F-002) |
| "closes it exactly once on drop" | dl.rs:17-18 | REAL | Drop 158-169 + single-owner type |
| "`dlsym` returns NULL for 'not found' — checked, with dlerror consulted for the diagnosis" | dl.rs:138-139 | FALSE | no dlerror call in the NULL branch (F-002) |
| "glibc's `dlerror` results are read before any other dl* call" | dl.rs:40-41 | REAL | reset-before-call at 105 and 140 |

## 7. Dead / duplicated / unreachable code
- Crate-level orphan (F-001): dl.rs + pulse/pulse_ffi are dead from the
  shipped-binary perspective.
- Mirrors platform-macos (Auditor note: same pattern filed for
  platform-windows in this wave; a consolidated wave-level finding is
  warranted).

## 8. Dependencies & flags
std only (extern "C" to the platform loader); no crates, no features.
`#![allow(unsafe_code)]` is module-scoped, with the workspace
`deny(unsafe_code)` satisfied elsewhere. Compiled and tested in the default
workspace lanes (Linux host).

## 9. Open questions for the next agent
- pulse.rs/pulse_ffi.rs auditor (later wave): the transmute step `symbol()`
  enables — verify each function pointer's signature against the real
  PulseAudio headers (the doc says the per-API modules own that invariant).
- Whoever owns KNOWN-LIMITS/parity: record the three orphaned platform-*
  crates (linux/windows/macos) in one place; today the parity columns can
  silently count them.
