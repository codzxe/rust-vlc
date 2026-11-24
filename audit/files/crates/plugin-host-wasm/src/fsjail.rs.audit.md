# AUDIT: crates/plugin-host-wasm/src/fsjail.rs
| Field | Value |
|---|---|
| Auditor | W12-A06 |
| Date | 2026-11-24 |
| Lines / functions | 381 / 6 prod + 4 test helpers + 7 tests |
| File verdict | REAL (executed, hostile focus file) |
| Reachable from shipped entry point? | NO at crate level (zero workspace dependents; see api.rs audit) — but the jail is the security core of the extension lane |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p plugin-host-wasm --lib fsjail` -> 7 passed, 0 failed; read imports.rs:405-540 (fs_open consumer) to verify the resolve→open→verify→bounded-slurp sequence; sed verification of quotes |

## 1. One-paragraph verdict
The path jail is genuinely implemented and hostile-input honest: roots are
canonicalized at construction (nonexistent root = typed error), `resolve`
length-guards (4096), trims, refuses empty paths and empty jails, joins
relative paths to each root, canonicalizes EVERY component (symlink escape and
`../` traversal both land outside the root and are denied), and the prefix
check is component-aware (`/root-evil` does not pass `/root`). The consumer
(fs_open, imports.rs:405-540) opens once, takes metadata from the handle,
re-verifies the jail before reading, and bounds the slurp with `take(cap + 1)`
so a file that grew after open cannot exhaust host memory. I executed all 7
tests — symlink escape, `../` traversal, sibling-prefix, empty jail,
nonexistent root, swap-between-resolve-and-open, and the happy twin — all
pass. The one real weakness is the documented residual TOCTOU: `verify_open`
re-canonicalizes the PATH, not the opened HANDLE, so a swap-back race can
verify an in-jail path while the host reads an outside handle; the module doc
admits this and bounds it by threat model (plugins have no write permission;
no hostile local user in the model).

## 2. What actually works (verified)
— verified by `~/.cargo/bin/cargo test -p plugin-host-wasm --lib fsjail` ->
"test result: ok. 7 passed; 0 failed" (symlink_escape_is_denied,
component_aware_prefix, swap_between_resolve_and_open_is_denied included).
— verified by reading imports.rs:448-513: jail.resolve -> File::open(resolved)
-> jail.verify_open(resolved) -> metadata-from-handle -> is_file check ->
`file.take(cap + 1)` bounded read -> ETooLarge past cap -> handle-table insert
under max_open_handles.

## 3. Findings
F-001 [S3]
Where: crates/plugin-host-wasm/src/fsjail.rs:11-19, 160-184; crates/plugin-host-wasm/src/imports.rs:466-485
Code:
```
//! Known limit (honest, `[VERIFY]`): the canonicalize-then-open
//! sequence has a TOCTOU window — a local process swapping a
//! directory symlink between check and open can redirect the read.
```
and (verify_open body, fsjail.rs:160-162):
```
    pub fn verify_open(&self, claimed: &Path) -> Result<(), HostError> {
        let canonical =
            std::fs::canonicalize(claimed).map_err(|e| HostError::PermissionDenied {
```
What is wrong: verify_open re-resolves the PATH string, not the already-open
handle. Race: t0 resolve admits P; t1 attacker swaps P to an outside symlink;
t2 File::open(P) opens the OUTSIDE file; t3 attacker swaps P back to the
in-jail file; t4 verify_open canonicalizes the CURRENT P — in-jail — and
passes; the read then drains the outside handle. The narrowing catches only
the swap-and-hold variant. Why it matters: it is a real hole against the
stated non-model attacker (a hostile local process), and the code chooses a
weaker primitive than needed (on Unix, fstat/`/proc/self/fd` or openat would
bind verification to the handle). S3 not S1 because the module documents the
window verbatim, the threat model excludes local attackers, and the plugin
itself cannot create symlinks (no write permission anywhere in the ABI).
What correct looks like: verify from the handle (fstat + readlink /proc/self/fd
on unix) or openat(O_NOFOLLOW) behind an ADR, as the doc itself records.
Confidence: HIGH — Verification: read (race reasoned, not executed)

F-002 [S4]
Where: crates/plugin-host-wasm/src/fsjail.rs:264-271, 274-283
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
What is wrong: on Windows, `symlink_escape_is_denied` creates no symlink and
its escape assertion block (`#[cfg(unix)]`, lines 274-283) compiles out — the
test passes on Windows while proving nothing about symlink/junction escape.
The name claims a guarantee the Windows build does not check. std's
canonicalize does resolve junctions, so the logic is probably right, but "the
sibling test" covers byte-prefix confusion, not link escape. Hygiene-level
because the crate ships nowhere today (unix CI is where it runs).
Confidence: HIGH — Verification: read

F-003 [S4]
Where: crates/plugin-host-wasm/src/fsjail.rs:199-206
Code:
```
    /// House test helper (media-probe precedent).
    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => unreachable!("{e}"),
        }
    }
```
What is wrong: third copy of the same helper in this crate (api.rs:442,
fsjail.rs:199, and error.rs uses inline matches); belongs in testkit.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PathJail::new | 42-59 | REAL | — | canonicalizes roots immediately; nonexistent root = typed Io error |
| PathJail::roots | 61-64 | REAL | — | accessor for display/events |
| PathJail::resolve | 73-135 | REAL | — | empty/4096/no-roots denials; absolute vs per-root candidates; NotFound falls through to next root; other errors deny; canonical prefix check |
| PathJail::under_root | 138-143 | REAL | — | Path::starts_with = component compare; /root-evil ≠ /root (tested) |
| PathJail::verify_open | 160-178 | REAL | S3 | path-not-handle re-verification (F-001); fail-closed on any error |
| shape | 181-194 | REAL | — | first 3 components + ellipsis; no raw buffers in events |
| tests (7) | 254-381 | REAL | S4 | see §5; Windows cfg vacuity (F-002) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| inside_reads_resolve | YES | real files, relative+absolute resolve |
| symlink_escape_is_denied | YES on unix / NO on Windows | real symlink to outside dir; Windows arm asserts nothing about escape (F-002); `../` half runs everywhere |
| component_aware_prefix | YES | real sibling dir root-evil with file, must deny |
| empty_jail_refuses_everything | YES | typed PermissionDenied |
| nonexistent_root_is_a_typed_error | YES | HostError::Io |
| swap_between_resolve_and_open_is_denied | YES on unix | real swap to symlink then verify_open must deny + message names rule; Windows arm asserts the happy path only |
| unswapped_path_passes_reverification | YES | guards against over-blocking |
All 7 executed by me: 7 passed, 0 failed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "std::fs::canonicalize resolves symlinks on every path component, so a symlink pointing outside the root ... is denied" | fsjail.rs:3-7 | REAL | symlink_escape_is_denied (unix) executed |
| "Parent traversal is equally refused" | fsjail.rs:283-284 | REAL | ../secret test executed |
| "a root that does not exist yet is an error — honest refusal" | fsjail.rs:36-37 | REAL | nonexistent_root_is_a_typed_error executed |
| "the attacker model is the plugin (which cannot create symlinks at all)" | fsjail.rs:17-19 | REAL-UNPROVEN | no write-capable import exists in V1_IMPORTS (api.rs catalog verified); full proof sits with imports.rs auditor |

## 7. Dead / duplicated / unreachable code
No dead code in-file. Duplicated: `ok` test helper (F-003). Crate-level
unreachability as per api.rs audit.

## 8. Dependencies & flags
std-only (path, fs). No feature gates. Consumer wiring: state.rs:22
`jail: Option<PathJail>` in embedder-provided HostServices — roots can never
come from a plugin manifest (api.rs Grants doc + state.rs default = None).

## 9. Open questions for the next agent
- imports.rs auditor: confirm fs_read/fs_size/fs_close operate ONLY on the
  handle table (no path re-resolution) — I verified fs_open only.
- If a Windows lane ever ships this crate, F-002 must become a real junction
  test; and F-001's swap-back race should be re-rated against that lane's
  threat model.
