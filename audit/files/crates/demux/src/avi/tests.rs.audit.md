# AUDIT: crates/demux/src/avi/tests.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-23 |
| Lines / functions | 3128 / 81 (58 #[test] + 23 helpers/types/impls) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO for this file — `#[cfg(test)] mod tests;` (avi/mod.rs:1270-1271), test-only. The code under test IS reachable: `AviDemuxer::new` <- apps/cli/src/containers/avi.rs:40 (`use demux::avi::AviDemuxer`) <- lumen-cli open path. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 5 |
| Commands run | rg (constant/reason cross-refs, callers), sed -n quote verification, awk over audit/runs/baseline/cargo-test-workspace.txt. No cargo runs (baseline cited instead). |

## 1. One-paragraph verdict
This is a genuine, hostile-grade unit-test suite: it builds real AVI byte streams with an in-file RIFF builder (no mocking of the demuxer), asserts exact tick ladders, packet counts, payload bytes, damage-ledger `(skipped, reason)` pairs, typed error codes, and seek/re-anchor landings. All 58 tests pass in the orchestrator baseline (demux unittests: `test result: ok. 462 passed; 0 failed; 0 ignored` — baseline cargo-test-workspace.txt line ~2111; avi::tests listed at lines 1830-1887). Mutation reasoning says the suite would catch broken track-table parsing, ladder timing, pad stripping, idx1 truth, resync, budget exhaustion, and seek boundary inclusivity. The one material hole: the demuxer's 32 MiB `MAX_CHUNK_PAYLOAD` bounds check — the primary hostile-input memory cap — has ZERO test coverage in this file or anywhere in the repo, plus three smaller untested damage paths and some dead test code.

## 2. What actually works (verified)
— verified by `rg -c "^#\[test\]"` -> 58; baseline `awk` over cargo-test-workspace.txt -> demux unittests `462 passed; 0 failed`, with all 58 `avi::tests::*` lines `... ok` (baseline lines 1830-1887). Test binaries built and run by the orchestrator; no re-run attempted here (cargo-build-budget avoided; baseline cited per spec §2).

## 3. Findings
F-001 [S3]
Where: crates/demux/src/avi/tests.rs (whole file; the gap) vs crates/demux/src/avi/mod.rs:516-518
Code (mod.rs:516-518, verbatim):
```
                    let size = u64::from(header.size);
                    if size > MAX_CHUNK_PAYLOAD {
                        self.damage.note(size, "chunk-payload-over-ceiling");
```
What is wrong: the `chunk-payload-over-ceiling` path — the bounds check that stops a declared chunk size from being trusted into a >32 MiB read — is asserted by no test. `rg -n "chunk-payload-over-ceiling"` over the whole repo returns only mod.rs:518 and a doc comment in error.rs:137. Every other damage reason in mod.rs (`riff-extends-past-stream`, `movi-extends-past-riff`, `chunk-payload-past-eof`, `chunk-payload-past-list-bound`, `rec-extends-past-list-bound`, `unknown-chunk-in-movi`, `chunk-jump-past-list-bound`, `movi-resync-idx1/scan/parent`, pcm/strf/strh reasons) is pinned by name in this file; the memory cap is the exception. Why it matters: deleting or inverting the `if size > MAX_CHUNK_PAYLOAD` guard passes the entire suite (also fuzz_loop_stable.rs only cites the constant in a comment, line 46); a regression to unbounded reads from header-declared sizes would be invisible to CI. What correct looks like: a test building a file whose chunk declares e.g. `MAX_CHUNK_PAYLOAD + 1` bytes, asserting reason `chunk-payload-over-ceiling`, the damage byte total, and that the walk continues. Confidence: HIGH — Verification: read + rg (counts pasted above).
F-002 [S4]
Where: crates/demux/src/avi/tests.rs (whole file) vs mod.rs:643 and mod.rs:1212
Code (mod.rs:642-644, verbatim):
```
                        Err(_) => {
                            self.damage.note(0, "packet-duration-unrepresentable");
```
What is wrong: two more damage paths have zero coverage: `packet-duration-unrepresentable` (0 refs in tests.rs) and `idx1-before-movi` (0 refs). Blast radius: small (degenerate timing bases / weird top-level order), but the file's own standard is name-pinned coverage of every reason. Confidence: HIGH — Verification: rg counts (`packet-duration-unrepresentable: 0`, `idx1-before-movi: 0`).
F-003 [S4]
Where: crates/demux/src/avi/tests.rs:2616-2620
Code (verbatim):
```
    assert_eq!(
        avi.damage_total(),
        36,
        "duplicate movi (4) + duplicate idx1 (32) payload bytes"
    );
```
What is wrong: `duplicate_movi_and_duplicate_idx1_count_as_damage` asserts only the summed byte total, not the distinct reasons `duplicate-movi` / `duplicate-idx1` (mod.rs:1167, 1191). If both bytes were attributed to one reason, or the reasons were swapped, this test still passes — weaker than the file's own pin-by-name standard. Confidence: HIGH — Verification: read + rg.
F-004 [S4]
Where: crates/demux/src/avi/tests.rs:2721-2734
Code (verbatim):
```
    let truth = from_start_truth(
        &[video_stream()],
        &[
            (b"00dc", &b"a"[..]),
```
(and at 2733-2734:)
```
    let mut truth_indexed = truth;
    truth_indexed.clear();
```
What is wrong: `from_start_truth` runs a full build+demux+drain whose result is immediately cleared — dead computation kept only as an allocation reuse trick, with a comment admitting the layout was wrong. Wasted work and reader confusion in a 3.1k-line file. Confidence: HIGH — Verification: read (sed -n '2721,2735p').
F-005 [S4]
Where: crates/demux/src/avi/tests.rs:2155-2161, 2597-2601, 2805
Code (2155-2161, verbatim):
```
    let movi_list_start = built
        .chunk_positions
        .first()
        .copied()
        .unwrap_or(0)
        .saturating_sub(12);
    let _ = movi_list_start;
```
What is wrong: three dead-code spots: (a) `movi_list_start` computed then discarded; (b) `duplicate_movi...` writes an empty LIST chunk then truncates it back (`chunk(&mut bytes, b"LIST", &[]); let _ = extra_start; ... bytes.truncate(extra_start);` 2597-2601); (c) `let _ = index;` at 2805 in the interleaved-reanchor closure. All harmless to correctness, all hygiene debt the fmt gate cannot see. Confidence: HIGH — Verification: read (sed verified each).
F-006 [S4]
Where: crates/demux/src/avi/tests.rs:2474-2482 (test body)
Code (verbatim):
```
    let built = build(
        &[video_stream()],
        &[
            (b"00dc", &b"a"[..]),
```
What is wrong: `seek_does_not_move_the_read_position` exercises only the no-idx1 linear fallback (`with_idx1 = false`). The indexed-seek path's position restoration has no direct positions-log assertion (indirect coverage only, via seek-then-drain tests like `absolute_idx1_offsets_are_autodetected_and_seek`:1492-1500). Also no end-to-end test with stream ordinals ≥ 10 (tags like `10dc`; covered only at riff.rs unit level, riff.rs:800-803). Confidence: MEDIUM — Verification: read.

## 4. Per-function table
Helpers (all verified by read; all test-only, unreachable from release):
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| chunk | 19-28 | REAL | — | RIFF chunk writer w/ WORD pad; honest |
| end_sized | 30-33 | REAL | — | back-patches u32le size; caller-guarded |
| avih | 35-50 | REAL | — | 56-byte main header |
| strh | 53-74 | REAL | — | 56-byte stream header |
| strf_video | 76-87 | REAL | — | signed biHeight honored (line 81) |
| strf_pcm / strf_wave | 89-104 | REAL | — | WAVEFORMATEX builder |
| strf_extensible | 120-135 | REAL | — | 0xFFFE + cbSize 22 + GUID |
| Stream enum | 138-171 | REAL | — | adversarial variants incl. ShortStrh |
| video_stream / audio_stream | 173-191 | REAL | — | fixtures |
| build | 203-309 | REAL | — | full RIFF/AVI writer, idx1 rel. to movi form (297) |
| demuxer_over | 311-319 | REAL | — | unreachable! on error — acceptable in tests |
| drain_packets | 323-334 | REAL | — | asserts clean drain |
| start_and_inspect | 756-785 | REAL | — | #[allow(type_complexity)] justified |
| audio_case | 790-804 | REAL | — | 4 sample-frames payload |
| patch_size | 1324-1333 | REAL | — | wrapping_add size patch |
| smash_chunk_header | 1815-1819 | REAL | — | zero fourcc + garbage size |
| drain_with_damage | 1832-1845 | REAL | — | pins (skipped, reason) order |
| resync_body | 1851-1867 | REAL | — | 6-chunk two-stream body |
| span_at_nanos | 2264-2270 | REAL | — | 1/1e9 base query span |
| video_stream_of / from_start_truth / drain_through | 2660-2794 | REAL | — | truth oracle; see F-004 for dead call |
| impl SourceStream for Broken | 2215-2237 | REAL | — | mock of the SOURCE, not the thing under test — legitimate |
| impl SourceStream for Tracked | 2446-2472 | REAL | — | position-recording wrapper, real demuxer |

Tests (all REAL; all pass per baseline lines 1830-1887; "fail-if-broken" = mutation reasoning):
| Test | Lines | Fail if broken? | Reason |
|---|---|---|---|
| two_stream_start_reports_the_track_table | 341-412 | YES | exact ids/codec/timebase/span/stream_format + idempotent start + TrackId(9)->None |
| strn_title_surfaces_on_the_track | 415-450 | YES | title round-trip through raw bytes |
| unsupported_stream_types_are_filtered_with_damage | 453-484 | YES | filter + Recovered(reason) order |
| all_streams_unsupported_is_typed | 487-506 | YES | typed refusal code |
| zero_rate_or_scale_skips_the_stream_not_the_file | 515-555 | YES | ordinal alignment (handle 1) + packets |
| all_streams_degenerate_is_typed_no_supported_streams | 558-583 | YES | code+kind+stage |
| negative_biheight_accepts_bottom_up_dib_with_damage | 586-623 | YES | abs height in StreamFormat + note |
| short_strh_skips_the_stream_siblings_survive | 626-646 | YES | 55-byte strh containment |
| short_strf_skips_the_stream_siblings_survive | 649-707 | YES | 39/15-byte strf, both orders |
| channels_over_ceiling_skips_the_stream_siblings_survive | 710-743 | YES | nChannels 65535 |
| pcm_wave_tag_matrix_maps_every_width_and_kind | 807-857 | YES | 6 (tag,bits) combos, codec strings, PTS [0,4] |
| wave_format_extensible_resolves_subformat_guids | 860-923 | YES | 5 GUID/mask combos, ffmpeg-shaped |
| impossible_pcm_combinations_skip_the_stream_siblings_survive | 926-993 | YES | 6 refusal reasons by name |
| float_pcm_without_sample_ladder_skips_the_stream | 996-1027 | YES | ssize 0 refusal |
| block_align_disagreements_are_damage_notes_not_skips | 1030-1062 | YES | strh ladder wins, PTS [0,2] |
| mjpa_handler_is_filtered_like_any_unsupported_handler | 1065-1093 | YES | probe/demuxer stage agreement |
| packets_carry_the_avi_ladder_timing | 1100-1158 | YES | video ticks 0..2, dts==pts, 33_333_333ns, audio [0,2], 41_667ns rounding |
| odd_payloads_strip_the_word_pad | 1161-1179 | YES | exact payload bytes, damage 0 |
| rec_lists_are_transparent | 1182-1225 | YES | 3 payloads in order |
| rec_list_overrunning_parent_bound_clamps_with_damage | 1228-1286 | YES | clamp 20 bytes, reason pinned |
| unknown_movi_chunks_skip_with_damage | 1289-1321 | YES | JUNK splice, sizes re-patched |
| pcm_misalignment_is_damage_not_silence | 1336-1363 | YES | 6-byte chunk vs align 4 |
| idx1_keyframe_flag_is_the_truth_when_present | 1366-1400 | YES | middle entry non-key |
| lying_idx1_size_clamps_to_available_bytes | 1403-1447 | YES | 0xFFFFFFF0 clamp, salvage seeks |
| absolute_idx1_offsets_are_autodetected_and_seek | 1450-1501 | YES | dwOffset rewrite + seek to tick 1 |
| idx1_damage_coalesces_per_reason | 1504-1561 | YES | 1000 lying entries -> 1 event (anti-storm) |
| broken_magic_is_typed_not_avi | 1568-1583 | YES | 'X' first byte |
| truncated_head_is_typed_at_the_boundary | 1586-1641 | YES | cuts {1,3}/{4,5,11}/{12,20,30,44}, sticky replay |
| truncated_movi_payload_delivers_prefix_damage_then_eos | 1644-1707 | YES | exact 6-event ledger, EOS idempotent |
| cut_exactly_at_a_chunk_payload_start_is_damage_then_eos | 1710-1743 | YES | zero-payload boundary |
| chunk_size_past_movi_bound_clamps_and_continues | 1746-1804 | YES | clamp math pinned (in_movi, 40) |
| garbage_chunk_jump_past_bound_resyncs_via_idx1 | 1870-1909 | YES | exact 3-entry ledger + 5 payloads |
| garbage_chunk_jump_past_bound_resyncs_by_scan_without_idx1 | 1912-1953 | YES | odd garbage size, pad accounted |
| garbage_chunk_jump_without_landing_is_typed_never_silent_eos | 1956-2012 | YES | typed terminal, damage_total pinned |
| garbage_chunk_jump_inside_rec_hands_back_to_parent | 2015-2077 | YES | parent handback ledger |
| resync_budget_exhaustion_is_the_typed_terminal | 2080-2148 | YES | 33 vs 32 budget; total == 66*G-7272 pins MAX_WALK_RESYNCS exactly |
| missing_header_and_missing_movi_are_distinct_typed_errors | 2151-2187 | YES | two distinct codes (dead var: F-005) |
| methods_before_start_are_typed_not_panics | 2190-2204 | YES | next_event + seek before start |
| unusable_sources_are_rejected_at_construction | 2207-2258 | YES | 3 source-capability codes, stage Open |
| seek_lands_on_last_keyframe_at_or_below_target | 2273-2353 | YES | 0/45/65/100ms landings, boundary inclusive |
| seek_without_idx1_walks_the_movi | 2356-2385 | YES | 50ms -> tick 2 rounding; pure query |
| seek_out_of_range_is_typed | 2388-2426 | YES | past-end / negative / unknown span |
| seek_does_not_move_the_read_position | 2429-2522 | YES | positions log equality (linear path only — F-006) |
| lying_hdrl_size_does_not_swallow_the_movi | 2529-2542 | YES | 0x7FFF0000 hdrl lie |
| riff_extent_clamps_the_walk_to_real_bytes | 2545-2589 | YES | trailing garbage invisible; shrink -> typed Truncated |
| duplicate_movi_and_duplicate_idx1_count_as_damage | 2592-2621 | PARTIAL | total only, reasons not pinned (F-003) |
| eos_is_idempotent_and_terminates | 2624-2640 | YES | 3x EOS |
| error_module_constructors_stay_demux_staged | 2643-2652 | YES | stage+code cross-check |
| reanchor_moves_the_stream_and_packets_continue_from_the_landing | 2695-2780 | YES | exact resumed truth [(0,2),(0,3)] (dead call F-004) |
| reanchor_on_interleaved_streams_keeps_both_ladders_true | 2797-2842 | YES | truth[4..] equality across both tracks |
| reanchor_without_idx1_scans_and_continues | 2845-2874 | YES | scan landing + resume |
| reanchor_failure_leaves_the_demuxer_at_its_position_d4 | 2877-2905 | YES | D4 continuation after typed refusal |
| reanchor_from_eof_revives_the_stream | 2908-2932 | YES | backward seek after EOS replays 3 packets |
| reanchor_clears_pending_events_stale_packets_never_reappear | 2935-2982 | YES | pending packet 'b' dropped |
| reanchor_at_end_lands_last_and_past_end_is_typed | 2985-3010 | YES | at-end inclusive, past-end typed |
| reanchor_landing_agrees_with_the_pure_query | 3013-3036 | YES | ticks+base equality |
| slow_linear_scan_is_bounded_and_typed | 3039-3098 | YES | 1002-packet budget; too-slow typed, D4, query unbounded, past-end typed |
| seekability_reports_the_container_shape | 3101-3128 | YES | None/Fast{indexed}/SlowLinear |

## 5. Test honesty
Covered by the tables above. Summary: 57/58 would fail loudly on relevant breakage (most via exact-value or unreachable!-on-violation asserts); 1 (`duplicate_movi_and_duplicate_idx1_count_as_damage`) is PARTIAL (byte total only). No tautologies, no #[ignore], no cfg-gating beyond the module-level `#[cfg(test)]`, no goldens generated by the code under test (expected values are hand-derived arithmetic, e.g. 66*G-7272 at 2143-2147).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "in-memory containers built by the same byte-level rules the fixture writer uses" | tests.rs:1-4 header | REAL | builder mirrors RIFF rules; cross-checked mod.rs doc; corpus tests are separate files |
| "Unit tests for the AVI demuxer core" | tests.rs:1 | REAL | baseline lines 1830-1887, 58 ok |

## 7. Dead / duplicated / unreachable code
F-004/F-005 items (truth call cleared; movi_list_start; write-then-truncate; `let _ = index`). Nothing else found. Module itself is `#[cfg(test)]`-gated (correctly test-only).

## 8. Dependencies & flags
No feature flags; test module uses only crate-internal `test_support` (mem_stream/BytesStream), media_types, plugin_api. Note for other agents: `demux::avi::test_support` is PUBLIC and compiled into release, and apps/cli/src/demux_host.rs:739 calls `test_support::mem_stream` — a test seam inside a shipped CLI path (not this file's defect; flagged for the demux_host/test_support auditors).

## 9. Open questions for the next agent
- Who owns the missing `chunk-payload-over-ceiling` test — avi/tests.rs or a corpus/fuzz file? (F-001; fuzz_loop_stable.rs only mentions the constant in a comment.)
- Confirm at the engine level whether indexed-seek position restoration matters (F-006) — pure-query seek that moved the read cursor would corrupt subsequent drains.
