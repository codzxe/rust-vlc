# AUDIT: crates/subtitles/tests/fuzz_survival.rs
| Field | Value |
|---|---|
| Auditor | W09-A04 |
| Date | 2026-11-23 (repo frozen; audit session 2025-07-14 clock) |
| Lines / functions | 229 / 1 helper struct (2 methods) + 1 seed builder + 4 tests |
| File verdict | REAL (executed; deterministic; asserts mutations bite in both directions) |
| Reachable from shipped entry point? | N/A (test) — runs in workspace test lane; code under test (subtitles::model) is NOT reachable from shipped binaries (pgs/mod.rs audit F-002) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `./target/debug/deps/fuzz_survival-9fa7a848a5721d3d` → 4 passed, 0 failed, 0.18 s (exit 0); baseline cross-check cargo-test-workspace.txt:6068-6076; rg/sed verification of helpers under test |

## 1. One-paragraph verdict
A genuinely honest fuzz-lite suite: deterministic xorshift64* with fixed
non-zero seeds, four distinct mutation operators (header-biased byte flip,
truncate, insert, chunk splice), a strong idempotence invariant (accepted
mutation must re-serialize and re-parse to the identical value), and — the
part most fuzz suites fake — counters proving the mutations actually produce
both accepted (>100) and rejected (>100) documents. Constructor and resolve
sweeps hit full-range i64/i32 inputs (i64::MIN/MAX included by construction)
and assert result invariants, not just absence of panic. It passes; I ran it.
No findings. What was checked is listed in §3.

## 2. What actually works (verified)
— `./target/debug/deps/fuzz_survival-9fa7a848a5721d3d` → `test result: ok.
4 passed; 0 failed; 0 ignored; 0 measured` in 0.18 s.
— Mutation-operator safety re-derived: splice uses `len = 1 +
below(24.min(bytes.len()))` and `from = below(bytes.len()-len+1)` — no
underflow; truncate/insert use `below(len+1)` — in-range.
— RNG: xorshift64* correctly implemented (L16-23); all four seeds are
non-zero constants → deterministic, reproducible, no degenerate zero-state.
— Downstream invariants verified against source: `Percent::new` rejects
outside 0..=100 and stores u8 (position.rs:41-46) so the sweep's
`pc.get() <= 100` assertion is complete; `percent_to_px`/`line_number_to_px`
use u64 intermediates + saturating/clamp (position.rs:296-318), so the
4096-px viewport cap in the sweep leaves no untested overflow path (extremes
are unit-tested at u32::MAX, position.rs:400).

## 3. Findings
None. Checked, with method:
1. Panic-freeness claims — executed (4/4 pass, incl. 4000 mutation rounds,
   2000 random buffers, 2×20 000 constructor/resolve sweeps).
2. Idempotence claim ("accepted documents re-serialize stably") — read
   (L128-135: serialize → parse → assert_eq on the full cue vec; a drift
   fails the test with the round number).
3. "Mutations must actually bite" — read (L140-145: accepted>100 AND
   rejected>100 guards against a fuzz that only produces garbage).
4. Ok-or-typed claims — read: `parse_document` returns
   `Result<_, WireError>` (wire.rs:191, typed by signature); constructor
   sweep `matches!`-asserts the exact ModelError variants (L182-185,
   190-193, 199-202).
5. Fuzzer self-safety (no index panic inside the fuzzer itself) — re-derived
   arithmetic (see §2).
6. Coverage boundaries — noted, acceptable: random buffers capped at 300
   bytes (header formation only; larger docs covered by the mutation test
   over the ~5 KB seed); viewports capped at 4096 px (safe: math is
   saturating by construction, extremes unit-tested); `String::from_utf8_lossy`
   means the parser never sees invalid UTF-8 — deliberate, documented at
   L124-125.
7. Test-name honesty — the file fuzzes the model WIRE format
   (MAGIC-gated, wire.rs:196), not SRT/VTT/ASS; those have their own suites
   (srt_fuzz_survival.rs, vtt_fuzz.rs, in-crate ass fuzz). No name lies.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Lcg::next_u64` / `below` | 15-32 | REAL | — | xorshift64*, modulo below; deterministic |
| `seed_document` | 34-77 | REAL | — | 48 cues, plain/styled text incl. 日本語 + escapes, all 9 alignments, 3 line placements; serialized via the real `serialize_document` |
| `mutated_documents_never_panic` | 79-147 | REAL | — | 4000 rounds × 1-3 mutations, 4 operators, idempotence assert, bite counters |
| `random_buffers_never_panic` | 150-168 | REAL | — | 2000 buffers ≤300 B, ASCII-biased; result deliberately discarded (pure no-panic probe) |
| `constructor_sweep_is_ok_or_typed` | 170-205 | REAL | — | full-range i64 interval pairs, i32 percents, 0/0 viewports; variant-matched errors |
| `resolve_sweep_never_panics` | 207-229 | REAL | — | asserts resolved anchors stay within the viewport; line_height 0..199 incl. 0 (div-zero probe) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mutated_documents_never_panic | YES | parser panic fails the process; serialize/parse drift trips assert_eq; a too-weak mutator trips the bite counters |
| random_buffers_never_panic | YES (panic only) | result discarded — by design a no-panic probe, honestly named |
| constructor_sweep_is_ok_or_typed | YES | wrong error variant or broken invariant (contains/empty, duration sign) fails |
| resolve_sweep_never_panics | YES | out-of-viewport anchor (x_px > width) fails the assert |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every outcome is Ok-or-typed" | L3-4 | REAL | WireError/ModelError signatures + variant matches (ran) |
| "anything the parser accepts re-serializes stably (roundtrip idempotence under mutation)" | L4-5 | REAL | L128-135 assert_eq (ran) |
| "deterministic seeded mutations" | L2, 12 | REAL | fixed seeds, xorshift64* (re-derived) |

## 7. Dead / duplicated / unreachable code
None. No #[ignore], no cfg gates (rg verified).

## 8. Dependencies & flags
subtitles::model only (the crate under test) — no testkit here (no .must()
in this file), no features. Deterministic; no external fuzzer dependency.

## 9. Open questions for the next agent
- model/wire.rs auditor: the idempotence sweep is only as strong as
  serialize_document's canonicalization — confirm it emits a total order
  (sorted cues, stable style serialization), else assert_eq could pass
  vacuously for reordered-but-equal docs (it cannot — assert_eq is on the
  parsed Vec, order-sensitive; noting for completeness).
