# AUDIT: crates/platform-macos/src/error.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-21 |
| Lines / functions | 114 / 4 rows (enum + Display + Error impl; no tests in-file) |
| File verdict | REAL / UNREACHABLE (whole crate: no dependents) |
| Reachable from shipped entry point? | NO. rg over every Cargo.toml outside the crate: zero dependents (`apps/cli`, `apps/desktop-tauri`, no crate depends on `platform-macos`). The crate is compiled as a workspace member (baseline build #1 CLEAN) and its seam tests run (16/16 green, nofailfast :3687–3712), but no shipped binary links it. In-crate the vocabulary is live: `PlatformError` is constructed/propagated by audio.rs, capability.rs, dialog.rs, gpu.rs and the `src/macos/` FFI modules (lib.rs:71 use; :157/:182/:206 `NotCompiledForThisOs { crate_os: "macos" }` fallbacks). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `diff` of error.rs against platform-linux and platform-windows (byte-identical except the card number in line 1: 7.A2/7.A1 vs 7.A3); `rg -l "platform-macos"` over all Cargo.tomls + sources (no dependents; cross-mentions are doc comments in the sibling crates' lib.rs:5); `rg -n "PlatformError"` within the crate (used by 9 modules); `rg -n "platform_macos"` in nofailfast baseline (lib 0 tests — all tests live in tests/seam_behaviour.rs, 16/16 green incl. `platform_errors_render_their_details` :3696); read the rendering test (seam_behaviour.rs:351–381); `sed -n` on quoted ranges. |

## 1. One-paragraph verdict
A clean, small, typed error vocabulary — eight variants, structured payloads, honest Display, no bare strings — that is used consistently inside the macOS adapter and rendered correctly by the crate's seam tests (4 of 8 variants asserted, green in baseline). Two structural problems: the entire 114-line file is byte-identical in `platform-linux` and `platform-windows` (the only diff is the card number in line 1 — verbatim triplication with the unification deferred by `[PROPOSAL]`), and the copy-paste has already produced a self-referential doc bug: platform-macos's header says the shape "is mirrored in platform-windows/platform-macos", naming itself and omitting platform-linux. Additionally the whole platform-macos crate is unreachable from any shipped binary (no dependents), so none of this vocabulary can reach a user today.

## 2. What actually works (verified)
— Typed vocabulary with structured payloads (variant per failure class, `Option<i64>` raw codes, `&'static str` APIs/symbols): read in full; constructed at real call sites (lib.rs:157/182/206 fallbacks, macos/coreaudio.rs, macos/objc.rs, macos/metal.rs, macos/dialogs.rs).
— `Display` renders each variant's human form with all payload fields: 4/8 variants asserted exact-substring by `platform_errors_render_their_details` (seam_behaviour.rs:351–381), green in the no-fail-fast baseline (:3696).
— `impl std::error::Error` — composes with `anyhow`-style chains if a host ever links the crate.
— The `NotCompiledForThisOs` fallback design: the crate runs on any host and answers the typed reason instead of faking success (`unavailable_audio_devices_returns_the_typed_reason` etc., green :3698–3701).

## 3. Findings
### F-001 [S3]
Where: crates/platform-macos/src/error.rs:1-8
Code:
```
//! The typed platform-error vocabulary (card 7.A3).
//!
//! Every failure a platform seam can produce is one of these variants
//! — never a bare string, never a fake success (MP §8: "errors are
//! data, and the error path is a first-class outcome"). The same
//! shape is mirrored in `platform-windows`/`platform-macos` so the
//! Lead can later unify them into a shared L5 vocabulary crate
//! `[PROPOSAL]`.
```
What is wrong: The file is triplicated verbatim. `diff crates/platform-macos/src/error.rs crates/platform-linux/src/error.rs` → only line 1 differs (`card 7.A3` vs `card 7.A2`); same against platform-windows (`card 7.A1`). 114 lines × 3 of identical enum + Display + Error. The header itself shows the copy-paste scar: written from platform-macos, "mirrored in `platform-windows`/`platform-macos`" names the file it sits in and omits `platform-linux` — the sentence was authored once (in platform-linux) and copied without rereading.
Why it matters: Three identical typed vocabularies are three places a new variant (or a Display fix) must land; the first one-off edit silently diverges the crates whose doc promises "the same shape". The `[PROPOSAL]` defers the fix but does not bound the drift window — and the drift has technically already started (each file cites a different card).
What correct looks like: One `platform-types` (or audio-output-adjacent) crate with the shared vocabulary; the three adapters depend on it. Until then, at least a cross-crate test asserting the three enums' discriminants and Display strings stay identical.
Confidence: HIGH — Verification: diff run (output cited above); headers read in all three files.
### F-002 [S4]
Where: crates/platform-macos/tests/seam_behaviour.rs:352
Code:
```
    let cases: [(PlatformError, &str); 4] = [
```
What is wrong: The rendering test covers 4 of 8 variants (`NotCompiledForThisOs`, `CallFailed { code: Some }`, `InvalidEncoding`, `SymbolMissing`). `LibraryUnavailable` (the `names.join(", ")` formatting), `ServiceUnavailable`, `TimedOut` (the `{deadline_ms} ms` interpolation), `SpawnFailed`, and `CallFailed { code: None }` have zero rendering coverage anywhere.
Why it matters: The Display strings are the "display-safe (V8)" surface this vocabulary advertises; half of them are unasserted, and formatting bugs in the untested half (e.g., the join, the None-code arm) would ship silently.
What correct looks like: Extend the table to all 8 variants + the `code: None` arm (9 cases).
Confidence: HIGH — Verification: test read (:351–381); rg for other Display assertions in the crate — none.
### F-003 [S4]
Where: crates/platform-macos/src/error.rs:14
Code:
```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformError {
```
What is wrong: Nothing in the file — this row records the crate-level status: `platform-macos` has zero dependents (no Cargo.toml in apps/ or crates/ outside the crate references it), so the entire vocabulary is UNREACHABLE from any shipped entry point. The crate's own manifest is honest about its stage (card 7.A3, FFI "compile-verified via cargo check --target aarch64-apple-darwin … no linking, no runtime; Real-device behavior is [VERIFY]"), so this is recorded status, not fraud — but auditors of docs that speak of "platform adapters" should know none of it is linked into a shipped binary.
Why it matters: Verdict bookkeeping — REAL code, UNREACHABLE from shipped entry points.
What correct looks like: The L5 host (desktop-tauri) linking the adapter when its card lands.
Confidence: HIGH — Verification: rg over all manifests (no dependents).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1–8 | REAL | S3 | F-001: triplication + self-referential mirror sentence. |
| `PlatformError` (enum, 8 variants) | 14–74 | REAL | S4 | F-003: crate unreachable from shipped binaries; in-crate usage real (9 modules). |
| `impl Display for PlatformError` | 76–112 | REAL | S4 | F-002: 4/8 variants rendering-tested; all arms read — correct as written (join, Option-code two-arm, ms interpolation). |
| `impl std::error::Error` | 114 | REAL | — | Empty impl, correct. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| seam_behaviour.rs::platform_errors_render_their_details | YES (for 4 of 8 variants) | Exact-substring asserts on payload interpolation; green :3696. |
| (gap) LibraryUnavailable / ServiceUnavailable / TimedOut / SpawnFailed / CallFailed{None} rendering | NO TEST | F-002. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every failure a platform seam can produce is one of these variants — never a bare string" | error.rs L3–5 | REAL (within the crate) | 9 modules construct only PlatformError (rg); seam tests assert typed reasons everywhere. |
| "The same shape is mirrored in platform-windows/platform-macos" | error.rs L6–7 | REAL but self-mangled | diff proves byte-identity with BOTH siblings; sentence names itself and omits platform-linux (F-001). |
| "errors are data, and the error path is a first-class outcome" (MP §8 quote) | error.rs L4–5 | REAL | Structured payloads; PartialEq for testability. |

## 7. Dead / duplicated / unreachable code
- Duplication: the entire file × 3 (F-001).
- Unreachable: the whole crate from shipped entry points (F-003) — in-crate usage is real.

## 8. Dependencies & flags
std only (fmt). No unsafe in this file (the crate's `extern "C"` surface lives in src/macos/, another agent's scope). Crate compiled clean in baseline build #1; seam tests 16/16 green (nofailfast :3687–3712). No cfg(target_os) gates in this file — the vocabulary is host-independent by design (the NotCompiledForThisOs variant carries the target).

## 9. Open questions for the next agent
- To the platform-linux/windows auditors (W03 wave peers): your error.rs is the same file — F-001 applies verbatim; consider a shared finding.
- To whoever audits `src/macos/` FFI modules: the `[VERIFY]` real-device behavior gap (no macOS SDK in sandbox) is the crate's largest honesty debt — compile-checked `objc_msgSend` declarations with no runtime proof.
- To the Lead: the unification `[PROPOSAL]` (shared L5 vocabulary crate) should land before the fourth platform (web?) copies the file again.
