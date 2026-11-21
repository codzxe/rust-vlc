# AUDIT: crates/codecs/src/video/mjpeg/parser.rs#part2
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-11-21 |
| Lines / functions | 527–1049 of 1050 audited (chunk part2 of 2; part1 ≈1–525 — `parse_into`/`next_segment`/`is_jfif`/`parse_avi1`/`parse_sof0`/`parse_dqt`/`parse_dht` + `JpegInfo` — belongs to another agent) / 3 production items + 10 test helpers + 19 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open`/`lumen decode-bench` → apps/cli/src/pipeline_host.rs:242-245 `MjpegDecoder::new` → mjpeg/mod.rs:362 `context.decode_scan` → decoder.rs:546 `parser::parse_into` → parser.rs:235 `parse_sos` (my range, called for the SOS marker) and parser.rs:248 `parse_dri` (DRI marker). EXECUTED: audit/runs/baseline/mp4-smoke-bench-avi-mjpeg-pcm.txt `v_frames=60 v_decoded=60`; all 19 parser tests ok, baseline `test-workspace.log.txt` lines 721-739) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `Read` parser.rs 1–1050 (full file for context; audited 527–1049); `grep -n "video::mjpeg::parser" audit/runs/baseline/test-workspace.log.txt` (721-739: 19/19 ok); `rg -n "parse_sos\|parse_dri" crates/codecs/src/video/mjpeg` (sole caller: parse_into at 235/248); `rg -n "sof_index\|td\|ta" decoder.rs` (the consumer indexes this range's outputs — decoder.rs:611-624); `rg -n "restart_boundaries\|take_restart" decoder.rs bitreader.rs test-workspace.log.txt` (DRI exercised end-to-end by decoder test 691); `sed -n` on quoted ranges. |

## 1. One-paragraph verdict
Part2 is the DRI/SOS tail of the MJPEG marker walk plus the parser's entire test module. The production code (77 lines) is tight, bounds-safe, and spec-shaped: `parse_dri` enforces the exact 4-byte segment length, and `parse_sos` validates every field that the decoder later uses as a raw index (`sof_index` via component-ID position lookup, `td`/`ta` ≤ 3, table presence) — I traced each validation to the exact decoder line it protects (decoder.rs:611-624), and the truncation arithmetic (`end ≥ payload+4+2·Ns`) is proven against every byte the loop reads. The 19 tests all ran green in the baseline and are genuinely content-sensitive — the scratch-reuse differential (686-758) asserts non-leakage across a failed walk *and* byte-equality with a fresh parse, and the DQT dezigzag regression documents how a uniform-value fixture once hid a real permutation bug. The one real gap: the SOS damage gates that carry the decoder's panic-safety invariant (notably `td > 3 || ta > 3`, whose removal would turn decoder.rs:612-622 into an out-of-bounds panic) and `parse_dri` itself have zero direct tests — no test in the repo fails if those checks are deleted.

## 2. What actually works (verified)
— Shipped-binary execution: `mp4-smoke-bench-avi-mjpeg-pcm.txt` → `v_frames=60 v_decoded=60`; every one of those 60 frames walked through this range's `parse_sos` (the AVI fixture's JPEGs carry SOS headers).
— All 19 tests executed ok (baseline 721-739), zero ignored.
— Bounds safety of `parse_sos` proven by hand: `next_segment` (parser.rs:291-313) returns `end ≤ data.len()` with overflow-checked arithmetic; the `end < payload + 1 + 2*ns + 3` gate (567-569) makes every subsequent index (`data[payload+1+2i]`, `data[payload+2+2i]`, `data[payload+1+2ns]`..`data[payload+3+2ns]`) provably < end.
— Panic-safety chain verified end-to-end: `sof_index` is a `position()` lookup (575-579, never out of range), `td`/`ta` rejected above 3 (594-596), Huffman and quant table presence required (597-608) — these are exactly the invariants `decoder.rs:611-624` (`self.dc_tables[scan_comp.td as usize]`, `info.quant_tables[comp.quant_table as usize]`, direct indexing) relies on.
— `parse_dri` length gate: `pos + 6 != end` (529-531) is the exact `FF DD 00 04 Ri_hi Ri_lo` shape; restart semantics (Ri=0 → none) consumed correctly by decoder.rs:597-609; exercised end-to-end by `restart_boundaries_crossing_mcu_rows_decode_bit_exact` (baseline 691).
— Baseline spectral gate: Ss=0/Se=63/AhAl=0 enforced (611-619), the correct baseline restriction.

## 3. Findings

### F-001 [S3] The load-bearing SOS validation gates and all of `parse_dri` are unpinned by any test
Where: crates/codecs/src/video/mjpeg/parser.rs:594-608 (and 527-534)
Code:
```
        let td = tables >> 4;
        let ta = tables & 0x0f;
        if td > 3 || ta > 3 {
            return Err(bad_marker("mjpeg.sos.table_selector", payload + 2 + 2 * i));
        }
        if !info.huff_present[0][td as usize] || !info.huff_present[1][ta as usize] {
```
What is wrong: The tests in this module (624-1049) pin exactly two SOS paths: the happy path (`parses_the_minimal_grayscale_stream`, 761-785) and the non-interleaved gate (879-898). No test anywhere in the repo asserts the six other typed refusals of `parse_sos` — `sos.unknown_component` (575-579), `sos.duplicate_component` (581-590), `sos.table_selector` (594-596), `sos.missing_huffman_table` (597-602), `sos.missing_quant_table` (603-608), `sos.truncated` (567-569), the partial/spectral gate (611-619) — and `parse_dri`'s length gate (529-531) likewise has no unit test (the decoder's restart test builds a well-formed DRI, so it cannot fail on a broken length check).
Why it matters: The `td/ta ≤ 3` check is not cosmetic — it is the ONLY thing standing between a crafted SOS and an out-of-bounds panic at `decoder.rs:612-622` (`self.dc_tables[scan_comp.td as usize]` on a `[Option<_>; 4]`). Deleting or weakening it fails zero tests today; the in-crate fuzz tests are four fixed inputs, and the registered cargo-fuzz targets have never been executed (baseline ran only `fuzz_registry_check`, gate #18 — registry diff, not fuzzing). This is the "test proving nothing used as evidence" pattern waiting to happen: the suite is green while the panic-safety story rests on an unpinned branch.
What correct looks like: One table-driven test per typed refusal (a 5-line helper mutated from the existing `sos()` builder — the test infrastructure at 680-683 already makes this trivial), plus a `dri` bad-length case.
Confidence: HIGH — Verification: read of the full test module + `rg` for the error codes across tests (`rg "sos.table_selector\|sos.unknown_component\|sos.duplicate\|dri.length" crates/codecs` → no test hits).

### F-002 [S4] Dead defensive `unwrap_or(0)` in `parse_dri`
Where: crates/codecs/src/video/mjpeg/parser.rs:532
Code:
```
    info.restart_interval = u32::from(u16_at(data, pos + 4).unwrap_or(0));
```
What is wrong: The preceding `pos + 6 != end` gate (529) plus `next_segment`'s `end ≤ data.len()` (306-311) prove `data[pos+4]` and `data[pos+5]` exist, so `u16_at` cannot return `None` here. The `unwrap_or(0)` silently converts a would-be-impossible truncation into `restart_interval = 0` (restart markers disabled) instead of a typed error — a silent-fallback guard of the same class the mpeg12 headers audit flagged.
Why it matters: If a future edit relaxes the length gate, a truncated DRI would silently disable restart-interval handling instead of failing loudly; today it is dead code obscuring the invariant.
What correct looks like: `u16_at(data, pos + 4).ok_or_else(|| bad_marker("mjpeg.dri.truncated", pos))?` — or a comment stating why the None arm is unreachable.
Confidence: HIGH — Verification: read; invariant traced through 529-531 and 291-313.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_dri` | 527-534 | REAL | S4 | exact 4-byte length gate ✓; dead `unwrap_or(0)` → F-002; no direct test (F-001); Ri=0 semantics correct |
| `ScanComponent` struct | 536-546 | REAL | — | plain data; doc comments accurate |
| `parse_sos` | 548-621 | REAL | S3 | Ns==Nf gate ✓ (560-566, honest subset, pinned by test 879); bounds arithmetic proven safe; all 7 typed refusals correct; td/ta/huff/quant presence = decoder panic-safety → unpinned (F-001); spectral gate 611-619 ✓ |
| test helper `soi`/`seg` | 629-638 | REAL | — | correct segment framing (len = payload+2, BE) |
| test helper `dqt`/`dht_dc`/`dht_ac`/`dht` | 640-669 | REAL | — | minimal valid tables; single-symbol 1-bit-code table matches huffman.rs's pinned case |
| test helper `sof0`/`sos` | 671-683 | REAL | — | note `sof0` writes h-then-w (673-674) matching SOF0's height-first layout — correct per §B.2.2 |
| `parse_into_reuses_the_scratch_without_leaking_across_walks` | 686-758 | REAL | — | differential A/B + failed walk + fresh-parse equality on 11 fields + capacity assertions — would fail on any reset omission |
| `parses_the_minimal_grayscale_stream` | 761-785 | REAL | — | pins entropy_offset, scan[0] fields, table flags |
| `jfif_app0_is_detected` | 788-803 | REAL | — | pins part1's `is_jfif` |
| `missing_soi_is_structural` | 806-809 | REAL | — | pins part1 gate |
| `short_input_is_truncation` | 812-821 | REAL | — | pins 189-194 |
| `zero_dimensions_are_structural` | 824-833 | REAL | — | pins part1 `parse_sof0` gate 367-369 |
| `progressive_sof2_is_unsupported_not_corrupt` | 836-846 | REAL | — | pins FFC2 refusal; honest Unsupported-not-BadMarker taxonomy |
| `segment_overrun_is_truncation` | 849-855 | REAL | — | pins next_segment end>len check |
| `sampling_factor_three_is_unsupported` | 858-866 | REAL | — | pins 403-410 subset gate |
| `sos_before_sof_is_structural` | 869-876 | REAL | — | pins part1 ordering gate |
| `non_interleaved_scan_is_unsupported` | 879-898 | REAL | — | pins MY range's ns gate 560-566 |
| `oversubscribed_huffman_counts_are_structural` | 901-912 | REAL | — | pins part1 `parse_dht` |
| `eoi_before_sos_is_structural` | 915-922 | REAL | — | pins part1 |
| `two_byte_quant_table_ids_reject_the_upper_half` | 925-933 | REAL | — | pins part1 `parse_dqt` Tq gate |
| `dqt_dezigzag_maps_scan_order_to_natural_positions` | 936-964 | REAL | — | content-sensitive regression with distinct values; documents the uniform-fixture blind spot honestly |
| `avi1_polarity_parses_for_all_three_values` | 967-992 | REAL | — | pins part1 `parse_avi1` happy paths 0/1/2 |
| `avi1_polarity_out_of_vocabulary_is_structural` | 995-1007 | REAL | — | pins polarity gate |
| `avi1_segment_without_the_polarity_byte_is_structural` | 1010-1021 | REAL | — | pins the segment-boundary read (never read the next marker's byte) |
| `avi1_field_polarity_resets_between_walks` | 1024-1048 | REAL | — | pins reset() covering field_polarity |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| parse_into_reuses_the_scratch… (731) | YES | field-by-field equality vs fresh parse after a poisoned failed walk |
| parses_the_minimal_grayscale_stream (732) | YES | exact entropy_offset + selector values |
| non_interleaved_scan_is_unsupported (729) | YES | asserts the detail string |
| dqt_dezigzag… (725) | YES | distinct per-position values; any permutation error shifts the table |
| avi1 tests (721-724) | YES | polarity values and refusal codes asserted |
| All gate tests (726-728, 730, 733-739) | YES | assert specific error codes, not just `is_err()` |
| — coverage gap — | NO test pins: sos.table_selector, sos.unknown_component, sos.duplicate_component, sos.missing_huffman_table, sos.missing_quant_table, sos.truncated, spectral gate, dri.length | → F-001; mutation check (reasoned): deleting the `td > 3` check fails no test, yet creates an OOB panic path at decoder.rs:612-622 reachable by a crafted SOS |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Baseline subset: exactly one interleaved scan covering every frame component" | parser.rs:548-552 | REAL | enforced at 560-566; honest subset — legal-but-unsupported single-component scans of multi-component frames refuse typed (test 729) |
| "A component may appear only once in a scan (B.2.3)" | parser.rs:580 | REAL | 581-590 |
| "full spectral range, successive-approximation bits zero" required | parser.rs:549-550 | REAL | 611-619 |
| module docs: "On error the scratch may hold a partially walked state — the next call resets it first" | parser.rs:186-187 | REAL | reset at parse_into:198 (after SOI check); pinned by tests 731/724; NOTE the SOI-failure path returns before reset — harmless (headers restored by decoder.rs:501-503, next successful walk resets) |
| decoder.rs:46-47 "the parser proves the single interleaved scan covers every frame component" | decoder.rs (part1 doc) | REAL | parser.rs:560, this range |

## 7. Dead / duplicated / unreachable code
- `unwrap_or(0)` at 532 — dead branch (F-002).
- Nothing else: every production item here is on the shipped decode path.

## 8. Dependencies & flags
- No feature gates (mjpeg unconditional in codecs).
- Uses: `u16_at`/`next_segment`/`bad_marker`/`unsupported` (part1 + error.rs), `JpegInfo` (part1), `FieldPolarity` (field.rs). Outputs consumed by decoder.rs:611-624 (part1) — the tight parser↔decoder index contract is the reason F-001 matters.
- testkit::Must not used here (tests use unreachable!-style unwraps — acceptable in #[cfg(test)]).

## 9. Open questions for the next agent
- part1 owner: does `parse_sof0` reject duplicate component IDs (T.81: Ci "shall be unique")? I verified downstream safety (dup IDs end in unknown_component/duplicate_component refusals), but the SOF-level check is absent from what I read (380-399).
- huffman.rs owner: the single-symbol 1-bit-code table used by the test helpers (`dht_dc`) — `single_symbol_table_uses_a_one_bit_code` (baseline 706) pins it; confirm code '0' vs '1' canonical assignment matches T.81 F.1.2.3 (a 1-symbol table gets code '0', length 1).
