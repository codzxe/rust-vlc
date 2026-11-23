# AUDIT: crates/input/tests/common/mod.rs
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-02-14 |
| Lines / functions | 129 / 10 |
| File verdict | REAL (test-support module; one helper is a constant-false detector — see F-001) |
| Reachable from shipped entry point? | N/A — test-only module (tests/common, `#[allow(dead_code)]` idiom); the suites it serves (file_stream/truncation/bounded_reads) test the shipped `input` crate, which IS reachable (CLI FileAccess lane) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p input` → 33+5+9 passed, 0 failed (own run); python3 uid_map field check on this host |

## 1. One-paragraph verdict
Honest std-only test scaffolding: unique temp dirs (pid + atomic counter) removed recursively on drop, `file:` URIs built through the REAL shared constructor `SourceUri::from_file_path` (not string concatenation), truncation/append fault injectors, and a 251-period byte pattern. All three consuming suites ran green (47 tests total) on this host. One real defect: `running_privileged()` — the guard that is supposed to detect "effectively root so the mode-000 permission test cannot assert denial" — parses the third field of `/proc/self/uid_map` and returns true iff it equals **0**. That field is the mapping *length* and is never 0 in any valid uid_map (initial namespace: `0 0 4294967295`; user-ns root: `0 1000 1`), so the function is a constant `false` on every Linux system: under real root the guarded test fails loudly instead of skipping, and its Ok-branch skip path is dead code.

## 2. What actually works (verified)
— `cargo test -p input` → `33 passed` (lib) + `5 passed` (bounded_reads) + `9 passed` (file_stream, includes the mode-000 test's Err branch on this uid-1001 host) + 0 + 0, all exit 0 (own run). TempFile uniqueness/cleanup exercised by every one of those tests.

## 3. Findings
F-001 [S3]
Where: crates/input/tests/common/mod.rs:114-129
Code:
```
/// True when the process effectively runs as root (user namespaces with
/// uid 0 mapped): the permission-denial test cannot assert denial there.
pub fn running_privileged() -> bool {
    std::fs::read_to_string("/proc/self/uid_map")
        .ok()
        .and_then(|text| {
            text.lines().next().and_then(|first| {
                first
                    .split_whitespace()
                    .nth(2)
                    .and_then(|count| count.parse::<u32>().ok())
                    .map(|count| count == 0)
```
What is wrong: uid_map lines are `inside-id outside-id length`; the length is never 0 (kernel rejects 0-length mappings), so `count == 0` is never true. Verified on this host: fields `['0','0','4294967295']` → nth(2)='4294967295' ≠ 0 → false, while a user-ns root would read `0 1000 1` → also false. The function cannot ever return true, so (a) the guard in `file_stream.rs:113-116` is dead code, and (b) when the suite runs as actual root (common in Docker CI), `open()` on the mode-000 file SUCCEEDS, hits the Ok branch, and the test fails with "open succeeded on a mode-000 file without detected privilege" — a false failure of a correct implementation. Why it matters: broken skip logic converts an environment property into a red CI signal; the doc comment's claimed semantic ("user namespaces with uid 0 mapped") matches no uid_map shape. What correct looks like: `unsafe { libc::geteuid() } == 0` or parse `Uid:` from /proc/self/status. Confidence: HIGH — Verification: read + python3 field check on the live uid_map; the guard branch is unexecutable on any host I can construct.
(The failure mode is loud, not silent — severity S3, not S2.)

F-002 [S4]
Where: crates/input/tests/common/mod.rs:76-79
Code:
```
/// A deterministic byte pattern (period 251: every offset distinguishable).
pub fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}
```
What is wrong: "every offset distinguishable" holds only for `len ≤ 251`; the consuming tests use `pattern(1000)`/`pattern(5000)`, where offsets 251 bytes apart are identical. Harmless for the whole-buffer comparisons the suites actually do, but the doc overstates the property. Confidence: HIGH — Verification: read + caller sizes (truncation.rs:54, file_stream.rs:32).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TempFile::create | 28-39 | REAL | — | unique dir `lumen-2a1-<tag>-<pid>-<n>`; pid+counter race-safe |
| TempFile::file_uri | 47-52 | REAL | — | real SourceUri::from_file_path; must() on non-UTF-8 |
| TempFile::truncate_to | 55-58 | REAL | — | open+set_len fault injection |
| TempFile::append | 61-65 | REAL | — | growth fault injection |
| Drop for TempFile | 68-74 | REAL | — | recursive cleanup, errors ignored (test-scoped) |
| pattern | 77-79 | REAL | S4 | F-002 comment overstates |
| TempDir::create | 87-96 | REAL | — | same uniqueness scheme |
| TempDir::file_uri | 100-105 | REAL | — | same constructor |
| Drop for TempDir | 108-112 | REAL | — | cleanup |
| running_privileged | 116-129 | BROKEN (constant false) | S3 | F-001 |

## 5. Test honesty
This file contains no tests; it is the shared harness. The suite it feeds is honest (real files, real permission bits, real truncation); the one dishonest piece is the privilege guard (F-001), which the mode-000 test relies on.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "True when the process effectively runs as root" | mod.rs:114 | FALSE | F-001: never true on any valid uid_map |
| "every helper is exercised by at least one binary in this directory" | mod.rs:4-7 | REAL | file_stream/truncation/bounded_reads all ran (47 tests) |
| "Temp paths are ASCII on every CI host" | mod.rs:44-46 | REAL-UNPROVEN | held on this host; no non-ASCII CI host available to test |

## 7. Dead / duplicated / unreachable code
`running_privileged`'s true-branch is dead (F-001). `#[allow(dead_code)]` at L9 is the documented shared-module idiom — verified not hiding anything (all helpers have callers).

## 8. Dependencies & flags
Test-only: media-types (SourceUri), testkit (Must), std only (no tempfile crate — per C5). No unsafe. Unix-gated at the call site, not here.

## 9. Open questions for the next agent
- Does the root-CI failure (F-001) reproduce in the project's actual CI? Baseline shows this suite green (uid-1001 sandbox), so the latent failure has not yet fired — the orchestrator's CI logs would prove it either way.
