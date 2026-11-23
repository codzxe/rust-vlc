# AUDIT: crates/codecs/src/video/mjpeg/parser.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| Lines / functions | 1049 / 17 (+19 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `parser::parse_into` ← `decoder.rs:546 MjpegDecoder::scan_into` ← codec-api `Decoder` impl ← registry (baseline engine-info: decoders include mjpeg; rc-smoke headless-decode MJPEG fixture PASS) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 mjpeg::parser` → 19 passed / 0 failed (exit 0); `./target/debug/deps/fuzz_survival-ba00cbfdf690b00c` → 6 passed (exit 0) |

## 1. One-paragraph verdict
This is a real, hostile-input-hardened baseline JPEG marker walker. Every array index is guarded by a prior segment-length check or `.get()`; lengths go through `next_segment` with checked arithmetic; DHT histograms get a genuine Annex-C oversubscription check; DQT dezigzag direction is correct and regression-tested with distinct per-position values. The honest-subset rejections (non-baseline SOF, sampling >2, non-interleaved scans, spectral selection) are typed errors, not silent accepts. I executed all 19 unit tests plus the 6-test committed-corpus fuzz-survival run (single-byte flips, truncations, adversarial geometries) — all pass. Only cosmetic/classification debt found.

## 2. What actually works (verified)
— verified by running prebuilt test binary `codecs-bae0f1756cc0a809 mjpeg::parser` → "19 passed; 0 failed" and `fuzz_survival-ba00cbfdf690b00c` → "6 passed; 0 failed" (systematic_single_byte_flips_of_a_real_frame_survive + systematic_truncations_of_a_real_frame_survive included). Binary (Nov 23 18:07) is newer than source (Nov 23 17:35), so not stale.

## 3. Findings
F-001 [S3]
Where: parser.rs:6-7
Code:
```
//! decoder consumes. Everything is bounds-checked; no panics on any
//! input (fuzz guarantee).
```
What is wrong: "fuzz guarantee" overstates the evidence. The libFuzzer campaign is unwired — `mjpeg/fuzz.rs:15-21` admits "Building it needs `libfuzzer-sys` ... until they land the harness logic is proven by the in-crate deterministic smoke tests here and by the committed-corpus survival run". The actual guarantee is: bounds-checked by reading (I verified every index), plus a fixed corpus + systematic flips/truncations. Why it matters: a reader/auditor may believe continuous fuzzing exists. What correct looks like: "no panics on any input (verified by committed-corpus survival run; libFuzzer campaign unwired)". Confidence: HIGH — Verification: read.
F-002 [S4]
Where: parser.rs:274-284
Code:
```
            0xc1 | 0xc2 | 0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf => {
                return Err(unsupported(format!(
                    "non-baseline SOF marker FF{code:02x} \
```
What is wrong: DAC (0xCC, arithmetic-coding conditioning) is not in this arm; it falls to the `_` catch-all and is reported as "unknown marker FFcc". Arithmetic JPEG is rejected either way, but the diagnosis is wrong (unknown vs arithmetic). Cosmetic classification defect only. Confidence: HIGH — Verification: read (match arms enumerated: 0xcc absent).
F-003 [S4]
Where: parser.rs:380-399
Code:
```
    for i in 0..count {
        let base = payload + 6 + 3 * i;
        let id = data[base];
```
What is wrong: SOF0 does not reject duplicate component identifiers. With `components = [id=1, id=1]`, `parse_sos`'s `position(|c| c.id == cs)` maps both scan components to sof_index 0, and the stream dies later as `mjpeg.sos.duplicate_component` — a misattributed error for a SOF-level defect. No wrong decode possible (it errors), but the error site/message misleads. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: parser.rs:506-517
Code:
```
        if p + 17 + total > end {
            return Err(bad_marker("mjpeg.dht.symbol_truncation", p));
        }
```
What is wrong: a DHT with BITS all zero (total=0) is accepted and `huff_present` set true; `parse_sos` then accepts a scan selecting that table, and the failure surfaces only at decode time as a Huffman `Stop::InvalidCode` → concealment (a concealed frame, not a typed structural error). A zero-symbol table is structurally invalid JPEG. Blast radius: one wasted concealment path per malformed stream. Confidence: HIGH — Verification: read (huffman.rs:1-5 documents Stop::InvalidCode as concealable).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ZIGZAG const | 16-25 | REAL | — | byte-identical to ISO 10918-1 Fig A.6 zigzag (checked element-by-element) |
| ComponentInfo | 28-38 | REAL | — | plain data |
| JpegInfo | 41-72 | REAL | — | plain data |
| Parsed (cfg(test)) | 77-80 | REAL | — | test-only wrapper, correctly gated |
| JpegInfo::empty | 87-106 | REAL | — | no-alloc claim: all Vec::new(), correct |
| JpegInfo::reset | 114-132 | REAL | — | clears contents, keeps capacities; leak-tested (test L686) |
| u16_at | 136-140 | REAL | — | `.get()` + checked_add, no panic |
| marker mod | 144-157 | REAL | — | constants correct (SOI d8, EOI d9, SOS da, DQT db, DHT c4, DRI dd, SOF0 c0, APPn e0-ef, COM fe, TEM 01) |
| parse (cfg(test)) | 163-167 | REAL | — | test convenience only; production uses parse_into |
| parse_into | 188-287 | REAL | — | marker walk; fill-byte skip, duplicate SOI/SOF, SOS-before-SOF, EOI-before-SOS all typed; TEM +2; no panic path found (all indexes guarded) |
| next_segment | 291-313 | REAL | — | len<2 rejected, checked_add overflow guard, end>len → Truncated |
| is_jfif | 316-319 | REAL | — | reads ≤ data.len(); false positive impossible (byte after any segment is 0xFF ≠ 'J') |
| parse_avi1 | 330-350 | REAL | — | polarity byte proven inside declared segment before read (L340-343); 0/1/2 vocabulary enforced; damage → typed error |
| parse_sof0 | 353-415 | REAL | — | precision==8, zero-dim rejected, count 1..=4, sampling 1..=4 then honest 1..=2 subset; F-003 duplicate IDs |
| parse_dqt | 419-461 | REAL | — | Pq=1 rejected, Tq≤3, 65-byte table bounds, zero-value rejected (out of spec), dezigzag direction correct + regression-tested |
| parse_dht | 464-524 | REAL | — | class/table-id range, total≤256, genuine Annex-C oversubscription check (L492-505 verified arithmetic: last = code+count-1 ≥ 2^len), truncation guard; F-004 total=0 |
| parse_dri | 527-534 | REAL | — | exact length 4 enforced, unwrap_or(0) unreachable (bounds pre-proven) |
| ScanComponent | 538-546 | REAL | — | plain data |
| parse_sos | 553-621 | REAL | — | ns==components.len(), per-component ID resolution + duplicate check, td/ta≤3, table presence checks, Ss/Se/AhAl==0/63/0 enforced |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parse_into_reuses_the_scratch... (686) | YES | asserts no state leak across a failed walk + capacity retention + field-equality with fresh parse |
| parses_the_minimal_grayscale_stream (761) | YES | asserts width/height/components/tables/entropy_offset concretely |
| dqt_dezigzag... (936) | YES | distinct per-position values — the exact mutation that hid the salvage's permutation bug |
| oversubscribed_huffman_counts (901) | YES | 200 codes of length 1; asserts typed error code |
| avi1_polarity_* (967/995/1010) | YES | all 3 vocab values + out-of-vocab + missing-byte segment each assert distinct typed codes |
| remaining structural tests (806-933) | YES | each asserts a specific MjpegError code, not just is_err |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Everything is bounds-checked; no panics on any input (fuzz guarantee)" | parser.rs:6-7 | REAL for the bounds claim (read-verified; corpus+flips run passed), inflated for "fuzz guarantee" | F-001; mjpeg/fuzz.rs:15-21 admits campaign unwired |
| "in steady state this walk allocates nothing" | parser.rs:170-172 | REAL-UNPROVEN | clear+reserve+extend_from_slice pattern supports it; capacity assertions in test 686; not measured with an allocator probe |

## 7. Dead / duplicated / unreachable code
`parse`/`Parsed` are `#[cfg(test)]` — intentional, not dead in release. No duplication found. ZIGZAG shared with encoder via tools/xtask fixtures (doc claim, not independently diffed — UNVERIFIED, low value).

## 8. Dependencies & flags
No feature gates on this file (plain module). Depends only on `super::FieldPolarity`, `super::error`. No unsafe.

## 9. Open questions for the next agent
- decoder.rs `MAX_PIXELS` ceiling: verify the parser's width/height (≤65535 each) can't bypass the pixel-budget check before allocation (decoder.rs audit owner).
