# AUDIT: crates/codecs/src/audio/adpcm/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A02 |
| Date | 2026-11-23 |
| Lines / functions | 1386 / 16 production fns + 12 tests |
| File verdict | REAL (bit-exact against ffmpeg 7.1.5 on executed evidence; two envelope caveats below) |
| Reachable from shipped entry point? | YES for `adpcm_ms`/`adpcm_ima_wav`: apps/cli/src/pipeline_host.rs:298-302 `codec if codec.starts_with("adpcm_")` → `AdpcmDecoder::new` (HostDecoderProvider → playback-engine DecodeStage); apps/web/src/host.rs:614 same arm behind `builtin-codecs`. The 5 breadth flavors: no shipped demuxer emits their descriptors (see F-002). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `sed -n` quote checks (all cited lines); `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::adpcm::tests` → 12 passed; `... audio::adpcm::swf audio::adpcm::ima` → 13 passed; `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` → 4 passed; `./target/debug/deps/adpcm_corpus_differential-b20f58c6c52dee82` → 2 passed (real-corpus rows SKIP: /tmp corpus absent); ffmpeg 7.1.5 oracle runs (below); python3 ports of ima/ms/swf algorithms (below) |

## 1. One-paragraph verdict
This is a genuinely implemented seven-flavor ADPCM dispatcher, not a stub: the gate, both facts paths (WAVEFORMATEX re-parse + tag-0 raw skeleton), the coefficient-table intake, the damage ledger → `DecodeOutcome` mapping, and the pooled-slot rotation are all real code, and I executed the real compiled decoder (prebuilt test binaries) plus independent ffmpeg 7.1.5 oracle runs. The IMA/MS/SWF step and coefficient tables and the predictor arithmetic verify bit-exactly against ffmpeg on committed fixtures AND fresh ffmpeg encodes (mono+stereo each). The defects are envelope honesty, not decode math: the IMA-WAV channel gate accepts 3..=64 channels where the claimed oracle refuses to decode at all, and the five breadth flavors have no shipped demuxer that can ever reach them.

## 2. What actually works (verified)
- Unit tests: `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::adpcm::tests` → `test result: ok. 12 passed` (exit 0).
- Differential: `adpcm_breadth_differential` → 4 passed; `adpcm_corpus_differential` → 2 passed (fixtures; real-corpus rows skip loudly — /tmp/my-project/corpus-v4/media absent).
- Oracle authenticity (I re-measured with sandbox ffmpeg 7.1.5-0+deb13u1 — the exact version the pins claim):
  - `ffmpeg -i fixtures/adpcm_ima_mono.wav -f md5 -` → `MD5=8a8146087f7573fa71963c3ae2c65f3b` (= pinned, mod.rs truth table cited in adpcm_corpus_differential.rs:69).
  - `ffmpeg -i fixtures/adpcm_ms_mono.wav -f md5 -` → `MD5=a95d2cbb66f65a4adcb36b57bc68e48e` (= pin).
  - Committed `swf_payload.bin` re-wrapped as FLV audio tags → `MD5=9aac930510b6ef482b7303db62a7b200` (= pin).
  - Forced-decoder runs of `oki_mono.vox`/`oki_st.vox`/`ws_mono.bin`/`ws_st.bin`/`yamaha_mono_payload.bin`/`yamaha_st_payload.bin` → all six MD5s equal the pinned values.
- Independent algorithm re-derivation (python3 ports of ima.rs/ms.rs/swf.rs semantics): committed IMA mono → 12_246 frames, MD5 match; fresh ffmpeg stereo IMA encode → MD5 match; fresh stereo MS encode → MD5 match; fresh stereo SWF encode → MD5 match; committed SWF payload → MD5 match. The multiply-then-shift IMA form (ima.rs:85-89) and the partial-sum SWF form are both what ffmpeg 7.1.5 actually does — verified, not taken from docs.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/adpcm/mod.rs:320-327 (and the IMA arm at 437-461 with no channel cap)
Code:
```
        if channels == 0 || u32::from(channels) > 64 {
            // The demuxer's MAX_CHANNELS ceiling (parity keeps the
            // accepted set identical).
            return Err(unsupported(
                "adpcm.waveformat.channels",
                format!("channel count {channels} is outside 1..=64"),
            ));
        }
```
What is wrong: the IMA-WAV plan accepts 3..=64 channels, but the claimed oracle (ffmpeg 7.1.5) refuses to decode IMA WAV with >2 channels (`ffmpeg -i <hand-built 4ch ima wav>` → `[adpcm_ima_wav ...] Invalid number of channels`; its encoder supports only mono/stereo). The module doc's "bit-exactly against the ffmpeg 7.1 reference" envelope is silently 1..=2 channels; every accepted channel count above that is unverified against any reference. The demuxer (crates/demux/src/wav/format.rs:41 MAX_CHANNELS=64) will happily deliver such files, so the path is reachable.
Why it matters: a 3+ channel IMA WAV decodes through generalized-but-unpinned group logic; if the real-world format (or a future ffmpeg) disagrees, output is wrong with no test that would catch it.
What correct looks like: either cap IMA-WAV at 2 channels (typed Unsupported beyond, like the MS arm at 358-368 does) or document + pin the multichannel group form against an independent reference.
Confidence: HIGH — Verification: ran (ffmpeg refusal reproduced; python port of the 4ch decode produces output no oracle will confirm).

F-002 [S3]
Where: crates/codecs/src/audio/adpcm/mod.rs:21-28 (doc table), 509-523 (plan build); cross-ref apps/cli/src/pipeline_host.rs:298, apps/web/src/host.rs:614
Code:
```
                match descriptor {
                    "adpcm_yamaha" => Plan::Yamaha { channels, rate },
                    "adpcm_ima_oki" => Plan::ImaOki { channels, rate },
                    "adpcm_swf" => Plan::Swf { channels, rate },
                    "adpcm_ima_qt" => Plan::ImaQt { channels, rate },
```
What is wrong: no demuxer in the tree emits `adpcm_yamaha`/`adpcm_ima_oki`/`adpcm_swf`/`adpcm_ima_qt`/`adpcm_ima_ws` (rg over crates/demux/src: zero matches; only WAV/ASF emit `adpcm_ms`/`adpcm_ima_wav`). From `lumen open` (CLI/desktop/web), the five breadth flavors can never be constructed — they are reachable only if a plugin/external demuxer supplies the descriptor. The module docs describe them as landed breadth rows without stating that no shipped container path exists.
Why it matters: five of the seven advertised flavors are dead code from every shipped entry point today; the "adpcm" name in the CLI's landed-list error string (pipeline_host.rs:377) overstates what a user can actually open.
What correct looks like: a one-line doc/matrix caveat ("breadth flavors await their demux cards; reachable via the plugin seam"), or the matrix rows marked not-yet-reachable.
Confidence: HIGH — Verification: ran rg across crates/demux/src and apps/ (only adpcm_ms/adpcm_ima_wav emitters).

F-003 [S4]
Where: crates/codecs/src/audio/adpcm/mod.rs:1357-1366 (and the raw twin at 1376-1384)
Code:
```
        for i in 0..OUT_SLOTS * 3 {
            let outcome =
                match decoder.decode(packet(generation, block.clone()), &mut Capture::default()) {
                    Ok(outcome) => outcome,
                    Err(err) => unreachable!("decode {i}: {err}"),
                };
            assert_eq!(outcome, DecodeOutcome::Emitted { frames: 1 }, "packet {i}");
        }
        // The rotation stays bounded.
        assert!(decoder.slots.len() <= OUT_SLOTS);
```
What is wrong: the test name says "pooled rotation is reused across packets" but the body only asserts `slots.len() <= OUT_SLOTS`, which the cold-path eviction (mod.rs:802-809) guarantees even if every packet allocates a fresh slot. Mutation reasoning: replace `pooled_slot` with always-allocate — this test still passes. The reuse claim is unverified by this test.
What correct looks like: an allocation counter (test-only hook) or asserting a slot's identity/contents persist across decodes.
Confidence: HIGH — Verification: read (mutation reasoning only; no edit made).

F-004 [S4]
Where: crates/codecs/src/audio/adpcm/mod.rs:663-667
Code:
```
        if stats.skipped > 0 {
            Ok(DecodeOutcome::Recovered {
                skipped: stats.skipped as u32,
```
What is wrong: `u64 as u32` truncation; only reachable past 2^32 skipped blocks in one packet, which a bounded packet cannot carry. Hygiene only.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Plan::channels | 153-164 | REAL | — | straight match |
| Plan::rate | 166-175 | REAL | — | straight match |
| Plan::output_format | 177-185 | REAL | — | S16 interleaved native-endian; verified by test asserts |
| AdpcmDecoder::new | 279-281 | REAL | — | Default |
| AdpcmDecoder::accepts | 288-300 | REAL | — | 8 descriptors incl. refused `adpcm_oki`; executed in tests |
| parse_waveformat | 305-341 | REAL | S3 (F-001) | offsets verified (tag 0, ch 2, rate 4, block_align 12); 64-ch ceiling exceeds oracle envelope |
| configure_wav | 346-464 | REAL | — | MS coef intake at 20/22 verified vs WAVEFORMATEX layout; coef equality gate executed; `header+1` block-align floors correct (MS 7·ch, IMA 4·ch) |
| Decoder::configure | 467-537 | REAL | — | gate + typed refusals executed (12 tests); state reset per configure |
| Decoder::decode | 539-676 | REAL | — | bound math sound (saturating, covers ceil-block count + seeds); outcome mapping executed; u64→u32 cast (F-004) |
| Decoder::flush | 678-683 | REAL | — | Drained; no buffered data by construction |
| parse_raw_facts | 698-737 | REAL | — | tag-0 skeleton; ch 1..=2, rate≠0; executed |
| wav_out_bound | 743-751 | REAL | — | over-covers ceil(payload/ba) blocks × (4·ba+4) + 2·ch·blocks; saturating |
| raw_out_bound | 759-761 | REAL | — | 8×payload+64 ≥ SWF width-2 stereo worst case (verified by fuzz-lite bound assert, executed) |
| duration_nanos | 765-770 | REAL | — | u128 intermediate; rate=0 guarded |
| pooled_slot | 776-811 | REAL | — | Arc-ownership check via get_mut; rotation indices verified for all cold-path counts; eviction drops only the Vec handle (safe) |
| AdpcmError::into_engine_error / unsupported / Detail | 813-880 | REAL | — | typed stage-tagged errors; executed via refusal tests |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| gate_accepts_exactly_the_seven_flavors | YES | asserts 8 in / 4 out + a typed error code |
| lettered_oki_is_the_honest_typed_gap | YES | asserts code `adpcm.oki.no_reference` + Unsupported kind |
| missing_facts_and_zero_block_align_are_typed_unsupported | YES | two distinct typed codes |
| missing_facts_and_skeleton_damage_are_typed_unsupported | YES | five distinct damage codes (tag, channels, rate, short, descriptor) |
| tag_mismatch_and_coef_damage_are_typed_unsupported | YES | IMA/MS tag swap + wNumCoef 6 + non-standard pair |
| unconfigured_decode_is_typed_bad_state | YES | asserts `adpcm.not_configured` |
| ima_packet_decodes_and_recovers_typed | YES | exact sample count 33, Emitted vs Recovered vs stale drop |
| ms_packet_decodes_stereo_interleaved | YES | exact byte count + seed order [50,-50,100,-100] |
| yamaha_packets_decode_with_persistent_state | YES | 3×32 samples + assert_ne cross-packet state |
| swf_and_qt_packet_damage_recovers_typed | YES | skip outcomes + QT 34-byte → 64 samples |
| wav_pooled_rotation_is_reused_across_packets | NO (weak) | see F-003 — boundedness only, not reuse |
| raw_pooled_rotation_is_reused_across_packets | NO (weak) | same shape as F-003 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "seven flavors behind one AdpcmDecoder" | mod.rs:3-4 | REAL | 7 implemented + `adpcm_oki` refused (line 481-492, executed) |
| WAV flavors: "bit-exactly against the ffmpeg 7.1 reference on the real corpus" | mod.rs:32-37 | REAL (ch≤2) / over-broad (ch>2, F-001) | my ffmpeg runs + python ports match pins; corpus rows SKIP in sandbox (corpus dir absent) — fixture rows executed |
| breadth five: "bit-exact against ffmpeg" (via tests table) | mod.rs:23-27 | REAL | breadth differential executed (4 passed); I re-measured 7/8 pins as genuine ffmpeg outputs (qt rewrap UNVERIFIED — needs a MOV carrier) |
| "configure without facts is a typed Unsupported (adpcm.facts.missing)" | mod.rs:67-68 | REAL | executed refusal test |
| "A nonzero tag on a raw flavor is refused (adpcm.raw.facts.tag)" | mod.rs:69-71 | REAL | executed refusal test |
| "steady-state decode is allocation-free per packet" | mod.rs:115-118 | REAL-UNPROVEN | rotation code is real; no test counts allocations (F-003); zero_alloc harness covers codecs elsewhere (not run here) |
| "≤ 4 output bytes per input byte for the WAV block flavors" | mod.rs:560-562 | REAL (as a bound) | wav_out_bound adds per-block slack above 4×; the literal "≤4×" claim is loose at small block_align but allocation-safe |

## 7. Dead / duplicated / unreachable code
- Five breadth flavors: unreachable from shipped demuxers (F-002) — real code, plugin-seam-only.
- No dead functions found in this file; all 16 production fns are called (accepts/parse/configure_wav from configure; bounds/pooled_slot from decode).

## 8. Dependencies & flags
- std-only + codec_api/media_types/testkit (dev). No new deps (the D-018 claim holds — no external adpcm crate).
- `pub mod fuzz` is always compiled (fuzz.rs is W07-A10's file); `pub(crate)` submodules.

## 9. Open questions for the next agent
- W07-A03 (ms.rs): confirm the /256 trunc-division form and ADAPTATION table — my python port matched ffmpeg on committed+fresh encodes, so focus on damage paths.
- W07-A09 (yamaha.rs) / W07-A06 (oki.rs): pinned MD5s verified genuine via forced-decoder runs — cite `ffmpeg -c:a adpcm_yamaha -f mulaw -ar 22050 -ac 1 -i yamaha_mono_payload.bin -f md5 -` → aeddc262b3dd010955f9c73c6cfc86ad.
- qt_payload.bin pin: I could not re-create ffmpeg's MOV-carried decode (forced single-packet decode errors "invalid number of samples in packet" — expected). UNVERIFIED directly; relies on baseline + breadth test pass.
- Who owns the "landed: ... adpcm ..." error string at apps/cli/src/pipeline_host.rs:377 vs the baseline's "engine-info: decoders: mjpeg, pcm_s16le"? That mismatch deserves a UI/docs agent's attention.
