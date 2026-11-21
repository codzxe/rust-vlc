# AUDIT: crates/codecs/src/audio/mp2/decoder.rs
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-21 |
| Lines / functions | 678 / 20 (incl. 9 tests, 3 helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `Mp2Decoder` contract: `lumen open x.mpg` → PS demux (`crates/demux/src/ps/mod.rs:788-804` `probe_audio_codec` returns `"mp2"`) → CLI host `apps/cli/src/pipeline_host.rs:266-270` `"mp2" => Mp2Decoder::new()`; `codec-mp2` is a default feature (`crates/codecs/Cargo.toml:47-59`). `decode_stream` itself is test/fuzz/bench surface only (callers: `crates/codecs/tests/mp2_corpus_differential.rs:169`, `tests/mp2_fuzz_survival.rs:49`, `tests/mp2_bench.rs`), not the shipped lane. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -n "decode_stream\|Mp2Decoder"` (callers: CLI hosts, tests, fuzz); `rg '"mp2"' crates/demux/src` (none — PS uses `probe_audio_codec` returning `"mp2"` at ps/mod.rs:791-804); read `apps/cli/src/containers/ps.rs` (MpegAudioAssembler feeds one-frame packets to the contract decoder); `sed -n` quote verification; baseline `orchrun-test-workspace-nofailfast.log.txt:383-407,1093-1105` (all mp2 tests + corpus differential PASS). |

## 1. One-paragraph verdict
The MP2 decoder is real: the symphonia-bundle-mp3 engine is fed exactly-framed Layer II packets through a typed truncation/layer gate, output is pooled/rotated, pts arithmetic is sample-locked to the negotiated rate, and the in-repo ffmpeg differential (`tests/mp2_corpus_differential.rs::corpus_and_fixture_differential_vs_ffmpeg`) passed in the baseline run. The one substantive defect is that the contract path (`Mp2Decoder::decode`, the lane the shipped CLI actually drives) negotiates channels/rate once from the FIRST frame and never re-checks: a mid-stream geometry change — which `decode_stream`'s CBR gate refuses typed (`mp2.stream.cbr_mismatch`, header.rs:271-282) — silently mixes stale scratch data or truncates frames and keeps the wrong rate in every subsequent `AudioFrame`. Everything else is minor bookkeeping.

## 2. What actually works (verified)
- Baseline `cargo test --workspace` (orchrun-test-workspace-nofailfast.log.txt:383-407): all 7 in-file tests pass; :1093-1101 `tests/mp2_corpus_differential.rs` — 3/3 pass including `corpus_and_fixture_differential_vs_ffmpeg ... ok` (sample-locked vs ffmpeg 7.1.5 on real corpus files).
- `rg` reachability chain proven above (PS demux → CLI host → Mp2Decoder).
- Reachability of the refusal gates verified by reading `decode_one` (truncation) and `header.rs::parse_mp2_header` (Layer II gate), each with a passing baseline test asserting the exact error code (`a_truncated_frame_packet_is_a_typed_truncation`, `a_layer_iii_stream_is_refused_not_misdecoded`).

## 3. Findings

### F-001 [S2]
Where: crates/codecs/src/audio/mp2/decoder.rs:346-353
Code:
```rust
        if self.out_channels == 0 {
            let decoded = engine.last_decoded();
            let spec = decoded.spec();
            self.out_channels = spec.channels().count() as u32;
            self.out_rate = spec.rate();
        }
        let channels = self.out_channels as usize;
        let interleaved = &self.scratch[..frame_samples * channels];
```
What is wrong: `out_channels`/`out_rate` are latched from the first decoded frame and never revalidated. On a mid-stream channel change (first frame stereo, later mono): `decode_one` (lines 109-112) only grows `scratch` and writes `frames × actual_channels` fresh samples, but line 353 slices `frame_samples × stale_channels` — the tail is stale samples from the previous frame, emitted as valid audio. On the reverse direction (mono → stereo) the slice truncates half the frame. On a rate change, `out_rate` stays stale, so every later `AudioFrame.format.rate`, `pts` time base (line 372) and `duration` (lines 387-393) are wrong.
Why it matters: the shipped lane (PS demuxer → CLI assembler → this decoder) applies no cross-frame geometry gate — that gate exists only in `decode_stream` (`header.rs:261-284`, `mp2.stream.cbr_mismatch`), which the shipped path does not call. A corrupt or hostile `.mpg`/`.vob` with a mid-stream header change produces silently garbled audio and drifting timestamps instead of the typed refusal the module's own stream pipeline mandates. No panic (Layer II frames are always 1152 samples and `scratch` never shrinks), so it fails quietly.
What correct looks like: compare `spec.channels().count()`/`spec.rate()` (or the parsed header, which `decode_one` already returns and line 344 discards as `_header`) against `out_channels`/`out_rate` every packet; a disagreement is the typed `mp2.stream.cbr_mismatch`, matching `decode_stream`'s contract.
Confidence: HIGH — Verification: read; mutation reasoned: stereo-first stream with a mono frame yields 1152 fresh + 1152 stale f32s in `interleaved`.

### F-002 [S3]
Where: crates/codecs/src/audio/mp2/decoder.rs:398-399
Code:
```rust
        self.samples_fed += frame_samples as u64;
        Ok(DecodeOutcome::Emitted { frames: 1 })
```
What is wrong: when the engine decodes a packet to 0 frames (`frame_samples == 0`), the `emitted_samples > 0` guard (line 356) skips emission entirely, yet the outcome still claims `Emitted { frames: 1 }`.
Why it matters: the outcome is the engine's accounting surface (drain/EOF bookkeeping); reporting an emission that never reached the sink skews frame counts in stats/drain logic.
What correct looks like: `Ok(DecodeOutcome::Emitted { frames: u64::from(frame_samples > 0) })` or a distinct outcome for a zero-length decode.
Confidence: MEDIUM — Verification: read (engine 0-frame return not observed in a run; symphonia can return 0 frames for some packet shapes).

### F-003 [S4]
Where: crates/codecs/src/audio/mp2/decoder.rs:93
Code:
```rust
    let frame_bytes = packet.get(..need_len).unwrap_or(&[]);
```
What is wrong: bytes after the declared frame size in a packet are silently discarded — a packet carrying one frame + trailing data decodes its first frame and drops the rest with no typed complaint, despite the documented packet model ("one packet = one complete frame … exactly `frame_size` bytes", lines 20-22).
Why it matters: a demuxer violating the contract gets silent data loss instead of a typed error; the PS host's assembler comments (apps/cli/src/containers/ps.rs:21-29) confirm the one-frame contract is load-bearing.
What correct looks like: `if packet.len() > need_len { typed corrupt("mp2.frame.trailing_bytes", …) }` (or at least a debug log).
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/codecs/src/audio/mp2/decoder.rs:139-158
Code:
```rust
            media_types::error::CorruptDetail::Truncated {
                expected: 4,
                found: (data.len() - pos.min(data.len())) as u64,
            },
```
What is wrong: the `no_frames` error's detail claims `expected: 4` bytes regardless of context; and the second arm (lines 149-159, `geometry` returning `None`) reports `found: 0` even when bytes remain — the detail is filler, not the real measurement.
Why it matters: error details feed diagnostics/telemetry; wrong numbers mislead debugging of damaged files.
What correct looks like: report the scanned range (`data.len() - pos`) in both arms and drop the fake `expected: 4`.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `decode_one` (free fn) | 74-114 | REAL | — | Typed truncation gate before engine call; grows scratch monotonically; drops trailing bytes (F-003); no panic paths (`get(..)` bounded). |
| `decode_stream` (free fn) | 126-217 | REAL | — | ID3 skip → walk → CBR gate → decode-all; test/bench surface only (not shipped lane); spec latching (lines 188-193) mirrors F-001 but is protected here by `geometry`'s cbr gate at line 147. |
| `DecodedMp2` (struct) | 51-66 | REAL | — | Data carrier. |
| `Mp2Decoder` (struct) | 229-248 | REAL | — | State fields all consumed (except `packet_index`, see §7). |
| `Debug for Mp2Decoder` | 250-257 | REAL | — | Hides buffers — fine. |
| `Mp2Decoder::new` | 268-271 | REAL | — | Delegates to Default. |
| `Mp2Decoder::accepts` | 274-276 | REAL | — | Exactly `"mp2"`; pinned by test. |
| `Decoder::configure` | 280-323 | REAL | — | Descriptor gate, extra-data refusal (honest Layer II posture), engine init, full state reset. |
| `Decoder::decode` | 325-400 | REAL | S2 | Stale geometry latch (F-001), outcome over-report (F-002); pts/duration arithmetic otherwise correct (samples/rate, integer ns). |
| `Decoder::flush` | 402-408 | REAL | — | Layer II is stateless — `Drained` immediately; state-guarded. |
| `pooled_sample_slot` | 414-452 | REAL | — | 4-slot rotation; slot-len change degrades to per-call alloc but stays bounded; no panic. |
| test `descriptor_gate_is_exactly_mp2` | 458-465 | REAL | — | Would fail if `accepts` loosened. |
| test helpers `packet_of` / `Sink` / `lsf_frame` | 467-506 | REAL | — | Synthetic zero-payload frame (near-silence) — shape-only, no sample ground truth in-file. |
| test `decode_before_configure_is_typed_bad_state` | 508-529 | REAL | — | Asserts `mp2.not_configured` code. |
| test `configure_rejects_foreign_descriptors_and_unexpected_facts` | 531-566 | REAL | — | Asserts both refusal codes. |
| test `a_minimal_synthetic_stream_decodes_end_to_end` | 568-585 | REAL | — | Shape assertions (1152/16k/mono); sample truth lives in corpus differential (passes in baseline). |
| test `a_layer_iii_stream_is_refused_not_misdecoded` | 587-599 | REAL | — | Asserts `mp2.stream.layer_mismatch`. |
| test `a_truncated_frame_packet_is_a_typed_truncation` | 601-657 | REAL | — | Asserts exact Truncated{720,100} + per-frame isolation. |
| test `cbr_seek_plan_from_the_decoded_geometry_lands_on_frames` | 659-677 | REAL | — | Composes decode_stream geometry with seek planner. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 7 in-file tests | Yes | Each asserts an exact error code, count, or shape; mutation of the truncation gate or descriptor gate flips them. |
| `a_minimal_synthetic_stream_decodes_end_to_end` | Partially | Detects shape regressions; cannot detect wrong sample VALUES (zero-payload frame; the value truth is the external corpus differential, which passes in baseline). |
| Gap | — | No test covers a mid-stream channel/rate change through `Mp2Decoder::decode` (F-001) — the stream lane's cbr-gate tests cover only `decode_stream`. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ffmpeg's Layer II decode emits exactly frames × 1152 samples and so does this module — sample-locked" (lines 31-34) | module doc | REAL | baseline `corpus_and_fixture_differential_vs_ffmpeg ... ok` (orchrun log:1101). |
| "damage is never a panic" (line 125) | module doc | REAL-UNPROVEN for contract lane | fuzz survival tests pass (orchrun log:1104-1107); no panic path found by reading; 0-frame engine return not exercised. |
| "per-frame isolation … after a typed Corrupt the next packet decodes unchanged" (lines 22-24) | module doc | REAL | `a_truncated_frame_packet_is_a_typed_truncation` second half (lines 644-656). |

## 7. Dead / duplicated / unreachable code
- `packet_index` field (line 236): incremented (line 354), never read anywhere in the crate (`rg -n "packet_index" crates/codecs` → this file, writes only). Dead bookkeeping.
- `decode_stream` + `DecodedMp2`: not reachable from `lumen`/`lumen-tauri` — test/fuzz/bench-only surface (by design per mod.rs docs; noted, not dead).

## 8. Dependencies & flags
- `symphonia-bundle-mp3`/`symphonia-core` =0.6.1 behind `codec-mp2` (default-on, `crates/codecs/Cargo.toml:47-59,77-78`) — shipped build includes it (baseline build-release-workspace.log.txt exit 0).
- No unsafe, no unwrap/expect/index-panic in this file (verified by reading; `unwrap_or` forms only).

## 9. Open questions for the next agent
- `mp2/seek.rs` (next file, same agent) consumes `Mp2Geometry` — confirm the padding-refinement arithmetic against padded corpus streams.
- Does symphonia `MpaDecoder` ever return `frames() == 0` for a Layer II packet (F-002 premise)? An orchestrator run feeding the corpus through the contract decoder would settle it. Not blocking: baseline fuzz survival covers panics, not this accounting question.
