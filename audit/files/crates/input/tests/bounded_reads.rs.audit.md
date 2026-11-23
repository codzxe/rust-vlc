# AUDIT: crates/input/tests/bounded_reads.rs
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-23 |
| Lines / functions | 173 / 5 tests (+ shared common/mod.rs helpers) |
| File verdict | REAL (real files, real shipped code path, executed 5/5) |
| Reachable from shipped entry point? | YES — code under test is the shipped file lane: `lumen open <path>` → demux_host.rs:422-424 `input::FileAccess::new().open(&uri)` → input file.rs/BufferedReader |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p input --test bounded_reads` -> 5 passed / 0 failed (exit 0); read of buffered.rs accessors + contract.rs proof; `sed -n` quote verification |

## 1. One-paragraph verdict
Strong, honest acceptance tests over real temp files: capacity validation with exact typed codes and budget/attempted values, boundary-straddling reads (capacities 64 and degenerate 1) reconstructing byte-exact data with position checks at every step, the fixed-window-no-growth proof over a 2000-byte request through a 32-byte window, and a seek matrix covering window hits, backward drops, seek-to-length EOF, and the typed past-end error with cursor unmoved. All 5 pass under my run, and the code under test is the shipped CLI file lane. One real defect: the header still sells the retired 2.A1 invariant ("the source never sees a read larger than the capacity") while the proof it cites was repinned to request-bounded reads — the doc contradicts the contract it points at.

## 2. What actually works (verified)
— verified by `cargo test -p input --test bounded_reads` -> `test result: ok. 5 passed; 0 failed` over real files (TempFile writes to std::env::temp_dir; file_uri built through the shared `SourceUri::from_file_path` constructor — no hand-rolled format!).
— verified by read: `buffer_capacity()` returns `self.buf.len()` (buffered.rs:119-120) and `buffered_bytes()` returns `self.end - self.start` (buffered.rs:124-126) — so the no-growth assertions observe the live window, not a config echo.

## 3. Findings
F-001 [S3]
Where: crates/input/tests/bounded_reads.rs:6-10 vs crates/input/src/contract.rs:362-370
Code (this file):
```
//! The reader's memory is the fixed window plus one bounded result: the
//! window is allocated once and never grows (asserted below on the real
//! file), and the OS-facing chunk bound — the source never sees a read
//! larger than the capacity — is proven with instrumentation on the
//! synthetic source in the unit suite and `contract::assert_bounded_read`.
```
Code (contract.rs, the cited proof):
```
/// The bounded-memory proof (card 2.A1 AC, repinned for the 7-c part-2
/// bypass): reading `want` bytes — many times the buffer capacity —
/// returns byte-exact data via **one full-size source call** for the
```
What is wrong: the header asserts the retired invariant ("source never sees a read larger than the capacity"); the cited `assert_bounded_read` was repinned to the opposite semantics — one full-size source call bounded by the CALLER'S request, with contract.rs explicitly calling the old "chunked to the capacity" pin "an implementation detail, not the memory bound". A reader auditing the 2.A1 acceptance criteria from this file believes OS reads are window-clamped; they are request-clamped (a `read(2000)` with a 32-byte window issues a 2000-byte source read). Memory bound unaffected (window + caller-owned result), but the documented AC is stale. What correct looks like: rewrite the header to the repinned bound.
Confidence: HIGH — Verification: read (both files, quote-verified).
F-002 [S4]
Where: crates/input/tests/bounded_reads.rs:116-129
Code:
```
    let got = reader.read(2000).must();
    assert_eq!(got, data, "a 2000-byte request through a 32-byte window");
    assert_eq!(reader.buffer_capacity(), 32, "the buffer never grows");
    assert!(reader.buffered_bytes() <= 32);
```
What is wrong: precision only — the no-growth proof is post-read state (persistent growth IS caught because `buffer_capacity()` is `buf.len()`; a transient over-allocation inside `read()` would not be observable from these accessors). The header's "asserted below on the real file" is accurate for the persistent case; noting the limit of what is observable.
Confidence: HIGH — Verification: read (accessor semantics).

## 4. Per-function table (test inventory)
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| access_capacity_bounds_are_typed | 24-58 | REAL | — | Exact codes/kinds/budgets; legal set incl. 1 and ceiling; default = 64 KiB pinned |
| reads_straddle_the_window_boundary_byte_exactly | 60-88 | REAL | — | 10 chunk sizes across the 64-byte boundary; position tracked per step; past-end clean-empty |
| capacity_one_reads_byte_exactly | 90-110 | REAL | — | Degenerate window, byte-exact |
| the_fixed_window_never_grows_over_a_real_file | 112-129 | REAL | S4 | F-002 precision note |
| seek_positions_and_bounds_over_a_real_file | 131-173 | REAL | — | Window hit/drop, EOF-at-length, typed past-end with cursor unmoved, recovery seek |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| access_capacity_bounds_are_typed | YES | 4 refusal fields + 3 legal constructions + default |
| reads_straddle_the_window_boundary_byte_exactly | YES | Byte-exact `got == data` over all straddles; breaks on any window/reconstruction bug |
| capacity_one_reads_byte_exactly | YES | Degenerate path, exact bytes + final position |
| the_fixed_window_never_grows_over_a_real_file | YES (persistent growth) | buf.len() pinned at 32 after a 62× request |
| seek_positions_and_bounds_over_a_real_file | YES | 6 seek shapes incl. failed-seek cursor invariance |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "reads stay byte-exact through a fixed, capacity-bounded window over a real file" | bounded_reads.rs:1-4 | REAL | Tests 2-4, executed |
| "the window is allocated once and never grows" | bounded_reads.rs:6-7 | REAL (persistent) | F-002 precision |
| "the source never sees a read larger than the capacity" | bounded_reads.rs:8-9 | FALSE (retired) | contract.rs:362-370 repinned to request-bounded reads (F-001) |

## 7. Dead / duplicated / unreachable code
- None. common/mod.rs helpers (TempFile with pid+counter uniqueness, drop-recursive cleanup, shared-constructor URIs) are real and shared across the input integration binaries.

## 8. Dependencies & flags
- Test-only; exercises input's public API + plugin_api::Access. No feature gates. Std-only temp handling (no tempfile crate) per C5.

## 9. Open questions for the next agent
- The repin at contract.rs ("7-c part-2 bypass") suggests a demux-side consumer needed full-size reads — if any doc/matrix row still phrases the 2.A1 AC as window-clamped OS reads, that row inherits F-001's staleness.
