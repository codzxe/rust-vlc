# AUDIT: crates/codecs/src/video/av1/obu.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2026-11-21 |
| Lines / functions | 559 / 4 production (obu_error, read_leb128, parse_temporal_unit + struct defs) + 11 tests + 2 test helpers (MustErr, obu) |
| File verdict | REAL (spec-checked framing validator, executed end-to-end via the differential suite) |
| Reachable from shipped entry point? | YES. Chain: `lumen open <file.ivf>` → apps/cli containers/ivf.rs (AV01 → descriptor `av1`, ivf.rs:15-18) → pipeline_host.rs:304-308 `"av1" \| "av01"` arm → `Av1Decoder::new()` → `Av1Decoder::decode` → `obu::parse_temporal_unit` (decoder.rs:860). Gate: `#[cfg(feature = "codec-av1")]` (video/mod.rs:22); `codec-av1` is in codecs default features (codecs/Cargo.toml:50) which release.yml builds unmodified (release.yml:193 bare `cargo build --release --locked`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read (full file); Grep `parse_temporal_unit` crates/ → sole production caller decoder.rs:860; Grep `av1\|AV01` apps/cli/src/containers/ivf.rs → descriptor mapping :15-18, fixtures :260-262; rg -n "av1::obu" audit/runs/baseline/test-workspace.log.txt → :524-534 all 11 `ok`; sed -n '891,905p' baseline log → av1_differential 6/6 `ok` incl. `differential_pixel_md5_against_ffmpeg_dump`; Grep `trait MustErr` crates/ → 3 copies (obu.rs, h264_hw_path.rs, hevc_hw_path.rs). |

## 1. One-paragraph verdict
The honest-scope promise in the header ("framing validator, not a bitstream parser") is exactly what the code is: OBU header bit extraction (forbidden/type/ext/has_size/reserved) matches AV1 §5.3.1 bit-for-bit, the leb128 reader enforces the 8-byte / <2³² window with typed errors, sizes are checked against the buffer with `checked_add` (correctly defensive on 32-bit targets where `u32::MAX as usize + start` could wrap), and the OBU-count cap bounds the census. All 11 unit tests ran in the baseline (test-workspace.log.txt:524-534) and the full demux→decode differential suite passed 6/6 including a pixel-MD5 lane against ffmpeg (baseline :891-904) — this code is real and exercised. Two defects: a provably dead "fixup" block that misrepresents reachable logic, and a triple-duplicated test helper. The census's frame-header 3-bit peek is honest about its own limitation (reduced-still-picture streams refuse, `[VERIFY-P1]` recorded at :110-112).

## 2. What actually works (verified)
— All 11 unit tests executed and passed: baseline test-workspace.log.txt:524-534 (`empty_tu_is_typed_truncation` … `extension_byte_rules`, zero ignored).
— End-to-end AV1 decode through this framing: baseline :891-904 `av1_differential` 6/6 ok, including `differential_pixel_md5_against_ffmpeg_dump ... ok` and `fuzz_lite_framing_and_decode_survival ... ok`.
— Spec conformance of the header parse re-derived by me: type=`(h>>3)&0x0f`, ext=`0x04`, size=`0x02`, forbidden=`0x80`, reserved=`0x01` — matches AV1 §5.3.1 field order (f(1) forbidden, f(4) type, f(1) ext, f(1) has_size, f(1) reserved); extension byte temporal_id=`ext>>5`, spatial_id=`(ext>>3)&0x03`, reserved-low-3 — matches §5.3.2.
— leb128 boundary behavior verified by hand: `0xff×4,0x1f` → 0x1F_FFFF_FFFF > u32::MAX → `av1.obu.size_overflow` (matches test :531-534); 9 continuation bytes → `leb128_too_long` (read_leb128 loops i in 0..8, falls through — no ninth read, no shift beyond `<<49`, no u64 overflow).

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/video/av1/obu.rs:333-335 (plus the locals feeding it at 181-182)
Code:
```
    if !may_produce_picture && has_frame_header && has_tile_group {
        may_produce_picture = true;
    }
```
What is wrong: This fixup is provably dead. `has_frame_header` is set only in the `OBU_FRAME_HEADER | OBU_REDUNDANT_FRAME_HEADER` arm (:300-303), which sets `may_produce_picture = true` on the same path; `has_tile_group` is set only in the `OBU_TILE_GROUP` arm (:304-307), which also sets `may_produce_picture = true` on the same path. Therefore `has_frame_header && has_tile_group` implies `may_produce_picture`; the conjunction `!may_produce_picture && has_frame_header && has_tile_group` is unsatisfiable. The two locals `has_frame_header`/`has_tile_group` (:181-182) exist only to feed this dead branch.
Why it matters: Dead logic that reads as a live state machine transition misleads the next maintainer into believing a TU can reach this state (e.g. that a header-only OBU followed by a tile group is counted differently than one arm each). Mutation check: deleting :333-335 and the two locals compiles and passes all 11 tests plus the differential suite — nothing pins it, which is itself the proof it guards nothing.
What correct looks like: Delete the block and both locals (and their assignments), or — if the original intent was "a lone FRAME_HEADER without tiles may not emit" — implement that as a real rule (it currently is NOT the behavior: a header-only TU already sets may_produce_picture at :302, per the altref-overlay rationale at :292-298).
Confidence: HIGH — Verification: read; arms at :299-307 traced; no test references the fixup (baseline :524-534 all pass through the live arms only).
### F-002 [S4]
Where: crates/codecs/src/video/av1/obu.rs:352-363
Code:
```
    trait MustErr<E> {
        fn must_err(self) -> E;
    }

    impl<T, E> MustErr<E> for Result<T, E> {
        fn must_err(self) -> E {
```
What is wrong: Third copy of the identical test helper: `crates/codecs/tests/h264_hw_path.rs` and `crates/codecs/tests/hevc_hw_path.rs` each define their own `trait MustErr` (Grep: 3 definitions). testkit owns `Must` but not the error counterpart, so every negative-fixture suite re-invents it.
Why it matters: Test-helper drift (one copy gains a better panic message, others don't); noise per new suite.
What correct looks like: Add `MustErr` to `crates/testkit` beside `Must` and import it in all three sites.
Confidence: HIGH — Verification: Grep `trait MustErr` crates/ → obu.rs:352, h264_hw_path.rs, hevc_hw_path.rs.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `obu_error` | 121-127 | REAL | — | Pure typed-error constructor; 8 distinct codes emitted. |
| `read_leb128` | 134-158 | REAL | — | Bounded 8-iteration loop; truncation reports expected/consumed counts; `<< (i*7)` max `<<49` — no overflow; the u32::MAX value check lives at the caller (:253) where the spec window is enforced. |
| `parse_temporal_unit` | 169-343 | REAL | S3 | Total: every malformed-input path returns typed (11 negative tests). Count guard fires at 4097th OBU (:189-196). `checked_add` at :263 correct for 32-bit targets. Sizeless OBU swallows the remainder (:282-288) — the only spec-consistent interpretation. F-001 dead block at the tail. |
| `ObuInfo`/`TemporalUnit` (structs) | 68-116 | REAL | — | Absolute-offset payload ranges (test :413-416 documents the convention); census booleans documented with their consumer's contract. |
| `MAX_OBUS_PER_TU` + OBU consts | 42-65 | REAL | — | Type values 1-8,15 match spec §6.2-6.10; guard sized honestly (4K tile fan-out). |
| tests (11) | 388-557 | REAL | — | See §5. |
| `MustErr`/`obu` helpers | 352-386 | REAL | S4 | F-002. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| empty_tu_is_typed_truncation | YES | Asserts code string AND full Corrupt detail payload. |
| forbidden_and_reserved_bits_are_typed | YES | Single-byte inputs 0x80/0x01 → distinct codes. |
| sizeless_obu_takes_rest_of_unit | YES | Asserts exact payload ranges (1..1, 1..4) and payload_bytes=3. |
| sized_obus_chain_with_exact_consumption | YES | 3-OBU chain with extension ids and payload_bytes=13 — catches bit-order swaps (the comment at :216-221 says the differential suite pins this against real streams too). |
| frame_header_plus_tile_group_counts_as_picture | YES | Exercises both live census arms. |
| show_existing_overlay_tu_counts_as_picture | YES | The altref pattern: TD + header-only TU must count. |
| independent_frame_peek_matches_the_frame_header_bits | YES | Covers all four frame_types, show_existing, and the non-frame control (metadata) — 7 assertions. |
| truncated_payload_is_typed_with_expected_end | YES | Asserts expected=34/found=29 exactly. |
| leb128_boundaries | YES | 4 arms: multi-byte 300, 9-byte too-long, mid-truncation, >u32::MAX. |
| obu_count_guard_fires_typed | YES | Builds 4097 OBUs, asserts the code. |
| extension_byte_rules | YES | Reserved-low-bits and missing-extension-byte paths. |
| All 11 | Executed | baseline test-workspace.log.txt:524-534, all `ok`, none ignored. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "framing validator, not a bitstream parser … does not decode sequence-header or frame-header bits" | obu.rs:5-15 | REAL | Read: only headers, leb128, ranges, and a 3-bit peek (:310-323) are read; no header-bit decoding. |
| "one IVF packet = one TU, the demuxer's documented contract" | obu.rs:10-11 | REAL-UNPROVEN | ivf.rs:15-18 documents the passthrough; the differential fixtures pass byte-for-byte (baseline :891-904), but I did not re-read demux/src/ivf internals (another agent's file). |
| "bounded-OBU-count resource guard for hostile inputs" | obu.rs:20-21 | REAL | :189-196 + obu_count_guard test (baseline :530). |
| "a mid-stream drop aborts the process (rav1d 1.1.0 decode.rs on_error …)" | obu.rs:104-107 | UNVERIFIED | rav1d 1.1.0 source not present in sandbox (no ~/.cargo registry); the claimed upstream abort behavior cannot be checked here — decoder.rs's gating rationale rests on it. |
| "Every entry point is total: any byte string yields a parsed TU or a typed EngineError — no panics" | obu.rs:32-35 | REAL | Traced every index/arith path: `bytes.get` for header/ext/leb (:229, :137), `bytes[cursor]` only after the loop-condition bounds check (:188), guard+checked_add for size. Fuzz-lite lane in the differential suite (baseline :897) adds randomized evidence. |

## 7. Dead / duplicated / unreachable code
- F-001's fixup + the two locals — dead (see above).
- MustErr triplicated (F-002).
- Nothing unreachable: single production consumer (decoder.rs:860) is itself shipped-reachable (header table).

## 8. Dependencies & flags
- No external deps; media-types error vocabulary only. Module compiled under `codec-av1` (default-on; wasm targets never enable it — codecs/Cargo.toml:62-65 comment, rav1d doesn't build for wasm32).
- No unsafe. No Cargo.toml surface touched by this file.

## 9. Open questions for the next agent
- decoder.rs owner (my part2 covers the tail): confirm the pending-map "ages out at the cap" behavior for TUs whose census said may_produce_picture but which never emit — the overlay-TU doc (:91-97) promises this.
- Whoever audits rav1d admission records: the obu.rs:104-107 upstream-abort claim (on_error unwrap) should be quoted with a line number from the pinned tarball in the record; today it is prose only.
