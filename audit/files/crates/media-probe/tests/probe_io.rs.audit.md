# AUDIT: crates/media-probe/tests/probe_io.rs
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-02-14 |
| Lines / functions | 374 / 17 (10 #[test] + 7 helpers) |
| File verdict | REAL (executed live with the feature enabled) |
| Reachable from shipped entry point? | N/A (test binary) — but see F-001: the binary runs 0 tests under a default `cargo test` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p media-probe --features contract --test probe_io` (via ~/.cargo/bin/cargo) -> 10 passed, exit 0; baseline audit/runs/baseline/cargo-test-workspace.txt:3684-3687 -> "running 0 tests ... ok" (feature off); `wc -c` fixtures -> avi 425516, mp4 424452, trunc1 52 (all match test constants); sed quote verifications |

## 1. One-paragraph verdict
The probe-I/O proofs are real and strong — I executed all 10 with the
`contract` feature enabled (exit 0): instrumented read-ledger assertions
(exact ask sequences `[bound]`, `[bound, bound−424452]`), position restoration,
an 8-rung score ladder pinned to exact confidences over real FileAccess I/O,
wrong-extension content-wins, empty/1-byte/garbage typed unknowns, the
sub-magic truncation sweep (cuts 0..11 → `probe.no-demuxer`; cut 12 → avi @70
with no hints), the bounded EOS fill over a 52-byte truncated file, and the
1 MiB ceiling refusal. Every hardcoded byte count matches the real fixture
sizes (verified). The one structural defect: the whole file is
`#![cfg(feature = "contract")]`, so a plain `cargo test` — what the
orchestrator baseline ran, and what .github/workflows/rust.yml:26 runs —
compiles the binary to ZERO tests and prints a green "ok. 0 passed"; the
header's "CI's --all-features always does" is true only of ci.yml:234 /
ci-full.yml:169.

## 2. What actually works (verified)
— verified by `cargo test -p media-probe --features contract --test probe_io`
-> "test result: ok. 10 passed; 0 failed" exit 0 (live, this session).
Fixture facts cross-checked: tests/fixtures/generated/valid/avi_mjpeg_pcm.avi
= 425,516 B (larger than the biggest bound 262,144 — the "far larger" comment
holds), mp4 = 424,452 B (equals the hardcoded constant), trunc1 = 52 B.
CI coverage claim verified: .github/workflows/ci.yml:234 and ci-full.yml:169
run `cargo test --workspace --all-features`.

## 3. Findings
F-001 [S3]
Where: crates/media-probe/tests/probe_io.rs:14-18
Code:
```
//! Feature-gated (audit F-2): the instrumentation and executable
//! checks live in `media_probe::contract`, a test/conformance-only
//! module — run with `--features contract` (CI's `--all-features`
//! always does).
#![cfg(feature = "contract")]
```
What is wrong: under the default feature set this integration binary
contributes ZERO tests while printing a green "ok" — verified in the
orchestrator baseline (cargo-test-workspace.txt:3684-3687: "Running
tests/probe_io.rs ... running 0 tests ... ok. 0 passed"). So the evidence
store for this audit, and any lane running plain `cargo test`
(.github/workflows/rust.yml:26 `cargo test --verbose`), silently loses all
10 proofs. The header's claim is accurate for the main CI
(ci.yml:234 `cargo test --workspace --all-features`) — this is not fraud,
but a green-zero-tests line is actively misleading in exactly the runs a
developer or auditor runs by default. Why it matters: "0 passed; ok" is
indistinguishable from "all good" at a glance; a regression deleting this
file entirely would look identical in default lanes. What correct looks
like: keep the feature but add a compile-time canary — e.g. a
non-gated `#[test] fn contract_feature_is_enabled_in_this_lane() { assert!(
cfg!(feature = "contract")) }` so default runs FAIL visibly when the suite
is silently absent, or use `#[ignore]` (which at least shows in the
passed/ignored split).
Confidence: HIGH — Verification: ran both configurations + read workflows.

F-002 [S4]
Where: crates/media-probe/tests/probe_io.rs:117-120
Code:
```
        // The file is far larger than every bound here, so the fill is
        // exactly ONE window-sized request (the buffered reader serves
        // it from its fixed read-ahead, possibly across refills).
        assert_eq!(stream.requests(), &[bound], "one window-sized fill");
```
What is wrong: brittle pin on FileStream's internal serving shape — the
assert demands the window fill be a single `read(bound)` ask. If the
input layer ever splits large asks (e.g. a future buffered-window cap at
64 KiB making the 256 KiB bound fill in 4 asks), the depth-bound CONTRACT
(`assert_probe_depth_bounded`, the actual AC) still holds but this line
fails, rotting into a constant-bump. The comment even acknowledges
refills exist. What correct looks like: assert the SUM of asks ≤ bound
(and every ask ≤ remaining budget — already `assert_probe_depth_bounded`),
not the exact one-request shape.
Confidence: MEDIUM — Verification: read; the exact-shape pin currently
passes live (verified).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open_counting | 36-41 | REAL | — | real FileAccess + CountingStream wrap |
| TempDir::new/write/Drop | 48-69 | REAL | — | pid+counter unique, recursive cleanup; no tempfile dep |
| seeded_bytes | 73-88 | REAL | — | xorshift64*, state|=1 guards zero seed; correct |
| config | 90-95 | REAL | — | min_score 60 fixed |
| test depth_bound_holds_over_the_real_input_stack_for_every_window | 101-122 | REAL | S4 (F-002) | 7 bounds 12..=262144; ledger + depth + restore |
| test depth_bound_holds_at_the_doc_ceiling_over_the_real_stack | 124-144 | REAL | — | 1 MiB ceiling; fill+EOS ask exact; size 424452 verified real |
| test the_probe_never_consumes_real_stream_bytes | 146-169 | REAL | — | replay equality after probe at 3 bounds |
| test the_real_ladder_climbs_with_the_window | 171-200 | REAL | — | 8 exact score pins (70/80/90/95 × avi/mp4) |
| test extension_is_a_hint_not_truth | 206-245 | REAL | — | content-wins both ways; garbage error carries extension/size hints |
| test empty_and_one_byte_files_are_typed_unknowns | 247-260 | REAL | — | hostile-focus axis: empty + 1-byte → typed unknown |
| test pure_garbage_files_are_typed_unknowns | 262-292 | REAL | — | 3 seeds × 8 KiB; asserts no format named |
| test sub_magic_truncations_are_typed_unknowns_over_real_files | 298-327 | REAL | — | cuts 0/1/4/8/11 typed-unknown; cut-12 → avi@70, hints empty |
| test truncated_files_prove_the_eos_fill_is_bounded | 329-355 | REAL | — | 52-byte file, 64 KiB window, exact ask ledger, replay |
| test the_ceiling_is_enforced_before_any_byte_is_read | 361-373 | REAL | — | config refusal code pinned |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 10 | YES (when the feature is on) | exact ask ledgers, exact scores/sizes, typed error codes/stages, replay equality — no tautologies found; constants cross-checked against real fixture bytes |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "run with `--features contract` (CI's `--all-features` always does)" | probe_io.rs:16-17 | REAL for ci.yml/ci-full.yml; FALSE for rust.yml:26 and default local `cargo test` | workflows read; baseline "0 tests" captured |
| "The file (424_452 B) is smaller than the ceiling" | probe_io.rs:138 | REAL | wc -c = 424452 |
| "the 52 surviving bytes" | probe_io.rs:335 | REAL | wc -c = 52 |

## 7. Dead / duplicated / unreachable code
None. All helpers used. (The file itself is unreachable-as-tests in default
feature builds — F-001.)

## 8. Dependencies & flags
dev: testkit, input (real access stack), media_probe::contract (feature
`contract`, non-default — see F-001). No unsafe.

## 9. Open questions for the next agent
- crates/media-probe/src/contract.rs (CountingStream,
  assert_probe_depth_bounded) is the instrumentation these proofs lean on —
  audit it when assigned: if `requests()` recorded only `read` and not
  `read_into`, the "one window-sized fill" ledger could miss asks.
- Check whether any other integration suites share the silent-cfg pattern
  (rg `#![cfg(feature` in tests/) — same S3 shape would apply.
