# AUDIT: crates/platform-macos/src/macos/objc.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-21 |
| Lines / functions | 344 / 16 units (1 extern block with 16 fns, 8 safe wrappers, 1 struct + 2 impls) |
| File verdict | REAL-UNPROVEN (genuine, well-disciplined libobjc FFI; no committed darwin compile evidence; runtime never executed; orphan crate) |
| Reachable from shipped entry point? | NO — platform-macos is an orphan (W01: audit/files/crates/platform-macos/Cargo.toml.audit.md); this module serves dialogs.rs/metal.rs/coreaudio.rs inside the crate only. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `sed -n 1,344p` (full file, twice); Read-verification of quotes; per-helper caller scan (`rg -l` each of the 12 msg_send aliases + retain/nsstring helpers across the crate — no dead aliases); baseline scan for darwin evidence (none). |

## 1. One-paragraph verdict
This is the cleanest hand-rolled ObjC runtime binding I have audited in this
repo: `objc_msgSend` is pinned per signature via `#[link_name]` aliases (the
one sound way to call the variadic dispatcher from Rust), the stret caveat is
documented and holds (no large-struct returns are declared), ownership rules
are written down per category (alloc/new/copy/retain = ours; factory results =
autoreleased; class/selector lookups = runtime-owned) and every call site I
traced in dialogs.rs honors them; interior NULs are rejected; the
AutoreleasePool pairs alloc/init with drain on Drop and releases the alloc when
init fails; the pool struct holds raw pointers so it cannot cross threads
(pools are per-thread — the type system enforces it). Two findings: the same
unbacked "compile-verified for darwin targets" claim as its siblings, and one
doc/code contradiction on the NULL path of `nsstring_to_string`. No memory
safety defect traced.

## 2. What actually works (verified)
— Per-signature alias pattern: every declared alias (id; id,id; id,u64; id,cstr; id,ptr,u64; →u64; →i64; →bool; void; void,id; void,bool; void,f64,f64) matches a real call-site shape — cross-checked against dialogs.rs (runModal i64, count u64, objectAtIndex: id_u64, initWithObjects:count: id_ptr_u64, BOOL setters) and metal.rs usage (`rg -l` per alias).
— `AutoreleasePool` non-Send by construction: struct holds `ObjcId`/`ObjcSel` raw pointers (`pub type ObjcId = *mut c_void`, line 46) — Rust auto-traits deny cross-thread moves, which is exactly the per-thread pool requirement.
— No dead aliases: `msg_send_id_cstr` is consumed inside this module by `nsstring_from_utf8`; every other alias has dialogs.rs or metal.rs callers (verified per-symbol rg).

## 3. Findings

### F-001 [S3]
Where: crates/platform-macos/src/macos/objc.rs:22-24
Code:
```
//! **Evidence status:** compile-verified via `cargo check --target
//! aarch64-apple-darwin` and `x86_64-apple-darwin`; runtime NOT
//! exercised in this sandbox (no macOS SDK) — `[VERIFY]`.
```
What is wrong: identical to dialogs.rs F-001 — an executed-evidence claim with
no committed run. audit/runs/baseline/ (26 wave-0 runs + 6 orchestrator runs)
contains no darwin-target check; the single cross-target run is
orchrun-check-platform-windows.log.txt. W01's crate-level audit
(audit/files/crates/platform-macos/Cargo.toml.audit.md F-003) reached the same
verdict.
Why it matters: this module is the ABI foundation for every macOS integration
in the crate (dialogs, Metal, CoreAudio); its only typecheck assurance is an
unverifiable sentence, and unlike dialogs.rs it declares `#[link(name =
"libobjc")]` and 16 extern symbols that only a real darwin check can validate.
What correct looks like: committed `cargo check -p platform-macos --target
aarch64-apple-darwin` + `x86_64-apple-darwin` logs (REQUEST-ORCH-RUN logged).
Confidence: HIGH — Verification: read + baseline scan (evidence absence
verified; the check's pass/fail UNVERIFIED).

### F-002 [S4]
Where: crates/platform-macos/src/macos/objc.rs:225-235
Code:
```
/// # Errors
/// [`PlatformError::InvalidEncoding`] when the string cannot be read
/// (NULL pointer, or bytes that are not UTF-8).
```
vs the body:
```
pub unsafe fn nsstring_to_string(string: ObjcId) -> Result<String, PlatformError> {
    if string.is_null() {
        return Ok(String::new());
    }
```
What is wrong: the doc contract says a NULL pointer is an error
(`InvalidEncoding`); the code returns `Ok(String::new())` for NULL. A caller
who trusted the doc would treat `Ok` as "a real string was read" and could
surface an empty filename/path where the OS actually returned nothing.
Why it matters: in the one shipped-shape caller (dialogs.rs `url_path`), the
empty result is caught two lines later by an explicit `is_empty` check — so no
live bug today — but the contract and the code disagree, which is how future
callers get burned.
What correct looks like: either return `Err(InvalidEncoding)` on NULL as
documented, or fix the doc to "NULL yields an empty string (callers must
treat empty as no-value)".
Confidence: HIGH — Verification: read (both sites).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| extern block (`objc_getClass`, `sel_registerName`, `objc_retain`, `objc_release`, 12 msg_send aliases) | 55-131 | REAL-UNPROVEN | — | `#[allow(clashing_extern_declarations)]` justified in the comment (the lint cannot know the trampoline forwards register args verbatim); every alias shape cross-checked against call sites; stret caveat documented and true of the declared set |
| `class` | 134-148 | REAL-UNPROVEN | — | CString + null-check + typed ServiceUnavailable |
| `sel` | 153-166 | REAL-UNPROVEN | — | sel_registerName + typed refusal |
| `retain` | 171-185 | REAL-UNPROVEN | — | null-check + typed refusal; callers in gpu.rs/events.rs/metal.rs |
| `release` | 190-202 | REAL-UNPROVEN | — | unsafe, null no-op, caller-invariant documented |
| `cstring` | 208-221 | REAL | — | interior-NUL rejection (typed) |
| `nsstring_to_string` | 223-256 | REAL-UNPROVEN | S4 | F-002 NULL doc/code mismatch; UTF8String copy-out before any release — correct lifetime reasoning in the SAFETY comment |
| `nsstring_from_utf8` | 261-283 | REAL-UNPROVEN | — | class method, autoreleased (documented); interior-NUL typed error |
| `AutoreleasePool::new` | 293-328 | REAL-UNPROVEN | — | alloc→init, init-failure releases the alloc (correct), typed errors both steps |
| `Drop for AutoreleasePool` | 330-344 | REAL-UNPROVEN | — | drain pairs with alloc/init on the same thread; struct not Send (raw pointers) — per-thread rule enforced by the type system |

## 5. Test honesty
No tests exist in or for this file (rg over the crate finds no test module
touching objc helpers). The workspace suite cannot execute it (cfg'd out on
Linux). Recorded here; severity folded into F-001's evidence gap.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "objc_msgSend **typed per signature** … the one sound way to call a variadic-ABI dispatcher from Rust" | objc.rs:5-11 | REAL (as technique) | the 12 aliases each pin an exact shape; this is the established pattern (also used by production Rust ObjC bindings) |
| "methods returning large structs need objc_msgSend_stret (x86_64) — this crate declares no such method" | objc.rs:17-21 | REAL | verified: all declared returns are id/u64/i64/BOOL/void; the f64,f64 pair passes in float registers per both Apple ABIs (CGSize) |
| "compile-verified via cargo check --target aarch64-apple-darwin and x86_64-apple-darwin" | objc.rs:22-24 | REAL-UNPROVEN | F-001 — no committed evidence |
| Ownership discipline (alloc/new/copy/retain ours; factory autoreleased; class/sel runtime-owned) | objc.rs:26-36 | REAL (as code) | traced at every dialogs.rs call site in my dialogs.rs audit; retain/release pairing in this file correct |

## 7. Dead / duplicated / unreachable code
- None dead. All aliases have callers (dialogs.rs, metal.rs, or internal).
- Unreachable from shipped entry points (orphan crate — see header).

## 8. Dependencies & flags
- `#![allow(unsafe_code)]` + `#[allow(clashing_extern_declarations)]` — both
  locally justified with reasoning comments (the rare `#[allow]` that earns
  its place). `#[link(name = "libobjc", kind = "dylib")]` — correct dylib on
  macOS.

## 9. Open questions for the next agent
- metal.rs / coreaudio.rs owners: do their call sites honor the autoreleased-
  vs-owned discipline this module documents (e.g., `retain` before storing a
  factory result past the pool)? My dialogs.rs trace says yes for that lane.
- If the darwin check runs: the `msg_send_void_f64_f64` (CGSize) alias is the
  one most likely to differ between aarch64 and x86_64 — watch it.
