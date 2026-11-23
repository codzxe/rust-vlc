# AUDIT: crates/demux/src/adts/tests.rs
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-23 |
| Lines / functions | 643 / 29 tests + 4 helpers (Drain, drain, open, Bits, adif_raw) |
| File verdict | REAL (honest, arithmetic-tight unit suite; 2 coverage gaps) |
| Reachable from shipped entry point? | Test-only (`#[cfg(test)]` module) — exercised by `cargo test -p demux`; the code under test (adts/mod.rs) is reachable via lumen-cli `containers/mod.rs` "adts" arm |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/demux-5e0c7194d741938b --test-threads=2 adts` -> 32 passed, 0 failed (exit 0); hand-recomputed every pinned constant below |

## 1. One-paragraph verdict
A genuinely hostile-adjacent unit suite: every test asserts exact arithmetic (pts sequences, per-packet durations, byte-ramp payloads, sticky error replay, pool ceilings), the error paths are tested one typed code at a time, and I re-derived every pinned constant by hand — all correct (5120 samples @ 44.1 kHz = 116,099,773 ns; 7168 @ 48 kHz = 149,333,333 ns; pts [0,1024,2048,4096] for blocks [1,1,2,1]). All 29 tests run green in my execution. Two coverage gaps: the MPEG-2 (ID=1) ADTS lane is never constructed or asserted (the builder hardwires ID=0, and only `!head.mpeg2` is ever checked), and the committed-fixture "fuzz-lite" test asserts only existence + no-panic, not the typed outcomes its name advertises.

## 2. What actually works (verified)
- `./target/debug/deps/demux-5e0c7194d741938b --test-threads=2 adts` -> `test result: ok. 32 passed; 0 failed` (this file's 29 + 3 es/adts-lane tests; binary newer than source).
- Constants recomputed: 116_099_773 = floor(5120×1e9/44100) ✓ (141-145); 149_333_333 = floor(7×1024×1e9/48000) ✓ (387-390); ADTS payload ramp `(f*31+i)%251` parity between the ADTS and ADIF lanes (394-426) is a real cross-format differential ✓.
- The fixture dir `crates/demux/fixtures/adts/` really carries all four .bin files the test reads (ls verified: 0059-flip-idx15.bin, 0059-trunc-4k.bin, 0059-zero-range.bin, 0110-trunc-8k.bin).

## 3. Findings

F-001 [S4]
Where: crates/demux/src/adts/tests.rs:55-57 (and test_support.rs:60-63 builder)
Code:
```rust
    let frame = adts_frame(1, 4, 2, 1, 100, None);
    let head = AdtsHeader::parse(&frame).must();
    assert!(!head.mpeg2);
```
What is wrong: `adts_frame` hardwires `ID=0 (MPEG-4)` (test_support.rs:61-63: `head[1] = 0xF0 // syncword[3:0] | ID=0 (MPEG-4) | layer=0`), and no test anywhere in the suite constructs an MPEG-2 (ID=1) frame or asserts `head.mpeg2 == true`. The demuxer's MPEG-2 header handling (a real-world lane — MPEG-2/LC ADTS is what some broadcast/sat streams carry) is executed by zero tests; if the mpeg2 branch of `AdtsHeader::parse` swapped a field or mis-sized the header, this suite stays green. Also `rate_table_matches_the_spec` (93-98) pins only 4 of the 13 table entries. What correct looks like: one builder parameter for the ID bit + one round-trip assertion. Confidence: HIGH — Verification: read (rg for mpeg2 across the crate's tests: only negations).
Confidence: HIGH — Verification: read

F-002 [S4]
Where: crates/demux/src/adts/tests.rs:629-643
Code:
```rust
fn committed_mutation_fixtures_are_typed_or_recover_never_panics() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/adts");
    for name in [
        "0059-trunc-4k.bin",
        "0110-trunc-8k.bin",
```
What is wrong: the name promises "typed or recover"; the body asserts only that each fixture is non-empty and that `fuzz_demux_adts` does not panic. No typed-outcome assertion on the committed bytes (which outcome each mutation fixture produces is unchecked). The synthetic tests above do pin typed outcomes, so the demuxer behavior is covered — but this test, specifically, would survive any behavior regression on these exact files as long as nothing panics. Name overpromises. Confidence: HIGH — Verification: read + ran (green).
Confidence: HIGH — Verification: read

## 4. Per-function table
| Test / helper | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Drain / drain / open | 16-49 | REAL | — | collects packets/recovered/discontinuities/terminal code |
| header_parse_roundtrips_the_field_set | 53-81 | REAL | S4 | F-001: MPEG-4 only, mpeg2 never true |
| probe_detects_adts_adif_and_rejects_the_rest | 83-90 | REAL | — | syncword + ADIF + negatives |
| rate_table_matches_the_spec | 92-98 | REAL | S4 | 4/13 entries pinned (F-001 note) |
| clean_walk_stamps_frame_arithmetic | 102-146 | REAL | — | exact pts/duration/base/duration-total — all recomputed ✓ |
| id3_head_and_interframe_tags_are_skipped | 148-160 | REAL | — | head + interframe tags, byte-exact skip counts |
| pool_memory_stays_bounded | 162-175 | REAL | — | 64 frames vs 2-slot pool ceiling |
| lost_sync_mid_stream_resyncs_with_recovered_events | 179-206 | REAL | — | resync + hole exposed (pts 1024 not smoothed) ✓ |
| truncated_frame_is_the_typed_truncation | 208-225 | REAL | — | terminal code + sticky replay asserted |
| truncated_first_frame_..._at_start | 227-234 | REAL | — | start() error code |
| sampling_index_fifteen_is_typed | 236-242 | REAL | — | 0xF code |
| channel_config_zero_is_typed | 244-250 | REAL | — | code |
| crc_mismatch_is_typed_and_good_crc_walks | 252-265 | REAL | — | forged CRC-16 wrong/right (header_crc exported) |
| frame_shorter_than_its_header_is_typed_bad_header | 267-278 | REAL | — | flen forced to 6 |
| garbage_head_without_sync_is_the_typed_not_adts | 280-291 | REAL | — | incl. ID3-tagged garbage |
| nonzero_layer_head_is_not_adts | 293-302 | REAL | — | layer != 0 rejected |
| trailing_fragment_and_lost_sync_at_end_are_clean | 304-323 | REAL | — | both end shapes, skip counts exact |
| not_started_is_typed | 325-336 | REAL | — | next_event + reanchor codes |
| mid_stream_parameter_change_is_a_discontinuity | 338-353 | REAL | — | channel change exposed, clock arithmetic pinned |
| adif_cbr_walk_matches_the_block_arithmetic | 357-391 | REAL | — | header size 14, per-block ramp bytes, duration ✓ |
| adts_and_adif_agree_on_the_same_payload | 393-426 | REAL | — | real cross-container payload differential |
| Bits / adif_raw | 428-481 | REAL | — | hand bit-writer for malformed ADIF |
| adif_vbr / multi_program / bad_params / no_data | 483-516 | REAL | — | each typed code |
| adif_partial_final_block_is_marked | 518-531 | REAL | — | 60-byte remainder + discontinuity |
| adif_seek_is_format_inherent_arithmetic | 533-564 | REAL | — | Fast{indexed:false}, exact landing, reanchor splice |
| adts_seek_roundtrip_lands_at_or_before_the_target | 568-605 | REAL | — | SlowLinear, at-or-before, remaining-count 16-5-1 |
| adts_seek_past_the_end_lands_at_the_last_frame | 607-625 | REAL | — | clamp-to-only-frame |
| committed_mutation_fixtures_... | 629-643 | REAL | S4 | F-002: existence+no-panic only |

## 5. Test honesty
Covered in the table. Mutation reasoning: nearly every test pins exact values that mutate-break (e.g., if the clock counted declared-not-emitted samples, `lost_sync`'s `t.ticks == 1024` fails; if truncation were non-sticky, the replay assert at 220-224 fails). The two S4s are gaps, not fakes.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every typed error path" covered | tests.rs:1-3 | NEARLY REAL | every code asserted (rg `demux.adts.`: truncated, bad-sampling-index, bad-channel-config, crc-mismatch, bad-header, not-adts, not-started, adif-vbr, adif-multi-program, adif-bad-params, adif-no-data) — MPEG-2 lane untested (F-001) |
| "+FB fuzz-lite derivatives" | tests.rs:2-3 | REAL (weak) | fixtures exist, harness runs (F-002) |

## 7. Dead / duplicated / unreachable code
None. No #[ignore], no tautologies. Helpers are all used.

## 8. Dependencies & flags
`testkit::Must` (panicking unwraps for tests), `super::test_support` (builders incl. exported `header_crc`), `media_types`, `plugin_api`. No features. cfg(test) — not in the release binary.

## 9. Open questions for the next agent
- The 4 committed fixtures are derivatives of corpus lanes 0059/0110 (adts_corpus.rs is another agent's file) — their generating provenance is asserted nowhere in-tree.
- adts/mod.rs itself is not my file: F-001's risk depends on how `AdtsHeader::parse` branches on mpeg2 — the next agent should confirm the MPEG-2 header-length/field layout in mod.rs matches ISO 13818-7.
