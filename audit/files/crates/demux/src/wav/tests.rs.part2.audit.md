# AUDIT: crates/demux/src/wav/tests.rs#part2 (lines 460-919)
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-11-21 |
| Lines / functions | 460 (chunk 460-919 of 919; part1=1-459 is A04's) / 16 tests in range (15 full + 1 boundary) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (cfg(test)); code under test reachable: `lumen open x.wav` → probe RIFF/RF64/BW64+WAVE → `apps/cli/src/containers/mod.rs` wav arm → WavDemuxer; baseline: 34 `wav::tests::` all ok (`orchrun-test-workspace-nofailfast.log.txt:1928-1948`, incl. every test in this chunk) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | full Read 460-919 + helpers 1-130 (part1 context, read-only); `sed -n` verification of every quote; `sed -n '820,880p;930,975p;1023,1060p'` wav/mod.rs → step/locate/start logic (sticky gates verified); `grep -n "pub fn" wav/error.rs` → 11 constructors; repo-wide `grep payload_too_large / bad-format-params` → dead/unreachable-by-design statuses; baseline greps for all 16 in-range tests; IMA samples-per-block arithmetic hand-checked (2041 ✓) |

## 1. One-paragraph verdict
The error-path and seek battery of the WAV demuxer, and it is exact: every typed refusal code is pinned by a builder-crafted file whose bytes I verified (RIFF sizes patched honestly, fmt layouts correct, the IMA ADPCM samples-per-block 2041 arithmetic correct), the sentinel semantics (RF64-without-ds64 refused; plain-RIFF 0xFFFFFFFF delivered-with-damage) are pinned to full event vectors, and the seek tests pin landings, reanchor commits, block-boundary ADPCM behavior and D4 state preservation. All 16 in-range tests executed and passed in the baseline. Two S4s only: a fake-use `let _ = &mut b;` lint-suppression wart, and the `trait_misuse` test's pinning of a gratuitous bricking policy — one pre-`start` `next_event` permanently poisons the demuxer while the equivalent pre-`start` `seek` is a harmless typed refusal.

## 2. What actually works (verified)
- Byte-level honesty of the builders verified: `zero_data_size` RIFF size 36 = 4+8+16+8+0 ✓; `data_sliver` patches RIFF size and pads the odd byte ✓; `adpcm_seek`'s fmt is a real 20-byte IMA body and samples-per-block = (1024−4)×2+1 = 2041 ✓ (the standard mono IMA count); `oversized_info_value`'s LIST length arithmetic (4+8+value+pad) ✓.
- `declared_size_past_actual_is_typed_truncation_after_packets` (488-515) pins the full honest ordering: `Recovered{8, "data-extends-past-stream"}` → 8-byte prefix packet → typed `demux.wav.truncated` → `damage_total()==8`. Traced against mod.rs `step()` (declared vs effective end at 845-851) — the test would fail under any silent-EOF or silent-forfeit regression.
- Sentinel pair (775-854): RF64+0xFFFFFFFF without ds64 → `demux.wav.bad-structure` (the mandatory size-truth chunk absent); plain RIFF with head+data sentinels → payload delivered + exact two-note event vector `[riff-size-sentinel, data-sentinel-without-ds64, EndOfStream]`. The ffmpeg-divergence is documented in-test, not hidden.
- `seek_maps_frames_and_reanchors` (666-713): landing ticks 500, past-end typed `demux.wav.seek.out-of-range`, reanchor pts 500, exact 500×4 byte remainder — mutation-proof. `seek_errors_stay_sticky_free_and_demuxer_unmoved` (760-773) pins D4 behaviorally (full 40-byte drain after a failed seek).
- Error-code coverage cross-checked against the 11 constructors in wav/error.rs: 9 pinned by behavioral tests in this file (not_wav 464/912, truncated 479/485/512, bad_structure 803, no_fmt 613, no_data 543/576/629, not_started 643/656/663, seek_out_of_range 692, unsupported_format 916); the 2 unpinned (`bad_format_params`, `payload_too_large`) are documented-unreachable (mod.rs:837-839 comment) and documented-dead (error.rs:146-152 "Vocabulary without a minting site today") respectively — no live code unpinned.
- Baseline: all 16 in-range tests ok (nofailfast:1928-1948 region).

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/wav/tests.rs:491 and 514
Code:
```rust
    let mut b = wav_over(&pcm16_fmt(2, 8_000), &[1u8, 0, 2, 0, 3, 0, 4, 0], 16);
```
and (line 514):
```rust
    let _ = &mut b;
```
What is wrong: `b` is never mutated — it is only cloned at 493 — so the `mut` is unneeded, and line 514's `let _ = &mut b;` exists solely to manufacture a mutable borrow and silence the `unused_mut` warning the declaration would otherwise raise.
Why it matters: lint-suppression residue instead of deleting one word; it teaches readers that fake-use statements are the house style.
What correct looks like: `let b = wav_over(...)` and delete line 514.
Confidence: HIGH — Verification: read (both lines sed-verified; no other use of `b` in the test body).

### F-002 [S4]
Where: crates/demux/src/wav/tests.rs:657-663
Code:
```rust
    // Operational misuse is sticky (the landed convention — the
    // instance replays its shape; a fresh demuxer is the recovery).
    let err = match wav.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.wav.not-started");
```
What is wrong: the test pins a gratuitous bricking policy. One `next_event()` before `start()` routes through `self.fail(not_started())` (mod.rs:1052-1054) and sets sticky; `start()` then hits the sticky gate (mod.rs:1026-1028) and can never succeed — a side-effect-free misuse permanently poisons an unopened demuxer. The equivalent pre-`start` `seek` is a *non-sticky* typed refusal (`locate()` returns `error::not_started()` directly, mod.rs:944-946 — no `fail()`), so the two trait methods have inconsistent misuse policies, and the trait itself blesses pre-session queries ("hosts can validate targets before a session exists", plugin-api/src/demux.rs:39-43).
Why it matters: API robustness asymmetry with no functional need — `not_started` consumes no parsing state, so start-after-misuse could legitimately work. Mitigations: no shipped path trips it (containers call `start()` before wrapping), and the behavior is deliberately documented and test-pinned, so this is a design note, not a hidden defect.
What correct looks like: `not_started` misuses return the typed error without setting sticky (start remains usable), matching `locate()`'s non-sticky refusal.
Confidence: HIGH — Verification: read (fail/sticky gates traced in mod.rs:1046-1048, 1052-1054, 1026-1028; locate's non-sticky path at 944-946).

## 4. Per-function table
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| (not-wav boundary test tail) | 460-465 | REAL | — | code pinned; body starts in part1 (A04) |
| `truncated_mid_header_is_typed_truncation` | 467-486 | REAL | — | cut at 24 (mid-fmt) verified; sticky replay asserted (mechanism verified at mod.rs:1026) |
| `declared_size_past_actual_is_typed_truncation_after_packets` | 488-515 | REAL | S4 | strongest ordering pin; F-001 wart |
| `zero_data_size_is_typed_no_data` | 517-544 | REAL | — | RIFF size 36 arithmetic verified |
| `data_sliver_below_one_frame_is_no_data` | 546-577 | REAL | — | sliver-vs-trailing-partial policy boundary pinned |
| `trailing_partial_frame_is_damaged_and_dropped` | 579-600 | REAL | — | whole-frames-only delivery pinned |
| `missing_fmt_and_data_are_typed` | 602-630 | REAL | — | mild overlap with zero_data (same no-data code, different builder) |
| `trait_misuse_is_typed_not_a_panic` | 632-664 | REAL | S4 | F-002: bricking policy pinned as intended |
| `seek_maps_frames_and_reanchors` | 666-713 | REAL | — | landing/out-of-range/reanchor/remainder all pinned |
| `adpcm_seek_lands_on_block_boundaries` | 715-758 | REAL | — | IMA 2041 arithmetic verified; block landing + 2-block packet pinned |
| `seek_errors_stay_sticky_free_and_demuxer_unmoved` | 760-773 | REAL | — | D4 pinned behaviorally |
| `rf64_sentinel_without_ds64_is_a_declared_lie` | 775-804 | REAL | — | structural refusal pinned |
| `plain_riff_data_sentinel_means_to_stream_end` | 806-854 | REAL | — | full event vector pinned; ffmpeg divergence documented |
| `probe_recognizes_the_three_magics_and_rejects_other_forms` | 856-865 | REAL | — | RIFF/RF64/BW64 accepted; AVI/RIFX/short/FORM rejected |
| `oversized_info_value_is_damage_not_fatal` | 867-908 | REAL | — | LIST length arithmetic verified; damage-not-fatal pinned |
| `error_constructors_keep_their_codes` | 910-919 | REAL | — | 4 of 11 constructors, but module-wide coverage of live codes is complete (§2) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| truncated_mid_header | YES (code); NO (sticky mechanism) | replay passes by either sticky gate or re-parse — ambiguous externally |
| declared_size_past_actual | YES | four-observable ordering pin; silent-EOF or silent-forfeit regressions fail it |
| zero_data / data_sliver / trailing_partial / missing_fmt | YES | codes + delivered bytes pinned |
| trait_misuse | YES (as a pin of current behavior) | asserts the bricking itself — regression to recoverable-start would fail it (by design) |
| seek_maps / adpcm_seek / seek_errors | YES | landings, ticks, remainders, D4 drain pinned |
| rf64 / plain_riff sentinels | YES | code + full event vector pinned |
| probe_recognizes | YES | accept/reject matrix pinned |
| oversized_info | YES | info empty + damage>0 + delivery pinned |
| error_constructors | YES | 4 codes pinned (live coverage completed by behavioral tests) |
| Baseline execution | — | all in-range tests ok (nofailfast:1928-1948) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the AC error paths — truncated header/chunk, declared-size > actual, zero data size, unaligned chunk skip, unknown-format tag (0xFFFE without extensible body), unsupported bit depth" | tests.rs:1-7 (module header) | REAL | each listed path pinned in part1/part2 (this chunk pins the first three; the rest are part1's, A04) |
| "Both sentinel decisions are damage-noted (0-byte notes: the decision itself, not a byte range)" | tests.rs:835-836 | REAL | the full event vector at 839-853 proves the notes surface as events (unlike ogg's continued-lie note — cross-ref my ogg/tests.rs audit F-002) |

## 7. Dead / duplicated / unreachable code
- `let _ = &mut b;` (514) — fake-use residue (F-001).
- `missing_fmt_and_data_are_typed`'s second case duplicates `zero_data_size_is_typed_no_data`'s code pin with a different builder — acceptable (honest-RIFF vs patched-RIFF shapes).

## 8. Dependencies & flags
- cfg(test); no feature gates, no unsafe, no ignores; uses pub accessors (`info`, `damage_total`, `pool_bytes`, `format`, `fmt_chunk_bytes`, `fact_sample_length`, `seekability`) — same observability-surface point as ogg: what is not exposed cannot be asserted.

## 9. Open questions for the next agent
- Cross-format drift: past-end seek is REFUSED by wav (692) but CLAMPED-to-last-frame by ogg (ogg/tests.rs:608-626) — engine-level UX inconsistency worth one owner's decision (each is individually documented).
- A04 (part1): does any part1 test pin `bad_format_params`'s code? (Repo-wide grep says no — it is documented-unreachable residue; fine, but the residue pattern deserves one consolidated S4 somewhere.)
