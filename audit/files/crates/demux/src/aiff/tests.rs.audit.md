# AUDIT: crates/demux/src/aiff/tests.rs
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-23 |
| Lines / functions | 537 / 26 (6 builders/helpers + 20 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — AIFF lane wired: apps/cli/src/containers/mod.rs `("aiff") => aiff::open(stream, opts)` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p demux aiff` -> exit 0, 32 passed / 0 failed (20 aiff::tests + 8 chunks + 1 error + 4 fuzz tests); sed quote verification |

## 1. One-paragraph verdict
A strong, byte-honest unit suite. It builds AIFF/AIFC containers by hand (FORM/COMM/SSND/MARK/INST with correct big-endian layout, odd-payload pads, 80-bit extended rates) and attacks the parser with real malformed shapes: 4-byte head only, WAVE behind FORM magic, truncated COMM, SSND declaring 2 bytes past the stream (prefix delivery + Recovered(2,"ssnd-extends-past-stream") + typed truncation — event ORDER asserted), missing COMM/SSND, SSND offset past the sound area, COMM/SSND frame disagreement (damage note), byte-surgery denormal 80-bit rate (exponent zeroed at exact offsets 16/17, mantissa tail at 25), 8-bit and 12-bit unsupported depths, trailing partial frame, and pre-start trait misuse. Seek maps frames exactly (500/8000), refuses past-end typed, reanchors to the exact tail (500×4 bytes). All 20 tests pass under my run; the module adds 12 more (chunks/error/fuzz) that also pass. The one semantic sharp edge — start() is permanently poisoned after a pre-start next_event()/seek() call — is deliberately pinned by `trait_misuse_is_typed_not_a_panic` rather than accidental.

## 2. What actually works (verified)
— verified by `cargo test -p demux aiff` -> 32 passed; 0 failed (my run, exit 0; includes aiff::chunks::tests, aiff::fuzz::tests, aiff::error::tests).
— verified by read: the denormal-rate surgery offsets (comm[16]/[17] exponent, comm[25] mantissa tail) match the 80-bit extended layout in the COMM payload (channels@8, frames@10, bits@14, rate@16-25) — real byte-level attack, not a mock.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/aiff/tests.rs:472-476 (with mod.rs:645-647, 667)
Code:
```
    let err = match aiff.start() {
        Ok(_) => unreachable!("must refuse"),
        Err(e) => e,
    };
    assert_eq!(err.context.code, "demux.aiff.not-started");
}
```
What is wrong: The pinned semantics: after ONE pre-start `next_event()`/`seek()` call, `start()` can never succeed — `self.fail()` sets the sticky shape and `start()` mints it forever (mod.rs:645-647). A host that accidentally probes events before start gets an unrecoverable demuxer (must re-construct and re-open the file). Why it matters: sharp edge, but typed, deterministic, and clearly pinned as intended — a design wart, not silent breakage. What correct looks like: either a doc line on `start()` stating the poison rule, or not-started not being sticky. Confidence: HIGH — Verification: read + ran (test passes).

F-002 [S4]
Where: crates/demux/src/aiff/tests.rs:529-536
Code:
```
    let _ = CommChunk {
        channels: 1,
        num_frames: 0,
        sample_size: 16,
        sample_rate: 44_100,
        compression: None,
    };
    let _ = chunks::MARK;
}
```
What is wrong: Dead-value constructions whose only purpose is touching `CommChunk`/`chunks::MARK` (keeping dead-code lints quiet); they assert nothing. Confidence: HIGH — Verification: read.

Zero S0-S3 findings. Checked and clean: endianness (BE everywhere, samples verbatim per line 127-141), pad handling (odd COMM payload → SSND lands past the pad), damage-event ordering (Recovered BEFORE the prefix packet), pool quantum observable (`pool_bytes() == 1_200`), probe form discrimination (AIFF/AIFC yes, WAVE/RIFF/short no), seekability Fast{indexed:false} consistent with frame-exact linear math.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| drain | 17-31 | REAL | — | collects payload + events; errors fail via unreachable! |
| started_aiff | 33-38 | REAL | — | |
| form | 42-51 | REAL | — | FORM size patched BE |
| comm_chunk | 53-71 | REAL | — | 80-bit rate via extended80, odd pad |
| ssnd_chunk | 73-85 | REAL | — | `declared_extra` lies expressible |
| mark_and_inst_chunks | 87-106 | REAL | — | pstring + pad correct |
| drains_classic_pcm16be_whole_frames | 108-124 | REAL | — | codec pcm_s16be, offset/block (0,0) |
| big_endian_samples_arrive_verbatim | 126-141 | REAL | — | byte-order normalization is the codec's job — pinned |
| pts_ladder_and_span_count_sample_frames | 143-165 | REAL | — | span 300 ticks, pool 1200 |
| mark_and_inst_chunks_are_skip_tolerant | 167-178 | REAL | — | no damage |
| ssnd_offset_skips_into_the_sound_data | 180-194 | REAL | — | offset semantics correct |
| odd_comm_payload_takes_its_pad | 196-216 | REAL | — | AIFC sowt |
| aifc_compression_vocabulary_demuxes | 218-244 | REAL | — | NONE/sowt/fl32/fl64 |
| truncated_head_is_typed | 246-268 | REAL | — | + WAVE-behind-FORM → not-aiff |
| truncated_comm_is_typed_truncation | 270-282 | REAL | — | 24-byte cut |
| declared_ssnd_past_actual_delivers_prefix_then_truncated | 284-312 | REAL | — | Recovered(2)→packet→truncated ordering |
| missing_comm_and_ssnd_are_typed | 314-337 | REAL | — | no-comm / no-ssnd |
| zero_or_offset_past_ssnd_is_no_ssnd | 339-354 | REAL | — | offset 16 > 2 sound bytes |
| comm_declarations_disagreeing_with_ssnd_are_damage | 356-373 | REAL | — | Recovered(4,"comm-frames-short-of-ssnd") |
| non_normalizable_rate_is_structural | 375-393 | REAL | — | denormal 80-bit surgery at exact offsets |
| unsupported_depths_are_typed | 395-425 | REAL | — | 8-bit classic, 12-bit NONE AIFC |
| trailing_partial_frame_is_damaged_and_dropped | 427-446 | REAL | — | "ssnd-not-frame-aligned" |
| trait_misuse_is_typed_not_a_panic | 448-477 | REAL | S4 | poison semantics pinned (F-001) |
| seek_maps_frames_and_reanchors | 479-509 | REAL | — | 500→500, 1001→out-of-range, reanchor tail exact |
| probe_recognizes_both_forms | 511-518 | REAL | — | |
| error_constructors_keep_their_codes | 520-537 | REAL | S4 | taxonomy pin + dead tail (F-002) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 20 | YES | Exact typed codes, exact byte payloads, event ordering, damage byte counts, seek tick landings, reanchor tail length all pinned absolutely. Byte-level fixtures, no mocks of the parser. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "COMM/SSND framing, the 80-bit rate, MARK/INST skip-tolerance, and the AC error paths" | header lines 1-4 | REAL | inventory matches; 20/20 pass |
| "The ffmpeg-generated fixtures get their differential + mutated-family suite in wav_aiff_fixtures.rs" | header lines 4-6 | REAL (cross-ref) | file exists (crates/demux/tests/); its audit belongs to that file's owner |

## 7. Dead / duplicated / unreachable code
F-002's dead constructions only. No unreachable code.

## 8. Dependencies & flags
`#[cfg(test)]`-gated (mod tests in aiff/mod.rs). Uses super::test_support (pub mod, library-compiled seam — same crate-wide pattern). No feature gates.

## 9. Open questions for the next agent
- aiff/mod.rs owner: is the start()-poison rule (F-001) documented anywhere besides the test? If the engine crate ever calls next_event() speculatively, every AIFF open dies permanently — check playback-engine call order.
