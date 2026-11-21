# AUDIT: crates/platform-windows/src/capability.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-21 |
| Lines / functions | 200 / 14 (2 enums + 4 methods/impls, 1 enum + 1 method + Display, 1 enum + key(), 1 struct + 4 methods, module docs; 0 tests in file, 3 tests in tests/seam_behaviour.rs) |
| File verdict | UNREACHABLE (real, tested library code; no shipped binary depends on platform-windows — see dxgi.rs F-001) |
| Reachable from shipped entry point? | NO (crate orphaned — evidence in dxgi.rs audit F-001: no Cargo.toml in apps/ or crates/ depends on platform-windows; the module's only executed path in the baseline is the crate's own test binary) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | `rg -c "platform.(audio\|gpu\|dialogs)" docs/feature-matrix.toml` (all four keys: 0 matches) · `sed -n tests/seam_behaviour.rs` capability tests read · baseline log: platform-windows seam_behaviour 16/16 ok (:3718-3736), incl. `capability_classifies_and_converts_probe_errors`, `feature_keys_are_the_stable_matrix_vocabulary`, `platform_report_is_a_table_with_honest_defaults` |

## 1. One-paragraph verdict
A small, disciplined typed-capability layer: probe errors collapse into four honest reason variants, unprobed features answer `NotYetImplemented("feature was not probed by this adapter")` rather than a silent default, `PlatformReport` is a replace-in-place table with real tests (all passed in the no-fail-fast baseline). Two defects: the file shares the crate-wide death sentence — no shipped binary links platform-windows, so this "capability negotiation" layer reports to nobody (dxgi.rs F-001, cited, not re-derived); and `Feature::key()`'s documentation claims the strings are "the feature-matrix vocabulary" when none of the four keys exists anywhere in docs/feature-matrix.toml — a nonexistent contract that the test named after it "verifies" tautologically.

## 2. What actually works (verified)
— 16/16 platform-windows seam_behaviour tests passed (`orchrun-test-workspace-nofailfast.log.txt:3718-3736`), including the three covering this file: `capability_classifies_and_converts_probe_errors` (Available/WrongOs classification, CallFailed→ServiceNotReachable, LibraryUnavailable→LibraryNotFound conversions), `platform_report_is_a_table_with_honest_defaults` (unprobed default exact-equal assert, set/get, replace-not-append with a row count), `feature_keys_are_the_stable_matrix_vocabulary` (string equality — see F-002 for what it does not prove).
— `discover_on_a_foreign_host_reports_wrong_os_everywhere` passed — on this Linux host, the only reachable discover() behavior (WrongOs on every seam) is the tested one.
— `from_error` mapping read against the PlatformError variants: the four specific arms cover the typed causes; the catch-all (:101-104) preserves the full detail string, so no information is lost in the collapse.

## 3. Findings
### F-001 [S3]
Where: crates/platform-windows/src/capability.rs:155-160 (the report's audience), cross-ref dxgi.rs F-001
Code (capability.rs:157-160):
```
#[derive(Debug, Clone, Default)]
pub struct PlatformReport {
    entries: Vec<(Feature, Capability)>,
}
```
What is wrong: inherited crate-level defect — platform-windows is a dependency of no application or engine crate (rg over every Cargo.toml), so `PlatformReport` and the entire negotiation layer it names ("what this build can actually do, honestly") cannot be queried by any shipped build. The module doc's "a UI or engine can query before attempting a call" (:155-156) has no such UI or engine.
Why it matters: the crate's headline promise (MP §13 "no invented crate capabilities", quoted in the module docs) is inverted at the product level — the capability machinery itself is a capability nothing can invoke.
What correct looks like: wire the platform crates into a shipped binary (target-conditional dependency), or mark the layer as library-only and stop implying product-level capability reporting.
Confidence: HIGH — Verification: Cargo.toml exhaustion (dxgi.rs audit §3 F-001).
### F-002 [S4]
Where: crates/platform-windows/src/capability.rs:142-152 (doc + code), tests/seam_behaviour.rs:153-163
Code (capability.rs:143-150):
```
    /// The stable dotted key (feature-matrix vocabulary).
    #[must_use = "pure accessor"]
    pub fn key(self) -> &'static str {
        match self {
            Feature::AudioEnumeration => "platform.audio.enumeration",
            Feature::AudioNotifications => "platform.audio.notifications",
            Feature::GpuEnumeration => "platform.gpu.enumeration",
            Feature::FileDialogs => "platform.dialogs",
```
What is wrong: the doc claims these are "feature-matrix vocabulary" keys; `rg` over docs/feature-matrix.toml finds zero occurrences of any of the four strings (`platform.audio.enumeration`, `platform.audio.notifications`, `platform.gpu.enumeration`, `platform.dialogs`). The vocabulary does not exist in the matrix. The test `feature_keys_are_the_stable_matrix_vocabulary` (seam_behaviour.rs:153-163) asserts `key()` equals its own literals — a tautology that proves nothing about the claimed matrix contract while its name says it does.
Why it matters: a "stable key" contract that nothing consumes and no consumer matches is invented vocabulary; the test's name gives false assurance exactly where a cross-file contract should be pinned (contrast: pipeline.rs's schema pinning test that reads the real file).
What correct looks like: either add the four keys as row ids (or evidence tags) in docs/feature-matrix.toml and make the test read the matrix file like `schema_constants_pin_the_engine_copies` does, or drop the "feature-matrix vocabulary" doc claim.
Confidence: HIGH — Verification: rg counts (0 for all four) on both the matrix and a repo-wide sweep.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-8 | REAL | — | "computed from live probes" upheld by gpu_seam design; product-level reachability dead (F-001) |
| `enum Capability` | 13-22 | REAL | — | two variants; tested |
| `Capability::is_available` | 26-29 | REAL | — | trivial, tested |
| `Capability::from_probe_error` | 33-36 | REAL | — | delegation; tested |
| `enum UnavailableReason` | 41-71 | REAL | — | 4 typed reasons; documented |
| `UnavailableReason::from_error` | 75-107 | REAL | — | 4 specific arms + catch-all; the catch-all folds `CallFailed` (e.g. DXGI factory failure) into `ServiceNotReachable { service: "platform integration" }` — stretched semantics, detail preserved (table note, not a finding) |
| `Display for UnavailableReason` | 109-127 | REAL | — | human-readable, includes names+detail; `platform_errors_render_their_details` test passed |
| `enum Feature` | 130-140 | REAL | — | 4 lanes |
| `Feature::key` | 144-152 | REAL | S4 | F-002: keys are not matrix vocabulary |
| `PlatformReport` + `new` | 157-167 | REAL | — | Default empty |
| `PlatformReport::set` | 170-180 | REAL | — | replace-in-place, tested (rows==1 after re-set) |
| `PlatformReport::get` | 185-194 | REAL | — | unprobed → honest NotYetImplemented, exact-equal tested |
| `PlatformReport::iter` | 197-199 | REAL | — | used by report consumers (tests) |

## 5. Test honesty (3 tests in tests/seam_behaviour.rs, all passed — baseline :3718-3736)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `capability_classifies_and_converts_probe_errors` | YES | exact-equal asserts on classification and all three conversion arms incl. the CallFailed catch-all detail |
| `platform_report_is_a_table_with_honest_defaults` | YES | unprobed default exact-equal; replace-not-append proven by row count |
| `feature_keys_are_the_stable_matrix_vocabulary` | NO (tautology) | asserts literals equal themselves; never opens docs/feature-matrix.toml — F-002 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A Capability::Unavailable is a real implementation of graceful degradation ... never fakes success" | capability.rs:4-6 | REAL | typed reasons + tests passed |
| "The report is computed from live probes ..., not from compile-time assumptions alone" | capability.rs:6-8 | REAL (design) / UNREACHABLE (product) | gpu_seam probes live on Windows; no shipped build reaches it (F-001) |
| "unprobed features are honestly NotYetImplemented ..., never assumed available" | capability.rs:182-183 | REAL | exact-equal test passed |
| "The stable dotted key (feature-matrix vocabulary)" | capability.rs:143 | FALSE | F-002: 0 matches for all four keys in docs/feature-matrix.toml |
| "a UI or engine can query before attempting a call" | capability.rs:155-156 | UNREACHABLE | no UI or engine depends on the crate |

## 7. Dead / duplicated / unreachable code
- The whole file shares the crate's orphan status (F-001, dxgi.rs cross-ref).
- No dead items within the file: every method is exercised by the seam tests.

## 8. Dependencies & flags
- Depends only on crate::error (PlatformError). No unsafe, no OS APIs, no feature gates — the file is OS-neutral by construction (the OS-specificity lives in the probes that feed it).

## 9. Open questions for the next agent
- events.rs / dialog.rs (my remaining platform-windows files): same F-001 applies — verify their tests are also fallback-arm-only before crediting "executed" status.
- Matrix wave: if platform rows are added under these keys, `Feature::key` becomes a real contract — until then F-002 stands.
- platform-linux/platform-macos auditors: their capability.rs files are mirrored (crate docs say the seam vocabulary is deliberately mirrored across the three) — the same matrix-vocabulary claim should be checked there.
