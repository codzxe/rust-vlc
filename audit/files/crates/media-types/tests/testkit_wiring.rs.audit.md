# AUDIT: crates/media-types/tests/testkit_wiring.rs
| Field | Value |
|---|---|
| Auditor | W02-A05 |
| Date | 2026-11-23 |
| Lines / functions | 43 / 2 tests |
| File verdict | REAL (thin by design, and it does what it claims) |
| Reachable from shipped entry point? | N/A — test file; executed under `cargo test -p media-types` (CI: baseline cargo-test-workspace.txt; locally: binary run below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/testkit_wiring-c5d9a0408bf37964` -> 2 passed, exit 0; rg cross-refs (Cargo.toml dev-dep section, no testkit in media-types/src, xtask check_architecture rule 5) |

## 1. One-paragraph verdict
A 43-line wiring proof that does exactly what its header claims: it compiles `testkit` helpers against `media-types`' production vocabulary and asserts real behavior — `Timestamp::from_parts(90_000, 1, 90_000)` → 1 s in nanos (±1), the zero-numerator `timebase.zero_component` error path (time.rs:82/155), and `Packet::is_stale` semantics for both current and advanced generations (packet.rs:292). Both tests were executed by me (2/2, exit 0) and would fail under the obvious mutations (inverted staleness, non-rejecting from_parts, broken nanos math). The dev-only direction contract it demonstrates is structurally true (testkit appears ONLY under `[dev-dependencies]` at crates/media-types/Cargo.toml:24-29, and `rg testkit crates/media-types/src/` is empty) and IS mechanically enforced today by `xtask check-architecture` rule 5 `testkit-direction` (tools/xtask/src/cmd/check_architecture.rs:16,109,132) — which makes the file's own header ("will LATER enforce mechanically") stale. Two S4 nits, nothing more.

## 2. What actually works (verified)
— `./target/debug/deps/testkit_wiring-c5d9a0408bf37964` -> "test result: ok. 2 passed" exit 0 (testkit_helpers_exercise_the_vocabulary, generation_and_staleness_through_the_kit).
— Dev-only contract: `sed -n '15,32p' crates/media-types/Cargo.toml` shows `[dev-dependencies] testkit = { path = "../testkit" }` and NO `[dependencies]` section ("No dependencies. Deliberately boring"); `rg -rn "testkit" crates/media-types/src/` -> zero hits.
— Mechanical enforcement exists: `rg -n "testkit-direction" tools/xtask/src/cmd/check_architecture.rs` -> rule 5, "testkit only ever in [dev-dependencies]" (L16, L109, L132).

## 3. Findings
F-001 [S4]
Where: crates/media-types/tests/testkit_wiring.rs:4-6
Code:
```
//! the production vocabulary — the concrete demonstration of the
//! direction contract that card 1.E1's `check-architecture` will later
//! enforce mechanically.
```
What is wrong: the comment is stale. `check-architecture` (card 1.E1) is implemented and enforces the `testkit-direction` rule today (tools/xtask/src/cmd/check_architecture.rs:16 "testkit appears in no [dependencies]/[build-dependencies] table", rule id at L109/L132). The header under-sells the current state ("will later"). Confidence: HIGH — Verification: read + rg (cmd/mod.rs:19 marks check-architecture "(implemented)").
F-002 [S4]
Where: crates/media-types/tests/testkit_wiring.rs:23-24
Code:
```
let record = HardwareClass::Mock;
assert_eq!(record.as_str(), "mock");
```
What is wrong: this third assertion exercises `testkit`'s OWN enum (`testkit::HardwareClass`), not the media-types vocabulary — a self-referential check inside a test named "…exercise_the_vocabulary". It pins the evidence label (doc 10 §7.1) but proves no cross-crate wiring. Confidence: HIGH — Verification: read (import at L9: `use testkit::{HardwareClass, …}`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| test testkit_helpers_exercise_the_vocabulary | 11-25 | HONEST | S4 | 2 of 3 assertions hit production types (Timestamp math, zero_component error); third is testkit-self (F-002) |
| test generation_and_staleness_through_the_kit | 27-43 | HONEST | — | real is_stale both directions (current ⇒ fresh, advanced ⇒ stale) |

## 5. Test honesty (if tests exist)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| testkit_helpers_exercise_the_vocabulary | YES | as_nanos drift > 1 ns fails; from_parts accepting num=0 fails assert_err; zero_component string rename fails assert_contains |
| generation_and_staleness_through_the_kit | YES | is_stale inversion fails either assert; GenerationId::next failure trips .must() |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "testkit is a [dev-dependencies]-only crate (MP §3.7)" | testkit_wiring.rs:1-2 | REAL | Cargo.toml:24-29 (dev-only section); no src usage; xtask rule 5 |
| "check-architecture will later enforce mechanically" | testkit_wiring.rs:5-6 | STALE-COMMENT | enforcement implemented today (check_architecture.rs rule `testkit-direction`) — F-001 |
| "Assert helpers against real L0 types" | testkit_wiring.rs:13 | REAL (mostly) | Timestamp/error assertions yes; HardwareClass is testkit's own (F-002) |

## 7. Dead / duplicated / unreachable code
None in the file. Observation (not a finding): a stale build artifact `target/debug/deps/testkit_wiring-9c59e186e093eb25` contains a since-removed test `kind_vocabulary_round_trips_through_the_kit` (verified via `--list`; a similar live test exists at crates/plugin-api/tests/testkit_wiring.rs:37) — the current source has exactly the 2 tests audited above. Why it was removed: UNVERIFIED (no git access by rule).

## 8. Dependencies & flags
media-types itself has zero [dependencies] (L23 comment + manifest); this test file uses only media-types::prelude + testkit (path dev-dep). No feature flags, no cfg gates, no unsafe.

## 9. Open questions for the next agent
- Who audits crates/testkit itself (the `must`/`assert_*` helpers all test crates rely on)? Its panic-on-Ok semantics are the failure-detection backbone for every `#[test]` using `.must()`.
- If the removed `kind_vocabulary` test was deleted because a `Kind` vocabulary moved out of media-types, confirm no coverage hole was left (media-types' own unit tests presumably cover it — outside my assignment).
