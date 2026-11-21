# AUDIT: crates/codecs/src/audio/pcm/decoder.rs#part2 (lines 525–1048)
| Field | Value |
|---|---|
| Auditor | W07-A03 |
| Date | 2026-11-21 |
| Lines / functions | 524 (of 1573; part1=1–524 → W07-A05, part3=1049–1573 → W07-A04) / 16 items |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (chain: `lumen open` → playback-engine `DecoderProvider::decoder_for` → apps/cli/src/pipeline_host.rs:347-370 `codec.starts_with("pcm_")` → `PcmDecoder::with_format(fact.pcm.0/.1)` → `configure`/`decode`/`flush` audited here). Executed: baseline AVI first-vertical lane decodes 60/60 audio frames (`audit/runs/baseline/INDEX.md` §"contrast", `mp4-smoke-bench-avi-mjpeg-pcm.txt`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | sed -n quotes (528-532, 618-625, 655-672, 720-734, 740-758); rg "audio::pcm::decoder::tests" audit/runs/baseline/test-workspace.log.txt:459-480 (22 tests, all ok); rg "_planar|pcm_s16planar" crates/demux apps/cli (no planar descriptor minter — planar lane test-only); rg "fn from_descriptor|const ALL" crates/codecs/src/audio/pcm/mod.rs (11-format vocabulary); read part1 helpers `whole_frame_prefix`/`swap_groups_into`/`pooled_convert_slot`/`parse_waveformatex` (198-410) for cross-verification of this range's calls. |

## 1. One-paragraph verdict
This range is the decoder's contract surface — `PcmDecoder` struct, the two facts seams (`with_format`, `extra_data` WAVEFORMATEX negotiation in `resolve_facts`), the `Decoder` impl (`configure`/`decode`/`flush`), and the WAVEFORMATEX/WAVEFORMATEXTENSIBLE test battery. The decode path is correct on every edge I could construct: whole-sample-frame prefix arithmetic is layout-uniform (interleaved and planar reduce to the same `frame_bytes` multiple — verified by hand for 2ch planar), the zero-copy handoff condition is exact, the pooled-conversion fill lengths always match (`slot len == prefix.cut == segment.len()`, and `cut` is a multiple of `bps` so swap groups align), sub-frame packets are skipped whole, damaged tails are concealed-and-reported, and `Emitted { frames: 1 }` is only ever returned after a frame was actually accepted (the adpcm sibling's trap is not present here). The one defect is a doc-example provenance lie. The W2-15 regression battery (channel ceiling, checked align product, extensible length ladder) is mutation-sensitive and passes in the committed baseline.

## 2. What actually works (verified)
— verified by audit/runs/baseline/test-workspace.log.txt:459-480 → 22 `audio::pcm::decoder::tests` all ok, incl. `native_endian_interleaved_decode_is_zero_copy`, `zero_copy_path_never_materializes_a_pool_slot`, `partial_tail_is_concealed_not_dropped`, `sub_frame_packet_is_skipped_whole`, `planar_truncation_keeps_plane_boundaries_at_whole_frames`, `swap_groups_into_is_byte_identical_to_the_reference_swapped`.
— verified by audit/runs/baseline/INDEX.md (AVI first-vertical lane: `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60` on `avi_mjpeg_pcm.avi`) — this range's `decode` produced 60 real audio frames end-to-end in the shipped CLI.
— verified by read: `whole_frame_prefix` (part1:198-209) + this range's use at 721 — `cut = instants × frame_bytes`, `dropped_tail = len − cut`; for planar s16 2ch, whole planes require `len/2` even ⇒ `len % 4 == 0` — identical predicate to interleaved; no plane-boundary mislocation is possible.

## 3. Findings

### F-001 [S4] — doctest presents the `extra_data` path as the AVI first-vertical wiring; the shipped engine uses `with_format` and passes `extra_data: None`
Where: crates/codecs/src/audio/pcm/decoder.rs:530-531
Code:
```rust
/// // The AVI first vertical: s16le stereo 44100 — the container's
/// // `strf` (WAVEFORMATEX) rides `extra_data`.
```
What is wrong: In the shipped engine the AVI strf facts reach this decoder through `PcmDecoder::with_format` (apps/cli/src/pipeline_host.rs:347-359, whose comment states "the engine passes no extra_data"), never through `DecoderConfig::extra_data`. The doctest labels the `extra_data` form as "The AVI first vertical" wiring.
Why it matters: A reader tracing the shipped first-vertical lane from this doc is sent down the wrong seam; the two paths also differ in validation (the blob path enforces the 64-channel ceiling and full WAVEFORMATEX cross-checks, the seeded path deliberately does not — decoder.rs:128-135, 618-636, 655), so the distinction is not cosmetic.
What correct looks like: "A direct-contract configuration: the container's `strf` (WAVEFORMATEX) as `extra_data` (the shipped AVI lane instead harvests the facts and uses `PcmDecoder::with_format`)".
Confidence: HIGH — Verification: read (pipeline_host.rs:347-359 + baseline INDEX) / ran (baseline lane cite).

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| doc example (tail) | 525-579 | REAL | S4 | compiles+runs as doctest; F-001 provenance |
| `pub struct PcmDecoder` + Default | 580-599 | REAL | — | plan/seeded/generation/codec + pool(cursor); Debug derive |
| `PcmDecoder::new` | 604-606 | REAL | — | trivial |
| `PcmDecoder::with_format` | 618-636 | REAL | — | validates channels≠0, rate≠0 only; no 64-ceiling — deliberate (documented at part1:128-135, self-limiting: a frame is only emitted when `data.len() ≥ channels×bps`, so an absurd seeded count can never materialize a frame); no panic paths |
| `PcmDecoder::resolve_facts` | 643-673 | REAL | — | 3 arms: absent→typed `pcm.stream_format_absent`; seeded-only→as-is; both→parse + conflict check `pcm.stream_format_conflict`; let-chain is fine; no swallowed errors |
| `Decoder::configure` | 677-706 | REAL | — | `_planar` suffix strip → `PcmFormat::from_descriptor` (typed refuse `pcm.codec_descriptor`); facts → `PcmPlan::new` (validates nonzero rate/channels, saturating frame_bytes); no partial-state on error; planar lane has no shipped descriptor minter (test-only today) |
| `Decoder::decode` | 708-777 | REAL | — | stale guard (codec-api drop_stale_packet); BadState guard; empty→NeedMoreInput; sub-frame→Recovered{skipped:1,CorruptInput}; zero-copy iff `!swap && cut==len`; conversion path lengths proven equal (slot `len == prefix.cut == segment.len()`, `cut % bps == 0`); duration floor-div saturating; `Emitted{frames:1}` only after `out.accept` — honest; `dropped_tail>0` → ConcealedFrame after emitting the prefix frame |
| `Decoder::flush` | 779-786 | REAL | — | BadState if unconfigured; Drained otherwise — matches stream-stateless docs; contrast: adpcm flush (mod.rs:678-683) lacks the BadState check (other agent's file) |
| tests: `waveformat`/`configured`/`packet`/`Capture`/`decode_one` helpers | 791-868 | REAL | — | helpers; `unreachable!` only in test code |
| `waveformatex_accepts_16_and_18_byte_forms` | 872-881 | REAL | — | mutation-sensitive (length gate) |
| `waveformatextensible_length_ladder_pins_the_physical_bound` | 884-929 | REAL | — | strong ladder: 40+vendor tails accepted, 17/19..39 refused `pcm.waveformat.length`, 16/18 extensible refused `pcm.waveformat.extensible`; pins W2-15 (a4) |
| `channel_counts_beyond_the_ceiling_are_typed_refusals` | 932-968 | REAL | — | pins W2-15 (g1) align-truncation fix (32768×s16 align-0 wrap case); 64 boundary accepted; order of checks (channels before tag/bits/align) matches parse order |
| `waveformatex_rejects_short_long_and_wrong_tags` | 971-1006 | REAL | — | short/long(38B)/float-tag-on-int/extensible-too-short; all typed |
| `waveformatextensible_resolves_guids_and_decodes` | 1008-1048 (+continues into part3 — chunk boundary cuts this test; tail owned by W07-A04) | REAL | — | E2E: s24 extensible blob → configure → 6-byte packet → Emitted, S24 2ch; float GUID + int descriptor → typed tag error |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all five tests in range + the 17 later `decoder::tests` (baseline :459-480) | YES | each pins a typed context code, an accepted/rejected length, or decoded output; mutation of the length gate, ceiling, align product, GUID resolution, prefix arithmetic, swap loop or pool fill flips at least one; e.g. breaking `whole_frame_prefix` breaks `partial_tail_is_concealed_not_dropped` + `sub_frame_packet_is_skipped_whole`; breaking the pool breaks `converted_path_reuses_pooled_slots_and_stays_bit_exact` |
| doctest (525-579) | YES | asserts `Emitted { frames: 1 }` via `unreachable!` on any other outcome |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Silence is never fabricated … an honest shorter frame is not" (module docs 47-56, part1) | decoder.rs:32-56 | REAL | code path 722-733 + 769-776: no padding, tail dropped and reported; test `partial_tail_is_concealed_not_dropped` ok (baseline :467) |
| "re-configuration after a seek is free" (517-521) | decoder.rs:517-521 | REAL | configure is stateless re: decode; pool reused by length match (part1:307-341) |
| "output is at most the input" (58-68, part1) | decoder.rs:58-68 | REAL | both paths hand out exactly `prefix.cut ≤ data.len()` bytes |
| "the container's `strf` rides `extra_data`" (530-531) | decoder.rs:530-531 | FALSE (for the shipped wiring) | pipeline_host.rs:348-349 "the engine passes no extra_data"; F-001 |

## 7. Dead / duplicated / unreachable code
- The `_planar` configure lane (679-682) has no shipped descriptor minter (rg over demux/apps: none) — exercised by tests only (`planar_clean_passes_through_with_planar_layout` etc.). Forward-looking vocabulary, tested, not dead weight but unreachable in the shipped CLI today.
- No duplication in range. (`decode_payload`/`swapped` free surface at part1:215-244 duplicates the contract path's logic by design — hot/cold twins, differential-pinned.)

## 8. Dependencies & flags
- No external deps used in this range; no feature gates; no unsafe. `cfg!(target_endian)` in part1's plan builder is the only target-conditional.

## 9. Open questions for the next agent
- Part1 owner (W07-A05): `pooled_convert_slot` cold path `unreachable!("fresh SharedBytes has refcount 1")` (decoder.rs:330) — verify SharedBytes::from_vec can never return a shared handle (panic path in production code otherwise).
- Part3 owner (W07-A04): the `waveformatextensible_resolves_guids_and_decodes` test's tail (1049-1060-ish) — confirm the float-GUID/int-descriptor assertion matches the tag check at 453-472.
- Engine team: `decode` can deliver a frame AND return `Recovered{ConcealedFrame}` (769-773) — same outcome-accounting question as adpcm; confirm the pipeline counts frames from the sink, not the outcome.
