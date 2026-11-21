# AUDIT: crates/codecs/src/audio/flac/decoder.rs
| Field | Value |
|---|---|
| Auditor | W07-A01 |
| Date | 2026-11-21 |
| Lines / functions | 716 / 22 (incl. 4 test fns + 1 doctest) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.flac` → apps/cli/src/containers/mod.rs:62 `"flac" => es::open_flac(...)` → engine `Pipeline::wire` → apps/cli/src/pipeline_host.rs:259 `"flac" => self.seeded(track, config, FlacDecoder::new)`; also flac-in-WAV facts seeding and apps/web/src/host.rs:624). No baseline run opens a FLAC end-to-end (rc-smoke is AVI-only by design, INDEX.md #20), but the decoder's unit/contract tests executed in baseline run #3 (test-workspace.log.txt:358-382, all `ok`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "FlacDecoder" crates apps` (callers: pipeline_host.rs:55/259, apps/web host.rs:624, tests, fuzz); `rg -n "flac" apps/cli/src/containers/mod.rs` (registry arm); `rg -n "fn drop_stale_packet"` (codec-api/src/decode.rs:203); `rg -n "sample_rate|bits_per_sample|max_block_size" flac/metadata.rs` (validation of facts: rate≠0, bps 4..=32, min≥1 & max≥min — decoder's configure-time assumptions confirmed); `sed -n` on every quoted range; `rg -n "flac" audit/runs/baseline/test-workspace.log.txt` (executed evidence). |

## 1. One-paragraph verdict
This is a genuinely implemented, panic-hostile claxon wrapper: configure refuses to guess geometry without the 34-byte STREAMINFO (typed `flac.streaminfo.missing`), the channel-count gate runs before any claxon plane access (the fuzz-found panic class), all output writes are length-checked before the interleaver, and error taxonomy is consistently typed. The only defects found are diagnostic/test-honesty/hygiene level: a fabricated `expected` length in the `Truncated` error detail, an in-file "regression" test whose body cannot exercise the gate it names (real regression lives in tests/flac_bitexact.rs:169), scratch-buffer loss on every error path (contradicting the D-1 warm-up discipline on damaged streams), and a per-sample `block.channel()` call in the interleave hot loop. No S0–S2 found: I traced every arithmetic/indexing path (`max_output_len` sizing, `dst` bounds check at 256, `checked_shl`, `rate.max(1)` division guard, `i64/u64::try_from…unwrap_or` saturations) and found no panic or wrong-output path.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/test-workspace.log.txt:358-382` → `audio::flac::decoder::tests::*` and `contract_tests::channel_count_mismatch_is_typed_not_panic` all `ok` (baseline run #3, 1681 passed).
— Verified by `rg` call chain: `FlacDecoder::new` is constructed by the shipped CLI's `HostDecoderProvider::decoder_for` ("flac" arm, pipeline_host.rs:253-260) and by apps/web/src/host.rs:624 — the decoder is wired, contradicting `lumen engine-info`'s hard-coded `decoders: mjpeg, pcm_s16le` literal (standing defect already on INDEX.md's correction list).
— Verified by reading `flac/metadata.rs:182-197` that the facts the decoder relies on (rate≠0, bps∈4..=32, block sizes ordered/nonzero) are enforced at configure; the module comment's claim at decoder.rs:583-586 is accurate.

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/audio/flac/decoder.rs:183-189
Code:
```
                if truncated {
                    media_types::error::CorruptDetail::Truncated {
                        expected: u64::from(facts.max_frame_size.unwrap_or(u32::MAX)),
                        found: packet.len() as u64,
                    }
```
What is wrong: When STREAMINFO carries no max frame size (spec value 0 = unknown; `max_frame_size` is `None`), the reported `expected` is fabricated as `u32::MAX` (4294967295 bytes), a number that has no relation to the stream.
Why it matters: The structured `Truncated` detail is consumer-facing diagnostics (UI error strings, logs); it asserts a bogus expected size for every truncated frame in max-frame-size-less streams — the most common FLAC shape (ffmpeg's default writes max_frame_size only when known).
What correct looks like: `expected: facts.max_frame_size.map(u64::from)` or an `Option`/0-unknown encoding in `CorruptDetail::Truncated`; never a sentinel masquerading as a measured bound.
Confidence: HIGH — Verification: read.

### F-002 [S3]
Where: crates/codecs/src/audio/flac/decoder.rs:454-477 (test `contract_tests::channel_count_mismatch_is_typed_not_panic`)
Code:
```
        let frame = [0xFFu8, 0xF8, 0x51, 0x00, 0x00, 0xC5, 0x00, 0x00];
        let mut scratch = Vec::new();
        let mut out = vec![0u8; max_output_len(&facts)];
        let result = decode_into(&frame, &facts, &mut scratch, &mut out);
        assert!(result.is_err());
```
What is wrong: The test's own comment (lines 467-472) concedes the real decodable-frame regression lives in `tests/flac_bitexact.rs`; the 8-byte hand-crafted "frame" here cannot decode (block-size nibble 0x5 ⇒ 4608 samples in 8 bytes ⇒ claxon `UnexpectedEof`/CRC failure) and the assertion is only `result.is_err()`. Delete the channel gate at lines 231-240 entirely and this test still passes — it pins "garbage frame errors", which every other error path already guarantees, not the fuzz-found panic class its name and module doc (446-451) claim.
Why it matters: A named regression test that cannot fail under regression of its named subject is false evidence inventory; a future refactor that reorders the gate after `interleave_block` ships green on this test.
What correct looks like: Either use the fixture-backed decodable-frame form (as flac_bitexact.rs:169-190 already does — mono frame + `doctored.channels = 2` + assert `err.context.code == "flac.frame.channels"`), or rename this in-file test to what it actually pins (`garbage_frame_is_typed_not_panic`).
Confidence: MEDIUM — Verification: read; claxon's rejection of the 8-byte frame is reasoned (header nibbles ⇒ 4608-sample block, 8 bytes available), not executed.

### F-003 [S4]
Where: crates/codecs/src/audio/flac/decoder.rs:166-168 (with 231 and 267)
Code:
```
    let taken = std::mem::take(scratch);
    let mut reader = claxon::frame::FrameReader::new(std::io::Cursor::new(packet));
    let decoded = match reader.read_next_or_eof(taken) {
```
What is wrong: The scratch `Vec<i32>` is taken from the decoder on every call but restored (`*scratch = decoded.into_buffer()`, line 267) only on the success path. Every typed-error return (claxon Err at 178-218, channel gate at 231, slot-size guard at 256) drops the buffer; the next packet reallocates a max-block-sized `Vec<i32>` (e.g. 4608×2ch×4B ≈ 36 KB per corrupt packet).
Why it matters: The module's D-1 allocation-discipline section (51-59) advertises a reused decode buffer; on damaged/sync-scanning streams — exactly what the fuzz-survival lane models — allocation is per-packet, defeating the stated discipline. Bounded, so not a perf defect of consequence on healthy files.
What correct looks like: Restore the buffer on error paths too (claxon's `read_next_or_eof` consumes the Vec; wrap so an owned Vec is recovered, e.g. re-thread `reader`'s buffer out on Err, or accept and document the error-path realloc explicitly).
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/codecs/src/audio/flac/decoder.rs:296-304
Code:
```
    for i in 0..block_size as usize {
        for ch in 0..channels as usize {
            let plane = block.channel(ch as u32);
```
What is wrong: `block.channel(ch)` (a slice-indexing call with per-call bounds panics) is invoked once per sample×channel instead of hoisting `channels` plane slices out of the inner loop — for a 4608-sample stereo block that is 9216 calls where 2 would do.
Why it matters: Hot-path per-sample overhead in the interleave loop for every FLAC frame decoded; small relative to entropy decoding, but it is pure avoidable work and the pattern invites copy-paste into future audio paths.
What correct looks like: `let planes: Vec<&[i32]> = (0..channels).map(|ch| block.channel(ch)).collect();` before the loop (safe post-gate: channels were checked equal at 231), then index `planes[ch][i]`.
Confidence: HIGH — Verification: read (no execution; magnitude reasoned).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `decode_frame` (free surface) | 93-104 | REAL | — | Allocates per call (documented free surface); truncates `out` to exact length; no panic path. |
| `BlockMeta` (struct) | 107-115 | REAL | — | Trivial data holder. |
| `bytes_per` | 119-121 | REAL | — | Trivial width helper. |
| `max_output_len` | 125-128 | REAL | — | max_block×ch×width; facts validated (metadata.rs:194-197) so max_block ≥ 1 — no zero-length pool. |
| `output_sample_format` | 133-139 | REAL | — | Left-justify mapping; bps∈4..=32 guaranteed upstream ⇒ shift ≤ 16 < 32, `checked_shl` never saturates in practice. Pinned by tests:689-698. |
| `decode_into` | 150-279 | REAL | F-001,F-003 | Core: empty-packet typed corrupt; claxon error taxonomy (Io/Format/Unsupported); channel gate BEFORE plane access (verified order 223→240→266); slot-size guard before interleave; scratch loss on error paths. |
| `interleave_block` | 287-323 | REAL | F-004 | All reads `get`-bounded, writes pre-checked; `checked_shl().unwrap_or(0)`; widths 2/3/4 match negotiated format — no panic path found. |
| `write_i24` | 328-339 | REAL | — | Native-endian low-24; big/little branches; pinned by tests:701-715. |
| `FlacDecoder` (struct + const) | 410-430 | REAL | — | State: facts/generation/codec/scratch/pool/cursor. |
| `FlacDecoder::new` | 434-437 | REAL | — | Trivial `default()`. |
| `FlacDecoder::accepts` | 441-443 | REAL | — | Exact `"flac"` match; pinned by tests:681-686. |
| `contract_tests::channel_count_mismatch_is_typed_not_panic` | 454-516 | SHALLOW | F-002 | See finding — cannot fail under gate regression. |
| `Decoder::configure` | 520-549 | REAL | — | Descriptor gate → typed unsupported; mandatory extra_data (`flac.streaminfo.missing`) — refuses to guess, correct; pool cleared per configure (no stale slot sizes). |
| `Decoder::decode` | 551-604 | REAL | — | Stale-generation guard first (codec-api decode.rs:203); empty packet ⇒ NeedMoreInput; duration via `rate.max(1)` + `TimeBase::new` err-mapped (no div-by-zero/panic); pts/duration saturate via `try_from…unwrap_or`. |
| `Decoder::flush` | 606-612 | REAL | — | Stateless-by-design ⇒ Drained; BadState before configure. |
| `PooledBlock` (struct) | 617-620 | REAL | — | Trivial. |
| `pooled_sample_slot` | 629-674 | REAL | — | Exclusive-ownership reuse; fresh-slot `get_mut` failure surfaced typed, not fabricated; eviction bounded at SAMPLE_SLOTS=4. Traced: no aliasing (fill happens while exclusively borrowed; returned clone is the only share). |
| doctest (FlacDecoder docs) | 351-409 | REAL | — | Executed by cargo test doctest lane (baseline run #3); asserts both the typed refusal and NeedMoreInput — would fail if configure stopped demanding facts. |
| `tests::descriptor_gate_accepts_only_flac` | 681-686 | REAL | — | Would fail if `accepts` widened. |
| `tests::output_width_mapping_is_left_justifying` | 689-698 | REAL | — | Would fail if mapping changed. |
| `tests::write_i24_is_native_order_low_24_bits` | 701-715 | REAL | — | Would fail if byte order/mask broke. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| contract_tests::channel_count_mismatch_is_typed_not_panic (455) | NO (for its named subject) | Input frame cannot decode; asserts only `is_err()` — any typed-error path satisfies it. Real coverage is external: tests/flac_bitexact.rs:169-190 uses a decodable mono fixture + doctored 2ch facts and asserts the exact code `flac.frame.channels`. |
| tests::descriptor_gate_accepts_only_flac (681) | YES | Direct assertions on `accepts`. |
| tests::output_width_mapping_is_left_justifying (689) | YES | Exact (format, shift) pairs. |
| tests::write_i24_is_native_order_low_24_bits (701) | YES | Byte-exact expectations both endians. |
| doctest (351-409) | YES | `expect_err("facts are mandatory")` + exact error code + NeedMoreInput outcome. |
| External: flac_bitexact.rs (differential vs ffmpeg goldens) | YES | Per-sample comparisons against committed reference PCM (verified file exists and asserts exact codes; execution cited via baseline log lines 358-382 for in-crate tests). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "claxon owns exactly one job: entropy-coded frame decode … metadata, seeking, error taxonomy, contract surface are ours" | decoder.rs module docs 14-18 | REAL | Code structure matches: this file + metadata.rs/seek.rs/error.rs are all in-house; claxon used only at line 167-168. |
| "configure without facts is a typed Unsupported (flac.streaminfo.missing), never a guess" | decoder.rs 30-37 | REAL | configure 529-539 enforces; doctest asserts the code. |
| "≤16→S16, 17–24→S24, 25–32→S32, shift width−bits" | decoder.rs 39-49 | REAL | output_sample_format 133-139 + test 689-698. |
| "zero per-frame allocation after warm-up" (D-1) | decoder.rs 51-59 | PARTLY FALSE | True on the success path only; error paths drop the scratch buffer (F-003). Output-pool part holds. |
| "Bit-exact against the reference" | decoder.rs 47-48 | REAL-UNPROVEN (for me) | tests/flac_bitexact.rs exists with ffmpeg goldens; baseline run #3 executed the suite green, but I did not re-run and did not verify golden provenance (W07-A01 scope: this file). |
| FLAC decode row in feature matrix | docs/feature-matrix.toml (not my row range) | UNVERIFIED | Out of assigned scope; end-to-end `lumen open` of a FLAC never ran in baseline (rc-smoke AVI-only, INDEX.md #20). |

## 7. Dead / duplicated / unreachable code
None dead. `decode_frame` free surface is used by fuzz (flac/fuzz.rs:46,140) and tests. `pooled_sample_slot` is a local re-author of the MJPEG pattern — acknowledged duplication in its own doc (622-628), acceptable given module independence. Note: the whole FLAC lane is invisible to `lumen engine-info`'s hard-coded decoder list (INDEX.md "Standing defects") — reporting gap, not unreachability.

## 8. Dependencies & flags
`claxon =0.4.3` exact-pinned, optional behind `codec-flac` (Cargo.toml:68); `default` includes `codec-flac` (Cargo.toml:47-59) so the shipped workspace build compiles it. `symphonia-core` not used here. No unsafe. Dev-deps (input/demux/plugin-api/testkit) are downward test-only — consistent with boundaries.

## 9. Open questions for the next agent
- flac/seek.rs `parse_header_at` first_sample arithmetic (the 47×4608-vs-47×3924 claxon-discrepancy comment at 241-252) — owned by the seek-file agent; confirm `SeekPlanner::parse_header_at` never misparses a variable-blocksize stream into a WORSE estimate than claxon's fallback.
- Who harvests the 34-byte STREAMINFO for native FLAC (es::open_flac) vs WAV (wav.rs)? The `seeded` comment says facts pre-pass; confirm both lanes actually populate `extra_data` — if a lane forgets, open fails typed (honest) but users see "unsupported".
