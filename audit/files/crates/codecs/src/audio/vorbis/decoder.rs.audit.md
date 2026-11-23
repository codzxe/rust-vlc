# AUDIT: crates/codecs/src/audio/vorbis/decoder.rs
| Field | Value |
|---|---|
| Auditor | W07-A03 |
| Date | 2026-11-24 |
| Lines / functions | 1139 / 17 (incl. 3 unit tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:271-275 `"vorbis" => self.seeded(track, config, VorbisDecoder::new)` → playback-engine/src/pipeline.rs:2362 `limb.decoder.decode(...)`; web lane apps/web/src/host.rs:639-641 behind `codec-vorbis` (default-on) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 2 |
| Commands run | `target/debug/deps/vorbis_bitexact-e8223d5cf8b5f437` → 5 passed exit 0; `target/debug/deps/codecs-bae0f1756cc0a809 vorbis` → 21 passed exit 0; `target/debug/deps/vorbis_fuzz_survival-d8fc9b5caa4a5ec1` → 1 passed; `target/debug/deps/vorbis_vlc_differential-3a9245a4339ccea0` → 1 passed (VLC half skip, honest); sed/rg/od verifications |

## 1. One-paragraph verdict
REAL. The hostile-focus question answers cleanly: this is a **symphonia wrapper, not hand-rolled** — `symphonia_codec_vorbis::VorbisDecoder as Engine` (line 107) does the entire entropy+DSP decode (`engine.decode_ref` at 274); the crate pins `symphonia-codec-vorbis = "=0.6.1"` (crates/codecs/Cargo.toml:103, feature `codec-vorbis` line 80). What is in-tree and genuinely implemented: header-chain validation handoff, the granule-domain sample clock (shift measurement, granule tail clamp), the stream-head hold + amortized drain (issue #38), pooled slot rotation, interleaving to native-endian F32, and the typed error taxonomy. The hand-rolled parts are nontrivial (~700 lines of timeline logic) and are exercised end-to-end by the bitexact suite, which passes. The one real defect: `decode` systematically returns `DecodeOutcome::Emitted { frames: 0 }` while the head hold is armed — a direct violation of the codec-api contract, whitewashed by the test harness's comment instead of fixed.

## 2. What actually works (verified)
- `target/debug/deps/vorbis_bitexact-e8223d5cf8b5f437` → `5 passed; 0 failed` incl. `stream_head_encoder_delay_is_trimmed_decoder_side`, `mid_stream_corruption_is_typed_and_recovery_scoped`, `fixtures_are_pinned_against_the_ffmpeg_reference` — the head-trim, corruption-recovery and ffmpeg-differential claims all execute and pass.
- `target/debug/deps/codecs-bae0f1756cc0a809 vorbis` → `21 passed` (headers/seek/decoder/fuzz/comments unit tests).
- `target/debug/deps/vorbis_fuzz_survival-d8fc9b5caa4a5ec1` → `mutated_and_truncated_inputs_never_panic ... ok` (the "never a panic" claim, on the fuzz corpus).
- Pool semantics verified against media-types/src/packet.rs:170-182 (`slice` asserts containment) and 213-216 (`get_mut` needs exclusive Arc) — the byte-range math in release/drain provably stays in bounds because `decode_block_into` guarantees `emitted*channels*4 <= bytes.len()` (typed error otherwise, 296-302).

## 3. Findings
F-001 [S2]
Where: crates/codecs/src/audio/vorbis/decoder.rs:906-907 and 926-927
Code:
```rust
            self.packet_index += 1;
            let frames = u32::try_from(drained_frames).unwrap_or(u32::MAX);
            return Ok(DecodeOutcome::Emitted { frames });
```
What is wrong: while the head hold is armed (every packet until the first stamped page — i.e. the head of EVERY stream), `drained_frames` is 0, so `decode` returns `Emitted { frames: 0 }`. codec-api/src/decode.rs:126 defines the contract: "`frames >= 1`; nothing to emit is `NeedMoreInput`", and codec-api/src/contract.rs:293-296 makes it a hard violation: "Emitted { frames: 0 } lies: nothing emitted is NeedMoreInput". The same violation fires on pure-padding tail packets (926-927). Why it matters: the outcome is a wire-level contract surface; any consumer applying `decode_outcome_agrees_with_sink` (the reusable suite of doc 01 §2.4(e)) rejects this decoder — and that suite is only ever run against `MjpegDecoder` (crates/codecs/tests/contract.rs:2-4, 18), never against VorbisDecoder, which is why it goes undetected. The in-repo harness papers over it: crates/codecs/tests/vorbis/mod.rs:234-239 accepts both outcomes with the comment "any count is the contract's `Emitted`" — contradicting codec-api. The playback pipeline currently ignores the count (pipeline.rs:2362-2376 inspects only `Recovered`), so no live stall today — that is luck, not correctness. What correct looks like: return `NeedMoreInput` when nothing was accepted this call. Confidence: HIGH — Verification: ran bitexact suite (passes because the harness whitelists), read contract sources.

F-002 [S3]
Where: crates/codecs/src/audio/vorbis/decoder.rs:669-675
Code:
```rust
    /// The granule-timeline position of the next block's first sample
    /// (`raw − shift`; the seek landing surface's observable).
    #[must_use]
    pub fn next_first_sample(&self) -> u64 {
        self.raw_next
            .saturating_sub_signed(self.shift.unwrap_or(0).max(0))
    }
```
What is wrong: the doc promises `raw − shift`, the code computes `raw − max(shift, 0)`. The module's own docs (decoder.rs:72-74) state a mid-stream seek landing yields a NEGATIVE shift ("it encodes the page's absolute offset") — exactly the case where `raw − shift` ≠ `raw − max(shift,0)`; after a mid-stream seek this observable reports the raw counter, not the granule position, off by the landing page's absolute offset. `saturating_sub_signed` already handles negative shifts safely, so the `.max(0)` clamp protects nothing — it just breaks the documented formula. Additionally the function has zero callers in the repo (rg `next_first_sample` → only definition + a seek.rs doc mention): dead public API whose only behavior contradicts its doc. Confidence: HIGH — Verification: read + rg callers.

F-003 [S3]
Where: crates/codecs/src/audio/vorbis/decoder.rs:681-689
Code:
```rust
    pub fn timeline_shift(&self) -> Option<i64> {
        self.shift
    }
```
(and `emitted_total` at 687-689 returning `self.raw_next`)
What is wrong: `timeline_shift` and `emitted_total` — the "playback layer's head-trim coordinate" and the raw-emission diagnostic — have zero callers anywhere in the workspace (rg over crates/ + apps/). The "playback layer" that supposedly consumes them does not exist; the trims are all decoder-side. Dead public API surface kept alive by doc claims. Confidence: HIGH — Verification: rg `timeline_shift|emitted_total` across the repo → only decoder.rs definitions.

F-004 [S3]
Where: crates/codecs/src/audio/vorbis/decoder.rs:90-97 (claim) vs 1009-1057 (mechanism)
Code:
```rust
//! pooled [`SharedBytes`] rotation sized to the maximum block
//! (`blocksize_long / 2 × channels × 4` bytes) once at configure —
//! steady-state decodes refill a slot in place, zero per-block
//! allocation after warm-up (the 5.A1 `pooled_sample_slot` pattern).
```
What is wrong: "zero per-block allocation after warm-up" holds only while at most 3 decoded frames are alive downstream (`SAMPLE_SLOTS = 4`, line 114; reuse requires exclusive Arc, `get_mut`, packet.rs:213-216). The head-hold's own doc (decoder.rs:129-130) cites a consumer ring of 131,072 frames ≈ 32 block-sized frame refs in flight at 4096-sample blocks. If the sink chain retains `SharedBytes` refs (does not copy), the 4-slot pool thrashes and every block takes the cold path: a fresh `vec![0; slot_len]` (up to 128 KiB, line 1032) per packet. Whether the ring retains refs is not established from this file. UNVERIFIED: sink retention behavior (needs the audio-engine ring audit). The claim is stated unconditionally; it is conditional. Confidence: MEDIUM — Verification: read; retention UNVERIFIED.

F-005 [S4]
Where: crates/codecs/src/audio/vorbis/decoder.rs:303-305
Code:
```rust
    for (cursor, value) in scratch[..need].iter().enumerate() {
        dst[cursor * 4..cursor * 4 + 4].copy_from_slice(&value.to_ne_bytes());
    }
```
What is wrong: per-sample 4-byte copies where a chunked cast-copy of `scratch[..need]` would do; correct but needlessly scalar. Negligible impact at ≤4096 samples/block — hygiene only. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/codecs/src/audio/vorbis/decoder.rs:857-858 and 911-913
Code:
```rust
                    pts_ticks: i64::try_from(raw_first).unwrap_or(i64::MAX)
                        - shift_now.unwrap_or(0),
```
What is wrong: non-saturating i64 subtraction for pts, while the sibling paths use `.saturating_sub(shift)` (487-491, 574-580). Provably safe on the normal path (pts reduces to the previous stamp's ticks because `raw_first == raw_next` at measurement time), but a container stamp within ~130k of i64::MAX feeding a hostile granule through `measure_shift` (seek.rs:80, itself non-saturating) could overflow here (debug panic / release wrap). UNVERIFIED whether demux::ogg clamps granule→ticks. Inconsistent discipline for no benefit. Confidence: LOW on reachability, HIGH on the inconsistency — Verification: read + arithmetic reasoning.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `build_engine` | 229-258 | REAL | — | symphonia engine from validated chain; >8ch typed refusal; gapless off (ours owns the clock) |
| `decode_block_into` | 267-307 | REAL | — | engine delegation + typed error + engine.reset() poison boundary; dst-size typed check |
| `decode_block` | 318-338 | REAL | — | free surface, fresh engine per call (documented cold-start); fuzz entry |
| `max_output_len` | 343-345 | REAL | — | bounded: blocksize validated 2^6..2^13 (headers.rs:88) → ≤128 KiB/slot |
| `slot_samples` | 351-353 | REAL | — | trivial |
| `VorbisDecoder::new` | 429-431 | REAL | — | Default |
| `release_head_hold` | 448-517 | REAL | — | un-trimmed/give-up release; byte-range math verified in-bounds |
| `drain_head_hold` | 530-646 | REAL | — | amortized release + FIFO pending; budget math consistent |
| `accepts` | 650-652 | REAL | — | tested |
| `output_format` | 657-667 | REAL-UNPROVEN | — | trivial map; no direct test found |
| `next_first_sample` | 672-675 | FALSE-doc/dead | S3 | F-002 |
| `timeline_shift` | 681-683 | REAL/dead | S3 | F-003 |
| `emitted_total` | 687-689 | REAL/dead | S3 | F-003 |
| `Decoder::configure` | 693-736 | REAL | — | typed refusals; re-arms hold; resets all state |
| `Decoder::decode` | 738-955 | REAL | S2 | granule clamp + head hold correct; F-001 Emitted{0} |
| `Decoder::flush` | 957-993 | REAL | — | two-shape hold release per docs; `Drained` immediately |
| `pooled_sample_slot` | 1009-1057 | REAL | S3 | mechanism real (F-004 claim conditional) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| `descriptor_gate_accepts_only_vorbis` (1064) | YES | pure predicate |
| `decode_before_configure_is_typed_bad_state` (1073) | YES | error code asserted |
| `configure_rejects_foreign_descriptors_and_short_chains` (1104) | YES | two typed codes asserted |
| bitexact suite (ran): `stream_head_encoder_delay_is_trimmed_decoder_side` | YES | pins the head-trim against ffmpeg reference data |
| bitexact: `mid_stream_corruption_is_typed_and_recovery_scoped` | YES | typed Corrupt + recovery emission count |
| `vorbis_fuzz_survival::mutated_and_truncated_inputs_never_panic` (ran) | YES | mutations over real fixtures |
| In-file coverage gap | — | the ~500 lines of hold/drain/clamp logic have NO in-file unit tests; only the external bitexact suite touches them (it does, and passes) |
| tests/vorbis/mod.rs:234-243 harness | NO for F-001 | explicitly whitelists `Emitted{0}` — the harness adapts to the bug, not the contract |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "symphonia-codec-vorbis 0.6.1, exact-pinned … engine owns entropy+DSP of one packet; header chain, comments, granule arithmetic, error taxonomy are ours" | decoder.rs:7-19 | REAL | import line 107; Cargo.toml:103 `symphonia-codec-vorbis = { version = "=0.6.1", optional = true }`; decode_ref at 274; bitexact run |
| "each packet emits (prev_n + n)/4 inter-channel samples" | decoder.rs:27-28 | REAL (delegated) | actual accounting uses engine `frames()`; the formula lives in seek.rs `join_emission` (unit-tested, ran) |
| "after a typed Corrupt the engine's overlap state is reset … never a panic" | decoder.rs:29-32 | REAL | engine.reset() at 280; fuzz survival test passed |
| "steady-state … zero per-block allocation after warm-up" | decoder.rs:96-97 | REAL-UNPROVEN | F-004: conditional on ≤3 in-flight refs; UNVERIFIED sink retention |
| "differential suite pins OUR output against ffmpeg's" | decoder.rs:84-88 | REAL | `fixtures_are_pinned_against_the_ffmpeg_reference` passed (canonicalized −0.0 MD5, honestly documented in tests/vorbis/mod.rs:328-340) |
| "any count is the contract's Emitted" (harness comment) | tests/vorbis/mod.rs:237-238 | FALSE | codec-api decode.rs:126 + contract.rs:293-296 say frames must be ≥1 |
| doctest at 366-388 | decoder.rs | REAL-UNPROVEN | not executed this session (lib-test binary ran 21 unit tests; doctests not run — cargo-build-budget) |

## 7. Dead / duplicated / unreachable code
`next_first_sample`, `timeline_shift`, `emitted_total` — zero in-tree callers (F-002/F-003). Everything else is reachable via the CLI/web decoder dispatch. Note: engine-info's decoder list is a hard-coded 2-entry vec (apps/cli/src/host.rs:1643-1658: mjpeg, pcm_s16le) while the real dispatch lands 13+ decoders — the info command understates; the "landed:" error string (pipeline_host.rs:376-377) matches reality. (Cross-reference for whichever agent owns apps/cli/src/host.rs.)

## 8. Dependencies & flags
`symphonia-codec-vorbis =0.6.1` + `symphonia-core =0.6.1` behind `codec-vorbis` (default-on, Cargo.toml:47-59, 80). Release lane (`cargo build --release --locked -p lumen-cli -p desktop`, baseline: clean) includes it. No unsafe in this file.

## 9. Open questions for the next agent
- Does the audio sink chain retain `SharedBytes` refs beyond 4 in flight (F-004 pool-thrash)? Audit the LimbSink/ring retention in playback-engine + audio-engine.
- Does demux::ogg clamp Ogg granule → i64 ticks (F-006 overflow reachability)?
- vorbis/fuzz.rs, headers.rs, comments.rs, error.rs, seek.rs are separate assignments (other agents) — seek.rs:80 `measure_shift` uses the same non-saturating subtraction noted in F-006.
