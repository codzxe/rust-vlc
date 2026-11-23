# AUDIT: crates/codecs/src/video/mjpeg/huffman.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 463 / 3 prod fns + 9 tests (incl. in-test oracle) |
| File verdict | REAL — clean-room Annex F decoder, strong differential tests; one non-atomic-rebuild seam defect |
| Reachable from shipped entry point? | YES: apps/cli/src/pipeline_host.rs:242 `"mjpeg" => MjpegDecoder::new()` (the first vertical) → scan decode → `HuffmanDecoder::decode` (decoder.rs:792/:802 refresh_table → slots) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 mjpeg::huffman` → 9 passed; 0 failed, exit 0; rg/sed (callers, quotes) |

## 1. One-paragraph verdict
A genuine, spec-cited (Annex F.2.2.3) canonical Huffman decoder with an
8-bit look-ahead LUT whose equivalence to the bit-at-a-time walk is pinned
by a 600-round randomized differential against an in-test naive oracle —
I ran all 9 tests live, including that differential. Build validation is
total (oversubscription, count mismatch, 16-bit overflow all typed), and
the truncation semantics of the padded look-ahead are tested to never
fabricate symbols. The one real defect is at the persistence seam:
`rebuild` is not failure-atomic — a rejected table leaves `min/max/valptr`
half-updated while `lut` and `values` keep the old table — and the mjpeg
decoder's `dht_key` short-circuit (updated only on success) then lets a
subsequent frame decode against that hybrid without rebuilding.

## 2. What actually works (verified)
— verified by `./target/debug/deps/codecs-bae0f1756cc0a809 mjpeg::huffman`
→ `test result: ok. 9 passed; 0 failed` (exit 0), incl.
`lut_decode_matches_the_bit_walk_on_random_streams` (600 random tables ×
streams, fast-vs-naive symbol/Stop agreement) and
`truncation_mid_code_is_exhausted_not_fabricated`.

## 3. Findings

F-001 [S2]
Where: crates/codecs/src/video/mjpeg/huffman.rs:88-113 (+ consumer
crates/codecs/src/video/mjpeg/decoder.rs:781-813)
Code:
```rust
    pub(crate) fn rebuild(&mut self, bits: &[u8; 17], values: &[u8]) -> Result<(), Stop> {
        self.min_code = [0i32; 17];
        self.max_code = [-1i32; 17];
        self.val_ptr = [0usize; 17];
```
What is wrong: `rebuild` mutates `self` progressively and returns
`Err(Stop::InvalidCode)` mid-way (oversubscription at :101-104, count
mismatch at :111-113). On failure the decoder holds a HYBRID: canonical
arrays partially rewritten for the new table (all `max_code` reset to -1,
lengths up to the failure point set to new values), while `lut`
(:118-132, never reached) and `values` (:136-137, never reached) still
describe the OLD table. The consumer makes this worse:
`prepare_tables` (decoder.rs:781-813) propagates the error with `?`
BEFORE updating `dht_key`, so the key still matches the old payload; a
subsequent frame carrying the old (valid) tables hits the
`key.matches(info)` short-circuit at :782, skips the rebuild, and decodes
with the hybrid: fast path (≤8-bit codes) uses the old LUT — consistent —
but the slow path (9..=16-bit codes) uses the new partial
min/max/val_ptr against the old `values`, yielding wrong symbols or
spurious `Stop::InvalidCode` on a VALID frame.
Why it matters: this is the first-vertical video path; one damaged DHT
frame (per-frame DHT re-declaration is the AVI/MJPEG norm) permanently
poisons the persistent table slots for every following good frame.
What correct looks like: build into temporaries and swap on success, or
restore/reset the derived arrays on the error path (and have
`prepare_tables` clear `dht_key` on failure).
Confidence: HIGH — Verification: read (state sequencing traced through
both files); NOT executed — no test drives a failed `rebuild` followed by
a matching-key frame (UNVERIFIED by execution; the existing
oversubscription test uses `new`, which discards on error).

F-002 [S4]
Where: crates/codecs/src/video/mjpeg/huffman.rs:183
Code:
```rust
            let code = reader.peek_bits(length_u32).unwrap_or(0) as i32;
```
What is wrong: the slow-path walk has already verified
`available >= length` (:179-182), so `peek_bits` failing here would be a
reader invariant violation — but the failure is silenced into `code = 0`,
which would then be compared against min/max and could return a WRONG
symbol from the table instead of surfacing the inconsistency. Swallowed
error in the exact shape checklist 6.2 flags.
Confidence: HIGH — Verification: read (unreachable today by the available
check; the pattern is the defect).

F-003 [S4]
Where: crates/codecs/src/video/mjpeg/huffman.rs:254-261 (test module)
What is wrong: the only oversubscription test builds via
`HuffmanDecoder::new`; nothing tests `rebuild` failure semantics — the
exact seam that F-001 shows is broken. Test-honesty gap: the V-3 in-place
rebuild optimization (the reason `rebuild` exists) has no negative-path
test.
Confidence: HIGH — Verification: read (all 9 tests inspected).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Stop | 28-35 | REAL | - | two typed stops |
| HuffmanDecoder::new | 65-75 | REAL | - | failure discards (atomic) |
| HuffmanDecoder::rebuild | 88-139 | REAL (non-atomic) | S2 | F-001; canonical validation total; LUT fill in-bounds (val_ptr+offset < values.len() proven by count check) |
| HuffmanDecoder::decode | 146-204 | REAL | S4 | fast/slow paths sound; padding can't fabricate (tested); F-002 unwrap_or |
| tests: decode_naive oracle | 316-344 | REAL | - | spec walk kept as differential oracle |
| 9 tests | 228-462 | REAL | S4 | differential + truncation honesty strong; F-003 gap |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| decodes_each_present_code / code_absent / single_symbol / sixteen_bit | YES | exact symbol/Stop assertions |
| oversubscribed / code_overflow | YES | build rejections pinned (via new()) |
| truncation_mid_code_is_exhausted_not_fabricated | YES | the LUT's核心 honesty property |
| short_codes_decode_near_the_tail | YES | available-boundary semantics |
| lut_decode_matches_the_bit_walk_on_random_streams | YES | 600-round differential; any divergence in symbol or terminal Stop fails |
| (missing) rebuild-failure atomicity | - | F-001 seam untested |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Build is total (no panics)" | huffman.rs:5 | REAL | checked_add/get, 9/9 ran |
| "same bits consumed as the bit-at-a-time walk" | huffman.rs:21-23 | REAL | differential test ran live |
| "a table-set change on a persistent decoder therefore allocates nothing" | huffman.rs:82-84 | REAL (alloc) / FALSE (safety) | clear+extend reuses capacity; but failure leaves hybrid state (F-001) |
## 7. Dead / duplicated / unreachable code
None; `decode_naive` is deliberately test-only (oracle), clearly labeled.
## 8. Dependencies & flags
`super::bitreader` only. pub(crate) — correctly internal to mjpeg.
## 9. Open questions for the next agent
- decoder.rs (not assigned to me): confirm the frame-level outcome when
  `prepare_tables` errors (EngineError vs Recovered) and whether the
  pipeline drops the decoder instance — if the decoder is dropped per
  error, F-001's reachability shrinks; the persistent-decoder assumption
  comes from decoder.rs:397's "seen so far" diagnostics + V-3's premise.
- Add a regression test: bad DHT frame → good frame with the pre-bad
  payload → assert decode correctness.
