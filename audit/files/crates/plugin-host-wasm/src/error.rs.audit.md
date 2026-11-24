# AUDIT: crates/plugin-host-wasm/src/error.rs
| Field | Value |
|---|---|
| Auditor | W12-A06 |
| Date | 2026-11-24 |
| Lines / functions | 413 / 2 enums (23 + 14 variants), 2 Display impls, 2 Error impls, 2 From impls, 1 test |
| File verdict | REAL — with one PAPER-ONLY enforcement variant (HostCapacity) |
| Reachable from shipped entry point? | NO (crate has zero workspace dependents; see api.rs audit — release lane is lumen-cli+desktop) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `cargo test -p plugin-host-wasm --lib error::` -> 1 passed; per-variant construction-count sweep via rg (all 37 variants except HostCapacity have construction sites); `rg -n 'HostCapacity' crates/ apps/ tools/` -> only error.rs:121,284 |

## 1. One-paragraph verdict
A well-shaped typed error vocabulary: every variant carries actionable Display
text naming the doc rule and the fix path, PackageError is boxed into HostError
with a proper `source()` chain, and 36 of 37 variants are genuinely constructed
somewhere in the crate (verified by sweep). Two defects: `HostError::HostCapacity`
documents host-level resource ceilings ("too many active instances or the total
extension-heap budget") that are enforced NOWHERE — the variant is never
constructed and its companion constants `HOST_MAX_ACTIVE_INSTANCES` /
`HOST_TOTAL_HEAP_BYTES` are never read; and the single test's name ("every
variant displays actably") is false advertising — it checks 4 of 37 variants
with a length assertion.

## 2. What actually works (verified)
— verified by `~/.cargo/bin/cargo test -p plugin-host-wasm --lib error::` ->
"test result: ok. 1 passed".
— verified by construction-site sweep: 36/37 variants have real construction
sites (e.g. SignatureUnparsable x13, PermissionDenied x25, Settings x24),
so the vocabulary is not decorative.

## 3. Findings
F-001 [S2]
Where: crates/plugin-host-wasm/src/error.rs:118-124; crates/plugin-host-wasm/src/limits.rs:85-89
Code:
```
    /// A host-level resource ceiling was hit while spawning: too many
    /// active instances or the total extension-heap budget is
    /// exhausted (doc 06 §5.4 host accounting).
    HostCapacity {
        /// Human-readable accounting detail.
        detail: String,
    },
```
and (limits.rs:85-89):
```
/// 4 active instances per host (doc 06 §5.4 row 8).
pub const HOST_MAX_ACTIVE_INSTANCES: usize = 4;

/// 256 MiB total extension heap across instances (doc 06 §5.4 row 9).
pub const HOST_TOTAL_HEAP_BYTES: u64 = 256 * 1024 * 1024;
```
What is wrong: `HostError::HostCapacity` is never constructed anywhere in the
repository (rg over crates/, apps/, tools/ finds only its definition at
error.rs:121 and its Display arm at error.rs:284). The two constants that
define the ceilings it reports are never read either (rg: only their
definitions). Doc 06 §5.4 rows 8-9 (host accounting) is therefore
PAPER-ONLY: nothing counts active instances, nothing sums extension heaps,
nothing refuses a spawn. Why it matters: an extension lane that ever ships
without an instance-count/heap ceiling can be exhausted by any installed
extension — the documented control does not exist. What correct looks like:
supervisor/instance spawn path checking the two constants and returning
HostCapacity.
Confidence: HIGH — Verification: ran (rg absence proof over the whole repo)
Cross-ref: limits.rs belongs to another agent — flag there: dead constants.

F-002 [S3]
Where: crates/plugin-host-wasm/src/error.rs:386-411
Code:
```
    #[test]
    fn every_variant_displays_actably() {
        // MP §8: error strings name the rule and the fix path.
        let cases: Vec<HostError> = vec![
```
(4-element vec; assertion:)
```
        for err in cases {
            let text = err.to_string();
            assert!(text.len() > 20, "actionable message: {text}");
            assert!(!text.is_empty());
        }
```
What is wrong: The test name claims EVERY variant; the body exercises 4 of 23
HostError variants and 0 of 14 PackageError variants. The assertions
(`len() > 20`, `!is_empty()`) are tautological for compile-time-fixed strings —
a Display arm that printed the wrong rule would still pass. Why it matters:
Display regressions in 33 variants are invisible; the test is evidence-shaped
but proves almost nothing. What correct looks like: iterate a match over all
variants (or a macro-built exhaustive list) asserting a doc-rule substring per
variant.
Confidence: HIGH — Verification: read + ran (passed, proving nothing about the
other 33 variants)
What correct looks like: exhaustive variant iteration.

F-003 [S4]
Where: crates/plugin-host-wasm/src/error.rs:370-374
Code:
```
impl From<std::io::Error> for HostError {
    fn from(err: std::io::Error) -> Self {
        HostError::Io(err.to_string())
    }
}
```
What is wrong: flattening `std::io::Error` into a String drops the error
source chain (`HostError::source()` returns None for `Io`), losing errno /
raw-os-error context for downstream diagnostics. Hygiene-level; the Display
text keeps the human sentence.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HostError (enum, 23 variants) | 14-131 | REAL | S2 | HostCapacity variant PAPER-ONLY (F-001); other 22 constructed |
| PackageError (enum, 14 variants) | 137-211 | REAL | — | all 14 constructed somewhere |
| Display for HostError | 213-297 | REAL | — | names doc rule + fix; GuestMemoryOob prints range |
| Display for PackageError | 299-356 | REAL | — | includes cap constants from package.rs |
| Error for HostError (source) | 359-365 | REAL | — | Package chains to inner |
| Error for PackageError | 368 | REAL | — | |
| From<io::Error> | 370-374 | REAL | S4 | F-003: loses source |
| From<PackageError> | 376-380 | REAL | — | boxes |
| tests::every_variant_displays_actably | 386-411 | SHALLOW | S3 | F-002: 4/37 variants, tautological asserts |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| every_variant_displays_actably | BARELY | only 4 variants covered; assertions are length checks on fixed strings; a swapped message between two covered variants would pass |
Executed by me: 1 passed, 0 failed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every failure mode the host can produce is a variant here" | error.rs:3-4 | REAL | 36/37 variants constructed; HostCapacity is the counterexample for its OWN failure mode (F-001) |
| HostCapacity: "too many active instances or the total extension-heap budget is exhausted" | error.rs:118-121 | PAPER-ONLY | never constructed; ceiling constants never read |
| "never a panic, never a swallowed Result" | error.rs:3-5 | REAL-UNPROVEN | claim spans other files (imports.rs etc.) — audited by their agents |

## 7. Dead / duplicated / unreachable code
Dead: HostError::HostCapacity (error.rs:121), HOST_MAX_ACTIVE_INSTANCES
(limits.rs:86), HOST_TOTAL_HEAP_BYTES (limits.rs:89). Crate unreachable from
shipped lane (see api.rs audit).

## 8. Dependencies & flags
std-only (fmt, path). No feature gates. References crate::package::PACKAGE_CAP_BYTES
in Display — fine.

## 9. Open questions for the next agent
- limits.rs auditor: confirm no other mechanism enforces rows 8-9 (I found
  none; if one exists in supervisor.rs, downgrade F-001 to S3 dead-variant).
- Was doc 06 §5.4 host accounting promised for v1? If the doc marks it
  `[PROPOSAL]`, F-001 stays S2 (declared-but-unread constants) but the doc
  row should be re-checked by the doc-06 auditor.
