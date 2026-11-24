# AUDIT: crates/plugin-api/tests/testkit_wiring.rs
| Field | Value |
|---|---|
| Auditor | W12-A10 |
| Date | 2026-11-24 |
| Lines / functions | 59 / 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | Test artifact — runs in `cargo test --workspace` (baseline :4871-4872 both ok). The crate under test, plugin-api, IS in the lumen-cli release lane (baseline cargo-build-release-lumen-cli.txt:140) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `sed -n` quote verification; `rg -n 'ALL' crates/plugin-api/src/kind.rs` (16 kinds confirmed); `rg -n 'sixteen' docs/06-plugin-architecture.md` (:43 "The sixteen kinds", :49 "16 kinds"); `rg -n 'pub fn assert_err|assert_contains' crates/testkit/src/lib.rs` (real assert!-backed helpers); baseline cite :4871-4872 |

## 1. One-paragraph verdict
A small, honest wiring proof that does exactly what its header claims: it exercises plugin-api's otherwise-dead `[dev-dependencies]` entry on testkit (the declared purpose — closing the cargo-shear dead-dep finding T5) against real production vocabulary. The assertions are genuine: Semver precedence of a pre-release vs its release, two leading-zero error lanes with message-content checks, and the 16-kind registry vocabulary (count, uniqueness, three spellings) — all verified against kind.rs (`PluginKind::ALL: [PluginKind; 16]` with the 16 §1.2 kinds) and doc 06 §1.2. Both tests pass in the baseline. The only defect is a redundant sort.

## 2. What actually works (verified)
— verified by `audit/runs/baseline/cargo-test-workspace.txt:4871-4872` -> `testkit_helpers_exercise_the_vocabulary … ok`, `kind_vocabulary_round_trips_through_the_kit … ok`. Helper reality verified by read: testkit's `assert_contains` is `assert!(haystack.contains(needle), …)` (testkit/src/lib.rs:79-84) and `assert_err` panics on `Ok` (lib.rs:100-105) — the test cannot pass vacuously.

## 3. Findings
F-001 [S4]
Where: crates/plugin-api/tests/testkit_wiring.rs:41-44
Code:
```
    let mut names: Vec<&str> = PluginKind::ALL.iter().map(|k| k.as_str()).collect();
    names.sort_unstable();
    let distinct = names.iter().collect::<std::collections::BTreeSet<_>>().len();
    assert_eq!(distinct, names.len(), "manifest kind names are unique");
```
What is wrong: `sort_unstable()` is dead work — the distinctness check uses a `BTreeSet`, which dedups regardless of order, and nothing after reads the sorted order. Cosmetic; also `mut` exists only for the sort. Confidence: HIGH — Verification: read.

## 4. Per-function table (file <100 LOC — table optional, included for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| testkit_helpers_exercise_the_vocabulary | 18-34 | REAL | — | Semver precedence + 2 typed-error lanes; would fail on any parse/precedence/diagnostic regression |
| kind_vocabulary_round_trips_through_the_kit | 36-59 | REAL | S4 | count/uniqueness/spellings genuine; dead sort (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| testkit_helpers_exercise_the_vocabulary | YES | exact Ordering::Less + error-message content asserted through real assert!-backed helpers |
| kind_vocabulary_round_trips_through_the_kit | YES | `ALL.len()==16` + BTreeSet uniqueness + three exact spellings ("access", "audio-output", "service-discovery" — verified against kind.rs:81-96) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "testkit is a [dev-dependencies]-only crate" (contract demonstrated) | header :1-4 | REAL | plugin-api/Cargo.toml:32-33 declares testkit under [dev-dependencies]; this file is its only in-crate consumer |
| "closes that finding the way the contract intends" (T5 dead-dep) | header :9-12 | REAL | the dep is now exercised; both tests compile+pass only through testkit |
| "sixteen kinds … §1.2 rows present" | :39,45 | REAL | kind.rs:80 `[PluginKind; 16]`; doc 06 §1.2 "The sixteen kinds" |

## 7. Dead / duplicated / unreachable code
The `sort_unstable` (F-001). Nothing else.

## 8. Dependencies & flags
plugin-api (crate under test), testkit (dev). No features, no unsafe, no ignores.

## 9. Open questions for the next agent
- None. (Cross-note: the media-types precedent file carries the same test name — baseline :3812 vs :4871 — harmless, different crates.)
