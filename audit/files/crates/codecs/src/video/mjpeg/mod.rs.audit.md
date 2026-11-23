# AUDIT: crates/codecs/src/video/mjpeg/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 436 / 8 (decode_image, 3 types, MjpegDecoder::{new,accepts}, Decoder::{configure,decode,flush}) + 2 unit tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — lumen-cli pipeline_host.rs:242-244 `"mjpeg" => MjpegDecoder::new()` (first-vertical decoder, confirmed in engine-info) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | sed/rg verification; baseline test results cited (unit tests + staged execution of the decoder via stage_bench example earlier this session); VideoFrame struct field check |

## 1. One-paragraph verdict
The shipped MJPEG Decoder adapter: stateless per packet, typed refusals for out-of-subset input, entropy damage concealed (mid-gray) and reported as `Recovered { ConcealedFrame }`, generational stale-drop, pooled output slots (zero steady-state alloc per frame), honest unsupported-list in the module docs. This is one of the two decoders the first vertical actually registers, and the full pipeline (parse→huffman→idct→assemble) was executed end-to-end this session via stage_bench on a real fixture. Two defects: the AVI1 field polarity is surfaced on the free `DecodedFrame` but DROPPED on the contract path (`VideoFrame` has no polarity field — downstream field-pair weaving, explicitly deferred to "the layer that owns it", receives half-height frames it cannot order), and the `codec` field is write-only.

## 2. What actually works (verified)
— verified by executing `./target/debug/examples/stage_bench-62f5db167ab6014a /tmp/bench_420_odd.mjpeg --passes 5` this session: the underlying decoder::decode pipeline (which this module's `decode_image`/`MjpegDecoder::decode` wrap) decodes real MJPEG, exit 0.
— verified by baseline audit/runs/baseline/cargo-test-workspace.txt:837-838 -> `video::mjpeg::tests::descriptor_gate_accepts_the_vertical_vocabulary ... ok`, `jfif_color_tag_is_bt601_full_range ... ok`.
— verified reachability: apps/cli/src/pipeline_host.rs:242-244 constructs `MjpegDecoder::new()` for "mjpeg" tracks; baseline engine-info lists mjpeg as a shipped decoder.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/video/mjpeg/mod.rs:56-58 (doc) vs 388-389 (code) vs media-types VideoFrame
Code: (verbatim doc L56-58)
```
//! operation), deliberately outside a stateless per-packet decoder;
//! the polarity is surfaced on [`DecodedFrame`] for the layer that
//! owns it. Measured consequence (worklog 5.H3): ffmpeg 7.1's mjpeg
```
and code L388-389:
```
            color: JFIF_COLOR,
            interlaced: meta.field_polarity.is_some(),
```
What is wrong: the module doc says field polarity is "surfaced ... for the layer that owns it", but the CONTRACT path (the only path the shipped engine uses) reduces polarity to a single `interlaced` bool — `media_types::frame::VideoFrame` (frame.rs:211-225) has no polarity field. `FirstField`/`SecondField` frames arrive at the sink as half-height interlaced frames with no way to distinguish first from second field, so the deferred weaving layer cannot do its job through the shipped contract. Why it matters: AVI1 field-based MJPEG (the 5.H3 feature this module advertises) is only half-delivered on the shipped path; consumers get unordered half-frames. Mitigation: ffmpeg does not emit AVI1 (doc L58-63, "differential pins those"), so real-world blast radius is limited to hand-built streams. What correct looks like: a polarity/field-order field on VideoFrame (media-types extension) or the contract emitting only `Interleaved` frames. Confidence: HIGH — Verification: read of both structs + doc.
F-002 [S4]
Where: crates/codecs/src/video/mjpeg/mod.rs:280, 323
Code: (verbatim, L323)
```
        self.codec = Some(cfg.codec.to_string());
```
What is wrong: the `codec` field is written at configure and never read (only reachable via the derived Debug) — write-only state labeled "diagnostics". Confidence: HIGH — Verification: rg `self.codec` -> declaration + one write only.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| decode_image | 146-165 | REAL | - | free entry; maps ConcealCause→ConcealReason 1:1; used by fuzz harness |
| ConcealReason / DecodedFormat / DecodedFrame | 169-211 | REAL | - | honest observability types |
| JFIF_COLOR | 213-219 | REAL | - | BT.601 full + BT.709 primaries + sRGB transfer, pinned by unit test |
| MjpegDecoder::new / accepts | 294-306 | REAL | - | mjpeg|jpeg vocabulary, unit-tested |
| Decoder::configure | 309-325 | REAL | S4 | typed descriptor refusal; codec field dead (F-002); state carry-over across reconfigure is intentional (allocation-only state, L229-230) |
| Decoder::decode | 337-401 | REAL | S3 | stale guard, pooled slot handoff, concealed→Recovered{ConcealedFrame} exact; polarity drop (F-001) |
| Decoder::flush | 403-415 | REAL | - | Drained; not-configured typed |
| tests (2) | 422-435 | REAL | - | trivial but genuine gates |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| descriptor_gate_accepts_the_vertical_vocabulary | YES | direct behavioral assert |
| jfif_color_tag_is_bt601_full_range | YES (narrowly) | pins the const; a color regression fails it |
The heavy coverage (bit-exact, fuzz survival, contract suite) lives in sibling test files (mjpeg suites pass in baseline); the doctest at L234-274 is a real negative-path pin (empty packet must Err).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "stateless between packets ... flush is immediately drained" | L4-6 | REAL | no inter-frame state in struct; flush → Drained (L414) |
| unsupported list (progressive, 12-bit, 4-component, etc.) | L34-39 | REAL-UNPROVEN here | typed Unsupported paths live in parser/decoder.rs (not in my assignment); fuzz survival suites pass in baseline |
| "the polarity is surfaced on DecodedFrame for the layer that owns it" | L56-58 | HALF-TRUE | true for the free path; NOT surfaced on the shipped contract path (F-001) |
| "zero steady-state allocation" (pooled slots) | L354-359 | REAL-UNPROVEN | pool exists (decoder.rs FRAME_SLOTS=4, pooled_frame_slot L853+); Arc-retention safety not audited here — see open questions |

## 7. Dead / duplicated / unreachable code
- `codec: Option<String>` write-only (F-002).
- Nothing else; all module children are wired (fuzz is pub for the registered target).

## 8. Dependencies & flags
In-tree: codec_api (Decoder contract, drop_stale_packet), media_types. No feature gates. Shipped in the release lane (first vertical).

## 9. Open questions for the next agent
- decoder.rs (mjpeg) is NOT in my assignment: audit `pooled_frame_slot`'s reuse semantics — if a consumer retains >FRAME_SLOTS(4) SharedBytes, does refill copy-on-write or mutate a referenced frame?
- The engine-side "Recovered-class conversion + bounded RecoveryAttempts terminal" (L97-104) is playback-engine code — verify that wiring exists (the doc cites doc 03 §4.3(d) and audit M9).
