# AUDIT: crates/platform-macos/src/capability.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-21 |
| Lines / functions | 200 / 9 items (2 enums + 4 impls + Feature + PlatformReport + 3 methods) |
| File verdict | UNREACHABLE (compiles on every OS, executed by in-crate tests only; the crate is an orphan — see W03-A02 coreaudio.rs audit + W01-A09 manifest F-001, not re-filed per file) |
| Reachable from shipped entry point? | NO. Sole consumer chain: `MacOsPlatform::discover` (lib.rs:106-109 `report.set(...)`) → `MacOsPlatform::report` — and no app depends on platform-macos (`rg 'platform-macos' --glob 'Cargo.toml'`: only its own manifest). Compiled on Linux (no cfg gate — lib.rs:53) and executed by `tests/seam_behaviour.rs` (16/16 pass, orchrun :3691-3712). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `sed -n` full read · `rg -n 'platform\.' docs/feature-matrix.toml` (ZERO platform.* rows — the F-001 proof; 252 `id =` rows, kebab-case) · `rg -rn 'platform\.audio\.enumeration\|platform\.gpu\.enumeration\|platform\.dialogs' --glob '!crates/platform-*'` (zero consumers) · `ls crates/platform-{linux,windows}/src` + `rg 'platform\.audio\.enumeration' …/capability.rs` (:147 in both — triplicated vocabulary) · `sed -n '3684,3712p' orchrun…log` (16/16 seam tests incl. the three capability tests) |

## 1. One-paragraph verdict
The typed capability-negotiation vocabulary is internally clean: honest defaults (unprobed → `NotYetImplemented`, never assumed available), replace-in-place report semantics, typed reasons with no bare strings, and three real tests that ran and passed. But its central provenance claim is hollow — `Feature::key()` is documented and tested as "the stable dotted key (feature-matrix vocabulary)" while the feature matrix contains no `platform.*` row at all (252 rows, all kebab-case ids) and no code outside the three platform crates reads these strings; the "matrix vocabulary" is a private naming convention dressed as a ledger integration. Second, `from_error`'s catch-all classifies every non-vocabulary error — including HAL/API call failures that have nothing to do with reachability — as `ServiceNotReachable`, a mislabel saved only by the preserved detail string. And the whole layer belongs to the orphan platform-macos crate, so no shipped binary can query it.

## 2. What actually works (verified)
— Honest-defaults contract: `get` returns `NotYetImplemented { detail: "feature was not probed by this adapter" }` for absent rows (:185-194) — pinned by `platform_report_is_a_table_with_honest_defaults` (seam_behaviour.rs:159-184, passed in baseline, orchrun :3705).
— `set` replace-in-place (:170-180): the report cannot accumulate duplicate rows — pinned by the same test (rows == 1 after re-set).
— Typed classification for the three vocabulary errors (:78-92): WrongOs/LibraryNotFound/ServiceNotReachable map 1:1 with payloads — pinned by `capability_classifies_and_converts_probe_errors` (:104-142, passed).
— Bounded by construction: `PlatformReport` holds at most one row per `Feature` (4 variants) — the Vec cannot grow unboundedly.
— All 16 platform-macos seam tests pass in the no-fail-fast baseline (orchrun :3691-3712).

## 3. Findings

### F-001 [S3]
Where: crates/platform-macos/src/capability.rs:142-152
Code:
```
impl Feature {
    /// The stable dotted key (feature-matrix vocabulary).
    #[must_use = "pure accessor"]
    pub fn key(self) -> &'static str {
        match self {
            Feature::AudioEnumeration => "platform.audio.enumeration",
```
What is wrong: The doc comment claims these keys are "feature-matrix vocabulary", and the test is named `feature_keys_are_the_stable_matrix_vocabulary` — but docs/feature-matrix.toml contains ZERO `platform.*` rows (searched: no match for `platform.` anywhere in the file; its 252 rows use kebab-case ids like `afeat-audio-device-switch`). No code outside the three platform crates consumes the strings either (`rg` over the repo minus `crates/platform-*`: zero hits). The keys are a private convention mirrored into three crates, spoken by no ledger and read by no consumer.
Why it matters: This is the "evidence string that proves nothing" pattern in miniature: a reader (or the parity tooling) believing these keys address matrix rows will find nothing; the test name asserts a relationship the artifact contradicts. If a matrix row for platform capabilities is ever added, nothing enforces these strings match it.
What correct looks like: Either add the four rows to docs/feature-matrix.toml (with honest statuses — all UNREACHABLE per the orphan crate) or reword the doc to "stable dotted key (proposed matrix vocabulary; no rows yet)" and rename the test to stop claiming matrix membership.
Confidence: HIGH — Verification: read + exhaustive rg of the matrix and repo.

### F-002 [S4]
Where: crates/platform-macos/src/capability.rs:101-104
Code:
```
            other => UnavailableReason::ServiceNotReachable {
                service: "platform integration",
                detail: other.to_string(),
            },
```
What is wrong: Every `PlatformError` outside the three mapped variants collapses into `ServiceNotReachable` — most prominently `CallFailed`, which the crate's own CoreAudio lane produces for plain API refusals (bad parameters, device busy) that have nothing to do with service reachability. The reason class lies about the failure's nature; only the `detail` string preserves the truth.
Why it matters: A UI surfacing "service not reachable" for what is actually a rejected API call misdirects diagnosis. Mitigations keep this S4: the detail keeps the API name, the behavior is test-pinned as intentional (seam_behaviour.rs:128-141 asserts CallFailed → ServiceNotReachable with `AudioObjectGetPropertyDataSize` in the detail), and nothing shipped reads it.
What correct looks like: A fifth `UnavailableReason::CallFailed { api, code }` variant (the error vocabulary already carries both) instead of the catch-all.
Confidence: HIGH — Verification: read + the pinning test traced.

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Capability` enum | 13-22 | REAL | — | Two-variant verdict; honest doc. |
| `Capability::is_available` | 24-29 | REAL | — | Matches only Available. |
| `Capability::from_probe_error` | 31-37 | REAL | — | Delegates to UnavailableReason::from_error. |
| `UnavailableReason` enum | 41-71 | REAL | — | Four typed reasons, payloads documented. |
| `UnavailableReason::from_error` | 73-107 | REAL | S4 | F-002 catch-all; other mappings 1:1 correct. |
| `Display for UnavailableReason` | 109-127 | REAL | — | Human forms of all four; no secrets. |
| `Feature` enum | 130-140 | REAL | — | 4 lanes (audio×2, gpu, dialogs). |
| `Feature::key` | 142-153 | REAL | S3 | F-001: "matrix vocabulary" claim unbacked. |
| `PlatformReport` + `new`/`set`/`get`/`iter` | 157-200 | REAL | — | Replace-in-place set; honest get default; bounded by Feature count; iter yields live rows. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `capability_classifies_and_converts_probe_errors` (seam_behaviour.rs:104) | YES | Asserts exact variant + payload for WrongOs, ServiceUnavailable, CallFailed (incl. api name in detail). |
| `feature_keys_are_the_stable_matrix_vocabulary` (:144) | PARTIAL | Pins the four strings (kills renames) but asserts nothing about the matrix — the name's matrix claim is untestable there (F-001). |
| `platform_report_is_a_table_with_honest_defaults` (:159) | YES | Unprobed default, replace-not-append, iter consistency — kills the obvious mutations. |
All three ran and passed in the baseline (orchrun :3692-3696, :3705 within 16/16).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The stable dotted key (feature-matrix vocabulary)" | capability.rs:143 | FALSE | Zero platform.* rows in docs/feature-matrix.toml; zero consumers outside platform crates (F-001). |
| "computed from live probes… not from compile-time assumptions alone" | capability.rs:6-8 | REAL-UNPROVEN | discover() does probe live (lib.rs:143-210); but no shipped binary ever runs it (orphan crate). |
| "A Capability::Unavailable is a real implementation of graceful degradation… never fakes success" | capability.rs:4-6 | REAL | Fallback seams return the typed reason; tested. |

## 7. Dead / duplicated / unreachable code
Unreachable in product: the entire file (orphan crate — see coreaudio.rs audit §7 / W01-A09 manifest F-001; not re-filed). Duplicated: byte-identical vocabulary mirrored in `platform-linux/src/capability.rs:147` and `platform-windows/src/capability.rs:147` — documented `[PROPOSAL]` unification (lib.rs:47-50), but three manual-sync copies of a "stable" vocabulary is a drift hazard with no test comparing them.

## 8. Dependencies & flags
Only `crate::error::PlatformError` — no deps, no cfg gates, no unsafe. Compiles on every OS (unlike the FFI modules), which is why the 16 seam tests can run on Linux.

## 9. Open questions for the next agent
- platform-linux/platform-windows capability.rs owners (wave-3 colleagues): confirm the three copies are byte-identical or diverged — a cross-crate equality test would freeze the mirror.
- Matrix owner: is a platform-capability row set planned? If yes, F-001's keys need to land in the matrix or be reworded.
