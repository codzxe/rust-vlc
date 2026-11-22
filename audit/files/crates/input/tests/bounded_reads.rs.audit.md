# AUDIT: crates/input/tests/bounded_reads.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 174 / 5 `#[test]` fns + shared `common` module (not my file) |
| File verdict | REAL-UNPROVEN (integration test file; genuinely behavioral over real files) |
| Reachable from shipped entry point? | N/A (test file) — the code under test (`FileAccess`/`FileSource`/`BufferedReader`) is the shipped local-open path (my file.rs audit) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | full-file read; cross-check of asserted error codes against input/src (capacity-zero/out-of-range at file.rs:137-142; seek-out-of-range); W00 baseline cargo-test.log (input suite green, UNVERIFIED by my own run) |

## 1. One-paragraph verdict

The card-2.A1 acceptance suite over **real files** (not the synthetic source): capacity validation at the Access surface (zero/over-ceiling typed with exact kind+code+stage; legal 1/64/ceiling construct), boundary-straddling read sizes (1 through 1000 across a 64-byte window, byte-exact with position tracking at every step), the degenerate capacity-1 window, the window-never-grows proof over a real `FileSource` (2000-byte request through a 32-byte window, capacity and `buffered_bytes` asserted after), and seek semantics (window-hit vs window-drop, seek-to-length clean EOF, past-end typed error with cursor unmoved, recovery after failure). The module doc honestly scopes what is NOT proven here (the OS-facing chunk bound is proven on the synthetic source in the unit suite + `contract::assert_bounded_read` — my contract.rs audit).

## 2. What was checked and how (zero-findings file, §13 requirement)

- Every assertion is behavioral: byte-exactness against the `pattern` generator (`common::pattern`), exact positions, exact error codes, exact capacity values. No tautologies, no `assert!(true)`, no mocked subject — the subject (`BufferedReader`/`FileSource`/`FileAccess`) is the real shipped code over real temp files.
- Mutation reasoning (UNVERIFIED by execution): breaking the window's index math would fail `reads_straddle_the_window_boundary_byte_exactly` (position assert at 78); growing the buffer would fail `the_fixed_window_never_grows_over_a_real_file` (126-127); breaking cursor-unchanged-on-failed-seek would fail line 167; misclassifying capacity errors would fail the kind/code/stage asserts at 27-29 and 35-47.
- Error-code strings cross-checked against the producing code: `input.buffer.capacity-zero` / `input.buffer.capacity-out-of-range` (file.rs:137-142), `input.seek.out-of-range` (file.rs:91 / seekable.rs) — consistent.
- W00 baseline: the workspace `cargo test` run (1751 pass) includes this suite green — cited from audit/runs/baseline/cargo-test.log; I could not re-run (ADAPTATION: no cargo).

## 3. Per-function table

| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `access_capacity_bounds_are_typed` | 24-58 | REAL-UNPROVEN | — | zero/over/1/64/ceiling/default; ResourceLimit payload fields asserted |
| `reads_straddle_the_window_boundary_byte_exactly` | 63-88 | REAL-UNPROVEN | — | 10 chunk sizes across the 64-byte boundary; position tracked per read |
| `capacity_one_reads_byte_exactly` | 92-110 | REAL-UNPROVEN | — | degenerate window |
| `the_fixed_window_never_grows_over_a_real_file` | 115-129 | REAL-UNPROVEN | — | the D-2 memory bound on the real path |
| `seek_positions_and_bounds_over_a_real_file` | 135-173 | REAL-UNPROVEN | — | window-hit/drop, EOF, typed past-end, recovery |

## 4. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all five | YES | see §2 mutation reasoning |

## 5. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "reads stay byte-exact through a fixed, capacity-bounded window over a real file" | module docs 1-4 | REAL-UNPROVEN | the five tests; green in W00 run |
| "the window is allocated once and never grows (asserted below on the real file)" | module docs 6-7 | REAL-UNPROVEN | test 4 asserts capacity + buffered_bytes post-read |
| "the OS-facing chunk bound … is proven with instrumentation on the synthetic source" | module docs 8-10 | REAL (scope honesty) | proven in the unit suite (contract.rs audit) — correctly not claimed here |

## 6. Dead / duplicated / unreachable code

None (test file; all tests run in the default harness — no `#[ignore]`, no cfg gates).

## 7. Dependencies & flags

dev-deps: `input` (path), `media-types`, `plugin-api`, `testkit`; `mod common` (tests/common — shared fixtures, another agent's file).

## 8. Open questions for the next agent

- `tests/common/mod.rs` (TempFile/pattern) — fixture quality; small shared module, likely another agent's list.
- The unit-suite counterpart for the chunk bound lives in src (contract.rs) — audited by me; both halves of the 2.A1 AC exist and are green per W00.
