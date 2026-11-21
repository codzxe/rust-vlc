# AUDIT: crates/codecs/src/audio/pcm/decoder.rs#part3 (lines 1070-1572)
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-21 |
| Lines / functions | 503 (of 1572; chunk part3 of 3 — nominal thirds 1-524/525-1048/1049-1572; the boundary test `waveformatextensible_resolves_guids_and_decodes` (1009-1068) straddles the 1049 line, so I take my range from the clean test boundary 1070 and double-cover 1009-1069 for continuity; production code 1-787 belongs to part1(W07-A05)/part2(W07-A03)) / 17 test functions (+1 boundary-covered test, all in `mod tests`) |
| File verdict | REAL (the file's test half: executed 22/22 green in baseline, plus counting-allocator zero-alloc proof and fixture/attack/fuzz integration suites) |
| Reachable from shipped entry point? | YES — this is the shipped first-vertical audio decoder: `apps/cli/src/pipeline_host.rs:357-370` `codec if codec.starts_with("pcm_")` → `codecs::audio::pcm::PcmDecoder::with_format(channels, rate)` from demux facts. Executed: baseline INDEX.md AVI first-vertical `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60` (MJPEG+PCM lane); `test-workspace.log.txt:459-480` all 22 `audio::pcm::decoder::tests` ok; `:1247-1250` `pcm_contract_path_is_zero_allocation_per_buffer_after_warm_up ... ok`. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read 1-799 (production context: PcmPlan/parse_waveformatex/PcmDecoder/Decoder impl), 799-1572 (test module, my range in full); rg function map; rg `stale|drop_stale` across crates/codecs (coverage hunt) → only adpcm/contract.rs(mjpeg)/codec-api hits; Read codec-api/src/decode.rs:203-227 (drop_stale_packet + its own unit tests); Read tests/pcm_zero_alloc.rs:1-120 + tail (counting-allocator proof); rg `decode_pcm` in pcm/mod.rs:281-289 (reference-shape check); baseline log greps (:459-488, :1200-1250); sed verification of every quoted range. |

## 1. One-paragraph verdict
This is the strongest test half I have audited in the audio tree: hand-computed duration floors (33,333,333 ns pinned arithmetically), pointer-identity zero-copy proofs, a randomized bit-exactness differential between the hot-path group-swap and its reference twin across every width and adversarial length, a 64-packet stale-slot-content trap with content varying per packet, and an end-to-end counting-allocator zero-allocation proof in the integration suite that covers BOTH the zero-copy and the byte-swapped conversion shapes. The production code these tests pin (part1/part2 territory) is honest about its damage model, and every assertion I checked computes to the expected value by hand. The single defect is a coverage gap, not a bug: `PcmDecoder::decode`'s stale-generation drop branch (line 717) is executed by no test anywhere — the adpcm sibling pins the same seam, PCM does not.

## 2. What actually works (verified)
— All 17 tests in my range executed green: baseline `test-workspace.log.txt:459-480` (22 decoder tests total, incl. my 17 + part2's 4 + boundary test), e.g. `:461 converted_path_reuses_pooled_slots_and_stays_bit_exact ... ok`, `:466 native_endian_interleaved_decode_is_zero_copy ... ok`, `:472 swap_groups_into_is_byte_identical_to_the_reference_swapped ... ok`.
— Duration math re-derived by me: 1470 instants @ 44100 Hz → 1470·10⁹/44100 = 33,333,333.33 → floor 33,333,333 (test line 1460 asserts exactly this); 2 instants @ 44100 → 45,351.47 → 45,351 (line 1270); `u64::MAX` saturates to `u64::MAX/44_100` (line 1465) matching `saturating_mul` + `checked_div` semantics at lines 163-168.
— `s16le_stereo_decodes_whole_frames_with_format_and_duration` (1249-1280): asserts full `AudioFormat` equality, 8 output bytes, and all four sample VALUES via `from_ne_bytes` (endianness-portable) — would fail on any byte-dropping or value-mangling mutation.
— `native_endian_interleaved_decode_is_zero_copy` (1283-1318) and `zero_copy_path_never_materializes_a_pool_slot` (1556-1571): pointer identity (`as_ptr()` equality with the packet's allocation) + `convert_pool.is_empty()` — these FAIL if the decode copies; the zero-copy handoff (decode line 741 `packet.data.clone()`) is genuinely proven, not claimed.
— `swap_groups_into_is_byte_identical_to_the_reference_swapped` (1471-1498): xorshift-randomized data, bps ∈ {2,3,4,8}, lengths ∈ {0,1,2,7,64,1470·bps,4096}, equality against `swapped` AND an independent per-group reversal check — catches any hot-path swap regression.
— `converted_path_reuses_pooled_slots_and_stays_bit_exact` (1501-1553): 64 varying-content packets at stable geometry (a stale pooled slot would surface as wrong bytes), bit-exact vs the free surface, pool bounded ≤ `CONVERT_SLOTS`, geometry change (400-byte packet) still bit-exact and bounded.
— The "allocation-free per buffer after warm-up (D-1)" doc claim (module docs 66-68) is proven with a counting global allocator in `tests/pcm_zero_alloc.rs` (both the s16le zero-copy and a byte-swapped s16be twin built from the real AVI fixture, 256 packets each): baseline `:1247-1250` ok. Contrast: adpcm's identical claim has no such probe (my adpcm/mod.rs#part2 audit F-001).
— Planar damage model (1387-1420): 19 damaged bytes → 16-byte prefix keeps both planes' true boundaries (L at 0, R at 8); values asserted per plane — pins the one-expression-prefix-serves-both-layouts design claim (module docs 36-45).

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/audio/pcm/decoder.rs:717-719 (untested branch; test gap in my range 1070-1572)
Code:
```rust
        if let Some(outcome) = drop_stale_packet(&packet, generation) {
            return Ok(outcome);
        }
```
What is wrong: no test in the PCM test module (or any integration suite — verified `rg "stale|drop_stale"` over crates/codecs/tests: the only decoder-level stale tests are adpcm's `ima_packet_decodes_and_recovers_typed:1188-1192` and contract.rs:89 which uses the MJPEG decoder; pcm_fuzz_survival always decodes with the true generation) ever feeds `PcmDecoder::decode` a packet minted under a stale `GenerationId`, so this branch's behavior at this call site is executed by nothing.
Why it matters: the seek/reconfigure path (engine `Pipeline::seek_flush` mints new generations) relies on this drop; a regression here (e.g. accepting a stale packet and emitting a frame with the wrong generation) would surface as a post-seek glitch that no test would catch. Blast radius is small because the helper itself is unit-tested (codec-api/src/decode.rs:203-227) and ten sibling decoders pin the same one-line pattern.
What correct looks like: mirror adpcm's test — decode one good packet, then one stale-generation packet, assert `DecodeOutcome::NeedMoreInput` and that the sink saw nothing new.
Confidence: HIGH — Verification: read + exhaustive rg for coverage; branch behavior not executed anywhere in baseline.

## 4. Per-function table
| Function (all `#[test]`, my range) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `waveformatex_rejects_width_and_align_lies` | 1071-1107 | REAL | — | 4 refusal codes (bits/align/channels/rate), exact context strings |
| `float_descriptors_demand_the_float_tag` | 1110-1126 | REAL | — | F32Le + float tag ok; int tag refused — pins the tag/descriptor coupling |
| `configure_without_facts_is_a_typed_unsupported` | 1131-1150 | REAL | — | kind + code + stage all asserted |
| `with_format_seeds_facts_and_conflicts_are_typed` | 1153-1217 | REAL | — | seeded-only ok, agreeing ok, rate-conflict typed, zero-seed constructor refusals; channel-count-conflict variant untested (same code path) |
| `unconfigured_decode_and_flush_are_typed_bad_state` | 1220-1244 | REAL | — | both BadState surfaces |
| `s16le_stereo_decodes_whole_frames_with_format_and_duration` | 1249-1280 | REAL | — | format equality + values + duration floor — the anchor test |
| `native_endian_interleaved_decode_is_zero_copy` | 1283-1318 | REAL | — | pointer identity; cfg-picks LE/BE descriptor (BE arm never runs on x86_64 CI — honest cfg, not a lie) |
| `partial_tail_is_concealed_not_dropped` | 1323-1344 | REAL | — | ConcealedFrame + 12-byte prefix + decoder-still-usable |
| `sub_frame_packet_is_skipped_whole` | 1347-1361 | REAL | — | lengths 1..4 → skipped:1, nothing emitted |
| `empty_packet_is_need_more_input` | 1364-1369 | REAL | — | trivial, correct |
| `flush_drains_and_sticks` | 1372-1382 | REAL | — | Drained twice (idempotence) |
| `planar_truncation_keeps_plane_boundaries_at_whole_frames` | 1387-1420 | REAL | — | the design-critical planar damage case; values per plane |
| `planar_clean_passes_through_with_planar_layout` | 1423-1442 | REAL | — | layout + byte equality on LE host |
| `duration_floors_to_whole_nanoseconds` | 1447-1466 | REAL | — | floor + exact + saturating cases, all hand-verified |
| `swap_groups_into_is_byte_identical_to_the_reference_swapped` | 1471-1498 | REAL | — | randomized differential, all widths, ragged lengths |
| `converted_path_reuses_pooled_slots_and_stays_bit_exact` | 1501-1553 | REAL | — | 64 varying packets + geometry change; "reuses" itself is proven by pcm_zero_alloc.rs's allocator counts, not here (here: bounded + bit-exact) |
| `zero_copy_path_never_materializes_a_pool_slot` | 1556-1571 | REAL | — | pool empty + pointer identity |
| (boundary, part2's head) `waveformatextensible_resolves_guids_and_decodes` | 1009-1068 | REAL | — | double-covered for continuity: GUID resolution + s24 configure/decode + tag/subformat refusals; decode asserts format fields only (values pinned by the s16/planar tests + differentials) |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 17 in my range except `empty_packet`/`flush_drains` | YES | exact codes, values, counts, pointer identities — every mutation I reasoned through (drop a byte, swap wrong group, pool slot reuse returning stale bytes, prefix off-by-one) trips at least one |
| `empty_packet_is_need_more_input`, `flush_drains_and_sticks` | YES (weakly) | pin the two trivial outcomes; would catch a flush that errors or a state reset |
| `converted_path_reuses_pooled_slots_and_stays_bit_exact` — reference caveat | YES | the "reference" `decode_pcm` (pcm/mod.rs:281-289) shares `whole_frame_prefix`/`PcmPlan` with the subject, so prefix bugs are common-mode here — but tests `partial_tail`/`planar_truncation` pin the prefix with hand-built expectations, closing the blind spot |
| stale-generation branch (decode:717) | NO test exists | F-001 |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "steady-state decoding allocates nothing per buffer (D-1) — bounded memory, not unbounded caching" | decoder.rs module docs 65-68 | REAL | `pcm_zero_alloc.rs` counting allocator, both shapes, 256 packets — baseline :1247-1250 ok |
| "The output is at most the input: every conversion is a width- and order-preserving permutation" | decoder.rs module docs 60-61 | REAL | `swapped`/`swap_groups_into` are pure permutations (read); pooled slot len == prefix.cut (decode:749) |
| "A packet with no decodable sample-frame is skipped whole … Silence is never fabricated" | decoder.rs module docs 52-56 | REAL | `sub_frame_packet_is_skipped_whole` + `partial_tail_is_concealed_not_dropped` (:467, :471 ok) |
| Damage-model prefix rule serves both layouts | decoder.rs module docs 36-45 | REAL | `planar_truncation_keeps_plane_boundaries_at_whole_frames` (:469 ok) |
| Duration "floor division … never rounded up" | decoder.rs 160-162 | REAL | `duration_floors_to_whole_nanoseconds` (:462 ok), values hand-checked |

## 7. Dead / duplicated / unreachable code
- No dead code in my range. `swap_groups_into`'s ragged-tail arms (267-271, 277-283) are unreachable via the contract path (prefix is always a multiple of bps) but deliberately total — documented at 252-255 and pinned by the differential test's adversarial lengths. Correct defense-in-depth, not dead weight.
- F-001's untested branch is the only unreachable-by-tests code.

## 8. Dependencies & flags
- Test module uses only crate-internal items + `media_types`; `pcm` is unconditional (no feature gate) — ships in every build, `--no-default-features` included.
- `tests/pcm_zero_alloc.rs` depends on `testkit::alloc_count::CountingGlobalAlloc` + serialized test execution — sound design (allocator is process-global).

## 9. Open questions for the next agent
- part1/part2 owners: `parse_waveformatex` reads `nAvgBytesPerSec` (bytes 8-11) never — advisory by spec, consistent with adpcm; confirm no cross-check was intended (docs say only align/bits are cross-checked).
- The `with_format` + `extra_data` agreement check (658-669) compares channels/rate only; a WAVEFORMATEXTENSIBLE blob whose `wBitsPerSample` disagrees with the descriptor is refused by `parse_waveformatex` before the agreement check — no gap found.
- Hostile-question tie-in (log): PCM is the one audio decoder the shipped `engine-info` literal names truthfully; its rate handling is pass-through (no resampling in the decoder — correct; resampling belongs to audio-engine/src/resample.rs, another agent's file).
