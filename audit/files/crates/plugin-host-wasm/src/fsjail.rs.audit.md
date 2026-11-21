# AUDIT: crates/plugin-host-wasm/src/fsjail.rs
| Field | Value |
|---|---|
| Auditor | W12-A05 |
| Date | 2026-11-21 |
| Lines / functions | 381 / 12 (1 struct + 5 methods + 1 free fn + 4 test helpers + 7 tests) |
| File verdict | REAL (canonicalize-based symlink-safe jail with honest, documented TOCTOU narrowing; all 7 tests green in the baseline repair run) |
| Reachable from shipped entry point? | NO — plugin-host-wasm has no shipped consumer (rg over apps/*/Cargo.toml: zero hits). Reached by the wasm host's `fs_open` import (imports.rs, W12-A02) and exercised end-to-end by tests/plugin_sandbox_security.rs (9/9 green in repair run). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | Read fsjail.rs 1-381 (full); `rg -ln "PathJail|fsjail" crates apps` (callers: state.rs, lib.rs, tests/plugin_sandbox_security.rs, tests/plugin_examples_conformance.rs); `rg -n "PathJail|verify_open|fs_open" crates/plugin-host-wasm/src/state.rs crates/plugin-host-wasm/src/imports.rs | head` (resolve→open→verify_open sequence confirmed in the host's fs_open); baseline `orchrun-test-workspace-nofailfast.log.txt:4506-4515` (fsjail::tests 7/7 ok), `:4601-4608` (plugin_sandbox_security green, incl. plugin_sandbox_escape at 4609). Quotes verified with the Read tool. |

## 1. One-paragraph verdict
A genuinely sound read-only path jail: roots canonicalize at construction (a dead root is a typed error, not a silent no-op), `resolve` canonicalizes every candidate so symlink escapes land outside the prefix check, `starts_with` on `Path` is component-aware (the `/root` vs `/root-evil` confusion is impossible and pinned by test), and the `verify_open` re-canonicalization narrows the classic check→open TOCTOU to the re-resolution window — with the residual window and its openat/O_NOFOLLOW closure honestly recorded as `[VERIFY]`/ADR-blocked in the module docs rather than claimed closed. The tests touch the real filesystem with unique temp dirs and cover inside/symlink-escape/traversal/sibling-prefix/empty-jail/dead-root/swap/no-swap. Real defects are small: the multi-root "first hit wins" contract is violated by an early hard denial when an earlier root's candidate resolves (via symlink) outside its own root, the `shape()` doc comment says two components while the code takes three, and the Windows arms of two tests assert nothing meaningful about symlinks.

## 2. What actually works (verified)
— Verified by baseline `orchrun-test-workspace-nofailfast.log.txt:4506-4515` → all 7 `fsjail::tests::*` `ok` (inside_reads_resolve, symlink_escape_is_denied, component_aware_prefix, empty_jail_refuses_everything, nonexistent_root_is_a_typed_error, swap_between_resolve_and_open_is_denied, unswapped_path_passes_reverification).
— Verified by baseline `:4609` → `plugin_sandbox_escape ... ok` — the end-to-end test that plants a symlink in the granted media dir and drives a REAL wasm plugin whose `fs_open` walks resolve→open→verify_open; the jail holds.
— Verified by read: `Path::starts_with` compares components (std semantics), so the module's `/root-evil` claim (136-137) is correct, and `canonicalize` resolves every component — the escape-detection reasoning (101-103) is sound.
— Verified by mutation reasoning: drop the `verify_open` call ordering and `swap_between_resolve_and_open_is_denied` (326-360) fails — it performs a real remove+symlink swap between `resolve` and `verify_open` and asserts the typed denial.

## 3. Findings

### F-001 [S3]
Where: crates/plugin-host-wasm/src/fsjail.rs:100-118 (with doc 65-67)
Code:
```
        for candidate in candidates {
            // Canonicalize resolves ALL symlink components; a link
            // pointing outside the root lands outside and fails the
            // prefix check below.
            match std::fs::canonicalize(&candidate) {
                Ok(canonical) => {
                    if self.under_root(&canonical) {
                        return Ok(canonical);
                    }
                    // Escaped via symlink: fall through to denial with
                    // the shape of the attempt.
                    return Err(HostError::PermissionDenied {
```
What is wrong: the documented contract is "Relative paths try each root in order (first hit wins)" (65-67), but the loop RETURNS a hard denial the moment any root's candidate canonicalizes outside that root — the remaining roots are never tried. With roots [A, B] where A holds a symlink `song.lrc` → outside and B holds a real in-jail `song.lrc`, `resolve("song.lrc")` denies instead of falling through to B's legitimate hit. (The `NotFound` arm, 119, correctly continues.)
Why it matters: a user who granted two directories gets a spurious denial caused entirely by junk in the OTHER granted directory — and the failure mode is indistinguishable (same typed error) from a genuine escape attempt, so the plugin author cannot tell a hostile layout from a harmless one. Security posture (fail-closed) is fine; the contract drift and diagnosability are the defect.
What correct looks like: either continue to the next root when a candidate resolves outside ITS root (matching the documented first-hit-wins), or keep the hard denial and fix the doc + error detail to say "an earlier root resolved outside itself; no further roots were tried".
Confidence: HIGH — Verification: read (control flow traced; no test constructs a multi-root mixed case — checked all 7 tests).
### F-002 [S4]
Where: crates/plugin-host-wasm/src/fsjail.rs:178-186
Code:
```
/// A path shape for events: the first two components plus an
/// ellipsis (doc 06 §5.3: never raw buffers, never full paths that
/// could carry secrets).
fn shape(path: &Path) -> String {
    let comps: Vec<String> = path
        .components()
        .take(3)
```
What is wrong: the doc comment says "the first two components"; the code takes THREE (`.take(3)`). One of the two is wrong, and since this is the privacy redaction function (doc 06 §5.3: shapes must not carry secrets), an off-by-one in the disclosure width deserves the comment and code to agree.
Why it matters: the redaction width is a privacy contract; a reviewer trusting the comment audits a different disclosure than ships.
What correct looks like: decide the width (three components seems the intent) and fix the comment; better, make it a named const with the doc reference.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/plugin-host-wasm/src/fsjail.rs:262-272, 361-366
Code:
```
        #[cfg(windows)]
        {
            // Junction-free environments: skip the symlink fixture,
            // the prefix logic is covered by the sibling test.
            ok(
                std::fs::copy(secret.join("secret.txt"), media.join("escape.txt"))
                    .map_err(|e| e.to_string()),
            );
        }
```
(and the swap test's Windows arm, 361-366):
```
        #[cfg(windows)]
        {
            // Junction-free environments: the prefix re-check itself is
            // cross-platform; the swap fixture needs symlink privilege.
            assert!(jail.verify_open(&resolved).is_ok());
        }
```
What is wrong: on Windows, `symlink_escape_is_denied` never tests a symlink escape (it copies a file and asserts nothing about it), and `swap_between_resolve_and_open_is_denied` asserts the OK path — the security property both tests exist for is silently untested on that platform. The comments are honest about why (symlink privilege), but no `#[ignore]`/skip marker surfaces the gap in test output.
Why it matters: a Windows-targeting contributor sees "all tests green" while the jail's central security property (symlink/traversal denial) has zero Windows coverage.
What correct looks like: at minimum emit an eprintln/skip note, or gate on symlink-availability detection rather than platform; ideally use junctions (creatable without privilege) for the Windows arm.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/plugin-host-wasm/src/fsjail.rs:73-93
Code:
```
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err(HostError::PermissionDenied {
                detail: "empty path".to_string(),
            });
        }
        // Length guard before touching the filesystem (dirty-world
        // inputs; D-2 bounded memory).
        if trimmed.len() > 4096 {
```
What is wrong: the empty-path and 4096-byte guards have no unit tests (no test in 241-380 drives an empty string, whitespace-only string, or an over-long path), and the 4096 constant is unexplained (it is the classic PATH_MAX, but nothing says so).
Why it matters: the two cheapest denial paths in the file are unpinned; a regression that reorders trim/len handling before the roots check would pass the suite.
What correct looks like: three one-line tests (empty, `"   "`, 4097×'a') asserting PermissionDenied, and a comment naming PATH_MAX as the source of 4096.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PathJail::new | 42-58 | REAL | — | canonicalizes each root; dead root ⇒ typed Io error (tested 316-323); duplicate roots harmless (prefix check idempotent) |
| roots | 61-63 | REAL | — | display/events accessor |
| PathJail::resolve | 73-133 | REAL | S3 | trim/empty/4096 guards (untested = F-004); absolute single-candidate vs relative per-root join; early hard denial on out-of-root resolution = F-001; NotFound continues; other io errors typed-denied |
| under_root | 138-140 | REAL | — | `Path::starts_with` component-aware — claim verified against std semantics; tested by component_aware_prefix |
| verify_open | 160-175 | REAL | — | re-canonicalize + prefix recheck; fail-closed on any error; honest TOCTOU residual documented 150-154; tested both arms (326-380) |
| shape | 181-192 | REAL | S4 | doc/code width mismatch = F-002; empty path ⇒ "…" |
| TempDir (test helper) | 208-232 | REAL | — | pid+nanos uniqueness; Drop self-cleans; no symlink-attack concern (own temp namespace) |
| write_file (test helper) | 234-239 | REAL | — | plain create+write |
| inside_reads_resolve | 241-252 | REAL | — | relative AND absolute both asserted |
| symlink_escape_is_denied | 254-290 | REAL | S4 | real unix symlink + parent traversal; Windows arm asserts nothing = F-003 |
| component_aware_prefix | 292-305 | REAL | — | `/root` vs `/root-evil` — the exact byte-prefix trap, denied |
| empty_jail_refuses_everything | 307-314 | REAL | — | no-roots denial |
| nonexistent_root_is_a_typed_error | 316-323 | REAL | — | construction honesty |
| swap_between_resolve_and_open_is_denied | 326-367 | REAL | S4 | real remove+symlink swap; asserts typed denial AND the message text; Windows arm asserts the OK path = F-003 |
| unswapped_path_passes_reverification | 369-380 | REAL | — | the happy twin — narrowing must not break legitimate opens |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| inside_reads_resolve | YES | both resolution forms asserted ok + suffix |
| symlink_escape_is_denied | YES (unix) | real symlink planted; denial asserted typed — a prefix-check removal fails the `let Err` binding |
| component_aware_prefix | YES | byte-prefix confusion would resolve ok and hit the `unreachable!` |
| empty_jail_refuses_everything | YES | roots-empty guard removal ⇒ canonicalize of "anything" ⇒ likely NotFound-continue ⇒ final denial — WEAK: passes either way through the final arm. Still pins the typed shape. |
| nonexistent_root_is_a_typed_error | YES | construction error asserted |
| swap_between_resolve_and_open_is_denied | YES (unix) | deterministic swap fixture; asserts both the denial and the reason string |
| unswapped_path_passes_reverification | YES | verifies the narrowing is not a blanket deny |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a granted path resolves only if its canonical form falls under one of the canonical jail roots" | fsjail.rs:4-6 | REAL | resolve 100-126 + under_root; symlink test green |
| "`std::fs::canonicalize` resolves symlinks on every path component, so a symlink pointing outside the root canonicalizes outside the root and is denied" | fsjail.rs:6-9 | REAL | symlink_escape_is_denied (unix) + plugin_sandbox_escape (end-to-end, repair run 4609) |
| "Relative paths try each root in order (first hit wins)" | fsjail.rs:65-67 | FALSE for out-of-root resolutions | F-001: early hard denial stops the search |
| "`fs_open` … re-checks the jail prefix on the same path before any byte is read" | fsjail.rs:14-17 | REAL | verify_open 160-175 + swap test + fs_open_slurp_is_bounded (repair run 4603) |
| "the first two components plus an ellipsis" | fsjail.rs:179-180 | FALSE | F-002: `.take(3)` |

## 7. Dead / duplicated / unreachable code
None. Every method has a caller (state.rs/imports.rs via the host; tests). The `ok` test helper duplicates package.rs's — house style ("media-probe precedent", 199-200), acceptable.

## 8. Dependencies & flags
std-only (`std::path`, `std::fs` — canonicalize, symlink only under `#[cfg(unix)]` in tests). `crate::error::HostError` (W12-A10). No libc (MP §9 rule), which is exactly what blocks O_NOFOLLOW — the module doc records this trade honestly.

## 9. Open questions for the next agent
- W12-A02 (imports.rs `fs_open`): confirm the resolve→open→verify_open ordering claim (I confirmed the call sites exist by grep but did not audit the ordering inside fs_open — that file is yours).
- W12-A09 (install.rs/state.rs): does anything persist jail roots across sessions where a root could be deleted between construction and use (roots canonicalize once at PathJail::new — a root deleted later makes resolve return NotFound-denials, which is safe but maybe surprising)?
