# AUDIT: crates/codecs/src/audio/mp3/header.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-21 |
| Lines / functions | 653 / 24 (17 prod incl. 5 const tables + 4 enums/structs, 7 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.mp3` → demux `es/mp3.rs` mints track codec `"mp3"` → `apps/cli/src/pipeline_host.rs:261` `Mp3Decoder::new()` → `Mp3Decoder::decode` → `decoder.rs:131 parse_frame_header`; `walk_frames` via `decoder.rs:197 decode_stream`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | rg `walk_frames\|parse_frame_header\|scan_sync\|side_info_size` (callers: decoder.rs:131,197; seek.rs:37,128; xing.rs:125,290; fuzz.rs:48; mp2/header.rs:37-38 re-imports the family parser); rg `"mp3"` crates/demux/src (codec descriptor minted by es/mp3.rs:249, mp4/mod.rs:463,1171, ts/mod.rs:353, mkv/header.rs:827, ps/mod.rs:803); rg `impl DecoderProvider` apps/cli/src (pipeline_host.rs:235-381 dispatch); grep baseline test log (all 7 tests ran ok); sed -n verification of every quote; table diff vs demux/src/es/mp3.rs:126-138 (identical) |

## 1. One-paragraph verdict
This file is the real thing: the ISO 11172-3/13818-3 bitrate/samplerate tables are byte-for-byte correct (cross-checked against the spec and against ffmpeg's `ff_mpa_decode_header` coefficient scheme 144/144/72/12×4), every reserved bitfield is typed-rejected, frame arithmetic is u64-then-`u32::try_from` with no overflow path, and the walker is bounded with resync. All 7 tests executed and passed in the baseline run (`audit/runs/baseline/test-workspace.log.txt:420-426`). The one real functional gap is the refusal of free-format (bitrate index 0) streams — spec-legal, VLC-supported, refused here with a typed error and honestly documented as doc 04 §3.2's known gap. Everything else is hygiene-level: an over-strict reserved-emphasis rejection, deliberate table duplication with the demux crate, and two test-coverage holes (MPEG-2.5, Layer I padding).

## 2. What actually works (verified)
— Tables and formulas: verified by reading against ISO/ffmpeg semantics AND by baseline execution: `audio::mp3::header::tests::parses_the_canonical_header ... ok` etc., 7/7 ok (`audit/runs/baseline/test-workspace.log.txt:420-426`). Spot-recomputed: 144·128000/44100 = 417 (test:417 ✓), (12·448000/44100)·4 = 484 (test:484 ✓), 72·64000/24000 = 192 (test:192 ✓), padded 0x92 header → 418 ✓.
— Reachability verified by rg: CLI provider builds `Mp3Decoder` for `"mp3"|"mp1"` (`apps/cli/src/pipeline_host.rs:261-265`), and `"mp2"` reuses this file's parser via `mp2/header.rs:37-38`.

## 3. Findings
### F-001 [S3] Free-format MP3 (bitrate index 0) is refused outright; scan_sync also skips free-format sync words
Where: crates/codecs/src/audio/mp3/header.rs:301-311 and 389-391
Code:
```rust
        None if bitrate_index == 0 => {
            // Free-format: doc 04 §3.2's known gap — refuse, never guess.
            return Err(unsupported(
                "mp3.free_format",
```
```rust
/// free-format headers are NOT treated as sync points (they cannot be
/// framed by table arithmetic).
```
What is wrong: A spec-legal constant-bitrate stream that uses free-format encoding (bitrate index 0000 — legal in ISO 11172-3 §2.4.2.3, supported by VLC and ffmpeg via frame-size measurement) is refused at every layer: parse refuses, and `scan_sync` skips its sync words, so `decode_stream` reports `mp3.stream.no_frames` for a whole free-format file.
Why it matters: Free-format files are rare but real (some old rippers produce them); VLC plays them. This is a functional compatibility gap, not a crash.
What correct looks like: Measure the frame size from the distance to the next sync word (the standard free-format detection: sync at n and next sync at n+size, size = 144·bitrate/rate ⇒ bitrate = (size−padding)·rate/144), then trust the measured size. The refusal is at least typed and documented (doc 04 §3.2), which keeps this at S3 rather than higher.
Confidence: HIGH — Verification: read (spec + code path traced to `mp3.stream.no_frames` refusal).

### F-002 [S4] Reserved emphasis (bits `10`) rejected as corrupt — stricter than VLC/ffmpeg
Where: crates/codecs/src/audio/mp3/header.rs:332-345
Code:
```rust
    let emphasis = match bytes[3] & 0x3 {
        0 => Emphasis::None,
        1 => Emphasis::Ms50_15,
        2 => Emphasis::Reserved,
        _ => Emphasis::CcitJ17,
    };
    if emphasis == Emphasis::Reserved {
```
What is wrong: ffmpeg and libmad accept emphasis=2 headers and decode them (the field is informational); this parser rejects the frame as `mp3.header.emphasis_reserved`, so a stream with those bits set loses sync and (via `scan_sync`) the whole frame is skipped.
Why it matters: Blast radius is near-zero (emphasis 10 is essentially never produced), but it is a decode-refusal on data other decoders accept.
What correct looks like: Accept and carry the reserved value (or warn), reserving rejection for fields that actually break framing.
Confidence: MEDIUM — Verification: read (ffmpeg behavior from knowledge, not re-run — UNVERIFIED at binary level).

### F-003 [S4] ISO tables duplicated between codecs and demux crates
Where: crates/codecs/src/audio/mp3/header.rs:118-139 vs crates/demux/src/es/mp3.rs:126-138
Code:
```rust
const BITRATES_V1_L1: [u32; 14] = [
    32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
];
```
What is wrong: The five bitrate tables and the sample-rate table exist twice in the repo (codecs-side and demux-side), with frame-size arithmetic also duplicated (`144/72` at header.rs:219-229 and es/mp3.rs:222-224). Today they are identical (verified by side-by-side read); tomorrow a one-sided edit makes demux framing and decoder framing disagree.
Why it matters: Divergence would corrupt packet boundaries silently (demux cuts at one size, decoder validates another → `mp3.frame` errors on valid files).
What correct looks like: The layering note (es/mp3.rs:12-14 "this crate cannot depend on codecs") explains it; the correct fix is a shared `media-types`-level table module both can import. Documented, consistent today — S4.
Confidence: HIGH — Verification: read both, values compared entry-by-entry.

### F-004 [S4] Test-coverage holes: MPEG-2.5 and Layer I padding never tested
Where: crates/codecs/src/audio/mp3/header.rs:483-653 (test module)
Code:
```rust
    /// A canonical MPEG-1 Layer III 128 kbps 44.1 kHz stereo header.
    const MP3_128K_44100: [u8; 4] = [0xFF, 0xFB, 0x90, 0x00];
```
What is wrong: No test constructs an MPEG-2.5 header (version bits `00` — the `SAMPLE_RATES[0]` 11025/12000/8000 row and the V2_5 half-granule path in `samples_per_frame` are unexecuted by tests), and no test exercises the Layer I padding slot (padding adds 4 bytes there, 1 byte in II/III — the `*4` around the whole parenthesized expression at line 215).
Why it matters: The two most version-asymmetric code paths are pinned only by the fuzz harness (which asserts no panics, not values).
What correct looks like: One MPEG-2.5 L3 8 kHz header and one padded L1 header with exact frame_size assertions.
Confidence: HIGH — Verification: read (all 7 tests enumerated; none touch V2_5/padded-L1).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum MpegVersion` | 30-40 | REAL | — | data type; Reserved variant rejected at parse |
| `enum Layer` | 43-53 | REAL | — | data type |
| `enum ChannelMode` | 56-66 | REAL | — | data type |
| `enum Emphasis` | 69-79 | REAL | — | Reserved variant → F-002 |
| `struct FrameHeader` | 83-113 | REAL | — | fully validated on construct |
| `BITRATES_V1_L1/V1_L2/V1_L3/V2_L1/V2_L23` | 118-131 | REAL | — | verified against ISO 11172-3/13818-3 tables, entry-by-entry |
| `SAMPLE_RATES` | 134-139 | REAL | — | 2.5/2/1 rows correct; reserved row zeroed, unreachable |
| `table_bitrate` | 143-160 | REAL | — | index 0/15 → None; bounds via `row.get`; no panic path |
| `table_sample_rate` | 164-175 | REAL | — | index 3 → None; Reserved version → None (dead: rejected earlier) |
| `channel_count` | 179-184 | REAL | — | mono 1, others 2 ✓ |
| `samples_per_frame` | 188-197 | REAL | — | 384/1152/1152/576 ✓ matches spec+ffmpeg; Reserved → 0 (dead path) |
| `compute_frame_size` | 202-234 | REAL | — | u64 math, `u32::try_from`, L1 `(12·br/sr+pad)·4`, L2 144 all versions, L3 144/72 — all correct; zero-guard |
| `parse_frame_header` | 243-381 | REAL | — | 8 typed error paths all distinct (verified in test); no panics; bit extraction verified byte-by-byte (b2: br[7:4] sr[3:2] pad[1] priv[0]; b3: mode[7:6] ext[5:4] copy[3] orig[2] emph[1:0]) |
| `scan_sync` | 392-404 | REAL | — | O(n) bounded; free-format not a sync point (F-001); redundant sync pre-check before parse (harmless micro-opt) |
| `struct FrameAt` | 407-413 | REAL | — | data type |
| `walk_frames` | 424-453 | REAL | — | bounded loop; `saturating_add` (no overflow, frame_size ≤ 2881); truncated tail dropped (documented; diverges from ffmpeg's short final packet — noted in es/mp3.rs:33-35 doc); collects whole-buffer Vec (test/differential surface only, production decode is per-packet) |
| `side_info_size` | 458-477 | REAL | — | 17/32 (V1) and 9/17 (V2/2.5) ✓ spec; Reserved → 0 dead path |
| test `parses_the_canonical_header` | 486-501 | REAL | — | mutation: wrong 144-coeff → 417 assertion fails |
| test `layer_ii_and_mpeg2_lengths_match_the_spec_formulas` | 503-533 | REAL | — | pins L2/L3-MPEG2/L1 formulas independently |
| test `mode_extension_emphasis_and_crc_bits_round_trip` | 535-550 | REAL | — | pins bit extraction incl. protection bit |
| test `error_paths_are_each_typed` | 552-602 | REAL | — | 8/8 codes asserted; mutation: any removed check fails |
| test `scan_sync_recovers_past_garbage` | 604-616 | REAL | — | mutation: scan returns `from` → offset assertion fails |
| test `walk_frames_resyncs_and_drops_a_truncated_tail` | 618-641 | REAL | — | pins resync + truncation drop |
| test `padding_bit_adds_one_slot` | 643-652 | REAL | — | 418 = 417+1 ✓ |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 7 tests | YES | Every test asserts exact numeric frame sizes / typed codes / offsets computed by hand from the spec formulas, not by the code under test. No golden generated from the subject. All executed in baseline (test-workspace.log.txt:420-426). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "MP3 free-format is a known unsupported gap" | module doc line 22-23 citing doc 04 §3.2 `[VERIFY-P0]` | REAL (honest refusal) | header.rs:301-311 typed `mp3.free_format` |
| "one packet = one frame, offsets computed HERE" | module doc lines 5-9 | REAL | decoder.rs:131 `parse_frame_header(packet)?` then engine gets whole packet |
| "no loops on any input" (walk bounded) | header.rs:422 | REAL | while-loop strictly advances pos; `saturating_add` |
| MP3 decode end-to-end in shipped build | doc 04 §3.1 rows | REAL-UNPROVEN at binary level | CLI provider wires Mp3Decoder (pipeline_host.rs:261); no baseline run decodes an .mp3 file (smoke corpus is MP4/AVI only) — engine-info literal under-reports (baseline INDEX standing defect) |

## 7. Dead / duplicated / unreachable code
- Dead defensive arms (unreachable in practice, harmless): `table_bitrate`/`table_sample_rate` Reserved rows, `samples_per_frame`/`side_info_size` Reserved arms — all guarded by earlier parse rejection.
- Duplication with `crates/demux/src/es/mp3.rs` (F-003): tables + frame arithmetic, verified identical today.
- Nothing unreachable: `parse_frame_header` is the production hot path (decoder.rs:131), `walk_frames`/`scan_sync` serve decode_stream/seek, `side_info_size` serves xing.rs:125.

## 8. Dependencies & flags
- `media_types` (EngineError/CorruptDetail) and `super::error` only — no external deps, no feature gate on this file itself; the enclosing `codec-mp3` feature is in `codecs` default features (crates/codecs/Cargo.toml `default = [..., "codec-mp3", ...]`), so the shipped workspace build compiles it (baseline build #1 CLEAN).

## 9. Open questions for the next agent
- `crates/codecs/src/audio/mp3/error.rs` (another agent's file): confirm `corrupt()`/`unsupported()` produce the codes asserted here without allocation blowups.
- Whoever audits `apps/cli/src/bench.rs`: the decode-bench lane claims "no decoder in this build" for mp3-in-mp4 — but `pipeline_host.rs:261` proves an mp3 decoder exists; bench output is a hard-coded lie about capability (already a Wave-0 standing defect).
