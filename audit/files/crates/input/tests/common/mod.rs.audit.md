# AUDIT: crates/input/tests/common/mod.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 129 / 6 functions + 2 Drop impls |
| File verdict | REAL (test-support module) |
| Reachable from shipped entry point? | N/A — test-only shared module for the input crate's integration tests (`#![allow(dead_code)]` shared-module idiom, documented at 4-8). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | `cat` (exit 0). No cargo (adaptation) — test execution UNVERIFIED; W00 baseline: 1751 pass / 2 fail / 25 ignored workspace-wide. |

## 1. One-paragraph verdict

Honest test scaffolding: unique temp dirs named `lumen-2a1-<tag>-<pid>-<counter>` (collision-safe within and across processes on one host), recursive cleanup on Drop with errors deliberately ignored (correct for teardown), a period-251 byte pattern (every offset distinguishable), a directory twin, and a Linux-only privilege probe. Two hygiene notes: the `running_privileged` helper reads `/proc/self/uid_map` and answers `false` on any non-Linux platform, which silently changes a permission-denial test's expectations on macOS/Windows; and cleanup failure is unobservable by design (acceptable, but leaked temp dirs are invisible).

## 2. What actually works (verified)

- Uniqueness: `std::process::id()` + `AtomicU64` counter — no two `TempFile`s in one process share a dir; pid separates concurrent processes.
- `file_uri` uses the production `SourceUri::from_file_path` (percent-encoding, drive-form normalization) rather than hand-rolled URI text — the fixture tests the real constructor.
- `truncate_to`/`append` re-open the file with the minimal OpenOptions for the fault they inject.

## 3. Findings

### F-001 [S4] `running_privileged` is Linux-only and answers `false` elsewhere — permission tests change meaning per-OS
Where: crates/input/tests/common/mod.rs:118-129
Code (118-121, verbatim):
```rust
/// True when the process effectively runs as root (user namespaces with
/// uid 0 mapped): the permission-denial test cannot assert denial there.
pub fn running_privileged() -> bool {
    std::fs::read_to_string("/proc/self/uid_map")
```
What is wrong: on macOS/Windows the read fails, `.ok()` → `None`, `unwrap_or(false)` → "not privileged", so a permission-denial test would assert denial that may not be enforceable (or deny for different reasons) — the guard exists precisely because the assumption is environment-dependent, and it hard-codes the Linux `/proc` idiom. Fine for the Linux CI reality (W00 ran on Linux); misleading for the Windows-first product claims in the docs.
Confidence: HIGH — Verification: read.

### F-002 [S4] Drop cleanup swallows all errors with no counter
Where: crates/input/tests/common/mod.rs:60-65
Code (verbatim):
```rust
impl Drop for TempFile {
    fn drop(&self) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}
```
What is wrong: nothing functionally — teardown best-effort is the right call in a test helper — but a CI host that fails cleanup accumulates temp dirs with zero visibility. `std::process::id` in the name at least makes them attributable. Hygiene only.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `TempFile::create` | 18-33 | REAL | — | unique dir + write |
| `TempFile::file_uri` | 35-48 | REAL | — | production SourceUri constructor |
| `TempFile::truncate_to` | 50-55 | REAL | — | fault injection |
| `TempFile::append` | 57-63 | REAL | — | growth case |
| `Drop for TempFile` | 60-65 | REAL | S4 | F-002 |
| `pattern` | 68-71 | REAL | — | period-251 |
| `TempDir::create`/`file_uri` | 75-105 | REAL | — | directory twin |
| `Drop for TempDir` | 107-112 | REAL | — | same as F-002 |
| `running_privileged` | 118-129 | REAL | S4 | Linux-only probe (F-001) |

## 5. Test honesty

This file IS test support; it contains no assertions of its own. Its consumers (the input integration tests, other agents) determine honesty. The `#[allow(dead_code)]` rationale (each binary links the whole module, uses part) is documented and legitimate.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "nothing here hides a defect (every helper is exercised by at least one binary)" | mod.rs:4-8 | UNVERIFIED | would need the test binaries' usage matrix; the claim is plausible given truncate/append/pattern names appear in fault-injection test names |

## 7. Dead / duplicated / unreachable code

None observed; all helpers have named fault-injection consumers per the doc claim (UNVERIFIED in detail).

## 8. Dependencies & flags

std + media-types + testkit (dev). No features. `#![allow(dead_code)]` at module top is the documented shared-module idiom, not lint suppression of a real defect.

## 9. Open questions for the next agent

- Which integration binaries consume `TempDir` vs `TempFile` (confirm the "every helper exercised" claim)? Files: crates/input/tests/*.rs (other agents).
