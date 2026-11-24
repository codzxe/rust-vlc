# AUDIT: crates/testkit/src/alloc_count.rs
| Field | Value |
|---|---|
| Auditor | W16-A10 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / functions | 328 / 9 fns + unsafe GlobalAlloc impl (4 methods) + 3 unit tests |
| File verdict | REAL (baseline-verified: unit + e2e + doctest all green) |
| Reachable from shipped entry point? | NO — dev/test instrument; reachable from 12+ test binaries across codecs/playback-engine/audio-output/desktop (rg consumers) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg consumers; baseline citations L6205-6248 |

## 1. Verdict
The zero-allocation proof primitive is real and measured, not argued: a GlobalAlloc wrapper
that forwards verbatim to System and bumps atomics, a snapshot/delta guard whose assert_zero
fails loudly, plus a whole-binary mutex serializer to keep sibling test threads out of measured
windows. Every claim in the module docs is backed by code I can read and by baseline runs:
3 unit tests ok (L6205-6207), the e2e install test `instrument_end_to_end ... ok` (L6237), the
doctest compiles (L6244), package exit 0 (L6248).

## 2. What actually works (verified)
- Baseline (cargo-test-workspace.txt): `counters_track_note_pairs_and_the_delta_arithmetic`,
  `assert_zero_fails_loudly_on_a_nonzero_window`, `const_constructor_matches_default` — all ok;
  `tests/alloc_count_e2e.rs` 1 passed — the process-global wiring is proven, not just claimed.
- Consumers are real and many: crates/codecs/tests/{pcm_zero_alloc,zero_alloc_decode}.rs,
  crates/audio-output/tests/zero_alloc_null_sink.rs, apps/desktop/tests/zero_alloc_transport.rs,
  the soak harness (5 files) — this is the workspace's D-6 primitive in actual use.
- Unsafe audit: alloc/alloc_zeroed/dealloc/realloc each forward to System with the identical
  layout; counters update only on non-null results (:241-243, 268-271); no allocation, lock, or
  layout arithmetic inside the allocator — the documented invariant (:231-235) is upheld.
- Realloc accounting is honest: a grow inside a window registers BOTH a dealloc event of the
  old size and an alloc event of the new size (:262-273), so assert_zero catches it via
  alloc_calls/total_bytes even though deallocs alone are legal.

## 3. Findings
F-001 [S4]
Where: crates/testkit/src/alloc_count.rs:100-108
Code:
        self.total_bytes.fetch_add(size as u64, Ordering::Relaxed);
        self.live_bytes.fetch_add(size as isize, Ordering::Relaxed);
What is wrong: `size as isize` silently wraps on 32-bit targets for sizes > 2^31 (and the u64
path can wrap total_bytes only after 2^64 bytes — unreachable). live_bytes is test-only
bookkeeping never asserted by assert_zero, so the blast radius is a wrong debug number in a
failure message; on the shipped 64-bit targets it cannot trigger. Workspace targets are 64-bit
desktop + wasm32 (4 GiB arena) — unreachable there too.
What correct looks like: saturating_add on live_bytes or an isize::try_from.
Confidence: LOW (impact) — Verification: read.
Zero-findings list otherwise: unsafe blocks audited (4, each forwarded, SAFETY comments present);
Relaxed ordering acceptable under the documented whole-binary serialization (:39-46) — any
counter split is still monotonic per counter; catch_unwind unit test proves the failure path;
poisoned mutex handled (:66-69); doc claims vs code checked one-by-one (usage example matches
the API, must_use annotations on all evidence-returning methods).

## 4. Per-function table (>100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| serialize_tests | 64-70 | REAL | — | poison-recovering binary serializer |
| AllocState::new | 91-98 | REAL | — | const ctor; tested == Default |
| note_alloc / note_dealloc | 100-109 | REAL | S4 | isize cast nit (F-001) |
| AllocState::snapshot | 113-120 | REAL | — | zero-alloc read |
| AllocState::guard | 126-131 | REAL | — | window start |
| CountingGuard::delta | 175-183 | REAL | — | underflow-free (monotonic counters) |
| CountingGuard::assert_zero | 194-204 | REAL | — | loud failure with deltas; #[track_caller] |
| CountingGlobalAlloc::{new,state} | 218-228 | REAL | — | const ctor for the static |
| GlobalAlloc impl | 237-274 | REAL | — | verbatim forwarding, null-guarded counting |
| tests | 276-328 | REAL | — | 3 tests incl. catch_unwind failure-path proof |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| counters_track… | YES | exact delta arithmetic (2/1/576/64) |
| assert_zero_fails_loudly… | YES | catch_unwind must see the panic |
| const_constructor_matches_default | YES (weak) | equality of two zeros — tautology risk acknowledged; the strong wiring proof is the separate e2e binary |
| alloc_count_e2e (other file) | YES | installs the real global allocator and measures |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "measured, not argued" D-6 primitive | :1-7 | REAL | e2e test green in baseline L6237 |
| "Locking an uncontended Mutex allocates nothing" | :62-63 | REAL | guard path has no alloc (read) |
| "counting is unconditional (never paused)" | :39-43 | REAL | no pause flag exists in code |

## 7. Dead / duplicated / unreachable code
None. live_bytes is informational only (never asserted) — intentional, documented at :82-84.

## 8. Dependencies & flags
std only; `#[allow(unsafe_code)]` (:236) is the single deviation from the workspace unsafe ban,
Lead-approved (ED-b) and documented — not lint-hiding.

## 9. Open questions for the next agent
None blocking. If a 32-bit target ever ships, revisit F-001.
