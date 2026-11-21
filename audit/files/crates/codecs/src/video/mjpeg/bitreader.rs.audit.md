# AUDIT: crates/codecs/src/video/mjpeg/bitreader.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-11-21 |
| Lines / functions | 425 / 27 items (2 consts, ReaderStop, struct, 12 methods, mask_bits, 16 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open`/`decode-bench` → apps/cli/src/pipeline_host.rs:242-245 `MjpegDecoder::new` → mjpeg/mod.rs:362 `decode_scan` → decoder.rs:585 `BitReader::new(entropy)`; huffman.rs:147-202 drives `refill`/`available`/`peek8_padded`/`peek_bits`/`consume`/`consume_all`; decoder.rs:602 `take_restart`; decoder.rs:968 `read_bits`. EXECUTED: baseline mp4-smoke-bench-avi-mjpeg-pcm.txt `v_decoded=60`; all 15 bitreader tests ok (test-workspace.log.txt:651-665) + huffman's random-stream equivalence oracle ok (:703)) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `Read` full file; `rg -n "peek_bits\|peek8_padded\|consume(\|read_bits\|refill"` over mjpeg/ (production callers: huffman.rs:147-202 with peek lengths 9..=16, decoder.rs:602/968/644-649); `sed -n '173,190p' huffman.rs` (caller context for the n=32 question); hand-simulation of the accumulator across feed/consume/peek (the 0xab cd ef 01 23 test traced bit-by-bit); `grep -n "video::mjpeg::bitreader" baseline log` (15/15 ok). |

## 1. One-paragraph verdict
This is a correct, spec-faithful entropy reader: I hand-simulated the 32-bit accumulator through feeds, consumes, peeks, stuffed `FF 00` pairs, fill bytes, and both marker bands, and the state machine matches §F.1.2.3/F.2.2.4 exactly — stuffed literals re-enter as `0xFF`, fill bytes are skipped one at a time, restart markers park the position and are consumed only by a sequence-checked `take_restart`, truncation mid-stuffing-pair is typed as end-of-data, and no bit is ever fabricated past the end. The refill discipline (`while nbits <= 24`) provably never loses a valid bit on the `<< 8` (entry nbits ≤ 24 ⇒ window [23..0] moves to [31..8]), and the reader is pinned by 15 executed content-sensitive tests plus huffman's random-stream LUT-vs-bit-walk equivalence oracle. The defects are peripheral: `peek_bits`' inline mask expression panics on the documented-legal `n = 32` in debug builds (`1u32 << 32` shift-overflow) while the correct `mask_bits` helper sits unused 90 lines below (no current caller exceeds 16, so it is latent — and the baseline's `--release` test profile would not have caught it either); the struct's invariant doc misstates the accumulator layout; and `consume` guards its precondition with only a `debug_assert`.

## 2. What actually works (verified)
— Executed in the shipped lane: every MJPEG frame's entropy walk goes through this reader (AVI first-vertical, 60/60 frames decoded).
— Hand-simulation of the accumulator (the exact byte sequence of test `peek_bits_does_not_consume`): feed×4 → acc=0xabcdef01, nbits=32; consume(8) → nbits=24, acc untouched; peek(8) = acc>>(24−8)&0xff = 0xcd — the window is the LOW `nbits` bits, next bit at position `nbits−1`; refill at nbits=20 ≤ 24 feeds the fifth byte: acc=(0xabcdef01<<8)|0x23=0xcdef0123, nbits=28. All assertions of the test reproduced by hand — the implementation matches.
— Byte state machine verified against §F.1.2.3: `FF 00` → literal FF (pos+=2) ✓; `FF FF` → fill, pos+=1, re-examine (consecutive fills handled by the loop) ✓; `FF D0..D7` → `ReaderStop::Restart(m)` with pos parked at the FF ✓; other `FF xx` → `Marker` ✓; `FF` at EOS → EndOfData ✓ (tests 260-345 pin each arm).
— `take_restart` (219-230): verifies `m == expected_m`, clears acc/nbits (discarding the encoder's byte-boundary padding), advances pos past `FF Dm`, and on mismatch leaves the reader byte-identical (test 292-298 pins both the rejection and the preserved cause).
— `read_bits` hostile-input discipline: any `n` accepted; `n` beyond the remainder consumes the partial bits and returns None (test 398-406 with n=200) — no panic, no fabricated bits; `n=0` → Some(0) (test 392-395, the `s=0` DC-extend case).
— Overflow-safety of the shift discipline: `feed` is only ever entered with nbits ≤ 24 (refill's loop condition; `next_bit` only at nbits==0; `take_restart` resets), so `acc << 8` can never push a valid bit out of the u32.

## 3. Findings

### F-001 [S3] `peek_bits`' inline mask panics in debug builds for the documented-legal `n = 32`, while the correct `mask_bits` helper is unused
Where: crates/codecs/src/video/mjpeg/bitreader.rs:140-143 (contract at 131/135; unused helper at 234-243)
Code:
```
        Some(
            (self.acc >> (self.nbits - n))
                & ((1u32 << n) - 1).max(if n == 32 { u32::MAX } else { 0 }),
        )
```
What is wrong: The doc says "Look at the next `n ≤ 32` bits" and the `debug_assert!(n <= 32)` explicitly admits 32 — but for n=32 the expression evaluates `1u32 << 32` FIRST: in debug builds (and any build with overflow checks) that is a shift-amount-overflow panic; only in release (masked shift → 1, minus 1 → 0, `.max(u32::MAX)` → correct) does the intended fix-up actually work. The author clearly knew n=32 was special (the `.max(if n == 32 …)` patch) but bolted the fix on the wrong side of the shift. `read_bits` already uses the correct helper `mask_bits(n)` (line 201, handles n ≥ 32 properly at 238-240) — `peek_bits` reinvents it, incorrectly.
Why it matters: A legal-by-contract call panics in exactly the builds (debug/test) where new callers get written; and because the baseline test suite ran `--release` (INDEX.md recorded deviation), even a future test calling `peek_bits(32)` would pass in CI and detonate in a developer's debug run. Current callers are safe (huffman.rs:183 peeks 9..=16 only), so this is latent — hence S3, not higher.
What correct looks like: `Some((self.acc >> (self.nbits - n)) & mask_bits(n))` — one line, reuses the tested helper.
Confidence: HIGH — Verification: read + Rust shift-overflow semantics (debug panic / release mask); caller max verified by reading huffman.rs:175-183 (9..=16).

### F-002 [S4] The struct's invariant doc misstates the accumulator layout
Where: crates/codecs/src/video/mjpeg/bitreader.rs:47-48
Code:
```
    /// Bit accumulator: the `nbits` most significant valid bits hold
    /// the stream window (MSB of the window = next bit to consume).
```
What is wrong: The window is the LOW `nbits` bits of `acc` (next bit to consume at position `nbits − 1`; bits above are stale, already-consumed data) — proven by the hand simulation in §2 (after consume(8) of acc=0xabcdef01, the window 0xcdef01 occupies bits [23..0], not the most significant bits). As written, a maintainer would expect an MSB-aligned window and "fix" peek/consume accordingly.
Why it matters: Wrong invariant comments on bit-twiddling state are the classic seed of real regressions; the second sentence is correct, the first is not.
What correct looks like: `/// Bit accumulator: the window is the LOW nbits bits of acc (next bit to consume = bit nbits−1); bits above nbits−1 are stale consumed data.`
Confidence: HIGH — Verification: hand simulation (§2) + every accessor's arithmetic.

### F-003 [S4] `consume` guards its precondition with only a `debug_assert` — a release-build violation wraps `nbits`
Where: crates/codecs/src/video/mjpeg/bitreader.rs:162-165
Code:
```
    pub(crate) fn consume(&mut self, n: u32) {
        debug_assert!(n <= self.nbits);
        self.nbits -= n;
    }
```
What is wrong: In release, `consume(n > available())` underflows `nbits` (u32 wrap → ~4 billion), after which `peek_bits` computes `self.nbits - n` as a huge shift (shift-overflow panic in debug, garbage window in release). The precondition is documented ("Consume `n ≤ available()`") and every current caller respects it (huffman.rs:158-159/179-180 guard on `available` first), so this is contract-discipline, not a live bug.
Why it matters: pub(crate) API whose failure mode is catastrophic state corruption rather than a typed error, in a module whose whole ethos (per the module docs) is "no fabricated bits, typed stops".
What correct looks like: `if n > self.nbits { self.nbits = 0; } else { self.nbits -= n; }` (clamp — same observable contract as read_bits' tail consumption) or return a Result.
Confidence: HIGH — Verification: read + caller audit.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-25 | REAL | — | behavior contract verified arm-by-arm; "identical to the bit-at-a-time reader" pinned by tests + huffman oracle (703) |
| `RST_FIRST`/`RST_LAST` | 27-29 | REAL | — | D0..D7 ✓ |
| `ReaderStop` | 32-40 | REAL | — | consumed by decoder.rs:644-649 concealment-cause mapping |
| `BitReader` struct | 43-54 | REAL | S4 | invariant doc wrong → F-002 |
| `new` | 58-66 | REAL | — | trivial |
| `feed` | 70-112 | REAL | — | byte state machine spec-exact (§2); idempotent after stop ✓; truncated-pair typed ✓ |
| `refill` | 117-123 | REAL | — | `nbits <= 24` discipline makes every `<< 8` lossless (proven) |
| `available` | 127-129 | REAL | — | — |
| `peek_bits` | 134-144 | REAL | S3 | n=32 debug panic → F-001; n ≤ 16 for all current callers |
| `peek8_padded` | 151-157 | REAL | — | right-pad with zeros; LUT contract "padding never yields a decode" upheld by huffman's available-check |
| `consume` | 162-165 | REAL | S4 | debug_assert only → F-003 |
| `consume_all` | 170-172 | REAL | — | truncation path ✓ |
| `next_bit` (cfg(test)) | 179-188 | REAL | — | the equivalence oracle's reference walk; honestly test-gated |
| `read_bits` | 195-204 | REAL | — | any-n discipline; partial-bits-consumed semantics pinned (tests 320-326, 398-406); uses mask_bits correctly |
| `stop` | 207-209 | REAL | — | — |
| `take_restart` | 219-230 | REAL | — | sequence check ✓, padding discard ✓, pos advance ✓, error state-preserving ✓ (tests 276-298, 329-337, 340-345) |
| `mask_bits` | 234-243 | REAL | — | correct for 0/32/other — and UNUSED by peek_bits (F-001) |
| 16 tests | 249-424 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| reads_msb_first_in_order (651) | YES | exact bit sequence asserted |
| stuffed_ff_yields_literal_bits (661) | YES | 16 bits, first 8 all-ones |
| restart_marker_stops_feeding_and_can_be_taken (660) | YES | stop cause + take + post-marker behavior |
| wrong_restart_number_is_rejected (665) | YES | RST3≠RST0, cause preserved |
| non_restart_marker_stops_the_scan (652) | YES | Marker(0xd9) asserted |
| truncated_stuffing_pair_is_end_of_data (664) | YES | EndOfData cause |
| read_bits_returns_none_on_partial_availability (656) | YES | 5 remaining, 16 requested |
| fill_bytes_before_a_restart_are_tolerated (651→ line 651 is first; this at 651?) — (log line for it: 651) | YES | FF FF D0 → RST0 |
| take_restart_without_a_pending_marker_fails (663) | YES | err + reader still usable |
| peek_bits_does_not_consume (654) | YES | pins the word-refill accumulator bit-for-bit (I reproduced every assertion by hand) |
| peek8_pads_with_zeros_past_the_end (654) | YES | 0b1100_0000 exact |
| read_bits_spans_the_word_refill (657) | YES | 3×16-bit reads across boundaries |
| read_bits_zero_returns_zero (658) | YES | s=0 extend |
| oversized_read_consumes_the_tail_without_panicking (653) | YES | n=200, available 0 after |
| stuffing_survives_the_word_refill (662) | YES | stuffed FF + fill + marker in one stream |
| (cross-file) huffman lut_decode_matches_the_bit_walk_on_random_streams (703) | YES | random streams through LUT+word-refill vs the bit-at-a-time `next_bit` oracle — the strongest pin of ED-FIX-1 equivalence |
Mutation checks (reasoned): swapping `pos += 2` for `+= 1` in the stuffed case fails 661/662; breaking the restart m arithmetic fails 660/665; making peek consume fails 654; the n=32 mask bug is NOT covered (no test peeks >16) — consistent with F-001 being latent.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`0xFF 0x00` … is a stuffed literal `0xFF`" | bitreader.rs:5-6 | REAL | feed 91-97 + test 661 |
| "`0xFF D0..D7` is a restart marker … consumed at the MCU boundary via take_restart" | bitreader.rs:7-9 | REAL | feed 102-105, take_restart 219-230, decoder.rs:597-609 |
| "`0xFF 0xFF` before a marker is a fill byte: skipped" | bitreader.rs:11 | REAL | feed 98-101 (one byte at a time — consecutive fills handled) |
| "Past the end of the data, no bits are fabricated" | bitreader.rs:12-13 | REAL | read_bits None + huffman's available-check; peek8_padded's padding is look-ahead-only (never consumed as data) |
| "The observable state machine … is identical to the bit-at-a-time reader" | bitreader.rs:22-25 | REAL | huffman oracle test 703 + the 15 unit tests |
| "Look at the next `n ≤ 32` bits" (peek_bits contract) | bitreader.rs:131-132 | FALSE for debug builds at n=32 | F-001 |

## 7. Dead / duplicated / unreachable code
- `mask_bits` is used by `read_bits` but NOT by `peek_bits` (F-001) — near-duplicate mask logic, one correct, one not.
- `next_bit` is cfg(test) — documented as the oracle's reference walk; not dead (huffman's random-stream test consumes it).
- Nothing unreachable: every production method has a live caller (callers listed in the header).

## 8. Dependencies & flags
- No feature gates, no unsafe, no allocation (the reader is borrow-only — consistent with the D-1 zero-alloc decode story).
- Consumed by decoder.rs (new/take_restart/stop/read_bits) and huffman.rs (refill/available/peek8_padded/peek_bits/consume/consume_all) — both on the shipped MJPEG lane.

## 9. Open questions for the next agent
- huffman.rs owner (W06-A06): `peek_bits(length_u32).unwrap_or(0)` at huffman.rs:183 — the preceding `available < length_u32` guard makes the None arm unreachable; if that guard is ever relaxed, the `unwrap_or(0)` would decode padding as a real code (exactly what the module docs promise never happens). Consider `?`-propagating instead.
- decoder.rs part2 owner: the `Stop::Exhausted` → `reader.stop()` mapping (decoder.rs:644-649) — confirm `Marker` codes other than D9 (e.g. a stray D8/SOS inside entropy) still surface as PrematureMarker rather than being swallowed (the bitreader arms here make them Marker(other) — they do).
