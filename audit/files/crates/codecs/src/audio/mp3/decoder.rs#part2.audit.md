# AUDIT: crates/codecs/src/audio/mp3/decoder.rs#part2 (lines 429-828)
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-21 |
| Lines / functions | 400 (of 828; chunk part2 of 2 — starts at `impl Mp3Decoder`, line 429; part1 = 1-428, same agent) / 10 units: `Mp3Decoder::{new, accepts, engine_codec}`, `Decoder::{configure, decode, flush}`, `pooled_sample_slot`, tests (2 helpers + 7 tests) |
| File verdict | SHALLOW for the shipped lane (the contract path runs — CLI lanes green — but mis-trims Layer I/II streams and stamps a 529-sample hole into the pts timeline; its value-level behavior is asserted by no test) |
| Reachable from shipped entry point? | YES. `lumen open x.mp3|x.mp2` → `demux/src/es/mp3.rs` → descriptor `mp3`/`mp2` → `apps/cli/src/pipeline_host.rs:261-268` → `Mp3Decoder::new()` + `configure` + `decode`/`flush` (this range). Also MP3-in-MP4 (esds OTI 0x6b), MP3/MP2-in-TS (PIDs 0x03/0x04), MKV A_MPEG/L2/L3, and apps/web/src/host.rs. Executed: `es_mp3_family_lane ... ok`, `es_mp2_family_lane ... ok` (orchrun-test-workspace-nofailfast.log.txt:2943, :2793; apps/cli/tests/container_conformance.rs:700-744); 7 unit tests ok (baseline :409-415); fuzz contract loop (mp3/fuzz.rs:142-179) exercised for panic-freedom. |
| Findings | S0:0 S1:0 S2:2 S3:1 S4:2 |
| Commands run | Read 429-828 (full) + 1-428 (part1 context); engine-contract verification against symphonia-bundle-mp3-0.6.1 source (registry cache); arithmetic re-derivation of trim/pts timelines (worked the frame-0/frame-1 sequence by hand, twice); grep for `facts::encode` producers (zero — corroborated by sibling xing.rs.audit.md F-002); baseline greps (:409-415, orchrun :2793/:2943, container_conformance.rs:700-744 read in full); sed verification of every quoted range. |

## 1. One-paragraph verdict
This is the half the shipped binary actually runs, and it is the weakest audio file I have audited: `configure` unconditionally adds the 529-sample Layer III synthesis latency to the start trim for EVERY descriptor — including `mp2` and `mp1`, whose output the module's own doc (line 59-60) and the project's own MP2 differential (mp2_48k: 72576 = 63×1152 samples, no shift) prove aligns at shift 0 — so every shipped MP2/MP1 playback chops ~11 ms of real audio off the stream head. The pts arithmetic then mixes two timelines: the first emitted frame is stamped on the LAME/ffmpeg timeline (`delay`) while every later frame is stamped with the RAW frame start (`samples_fed`, no −529/−delay compensation), manufacturing a 529-sample hole between frame 0's span and frame 1's pts — the engine consumes exactly these pts (pipeline.rs `first_pts_ns` anchoring, held-frame sorting). And none of it is observable by any test: the differentials pin `decode_stream` (part1), the fuzz loop counts frames without checking values, and the CLI lanes assert wall-clock time that my hand-computation shows is computed from the same buggy raw-count bookkeeping (1.512 s = 63×1152/48000 exactly), so the defects are pinned in place by tests that cannot see them.

## 2. What actually works (verified)
— Typed refusal surface: foreign descriptor (`mp3.codec_descriptor`), malformed facts blob (`mp3.facts.length`), decode/flush before configure (`mp3.not_configured`), stale-generation drop (515-517), empty packet (518-520), per-frame isolation after a typed `Corrupt` (error return skips `packet_index += 1`, next packet unaffected). All pinned by unit tests (baseline :409-412) except the stale drop (see F-003).
— Start-trim spanning (533-537): `pending_start_trim` decrements correctly across packet boundaries; `emit.get(drop*channels..)` is bounds-safe.
— End-trim arithmetic (539-550): `padding − 529` clamped to the frame, `saturating_sub` — identical to `decode_stream`'s whole-stream window end (`total − padding + 529`), re-derived by me and pinned indirectly by part1's `gapless_window_*` tests (same math, different surface).
— `pooled_sample_slot` (616-654): Arc-correct rotation (get_mut on shared slot refused), fill-in-place closure (no intermediate Vec — unlike adpcm), bound `SAMPLE_SLOTS`, `slot_len = 1152·ch·4` proven ≥ any frame's output (L1 384, L2 1152, L3 1152|576 per channel); the cold-path `None` arm returns a typed error instead of the adpcm sibling's dead `0` — better.
— Execution: the whole contract path runs in the shipped binary through `es_mp3_family_lane`/`es_mp2_family_lane` (orchrun :2943/:2793 ok) and the fuzz contract loop; no panics in any baseline run.

## 3. Findings
### F-001 [S2]
Where: crates/codecs/src/audio/mp3/decoder.rs:493 (misapplication) vs module doc 59-60 and tests/data/mp3_bitexact_reference.txt:81-87
Code:
```rust
        self.pending_start_trim = delay.saturating_add(LAYER3_PIPELINE_LATENCY as u32);
```
against the module's own statement:
```rust
//! to ±1 LSB once aligned (Layer II/MP2 has no such offset — its
//! output aligns at shift 0).
```
and the project's own MP2 differential row (`[fixture.mp2_48k] samples_per_channel = 72576` = 63 × 1152 — the FULL raw frame output, un-shifted, matching ffmpeg's MD5).
What is wrong: `LAYER3_PIPELINE_LATENCY` (529) is a Layer-III-only property, but `configure` adds it to the start trim for every descriptor this decoder accepts — `mp2` and `mp1` included (the descriptor is already in hand at line 452; `engine_codec` at 441-447 distinguishes them). An MP2 stream's first 529 genuine samples (≈11.0 ms at 48 kHz) are discarded as if they were Layer III priming.
Why it matters: every shipped MP2/MP1 playback (`lumen open *.mp2`, MP2-in-TS broadcast lanes, MKV A_MPEG/L2) starts 529 samples late — an audible clip at the stream head and a 529-sample content/timeline mismatch. The project's own evidence chain (module doc + mp2 differential proving shift 0) contradicts the code; the two decode surfaces also disagree with each other (`decode_stream` applies NO trim to MP2 — correct — while the contract path chops 529).
What correct looks like: `let latency = if descriptor == "mp3" { LAYER3_PIPELINE_LATENCY } else { 0 };` (and the same conditional inside the end-trim's `saturating_sub` at 545-547, which currently under-trims L1/L2 by the symmetric error — for MP2 the padding should drop in full, not `padding − 529`), plus an mp2 lane assertion on the first frame's sample count. **Measured in-session (audit/runs/tmp-w07a04/EVIDENCE.md, ffmpeg 7.1.5): for a TAGLESS mp3, ffmpeg's decode is exactly frames×1152 samples with the source's sample 0 at output index 1105 (= 576+529 delay head retained) — the reference applies NO head skip without a LAME tag. Since no demuxer produces the facts blob (xing.rs.audit.md F-002), every shipped `*.mp3` playback is tagless to this decoder: the unconditional 529 trim misaligns EVERY shipped MP3 output by 529 samples (≈12 ms @44.1 kHz), not just mp2/mp1.**
Confidence: HIGH — Verification: read + arithmetic re-derivation + the repo's own differential row + in-session ffmpeg measurement (tmp-w07a04: tagless.mp3, 78 frames → 359,424-byte decode; correlation shift 1105; MP2 twin shift ≈481 with the repo differential proving relative shift 0).

### F-002 [S2]
Where: crates/codecs/src/audio/mp3/decoder.rs:580-589
Code:
```rust
                pts: if self.samples_fed < u64::from(self.delay) {
                    // Still inside the delay window: the first emitted
                    // sample's true index is the delay boundary.
                    Stamp::Present(Timestamp::new(i64::from(self.delay), base))
                } else {
                    Stamp::Present(Timestamp::new(
                        i64::try_from(self.samples_fed).unwrap_or(i64::MAX),
                        base,
                    ))
                },
```
What is wrong: the two branches are on different timelines. `samples_fed` counts RAW per-channel samples fed to the engine (incremented by the full `frame_samples` at 601, before/independent of any trimming), while the emitted content is the raw stream minus the head trim (`delay + 529`, per 493 and 533-537). The if-branch stamps the first emitted frame at the LAME/ffmpeg-timeline index `delay`; the else-branch stamps every later frame at its RAW timeline index with no `−529` (and no `−delay`) compensation. Worked by hand for the no-facts case (delay=0): frame 0 emits 623 samples [0,623) and is stamped 0 (correct by coincidence); frame 1's content occupies output positions [623,1775) but is stamped 1152 — a 529-sample hole between frame 0's pts+duration and frame 1's pts, and every subsequent frame is 529 samples (≈12 ms @44.1 kHz) later than its content's true position. With facts (delay=1105): frame 0 stamped 1105 (correct), frame 1 stamped 1152 instead of 623 — same hole. The container's own `packet.pts` is discarded entirely (never read).
Why it matters: the playback engine consumes these pts for real (`pipeline.rs:887-930` first-frame pts anchoring, `:1574` held-frame sorting by pts, audio-switch anchoring at `:1633-1650`) — a 529-sample discontinuity in the audio timeline and a constant ~12 ms skew against any video track in the same container (MP3-in-MP4/MKV/TS all reach this code).
What correct looks like: track output-timeline position (e.g. `samples_emitted` accumulated from `emitted_samples`) and stamp `pts = samples_emitted_before_this_frame` — or subtract the applied head trim once from `samples_fed` — and preserve `packet.pts` as the anchor when the container supplies one.
Confidence: HIGH — Verification: read + hand-simulated both the delay=0 and delay=1105 sequences; corroborated by the CLI lane's own assertion (`ok: time 1.512000s` = 63×1152/48000, the RAW count — the 529 trimmed samples do not shorten the reported time, exactly what this bookkeeping produces).

### F-003 [S3]
Where: crates/codecs/src/audio/mp3/decoder.rs:502-603 (whole `decode` body) — test blind spot
Code:
```rust
        let (frame_samples, _header) =
            decode_one(engine, packet.data.as_slice(), &mut self.scratch)?;
```
What is wrong: no test anywhere asserts the VALUES this function produces. The unit tests (656-828) cover the descriptor gate, facts round-trip, BadState and configure refusals, and `decode_stream`'s window (part1's lane); the fuzz harness drives `decode` over up to 64 real frames but its sink only counts (`CountingSink`, fuzz.rs:158-178, `outcome.contract_frames`); the CLI lanes assert wall-time strings. Trimmed sample counts, pts, duration, and the pool's output bytes of the CONTRACT path are asserted by nothing — which is precisely why F-001 and F-002 (and the tagless-MP3 surface disagreement noted in part1 §9) survive a green test suite. The stale-generation drop (515-517) is likewise executed by no test (same gap I filed for PCM).
Why it matters: the shipped playback surface is the least-tested surface in the mp3 module; every differential MD5 pins a DIFFERENT function (`decode_stream`).
What correct looks like: one unit test decoding 2-3 synthetic frames through `Decoder::decode` with a capturing sink, asserting emitted sample counts, pts sequence continuity (pts[i]+duration[i] == pts[i+1]), and format; one stale-generation case.
Confidence: HIGH — Verification: read + exhaustive caller/test grep (unit, fuzz, integration, CLI lanes).

### F-004 [S4]
Where: crates/codecs/src/audio/mp3/decoder.rs:539-549
Code:
```rust
        let is_last = self.total_frames > 0 && self.packet_index + 1 == self.total_frames;
```
What is wrong: (a) the end trim is applied only to the LAST declared frame, but the LAME padding field is 12 bits (≤ 4095), so `padding − 529` can exceed one frame (1152) — the contract path then leaves up to `padding − 529 − 1152` extra tail samples that `decode_stream`'s whole-window slice removes; (b) if the facts' `total_frames` is wrong (larger than the stream), no end trim ever fires; if smaller, a MID-stream frame is end-trimmed and later frames play on.
Why it matters: edge-case gapless inaccuracy, bounded by ≤ ~2.9k samples; today unreachable in the shipped lane because no demuxer produces the facts blob (xing.rs.audit.md F-002) — it will bite the moment the wiring lands.
What correct looks like: carry a `pending_end_trim` symmetric to the start trim (spanning packets), and treat `total_frames` claims as advisory with a walked-count cross-check like `decode_stream`'s Xing gate.
Confidence: HIGH — Verification: read + arithmetic.

### F-005 [S4]
Where: crates/codecs/src/audio/mp3/decoder.rs:605-611
Code:
```rust
        // MPEG audio keeps no cross-frame state: nothing buffered.
        Ok(DecodeOutcome::Drained)
```
What is wrong: the comment is false as stated — Layer III has the bit reservoir (a frame's `main_data_begin` reaches into previous frames' payload); the state lives in the engine (`MpaDecoder`'s `Layer3` scratch), not this struct. The CONCLUSION (nothing for this wrapper to drain) is right, but the stated reason would mislead a maintainer into thinking mid-stream seek/reconfigure needs no engine `reset()`.
Why it matters: the decoder never calls `engine.reset()` after a generation change; whether symphonia's Layer3 stale-reservoir state can leak across a seek boundary is unexamined, and the comment's rationale hides that question.
What correct looks like: "this wrapper buffers nothing; the engine's bit-reservoir state is reset by reconfigure (or: NOT reset — evaluate)", plus either a `reset()` on configure or a note that stale reservoir data is harmless (first frame after a seek references missing main_data → symphonia conceals).
Confidence: MEDIUM — Verification: read; symphonia Layer3 reservoir behavior not traced end-to-end by me.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Mp3Decoder::new` | 431-434 | REAL | — | Default; trivial |
| `Mp3Decoder::accepts` | 437-439 | REAL | — | mp1/mp2/mp3 exactly; pinned by test |
| `Mp3Decoder::engine_codec` | 441-447 | REAL | — | `_ => CODEC_ID_MP3` catch-all — safe (accepts already gated); the L1/L2 distinction exists HERE but is not used for the latency decision (F-001) |
| `Decoder::configure` | 451-500 | REAL (defective) | S2 (F-001) | typed facts gate; resets ALL bookkeeping; unconditional +529 (F-001) |
| `Decoder::decode` | 502-603 | REAL (defective) | S2 (F-002), S3 (F-003) | trim spanning correct; pts timeline hole (F-002); pool fill-in-place correct; duration floor-ns correct; `packet.pts` discarded |
| `Decoder::flush` | 605-611 | REAL | S4 (F-005) | Drained; misleading comment |
| `pooled_sample_slot` | 616-654 | REAL | — | Arc-correct; cold-path typed error; rotation phase differs from pcm/adpcm twins (advance-then-evict vs evict-then-advance) — equivalent, no bug |
| tests: `descriptor_gate_accepts_the_mpeg_audio_family` | 660-668 | REAL | — | incl. case-sensitivity (`"MP3"` refused) |
| tests: `facts_blob_round_trips_and_rejects_bad_lengths` | 670-676 | REAL | — | 11-byte and empty refusals |
| tests: `decode_before_configure_is_typed_bad_state` | 678-707 | REAL | — | code + kind asserted |
| tests: `configure_rejects_foreign_descriptors_and_bad_facts` | 709-744 | REAL | — | two codes |
| tests: `xing_first_frame` / `plain_audio_frame` (helpers) | 748-777 | REAL | — | hand-built Xing/LAME frame; 417-byte MPEG-1 L3 128k stereo shape |
| tests: `xing_frame_count_inconsistency_is_typed` | 779-791 | REAL | — | decode_stream lane; code+kind+stage |
| tests: `gapless_window_applies_the_pipeline_latency_shift` | 793-812 | REAL | — | part1's surface; 452·2 asserted (hand-verified) |
| tests: `gapless_window_clamps_when_padding_undercuts_the_latency` | 814-827 | REAL | — | 573·2 asserted (hand-verified) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| 4 unit tests (gate/facts/bad-state/refusals) | YES | exact codes/kinds on the configure surface |
| 3 stream tests (xing/gapless×2) | YES — but for `decode_stream`, NOT for this range's `Decoder::decode` | they pin part1's function; the contract path's trim/pts/pool (this range) has no value assertions (F-003) |
| fuzz contract loop (mp3/fuzz.rs:142-179) | NO (values) / YES (panics) | CountingSink counts frames only |
| CLI lanes `es_mp3_family_lane`/`es_mp2_family_lane` | NO for F-001/F-002 | they assert wall-time (1.512 s = raw count — computed FROM the buggy bookkeeping) and acceptance, not sample counts or pts |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "gapless 5.A3" / "status = implemented (probe/decode/seek/gapless both layers)" | docs/formats/audio-mp3-mp2.md:9, feature-matrix row 649 (per xing.rs.audit.md F-002) | FALSE for the shipped lane | no producer of the 12-byte facts blob exists (`facts::encode` = tests only); configure always sees delay=padding=total=0 → no LAME trim, no end trim ever fires in playback; sibling audit xing.rs F-002 recorded this — I corroborate from the decoder side (460-470) |
| "Our window therefore applies LAYER3_PIPELINE_LATENCY to both ends" | decoder.rs:61-62 (module doc) | PARTLY FALSE | true in `decode_stream` (L3-only consumers); the contract path applies the shift to L1/L2 too (F-001) and drops it from the end trim for them (under-trim) |
| "the first emitted sample's true index is the delay boundary" | decoder.rs:581-583 (comment) | FALSE for every frame after the first | F-002: later frames use raw `samples_fed` — the delay/shift compensation applies only while `samples_fed < delay` |
| "Without a `total_frames` claim the end trim is honestly not applied" | decoder.rs:376-379 (struct doc) | REAL | verified: `is_last` requires `total_frames > 0` (539) |

## 7. Dead / duplicated / unreachable code
- `self.delay`/`self.padding`/`self.total_frames` fields: live code but permanently 0 in every shipped invocation (no facts producer — xing.rs.audit.md F-002). The whole gapless contract channel is dead-in-practice.
- `Debug` skips nothing misleading; `finish_non_exhaustive` honest.
- No duplication: `decode` and `decode_stream` share `decode_one`; the trim arithmetic is duplicated between them with the L1/L2 divergence (F-001) as the one behavioral difference — the duplication is the defect vector.

## 8. Dependencies & flags
- Same as part1 (`codec-mp3` default-on). `SAMPLE_SLOTS`/`MAX_SAMPLES_PER_FRAME` defined in part1, used here; bound re-verified (1152 max across layers).
- `TimeBase::new(1, rate)` with `rate.max(1)` guard at 569 — a 0 negotiated rate is refused typed (569-571); `out_rate` comes from the engine spec (never 0 in practice).

## 9. Open questions for the next agent
- F-002's engine-side blast radius: does the playback engine's audio clock rebase on first pts (self-healing the constant offset) or schedule per-frame pts (hole audible as a 12 ms gap)? pipeline.rs owner should answer with the sink pacing path.
- If the facts-blob wiring ever lands (xing.rs F-002's fix), F-004's mid-stream `is_last` hazard becomes live — fix together.
- REQUEST-ORCH-RUN ~~(tagless-mp3 measurement)~~ RESOLVED IN-SESSION: see audit/runs/tmp-w07a04/EVIDENCE.md — ffmpeg tagless decode = frames×1152, no head skip (source sample 0 at index 1105); the contract decoder's default 529 trim misaligns every shipped tagless MP3 (all of them today), and is provably wrong for MP2/MP1.
