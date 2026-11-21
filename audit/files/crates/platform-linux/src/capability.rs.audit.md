# AUDIT: crates/platform-linux/src/capability.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-21 |
| Lines / functions | 201 / 10 (2 enums + from_error + Display, Feature::key, PlatformReport 4 methods) |
| File verdict | UNREACHABLE (real, tested, orphan crate) |
| Reachable from shipped entry point? | NO — `platform-linux` has zero workspace dependents (verified root + all apps/crates Cargo.tomls); used only inside the crate (lib.rs `discover`, seam_behaviour/linux_integrations tests) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "platform\.(audio\|gpu\|dialogs)" docs/feature-matrix.toml` → **zero rows carry these keys** (252 `id =` rows searched); PlatformError variant list read (error.rs:15-69 — 8 variants); test read: `capability_classifies_and_converts_probe_errors` (seam_behaviour.rs) covers 4 of 8 variants, catch-all untested; baseline: `capability_classifies_and_converts_probe_errors ... ok` (:3667), `platform_report_is_a_table_with_honest_defaults ... ok` (:3674), `discover_assembles_this_hosts_report ... ok` (:3653) |

## 1. One-paragraph verdict
The honest-degradation vocabulary of the (orphan) Linux platform adapter:
clean typed enums, a replace-in-place report table with honest
"not probed" defaults, live-probe assembly executed in the committed
baseline. Two real defects eat into the module's own promise: the
error→reason conversion silently misclassifies half the error taxonomy
(CallFailed, TimedOut, InvalidEncoding, SpawnFailed all collapse into
"ServiceNotReachable: platform integration" — the exact "bare string
guess" shape the module doc forbids), and the "feature-matrix vocabulary"
claim is false — none of the four dotted keys exists in any of the
feature matrix's 252 rows; the test that claims to pin "the stable matrix
vocabulary" only pins the keys against themselves.

## 2. What actually works (verified)
— Tests executed ok in baseline: orchrun log :3653 (`discover_assembles_
this_hosts_report`), :3667 (`capability_classifies_and_converts_probe_
errors`), :3674 (`platform_report_is_a_table_with_honest_defaults`) — the
report assembly and conversions ran on this host.
— `PlatformReport::set` replace-in-place verified by reading (:170-180) and
pinned by the test (set twice → one row, second verdict wins).
— Unprobed default verified: `get` on an absent row returns
`NotYetImplemented { detail: "feature was not probed by this adapter" }`
(:185-194) — never assumed Available.
— Boundedness: the table is capped at 4 rows by construction (Feature has
4 variants; set replaces) — no unbounded growth path.
— SymbolMissing→LibraryNotFound mapping keeps the symbol visible in the
detail string (:95-100) — good diagnosis preservation, tested.

## 3. Findings
### F-001 [S3]
Where: crates/platform-linux/src/capability.rs:101-104
Code:
```rust
            other => UnavailableReason::ServiceNotReachable {
                service: "platform integration",
                detail: other.to_string(),
            },
```
What is wrong: The catch-all maps FOUR of PlatformError's eight variants —
`CallFailed`, `TimedOut`, `InvalidEncoding`, `SpawnFailed` (error.rs:46-69) —
into `ServiceNotReachable { service: "platform integration" }`. None of
those means the service was unreachable: a failed API call, an expired
deadline, a bad string encoding, and a missing helper binary are four
different failure classes, all relabeled "not reachable". The
`UnavailableReason` enum has no variant that fits them, so the taxonomy
lies for half the error space.
Why it matters: This module IS the typed-truth story ("the reason is typed,
never a bare string guess", :19-20); a UI branching on the reason class
("retry later" vs "install a library" vs "report a bug") gets the wrong
branch for CallFailed/TimedOut/InvalidEncoding/SpawnFailed outcomes. The
zenity/kdialog `SpawnFailed` (dialogs) is the most realistic mislabel:
"platform integration not reachable: <spawn text>" for a missing helper
program.
What correct looks like: Add `UnavailableReason::CallFailed`, `TimedOut`
(or a generic `Failed { class, detail }`) and map each variant honestly;
the catch-all arm should be unreachable.
Confidence: HIGH — Verification: variant list read at error.rs:15-69; the
conversion test covers only NotCompiledForThisOs/LibraryUnavailable/
SymbolMissing/ServiceUnavailable (test read) — the catch-all path has zero
test coverage.
Severity: S3 — the detail string is preserved, so information is not lost,
only the class; blast radius is downstream branching on the wrong class.

### F-002 [S3]
Where: crates/platform-linux/src/capability.rs:142-151 (module reachability)
Code:
```rust
impl Feature {
    /// The stable dotted key (feature-matrix vocabulary).
    #[must_use = "pure accessor"]
    pub fn key(self) -> &'static str {
```
What is wrong: (Reachability, restated from the pulse_ffi audit for this
file's own ledger:) the crate is an orphan — no shipped entry point links
platform-linux, so this capability negotiation answers nobody in the
product. The keys this function mints feed only the crate's own tests.
Why it matters: The whole "UI or engine can query before attempting a
call" design (:155-156) has no querier; capability negotiation for the
Linux lane is dead code in every shipped binary (the shipped audio lane
is the null device by feature string, baseline INDEX.md).
What correct looks like: device-discovery/audio-output depend on the crate
target-gated and query the report at open time.
Confidence: HIGH — Verification: Cargo.toml sweep (see pulse_ffi audit).
(Severity shared with the crate-level finding; counted once here.)

### F-003 [S4]
Where: crates/platform-linux/src/capability.rs:143 (claim) + tests/seam_behaviour.rs:194
Code:
```rust
    /// The stable dotted key (feature-matrix vocabulary).
```
What is wrong: None of the four keys (`platform.audio.enumeration`,
`platform.audio.notifications`, `platform.gpu.enumeration`,
`platform.dialogs`) appears in any of the 252 rows of
docs/feature-matrix.toml (searched: zero matches — the matrix's ids are
dashed slugs like `playback-snapshot`). The test
`feature_keys_are_the_stable_matrix_vocabulary` (seam_behaviour.rs:194-205)
asserts only `key() == "<the same literal>"` — a self-consistency pin, not
a matrix-membership pin. The doc comment and the test name both claim a
matrix vocabulary that does not exist.
Why it matters: Evidence-string inflation inside the repo's own test
names: a future auditor greps the matrix for these keys, finds nothing,
and cannot tell whether the crate or the matrix drifted.
What correct looks like: Either add the four dotted keys to the matrix
rows they implement (or a cross-ref table the test parses), or rename the
doc/test to "stable wire vocabulary" without the matrix claim.
Confidence: HIGH — Verification: rg over feature-matrix.toml (zero hits);
test body read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Capability` (enum) | 13-22 | REAL | — | Two-state verdict |
| `Capability::is_available` | 26-29 | REAL | — | Trivial, tested |
| `Capability::from_probe_error` | 33-36 | REAL | — | Delegates |
| `UnavailableReason` (enum) | 41-71 | REAL | — | 4 classes; no CallFailed/TimedOut class (F-001) |
| `UnavailableReason::from_error` | 76-106 | REAL | S3 (F-001) | 4 exact arms + lying catch-all for 4 more variants; untested arm |
| `UnavailableReason::Display` | 109-127 | REAL | — | Readable strings, tested for substrings |
| `Feature` (enum) + `key` | 130-153 | REAL | S4 (F-003) | 4 features; dotted keys not in any matrix row |
| `PlatformReport::new` | 165-167 | REAL | — | Empty table |
| `PlatformReport::set` | 170-180 | REAL | — | Replace-in-place, O(4) max |
| `PlatformReport::get` | 185-194 | REAL | — | Honest not-probed default, tested |
| `PlatformReport::iter` | 197-199 | REAL | — | Borrow iteration |

## 5. Test honesty (if tests exist in/for this file)
Tests live in tests/seam_behaviour.rs (crate-level), all ran ok in baseline:
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| capability_classifies_and_converts_probe_errors | YES for the 4 covered variants | Exact enum equality + symbol-in-detail substring; catch-all arm NOT covered (F-001 untested) |
| platform_report_is_a_table_with_honest_defaults | YES | Default verdict, replace-in-place, row count all pinned |
| feature_keys_are_the_stable_matrix_vocabulary | NO (tautology) | Asserts `key() == "literal"` — pins stability against accidental change only; proves nothing about the matrix (F-003) |
| discover_assembles_this_hosts_report (linux_integrations) | YES | Host-truth assembly end-to-end |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the reason is typed, never a bare string guess" | module doc :19-20 | FALSE for 4 of 8 error variants | F-001 catch-all |
| "The report is computed from live probes" | module doc :6-8 | REAL | discover_assembles_this_hosts_report ok (orchrun :3653) |
| "The stable dotted key (feature-matrix vocabulary)" | :143 | FALSE | No matrix row carries these keys (rg zero hits over 252 rows) |
| "a UI or engine can query before attempting a call" | :155-156 | UNREACHABLE in product | Orphan crate (F-002); no querier exists |

## 7. Dead / duplicated / unreachable code
Whole file unreachable from shipped entry points (F-002, crate-level).
Duplicated: the identical capability module + seam tests exist in
platform-windows and platform-macos (same test names at
platform-windows/tests/seam_behaviour.rs — verified by rg -l); the trio is
deliberate template symmetry per doc 01 §2.7, but `from_error`'s catch-all
and the key() set are copy-paste-parallel — a fix must land in three
places (noted for the maintainers).

## 8. Dependencies & flags
`crate::error` only; no unsafe, no I/O, no feature gates. `Default`-derived
report; bounded by enum cardinality.

## 9. Open questions for the next agent
- The same `from_error` catch-all exists in platform-macos/src/capability.rs
  and platform-windows (verify when those files are audited — same defect
  class, F-001 triplicated).
- If the matrix is meant to grow platform.* rows, the keys here are the
  natural ids — confirm with the parity/matrix wave which side should move.
