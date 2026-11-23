# AUDIT: crates/codecs/src/video/mjpeg/bitreader.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 425 (245 source + 180 tests) / 11 non-test fns + 15 tests |
| File verdict | REAL (byte-unstuffing state machine and accumulator verified by trace + baseline 15/15) |
| Reachable from shipped entry point? | YES — pipeline_host "mjpeg" → decode_image → scan loop `decode_block` → huffman.rs:145/149/183 (`refill`/`peek8_padded`/`peek_bits`/`consume`) and `read_bits`; mjpeg is a shipped decoder (rc-smoke engine-info) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read of full file + manual state-machine traces (stuffing, fill, restart, truncation); `rg peek_bits|read_bits\(` caller map; sed+od quote verification; baseline citation audit/runs/baseline/cargo-test-workspace.txt:748-762 |

## 1. One-paragraph verdict
Correct, hostile-input-safe entropy bit reader. I hand-traced the §F.1.2.3 state machine on every input class: literal bytes feed MSB-first; `FF 00` unstuffing yields a literal `FF` with pos+2; `FF FF` fill bytes are skipped one at a time (so `FF FF D0` stops at Restart(0) with pos landing exactly on the final FF, and `take_restart`'s pos+=2 consumes precisely the marker pair); `FF Dm` records Restart(m) without feeding; any other `FF xx` is a Marker stop; a truncated stuffing pair is EndOfData. The accumulator invariant holds (refill only feeds while nbits ≤ 24, so nbits ≤ 32 and no shift drops valid bits; `peek8_padded`'s left-shift provably zero-pads regardless of stale accumulator bits). No fabricated bits past the end — partial reads consume the tail and return None. One latent defect: `peek_bits`'s mask expression evaluates `(1u32 << n) - 1` before its own n==32 fix-up, so n=32 panics in debug builds — unreachable today (huffman.rs only peeks 9..=16) but the API contract (`n ≤ 32`, debug_assert permits it) is a lie for 32.

## 2. What actually works (verified)
— Manual traces of feed/take_restart on `[0xff,0x00,...]`, `[0xff,0xff,0xd0]`, `[0x42,0xff]` (truncated pair), `[0xff,0xd3]` (wrong restart number preserved) — all match the test expectations and the spec behavior.
— Accumulator arithmetic re-derived: refill bound nbits≤24→32 total; peek window `(acc >> (nbits-n)) & mask` correct for all n ≤ nbits; peek8_padded `(acc << (8-nbits)) & 0xff` zero-pads by construction.
— Baseline: `bitreader::tests` 15/15 ok (audit/runs/baseline/cargo-test-workspace.txt:748-762), including the oversized-read (n=200) no-panic case and the word-refill crossing cases.

## 3. Findings
F-001 [S4]
Where: bitreader.rs:140-143
Code: (verbatim)
```
        Some(
            (self.acc >> (self.nbits - n))
                & ((1u32 << n) - 1).max(if n == 32 { u32::MAX } else { 0 }),
        )
```
What is wrong: For n == 32 the subexpression `(1u32 << 32) - 1` is evaluated BEFORE the `.max(...)` fix-up: in debug builds `1u32 << 32` panics ("attempt to shift left with overflow"); in release the shift amount is masked to 0 giving `1-1 = 0`, then `.max(u32::MAX)` yields the correct mask — correct by accident in release, a panic in debug. The doc contract says "next `n ≤ 32` bits" and `debug_assert!(n <= 32)` explicitly permits 32. The correct helper already exists in this file — `mask_bits` (lines 235-243), which `read_bits` uses — but `peek_bits` re-derives the mask badly. Currently unreachable: the only production caller (huffman.rs:183) passes lengths 9..=16. What correct looks like: `& mask_bits(n)`.
Confidence: HIGH — Verification: read (shift semantics; no caller with n=32 executed).
F-002 (resolved as non-issue, recorded for completeness): `read_bits` with hostile n (e.g., 200) cannot shift-overflow because `mask_bits` handles n ≥ 32 without shifting — verified by reading `mask_bits` and by the passing oversized-read test.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| BitReader::new | 58-66 | REAL | — | trivial |
| feed | 70-112 | REAL | — | full F.1.2.3 state machine, traced by hand; idempotent after stop |
| refill | 117-123 | REAL | — | nbits ≤ 24 guard keeps shifts lossless |
| available | 127-129 | REAL | — | trivial |
| peek_bits | 134-144 | REAL | S4 | F-001 mask landmine for n=32 (debug panic) |
| peek8_padded | 151-157 | REAL | — | zero-padding proven by shift algebra |
| consume / consume_all | 162-172 | REAL | — | debug_assert guards; truncation discards partials |
| next_bit (cfg(test)) | 179-188 | REAL | — | bit-at-a-time equivalence oracle |
| read_bits | 195-204 | REAL | — | partial→None+tail consumed; any n accepted w/o panic |
| stop | 207-209 | REAL | — | accessor |
| take_restart | 219-230 | REAL | — | sequence check, padding discard, exact 2-byte consume, state preserved on Err |
| mask_bits | 235-243 | REAL | — | correct for 0/32/general n |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 15 tests | YES | exact bit sequences/stops asserted; my independent traces agree with every expectation (e.g., fill-byte case: stop Restart(0) then EndOfData after take) |
| GAP: no test calls `peek_bits(32)` | — | F-001 would be caught by exactly that test |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The observable state machine ... is identical to the bit-at-a-time reader" | bitreader.rs:22-25 | REAL | both paths tested; next_bit oracle shares feed() |
| "Past the end of the data, no bits are fabricated" | bitreader.rs:12-13 | REAL | read_bits partial → None + consume_all; peek8_padded padding is gated by the huffman layer's `available` checks (huffman.rs:154-167) |
| "Look at the next `n ≤ 32` bits" | bitreader.rs:131-132 | SHALLOW | contract broken at exactly n=32 in debug (F-001) |

## 7. Dead / duplicated / unreachable code
`next_bit` is cfg(test)-only (documented as the oracle) — legitimate. No duplication (h264 and mpeg12 have their own readers — separate modules, not checked here for duplication depth).

## 8. Dependencies & flags
No deps, no features. Consumers: mjpeg/huffman.rs (decode), decoder.rs (restart handling), tests.

## 9. Open questions for the next agent
mpeg12/bitreader.rs is a sibling reader — worth checking whether it shares the same peek-mask pattern (rg showed `mpeg12::bitreader::tests` exists at baseline:839).
