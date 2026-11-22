# AUDIT: crates/input/tests/truncation.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 211 / 6 `#[test]`s + open_with_capacity + assert_truncation helpers; whole file gated `#![cfg(feature = "contract")]` |
| File verdict | **REAL tests, GATED OFF the default suite** — card 2.A1's acceptance evidence runs only under `--features contract` / `--all-features` |
| Reachable from shipped entry point? | Subject: `input::FileAccess` — the production access layer every local open uses. The tests themselves: NOT in `cargo test --workspace` (the W00 baseline command) — the `contract` feature is non-default (`crates/input/Cargo.toml [features] contract = []`, no `default =` entry), so the file compiles to nothing without the flag. CI's `ci-full.yml:169` / `ci.yml:234` run `cargo test --workspace --all-features`, which would include them — whether CI itself ever ran is wave 17's question (baseline parity report: 0/252 verified). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | Read of the full file; sed of crates/input/Cargo.toml `[features]`; grep of .github/workflows/ci*.yml for `cargo test` flags. No cargo (static). |

## 1. One-paragraph verdict

The tests themselves are excellent: real fault injection (`set_len` on live files under open streams, several cursor/window geometries), exact `(expected, found)` byte pairs in the typed error, the full Display string asserted verbatim (44-47), the snapshot-drain semantics bounded and byte-verified (100-106: at most one window of pre-truncation bytes, byte-identical to the pre-shrink data), stickiness of the error across retries (113-115), surviving-region reads plus exact-boundary reads after the shrink (171-188), and a failed seek that provably never moves (203-210). The one real problem is the gate: `#![cfg(feature = "contract")]` means the plain `cargo test --workspace` run — the W00 baseline command, 1751 passed — never compiled or executed any of this. Card 2.A1's truncation acceptance evidence is invisible to the default suite, and its existence in CI depends on workflows whose actual execution is itself unverified.

## 2. What was checked

- Fault geometry coverage: mid-read (53-67, cursor 200, window 16 — multiple refills), below-cursor snapshot drain (76-116), to-zero (121-134), mid-refill (140-155), surviving-region + crossing (162-189), seek-bounds (194-211). Six distinct geometries — no copy-paste shapes.
- The silent-EOF failure mode is explicitly asserted against (94: `assert!(!piece.is_empty(), "a shrunken source never EOFs cleanly")`).
- The Display assertion (43-47) pins the redaction promise ("no secrets") to an exact string.
- `assert_truncated` (input::contract) re-derives the (expected, found) pair — the contract module doubles as the structured-error oracle.
- Feature gate chain: `crates/input/Cargo.toml` — `contract = []`, no default inclusion (verified by sed). CI: `ci-full.yml:169` and `ci.yml:234` both pass `--all-features` (verified by grep). Baseline: `cargo test --workspace --locked` (BASELINE-SUMMARY.md) — no features ⇒ these tests absent from the 1751.

## 3. Findings

### F-001 [S3] — the acceptance tests for truncation are feature-gated out of the default test run
Where: crates/input/tests/truncation.rs:11-15; crates/input/Cargo.toml `[features]`
Code (truncation.rs:11-15):
```rust
//! Feature-gated (audit F-2): the suite drives the shared executable
//! contract in `input::contract`, which is test/conformance-only —
//! run with `--features contract` (CI's `--all-features` always
//! does).
#![cfg(feature = "contract")]
```
What is wrong: the gate is honest and its rationale is stated (the contract module carries "panicky conformance vocabulary" kept out of release binaries — Cargo.toml comment), but the consequence is that the default `cargo test --workspace` — the command the W00 baseline ran, and the command any contributor runs — silently compiles this file to nothing: no test output, no skip marker, no warning. Six acceptance tests for a data-integrity feature (typed truncation vs silent EOF vs panic) are invisible unless the runner knows the flag.
Why it matters: green-by-omission. The W00 baseline's "1751 passed" cannot speak for these tests; their pass status is UNVERIFIED in this audit (no cargo). The header's claim that "CI's --all-features always does" is true of the workflow *files* (ci-full.yml:169, ci.yml:234) but CI execution itself is unverified per the baseline parity report (0/252 verified, CI billing-blocked history).
What correct looks like: move the pure assertions (typed error, exact pairs) into an ungated test and keep only the contract-module *helpers* behind the feature; or make `contract` a default feature of dev builds (`[features] default = []` + dev-dependency self-feature, or `required-features` documentation in the README of the crate).
Confidence: HIGH — Verification: read (feature table + workflow greps + baseline command).

## 4. Per-function table

| Item | Lines | Verdict | Notes |
|---|---|---|---|
| open_with_capacity | 30-34 | REAL | small windows force multi-refill geometry |
| assert_truncation | 38-48 | REAL | code + (1000, 300) + exact Display |
| file_truncated_mid_read_surfaces_the_typed_error | 53-67 | REAL | core AC; live size truth (66) |
| truncation_below_the_cursor_drains_the_snapshot_then_errors_typed | 76-116 | REAL | the strongest test: bounded snapshot, byte-identity, stickiness |
| truncation_to_zero_is_typed | 121-134 | REAL | degenerate shrink |
| truncation_lands_mid_refill | 140-155 | REAL | fault mid-transfer |
| surviving_region_reads_then_crossing_errors_typed | 162-189 | REAL | seek-in-live + exact-boundary read |
| seek_beyond_the_shrunk_length_is_typed_out_of_range | 194-211 | REAL | failed seek never moves |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all six | YES | exact byte pairs, exact strings, position checks, bounded snapshot — silent-EOF and panic regressions both fail loudly |

UNVERIFIED by execution — and notably UNVERIFIED BY THE BASELINE TOO, because the gate excluded them from `cargo test --workspace --locked` (F-001). This file's pass status has no recorded run in audit/runs/baseline/.

## 6-9

- Doc claim "never a panic, never a silent EOF" (2-3): the tests assert both halves (the loop at 89-99 fails on empty Ok; every Err is destructured).
- Doc claim "CI's --all-features always does": REAL for the workflow files; UNVERIFIED for actual CI execution (wave 17).
- No dead code within the file.
- Open questions: input/src/contract.rs (the shared oracle) and file_stream.rs (the grow complement) belong to the input crate's agent; wave 16 should count how many suites repo-wide hide behind non-default features.
