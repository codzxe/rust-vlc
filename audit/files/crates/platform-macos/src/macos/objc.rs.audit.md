# AUDIT: crates/platform-macos/src/macos/objc.rs
| Field | Value |
|---|---|
| Auditor | W08-A05 |
| Date | 2026-11-23 |
| Lines / functions | 344 / 10 (extern block w/ 14 fns, class, sel, retain, release, cstring, nsstring_to_string, nsstring_from_utf8, AutoreleasePool::new, Drop::drop) |
| File verdict | REAL-UNPROVEN (real hand-rolled libobjc FFI; never executed anywhere; two soundness/doc defects) |
| Reachable from shipped entry point? | NO — zero workspace crates depend on platform-macos (rg over all Cargo.tomls: no matches outside the crate itself). In-crate callers: macos/metal.rs:50-52 and macos/dialogs.rs:38-41, both behind `cfg(target_os="macos")`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | rg unsafe/SAFETY counts (exit 0); rg platform-macos dependents (exit 1 = none); sed quote-verification (exit 0); baseline cite (no build run — no darwin target in sandbox) |

## 1. One-paragraph verdict
The one file in my set where the unsafe discipline is genuinely close to airtight: all 11 unsafe blocks plus the extern block and both `unsafe fn`s carry local SAFETY comments, and the per-signature `#[link_name = "objc_msgSend"]` aliasing scheme — the module's core trick — is sound for every declared shape (id/pointer/u64/i64/u8/f64 arguments are register-passed identically to a variadic call on both aarch64- and x86_64-apple-darwin; the module's own no-struct-returns claim at lines 22-26 is correct, so no `objc_msgSend_stret` is needed; `msg_send_void_f64_f64` for `setDrawableSize:` (CGSize) is ABI-identical on both targets: two float registers). Defects: `retain()` is a **safe** function that calls `objc_retain` (a write through a caller-supplied raw pointer) — the safe/unsafe boundary is unsound by construction even though all three in-tree callers (metal.rs:249,312,356) pass live objects; `nsstring_to_string`'s doc promises `InvalidEncoding` on NULL but the code returns `Ok("")`; `AutoreleasePool::new` leaks the created pool if the trailing `sel("drain")` registration fails. Nothing here has ever run (Linux sandbox; 16/16 platform-macos baseline tests exercise only the fallback arms, cargo-test-workspace.txt:4004-4042).

## 2. What actually works (verified)
- Unsafe-block audit (hostile focus): 11/11 blocks (lines 143, 162, 184, 208, 241, 249, 271, 304, 315, 319, 340) each preceded by an accurate SAFETY comment — verified by rg -c "unsafe \{" vs "SAFETY:" plus line-by-line read. Every claim in each comment matches the code beneath it (ownership, NUL-termination, copy-before-free, same-thread pairing).
- ABI reasoning verified against Apple ABI knowledge: per-signature msgSend aliases sound for the 10 declared shapes; stret caveat claim correct (returns used: id, u64, i64, BOOL(u8), void — no struct returns); CGSize-as-two-f64 correct on both Apple ABIs.
- `AutoreleasePool` is `!Send` (contains raw pointers) — the per-thread pool discipline is compiler-enforced, verified by auto-trait reasoning.
- Baseline: platform-macos 16/16 tests pass on Linux (cargo-test-workspace.txt:4004-4042) — all on the `#[cfg(not(target_os="macos"))]` fallback arms; this file contributes zero executed coverage.

## 3. Findings
F-001 [S3]
Where: crates/platform-macos/src/macos/objc.rs:172-184
Code:
```
pub fn retain(object: ObjcId) -> Result<ObjcId, PlatformError> {
    // SAFETY: objc_retain on a live object adds one reference the
    // caller must later release exactly once (the owning newtype's
    // Drop does); a NULL input returns NULL — checked.
    let retained = unsafe { objc_retain(object) };
```
What is wrong: `retain` is a **safe** function whose body calls `objc_retain` on an arbitrary caller-supplied `*mut c_void`. `objc_retain` writes to the object (increments the refcount) — passing a dangling or non-object pointer is UB reachable from safe code. The SAFETY comment states the precondition ("on a live object") but the type system cannot see it: the precondition lives in a comment on a function safe callers may call freely. Its sibling `release` (line 202) is correctly `pub unsafe fn` with the invariant in a `# Safety` doc section; `nsstring_to_string` (232) likewise. `retain` must be `unsafe fn` with the same treatment.
Why it matters: unsound safe/unsafe boundary — the crate's exported API can produce UB without any `unsafe` at the call site; any future wiring of platform-macos inherits the hole.
What correct looks like: `pub unsafe fn retain(object: ObjcId) -> ...` with a `# Safety` caller-invariant section.
Confidence: HIGH — Verification: read. (No current in-tree misuse: the only callers are metal.rs:249, 312, 356, all passing live MTLDevice/CAMetalLayer/CAMetalDrawable pointers.)

F-002 [S3]
Where: crates/platform-macos/src/macos/objc.rs:225-235
Code:
```
/// # Errors
/// [`PlatformError::InvalidEncoding`] when the string cannot be read
/// (NULL pointer, or bytes that are not UTF-8).
///
/// # Safety (caller invariant)
/// `string` is a live `NSString` (not NULL-checked callers must pass
/// non-NULL), valid for the duration of this call.
pub unsafe fn nsstring_to_string(string: ObjcId) -> Result<String, PlatformError> {
    if string.is_null() {
        return Ok(String::new());
    }
```
What is wrong: the doc promises `InvalidEncoding` "when the string cannot be read (NULL pointer, ...)"; the code returns `Ok(String::new())` for NULL. The doc is false, and the actual behavior swallows a NULL (a failed upstream `msgSend` returning nil) into a silent empty string — the caller cannot distinguish "empty title" from "the NSString read failed".
Why it matters: callers (dialogs.rs:362 reads an NSURL path; metal.rs:142 reads a device name) get "" instead of an error; the misleading doc compounds it.
What correct looks like: either return the documented error, or fix the doc and give callers a way to detect NULL.
Confidence: HIGH — Verification: read (doc and code quoted verbatim).

F-003 [S3]
Where: crates/platform-macos/src/macos/objc.rs:328-331 (with 304-315)
Code:
```
        Ok(AutoreleasePool {
            pool,
            drain: sel("drain")?,
        })
    }
```
What is wrong: if `sel("drain")` fails (registration/OOM), the `?` returns `Err` after the pool was already allocated and initialized (lines 304-315) — the `AutoreleasePool` struct is never constructed, so `Drop` never runs, so the pool is never drained: the pool object leaks and the function's "released exactly once" discipline (module doc lines 32-44) is violated on this path.
Why it matters: error-path leak in the ownership-discipline primitive the whole module's unsafe story rests on; small (selector registration essentially only fails on OOM) but it is exactly the class of bug the module doc promises cannot happen.
What correct looks like: register `alloc`/`init`/`drain` selectors before creating anything, or drain the pool before returning the error.
Confidence: HIGH — Verification: read.
F-004 [S3]
Where: whole file / crates/platform-macos/Cargo.toml
What is wrong: platform-macos has zero dependents in the workspace (rg over every Cargo.toml: no matches). 344 lines of runtime FFI are unreachable from lumen-cli/desktop; the release lane never compiles the `#[cfg(target_os="macos")]` module.
Confidence: HIGH — Verification: ran rg (exit 1, no dependents).
F-005 [S4]
Where: crates/platform-macos/src/macos/objc.rs:28-30
Code:
```
//! **Evidence status:** compile-verified via `cargo check --target
//! aarch64-apple-darwin` and `x86_64-apple-darwin`; runtime NOT
//! exercised in this sandbox (no macOS SDK) — `[VERIFY]`.
```
What is wrong: the claim is recorded (worklog.md:556) but cannot be re-executed here (no rustup, no darwin targets installed) — UNVERIFIED; and `cargo check` proves typechecking only, not linking against libobjc.
Confidence: MEDIUM — Verification: UNVERIFIED (toolchain absent).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| extern block (libobjc + 10 msgSend aliases) | 62-132 | REAL-UNPROVEN | — | ABI declarations correct; SAFETY comment + justified `clashing_extern_declarations` allow; per-shape aliases sound |
| class | 139-151 | REAL-UNPROVEN | — | NULL → typed ServiceUnavailable; interior-NUL rejected |
| sel | 158-170 | REAL-UNPROVEN | — | NULL → typed error |
| retain | 180-193 | REAL-UNPROVEN | S3 | F-001: safe fn over a raw-pointer write — unsound boundary |
| release | 202-211 | REAL-UNPROVEN | — | correctly unsafe, invariant documented, NULL no-op |
| cstring | 215-221 | REAL | — | interior-NUL rejection; host-testable logic, correct |
| nsstring_to_string | 232-255 | REAL-UNPROVEN | S3 | F-002: doc contradicts NULL handling; copy-before-free ordering correct |
| nsstring_from_utf8 | 264-280 | REAL-UNPROVEN | — | autoreleased result documented; NULL checked |
| AutoreleasePool::new | 297-332 | REAL-UNPROVEN | S4 | F-003: drain-selector failure leaks pool; alloc/init failure path correctly releases |
| Drop::drop | 335-344 | REAL-UNPROVEN | — | same-thread drain pairing documented; !Send enforced |

## 5. Test honesty
No tests in this file; crate tests (16 passing, baseline cargo-test-workspace.txt:4016-4034) run only the non-macOS fallback arms (`discover_on_a_foreign_host_reports_wrong_os_everywhere` etc.). Deleting objc.rs entirely would not fail a single test. Mutation verdict: zero coverage of the audited code.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compile-verified via cargo check --target aarch64-apple-darwin and x86_64-apple-darwin" | objc.rs:28-30 | UNVERIFIED | worklog.md:556 records it; not re-runnable (no darwin toolchain) |
| "this crate declares no such method [struct returns]" (stret caveat) | objc.rs:22-26 | REAL | verified: all aliases return id/u64/i64/BOOL/void (lines 87-131) |
| "NULL ... — checked" (retain doc) | objc.rs:182-183 | REAL | line 185-191 checks |
| nsstring_to_string errors with InvalidEncoding on NULL | objc.rs:226-227 | FALSE | code returns Ok("") (233-235) — F-002 |
| "released exactly once (the owning newtype's Drop ...)" ownership discipline | objc.rs:34-37 | FALSE on one path | AutoreleasePool::new leaks on sel("drain") failure (328-331) — F-003 |

## 7. Dead / duplicated / unreachable code
Whole file unreachable from shipped entry points (F-004). No unused aliases: all 10 msgSend shapes consumed by metal.rs/dialogs.rs (verified by import lists at metal.rs:50-52, dialogs.rs:38-41).

## 8. Dependencies & flags
`#![allow(unsafe_code)]` (line 48) — documented SAFETY-SCOPED exception. `#[allow(clashing_extern_declarations)]` (line 74) — justified in-comment. Links `libobjc` dylib. No objc2/objc crates (clean-room rule honored). No cargo features.

## 9. Open questions for the next agent
- metal.rs / macos/dialogs.rs (not my files) inherit F-001/F-002 — check their retain/nsstring call sites for NULL-tolerance assumptions.
- If platform-macos is ever wired in, make `retain` unsafe before review; it is the only safe fn in the crate that can write through a raw pointer.
